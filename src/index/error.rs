//! Defines errors and result aliases for index scans, persistence, and loads.

use std::{io, path::PathBuf};

use thiserror::Error;

use crate::{DirTreeError, path::PathError};

/// Holds an index operation's value or its [`IndexError`].
pub type IndexResult<T> = std::result::Result<T, IndexError>;
/// Holds a store operation's value or its [`StoreError`].
pub(crate) type StoreResult<T> = std::result::Result<T, StoreError>;

/// Reports failures from [`super::IndexerService`] and workspace index
/// operations.
#[derive(Debug, Error)]
#[expect(
    private_interfaces,
    reason = "StoreError, DirTreeError, and PathError are pub(crate), \
              IndexError is pub"
)]
pub enum IndexError {
    /// Reports a database access or row encoding/decoding failure.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Reports a directory traversal failure during a filesystem scan.
    #[error(transparent)]
    Scan(#[from] DirTreeError),
    /// Reports that path validation rejected a non-relative or unsafe path.
    #[error(transparent)]
    Path(#[from] PathError),
    /// Reports when the index cannot read a Markdown note or decode it as
    /// UTF-8.
    #[error("failed to parse note {path}")]
    NoteParse {
        /// Names the note that the index could not read or parse.
        path: PathBuf,
        /// Holds the underlying read error.
        #[source]
        source: io::Error,
    },
    /// Reports when a scan cannot inspect file metadata.
    #[error("failed to inspect {path}")]
    Inspect {
        /// Names the file whose metadata the scan could not inspect.
        path: PathBuf,
        /// Holds the underlying metadata error.
        #[source]
        source: io::Error,
    },
}

/// Reports low-level failures while persisting the index.
#[derive(Debug, Error)]
pub(crate) enum StoreError {
    /// Reports a filesystem access failure.
    #[error("failed to access {path}")]
    Io {
        /// Names the path that the store could not access.
        path: PathBuf,
        /// Holds the underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// Reports a failure when the index opens, reads, or writes the redb
    /// database.
    #[error("failed to access the database at {path}")]
    Redb {
        /// Names the database file.
        path: PathBuf,
        /// Holds the underlying redb error.
        #[source]
        source: Box<redb::Error>,
    },
    /// Reports a row serialization failure.
    #[error("failed to serialize the row for {path}")]
    Serialize {
        /// Names the row's path.
        path: PathBuf,
        /// Holds the underlying serialization error.
        #[source]
        source: postcard::Error,
    },
    /// Reports a failure when the index decodes a stored row.
    #[error("failed to deserialize the row for {path}")]
    Deserialize {
        /// Names the row's path.
        path: PathBuf,
        /// Holds the underlying deserialization error.
        #[source]
        source: postcard::Error,
    },
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::*;

    mod store_error_display {
        use super::*;

        #[test]
        fn io_includes_path_in_message() {
            let err = StoreError::Io {
                path: PathBuf::from("data.csv"),
                source: io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "denied",
                ),
            };

            assert!(err.to_string().contains("data.csv"));
        }

        #[test]
        fn redb_includes_path_in_message() {
            let err = StoreError::Redb {
                path: PathBuf::from(".traces/index.redb"),
                source: Box::new(redb::Error::DatabaseAlreadyOpen),
            };

            assert!(err.to_string().contains(".traces/index.redb"));
        }

        #[test]
        fn serialize_includes_path_in_message() {
            let err = StoreError::Serialize {
                path: PathBuf::from("note.md"),
                source: postcard::Error::DeserializeUnexpectedEnd,
            };

            assert!(err.to_string().contains("note.md"));
        }

        #[test]
        fn deserialize_includes_path_in_message() {
            let err = StoreError::Deserialize {
                path: PathBuf::from("note.md"),
                source: postcard::Error::DeserializeUnexpectedEnd,
            };

            assert!(err.to_string().contains("note.md"));
        }
    }

    mod index_error_display {
        use super::*;

        #[test]
        fn note_parse_includes_path_in_message() {
            let err = IndexError::NoteParse {
                path: PathBuf::from("notes/bad.md"),
                source: io::Error::new(io::ErrorKind::InvalidData, "not utf8"),
            };

            assert!(err.to_string().contains("bad.md"));
        }

        #[test]
        fn inspect_message_starts_with_failed_to_inspect() {
            let err = IndexError::Inspect {
                path: PathBuf::from("notes/gone.md"),
                source: io::Error::new(io::ErrorKind::NotFound, "missing"),
            };

            assert_eq!(err.to_string(), "failed to inspect notes/gone.md");
        }

        #[test]
        fn inspect_exposes_the_io_source() {
            let source =
                io::Error::new(io::ErrorKind::PermissionDenied, "denied");
            let err = IndexError::Inspect {
                path: PathBuf::from("notes/locked.md"),
                source,
            };

            let reported = err
                .source()
                .expect("io source")
                .downcast_ref::<io::Error>()
                .expect("io::Error");
            assert_eq!(reported.kind(), io::ErrorKind::PermissionDenied);
        }
    }

    mod transparent_forwarding {
        use super::*;

        #[test]
        fn store_display_matches_the_wrapped_store_error_with_no_added_text() {
            let store_error = StoreError::Redb {
                path: PathBuf::from(".traces/index.redb"),
                source: Box::new(redb::Error::DatabaseAlreadyOpen),
            };
            let store_message = store_error.to_string();

            let wrapped = IndexError::Store(store_error);

            assert_eq!(wrapped.to_string(), store_message);
        }

        #[test]
        fn scan_display_matches_the_wrapped_dir_tree_error_with_no_added_text()
        {
            let walk_error = DirTreeError::NodeInaccessible {
                path: PathBuf::from("orphan.md"),
                source: io::Error::other("boom"),
            };
            let walk_message = walk_error.to_string();

            let wrapped = IndexError::Scan(walk_error);

            assert_eq!(wrapped.to_string(), walk_message);
        }

        #[test]
        fn store_source_skips_straight_to_the_store_errors_own_source() {
            // `#[error(transparent)]` hides the wrapping variant from the
            // source chain: `.source()` returns the wrapped error's source,
            // not the `StoreError`.
            let err = IndexError::Store(StoreError::Io {
                path: PathBuf::from("x"),
                source: io::Error::new(io::ErrorKind::BrokenPipe, "pipe"),
            });

            let source = err.source().expect("source present");
            assert!(source.downcast_ref::<StoreError>().is_none());
            assert_eq!(
                source.downcast_ref::<io::Error>().map(io::Error::kind),
                Some(io::ErrorKind::BrokenPipe)
            );
        }

        #[test]
        fn scan_source_skips_straight_to_the_dir_tree_errors_own_source() {
            let err = IndexError::Scan(DirTreeError::NodeInaccessible {
                path: PathBuf::from("x"),
                source: io::Error::new(io::ErrorKind::InvalidData, "bad"),
            });

            let source = err.source().expect("source present");
            assert!(source.downcast_ref::<DirTreeError>().is_none());
            assert_eq!(
                source.downcast_ref::<io::Error>().map(io::Error::kind),
                Some(io::ErrorKind::InvalidData)
            );
        }

        #[test]
        fn note_parse_source_returns_the_io_error() {
            let err = IndexError::NoteParse {
                path: PathBuf::from("x"),
                source: io::Error::new(io::ErrorKind::InvalidData, "bad"),
            };

            let source = err.source().expect("source present");
            assert_eq!(
                source.downcast_ref::<io::Error>().map(io::Error::kind),
                Some(io::ErrorKind::InvalidData)
            );
        }

        #[test]
        fn store_error_converts_to_the_store_variant() {
            let store_error = StoreError::Io {
                path: PathBuf::from("x"),
                source: io::Error::other("boom"),
            };

            let converted: IndexError = store_error.into();

            assert!(matches!(
                converted,
                IndexError::Store(StoreError::Io { .. })
            ));
        }

        #[test]
        fn dir_tree_error_converts_to_the_scan_variant() {
            let walk_error = DirTreeError::NodeInaccessible {
                path: PathBuf::from("x"),
                source: io::Error::other("boom"),
            };

            let converted: IndexError = walk_error.into();

            assert!(matches!(
                converted,
                IndexError::Scan(DirTreeError::NodeInaccessible { .. })
            ));
        }
    }
}
