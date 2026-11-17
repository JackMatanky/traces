//! Parses CLI arguments and dispatches commands.
//!
//! Parses process arguments with [`clap`], routes to the selected subcommand
//! (or the default `-i` template dispatch), and translates domain errors into
//! [`CliError`] diagnostics.
//!
//! # Key Types
//!
//! - [`CommandOutcome`]: top-level result of a successful command.
//! - [`UserAbort`]: deliberate cancellation or interruption.
//! - [`CliError`]: unified diagnostic error for all CLI operations.
//!
//! Submodules contain command-specific logic. This module handles argument
//! flow and shared helpers.

mod completions;
mod cwd;
mod error;
mod index;
pub mod init;
mod list;
#[cfg(test)]
mod query_pipeline_tests;
mod table;
mod task;
mod template;
mod tracked;
mod trust;
mod untrust;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use cwd::Cwd;
#[cfg(test)]
pub(crate) use cwd::CwdGuard;
pub use error::{CliError, CliResult};

use crate::{
    Config, ConfigService, DialogProvider,
    config::{TrustRequests, TrustScope},
    index::{IndexStore, IndexerService},
    query::{
        QueryBuilder, QueryError, QueryMode, QueryService, QuerySet,
        SortDirection, SortOrder, SourceSelector,
    },
    schema::{SchemaService, warn_schema_construction_diagnostics},
};

/// Represents the top-level result of a CLI command.
///
/// Returned by [`run`] to distinguish normal completion from deliberate user
/// cancellation or interruption. The process exit code is `0` for `Completed`
/// and `Aborted(UserAbort::Cancelled)`. `Aborted(UserAbort::Interrupted)` exits
/// with `130`. Callers can match on the variant if needed.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum CommandOutcome {
    /// The command completed its work without interruption.
    Completed,
    /// The user canceled or interrupted an interactive prompt before the
    /// command could finish.
    Aborted(UserAbort),
}

/// Identifies the user gesture that ended an interactive command.
///
/// Extracted from dialog-layer cancellation or interruption errors in the error
/// source chain.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum UserAbort {
    /// The user pressed Escape to cancel an interactive prompt.
    Cancelled,
    /// The user pressed Control-C to interrupt an interactive prompt.
    Interrupted,
}

/// Defines the root command-line parser for `traces`.
///
/// Dispatches to a subcommand when one is present, or to the default
/// `-i`/`--input` template path when the caller passes the flag. Returns
/// [`CliError::NoCommand`] when neither is present.
#[derive(Debug, Parser)]
#[command(
    name = "traces",
    version,
    about = "Template-driven personal knowledge management",
    args_conflicts_with_subcommands = true
)]
struct Cli {
    /// Subcommand to run when not using `-i` or `--input`.
    #[command(subcommand)]
    command: Option<Commands>,
    /// Optional template name for the default `traces -i <name>` dispatch.
    ///
    /// Passing `-i` with no value opens the interactive fuzzy picker.
    //
    // Uses `Option<Option<_>>` instead of `Option<PathBuf>` to separate three
    // states:
    //
    // - `None` (flag absent) returns `CliError::NoCommand`.
    // - `Some(None)` (flag present, no value) opens the interactive picker.
    // - `Some(Some(name))` runs `-i <name>`.
    //
    // `Option<PathBuf>` collapses the first two states into `None`.
    #[arg(short = 'i', long = "input", value_name = "NAME", num_args = 0..=1)]
    #[expect(
        clippy::option_option,
        reason = "the three states are load-bearing: None (flag absent) is \
                  CliError::NoCommand, Some(None) (flag present, no value) is \
                  the interactive picker, Some(Some(name)) is the ordinary -i \
                  <name> dispatch; Option<PathBuf> alone can't distinguish \
                  the first two"
    )]
    input: Option<Option<PathBuf>>,
}

impl Cli {
    /// Runs the parsed command with the given services.
    ///
    /// # Errors
    ///
    /// - [`CliError::NoCommand`] if neither a subcommand nor `-i`/`--input` is
    ///   present.
    /// - Any [`CliError`] returned by command execution.
    fn run(
        self,
        service: &ConfigService,
        provider: Arc<dyn DialogProvider>,
    ) -> Result<CommandOutcome, CliError> {
        let result = match self.command {
            Some(command) => command.run(service, provider),
            None => match self.input {
                Some(Some(name)) => run_with_config(service, |config| {
                    template::Template::new(name).run(config, provider)
                }),
                Some(None) => run_with_config(service, |config| {
                    template::Template::interactive().run(config, provider)
                }),
                None => Err(CliError::NoCommand),
            },
        };
        match result {
            Ok(()) => Ok(CommandOutcome::Completed),
            Err(error) => error
                .user_abort()
                .map_or(Err(error), |abort| Ok(CommandOutcome::Aborted(abort))),
        }
    }
}

/// Enumerates top-level `traces` subcommands.
#[derive(Debug, Subcommand)]
enum Commands {
    /// Initializes local traces configuration in the current directory.
    Init(init::Init),
    /// Grants trust to one or more project roots.
    Trust(trust::Trust),
    /// Revokes trust from one or more project roots.
    Untrust(untrust::Untrust),
    /// Builds or rebuilds the persisted [`crate::index::WorkspaceIndex`].
    Index(index::Index),
    /// Queries pages and prints matching file paths as a Markdown bullet list.
    List(list::List),
    /// Queries pages and prints matching records as a Markdown table.
    Table(table::Table),
    /// Queries tasks and prints matching checkbox lines.
    Task(task::Task),
    /// Renders a template and writes it to disk, or lists available templates.
    #[command(alias = "tmpl")]
    Template(template::Template),
    /// Generates shell completion scripts or lists available template names.
    #[command(alias = "completion")]
    Completions(completions::Completions),
    /// Inspects or cleans the tracked-configuration store.
    Tracked(tracked::Tracked),
}

