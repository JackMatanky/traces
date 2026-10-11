//! Orchestrates config loading and trust administration.
//!
//! [`ConfigService`] is the single entry point for discovering, building, and
//! trusting config files.
//!
//! # Loading Pipeline
//!
//! 1. Discover local and global TOML candidates from `cwd`.
//! 2. Record local candidates in the tracking store.
//! 3. Verify trust before parsing local content.
//! 4. Parse TOML into [`super::raw::RawConfig`].
//! 5. Merge global before local so local values win.
//!
//! Trust administration resolves subjects and persists durable state through
//! two stores owned by [`ConfigService`]: [`TrustStore`] and
//! [`TrackerStore`].

use std::path::{Path, PathBuf};

use super::{
    ConfigBuilder,
    discovery::{self, DiscoveryContext, DiscoveryOutcome},
    error::{ConfigError, ConfigTrustError},
    file::{
        Discovered as FileDiscovered, GlobalConfigFile, LocalConfigFile,
        Parsed, TrustOutcome, Trusted,
    },
    model::Config,
    tracker::TrackerStore,
    trust::{TrustRequest, TrustStore},
};

/// Selects the deepest local config containing the discovery anchor, plus the
/// optional global config merged before it.
#[derive(Debug)]
struct ConfigBuilderInput {
    /// Selected local config; this is merged after `global`.
    local: LocalConfigFile<FileDiscovered>,
    /// Optional global config; this is merged before `local`.
    global: Option<GlobalConfigFile<FileDiscovered>>,
}

impl TryFrom<DiscoveryOutcome> for ConfigBuilderInput {
    type Error = ConfigError;

    #[inline]
    fn try_from(outcome: DiscoveryOutcome) -> Result<Self, Self::Error> {
        let (_kind, anchor, discovered_locals, discovered_globals) =
            outcome.into_parts();
        let anchor_path = anchor.path().to_path_buf();

        let discovered_locals = discovered_locals.into_vec();
        if discovered_locals.is_empty() {
            return Err(super::error::ConfigDiscoveryError::NotFound {
                cwd: anchor_path,
            }
            .into());
        }

        let local = discovered_locals
            .into_iter()
            .filter(|file| anchor_path.starts_with(file.root()))
            .max_by_key(|file| file.root().components().count())
            .ok_or(super::error::ConfigDiscoveryError::NotFound {
                cwd: anchor_path,
            })?;
        let global = discovered_globals.into_iter().next();
        Ok(Self {
            local,
            global,
        })
    }
}

/// Entry point for config loading and trust administration.
///
/// Filesystem discovery (`load`) and `TrustRequest` operations (`trust`,
/// `untrust`) are separate surfaces on this type.
///
/// Durable state lives in two stores:
///
/// - `TrackerStore` records config files discovery has seen.
/// - `TrustStore` records workspace roots and config content baselines.
#[derive(Clone, Debug)]
pub struct ConfigService {
    tracked: TrackerStore,
    trust: TrustStore,
}

