//! Query capability: class-expansion source selectors through the spawned
//! CLI, inlinks rendered from the persisted store, and the query-command
//! stream/diagnostic contracts (relocated from the dissolved `dispatch.rs`,
//! ticket 19 — relocation only, no assertion changes).

use pretty_assertions::assert_eq;

use super::support::{Sandbox, plain};

/// Shared `items.md` fixture for the filter-shortcut tests below: one task
/// per completion/status combination they each narrow differently.
const ITEMS_MD: &str = "- [ ] Bravo task\n- [x] Charlie done\n- [/] Alpha \
                        in-progress\n- [!] Delta on-hold\n";

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

/// Verifies inlinks render from the persisted store through a spawned query:
/// each CLI invocation is a cold process, so the rendered `inlinks` column
/// proves the cold link read persisted and survived the store round-trip.
///
/// The integration differential compares complete rows, including inlinks;
/// this test separately checks their rendering across the CLI process boundary.
///
/// Executes vs asserts:
/// - Executes: linked notes, `traces trust`, `traces index`, then `traces table
///   --column file.name --column inlinks`.
/// - Asserts: the target row's `inlinks` cell carries the linker path.
///
/// Defect class: silent-wrong-answers (cold-path link rows lost or
/// misattributed across the store round-trip).
#[test]
fn inlinks_render_from_the_persisted_store() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("notes/foo.md", "# Foo\n");
    sandbox.write_note("a.md", "see [[notes/foo]]\n");
    sandbox.run(&["index"]).assert_success();

    let table =
        sandbox.run(&["table", "--column", "file.name", "--column", "inlinks"]);
    table.assert_success();
    table.assert_never_exit_101();
    let foo_row = table
        .stdout
        .lines()
        .find(|line| line.starts_with("| foo"))
        .expect("foo row present");
    assert!(
        foo_row.contains("a.md"),
        "inlinks must render the linker; row: {foo_row}"
    );
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

/// Runs `table --column` and checks the rendered Markdown reaches stdout.
///
/// Proves `--column` argv parsing and rendering are wired together: each is
/// simple alone, but nothing below this test proves the connection.
#[test]
fn table_renders_a_markdown_table_with_one_row_per_page() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("dune.md", "---\nrating: 9\n---\n");

    let table =
        sandbox.run(&["table", "--column", "file.name", "--column", "rating"]);

    assert!(table.is_success(), "stderr: {}", table.stderr);
    assert!(
        table.stdout.contains("| file.name | rating |"),
        "stdout: {}",
        table.stdout
    );
    assert!(
        table.stdout.contains("dune") && table.stdout.contains('9'),
        "stdout: {}",
        table.stdout
    );
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

/// Runs `task` across custom status markers, nested outlines, and line
/// numbers, checking marker characters, indentation, and clickable
/// coordinates are rendered faithfully.
///
/// Collapses the deleted `task_prints_a_checkbox_line_per_task` (ticket 16,
/// ledger D5): a plain todo/done checkbox pair is folded into this
/// exact-stdout contract.
#[test]
fn task_preserves_custom_markers_indents_nested_depth_and_formats_line_numbers()
{
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note(
        "tasks.md",
        "- [ ] Root task\n- [x] Done plain\n  - [/] In progress\n    - [-] \
         Cancelled\n  - [!] On hold\n- [?] Unknown marker\n",
    );

    // 1. Plain task output:
    let task = sandbox.run(&["task"]);
    assert!(task.is_success(), "stderr: {}", task.stderr);
    let expected =
        "- [ ] Root task (tasks.md)\n- [x] Done plain (tasks.md)\n  - [/] In \
         progress (tasks.md)\n    - [-] Cancelled (tasks.md)\n  - [!] On hold \
         (tasks.md)\n- [?] Unknown marker (tasks.md)\n";
    assert_eq!(task.stdout, expected);

    // 2. Clickable line numbers (-l / --line-numbers):
    let task_lines = sandbox.run(&["task", "-l"]);
    assert!(task_lines.is_success(), "stderr: {}", task_lines.stderr);
    let expected_lines = "- [ ] Root task (tasks.md:1)\n- [x] Done plain \
                          (tasks.md:2)\n  - [/] In progress (tasks.md:3)\n    \
                          - [-] Cancelled (tasks.md:4)\n  - [!] On hold \
                          (tasks.md:5)\n- [?] Unknown marker (tasks.md:6)\n";
    assert_eq!(task_lines.stdout, expected_lines);
}

