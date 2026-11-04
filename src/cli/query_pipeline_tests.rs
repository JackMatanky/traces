//! Pipeline tests shared by the page-query commands (`list` and `table`).
//!
//! Both commands run the same pipeline: resolve the sort order against the
//! trusted root, refresh the [`WorkspaceIndex`], apply the source selector and
//! `--where` filters, then render. These rstest tables pin that shared
//! pipeline once per behaviour for both modes; each command's own
//! formatting-specific tests stay beside its implementation.
//!
//! [`WorkspaceIndex`]: crate::index::WorkspaceIndex

use std::{fs, path::Path};

use pretty_assertions::assert_eq;
use rstest::rstest;

use super::{SortArgs, error::CliError, list::List, table::Table};
use crate::{
    Config, DirTreeError,
    index::IndexError,
    query::{QueryBuilderError, QueryError},
};

/// Which page-query command the table drives.
#[derive(Copy, Clone, Debug)]
enum Mode {
    /// `traces list`: Markdown bullets of `file.path`.
    List,
    /// `traces table`: a Markdown table with one `file.path` column.
    Table,
}

/// Either command, exposing the common `render` seam.
enum Query {
    List(List),
    Table(Table),
}

impl Query {
    fn render(self, config: &Config) -> Result<(String, usize), CliError> {
        match self {
            Self::List(list) => list.render(config),
            Self::Table(table) => table.render(config),
        }
    }
}

/// Builds the command for `mode` with the shared `file.path` column for
/// `table` (the only column both modes' expected outputs reference).
fn query(
    mode: Mode,
    from: Option<String>,
    filter: Vec<String>,
    sort: SortArgs,
) -> Query {
    match mode {
        Mode::List => Query::List(List {
            from,
            filter,
            sort,
        }),
        Mode::Table => Query::Table(Table {
            from,
            filter,
            sort,
            columns: vec!["file.path".to_owned()],
        }),
    }
}

fn config(root: &Path) -> Config {
    Config::test_default(root)
}

#[rstest]
#[case::list(Mode::List, "- a.md\n- b.md\n")]
#[case::table(
    Mode::Table,
    "| file.path |\n|-----------|\n| a.md      |\n| b.md      |\n"
)]
fn sort_orders_by_the_named_field_with_explicit_ascending(
    #[case] mode: Mode,
    #[case] expected: &str,
) {
    let temp = tempfile::tempdir().expect("create temp dir");
    fs::write(temp.path().join("a.md"), "---\nrating: 3\n---\n")
        .expect("write a.md");
    fs::write(temp.path().join("b.md"), "---\nrating: 9\n---\n")
        .expect("write b.md");
    let cmd = query(mode, None, vec![], SortArgs {
        keys: vec!["rating".to_owned()],
        asc: true,
        desc: false,
    });

    let (rendered, count) =
        cmd.render(&config(temp.path())).expect("valid query");

    assert_eq!(rendered, expected);
    assert_eq!(count, 2);
}

#[rstest]
#[case::list(Mode::List, "- b.md\n- a.md\n")]
#[case::table(
    Mode::Table,
    "| file.path |\n|-----------|\n| b.md      |\n| a.md      |\n"
)]
fn desc_reverses_the_sort_order(#[case] mode: Mode, #[case] expected: &str) {
    let temp = tempfile::tempdir().expect("create temp dir");
    fs::write(temp.path().join("a.md"), "---\nrating: 3\n---\n")
        .expect("write a.md");
    fs::write(temp.path().join("b.md"), "---\nrating: 9\n---\n")
        .expect("write b.md");
    let cmd = query(mode, None, vec![], SortArgs {
        keys: vec!["rating".to_owned()],
        asc: false,
        desc: false,
    });

    let (rendered, count) =
        cmd.render(&config(temp.path())).expect("valid query");

    assert_eq!(rendered, expected);
    assert_eq!(count, 2);
}

