//! Trust request targets, trust status models, and the persistent trust store.
//!
//! # Types
//!
//! - [`TrustRequest`] names either a workspace root or one local config file.
//! - [`ConfigTrustStatus`] describes root trust plus config-baseline states for
//!   detecting stale local config files.
//! - [`TrustStore`] owns cryptographic trust verification and the content-hash
//!   companion file that backs it.

use std::{
    fmt::{self, Display, Formatter},
    fs,
    path::{Path, PathBuf},
};

use super::error::ConfigTrustError;
use crate::{Blake3Digest, CleanMode, FilePathTracker, dirs, hash::HashError};

/// File-name suffix of the content-hash companion beside a trusted root.
const COMPANION_SUFFIX: &str = ".hash";

/// Targets a workspace root or config file for a trust operation.
///
/// Create via `TrustRequest::from` with a `&Path` for root-only trust, or with
/// a `&LocalConfigFile<Discovered>` for config-file trust that also records a
/// content baseline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrustRequest {
    /// Trust a workspace root without binding to a config file.
    Root(PathBuf),
    /// Trust a specific config file and its root.
    Config {
        /// The workspace root.
        root: PathBuf,
        /// The config file path.
        path: PathBuf,
    },
}

impl TrustRequest {
    /// Returns the workspace root this request refers to.
    #[inline]
    #[must_use]
    pub(crate) fn root_path(&self) -> &Path {
        match self {
            Self::Root(root)
            | Self::Config {
                root,
                ..
            } => root,
        }
    }

    /// Returns the config file path, when this request carries one.
    #[inline]
    #[must_use]
    pub(crate) fn config_file(&self) -> Option<&Path> {
        match self {
            Self::Root(_) => None,
            Self::Config {
                path,
                ..
            } => Some(path),
        }
    }
}

impl From<&Path> for TrustRequest {
    #[inline]
    fn from(root: &Path) -> Self {
        Self::Root(root.to_path_buf())
    }
}

/// Trust requests resolved from a single discovery operation.
pub(crate) type TrustRequests = Vec<TrustRequest>;

/// Scope for resolving trust subjects from a user-supplied path.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum TrustScope {
    /// Resolve a single trust target.
    NearestLocal,
    /// Resolve descendant trust targets.
    LocalSubtree,
}

/// Trust state for a config file relative to its workspace root.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ConfigTrustStatus {
    /// The workspace root is trusted and, when a baseline hash exists, the
    /// config file's content still matches it.
    Trusted,
    /// The workspace root is not trusted.
    Untrusted,
    /// The workspace root is trusted, but no content-hash baseline was ever
    /// recorded for this config file.
    MissingBaseline,
    /// The workspace root is trusted and a baseline hash exists, but the config
    /// file's current content no longer matches it.
    Stale,
}

impl Display for ConfigTrustStatus {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Trusted => "trusted",
            Self::Untrusted => "untrusted",
            Self::MissingBaseline => "missing baseline",
            Self::Stale => "stale",
        })
    }
}

/// Result of checking whether a local config file may be parsed.
///
/// Only [`Self::Trusted`] carries already-read content. Keeping content and
/// status together prevents callers from pairing trusted status with missing
/// content.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ConfigTrustCheck {
    /// The workspace root is not trusted.
    Untrusted,
    /// The workspace root is trusted and a baseline hash exists, but the config
    /// file's current content no longer matches it.
    Stale,
    /// The workspace root is trusted, but no content-hash baseline was ever
    /// recorded for this config file.
    MissingBaseline,
    /// The workspace root is trusted and the config file's content matches its
    /// baseline hash.
    ///
    /// Carries the content read while verifying it, so parsing can reuse the
    /// same buffer without a second filesystem read.
    Trusted(String),
}

impl ConfigTrustCheck {
    /// Returns the status-only view, discarding any trusted content.
    #[inline]
    #[must_use]
    pub(crate) const fn status(&self) -> ConfigTrustStatus {
        match self {
            Self::Untrusted => ConfigTrustStatus::Untrusted,
            Self::Stale => ConfigTrustStatus::Stale,
            Self::MissingBaseline => ConfigTrustStatus::MissingBaseline,
            Self::Trusted(_) => ConfigTrustStatus::Trusted,
        }
    }
}