/// Runs `task` with shortcut filters (`--todo`, `--done`, `--status`),
/// verifying output filtering.
#[test]
fn task_filter_shortcuts_narrow_output_accurately() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("items.md", ITEMS_MD);

    // 1. Shortcut --todo: returns incomplete tasks (Todo, InProgress, OnHold),
    //    excludes Done
    let todo = sandbox.run(&["task", "--todo"]);
    assert!(todo.is_success(), "stderr: {}", todo.stderr);
    assert!(todo.stdout.contains("- [ ] Bravo task"));
    assert!(todo.stdout.contains("- [/] Alpha in-progress"));
    assert!(todo.stdout.contains("- [!] Delta on-hold"));
    assert!(!todo.stdout.contains("Charlie done"));

    // 2. Shortcut --done: returns only completed tasks
    let done = sandbox.run(&["task", "--done"]);
    assert!(done.is_success(), "stderr: {}", done.stderr);
    assert!(done.stdout.contains("- [x] Charlie done"));
    assert!(!done.stdout.contains("Bravo task"));
    assert!(!done.stdout.contains("Alpha in-progress"));

    // 3. Shortcut --status <char>: filters to exact status symbol
    let in_progress = sandbox.run(&["task", "--status", "/"]);
    assert!(in_progress.is_success(), "stderr: {}", in_progress.stderr);
    assert!(in_progress.stdout.contains("- [/] Alpha in-progress"));
    assert!(!in_progress.stdout.contains("Bravo task"));
    assert!(!in_progress.stdout.contains("Charlie done"));
}

/// Runs `task` with `--table`, `--count`, and `--from <file.md>`, proving
/// tabular output, count output, and direct note file resolution.
#[test]
fn task_renders_tables_outputs_counts_and_resolves_bare_markdown_sources() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note(
        "todo.md",
        "- [ ] Alpha item \u{1f4c5} 2026-10-01 \u{1f53a}\n- [x] Beta item \
         \u{1f4c5} 2026-10-05 \u{1f53d}\n",
    );
    sandbox.write_note("other.md", "- [ ] Gamma item\n");

    // 1. Default --table rendering:
    let table = sandbox.run(&["task", "--table"]);
    assert!(table.is_success(), "stderr: {}", table.stderr);
    assert!(
        table.stdout.contains("Task")
            && table.stdout.contains("Status")
            && table.stdout.contains("Due")
            && table.stdout.contains("Priority")
            && table.stdout.contains("File"),
        "table headers: {}",
        table.stdout
    );
    assert!(
        table.stdout.contains("Alpha item") && table.stdout.contains("highest")
    );
    assert!(table.stdout.contains("Beta item") && table.stdout.contains("low"));

    // 2. Custom --table columns:
    let custom_table = sandbox.run(&[
        "task",
        "--table",
        "--column",
        "list.text",
        "--column",
        "list.priority",
    ]);
    assert!(custom_table.is_success(), "stderr: {}", custom_table.stderr);
    assert!(
        custom_table.stdout.contains("| list.text")
            && custom_table.stdout.contains("list.priority |"),
        "stdout: {}",
        custom_table.stdout
    );
    assert!(
        custom_table.stdout.contains("Alpha item")
            && custom_table.stdout.contains("highest")
    );

    // 3. Count output: single integer on stdout, completely empty stderr
    let count = sandbox.run(&["task", "--count"]);
    assert!(count.is_success(), "stderr: {}", count.stderr);
    assert_eq!(count.stdout, "3\n");
    assert_eq!(count.stderr, "");

    // 4. Resolving bare .md paths via --from:
    let from_bare = sandbox.run(&["task", "--from", "todo.md"]);
    assert!(from_bare.is_success(), "stderr: {}", from_bare.stderr);
    assert!(from_bare.stdout.contains("Alpha item (todo.md)"));
    assert!(from_bare.stdout.contains("Beta item (todo.md)"));
    assert!(!from_bare.stdout.contains("Gamma item"));

    // Count on bare .md source:
    let from_count = sandbox.run(&["task", "--from", "todo.md", "--count"]);
    assert!(from_count.is_success(), "stderr: {}", from_count.stderr);
    assert_eq!(from_count.stdout, "2\n");
    assert_eq!(from_count.stderr, "");
}

/// Runs `task --from` against a bare `.md` path that does not exist on
/// disk, checking it degrades to a graceful zero-match result rather than
/// erroring or silently resolving to some other source kind.
///
/// `normalize_source_input` (`src/cli/mod.rs`) only rewrites `--from` into a
/// `.md` file source when `candidate.is_file()` succeeds; a nonexistent bare
/// path falls through unchanged into `SourceSelector::parse`, which still
/// succeeds and simply matches nothing. This proves that fallthrough is
/// visible as "0 task(s)", not a crash or a confusing match against an
/// unrelated source kind.
#[test]
fn task_from_nonexistent_markdown_path_matches_nothing_without_erroring() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("todo.md", "- [ ] real task\n");

    let from_missing = sandbox.run(&["task", "--from", "missing.md"]);

    assert!(from_missing.is_success(), "stderr: {}", from_missing.stderr);
    assert_eq!(from_missing.stdout, "");
    assert!(
        from_missing.stderr.contains("0 task(s)"),
        "stderr: {}",
        from_missing.stderr
    );
}