impl ConfigService {
    /// Creates a service backed by the OS-correct tracked-config and trust
    /// stores.
    #[must_use]
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            tracked: TrackerStore::new(),
            trust: TrustStore::new(),
        }
    }

    /// Creates a service backed by explicit tracked-config and trust-store
    /// roots.
    ///
    /// Test-only constructor for `crate::cli::trust` tests that need isolated
    /// stores instead of real OS state directories.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    pub fn at(tracked_root: PathBuf, trusted_root: PathBuf) -> Self {
        Self {
            tracked: TrackerStore::at(tracked_root),
            trust: TrustStore::at(trusted_root),
        }
    }

    /// Returns the trust-administration store.
    #[inline]
    #[must_use]
    pub(crate) const fn trust_store(&self) -> &TrustStore {
        &self.trust
    }

    /// Returns the tracked-config store.
    #[inline]
    #[must_use]
    pub(crate) const fn tracker_store(&self) -> &TrackerStore {
        &self.tracked
    }

    /// Discovers and builds config from `cwd` in one service-owned pipeline.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::Discovery`] when no local config is found or discovery
    ///   cannot inspect a path.
    /// - [`ConfigError::Read`] when a selected config file cannot be read or
    ///   parsed.
    /// - [`ConfigError::Trust`] when the local config's workspace is not
    ///   trusted, is missing its baseline hash, or is stale, or when the trust
    ///   or tracking store cannot be read or hashed.
    /// - [`ConfigError::Value`] when a parsed config value fails domain
    ///   validation.
    #[inline]
    pub(crate) fn load(&self, cwd: &Path) -> Result<Config, ConfigError> {
        let discovered = Self::discover(cwd)?;
        self.build(discovered)
    }

    /// Discovers config files from `cwd`.
    ///
    /// The local project config is required; the global config is optional.
    ///
    /// # Errors
    ///
    /// - [`super::error::ConfigDiscoveryError::NotFound`] when no local config
    ///   exists in any ancestor directory.
    /// - [`super::error::ConfigDiscoveryError::Inaccessible`] when a path
    ///   cannot be accessed.
    #[inline]
    fn discover(
        cwd: &Path,
    ) -> super::error::ConfigDiscoveryResult<DiscoveryOutcome> {
        discovery::run(DiscoveryContext::Full(cwd.to_path_buf()))
    }

    /// Builds a [`Config`] from discovered candidates.
    ///
    /// Selects the local and optional global config files according to
    /// [`ConfigBuilderInput`]'s precedence rules. Parses and merges global
    /// settings before local settings, resolving the final output directory.
    ///
    /// Trust and tracking notes:
    ///
    /// - The local candidate's root is checked against the trust store before
    ///   parsing; a global candidate is never checked.
    /// - Recording the candidate in the tracking store is best-effort; a write
    ///   failure does not fail the build.
    ///
    /// # Errors
    ///
    /// - [`super::error::ConfigDiscoveryError::NotFound`] when discovery output
    ///   selected no local config, or none contains the discovery anchor.
    /// - [`ConfigError::Trust`] when the local config's workspace is not
    ///   trusted, is missing its baseline hash, or is stale
    ///   ([`ConfigTrustError::Untrusted`]), or when the trust store cannot be
    ///   read or the config file cannot be hashed
    ///   ([`ConfigTrustError::Tracker`], [`ConfigTrustError::Hash`]).
    /// - [`ConfigError::Read`] when a selected local or global config file
    ///   cannot be read or parsed.
    /// - [`ConfigError::Value`] when a parsed config value fails domain
    ///   validation.
    fn build(
        &self,
        discovered: DiscoveryOutcome,
    ) -> Result<Config, ConfigError> {
        let input = ConfigBuilderInput::try_from(discovered)?;
        let (root, trusted_local) = self.verify_trust(input.local)?;
        let parsed_local = LocalConfigFile::<Parsed>::try_from(trusted_local)?;
        let parsed_global = match input.global {
            Some(global) => Some(GlobalConfigFile::<Parsed>::try_from(global)?),
            None => None,
        };
        ConfigBuilder::new(root, parsed_local, parsed_global)
            .build()
            .map_err(Into::into)
    }

    /// Verifies trust for `local`, returning its project root and the trusted
    /// file ready to parse.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Untrusted`] if trust verification halts instead of
    ///   confirming trust.
    /// - [`ConfigTrustError::Tracker`] if the trust store cannot be read.
    /// - [`ConfigTrustError::Hash`] if the config file cannot be read for
    ///   hashing.
    fn verify_trust(
        &self,
        local: LocalConfigFile<FileDiscovered>,
    ) -> Result<(PathBuf, LocalConfigFile<Trusted>), ConfigTrustError> {
        let tracked_local = local.into_tracked(&self.tracked);
        let trusted_local = match tracked_local.verify_trust(&self.trust)? {
            TrustOutcome::Trusted(trusted) => trusted,
            TrustOutcome::Halted(file, status) => {
                return Err(ConfigTrustError::Untrusted {
                    root: file.root().to_path_buf(),
                    path: file.path().to_path_buf(),
                    status,
                });
            }
        };
        let root = trusted_local.root().to_path_buf();
        Ok((root, trusted_local))
    }

    /// Grants trust for a workspace root.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] when trust cannot be recorded.
    /// - [`ConfigTrustError::Hash`] when the config file cannot be hashed.
    #[inline]
    #[cfg_attr(
        not(any(test, feature = "test-utils")),
        expect(
            dead_code,
            reason = "no current caller in cli; kept for the test_support \
                      create_trusted_project/trust_project fixtures"
        )
    )]
    pub(crate) fn trust(
        &self,
        subject: &TrustRequest,
    ) -> Result<(), ConfigTrustError> {
        self.trust.trust(subject)
    }

    /// Removes trust for `subject`'s workspace root.
    ///
    /// # Errors
    ///
    /// - `ConfigTrustError::Tracker` when the trust store cannot remove the
    ///   root entry or its content-hash companion.
    #[inline]
    #[cfg_attr(
        not(any(test, feature = "test-utils")),
        expect(
            dead_code,
            reason = "pub fn on a privately-nested module; no current caller \
                      in cli, kept for the test_support fixtures and as a \
                      genuine administrative API"
        )
    )]
    pub fn untrust(
        &self,
        subject: &TrustRequest,
    ) -> Result<usize, ConfigTrustError> {
        self.trust.untrust(subject)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::*;
    use crate::config::{
        discovery::{DiscoveryAnchor, DiscoveryScope},
        error::{ConfigDiscoveryError, ConfigReadError, ConfigValueError},
        file::{Discovered, LocalConfigFile},
        trust::ConfigTrustStatus,
    };

    struct Fixture {
        temp: tempfile::TempDir,
        tracked_root: PathBuf,
        _trusted_root: PathBuf,
        service: ConfigService,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().expect("create temp dir");
            let tracked_root = temp.path().join("tracked-store");
            let trusted_root = temp.path().join("trust-store");
            let service =
                ConfigService::at(tracked_root.clone(), trusted_root.clone());
            Self {
                temp,
                tracked_root,
                _trusted_root: trusted_root,
                service,
            }
        }

        fn target_dir(&self, name: &str) -> PathBuf {
            let path = self.temp.path().join(name);
            fs::create_dir_all(&path).expect("create target dir");
            path
        }

        fn create_config(root: &Path, contents: &str) -> PathBuf {
            let config_path = root.join(".traces/config.toml");
            fs::create_dir_all(config_path.parent().unwrap())
                .expect("create config parent");
            fs::write(&config_path, contents).expect("write config");
            config_path
        }

        fn discovered_config(
            config_path: &Path,
        ) -> LocalConfigFile<Discovered> {
            LocalConfigFile::<Discovered>::try_new(config_path.to_path_buf())
                .expect("valid local config")
        }

        fn trust_config(&self, config_path: &Path) {
            let config = Self::discovered_config(config_path);
            self.service
                .trust(&TrustRequest::from(&config))
                .expect("trust candidate root");
        }
    }

    mod load {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_discovery_error_when_no_config_found() {
            // Arrange
            let fixture = Fixture::new();
            let cwd = fixture.target_dir("project/notes/daily");

            // Act
            let result = fixture.service.load(&cwd);

            // Assert
            assert!(matches!(
                result,
                Err(ConfigError::Discovery(
                    ConfigDiscoveryError::NotFound { .. }
                ))
            ));
        }

        #[test]
        fn discovers_and_builds_trusted_local_config() {
            // Arrange
            let fixture = Fixture::new();
            let root = fixture.target_dir("project");
            let cwd = root.join("notes/daily");
            fs::create_dir_all(&cwd).unwrap();

            let config_path = Fixture::create_config(
                &root,
                "[templates]\ndirectory = \".traces/templates\"\noutput_dir = \
                 \"notes\"",
            );
            fixture.trust_config(&config_path);

            // Act
            let result = fixture.service.load(&cwd);

            // Assert
            assert!(result.is_ok());
            let config = result.unwrap();
            assert_eq!(config.root(), root.as_path());
            assert_eq!(config.output_dir(), root.join("notes"));
        }

        #[test]
        fn fails_to_load_when_tag_filter_is_invalid() {
            // Arrange
            let fixture = Fixture::new();
            let root = fixture.target_dir("project");
            let cwd = root.join("notes");
            fs::create_dir_all(&cwd).unwrap();

            let config_path = Fixture::create_config(
                &root,
                "[templates]\ndirectory = \
                 \".traces/templates\"\n\n[tasks]\ntag_filters = \
                 [\"1invalid\"]\n",
            );
            fixture.trust_config(&config_path);

            // Act
            let result = fixture.service.load(&cwd);

            // Assert
            assert!(matches!(
                result,
                Err(ConfigError::Value(
                    ConfigValueError::InvalidTagFilter { .. }
                ))
            ));
        }
    }

    mod build {
        use pretty_assertions::assert_eq;

        use super::*;

        fn local_candidates(
            fixture: &Fixture,
        ) -> (PathBuf, PathBuf, DiscoveryOutcome) {
            let cwd = fixture.target_dir("project");
            let config_path = Fixture::create_config(&cwd, "");
            let local = Fixture::discovered_config(&config_path);
            let candidates = DiscoveryOutcome::new(
                DiscoveryAnchor::Directory(cwd.clone()),
                vec![local],
                Vec::new(),
            );
            (cwd, config_path, candidates)
        }

        #[test]
        fn records_candidate_in_tracking_store() {
            // Arrange
            let fixture = Fixture::new();
            let (_cwd, config_path, candidates) = local_candidates(&fixture);
            fixture.trust_config(&config_path);

            // Act
            let result = fixture.service.build(candidates);

            // Assert
            assert!(result.is_ok());
            let tracked = fixture.service.tracker_store().list().unwrap();
            assert_eq!(tracked, vec![config_path.canonicalize().unwrap()]);
        }

        #[test]
        fn tracking_record_is_idempotent() {
            // Arrange
            let fixture = Fixture::new();
            let (_cwd, config_path, candidates) = local_candidates(&fixture);
            fixture.trust_config(&config_path);
            fixture.service.build(candidates.clone()).unwrap();

            // Act
            let result = fixture.service.build(candidates);

            // Assert
            assert!(result.is_ok());
            let tracked = fixture.service.tracker_store().list().unwrap();
            assert_eq!(tracked.len(), 1);
        }

        #[test]
        fn succeeds_even_when_tracking_store_write_fails() {
            // Arrange
            let fixture = Fixture::new();
            let (cwd, config_path, candidates) = local_candidates(&fixture);
            fixture.trust_config(&config_path);

            // Occupy the tracked-store path with a file so directory creation
            // fails
            fs::write(&fixture.tracked_root, "").unwrap();

            // Act
            let result = fixture.service.build(candidates);

            // Assert
            assert!(result.is_ok());
            assert_eq!(result.unwrap().root(), cwd.as_path());
        }

        #[test]
        fn rejects_untrusted_root() {
            // Arrange
            let fixture = Fixture::new();
            let (_cwd, _config_path, candidates) = local_candidates(&fixture);
            // Do NOT trust the root

            // Act
            let result = fixture.service.build(candidates);

            // Assert
            assert!(matches!(
                result,
                Err(ConfigError::Trust(ConfigTrustError::Untrusted {
                    status: ConfigTrustStatus::Untrusted,
                    ..
                }))
            ));
        }

        #[test]
        fn rejects_trusted_but_stale_root() {
            // Arrange
            let fixture = Fixture::new();
            let (_cwd, config_path, candidates) = local_candidates(&fixture);
            fixture.trust_config(&config_path);

            // Edit config after trusting to make it stale
            fs::write(&config_path, "directory = \"changed\"").unwrap();

            // Act
            let result = fixture.service.build(candidates);

            // Assert
            assert!(matches!(
                result,
                Err(ConfigError::Trust(ConfigTrustError::Untrusted {
                    status: ConfigTrustStatus::Stale,
                    ..
                }))
            ));
        }

        #[test]
        fn rejects_trusted_root_without_a_baseline() {
            // Arrange
            let fixture = Fixture::new();
            let (cwd, _config_path, candidates) = local_candidates(&fixture);
            fixture
                .service
                .trust(&TrustRequest::from(cwd.as_path()))
                .expect("trust workspace root only");

            // Act
            let result = fixture.service.build(candidates);

            // Assert
            assert!(matches!(
                result,
                Err(ConfigError::Trust(ConfigTrustError::Untrusted {
                    status: ConfigTrustStatus::MissingBaseline,
                    ..
                }))
            ));
        }
    }

    /// Tests discovery-output selection and the build pipeline.
    ///
    /// Migrated from the standalone `builder` module and kept as a separate
    /// fixture because it needs local/global candidate helpers the outer test
    /// [`Fixture`] does not.
    ///
    /// [`Fixture`]: super::Fixture
    mod builder {
        use super::*;

        struct Fixture {
            temp: tempfile::TempDir,
            trust_store: tempfile::TempDir,
            tracked_store: tempfile::TempDir,
        }

        impl Fixture {
            fn new() -> Self {
                Self {
                    temp: tempfile::tempdir().expect("create temp dir"),
                    trust_store: tempfile::tempdir()
                        .expect("create trust store"),
                    tracked_store: tempfile::tempdir()
                        .expect("create tracked store"),
                }
            }

            fn service(&self) -> ConfigService {
                ConfigService::at(
                    self.tracked_store.path().to_path_buf(),
                    self.trust_store.path().to_path_buf(),
                )
            }

            fn write_config(&self, subpath: &str, contents: &str) -> PathBuf {
                let path = self.temp.path().join(subpath);
                let parent = path.parent().expect("config path parent");
                fs::create_dir_all(parent).expect("create config parent");
                fs::write(&path, contents).expect("write config");
                path
            }

            fn local(
                &self,
                root_subpath: &str,
            ) -> LocalConfigFile<FileDiscovered> {
                let root = self.temp.path().join(root_subpath);
                let path = root.join(".traces/config.toml");
                if !path.exists() {
                    self.write_config(
                        &format!("{root_subpath}/.traces/config.toml"),
                        "[templates]",
                    );
                }
                LocalConfigFile::<FileDiscovered>::try_new(path)
                    .expect("valid local config")
            }

            fn global(
                &self,
                root_subpath: &str,
            ) -> GlobalConfigFile<FileDiscovered> {
                let root = self.temp.path().join(root_subpath);
                let path = root.join("config.toml");
                if !path.exists() {
                    self.write_config(
                        &format!("{root_subpath}/config.toml"),
                        "[templates]",
                    );
                }
                GlobalConfigFile::<FileDiscovered>::try_new(path)
                    .expect("valid global config")
            }

            fn trust(&self, local: &LocalConfigFile<FileDiscovered>) {
                self.service()
                    .trust(&TrustRequest::from(local))
                    .expect("trust local");
            }
        }

        fn build(
            fixture: &Fixture,
            local: LocalConfigFile<FileDiscovered>,
            global: Option<GlobalConfigFile<FileDiscovered>>,
        ) -> Result<Config, ConfigError> {
            fixture.trust(&local);
            let anchor = local.root().to_path_buf();
            let outcome = DiscoveryOutcome::with_kind(
                DiscoveryScope::Full,
                DiscoveryAnchor::Directory(anchor),
                vec![local],
                global.into_iter().collect(),
            );
            fixture.service().build(outcome)
        }

        mod input {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn rejects_full_discovery_without_local() {
                let fixture = Fixture::new();
                let anchor = fixture.temp.path().join("project");
                let outcome = DiscoveryOutcome::with_kind(
                    DiscoveryScope::Full,
                    DiscoveryAnchor::Directory(anchor.clone()),
                    Vec::new(), // Empty locals
                    Vec::new(),
                );

                let error = ConfigBuilderInput::try_from(outcome)
                    .expect_err("missing locals");

                assert!(matches!(
                    error,
                    ConfigError::Discovery(ConfigDiscoveryError::NotFound {
                        cwd
                    }) if cwd == anchor
                ));
            }

            #[test]
            fn rejects_full_discovery_without_anchor_local() {
                let fixture = Fixture::new();
                let local = fixture.local("project");
                let anchor = fixture.temp.path().join("other");
                let outcome = DiscoveryOutcome::with_kind(
                    DiscoveryScope::Full,
                    DiscoveryAnchor::Directory(anchor.clone()),
                    vec![local],
                    Vec::new(),
                );

                let error = ConfigBuilderInput::try_from(outcome)
                    .expect_err("missing anchor local");

                assert!(matches!(
                    error,
                    ConfigError::Discovery(ConfigDiscoveryError::NotFound {
                        cwd
                    }) if cwd == anchor
                ));
            }

            #[test]
            fn selects_nearest_local_for_full_discovery() {
                let fixture = Fixture::new();
                let parent = fixture.local("parent");
                let child = fixture.local("parent/child");
                let anchor = fixture.temp.path().join("parent/child/notes");

                let outcome = DiscoveryOutcome::with_kind(
                    DiscoveryScope::Full,
                    DiscoveryAnchor::Directory(anchor),
                    vec![parent, child.clone()],
                    Vec::new(),
                );

                let input = ConfigBuilderInput::try_from(outcome)
                    .expect("select builder input");

                assert_eq!(input.local.root(), child.root());
            }

            #[test]
            fn selects_first_discovered_global() {
                let fixture = Fixture::new();
                let local = fixture.local("project");
                let global1 = fixture.global("global1");
                let global2 = fixture.global("global2");

                let outcome = DiscoveryOutcome::with_kind(
                    DiscoveryScope::Full,
                    DiscoveryAnchor::Directory(local.root().to_path_buf()),
                    vec![local],
                    vec![global1.clone(), global2],
                );

                // Act
                let result = ConfigBuilderInput::try_from(outcome);

                // Assert
                let input = result.expect("select builder input");
                let global = input.global.expect("expected global");
                assert_eq!(global.path(), global1.path());
            }
        }

        mod merge {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn extracts_local_output_dir() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[templates]\noutput_dir = \"local_out\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(
                    config.output_dir(),
                    fixture.temp.path().join("project/local_out")
                );
            }

            #[test]
            fn leaves_global_template_dir_empty_when_missing() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[templates]\noutput_dir = \"local_out\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(config.global_template_dir(), None);
            }

            #[test]
            fn prioritizes_local_output_dir() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[templates]\noutput_dir = \"local_out\"",
                );
                let global_path = fixture.write_config(
                    "global/config.toml",
                    "[templates]\noutput_dir = \"global_out\"",
                );

                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();
                let global =
                    GlobalConfigFile::<FileDiscovered>::try_new(global_path)
                        .unwrap();

                // Act
                let config =
                    build(&fixture, local, Some(global)).expect("build");

                // Assert
                assert_eq!(
                    config.output_dir(),
                    fixture.temp.path().join("project/local_out")
                );
            }

            #[test]
            fn uses_local_root_when_output_dir_missing() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[templates]");
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config =
                    build(&fixture, local.clone(), None).expect("build");

                // Assert
                assert_eq!(config.output_dir(), local.root());
            }

            #[test]
            fn returns_error_when_global_parsing_fails() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[templates]");
                let global_path =
                    fixture.write_config("global/config.toml", "[[[BAD TOML");

                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();
                let global =
                    GlobalConfigFile::<FileDiscovered>::try_new(global_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, Some(global));

                assert!(matches!(
                    result,
                    Err(ConfigError::Read(ConfigReadError::Parse { .. }))
                ));
            }

            #[test]
            fn returns_error_when_local_parsing_fails() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[[[BAD TOML");
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                assert!(matches!(
                    result,
                    Err(ConfigError::Read(ConfigReadError::Parse { .. }))
                ));
            }
        }

        mod schemas {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn extracts_local_class_field() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[schemas]\nclass_field = \"kind\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(config.schemas().class_field_name(), "kind");
            }

            #[test]
            fn defaults_class_field_when_unconfigured() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[templates]");
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(config.schemas().class_field_name(), "class");
            }

            #[test]
            fn extracts_local_directory() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[schemas]\ndirectory = \"custom/schemas\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(
                    config.schemas().directory(),
                    fixture.temp.path().join("project/custom/schemas")
                );
            }

            #[test]
            fn defaults_directory_when_unconfigured() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[templates]");
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(
                    config.schemas().directory(),
                    fixture.temp.path().join("project/.traces/schemas")
                );
            }

            #[test]
            fn prioritizes_local_class_field_over_global() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[schemas]\nclass_field = \"local_kind\"",
                );
                let global_path = fixture.write_config(
                    "global/config.toml",
                    "[schemas]\nclass_field = \"global_kind\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();
                let global =
                    GlobalConfigFile::<FileDiscovered>::try_new(global_path)
                        .unwrap();

                // Act
                let config =
                    build(&fixture, local, Some(global)).expect("build");

                // Assert
                assert_eq!(config.schemas().class_field_name(), "local_kind");
            }

            #[test]
            fn rejects_unknown_key() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[schemas]\nbogus = 1",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                // Assert
                assert!(matches!(
                    result,
                    Err(ConfigError::Read(ConfigReadError::Parse { .. }))
                ));
            }

            #[test]
            fn rejects_empty_class_field() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[schemas]\nclass_field = \"   \"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                // Assert
                assert!(matches!(
                    result,
                    Err(ConfigError::Value(
                        ConfigValueError::InvalidFieldKey {
                            table: "schemas",
                            ..
                        }
                    ))
                ));
            }

            #[test]
            fn rejects_a_class_field_containing_a_slash() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[schemas]\nclass_field = \"a/b\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                // Assert
                assert!(matches!(
                    result,
                    Err(ConfigError::Value(
                        ConfigValueError::InvalidFieldKey {
                            table: "schemas",
                            ..
                        }
                    ))
                ));
            }
        }

        mod frontmatter {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn parses_title() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\ntitle = \"Title\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(config.frontmatter().title_name(), "Title");
            }

            #[test]
            fn parses_aliases() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\naliases = \"Aliases\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(config.frontmatter().aliases_name(), "Aliases");
            }

            #[test]
            fn defaults_title_when_unconfigured() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[templates]");
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(config.frontmatter().title_name(), "title");
            }

            #[test]
            fn defaults_aliases_when_unconfigured() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[templates]");
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(config.frontmatter().aliases_name(), "aliases");
            }

            #[test]
            fn parses_tags() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\ntags = \"Categories\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(config.frontmatter().tags_name(), "Categories");
            }

            #[test]
            fn defaults_tags_when_unconfigured() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[templates]");
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                assert_eq!(config.frontmatter().tags_name(), "tags");
            }

            #[test]
            fn parses_date_created() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter.date_created]\nname = \"created\"\nformat = \
                     \"%Y-%m-%d\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                let created = config.frontmatter().date_created();
                assert_eq!(created.name(), "created");
                assert_eq!(created.format(), "%Y-%m-%d");
            }

            #[test]
            fn parses_date_modified() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter.date_modified]\nname = \"modified\"\nformat \
                     = \"%Y-%m-%dT%H:%M:%S\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                let modified = config.frontmatter().date_modified();
                assert_eq!(modified.name(), "modified");
                assert_eq!(modified.format(), "%Y-%m-%dT%H:%M:%S");
            }

            #[test]
            fn defaults_date_created_when_unconfigured() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[templates]");
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                let created = config.frontmatter().date_created();
                assert_eq!(created.name(), "date_created");
                assert_eq!(created.format(), "%Y-%m-%dT%H:%M:%S");
            }

            #[test]
            fn defaults_date_modified_when_unconfigured() {
                let fixture = Fixture::new();
                let local_path = fixture
                    .write_config("project/.traces/config.toml", "[templates]");
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                let modified = config.frontmatter().date_modified();
                assert_eq!(modified.name(), "date_modified");
                assert_eq!(modified.format(), "%Y-%m-%dT%H:%M:%S");
            }

            #[test]
            fn rejects_unknown_key() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\nbogus = 1",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                // Assert
                assert!(matches!(
                    result,
                    Err(ConfigError::Read(ConfigReadError::Parse { .. }))
                ));
            }

            #[test]
            fn rejects_unknown_date_field_key() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter.date_created]\nname = \"created\"\nbogus = 1",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                // Assert
                assert!(matches!(
                    result,
                    Err(ConfigError::Read(ConfigReadError::Parse { .. }))
                ));
            }

            #[test]
            fn defaults_date_created_format_when_only_name_configured() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter.date_created]\nname = \"created_at\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                let created = config.frontmatter().date_created();
                assert_eq!(created.name(), "created_at");
                assert_eq!(created.format(), "%Y-%m-%dT%H:%M:%S");
            }

            #[test]
            fn defaults_date_modified_name_when_only_format_configured() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter.date_modified]\nformat = \"%Y-%m-%d\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let config = build(&fixture, local, None).expect("build");

                // Assert
                let modified = config.frontmatter().date_modified();
                assert_eq!(modified.name(), "date_modified");
                assert_eq!(modified.format(), "%Y-%m-%d");
            }

            #[test]
            fn rejects_empty_date_created_name() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter.date_created]\nname = \"   \"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                // Assert
                assert!(matches!(
                    result,
                    Err(ConfigError::Value(
                        ConfigValueError::InvalidFieldKey {
                            table: "frontmatter",
                            ..
                        }
                    ))
                ));
            }

            #[test]
            fn prioritizes_local_title_over_global() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\ntitle = \"local_title\"",
                );
                let global_path = fixture.write_config(
                    "global/config.toml",
                    "[frontmatter]\ntitle = \"global_title\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();
                let global =
                    GlobalConfigFile::<FileDiscovered>::try_new(global_path)
                        .unwrap();

                // Act
                let config =
                    build(&fixture, local, Some(global)).expect("build");

                // Assert
                assert_eq!(config.frontmatter().title_name(), "local_title");
            }

            #[test]
            fn prioritizes_local_aliases_over_global() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\naliases = \"aka\"",
                );
                let global_path = fixture.write_config(
                    "global/config.toml",
                    "[frontmatter]\naliases = \"aliases\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();
                let global =
                    GlobalConfigFile::<FileDiscovered>::try_new(global_path)
                        .unwrap();

                // Act
                let config =
                    build(&fixture, local, Some(global)).expect("build");

                // Assert
                assert_eq!(config.frontmatter().aliases_name(), "aka");
            }

            #[test]
            fn falls_back_to_global_aliases_when_local_omits_aliases() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\ntitle = \"local_title\"",
                );
                let global_path = fixture.write_config(
                    "global/config.toml",
                    "[frontmatter]\naliases = \"aka\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();
                let global =
                    GlobalConfigFile::<FileDiscovered>::try_new(global_path)
                        .unwrap();

                // Act
                let config =
                    build(&fixture, local, Some(global)).expect("build");

                // Assert
                assert_eq!(config.frontmatter().title_name(), "local_title");
                assert_eq!(config.frontmatter().aliases_name(), "aka");
            }

            #[test]
            fn rejects_empty_title() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\ntitle = \"\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                // Assert
                assert!(matches!(
                    result,
                    Err(ConfigError::Value(
                        ConfigValueError::InvalidFieldKey {
                            table: "frontmatter",
                            ..
                        }
                    ))
                ));
            }

            #[test]
            fn rejects_whitespace_only_aliases() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\naliases = \"   \"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                // Assert
                assert!(matches!(
                    result,
                    Err(ConfigError::Value(
                        ConfigValueError::InvalidFieldKey {
                            table: "frontmatter",
                            ..
                        }
                    ))
                ));
            }

            #[test]
            fn rejects_empty_tags() {
                let fixture = Fixture::new();
                let local_path = fixture.write_config(
                    "project/.traces/config.toml",
                    "[frontmatter]\ntags = \"\"",
                );
                let local =
                    LocalConfigFile::<FileDiscovered>::try_new(local_path)
                        .unwrap();

                // Act
                let result = build(&fixture, local, None);

                // Assert
                assert!(matches!(
                    result,
                    Err(ConfigError::Value(
                        ConfigValueError::InvalidFieldKey {
                            table: "frontmatter",
                            ..
                        }
                    ))
                ));
            }
        }
    }
}
