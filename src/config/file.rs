//! Tracks Config File lifecycle states with typestate markers.
//!
//! [`ConfigFile`] carries a path and source marker through discovery, Trust
//! Verification, and parsing without permitting out-of-order transitions.
//!
//! # Key Types
//!
//! - [`LocalConfigFile`]: identifies a project-local Config File.
//! - [`GlobalConfigFile`]: identifies a user-global Config File.
//! - [`Discovered`]: marks a discovered Config File before verification.
//! - [`Trusted`]: holds local Config File content that passed Trust
//!   Verification.
//! - [`Parsed`]: holds parsed raw configuration from a Config File.
use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

use super::{
    error::{ConfigReadError, ConfigTrustError},
    raw::RawConfig,
    tracker::TrackerStore,
    trust::{ConfigTrustCheck, TrustRequest, TrustStore},
};

/// Names the local Config File directory relative to the project root.
pub(crate) const LOCAL_CONFIG_DIR: &str = ".traces";

/// Names the file shared by local and global Config Files.
pub(super) const CONFIG_FILE_NAME: &str = "config.toml";

/// Names the local Config File relative to the project root.
///
/// Matches [`LOCAL_CONFIG_DIR`] joined with [`CONFIG_FILE_NAME`]; a test pins
/// this invariant because `concat!` cannot reference constants.
pub(crate) const LOCAL_CONFIG_FILE: &str = ".traces/config.toml";

/// Marks a Config File as local to a project.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct IsLocal;

/// Marks a Config File as global to the user.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct IsGlobal;

/// Marks a discovered Config File without asserting that it exists.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Discovered;

/// Holds local Config File content that passed Trust Verification.
///
/// Parsing reuses the checked content, avoiding a second read that could race
/// with a change on disk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Trusted {
    content: String,
}

/// Holds [`RawConfig`] parsed from a Config File.
#[derive(Clone, Debug)]
pub(crate) struct Parsed {
    raw: Box<RawConfig>,
}

impl Parsed {
    /// Reads and parses a global Config File from disk.
    ///
    /// Global files have no Trust Verification gate, so parsing reads them
    /// directly.
    ///
    /// # Errors
    ///
    /// - [`Io`] if the Config File cannot be read.
    /// - [`Parse`] if its content cannot be parsed as TOML.
    ///
    /// [`Io`]: ConfigReadError::Io
    /// [`Parse`]: ConfigReadError::Parse
    fn read(path: &Path) -> Result<Self, ConfigReadError> {
        let content = std::fs::read_to_string(path).map_err(|source| {
            ConfigReadError::Io {
                path: path.to_path_buf(),
                source,
            }
        })?;
        Self::from_content(path, &content)
    }

    /// Parses Config File content already read from disk.
    ///
    /// The path supplies error context; local Trust Verification provides the
    /// content, so parsing never reads it again.
    ///
    /// # Errors
    ///
    /// - [`Parse`] if the content cannot be parsed as TOML.
    ///
    /// [`Parse`]: ConfigReadError::Parse
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

/// Identifies a project-local [`ConfigFile`].
pub(crate) type LocalConfigFile<State> = ConfigFile<IsLocal, State>;

/// Identifies a user-global [`ConfigFile`].
pub(crate) type GlobalConfigFile<State> = ConfigFile<IsGlobal, State>;

/// Carries a Config File through source-specific lifecycle states.
///
/// `Source` distinguishes local from global files; `State` restricts valid
/// transitions at compile time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConfigFile<Source, State> {
    root: PathBuf,
    path: PathBuf,
    state: State,
    _marker: PhantomData<Source>,
}

impl<Source, State> ConfigFile<Source, State> {
    /// Returns the root used to resolve this Config File's paths.
    ///
    /// Local files use the project root; global files use the global Config
    /// File directory.
    #[inline]
    #[must_use]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the Config File's filesystem path.
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

    /// Moves the Config File into the next lifecycle state.
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
    /// Derives a local Config File path from the project root.
    ///
    /// Path construction does not require the file to exist.
    pub(crate) fn at(root: &Path) -> Self {
        Self::new(root.to_path_buf(), root.join(LOCAL_CONFIG_FILE), Discovered)
    }

    /// Derives the project root from a local Config File path.
    ///
    /// Returns `None` unless the path ends in `.traces/config.toml` and the
    /// `.traces` directory has a parent.
    #[inline]
    pub(crate) fn from_config_path(path: &Path) -> Option<Self> {
        let traces_dir = path.parent()?;
        if path.file_name() != Some(CONFIG_FILE_NAME.as_ref())
            || traces_dir.file_name() != Some(LOCAL_CONFIG_DIR.as_ref())
        {
            return None;
        }
        let root = traces_dir.parent()?;
        Some(Self::new(root.to_path_buf(), path.to_path_buf(), Discovered))
    }

