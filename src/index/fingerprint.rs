//! Fingerprints the config settings that shape persisted index rows.
//!
//! The store records an [`IndexFingerprint`] beside the rows it protects. A
//! refresh compares it with the fingerprint of the running config; a mismatch
//! is config drift, healed by a full rebuild.
//!
//! The fingerprint is deliberately wider than what index-time code reads today:
//! `title`, `aliases`, and the date keys are included on purpose. Matching too
//! much costs one harmless rebuild per edit, while matching too little serves
//! stale rows.
//!
//! # Exhaustive destructuring
//!
//! [`IndexFingerprint::new`] destructures every config digest bundle without
//! `..`. Adding a config setting then fails to compile until the setting is
//! either fingerprinted or explicitly ignored here. A source-scanning test pins
//! the convention because the compiler cannot tell a rest pattern from an
//! exhaustive one.
//!
//! # Key Types
//!
//! - [`IndexFingerprint`]: one digest per index-relevant setting.
//! - [`IndexSetting`]: names one such setting for diagnosis.
//! - [`StoredFingerprint`]: what a store holds, before comparison.
//! - [`DriftCause`]: why a store no longer matches the running config.

use std::fmt::{self, Display, Formatter};

use crate::{
    Blake3Digest,
    config::{
        ConfigDigests, DateFieldDigests, FrontmatterDigests, SchemaDigests,
        TaskDigests,
    },
};

/// Counts the settings that enter the fingerprint.
pub(super) const SETTING_COUNT: usize = 10;

/// Names one config setting that shapes persisted index rows.
///
/// Variants are named after their keys in the config file, and [`Display`] adds
/// the section. The type serves diagnosis only: config never learns of it.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum IndexSetting {
    /// The `[frontmatter] title` key.
    Title,
    /// The `[frontmatter] aliases` key.
    Aliases,
    /// The `[frontmatter] tags` key.
    Tags,
    /// The `[frontmatter.date_created] name` key.
    DateCreatedName,
    /// The `[frontmatter.date_created] format` pattern.
    DateCreatedFormat,
    /// The `[frontmatter.date_modified] name` key.
    DateModifiedName,
    /// The `[frontmatter.date_modified] format` pattern.
    DateModifiedFormat,
    /// The `[tasks] statuses` table.
    Statuses,
    /// The `[tasks] tag_filters` list.
    TagFilters,
    /// The `[schemas] class_field` key.
    ClassField,
}

impl IndexSetting {
    /// Lists every setting in fingerprint order.
    pub(super) const ALL: [Self; SETTING_COUNT] = [
        Self::Title,
        Self::Aliases,
        Self::Tags,
        Self::DateCreatedName,
        Self::DateCreatedFormat,
        Self::DateModifiedName,
        Self::DateModifiedFormat,
        Self::Statuses,
        Self::TagFilters,
        Self::ClassField,
    ];

    /// Returns the section-qualified key, such as `[frontmatter] tags`.
    const fn qualified(self) -> &'static str {
        match self {
            Self::Title => "[frontmatter] title",
            Self::Aliases => "[frontmatter] aliases",
            Self::Tags => "[frontmatter] tags",
            Self::DateCreatedName => "[frontmatter.date_created] name",
            Self::DateCreatedFormat => "[frontmatter.date_created] format",
            Self::DateModifiedName => "[frontmatter.date_modified] name",
            Self::DateModifiedFormat => "[frontmatter.date_modified] format",
            Self::Statuses => "[tasks] statuses",
            Self::TagFilters => "[tasks] tag_filters",
            Self::ClassField => "[schemas] class_field",
        }
    }
}

impl Display for IndexSetting {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.qualified())
    }
}

/// Explains why a stored index no longer matches the running config.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriftCause {
    /// These settings differ from the recorded fingerprint, in fingerprint
    /// order.
    Settings(Box<[IndexSetting]>),
    /// The store holds rows but no readable fingerprint, so no setting can be
    /// blamed.
    Unrecorded,
}

impl Display for DriftCause {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Settings(settings) => {
                for (position, setting) in settings.iter().enumerate() {
                    if position > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{setting}")?;
                }
                Ok(())
            }
            Self::Unrecorded => f.write_str("no recorded fingerprint"),
        }
    }
}

