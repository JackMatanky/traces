//! Shared test arrangement utilities for both integration and E2E test suites.
//!
//! # Architecture Rules
//!
//! 1. **Zero Crate Exports**: This module imports NOTHING from `traces_pkm`.
//! 2. **Feature-Independent**: Must compile identically with or without
//!    `test-utils`.
//! 3. **Layer Separation**: Arrangement only. Trust and CLI invocation remain
//!    layer-local: integration tests trust via facade (`testing::TestProject`),
//!    E2E via CLI spawn.
//! 4. **Assertion Style Convention**: Tests written for this spec and onward
//!    MUST convert `.expect` on behavior-under-test to assertions (`assert!`,
//!    `assert_eq!`), reserving `.expect` strictly for test harness and fixture
//!    setup invariants.
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
            let _ = writeln!(out, "directory = \"{tmpl}\"");
        }

        if self.schemas_dir.is_some() || self.schemas_class_field.is_some() {
            out.push_str("[schemas]\n");
            if let Some(dir) = &self.schemas_dir {
                let _ = writeln!(out, "directory = \"{dir}\"");
            }
            if let Some(cf) = &self.schemas_class_field {
                let _ = writeln!(out, "class_field = \"{cf}\"");
            }
            out.push('\n');
        }

        if !self.task_tag_filters.is_empty() || !self.task_statuses.is_empty() {
            out.push_str("[tasks]\n");
            if !self.task_tag_filters.is_empty() {
                let filters = self
                    .task_tag_filters
                    .iter()
                    .map(|f| format!("\"{f}\""))
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(out, "tag_filters = [{filters}]");
            }
            out.push('\n');

            for (symbol, name, kind) in &self.task_statuses {
                out.push_str("[[tasks.statuses]]\n");
                let _ = writeln!(out, "symbol = \"{symbol}\"");
                let _ = writeln!(out, "name = \"{name}\"");
                let _ = writeln!(out, "kind = \"{kind}\"\n");
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
    fn writes_config_file_to_disk() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let fixture = ConfigFixture::minimal();
        let path = fixture.write_to(temp.path()).expect("write config");
        assert_eq!(path, temp.path().join(CONFIG_FILE_REL));
        let read = fs::read_to_string(&path).expect("read written config");
        assert_eq!(read, MINIMAL_CONFIG_TOML);
    }
}
