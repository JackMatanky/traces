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
        ConfigSubDir, DEFAULT_LOCAL_TEMPLATES_DIR, LOCAL_CONFIG_DIR,
        LOCAL_CONFIG_FILE, RawConfig, RawTemplateConfig,
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
    /// Nothing is written when the chosen template directory is invalid, and
    /// everything this run created is removed again if a later step fails.
    ///
    /// # Errors
    ///
    /// - [`CliError::CurrentDirectory`] if the current directory cannot be
    ///   read.
    /// - [`CliError::InitPrompt`] if collecting input interactively fails.
    /// - [`CliError::InitAlreadyInitialized`] if `.traces` already exists.
    /// - [`CliError::InitInvalidTemplateDir`] if the template directory is
    ///   absolute, contains `..`, or resolves outside the project.
    /// - [`CliError::InitScaffold`] if directory scaffolding fails.
    /// - [`CliError::InitConfigWrite`] if serializing or writing the local
    ///   config file fails.
    #[inline]
    pub fn run(self, provider: &dyn DialogProvider) -> CliResult {
        let root = super::current_dir()?.into_inner();
        let input = Self::collect_config(provider)?;
        let traces_dir = root.join(LOCAL_CONFIG_DIR);
        if traces_dir.exists() {
            return Err(CliError::InitAlreadyInitialized {
                root,
            });
        }
        let templates_dir = ConfigSubDir::resolve(&input.directory, &root)
            .map_err(|source| CliError::InitInvalidTemplateDir {
                directory: input.directory.clone(),
                source,
            })?;

        let mut created = Vec::new();
        let result =
            Self::scaffold_directories(&root, &templates_dir, &mut created)
                .and_then(|()| {
                    Self::write_config_file(
                        &root,
                        &input.directory,
                        &input.output_dir,
                    )
                });
        if let Err(error) = result {
            Self::undo(&created);
            return Err(error);
        }
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

    /// Creates `.traces/` and every missing directory down to `templates_dir`,
    /// recording each directory it creates in `created` (in creation order) so
    /// a failure can be undone.
    ///
    /// # Errors
    ///
    /// - [`CliError::InitScaffold`] if creating a directory fails.
    fn scaffold_directories(
        root: &Path,
        templates_dir: &Path,
        created: &mut Vec<PathBuf>,
    ) -> CliResult {
        let scaffold_error = |source| CliError::InitScaffold {
            root: root.to_path_buf(),
            source,
        };
        if templates_dir.metadata().is_ok_and(|meta| !meta.is_dir()) {
            return Err(scaffold_error(io::Error::from(
                io::ErrorKind::NotADirectory,
            )));
        }
        let traces_dir = root.join(LOCAL_CONFIG_DIR);
        fs::create_dir(&traces_dir).map_err(scaffold_error)?;
        created.push(traces_dir);

        let mut missing: Vec<&Path> = templates_dir
            .ancestors()
            .take_while(|dir| *dir != root && !dir.exists())
            .collect();
        missing.reverse();
        for dir in missing {
            fs::create_dir(dir).map_err(scaffold_error)?;
            created.push(dir.to_path_buf());
        }
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

    /// Removes the directories a failed run created, newest first. Never
    /// recursive: a directory that gained other content is left alone, which
    /// also protects a config file this run did not create.
    fn undo(created: &[PathBuf]) {
        for dir in created.iter().rev() {
            // Best effort: the original error is the one worth reporting.
            drop(fs::remove_dir(dir));
        }
    }
}

/// Errors raised while scaffolding a local config file on disk.
#[derive(Debug, Error)]
pub(crate) enum ConfigScaffoldError {
    /// The collected template and output directories could not be serialized to
    /// TOML.
    #[error("failed to serialize local config")]
    Serialize {
        /// Source TOML serialization error.
        #[source]
        source: toml::ser::Error,
    },
    /// The serialized local config could not be written to disk.
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

/// Serializes `directory`/`output_dir` as the local template config and writes
/// it to `root.join(LOCAL_CONFIG_FILE)`.
///
/// `[schemas]` and `[frontmatter]` are written as empty tables (their serde
/// defaults), so a freshly scaffolded config behaves identically to one that
/// omits those tables entirely.
///
/// Uses [`std::fs::File::create_new`] rather than [`std::fs::write`] so this
/// fails atomically if the file already exists, preventing a concurrent `traces
/// init` or a file planted between the existence check and write from being
/// silently clobbered.
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
    let path = root.join(LOCAL_CONFIG_FILE);
    let mut file = fs::File::create_new(&path).map_err(|source| {
        ConfigScaffoldError::Write {
            source,
        }
    })?;
    if let Err(source) = file.write_all(contents.as_bytes()) {
        // `create_new` succeeded, so a half-written file is ours to remove.
        drop(file);
        drop(fs::remove_file(&path));
        return Err(ConfigScaffoldError::Write {
            source,
        });
    }
    Ok(())
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
            assert!(preset.path().join("custom/templates").is_dir());
            assert!(!preset.path().join(".traces/templates").exists());
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

        #[rstest::rstest]
        #[case::absolute("/etc/templates")]
        #[case::parent("../outside")]
        #[case::nested_parent("a/../../outside")]
        fn rejects_invalid_template_directory_without_writing(
            #[case] input: &str,
        ) {
            let root = tempfile::tempdir().expect("create temp dir");
            let _guard = CwdGuard::enter(root.path());
            let provider = crate::PresetDialogProvider::new().with_text(input);

            let error = Init
                .run(&provider)
                .expect_err("invalid template directory fails init");

            assert!(matches!(error, CliError::InitInvalidTemplateDir { .. }));
            assert!(!root.path().join(".traces").exists());
        }

        #[cfg(unix)]
        #[test]
        fn rejects_symlink_escape_without_writing() {
            let outside = tempfile::tempdir().expect("create outside dir");
            let root = tempfile::tempdir().expect("create temp dir");
            std::os::unix::fs::symlink(
                outside.path(),
                root.path().join("link"),
            )
            .expect("create escaping symlink");
            let _guard = CwdGuard::enter(root.path());
            let provider =
                crate::PresetDialogProvider::new().with_text("link/templates");

            let error = Init
                .run(&provider)
                .expect_err("escaping template directory fails init");

            assert!(matches!(error, CliError::InitInvalidTemplateDir { .. }));
            assert!(!root.path().join(".traces").exists());
            assert!(!outside.path().join("templates").exists());
        }

        /// A failure after `.traces` exists must not block a re-run.
        #[test]
        fn cleans_up_after_a_mid_run_failure() {
            let root = tempfile::tempdir().expect("create temp dir");
            let _guard = CwdGuard::enter(root.path());
            // A parent that is a regular file lets `.traces` be created and
            // the nested template directory fail, on any platform and user.
            fs::write(root.path().join("docs"), "").expect("create blocker");
            let failing =
                crate::PresetDialogProvider::new().with_text("docs/templates");

            let error = Init.run(&failing).expect_err("blocked parent fails");

            assert!(matches!(error, CliError::InitScaffold { .. }));
            assert!(!root.path().join(".traces").exists());
            fs::remove_file(root.path().join("docs")).expect("remove blocker");
            Init.run(
                &crate::PresetDialogProvider::new().with_text("docs/templates"),
            )
            .expect("re-run succeeds after cleanup");
            assert!(root.path().join("docs/templates").is_dir());
        }

        #[test]
        fn rejects_a_template_directory_that_is_a_file_without_writing() {
            let root = tempfile::tempdir().expect("create temp dir");
            let _guard = CwdGuard::enter(root.path());
            fs::write(root.path().join("templates"), "").expect("create file");
            let provider =
                crate::PresetDialogProvider::new().with_text("templates");

            let error = Init.run(&provider).expect_err("a file is not a dir");

            assert!(matches!(error, CliError::InitScaffold { .. }));
            assert!(!root.path().join(".traces").exists());
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

    mod undo {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn undo_leaves_a_config_file_it_did_not_create() {
            let root = tempfile::tempdir().expect("create temp dir");
            let traces = root.path().join(".traces");
            fs::create_dir(&traces).expect("create .traces");
            let foreign = root.path().join(LOCAL_CONFIG_FILE);
            fs::write(&foreign, "planted").expect("plant config");

            Init::undo(std::slice::from_ref(&traces));

            assert_eq!(fs::read_to_string(&foreign).expect("read"), "planted");
            assert!(traces.is_dir());
        }
    }

    mod scaffold_local {
        use pretty_assertions::assert_eq;

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
            let written: RawConfig =
                toml::from_str(&contents).expect("parse written config");
            assert_eq!(
                written.templates.directory.as_deref(),
                Some(Path::new("templates"))
            );
            assert_eq!(
                written.templates.output_dir.as_deref(),
                Some(Path::new("notes"))
            );
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
