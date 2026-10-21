//! Holds canonical metadata key names, date fields, and their config digests.

use super::digest::setting;
use crate::{
    Blake3Digest, Blake3Hasher, FieldName, FieldNameError,
    config::{
        error::ConfigValueError,
        raw::{RawDateFieldConfig, RawFrontmatterConfig},
    },
};

/// Names the default title field.
const DEFAULT_TITLE_FIELD: &str = "title";

/// Names the default aliases field.
const DEFAULT_ALIASES_FIELD: &str = "aliases";

/// Names the default tags field.
const DEFAULT_TAGS_FIELD: &str = "tags";

/// Names the default creation date field.
const DEFAULT_DATE_CREATED_FIELD: &str = "date_created";

/// Names the default modification date field.
const DEFAULT_DATE_MODIFIED_FIELD: &str = "date_modified";

/// Holds canonical metadata key names and date formats from `[frontmatter]`.
#[derive(Clone, Debug)]
pub struct FrontmatterConfig {
    title: FieldName,
    aliases: FieldName,
    tags: FieldName,
    date_created: DateFieldConfig,
    date_modified: DateFieldConfig,
}

impl FrontmatterConfig {
    /// Returns the frontmatter key for a note's display title.
    #[inline]
    #[must_use]
    pub fn title_name(&self) -> &str {
        self.title.as_str()
    }

    /// Returns the frontmatter key for a note's aliases.
    #[inline]
    #[must_use]
    pub fn aliases_name(&self) -> &str {
        self.aliases.as_str()
    }

    /// Returns the frontmatter key for a note's tags.
    #[inline]
    #[must_use]
    pub fn tags_name(&self) -> &str {
        self.tags.as_str()
    }

    /// Returns the creation date field name and format.
    #[inline]
    #[must_use]
    pub const fn date_created(&self) -> &DateFieldConfig {
        &self.date_created
    }

    /// Returns the modification date field name and format.
    #[inline]
    #[must_use]
    pub const fn date_modified(&self) -> &DateFieldConfig {
        &self.date_modified
    }

    /// Digests the metadata role names and date formats.
    pub(super) fn digests(&self) -> FrontmatterDigests {
        let Self {
            title,
            aliases,
            tags,
            date_created,
            date_modified,
        } = self;
        FrontmatterDigests {
            title: setting(FRONTMATTER_TITLE, |fp| title.feed_into(fp)),
            aliases: setting(FRONTMATTER_ALIASES, |fp| aliases.feed_into(fp)),
            tags: setting(FRONTMATTER_TAGS, |fp| tags.feed_into(fp)),
            date_created: date_created.digests(&DATE_CREATED),
            date_modified: date_modified.digests(&DATE_MODIFIED),
        }
    }

    /// Builds test metadata settings with custom title and aliases keys.
    ///
    /// # Panics
    ///
    /// - `title` or `aliases` is empty, whitespace-only, or contains `/`.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test-only constructor; an invalid literal here is a test \
                  fixture bug, not a recoverable caller error"
    )]
    pub fn for_test<T: Into<String>, A: Into<String>>(
        title: T,
        aliases: A,
    ) -> Self {
        Self {
            title: FieldName::try_from(title.into())
                .expect("test fixture title is a valid field key"),
            aliases: FieldName::try_from(aliases.into())
                .expect("test fixture aliases is a valid field key"),
            ..Self::default()
        }
    }

    /// Overrides the tags key in test metadata settings.
    ///
    /// # Panics
    ///
    /// - `tags` is empty, whitespace-only, or contains `/`.
    #[cfg(any(test, feature = "test-utils"))]
    #[inline]
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "test-only constructor; an invalid literal here is a test \
                  fixture bug, not a recoverable caller error"
    )]
    pub fn with_tags_name<T: Into<String>>(mut self, tags: T) -> Self {
        self.tags = FieldName::try_from(tags.into())
            .expect("test fixture tags is a valid field key");
        self
    }
}

