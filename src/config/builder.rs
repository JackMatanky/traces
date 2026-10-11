//! Merges local and global layers into one resolved [`Config`].
//!
//! [`ConfigBuilder`] applies local-over-global precedence field by field,
//! resolving relative paths (such as template directories) against each layer's
//! own config file root before the local and global layers are merged, so a
//! global config's relative paths never resolve against the local project root
//! by mistake.

use std::path::PathBuf;

use super::{
    Config, FrontmatterConfig, SchemasConfig, TaskConfig,
    error::ConfigValueError,
    file::{GlobalConfigFile, LocalConfigFile, Parsed},
    model::{
        ConfigSubDir, DEFAULT_CLASS_FIELD, DEFAULT_LOCAL_SCHEMAS_DIR,
        TemplateConfig,
    },
    raw::{RawDateFieldConfig, RawFrontmatterConfig, RawTaskConfig},
};
use crate::FieldName;

/// Merges local and optional global config files into a resolved [`Config`].
///
/// Applies local-over-global precedence for unconfigured fields, resolves
/// template directories against their respective config file roots, and
/// validates domain model invariants.
pub(crate) struct ConfigBuilder {
    root: PathBuf,
    local: LocalConfigFile<Parsed>,
    global: Option<GlobalConfigFile<Parsed>>,
}

impl ConfigBuilder {
    /// Creates a new builder for `root` with the local and global layers
    /// (`local`, optional `global`).
    #[inline]
    #[must_use]
    pub(crate) fn new(
        root: PathBuf,
        local: LocalConfigFile<Parsed>,
        global: Option<GlobalConfigFile<Parsed>>,
    ) -> Self {
        Self {
            root,
            local,
            global,
        }
    }

    /// Merges local and global layers and builds the resolved [`Config`].
    ///
    /// # Errors
    ///
    /// - [`ConfigValueError::InvalidSubDir`] when a configured template or
    ///   schema directory escapes the workspace root.
    /// - [`ConfigValueError::InvalidFieldKey`] when a `[frontmatter]` or
    ///   `[schemas]` key name is invalid.
    /// - [`ConfigValueError::InvalidTagFilter`] when a `[tasks] tag_filters`
    ///   entry is not a valid tag.
    /// - [`ConfigValueError::Task`] when a `[tasks]` status uses a prohibited
    ///   delimiter symbol.
    pub(crate) fn build(self) -> Result<Config, ConfigValueError> {
        Ok(Config::new(
            self.resolve_templates()?,
            self.resolve_schemas()?,
            self.resolve_frontmatter()?,
            self.resolve_tasks()?,
            self.root,
        ))
    }

    fn resolve_templates(&self) -> Result<TemplateConfig, ConfigValueError> {
        let local_raw = self.local.raw();
        let global_raw = self.global.as_ref().map(GlobalConfigFile::raw);

        let local_template_dir = local_raw
            .templates
            .directory
            .as_ref()
            .map(|dir| ConfigSubDir::resolve(dir, self.local.root()))
            .transpose()?;

        let global_template_dir = self
            .global
            .as_ref()
            .and_then(|g| {
                g.raw()
                    .templates
                    .directory
                    .as_ref()
                    .map(|dir| ConfigSubDir::resolve(dir, g.root()))
            })
            .transpose()?;
        let output_dir = merge_optional(
            local_raw.templates.output_dir.as_ref(),
            global_raw.and_then(|g| g.templates.output_dir.as_ref()),
        )
        .map_or_else(
            || self.root.clone(),
            |dir| {
                if dir.is_absolute() {
                    dir
                } else {
                    self.root.join(dir)
                }
            },
        );

        Ok(TemplateConfig::new(
            local_template_dir,
            global_template_dir,
            output_dir,
        ))
    }

