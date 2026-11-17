//! Builds the `MiniJinja` [`Environment`] that powers template rendering.
//!
//! [`TemplateEngine`] wraps a `MiniJinja` [`Environment`] and registers the
//! helper namespaces that templates can call. It also provides the [`render`]
//! method that evaluates a template source string.
//!
//! # Helper Namespaces
//!
//! | Namespace | Module | Registered By |
//! |-----------|--------|---------------|
//! | `file` | [`crate::template::engine::file`] | [`file::FileOps`] |
//! | `query` | [`crate::template::engine::query`] | [`query::QueryOps`] |
//! | `path` | [`crate::template::engine::path`] | [`path::PathOps`] |
//! | `ui` | [`crate::template::engine::ui`] | [`ui::UiOps`] |
//! | `date` | [`crate::template::engine::date`] | [`date::DateOps`] |
//! | `duration` | [`crate::template::engine::duration`] | [`duration::DurationOps`] |
//! | `str` | [`crate::template::engine::string`] | [`string::StrOps`] |
//! | `num` | [`crate::template::engine::num`] | [`num::NumOps`] |
//! | `yaml` | [`crate::template::engine::yaml`] | [`yaml::YamlOps`] |
//! | `schema` | [`crate::template::engine::schema`] | [`schema::SchemaOps`] |
//! | `uuid` | none | built-in filter |
//!
//! The `uuid` filter is registered directly on the [`Environment`] as a plain
//! function, not through an [`Object`] namespace.
//!
//! # Construction
//!
//! [`TemplateEngine::new`] builds the [`Environment`], registers the `uuid`
//! filter and all namespaces, then returns the ready-to-render engine. The
//! engine holds a reference to the [`TemplateLoader`] for `include` resolution
//! and a [`DialogProvider`] for `ui.*` helpers.
//!
//! # Rendering
//!
//! [`TemplateEngine::render`] evaluates `source` with the given `name` as the
//! template identifier. Errors carry the filename and line number so users can
//! locate failures in their template source.
//!
//! [`render`]: TemplateEngine::render
//! [`Environment`]: minijinja::Environment
//! [`Object`]: minijinja::value::Object

mod cache;
mod date;
mod duration;
mod error;
mod file;
mod num;
mod path;
mod query;
mod schema;
mod string;
mod ui;
mod yaml;

use std::{path::PathBuf, sync::Arc};

use minijinja::{Environment, Error};
use uuid::Uuid;

use self::{
    date::DateOps,
    duration::DurationOps,
    file::{FileOps, WRITE_TO_KEY},
    num::NumOps,
    path::PathOps,
    query::QueryOps,
    schema::SchemaOps,
    string::StrOps,
    ui::UiOps,
    yaml::YamlOps,
};
use super::{
    error::TemplateError, loader::TemplateLoader, path::DeclaredOutputPath,
};
use crate::{
    Config, DialogProvider, IndexerService,
    schema::{SchemaService, warn_schema_construction_diagnostics},
};

/// Renders template source with [`TemplateLoader`] support for `{% include %}`
/// and `{% extends %}`.
///
/// [`TemplateService`] keeps a [`TemplateLoader`] clone for inclusion
/// resolution. It builds the clone from the same [`Config`] as this engine, so
/// both use the same template directory order.
///
/// [`TemplateService`]: super::service::TemplateService
/// [`TemplateLoader`]: super::loader::TemplateLoader
/// [`Config`]: crate::Config
#[derive(Debug)]
pub(super) struct TemplateEngine {
    env: Environment<'static>,
}

