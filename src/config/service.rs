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
    error::{
        ConfigDiscoveryError, ConfigDiscoveryResult, ConfigError,
        ConfigTrustError,
    },
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
            return Err(ConfigDiscoveryError::NotFound {
                cwd: anchor_path,
            }
            .into());
        }

        let local = discovered_locals
            .into_iter()
            .filter(|file| anchor_path.starts_with(file.root()))
            .max_by_key(|file| file.root().components().count())
            .ok_or(ConfigDiscoveryError::NotFound {
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
    /// - [`ConfigDiscoveryError::NotFound`] when no local config exists in any
    ///   ancestor directory.
    /// - [`ConfigDiscoveryError::Inaccessible`] when a path cannot be accessed.
    #[inline]
    fn discover(cwd: &Path) -> ConfigDiscoveryResult<DiscoveryOutcome> {
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
    /// - [`ConfigDiscoveryError::NotFound`] when discovery output selected no
    ///   local config, or none contains the discovery anchor.
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
    /// - [`ConfigTrustError::Tracker`] when the trust store cannot remove the
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
        error::{ConfigDiscoveryError, ConfigValueError},
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
            fs::create_dir_all(config_path.parent().expect("config parent"))
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
            fs::create_dir_all(&cwd).expect("create cwd");

            let config_path = Fixture::create_config(
                &root,
                "[templates]\ndirectory = \".traces/templates\"\noutput_dir = \
                 \"notes\"",
            );
            fixture.trust_config(&config_path);

            // Act
            let result = fixture.service.load(&cwd);

            // Assert
            let config = result.expect("load trusted local config");
            assert_eq!(config.root(), root.as_path());
            assert_eq!(config.output_dir(), root.join("notes"));
        }

        #[test]
        fn fails_to_load_when_tag_filter_is_invalid() {
            // Arrange
            let fixture = Fixture::new();
            let root = fixture.target_dir("project");
            let cwd = root.join("notes");
            fs::create_dir_all(&cwd).expect("create cwd");

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
            let (cwd, config_path, candidates) = local_candidates(&fixture);
            fixture.trust_config(&config_path);

            // Act
            let result = fixture.service.build(candidates);

            // Assert
            let config = result.expect("build trusted config");
            assert_eq!(config.root(), cwd.as_path());
            let tracked = fixture
                .service
                .tracker_store()
                .list()
                .expect("list tracked configs");
            assert_eq!(tracked, vec![
                config_path.canonicalize().expect("canonical config path")
            ]);
        }

        #[test]
        fn succeeds_even_when_tracking_store_write_fails() {
            // Arrange
            let fixture = Fixture::new();
            let (cwd, config_path, candidates) = local_candidates(&fixture);
            fixture.trust_config(&config_path);

            // Occupy the tracked-store path with a file so directory creation
            // fails
            fs::write(&fixture.tracked_root, "").expect("block tracking store");

            // Act
            let result = fixture.service.build(candidates);

            // Assert
            let config = result.expect("build despite tracking write failure");
            assert_eq!(config.root(), cwd.as_path());
        }

        #[test]
        fn fails_with_untrusted_when_trust_verification_halts() {
            // Arrange
            let fixture = Fixture::new();
            let (_cwd, config_path, candidates) = local_candidates(&fixture);
            fixture.trust_config(&config_path);
            fs::write(&config_path, "directory = \"changed\"")
                .expect("edit config after trusting");

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
    }

    mod builder_input {
        use pretty_assertions::assert_eq;

        use super::*;

        fn local(
            temp: &tempfile::TempDir,
            name: &str,
        ) -> LocalConfigFile<FileDiscovered> {
            LocalConfigFile::<FileDiscovered>::try_new(
                temp.path().join(name).join(".traces/config.toml"),
            )
            .expect("valid local candidate")
        }

        fn global(
            temp: &tempfile::TempDir,
            name: &str,
        ) -> GlobalConfigFile<FileDiscovered> {
            GlobalConfigFile::<FileDiscovered>::try_new(
                temp.path().join(name).join("config.toml"),
            )
            .expect("valid global candidate")
        }

        #[test]
        fn rejects_full_discovery_without_local() {
            let fixture = tempfile::tempdir().expect("temp dir");
            let anchor = fixture.path().join("project");
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
            let fixture = tempfile::tempdir().expect("temp dir");
            let local = local(&fixture, "project");
            let anchor = fixture.path().join("other");
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
            let fixture = tempfile::tempdir().expect("temp dir");
            let parent = local(&fixture, "parent");
            let child = local(&fixture, "parent/child");
            let anchor = fixture.path().join("parent/child/notes");

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
            let fixture = tempfile::tempdir().expect("temp dir");
            let local = local(&fixture, "project");
            let global1 = global(&fixture, "global1");
            let global2 = global(&fixture, "global2");

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
}
