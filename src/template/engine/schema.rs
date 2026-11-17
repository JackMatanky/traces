//! Registers the `schema` namespace of template helpers.
//!
//! [`SchemaOps`] is the value of the `schema` global that
//! [`super::TemplateEngine`] registers. It exposes one Template Helper:
//!
//! - `schema.get(name)` binds the [`Schema`] that resolves under that name. The
//!   call fails the render when no Schema resolves under that name. [`Schema`]
//!   implements the [`Object`] trait itself, in the same way as the `QuerySet`
//!   of [`super::query`].
//!
//! The bound [`Schema`] exposes one attribute and three Template Helpers:
//!
//! - `.name`: the Schema name, read from the source file stem.
//! - `.field(name)`: the selectable values of the named field. A `select` field
//!   returns plain strings for a simple string list and objects with `value`,
//!   `label`, and extra keys for structured inline values and values files. A
//!   `file` field returns a [`SourceSelector`] filter. That filter is
//!   declarative data: building it runs no query. Templates combine it with
//!   `query.from(...)` and `| with_children` or `| with_descendants`. Every
//!   other field type returns `none`.
//! - `.children()`: the Schemas that directly `extend` this Schema. Each result
//!   is a bound [`Schema`]. The result is empty, not an error, when nothing
//!   directly extends this Schema.
//! - `.descendants()`: the Schemas that `extend` this Schema at any depth. Each
//!   result is a bound [`Schema`], so a template walks the whole subtree
//!   through `.name`, `.field(...)`, `.children()`, and `.descendants()` again.
//!   The result is empty, not an error, when nothing extends this Schema.
//!
//! `schema.get` and `.field` are structural references. A typo in either name
//! fails the render with template context instead of panicking. Class-based
//! references (`from_class` and `file`-field filters) degrade a missing class
//! to exact matching and log a warning. Structural Schema and field names
//! always fail the render.
//!
//! # Resolution Timing
//!
//! [`super::TemplateEngine`] loads and resolves every Schema TOML file once,
//! during its own construction. Renders never re-resolve the registry. Every
//! render of that engine shares one `Arc<SchemaService>`, and [`Schema`] values
//! share the field map behind that service instead of deep-cloning it per call.
//!
//! [`Schema`] carries no context fields in its [`Object`] implementation. Each
//! call to `.field()`, `.children()`, or `.descendants()` reads the render's
//! `Arc<SchemaService>` from the typed extension storage of [`State`], which
//! the `schema.get` branch seeds on first use (see [`cached_service`]). The
//! extension stores the shared service only; it never resolves the registry.
//! `.children()` and `.descendants()` do not memoize results: each call
//! rereads the shared service and the target Schema's precomputed
//! `children`/`descendants` set, with no registry scan per call.
//!
//! # Examples
//!
//! ```text
//! {% set book = schema.get('book') %}
//! {{ book.field('status') | join(', ') }}
//! {{ book.children() | map(attribute='name') | join(', ') }}
//! ```

use std::{collections::BTreeSet, sync::Arc};

use indexmap::IndexMap;
use minijinja::{
    Environment, Error, ErrorKind, State,
    value::{Enumerator, Object, Serde, Value},
};

use super::error::TemplateEngineResult;
use crate::{
    FieldValue,
    query::{ClassExpansionMode, SourceAtom, SourceExpr, SourceSelector},
    schema::{
        Schema, SchemaFileFieldRef, SchemaSelectFieldEntry, SchemaService,
    },
};

/// Lists the method names that the `schema` global exposes.
const METHODS: &[&str] = &["get"];

/// Lists the keys that a bound [`Schema`] exposes.
///
/// `field`, `children`, and `descendants` are Template Helpers, and `name` is a
/// plain attribute. [`Object::enumerate`] returns this list for [`Schema`].
const SCHEMA_METHODS: &[&str] = &["children", "descendants", "field", "name"];