impl TemplateEngine {
    /// Builds a [`TemplateEngine`] with `loader` and registers its helper
    /// functions and [`uuid`].
    ///
    /// Registers functions from the [`date`], [`duration`], [`mod@file`],
    /// [`path`], [`num`], [`query`], [`mod@schema`], [`string`], [`ui`], and
    /// [`mod@yaml`] submodules. Debug mode adds line and column locations to
    /// render errors.
    ///
    /// # Arguments
    ///
    /// - `loader`: The [`TemplateLoader`] used for `{% include %}` and `{%
    ///   extends %}` resolution.
    /// - `provider`: The [`DialogProvider`] implementation handling `ui.*`
    ///   calls.
    /// - `config`: Supplies the project root that confines file operations,
    ///   queries, and path inspections. It also supplies the `[schemas]` class
    ///   field that `query.from()`/`lists.from()`/`tasks.from()` source
    ///   expressions use, and the Schema registry directory for the `schema`
    ///   namespace.
    ///
    /// # Errors
    ///
    /// - [`SchemaLoad`] if reading or listing the registry directory fails,
    ///   parsing a Schema file fails, or the `extends` DAG contains a cycle.
    ///
    /// [`SchemaLoad`]: TemplateError::SchemaLoad
    /// [`uuid`]: fn@uuid
    /// [`DialogProvider`]: crate::DialogProvider
    /// [`TemplateLoader`]: super::loader::TemplateLoader
    #[inline]
    pub(super) fn new(
        loader: &TemplateLoader,
        provider: Arc<dyn DialogProvider>,
        config: &Config,
    ) -> Result<Self, TemplateError> {
        let mut env = Environment::new();
        // Debug mode supplies `minijinja::Error::range()` and
        // `template_source()`. `crate::cli::error` uses them to compute
        // template diagnostic locations in `render_error_location`. The engine
        // retains source text and the failing span only on error.
        env.set_debug(true);
        env.set_loader({
            let loader = loader.clone();
            move |name| loader.load(name)
        });
        let root = config.root_arc();
        let class_field = config.class_field_arc();
        // The engine resolves `SchemaService` once and shares it with
        // `QueryOps` below. The `query`/`lists`/`tasks` `.from()` helpers and
        // `schema.get()` use this same service for the engine's lifetime. They
        // do not resolve or cache it during a render.
        let schema_directory = config.schemas().directory();
        let construction = SchemaService::load_verbose(schema_directory)?;
        warn_schema_construction_diagnostics(&construction);
        let service = Arc::new(construction.service);
        let indexer = Arc::new(IndexerService::from(config));

        FileOps::new(Arc::clone(&root)).register(&mut env);
        QueryOps::page(
            Arc::clone(&indexer),
            &class_field,
            Arc::clone(&service),
        )
        .register(&mut env);
        QueryOps::list(
            Arc::clone(&indexer),
            &class_field,
            Arc::clone(&service),
        )
        .register(&mut env);
        QueryOps::task(
            Arc::clone(&indexer),
            &class_field,
            Arc::clone(&service),
        )
        .register(&mut env);
        QueryOps::register_terminal_filters(&mut env);
        PathOps::new(Arc::clone(&root)).register(&mut env);
        UiOps::new(provider).register(&mut env);
        DateOps::register(&mut env);
        DurationOps::register(&mut env);
        StrOps::register(&mut env);
        NumOps::register(&mut env);
        YamlOps::register(&mut env);
        SchemaOps::new(service).register(&mut env);
        env.add_function("uuid", uuid);
        Ok(Self {
            env,
        })
    }

    /// Compiles and renders `source` as template `name` with an empty context.
    ///
    /// Returns the rendered text and any path captured by `file.write_to()` in
    /// [`RenderOutput`]. The engine gives `name` to `MiniJinja` as the template
    /// identifier. Diagnostic errors then report that name instead of
    /// `<string>`. The engine collects `file.write_to()` state across the full
    /// render tree, including included and extended templates.
    ///
    /// # Errors
    ///
    /// - [`minijinja::Error`] if `source` fails to parse or render. The error
    ///   also happens if a referenced `{% include %}` or `{% extends %}`
    ///   template target fails to load or render.
    #[inline]
    pub(super) fn render(
        &self,
        source: &str,
        name: &str,
    ) -> Result<RenderOutput, Error> {
        let captured = self
            .env
            .template_from_named_str(name, source)?
            .render_captured(minijinja::context!())?;
        let write_to = captured
            .state()
            .get_temp(WRITE_TO_KEY)
            .and_then(|value| value.as_str().map(PathBuf::from))
            .map(DeclaredOutputPath::new);
        Ok(RenderOutput {
            content: captured.into_output(),
            write_to,
        })
    }
}

