//! Best-effort store of local config files discovery has seen.
//!
//! [`TrackerStore`] records which `.traces/config.toml` paths configuration
//! loading has observed, independent of whether their workspace is trusted.

use std::path::{Path, PathBuf};

use super::error::ConfigTrustError;
use crate::{CleanMode, FilePathTracker, dirs};

/// Registry of local config files observed by config loading and trust
/// grants: best-effort on load, required on grant. Kept separate from
/// [`super::trust::TrustStore`]'s trust decisions.
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

    /// Records a config file path.
    ///
    /// Use when failing to record must abort the caller, as when granting
    /// trust. Config loading uses [`Self::record_best_effort`].
    ///
    /// # Errors
    ///
    /// - [`ConfigTrustError::Tracker`] if the path cannot be canonicalized or
    ///   the store cannot be written.
    #[inline]
    pub(crate) fn record(&self, path: &Path) -> Result<(), ConfigTrustError> {
        self.store.record(path).map_err(Into::into)
    }

    /// Records a config file path seen during loading.
    ///
    /// Tracking is bookkeeping, so failures warn and do not fail config
    /// loading.
    #[inline]
    pub(crate) fn record_best_effort(&self, path: &Path) {
        if let Err(error) = self.record(path) {
            tracing::warn!(
                path = %path.display(),
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

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    mod fixtures {
        use super::*;

        /// Isolated tracker store plus project directories beneath one temp
        /// dir.
        pub(super) struct Fixture {
            temp: tempfile::TempDir,
            pub(super) store: TrackerStore,
        }

        impl Fixture {
            pub(super) fn new() -> Self {
                let temp = tempfile::tempdir().expect("create temp dir");
                let store = TrackerStore::at(temp.path().join("tracked-store"));
                Self {
                    temp,
                    store,
                }
            }

            /// Writes `<name>/.traces/config.toml` and returns its path.
            pub(super) fn write_config(&self, name: &str) -> PathBuf {
                let path =
                    self.temp.path().join(name).join(".traces/config.toml");
                fs::create_dir_all(path.parent().expect("config parent"))
                    .expect("create config dir");
                fs::write(&path, "").expect("write config");
                path
            }
        }
    }

    use fixtures::Fixture;

    mod record {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn stores_the_canonical_config_path() {
            let fixture = Fixture::new();
            let path = fixture.write_config("project");

            fixture.store.record(&path).expect("record");

            assert_eq!(fixture.store.list().expect("list"), vec![
                path.canonicalize().expect("canonicalize")
            ]);
        }

        #[test]
        fn keeps_a_single_entry_when_recorded_twice() {
            let fixture = Fixture::new();
            let path = fixture.write_config("project");

            fixture.store.record(&path).expect("first record");
            fixture.store.record(&path).expect("second record");

            assert_eq!(fixture.store.list().expect("list").len(), 1);
        }

        #[test]
        fn reports_a_missing_path() {
            let fixture = Fixture::new();

            let error = fixture
                .store
                .record(Path::new("/nonexistent/x"))
                .expect_err("missing path cannot be recorded");

            assert!(matches!(error, ConfigTrustError::Tracker(_)));
        }
    }

    mod record_best_effort {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn swallows_a_missing_path() {
            let fixture = Fixture::new();

            fixture.store.record_best_effort(Path::new("/nonexistent/x"));

            assert_eq!(
                fixture.store.list().expect("list"),
                Vec::<PathBuf>::new()
            );
        }
    }

    mod list {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_empty_when_nothing_was_recorded() {
            let fixture = Fixture::new();

            let result = fixture.store.list();

            assert_eq!(result.expect("list"), Vec::<PathBuf>::new());
        }
    }

    mod clean {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn prunes_entries_whose_config_was_deleted() {
            let fixture = Fixture::new();
            let path = fixture.write_config("project");
            fixture.store.record(&path).expect("record");
            fs::remove_file(&path).expect("remove config");

            let removed = fixture.store.clean();

            assert_eq!(removed.expect("clean"), 1);
            assert_eq!(
                fixture.store.list().expect("list"),
                Vec::<PathBuf>::new()
            );
        }

        #[test]
        fn leaves_live_entries_untouched() {
            let fixture = Fixture::new();
            let path = fixture.write_config("project");
            fixture.store.record(&path).expect("record");

            let removed = fixture.store.clean();

            assert_eq!(removed.expect("clean"), 0);
            assert_eq!(fixture.store.list().expect("list").len(), 1);
        }
    }
}
