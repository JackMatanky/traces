//! Trust request targets, trust status models, and the persistent trust store.
//!
//! # Types
//!
//! - [`TrustRequest`] names either a workspace root or one local config file.
//! - [`WorkspaceTrustStatus`] describes root trust.
//! - [`ConfigTrustStatus`] adds config-baseline states for detecting stale
//!   local config files.
//! - [`TrustStore`] owns cryptographic trust verification and the content-hash
//!   companion file that backs it.

use std::{
    fmt::{self, Display, Formatter},
    fs,
    path::{Path, PathBuf},
};

use super::{
    error::ConfigTrustError,
    file::{Discovered, LocalConfigFile, Tracked},
};
use crate::{
    Blake3FileHash, CleanMode, FilePathTracker, dirs, hash::HashError,
};

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

impl From<&LocalConfigFile<Discovered>> for TrustRequest {
    #[inline]
    fn from(file: &LocalConfigFile<Discovered>) -> Self {
        Self::Config {
            root: file.root().to_path_buf(),
            path: file.path().to_path_buf(),
        }
    }
}

impl From<&LocalConfigFile<Tracked>> for TrustRequest {
    #[inline]
    fn from(file: &LocalConfigFile<Tracked>) -> Self {
        Self::Config {
            root: file.root().to_path_buf(),
            path: file.path().to_path_buf(),
        }
    }
}

/// Trust requests resolved from a single discovery operation.
pub(crate) type TrustRequests = Vec<TrustRequest>;

/// Trust state for a workspace root.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkspaceTrustStatus {
    /// The workspace root is trusted.
    Trusted,
    /// The workspace root is not trusted.
    Untrusted,
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

impl From<WorkspaceTrustStatus> for ConfigTrustStatus {
    #[inline]
    fn from(status: WorkspaceTrustStatus) -> Self {
        match status {
            WorkspaceTrustStatus::Trusted => Self::Trusted,
            WorkspaceTrustStatus::Untrusted => Self::Untrusted,
        }
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
    /// The workspace root is trusted and a baseline hash exists, but the
    /// config file's current content no longer matches it.
    Stale,
    /// The workspace root is trusted, but no content-hash baseline was ever
    /// recorded for this config file.
    MissingBaseline,
    /// The workspace root is trusted and the config file's content matches
    /// its baseline hash.
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

const COMPANION_SUFFIX: &str = ".hash";

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
    /// - [`ConfigTrustError::Hash`] when the config file cannot be hashed.
    #[inline]
    pub(crate) fn trust(
        &self,
        subject: &TrustRequest,
    ) -> Result<(), ConfigTrustError> {
        if let Some(config_file) = subject.config_file() {
            let digest = Blake3FileHash::from_path(config_file)?;
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
        if subject.config_file().is_some() {
            self.config_trust_status(subject)
        } else {
            self.workspace_trust_status(subject).map(ConfigTrustStatus::from)
        }
    }

    /// Returns the workspace-root trust status.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] when the trust store cannot be read.
    pub(crate) fn workspace_trust_status(
        &self,
        subject: &TrustRequest,
    ) -> Result<WorkspaceTrustStatus, ConfigTrustError> {
        if self.store.contains(subject.root_path())? {
            Ok(WorkspaceTrustStatus::Trusted)
        } else {
            Ok(WorkspaceTrustStatus::Untrusted)
        }
    }

    /// Returns the config-file trust status.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] when the trust store cannot be read.
    /// - [`ConfigTrustError::Hash`] when the config file cannot be hashed.
    #[inline]
    pub(crate) fn config_trust_status(
        &self,
        subject: &TrustRequest,
    ) -> Result<ConfigTrustStatus, ConfigTrustError> {
        let Some(config_file) = subject.config_file() else {
            return Ok(if self.store.contains(subject.root_path())? {
                ConfigTrustStatus::Trusted
            } else {
                ConfigTrustStatus::Untrusted
            });
        };
        self.config_file_trust_check(subject.root_path(), config_file)
            .map(|check| check.status())
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
    /// - [`ConfigTrustError::Hash`] when the config file cannot be read.
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
        let content = fs::read_to_string(config_path).map_err(|source| {
            ConfigTrustError::Hash(HashError {
                path: config_path.to_path_buf(),
                source,
            })
        })?;
        let current = Blake3FileHash::from(content.as_str());
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
    /// - If the trust store cannot remove the root entry or its content-hash
    ///   companion.
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
