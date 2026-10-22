//! Orchestrates Config File loading and trust administration.
//!
//! [`ConfigService`] discovers Config Files, verifies Trust Status before
//! reading local content, and builds the effective Configuration Scope. It
//! records each local Tracked Config Path in the [`TrackerStore`] on a
//! best-effort basis. Granting trust records that path before writing to the
//! [`TrustStore`], so every trusted root with a local Config File has its path
//! tracked. [`ConfigService::trust_subjects`] resolves user-supplied paths into
//! trust requests.

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

/// Coordinates Config File loading and trust administration.
///
/// Loading uses discovery, read, trust and value stages. Trust Verification
/// precedes parsing local content; global Config Files need no trust check. The
/// Tracker Store records Tracked Config Paths, while the Trust Store records
/// workspace roots and Companion Hashes.
#[derive(Clone, Debug)]
pub struct ConfigService {
    tracked: TrackerStore,
    trust: TrustStore,
}

impl ConfigService {
    /// Returns the trust-administration store.
    #[inline]
    #[must_use]
    pub(crate) const fn trust_store(&self) -> &TrustStore {
        &self.trust
    }

    /// Returns the Tracker Store.
    #[inline]
    #[must_use]
    pub(crate) const fn tracker_store(&self) -> &TrackerStore {
        &self.tracked
    }

    /// Discovers and builds the effective Configuration Scope from `cwd`.
    ///
    /// # Errors
    ///
    /// - [`Discovery`] if discovery finds no local Config File or cannot
    ///   inspect a path.
    /// - [`Read`] if reading or parsing a selected Config File fails.
    /// - [`Trust`] if Trust Verification rejects the local Config File or
    ///   reading the Trust Store or Config File for verification fails.
    /// - [`Value`] if validation rejects a parsed value.
    ///
    /// [`Discovery`]: ConfigError::Discovery
    /// [`Read`]: ConfigError::Read
    /// [`Trust`]: ConfigError::Trust
    /// [`Value`]: ConfigError::Value
    #[inline]
    pub(crate) fn load(&self, cwd: &Path) -> Result<Config, ConfigError> {
        let discovered = ConfigLocator::from_env().project(cwd)?;
        self.build(discovered)
    }

    /// Builds a [`Config`] from discovered Config Files.
    ///
    /// Merges global values before local values. Records the local Tracked
    /// Config Path on a best-effort basis, then performs Trust Verification
    /// before parsing local content.
    ///
    /// # Errors
    ///
    /// - [`Trust`] if Trust Verification rejects the local Config File or
    ///   reading the Trust Store or Config File for verification fails.
    /// - [`Read`] if reading or parsing a selected Config File fails.
    /// - [`Value`] if validation rejects a parsed value.
    ///
    /// [`Trust`]: ConfigError::Trust
    /// [`Read`]: ConfigError::Read
    /// [`Value`]: ConfigError::Value
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

    /// Grants trust for a [`TrustRequest`].
    ///
    /// Records a local Tracked Config Path first, aborting the grant if that
    /// fails. A root-only request has no path to track.
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if recording the path or trust fails.
    /// - [`Hash`] if reading the Config File for hashing fails.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    /// [`Hash`]: ConfigTrustError::Hash
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

    /// Revokes trust for a [`TrustRequest`] without untracking its path.
    ///
    /// # Errors
    ///
    /// - `ConfigTrustError::Tracker` if removing the root or Companion Hash
    ///   fails.
    #[inline]
    pub fn untrust(
        &self,
        subject: &TrustRequest,
    ) -> Result<usize, ConfigTrustError> {
        self.trust.untrust(subject)
    }

