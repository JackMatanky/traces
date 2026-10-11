//! Best-effort store of local config files discovery has seen.
//!
//! [`TrackerStore`] records which `.traces/config.toml` paths configuration
//! loading has observed, independent of whether their workspace is trusted.

use std::path::PathBuf;

use super::{
    error::ConfigTrustError,
    file::{Discovered, LocalConfigFile},
};
use crate::{CleanMode, FilePathTracker, dirs};

/// Best-effort registry of local config files observed during config loading,
/// kept separate from [`super::trust::TrustStore`]'s trust decisions.
#[derive(Clone, Debug)]
pub(crate) struct TrackerStore {
    store: FilePathTracker,
}

impl TrackerStore {
    /// Creates a store backed by the OS-correct tracked-config directory.
    #[must_use]
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            store: FilePathTracker::from(dirs::TRACKED_CONFIGS.clone()),
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

    /// Records that discovery saw a config file.
    ///
    /// Best-effort: tracking is bookkeeping, so write failures warn and do not
    /// fail config loading.
    #[inline]
    pub(crate) fn record(&self, config: &LocalConfigFile<Discovered>) {
        if let Err(error) = self.store.record(config.path()) {
            tracing::warn!(
                path = %config.path().display(),
                error = %error,
                "failed to record seen config file"
            );
        }
    }

    /// Lists the canonical paths of all live tracked configs.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] if reading the tracking store fails.
    #[inline]
    pub(crate) fn list(&self) -> Result<Vec<PathBuf>, ConfigTrustError> {
        self.store.list_all(&[]).map_err(Into::into)
    }

    /// Removes dangling tracked-config entries, returning the count removed.
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] if reading or cleaning the store fails.
    #[inline]
    pub(crate) fn clean(&self) -> Result<usize, ConfigTrustError> {
        self.store.clean(CleanMode::EntriesOnly).map_err(Into::into)
    }
}
