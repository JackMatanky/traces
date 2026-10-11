//! Process-level workflows: a first-run command chain and exact stream
//! contracts across mutating and inspecting commands.
//!
//! Every command runs through the spawned binary against sandbox-owned state.
//! The first-run workflow uses `Sandbox::from_dirs` so the sandbox operates on
//! the same directory initialized by `traces init`.

use pretty_assertions::assert_eq;

use super::support::{self, Sandbox};

/// Chains `init` → `trust` → `index` → `list` → `table` → `task` →
/// `template --dry-run` in one project directory, matching a real first
/// run.
///
/// Exercises seven spawned command steps in one project directory.
/// After explicit trust and index commands, subsequent queries and template
/// inspection succeed; this test does not assert the pre-trust failure path.
#[test]
fn init_trust_index_list_table_task_and_template_chain_through_one_project() {
    let root = tempfile::tempdir().expect("create project temp dir");
    let state_dir = tempfile::tempdir().expect("create state temp dir");
    let config_home = tempfile::tempdir().expect("create config home temp dir");
    let sandbox = Sandbox::from_dirs(root, state_dir, config_home);

    let init = sandbox.run(&["init"]);
    assert!(init.is_success(), "stderr: {}", init.stderr);

    // `init` scaffolds `.traces/` and writes local config, but does not
    // establish CLI trust (confirmed against `src/cli/init.rs`'s `run`, which
    // never calls `ConfigService::trust`) — an explicit `trust` step is
    // required before any other command can load this config.
    let trust = sandbox.run(&["trust"]);
    assert!(trust.is_success(), "stderr: {}", trust.stderr);

    sandbox.write_note(
        "notes/golden.md",
        "---\nrating: 8\n---\n\n- [ ] buy milk\n",
    );

    let index = sandbox.run(&["index"]);
    assert!(index.is_success(), "stderr: {}", index.stderr);

    let list = sandbox.run(&["list"]);
    assert!(list.is_success(), "stderr: {}", list.stderr);

    // `list` with no `--from` filter matches every project file, not only
    // Markdown Notes (Page/Note split, schema-query-decouple#1): `init` +
    // `trust` already scaffolded `.traces/config.toml` by this point.
    assert_eq!(list.stdout, "- notes/golden.md\n");

    let table =
        sandbox.run(&["table", "--column", "file.name", "--column", "rating"]);
    assert!(table.is_success(), "stderr: {}", table.stderr);
    assert!(
        table.stdout.contains("golden") && table.stdout.contains('8'),
        "stdout: {}",
        table.stdout
    );

    let task = sandbox.run(&["task"]);
    assert!(task.is_success(), "stderr: {}", task.stderr);
    assert!(task.stdout.contains("- [ ] buy milk"), "stdout: {}", task.stdout);

    // Notes live under `notes/`, scoped away from `.traces/templates/` (default
    // init's template directory): `query.from()` indexes every markdown file
    // under the project root, including the template file itself, so an
    // unscoped query here would also count `report.md`.
    sandbox.write_note(
        ".traces/templates/report.md",
        "{{ query.from(\"notes/\") | length }} note(s)",
    );

    let template =
        sandbox.run(&["template", "-i", "report", "--dry-run", "--no-input"]);
    assert!(template.is_success(), "stderr: {}", template.stderr);
    assert_eq!(template.stdout, "1 note(s)");
}

/// Verifies exact stdout/stderr stream contracts:
/// - Mutating commands (`trust`, `untrust`, `index`, `template -i ...
///   --no-input`) emit `stdout == ""` with status summaries on stderr.
/// - Inspecting commands (`trust list`, `trust --show`, `template --list`,
///   `completions --list-templates`) emit clean machine-readable stdout with
///   empty stderr.
///
/// Executes vs asserts:
/// - Executes: sequential trust, inspect, index, instantiate, and untrust
///   commands against one project.
/// - Asserts: exact stdout string, exact stderr string, code 0, and never 101.
///
/// Defect class: stream-split-regression (prevents mixing diagnostic and
/// payload streams).
#[test]
fn stream_contracts_exact_outputs_and_mutating_empty_stdout() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    sandbox.write_template("alpha.md", "Template alpha content\n");
    sandbox.write_note("notes/first.md", "# First note\n");

    // Mutating: `trust`
    let trust = sandbox.run(&["trust"]);
    trust.assert_code(0);
    trust.assert_never_exit_101();
    trust.assert_stdout_eq("");
    support::assert_reports_same_path(
        &trust.stderr,
        "trusted ",
        "\n",
        sandbox.root(),
    );

    // Query stream: `trust list`
    let trust_list = sandbox.run(&["trust", "list"]);
    trust_list.assert_code(0);
    trust_list.assert_never_exit_101();
    support::assert_reports_same_path(
        &trust_list.stdout,
        "",
        "\n",
        sandbox.root(),
    );
    trust_list.assert_stderr_eq("");

    // Query stream: `trust --show`
    let trust_show = sandbox.run(&["trust", "--show"]);
    trust_show.assert_code(0);
    trust_show.assert_never_exit_101();
    support::assert_reports_same_path(
        &trust_show.stdout,
        "",
        "\ttrusted\n",
        &sandbox.root().join(".traces/config.toml"),
    );
    trust_show.assert_stderr_eq("");

    // Query stream: `template --list`
    let template_list = sandbox.run(&["template", "--list"]);
    template_list.assert_code(0);
    template_list.assert_never_exit_101();
    template_list.assert_stdout_eq("alpha\n");
    template_list.assert_stderr_eq("");

    // Query stream: `completions --list-templates`
    let comp_list = sandbox.run(&["completions", "--list-templates"]);
    comp_list.assert_code(0);
    comp_list.assert_never_exit_101();
    comp_list.assert_stdout_eq("alpha\n");
    comp_list.assert_stderr_eq("");

    // Mutating: `index`
    let index = sandbox.run(&["index"]);
    index.assert_code(0);
    index.assert_never_exit_101();
    index.assert_stdout_eq("");
    support::assert_reports_same_path(
        &index.stderr,
        "indexed 2 file(s) under ",
        "\n",
        sandbox.root(),
    );

    // Mutating: `template -i alpha --no-input`
    let tmpl_inst = sandbox.run(&["template", "-i", "alpha", "--no-input"]);
    tmpl_inst.assert_code(0);
    tmpl_inst.assert_never_exit_101();
    tmpl_inst.assert_stdout_eq("");
    support::assert_reports_same_path(
        &tmpl_inst.stderr,
        "wrote ",
        "\n",
        &sandbox.root().join("alpha.md"),
    );

    // Mutating: `trust clean`
    let clean = sandbox.run(&["trust", "clean"]);
    clean.assert_code(0);
    clean.assert_never_exit_101();
    clean.assert_stdout_eq("");
    clean.assert_stderr_eq("removed 0 stale trust entries\n");

    // Mutating: `untrust`
    let untrust = sandbox.run(&["untrust"]);
    untrust.assert_code(0);
    untrust.assert_never_exit_101();
    untrust.assert_stdout_eq("");
    support::assert_reports_same_path(
        &untrust.stderr,
        "untrusted ",
        "\n",
        sandbox.root(),
    );
}
