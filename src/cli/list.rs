//! Runs the page-level list query command.
//!
//! Handles `traces list` by refreshing the trusted root's
//! [`crate::index::WorkspaceIndex`], selecting a source scope, applying the
//! optional filter, and printing matching pages as a Markdown bullet list.

use clap::Args;

use super::{
    SortArgs,
    error::{CliError, CliResult},
};
use crate::Config;

/// Defines the field path rendered for each `traces list` bullet.
const LIST_FIELD: &str = "file.path";

/// Defines arguments for `traces list`.
///
/// Queries pages from the trusted project root and prints matching file paths
/// as a Markdown bullet list.
#[derive(Debug, Args)]
pub(super) struct List {
    /// Selects the source scope, such as `#tag`, `folder/`, or `@Class*`.
    ///
    /// Omit to query every indexed page.
    #[arg(long)]
    pub(super) from: Option<String>,
    /// Narrows results with a filter expression, such as
    /// `"file.folder == \"books\""`.
    ///
    /// Repeatable. Multiple `--where` flags combine with AND.
    #[arg(long = "where")]
    pub(super) filter: Vec<String>,
    /// Configures result sorting with [`SortArgs`].
    #[command(flatten)]
    pub(super) sort: SortArgs,
}

impl List {
    /// Runs `traces list` for the trusted project root.
    ///
    /// Refreshes the root's [`crate::index::WorkspaceIndex`] and writes one
    /// Markdown bullet per matching page to standard output.
    ///
    /// # Errors
    ///
    /// - [`CliError::Index`] if refreshing the [`crate::index::WorkspaceIndex`]
    ///   fails.
    /// - [`CliError::Query`] if `--where` is an invalid filter expression, the
    ///   parser rejects the source selector, or `--sort` names a malformed
    ///   field path.
    #[expect(
        clippy::print_stdout,
        reason = "list output is primary command output, not diagnostic text; \
                  mirrors the precedent in crate::cli::task"
    )]
    pub(super) fn run(&self, config: &Config) -> CliResult {
        let root = config.root();
        let (rendered, count) = self.render(config)?;
        print!("{rendered}");
        eprintln!("{count} page(s) from {}", root.display());
        Ok(())
    }

    /// Renders matching pages from `config`'s [`crate::index::WorkspaceIndex`]
    /// as a Markdown bullet list alongside the matched row count.
    ///
    /// Split from [`Self::run`] so tests can assert on rendered content without
    /// capturing process standard output. Uses `pub(super)` so
    /// `super::query_pipeline_tests` can drive it alongside
    /// [`super::table::Table::render`].
    ///
    /// # Errors
    ///
    /// - [`CliError::Index`] if refreshing the [`crate::index::WorkspaceIndex`]
    ///   fails.
    /// - [`CliError::Query`] if `--where` is an invalid filter expression, the
    ///   parser rejects the source selector, or `--sort` names a malformed
    ///   field path.
    pub(super) fn render(
        &self,
        config: &Config,
    ) -> Result<(String, usize), CliError> {
        let root = config.root();
        let order = self.sort.resolve(root)?;
        let rows = super::refresh_page_query(
            config,
            self.from.as_deref(),
            &self.filter,
            order,
        )?;
        let count = rows.len();
        let rendered = rows
            .list(LIST_FIELD)
            .map_err(|source| super::query_error(root, source))?;
        Ok((rendered, count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestProject;
    mod render {
        use std::{fs, path::Path};

        use pretty_assertions::assert_eq;

        use super::*;
        use crate::query::{QueryBuilderError, QueryError};

        fn config(root: &Path) -> Config {
            Config::test_default(root)
        }

        #[test]
        fn renders_a_bullet_per_page_path_in_sorted_order() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join("a.md"), "# A\n").expect("write a.md");
            fs::write(temp.path().join("b.md"), "# B\n").expect("write b.md");
            let list = List {
                from: None,
                filter: vec![],
                sort: SortArgs::default(),
            };

            let (rendered, count) =
                list.render(&config(temp.path())).expect("valid query");

            assert_eq!(rendered, "- a.md\n- b.md\n");
            assert_eq!(count, 2);
        }

        #[test]
        fn from_folder_selects_only_matching_pages() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::create_dir_all(temp.path().join("projects")).expect("mkdir");
            fs::write(temp.path().join("projects/a.md"), "# A\n")
                .expect("write projects/a.md");
            fs::write(temp.path().join("b.md"), "# B\n").expect("write b.md");
            let list = List {
                from: Some("projects/".to_owned()),
                filter: vec![],
                sort: SortArgs::default(),
            };

            let (rendered, count) =
                list.render(&config(temp.path())).expect("valid query");

            assert_eq!(rendered, "- projects/a.md\n");
            assert_eq!(count, 1);
        }

        #[test]
        fn from_parses_and_resolves_the_shared_source_dsl() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let schemas = temp.path().join(".traces/schemas");
            fs::create_dir_all(&schemas).expect("create Schema registry");
            fs::create_dir_all(temp.path().join("books"))
                .expect("create books");
            fs::write(schemas.join("book.toml"), "")
                .expect("write book Schema");
            fs::write(schemas.join("sci_fi.toml"), "extends = [\"book\"]\n")
                .expect("write sci_fi Schema");
            fs::write(
                temp.path().join("books/dune.md"),
                "---\nclass: sci_fi\n---\n#book\n",
            )
            .expect("write dune");
            fs::write(
                temp.path().join("books/movie.md"),
                "---\nclass: movie\n---\n#movie\n",
            )
            .expect("write movie");
            let list = List {
                from: Some(
                    "class(book).with_children() and \"books/dune.md\""
                        .to_owned(),
                ),
                filter: vec![],
                sort: SortArgs::default(),
            };

            let (rendered, count) =
                list.render(&config(temp.path())).expect("valid query");

            assert_eq!(rendered, "- books/dune.md\n");
            assert_eq!(count, 1);
        }

        #[test]
        fn from_reports_an_unparsable_source_expression() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join("a.md"), "#book\n").expect("write a.md");
            let list = List {
                from: Some("#book and".to_owned()),
                filter: vec![],
                sort: SortArgs::default(),
            };

            let error =
                list.render(&config(temp.path())).expect_err("invalid source");

            assert!(matches!(error, CliError::Query {
                source: QueryError::Builder(QueryBuilderError::Syntax(_)),
                ..
            }));
        }

        #[test]
        fn where_stacking_composes_as_and() {
            let temp = tempfile::tempdir().expect("create temp dir");
            fs::write(temp.path().join("a.md"), "---\nrating: 1\n---\n")
                .expect("write a.md");
            fs::write(temp.path().join("b.md"), "---\nrating: 5\n---\n")
                .expect("write b.md");
            fs::write(temp.path().join("c.md"), "---\nrating: 9\n---\n")
                .expect("write c.md");
            let list = List {
                from: None,
                filter: vec!["rating > 2".to_owned(), "rating < 9".to_owned()],
                sort: SortArgs::default(),
            };

            let (rendered, count) =
                list.render(&config(temp.path())).expect("valid query");

            assert_eq!(rendered, "- b.md\n");
            assert_eq!(count, 1);
        }
    }

    mod argv {
        use clap::Parser as _;
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::cli::{Cli, Commands};

        /// Extracts the parsed [`List`] arguments from `cli`.
        ///
        /// Uses an `Option`-returning match to assert the variant with
        /// `.expect()`. This avoids `panic!`, `unreachable!`, and
        /// `assert!(false, ..)`, which the crate `clippy` policy denies.
        fn list_args(cli: &Cli) -> &List {
            match &cli.command {
                Some(Commands::List(list)) => Some(list),
                _ => None,
            }
            .expect("expected the List subcommand")
        }

        #[test]
        fn parses_from_where_and_sort_flags() {
            let cli = Cli::try_parse_from([
                "traces",
                "list",
                "--from",
                "#tag",
                "--where",
                "rating > 5",
                "--sort",
                "rating",
                "--desc",
            ])
            .expect("parse list argv");

            let list = list_args(&cli);

            assert_eq!(list.from.as_deref(), Some("#tag"));
            assert_eq!(list.filter, vec!["rating > 5".to_owned()]);
            assert_eq!(list.sort.keys, vec!["rating".to_owned()]);
            assert!(list.sort.desc);
            assert!(!list.sort.asc);
        }

        #[test]
        fn repeated_where_flags_collect_into_one_vec_per_occurrence() {
            let cli = Cli::try_parse_from([
                "traces",
                "list",
                "--where",
                "rating > 2",
                "--where",
                "rating < 9",
            ])
            .expect("parse list argv");

            let list = list_args(&cli);

            assert_eq!(list.filter, vec![
                "rating > 2".to_owned(),
                "rating < 9".to_owned()
            ]);
        }

        #[test]
        fn defaults_from_where_and_sort_flags() {
            let cli = Cli::try_parse_from(["traces", "list"])
                .expect("parse list argv");

            let list = list_args(&cli);

            assert_eq!(list.from, None);
            assert_eq!(list.filter, Vec::<String>::new());
            assert_eq!(list.sort.keys, Vec::<String>::new());
            assert!(!list.sort.asc);
            assert!(!list.sort.desc);
        }

        #[test]
        fn rejects_conflicting_asc_and_desc_flags() {
            let error = Cli::try_parse_from([
                "traces", "list", "--sort", "rating", "--asc", "--desc",
            ])
            .expect_err("asc and desc conflict");
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }

        #[test]
        fn rejects_asc_without_sort_flag() {
            let error = Cli::try_parse_from(["traces", "list", "--asc"])
                .expect_err("--asc requires --sort");
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
        }
    }

    mod run {

        use super::*;

        #[test]
        fn succeeds_for_a_trusted_project_root() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::trusted(temp.path().join("project"));
            project.write_note("a.md", "# A\n");
            let list = List {
                from: None,
                filter: vec![],
                sort: SortArgs::default(),
            };

            list.run(&project.config()).expect("run list command");
        }
    }
}
