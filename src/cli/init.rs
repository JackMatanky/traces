//! Local project scaffold command.
//!
//! Handles `traces init` by collecting template and output directories
//! interactively, creating `.traces/` plus its template directory, and writing
//! the initial local config.

use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::Args;

use super::error::{CliError, CliResult};
use crate::{
    DialogProvider,
    config::{ConfigService, DEFAULT_LOCAL_TEMPLATES_DIR, LOCAL_CONFIG_DIR},
};

const DEFAULT_OUTPUT_DIRECTORY: &str = ".";

/// Arguments for `traces init`.
///
/// Initializes traces configuration in the current directory by scaffolding
/// `.traces/`, a template directory, and a local config file.
#[derive(Debug, Args)]
pub struct Init;

impl Init {
    /// Runs `traces init` using the current directory as the project root.
    ///
    /// # Errors
    ///
    /// - [`CliError::CurrentDirectory`] if the current directory cannot be
    ///   read.
    /// - [`CliError::InitPrompt`] if collecting input interactively fails.
    /// - [`CliError::InitAlreadyInitialized`] if `.traces` already exists.
    /// - [`CliError::InitScaffold`] if directory scaffolding fails.
    /// - [`CliError::InitConfigWrite`] if serializing or writing the local
    ///   config file fails.
    #[inline]
    pub fn run(self, provider: &dyn DialogProvider) -> CliResult {
        let root = super::current_dir()?.into_inner();
        let input = Self::collect_config(provider)?;
        Self::scaffold_directory(&root)?;
        Self::write_config_file(&root, &input.directory, &input.output_dir)?;
        eprintln!("initialised traces in {}", root.display());
        Ok(())
    }

    /// Collects template and output directories from the user.
    ///
    /// # Errors
    ///
    /// - [`CliError::InitPrompt`] if either prompt fails.
    fn collect_config(
        provider: &dyn DialogProvider,
    ) -> Result<InitInput, CliError> {
        let directory = provider
            .text("Template directory", Some(DEFAULT_LOCAL_TEMPLATES_DIR))
            .map_err(|source| CliError::InitPrompt {
                source,
            })?;
        let output_dir = provider
            .text("Output directory", Some(DEFAULT_OUTPUT_DIRECTORY))
            .map_err(|source| CliError::InitPrompt {
                source,
            })?;
        Ok(InitInput {
            directory: PathBuf::from(directory),
            output_dir: PathBuf::from(output_dir),
        })
    }

    /// Scaffolds `.traces/` and `.traces/templates/`.
    ///
    /// # Errors
    ///
    /// - [`CliError::InitAlreadyInitialized`] if `.traces` already exists under
    ///   `root`.
    /// - [`CliError::InitScaffold`] if creating directories fails.
    fn scaffold_directory(root: &Path) -> CliResult {
        let traces_dir = root.join(LOCAL_CONFIG_DIR);
        if traces_dir.exists() {
            return Err(CliError::InitAlreadyInitialized {
                root: root.to_path_buf(),
            });
        }
        fs::create_dir(&traces_dir).map_err(|source| {
            CliError::InitScaffold {
                root: root.to_path_buf(),
                source,
            }
        })?;
        fs::create_dir(root.join(DEFAULT_LOCAL_TEMPLATES_DIR)).map_err(
            |source| CliError::InitScaffold {
                root: root.to_path_buf(),
                source,
            },
        )?;
        Ok(())
    }

    /// Writes the local config file under `root` via [`ConfigService`].
    ///
    /// # Errors
    ///
    /// - [`CliError::InitConfigWrite`] if serializing or writing the local
    ///   config file fails.
    fn write_config_file(
        root: &Path,
        directory: &Path,
        output_dir: &Path,
    ) -> CliResult {
        ConfigService::scaffold_local(root, directory, output_dir).map_err(
            |source| CliError::InitConfigWrite {
                root: root.to_path_buf(),
                source,
            },
        )
    }
}