impl Default for FrontmatterConfig {
    /// Builds canonical metadata defaults from hardcoded field names.
    ///
    /// # Panics
    ///
    /// - [`DEFAULT_TITLE_FIELD`], [`DEFAULT_ALIASES_FIELD`],
    ///   [`DEFAULT_TAGS_FIELD`], [`DEFAULT_DATE_CREATED_FIELD`], or
    ///   [`DEFAULT_DATE_MODIFIED_FIELD`] stops being a valid field key.
    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "DEFAULT_TITLE_FIELD/DEFAULT_ALIASES_FIELD/\
                  DEFAULT_TAGS_FIELD are hardcoded constants; failure here \
                  means a constant itself is malformed, not a recoverable \
                  caller error"
    )]
    fn default() -> Self {
        Self {
            title: FieldName::try_from(DEFAULT_TITLE_FIELD)
                .expect("DEFAULT_TITLE_FIELD is a valid field key"),
            aliases: FieldName::try_from(DEFAULT_ALIASES_FIELD)
                .expect("DEFAULT_ALIASES_FIELD is a valid field key"),
            tags: FieldName::try_from(DEFAULT_TAGS_FIELD)
                .expect("DEFAULT_TAGS_FIELD is a valid field key"),
            date_created: DateFieldConfig::default_for(
                DEFAULT_DATE_CREATED_FIELD,
            ),
            date_modified: DateFieldConfig::default_for(
                DEFAULT_DATE_MODIFIED_FIELD,
            ),
        }
    }
}

impl TryFrom<RawFrontmatterConfig> for FrontmatterConfig {
    type Error = ConfigValueError;

    /// Resolves canonical metadata fields from raw Config File values.
    ///
    /// # Errors
    ///
    /// - [`InvalidFieldKey`] if a configured title, aliases, tags, creation
    ///   date, or modification date key is empty or contains `/`.
    ///
    /// [`InvalidFieldKey`]: ConfigValueError::InvalidFieldKey
    #[inline]
    fn try_from(raw: RawFrontmatterConfig) -> Result<Self, Self::Error> {
        let invalid_key =
            |source| ConfigValueError::invalid_field_key("frontmatter", source);
        Ok(Self {
            title: FieldName::try_from(
                raw.title.unwrap_or_else(|| DEFAULT_TITLE_FIELD.to_owned()),
            )
            .map_err(invalid_key)?,
            aliases: FieldName::try_from(
                raw.aliases.unwrap_or_else(|| DEFAULT_ALIASES_FIELD.to_owned()),
            )
            .map_err(invalid_key)?,
            tags: FieldName::try_from(
                raw.tags.unwrap_or_else(|| DEFAULT_TAGS_FIELD.to_owned()),
            )
            .map_err(invalid_key)?,
            date_created: DateFieldConfig::from_raw_or_default(
                raw.date_created,
                DEFAULT_DATE_CREATED_FIELD,
            )
            .map_err(invalid_key)?,
            date_modified: DateFieldConfig::from_raw_or_default(
                raw.date_modified,
                DEFAULT_DATE_MODIFIED_FIELD,
            )
            .map_err(invalid_key)?,
        })
    }
}

/// Holds a frontmatter date key and its format string.
#[derive(Clone, Debug)]
pub struct DateFieldConfig {
    name: FieldName,
    format: String,
}

impl DateFieldConfig {
    /// Returns the frontmatter date key name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the date format used for this field's value.
    #[inline]
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Resolves a date field from raw values or role-specific defaults.
    ///
    /// # Errors
    ///
    /// - [`Empty`] if the configured name is empty or whitespace-only.
    /// - [`ContainsSlash`] if the configured name contains `/`.
    ///
    /// [`Empty`]: FieldNameError::Empty
    /// [`ContainsSlash`]: FieldNameError::ContainsSlash
    ///
    /// # Panics
    ///
    /// - `default_name` is invalid when `raw` is absent.
    #[inline]
    fn from_raw_or_default(
        raw: Option<RawDateFieldConfig>,
        default_name: &str,
    ) -> Result<Self, FieldNameError> {
        Ok(match raw {
            None => Self::default_for(default_name),
            Some(raw) => Self {
                name: FieldName::try_from(
                    raw.name.unwrap_or_else(|| default_name.to_owned()),
                )?,
                format: raw.format.unwrap_or_else(|| {
                    crate::DEFAULT_DATETIME_PATTERN.to_owned()
                }),
            },
        })
    }

    /// Digests the date key name and format under `labels`.
    fn digests(&self, labels: &DateLabels) -> DateFieldDigests {
        let Self {
            name,
            format,
        } = self;
        DateFieldDigests {
            name: setting(labels.name, |fp| name.feed_into(fp)),
            format: setting(labels.format, |fp| fp.bytes(format.as_bytes())),
        }
    }

