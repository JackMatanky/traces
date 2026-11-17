//! Coordinates template resolution, rendering, and file output.
//!
//! [`TemplateService`] manages the template pipeline for a [`Config`].
//!
//! - Resolves template input paths with [`TemplateLoader`].
//! - Reads template source files from disk.
//! - Renders template source with [`TemplateEngine`]. It uses the resolved
//!   absolute path as the template name to report the file and line number in
//!   errors.
//! - Resolves output paths and writes files with [`TemplateWriteTarget`].

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::{
    engine::{RenderOutput, TemplateEngine},
    error::TemplateError,
    loader::TemplateLoader,
    path::{DeclaredOutputPath, TemplatePath, TemplatePathInput},
    writer::{TemplateWriteTarget, WriteMode, WriteOutcome},
};
use crate::{Config, DialogProvider};

/// Resolves, renders, and writes templates for one configuration.
///
/// The service holds the configured template loader and `MiniJinja` engine. It
/// supports multiple renders with the same [`Config`].
///
/// # Interaction with [`WriteMode`]
///
/// [`WriteMode`] controls file writes, not prompt calls. The [`DialogProvider`]
/// receives every `ui.*` call in every mode. Select a suitable
/// [`DialogProvider`] to prevent interactive calls. Do not use
/// [`WriteMode::DryRun`] for this purpose.
pub struct TemplateService<'a> {
    config: &'a Config,
    loader: TemplateLoader,
    engine: TemplateEngine,
    provider: Arc<dyn DialogProvider>,
}

impl<'a> TemplateService<'a> {
    /// Creates a template service for `config` and `provider`.
    ///
    /// The provider handles all `ui.*` calls and interactive output collision
    /// prompts. [`WriteMode::DryRun`] skips file writes but keeps `ui.*` calls.
    ///
    /// # Errors
    ///
    /// - `TemplateError::SchemaLoad` if the service cannot load the Schema
    ///   registry.
    #[inline]
    pub fn new(
        config: &'a Config,
        provider: Arc<dyn DialogProvider>,
    ) -> Result<Self, TemplateError> {
        let loader = TemplateLoader::from(config);
        let engine =
            TemplateEngine::new(&loader, Arc::clone(&provider), config)?;
        Ok(Self {
            config,
            loader,
            engine,
            provider,
        })
    }

    /// Lists available template names for `config`.
    ///
    /// Returns top-level `.md` file stems from the local directory, then the
    /// global directory. It excludes local duplicates and skips missing or
    /// unreadable template directories.
    #[inline]
    #[must_use]
    pub fn list_available(config: &Config) -> Vec<String> {
        TemplateLoader::from(config).list_available()
    }

    /// Resolves and renders a template, then writes or previews the result.
    ///
    /// The service selects the output path in this order: `output`,
    /// `file.write_to()`, then the configured default. Dry-run mode returns the
    /// rendered content without resolving or checking the output path.
    ///
    /// # Arguments
    ///
    /// - `name`: template identifier to resolve and render.
    /// - `output`: optional output file path that overrides the configured
    ///   path.
    /// - `mode`: selects whether the service previews or writes the output.
    ///
    /// # Errors
    ///
    /// - `TemplateError::Resolve` if the template name does not resolve.
    /// - `TemplateError::Read` if the service cannot read the resolved template
    ///   source.
    /// - `TemplateError::Render` if `MiniJinja` cannot evaluate the template.
    /// - `TemplateError::OutputPathEscapesRoot` if the output path escapes the
    ///   project root.
    /// - `TemplateError::OutputPathUnverifiable` if `strict-path` cannot check
    ///   path confinement.
    /// - `TemplateError::Prompt` if a template prompt fails or you cancel it.
    /// - `TemplateError::OutputFileAlreadyExists` if the file exists under
    ///   [`CommitPolicy::CreateNew`].
    /// - `TemplateError::Write` if the service cannot create a directory or
    ///   file, or write the rendered content.
    ///
    /// [`CommitPolicy::CreateNew`]: super::writer::CommitPolicy::CreateNew
    #[inline]
    pub fn render_to_file(
        &self,
        name: &TemplatePathInput,
        output: Option<&Path>,
        mode: WriteMode,
    ) -> Result<WriteOutcome, TemplateError> {
        self.write(self.render(name)?, output, mode)
    }

