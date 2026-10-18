//! Encodes resolved settings as per-section digests.
//!
//! Every setting digests on its own under a section-qualified label, so a
//! stored digest says which setting it covers and cannot collide with a
//! sibling. [`Config::hash`] composes all of them under one version label.
//!
//! # Exhaustive destructuring
//!
//! Each `digests` constructor and each bundle's `feed_into` destructures its
//! type without `..`. Adding a setting to a config type then fails to compile
//! here until the setting gets a label and a place in the encoding. A
//! source-scanning test pins the convention because the compiler cannot tell
//! a rest pattern from an exhaustive one.
//!
//! # Key Types
//!
//! - [`ConfigDigests`]: one digest per setting, grouped by section.

use super::{
    Config, DateFieldConfig, FrontmatterConfig, SchemasConfig, TaskConfig,
    TemplateConfig,
};
use crate::{Blake3Digest, TaskStatus, hash::Blake3Hasher};

/// Versions the composed Config Fingerprint encoding.
const CONFIG_LABEL: &[u8] = b"traces.config.v1";

const ROOT: &[u8] = b"root";
const TEMPLATES_LOCAL: &[u8] = b"templates.local";
const TEMPLATES_GLOBAL: &[u8] = b"templates.global";
const TEMPLATES_OUTPUT: &[u8] = b"templates.output";
const SCHEMAS_DIRECTORY: &[u8] = b"schemas.directory";
const SCHEMAS_CLASS_FIELD: &[u8] = b"schemas.class_field";
const FRONTMATTER_TITLE: &[u8] = b"frontmatter.title";
const FRONTMATTER_ALIASES: &[u8] = b"frontmatter.aliases";
const FRONTMATTER_TAGS: &[u8] = b"frontmatter.tags";
const TASKS_STATUSES: &[u8] = b"tasks.statuses";
const TASKS_TAG_FILTERS: &[u8] = b"tasks.tag_filters";

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

/// Holds one framed digest per resolved setting, grouped by config section.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ConfigDigests {
    pub(crate) root: Blake3Digest,
    pub(crate) templates: TemplateDigests,
    pub(crate) schemas: SchemaDigests,
    pub(crate) frontmatter: FrontmatterDigests,
    pub(crate) tasks: TaskDigests,
}

/// Holds the digests of [`TemplateConfig`] settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TemplateDigests {
    pub(crate) local: Blake3Digest,
    pub(crate) global: Blake3Digest,
    pub(crate) output: Blake3Digest,
}

/// Holds the digests of [`SchemasConfig`] settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SchemaDigests {
    pub(crate) directory: Blake3Digest,
    pub(crate) class_field: Blake3Digest,
}

/// Holds the digests of [`FrontmatterConfig`] settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FrontmatterDigests {
    pub(crate) title: Blake3Digest,
    pub(crate) aliases: Blake3Digest,
    pub(crate) tags: Blake3Digest,
    pub(crate) date_created: DateFieldDigests,
    pub(crate) date_modified: DateFieldDigests,
}

/// Holds the digests of one [`DateFieldConfig`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DateFieldDigests {
    pub(crate) name: Blake3Digest,
    pub(crate) format: Blake3Digest,
}

/// Holds the digests of [`TaskConfig`] settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TaskDigests {
    pub(crate) statuses: Blake3Digest,
    pub(crate) tag_filters: Blake3Digest,
}

/// Digests one setting: the label's length-prefixed bytes, then the value
/// bytes `feed` adds.
///
/// The label frame lives here rather than in [`Blake3Hasher`], whose value
/// methods stay label-agnostic.
fn setting<F>(label: &[u8], feed: F) -> Blake3Digest
where
    F: FnOnce(&mut Blake3Hasher),
{
    let mut fp = Blake3Hasher::new();
    fp.bytes(label);
    feed(&mut fp);
    fp.finish()
}

impl ConfigDigests {
    /// Composes every setting digest into one digest under
    /// `b"traces.config.v1"`.
    ///
    /// Settings enter in declaration order, so the result is the Config
    /// Fingerprint. Changing the order, the labels, or the version label
    /// invalidates every persisted fingerprint.
    pub(crate) fn compose(&self) -> Blake3Digest {
        let Self {
            root,
            templates,
            schemas,
            frontmatter,
            tasks,
        } = self;
        let mut fp = Blake3Hasher::new();
        fp.bytes(CONFIG_LABEL);
        fp.digest(root);
        templates.feed_into(&mut fp);
        schemas.feed_into(&mut fp);
        frontmatter.feed_into(&mut fp);
        tasks.feed_into(&mut fp);
        fp.finish()
    }
}