    /// Builds a date field from a hardcoded role name and default format.
    ///
    /// # Panics
    ///
    /// - `name` is not a valid field key; callers pass only
    ///   [`DEFAULT_DATE_CREATED_FIELD`] or [`DEFAULT_DATE_MODIFIED_FIELD`].
    #[inline]
    #[expect(
        clippy::expect_used,
        reason = "callers only pass hardcoded role-name constants; failure \
                  here means a constant itself is malformed, not a \
                  recoverable caller error"
    )]
    fn default_for(name: &str) -> Self {
        Self {
            name: FieldName::try_from(name)
                .expect("role-name constant is a valid field key"),
            format: crate::DEFAULT_DATETIME_PATTERN.to_owned(),
        }
    }
}

const FRONTMATTER_TITLE: &[u8] = b"frontmatter.title";
const FRONTMATTER_ALIASES: &[u8] = b"frontmatter.aliases";
const FRONTMATTER_TAGS: &[u8] = b"frontmatter.tags";

/// Labels the two settings of a [`DateFieldConfig`] for one metadata role.
struct DateLabels {
    name: &'static [u8],
    format: &'static [u8],
}

const DATE_CREATED: DateLabels = DateLabels {
    name: b"frontmatter.date_created.name",
    format: b"frontmatter.date_created.format",
};

const DATE_MODIFIED: DateLabels = DateLabels {
    name: b"frontmatter.date_modified.name",
    format: b"frontmatter.date_modified.format",
};

/// Holds the digests of [`FrontmatterConfig`] settings.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrontmatterDigests {
    pub(crate) title: Blake3Digest,
    pub(crate) aliases: Blake3Digest,
    pub(crate) tags: Blake3Digest,
    pub(crate) date_created: DateFieldDigests,
    pub(crate) date_modified: DateFieldDigests,
}

impl FrontmatterDigests {
    /// Feeds every setting digest in declaration order.
    pub(super) fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            title,
            aliases,
            tags,
            date_created,
            date_modified,
        } = self;
        fp.digest(title);
        fp.digest(aliases);
        fp.digest(tags);
        date_created.feed_into(fp);
        date_modified.feed_into(fp);
    }
}

/// Holds the digests of one [`DateFieldConfig`].
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct DateFieldDigests {
    pub(crate) name: Blake3Digest,
    pub(crate) format: Blake3Digest,
}

impl DateFieldDigests {
    /// Feeds both setting digests in declaration order.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            name,
            format,
        } = self;
        fp.digest(name);
        fp.digest(format);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod frontmatter_for_test {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn sets_the_expected_title_and_aliases() {
            let config = FrontmatterConfig::for_test("heading", "also_known");

            assert_eq!(config.title_name(), "heading");
            assert_eq!(config.aliases_name(), "also_known");
        }

        #[test]
        fn defaults_tags_to_tags() {
            let config = FrontmatterConfig::for_test("heading", "also_known");

            assert_eq!(config.tags_name(), "tags");
        }
    }

    mod digests {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        fn ordered(digests: &FrontmatterDigests) -> [Blake3Digest; 7] {
            let FrontmatterDigests {
                title,
                aliases,
                tags,
                date_created,
                date_modified,
            } = digests;
            [
                *title,
                *aliases,
                *tags,
                date_created.name,
                date_created.format,
                date_modified.name,
                date_modified.format,
            ]
        }

        fn field(name: &str) -> FieldName {
            FieldName::try_from(name).expect("valid field name")
        }

        #[rstest]
        #[case::title(|c: &mut FrontmatterConfig| c.title = field("heading"), 0)]
        #[case::aliases(|c: &mut FrontmatterConfig| c.aliases = field("akas"), 1)]
        #[case::tags(|c: &mut FrontmatterConfig| c.tags = field("labels"), 2)]
        #[case::created_name(|c: &mut FrontmatterConfig| c.date_created.name = field("made"), 3)]
        #[case::created_format(|c: &mut FrontmatterConfig| c.date_created.format = "%Y".to_owned(), 4)]
        #[case::modified_name(|c: &mut FrontmatterConfig| c.date_modified.name = field("edited"), 5)]
        #[case::modified_format(|c: &mut FrontmatterConfig| c.date_modified.format = "%Y".to_owned(), 6)]
        fn changes_only_the_digest_of_the_changed_setting(
            #[case] mutate: fn(&mut FrontmatterConfig),
            #[case] expected: usize,
        ) {
            let before = ordered(&FrontmatterConfig::default().digests());
            let mut changed = FrontmatterConfig::default();
            mutate(&mut changed);
            let after = ordered(&changed.digests());

            let differing: Vec<usize> = before
                .iter()
                .zip(after.iter())
                .enumerate()
                .filter(|(_, (old, new))| old != new)
                .map(|(index, _)| index)
                .collect();

            assert_eq!(differing, [expected]);
        }
    }
}
