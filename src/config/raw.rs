//! Parses Config File TOML into strict Serde types.
//!
//! [`RawConfig`] holds configured values before
//! [`super::builder::ConfigBuilder`] applies Configuration Scope precedence.
//! The builder also resolves paths.

mod table;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Holds Config File values before merge and path resolution.
///
/// Local and global Config Files use these tables. Serde rejects unknown keys.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConfig {
    /// Holds the `[templates]` table.
    #[serde(default, deserialize_with = "table::table")]
    pub(crate) templates: RawTemplateConfig,
    /// Holds the `[schemas]` table.
    #[serde(default, deserialize_with = "table::table")]
    pub(crate) schemas: RawSchemasConfig,
    /// Holds the `[frontmatter]` table.
    #[serde(default, deserialize_with = "table::table")]
    pub(crate) frontmatter: RawFrontmatterConfig,
    /// Holds the `[tasks]` table for Task Configuration.
    #[serde(default, deserialize_with = "table::table")]
    pub(crate) tasks: RawTaskConfig,
}

/// Holds the `[templates]` table before the builder merges values.
///
/// Serialization omits absent fields when it creates a Config File scaffold.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTemplateConfig {
    /// Names the Template Directory before resolution against the Config File
    /// root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) directory: Option<PathBuf>,
    /// Names the directory for rendered templates.
    ///
    /// [`super::builder::ConfigBuilder`] resolves relative values against the
    /// project root. It uses that root when this field is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) output_dir: Option<PathBuf>,
}

/// Holds the `[schemas]` table before the builder merges values.
///
/// Serialization omits absent fields when it creates a Config File scaffold.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSchemasConfig {
    /// Names the frontmatter key for a Note's File Class before the builder
    /// applies the `class` default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) class_field: Option<String>,
    /// Names the schema directory before the builder applies the
    /// `.traces/schemas/` default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) directory: Option<PathBuf>,
}

/// Holds the `[frontmatter]` table before the builder merges values.
///
/// Serialization omits absent fields when it creates a Config File scaffold.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawFrontmatterConfig {
    /// Names the frontmatter key for a Note's display title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    /// Names the frontmatter key for a Note's aliases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) aliases: Option<String>,
    /// Names the frontmatter key for a Note's tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tags: Option<String>,
    /// Holds the creation timestamp's frontmatter key and date format.
    #[serde(
        default,
        deserialize_with = "table::opt_table",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) date_created: Option<RawDateFieldConfig>,
    /// Holds the modification timestamp's frontmatter key and date format.
    #[serde(
        default,
        deserialize_with = "table::opt_table",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) date_modified: Option<RawDateFieldConfig>,
}

/// Holds an optional frontmatter key and format for a date field.
///
/// [`super::model::DateFieldConfig`] supplies role-specific defaults when
/// values are missing. Serialization omits absent fields when it creates a
/// Config File scaffold.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDateFieldConfig {
    /// Names the frontmatter date key when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    /// Specifies the date format for the key's value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) format: Option<String>,
}

/// Holds the `[tasks]` table before the builder merges values.
///
/// Serialization omits empty fields when it creates a Config File scaffold.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTaskConfig {
    /// Lists tags that mark status-marked items as Tasks.
    ///
    /// An entry can omit `#`. [`super::builder::ConfigBuilder`] adds it before
    /// it creates a [`crate::Tag`]. An empty list uses global filters. If no
    /// global filters exist, every status-marked item is a Task.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) tag_filters: Vec<String>,
    /// Lists task statuses that add to or replace defaults with the same
    /// symbol.
    #[serde(
        default,
        deserialize_with = "table::table_list",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub(crate) statuses: Vec<RawTaskStatus>,
}

/// Holds a `[[tasks.statuses]]` entry from a Config File.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTaskStatus {
    /// Identifies the status marker inside `[<char>]`.
    pub(crate) symbol: char,
    /// Names the status for display.
    pub(crate) name: String,
    /// Classifies the status in the task workflow.
    pub(crate) kind: RawTaskStatusKind,
}

