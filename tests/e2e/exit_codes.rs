//! Boundary exit-code pins: every reachable clap conflict/requires rule exits
//! 2, the default `-i` dispatch, and the diagnostics that no other spawned
//! test witnesses (tracked-store failures, invalid init template directory,
//! schema load failure).

use pretty_assertions::assert_eq;
use rstest::rstest;

use super::support::{Run, Sandbox};

/// Asserts the boundary contract of a domain failure: exit 1, empty stdout,
/// and the diagnostic code alone on the first stderr line.
fn assert_domain_failure(run: &Run, code: &str) {
    run.assert_code(1);
    run.assert_never_exit_101();
    run.assert_stdout_eq("");
    assert_eq!(
        run.stderr.lines().next().unwrap_or(""),
        code,
        "unexpected first diagnostic line; stderr: {}",
        run.stderr
    );
}

/// Pins exit 2 for every clap conflict, requires, and unknown-argument rule
/// reachable at the process boundary.
///
/// Executes vs asserts:
/// - Executes: one spawned `traces` per table row, each violating one rule.
/// - Asserts: exit code 2, empty stdout, `Usage:` on stderr, and an error
///   fragment naming the offending flag.
///
/// Defect class: unwitnessed-contract (argv validation exit code).
#[rstest]
#[case::task_todo_with_done(&["task", "--todo", "--done"], "--todo")]
#[case::task_column_without_table(&["task", "--column", "file.name"], "--table")]
#[case::template_dry_run_with_output(
    &["template", "--dry-run", "-o", "x"],
    "--dry-run"
)]
#[case::template_name_with_input(
    &["template", "daily", "-i", "daily"],
    "--input"
)]
#[case::template_list_with_name(&["template", "--list", "daily"], "--list")]
#[case::init_rejects_input_flag(&["init", "-i", "daily"], "-i")]
#[case::trust_action_with_all(&["trust", "list", "--all"], "--all")]
#[case::trust_action_with_show(&["trust", "clean", "--show"], "--show")]
#[case::trust_action_with_path(&["trust", "list", "somewhere"], "somewhere")]
#[case::completions_without_mode(&["completions"], "--shell")]
#[case::completions_shell_with_list_templates(
    &["completions", "--shell", "bash", "--list-templates"],
    "--list-templates"
)]
#[case::root_input_with_dry_run(&["-i", "daily", "--dry-run"], "--dry-run")]
#[case::list_asc_with_desc(
    &["list", "--sort", "file.name", "--asc", "--desc"],
    "--asc"
)]
#[case::list_asc_without_sort(&["list", "--asc"], "--sort")]
#[case::table_asc_with_desc(
    &["table", "--column", "file.name", "--sort", "file.name", "--asc", "--desc"],
    "--asc"
)]
#[case::table_without_column(&["table"], "--column")]
fn rejects_invalid_arguments_with_exit_two(
    #[case] args: &[&str],
    #[case] fragment: &str,
) {
    let sandbox = Sandbox::new();

    let run = sandbox.run(args);

    run.assert_code(2);
    run.assert_never_exit_101();
    run.assert_stdout_eq("");
    assert!(
        run.stderr.contains("Usage:"),
        "missing Usage: in stderr: {}",
        run.stderr
    );
    assert!(
        run.stderr.contains(fragment),
        "stderr must name `{fragment}`: {}",
        run.stderr
    );
}

/// Pins the default `traces -i` dispatch (flag without a value) with
/// templates present: stdin is not a terminal, so the picker refuses to run
/// instead of silently choosing a template.
///
/// Executes vs asserts:
/// - Executes: bare `traces -i` in a trusted project that has a template.
/// - Asserts: exit 1, empty stdout, first stderr line is the picker diagnostic
///   code, and nothing was written to disk.
///
/// Defect class: nondeterministic-TTY.
#[test]
fn refuses_bare_input_flag_without_a_terminal() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_template("daily.md", "rendered");

    let run = sandbox.run(&["-i"]);

    assert_domain_failure(&run, "traces::cli::template::picker_failed");
    assert!(!sandbox.root().join("daily.md").exists());
}

