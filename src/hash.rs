//! Hashes file contents, framed fields, and path bytes with BLAKE3.
//!
//! # Key Types
//!
//! - [`Blake3Digest`]: BLAKE3 digest of file contents or framed fields.
//! - [`Blake3PathHash`]: BLAKE3 hex digest of path bytes.
//! - [`HashError`]: file-read failure while hashing contents.

use std::{
    fmt::{self, Display, Formatter},
    io,
    path::{Path, PathBuf},
};

use thiserror::Error;

/// Stores a BLAKE3 digest: of file contents ([`Self::from_path`],
/// `From<&str>`) or of structured values built with `Blake3Hasher`.
///
/// Prefer hashing already-loaded content when the caller has it. Hashing a path
/// and then reading the same path again creates a time-of-check to time-of-use
/// window where the file contents can change between reads.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Blake3Digest(blake3::Hash);

impl Blake3Digest {
    /// Computes the BLAKE3 hash of `path`'s current contents.
    ///
    /// Reads `path` fully into memory before hashing. Use this only when the
    /// path read itself is the source of truth; otherwise hash the content that
    /// was already read to avoid a time-of-check to time-of-use gap.
    ///
    /// # Errors
    ///
    /// Returns a [`HashError`] if `path` cannot be read.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    pub fn from_path(path: &Path) -> Result<Self, HashError> {
        let contents = std::fs::read(path).map_err(|source| HashError {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self(blake3::hash(&contents)))
    }

    /// Returns the BLAKE3 hex representation without heap allocation.
    #[inline]
    #[must_use]
    pub fn to_hex(self) -> impl AsRef<str> + Display {
        self.0.to_hex()
    }
}

impl From<&str> for Blake3Digest {
    #[inline]
    fn from(content: &str) -> Self {
        Self(blake3::hash(content.as_bytes()))
    }
}

impl Display for Blake3Digest {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Incremental, injective encoder behind structured digests.
///
/// Every field is length-prefixed with a little-endian `u64`, so no value, even
/// one containing delimiter bytes, can be re-read as a different field split.
/// Paths are fed as raw OS bytes, and optional paths carry a presence tag so
/// `None` differs from an empty path. The framing is part of every persisted
/// digest: changing it invalidates them.
#[derive(Debug)]
pub(crate) struct Blake3Hasher(blake3::Hasher);

impl Blake3Hasher {
    /// Starts an empty hasher.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self(blake3::Hasher::new())
    }

    /// Feeds `bytes` behind a length prefix.
    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        self.len(bytes.len());
        self.0.update(bytes);
    }

    /// Feeds `len` as a little-endian `u64`.
    ///
    /// Saturates instead of truncating; a length above `u64::MAX` is
    /// unreachable for in-memory data.
    pub(crate) fn len(&mut self, len: usize) {
        self.0.update(&u64::try_from(len).unwrap_or(u64::MAX).to_le_bytes());
    }

    /// Feeds `path`'s raw OS bytes, so non-UTF-8 paths stay distinct.
    pub(crate) fn path(&mut self, path: &Path) {
        self.bytes(path.as_os_str().as_encoded_bytes());
    }

    /// Feeds an optional path behind a presence tag.
    pub(crate) fn optional_path(&mut self, path: Option<&Path>) {
        match path {
            Some(path) => {
                self.0.update(&[1]);
                self.path(path);
            }
            None => {
                self.0.update(&[0]);
            }
        }
    }

    /// Finalizes the hasher into a digest.
    #[must_use]
    pub(crate) fn finish(self) -> Blake3Digest {
        Blake3Digest(self.0.finalize())
    }
}

/// Stores a BLAKE3 hex hash of path bytes.
///
/// Used as a hash-keyed store filename. Callers that need canonical keys must
/// canonicalize before constructing this value.
///
/// Stores the hex digest as `[u8; 64]` instead of a [`String`] because BLAKE3
/// hex encoding is always exactly 64 ASCII bytes. The fixed array avoids a heap
/// allocation while still exposing the digest as UTF-8 when needed.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Blake3PathHash([u8; 64]);

impl From<&Path> for Blake3PathHash {
    #[inline]
    fn from(path: &Path) -> Self {
        let hash = blake3::hash(path.as_os_str().as_encoded_bytes());
        let mut bytes = [0_u8; 64];
        bytes.copy_from_slice(hash.to_hex().as_bytes());
        Self(bytes)
    }
}

impl Blake3PathHash {
    /// Returns the hash string used as a store entry filename.
    ///
    /// # Panics
    ///
    /// Panics if the stored digest bytes are not valid UTF-8. BLAKE3 hex output
    /// is always ASCII, so this can only happen if the invariant inside
    /// [`Blake3PathHash`] is broken.
    #[inline]
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "blake3's hex digest is always ASCII; failure here means \
                  blake3 itself violated its documented output format, not a \
                  recoverable caller error"
    )]
    pub(crate) fn as_str(&self) -> &str {
        str::from_utf8(&self.0)
            .expect("blake3 hex digest is always valid ASCII/UTF-8")
    }
}