/// Classifies a task status in `[[tasks.statuses]]`.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RawTaskStatusKind {
    /// Marks work that has not started.
    Todo,
    /// Marks work in progress.
    InProgress,
    /// Marks work paused for an external dependency.
    OnHold,
    /// Marks completed work.
    Done,
    /// Marks abandoned work.
    Cancelled,
    /// Excludes a status-marked item from Task recognition.
    NonTask,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::RawConfig;

    #[rstest]
    #[case::unknown_schema_key("[schemas]\nbogus = 1")]
    #[case::unknown_frontmatter_key("[frontmatter]\nbogus = 1")]
    #[case::unknown_date_field_key(
        "[frontmatter.date_created]\nname = \"created\"\nbogus = 1"
    )]
    fn rejects_unknown_fields(#[case] content: &str) {
        let result = toml::from_str::<RawConfig>(content);

        let error = result.expect_err("unknown field should fail");
        assert!(error.to_string().contains("bogus"));
    }

    #[rstest]
    #[case::templates("templates = [\"a\", \"b\"]")]
    #[case::schemas("schemas = [\"a\", \"b\"]")]
    #[case::frontmatter("frontmatter = [\"a\", \"b\", \"c\", \"d\", \"e\"]")]
    #[case::tasks("tasks = [[\"a\"], []]")]
    #[case::date_created(
        "[frontmatter]\ndate_created = [\"name\", \"format\"]"
    )]
    #[case::date_modified(
        "[frontmatter]\ndate_modified = [\"name\", \"format\"]"
    )]
    #[case::task_status("[tasks]\nstatuses = [[\"x\", \"Name\", \"todo\"]]")]
    fn rejects_an_array_where_a_table_belongs(#[case] content: &str) {
        let result = toml::from_str::<RawConfig>(content);

        let error = result.expect_err("array section should fail");
        assert!(
            error.message().contains("expected struct"),
            "unexpected message: {}",
            error.message()
        );
        assert!(error.span().is_some());
    }

    mod proptests {
        use std::path::PathBuf;

        use proptest::prelude::*;

        use super::super::{
            RawConfig, RawDateFieldConfig, RawFrontmatterConfig,
            RawSchemasConfig, RawTaskConfig, RawTaskStatus, RawTaskStatusKind,
            RawTemplateConfig,
        };

        /// Arbitrary text, including control characters and non-ASCII
        /// letters, over a bounded length so shrinking stays readable.
        fn text() -> impl Strategy<Value = String> {
            "\\PC{0,32}"
        }

        fn optional_text() -> impl Strategy<Value = Option<String>> {
            prop::option::of(text())
        }

        fn optional_path() -> impl Strategy<Value = Option<PathBuf>> {
            prop::option::of(any::<String>().prop_map(PathBuf::from))
        }

        fn optional_date_field()
        -> impl Strategy<Value = Option<RawDateFieldConfig>> {
            prop::option::of((optional_text(), optional_text()).prop_map(
                |(name, format)| RawDateFieldConfig {
                    name,
                    format,
                },
            ))
        }

        fn task_status() -> impl Strategy<Value = RawTaskStatus> {
            (
                any::<char>(),
                text(),
                prop::sample::select(vec![
                    RawTaskStatusKind::Todo,
                    RawTaskStatusKind::InProgress,
                    RawTaskStatusKind::OnHold,
                    RawTaskStatusKind::Done,
                    RawTaskStatusKind::Cancelled,
                    RawTaskStatusKind::NonTask,
                ]),
            )
                .prop_map(|(symbol, name, kind)| RawTaskStatus {
                    symbol,
                    name,
                    kind,
                })
        }

        fn raw_config() -> impl Strategy<Value = RawConfig> {
            (
                optional_path(),
                optional_path(),
                optional_text(),
                optional_path(),
                optional_text(),
                optional_text(),
                optional_text(),
                optional_date_field(),
                optional_date_field(),
                prop::collection::vec(text(), 0..=3),
                prop::collection::vec(task_status(), 0..=3),
            )
                .prop_map(
                    |(
                        template_directory,
                        template_output_dir,
                        class_field,
                        schema_directory,
                        title,
                        aliases,
                        tags,
                        date_created,
                        date_modified,
                        tag_filters,
                        statuses,
                    )| {
                        RawConfig {
                            templates: RawTemplateConfig {
                                directory: template_directory,
                                output_dir: template_output_dir,
                            },
                            schemas: RawSchemasConfig {
                                class_field,
                                directory: schema_directory,
                            },
                            frontmatter: RawFrontmatterConfig {
                                title,
                                aliases,
                                tags,
                                date_created,
                                date_modified,
                            },
                            tasks: RawTaskConfig {
                                tag_filters,
                                statuses,
                            },
                        }
                    },
                )
        }

        proptest! {
            /// Pins the Config File scaffold law: every generated
            /// configuration survives a serialise-then-parse round trip
            /// unchanged, so scaffolded files never lose configured values.
            #[test]
            fn toml_serialization_round_trips(config in raw_config()) {
                // Act
                let written = toml::to_string(&config)
                    .expect("RawConfig always serialises");
                let reparsed = toml::from_str::<RawConfig>(&written)
                    .expect("scaffolded Config File TOML must re-parse");

                // Assert
                prop_assert_eq!(reparsed, config);
            }
        }
    }
}
