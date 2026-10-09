//! Single end-to-end proof of the full first-run user journey: `init` →
//! `trust` → `index` → `list` → `table` → `task` → `template --no-input`,
//! chained in one project directory.
//!
//! `init` runs in-process (via `PresetDialogProvider`, like `init.rs`) because
//! it has no non-interactive mode. Every subsequent command spawns the real
//! binary, matching `Sandbox`'s process-spawn model — but this test cannot
//! reuse `Sandbox` itself, since `Sandbox::new()` always creates its own fresh
//! tempdirs disconnected from the directory `init` scaffolded here. Isolation
//! env vars mirror `Sandbox::command`'s.

use pretty_assertions::assert_eq;

use super::support::Sandbox;

/// Chains `init` → `trust` → `index` → `list` → `table` → `task` →
/// `template --dry-run` in one project directory, matching a real first
/// run.
///
/// Exercises command sequence composition across seven spawned process steps
/// in one project directory, demonstrating that `trust` unblocks later commands
/// and `index` allows subsequent queries and template inspection to succeed.
///
/// Note: `list` could cold-rebuild on its own, and no command is attempted
/// before `trust` to assert an untrusted failure here.
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