/// Pins `traces -i <name>` dispatch: it renders like `traces template -i
/// <name>` and writes the file at the default output path.
#[test]
fn renders_named_input_flag_like_the_template_command() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_template("daily.md", "rendered");

    let run = sandbox.run(&["-i", "daily"]);

    run.assert_success();
    run.assert_stdout_eq("");
    assert_eq!(
        std::fs::read_to_string(sandbox.root().join("daily.md"))
            .expect("read rendered file"),
        "rendered"
    );
}

/// Restores a directory's permissions on drop so the sandbox can delete it
/// even when an assertion panics.
#[cfg(unix)]
struct Unreadable<'a>(&'a std::path::Path);

#[cfg(unix)]
impl<'a> Unreadable<'a> {
    /// Creates `dir` and removes every permission bit from it. Returns
    /// `None` when the process can still read the directory (for example
    /// when running as root), where the failure cannot be provoked.
    fn new(dir: &'a std::path::Path) -> Option<Self> {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::create_dir_all(dir).expect("create tracked store");
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o0))
            .expect("remove directory permissions");
        let guard = Self(dir);
        if std::fs::read_dir(dir).is_ok() {
            eprintln!("skipping: directory permissions are not enforced");
            return None;
        }
        Some(guard)
    }
}

#[cfg(unix)]
impl Drop for Unreadable<'_> {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(
            self.0,
            std::fs::Permissions::from_mode(0o755),
        );
    }
}

/// Pins `tracked list` and `tracked clean` failures when the tracked-config
/// store root exists but cannot be read.
///
/// A corrupt store *file* is deliberately tolerated (an absent or
/// non-directory root is read as empty), so the only boundary-reachable
/// failure is an unreadable store directory.
///
/// Executes vs asserts:
/// - Executes: the command against a tracked store directory with mode 000.
/// - Asserts: exit 1, empty stdout, first stderr line is the code.
///
/// Defect class: unwitnessed-contract (store failure diagnostic).
#[cfg(unix)]
#[rstest]
#[case::list("list", "traces::cli::tracked::list_failed")]
#[case::clean("clean", "traces::cli::tracked::clean_failed")]
fn reports_failure_for_an_unreadable_tracked_store(
    #[case] action: &str,
    #[case] code: &str,
) {
    let sandbox = Sandbox::new();
    let store = sandbox.state_dir().join("tracked-configs");
    let Some(_guard) = Unreadable::new(&store) else {
        return;
    };

    let run = sandbox.run(&["tracked", action]);

    assert_domain_failure(&run, code);
}

/// Pins `traces init` refusing a template directory that resolves outside the
/// project, and that it leaves nothing behind.
///
/// A non-terminal `init` always uses the default `.traces/templates`
/// directory, so the only way to reach the diagnostic is a `.traces` symlink
/// that does not yet resolve (`exists()` is false) but would point outside
/// the project.
#[cfg(unix)]
#[test]
fn rejects_template_dir_escaping_through_a_dangling_symlink() {
    let sandbox = Sandbox::new();
    let outside = tempfile::tempdir().expect("create outside dir");
    std::os::unix::fs::symlink(
        outside.path().join("missing"),
        sandbox.root().join(".traces"),
    )
    .expect("create dangling .traces symlink");

    let run = sandbox.run(&["init"]);

    assert_domain_failure(&run, "traces::cli::init::invalid_template_dir");
    assert!(!outside.path().join("missing").exists());
}

/// Pins the schema-load diagnostic for a schema file that cannot be parsed.
///
/// Executes vs asserts:
/// - Executes: `traces list --from @book` with a malformed schema TOML.
/// - Asserts: exit 1, empty stdout, first stderr line is the code.
///
/// Defect class: unwitnessed-contract (schema failure diagnostic).
#[test]
fn reports_schema_failure_for_a_malformed_schema_file() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_schema("book", "this is = = not toml");
    sandbox.write_note("a.md", "---\nclass: book\n---\n# A\n");

    let run = sandbox.run(&["list", "--from", "@book"]);

    assert_domain_failure(&run, "traces::cli::query::schema_failed");
}
