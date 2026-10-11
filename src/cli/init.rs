//! Local project scaffold command.
//!
//! Handles `traces init` by collecting template and output directories
//! interactively, creating `.traces/` plus its template directory, and writing
//! the initial local config.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::Args;
use thiserror::Error;

use super::error::{CliError, CliResult};
use crate::{
    DialogProvider,
    config::{
        DEFAULT_LOCAL_TEMPLATES_DIR, LOCAL_CONFIG_DIR, LOCAL_CONFIG_FILE,
        RawConfig, RawTemplateConfig,
    },
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

    /// Writes the local config file under `root`.
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
        scaffold_local(root, directory, output_dir).map_err(|source| {
            CliError::InitConfigWrite {
                root: root.to_path_buf(),
                source,
            }
        })
    }
}

/// Errors raised while scaffolding a local config file on disk.
#[derive(Debug, Error)]
pub(crate) enum ConfigScaffoldError {
    /// The collected template and output directories could not be
    /// serialised to TOML.
    #[error("failed to serialise local config")]
    Serialize {
        /// Source TOML serialization error.
        #[source]
        source: toml::ser::Error,
    },
    /// The serialised local config could not be written to disk.
    ///
    /// Includes the case where the file already exists, since
    /// [`std::fs::File::create_new`] is used to prevent silent clobbering.
    #[error("failed to write local config file")]
    Write {
        /// Source filesystem error.
        #[source]
        source: io::Error,
    },
}

/// Serialises `directory`/`output_dir` as the local template config and
/// writes it to `root.join(LOCAL_CONFIG_FILE)`.
///
/// `[schemas]` and `[frontmatter]` are written as empty tables (their serde
/// defaults), so a freshly scaffolded config behaves identically to one that
/// omits those tables entirely.
///
/// Uses [`std::fs::File::create_new`] rather than [`std::fs::write`] so this
/// fails atomically if the file already exists, preventing a concurrent
/// `traces init` or a file planted between the existence check and write
/// from being silently clobbered.
///
/// # Errors
///
/// - [`ConfigScaffoldError::Serialize`] when TOML serialization fails.
/// - [`ConfigScaffoldError::Write`] when the file already exists, or creating
///   or writing it fails.
fn scaffold_local(
    root: &Path,
    directory: &Path,
    output_dir: &Path,
) -> Result<(), ConfigScaffoldError> {
    let config = RawConfig {
        templates: RawTemplateConfig {
            directory: Some(directory.to_path_buf()),
            output_dir: Some(output_dir.to_path_buf()),
        },
        ..RawConfig::default()
    };
    let contents = toml::to_string(&config).map_err(|source| {
        ConfigScaffoldError::Serialize {
            source,
        }
    })?;
    let mut file = fs::File::create_new(root.join(LOCAL_CONFIG_FILE)).map_err(
        |source| ConfigScaffoldError::Write {
            source,
        },
    )?;
    file.write_all(contents.as_bytes()).map_err(|source| {
        ConfigScaffoldError::Write {
            source,
        }
    })
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

        /// Checks user-selected paths through the in-process dialog seam.
        #[test]
        fn scaffolds_config_with_custom_template_and_output_paths() {
            let preset = tempfile::tempdir().expect("create preset temp dir");
            {
                let _guard = CwdGuard::enter(preset.path());
                let provider = crate::PresetDialogProvider::new()
                    .with_text("custom/templates")
                    .with_text("notes");

                Init.run(&provider).expect("run preset init");
            }

            let config = read_config(preset.path());
            let templates = config.get("templates").expect("templates table");
            assert_eq!(
                templates.get("directory").and_then(toml::Value::as_str),
                Some("custom/templates")
            );
            assert_eq!(
                templates.get("output_dir").and_then(toml::Value::as_str),
                Some("notes")
            );
            assert!(preset.path().join(".traces/templates").is_dir());
        }

        /// Checks that the dialog's defaults are written to the local config.
        #[test]
        fn scaffolds_config_with_default_template_and_output_paths() {
            let defaults =
                tempfile::tempdir().expect("create defaults temp dir");
            {
                let _guard = CwdGuard::enter(defaults.path());
                let provider = crate::PresetDialogProvider::new();

                Init.run(&provider).expect("run default init");
            }

            let config = read_config(defaults.path());
            let templates = config.get("templates").expect("templates table");
            assert_eq!(
                templates.get("directory").and_then(toml::Value::as_str),
                Some(".traces/templates")
            );
            assert_eq!(
                templates.get("output_dir").and_then(toml::Value::as_str),
                Some(".")
            );
            assert!(defaults.path().join(".traces/templates").is_dir());
        }

        /// Existing initialization must not be overwritten.
        #[test]
        fn refuses_existing_traces_directory() {
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

        fn read_config(root: &Path) -> toml::Value {
            let config_path = root.join(".traces/config.toml");
            let contents =
                fs::read_to_string(config_path).expect("read config");
            toml::from_str(&contents).expect("parse config")
        }
    }

    mod scaffold_local {
        use super::*;

        #[test]
        fn writes_the_local_config_file() {
            let root = tempfile::tempdir().expect("create temp dir");
            fs::create_dir(root.path().join(".traces"))
                .expect("create .traces dir");

            scaffold_local(
                root.path(),
                Path::new("templates"),
                Path::new("notes"),
            )
            .expect("scaffold local config");

            let contents =
                fs::read_to_string(root.path().join(LOCAL_CONFIG_FILE))
                    .expect("read written config");
            assert!(contents.contains("templates"));
        }

        #[test]
        fn refuses_to_overwrite_an_existing_config_file() {
            let root = tempfile::tempdir().expect("create temp dir");
            fs::create_dir(root.path().join(".traces"))
                .expect("create .traces dir");
            scaffold_local(
                root.path(),
                Path::new("templates"),
                Path::new("notes"),
            )
            .expect("first scaffold succeeds");

            let error = scaffold_local(
                root.path(),
                Path::new("other"),
                Path::new("elsewhere"),
            )
            .expect_err(
                "second scaffold at the same root must fail, not clobber",
            );

            assert!(matches!(error, ConfigScaffoldError::Write { .. }));
        }
    }
}