/// The user's chosen template/output directories, collected interactively.
struct InitInput {
    directory: PathBuf,
    output_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fixtures {
        use super::super::*;
        use crate::DialogError;

        pub(super) struct CancellingDialogProvider;
        impl DialogProvider for CancellingDialogProvider {
            fn is_interactive(&self) -> bool {
                true
            }

            fn text(
                &self,
                _label: &str,
                _default: Option<&str>,
            ) -> Result<String, DialogError> {
                Err(DialogError::UserCancelled)
            }

            fn confirm(
                &self,
                _label: &str,
                _default: Option<bool>,
            ) -> Result<bool, DialogError> {
                Err(DialogError::UserCancelled)
            }

            fn select(
                &self,
                _label: &str,
                _items: &[String],
            ) -> Result<usize, DialogError> {
                Err(DialogError::UserCancelled)
            }

            fn multi_select(
                &self,
                _label: &str,
                _items: &[String],
            ) -> Result<Vec<usize>, DialogError> {
                Err(DialogError::UserCancelled)
            }
        }
    }
    use fixtures::*;

    mod run {
        use super::*;
        use crate::cli::{CwdGuard, UserAbort};

        #[test]
        fn leaves_no_traces_directory_when_the_prompt_is_cancelled() {
            let root = tempfile::tempdir().expect("create temp dir");
            let _guard = CwdGuard::enter(root.path());

            let error = Init
                .run(&CancellingDialogProvider)
                .expect_err("cancelled prompt fails init");

            assert_eq!(error.user_abort(), Some(UserAbort::Cancelled));
            assert!(!root.path().join(".traces").exists());
        }

        /// Exercises the interactive `traces init` dialog pipeline with both
        /// preset custom paths and defaults, and confirms
        /// already-initialized refusal.
        ///
        /// Relocated from `tests/e2e/init.rs` to keep in-process dialog mocking
        /// inside the crate boundary while E2E tests execute only
        /// spawned binaries.
        ///
        /// Executes vs asserts:
        /// - Executes: `Init.run(&provider)` with `PresetDialogProvider`
        ///   queueing custom answers ("custom/templates", "notes") and default
        ///   empty answers.
        /// - Asserts: written `.traces/config.toml` matches input paths,
        ///   `.traces/templates` directory is created, and existing `.traces`
        ///   directory fails with `CliError::InitAlreadyInitialized`.
        ///
        /// Defect class: layer-discipline (moves in-process mock out of
        /// black-box E2E suite).
        #[test]
        fn scaffolds_preset_defaults_and_refuses_existing_traces_dir() {
            let preset = tempfile::tempdir().expect("create preset temp dir");
            {
                let _guard = CwdGuard::enter(preset.path());
                let provider = crate::PresetDialogProvider::new()
                    .with_text("custom/templates")
                    .with_text("notes");

                Init.run(&provider).expect("run preset init");
            }

            assert_config(preset.path(), "custom/templates", "notes");
            assert!(preset.path().join(".traces/templates").is_dir());

            let defaults =
                tempfile::tempdir().expect("create defaults temp dir");
            {
                let _guard = CwdGuard::enter(defaults.path());
                let provider = crate::PresetDialogProvider::new();

                Init.run(&provider).expect("run default init");
            }

            assert_config(defaults.path(), ".traces/templates", ".");
            assert!(defaults.path().join(".traces/templates").is_dir());

            let existing =
                tempfile::tempdir().expect("create existing temp dir");
            fs::create_dir(existing.path().join(".traces"))
                .expect("create existing traces dir");
            let result = {
                let _guard = CwdGuard::enter(existing.path());
                let provider = crate::PresetDialogProvider::new();

                Init.run(&provider)
            };

            let error =
                result.expect_err("existing .traces directory fails init");
            assert!(matches!(error, CliError::InitAlreadyInitialized { .. }));
        }

        fn assert_config(
            root: &Path,
            expected_directory: &str,
            expected_output_dir: &str,
        ) {
            let config_path = root.join(".traces/config.toml");
            let contents =
                fs::read_to_string(config_path).expect("read config");
            let value: toml::Value =
                toml::from_str(&contents).expect("parse config");
            let templates = value
                .get("templates")
                .and_then(toml::Value::as_table)
                .expect("templates table");

            assert_eq!(table_str(templates, "directory"), expected_directory);
            assert_eq!(table_str(templates, "output_dir"), expected_output_dir);
        }

        fn table_str<'a>(table: &'a toml::value::Table, key: &str) -> &'a str {
            table.get(key).and_then(toml::Value::as_str).expect("string value")
        }
    }
}