    fn resolve_schemas(&self) -> Result<SchemasConfig, ConfigValueError> {
        let local_raw = self.local.raw();
        let global_raw = self.global.as_ref().map(GlobalConfigFile::raw);

        let class_field = FieldName::try_from(
            merge_optional(
                local_raw.schemas.class_field.as_ref(),
                global_raw.and_then(|g| g.schemas.class_field.as_ref()),
            )
            .unwrap_or_else(|| DEFAULT_CLASS_FIELD.to_owned()),
        )
        .map_err(|source| {
            ConfigValueError::invalid_field_key("schemas", source)
        })?;

        let (raw_dir, dir_root) = match &local_raw.schemas.directory {
            Some(dir) => (Some(dir.clone()), self.local.root()),
            None => match self.global.as_ref() {
                Some(g) => match &g.raw().schemas.directory {
                    Some(dir) => (Some(dir.clone()), g.root()),
                    None => (None, self.local.root()),
                },
                None => (None, self.local.root()),
            },
        };
        let directory = ConfigSubDir::from_raw_or_default(
            raw_dir.clone(),
            DEFAULT_LOCAL_SCHEMAS_DIR,
        )
        .map_err(|source| ConfigValueError::InvalidSubDir {
            path: raw_dir
                .unwrap_or_else(|| PathBuf::from(DEFAULT_LOCAL_SCHEMAS_DIR)),
            source,
        })?
        .resolve_against(dir_root)?;

        Ok(SchemasConfig::new(directory, class_field))
    }

    fn resolve_frontmatter(
        &self,
    ) -> Result<FrontmatterConfig, ConfigValueError> {
        let local_raw = self.local.raw();
        let global_raw = self.global.as_ref().map(GlobalConfigFile::raw);

        let raw_frontmatter = RawFrontmatterConfig {
            title: merge_optional(
                local_raw.frontmatter.title.as_ref(),
                global_raw.and_then(|g| g.frontmatter.title.as_ref()),
            ),
            aliases: merge_optional(
                local_raw.frontmatter.aliases.as_ref(),
                global_raw.and_then(|g| g.frontmatter.aliases.as_ref()),
            ),
            tags: merge_optional(
                local_raw.frontmatter.tags.as_ref(),
                global_raw.and_then(|g| g.frontmatter.tags.as_ref()),
            ),
            date_created: merge_date_field(
                local_raw.frontmatter.date_created.as_ref(),
                global_raw.and_then(|g| g.frontmatter.date_created.as_ref()),
            ),
            date_modified: merge_date_field(
                local_raw.frontmatter.date_modified.as_ref(),
                global_raw.and_then(|g| g.frontmatter.date_modified.as_ref()),
            ),
        };
        FrontmatterConfig::try_from(raw_frontmatter)
    }

    fn resolve_tasks(&self) -> Result<TaskConfig, ConfigValueError> {
        let local_raw = self.local.raw();
        let global_raw = self.global.as_ref().map(GlobalConfigFile::raw);

        let raw_tasks = RawTaskConfig {
            tag_filters: if local_raw.tasks.tag_filters.is_empty() {
                global_raw
                    .map(|g| g.tasks.tag_filters.clone())
                    .unwrap_or_default()
            } else {
                local_raw.tasks.tag_filters.clone()
            },
            statuses: if local_raw.tasks.statuses.is_empty() {
                global_raw.map(|g| g.tasks.statuses.clone()).unwrap_or_default()
            } else {
                local_raw.tasks.statuses.clone()
            },
        };
        TaskConfig::try_from(raw_tasks)
    }
}

fn merge_optional<T: Clone>(
    local: Option<&T>,
    global: Option<&T>,
) -> Option<T> {
    local.or(global).cloned()
}

