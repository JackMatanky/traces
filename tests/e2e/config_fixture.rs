//! On-disk configuration fixtures for spawned end-to-end tests.
use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

pub const CONFIG_FILE_REL: &str = ".traces/config.toml";
pub const DEFAULT_TEMPLATES_DIR_NAME: &str = "templates";
pub const DEFAULT_SCHEMAS_DIR_REL: &str = ".traces/schemas";

pub const MINIMAL_CONFIG_TOML: &str =
    "[templates]\ndirectory = \"templates\"\n";

/// Explicit declarative configuration fixture.
///
/// Supports building structured configurations with sections for `templates`,
/// `schemas`, and `tasks`, as well as raw TOML writing for invalid/malformed
/// cases.
#[derive(Clone, Debug, Default)]
pub struct ConfigFixture {
    templates_dir: Option<String>,
    schemas_dir: Option<String>,
    schemas_class_field: Option<String>,
    task_tag_filters: Vec<String>,
    task_statuses: Vec<(char, String, String)>,
    raw_content: Option<String>,
}

impl ConfigFixture {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a minimal valid configuration fixture.
    #[must_use]
    pub fn minimal() -> Self {
        Self::new().with_templates(DEFAULT_TEMPLATES_DIR_NAME)
    }

    /// Configures the template directory.
    #[must_use]
    pub fn with_templates(mut self, dir: impl Into<String>) -> Self {
        self.templates_dir = Some(dir.into());
        self
    }

    /// Configures the schemas directory and optional class field name.
    #[must_use]
    pub fn with_schemas(
        mut self,
        dir: impl Into<String>,
        class_field: Option<&str>,
    ) -> Self {
        self.schemas_dir = Some(dir.into());
        self.schemas_class_field = class_field.map(str::to_owned);
        self
    }

    /// Configures task tag filters.
    #[must_use]
    pub fn with_task_tag_filters(mut self, filters: &[&str]) -> Self {
        self.task_tag_filters = filters.iter().map(|&s| s.to_owned()).collect();
        self
    }

    /// Configures a task status symbol, name, and kind.
    #[must_use]
    pub fn with_task_status(
        mut self,
        symbol: char,
        name: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        self.task_statuses.push((symbol, name.into(), kind.into()));
        self
    }

    /// Creates a fixture holding explicit raw TOML, bypassing structured
    /// serialization. Used for invalid/malformed TOML testing.
    #[must_use]
    pub fn raw(content: impl Into<String>) -> Self {
        Self {
            raw_content: Some(content.into()),
            ..Self::default()
        }
    }

    /// Renders the configuration as a TOML string.
    #[must_use]
    pub fn to_toml_string(&self) -> String {
        if let Some(raw) = &self.raw_content {
            return raw.clone();
        }

        let mut out = String::new();
        if let Some(tmpl) = &self.templates_dir {
            out.push_str("[templates]\n");
            let _ = writeln!(out, "directory = {}", toml_string(tmpl));
        }

        if self.schemas_dir.is_some() || self.schemas_class_field.is_some() {
            out.push_str("[schemas]\n");
            if let Some(dir) = &self.schemas_dir {
                let _ = writeln!(out, "directory = {}", toml_string(dir));
            }
            if let Some(cf) = &self.schemas_class_field {
                let _ = writeln!(out, "class_field = {}", toml_string(cf));
            }
            out.push('\n');
        }

        if !self.task_tag_filters.is_empty() || !self.task_statuses.is_empty() {
            out.push_str("[tasks]\n");
            if !self.task_tag_filters.is_empty() {
                let filters = self
                    .task_tag_filters
                    .iter()
                    .map(|filter| toml_string(filter))
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(out, "tag_filters = [{filters}]");
            }
            out.push('\n');

            for (symbol, name, kind) in &self.task_statuses {
                out.push_str("[[tasks.statuses]]\n");
                let _ = writeln!(
                    out,
                    "symbol = {}",
                    toml_string(&symbol.to_string())
                );
                let _ = writeln!(out, "name = {}", toml_string(name));
                let _ = writeln!(out, "kind = {}\n", toml_string(kind));
            }
        }

        out
    }

