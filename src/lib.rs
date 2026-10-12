//! Process Obsidian-style Markdown notes with frontmatter, inline fields,
//! wikilinks, and task lists.
//!
//! # Modules
//!
//! - [`cli`] - Command-line interface definitions, argument parsing, and
//!   command execution flow.
//! - `config` - Project configuration loading, discovery, TOML parsing, and
//!   trust verification.
//! - `dialog` - Object-safe dialog prompts for interactive and preset input.
//! - `dirs` - XDG and platform-specific directory resolution for configuration
//!   and persistent state.
//! - `dirtree` - Directory-tree traversal with classified walk errors.
//! - `field` - Validated field-name and field-key primitives shared across note
//!   metadata and schemas.
//! - `file` - File metadata and file-name newtypes shared by the index,
//!   template, and schema layers.
//! - `file_class_expander` - Implements schema-aware File Class expansion for
//!   the query domain, without either depending on the other.
//! - `file_tracker` - Hash-keyed file and path tracker using BLAKE3-named
//!   entries.
//! - `hash` - BLAKE3 hashing for file contents and canonicalized paths.
//! - `index` - Persistent file index, note parsing, and link graph
//!   construction.
//! - `query` - Read-side source selection, record projection, transformations,
//!   and output rendering.
//! - `note` - Markdown note parsing, YAML frontmatter extraction, and task
//!   processing.
//! - `path` - Root-relative path validation and confinement.
//! - `schema` - Schema registry and field resolution: parses
//!   `.traces/schemas/*.toml` and linearizes the `extends` DAG into effective
//!   field definitions.
//! - `template` - Template loading, path expansion, custom engine bindings, and
//!   note rendering.
//! # Key Types
//!
//! Always available:
//!
//! - [`Note`], [`ListItem`], [`TaskListItem`]: Parsed note and task records.
//! - [`Tag`], [`TagError`]: Validated Markdown tags.
//! - [`SourceLine`], [`PositionError`]: Strongly-typed 1-indexed source line
//!   numbers and position errors.
//! - [`TaskStatus`], [`TaskStatusMap`], [`TaskStatusType`]: Task status symbols
//!   and lookup tables.
//! - [`FileMeta`]: Filesystem metadata for indexed files.
//! - [`DialogProvider`], [`PresetDialogProvider`], [`TerminalDialogProvider`]:
//!   Interactive and preset dialog prompts.
//! - [`parse_markdown`], [`MarkdownParserInput`]: Note parsing entry points.
//!
//! Available with the `test-utils` feature under `testing`:
//!
//! - `testing::Config`, `testing::ConfigService`, `testing::TrustRequest`:
//!   Project configuration and trust verification.
//! - `testing::WorkspaceIndex`, `testing::IndexerService`: Persistent index
//!   engine and services.
//! - `testing::QueryBuilder`, `testing::QuerySet`, `testing::QueryService`:
//!   Query planning and execution.
//! - `testing::Schema`, `testing::SchemaService`: Schema registry and
//!   inheritance graph.
//! - `testing::TemplateService`, `testing::CommitPolicy`, `testing::WriteMode`:
//!   Template rendering and file writes.
//! - `testing::Blake3Digest`, `testing::Blake3PathHash`: BLAKE3 hashing
//!   primitives.
//! # Visibility Probes
//!
//! Internal configuration structures remain private to the crate and unnameable
//! from outside. Each compile-fail probe is paired with a positive-control
//! doctest verifying that the corresponding public interface is nameable.
//!
//! ```compile_fail
//! // ConfigError must not be nameable from outside the crate.
//! use traces_pkm::ConfigError;
//! ```
//!
//! ```rust
//! // Positive control: CliError is the public error entrypoint.
//! use traces_pkm::cli::CliError;
//! ```
//!
//! ```compile_fail
//! // ConfigBuilder must not be nameable from outside the crate.
//! use traces_pkm::ConfigBuilder;
//! ```
//!
//! ```rust
//! // Positive control: Note is a public item always available.
//! use traces_pkm::Note;
//! ```
//!
//! ```compile_fail
//! // SchemasConfig must not be nameable from outside the crate.
//! use traces_pkm::SchemasConfig;
//! ```
//!
//! ```rust
//! // Positive control: Tag is a public item always available.
//! use traces_pkm::Tag;
//! ```
//!
//! ```compile_fail
//! // FrontmatterConfig must not be nameable from outside the crate.
//! use traces_pkm::FrontmatterConfig;
//! ```
//!
//! ```rust
//! // Positive control: TaskStatus is a public item always available.
//! use traces_pkm::TaskStatus;
//! ```