/// Reads a config file as UTF-8 text for baseline hashing.
///
/// Trust recording and trust checking both go through this function so a
/// baseline is only ever recorded for content that a later check can read.
///
/// # Errors
///
/// - [`ConfigTrustError::Hash`] when the file cannot be read, or is not valid
///   UTF-8.
fn read_config(path: &Path) -> Result<String, ConfigTrustError> {
    fs::read_to_string(path).map_err(|source| {
        ConfigTrustError::Hash(HashError {
            path: path.to_path_buf(),
            source,
        })
    })
}

/// Persistent store of trusted workspace roots and their config content
/// baselines.
///
/// Owns cryptographic trust verification: granting, revoking, and checking
/// trust for a workspace root, plus the content-hash companion file that
/// detects stale local config.
#[derive(Clone, Debug)]
pub(crate) struct TrustStore {
    store: FilePathTracker,
}

impl TrustStore {
    /// Creates a store backed by the OS-correct trust-store directory.
    #[must_use]
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            store: FilePathTracker::from(dirs::TRUSTED_CONFIGS.clone()),
        }
    }

    /// Creates a store backed by an explicit root. Test-only.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    pub(crate) fn at(root: PathBuf) -> Self {
        Self {
            store: FilePathTracker::at(root),
        }
    }

    /// Grants trust for a workspace root.
    ///
    /// When `subject` carries a config file, hashes and records the content
    /// baseline before granting workspace trust, so a hashing or write failure
    /// never leaves the workspace trusted without a baseline.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] when trust cannot be recorded.
    /// - [`ConfigTrustError::Hash`] when the config file cannot be read, or is
    ///   not valid UTF-8.
    #[inline]
    pub(crate) fn trust(
        &self,
        subject: &TrustRequest,
    ) -> Result<(), ConfigTrustError> {
        if let Some(config_file) = subject.config_file() {
            let content = read_config(config_file)?;
            let digest = Blake3Digest::from(content.as_str());
            self.store.write_companion(
                subject.root_path(),
                COMPANION_SUFFIX,
                digest.to_string(),
            )?;
        }
        self.store.record(subject.root_path())?;
        Ok(())
    }

    /// Returns the trust status for `subject`.
    ///
    /// For config-file subjects, checks the baseline hash. For root-only
    /// subjects, checks only workspace presence.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] when the trust store cannot be read.
    /// - [`ConfigTrustError::Hash`] when the config file cannot be hashed.
    #[inline]
    pub(crate) fn trust_status(
        &self,
        subject: &TrustRequest,
    ) -> Result<ConfigTrustStatus, ConfigTrustError> {
        match subject.config_file() {
            Some(config_file) => self
                .config_file_trust_check(subject.root_path(), config_file)
                .map(|check| check.status()),
            None if self.store.contains(subject.root_path())? => {
                Ok(ConfigTrustStatus::Trusted)
            }
            None => Ok(ConfigTrustStatus::Untrusted),
        }
    }

    /// Checks config-file trust and returns content only when trusted.
    ///
    /// Takes `root` and `config_path` directly because a root-only
    /// [`TrustRequest`] has no config path, while trusted parsing must carry
    /// content.
    ///
    /// Reads the config file once and hashes the buffer in memory, returning
    /// the same content for parsing to avoid a second filesystem read.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] when the trust store cannot be read.
    /// - [`ConfigTrustError::Hash`] when the config file cannot be read, or is
    ///   not valid UTF-8.
    pub(crate) fn config_file_trust_check(
        &self,
        root: &Path,
        config_path: &Path,
    ) -> Result<ConfigTrustCheck, ConfigTrustError> {
        if !self.store.contains(root)? {
            return Ok(ConfigTrustCheck::Untrusted);
        }
        let Some(recorded) =
            self.store.read_companion(root, COMPANION_SUFFIX)?
        else {
            return Ok(ConfigTrustCheck::MissingBaseline);
        };
        let content = read_config(config_path)?;
        let current = Blake3Digest::from(content.as_str());
        if recorded.trim() == current.to_hex().as_ref() {
            Ok(ConfigTrustCheck::Trusted(content))
        } else {
            Ok(ConfigTrustCheck::Stale)
        }
    }

    /// Removes trust for `subject`'s workspace root.
    ///
    /// Removes the workspace and its config-baseline companion. Returns the
    /// number of root entries removed.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] if the trust store cannot remove the
    ///   root entry or its content-hash companion.
    #[inline]
    pub(crate) fn untrust(
        &self,
        subject: &TrustRequest,
    ) -> Result<usize, ConfigTrustError> {
        self.store
            .remove_with_companions(subject.root_path(), &[COMPANION_SUFFIX])
            .map_err(Into::into)
    }

    /// Lists the canonical paths of all currently trusted roots.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] if reading the trust store fails.
    #[inline]
    pub(crate) fn list(&self) -> Result<Vec<PathBuf>, ConfigTrustError> {
        self.store.list_all(&[COMPANION_SUFFIX]).map_err(Into::into)
    }

    /// Removes dangling trust entries and their content-hash companions.
    ///
    /// Returns the number of root entries removed.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] if reading the trust store fails,
    ///   removing a stale root entry fails, or removing an existing
    ///   content-hash companion fails.
    #[inline]
    pub(crate) fn clean(&self) -> Result<usize, ConfigTrustError> {
        self.store
            .clean(CleanMode::WithCompanions(&[COMPANION_SUFFIX]))
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use fixtures::Fixture;

    use super::*;
    use crate::config::file::LocalConfigFile;

    mod fixtures {
        use super::*;

        /// Isolated trust store plus a project directory beneath one temp dir.
        pub(super) struct Fixture {
            temp: tempfile::TempDir,
            pub(super) store: TrustStore,
        }

        impl Fixture {
            pub(super) fn new() -> Self {
                let temp = tempfile::tempdir().expect("create temp dir");
                let store = TrustStore::at(temp.path().join("trust-store"));
                Self {
                    temp,
                    store,
                }
            }

            pub(super) fn project(&self, name: &str) -> PathBuf {
                let root = self.temp.path().join(name);
                fs::create_dir_all(&root).expect("create project dir");
                root
            }

            pub(super) fn write_config(root: &Path, contents: &str) -> PathBuf {
                let path = root.join(".traces/config.toml");
                fs::create_dir_all(path.parent().expect("config parent"))
                    .expect("create config dir");
                fs::write(&path, contents).expect("write config");
                path
            }

            pub(super) fn config_request(path: &Path) -> TrustRequest {
                let config = LocalConfigFile::from_config_path(path)
                    .expect("valid local config path");
                TrustRequest::from(&config)
            }
        }
    }

    mod formatting {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn missing_baseline_displays_distinctly_from_stale() {
            assert_eq!(
                ConfigTrustStatus::MissingBaseline.to_string(),
                "missing baseline"
            );
            assert_eq!(ConfigTrustStatus::Stale.to_string(), "stale");
        }
    }

    mod trust {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn records_workspace_trust() {
            let fixture = Fixture::new();
            let subject =
                TrustRequest::from(fixture.project("project").as_path());

            fixture.store.trust(&subject).expect("trust root");

            assert_eq!(
                fixture.store.trust_status(&subject).expect("status"),
                ConfigTrustStatus::Trusted
            );
        }

        #[test]
        fn records_config_trust_when_content_is_readable() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            let subject =
                Fixture::config_request(&Fixture::write_config(&root, "a = 1"));

            fixture.store.trust(&subject).expect("trust config");

            assert_eq!(
                fixture.store.trust_status(&subject).expect("status"),
                ConfigTrustStatus::Trusted
            );
        }

        #[test]
        fn fails_with_hash_error_when_config_is_unreadable() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            let config_path = Fixture::write_config(&root, "a = 1");
            let subject = Fixture::config_request(&config_path);
            fs::remove_file(&config_path).expect("remove config");

            let result = fixture.store.trust(&subject);

            assert!(matches!(result, Err(ConfigTrustError::Hash(_))));
        }

        #[test]
        fn leaves_workspace_untrusted_when_config_is_unreadable() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            let config_path = Fixture::write_config(&root, "a = 1");
            let subject = Fixture::config_request(&config_path);
            fs::remove_file(&config_path).expect("remove config");
            fixture.store.trust(&subject).expect_err("hash failure");

            let status =
                fixture.store.trust_status(&TrustRequest::from(root.as_path()));

            assert_eq!(status.expect("status"), ConfigTrustStatus::Untrusted);
        }

        #[test]
        fn fails_with_hash_error_when_config_is_not_utf8() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            let config_path = Fixture::write_config(&root, "");
            fs::write(&config_path, [0xff_u8, 0xfe]).expect("write bytes");
            let subject = Fixture::config_request(&config_path);

            let result = fixture.store.trust(&subject);

            assert!(matches!(result, Err(ConfigTrustError::Hash(_))));
        }
    }

    mod trust_status {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_untrusted_for_unknown_workspace() {
            let fixture = Fixture::new();
            let subject =
                TrustRequest::from(fixture.project("project").as_path());

            let result = fixture.store.trust_status(&subject);

            assert_eq!(result.expect("status"), ConfigTrustStatus::Untrusted);
        }

        #[test]
        fn returns_untrusted_for_unknown_config() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            let subject =
                Fixture::config_request(&Fixture::write_config(&root, "a = 1"));

            let result = fixture.store.trust_status(&subject);

            assert_eq!(result.expect("status"), ConfigTrustStatus::Untrusted);
        }

        #[test]
        fn returns_stale_when_config_content_changes_after_trust() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            let config_path = Fixture::write_config(&root, "a = 1");
            let subject = Fixture::config_request(&config_path);
            fixture.store.trust(&subject).expect("trust config");
            fs::write(&config_path, "a = 2").expect("edit config");

            let result = fixture.store.trust_status(&subject);

            assert_eq!(result.expect("status"), ConfigTrustStatus::Stale);
        }

        #[test]
        fn returns_missing_baseline_when_only_the_workspace_was_trusted() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            let subject =
                Fixture::config_request(&Fixture::write_config(&root, "a = 1"));
            fixture
                .store
                .trust(&TrustRequest::from(root.as_path()))
                .expect("trust root");

            let result = fixture.store.trust_status(&subject);

            assert_eq!(
                result.expect("status"),
                ConfigTrustStatus::MissingBaseline
            );
        }
    }

    mod config_file_trust_check {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_the_trusted_content_when_the_baseline_matches() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            let config_path = Fixture::write_config(&root, "a = 1");
            fixture
                .store
                .trust(&Fixture::config_request(&config_path))
                .expect("trust config");

            let result =
                fixture.store.config_file_trust_check(&root, &config_path);

            assert_eq!(
                result.expect("check"),
                ConfigTrustCheck::Trusted("a = 1".to_owned())
            );
        }
    }

    mod untrust {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_zero_when_the_root_was_never_trusted() {
            let fixture = Fixture::new();
            let subject =
                TrustRequest::from(fixture.project("project").as_path());

            let result = fixture.store.untrust(&subject);

            assert_eq!(result.expect("untrust"), 0);
        }

        #[test]
        fn removes_trust_and_its_baseline() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            let subject =
                Fixture::config_request(&Fixture::write_config(&root, "a = 1"));
            fixture.store.trust(&subject).expect("trust config");

            let removed = fixture.store.untrust(&subject);

            assert_eq!(removed.expect("untrust"), 1);
            assert_eq!(
                fixture.store.trust_status(&subject).expect("status"),
                ConfigTrustStatus::Untrusted
            );
        }
    }

    mod list {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_empty_when_no_root_is_trusted() {
            let fixture = Fixture::new();

            let result = fixture.store.list();

            assert_eq!(result.expect("list"), Vec::<PathBuf>::new());
        }

        #[test]
        fn returns_the_canonical_trusted_roots() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            fixture
                .store
                .trust(&TrustRequest::from(root.as_path()))
                .expect("trust root");

            let result = fixture.store.list();

            assert_eq!(result.expect("list"), vec![
                root.canonicalize().expect("canonicalize")
            ]);
        }
    }

    mod clean {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn prunes_a_root_whose_directory_was_deleted() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            fixture
                .store
                .trust(&TrustRequest::from(root.as_path()))
                .expect("trust root");
            fs::remove_dir_all(&root).expect("remove project");

            let removed = fixture.store.clean();

            assert_eq!(removed.expect("clean"), 1);
            assert_eq!(
                fixture.store.list().expect("list"),
                Vec::<PathBuf>::new()
            );
        }

        #[test]
        fn leaves_live_roots_untouched() {
            let fixture = Fixture::new();
            let root = fixture.project("project");
            fixture
                .store
                .trust(&TrustRequest::from(root.as_path()))
                .expect("trust root");

            let removed = fixture.store.clean();

            assert_eq!(removed.expect("clean"), 0);
            assert_eq!(fixture.store.list().expect("list").len(), 1);
        }
    }
}