/// Holds the rendered template text and its optional declared output path.
#[derive(Debug)]
pub(super) struct RenderOutput {
    /// The rendered template content.
    pub(super) content: String,
    /// The output path set by `file.write_to()`, if invoked during rendering.
    pub(super) write_to: Option<DeclaredOutputPath>,
}

/// Generates a random UUID v4 string in RFC 4122 format.
///
/// The output format is `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`. The engine
/// registers this function as `uuid()` through [`TemplateEngine::new`], not
/// as a namespace-qualified helper such as [`mod@file`], [`ui`], or
/// [`date`].
fn uuid() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::*;

    fn loader_from_dir(path: &Path) -> TemplateLoader {
        TemplateLoader::new(Some(path.to_path_buf()), None)
    }

    /// Builds a [`Config`] rooted at `root` with no template directories and
    /// default `[schemas]` settings. Tests use it when they only need a project
    /// root for [`TemplateEngine::new`].
    fn config_for(root: &Path) -> Config {
        Config::test_default(root)
    }

    /// Creates a deterministic [`crate::DialogProvider`] for tests that do not
    /// call `ui.*` functions.
    fn preset_provider() -> Arc<dyn DialogProvider> {
        Arc::new(crate::PresetDialogProvider::new())
    }

    mod render {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn evaluates_minijinja_syntax() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{% for n in [1, 2] %}{{ n }}{% endfor %}", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.content, "12");
        }

        #[test]
        fn builtin_arithmetic_matches_jinja2_semantics() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render(
                    "{{ 7 // -2 }}|{{ 7 % -2 }}|{{ 2.5 | round }}",
                    "test.md",
                )
                .expect("render succeeds");

            assert_eq!(rendered.content, "-4|-1|2.0");

            let error = engine
                .render("{{ 1.0 / 0.0 }}", "test.md")
                .expect_err("division by zero fails");

            assert_eq!(error.kind(), minijinja::ErrorKind::InvalidOperation);
        }

        #[test]
        fn resolves_include_from_local_dir() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join("partial.md"), "included")
                .expect("write partial");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{% include \"partial.md\" %}!", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.content, "included!");
        }

        #[test]
        fn resolves_a_dot_prefixed_base_directory() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let dir = temp.path().join(".traces/templates");
            fs::create_dir_all(&dir).expect("create dotted template dir");
            fs::write(dir.join("daily.md"), "hello").expect("write template");
            let engine = TemplateEngine::new(
                &loader_from_dir(&dir),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{% include \"daily.md\" %}", "test.md")
                .expect("render");

            assert_eq!(rendered.content, "hello");
        }

        #[test]
        fn resolves_a_dot_prefixed_include_name() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join(".draft.md"), "secret")
                .expect("write template");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{% include \".draft.md\" %}", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.content, "secret");
        }

        #[test]
        fn falls_back_to_global_when_missing_from_local() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let local_dir = temp.path().join("local");
            let global_dir = temp.path().join("global");
            fs::create_dir_all(&local_dir).expect("create local dir");
            fs::create_dir_all(&global_dir).expect("create global dir");
            fs::write(global_dir.join("shared.md"), "from global")
                .expect("write template");
            let engine = TemplateEngine::new(
                &TemplateLoader::new(Some(local_dir), Some(global_dir)),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{% include \"shared.md\" %}", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.content, "from global");
        }

        #[test]
        fn stem_matches_an_include() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join("daily.md"), "hello")
                .expect("write template");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{% include \"daily\" %}", "test.md")
                .expect("extension-less include name is stem-matched");

            assert_eq!(rendered.content, "hello");
        }

        #[test]
        fn path_tests_and_filters_are_registered_and_resolve_against_root() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join("main.rs"), "fn main() {}")
                .expect("write fixture");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render(
                    "{{ 'main.rs' is path_exists }}-{{ 'missing.rs' is \
                     path_exists }}-{{ 'main.rs' is is_file_path }}-{{ '.' is \
                     is_dir_path }}-{{ '/foo/bar/main.rs' | path_basename \
                     }}-{{ '/foo/bar/main.rs' | path_extension }}-{{ \
                     '/foo/bar/main.rs' | path_parent }}",
                    "test.md",
                )
                .expect("render succeeds");

            assert_eq!(
                rendered.content,
                "True-False-True-True-main-rs-/foo/bar"
            );
        }
    }

    mod write_to {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn is_none_when_the_template_never_calls_it() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("no output path here", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.write_to, None);
        }

        #[test]
        fn captures_a_write_to_call() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{{ file.write_to(\"notes/daily.md\") }}", "test.md")
                .expect("render succeeds");

            assert_eq!(
                rendered.write_to,
                Some(DeclaredOutputPath::new(PathBuf::from("notes/daily.md")))
            );
        }

        #[test]
        fn does_not_leak_between_renders() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");
            engine
                .render("{{ file.write_to(\"first.md\") }}", "test.md")
                .expect("render succeeds");

            let rendered = engine
                .render("no write_to here", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.write_to, None);
        }

        #[test]
        fn calling_an_unknown_file_method_fails() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let error = engine
                .render("{{ file.move_to(\"x.md\") }}", "test.md")
                .expect_err("unknown method fails");

            assert_eq!(error.kind(), minijinja::ErrorKind::UnknownMethod);
        }
    }

    /// Checks that each namespace, filter, and function is accessible through
    /// [`TemplateEngine`].
    ///
    /// Each collaborator's tests check its full behavior (`file`, `date`,
    /// `string`, `ui`, `schema`).
    mod utilities {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn file_include_reads_relative_to_root() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join("snippet.md"), "inlined")
                .expect("write fixture");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{{ file.include(\"snippet.md\") }}", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.content, "inlined");
        }

        #[test]
        fn ui_confirm_is_reachable() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{{ ui.confirm(\"proceed?\") }}", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.content, "False");
        }

        #[test]
        fn date_now_is_reachable() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{{ date.now(format=\"%Y\") }}", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.content.len(), 4);
        }

        #[test]
        fn schema_get_is_reachable() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::create_dir_all(temp.path().join(".traces/schemas"))
                .expect("create schemas dir");
            fs::write(
                temp.path().join(".traces/schemas/book.toml"),
                "[fields.status]\ntype = \"select\"\nvalues = [\"reading\"]\n",
            )
            .expect("write schema fixture");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render(
                    "{{ schema.get('book').field('status') | join(',') }}",
                    "test.md",
                )
                .expect("render succeeds");

            assert_eq!(rendered.content, "reading");
        }

        #[test]
        fn uuid_function_returns_a_valid_v4_uuid() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{{ uuid() }}", "test.md")
                .expect("render succeeds");

            let parsed = ::uuid::Uuid::parse_str(&rendered.content)
                .expect("uuid() produces a parseable UUID");
            assert_eq!(parsed.get_version(), Some(::uuid::Version::Random));
        }

        #[test]
        fn case_filters_are_reachable() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render("{{ \"hello world\" | snake_case }}", "test.md")
                .expect("render succeeds");

            assert_eq!(rendered.content, "hello_world");
        }

        #[test]
        fn numeric_filters_are_reachable() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let rendered = engine
                .render(
                    "{{ 3.14 | ceil }} {{ 42 | sqrt }} {{ 3.14159 | \
                     num_format(2) }}",
                    "test.md",
                )
                .expect("render succeeds");

            assert_eq!(rendered.content, "4.0 6.48074069840786 3.14");
        }

        #[test]
        fn evaluates_from_yaml_filter() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let template = "{% set text = \"title: Hello\" %}{{ text | \
                            from_yaml | to_yaml | trim }}";
            let rendered =
                engine.render(template, "test.md").expect("render succeeds");

            assert_eq!(rendered.content, "title: Hello");
        }

        #[test]
        fn evaluates_to_yaml_filter_on_collections() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let engine = TemplateEngine::new(
                &loader_from_dir(temp.path()),
                preset_provider(),
                &config_for(temp.path()),
            )
            .expect("valid test schema directory");

            let template = "{% set items = [\"apple\", \"banana\"] %}{{ items \
                            | to_yaml | trim }}";
            let rendered =
                engine.render(template, "test.md").expect("render succeeds");

            assert_eq!(rendered.content, "- apple\n- banana");
        }
    }
}