/// Holds one framed digest per index-relevant setting, in [`IndexSetting`]
/// order.
///
/// The persisted form is the digests' raw bytes concatenated. Root, template
/// directories, and the schema directory are excluded, so moving a project does
/// not rebuild its index.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) struct IndexFingerprint([Blake3Digest; SETTING_COUNT]);

/// Holds what a store records about the fingerprint of its rows.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum StoredFingerprint {
    /// The store has never held an index, so there is nothing to diff.
    Virgin,
    /// The store holds rows but no readable fingerprint beside them.
    Unreadable,
    /// The fingerprint recorded with the rows.
    Recorded(Box<IndexFingerprint>),
}

impl IndexFingerprint {
    /// Selects the index-relevant digests of `digests`.
    pub(super) fn new(digests: &ConfigDigests) -> Self {
        let ConfigDigests {
            root: _,
            templates: _,
            schemas:
                SchemaDigests {
                    directory: _,
                    class_field,
                },
            frontmatter:
                FrontmatterDigests {
                    title,
                    aliases,
                    tags,
                    date_created:
                        DateFieldDigests {
                            name: date_created_name,
                            format: date_created_format,
                        },
                    date_modified:
                        DateFieldDigests {
                            name: date_modified_name,
                            format: date_modified_format,
                        },
                },
            tasks:
                TaskDigests {
                    statuses,
                    tag_filters,
                },
        } = *digests;
        Self([
            title,
            aliases,
            tags,
            date_created_name,
            date_created_format,
            date_modified_name,
            date_modified_format,
            statuses,
            tag_filters,
            class_field,
        ])
    }

    /// Returns the digests' raw bytes, in [`IndexSetting`] order.
    pub(super) fn encode(&self) -> [[u8; Blake3Digest::LEN]; SETTING_COUNT] {
        self.0.map(Blake3Digest::to_bytes)
    }

    /// Decodes bytes written from [`Self::encode`], or `None` if they are not
    /// exactly one digest per setting.
    pub(super) fn decode(bytes: &[u8]) -> Option<Self> {
        let (digests, rest) = bytes.as_chunks::<{ Blake3Digest::LEN }>();
        if !rest.is_empty() {
            return None;
        }
        let digests: &[[u8; Blake3Digest::LEN]; SETTING_COUNT] =
            digests.try_into().ok()?;
        Some(Self((*digests).map(Blake3Digest::from_bytes)))
    }

    /// Explains why `stored` no longer protects the running config.
    ///
    /// Returns `None` when nothing drifted. A virgin store has no rows to
    /// protect, so it never drifts. Rows without a readable fingerprint drift
    /// with [`DriftCause::Unrecorded`]: with nothing to diff, failing closed is
    /// the only safe answer, and no setting can honestly be blamed.
    pub(super) fn drift(
        &self,
        stored: &StoredFingerprint,
    ) -> Option<DriftCause> {
        let recorded = match stored {
            StoredFingerprint::Virgin => return None,
            StoredFingerprint::Unreadable => {
                return Some(DriftCause::Unrecorded);
            }
            StoredFingerprint::Recorded(recorded) => recorded,
        };
        let changed: Box<[IndexSetting]> = IndexSetting::ALL
            .into_iter()
            .zip(self.0.iter().zip(&recorded.0))
            .filter(|(_, (expected, recorded))| expected != recorded)
            .map(|(setting, _)| setting)
            .collect();
        (!changed.is_empty()).then_some(DriftCause::Settings(changed))
    }