pub mod cli;
mod config;
mod date;
mod delimiter;
mod dialog;
mod dirs;
mod dirtree;
mod duration;
mod env_vars;
mod field;
mod file;
mod file_class_expander;
mod file_tracker;
mod hash;
mod index;
mod lexer;
mod note;
mod path;
mod position;
mod query;
mod schema;
mod strsim;
mod tag;
mod task;
mod template;
#[cfg(test)]
mod temporal_oracle;
mod yaml;

pub(crate) use config::{Config, ConfigService, TaskConfig, TrustRequest};
pub(crate) use date::{DATE_PATTERN, DATETIME_PATTERN};
pub use date::{DateError, DateTimeValue, DateValue};
pub(crate) use delimiter::DelimiterType;
pub use dialog::{
    DialogError, DialogProvider, DialogResult, PresetDialogProvider,
    TerminalDialogProvider,
};
pub(crate) use dirtree::{DirNode, DirTree, DirTreeError};
pub use duration::{DurationError, DurationValue};
pub(crate) use duration::{DurationSeconds, DurationUnit, UNIT_SPELLINGS_HINT};
pub(crate) use field::{
    FieldKey, FieldKeyRef, FieldName, FieldNameError, FieldNameRef, FieldValue,
};
pub use file::FileMeta;
pub(crate) use file::{BaseName, BaseNameRef, FileName};
pub(crate) use file_tracker::{
    CleanMode, FilePathTracker, FilePathTrackerError,
};
pub(crate) use hash::{Blake3Digest, Blake3PathHash};
pub(crate) use index::IndexerService;
pub(crate) use lexer::{
    LexError, SpannedTokenStream, TokenSpec, lexical_unquote,
};
pub(crate) use note::NoteFieldType;
pub use note::{
    Heading, ListItem, ListItemType, ListText, MarkdownParserInput, Note,
    NoteFieldValue, NoteFieldValueRef, TaskListItem, parse_markdown,
};
pub(crate) use position::{BytePos, ByteSpan, LineIndex, Spanned};
pub use position::{PositionError, SourceLine};
pub use tag::{Tag, TagError};
pub use task::{
    TaskDate, TaskDateSet, TaskDateSetIter, TaskDateType, TaskError,
    TaskPriority, TaskStatus, TaskStatusMap, TaskStatusSymbol, TaskStatusType,
};
#[expect(unused_imports, reason = "crate re-export")]
pub(crate) use template::TemplateService;

#[cfg(any(test, feature = "test-utils"))]
pub mod testing {
    //! Test-only exports for integration tests and benchmarks.
    //!
    //! The membership of this module is frozen: exactly the 45 identifiers
    //! required by external test targets under the `test-utils` feature.

    pub use crate::{
        config::{Config, ConfigService, TaskConfig, TrustRequest},
        file::FileFormat,
        hash::{Blake3Digest, Blake3PathHash},
        index::{
            FileEntry, IndexerService, InlinkMap, RefreshReport, WorkspaceIndex,
        },
        path::codec as path_codec,
        query::{
            QueryBuilder, QueryBuilderError, QueryResult, QueryRow,
            QueryService, QuerySet, SourceSelector,
        },
        schema::{Schema, SchemaFieldDef, SchemaService, SchemaServiceError},
        template::{
            CommitPolicy, RenderFailureKind, TemplatePathInput,
            TemplateService, WriteMode, WriteOutcome,
        },
        test_support::{
            DEFAULT_SCHEMA_VALUES_DIR, DEFAULT_SCHEMAS_DIR,
            DEFAULT_TEMPLATES_DIR, TestProject, TzGuard, build_test_index,
            create_trusted_project, fixture_service, parse_note,
            parse_note_str, parse_tag, resolve_safe_path, write_note,
            write_schema, write_template,
        },
    };
}