/// Carries the shared [`SchemaService`] of one render.
///
/// The extension lets the [`Object`] implementation of [`Schema`] reach
/// [`SchemaService::matches`], [`SchemaService::children_of`], and
/// [`SchemaService::descendants_of`] without a context field. The `"get"`
/// branch of [`SchemaOps::get_value`] seeds it before it returns any [`Schema`]
/// value, so every live [`Schema`] value in a render finds it already present.
/// The first insertion wins: every seed in a render holds the same shared
/// service, so any of them is equivalent.
#[derive(Debug)]
struct SchemaServiceCache(Arc<SchemaService>);

/// Represents the `schema` namespace of the template environment.
#[derive(Debug)]
pub(super) struct SchemaOps {
    service: Arc<SchemaService>,
}

impl SchemaOps {
    /// Wraps the shared [`SchemaService`] that the `schema` helpers read.
    #[inline]
    #[must_use]
    pub(super) const fn new(service: Arc<SchemaService>) -> Self {
        Self {
            service,
        }
    }

    /// Registers the namespace object as the `schema` global in `env`.
    #[inline]
    pub(super) fn register(self, env: &mut Environment<'static>) {
        env.add_global("schema", Value::from_object(self));
    }
}

impl Object for SchemaOps {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        match key.as_str()? {
            "get" => {
                let ops = Arc::clone(self);
                Some(Value::from_function(
                    move |state: &mut State,
                          name: &str|
                          -> TemplateEngineResult<Value> {
                        state.get_or_insert_extension_with(|| {
                            SchemaServiceCache(Arc::clone(&ops.service))
                        });
                        ops.service
                            .get(name)
                            .cloned()
                            .map(Value::from_dyn_object)
                            .ok_or_else(|| {
                                Error::new(
                                    ErrorKind::InvalidOperation,
                                    format!("unknown Schema {name:?}"),
                                )
                            })
                    },
                ))
            }
            _ => None,
        }
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Str(METHODS)
    }
}

/// Returns the shared [`SchemaService`] of the current render.
///
/// The `"get"` branch of [`SchemaOps::get_value`] seeds the extension before it
/// returns the first [`Schema`] value of the render.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if no `schema.get` call has run in this
///   render.
///
/// [`ErrorKind::InvalidOperation`]: minijinja::ErrorKind::InvalidOperation
fn cached_service(state: &State) -> TemplateEngineResult<Arc<SchemaService>> {
    state
        .get_extension::<SchemaServiceCache>()
        .map(|cached| Arc::clone(&cached.0))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidOperation,
                "internal error: no Schema service cached for this render",
            )
        })
}

impl Object for Schema {
    #[inline]
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        match key.as_str()? {
            "name" => Some(Value::from(self.name())),
            "field" => {
                let schema = Arc::clone(self);
                Some(Value::from_function(
                    move |name: &str| -> TemplateEngineResult<Value> {
                        let field = schema.field(name).ok_or_else(|| {
                            unknown_field_error(&schema, name)
                        })?;
                        if let Some(filter) = field.file_filter() {
                            let source = file_field_source(&filter)
                                .map_err(glob_compile_error)?;
                            return Ok(Value::from_object(source));
                        }
                        Ok(field.select_values().map_or_else(
                            || Value::from(()),
                            |values| {
                                Value::from(
                                    values
                                        .iter()
                                        .map(select_entry_value)
                                        .collect::<Vec<_>>(),
                                )
                            },
                        ))
                    },
                ))
            }
            "children" => {
                let schema = Arc::clone(self);
                Some(Value::from_function(
                    move |state: &State| -> TemplateEngineResult<Value> {
                        bind_related(state, &schema, SchemaService::children_of)
                    },
                ))
            }
            "descendants" => {
                let schema = Arc::clone(self);
                Some(Value::from_function(
                    move |state: &State| -> TemplateEngineResult<Value> {
                        bind_related(
                            state,
                            &schema,
                            SchemaService::descendants_of,
                        )
                    },
                ))
            }
            _ => None,
        }
    }

    #[inline]
    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Str(SCHEMA_METHODS)
    }
}

