//! Orchestrates config loading and trust administration.
//!
//! [`ConfigService`] is the single entry point for discovering, building, and
//! trusting config files.
//!
//! # Loading Pipeline
//!
//! 1. Discover local and global TOML candidates from `cwd`.
//! 2. Record the local candidate in the tracking store (best-effort), then
//!    verify trust before parsing its content.
//! 3. Parse TOML into [`super::raw::RawConfig`].
//! 4. Merge global before local so local values win.
//!
//! Granting trust records a config subject in the tracking store first, so
//! every trusted root that has a local config has that config path tracked.
//! [`ConfigService::trust_subjects`] resolves a user-supplied path into trust
//! subjects. Durable state lives in two stores owned by [`ConfigService`]:
//! [`TrustStore`] and [`TrackerStore`].

use std::{
    io,
    path::{Path, PathBuf},
};

use super::{
    ConfigBuilder,
    discovery::{ConfigLocator, DiscoveredConfigs},
    error::{
        ConfigDiscoveryError, ConfigDiscoveryResult, ConfigError,
        ConfigTrustError,
    },
    file::{
        Discovered as FileDiscovered, GlobalConfigFile, LocalConfigFile, Parsed,
    },
    model::Config,
    tracker::TrackerStore,
    trust::{TrustRequest, TrustRequests, TrustScope, TrustStore},
};

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
        let discovered = ConfigLocator::from_env().project(cwd)?;
        self.build(discovered)
    }

    /// Builds a [`Config`] from discovered files.
    ///
    /// Parses and merges global settings before local settings, resolving the
    /// final output directory.
    ///
    /// Trust and tracking notes:
    ///
    /// - The local config's root is checked against the trust store before
    ///   parsing; a global config is never checked.
    /// - Recording the local config in the tracking store is best-effort; a
    ///   write failure does not fail the build.
    ///
    /// # Errors
    ///
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
        discovered: DiscoveredConfigs,
    ) -> Result<Config, ConfigError> {
        let DiscoveredConfigs {
            local,
            global,
        } = discovered;
        let trusted_local = local.into_trusted(&self.tracked, &self.trust)?;
        let root = trusted_local.root().to_path_buf();
        let parsed_local = LocalConfigFile::<Parsed>::try_from(trusted_local)?;
        let parsed_global = match global {
            Some(global) => Some(GlobalConfigFile::<Parsed>::try_from(global)?),
            None => None,
        };
        ConfigBuilder::new(root, parsed_local, parsed_global)
            .build()
            .map_err(Into::into)
    }

    /// Grants trust for `subject`.
    ///
    /// A config subject is recorded in the tracker first, so a trusted root
    /// that has a local config always has that config path tracked. A failure
    /// to record aborts the grant. A root-only subject has nothing to track.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] when the config path cannot be tracked,
    ///   or trust cannot be recorded.
    /// - [`ConfigTrustError::Hash`] when the config file cannot be hashed.
    #[inline]
    pub(crate) fn trust(
        &self,
        subject: &TrustRequest,
    ) -> Result<(), ConfigTrustError> {
        if let Some(config_file) = subject.config_file() {
            self.tracked.record(config_file)?;
        }
        self.trust.trust(subject)
    }

    /// Removes trust for `subject`'s workspace root, leaving the tracker
    /// untouched.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] when the trust store cannot remove the
    ///   root entry or its content-hash companion.
    #[inline]
    pub fn untrust(
        &self,
        subject: &TrustRequest,
    ) -> Result<usize, ConfigTrustError> {
        self.trust.untrust(subject)
    }

    /// Resolves a user-supplied path into trust subjects.
    ///
    /// The canonical path is the workspace identity. A path that does not exist
    /// yet falls back to the given path, so a directory about to be created can
    /// still be named.
    ///
    /// - An explicit `.traces/config.toml` path denotes that config.
    /// - Any other file resolves through its parent directory.
    /// - A directory with [`TrustScope::NearestLocal`] resolves to the nearest
    ///   enclosing local config, or to a root-only request when none exists.
    /// - A directory with [`TrustScope::LocalSubtree`] resolves to every local
    ///   config at or below it, else the enclosing one.
    ///
    /// # Errors
    ///
    /// - [`ConfigDiscoveryError::Inaccessible`] when a path cannot be
    ///   canonicalized for a reason other than not existing, or inspected.
    /// - [`ConfigDiscoveryError::NotFound`] when [`TrustScope::LocalSubtree`]
    ///   finds no local config at, below, or above the directory.
    #[inline]
    pub(crate) fn trust_subjects(
        path: &Path,
        scope: TrustScope,
    ) -> ConfigDiscoveryResult<TrustRequests> {
        let start = canonical_or_given(path)?;
        let locator = ConfigLocator::from_env();
        if let Some(config) = LocalConfigFile::from_config_path(&start) {
            return match scope {
                TrustScope::NearestLocal => {
                    Ok(vec![TrustRequest::from(&config)])
                }
                TrustScope::LocalSubtree => {
                    Ok(config_requests(&locator.local_subtree(config.root())?))
                }
            };
        }
        let dir = if start.is_file() {
            start.parent().unwrap_or(&start)
        } else {
            &start
        };
        match scope {
            TrustScope::NearestLocal => match locator.nearest_local(dir) {
                Ok(config) => Ok(vec![TrustRequest::from(&config)]),
                Err(ConfigDiscoveryError::NotFound {
                    ..
                }) => Ok(vec![TrustRequest::from(dir)]),
                Err(error) => Err(error),
            },
            TrustScope::LocalSubtree => {
                Ok(config_requests(&locator.local_subtree(dir)?))
            }
        }
    }
}

