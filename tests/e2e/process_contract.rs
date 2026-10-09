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
    version.assert_stdout_eq("traces 0.1.0\n");
    version.assert_stderr_eq("");
}

/// Verifies that argv failures exit with status 2 and print usage/error to
/// stderr, observing clap's process exit 2 which is structurally unreachable
/// in-crate.
///
/// Executes vs asserts:
/// - Executes: unknown subcommand (`nonexistent`), missing required flag
///   (`table` without `--column`), and invalid enum value (`completions --shell
///   tcsh`).
/// - Asserts: numeric exit code 2, never 101, stdout `""`, and stderr contains
///   the relevant diagnostic and usage or help guidance.
///
/// Defect class: unwitnessed-contract (clap exit-2 path unreachable in-crate).
#[test]
fn argv_failure_exits_code_two_on_invalid_command_or_flags() {
    let sandbox = Sandbox::new();

    // Unknown subcommand
    let unknown = sandbox.run(&["nonexistent_command"]);
    assert!(unknown.is_failure());
    unknown.assert_code(2);
    unknown.assert_never_exit_101();
    unknown.assert_stdout_eq("");
    assert!(unknown.stderr.contains("unrecognized subcommand"));
    assert!(unknown.stderr.contains("Usage:"), "stderr: {}", unknown.stderr);

    // Missing required flag: `table` requires at least one `--column`
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

    // Invalid enum value for `--shell`
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
