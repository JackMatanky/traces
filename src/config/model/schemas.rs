//! Holds schema registry settings and their config digests.

use std::path::{Path, PathBuf};

use super::digest::setting;
use crate::{Blake3Digest, Blake3Hasher, FieldName};

/// Names the default local schemas directory.
pub(in crate::config) const DEFAULT_LOCAL_SCHEMAS_DIR: &str =
    ".traces/schemas/";

/// Names the default schema class field.
pub(in crate::config) const DEFAULT_CLASS_FIELD: &str = "class";

/// Holds the schema registry directory and class field name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemasConfig {
    directory: PathBuf,
    class_field: FieldName,
}

impl SchemasConfig {
    /// Assembles validated schema settings from resolved values.
    ///
    /// Requires `directory` to have been resolved against its Configuration
    /// Scope root; this constructor performs no further validation.
    #[inline]
    #[must_use]
    pub(in crate::config) const fn new(
        directory: PathBuf,
        class_field: FieldName,
    ) -> Self {
        Self {
            directory,
            class_field,
        }
    }

    /// Returns the frontmatter key for a note's file class.
    ///
    /// Defaults to `class` when unconfigured.
    #[inline]
    #[must_use]
    pub fn class_field_name(&self) -> &str {
        self.class_field.as_str()
    }

    /// Returns the schema registry directory.
    ///
    /// Config File loading resolves a configured directory against its
    /// Configuration Scope root, or the workspace root when unconfigured.
    /// [`SchemasConfig::default`] retains a relative directory because it has
    /// no workspace root.
    #[inline]
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Builds test schema settings with a validated class field.
    ///
    /// # Panics
    ///
    /// - `class_field` is not a valid field key.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test inputs must validate; Default carries the same \
                  expectation"
    )]
    pub fn for_test(class_field: &str) -> Self {
        Self {
            directory: PathBuf::from(DEFAULT_LOCAL_SCHEMAS_DIR),
            class_field: FieldName::try_from(class_field)
                .expect("test class field validates as a field key"),
        }
    }

    /// Resolves the default schema directory against `root` for tests.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    pub(super) fn default_for_root(root: &Path) -> Self {
        Self {
            directory: root.join(DEFAULT_LOCAL_SCHEMAS_DIR),
            ..Self::default()
        }
    }
}

impl Default for SchemasConfig {
    /// Builds schema defaults from the hardcoded class field.
    ///
    /// # Panics
    ///
    /// - [`DEFAULT_CLASS_FIELD`] stops being a valid field key.
    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "DEFAULT_CLASS_FIELD is a hardcoded constant; failure here \
                  means the constant itself is malformed, not a recoverable \
                  caller error"
    )]
    fn default() -> Self {
        Self {
            directory: PathBuf::from(DEFAULT_LOCAL_SCHEMAS_DIR),
            class_field: FieldName::try_from(DEFAULT_CLASS_FIELD)
                .expect("DEFAULT_CLASS_FIELD is a valid field key"),
        }
    }
}

const SCHEMAS_DIRECTORY: &[u8] = b"schemas.directory";
const SCHEMAS_CLASS_FIELD: &[u8] = b"schemas.class_field";

/// Holds the digests of [`SchemasConfig`] settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SchemaDigests {
    directory: Blake3Digest,
    class_field: Blake3Digest,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "reserved for index cache invalidation")
)]
impl SchemaDigests {
    /// Returns the digest of the schema registry directory setting.
    #[inline]
    #[must_use]
    pub(crate) const fn directory(&self) -> Blake3Digest {
        self.directory
    }

    /// Returns the digest of the class field setting.
    #[inline]
    #[must_use]
    pub(crate) const fn class_field(&self) -> Blake3Digest {
        self.class_field
    }
}

impl SchemaDigests {
    /// Feeds every setting digest in declaration order.
    pub(super) fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            directory,
            class_field,
        } = self;
        fp.digest(directory);
        fp.digest(class_field);
    }
}

impl SchemasConfig {
    /// Digests the schema registry directory and the class field.
    pub(super) fn digests(&self) -> SchemaDigests {
        let Self {
            directory,
            class_field,
        } = self;
        SchemaDigests {
            directory: setting(SCHEMAS_DIRECTORY, |fp| fp.path(directory)),
            class_field: setting(SCHEMAS_CLASS_FIELD, |fp| {
                class_field.feed_into(fp);
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod schemas_for_test {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn sets_the_expected_class_field() {
            let config = SchemasConfig::for_test("kind");

            assert_eq!(config.class_field_name(), "kind");
        }
    }
}