    /// Records a Tracked Config Path and performs Trust Verification.
    ///
    /// The Tracker Store records the path best-effort before verification, so
    /// refused Config Files remain tracked. On success, the returned file
    /// carries the verified content for parsing.
    ///
    /// # Errors
    ///
    /// - [`Untrusted`] if the Trust Status is untrusted, stale, or missing a
    ///   Companion Hash.
    /// - [`Tracker`] if the Trust Store cannot be read.
    /// - [`Hash`] if the Config File cannot be read for hashing.
    ///
    /// [`Untrusted`]: ConfigTrustError::Untrusted
    /// [`Tracker`]: ConfigTrustError::Tracker
    /// [`Hash`]: ConfigTrustError::Hash
    #[inline]
    pub(crate) fn into_trusted(
        self,
        tracker: &TrackerStore,
        trust: &TrustStore,
    ) -> Result<LocalConfigFile<Trusted>, ConfigTrustError> {
        tracker.record_best_effort(self.path());
        match trust.config_file_trust_check(self.root(), self.path())? {
            ConfigTrustCheck::Trusted(content) => {
                Ok(self.transition_to(Trusted {
                    content,
                }))
            }
            other @ (ConfigTrustCheck::Untrusted
            | ConfigTrustCheck::Stale
            | ConfigTrustCheck::MissingBaseline) => {
                Err(ConfigTrustError::Untrusted {
                    root: self.root,
                    path: self.path,
                    status: other.status(),
                })
            }
        }
    }
}

impl GlobalConfigFile<Discovered> {
    /// Derives a global Config File path from its directory.
    #[inline]
    pub(super) fn in_dir(dir: &Path) -> Self {
        Self::new(dir.to_path_buf(), dir.join(CONFIG_FILE_NAME), Discovered)
    }
}

impl<Source> ConfigFile<Source, Parsed> {
    /// Returns parsed [`RawConfig`] from this Config File.
    #[inline]
    #[must_use]
    pub(super) fn raw(&self) -> &RawConfig {
        &self.state.raw
    }

    /// Parses in-memory Config File content for tests.
    ///
    /// # Errors
    ///
    /// - [`Parse`] if the test content cannot be parsed as TOML.
    ///
    /// [`Parse`]: ConfigReadError::Parse
    #[cfg(test)]
    pub(crate) fn from_content_for_test(
        root: PathBuf,
        path: PathBuf,
        content: &str,
    ) -> Result<Self, ConfigReadError> {
        let parsed = Parsed::from_content(&path, content)?;
        Ok(Self::new(root, path, parsed))
    }
}

impl TryFrom<LocalConfigFile<Trusted>> for LocalConfigFile<Parsed> {
    type Error = ConfigReadError;

    /// Parses content already verified for the local Config File.
    ///
    /// # Errors
    ///
    /// - [`Parse`] if the verified content cannot be parsed as TOML.
    ///
    /// [`Parse`]: ConfigReadError::Parse
    #[inline]
    fn try_from(file: LocalConfigFile<Trusted>) -> Result<Self, Self::Error> {
        let parsed = Parsed::from_content(file.path(), &file.state.content)?;
        Ok(file.transition_to(parsed))
    }
}

impl TryFrom<GlobalConfigFile<Discovered>> for GlobalConfigFile<Parsed> {
    type Error = ConfigReadError;

    /// Reads and parses the discovered global Config File.
    ///
    /// # Errors
    ///
    /// - [`Io`] if the Config File cannot be read.
    /// - [`Parse`] if its content cannot be parsed as TOML.
    ///
    /// [`Io`]: ConfigReadError::Io
    /// [`Parse`]: ConfigReadError::Parse
    #[inline]
    fn try_from(
        file: GlobalConfigFile<Discovered>,
    ) -> Result<Self, Self::Error> {
        let parsed = Parsed::read(file.path())?;
        Ok(file.transition_to(parsed))
    }
}