#[rstest]
#[case::list(Mode::List)]
#[case::table(Mode::Table)]
fn rejects_a_malformed_sort_field_path(#[case] mode: Mode) {
    let temp = tempfile::tempdir().expect("create temp dir");
    fs::write(temp.path().join("a.md"), "# A\n").expect("write a.md");
    let cmd = query(mode, None, vec![], SortArgs {
        keys: vec!["file..bad".to_owned()],
        asc: false,
        desc: false,
    });

    let error =
        cmd.render(&config(temp.path())).expect_err("malformed sort fails");

    assert!(matches!(error, CliError::Query {
        source: QueryError::Builder(QueryBuilderError::FieldPath(_)),
        ..
    }));
}

#[rstest]
#[case::list(Mode::List, "- a.md\n")]
#[case::table(Mode::Table, "| file.path |\n|-----------|\n| a.md      |\n")]
fn from_tag_selects_only_matching_pages(
    #[case] mode: Mode,
    #[case] expected: &str,
) {
    let temp = tempfile::tempdir().expect("create temp dir");
    fs::write(temp.path().join("a.md"), "#projects\n").expect("write a.md");
    fs::write(temp.path().join("b.md"), "#books\n").expect("write b.md");
    let cmd =
        query(mode, Some("#projects".to_owned()), vec![], SortArgs::default());

    let (rendered, count) =
        cmd.render(&config(temp.path())).expect("valid query");

    assert_eq!(rendered, expected);
    assert_eq!(count, 1);
}

#[rstest]
#[case::list(Mode::List, "- a.md\n")]
#[case::table(Mode::Table, "| file.path |\n|-----------|\n| a.md      |\n")]
fn where_filters_by_frontmatter_field(
    #[case] mode: Mode,
    #[case] expected: &str,
) {
    let temp = tempfile::tempdir().expect("create temp dir");
    fs::write(temp.path().join("a.md"), "---\nrating: 9\n---\n")
        .expect("write a.md");
    fs::write(temp.path().join("b.md"), "---\nrating: 3\n---\n")
        .expect("write b.md");
    let cmd =
        query(mode, None, vec!["rating > 5".to_owned()], SortArgs::default());

    let (rendered, count) =
        cmd.render(&config(temp.path())).expect("valid query");

    assert_eq!(rendered, expected);
    assert_eq!(count, 1);
}

#[rstest]
#[case::list(Mode::List)]
#[case::table(Mode::Table)]
fn rejects_unparsable_filter_expression(#[case] mode: Mode) {
    let temp = tempfile::tempdir().expect("create temp dir");
    fs::write(temp.path().join("a.md"), "# A\n").expect("write a.md");
    let cmd = query(
        mode,
        None,
        vec!["not a valid expression".to_owned()],
        SortArgs::default(),
    );

    let error =
        cmd.render(&config(temp.path())).expect_err("unparsable filter fails");

    assert!(matches!(error, CliError::Query {
        source: QueryError::Builder(QueryBuilderError::Syntax(_)),
        ..
    }));
}

#[cfg(unix)]
#[rstest]
#[case::list(Mode::List)]
#[case::table(Mode::Table)]
fn wraps_refresh_failure_as_a_cli_index_error(#[case] mode: Mode) {
    use std::os::unix::fs::PermissionsExt as _;

    struct RestorePermissions<'a>(&'a Path);

    impl Drop for RestorePermissions<'_> {
        fn drop(&mut self) {
            let _ =
                fs::set_permissions(self.0, fs::Permissions::from_mode(0o700));
        }
    }

    let temp = tempfile::tempdir().expect("create temp dir");
    let locked = temp.path().join("locked");
    fs::create_dir(&locked).expect("create locked dir");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000))
        .expect("revoke read permission");
    let _restore = RestorePermissions(&locked);
    let cmd = query(mode, None, vec![], SortArgs::default());

    let error = cmd
        .render(&config(temp.path()))
        .expect_err("unreadable subdirectory fails");

    assert!(matches!(error, CliError::Index {
        source: IndexError::Scan(DirTreeError::NodeInaccessible { .. }),
        ..
    }));
}
