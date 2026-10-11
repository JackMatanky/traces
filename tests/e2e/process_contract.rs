//! Process-boundary contracts: numeric exit codes and argv/usage failures.

use super::support::Sandbox;

/// Verifies that `--help` and `--version` exit with status 0, write expected
/// content to stdout, and emit nothing to stderr.
///
/// Executes vs asserts:
/// - Executes: `traces --help` and `traces --version`.
/// - Asserts: numeric code 0, never 101, empty stderr, exact/expected stdout.
///
/// Defect class: unwitnessed-contract (exit code & stream split contract).
#[test]
fn exit_code_zero_for_help_and_version() {
    let sandbox = Sandbox::new();

    let help = sandbox.run(&["--help"]);
    help.assert_success();
    help.assert_never_exit_101();
    assert!(help.stdout.contains("Usage:"));
    help.assert_stderr_eq("");

    let version = sandbox.run(&["--version"]);
    version.assert_code(0);
    version.assert_never_exit_101();
    version.assert_stdout_eq(concat!(
        "traces ",
        env!("CARGO_PKG_VERSION"),
        "\n"
    ));
    version.assert_stderr_eq("");
}

/// Unknown subcommands exit 2 and print usage to stderr.
#[test]
fn unknown_subcommand_exits_two_with_usage() {
    let sandbox = Sandbox::new();
    let unknown = sandbox.run(&["nonexistent_command"]);
    unknown.assert_code(2);
    unknown.assert_never_exit_101();
    unknown.assert_stdout_eq("");
    assert!(unknown.stderr.contains("unrecognized subcommand"));
    assert!(unknown.stderr.contains("Usage:"), "stderr: {}", unknown.stderr);
}

/// A missing required column exits 2 and names the flag in the diagnostic.
#[test]
fn table_without_column_exits_two_with_usage() {
    let sandbox = Sandbox::new();
    let missing_col = sandbox.run(&["table"]);
    missing_col.assert_code(2);
    missing_col.assert_never_exit_101();
    missing_col.assert_stdout_eq("");
    assert!(missing_col.stderr.contains("--column"));
    assert!(
        missing_col.stderr.contains("Usage:"),
        "stderr: {}",
        missing_col.stderr
    );
}

/// An unsupported completion shell exits 2 and directs the user to help.
#[test]
fn unsupported_completion_shell_exits_two_with_help_hint() {
    let sandbox = Sandbox::new();
    let invalid_shell = sandbox.run(&["completions", "--shell", "tcsh"]);
    invalid_shell.assert_code(2);
    invalid_shell.assert_never_exit_101();
    invalid_shell.assert_stdout_eq("");
    assert!(invalid_shell.stderr.contains("invalid value 'tcsh'"));
    assert!(
        invalid_shell.stderr.contains("try '--help'"),
        "stderr: {}",
        invalid_shell.stderr
    );
}

/// Verifies that domain failures exit with status 1 and write the miette
/// diagnostic to stderr with empty stdout.
///
/// Executes vs asserts:
/// - Executes: `traces list` in an untrusted workspace root.
/// - Asserts: numeric exit code 1, never 101, stdout `""`, and stderr
///   diagnostic.
///
/// Defect class: unwitnessed-contract (exit-1 domain failure contract).
#[test]
fn domain_failure_exits_code_one_for_untrusted_root() {
    let sandbox = Sandbox::new();
    sandbox.write_config();

    let list = sandbox.run(&["list"]);
    list.assert_code(1);
    list.assert_never_exit_101();
    list.assert_stdout_eq("");
    assert!(list.stderr.contains("config_build_untrusted"));
}
