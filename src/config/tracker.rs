//! Records Tracked Config Paths independently of Trust Verification.
//!
//! [`TrackerStore`] records local Config Files seen during loading or trust
//! grants. Loading treats recording as best effort; grants require success.

use std::path::{Path, PathBuf};

use super::error::ConfigTrustError;
use crate::{CleanMode, FilePathTracker, dirs};

/// Stores Tracked Config Paths separately from [`super::trust::TrustStore`].
///
/// Loading records paths on a best-effort basis; trust grants require a
/// successful record before they add a trusted root.
#[derive(Clone, Debug)]
pub(crate) struct TrackerStore {
    store: FilePathTracker,
}

impl TrackerStore {
    /// Records a Tracked Config Path, failing if the store cannot persist it.
    ///
    /// Trust grants use this method; configuration loading uses
    /// [`Self::record_best_effort`].
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if the path cannot be canonicalized or the store cannot be
    ///   written.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    #[inline]
    pub(crate) fn record(&self, path: &Path) -> Result<(), ConfigTrustError> {
        self.store.record(path).map_err(Into::into)
    }

    /// Records a Tracked Config Path without failing configuration loading.
    ///
    /// Logs a warning if [`Self::record`] fails.
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

    /// Lists the canonical paths of live Tracked Config Paths.
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if the Tracker Store cannot be read.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    #[inline]
    pub(crate) fn list(&self) -> Result<Vec<PathBuf>, ConfigTrustError> {
        self.store.list_all(&[]).map_err(Into::into)
    }

    /// Removes dangling Tracked Config Paths and returns the number removed.
    ///
    /// # Errors
    ///
    /// - [`Tracker`] if the Tracker Store cannot be read or cleaned.
    ///
    /// [`Tracker`]: ConfigTrustError::Tracker
    #[inline]
    pub(crate) fn clean(&self) -> Result<usize, ConfigTrustError> {
        self.store.clean(CleanMode::EntriesOnly).map_err(Into::into)
    }

    /// Creates a Tracker Store at an explicit root instead of the OS
    /// tracked-config directory.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    #[inline]
    pub(crate) fn for_test<P: Into<PathBuf>>(root: P) -> Self {
        Self {
            store: FilePathTracker::for_test(root),
        }
    }
}

impl Default for TrackerStore {
    /// Creates a Tracker Store in the OS tracked-config directory.
    #[inline]
    fn default() -> Self {
        Self {
            store: FilePathTracker::from(dirs::TRACKED_CONFIGS.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    mod fixtures {
        use super::*;

        /// Holds an isolated Tracker Store and project directories for tests.
        pub(super) struct Fixture {
            temp: tempfile::TempDir,
            pub(super) store: TrackerStore,
        }

        impl Fixture {
            pub(super) fn new() -> Self {
                let temp = tempfile::tempdir().expect("create temp dir");
                let store =
                    TrackerStore::for_test(temp.path().join("tracked-store"));
                Self {
                    temp,
                    store,
                }
            }

            /// Writes a local Config File and returns its path.
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