impl Commands {
    /// Routes a parsed subcommand to its handler.
    ///
    /// # Errors
    ///
    /// - Any [`CliError`] returned by the selected subcommand.
    fn run(
        self,
        service: &ConfigService,
        provider: Arc<dyn DialogProvider>,
    ) -> CliResult {
        match self {
            Self::Init(args) => args.run(provider.as_ref()),
            Self::Trust(args) => args.run(service),
            Self::Untrust(args) => args.run(service),
            Self::Index(args) => {
                run_with_config(service, |config| args.run(config))
            }
            Self::List(args) => {
                run_with_config(service, |config| args.run(config))
            }
            Self::Table(args) => {
                run_with_config(service, |config| args.run(config))
            }
            Self::Task(args) => {
                run_with_config(service, |config| args.run(config))
            }
            Self::Template(args) => {
                run_with_config(service, |config| args.run(config, provider))
            }
            Self::Completions(args) => args.run(service),
            Self::Tracked(args) => args.run(service.tracker_store()),
        }
    }
}

/// Parses process arguments and runs the selected `traces` command.
///
/// Yields [`CommandOutcome::Completed`] on success, or
/// [`CommandOutcome::Aborted`] if the user canceled or interrupted an
/// interactive prompt.
///
/// # Errors
///
/// - [`CliError::NoCommand`] when neither a subcommand nor `-i`/`--input` is
///   present.
/// - Any [`CliError`] for command execution failures (such as configuration
///   loading, indexing, query evaluation, or template instantiation).
#[inline]
pub fn run() -> Result<CommandOutcome, CliError> {
    let provider: Arc<dyn DialogProvider> =
        Arc::new(crate::TerminalDialogProvider::new());
    Cli::parse().run(&ConfigService::default(), provider)
}

/// Reads the process current directory as a [`Cwd`].
///
/// # Errors
///
/// - [`CliError::CurrentDirectory`] if the directory does not exist or the
///   process lacks access.
fn current_dir() -> Result<Cwd, CliError> {
    Cwd::new().map_err(|source| CliError::CurrentDirectory {
        source,
    })
}

/// Reads the current directory and loads its effective configuration.
///
/// # Errors
///
/// - [`CliError::CurrentDirectory`] if reading the current directory fails.
/// - [`CliError::ConfigLoad`] if configuration discovery or loading fails.
fn load_config(service: &ConfigService) -> Result<Config, CliError> {
    let cwd = current_dir()?.into_inner();
    service.load(&cwd).map_err(|source| CliError::ConfigLoad {
        cwd,
        source,
    })
}

/// Loads the current configuration and runs `handler` against a reference to
/// it.
///
/// Isolates the [`Config`] stack temporary in its own non-inlined frame, away
/// from the larger dispatch frames in [`Cli::run`] and [`Commands::run`].
/// Otherwise, `clippy::large_stack_frames` sums every match arm's locals into
/// those frames.
///
/// # Errors
///
/// - [`CliError::CurrentDirectory`] if reading the current directory fails.
/// - [`CliError::ConfigLoad`] if configuration discovery or loading fails.
/// - Any [`CliError`] returned by `handler`.
#[inline(never)]
fn run_with_config(
    service: &ConfigService,
    handler: impl FnOnce(&Config) -> CliResult,
) -> CliResult {
    handler(&load_config(service)?)
}

/// Defines shared `--sort`, `--asc`, and `--desc` CLI arguments for
/// [`list::List`] and [`table::Table`].
///
/// Flattened into both subcommands with `#[command(flatten)]` to share one
/// definition. `clap` enforces that `--asc` and `--desc` conflict and need
/// `--sort`.
#[derive(Debug, Default, clap::Args)]
pub(super) struct SortArgs {
    /// Specifies field paths to sort by.
    ///
    /// Repeatable. Multiple `--sort` flags or comma-separated values combine
    /// into composite sort terms. Defaults to descending order unless `+` or
    /// `--asc` overrides it.
    #[arg(long = "sort", value_delimiter = ',', num_args = 1..)]
    keys: Vec<String>,
    /// Sorts in ascending order.
    ///
    /// Conflicts with `--desc` and needs `--sort`.
    #[arg(long, conflicts_with = "desc", requires = "keys")]
    asc: bool,
    /// Sorts in descending order (the default).
    ///
    /// Conflicts with `--asc` and needs `--sort`.
    #[arg(long, conflicts_with = "asc", requires = "keys")]
    desc: bool,
}

impl SortArgs {
    /// Resolves these arguments into an optional composite [`SortOrder`] with
    /// [`SortOrder::parse`].
    ///
    /// `--asc` and `--desc` set the default direction for segments without
    /// `+` or `-`. Descending is the default when neither flag is present.
    ///
    /// # Errors
    ///
    /// - [`CliError::Query`] if any sort field path is malformed.
    fn resolve(&self, root: &Path) -> Result<Option<SortOrder>, CliError> {
        if self.keys.is_empty() {
            return Ok(None);
        }
        let default_direction = if self.asc {
            SortDirection::Ascending
        } else {
            SortDirection::default()
        };
        SortOrder::parse(&self.keys.join(","), default_direction)
            .map_err(|error| query_error(root, error.into()))
    }
}

/// Synchronizes `root`'s index store and evaluates a query with the given
/// `mode`, `from` source, `filters`, and `order`.
///
/// Shared by [`list::List`], [`table::Table`], and [`task::Task`].
///
/// # Errors
///
/// - [`CliError::Index`] if syncing the index or querying the store fails.
/// - [`CliError::Query`] if any filter expression is malformed or the parser
///   rejects the source selector.
fn refresh_query<'a>(
    config: &Config,
    from: Option<&str>,
    filters: impl IntoIterator<Item = &'a str>,
    order: Option<SortOrder>,
    mode: QueryMode,
) -> Result<QuerySet, CliError> {
    let root = config.root();
    let store =
        IndexerService::from(config).refresh_store().map_err(|source| {
            CliError::Index {
                root: root.to_path_buf(),
                source,
            }
        })?;
    let source = parse_source(config, from)?;
    let has_classes = source.has_classes();
    let mut builder = match mode {
        QueryMode::Pages => QueryBuilder::pages(source),
        QueryMode::Lists => QueryBuilder::lists(source),
        QueryMode::Tasks => QueryBuilder::tasks(source),
    };
    for expr in filters {
        builder = builder
            .filter(expr)
            .map_err(|error| query_error(root, error.into()))?;
    }
    if let Some(order) = order {
        builder = builder.order(order);
    }
    run_query_builder_from_store(config, &store, builder, has_classes)
}