fn merge_date_field(
    local: Option<&RawDateFieldConfig>,
    global: Option<&RawDateFieldConfig>,
) -> Option<RawDateFieldConfig> {
    if local.is_none() && global.is_none() {
        return None;
    }
    Some(RawDateFieldConfig {
        name: local
            .and_then(|l| l.name.as_deref())
            .or_else(|| global.and_then(|g| g.name.as_deref()))
            .map(ToOwned::to_owned),
        format: local
            .and_then(|l| l.format.as_deref())
            .or_else(|| global.and_then(|g| g.format.as_deref()))
            .map(ToOwned::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod build {
        use super::*;

        mod fixtures {
            use super::*;

            /// Creates an existing directory for path-confined config
            /// resolution.
            pub(super) fn temp_root(
                temp: &tempfile::TempDir,
                name: &str,
            ) -> PathBuf {
                let root = temp.path().join(name);
                std::fs::create_dir_all(&root).expect("create project dir");
                root
            }

            pub(super) fn build_values(
                temp: &tempfile::TempDir,
                local_toml: &str,
                global_toml: Option<&str>,
            ) -> Result<Config, ConfigValueError> {
                let root = temp_root(temp, "project");
                let local = LocalConfigFile::<Parsed>::from_content_for_test(
                    root.clone(),
                    root.join(".traces/config.toml"),
                    local_toml,
                )
                .expect("parse local config");
                let global = global_toml.map(|content| {
                    let global_root = temp_root(temp, "global");
                    GlobalConfigFile::<Parsed>::from_content_for_test(
                        global_root.clone(),
                        global_root.join("config.toml"),
                        content,
                    )
                    .expect("parse global config")
                });
                ConfigBuilder::new(root, local, global).build()
            }
        }

        mod defaults {
            use pretty_assertions::assert_eq;

            use super::fixtures::build_values;

            #[test]
            fn creates_default_config_when_local_and_global_are_empty() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let root = temp.path().join("project");

                let config = build_values(&temp, "", None)
                    .expect("build default config");

                assert_eq!(config.root(), root.as_path());
                assert_eq!(config.schemas().class_field_name(), "class");
                assert_eq!(
                    config.schemas().directory(),
                    root.join(".traces/schemas").as_path()
                );
            }

            #[test]
            fn resolves_default_to_local_root_when_global_directory_is_unset() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let root = temp.path().join("project");

                let config = build_values(&temp, "", Some(""))
                    .expect("build config with global layer");

                assert_eq!(
                    config.schemas().directory(),
                    root.join(".traces/schemas").as_path()
                );
            }
        }

        mod precedence {
            use pretty_assertions::assert_eq;

            use super::fixtures::build_values;

            #[test]
            fn uses_local_class_field_over_global() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let local = "[schemas]\nclass_field = \"type\"";
                let global = "[schemas]\nclass_field = \"global_type\"";

                let config = build_values(&temp, local, Some(global))
                    .expect("build merged config");

                assert_eq!(config.schemas().class_field_name(), "type");
            }
        }

        mod directories {
            use pretty_assertions::assert_eq;

            use super::fixtures::build_values;

            #[test]
            fn resolves_local_template_directory_against_local_root() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let root = temp.path().join("project");
                let local = "[templates]\ndirectory = \"my_templates\"";
                let global = "[templates]\ndirectory = \"global_templates\"";

                let config = build_values(&temp, local, Some(global))
                    .expect("build merged config");

                assert_eq!(
                    config.local_template_dir(),
                    Some(root.join("my_templates").as_path())
                );
            }

            #[test]
            fn resolves_global_template_directory_against_global_root() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let global_root = temp.path().join("global");
                let global = "[templates]\ndirectory = \"global_templates\"";

                let config = build_values(
                    &temp,
                    "[templates]\ndirectory = \"my_templates\"",
                    Some(global),
                )
                .expect("build merged config");

                assert_eq!(
                    config.global_template_dir(),
                    Some(global_root.join("global_templates").as_path())
                );
            }

            #[test]
            fn resolves_global_schema_directory_against_global_root() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let global_root = temp.path().join("global");
                let global = "[schemas]\ndirectory = \"global_schemas\"";

                let config = build_values(&temp, "", Some(global))
                    .expect("build merged config");

                assert_eq!(
                    config.schemas().directory(),
                    global_root.join("global_schemas").as_path()
                );
            }

            #[test]
            fn resolves_global_output_directory_against_local_root() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let root = temp.path().join("project");
                let global = "[templates]\noutput_dir = \"global_output\"";

                let config = build_values(&temp, "", Some(global))
                    .expect("build merged config");

                assert_eq!(
                    config.output_dir(),
                    root.join("global_output").as_path()
                );
            }
        }

        mod tag_filters {
            use pretty_assertions::assert_eq;

            use super::fixtures::build_values;

            #[test]
            fn uses_local_tag_filters_over_global_when_local_is_non_empty() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let local = "[tasks]\ntag_filters = [\"task\"]\n";
                let global = "[tasks]\ntag_filters = [\"todo\"]\n";
                let expected = crate::Tag::parse("#task").expect("valid tag");

                let config = build_values(&temp, local, Some(global))
                    .expect("build merged config");

                assert_eq!(config.tasks().tag_filters(), [expected]);
            }

            #[test]
            fn falls_back_to_global_tag_filters_when_local_is_empty() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let global = "[tasks]\ntag_filters = [\"todo\"]\n";
                let expected = crate::Tag::parse("#todo").expect("valid tag");

                let config = build_values(&temp, "", Some(global))
                    .expect("build merged config");

                assert_eq!(config.tasks().tag_filters(), [expected]);
            }
        }

        mod task_statuses {
            use pretty_assertions::assert_eq;

            use super::fixtures::build_values;

            #[test]
            fn uses_local_task_statuses_over_global_when_local_is_non_empty() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let local = r#"[[tasks.statuses]]
symbol = "?"
name = "Blocked"
kind = "on-hold"
"#;
                let global = r#"[[tasks.statuses]]
symbol = "~"
name = "Waiting"
kind = "todo"
"#;

                let config = build_values(&temp, local, Some(global))
                    .expect("build merged config");

                let status = config
                    .tasks()
                    .statuses()
                    .by_symbol('?'.into())
                    .expect("local status");
                assert_eq!(status.name(), "Blocked");
                assert_eq!(status.kind(), crate::TaskStatusType::OnHold);
                assert!(
                    config.tasks().statuses().by_symbol('~'.into()).is_none()
                );
            }

            #[test]
            fn falls_back_to_global_task_statuses_when_local_is_empty() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let global = r#"[[tasks.statuses]]