    /// Resolves `name` and renders its source into a [`RenderedTemplate`].
    ///
    /// Runs the resolution and rendering steps of [`Self::render_to_file`]. It
    /// lets callers inspect rendered content before they call [`Self::write`].
    /// The split prevents repeated `ui.*` calls when callers use the rendered
    /// result more than once.
    ///
    /// The method gives the resolved absolute path to [`TemplateEngine`] as the
    /// template name. The [`minijinja::Error::name`] method then reports the
    /// path and line number when rendering fails.
    ///
    /// # Errors
    ///
    /// - [`Resolve`] if `name` does not resolve to a template file.
    /// - [`Read`] if the service cannot read the resolved template file.
    /// - [`Render`] if template evaluation or a template helper call fails.
    ///
    /// [`Resolve`]: TemplateError::Resolve
    /// [`Read`]: TemplateError::Read
    /// [`Render`]: TemplateError::Render
    #[inline]
    pub(crate) fn render(
        &self,
        name: &TemplatePathInput,
    ) -> Result<RenderedTemplate, TemplateError> {
        let resolved = self.loader.find(name)?;
        let resolved_path = resolved.absolute();
        let template_source = Self::read_template(&resolved)?;
        let rendered =
            self.render_template(&template_source, &resolved_path)?;
        Ok(RenderedTemplate {
            resolved,
            content: rendered.content,
            declared: rendered.write_to,
        })
    }

    /// Writes or previews a rendered template.
    ///
    /// Dry-run mode returns [`WriteOutcome::Previewed`] without resolving an
    /// output path. Commit mode selects the output path in this order:
    ///
    /// 1. Use the explicit `output` path.
    /// 2. Use the path from the template's `file.write_to()` call.
    /// 3. Use the default path from [`Self::default_output_path`].
    ///
    /// The service checks for an existing file atomically with
    /// [`std::fs::File::create_new`].
    ///
    /// # Arguments
    ///
    /// - `rendered`: template result from [`Self::render`].
    /// - `output`: optional output file path that overrides the configured
    ///   path.
    /// - `mode`: selects whether the service previews or writes the output.
    ///
    /// # Errors
    ///
    /// - [`OutputPathEscapesRoot`] if the declared or explicit path escapes the
    ///   project root. This error does not happen in dry-run mode.
    /// - [`OutputFileAlreadyExists`] if the target exists and `mode` uses
    ///   [`CommitPolicy::CreateNew`]. This error does not happen in dry-run
    ///   mode.
    /// - [`OutputPathUnverifiable`] if the service cannot check path
    ///   confinement. This error does not happen in dry-run mode.
    /// - [`Prompt`] if a collision prompt fails or you cancel it. This error
    ///   does not happen in dry-run mode.
    /// - [`Write`] if the service cannot create a directory or file, or write
    ///   the rendered content.
    ///
    /// [`OutputPathEscapesRoot`]: TemplateError::OutputPathEscapesRoot
    /// [`OutputPathUnverifiable`]: TemplateError::OutputPathUnverifiable
    /// [`OutputFileAlreadyExists`]: TemplateError::OutputFileAlreadyExists
    /// [`Prompt`]: TemplateError::Prompt
    /// [`Write`]: TemplateError::Write
    /// [`CommitPolicy::CreateNew`]: super::writer::CommitPolicy::CreateNew
    #[inline]
    pub(crate) fn write(
        &self,
        rendered: RenderedTemplate,
        output: Option<&Path>,
        mode: WriteMode,
    ) -> Result<WriteOutcome, TemplateError> {
        let WriteMode::Commit(policy) = mode else {
            return Ok(WriteOutcome::Previewed(rendered.content));
        };
        let target = TemplateWriteTarget::new(self.config.root())
            .with_requested(output)
            .with_declared(rendered.declared);
        let resolved_path = target.write(
            &rendered.content,
            policy,
            self.provider.as_ref(),
            || self.default_output_path(&rendered.resolved),
        )?;
        Ok(WriteOutcome::Written(resolved_path))
    }

    /// Reads the source text of `resolved` from disk and returns the file
    /// contents as a string.
    ///
    /// # Errors
    ///
    /// - [`Read`] if reading the template file fails.
    ///
    /// [`Read`]: TemplateError::Read
    fn read_template(resolved: &TemplatePath) -> Result<String, TemplateError> {
        resolved.read().map_err(|source| TemplateError::Read {
            path: resolved.absolute(),
            source,
        })
    }