/// Reports that a file could not be read for hashing.
#[derive(Debug, Error)]
#[error("failed to read {path} for hashing")]
pub struct HashError {
    /// Path that could not be read.
    pub(crate) path: PathBuf,
    /// Source I/O error.
    #[source]
    pub(crate) source: io::Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod blake3_digest {
        use std::fs;

        use pretty_assertions::{assert_eq, assert_ne};

        use super::*;

        #[test]
        fn deterministic_for_same_content() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let path = temp.path().join("file.txt");
            fs::write(&path, "hello").expect("write file");

            // Act
            let first = Blake3Digest::from_path(path.as_path());
            let second = Blake3Digest::from_path(path.as_path());

            // Assert
            assert!(first.is_ok());
            assert!(second.is_ok());
            assert_eq!(first.unwrap(), second.unwrap());
        }

        #[test]
        fn differs_when_content_changes() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let path1 = temp.path().join("file1.txt");
            let path2 = temp.path().join("file2.txt");
            fs::write(&path1, "hello").expect("write file 1");
            fs::write(&path2, "goodbye").expect("write file 2");

            // Act
            let first = Blake3Digest::from_path(path1.as_path());
            let second = Blake3Digest::from_path(path2.as_path());

            // Assert
            assert!(first.is_ok());
            assert!(second.is_ok());
            assert_ne!(first.unwrap(), second.unwrap());
        }

        #[test]
        fn error_when_file_is_missing() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let path = temp.path().join("missing.txt");

            // Act
            let result = Blake3Digest::from_path(path.as_path());

            // Assert
            assert!(matches!(result, Err(HashError { .. })));
        }

        #[test]
        fn implements_display_formatting() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let path = temp.path().join("file.txt");
            fs::write(&path, "hello").expect("write file");
            let hash =
                Blake3Digest::from_path(path.as_path()).expect("hash file");

            // Act
            let display_string = format!("{hash}");

            // Assert
            assert_eq!(display_string.len(), 64);
            let expected_raw_hash = blake3::hash(b"hello").to_hex().to_string();
            assert_eq!(display_string, expected_raw_hash);
        }
    }

    mod blake3_hasher {
        use pretty_assertions::assert_ne;

        use super::*;

        fn digest(feed: impl FnOnce(&mut Blake3Hasher)) -> Blake3Digest {
            let mut hasher = Blake3Hasher::new();
            feed(&mut hasher);
            hasher.finish()
        }

        #[test]
        fn length_prefixes_keep_adjacent_fields_apart() {
            let first = digest(|hasher| {
                hasher.bytes(b"a\0b");
                hasher.bytes(b"c");
            });
            let second = digest(|hasher| {
                hasher.bytes(b"a");
                hasher.bytes(b"b\0c");
            });

            assert_ne!(first, second);
        }

        #[test]
        fn an_absent_optional_path_differs_from_an_empty_one() {
            let absent = digest(|hasher| hasher.optional_path(None));
            let empty =
                digest(|hasher| hasher.optional_path(Some(Path::new(""))));

            assert_ne!(absent, empty);
        }

        #[cfg(unix)]
        #[test]
        fn paths_differing_only_in_non_utf8_bytes_stay_distinct() {
            use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

            let first = digest(|hasher| {
                hasher.path(Path::new(OsStr::from_bytes(b"/p\xFF")));
            });
            let second = digest(|hasher| {
                hasher.path(Path::new(OsStr::from_bytes(b"/p\xFE")));
            });

            assert_ne!(first, second);
        }
    }

    mod path_hash {
        use pretty_assertions::{assert_eq, assert_ne};

        use super::*;

        #[test]
        fn deterministic_for_same_path_string() {
            // Arrange
            let path = Path::new("/project/.traces/config.toml");

            // Act
            let first = Blake3PathHash::from(path);
            let second = Blake3PathHash::from(path);

            // Assert
            assert_eq!(first, second);
        }

        #[test]
        fn differs_for_different_path_strings() {
            // Arrange
            let first_path = Path::new("/project/first");
            let second_path = Path::new("/project/second");

            // Act
            let first = Blake3PathHash::from(first_path);
            let second = Blake3PathHash::from(second_path);

            // Assert
            assert_ne!(first, second);
        }

        #[test]
        fn matches_blake3_hex_formula_for_path_bytes() {
            // Arrange
            let path = Path::new("/project/.traces/config.toml");

            // Act
            let hash = Blake3PathHash::from(path);

            // Assert
            let expected_hex =
                blake3::hash(path.as_os_str().as_encoded_bytes())
                    .to_hex()
                    .to_string();
            assert_eq!(hash.as_str(), expected_hex.as_str());
        }

        #[test]
        fn exposes_as_str_for_inner_hash() {
            // Arrange
            let path = Path::new("/project/.traces/config.toml");
            let hash = Blake3PathHash::from(path);

            // Act
            let hash_str = hash.as_str();

            // Assert
            let expected_hex =
                blake3::hash(path.as_os_str().as_encoded_bytes())
                    .to_hex()
                    .to_string();
            assert_eq!(hash_str, expected_hex);
        }
    }
}