    /// Writes this configuration to `<root>/.traces/config.toml`, creating
    /// parent directories as needed. Returns the path to the written file.
    pub fn write_to(&self, root: &Path) -> std::io::Result<PathBuf> {
        let config_path = root.join(CONFIG_FILE_REL);
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&config_path, self.to_toml_string())?;
        Ok(config_path)
    }
}

fn toml_string(value: &str) -> String {
    let table = toml::Table::from_iter([(
        "value".to_owned(),
        toml::Value::String(value.to_owned()),
    )]);
    toml::to_string(&table)
        .expect("a string is serializable as TOML")
        .strip_prefix("value = ")
        .expect("the table contains only value")
        .trim_end_matches('\n')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn formats_minimal_configuration() {
        let fixture = ConfigFixture::minimal();
        assert_eq!(fixture.to_toml_string(), MINIMAL_CONFIG_TOML);
    }

    #[test]
    fn formats_raw_configuration() {
        let raw_toml = "malformed toml = [[";
        let fixture = ConfigFixture::raw(raw_toml);
        assert_eq!(fixture.to_toml_string(), raw_toml);
    }

    #[test]
    fn formats_declared_sections() {
        let fixture = ConfigFixture::new()
            .with_templates(DEFAULT_TEMPLATES_DIR_NAME)
            .with_schemas(DEFAULT_SCHEMAS_DIR_REL, Some("kind"))
            .with_task_tag_filters(&["task", "todo"])
            .with_task_status('?', "Blocked", "on-hold");

        let toml = fixture.to_toml_string();
        assert!(toml.contains("[templates]\ndirectory = \"templates\""));
        assert!(toml.contains(
            "[schemas]\ndirectory = \".traces/schemas\"\nclass_field = \
             \"kind\""
        ));
        assert!(toml.contains("tag_filters = [\"task\", \"todo\"]"));
        assert!(toml.contains("symbol = \"?\""));
    }

    #[test]
    fn serializes_special_characters_in_structured_values() {
        let fixture = ConfigFixture::new()
            .with_templates("dir\\\"name")
            .with_schemas("schema\\path", Some("kind\"field"))
            .with_task_tag_filters(&["todo\\daily", "a\"b"])
            .with_task_status('"', "On\"\nhold", "in\\progress");
        let doc: toml::Table =
            toml::from_str(&fixture.to_toml_string()).expect("valid TOML");
        let templates = doc
            .get("templates")
            .and_then(toml::Value::as_table)
            .expect("templates section");
        assert_eq!(
            templates.get("directory").and_then(toml::Value::as_str),
            Some("dir\\\"name")
        );
        let schemas = doc
            .get("schemas")
            .and_then(toml::Value::as_table)
            .expect("schemas section");
        assert_eq!(
            schemas.get("directory").and_then(toml::Value::as_str),
            Some("schema\\path")
        );
        assert_eq!(
            schemas.get("class_field").and_then(toml::Value::as_str),
            Some("kind\"field")
        );

        let tasks = doc
            .get("tasks")
            .and_then(toml::Value::as_table)
            .expect("tasks section");
        let filters = tasks
            .get("tag_filters")
            .and_then(toml::Value::as_array)
            .expect("tag filters");
        assert_eq!(
            filters.first().and_then(toml::Value::as_str),
            Some("todo\\daily")
        );
        assert_eq!(filters.get(1).and_then(toml::Value::as_str), Some("a\"b"));
        let status = tasks
            .get("statuses")
            .and_then(toml::Value::as_array)
            .and_then(|statuses| statuses.first())
            .and_then(toml::Value::as_table)
            .expect("configured status");
        assert_eq!(
            status.get("symbol").and_then(toml::Value::as_str),
            Some("\"")
        );
        assert_eq!(
            status.get("name").and_then(toml::Value::as_str),
            Some("On\"\nhold")
        );
        assert_eq!(
            status.get("kind").and_then(toml::Value::as_str),
            Some("in\\progress")
        );
    }

    #[test]
    fn writes_config_file_to_disk() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let fixture = ConfigFixture::minimal();
        let path = fixture.write_to(temp.path()).expect("write config");
        assert_eq!(path, temp.path().join(CONFIG_FILE_REL));
        let read = fs::read_to_string(&path).expect("read written config");
        assert_eq!(read, MINIMAL_CONFIG_TOML);
    }
}
