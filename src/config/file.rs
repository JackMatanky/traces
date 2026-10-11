//! Tracks config-file lifecycle states using typestate markers.
//!
//! [`ConfigFile`] pairs a config path with source and lifecycle markers so the
//! loader expresses valid transitions in types.
//!
//! # Lifecycle
//!
//! - [`Discovered`] means the path exists on disk but has not been tracked or
//!   trusted.
//! - [`Tracked`] means local discovery recorded the path in the best-effort
//!   store.
//! - [`Trusted`] carries local TOML content verified against a trust baseline.
//! - [`Parsed`] carries TOML decoded into [`RawConfig`].

use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

#[cfg(test)]
use super::trust::TrustRequest;
use super::{
    error::{ConfigDiscoveryError, ConfigReadError, ConfigTrustError},
    raw::RawConfig,
    tracker::TrackerStore,
    trust::{ConfigTrustCheck, ConfigTrustStatus, TrustStore},
};

/// Source marker for a local project config file.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct IsLocal;

/// Source marker for a global user config file.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct IsGlobal;

/// A config file discovered on disk, before tracking or trust checks.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Discovered;

/// A local config file recorded in the best-effort tracking store.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Tracked;

/// Local config content verified against a trust baseline.
///
/// Carries the exact content read during trust checking. Parsing reuses this
/// string so a second filesystem read cannot race the trusted content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Trusted {
    content: String,
}

/// Raw config data parsed from one validated config file.
#[derive(Clone, Debug)]
pub(crate) struct Parsed {
    raw: Box<RawConfig>,
}

impl Parsed {
    /// Parses `path`'s content directly from disk.
    ///
    /// Used for global config, which has no trust gate and thus no risk of a
    /// second, independent read racing the first.
    ///
    /// # Errors
    ///
    /// - [`ConfigReadError::Io`] when `path` cannot be read.
    /// - [`ConfigReadError::Parse`] when `path`'s content cannot be parsed as
    ///   TOML.
    fn read(path: &Path) -> Result<Self, ConfigReadError> {
        let content = std::fs::read_to_string(path).map_err(|source| {
            ConfigReadError::Io {
                path: path.to_path_buf(),
                source,
            }
        })?;
        Self::from_content(path, &content)
    }

    /// Parses already-read `content` for `path`.
    ///
    /// `path` is used only for error context. Local config content was read
    /// while verifying trust, so this avoids a second independent read through
    /// [`Self::read`].
    ///
    /// # Errors
    ///
    /// - [`ConfigReadError::Parse`] when `content` cannot be parsed as TOML.
    fn from_content(
        path: &Path,
        content: &str,
    ) -> Result<Self, ConfigReadError> {
        let raw = toml::from_str::<RawConfig>(content).map_err(|source| {
            ConfigReadError::Parse {
                path: path.to_path_buf(),
                source: Box::new(source),
            }
        })?;
        Ok(Self {
            raw: Box::new(raw),
        })
    }
}

/// A local project config file.
pub(crate) type LocalConfigFile<State> = ConfigFile<IsLocal, State>;

/// A global user config file.
pub(crate) type GlobalConfigFile<State> = ConfigFile<IsGlobal, State>;

/// A config file tracked through its lifecycle via typestate markers.
///
/// `Source` distinguishes local from global files; `State` enforces valid
/// transitions at compile time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfigFile<Source, State> {
    root: PathBuf,
    path: PathBuf,
    state: State,
    _marker: PhantomData<Source>,
}

impl<Source, State> ConfigFile<Source, State> {
    /// Root directory used for path resolution.
    ///
    /// For a local config this is the project root; for a global config it is
    /// the global config directory.
    #[inline]
    #[must_use]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Filesystem path to the config file.
    #[inline]
    #[must_use]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    const fn new(root: PathBuf, path: PathBuf, state: State) -> Self {
        Self {
            root,
            path,
            state,
            _marker: PhantomData,
        }
    }

    /// Moves the config file into the next lifecycle state.
    fn transition_to<NextState>(
        self,
        next_state: NextState,
    ) -> ConfigFile<Source, NextState> {
        ConfigFile {
            root: self.root,
            path: self.path,
            state: next_state,
            _marker: PhantomData,
        }
    }
}

