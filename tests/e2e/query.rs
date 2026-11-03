//! Query capability: class-expansion source selectors through the spawned
//! CLI, inlinks rendered from the persisted store, and the query-command
//! stream/diagnostic contracts (relocated from the dissolved `dispatch.rs`,
//! ticket 19 — relocation only, no assertion changes).

use pretty_assertions::assert_eq;

use super::support::{Sandbox, plain};

/// Paired CLI leg for the template composition-root class test (see
/// `tests/integration/template_render.rs`): `@class*` descendant expansion
/// resolves through the schema graph in a spawned `traces list`, and an
/// unknown class degrades to a literal match (no hierarchy, no error).
///
/// Executes vs asserts:
/// - Executes: schemas under `.traces/schemas/`, classed notes, `traces trust`,
///   `traces index`, then `traces list --from` with exact, descendant, and
///   unknown class selectors.
/// - Asserts: `@book*` lists exactly the `sci_fi` and `novel` classed notes
///   (transitive extends), `@book` matches nothing, `@horror` degrades to a
///   clean empty result (no note carries that class) — no error, no panic.
///
/// Defect class: reachability-transfer.
#[test]
fn class_descendant_selector_expands_through_the_schema_graph() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    sandbox.write_schema("book", "");
    sandbox.write_schema("sci_fi", "extends = [\"book\"]");
    sandbox.write_schema("novel", "extends = [\"sci_fi\"]");
    sandbox.write_note("shelf/a.md", "---\nclass: sci_fi\n---\n# A\n");
    sandbox.write_note("shelf/b.md", "---\nclass: novel\n---\n# B\n");
    sandbox.write_note("shelf/c.md", "# C\n");

    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();

    let descendants = sandbox.run(&["list", "--from", "@book*"]);
    descendants.assert_success();
    descendants.assert_never_exit_101();
    descendants.assert_stdout_eq("- shelf/a.md\n- shelf/b.md\n");

    let exact = sandbox.run(&["list", "--from", "@book"]);
    exact.assert_success();
    exact.assert_stdout_eq("");

    let unknown = sandbox.run(&["list", "--from", "@horror"]);
    unknown.assert_success();
    unknown.assert_never_exit_101();
    unknown.assert_stdout_eq("");
}

/// Sorts by a misspelled field and checks the "did you mean" suggestion
/// reaches captured stderr.
///
/// The suggestion logic itself is likely unit-tested elsewhere; only a
/// spawned process proves it survives Miette's fancy-diagnostic rendering
/// intact enough to still match as a substring.
#[test]
fn unknown_sort_field_reports_a_did_you_mean_suggestion() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "# A\n");

    let list = sandbox.run(&["list", "--sort", "file.nam"]);

    assert!(!list.is_success());
    assert!(list.stdout.is_empty(), "stdout: {}", list.stdout);
    assert!(
        list.stderr.contains("traces::cli::query::failed"),
        "stderr: {}",
        list.stderr
    );
    assert!(
        plain(&list.stderr).contains("did you mean `file.name`?"),
        "stderr: {}",
        list.stderr
    );
}

/// Passes an invalid `--where` expression on `list` and checks the CLI's
/// diagnostic-rendering path reports the expression location and repair.
#[test]
fn invalid_filter_expression_reports_its_location_and_repair() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "# A\n");

    let list = sandbox.run(&["list", "--where", "not a valid expression"]);

    assert!(!list.is_success());
    assert!(
        list.stderr.contains("traces::cli::query::failed"),
        "stderr: {}",
        list.stderr
    );
    assert!(
        plain(&list.stderr).contains("a comparison operator"),
        "stderr: {}",
        list.stderr
    );
}

/// Filters `list` by tag and checks matches land on stdout with a count on
/// stderr.
///
/// The stdout/stderr split is a CLI-only guarantee scripts pipe on; no
/// library query type has a stdout/stderr concept to unit-test against.
#[test]
fn list_prints_matching_pages_to_stdout_and_a_count_to_stderr() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("books/dune.md", "#book\n");
    sandbox.write_note("books/other.md", "# Other\n");

    let list = sandbox.run(&["list", "--from", "#book"]);

    assert!(list.is_success(), "stderr: {}", list.stderr);
    assert_eq!(list.stdout, "- books/dune.md\n");
    assert!(list.stderr.contains("1 page(s)"), "stderr: {}", list.stderr);
}

/// Passes an invalid `--where` expression on `task` (obsolete
/// `task.<field>` syntax) and checks the CLI's own diagnostic-rendering
/// path for the `task` subcommand, not just `list`'s.
///
/// The integration query file proves `QueryBuilder::filter` rejects
/// `task.<field>` at the library level; this proves the identical
/// diagnostic survives `task`'s own argv wiring and Miette rendering.
#[test]
fn task_invalid_where_reports_its_location_and_repair() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "- [ ] x\n");

    let task = sandbox.run(&["task", "--where", "task.completed == true"]);

    assert!(!task.is_success());
    assert!(
        task.stderr.contains("traces::cli::query::failed"),
        "stderr: {}",
        task.stderr
    );
    assert!(
        plain(&task.stderr).contains("did you mean `list.completed`?"),
        "stderr: {}",
        task.stderr
    );
}

/// Task count writes only a single integer to stdout.
#[test]
fn task_count_prints_only_the_match_total() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("todo.md", "- [ ] Alpha\n- [x] Beta\n");
    sandbox.write_note("other.md", "- [ ] Gamma\n");
    let count = sandbox.run(&["task", "--count"]);
    count.assert_success();
    count.assert_stdout_eq("3\n");
    count.assert_stderr_eq("");
}
