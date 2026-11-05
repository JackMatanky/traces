//! Stream and message contracts for state-changing and inspecting commands:
//! which text goes to stdout, which to stderr, and the exact status lines.

use std::path::Path;

use rstest::rstest;
use tempfile::TempDir;

use super::support::{self, Run, Sandbox};

/// Every subcommand name a generated completion script must mention.
const SUBCOMMANDS: [&str; 10] = [
    "init",
    "trust",
    "untrust",
    "index",
    "list",
    "table",
    "task",
    "template",
    "completions",
    "tracked",
];

/// Runs `args` with the sandbox environment but inside `dir`.
fn run_in(sandbox: &Sandbox, dir: &Path, args: &[&str]) -> Run {
    let output =
        sandbox.command(args).current_dir(dir).output().expect("spawn traces");
    Run {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Creates a project directory with a config, trusts it, and loads it once so
/// both the trust store and the tracked-config store record it.
fn trusted_and_tracked_project(sandbox: &Sandbox) -> TempDir {
    let dir = TempDir::new().expect("create extra project");
    std::fs::create_dir_all(dir.path().join(".traces/templates"))
        .expect("create template dir");
    std::fs::write(
        dir.path().join(".traces/config.toml"),
        "[templates]\ndirectory = \".traces/templates\"\n",
    )
    .expect("write config");
    run_in(sandbox, dir.path(), &["trust"]).assert_success();
    run_in(sandbox, dir.path(), &["index"]).assert_success();
    dir
}

/// Removes every project directory so its trust and tracked entries go stale
/// (a trust entry records the project root, a tracked entry its config file).
fn make_stale(projects: &[TempDir]) {
    for project in projects {
        std::fs::remove_dir_all(project.path())
            .expect("remove project directory");
    }
}

/// `traces init` reports success on stderr only.
#[test]
fn init_reports_success_on_stderr_with_empty_stdout() {
    let sandbox = Sandbox::new();

    let init = sandbox.run(&["init"]);

    init.assert_success();
    init.assert_stdout_eq("");
    let first = init.stderr.lines().next().unwrap_or("");
    assert!(
        first.starts_with("initialised traces in"),
        "stderr: {}",
        init.stderr
    );
}

/// `traces trust --show` prints exactly one `{path}\t{status}` line to
/// stdout, comparing paths by canonical identity.
#[rstest]
#[case::trusted(true, "\ttrusted\n")]
#[case::untrusted(false, "\tuntrusted\n")]
fn trust_show_prints_path_tab_status(
    #[case] trust_first: bool,
    #[case] suffix: &str,
) {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    if trust_first {
        sandbox.run(&["trust"]).assert_success();
    }

    let show = sandbox.run(&["trust", "--show"]);

    show.assert_success();
    show.assert_stderr_eq("");
    support::assert_reports_same_path(
        &show.stdout,
        "",
        suffix,
        &sandbox.root().join(".traces/config.toml"),
    );
}

/// `trust clean` removes a stale entry and reports the singular count.
#[test]
fn trust_clean_removes_a_stale_entry_and_reports_one() {
    let sandbox = Sandbox::new();
    let project = trusted_and_tracked_project(&sandbox);
    make_stale(&[project]);

    let clean = sandbox.run(&["trust", "clean"]);

    clean.assert_success();
    clean.assert_stdout_eq("");
    clean.assert_stderr_eq("removed 1 stale trust entry\n");
    sandbox.run(&["trust", "list"]).assert_stdout_eq("");
}

/// `trust clean` reports the plural count for two stale roots.
#[test]
fn trust_clean_reports_the_plural_count_for_two_stale_roots() {
    let sandbox = Sandbox::new();
    let projects = [
        trusted_and_tracked_project(&sandbox),
        trusted_and_tracked_project(&sandbox),
    ];
    make_stale(&projects);

    let clean = sandbox.run(&["trust", "clean"]);

    clean.assert_success();
    clean.assert_stderr_eq("removed 2 stale trust entries\n");
}

/// `tracked clean` reports the plural count for two stale roots.
#[test]
fn tracked_clean_reports_the_plural_count_for_two_stale_roots() {
    let sandbox = Sandbox::new();
    let projects = [
        trusted_and_tracked_project(&sandbox),
        trusted_and_tracked_project(&sandbox),
    ];
    make_stale(&projects);

    let clean = sandbox.run(&["tracked", "clean"]);

    clean.assert_success();
    clean.assert_stdout_eq("");
    clean.assert_stderr_eq("removed 2 stale tracked entries\n");
}

/// Cleaning an empty store succeeds and reports zero entries.
#[rstest]
#[case::trust(&["trust", "clean"], "removed 0 stale trust entries\n")]
#[case::tracked(&["tracked", "clean"], "removed 0 stale tracked entries\n")]
fn clean_on_an_empty_store_reports_zero(
    #[case] args: &[&str],
    #[case] message: &str,
) {
    let sandbox = Sandbox::new();

    let clean = sandbox.run(args);

    clean.assert_success();
    clean.assert_stdout_eq("");
    clean.assert_stderr_eq(message);
}

/// Every supported shell script exits 0, registers each subcommand in that
/// shell's own syntax, and writes nothing to stderr.
///
/// The registration pattern (`{}` stands for the subcommand name) is
/// stricter than a substring match: descriptions such as "tracked-config
/// store" would otherwise satisfy a bare name check.
#[rstest]
#[case::bash("bash", "traces,{})")]
#[case::elvish("elvish", "cand {} '")]
#[case::fish("fish", "-a \"{}\"")]
#[case::powershell("powershell", "[CompletionResult]::new('{}',")]
#[case::zsh("zsh", "({})\n")]
fn completions_script_registers_every_subcommand(
    #[case] shell: &str,
    #[case] pattern: &str,
) {
    let sandbox = Sandbox::new();

    let run = sandbox.run(&["completions", "--shell", shell]);

    run.assert_success();
    run.assert_stderr_eq("");
    for name in SUBCOMMANDS {
        let expected = pattern.replace("{}", name);
        assert!(
            run.stdout.contains(&expected),
            "{shell} script must register `{name}` as {expected:?}"
        );
    }
}