impl LocalConfigFile<Discovered> {
    /// Creates a discovered local config file from a `.traces/config.toml`
    /// path.
    ///
    /// Derives the project root from the parent of the `.traces` directory.
    ///
    /// # Errors
    ///
    /// - [`ConfigDiscoveryError::UnsupportedLocalCandidate`] when `path` does
    ///   not end with `.traces/config.toml` or has no parent `.traces`
    ///   directory.
    #[inline]
    pub(crate) fn try_new(path: PathBuf) -> Result<Self, ConfigDiscoveryError> {
        let Some(traces_dir) = path.parent() else {
            return Err(ConfigDiscoveryError::UnsupportedLocalCandidate {
                path,
            });
        };
        if traces_dir.file_name() != Some(".traces".as_ref())
            || path.file_name() != Some("config.toml".as_ref())
        {
            return Err(ConfigDiscoveryError::UnsupportedLocalCandidate {
                path,
            });
        }
        let Some(root) = traces_dir.parent() else {
            return Err(ConfigDiscoveryError::UnsupportedLocalCandidate {
                path,
            });
        };
        Ok(Self::new(root.to_path_buf(), path, Discovered))
    }

    /// Records this file and transitions it to the tracked state.
    ///
    /// Records the file in the tracker store on a best-effort basis: failures
    /// are logged, never returned. It always transitions to [`Tracked`].
    #[inline]
    pub(crate) fn into_tracked(
        self,
        tracked: &TrackerStore,
    ) -> LocalConfigFile<Tracked> {
        tracked.record(&self);
        self.transition_to(Tracked)
    }
}

impl GlobalConfigFile<Discovered> {
    /// Creates a discovered global config file from a `config.toml` path.
    ///
    /// Derives the root directory from the file's parent.
    ///
    /// # Errors
    ///
    /// - [`ConfigDiscoveryError::UnsupportedGlobalCandidate`] when `path` is
    ///   not named `config.toml` or has no parent directory.
    #[inline]
    pub(super) fn try_new(path: PathBuf) -> Result<Self, ConfigDiscoveryError> {
        if path.file_name() != Some("config.toml".as_ref()) {
            return Err(ConfigDiscoveryError::UnsupportedGlobalCandidate {
                path,
            });
        }
        let Some(root) = path.parent() else {
            return Err(ConfigDiscoveryError::UnsupportedGlobalCandidate {
                path,
            });
        };
        Ok(Self::new(root.to_path_buf(), path, Discovered))
    }
}

/// Result of checking whether a tracked config file may be parsed.
pub(crate) enum TrustOutcome {
    /// The file is trusted and ready to be parsed.
    Trusted(LocalConfigFile<Trusted>),
    /// The file is untrusted, missing its baseline hash, or stale.
    Halted(LocalConfigFile<Tracked>, ConfigTrustStatus),
}

impl LocalConfigFile<Tracked> {
    /// Verifies the trust status of this tracked config file.
    ///
    /// Yields [`TrustOutcome::Trusted`] when the workspace is trusted and the
    /// content hash matches the baseline. Yields [`TrustOutcome::Halted`] when
    /// trust is absent, the baseline is missing, or the content is stale,
    /// allowing the caller to prompt the user.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] when the underlying trust-store I/O
    ///   fails.
    /// - [`ConfigTrustError::Hash`] when hashing the config file fails.
    pub(crate) fn verify_trust(
        self,
        trust: &TrustStore,
    ) -> Result<TrustOutcome, ConfigTrustError> {
        let check = trust.config_file_trust_check(self.root(), self.path())?;
        Ok(match check {
            ConfigTrustCheck::Trusted(content) => {
                TrustOutcome::Trusted(self.transition_to(Trusted {
                    content,
                }))
            }
            other => {
                let status = other.status();
                TrustOutcome::Halted(self, status)
            }
        })
    }
}

impl<Source> ConfigFile<Source, Parsed> {
    /// Parsed raw config data.
    #[inline]
    #[must_use]
    pub(super) fn raw(&self) -> &RawConfig {
        &self.state.raw
    }
}

impl TryFrom<LocalConfigFile<Trusted>> for LocalConfigFile<Parsed> {
    type Error = ConfigReadError;

    #[inline]
    fn try_from(file: LocalConfigFile<Trusted>) -> Result<Self, Self::Error> {
        let parsed = Parsed::from_content(file.path(), &file.state.content)?;
        Ok(file.transition_to(parsed))
    }
}

impl TryFrom<GlobalConfigFile<Discovered>> for GlobalConfigFile<Parsed> {
    type Error = ConfigReadError;

    #[inline]
    fn try_from(
        file: GlobalConfigFile<Discovered>,
    ) -> Result<Self, Self::Error> {
        let parsed = Parsed::read(file.path())?;
        Ok(file.transition_to(parsed))
    }
}