/// Synchronizes `root`'s index store and returns page-level records selected by
/// `from`, filtered by `filters` (composed as AND), and ordered by `order`.
///
/// Shared by [`list::List`] and [`table::Table`].
///
/// # Errors
///
/// - [`CliError::Index`] if syncing the index or querying the store fails.
/// - [`CliError::Query`] if any filter expression or the sort field path is
///   malformed.
fn refresh_page_query(
    config: &Config,
    from: Option<&str>,
    filters: &[String],
    order: Option<SortOrder>,
) -> Result<QuerySet, CliError> {
    refresh_query(
        config,
        from,
        filters.iter().map(String::as_str),
        order,
        QueryMode::Pages,
    )
}

/// Synchronizes `root`'s index store and returns task-level records selected by
/// `from`, filtered by `filters` (composed as AND), and ordered by `order`.
///
/// Shared by [`task::Task`].
///
/// # Errors
///
/// - [`CliError::Index`] if syncing the index or querying the store fails.
/// - [`CliError::Query`] if any filter expression is malformed.
fn refresh_task_query<'a>(
    config: &Config,
    from: Option<&str>,
    filters: impl IntoIterator<Item = &'a str>,
    order: Option<SortOrder>,
) -> Result<QuerySet, CliError> {
    refresh_query(config, from, filters, order, QueryMode::Tasks)
}

/// Parses an optional source expression string into a [`SourceSelector`].
///
/// Normalizes unadorned relative paths pointing to existing `.md` files under
/// `config`'s root by appending the `.md` extension.
///
/// # Errors
///
/// - [`CliError::Query`] if the source expression fails to parse.
fn parse_source(
    config: &Config,
    from: Option<&str>,
) -> Result<SourceSelector, CliError> {
    let root = config.root();
    let source_str = from.map_or("", str::trim);
    let normalized = normalize_source_input(root, source_str);
    SourceSelector::parse(&normalized)
        .map_err(|source| query_error(root, source))
}

/// Normalizes unadorned relative paths pointing to existing `.md` files under
/// `root` by appending the `.md` extension.
///
/// Leaves tags (`#tag`), classes (`@Class`), folder globs, and already-quoted
/// expressions untouched. Wraps paths containing whitespace in quotes so the
/// query source lexer reads them as one quoted path atom.
fn normalize_source_input<'a>(
    root: &Path,
    input: &'a str,
) -> std::borrow::Cow<'a, str> {
    if input.is_empty()
        || input.starts_with(['#', '@', '"', '\''])
        || input.ends_with(['/', '\\'])
    {
        return std::borrow::Cow::Borrowed(input);
    }
    let path = Path::new(input);
    if !path.is_relative() {
        return std::borrow::Cow::Borrowed(input);
    }
    if path.extension().is_none() {
        let candidate = root.join(path).with_extension("md");
        if candidate.is_file() {
            let quoted = if input.contains(char::is_whitespace) {
                format!("\"{input}.md\"")
            } else {
                format!("{input}.md")
            };
            return std::borrow::Cow::Owned(quoted);
        }
    }
    if path.extension() == Some(std::ffi::OsStr::new("md"))
        && input.contains(char::is_whitespace)
        && root.join(path).is_file()
    {
        return std::borrow::Cow::Owned(format!("\"{input}\""));
    }
    std::borrow::Cow::Borrowed(input)
}

/// Runs `builder` against `store`, enabling class expansion when `has_classes`
/// is true.
///
/// # Errors
///
/// - [`CliError::SchemaQuery`] if loading schemas fails.
/// - [`CliError::Index`] if running the query against `store` fails.
fn run_query_builder_from_store(
    config: &Config,
    store: &IndexStore,
    builder: QueryBuilder,
    has_classes: bool,
) -> Result<QuerySet, CliError> {
    let root = config.root();
    let service = QueryService::new(config.schemas().class_field_name());
    let service = if has_classes {
        let schema_service = Arc::new(load_schema_service(config)?);
        service.with_class_expander(schema_service)
    } else {
        service
    };
    service.run_from_store(store, builder).map_err(|source| CliError::Index {
        root: root.to_path_buf(),
        source,
    })
}

/// Loads [`SchemaService`] from `config`'s schema directory and emits
/// construction warnings.
///
/// # Errors
///
/// - [`CliError::SchemaQuery`] if loading the schema directory fails.
fn load_schema_service(config: &Config) -> Result<SchemaService, CliError> {
    let root = config.root();
    let schema_directory = config.schemas().directory();
    let construction =
        SchemaService::load_verbose(schema_directory).map_err(|error| {
            CliError::SchemaQuery {
                root: root.to_path_buf(),
                source: error,
            }
        })?;
    warn_schema_construction_diagnostics(&construction);
    Ok(construction.service)
}

/// Wraps a [`QueryError`] as a [`CliError::Query`] against `root`.
fn query_error(root: &Path, source: QueryError) -> CliError {
    CliError::Query {
        root: root.to_path_buf(),
        source,
    }
}