    /// Renders `source` and uses `path` as the template name in errors.
    ///
    /// The method gives `path` to [`TemplateEngine`] as the template
    /// identifier. Syntax and prompt errors then report its absolute path and
    /// line number, not the default `<string>` name. The method does not read
    /// `path` again.
    ///
    /// # Errors
    ///
    /// - [`Render`] if template evaluation or an internal helper call fails.
    ///
    /// [`Render`]: TemplateError::Render
    fn render_template(
        &self,
        source: &str,
        path: &Path,
    ) -> Result<RenderOutput, TemplateError> {
        self.engine.render(source, &path.to_string_lossy()).map_err(|source| {
            TemplateError::Render {
                path: path.to_path_buf(),
                source,
            }
        })
    }

    /// Computes the default output path for `resolved`.
    ///
    /// The method joins [`Config::output_dir`] with the filename from
    /// [`TemplatePath::default_output_filename`]. This keeps templates with
    /// matching names in different directories separate. It treats the output
    /// directory as a trusted base, not as a user path.
    ///
    /// [`Config::output_dir`]: crate::Config::output_dir
    fn default_output_path(&self, resolved: &TemplatePath) -> PathBuf {
        self.config.output_dir().join(resolved.default_output_filename())
    }
}

impl std::fmt::Debug for TemplateService<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateService")
            .field("config", &self.config)
            .field("loader", &self.loader)
            .field("engine", &self.engine)
            .finish_non_exhaustive()
    }
}