#[cfg(test)]
impl<Source> ConfigFile<Source, Parsed> {
    pub(crate) fn from_content_for_test(
        root: PathBuf,
        path: PathBuf,
        content: &str,
    ) -> Result<Self, ConfigReadError> {
        let parsed = Parsed::from_content(&path, content)?;
        Ok(Self::new(root, path, parsed))
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    mod local_constructor {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn derives_root_from_traces_config_path() {
            let root = PathBuf::from("/project");
            let path = root.join(".traces/config.toml");

            let config = LocalConfigFile::<Discovered>::try_new(path.clone())
                .expect("valid local config path");

            assert_eq!(config.root(), root.as_path());
            assert_eq!(config.path(), path.as_path());
        }

        use rstest::rstest;

        #[rstest]
        #[case("/project/config.toml")] // parent is not .traces
        #[case("/project/.traces/other.toml")] // file is not config.toml
        #[case("config.toml")] // no parent
        fn rejects_invalid_paths(#[case] path: &str) {
            let error =
                LocalConfigFile::<Discovered>::try_new(PathBuf::from(path))
                    .expect_err(&format!("expected rejection for {path}"));
            assert!(matches!(
                error,
                ConfigDiscoveryError::UnsupportedLocalCandidate { .. }
            ));
        }
    }

    mod global_constructor {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn derives_root_from_parent_directory() {
            let root = PathBuf::from("/config/traces");
            let path = root.join("config.toml");

            let config = GlobalConfigFile::<Discovered>::try_new(path.clone())
                .expect("valid global config path");

            assert_eq!(config.root(), root.as_path());
            assert_eq!(config.path(), path.as_path());
        }

        use rstest::rstest;

        #[rstest]
        #[case("/config/traces/settings.toml")] // file is not config.toml
        fn rejects_invalid_paths(#[case] path: &str) {
            let error =
                GlobalConfigFile::<Discovered>::try_new(PathBuf::from(path))
                    .expect_err(&format!("expected rejection for {path}"));
            assert!(matches!(
                error,
                ConfigDiscoveryError::UnsupportedGlobalCandidate { .. }
            ));
        }
    }

    mod tracking_transitions {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn transitions_to_tracked_state() {
            let temp = tempfile::tempdir().expect("temp");
            let store = TrackerStore::at(temp.path().join("tracked"));
            let file = LocalConfigFile::<Discovered>::try_new(PathBuf::from(
                "/project/.traces/config.toml",
            ))
            .expect("valid local config path");

            let tracked = file.into_tracked(&store);

            assert_eq!(
                tracked.path(),
                Path::new("/project/.traces/config.toml")
            );
        }
    }

    mod trust_transitions {
        use super::*;

        struct Fixture {
            _temp: tempfile::TempDir,
            root: PathBuf,
            path: PathBuf,
            tracked_store: TrackerStore,
            trust_store: TrustStore,
        }

        impl Fixture {
            fn new(content: &str) -> Self {
                let temp = tempfile::tempdir().expect("temp dir");
                let root = temp.path().join("project");
                let path = root.join(".traces/config.toml");
                std::fs::create_dir_all(path.parent().expect("config parent"))
                    .expect("create config directory");
                std::fs::write(&path, content).expect("write config");
                let tracked_store =
                    TrackerStore::at(temp.path().join("tracked"));
                let trust_store = TrustStore::at(temp.path().join("trust"));
                Self {
                    _temp: temp,
                    root,
                    path,
                    tracked_store,
                    trust_store,
                }
            }

            fn tracked(&self) -> LocalConfigFile<Tracked> {
                LocalConfigFile::<Discovered>::try_new(self.path.clone())
                    .expect("discover config")
                    .into_tracked(&self.tracked_store)
            }
        }

        #[test]
        fn transitions_to_trusted_when_store_says_trusted() {
            let fixture = Fixture::new("");
            let tracked = fixture.tracked();
            fixture
                .trust_store
                .trust(&TrustRequest::from(&tracked))
                .expect("trust config");

            let result = tracked.verify_trust(&fixture.trust_store);

            assert!(matches!(result, Ok(TrustOutcome::Trusted(_))));
        }

        #[test]
        fn halts_untrusted_when_the_root_is_not_trusted() {
            let fixture = Fixture::new("");
            let tracked = fixture.tracked();

            let result = tracked.verify_trust(&fixture.trust_store);

            assert!(matches!(
                result,
                Ok(TrustOutcome::Halted(_, ConfigTrustStatus::Untrusted))
            ));
        }

        #[test]
        fn halts_with_missing_baseline_when_only_the_workspace_is_trusted() {
            let fixture = Fixture::new("");
            let tracked = fixture.tracked();
            fixture
                .trust_store
                .trust(&TrustRequest::from(fixture.root.as_path()))
                .expect("trust workspace without config baseline");

            let result = tracked.verify_trust(&fixture.trust_store);

            assert!(matches!(
                result,
                Ok(TrustOutcome::Halted(_, ConfigTrustStatus::MissingBaseline))
            ));
        }

        #[test]
        fn halts_stale_when_the_content_hash_differs() {
            let fixture = Fixture::new("old");
            let tracked = fixture.tracked();
            fixture
                .trust_store
                .trust(&TrustRequest::from(&tracked))
                .expect("trust original config");
            std::fs::write(&fixture.path, "new").expect("change config");

            let result = tracked.verify_trust(&fixture.trust_store);

            assert!(matches!(
                result,
                Ok(TrustOutcome::Halted(_, ConfigTrustStatus::Stale))
            ));
        }

        #[test]
        fn fails_with_hash_error_when_the_config_is_unreadable() {
            let fixture = Fixture::new("");
            let tracked = fixture.tracked();
            fixture
                .trust_store
                .trust(&TrustRequest::from(&tracked))
                .expect("trust config before removal");
            std::fs::remove_file(&fixture.path).expect("remove config");

            let result = tracked.verify_trust(&fixture.trust_store);

            assert!(matches!(result, Err(ConfigTrustError::Hash(_))));
        }
    }

    mod parsing {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn reads_valid_toml_into_raw_config() {
            let temp = tempfile::tempdir().expect("temp dir");
            let path = temp.path().join("config.toml");
            std::fs::write(&path, "[templates]\noutput_dir = \"out\"")
                .expect("write config");

            let parsed = Parsed::read(&path).expect("parse config");

            assert_eq!(
                parsed.raw.templates.output_dir.as_deref(),
                Some(Path::new("out"))
            );
        }

        #[test]
        fn returns_parse_error_on_invalid_toml() {
            let temp = tempfile::tempdir().expect("temp dir");
            let path = temp.path().join("config.toml");
            std::fs::write(&path, "[templates\nbad = ")
                .expect("write malformed config");

            let result = Parsed::read(&path);

            let err = result.expect_err("malformed config should fail");
            assert!(matches!(err, ConfigReadError::Parse { .. }));
            assert!(err.to_string().contains("failed to parse TOML syntax"));
        }

        #[rstest]
        #[case::local_malformed(".traces/config.toml", "[[[BAD TOML")]
        #[case::global_malformed("config.toml", "[[[BAD TOML")]
        fn rejects_malformed_content_with_its_path(
            #[case] relative_path: &str,
            #[case] content: &str,
        ) {
            let path = PathBuf::from("/project").join(relative_path);

            let result = Parsed::from_content(&path, content);

            assert!(
                matches!(result, Err(ConfigReadError::Parse { path: failed_path, .. }) if failed_path == path)
            );
        }
        #[test]
        fn read_reports_io_error_not_parse_error_for_unreadable_path() {
            let temp = tempfile::tempdir().expect("temp dir");
            let path = temp.path().join("missing_parent/config.toml");

            let result = Parsed::read(&path);

            assert!(matches!(result, Err(ConfigReadError::Io { .. })));
        }
    }

    mod template_dir {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_joined_path_when_configured() {
            let temp = tempfile::tempdir().expect("temp dir");
            let root = temp.path().join("project");
            let path = root.join(".traces/config.toml");
            std::fs::create_dir_all(path.parent().expect("config parent"))
                .expect("create config directory");
            std::fs::write(&path, "[templates]\ndirectory = \"tmpl\"")
                .expect("write config");

            let parsed = Parsed::read(&path).expect("parse config");
            let file =
                LocalConfigFile::<Parsed>::new(root.clone(), path, parsed);

            assert_eq!(
                file.raw()
                    .templates
                    .directory
                    .as_ref()
                    .map(|dir| file.root().join(dir)),
                Some(root.join("tmpl"))
            );
        }

        #[test]
        fn returns_none_when_omitted() {
            let temp = tempfile::tempdir().expect("temp dir");
            let root = temp.path().join("project");
            let path = root.join(".traces/config.toml");
            std::fs::create_dir_all(path.parent().expect("config parent"))
                .expect("create config directory");
            std::fs::write(&path, "").expect("write empty config");

            let parsed = Parsed::read(&path).expect("parse config");
            let file = LocalConfigFile::<Parsed>::new(root, path, parsed);

            assert_eq!(
                file.raw()
                    .templates
                    .directory
                    .as_ref()
                    .map(|dir| file.root().join(dir)),
                None
            );
        }
    }
}