    /// Builds the fingerprint of the default test configuration.
    #[cfg(test)]
    pub(super) fn for_test() -> Self {
        Self::new(&crate::Config::test_default("/project").digests())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the fingerprint of a copy of the default test config digests
    /// after `edit`.
    fn fingerprint_after(
        edit: impl FnOnce(&mut ConfigDigests),
    ) -> IndexFingerprint {
        let mut digests = crate::Config::test_default("/project").digests();
        edit(&mut digests);
        IndexFingerprint::new(&digests)
    }

    fn changed() -> Blake3Digest {
        Blake3Digest::from("changed")
    }

    mod new {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        type Edit = fn(&mut ConfigDigests, Blake3Digest);

        #[rstest]
        #[case::title(IndexSetting::Title, |d: &mut ConfigDigests, v| d.frontmatter.title = v)]
        #[case::aliases(IndexSetting::Aliases, |d: &mut ConfigDigests, v| d.frontmatter.aliases = v)]
        #[case::tags(IndexSetting::Tags, |d: &mut ConfigDigests, v| d.frontmatter.tags = v)]
        #[case::date_created_name(IndexSetting::DateCreatedName, |d: &mut ConfigDigests, v| d.frontmatter.date_created.name = v)]
        #[case::date_created_format(IndexSetting::DateCreatedFormat, |d: &mut ConfigDigests, v| d.frontmatter.date_created.format = v)]
        #[case::date_modified_name(IndexSetting::DateModifiedName, |d: &mut ConfigDigests, v| d.frontmatter.date_modified.name = v)]
        #[case::date_modified_format(IndexSetting::DateModifiedFormat, |d: &mut ConfigDigests, v| d.frontmatter.date_modified.format = v)]
        #[case::statuses(IndexSetting::Statuses, |d: &mut ConfigDigests, v| d.tasks.statuses = v)]
        #[case::tag_filters(IndexSetting::TagFilters, |d: &mut ConfigDigests, v| d.tasks.tag_filters = v)]
        #[case::class_field(IndexSetting::ClassField, |d: &mut ConfigDigests, v| d.schemas.class_field = v)]
        fn tracks_each_setting_under_its_own_name(
            #[case] setting: IndexSetting,
            #[case] edit: Edit,
        ) {
            let expected = IndexFingerprint::for_test();
            let edited = fingerprint_after(|d| edit(d, changed()));

            assert_eq!(
                edited.drift(&StoredFingerprint::Recorded(Box::new(expected))),
                Some(DriftCause::Settings(Box::from([setting])))
            );
        }

        #[rstest]
        #[case::root(|d: &mut ConfigDigests, v| d.root = v)]
        #[case::schema_directory(|d: &mut ConfigDigests, v| d.schemas.directory = v)]
        fn ignores_settings_that_do_not_shape_rows(#[case] edit: Edit) {
            let edited = fingerprint_after(|d| edit(d, changed()));

            assert_eq!(edited, IndexFingerprint::for_test());
        }

        #[test]
        fn ignores_the_project_location_and_template_directories() {
            let at = |root: &str, local: &str, output: &str| {
                let config = crate::Config::for_test(
                    root.into(),
                    Some(local.into()),
                    None,
                    output.into(),
                );
                IndexFingerprint::new(&config.digests())
            };

            assert_eq!(
                at("/old", "/old/templates", "/old/out"),
                at("/moved", "/moved/other", "/moved/else")
            );
        }
    }

    mod encode {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn lays_out_raw_digests_in_setting_order() {
            let digests = crate::Config::test_default("/project").digests();
            let FrontmatterDigests {
                title,
                aliases,
                tags,
                date_created,
                date_modified,
            } = digests.frontmatter;

            let bytes = IndexFingerprint::new(&digests).encode();

            assert_eq!(
                bytes,
                [
                    title,
                    aliases,
                    tags,
                    date_created.name,
                    date_created.format,
                    date_modified.name,
                    date_modified.format,
                    digests.tasks.statuses,
                    digests.tasks.tag_filters,
                    digests.schemas.class_field,
                ]
                .map(Blake3Digest::to_bytes)
            );
        }
    }

    mod decode {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        const ENCODED_LEN: usize = SETTING_COUNT * Blake3Digest::LEN;

        #[test]
        fn returns_the_fingerprint_that_was_encoded() {
            let fingerprint = IndexFingerprint::for_test();
            let stored = fingerprint.encode();

            assert_eq!(
                IndexFingerprint::decode(stored.as_flattened()),
                Some(fingerprint)
            );
        }

        #[rstest]
        #[case::empty(0)]
        #[case::one_byte_short(ENCODED_LEN - 1)]
        #[case::one_byte_long(ENCODED_LEN + 1)]
        #[case::whole_extra_digest(ENCODED_LEN + Blake3Digest::LEN)]
        fn rejects_bytes_of_the_wrong_length(#[case] len: usize) {
            assert_eq!(IndexFingerprint::decode(&vec![0; len]), None);
        }
    }

    mod drift {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn reports_nothing_for_a_virgin_store() {
            let drift =
                IndexFingerprint::for_test().drift(&StoredFingerprint::Virgin);

            assert_eq!(drift, None);
        }

        #[test]
        fn reports_nothing_when_the_recorded_fingerprint_matches() {
            let fingerprint = IndexFingerprint::for_test();

            let drift = fingerprint
                .drift(&StoredFingerprint::Recorded(Box::new(fingerprint)));

            assert_eq!(drift, None);
        }

        #[test]
        fn reports_an_unrecorded_fingerprint_without_naming_settings() {
            let drift = IndexFingerprint::for_test()
                .drift(&StoredFingerprint::Unreadable);

            assert_eq!(drift, Some(DriftCause::Unrecorded));
        }

        #[test]
        fn lists_changed_settings_in_fingerprint_order() {
            let recorded = fingerprint_after(|d| {
                d.schemas.class_field = changed();
                d.frontmatter.tags = changed();
            });

            let drift = IndexFingerprint::for_test()
                .drift(&StoredFingerprint::Recorded(Box::new(recorded)));

            assert_eq!(
                drift,
                Some(DriftCause::Settings(Box::from([
                    IndexSetting::Tags,
                    IndexSetting::ClassField
                ])))
            );
        }
    }

    mod display {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[rstest]
        #[case::title(IndexSetting::Title, "[frontmatter] title")]
        #[case::aliases(IndexSetting::Aliases, "[frontmatter] aliases")]
        #[case::tags(IndexSetting::Tags, "[frontmatter] tags")]
        #[case::date_created_name(
            IndexSetting::DateCreatedName,
            "[frontmatter.date_created] name"
        )]
        #[case::date_created_format(
            IndexSetting::DateCreatedFormat,
            "[frontmatter.date_created] format"
        )]
        #[case::date_modified_name(
            IndexSetting::DateModifiedName,
            "[frontmatter.date_modified] name"
        )]
        #[case::date_modified_format(
            IndexSetting::DateModifiedFormat,
            "[frontmatter.date_modified] format"
        )]
        #[case::statuses(IndexSetting::Statuses, "[tasks] statuses")]
        #[case::tag_filters(IndexSetting::TagFilters, "[tasks] tag_filters")]
        #[case::class_field(IndexSetting::ClassField, "[schemas] class_field")]
        fn qualifies_the_key_with_its_section(
            #[case] setting: IndexSetting,
            #[case] expected: &str,
        ) {
            assert_eq!(setting.to_string(), expected);
        }

        #[rstest]
        #[case::one(
            DriftCause::Settings(Box::from([IndexSetting::Tags])),
            "[frontmatter] tags"
        )]
        #[case::several(
            DriftCause::Settings(Box::from([
                IndexSetting::Tags,
                IndexSetting::ClassField
            ])),
            "[frontmatter] tags, [schemas] class_field"
        )]
        #[case::unrecorded(DriftCause::Unrecorded, "no recorded fingerprint")]
        fn joins_the_settings_of_a_drift_cause(
            #[case] cause: DriftCause,
            #[case] expected: &str,
        ) {
            assert_eq!(cause.to_string(), expected);
        }
    }

    mod exhaustive_destructuring {
        use pretty_assertions::assert_eq;
        use syn::{
            ImplItemFn, PatRest,
            visit::{self, Visit},
        };

        /// Collects rest patterns inside `IndexFingerprint::new`.
        #[derive(Default)]
        struct Scan {
            in_constructor: bool,
            rest_patterns: usize,
        }

        impl<'ast> Visit<'ast> for Scan {
            fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
                self.in_constructor = node.sig.ident == "new";
                visit::visit_impl_item_fn(self, node);
                self.in_constructor = false;
            }

            fn visit_pat_rest(&mut self, _: &'ast PatRest) {
                if self.in_constructor {
                    self.rest_patterns = self.rest_patterns.saturating_add(1);
                }
            }
        }

        fn rest_patterns(source: &str) -> usize {
            let file = syn::parse_file(source).expect("source parses");
            let mut scan = Scan::default();
            scan.visit_file(&file);
            scan.rest_patterns
        }

        #[test]
        fn flags_a_rest_pattern_in_a_constructor() {
            let source =
                "impl Fingerprint { fn new(d: &D) { let D { a, .. } = d; } }";

            assert_eq!(rest_patterns(source), 1);
        }

        #[test]
        fn finds_no_rest_pattern_in_the_fingerprint_constructor() {
            assert_eq!(rest_patterns(include_str!("fingerprint.rs")), 0);
        }
    }
}