    /// Resolves a user-supplied path into trust requests.
    ///
    /// Canonical paths identify workspaces; missing paths retain their given
    /// spelling. A Config File path identifies that file, while another file
    /// resolves through its parent. [`TrustScope::NearestLocal`] chooses the
    /// nearest enclosing local Config File or a root-only request;
    /// [`TrustScope::LocalSubtree`] collects Config Files at or below the
    /// directory, or the enclosing Config File.
    ///
    /// # Errors
    ///
    /// - [`Inaccessible`] if inspecting a path or canonicalizing it fails for a
    ///   reason other than its absence.
    /// - [`NotFound`] if [`TrustScope::LocalSubtree`] finds no local Config
    ///   File at, below, or above the directory.
    ///
    /// [`Inaccessible`]: ConfigDiscoveryError::Inaccessible
    /// [`NotFound`]: ConfigDiscoveryError::NotFound
    #[inline]
    pub(crate) fn trust_subjects(
        path: &Path,
        scope: TrustScope,
    ) -> ConfigDiscoveryResult<TrustRequests> {
        resolve_trust_subjects(&ConfigLocator::from_env(), path, scope)
    }

    /// Creates a service backed by explicit Tracker Store and Trust Store roots
    /// instead of the OS state directories.
    ///
    /// Tests use isolated stores; production code uses
    /// [`ConfigService::default`] for the OS-backed stores.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    #[inline]
    pub fn for_test<P: Into<PathBuf>, Q: Into<PathBuf>>(
        tracked_root: P,
        trusted_root: Q,
    ) -> Self {
        Self {
            tracked: TrackerStore::for_test(tracked_root),
            trust: TrustStore::for_test(trusted_root),
        }
    }
}

impl Default for ConfigService {
    /// Creates a service backed by the OS Tracker Store and Trust Store.
    #[inline]
    fn default() -> Self {
        Self {
            tracked: TrackerStore::default(),
            trust: TrustStore::default(),
        }
    }
}