/// Names the [`SchemaService`] hierarchy query that [`bind_related`] runs.
///
/// [`SchemaService::children_of`] serves `.children()`, and
/// [`SchemaService::descendants_of`] serves `.descendants()`.
type RelateFn = fn(&SchemaService, &str) -> Vec<Arc<Schema>>;

/// Binds the Schemas that `relate` finds for `schema`.
///
/// Each result becomes a bound [`Schema`] value in a list. [`bind_related`]
/// implements `.children()` and `.descendants()`; those helpers differ only
/// in the [`SchemaService`] query they select.
///
/// # Errors
///
/// - [`ErrorKind::InvalidOperation`] if the render has no cached Schema
///   service, which means `schema.get` has not run (see [`cached_service`]).
///
/// [`ErrorKind::InvalidOperation`]: minijinja::ErrorKind::InvalidOperation
fn bind_related(
    state: &State,
    schema: &Arc<Schema>,
    relate: RelateFn,
) -> TemplateEngineResult<Value> {
    let service = cached_service(state)?;
    let related = relate(&service, schema.name());
    Ok(Value::from(
        related.into_iter().map(Value::from_dyn_object).collect::<Vec<_>>(),
    ))
}

/// Converts a `select`-field entry into the value that `.field()` returns.
///
/// Returns a plain string when the label equals the value and the entry
/// carries no extra data. Returns a `{value, label, ...extra}` object for a
/// structured inline value or an entry from a values file.
fn select_entry_value(entry: &SchemaSelectFieldEntry) -> Value {
    if entry.label() == entry.value() && entry.extra().is_empty() {
        return Value::from(Serde(entry.value()));
    }
    let mut object: IndexMap<String, FieldValue> = entry.extra().clone();
    object.insert("value".to_owned(), entry.value().clone());
    object.insert("label".to_owned(), entry.label().clone());
    Value::from(Serde(&object))
}

/// Builds a [`SourceSelector`] filter from a `file` field.
///
/// The builder omits empty `folders` and `class` values instead of turning
/// them into always-true terms. A field that declares only classes therefore
/// narrows to Notes of those classes instead of matching every file. The
/// [`SourceAtom::Class`] term keeps an empty match set here: it has the same
/// unresolved shape as `@Book` or `class(Name)` from DSL text.
/// [`SourceSelector::resolve_classes`] fills that set in later, and that pass
/// also resolves the class atoms that DSL text produced.
///
/// # Errors
///
/// - [`regex::Error`] if a folder or extension glob fails to compile.
fn file_field_source(
    filter: &SchemaFileFieldRef<'_>,
) -> Result<SourceSelector, regex::Error> {
    let SchemaFileFieldRef {
        folders,
        ext,
        class,
    } = *filter;
    let mut terms = Vec::new();
    if let [first, rest @ ..] = folders {
        let first_glob = glob_for(first, ext)?;
        let rest_globs = rest
            .iter()
            .map(|folder| glob_for(folder, ext))
            .collect::<Result<Vec<_>, _>>()?;
        terms.push(SourceExpr::or(first_glob, rest_globs));
    } else if ext.is_some() {
        terms.push(SourceExpr::atom(glob_for("", ext)?));
    } else {
        // No folder and no extension: the path term stays empty, so the
        // class term (or `All`) sets the match.
    }
    if !class.is_empty() {
        terms.push(SourceExpr::atom(SourceAtom::Class {
            names: class.to_vec(),
            mode: ClassExpansionMode::Exact(BTreeSet::new()),
        }));
    }
    let mut terms = terms.into_iter();
    Ok(terms.next().map_or_else(
        || SourceSelector::All,
        |first| SourceSelector::Expr(SourceExpr::and(first, terms.collect())),
    ))
}

