//! Provides persistent file indexing, metadata caching, and incremental refresh
//! for a project root.
//!
//! [`IndexerService`] scans the filesystem, parses Markdown notes, derives
//! inbound links, and stores the resulting [`WorkspaceIndex`] in redb through
//! the [`store`] module.
//!
//! [`WorkspaceIndex`] holds file entries, parsed metadata, and link data in
//! memory. It stores no root path. Callers inspect its sorted
//! [`WorkspaceIndex::entries`] view and use [`crate::query::QueryService`] to
//! evaluate queries.
//!
//! Fresh builds use [`IndexerService::build`]. Cold CLI reads use
//! [`IndexerService::refresh_store`](service::IndexerService::refresh_store) to
//! load and refresh an existing store. Refresh compares the stored
//! [`fingerprint`] with the running config and rebuilds the index when they
//! differ.
//!
//! [`fingerprint`]: mod@fingerprint
//! [`store`]: mod@store
mod codec;
mod delta;
mod entry;
mod error;
mod fingerprint;
mod inlinks;
mod refresh;
mod service;
mod sort;
mod store;
mod tables;
mod trie;

pub(crate) use entry::RowIndex;
pub use entry::{FileEntry, WorkspaceIndex};
pub use error::{IndexError, IndexResult};
#[cfg(any(test, feature = "test-utils"))]
pub use fingerprint::{DriftCause, IndexSetting};
#[cfg(any(test, feature = "test-utils"))]
pub use inlinks::InlinkMap;
#[cfg(any(test, feature = "test-utils"))]
pub use refresh::RefreshReport;
pub use service::IndexerService;
pub(crate) use sort::SortedByPath;
pub(crate) use store::IndexStore;

/// Names the project-relative index database path.
const INDEX_FILE: &str = ".traces/index.redb";

/// Builds a [`WorkspaceIndex`] from the stored pass or its retained in-memory
/// update.
///
/// # Errors
///
/// - [`IndexError::Store`] if materialization cannot read unchanged notes.
fn materialize_apply(
    persisted: Result<refresh::Persisted, refresh::PersistFailed>,
) -> IndexResult<WorkspaceIndex> {
    match persisted {
        Ok(persisted) => persisted.into_index(),
        Err(failed) => {
            tracing::warn!(
                source = %failed.source(),
                "failed to persist refreshed index"
            );
            failed.into_index()
        }
    }
}

#[cfg(test)]
mod tests {
    /// Holds fixtures shared by the `service.rs` and `store.rs` tests.
    pub(crate) mod fixtures {
        use std::{fs, path::Path};

        /// Restores directory permissions on drop so the temporary directory
        /// can be removed.
        #[cfg(unix)]
        pub struct PermissionsGuard<'a>(pub &'a Path);

        #[cfg(unix)]
        impl Drop for PermissionsGuard<'_> {
            fn drop(&mut self) {
                use std::os::unix::fs::PermissionsExt as _;

                let _ = fs::set_permissions(
                    self.0,
                    fs::Permissions::from_mode(0o700),
                );
            }
        }
    }
}
