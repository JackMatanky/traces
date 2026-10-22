//! Models trust requests and Trust Status, and persists Trust Verification.
//!
//! # Key Types
//!
//! - [`TrustRequest`]: identifies a workspace root or local Config File.
//! - [`ConfigTrustStatus`]: reports the root and Companion Hash status.
//! - [`TrustStore`]: grants, checks, and revokes trust for workspace roots.

use std::{
    fmt::{self, Display, Formatter},
    fs,
    path::{Path, PathBuf},
};

use super::error::ConfigTrustError;
use crate::{Blake3Digest, CleanMode, FilePathTracker, dirs, hash::HashError};

const COMPANION_SUFFIX: &str = ".hash";

/// Targets a workspace root or Config File for a trust operation.
///
/// A [`Path`] produces a root-only request. [`Self::Config`] retains a Config
/// File path and root so trust grants can record a Companion Hash.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrustRequest {
    /// Targets a workspace root without a Config File.
    Root(PathBuf),
    /// Targets a Config File and its workspace root.
    Config {
        /// Holds the workspace root.
        root: PathBuf,
        /// Holds the Config File path.
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

/// Collects trust requests resolved by one discovery operation.
pub(crate) type TrustRequests = Vec<TrustRequest>;

/// Selects how a user-supplied path resolves to trust requests.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum TrustScope {
    /// Selects the nearest local Config File or a root-only request.
    NearestLocal,
    /// Selects Config Files in a subtree or the enclosing Config File.
    LocalSubtree,
}

/// Describes the Trust Status of a Config File and its workspace root.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ConfigTrustStatus {
    /// Confirms root trust and, for a Config File, a matching Companion Hash.
    Trusted,
    /// Indicates that the workspace root lacks trust.
    Untrusted,
    /// Indicates that the trusted root lacks a Config File Companion Hash.
    MissingBaseline,
    /// Indicates that the Config File no longer matches its Companion Hash.
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

/// Couples Trust Verification with already-read Config File content.
///
/// Only [`Self::Trusted`] carries content, preventing callers from treating
/// unverified content as trusted.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ConfigTrustCheck {
    /// Indicates that the workspace root lacks trust.
    Untrusted,
    /// Indicates that the Config File no longer matches its Companion Hash.
    Stale,
    /// Indicates that the trusted root lacks a Config File Companion Hash.
    MissingBaseline,
    /// Holds verified content matching the Companion Hash.
    ///
    /// Reuses the bytes read for verification during parsing.
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

/// Reads a Config File as UTF-8 text for Companion Hash computation.
///
/// Both trust grants and checks use this reader, so a recorded hash always
/// describes content that a later check can read.
///
/// # Errors
///
/// - [`Hash`] if reading the file fails or its content is not valid UTF-8.
///
/// [`Hash`]: ConfigTrustError::Hash
fn read_config(path: &Path) -> Result<String, ConfigTrustError> {
    fs::read_to_string(path).map_err(|source| {
        ConfigTrustError::Hash(HashError {
            path: path.to_path_buf(),
            source,
        })
    })
}

/// Persists trusted workspace roots and their Companion Hashes.
///
/// Owns Trust Verification when granting, revoking, or checking trust.
#[derive(Clone, Debug)]
pub(crate) struct TrustStore {
    store: FilePathTracker,
}

impl TrustStore {
    /// Grants trust for a [`TrustRequest`]'s workspace root.
    ///
    /// For a Config File request, records its Companion Hash before trusting
    /// the root; reading or recording failures prevent the trust grant.
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if recording the Companion Hash or root fails.
    /// - [`Hash`] if reading the Config File fails or its content is not UTF-8.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    /// [`Hash`]: ConfigTrustError::Hash
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

    /// Returns the Trust Status for a [`TrustRequest`].
    ///
    /// Checks the Companion Hash for a Config File request; checks only root
    /// presence for a root-only request.
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if reading the Trust Store fails.
    /// - [`Hash`] if reading the Config File fails or its content is not UTF-8.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    /// [`Hash`]: ConfigTrustError::Hash
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

    /// Checks Trust Status and returns Config File content only when trusted.
    ///
    /// Reads and hashes content once so parsing can reuse verified bytes.
    /// Accepts a root and Config File path separately because a root-only
    /// [`TrustRequest`] has no Config File path.
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if reading the Trust Store or Companion Hash fails.
    /// - [`Hash`] if reading the Config File fails or its content is not UTF-8.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    /// [`Hash`]: ConfigTrustError::Hash
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

    /// Revokes trust for a [`TrustRequest`]'s workspace root.
    ///
    /// Removes the root and its Companion Hash; returns the number of root
    /// entries removed.
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if removing the root or Companion Hash fails.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    #[inline]
    pub(crate) fn untrust(
        &self,
        subject: &TrustRequest,
    ) -> Result<usize, ConfigTrustError> {
        self.store
            .remove_with_companions(subject.root_path(), &[COMPANION_SUFFIX])
            .map_err(Into::into)
    }

    /// Lists the canonical paths of trusted workspace roots.
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if reading the Trust Store fails.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    #[inline]
    pub(crate) fn list(&self) -> Result<Vec<PathBuf>, ConfigTrustError> {
        self.store.list_all(&[COMPANION_SUFFIX]).map_err(Into::into)
    }

    /// Removes dangling trust entries and their Companion Hashes.
    ///
    /// Returns the number of root entries removed.
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if reading the Trust Store or removing a stale root or its
    ///   Companion Hash fails.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    #[inline]
    pub(crate) fn clean(&self) -> Result<usize, ConfigTrustError> {
        self.store
            .clean(CleanMode::WithCompanions(&[COMPANION_SUFFIX]))
            .map_err(Into::into)
    }

    /// Creates a Trust Store at an explicit root instead of the OS state
    /// directory.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    #[inline]
    pub(crate) fn for_test<P: Into<PathBuf>>(root: P) -> Self {
        Self {
            store: FilePathTracker::for_test(root),
        }
    }
}

impl Default for TrustStore {
    /// Creates a Trust Store in the OS state directory.
    #[inline]
    fn default() -> Self {
        Self {
            store: FilePathTracker::from(dirs::TRUSTED_CONFIGS.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use fixtures::Fixture;

    use super::*;
    use crate::config::file::LocalConfigFile;

    mod fixtures {
        use super::*;

        /// Holds an isolated Trust Store and a temporary project directory.
        pub(super) struct Fixture {
            temp: tempfile::TempDir,
            pub(super) store: TrustStore,
        }

        impl Fixture {
            pub(super) fn new() -> Self {
                let temp = tempfile::tempdir().expect("create temp dir");
                let store =
                    TrustStore::for_test(temp.path().join("trust-store"));
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
