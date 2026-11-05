//! Help-text snapshots: `--help` for the root command, every subcommand, and
//! every subcommand alias, compared byte-for-byte with the files in
//! `tests/baselines/help/`.
//!
//! Set `TRACES_UPDATE_GOLDEN=1` to rewrite the baseline files from the real
//! binary output instead of comparing. Review the resulting diff before
//! committing it: a changed baseline is a changed user-facing contract.

use std::path::PathBuf;

use pretty_assertions::assert_eq;
use rstest::rstest;

use super::support::Sandbox;

/// Returns the baseline file for `name` under `tests/baselines/help/`.
fn baseline_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/baselines/help")
        .join(format!("{name}.txt"))
}

/// Strips trailing spaces from every line: clap pads blank doc-comment lines,
/// and the repository's whitespace hook rejects baselines that keep them.
fn strip_trailing_spaces(text: &str) -> String {
    text.split_inclusive('\n')
        .map(|line| match line.strip_suffix('\n') {
            Some(body) => format!("{}\n", body.trim_end_matches(' ')),
            None => line.trim_end_matches(' ').to_owned(),
        })
        .collect()
}

/// Pins the exact `--help` text of every command and alias.
///
/// Executes vs asserts:
/// - Executes: `traces [<sub>] --help` in an empty sandbox.
/// - Asserts: exit 0, empty stderr, and stdout (minus trailing spaces) equal to
///   the baseline file. Aliases share the baseline of the command they alias.
///
/// Defect class: unwitnessed-contract (user-facing help text and flag
/// surface).
#[rstest]
#[case::root(&["--help"], "root")]
#[case::init(&["init", "--help"], "init")]
#[case::trust(&["trust", "--help"], "trust")]
#[case::untrust(&["untrust", "--help"], "untrust")]
#[case::index(&["index", "--help"], "index")]
#[case::list(&["list", "--help"], "list")]
#[case::table(&["table", "--help"], "table")]
#[case::task(&["task", "--help"], "task")]
#[case::template(&["template", "--help"], "template")]
#[case::template_alias(&["tmpl", "--help"], "template")]
#[case::completions(&["completions", "--help"], "completions")]
#[case::completions_alias(&["completion", "--help"], "completions")]
#[case::tracked(&["tracked", "--help"], "tracked")]
fn help_text_matches_baseline(#[case] args: &[&str], #[case] baseline: &str) {
    let sandbox = Sandbox::new();

    let run = sandbox.run(args);
    // clap derives the usage name from argv[0], which ends in `.exe` on
    // Windows.
    let actual =
        strip_trailing_spaces(&run.stdout).replace("traces.exe", "traces");

    run.assert_success();
    run.assert_stderr_eq("");
    let path = baseline_path(baseline);
    if std::env::var_os("TRACES_UPDATE_GOLDEN").is_some_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().expect("baseline parent"))
            .expect("create baseline dir");
        std::fs::write(&path, &actual).expect("write help baseline");
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .expect(
            "read help baseline; run with TRACES_UPDATE_GOLDEN=1 to create it",
        )
        .replace("\r\n", "\n");
    assert_eq!(actual, expected);
}
