//! Implements row encoding and decoding for redb payloads in the index store.
//!
//! Rows use postcard and include [`StoreError`] context. [`path_from_bytes`]
//! decodes invalid UTF-8 lossily. The path codec for serde fields lives at
//! [`crate::path::codec`].

use std::{
    mem,
    path::{Path, PathBuf},
    str,
};

use serde::{Serialize, de::DeserializeOwned};

use super::error::{StoreError, StoreResult};

/// Serializes `value` into `buf`, reusing its existing allocation.
///
/// Clears `buf` before writing. The returned slice borrows from `buf` and is
/// valid until the next call to this function or `buf.clear()`.
///
/// # Errors
///
/// - [`StoreError::Serialize`] if postcard serialization fails.
pub(super) fn encode_row<'a, T: Serialize>(
    path: &Path,
    value: &T,
    buf: &'a mut Vec<u8>,
) -> StoreResult<&'a [u8]> {
    buf.clear();
    *buf = postcard::to_extend(value, mem::take(buf)).map_err(|source| {
        StoreError::Serialize {
            path: path.to_path_buf(),
            source,
        }
    })?;
    Ok(buf)
}

/// Deserializes `bytes` and records `path` in any storage error.
///
/// # Errors
///
/// - [`StoreError::Deserialize`] if postcard deserialization fails.
pub(super) fn decode_row<T: DeserializeOwned>(
    path: &Path,
    bytes: &[u8],
) -> StoreResult<T> {
    postcard::from_bytes(bytes).map_err(|source| StoreError::Deserialize {
        path: path.to_path_buf(),
        source,
    })
}

/// Exposes a borrowed project-relative path's native bytes as a redb key.
///
/// The store writes keys with [`Path::as_encoded_bytes`] and reads them through
/// [`path_from_bytes`]. Serde row payloads use [`crate::path::codec`].
///
/// The wrapper keeps the borrowed path for error construction and row payloads.
/// Converting the bytes back to an [`std::ffi::OsStr`] would require
/// [`std::ffi::OsStr::from_encoded_bytes_unchecked`].
#[derive(Copy, Clone)]
pub(super) struct PathKey<'a>(&'a Path);

impl<'a> PathKey<'a> {
    #[inline]
    pub(super) fn new(path: &'a Path) -> Self {
        Self(path)
    }

    #[inline]
    pub(super) fn as_bytes(&self) -> &'a [u8] {
        self.0.as_os_str().as_encoded_bytes()
    }

    #[inline]
    pub(super) fn path(&self) -> &'a Path {
        self.0
    }
}

/// Builds a path from byte-oriented store data.
///
/// Tries UTF-8 first and falls back to lossy decoding for non-Unicode paths.
/// The fallback can only surface in row-key decode context and query-output
/// paths (tag, class, and folder lookups); stored `LINKS` edges resolve
/// byte-exactly through loaded rows instead, never through this fallback.
pub(super) fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    str::from_utf8(bytes).map_or_else(
        |_| PathBuf::from(String::from_utf8_lossy(bytes).into_owned()),
        PathBuf::from,
    )
}

#[cfg(test)]
mod tests {
    mod encode {
        use pretty_assertions::assert_eq;
        use serde::{Deserialize, Serialize};

        use super::super::encode_row;
        use crate::index::error::StoreError;

        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        struct TestRow {
            value: String,
        }

        #[test]
        fn serializes_successfully() {
            let path = std::path::Path::new("test.md");
            let row = TestRow {
                value: "hello".to_owned(),
            };

            let mut buf = Vec::new();
            let bytes =
                encode_row(path, &row, &mut buf).expect("encode succeeds");
            let decoded: TestRow =
                postcard::from_bytes(bytes).expect("decode succeeds");
            assert_eq!(decoded, row);
        }

        #[test]
        fn fails_on_serialization_error() {
            struct Failing;
            impl Serialize for Failing {
                fn serialize<S>(
                    &self,
                    _serializer: S,
                ) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    Err(serde::ser::Error::custom("forced error"))
                }
            }
            let path = std::path::Path::new("test.md");

            let mut buf = Vec::new();
            let result = encode_row(path, &Failing, &mut buf);