impl TemplateDigests {
    /// Feeds every setting digest in declaration order.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            local,
            global,
            output,
        } = self;
        fp.digest(local);
        fp.digest(global);
        fp.digest(output);
    }
}

impl SchemaDigests {
    /// Feeds every setting digest in declaration order.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            directory,
            class_field,
        } = self;
        fp.digest(directory);
        fp.digest(class_field);
    }
}

impl FrontmatterDigests {
    /// Feeds every setting digest in declaration order.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
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

impl TaskDigests {
    /// Feeds every setting digest in declaration order.
    fn feed_into(&self, fp: &mut Blake3Hasher) {
        let Self {
            statuses,
            tag_filters,
        } = self;
        fp.digest(statuses);
        fp.digest(tag_filters);
    }
}

impl Config {
    /// Digests every resolved setting separately.
    ///
    /// Each digest starts with its section-qualified label, so a stored
    /// digest names the setting it covers. [`ConfigDigests::compose`] folds
    /// them into the Config Fingerprint.
    #[inline]
    #[must_use]
    pub(crate) fn digests(&self) -> ConfigDigests {
        let Self {
            root,
            templates,
            schemas,
            frontmatter,
            tasks,
        } = self;
        ConfigDigests {
            root: setting(ROOT, |fp| fp.path(root)),
            templates: templates.digests(),
            schemas: schemas.digests(),
            frontmatter: frontmatter.digests(),
            tasks: tasks.digests(),
        }
    }
}

impl TemplateConfig {
    /// Digests the Template Directories and the output directory.
    fn digests(&self) -> TemplateDigests {
        let Self {
            local,
            global,
            output,
        } = self;
        TemplateDigests {
            local: setting(TEMPLATES_LOCAL, |fp| {
                fp.optional_path(local.as_deref());
            }),
            global: setting(TEMPLATES_GLOBAL, |fp| {
                fp.optional_path(global.as_deref());
            }),
            output: setting(TEMPLATES_OUTPUT, |fp| fp.path(output)),
        }
    }
}

impl SchemasConfig {
    /// Digests the schema registry directory and the class field.
    fn digests(&self) -> SchemaDigests {
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

impl FrontmatterConfig {
    /// Digests the metadata role names and date formats.
    fn digests(&self) -> FrontmatterDigests {
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
}

impl DateFieldConfig {
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
}

impl TaskConfig {
    /// Digests the task statuses and tag filters.
    ///
    /// Statuses enter in symbol order so map iteration order never leaks;
    /// filters enter in configured order.
    fn digests(&self) -> TaskDigests {
        let Self {
            statuses,
            tag_filters,
        } = self;
        TaskDigests {
            statuses: setting(TASKS_STATUSES, |fp| {
                let mut ordered: Vec<&TaskStatus> = statuses.iter().collect();
                ordered
                    .sort_unstable_by_key(|status| status.symbol().as_char());
                fp.len(ordered.len());
                for status in ordered {
                    let mut symbol = [0_u8; 4];
                    fp.bytes(
                        status
                            .symbol()
                            .as_char()
                            .encode_utf8(&mut symbol)
                            .as_bytes(),
                    );
                    fp.bytes(status.name().as_bytes());
                    fp.bytes(status.kind().as_str().as_bytes());
                }
            }),
            tag_filters: setting(TASKS_TAG_FILTERS, |fp| {
                fp.len(tag_filters.len());
                for tag in tag_filters {
                    fp.bytes(tag.as_str().as_bytes());
                }
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    mod fixtures {
        use std::path::PathBuf;

        use super::super::*;

        pub(in super::super) const NAMES: [&str; 15] = [
            "root",
            "templates.local",
            "templates.global",
            "templates.output",
            "schemas.directory",
            "schemas.class_field",
            "frontmatter.title",
            "frontmatter.aliases",
            "frontmatter.tags",
            "frontmatter.date_created.name",
            "frontmatter.date_created.format",
            "frontmatter.date_modified.name",
            "frontmatter.date_modified.format",
            "tasks.statuses",
            "tasks.tag_filters",
        ];

        pub(in super::super) fn base() -> Config {
            Config::for_test(
                PathBuf::from("/project"),
                Some(PathBuf::from("/project/local")),
                Some(PathBuf::from("/global")),
                PathBuf::from("/project/out"),
            )
        }

        /// Lists every setting digest in [`NAMES`] order.
        pub(in super::super) fn ordered(
            digests: &ConfigDigests,
        ) -> [Blake3Digest; 15] {
            let ConfigDigests {
                root,
                templates:
                    TemplateDigests {
                        local,
                        global,
                        output,
                    },
                schemas:
                    SchemaDigests {
                        directory,
                        class_field,
                    },
                frontmatter:
                    FrontmatterDigests {
                        title,
                        aliases,
                        tags,
                        date_created,
                        date_modified,
                    },
                tasks:
                    TaskDigests {
                        statuses,
                        tag_filters,
                    },
            } = digests;
            [
                *root,
                *local,
                *global,
                *output,
                *directory,
                *class_field,
                *title,
                *aliases,
                *tags,
                date_created.name,
                date_created.format,
                date_modified.name,
                date_modified.format,
                *statuses,
                *tag_filters,
            ]
        }
    }

    mod setting {
        use pretty_assertions::assert_eq;

        use super::super::*;

        #[test]
        fn hashes_the_label_frame_before_the_value_frame() {
            let mut raw = blake3::Hasher::new();
            raw.update(&13_u64.to_le_bytes());
            raw.update(b"test.my_label");
            raw.update(&5_u64.to_le_bytes());
            raw.update(b"value");

            let digest = setting(b"test.my_label", |fp| fp.bytes(b"value"));

            assert_eq!(
                digest.to_hex().as_ref(),
                raw.finalize().to_hex().as_str()
            );
        }
    }

    mod config_digests {
        use std::{
            path::{Path, PathBuf},
            sync::Arc,
        };

        use pretty_assertions::{assert_eq, assert_ne};
        use rstest::rstest;

        use super::{
            super::*,
            fixtures::{NAMES, base, ordered},
        };
        use crate::{FieldName, TaskStatusSymbol, TaskStatusType};

        #[test]
        fn frames_a_required_path_setting_as_label_then_value() {
            let mut raw = blake3::Hasher::new();
            raw.update(&19_u64.to_le_bytes());
            raw.update(b"schemas.class_field");
            raw.update(&5_u64.to_le_bytes());
            raw.update(b"class");

            let digests = base().digests();

            assert_eq!(
                digests.schemas.class_field.to_hex().as_ref(),
                raw.finalize().to_hex().as_str()
            );
        }

        #[test]
        fn frames_a_present_optional_path_with_its_presence_tag() {
            let mut raw = blake3::Hasher::new();
            raw.update(&15_u64.to_le_bytes());
            raw.update(b"templates.local");
            raw.update(&[1]);
            raw.update(&14_u64.to_le_bytes());
            raw.update(b"/project/local");

            let digests = base().digests();

            assert_eq!(
                digests.templates.local.to_hex().as_ref(),
                raw.finalize().to_hex().as_str()
            );
        }

        #[test]
        fn composes_every_setting_digest_under_the_version_label() {
            let digests = base().digests();
            let mut raw = blake3::Hasher::new();
            raw.update(&16_u64.to_le_bytes());
            raw.update(b"traces.config.v1");
            for digest in ordered(&digests) {
                let hash = blake3::Hash::from_hex(digest.to_hex().as_ref())
                    .expect("digest hex parses");
                raw.update(&32_u64.to_le_bytes());
                raw.update(hash.as_bytes());
            }

            assert_eq!(
                digests.compose().to_hex().as_ref(),
                raw.finalize().to_hex().as_str()
            );
        }

        #[test]
        fn keeps_settings_with_equal_values_apart_by_label() {
            let mut config = base();
            config.frontmatter = FrontmatterConfig::for_test("same", "same")
                .with_tags_name("same");

            let named: Vec<_> =
                NAMES.into_iter().zip(ordered(&config.digests())).collect();

            for (i, (first_name, first)) in named.iter().enumerate() {
                for (second_name, second) in named.iter().skip(i + 1) {
                    assert_ne!(
                        first, second,
                        "{second_name} collides with {first_name}"
                    );
                }
            }
        }

        #[rstest]
        #[case::root(|c: &mut Config| c.root = Arc::from(Path::new("/other")), "root")]
        #[case::local_templates(|c: &mut Config| c.templates.local = None, "templates.local")]
        #[case::global_templates(|c: &mut Config| c.templates.global = None, "templates.global")]
        #[case::output(|c: &mut Config| c.templates.output = PathBuf::from("/o"), "templates.output")]
        #[case::schemas_directory(|c: &mut Config| c.schemas.directory = PathBuf::from("/s"), "schemas.directory")]
        #[case::class_field(|c: &mut Config| c.schemas.class_field = FieldName::try_from("kind").expect("valid"), "schemas.class_field")]
        #[case::title(|c: &mut Config| c.frontmatter.title = FieldName::try_from("heading").expect("valid"), "frontmatter.title")]
        #[case::aliases(|c: &mut Config| c.frontmatter.aliases = FieldName::try_from("akas").expect("valid"), "frontmatter.aliases")]
        #[case::tags(|c: &mut Config| c.frontmatter.tags = FieldName::try_from("labels").expect("valid"), "frontmatter.tags")]
        #[case::created_name(|c: &mut Config| c.frontmatter.date_created.name = FieldName::try_from("made").expect("valid"), "frontmatter.date_created.name")]
        #[case::created_format(|c: &mut Config| c.frontmatter.date_created.format = "%Y".to_owned(), "frontmatter.date_created.format")]
        #[case::modified_name(|c: &mut Config| c.frontmatter.date_modified.name = FieldName::try_from("edited").expect("valid"), "frontmatter.date_modified.name")]
        #[case::modified_format(|c: &mut Config| c.frontmatter.date_modified.format = "%Y".to_owned(), "frontmatter.date_modified.format")]
        #[case::added_status(|c: &mut Config| c.tasks.statuses.insert(TaskStatus::new(
            TaskStatusSymbol::new('!'),
            "Urgent",
            TaskStatusType::Todo,
        )).expect("insert status"), "tasks.statuses")]
        #[case::tag_filters(|c: &mut Config| c.tasks = TaskConfig::from_tags(&["#work"]), "tasks.tag_filters")]
        fn changes_only_the_digest_of_the_changed_setting(
            #[case] mutate: fn(&mut Config),
            #[case] expected: &str,
        ) {
            let before = ordered(&base().digests());
            let mut changed = base();
            mutate(&mut changed);
            let after = ordered(&changed.digests());

            let differing: Vec<&str> = NAMES
                .iter()
                .zip(before.iter().zip(after.iter()))
                .filter(|(_, (old, new))| old != new)
                .map(|(name, _)| *name)
                .collect();

            assert_eq!(differing, [expected]);
        }
    }

    mod exhaustive_destructuring {
        use pretty_assertions::assert_eq;
        use syn::{
            ImplItemFn, ItemImpl, PatRest, Type,
            visit::{self, Visit},
        };

        /// Collects rest patterns and `digests` constructors from a source
        /// file.
        #[derive(Default)]
        struct Scan {
            self_type: String,
            current_fn: String,
            rest_patterns: Vec<String>,
            constructors: Vec<String>,
        }

        impl<'ast> Visit<'ast> for Scan {
            fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
                if let Type::Path(path) = &*node.self_ty {
                    self.self_type = path
                        .path
                        .segments
                        .last()
                        .map(|segment| segment.ident.to_string())
                        .unwrap_or_default();
                }
                visit::visit_item_impl(self, node);
            }

            fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
                self.current_fn = node.sig.ident.to_string();
                if self.current_fn == "digests" {
                    self.constructors.push(self.self_type.clone());
                }
                visit::visit_impl_item_fn(self, node);
            }

            fn visit_pat_rest(&mut self, _: &'ast PatRest) {
                self.rest_patterns
                    .push(format!("{}::{}", self.self_type, self.current_fn));
            }
        }

        fn scan(source: &str) -> Scan {
            let file = syn::parse_file(source).expect("source parses");
            let mut scan = Scan::default();
            scan.visit_file(&file);
            scan
        }

        #[test]
        fn flags_a_rest_pattern_in_a_destructuring() {
            let source = "impl Section { fn digests(&self) { let Self { a, .. \
                          } = self; } }";

            assert_eq!(scan(source).rest_patterns, ["Section::digests"]);
        }

        #[test]
        fn finds_no_rest_pattern_in_the_digest_source() {
            let scan = scan(include_str!("digest.rs"));

            assert_eq!(scan.rest_patterns, Vec::<String>::new());
        }

        #[test]
        fn finds_a_digests_constructor_on_every_section_type() {
            let mut constructors = scan(include_str!("digest.rs")).constructors;
            constructors.sort_unstable();

            assert_eq!(constructors, [
                "Config",
                "DateFieldConfig",
                "FrontmatterConfig",
                "SchemasConfig",
                "TaskConfig",
                "TemplateConfig",
            ]);
        }
    }
}