symbol = "?"
name = "Blocked"
kind = "on-hold"
"#;

                let config = build_values(&temp, "", Some(global))
                    .expect("build merged config");

                let status = config
                    .tasks()
                    .statuses()
                    .by_symbol('?'.into())
                    .expect("global status");
                assert_eq!(status.name(), "Blocked");
                assert_eq!(status.kind(), crate::TaskStatusType::OnHold);
            }
        }

        mod errors {
            use super::{ConfigValueError, fixtures::build_values};

            #[test]
            fn fails_to_build_when_a_tag_filter_entry_is_invalid() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let local = "[tasks]\ntag_filters = [\"1invalid\"]\n";

                let result = build_values(&temp, local, None);

                assert!(matches!(
                    result,
                    Err(ConfigValueError::InvalidTagFilter { .. })
                ));
            }
        }

        mod values {
            use pretty_assertions::assert_eq;
            use rstest::rstest;

            use super::{Config, ConfigValueError, fixtures::build_values};

            #[rstest]
            #[case::title(|c: &Config| c.frontmatter().title_name().to_owned(), "title")]
            #[case::aliases(|c: &Config| c.frontmatter().aliases_name().to_owned(), "aliases")]
            #[case::tags(|c: &Config| c.frontmatter().tags_name().to_owned(), "tags")]
            #[case::date_created_name(|c: &Config| c.frontmatter().date_created().name().to_owned(), "date_created")]
            #[case::date_created_format(|c: &Config| c.frontmatter().date_created().format().to_owned(), "%Y-%m-%dT%H:%M:%S")]
            #[case::date_modified_name(|c: &Config| c.frontmatter().date_modified().name().to_owned(), "date_modified")]
            #[case::date_modified_format(|c: &Config| c.frontmatter().date_modified().format().to_owned(), "%Y-%m-%dT%H:%M:%S")]
            fn defaults_each_field_when_unconfigured(
                #[case] accessor: fn(&Config) -> String,
                #[case] expected: &str,
            ) {
                let temp = tempfile::tempdir().expect("temp dir");

                let config = build_values(&temp, "[templates]", None)
                    .expect("build default config");

                assert_eq!(accessor(&config), expected);
            }

            #[rstest]
            #[case::class_field("[schemas]\nclass_field = \"kind\"", |c: &Config| c.schemas().class_field_name().to_owned(), "kind")]
            #[case::schema_directory("[schemas]\ndirectory = \"custom/schemas\"", |c: &Config| c.schemas().directory().strip_prefix(c.root()).expect("schema dir under root").to_string_lossy().into_owned(), "custom/schemas")]
            #[case::title("[frontmatter]\ntitle = \"Title\"", |c: &Config| c.frontmatter().title_name().to_owned(), "Title")]
            #[case::aliases("[frontmatter]\naliases = \"Aliases\"", |c: &Config| c.frontmatter().aliases_name().to_owned(), "Aliases")]
            #[case::tags("[frontmatter]\ntags = \"Categories\"", |c: &Config| c.frontmatter().tags_name().to_owned(), "Categories")]
            #[case::output_dir("[templates]\noutput_dir = \"local_out\"", |c: &Config| c.output_dir().strip_prefix(c.root()).expect("output dir under root").to_string_lossy().into_owned(), "local_out")]
            fn parses_each_configured_field(
                #[case] toml: &str,
                #[case] accessor: fn(&Config) -> String,
                #[case] expected: &str,
            ) {
                let temp = tempfile::tempdir().expect("temp dir");

                let config =
                    build_values(&temp, toml, None).expect("build config");

                assert_eq!(accessor(&config), expected);
            }

            #[rstest]
            #[case::title("[frontmatter]\ntitle = \"local_title\"", "[frontmatter]\ntitle = \"global_title\"", |c: &Config| c.frontmatter().title_name().to_owned(), "local_title")]
            #[case::aliases("[frontmatter]\naliases = \"aka\"", "[frontmatter]\naliases = \"aliases\"", |c: &Config| c.frontmatter().aliases_name().to_owned(), "aka")]
            #[case::output_dir("[templates]\noutput_dir = \"local_out\"", "[templates]\noutput_dir = \"global_out\"", |c: &Config| c.output_dir().strip_prefix(c.root()).expect("output dir under root").to_string_lossy().into_owned(), "local_out")]
            fn prioritizes_local_field_over_global(
                #[case] local: &str,
                #[case] global: &str,
                #[case] accessor: fn(&Config) -> String,
                #[case] expected: &str,
            ) {
                let temp = tempfile::tempdir().expect("temp dir");

                let config = build_values(&temp, local, Some(global))
                    .expect("build merged config");

                assert_eq!(accessor(&config), expected);
            }

            #[rstest]
            #[case::class_field_whitespace(
                "[schemas]\nclass_field = \"   \"",
                "schemas"
            )]
            #[case::class_field_slash(
                "[schemas]\nclass_field = \"a/b\"",
                "schemas"
            )]
            #[case::date_created_name_whitespace(
                "[frontmatter.date_created]\nname = \"   \"",
                "frontmatter"
            )]
            #[case::title_empty("[frontmatter]\ntitle = \"\"", "frontmatter")]
            #[case::aliases_whitespace(
                "[frontmatter]\naliases = \"   \"",
                "frontmatter"
            )]
            #[case::tags_empty("[frontmatter]\ntags = \"\"", "frontmatter")]
            fn rejects_invalid_field_keys(
                #[case] toml: &str,
                #[case] table: &str,
            ) {
                let temp = tempfile::tempdir().expect("temp dir");

                let result = build_values(&temp, toml, None);

                assert!(
                    matches!(result, Err(ConfigValueError::InvalidFieldKey { table: actual, .. }) if actual == table)
                );
            }

            #[rstest]
            #[case::created(
                "[frontmatter.date_created]\nname = \"created\"\nformat = \
                 \"%Y-%m-%d\"",
                true,
                "created",
                "%Y-%m-%d"
            )]
            #[case::modified(
                "[frontmatter.date_modified]\nname = \"modified\"\nformat = \
                 \"%Y-%m-%dT%H:%M:%S\"",
                false,
                "modified",
                "%Y-%m-%dT%H:%M:%S"
            )]
            #[case::created_name_only(
                "[frontmatter.date_created]\nname = \"created_at\"",
                true,
                "created_at",
                "%Y-%m-%dT%H:%M:%S"
            )]
            #[case::modified_format_only(
                "[frontmatter.date_modified]\nformat = \"%Y-%m-%d\"",
                false,
                "date_modified",
                "%Y-%m-%d"
            )]
            fn resolves_date_field_name_and_format(
                #[case] toml: &str,
                #[case] created: bool,
                #[case] expected_name: &str,
                #[case] expected_format: &str,
            ) {
                let temp = tempfile::tempdir().expect("temp dir");

                let config =
                    build_values(&temp, toml, None).expect("build date config");
                let date = if created {
                    config.frontmatter().date_created()
                } else {
                    config.frontmatter().date_modified()
                };

                assert_eq!(date.name(), expected_name);
                assert_eq!(date.format(), expected_format);
            }

            #[test]
            fn falls_back_to_global_aliases_when_local_omits_aliases() {
                let temp = tempfile::tempdir().expect("temp dir");
                let local = "[frontmatter]\ntitle = \"local_title\"";
                let global = "[frontmatter]\naliases = \"aka\"";

                let config = build_values(&temp, local, Some(global))
                    .expect("build merged config");

                assert_eq!(config.frontmatter().title_name(), "local_title");
                assert_eq!(config.frontmatter().aliases_name(), "aka");
            }

            #[test]
            fn leaves_global_template_dir_empty_when_missing() {
                let temp = tempfile::tempdir().expect("temp dir");

                let config = build_values(
                    &temp,
                    "[templates]\noutput_dir = \"local_out\"",
                    None,
                )
                .expect("build config");

                assert_eq!(config.global_template_dir(), None);
            }

            #[test]
            fn uses_local_root_when_output_dir_missing() {
                let temp = tempfile::tempdir().expect("temp dir");

                let config = build_values(&temp, "[templates]", None)
                    .expect("build config");

                assert_eq!(config.output_dir(), config.root());
            }
        }
    }
}
