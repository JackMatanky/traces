//! Process-boundary contracts: numeric exit codes, argv/usage failures,
//! exact stream splits, and never-101 panic freedom.

use pretty_assertions::assert_eq;

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
/// - Asserts: numeric exit code 2, never 101, stdout `""`, and stderr
///   diagnostic.
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

    // Missing required flag: `table` requires at least one `--column`
    let missing_col = sandbox.run(&["table"]);
    missing_col.assert_code(2);
    missing_col.assert_never_exit_101();
    missing_col.assert_stdout_eq("");
    assert!(missing_col.stderr.contains("--column"));

    // Invalid enum value for `--shell`
    let invalid_shell = sandbox.run(&["completions", "--shell", "tcsh"]);
    invalid_shell.assert_code(2);
    invalid_shell.assert_never_exit_101();
    invalid_shell.assert_stdout_eq("");
    assert!(invalid_shell.stderr.contains("invalid value 'tcsh'"));
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

/// Verifies exact stdout/stderr stream contracts:
/// - Mutating commands (`trust`, `untrust`, `index`, `template -i ...
///   --no-input`) emit `stdout == ""` with status summaries on stderr.
/// - Inspecting commands (`trust list`, `trust --show`, `template --list`,
///   `completions --list-templates`) emit clean machine-readable stdout with
///   empty stderr.
///
/// Executes vs asserts:
/// - Executes: sequential trust, query, index, instantiate, and untrust
///   lifecycle.
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

    let canonical_root =
        sandbox.root().canonicalize().expect("canonicalize root");
    let root_str = canonical_root.to_string_lossy();
    let config_path_str = canonical_root
        .join(".traces/config.toml")
        .to_string_lossy()
        .into_owned();

    // Mutating: `trust`
    let trust = sandbox.run(&["trust"]);
    trust.assert_code(0);
    trust.assert_never_exit_101();
    trust.assert_stdout_eq("");
    trust.assert_stderr_eq(&format!("trusted {root_str}\n"));

    // Query stream: `trust list`
    let trust_list = sandbox.run(&["trust", "list"]);
    trust_list.assert_code(0);
    trust_list.assert_never_exit_101();
    trust_list.assert_stdout_eq(&format!("{root_str}\n"));
    trust_list.assert_stderr_eq("");

    // Query stream: `trust --show`
    let trust_show = sandbox.run(&["trust", "--show"]);
    trust_show.assert_code(0);
    trust_show.assert_never_exit_101();
    trust_show.assert_stdout_eq(&format!("{config_path_str}\ttrusted\n"));
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
    index.assert_stderr_eq(&format!("indexed 2 file(s) under {root_str}\n"));

    // Mutating: `template -i alpha --no-input`
    let tmpl_inst = sandbox.run(&["template", "-i", "alpha", "--no-input"]);
    tmpl_inst.assert_code(0);
    tmpl_inst.assert_never_exit_101();
    tmpl_inst.assert_stdout_eq("");
    tmpl_inst.assert_stderr_eq(&format!("wrote {root_str}/alpha.md\n"));

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
    untrust.assert_stderr_eq(&format!("untrusted {root_str}\n"));
}

/// Verifies `trust --all` and `untrust --all` mutating streams and asserts
/// the durable count via follow-up spawned `trust list`.
///
/// Executes vs asserts:
/// - Executes: `trust --all` then `trust list`, then `untrust --all` then
///   `trust list`.
/// - Asserts: stdout `""` on mutating calls, exact non-empty list, then exact
///   empty list.
///
/// Defect class: unwitnessed-contract (batch trust commands stream &
/// persistence contract).
#[test]
fn trust_all_and_untrust_all_count_asserted_via_trust_list() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    let canonical_root =
        sandbox.root().canonicalize().expect("canonicalize root");
    let root_str = canonical_root.to_string_lossy();

    // Trust via --all
    let trust_all = sandbox.run(&["trust", "--all"]);
    trust_all.assert_code(0);
    trust_all.assert_never_exit_101();
    trust_all.assert_stdout_eq("");

    // Verify root is in trust list
    let list_after_trust = sandbox.run(&["trust", "list"]);
    list_after_trust.assert_code(0);
    assert_eq!(list_after_trust.stdout, format!("{root_str}\n"));

    // Untrust via --all
    let untrust_all = sandbox.run(&["untrust", "--all"]);
    untrust_all.assert_code(0);
    untrust_all.assert_never_exit_101();
    untrust_all.assert_stdout_eq("");

    // Verify trust list is now empty
    let list_after_untrust = sandbox.run(&["trust", "list"]);
    list_after_untrust.assert_code(0);
    list_after_untrust.assert_stdout_eq("");
}