/// Build isolated fixtures for the crate's own `#[cfg(test)]` suites and,
/// under the `test-utils` feature, for external `tests/`/`benches/`
/// consumers.
///
/// - [`TestProject`] encapsulates an isolated workspace fixture managing paths,
///   configuration, trust records, schemas, templates, and on-disk index
///   persistence.
/// - [`fixture_service`] returns a [`ConfigService`] backed by temporary
///   directories.
/// - [`create_trusted_project`] writes a minimal config and trusts it.
/// - [`write_note`], [`write_template`], and [`write_schema`] write fixture
///   files.
/// - [`parse_note`], [`parse_note_str`], and [`build_test_index`] construct
///   in-memory notes and indexes with zero disk I/O.
/// - [`parse_tag`] parses tag string slices for fixture data.
/// - [`TzGuard`] (test-only) pins the process's `TZ` for deterministic
///   local-zone assertions.
#[cfg(any(test, feature = "test-utils"))]
mod test_support {
    #![expect(
        clippy::expect_used,
        reason = "fixture-only code; a failed .expect() here means the \
                  fixture itself is broken and should panic immediately"
    )]

    use std::{
        path::{Component, Path, PathBuf},
        sync::Arc,
    };

    use crate::{
        Config, ConfigService, IndexerService, MarkdownParserInput, Note, Tag,
        config::{LocalConfigFile, TrustRequest},
        index::WorkspaceIndex,
        parse_markdown,
    };

    /// Default template directory relative to project root.
    pub const DEFAULT_TEMPLATES_DIR: &str = "templates";

    /// Default schemas directory relative to project root.
    pub const DEFAULT_SCHEMAS_DIR: &str = ".traces/schemas";

    /// Default schema values directory relative to project root.
    pub const DEFAULT_SCHEMA_VALUES_DIR: &str = ".traces/schemas/values";

    /// Minimal local config TOML shared by every crate-side fixture that
    /// writes `.traces/config.toml` directly. Single-sourced here (ticket
    /// 17); the E2E harness keeps its own copy and imports nothing from the
    /// crate.
    const LOCAL_CONFIG_TOML: &str = "[templates]\ndirectory = \"templates\"\n";

    /// Encapsulates an isolated workspace fixture directory, configuration,
    /// trust records, and on-disk index persistence for tests.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use traces_pkm::testing::TestProject;
    /// let temp = tempfile::tempdir().unwrap();
    /// let project = TestProject::trusted(temp.path().join("project"));
    /// project.write_note("notes/sample.md", "# Sample\n");
    /// let index = project.build_index();
    /// assert_eq!(index.entries().len(), 1);
    /// ```
    #[derive(Clone, Debug)]
    pub struct TestProject {
        root: PathBuf,
        service: ConfigService,
    }

    impl TestProject {
        fn project_service(root: &Path) -> ConfigService {
            let state_root = root.join(".traces");
            fixture_service(&state_root)
        }

        /// Creates an empty directory with an isolated config service and no
        /// `.traces/` directory.
        ///
        /// # Panics
        ///
        /// - Panics if `root` cannot be created. Fixture-only code: a panic
        ///   here means the fixture setup itself is broken.
        #[inline]
        #[must_use]
        pub fn empty<P: Into<PathBuf>>(root: P) -> Self {
            let root = root.into();
            std::fs::create_dir_all(&root).expect("create project dir");
            let service = Self::project_service(&root);
            Self {
                root,
                service,
            }
        }

        /// Writes `.traces/config.toml` without recording trust in the service.
        ///
        /// # Panics
        ///
        /// - Panics if the config file cannot be written. Fixture-only code.
        #[inline]
        #[must_use]
        pub fn untrusted<P: Into<PathBuf>>(root: P) -> Self {
            let root = root.into();
            std::fs::create_dir_all(&root).expect("create project dir");
            let config_path = root.join(".traces/config.toml");
            std::fs::create_dir_all(
                config_path.parent().expect("config parent"),
            )
            .expect("create config parent");
            std::fs::write(&config_path, LOCAL_CONFIG_TOML)
                .expect("write config file");
            let service = Self::project_service(&root);
            Self {
                root,
                service,
            }
        }

        /// Writes `.traces/config.toml` and records it as trusted in the
        /// isolated trust store.
        ///
        /// # Panics
        ///
        /// - Panics if the fixture setup fails; see [`create_trusted_project`].
        #[inline]
        #[must_use]
        pub fn trusted<P: Into<PathBuf>>(root: P) -> Self {
            let root = root.into();
            let service = Self::project_service(&root);
            let _ = create_trusted_project(&service, &root);
            Self {
                root,
                service,
            }
        }

        /// Returns the project root directory.
        #[inline]
        #[must_use]
        pub fn root(&self) -> &Path {
            &self.root
        }

        /// Returns the backing [`ConfigService`].
        #[inline]
        #[must_use]
        pub fn service(&self) -> &ConfigService {
            &self.service
        }

        /// Loads this project's effective configuration.
        ///
        /// # Panics
        ///
        /// Panics if discovery, trust verification, or parsing fails.
        /// Fixture-only code: a panic here means the fixture setup itself is
        /// broken.
        #[inline]
        #[must_use]
        pub fn config(&self) -> Config {
            self.service.load(&self.root).expect("load fixture config")
        }

        /// Returns test [`Config`] rooted at this project, configuring the
        /// local `templates/` directory only when that directory exists on
        /// disk — the directory-existence flip is the accessor's whole
        /// contract, hence the name.
        #[inline]
        #[must_use]
        pub fn config_with_templates_if_present(&self) -> Config {
            let base = Config::test_default(&self.root);
            if self.root.join(DEFAULT_TEMPLATES_DIR).is_dir() {
                base.with_templates()
            } else {
                base
            }
        }

        /// Records trust for this project in its isolated trust store.
        ///
        /// # Panics
        ///
        /// - Panics if `.traces/config.toml` is missing or invalid.
        ///   Fixture-only code: a panic here means the test's fixture data is
        ///   wrong.
        #[inline]
        pub fn trust(&self) {
            let config = LocalConfigFile::at(&self.root);
            self.service
                .trust(&TrustRequest::from(&config))
                .expect("trust project config");
        }

        /// Removes trust for this project, returning the count of removed
        /// entries.
        ///
        /// # Panics
        ///
        /// - Panics if the trust entry cannot be removed. Fixture-only code.
        #[inline]
        #[must_use]
        pub fn untrust(&self) -> usize {
            self.service
                .untrust(&TrustRequest::from(self.root.as_path()))
                .expect("untrust project")
        }

        /// Safely writes content to `rel_path` beneath project root.
        ///
        /// # Panics
        ///
        /// - Panics if the path escapes `root` or the file cannot be written.
        ///   Fixture-only code: a panic here means the fixture setup is broken.
        #[inline]
        pub fn write_file<P: AsRef<Path>>(
            &self,
            rel_path: P,
            content: &str,
        ) -> PathBuf {
            let path = resolve_safe_path(&self.root, rel_path.as_ref());
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .expect("create parent directory");
            }
            std::fs::write(&path, content).expect("write file");
            path
        }

        /// Writes a note relative to the project root.
        ///
        /// # Panics
        ///
        /// - Panics if `rel_path` escapes `root` or the note cannot be written.
        ///   Fixture-only code: a panic here means the fixture setup is broken.
        #[inline]
        pub fn write_note<P: AsRef<Path>>(
            &self,
            rel_path: P,
            content: &str,
        ) -> PathBuf {
            self.write_file(rel_path, content)
        }

        /// Writes a template file under `templates/`.
        ///
        /// # Panics
        ///
        /// - Panics if the path escapes `root` or the template cannot be
        ///   written. Fixture-only code.
        #[inline]
        pub fn write_template<P: AsRef<Path>>(
            &self,
            name: P,
            content: &str,
        ) -> PathBuf {
            self.write_file(
                Path::new(DEFAULT_TEMPLATES_DIR).join(name.as_ref()),
                content,
            )
        }

        /// Writes a schema file under `.traces/schemas/`.
        ///
        /// # Panics
        ///
        /// - Panics if the path escapes `root` or the schema cannot be written.
        ///   Fixture-only code.
        #[inline]
        pub fn write_schema<P: AsRef<Path>>(
            &self,
            name: P,
            toml: &str,
        ) -> PathBuf {
            let file_name = format!("{}.toml", name.as_ref().display());
            self.write_file(
                Path::new(DEFAULT_SCHEMAS_DIR).join(file_name),
                toml,
            )
        }

        /// Writes a schema value file under `.traces/schemas/values/`.
        ///
        /// # Panics
        ///
        /// - Panics if the path escapes `root` or the file cannot be written.
        ///   Fixture-only code.
        #[inline]
        pub fn write_schema_value<P: AsRef<Path>>(
            &self,
            rel_path: P,
            toml: &str,
        ) -> PathBuf {
            self.write_file(
                Path::new(DEFAULT_SCHEMA_VALUES_DIR).join(rel_path.as_ref()),
                toml,
            )
        }

        /// Returns an [`IndexerService`] configured for this project.
        #[inline]
        #[must_use]
        pub fn indexer(&self) -> IndexerService {
            IndexerService::from(&self.config_with_templates_if_present())
        }

        /// Builds an in-memory index from disk files.
        ///
        /// # Panics
        ///
        /// - Panics if the index build fails. Fixture-only code.
        #[inline]
        #[must_use]
        pub fn build_index(&self) -> WorkspaceIndex {
            self.indexer().build().expect("build index")
        }

        /// Builds and persists the index to `index.redb`, returning both.
        ///
        /// # Panics
        ///
        /// - Panics if the build or persist fails. Fixture-only code.
        #[inline]
        #[must_use]
        pub fn build_and_persist(&self) -> (IndexerService, WorkspaceIndex) {
            let indexer = self.indexer();
            let index = indexer.build().expect("build index");
            indexer.persist(&index).expect("persist index");
            (indexer, index)
        }
    }

    /// Parses a Markdown note in memory with a custom path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use traces_pkm::testing::parse_note;
    /// let note = parse_note("notes/daily.md", "# Today\n\n- [ ] Task");
    /// assert_eq!(note.path().to_str(), Some("notes/daily.md"));
    /// ```
    #[inline]
    #[must_use]
    pub fn parse_note<P: AsRef<Path>>(path: P, src: &str) -> Note {
        parse_markdown(&MarkdownParserInput::for_test(path.as_ref(), src))
    }

    /// Parses a Markdown note in memory with the default path `note.md`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use traces_pkm::testing::parse_note_str;
    /// let note = parse_note_str("# Sample\n\nContent");
    /// assert_eq!(note.path().to_str(), Some("note.md"));
    /// ```
    #[inline]
    #[must_use]
    pub fn parse_note_str(src: &str) -> Note {
        parse_note(Path::new("note.md"), src)
    }

    /// Builds an in-memory [`WorkspaceIndex`] wrapped in an [`Arc`] from note
    /// path and content pairs with zero disk I/O.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use traces_pkm::testing::build_test_index;
    /// let index = build_test_index(&[("a.md", "# A"), ("b.md", "# B")]);
    /// assert_eq!(index.entries().len(), 2);
    /// ```
    #[inline]
    #[must_use]
    pub fn build_test_index(notes: &[(&str, &str)]) -> Arc<WorkspaceIndex> {
        Arc::new(WorkspaceIndex::for_test(notes))
    }

    /// Parses a [`Tag`] string slice for tests.
    ///
    /// # Panics
    ///
    /// - Panics if `s` is not a valid tag. Fixture-only code: a panic here
    ///   means the test's fixture data is wrong.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use traces_pkm::testing::parse_tag;
    /// let tag = parse_tag("#projects/active");
    /// assert_eq!(tag.as_str(), "#projects/active");
    /// ```
    #[inline]
    #[must_use]
    pub fn parse_tag(s: &str) -> Tag {
        Tag::parse(s).expect("valid test tag")
    }

    /// Resolves a project-relative fixture path beneath `root`.
    ///
    /// Prevents accidental writes outside the fixture directory by rejecting
    /// absolute paths and parent-directory traversal (`..`).
    ///
    /// # Panics
    ///
    /// - Panics if `rel_path` is absolute or contains a `..` component.
    #[inline]
    #[must_use]
    pub fn resolve_safe_path<P: AsRef<Path>>(
        root: &Path,
        rel_path: P,
    ) -> PathBuf {
        let rel_path = rel_path.as_ref();
        assert!(
            rel_path.is_relative()
                && !rel_path
                    .components()
                    .any(|part| part == Component::ParentDir),
            "fixture path must stay inside project root: {}",
            rel_path.display()
        );
        root.join(rel_path)
    }

    /// Creates a [`ConfigService`] backed by isolated tracked-config and trust
    /// stores under `root`, never the real OS state directories.
    #[inline]
    #[must_use]
    pub fn fixture_service(root: &Path) -> ConfigService {
        ConfigService::at(root.join("tracked-store"), root.join("trust-store"))
    }

    /// Writes a minimal local config at `root/.traces/config.toml` configuring
    /// `templates` as the template directory name, and records `root` as
    /// trusted in `service`'s trust store. Note: does not create the template
    /// directory on disk.
    ///
    /// # Panics
    ///
    /// - Panics if `root` cannot be created, the config file cannot be written,
    ///   or trust cannot be recorded. Fixture-only code: a panic here means the
    ///   fixture setup itself is broken.
    #[inline]
    #[must_use]
    pub fn create_trusted_project(
        service: &ConfigService,
        root: &Path,
    ) -> PathBuf {
        std::fs::create_dir_all(root).expect("create project dir");
        let config_path = root.join(".traces/config.toml");
        std::fs::create_dir_all(config_path.parent().expect("config parent"))
            .expect("create config parent");
        std::fs::write(&config_path, LOCAL_CONFIG_TOML)
            .expect("write config file");
        let config = LocalConfigFile::at(root);
        service
            .trust(&TrustRequest::from(&config))
            .expect("trust project config");
        config_path
    }

    /// Writes a Markdown note at `root.join(rel_path)`, creating parent
    /// directories as needed.
    ///
    /// # Panics
    ///
    /// - Panics if the path escapes `root` or the note cannot be written.
    ///   Fixture-only code: a panic here means the fixture setup is broken.
    #[inline]
    pub fn write_note<P: AsRef<Path>>(
        root: &Path,
        rel_path: P,
        content: &str,
    ) -> PathBuf {
        let path = resolve_safe_path(root, rel_path.as_ref());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create note parent dir");
        }
        std::fs::write(&path, content).expect("write note");
        path
    }

    /// Writes a template file into `root/templates/name`, creating the
    /// `templates` directory if absent.
    ///
    /// # Panics
    ///
    /// - Panics if the path escapes `root` or the template cannot be written.
    ///   Fixture-only code: a panic here means the fixture setup is broken.
    #[inline]
    pub fn write_template<P: AsRef<Path>>(
        root: &Path,
        name: P,
        source: &str,
    ) -> PathBuf {
        let path = resolve_safe_path(
            root,
            Path::new(DEFAULT_TEMPLATES_DIR).join(name.as_ref()),
        );
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create templates dir");
        }
        std::fs::write(&path, source).expect("write template");
        path
    }

    /// Writes a schema file into `root/.traces/schemas/name.toml`, creating
    /// directories if needed.
    ///
    /// # Panics
    ///
    /// - Panics if the path escapes `root` or the schema cannot be written.
    ///   Fixture-only code: a panic here means the fixture setup is broken.
    #[inline]
    pub fn write_schema<P: AsRef<Path>>(
        root: &Path,
        name: P,
        toml: &str,
    ) -> PathBuf {
        let file_name = format!("{}.toml", name.as_ref().display());
        let path = resolve_safe_path(
            root,
            Path::new(DEFAULT_SCHEMAS_DIR).join(file_name),
        );
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create schemas dir");
        }
        std::fs::write(&path, toml).expect("write schema");
        path
    }

    #[cfg(any(test, feature = "test-utils"))]
    /// Serializes tests against the process-global `TZ` variable.
    static TZ_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[cfg(any(test, feature = "test-utils"))]
    thread_local! {
        /// Per-test-thread holder of the [`TzGuard`]; the guard drops when the
        /// thread ends, restoring `TZ`.
        static TZ_GUARD: std::cell::RefCell<Option<TzGuard>> =
            const { std::cell::RefCell::new(None) };
    }

    /// Owns the process-global `TZ` variable for the duration of one test.
    ///
    /// chrono's `Local` reads `TZ` per call through `std::env::var` (serialized
    /// by std's environment lock, so concurrent reads are memory-safe); it also
    /// caches the resolved offset in a thread-local for up to one second
    /// (chrono's internal `local::unix::Cache`), re-reading `TZ` only on that
    /// thread's first lookup or once the cache goes stale. The test harness
    /// runs each test on its own thread, so the first lookup on a fresh thread
    /// always happens after `TzGuard::set` has already swapped `TZ`, keeping
    /// the per-thread cache in step with the swap. A test that swaps `TZ`
    /// before its first local-clock read therefore observes its zone
    /// deterministically.
    ///
    /// Install with [`TzGuard::set`] to pin a zone or [`TzGuard::keep`] to hold
    /// the lock without changing the zone. The guard registers itself on the
    /// test thread and lives until the thread ends; a second `set` on the same
    /// thread just swaps the value, and the original guard still restores the
    /// true original. Holds a shared mutex so concurrent tests never interleave
    /// reads of one zone with writes of another; the edition-2024 `unsafe` on
    /// the write exists for non-Rust `getenv` readers, of which this dependency
    /// tree has none during tests. Tests whose assertions depend on the zone
    /// must hold the guard (`set` or `keep`); zone-stable assertions elsewhere
    /// are unaffected by a swap. The previous `TZ` is restored when the test
    /// thread ends.
    #[cfg(any(test, feature = "test-utils"))]
    #[derive(Debug)]
    pub struct TzGuard {
        previous: Option<String>,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    #[cfg(any(test, feature = "test-utils"))]
    impl TzGuard {
        /// Holds the `TZ` lock for this thread without changing the zone.
        #[inline]
        pub fn keep() {
            Self::install(None);
        }

        /// Sets `TZ` to `zone` for this thread, restoring the previous value
        /// when the thread ends.
        #[inline]
        pub fn set(zone: &str) {
            Self::install(Some(zone));
        }

        fn install(zone: Option<&str>) {
            let already_owned = TZ_GUARD.with(|slot| slot.borrow().is_some());
            if already_owned {
                // This thread already owns `TZ`; a second `set` just swaps the
                // value and the original guard still restores the true original
                // on drop.
                if let Some(zone) = zone {
                    set_var(zone);
                }
                return;
            }
            let lock = TZ_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let previous = std::env::var("TZ").ok();
            if let Some(zone) = zone {
                set_var(zone);
            }
            TZ_GUARD.with(|slot| {
                *slot.borrow_mut() = Some(Self {
                    previous,
                    _lock: lock,
                });
            });
        }
    }

    #[cfg(any(test, feature = "test-utils"))]
    impl Drop for TzGuard {
        #[inline]
        fn drop(&mut self) {
            match self.previous.take() {
                Some(previous) => set_var(&previous),
                None => remove_var(),
            }
        }
    }

    #[cfg(any(test, feature = "test-utils"))]
    #[expect(
        unsafe_code,
        reason = "chrono's `Local` reads the `TZ` environment variable, so \
                  mutating it is the only injection seam for deterministic \
                  local-zone tests; `set_var` is unsafe in edition 2024 \
                  because non-Rust `getenv` readers could race it, and this \
                  dependency tree has none during tests"
    )]
    fn set_var(value: &str) {
        // SAFETY: std::env::set_var serializes with every std::env::var read
        // via std's internal environment lock, so in-process readers are
        // memory-safe; writes also serialize test-side under `TZ_LOCK`.
        unsafe { std::env::set_var("TZ", value) };
    }

    #[cfg(any(test, feature = "test-utils"))]
    #[expect(
        unsafe_code,
        reason = "chrono's `Local` reads the `TZ` environment variable, so \
                  mutating it is the only injection seam for deterministic \
                  local-zone tests; `remove_var` is unsafe in edition 2024 \
                  because non-Rust `getenv` readers could race it, and this \
                  dependency tree has none during tests"
    )]
    fn remove_var() {
        // SAFETY: std::env::remove_var serializes with every std::env::var read
        // via std's internal environment lock, so in-process readers are
        // memory-safe; writes also serialize test-side under `TZ_LOCK`.
        unsafe { std::env::remove_var("TZ") };
    }

    #[cfg(test)]
    mod tz_guard_tests {
        use super::*;

        #[test]
        fn set_swaps_the_zone_and_drop_restores_it() {
            TzGuard::set("Etc/GMT-2");
            assert_eq!(std::env::var("TZ").ok().as_deref(), Some("Etc/GMT-2"));

            // A second set on the same thread swaps the value; the original
            // guard still restores the true original.
            TzGuard::set("UTC");
            assert_eq!(std::env::var("TZ").ok().as_deref(), Some("UTC"));

            // Dropping happens at thread end, so this test cannot observe the
            // restore directly; it only asserts the swap behavior above.
        }

        #[test]
        fn keep_holds_the_lock_without_changing_the_zone() {
            let before = std::env::var("TZ").ok();
            TzGuard::keep();

            assert_eq!(std::env::var("TZ").ok(), before);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        mod path_safety {
            use super::*;

            #[test]
            fn resolves_valid_relative_paths() {
                let root = Path::new("/workspace");
                let resolved =
                    resolve_safe_path(root, Path::new("notes/daily.md"));
                assert_eq!(resolved, Path::new("/workspace/notes/daily.md"));
            }

            #[test]
            #[should_panic(
                expected = "fixture path must stay inside project root"
            )]
            fn rejects_absolute_paths() {
                let root = Path::new("/workspace");
                let _ = resolve_safe_path(root, Path::new("/etc/passwd"));
            }

            #[test]
            #[should_panic(
                expected = "fixture path must stay inside project root"
            )]
            fn rejects_parent_directory_traversal() {
                let root = Path::new("/workspace");
                let _ = resolve_safe_path(root, Path::new("../escape.md"));
            }
        }

        mod service {
            use super::*;

            #[test]
            fn returns_usable_service_in_temporary_directory() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let service = fixture_service(temp.path());
                let result = service.load(temp.path());
                assert!(result.is_err(), "empty dir should fail discovery");
            }
        }

        mod project {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn creates_empty_workspace_without_traces_config() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let project = TestProject::empty(temp.path().join("proj"));
                assert!(project.root().is_dir());
                assert!(!project.root().join(".traces/config.toml").exists());
            }

            #[test]
            fn creates_trusted_workspace_and_persists_index() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let project = TestProject::trusted(temp.path().join("proj"));
                assert_eq!(project.root(), temp.path().join("proj").as_path());
                project.write_note("notes/sample.md", "# Sample\n");
                project.write_template("daily.md", "Template");
                project.write_schema("tag_schema", "fields = {}\n");
                project.write_schema_value("vals.toml", "vals = []\n");

                let (indexer, index) = project.build_and_persist();
                let reloaded = indexer.load().expect("load persisted index");
                assert_eq!(reloaded.entries().len(), index.entries().len());
                assert!(
                    reloaded.entries().iter().any(
                        |e| e.file().path() == Path::new("notes/sample.md")
                    )
                );
                assert!(reloaded.entries().iter().any(
                    |e| e.file().path() == Path::new("templates/daily.md")
                ));
            }

            #[test]
            fn config_detects_templates_directory() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let project = TestProject::empty(temp.path().join("proj"));
                assert!(
                    project
                        .config_with_templates_if_present()
                        .local_template_dir()
                        .is_none()
                );

                project.write_template("test.md", "content");
                assert_eq!(
                    project
                        .config_with_templates_if_present()
                        .local_template_dir(),
                    Some(project.root().join("templates").as_path())
                );
            }
        }
        mod memory {
            use pretty_assertions::assert_eq;

            use super::*;

            #[test]
            fn parses_notes_and_builds_index_without_disk() {
                let note = parse_note_str("# Test Note\n");
                assert_eq!(note.path(), Path::new("note.md"));

                let index =
                    build_test_index(&[("a.md", "# A"), ("b.md", "# B")]);
                assert_eq!(index.entries().len(), 2);

                let tag = parse_tag("#project");
                assert_eq!(tag.as_str(), "#project");
            }

            #[test]
            fn scan_skips_traces_dir() {
                let temp = tempfile::tempdir().expect("create temp dir");
                let project = TestProject::trusted(temp.path().join("proj"));
                project.write_note("a.md", "# A\n");
                let index = project.build_index();
                let paths: Vec<&Path> =
                    index.entries().iter().map(|e| e.file().path()).collect();
                assert_eq!(paths, [Path::new("a.md")]);
            }
        }
        mod visibility {
            use pretty_assertions::assert_eq;

            #[test]
            fn retains_config_internals_under_expected_names() {
                assert_eq!(
                    std::any::type_name::<crate::config::ConfigError>(),
                    "traces_pkm::config::error::ConfigError"
                );
                assert_eq!(
                    std::any::type_name::<crate::config::ConfigBuilder>(),
                    "traces_pkm::config::builder::ConfigBuilder"
                );
                assert_eq!(
                    std::any::type_name::<crate::config::SchemasConfig>(),
                    "traces_pkm::config::model::SchemasConfig"
                );
                assert_eq!(
                    std::any::type_name::<crate::config::FrontmatterConfig>(),
                    "traces_pkm::config::model::FrontmatterConfig"
                );
            }
        }
    }
}