/// Resolves trust subjects for `path` (or the current directory when absent) at
/// `all`'s scope.
///
/// Shared by [`trust::Trust`] and [`untrust::Untrust`].
///
/// # Errors
///
/// - [`CliError::CurrentDirectory`] if `path` is `None` and reading the current
///   directory fails.
/// - [`CliError::TrustTargetResolve`] if trust-target resolution fails.
fn resolve_trust_subjects(
    path: Option<&Path>,
    all: bool,
) -> Result<TrustRequests, CliError> {
    let cwd;
    let path = if let Some(path) = path {
        path
    } else {
        cwd = current_dir()?;
        cwd.as_ref()
    };
    let scope = if all {
        TrustScope::LocalSubtree
    } else {
        TrustScope::NearestLocal
    };
    ConfigService::trust_subjects(path, scope).map_err(|source| {
        CliError::TrustTargetResolve {
            path: path.to_path_buf(),
            source,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) mod fixtures {
        use std::{fs, path::Path, sync::Arc};

        use super::*;
        use crate::{
            cli::CwdGuard, dialog::PresetDialogProvider,
            test_support::TestProject,
        };

        /// Test [`DialogProvider`] that cancels selection prompts.
        pub(super) struct CancellingSelect;
        impl DialogProvider for CancellingSelect {
            fn is_interactive(&self) -> bool {
                true
            }

            fn text(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<String, crate::DialogError> {
                Ok(String::new())
            }

            fn confirm(
                &self,
                _: &str,
                _: Option<bool>,
            ) -> Result<bool, crate::DialogError> {
                Ok(true)
            }

            fn select(
                &self,
                _: &str,
                _: &[String],
            ) -> Result<usize, crate::DialogError> {
                Err(crate::DialogError::UserCancelled)
            }

            fn multi_select(
                &self,
                _: &str,
                _: &[String],
            ) -> Result<Vec<usize>, crate::DialogError> {
                Err(crate::DialogError::UserCancelled)
            }
        }

        /// Parses `argv` and drives it through [`Cli::run`] against an
        /// isolated, trusted project, writing (and returning the contents of)
        /// `daily.md`.
        pub(super) fn dispatch_argv_and_read_output(
            argv: &[&str],
            root: &Path,
        ) -> String {
            let cli = Cli::try_parse_from(argv).expect("parse argv");
            let project = TestProject::trusted(root.join("project"));
            project.write_template(
                "daily.md",
                "{% for n in [1, 2, 3] %}{{ n }}{% endfor %}",
            );
            let _guard = CwdGuard::enter(project.root());

            let provider: Arc<dyn DialogProvider> =
                Arc::new(PresetDialogProvider::new());
            cli.run(project.service(), provider).expect("run succeeds");

            fs::read_to_string(project.root().join("daily.md"))
                .expect("read written output")
        }
    }

    mod parse {
        use std::path::Path;

        use pretty_assertions::assert_eq;

        use super::*;

        /// Guards the `#[command(subcommand)]` wiring that [`run`] uses. Tests
        /// in `trust` check trust logic after parsing, so this test verifies
        /// that `traces trust ...` reaches [`Commands::Trust`] through [`Cli`].
        #[test]
        fn trust_argv_maps_to_trust_subcommand() {
            let cli = Cli::try_parse_from(["traces", "trust", "some/path"])
                .expect("parse trust argv");

            assert!(matches!(cli.command, Some(Commands::Trust(_))));
        }
        #[test]
        fn untrust_argv_maps_to_untrust_subcommand() {
            let cli = Cli::try_parse_from(["traces", "untrust", "some/path"])
                .expect("parse untrust argv");

            assert!(matches!(cli.command, Some(Commands::Untrust(_))));
        }

        #[test]
        fn init_argv_maps_to_init_subcommand() {
            let cli = Cli::try_parse_from(["traces", "init"])
                .expect("parse init argv");

            assert!(matches!(cli.command, Some(Commands::Init(_))));
        }

        #[test]
        fn index_argv_maps_to_index_subcommand() {
            let cli = Cli::try_parse_from(["traces", "index"])
                .expect("parse index argv");

            assert!(matches!(cli.command, Some(Commands::Index(_))));
        }

        #[test]
        fn list_argv_maps_to_list_subcommand() {
            let cli = Cli::try_parse_from(["traces", "list"])
                .expect("parse list argv");

            assert!(matches!(cli.command, Some(Commands::List(_))));
        }

        #[test]
        fn table_argv_maps_to_table_subcommand() {
            let cli = Cli::try_parse_from([
                "traces",
                "table",
                "--column",
                "file.path",
            ])
            .expect("parse table argv");

            assert!(matches!(cli.command, Some(Commands::Table(_))));
        }

        #[test]
        fn table_argv_without_a_column_flag_fails_to_parse() {
            let error = Cli::try_parse_from(["traces", "table"])
                .expect_err("table requires at least one --column");

            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
        }

        #[test]
        fn template_argv_maps_to_template_subcommand() {
            let cli =
                Cli::try_parse_from(["traces", "template", "-i", "daily"])
                    .expect("parse template argv");

            assert!(matches!(
                &cli.command,
                Some(Commands::Template(args))
                    if args.name().and_then(Path::to_str) == Some("daily")
            ));
        }

        #[test]
        fn template_positional_argv_maps_to_template_subcommand() {
            let cli = Cli::try_parse_from(["traces", "template", "daily"])
                .expect("parse template positional argv");

            assert!(matches!(
                &cli.command,
                Some(Commands::Template(args))
                    if args.name().and_then(Path::to_str) == Some("daily")
            ));
        }

        #[test]
        fn tmpl_alias_maps_to_template_subcommand() {
            let cli = Cli::try_parse_from(["traces", "tmpl", "-i", "daily"])
                .expect("parse tmpl argv");

            assert!(matches!(
                &cli.command,
                Some(Commands::Template(args))
                    if args.name().and_then(Path::to_str) == Some("daily")
            ));
        }

        #[test]
        fn completion_alias_maps_to_completions_subcommand() {
            let cli =
                Cli::try_parse_from(["traces", "completion", "--shell", "zsh"])
                    .expect("parse completion alias argv");

            assert!(matches!(&cli.command, Some(Commands::Completions(_))));
        }

        #[test]
        fn completions_shell_flag_accepts_bash() {
            let cli = Cli::try_parse_from([
                "traces",
                "completions",
                "--shell",
                "bash",
            ])
            .expect("parse completions --shell bash");

            assert!(matches!(&cli.command, Some(Commands::Completions(_))));
        }

        #[test]
        fn bare_input_flag_has_no_subcommand() {
            let cli = Cli::try_parse_from(["traces", "-i", "daily"])
                .expect("parse default -i argv");

            assert!(cli.command.is_none());
            assert_eq!(cli.input, Some(Some(PathBuf::from("daily"))));
        }

        #[test]
        fn bare_input_flag_without_value_sets_input_to_some_none() {
            let cli = Cli::try_parse_from(["traces", "-i"])
                .expect("parse bare -i flag");

            assert!(cli.command.is_none());
            assert_eq!(cli.input, Some(None));
        }

        #[test]
        fn no_args_has_no_command_and_no_input() {
            let cli =
                Cli::try_parse_from(["traces"]).expect("parse with no args");

            assert!(cli.command.is_none());
            assert_eq!(cli.input, None);
        }

        #[test]
        fn rejects_input_alongside_a_subcommand() {
            let error = Cli::try_parse_from(["traces", "init", "-i", "daily"])
                .expect_err("top-level input conflicts with subcommand");

            // `-i` after a subcommand is not that subcommand's flag, so clap
            // reports an unknown argument rather than an `ArgumentConflict`.
            assert_eq!(error.kind(), clap::error::ErrorKind::UnknownArgument);
        }

        #[test]
        fn template_list_flag_has_no_name() {
            let cli = Cli::try_parse_from(["traces", "template", "--list"])
                .expect("parse template --list argv");

            assert!(matches!(
                &cli.command,
                Some(Commands::Template(args))
                    if args.list && args.name.is_none()
            ));
        }

        #[test]
        fn template_list_flag_conflicts_with_input_name() {
            let error = Cli::try_parse_from([
                "traces", "template", "-i", "daily", "--list",
            ])
            .expect_err("template input conflicts with list");

            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
    }

    mod sort_args {
        use pretty_assertions::assert_eq;

        use super::*;

        fn resolve(keys: &[&str], asc: bool, desc: bool) -> Option<SortOrder> {
            let args = SortArgs {
                keys: keys.iter().map(|s| (*s).to_owned()).collect(),
                asc,
                desc,
            };
            args.resolve(Path::new("")).expect("valid sort")
        }

        /// Parses `input` the same way [`SortOrder::parse`] does, for asserting
        /// [`SortArgs::resolve`]'s output against the real grammar rather than
        /// decomposing its internal terms.
        fn expect_parse(
            input: &str,
            default_direction: SortDirection,
        ) -> SortOrder {
            SortOrder::parse(input, default_direction)
                .expect("valid parse")
                .expect("some terms")
        }

        #[test]
        fn returns_none_when_no_sort_fields_given() {
            assert_eq!(resolve(&[], false, false), None);
        }

        #[test]
        fn defaults_to_descending_without_flags() {
            let order =
                resolve(&["file.mtime"], false, false).expect("some sort");
            assert_eq!(
                order,
                expect_parse("file.mtime", SortDirection::Descending)
            );
        }

        #[test]
        fn asc_flag_reverses_every_unprefixed_field() {
            let order = resolve(&["file.folder", "file.name"], true, false)
                .expect("some sort");
            assert_eq!(
                order,
                expect_parse("file.folder,file.name", SortDirection::Ascending)
            );
        }

        #[test]
        fn prefix_modifiers_override_the_asc_flag() {
            let order = resolve(&["+file.folder", "-file.mtime"], true, false)
                .expect("some sort");
            assert_eq!(
                order,
                expect_parse(
                    "+file.folder,-file.mtime",
                    SortDirection::Ascending
                )
            );
        }

        #[test]
        fn blank_segments_from_trailing_comma_are_skipped() {
            let order =
                resolve(&["file.folder,"], false, false).expect("some sort");
            assert_eq!(
                order,
                expect_parse("file.folder", SortDirection::default())
            );
        }

        #[test]
        fn rejects_malformed_field_path() {
            let args = SortArgs {
                keys: vec!["file..bad".to_owned()],
                asc: false,
                desc: false,
            };
            assert!(matches!(
                args.resolve(Path::new("")),
                Err(CliError::Query {
                    source: QueryError::Builder(
                        crate::query::QueryBuilderError::FieldPath(_)
                    ),
                    ..
                })
            ));
        }
    }

    mod parse_source {
        use std::fs;

        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn appends_md_when_unadorned_path_exists_as_md() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::test_default(temp.path().to_path_buf());
            fs::write(temp.path().join("daily.md"), "# Daily")
                .expect("write daily.md");

            let source =
                parse_source(&config, Some("daily")).expect("parse source");
            assert_eq!(
                source,
                SourceSelector::parse("daily.md").expect("expected source")
            );
        }

        #[test]
        fn appends_md_for_nested_unadorned_path_when_file_exists() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::test_default(temp.path().to_path_buf());
            fs::create_dir_all(temp.path().join("notes")).expect("create dir");
            fs::write(temp.path().join("notes/daily.md"), "# Daily")
                .expect("write daily.md");

            let source = parse_source(&config, Some("notes/daily"))
                .expect("parse source");
            assert_eq!(
                source,
                SourceSelector::parse("notes/daily.md")
                    .expect("expected source")
            );
        }
        #[test]
        fn quotes_unadorned_path_with_spaces_when_file_exists() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::test_default(temp.path().to_path_buf());
            fs::create_dir_all(temp.path().join("my notes"))
                .expect("create dir");
            fs::write(temp.path().join("my notes/daily.md"), "# Daily")
                .expect("write daily.md");

            let source = parse_source(&config, Some("my notes/daily"))
                .expect("parse source");
            assert_eq!(
                source,
                SourceSelector::parse("\"my notes/daily.md\"")
                    .expect("expected source")
            );
        }

        #[test]
        fn quotes_direct_md_path_with_spaces_when_file_exists() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::test_default(temp.path().to_path_buf());
            fs::create_dir_all(temp.path().join("my notes"))
                .expect("create dir");
            fs::write(temp.path().join("my notes/daily.md"), "# Daily")
                .expect("write daily.md");

            let source = parse_source(&config, Some("my notes/daily.md"))
                .expect("parse source");
            assert_eq!(
                source,
                SourceSelector::parse("\"my notes/daily.md\"")
                    .expect("expected source")
            );
        }

        #[test]
        fn preserves_unadorned_path_when_no_md_file_exists() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::test_default(temp.path().to_path_buf());

            let source = parse_source(&config, Some("nonexistent"))
                .expect("parse source");
            assert_eq!(
                source,
                SourceSelector::parse("nonexistent").expect("expected source")
            );
        }

        #[test]
        fn returns_all_when_from_is_none() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::test_default(temp.path().to_path_buf());

            let source = parse_source(&config, None).expect("parse source");
            assert_eq!(source, SourceSelector::All);
        }

        #[test]
        fn preserves_tag_selector() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::test_default(temp.path().to_path_buf());

            let tag_source =
                parse_source(&config, Some("#daily")).expect("parse tag");
            assert_eq!(
                tag_source,
                SourceSelector::parse("#daily").expect("expected tag")
            );
        }

        #[test]
        fn preserves_class_selector() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let config = Config::test_default(temp.path().to_path_buf());

            let class_source =
                parse_source(&config, Some("@Daily")).expect("parse class");
            assert_eq!(
                class_source,
                SourceSelector::parse("@Daily").expect("expected class")
            );
        }
    }

    mod dispatch_end_to_end {
        use std::fs;

        use pretty_assertions::assert_eq;
        use rstest::rstest;

        use super::*;
        use crate::{
            ConfigService, cli::CwdGuard, dialog::PresetDialogProvider,
        };

        #[test]
        fn all_three_invocation_forms_produce_identical_output() {
            let form_a = tempfile::tempdir().expect("create temp dir");
            let form_b = tempfile::tempdir().expect("create temp dir");
            let form_c = tempfile::tempdir().expect("create temp dir");

            let via_template = fixtures::dispatch_argv_and_read_output(
                &["traces", "template", "-i", "daily"],
                form_a.path(),
            );
            let via_tmpl = fixtures::dispatch_argv_and_read_output(
                &["traces", "tmpl", "-i", "daily"],
                form_b.path(),
            );
            let via_default = fixtures::dispatch_argv_and_read_output(
                &["traces", "-i", "daily"],
                form_c.path(),
            );

            assert_eq!(via_template, "123");
            assert_eq!(via_tmpl, via_template);
            assert_eq!(via_default, via_template);
        }

        #[rstest]
        #[case::bare_dash_i(vec!["traces", "-i"])]
        #[case::bare_template_subcommand(vec!["traces", "template"])]
        fn bare_input_and_bare_template_both_reach_the_interactive_picker(
            #[case] argv: Vec<&str>,
        ) {
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().join("project");
            let service = ConfigService::for_test(
                temp.path().join("tracked-store"),
                temp.path().join("trust-store"),
            );
            let _ =
                crate::test_support::create_trusted_project(&service, &root);
            fs::create_dir_all(root.join("templates"))
                .expect("create empty templates dir");
            let _guard = CwdGuard::enter(&root);
            let provider: Arc<dyn DialogProvider> =
                Arc::new(PresetDialogProvider::new());
            let cli = Cli::try_parse_from(argv).expect("parse argv");

            let error = cli
                .run(&service, provider)
                .expect_err("no templates to pick from");

            assert!(matches!(error, CliError::NoTemplates));
        }
    }

    /// Covers end-to-end workflows for one shared project.
    ///
    /// These tests exercise shared-project behavior across five areas instead
    /// of isolated per-command tests:
    ///
    /// - Index building and syncing.
    /// - Page and task CLI queries.
    /// - Template `query`/`tasks` `QueryOps`.
    /// - Derived incoming links.
    /// - Diagnostics.
    ///
    /// See `cli::list`, `cli::table`, `cli::task`, and
    /// `template::engine::query` for exhaustive per-feature coverage.
    ///
    /// `list`, `table`, and `task` write primary output to standard output,
    /// which this module does not capture (see [`super::list::List::render`]).
    /// Their assertions drive [`WorkspaceIndex`] directly through the same
    /// interface those commands call.
    ///
    /// Diagnostics tests below and the `parse` and `dispatch_end_to_end` tests
    /// above test [`Cli::run`] dispatch directly wherever the result is on
    /// [`Result`].
    mod query_workflows {
        use std::{
            fs,
            path::{Path, PathBuf},
            sync::Arc,
        };

        use miette::Diagnostic as _;
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::{
            cli::CwdGuard,
            config::{Config, ConfigService},
            dialog::PresetDialogProvider,
            query::{QueryBuilderError, QueryError, SourceSelector},
            template::{
                TemplateError, TemplatePathInput, TemplateService, WriteMode,
                WriteOutcome,
            },
            test_support::TestProject,
        };

        /// Writes a trusted project under `root/project` with two notes.
        ///
        /// The notes include `#book` tags, a `rating` frontmatter field, one
        /// Markdown task, and a wikilink from `hyperion.md` to `dune.md`.
        /// Because `dune`'s stem is unique, the wikilink resolves unambiguously
        /// and gives `dune.md` a derived incoming link.
        ///
        /// Returns the trusted [`ConfigService`] (for [`Cli::run`] dispatch)
        /// and the project root (for direct [`WorkspaceIndex`] and
        /// [`TemplateService`] calls).
        fn seed_book_project(root: &Path) -> (ConfigService, PathBuf) {
            let project = TestProject::trusted(root.join("project"));
            project.write_template("daily.md", "");
            project.write_note(
                "books/dune.md",
                "---\nrating: 9\n---\n#book\n\n- [ ] read part two\n",
            );
            project.write_note(
                "books/hyperion.md",
                "---\nrating: 7\n---\n#book\n\nSee [[dune]] for comparison.\n",
            );
            (project.service().clone(), project.root().to_path_buf())
        }

        /// Renders `source` as a one-off template under `project`'s `templates`
        /// directory and returns its preview content.
        ///
        /// Mirrors `traces template -i report --dry-run` without going through
        /// [`ConfigService`] and trust. Uses [`Config::for_test`], matching
        /// render tests in [`crate::template::service`].
        fn render_query_template(project: &Path, source: &str) -> String {
            let templates_dir = project.join("templates");
            fs::write(templates_dir.join("report.md"), source)
                .expect("write report.md");
            let config = Config::test_default(project).with_templates();
            let service = TemplateService::new(
                &config,
                Arc::new(PresetDialogProvider::new()),
            )
            .expect("valid test schema directory");
            let input = TemplatePathInput::parse(Path::new("report"))
                .expect("valid template input");
            let outcome = service
                .render_to_file(&input, None, WriteMode::DryRun)
                .expect("template renders");
            let previewed = match outcome {
                WriteOutcome::Previewed(content) => Some(content),
                WriteOutcome::Written(_) => None,
            };
            previewed.expect("dry run always previews, never writes")
        }

        #[test]
        fn table_reflects_a_note_edit_made_between_two_cli_invocations_with_no_explicit_index_command()
         {
            let temp = tempfile::tempdir().expect("create temp dir");
            let (service, project) = seed_book_project(temp.path());
            let _guard = CwdGuard::enter(&project);

            // First invocation: no `.traces/index.redb` exists yet, so this
            // both builds and persists the index (the cold path of
            // `IndexerService::sync`), with no explicit `traces index` command
            // run.
            let first_outcome = Cli::try_parse_from([
                "traces",
                "table",
                "--column",
                "file.name",
                "--column",
                "rating",
            ])
            .expect("parse table argv")
            .run(&service, Arc::new(PresetDialogProvider::new()))
            .expect("table succeeds");
            assert_eq!(first_outcome, CommandOutcome::Completed);

            // Edit `dune.md` directly on disk, simulating an external editor;
            // no `traces index` command runs in between.
            fs::write(
                project.join("books/dune.md"),
                "---\nrating: 2\n---\n#book\n\n- [ ] read part two\n",
            )
            .expect("rewrite dune.md");

            let second_outcome = Cli::try_parse_from([
                "traces",
                "table",
                "--column",
                "file.name",
                "--column",
                "rating",
            ])
            .expect("parse table argv")
            .run(&service, Arc::new(PresetDialogProvider::new()))
            .expect("table succeeds");
            assert_eq!(second_outcome, CommandOutcome::Completed);

            // This test does not capture `table` standard output. Use `load()`
            // instead of `refresh_store()` to check that the edit reached the
            // store.
            let index = Arc::new(
                IndexerService::for_tests(&project)
                    .load()
                    .expect("load persisted index without refresh"),
            );
            let rendered = QueryService::new("class")
                .run(&index, QueryBuilder::pages(SourceSelector::All))
                .table(&["Name", "Rating"], &["file.name", "rating"])
                .expect("valid table");
            assert!(
                rendered.contains("| 2 "),
                "expected dune's updated rating to appear:\n{rendered}"
            );
            assert!(
                !rendered.contains("| 9 "),
                "expected dune's stale rating to be gone:\n{rendered}"
            );
        }

        #[test]
        fn template_query_ops_render_identically_to_the_equivalent_file_index_query()
         {
            let temp = tempfile::tempdir().expect("create temp dir");
            let (_service, project) = seed_book_project(temp.path());
            let indexer = IndexerService::for_tests(&project);
            indexer
                .persist(&indexer.build().expect("build index"))
                .expect("persist index");

            let rendered = render_query_template(
                &project,
                "{{ query.from(\"#book\").sort(\"rating\", \
                 true).table([\"Name\", \"Rating\"], [\"file.name\", \
                 \"rating\"]) }}",
            );

            let index = Arc::new(
                IndexerService::for_tests(&project)
                    .refresh()
                    .expect("refresh index"),
            );
            let expected = QueryService::new("class")
                .run(
                    &index,
                    QueryBuilder::pages(
                        SourceSelector::parse("#book").expect("valid source"),
                    ),
                )
                .sort("rating", true)
                .expect("valid sort")
                .table(&["Name", "Rating"], &["file.name", "rating"])
                .expect("valid table");

            assert_eq!(rendered, expected);
        }

        #[test]
        fn derived_inlinks_are_queryable_from_page_queries_and_templates() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let (_service, project) = seed_book_project(temp.path());
            let indexer = IndexerService::for_tests(&project);
            indexer
                .persist(&indexer.build().expect("build index"))
                .expect("persist index");

            let index = Arc::new(
                IndexerService::for_tests(&project)
                    .refresh()
                    .expect("refresh index"),
            );
            let inlinks = QueryService::new("class")
                .run(
                    &index,
                    QueryBuilder::pages(
                        SourceSelector::parse("books/").expect("valid source"),
                    ),
                )
                .sort("file.name", false)
                .expect("valid sort")
                .list("inlinks")
                .expect("valid list");
            assert_eq!(inlinks, "- books/hyperion.md\n- \n");

            let rendered = render_query_template(
                &project,
                "{{ query.from(\"books/\").sort(\"file.name\", \
                 false).list(\"inlinks\") }}",
            );
            assert_eq!(rendered, inlinks);
        }

        #[test]
        fn unknown_field_path_and_unparsable_filter_surface_actionable_cli_diagnostics()
         {
            let temp = tempfile::tempdir().expect("create temp dir");
            let (service, project) = seed_book_project(temp.path());
            let _guard = CwdGuard::enter(&project);

            let bad_field =
                Cli::try_parse_from(["traces", "list", "--sort", "file.nam"])
                    .expect("parse list argv")
                    .run(&service, Arc::new(PresetDialogProvider::new()))
                    .expect_err("unknown field path fails");
            assert!(matches!(
                &bad_field,
                CliError::Query {
                    source: QueryError::Builder(QueryBuilderError::FieldPath(error)),
                    ..
                } if error.suggestion.as_deref() == Some("file.name")
            ));
            assert_eq!(
                bad_field.code().map(|code| code.to_string()),
                Some("traces::cli::query::failed".to_owned())
            );
            assert!(bad_field.help().is_some());

            let bad_filter = Cli::try_parse_from([
                "traces",
                "list",
                "--where",
                "not a valid expression",
            ])
            .expect("parse list argv")
            .run(&service, Arc::new(PresetDialogProvider::new()))
            .expect_err("unparsable filter fails");
            assert!(matches!(bad_filter, CliError::Query {
                source: QueryError::Builder(QueryBuilderError::Syntax(_)),
                ..
            }));
        }

        #[test]
        fn template_render_errors_identify_the_failing_template_and_line_through_cli_dispatch()
         {
            let temp = tempfile::tempdir().expect("create temp dir");
            let (service, project) = seed_book_project(temp.path());
            fs::write(
                project.join("templates/report.md"),
                "line one\n{{ query.from().sort(\"nope.bad\") }}\n",
            )
            .expect("write report.md");
            let _guard = CwdGuard::enter(&project);

            let error = Cli::try_parse_from([
                "traces",
                "template",
                "-i",
                "report",
                "--dry-run",
            ])
            .expect("parse template argv")
            .run(&service, Arc::new(PresetDialogProvider::new()))
            .expect_err("malformed query call fails to render");

            assert!(matches!(error, CliError::TemplateInstantiate {
                source: TemplateError::Render { .. },
                ..
            }));
            let help = error.help().map(|h| h.to_string()).unwrap_or_default();
            assert!(
                help.contains("report.md:2:16"),
                "expected the failing template name, line, and column in help \
                 text, got: {help}"
            );
        }
    }

    mod run {
        use std::sync::Arc;

        use super::*;
        use crate::{
            cli::CwdGuard, config::ConfigService, dialog::PresetDialogProvider,
            test_support::TestProject,
        };

        struct InterruptingSelect;
        impl DialogProvider for InterruptingSelect {
            fn is_interactive(&self) -> bool {
                true
            }

            fn text(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<String, crate::DialogError> {
                Ok(String::new())
            }

            fn confirm(
                &self,
                _: &str,
                _: Option<bool>,
            ) -> Result<bool, crate::DialogError> {
                Ok(true)
            }

            fn select(
                &self,
                _: &str,
                _: &[String],
            ) -> Result<usize, crate::DialogError> {
                Err(crate::DialogError::UserInterrupted)
            }

            fn multi_select(
                &self,
                _: &str,
                _: &[String],
            ) -> Result<Vec<usize>, crate::DialogError> {
                Err(crate::DialogError::UserInterrupted)
            }
        }

        #[test]
        fn no_command_returns_no_command_error() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let service = ConfigService::for_test(
                temp.path().join("tracked-store"),
                temp.path().join("trust-store"),
            );
            let provider: Arc<dyn DialogProvider> =
                Arc::new(PresetDialogProvider::new());

            let error = Cli {
                command: None,
                input: None,
            }
            .run(&service, provider)
            .expect_err("no command should fail");

            assert!(matches!(error, CliError::NoCommand));
        }

        #[test]
        fn user_cancelled_during_template_picker_returns_aborted() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::trusted(temp.path().join("project"));
            project.write_template("daily.md", "content");
            let _guard = CwdGuard::enter(project.root());

            let outcome = Cli {
                command: None,
                input: Some(None),
            }
            .run(project.service(), Arc::new(fixtures::CancellingSelect))
            .expect("user abort is not an error");
            assert_eq!(outcome, CommandOutcome::Aborted(UserAbort::Cancelled));
        }

        /// Verifies that a user interrupt (`Ctrl-C`) during interactive dialog
        /// input maps to `CommandOutcome::Aborted(UserAbort::Interrupted)`.
        ///
        /// Chain of custody:
        ///
        /// 1. `DialogProvider` returns `DialogError::UserInterrupted` (e.g.
        ///    from SIGINT / Control-C).
        /// 2. `Template::run` wraps prompt errors into
        ///    `CliError::TemplatePicker { source }`.
        /// 3. `CliError::user_abort()` maps `DialogError::UserInterrupted` to
        ///    `UserAbort::Interrupted`.
        /// 4. `Cli::run` returns
        ///    `Ok(CommandOutcome::Aborted(UserAbort::Interrupted))`.
        /// 5. `src/main.rs` matches `Aborted(UserAbort::Interrupted)` and exits
        ///    with code 130.
        ///
        /// Defect class: untested contract (mapping link was previously
        /// untested).
        #[test]
        fn user_interrupted_during_template_picker_returns_aborted_interrupted()
        {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::trusted(temp.path().join("project"));
            project.write_template("daily.md", "content");
            let _guard = CwdGuard::enter(project.root());

            let outcome = Cli {
                command: None,
                input: Some(None),
            }
            .run(project.service(), Arc::new(InterruptingSelect))
            .expect("user abort is not an error");
            assert_eq!(
                outcome,
                CommandOutcome::Aborted(UserAbort::Interrupted)
            );
        }
    }

    mod current_dir_and_config {
        use std::fs;

        use super::*;
        use crate::{
            PresetDialogProvider, cli::CwdGuard, config::ConfigError,
            test_support::TestProject,
        };

        #[test]
        fn current_dir_reads_process_cwd() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let cwd = temp.path().canonicalize().expect("canonicalize");
            let guard = CwdGuard::enter(&cwd);
            let dir = current_dir().expect("current_dir succeeds");

            assert_eq!(dir.as_ref(), cwd);
            drop(guard);
        }

        #[test]
        fn load_config_fails_without_a_config_file() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let service = ConfigService::for_test(
                temp.path().join("tracked-store"),
                temp.path().join("trust-store"),
            );
            let _guard = CwdGuard::enter(temp.path());

            let error =
                load_config(&service).expect_err("no config should fail");

            assert!(matches!(error, CliError::ConfigLoad {
                source: ConfigError::Discovery(_),
                ..
            }));
        }

        #[test]
        fn cli_index_fails_before_indexing_when_task_status_uses_prohibited_delimiter()
         {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::empty(temp.path().join("project"));
            let traces_dir = project.root().join(".traces");
            fs::create_dir_all(&traces_dir).expect("create .traces dir");
            let config_toml = r#"
[[tasks.statuses]]
symbol = "["
name = "Invalid"
kind = "todo"
"#;
            fs::write(traces_dir.join("config.toml"), config_toml)
                .expect("write config.toml");
            project.trust();

            let _guard = CwdGuard::enter(project.root());

            let error = Cli::try_parse_from(["traces", "index"])
                .expect("parse index command")
                .run(project.service(), Arc::new(PresetDialogProvider::new()))
                .expect_err("prohibited delimiter config must fail");

            let is_prohibited = matches!(&error, CliError::ConfigLoad {
                source: crate::config::ConfigError::Value(
                    crate::config::ConfigValueError::Task(
                        crate::task::TaskError::ProhibitedStatusSymbol {
                            symbol: '[',
                        },
                    ),
                ),
                ..
            });
            assert!(
                is_prohibited,
                "error must preserve TaskError::ProhibitedStatusSymbol \
                 source, got: {error:?}"
            );

            let report = format!("{:?}", miette::Report::new(error));
            assert!(
                report.contains('['),
                "diagnostic report `{report}` must identify the prohibited \
                 symbol `[`"
            );

            assert!(
                !traces_dir.join("index.redb").exists(),
                "no index file must be created or mutated when config \
                 validation fails"
            );
        }
    }
}