/// Resolves a path with a [`ConfigLocator`] using
/// [`ConfigService::trust_subjects`] rules.
///
/// # Errors
///
/// - [`Inaccessible`] if inspecting or canonicalizing a path fails.
/// - [`NotFound`] if subtree discovery finds no local Config File.
///
/// [`Inaccessible`]: ConfigDiscoveryError::Inaccessible
/// [`NotFound`]: ConfigDiscoveryError::NotFound
fn resolve_trust_subjects(
    locator: &ConfigLocator,
    path: &Path,
    scope: TrustScope,
) -> ConfigDiscoveryResult<TrustRequests> {
    let start = canonical_or_given(path)?;
    if let Some(config) = LocalConfigFile::from_config_path(&start) {
        return match scope {
            TrustScope::NearestLocal => Ok(vec![TrustRequest::from(&config)]),
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

/// Canonicalizes a path while retaining its spelling if it does not exist.
///
/// Other failures propagate rather than trusting a possibly different path.
///
/// # Errors
///
/// - [`Inaccessible`] if canonicalization fails for a reason other than the
///   path not existing.
///
/// [`Inaccessible`]: ConfigDiscoveryError::Inaccessible
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

/// Collects one [`TrustRequest`] per discovered local Config File.
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
            let service = ConfigService::for_test(
                tracked_root.clone(),
                trusted_root.clone(),
            );
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

        /// Confines discovery to fixture directories and Config Files.
        fn locator(&self) -> ConfigLocator {
            ConfigLocator::new(
                vec![self.temp.path().to_path_buf()],
                self.temp.path().join("global-home"),
            )
        }

        /// Loads through [`Self::locator`] without reading OS state.
        ///
        /// # Errors
        ///
        /// - [`Discovery`] if discovery finds no local Config File or cannot
        ///   inspect a path.
        /// - [`Trust`] if Trust Verification rejects the local Config File or
        ///   reading the Trust Store or Config File for verification fails.
        /// - [`Read`] if reading or parsing a Config File fails.
        /// - [`Value`] if validation rejects a parsed value.
        ///
        /// [`Discovery`]: ConfigError::Discovery
        /// [`Trust`]: ConfigError::Trust
        /// [`Read`]: ConfigError::Read
        /// [`Value`]: ConfigError::Value
        fn load(&self, cwd: &Path) -> Result<Config, ConfigError> {
            self.service.build(self.locator().project(cwd)?)
        }
    }

    mod load {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_discovery_error_when_no_config_found() {
            let fixture = Fixture::new();
            let cwd = fixture.target_dir("project/notes/daily");

            let result = fixture.load(&cwd);

            assert!(matches!(
                result,
                Err(ConfigError::Discovery(
                    ConfigDiscoveryError::NotFound { .. }
                ))
            ));
        }

        #[test]
        fn discovers_and_builds_trusted_local_config() {
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

            let result = fixture.load(&cwd);

            let config = result.expect("load trusted local config");
            assert_eq!(config.root(), root.as_path());
            assert_eq!(config.output_dir(), root.join("notes"));
        }

        #[test]
        fn fails_to_load_when_tag_filter_is_invalid() {
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

            let result = fixture.load(&cwd);

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
                local: LocalConfigFile::from_root(&cwd),
                global: None,
            };
            (cwd, config_path, candidates)
        }

        #[test]
        fn records_candidate_in_tracking_store() {
            let fixture = Fixture::new();
            let (cwd, config_path, candidates) = local_candidates(&fixture);
            fixture.trust_config(&config_path);

            let result = fixture.service.build(candidates);

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
            let fixture = Fixture::new();
            let (cwd, config_path, candidates) = local_candidates(&fixture);
            fixture
                .service
                .trust_store()
                .trust(&TrustRequest::from(&Fixture::discovered_config(
                    &config_path,
                )))
                .expect("trust config without tracking it");

            // Block Tracker Store directory creation to exercise
            // best-effort tracking.
            fs::write(&fixture.tracked_root, "").expect("block tracking store");

            let result = fixture.service.build(candidates);

            let config = result.expect("build despite tracking write failure");
            assert_eq!(config.root(), cwd.as_path());
        }

        #[test]
        fn fails_with_untrusted_when_trust_verification_halts() {
            let fixture = Fixture::new();
            let (_cwd, config_path, candidates) = local_candidates(&fixture);
            fixture.trust_config(&config_path);
            fs::write(&config_path, "directory = \"changed\"")
                .expect("edit config after trusting");

            let result = fixture.service.build(candidates);

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

            let requests = resolve_trust_subjects(
                &fixture.locator(),
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

            let requests = resolve_trust_subjects(
                &fixture.locator(),
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

            let requests = resolve_trust_subjects(
                &fixture.locator(),
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

            let requests = resolve_trust_subjects(
                &fixture.locator(),
                &note,
                TrustScope::NearestLocal,
            )
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

            let requests = resolve_trust_subjects(
                &fixture.locator(),
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
        fn a_path_that_does_not_exist_yet_is_named_as_given() {
            let fixture = Fixture::new();
            let missing = fixture.temp.path().join("not/created/yet");

            let requests = resolve_trust_subjects(
                &fixture.locator(),
                &missing,
                TrustScope::NearestLocal,
            )
            .expect("resolve");

            assert_eq!(requests, vec![TrustRequest::Root(missing)]);
        }

        #[test]
        fn a_missing_config_path_scans_its_project_root_for_the_subtree() {
            let fixture = Fixture::new();
            let project = fixture.target_dir("project");
            let child = fixture.target_dir("project/child");
            let child_config = Fixture::create_config(&child, "");
            let missing = project.join(".traces/config.toml");

            let requests = resolve_trust_subjects(
                &fixture.locator(),
                &missing,
                TrustScope::LocalSubtree,
            )
            .expect("resolve");

            // Retain the given root spelling when the missing path cannot
            // be canonicalized.
            let expected =
                TrustRequest::from(&Fixture::discovered_config(&child_config));
            assert_eq!(requests, vec![expected]);
        }
    }
}