/// Builds the glob that matches files under `folder` with extension `ext`.
///
/// An empty `folder` matches every file in the project. The glob follows the
/// same compilation path as DSL glob text, so a `file`-field filter composes
/// like hand-written source.
///
/// # Errors
///
/// - [`regex::Error`] if the built glob fails to compile.
fn glob_for(
    folder: &str,
    ext: Option<&str>,
) -> Result<SourceAtom, regex::Error> {
    let suffix = ext.map_or_else(String::new, |ext| format!(".{ext}"));
    let glob = if folder.is_empty() {
        format!("**{suffix}")
    } else {
        format!("{folder}/**{suffix}")
    };
    SourceAtom::path(&glob)
}

/// Converts a glob compilation error into a [`minijinja::Error`].
///
/// The result uses [`ErrorKind::InvalidOperation`] and keeps the original
/// [`regex::Error`] as its [`source`].
///
/// [`source`]: std::error::Error::source
fn glob_compile_error(source: regex::Error) -> Error {
    super::error::invalid_operation("failed to compile file field glob", source)
}

/// Builds the error for a field name that `schema` does not define.
///
/// Appends a `; did you mean "..."?` hint when [`Schema::suggest_field`]
/// finds exactly one candidate. The hint changes the message only: the
/// lookup still fails.
fn unknown_field_error(schema: &Schema, field: &str) -> Error {
    let base = format!("schema {:?} has no field {field:?}", schema.name());
    let message = schema.suggest_field(field).map_or_else(
        || base.clone(),
        |name| format!("{base}; did you mean {name:?}?"),
    );
    Error::new(ErrorKind::InvalidOperation, message)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use minijinja::Environment;

    use super::*;
    use crate::schema::{SchemaError, SchemaFileError};

    /// Returns an [`Environment`] with `schema` registered against
    /// `directory`.
    fn env(directory: &Path) -> Environment<'static> {
        let mut env = Environment::new();
        schema_ops(directory).register(&mut env);
        env
    }

    fn schema_ops(directory: &Path) -> SchemaOps {
        SchemaOps::new(Arc::new(
            SchemaService::new(directory).expect("valid test schema directory"),
        ))
    }

    fn render(directory: &Path, source: &str) -> Result<String, Error> {
        env(directory).render_str(source, minijinja::context!())
    }

    /// Returns an [`Environment`] that adds the `query` global and the
    /// terminal query filters to [`env`].
    ///
    /// The extra globals cover the `query.from(...)` calls that a test
    /// template needs to compose with a `file`-field [`SourceSelector`].
    fn full_env(directory: &Path) -> Environment<'static> {
        let root: Arc<Path> = Arc::from(
            directory.parent().and_then(Path::parent).unwrap_or(directory),
        );
        let service = Arc::new(
            SchemaService::new(directory).expect("valid test schema directory"),
        );
        let mut env = Environment::new();
        crate::template::engine::QueryOps::page(
            Arc::new(crate::IndexerService::for_tests(root.as_ref())),
            "class",
            Arc::clone(&service),
        )
        .register(&mut env);
        crate::template::engine::QueryOps::register_terminal_filters(&mut env);
        SchemaOps::new(service).register(&mut env);
        env
    }

    fn render_full(directory: &Path, source: &str) -> Result<String, Error> {
        full_env(directory).render_str(source, minijinja::context!())
    }

    use crate::test_support::{write_note, write_schema};

    mod get_value {
        use super::*;

        #[test]
        fn returns_none_for_an_unknown_key() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let ops = Arc::new(schema_ops(temp.path()));

            assert!(ops.get_value(&Value::from("unknown")).is_none());
        }

        #[test]
        fn returns_none_for_a_non_string_key() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let ops = Arc::new(schema_ops(temp.path()));

            assert!(ops.get_value(&Value::from(1)).is_none());
        }
    }

    mod enumerate {
        use super::*;

        #[test]
        fn lists_every_method() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let ops = Arc::new(schema_ops(temp.path()));

            assert!(matches!(ops.enumerate(), Enumerator::Str(METHODS)));
        }

        #[test]
        fn every_enumerated_method_resolves_via_get_value() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let ops = Arc::new(schema_ops(temp.path()));

            for method in METHODS {
                assert!(
                    ops.get_value(&Value::from(*method)).is_some(),
                    "{method:?} is enumerated but get_value has no matching \
                     arm"
                );
            }
        }
    }

    mod get {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn binds_a_resolved_schema_by_name() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.status]
                type = "select"
                values = ["reading", "read"]
                "#,
            );

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').field('status') | join(',') }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "reading,read");
        }

        #[test]
        fn resolves_inheritance_through_extends() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.status]
                type = "select"
                values = ["reading", "read"]
                "#,
            );
            write_schema(
                temp.path(),
                "sci_fi",
                r#"
                extends = ["book"]
                "#,
            );

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('sci_fi').field('status') | join(',') }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "reading,read");
        }

        #[test]
        fn two_calls_in_the_same_render_both_resolve_the_same_schema() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.status]
                type = "select"
                values = ["reading"]
                "#,
            );

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').field('status') | join(',') }}-{{ \
                 schema.get('book').field('status') | join(',') }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "reading-reading");
        }
    }

    mod warnings {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn a_missing_extends_target_degrades_with_a_warning_instead_of_failing_the_render()
         {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "sci_fi",
                r#"
                extends = ["does_not_exist"]

                [fields.status]
                type = "select"
                values = ["reading"]
                "#,
            );

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('sci_fi').field('status') | join(',') }}",
            )
            .expect(
                "a degraded extends target should warn, not fail the render",
            );

            assert_eq!(rendered, "reading");
        }
    }

    mod field {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_none_for_a_non_list_field_type() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.title]
                type = "input"
                "#,
            );

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').field('title') is none }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "True");
        }

        #[test]
        fn file_field_composes_with_query_from_to_select_matching_files() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.cover]
                type = "file"
                folders = ["covers"]
                ext = "md"
                class = ["book"]
                "#,
            );
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);
            write_note(
                temp.path(),
                "covers/dune.md",
                "---\naliases:\n  - Friendly Dune\nclass: sci_fi\n---\n",
            );
            write_note(
                temp.path(),
                "covers/plain.md",
                "---\nclass: book\n---\n",
            );
            write_note(
                temp.path(),
                "covers/titled.md",
                "---\ntitle: Titled Cover\nclass: book\n---\n",
            );
            write_note(
                temp.path(),
                "misc/ignored.md",
                "---\nclass: book\n---\n",
            );
            write_note(temp.path(), "covers/ignored.txt", "");

            // `class = ["book"]` keeps the exact-match depth that a bare
            // `@Book`/`class(Book)` DSL atom gets. `covers/dune.md` (class
            // `sci_fi`, a `book` descendant) stays excluded: `.field()`
            // builds an unresolved `Exact` atom and does not expand classes
            // itself. `misc/ignored.md` (wrong folder) and
            // `covers/ignored.txt` (wrong extension) are excluded by the
            // glob.
            let rendered = render_full(
                &temp.path().join(".traces/schemas"),
                "{% for item in query.from(schema.get('book').field('cover')) \
                 %}{{ item.file.name }}{% if not loop.last %}|{% endif %}{% \
                 endfor %}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "plain|titled");
        }

        #[test]
        fn file_field_reaches_a_non_markdown_file_with_no_note() {
            // A `file` field matches a non-Markdown file that has no parsed
            // Note. `.field()` builds that selector without running a query;
            // only `query.from(...)` runs one, through the shared
            // resolution path.
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.cover]
                type = "file"
                folders = ["covers"]
                ext = "jpg"
                "#,
            );
            write_note(temp.path(), "covers/a.jpg", "");
            write_note(temp.path(), "notes/b.md", "# Unrelated Note");

            let rendered = render_full(
                &temp.path().join(".traces/schemas"),
                "{% for item in query.from(schema.get('book').field('cover')) \
                 %}{{ item.file.path }}{% endfor %}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "covers/a.jpg");
        }

        #[test]
        fn file_field_widens_to_descendants_via_with_descendants_filter() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.cover]
                type = "file"
                folders = ["covers"]
                ext = "md"
                class = ["book"]
                "#,
            );
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);
            write_note(
                temp.path(),
                "covers/dune.md",
                "---\nclass: sci_fi\n---\n",
            );
            write_note(
                temp.path(),
                "covers/plain.md",
                "---\nclass: book\n---\n",
            );

            let rendered = render_full(
                &temp.path().join(".traces/schemas"),
                "{% for item in query.from(schema.get('book').field('cover') \
                 | with_descendants) %}{{ item.file.name }}{% if not \
                 loop.last %}|{% endif %}{% endfor %}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "dune|plain");
        }

        #[test]
        fn file_field_refreshes_between_renders() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let schemas_dir = temp.path().join(".traces/schemas");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.cover]
                type = "file"
                folders = ["covers"]
                ext = "md"
                "#,
            );
            let env = full_env(&schemas_dir);
            let source = "{% for item in \
                          query.from(schema.get('book').field('cover')) %}{{ \
                          item.file.path }}{% if not loop.last %},{% endif \
                          %}{% endfor %}";
            write_note(temp.path(), "covers/first.md", "");

            let first = env
                .render_str(source, minijinja::context!())
                .expect("first render succeeds");
            write_note(temp.path(), "covers/second.md", "");
            let second = env
                .render_str(source, minijinja::context!())
                .expect("second render succeeds");

            assert_eq!(first, "covers/first.md");
            assert_eq!(second, "covers/first.md,covers/second.md");
        }
    }

    mod name {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_the_schemas_own_name() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "book", "");

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').name }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "book");
        }
    }

    mod children {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_a_direct_extender_as_a_bound_schema() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "thing", "");
            write_schema(temp.path(), "book", r#"extends = ["thing"]"#);
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('thing').children() | map(attribute='name') | \
                 join(',') }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "book");
        }

        #[test]
        fn excludes_a_transitive_descendant() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "thing", "");
            write_schema(temp.path(), "book", r#"extends = ["thing"]"#);
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('thing').children() | length }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "1");
        }

        #[test]
        fn returns_an_empty_list_for_a_leaf_schema_not_none_or_an_error() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "book", "");
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('sci_fi').children() | length }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "0");
        }
    }

    mod descendants {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_a_direct_extender_as_a_bound_schema() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "book", "");
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').descendants() | map(attribute='name') \
                 | join(',') }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "sci_fi");
        }

        #[test]
        fn returns_a_transitive_descendant_through_an_intermediate_schema() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "thing", "");
            write_schema(temp.path(), "book", r#"extends = ["thing"]"#);
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('thing').descendants() | map(attribute='name') \
                 | join(',') }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "book,sci_fi");
        }

        #[test]
        fn returns_an_empty_list_for_a_leaf_schema_not_none_or_an_error() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "book", "");
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('sci_fi').descendants() | length }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "0");
        }

        #[test]
        fn a_descendant_still_resolves_its_own_fields() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.status]
                type = "select"
                values = ["reading"]
                "#,
            );
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').descendants()[0].field('status') | \
                 join(',') }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "reading");
        }

        #[test]
        fn a_descendants_own_descendants_are_also_reachable() {
            // The `descendants` branch threads the cached service of the
            // render into every bound `Schema` that it returns: it calls
            // `bind_related`, which calls `cached_service`, and it never
            // builds a fresh or empty service. The service-level tests
            // already prove that the data is transitively correct; this
            // test proves that the render-facing chain
            // `.descendants().descendants()` works as the module docs
            // describe.
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "thing", "");
            write_schema(temp.path(), "book", r#"extends = ["thing"]"#);
            write_schema(temp.path(), "sci_fi", r#"extends = ["book"]"#);

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('thing').descendants()[0].descendants() | \
                 map(attribute='name') | join(',') }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "sci_fi");
        }
    }

    mod errors {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn schema_get_of_an_unknown_schema_surfaces_as_a_render_error_not_a_panic()
         {
            let temp = tempfile::tempdir().expect("create temp dir");

            let error = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('missing') }}",
            )
            .expect_err("unknown Schema should error");

            assert!(error.to_string().contains("unknown Schema"));
        }

        #[test]
        fn unknown_schema_error_carries_the_template_name_and_line() {
            // The error carries template context (name and line), not only
            // a message. The shared `render()` helper does not assert on
            // that context, so this test asserts it directly. Minijinja
            // records the name and line in every build; only the
            // byte-accurate column data that `crate::cli::error` reads
            // needs `set_debug(true)`. The test turns debug on to match the
            // wiring of `TemplateEngine::new`.
            let temp = tempfile::tempdir().expect("create temp dir");
            let mut env = Environment::new();
            env.set_debug(true);
            schema_ops(&temp.path().join(".traces/schemas")).register(&mut env);

            let error = env
                .render_named_str(
                    "note.md",
                    "line one\n{{ schema.get('missing') }}\n",
                    minijinja::context!(),
                )
                .expect_err("unknown Schema should error");

            assert_eq!(error.name(), Some("note.md"));
            assert_eq!(error.line(), Some(2));
        }

        #[test]
        fn field_of_an_unknown_field_surfaces_as_a_render_error_not_a_panic() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.title]
                type = "input"
                "#,
            );

            let error = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').field('missing') }}",
            )
            .expect_err("unknown field should error");

            assert!(error.to_string().contains("has no field"));
        }

        #[test]
        fn unknown_field_with_a_canonical_typo_suggests_the_exact_field() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.status]
                type = "select"
                values = ["draft"]
                "#,
            );

            let error = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').field('Status') }}",
            )
            .expect_err("unknown field should error");

            assert!(error.to_string().contains(r#"did you mean "status"?"#));
        }

        #[test]
        fn unknown_field_with_an_edit_distance_typo_suggests_the_closest_field()
        {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.status]
                type = "select"
                values = ["draft"]
                "#,
            );

            let error = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').field('statu') }}",
            )
            .expect_err("unknown field should error");

            assert!(error.to_string().contains(r#"did you mean "status"?"#));
        }

        #[test]
        fn unknown_field_with_no_close_candidate_has_no_suggestion() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.status]
                type = "select"
                values = ["draft"]
                "#,
            );

            let error = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').field('completely_unrelated') }}",
            )
            .expect_err("unknown field should error");

            let message = error.to_string();
            assert!(message.contains("has no field"));
            assert!(!message.contains("did you mean"));
        }

        #[test]
        fn a_malformed_schema_file_fails_construction_not_a_panic() {
            // Schema resolution happens once at construction, in
            // `SchemaService::new`, not on the first `schema.get` call. A
            // malformed Schema TOML therefore fails construction instead of
            // appearing as a render error.
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "book", "not valid toml [[[");

            let error =
                SchemaService::new(&temp.path().join(".traces/schemas"))
                    .expect_err(
                        "malformed Schema TOML fails construction, not a panic",
                    );

            assert!(matches!(error.as_error(), SchemaError::File {
                source: SchemaFileError::Parse(_),
                ..
            }));
        }

        #[test]
        fn a_broken_sibling_schema_still_breaks_construction() {
            // One malformed sibling Schema fails the whole registry. The
            // failure affects every render, even a render whose template
            // never uses `schema`.
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.status]
                type = "select"
                values = ["reading"]
                "#,
            );
            write_schema(temp.path(), "broken", "not valid toml [[[");

            let error =
                SchemaService::new(&temp.path().join(".traces/schemas"))
                    .expect_err(
                        "a broken sibling Schema fails the whole registry load",
                    );

            assert!(matches!(error.as_error(), SchemaError::File {
                source: SchemaFileError::Parse(_),
                ..
            }));
        }
    }

    mod register {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn register_makes_schema_reachable_through_a_real_environment() {
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.status]
                type = "select"
                values = ["reading"]
                "#,
            );

            let rendered = render(
                &temp.path().join(".traces/schemas"),
                "{{ schema.get('book').field('status') | join(',') }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "reading");
        }
        #[test]
        fn renders_structured_file_sourced_select_field_as_objects() {
            use pretty_assertions::assert_eq;

            let temp = tempfile::tempdir().expect("create temp dir");
            let schemas_dir = temp.path().join(".traces/schemas");
            std::fs::create_dir_all(schemas_dir.join("values"))
                .expect("values dir");

            std::fs::write(
                schemas_dir.join("values/countries.toml"),
                "[[entries]]\nslug = \"us\"\nname = \"United \
                 States\"\ncontinent = \"North America\"\n",
            )
            .expect("write values file");

            write_schema(
                temp.path(),
                "book",
                r#"
                [fields.country]
                type = "select"
                values = { path = "values/countries.toml", value = "slug", label = "name" }
                "#,
            );

            let rendered = render(
                &schemas_dir,
                "{{ schema.get('book').field('country')[0].label }} ({{ \
                 schema.get('book').field('country')[0].value }}, {{ \
                 schema.get('book').field('country')[0].continent }})",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "United States (us, North America)");
        }

        #[test]
        fn renders_structured_inline_select_field_as_objects() {
            use pretty_assertions::assert_eq;

            let temp = tempfile::tempdir().expect("create temp dir");
            let schemas_dir = temp.path().join(".traces/schemas");
            write_schema(
                temp.path(),
                "calendar",
                r#"
                [fields.month]
                type = "select"
                values = [
                    { value = "jan", label = "January", quarter = 1 },
                ]
                "#,
            );

            let rendered = render(
                &schemas_dir,
                "{{ schema.get('calendar').field('month')[0].label }}:Q{{ \
                 schema.get('calendar').field('month')[0].quarter }}",
            )
            .expect("render succeeds");

            assert_eq!(rendered, "January:Q1");
        }
        #[test]
        fn a_broken_schema_now_breaks_construction_even_when_the_template_never_touches_schema()
         {
            // Schema resolution happens once at construction, in
            // `SchemaService::new`, not on the first `schema.get` call. A
            // malformed Schema TOML therefore fails construction even when
            // the template never references `schema.*`.
            let temp = tempfile::tempdir().expect("create temp dir");
            write_schema(temp.path(), "broken", "not valid toml [[[");

            let error =
                SchemaService::new(&temp.path().join(".traces/schemas"))
                    .expect_err(
                        "a malformed Schema now fails construction \
                         unconditionally",
                    );

            assert!(matches!(error.as_error(), SchemaError::File {
                source: SchemaFileError::Parse(_),
                ..
            }));
        }
    }

    mod select_entry_value {
        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;

        #[test]
        fn literal_entry_returns_plain_string_value() {
            let entry = SchemaSelectFieldEntry::literal("draft".to_owned());

            let value = select_entry_value(&entry);

            assert_eq!(value.as_str(), Some("draft"));
        }

        #[rstest]
        #[case::different_label("draft", "Draft", IndexMap::new())]
        #[case::with_extra(
            "draft",
            "draft",
            IndexMap::from([("color".to_owned(), FieldValue::String("blue".to_owned()))])
        )]
        fn non_literal_entry_returns_object(
            #[case] val: &str,
            #[case] lbl: &str,
            #[case] extra: IndexMap<String, FieldValue>,
        ) {
            let entry = SchemaSelectFieldEntry::with_label(
                val.to_owned(),
                lbl.to_owned(),
                extra,
            );

            let value = select_entry_value(&entry);

            assert!(
                matches!(value.kind(), minijinja::value::ValueKind::Map),
                "expected map, got {:?}",
                value.kind()
            );
        }
    }
}
