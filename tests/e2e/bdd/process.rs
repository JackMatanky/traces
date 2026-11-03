//! Step definitions for spawned process execution, exit codes, and streams.

use pretty_assertions::assert_eq;
use rstest_bdd_macros::{then, when};

use super::{LastRun, Sandbox};

#[when(r#"I run "{cmd}""#)]
fn run_command(sandbox: &Sandbox, last_run: &mut LastRun, cmd: String) {
    let raw_args = cmd
        .strip_prefix("traces ")
        .or_else(|| cmd.strip_prefix("traces"))
        .unwrap_or(&cmd);
    let args: Vec<&str> = raw_args.split_whitespace().collect();
    last_run.0 = Some(sandbox.run(&args));
}

#[then("the exit code is {code}")]
fn exit_code_is(last_run: &LastRun, code: i32) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    run.assert_code(code);
    run.assert_never_exit_101();
}

#[then("the first line of stderr is:")]
fn first_line_of_stderr_is(last_run: &LastRun, docstring: String) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    let first_line = run.stderr.lines().next().unwrap_or("");
    assert_eq!(first_line, docstring.trim());
}

#[then("the first line of stdout is:")]
fn first_line_of_stdout_is(last_run: &LastRun, docstring: String) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    let first_line = run.stdout.lines().next().unwrap_or("");
    assert_eq!(first_line, docstring.trim());
}

#[then("stdout is empty")]
fn stdout_is_empty(last_run: &LastRun) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    run.assert_stdout_eq("");
}

#[then("stderr is empty")]
fn stderr_is_empty(last_run: &LastRun) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    run.assert_stderr_eq("");
}

#[then(r#"stdout is "{expected}""#)]
fn stdout_is(last_run: &LastRun, expected: String) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    run.assert_stdout_eq(&expected);
}

#[then(r#"stderr is "{expected}""#)]
fn stderr_is(last_run: &LastRun, expected: String) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    run.assert_stderr_eq(&expected);
}

#[then("stdout is:")]
fn stdout_is_docstring(last_run: &LastRun, docstring: String) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    run.assert_stdout_eq(&format!("{}\n", docstring.trim()));
}

#[then("stderr is:")]
fn stderr_is_docstring(last_run: &LastRun, docstring: String) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    run.assert_stderr_eq(&format!("{}\n", docstring.trim()));
}

#[then(r#"stderr reports "{prefix}" for the project root"#)]
fn stderr_reports_for_project_root(
    sandbox: &Sandbox,
    last_run: &LastRun,
    prefix: String,
) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    super::super::support::assert_reports_same_path(
        &run.stderr,
        &prefix,
        "\n",
        sandbox.root(),
    );
}
