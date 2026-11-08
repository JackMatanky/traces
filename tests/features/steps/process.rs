//! Step definitions for spawned process execution, exit codes, and streams.

use pretty_assertions::assert_eq;
use rstest_bdd_macros::{then, when};

use super::{LastRun, Sandbox};
use crate::support::assert_reports_same_path;

/// Splits a quoted command into arguments, dropping an optional leading
/// `traces` word.
fn command_args(cmd: &str) -> Vec<&str> {
    let mut words = cmd.split_whitespace().peekable();
    words.next_if_eq(&"traces");
    words.collect()
}

/// Turns a Gherkin docstring into the exact text a stream must contain.
fn expected_text(docstring: &str) -> String {
    format!("{}\n", docstring.trim_matches('\n'))
}

#[when(r#"I run "{cmd}""#)]
fn run_command(sandbox: &Sandbox, last_run: &mut LastRun, cmd: String) {
    last_run.set(sandbox.run(&command_args(&cmd)));
}

#[when(r#"I run "{cmd}" in time zone "{zone}""#)]
fn run_command_in_time_zone(
    sandbox: &Sandbox,
    last_run: &mut LastRun,
    cmd: String,
    zone: String,
) {
    last_run.set(sandbox.run_with_envs(&command_args(&cmd), &[(
        "TZ",
        std::ffi::OsStr::new(&zone),
    )]));
}

#[then("the exit code is {code:i32}")]
fn exit_code_is(last_run: &LastRun, code: i32) {
    let run = last_run.run();
    run.assert_code(code);
    run.assert_never_exit_101();
}

#[then("the first line of stderr is:")]
fn first_line_of_stderr_is(last_run: &LastRun, docstring: String) {
    let first_line = last_run.run().stderr.lines().next().unwrap_or("");
    assert_eq!(first_line, docstring.trim_matches('\n'));
}

#[then("stdout is empty")]
fn stdout_is_empty(last_run: &LastRun) {
    let run = last_run.run();
    run.assert_stdout_eq("");
}

#[then("stderr is empty")]
fn stderr_is_empty(last_run: &LastRun) {
    let run = last_run.run();
    run.assert_stderr_eq("");
}

#[then(r#"stdout is "{expected}""#)]
fn stdout_is(last_run: &LastRun, expected: String) {
    let run = last_run.run();
    run.assert_stdout_eq(&expected);
}

#[then("stdout is:")]
fn stdout_is_docstring(last_run: &LastRun, docstring: String) {
    last_run.run().assert_stdout_eq(&expected_text(&docstring));
}

#[then("stderr is:")]
fn stderr_is_docstring(last_run: &LastRun, docstring: String) {
    last_run.run().assert_stderr_eq(&expected_text(&docstring));
}

#[then(r#"stderr reports "{prefix}" for the project root"#)]
fn stderr_reports_for_project_root(
    sandbox: &Sandbox,
    last_run: &LastRun,
    prefix: String,
) {
    let run = last_run.run();
    assert_reports_same_path(&run.stderr, &prefix, "\n", sandbox.root());
}