impl From<&LocalConfigFile<Discovered>> for TrustRequest {
    /// Creates a [`TrustRequest`] for a discovered local Config File.
    #[inline]
    fn from(file: &LocalConfigFile<Discovered>) -> Self {
        Self::Config {
            root: file.root().to_path_buf(),
            path: file.path().to_path_buf(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    mod layout {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn constants_agree() {
            assert_eq!(
                LOCAL_CONFIG_FILE,
                format!("{LOCAL_CONFIG_DIR}/{CONFIG_FILE_NAME}")
            );
        }

        #[test]
        fn at_places_the_config_under_the_root() {
            let root = Path::new("/project");

            let config = LocalConfigFile::at(root);

            assert_eq!(config.root(), root);
            assert_eq!(
                config.path(),
                Path::new("/project/.traces/config.toml")
            );
        }

        #[test]
        fn in_dir_places_the_global_config_in_the_directory() {
            let dir = Path::new("/config/traces");

            let config = GlobalConfigFile::in_dir(dir);

            assert_eq!(config.root(), dir);
            assert_eq!(config.path(), Path::new("/config/traces/config.toml"));
        }
    }

    mod from_config_path {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn derives_root_from_traces_config_path() {
            let root = PathBuf::from("/project");
            let path = root.join(".traces/config.toml");

            let config = LocalConfigFile::from_config_path(&path)
                .expect("valid local config path");

            assert_eq!(config.root(), root.as_path());
            assert_eq!(config.path(), path.as_path());
        }

        #[rstest]
        #[case("/project/config.toml")] // parent is not .traces
        #[case("/project/.traces/other.toml")] // file is not config.toml
        #[case("config.toml")] // no parent
        fn rejects_other_paths(#[case] path: &str) {
            assert_eq!(
                LocalConfigFile::from_config_path(Path::new(path)),
                None
            );
        }
    }

    mod into_trusted {
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::config::trust::ConfigTrustStatus;

        struct Fixture {
            _temp: tempfile::TempDir,
            root: PathBuf,
            path: PathBuf,
            tracker: TrackerStore,
            trust: TrustStore,
        }

        impl Fixture {
            fn new(content: &str) -> Self {
                let temp = tempfile::tempdir().expect("temp dir");
                let root = temp.path().join("project");
                let path = root.join(".traces/config.toml");
                std::fs::create_dir_all(path.parent().expect("config parent"))
                    .expect("create config directory");
                std::fs::write(&path, content).expect("write config");
                let tracker = TrackerStore::at(temp.path().join("tracked"));
                let trust = TrustStore::at(temp.path().join("trust"));
                Self {
                    _temp: temp,
                    root,
                    path,
                    tracker,
                    trust,
                }
            }

            fn discovered(&self) -> LocalConfigFile<Discovered> {
                LocalConfigFile::from_config_path(&self.path)
                    .expect("discover config")
            }

            fn grant_baseline(&self) {
                self.trust
                    .trust(&TrustRequest::from(&self.discovered()))
                    .expect("trust config");
            }

            fn refusal(&self) -> ConfigTrustError {
                self.discovered()
                    .into_trusted(&self.tracker, &self.trust)
                    .expect_err("config is refused")
            }
        }

        #[test]
        fn yields_trusted_file_when_baseline_matches() {
            let fixture = Fixture::new("");
            fixture.grant_baseline();

            let trusted = fixture
                .discovered()
                .into_trusted(&fixture.tracker, &fixture.trust)
                .expect("trusted");

            assert_eq!(trusted.path(), fixture.path.as_path());
        }

        #[test]
        fn refuses_untrusted_when_the_root_is_not_trusted() {
            let fixture = Fixture::new("");

            assert!(matches!(fixture.refusal(), ConfigTrustError::Untrusted {
                status: ConfigTrustStatus::Untrusted,
                ..
            }));
        }

        #[test]
        fn refuses_missing_baseline_when_only_the_workspace_is_trusted() {
            let fixture = Fixture::new("");
            fixture
                .trust
                .trust(&TrustRequest::from(fixture.root.as_path()))
                .expect("trust workspace without config baseline");

            assert!(matches!(fixture.refusal(), ConfigTrustError::Untrusted {
                status: ConfigTrustStatus::MissingBaseline,
                ..
            }));
        }

        #[test]
        fn refuses_stale_when_the_content_hash_differs() {
            let fixture = Fixture::new("old");
            fixture.grant_baseline();
            std::fs::write(&fixture.path, "new").expect("change config");

            assert!(matches!(fixture.refusal(), ConfigTrustError::Untrusted {
                status: ConfigTrustStatus::Stale,
                ..
            }));
        }

        #[test]
        fn tracks_the_path_even_when_refused() {
            let fixture = Fixture::new("");

            let error = fixture.refusal();

            assert!(matches!(error, ConfigTrustError::Untrusted { .. }));
            assert_eq!(fixture.tracker.list().expect("list").len(), 1);
        }

        #[test]
        fn fails_with_hash_error_when_the_config_is_unreadable() {
            let fixture = Fixture::new("");
            fixture.grant_baseline();
            std::fs::remove_file(&fixture.path).expect("remove config");

            let result = fixture
                .discovered()
                .into_trusted(&fixture.tracker, &fixture.trust);

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
