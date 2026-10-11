//! Deserializes Config File TOML into strict serde types.
//!
//! [`RawConfig`] retains configured values before
//! [`super::builder::ConfigBuilder`] applies Configuration Scope precedence and
//! resolves paths.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Holds Config File values before merging and path resolution.
///
/// Local and global Config Files share these raw tables and reject unknown
/// keys.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConfig {
    /// Holds the `[templates]` table.
    #[serde(default)]
    pub(crate) templates: RawTemplateConfig,
    /// Holds the `[schemas]` table.
    #[serde(default)]
    pub(crate) schemas: RawSchemasConfig,
    /// Holds the `[frontmatter]` table.
    #[serde(default)]
    pub(crate) frontmatter: RawFrontmatterConfig,
    /// Holds the `[tasks]` table for Task Configuration.
    #[serde(default)]
    pub(crate) tasks: RawTaskConfig,
}

/// Holds the unmerged `[templates]` table from a Config File.
///
/// Serialization omits absent fields when scaffolding a Config File.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTemplateConfig {
    /// Names the Template Directory before resolution against its Config File
    /// root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) directory: Option<PathBuf>,
    /// Names the rendered-template output directory.
    ///
    /// [`super::builder::ConfigBuilder`] resolves relative values against the
    /// project root and uses that root when this field is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) output_dir: Option<PathBuf>,
}

/// Holds the unmerged `[schemas]` table from a Config File.
///
/// Serialization omits absent fields when scaffolding a Config File.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSchemasConfig {
    /// Names the frontmatter key for a Note's File Class(es), before applying
    /// the `class` default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) class_field: Option<String>,
    /// Names the schema directory before applying the `.traces/schemas/`
    /// default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) directory: Option<PathBuf>,
}

/// Holds the unmerged `[frontmatter]` table from a Config File.
///
/// Serialization omits absent fields when scaffolding a Config File.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawFrontmatterConfig {
    /// Names the frontmatter key holding a Note's display title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    /// Names the frontmatter key holding a Note's aliases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) aliases: Option<String>,
    /// Names the frontmatter key holding a Note's tags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tags: Option<String>,
    /// Holds the creation timestamp's frontmatter key and date format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) date_created: Option<RawDateFieldConfig>,
    /// Holds the modification timestamp's frontmatter key and date format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) date_modified: Option<RawDateFieldConfig>,
}

/// Holds an optional frontmatter key and format for a date field.
///
/// [`super::model::DateFieldConfig`] supplies role-specific defaults for
/// missing values. Serialization omits absent fields when scaffolding a Config
/// File.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawDateFieldConfig {
    /// Names the frontmatter date key when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    /// Specifies the date format for the key's value when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) format: Option<String>,
}

/// Holds the unmerged `[tasks]` table for Task Configuration.
///
/// Serialization omits empty fields when scaffolding a Config File.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTaskConfig {
    /// Lists tags that classify status-marked items as Tasks.
    ///
    /// Entries may omit `#`; [`super::builder::ConfigBuilder`] normalizes them
    /// before constructing a [`crate::Tag`]. An empty list falls back to
    /// global filters, or accepts every status-marked item if none exist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) tag_filters: Vec<String>,
    /// Lists task statuses that augment or override defaults by symbol.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) statuses: Vec<RawTaskStatus>,
}

/// Holds a `[[tasks.statuses]]` entry from a Config File.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
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
}