/// Stores a rendered template and its resolved output metadata.
///
/// The value contains rendered content, resolved [`TemplatePath`] metadata, and
/// an optional path from `file.write_to()`. Pass it to
/// [`TemplateService::write`] to finish output without rendering again or
/// repeating interactive `ui.*` calls.
#[derive(Debug)]
pub(crate) struct RenderedTemplate {
    resolved: TemplatePath,
    content: String,
    declared: Option<DeclaredOutputPath>,
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{super::writer::CommitPolicy, *};
    use crate::PresetDialogProvider;

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let parent = path.parent().expect("template path parent");
        fs::create_dir_all(parent).expect("create template parent");
        fs::write(&path, content).expect("write template");
        path
    }

    fn input(path: &str) -> TemplatePathInput {
        TemplatePathInput::parse(Path::new(path)).expect("valid template input")
    }

    /// Creates a [`DialogProvider`] for tests that do not prompt.
    ///
    /// The provider satisfies the argument required by
    /// [`TemplateService::new`].
    fn preset_provider() -> Arc<dyn DialogProvider> {
        Arc::new(PresetDialogProvider::new())
    }

    /// Returns the output path from [`WriteOutcome::Written`].
    ///
    /// # Panics
    ///
    /// Panics if `outcome` is [`WriteOutcome::Previewed`].
    fn written_path(outcome: WriteOutcome) -> PathBuf {
        let written = match outcome {
            WriteOutcome::Written(path) => Some(path),
            WriteOutcome::Previewed(_) => None,
        };
        written.expect("render_to_file with dry_run: false always writes")
    }

    mod render_to_file {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn renders_minijinja_syntax() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(
                &local_dir,
                "daily.md",
                "{% for item in [\"a\", \"b\"] %}{{ item | upper }}{% endfor \
                 %}{% if 1 == 1 %}-ok{% else %}-no{% endif %}",
            );
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            let contents = fs::read_to_string(written_path(outcome))
                .expect("read written output");
            assert_eq!(contents, "AB-ok");
        }

        #[test]
        fn writes_under_the_configured_output_directory() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().join("project");
            let local_dir = root.join("templates");
            write_file(&local_dir, "daily.md", "hello");
            let config = Config::for_test(
                root.clone(),
                Some(local_dir),
                None,
                PathBuf::from("notes"),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            assert_eq!(
                outcome,
                WriteOutcome::Written(root.join("notes/daily.md"))
            );
        }

        #[test]
        fn output_path_preserves_the_resolved_templates_directory() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "nested/report.md", "hello");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(
                    &input("nested/report.md"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            assert_eq!(
                outcome,
                WriteOutcome::Written(temp.path().join("nested/report.md"))
            );
        }

        #[test]
        fn normalizes_extension_input_but_keeps_directory() {
            // Both `notes/daily` and `notes/daily.md` resolve to the same
            // output path. The resolved subdirectory remains in the path.
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "notes/daily.md", "hello");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");
            let expected =
                WriteOutcome::Written(temp.path().join("notes/daily.md"));

            assert_eq!(
                service
                    .render_to_file(
                        &input("notes/daily"),
                        None,
                        WriteMode::Commit(CommitPolicy::CreateNew)
                    )
                    .expect("render_to_file"),
                expected
            );
            assert_eq!(
                service
                    .render_to_file(
                        &input("notes/daily.md"),
                        None,
                        WriteMode::Commit(CommitPolicy::Overwrite)
                    )
                    .expect("render_to_file"),
                expected
            );
        }

        #[test]
        fn propagates_resolution_errors() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::test_default(temp.path());
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let error = service
                .render_to_file(
                    &input("missing"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err("missing template fails");

            assert!(matches!(error, TemplateError::Resolve(_)));
        }

        #[cfg(unix)]
        #[test]
        fn propagates_read_errors_when_the_resolved_file_is_unreadable() {
            use std::os::unix::fs::PermissionsExt as _;

            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            let file = write_file(&local_dir, "daily.md", "hello");
            fs::set_permissions(&file, fs::Permissions::from_mode(0o000))
                .expect("revoke read permission");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let error = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err("unreadable template file fails");

            assert!(matches!(error, TemplateError::Read { .. }));
        }

        #[test]
        fn propagates_render_errors_for_invalid_syntax() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "broken.md", "{% if %}");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let error = service
                .render_to_file(
                    &input("broken"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err("invalid syntax fails to render");

            assert!(matches!(error, TemplateError::Render { .. }));
        }

        #[test]
        fn render_errors_name_the_real_template_and_line_not_string() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            let template_path = write_file(
                &local_dir,
                "broken-query.md",
                "line one\n{{ query.from().sort(\"nope.bad\") }}\n",
            );
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let error = service
                .render_to_file(
                    &input("broken-query"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err("malformed sort field path fails to render");

            assert!(matches!(
                &error,
                TemplateError::Render { source, .. }
                    if source.name() == Some(template_path.to_string_lossy().as_ref())
                        && source.line() == Some(2)
            ));
        }

        #[test]
        fn propagates_write_errors_when_the_output_directory_cannot_be_created()
        {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "hello");
            // This file blocks creation of the output directory.
            // `fs::create_dir_all` fails when a path component exists as a
            // file.
            fs::write(temp.path().join("notes"), "not a directory")
                .expect("write blocking file");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                PathBuf::from("notes/output"),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let error = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err("output directory cannot be created");

            assert!(matches!(error, TemplateError::Write { .. }));
        }

        #[test]
        fn resolves_include_against_the_template_directory() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "partial.md", "included");
            write_file(&local_dir, "daily.md", "{% include \"partial.md\" %}!");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            assert_eq!(
                fs::read_to_string(written_path(outcome))
                    .expect("read written output"),
                "included!"
            );
        }

        #[test]
        fn output_flag_overrides_the_default_output_path() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "hello");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");
            let override_path = Path::new("elsewhere.md");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    Some(override_path),
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            let output_path = written_path(outcome);
            assert_eq!(output_path, temp.path().join("elsewhere.md"));
            assert_eq!(
                fs::read_to_string(&output_path).expect("read"),
                "hello"
            );
        }

        #[test]
        fn output_flag_rejects_an_absolute_path() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "hello");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");
            let outside = temp.path().join("outside.md");

            let error = service
                .render_to_file(
                    &input("daily"),
                    Some(&outside),
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err("absolute -o is rejected");

            assert!(matches!(
                error,
                TemplateError::OutputPathEscapesRoot { path } if path == outside
            ));
        }

        #[test]
        fn output_flag_rejects_a_parent_traversal() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "hello");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");
            let traversal = Path::new("../escape.md");

            let error = service
                .render_to_file(
                    &input("daily"),
                    Some(traversal),
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err("parent traversal -o is rejected");

            assert!(matches!(
                error,
                TemplateError::OutputPathEscapesRoot { .. }
            ));
        }

        #[test]
        fn write_to_rejects_a_parent_traversal() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(
                &local_dir,
                "daily.md",
                "{{ file.write_to(\"../../escape.md\") }}",
            );
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let error = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err("parent traversal write_to is rejected");

            assert!(matches!(
                error,
                TemplateError::OutputPathEscapesRoot { .. }
            ));
        }

        #[cfg(unix)]
        #[test]
        fn write_to_rejects_writing_through_a_symlink_that_escapes_root() {
            use std::os::unix::fs::symlink;

            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().to_path_buf();
            let local_dir = root.join("templates");
            let outside = tempfile::tempdir().expect("create outside dir");
            write_file(
                &local_dir,
                "daily.md",
                "{{ file.write_to(\"link/secret.md\") }}",
            );
            symlink(outside.path(), root.join("link"))
                .expect("plant a symlink inside root pointing outside it");
            let config =
                Config::for_test(root.clone(), Some(local_dir), None, root);
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let error = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err(
                    "write_to through a symlink escaping root is rejected",
                );

            assert!(matches!(
                error,
                TemplateError::OutputPathEscapesRoot { .. }
            ));
            assert!(
                !outside.path().join("secret.md").exists(),
                "nothing was written outside root through the symlink"
            );
        }

        #[test]
        fn output_flag_overrides_write_to() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(
                &local_dir,
                "daily.md",
                "{{ file.write_to(\"from-template.md\") }}",
            );
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");
            let cli_override = Path::new("from-cli.md");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    Some(cli_override),
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            assert_eq!(
                outcome,
                WriteOutcome::Written(temp.path().join("from-cli.md"))
            );
        }

        #[test]
        fn write_to_overrides_the_default_output_path() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(
                &local_dir,
                "daily.md",
                "{{ file.write_to(\"from-template.md\") }}",
            );
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            assert_eq!(
                outcome,
                WriteOutcome::Written(temp.path().join("from-template.md"))
            );
        }

        #[test]
        fn refuses_to_overwrite_an_existing_output_without_force() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "new content");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let existing = temp.path().join("daily.md");
            fs::write(&existing, "old content").expect("seed existing output");
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let error = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect_err("existing output without force fails");

            assert!(matches!(
                error,
                TemplateError::OutputFileAlreadyExists { path } if path == existing
            ));
            assert_eq!(
                fs::read_to_string(&existing).expect("read"),
                "old content"
            );
        }

        #[test]
        fn force_overwrites_an_existing_output_silently() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "new content");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let existing = temp.path().join("daily.md");
            fs::write(&existing, "old content").expect("seed existing output");
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::Overwrite),
                )
                .expect("force overwrites");

            assert_eq!(outcome, WriteOutcome::Written(existing.clone()));
            assert_eq!(
                fs::read_to_string(&existing).expect("read"),
                "new content"
            );
        }

        #[test]
        fn dry_run_renders_without_writing_or_checking_existence() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "{{ 1 + 1 }}");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let default_output = temp.path().join("daily.md");
            fs::write(&default_output, "old content")
                .expect("seed existing output");
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(&input("daily"), None, WriteMode::DryRun)
                .expect("dry run never checks existence, so it never fails");

            assert_eq!(outcome, WriteOutcome::Previewed("2".to_owned()));
            assert_eq!(
                fs::read_to_string(&default_output).expect("read"),
                "old content"
            );
        }

        #[test]
        fn dry_run_never_computes_or_confines_an_output_path() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "hello");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");
            let escaping = Path::new("../../escape.md");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    Some(escaping),
                    WriteMode::DryRun,
                )
                .expect(
                    "dry run never confines -o, so an escaping path never \
                     fails",
                );

            assert_eq!(outcome, WriteOutcome::Previewed("hello".to_owned()));
            assert!(!temp.path().join("../escape.md").exists());
        }

        #[test]
        fn ui_functions_render_and_delegate_to_the_provider() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(
                &local_dir,
                "daily.md",
                "{{ ui.text_input(\"name\", \"anon\") }}|{{ \
                 ui.confirm(\"proceed?\") }}|{{ ui.select(\"pick\", [\"a\", \
                 \"b\", \"c\"]) }}|{{ ui.multi_select(\"pick\", [\"x\", \
                 \"y\", \"z\"]) | join(\",\") }}",
            );
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let provider = Arc::new(
                PresetDialogProvider::new()
                    .with_text("claude")
                    .with_confirm(true)
                    .with_select(1)
                    .with_multi_select([0, 2]),
            );
            let service = TemplateService::new(&config, provider)
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            assert_eq!(
                fs::read_to_string(written_path(outcome))
                    .expect("read written output"),
                "claude|True|b|x,z"
            );
        }

        #[test]
        fn ui_select_and_multi_select_render_keyed_items_and_fall_back_to_to_string()
         {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(
                &local_dir,
                "daily.md",
                "{{ ui.select(\"country\", [{\"label\": \"US\", \"value\": \
                 1}, {\"label\": \"GB\", \"value\": 44}]).value }}|{{ \
                 ui.multi_select(\"country\", [{\"label\": \"US\", \"value\": \
                 1}, {\"label\": \"GB\", \"value\": 44}]) | \
                 map(attribute=\"value\") | join(\",\") }}|{{ \
                 ui.select(\"pick\", [10, 20, 30]) }}",
            );
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let provider = Arc::new(
                PresetDialogProvider::new()
                    .with_select(1)
                    .with_multi_select([0, 1])
                    .with_select(2),
            );
            let service = TemplateService::new(&config, provider)
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            assert_eq!(
                fs::read_to_string(written_path(outcome))
                    .expect("read written output"),
                "44|1,44|30"
            );
        }

        #[test]
        fn ui_select_and_multi_select_honor_a_custom_attribute() {
            // The `default` fallback content is tested against `label_items`
            // in `ui.rs`. A render exposes only the provider's label text, not
            // the recovered item.
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(
                &local_dir,
                "daily.md",
                "{{ ui.select(\"country\", [{\"name\": \"US\"}, {\"name\": \
                 \"GB\"}], attribute=\"name\").name }}|{{ \
                 ui.multi_select(\"country\", [{\"name\": \"US\"}, {\"name\": \
                 \"GB\"}], attribute=\"name\") | map(attribute=\"name\") | \
                 join(\",\") }}",
            );
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let provider = Arc::new(
                PresetDialogProvider::new()
                    .with_select(1)
                    .with_multi_select([0, 1]),
            );
            let service = TemplateService::new(&config, provider)
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(
                    &input("daily"),
                    None,
                    WriteMode::Commit(CommitPolicy::CreateNew),
                )
                .expect("render_to_file");

            assert_eq!(
                fs::read_to_string(written_path(outcome))
                    .expect("read written output"),
                "GB|US,GB"
            );
        }

        #[test]
        fn dry_run_still_uses_the_injected_provider_for_ui_calls() {
            // `WriteMode` controls file writes, not `ui.*` calls. A queued
            // `PresetDialogProvider` answer remains available in dry-run mode.
            // Therefore, `--dry-run` can preview output that depends on a
            // selection. The CLI's `--no-input` option prevents prompts before
            // it passes a provider to this service.
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "{{ ui.text_input(\"name\") }}");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let provider =
                Arc::new(PresetDialogProvider::new().with_text("claude"));
            let service = TemplateService::new(&config, provider)
                .expect("valid test schema directory");

            let outcome = service
                .render_to_file(&input("daily"), None, WriteMode::DryRun)
                .expect("render_to_file");

            assert_eq!(outcome, WriteOutcome::Previewed("claude".to_owned()));
        }
    }

    mod render {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn renders_content_and_leaves_declared_none_without_write_to() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(&local_dir, "daily.md", "hello {{ 1 + 1 }}");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let rendered = service.render(&input("daily")).expect("render");

            assert_eq!(rendered.content, "hello 2");
            assert_eq!(rendered.declared, None);
        }

        #[test]
        fn captures_a_declared_write_to() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("templates");
            write_file(
                &local_dir,
                "daily.md",
                "{{ file.write_to(\"from-template.md\") }}",
            );
            let config = Config::for_test(
                temp.path().to_path_buf(),
                Some(local_dir),
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let rendered = service.render(&input("daily")).expect("render");

            assert_eq!(
                rendered.declared,
                Some(DeclaredOutputPath::new(PathBuf::from(
                    "from-template.md"
                )))
            );
        }

        #[test]
        fn propagates_resolution_errors() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::for_test(
                temp.path().to_path_buf(),
                None,
                None,
                temp.path().to_path_buf(),
            );
            let service = TemplateService::new(&config, preset_provider())
                .expect("valid test schema directory");

            let error = service
                .render(&input("missing"))
                .expect_err("missing template fails");

            assert!(matches!(error, TemplateError::Resolve(_)));
        }
    }
}