            assert!(matches!(result, Err(StoreError::Serialize { .. })));
        }
    }

    mod decode {
        use pretty_assertions::assert_eq;
        use serde::{Deserialize, Serialize};

        use super::super::decode_row;
        use crate::index::error::StoreError;

        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        struct TestRow {
            value: String,
        }

        #[test]
        fn deserializes_successfully() {
            let path = std::path::Path::new("test.md");
            let row = TestRow {
                value: "hello".to_owned(),
            };
            let bytes = postcard::to_allocvec(&row).unwrap();

            let decoded: TestRow =
                decode_row(path, &bytes).expect("decode succeeds");

            assert_eq!(decoded, row);
        }

        #[test]
        fn fails_on_corrupt_bytes() {
            let path = std::path::Path::new("test.md");
            let result: Result<TestRow, _> = decode_row(path, &[0xFF, 0x00]);
            assert!(matches!(result, Err(StoreError::Deserialize { .. })));
        }
    }

    mod path_from_bytes {
        use std::path::PathBuf;

        use pretty_assertions::assert_eq;

        use super::super::path_from_bytes;

        #[test]
        fn decodes_valid_utf8() {
            let bytes = b"hello.md";
            let path = path_from_bytes(bytes);
            assert_eq!(path, PathBuf::from("hello.md"));
        }

        #[test]
        fn falls_back_to_lossy_on_invalid_utf8() {
            let bytes = b"hello\xFF.md";
            let path = path_from_bytes(bytes);
            assert_eq!(path, PathBuf::from("hello\u{FFFD}.md"));
        }
    }

    mod proptests {
        use std::{
            path::PathBuf,
            time::{Duration, SystemTime},
        };

        use pretty_assertions::assert_eq;
        use proptest::prelude::*;

        use super::super::{decode_row, encode_row};
        use crate::{
            FileMeta, Note, file::FileFormat, test_support::parse_note,
        };

        /// Generates a relative path of one to three unicode segments.
        fn relative_path() -> impl Strategy<Value = PathBuf> {
            prop::collection::vec("[\\p{L}\\p{N}_ -]{1,8}", 1..=3)
                .prop_map(|segments| PathBuf::from(segments.join("/") + ".md"))
        }

        /// Generates a timestamp with sub-second precision after the epoch.
        fn timestamp() -> impl Strategy<Value = SystemTime> {
            (0_u64..4_000_000_000, 0_u32..1_000_000_000).prop_map(
                |(secs, nanos)| {
                    SystemTime::UNIX_EPOCH
                        .checked_add(Duration::new(secs, nanos))
                        .expect("timestamp within SystemTime range")
                },
            )
        }

        /// Generates Markdown with a heading, tags, links, and date fields.
        fn markdown() -> impl Strategy<Value = String> {
            let word = "[\\p{L}]{1,8}";
            (
                prop::collection::vec(word, 0..=3),
                prop::collection::vec(word, 0..=3),
                prop::collection::vec(word, 0..=3),
                prop::option::of((1_u32..=9999, 1_u32..=12, 1_u32..=28)),
            )
                .prop_map(|(headings, tags, links, date)| {
                    let mut parts = Vec::new();
                    if let Some((year, month, day)) = date {
                        parts.push(format!(
                            "---\ncreated: \
                             {year:04}-{month:02}-{day:02}\n---\n"
                        ));
                    }
                    parts.extend(
                        headings
                            .iter()
                            .map(|heading| format!("# {heading}\n\n")),
                    );
                    parts.extend(tags.iter().map(|tag| format!("#{tag} ")));
                    parts.extend(
                        links.iter().map(|link| format!("[[{link}]] ")),
                    );
                    parts.concat()
                })
        }

        proptest! {
            /// Pins the file-row law: any file row decodes to the row that
            /// was encoded, including non-ASCII paths and timestamps.
            #[test]
            fn file_row_round_trips(
                path in relative_path(),
                created_at in prop::option::of(timestamp()),
            ) {
                // Arrange
                let row = FileMeta::note_with_created_at_for_test(
                    path.clone(),
                    created_at,
                );
                let mut buf = Vec::new();

                // Act
                let bytes = encode_row(&path, &row, &mut buf)
                    .expect("file row encodes");
                let decoded: FileMeta = decode_row(&path, bytes)
                    .expect("file row decodes");

                // Assert
                prop_assert_eq!(decoded, row);
            }

            /// Pins the note-row law: a parsed note decodes to the note that
            /// was encoded, whatever headings, tags, links, and dates it has.
            #[test]
            fn note_row_round_trips(
                path in relative_path(),
                source in markdown(),
            ) {
                // Arrange
                let row = parse_note(&path, &source);
                let mut buf = Vec::new();

                // Act
                let bytes = encode_row(&path, &row, &mut buf)
                    .expect("note row encodes");
                let decoded: Note = decode_row(&path, bytes)
                    .expect("note row decodes");

                // Assert
                prop_assert_eq!(decoded, row);
            }

            /// Pins the reused-buffer law: encoding a second row into a
            /// buffer that holds a first row leaves only the second row.
            #[test]
            fn reused_buffer_holds_only_latest_row(
                first in relative_path(),
                second in relative_path(),
            ) {
                // Arrange
                let first_row = FileMeta::for_test(first.clone(), FileFormat::Other);
                let second_row = FileMeta::for_test(second.clone(), FileFormat::Note);
                let mut buf = Vec::new();
                encode_row(&first, &first_row, &mut buf).expect("first encodes");

                // Act
                let bytes = encode_row(&second, &second_row, &mut buf)
                    .expect("second encodes");
                let decoded: FileMeta = decode_row(&second, bytes)
                    .expect("second decodes");

                // Assert
                assert_eq!(decoded.path(), second_row.path());
            }
        }
    }
}