/// Canonicalizes `path`, falling back to it when it does not exist.
///
/// Any other failure (permission denied, symlink loop) is unexpected for a
/// trust operation, where the canonical path is the workspace identity, so it
/// propagates instead of silently trusting a possibly different path.
fn canonical_or_given(path: &Path) -> ConfigDiscoveryResult<PathBuf> {
    match path.canonicalize() {
        Ok(canonical) => Ok(canonical),
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            Ok(path.to_path_buf())
        }
        Err(source) => Err(ConfigDiscoveryError::Inaccessible {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// One config-file trust request per discovered local config.
fn config_requests(files: &[LocalConfigFile<FileDiscovered>]) -> TrustRequests {
    files.iter().map(TrustRequest::from).collect()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use super::*;
    use crate::config::{
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
            LocalConfigFile::from_config_path(config_path)
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

    mod trust {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn granting_a_config_subject_records_it_in_the_tracker() {
            let fixture = Fixture::new();
            let root = fixture.target_dir("project");
            let config_path = Fixture::create_config(&root, "");

            fixture.trust_config(&config_path);

            assert_eq!(
                fixture.service.tracker_store().list().expect("list"),
                vec![config_path.canonicalize().expect("canonicalize")]
            );
        }

        #[test]
        fn granting_a_root_only_subject_tracks_nothing() {
            let fixture = Fixture::new();
            let root = fixture.target_dir("project");

            fixture
                .service
                .trust(&TrustRequest::from(root.as_path()))
                .expect("trust root");

            assert_eq!(
                fixture.service.tracker_store().list().expect("list"),
                Vec::<PathBuf>::new()
            );
        }

        #[test]
        fn a_failing_tracker_aborts_the_grant() {
            let fixture = Fixture::new();
            let root = fixture.target_dir("project");
            let config_path = Fixture::create_config(&root, "");
            fs::write(&fixture.tracked_root, "").expect("block tracking store");
            let request =
                TrustRequest::from(&Fixture::discovered_config(&config_path));

            let error = fixture
                .service
                .trust(&request)
                .expect_err("tracking failure aborts the grant");

            assert!(matches!(error, ConfigTrustError::Tracker(_)));
            assert_eq!(
                fixture
                    .service
                    .trust_store()
                    .trust_status(&request)
                    .expect("status"),
                ConfigTrustStatus::Untrusted
            );
        }

        #[test]
        fn granting_a_missing_config_fails() {
            let fixture = Fixture::new();
            let root = fixture.target_dir("project");
            let request = TrustRequest::Config {
                path: root.join(".traces/config.toml"),
                root,
            };

            let result = fixture.service.trust(&request);

            assert!(result.is_err());
        }

        #[test]
        fn untrust_keeps_the_config_tracked() {
            let fixture = Fixture::new();
            let root = fixture.target_dir("project");
            let config_path = Fixture::create_config(&root, "");
            fixture.trust_config(&config_path);

            fixture
                .service
                .untrust(&TrustRequest::from(root.as_path()))
                .expect("untrust");

            assert_eq!(
                fixture.service.tracker_store().list().expect("list").len(),
                1
            );
        }

        #[test]
        fn a_refused_load_still_tracks_the_config() {
            let fixture = Fixture::new();
            let root = fixture.target_dir("project");
            Fixture::create_config(&root, "");

            let result = fixture.service.load(&root);

            assert!(matches!(
                result,
                Err(ConfigError::Trust(ConfigTrustError::Untrusted { .. }))
            ));
            assert_eq!(
                fixture.service.tracker_store().list().expect("list").len(),
                1
            );
        }
    }

    mod build {
        use pretty_assertions::assert_eq;

        use super::*;

        fn local_candidates(
            fixture: &Fixture,
        ) -> (PathBuf, PathBuf, DiscoveredConfigs) {
            let cwd = fixture.target_dir("project");
            let config_path = Fixture::create_config(&cwd, "");
            let candidates = DiscoveredConfigs {
                local: LocalConfigFile::at(&cwd),
                global: None,
            };
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
            fixture
                .service
                .trust_store()
                .trust(&TrustRequest::from(&Fixture::discovered_config(
                    &config_path,
                )))
                .expect("trust config without tracking it");

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

    mod trust_subjects {
        use pretty_assertions::assert_eq;

        use super::*;

        fn canonical(path: &Path) -> PathBuf {
            path.canonicalize().expect("canonicalize")
        }

        fn config_request(root: &Path, config: &Path) -> TrustRequest {
            TrustRequest::Config {
                root: canonical(root),
                path: canonical(config),
            }
        }

        #[test]
        fn nearest_local_returns_the_directorys_config() {
            let fixture = Fixture::new();
            let project = fixture.target_dir("project");
            let config = Fixture::create_config(&project, "");

            let requests = ConfigService::trust_subjects(
                &project,
                TrustScope::NearestLocal,
            )
            .expect("resolve");

            assert_eq!(requests, vec![config_request(&project, &config)]);
        }

        #[test]
        fn nearest_local_without_a_config_falls_back_to_the_root() {
            let fixture = Fixture::new();
            let project = fixture.target_dir("project");

            let requests = ConfigService::trust_subjects(
                &project,
                TrustScope::NearestLocal,
            )
            .expect("resolve");

            assert_eq!(requests, vec![TrustRequest::Root(canonical(&project))]);
        }

        #[test]
        fn an_explicit_config_path_denotes_that_config() {
            let fixture = Fixture::new();
            let project = fixture.target_dir("project");
            let config = Fixture::create_config(&project, "");

            let requests = ConfigService::trust_subjects(
                &config,
                TrustScope::NearestLocal,
            )
            .expect("resolve");

            assert_eq!(requests, vec![config_request(&project, &config)]);
        }

        #[test]
        fn a_non_config_file_resolves_through_its_parent() {
            let fixture = Fixture::new();
            let project = fixture.target_dir("project");
            let config = Fixture::create_config(&project, "");
            let note = project.join("notes/todo.md");
            fs::create_dir_all(note.parent().expect("note parent"))
                .expect("create notes");
            fs::write(&note, "").expect("write note");

            let requests =
                ConfigService::trust_subjects(&note, TrustScope::NearestLocal)
                    .expect("resolve");

            assert_eq!(requests, vec![config_request(&project, &config)]);
        }

        #[test]
        fn local_subtree_returns_every_config_beneath_the_directory() {
            let fixture = Fixture::new();
            let parent = fixture.target_dir("parent");
            let child = fixture.target_dir("parent/child");
            let parent_config = Fixture::create_config(&parent, "");
            let child_config = Fixture::create_config(&child, "");

            let requests = ConfigService::trust_subjects(
                &parent,
                TrustScope::LocalSubtree,
            )
            .expect("resolve");

            assert_eq!(requests, vec![
                config_request(&parent, &parent_config),
                config_request(&child, &child_config),
            ]);
        }

        #[test]
        fn local_subtree_without_any_config_is_not_found() {
            let fixture = Fixture::new();
            let project = fixture.target_dir("project");

            let result = ConfigService::trust_subjects(
                &project,
                TrustScope::LocalSubtree,
            );

            assert!(matches!(
                result,
                Err(ConfigDiscoveryError::NotFound { .. })
            ));
        }
    }
}
