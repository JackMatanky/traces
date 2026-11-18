//! Semantic help contracts for the root command, subcommands, and aliases.
//! Command names, argument syntax, and documented options are stable surfaces;
//! explanatory prose and line wrapping are not.

use rstest::rstest;

use super::support::Sandbox;

/// Exercises help in an empty sandbox without requiring configuration.
///
/// Asserts exit 0, empty stderr, usage syntax, documented option signatures,
/// and nested command names. Aliases must expose their canonical command.
///
/// Defect class: unwitnessed-contract (user-facing help and flag surface).
#[rstest]
#[case::root(
    &["--help"],
    &["traces [OPTIONS]", "traces <COMMAND>"],
    &["-i, --input [<NAME>]", "-V, --version"],
    &[
        "init", "trust", "untrust", "index", "list", "table", "task",
        "template", "completions", "tracked", "help",
    ],
)]
#[case::init(&["init", "--help"], &["traces init"], &[], &[])]
#[case::trust(
    &["trust", "--help"],
    &["traces trust [OPTIONS] [PATH]", "traces trust <COMMAND>"],
    &["--show", "--all"],
    &["list", "clean", "help"],
)]
#[case::untrust(
    &["untrust", "--help"],
    &["traces untrust [OPTIONS] [PATH]"],
    &["--all"],
    &[],
)]
#[case::index(&["index", "--help"], &["traces index"], &[], &[])]
#[case::list(
    &["list", "--help"],
    &["traces list [OPTIONS]"],
    &["--from <FROM>", "--where <FILTER>", "--sort <KEYS>...", "--asc", "--desc"],
    &[],
)]
#[case::table(
    &["table", "--help"],
    &["traces table [OPTIONS] --column <COLUMNS>"],
    &[
        "--from <FROM>", "--where <FILTER>", "--sort <KEYS>...", "--asc",
        "--desc", "--column <COLUMNS>",
    ],
    &[],
)]
#[case::task(
    &["task", "--help"],
    &["traces task [OPTIONS]"],
    &[
        "--from <FROM>", "--where <FILTER>", "--todo", "--done",
        "--status <CHAR>", "--sort <KEYS>...", "--asc", "--desc",
        "-l, --line-numbers", "--table", "--column <COLUMNS>", "--count",
    ],
    &[],
)]
#[case::template(
    &["template", "--help"],
    &["traces template [OPTIONS] [NAME]"],
    &[
        "-i, --input <NAME>", "-l, --list", "-o, --output <PATH>",
        "-f, --force", "-n, --dry-run", "--no-input",
    ],
    &[],
)]
#[case::template_alias(
    &["tmpl", "--help"],
    &["traces template [OPTIONS] [NAME]"],
    &[
        "-i, --input <NAME>", "-l, --list", "-o, --output <PATH>",
        "-f, --force", "-n, --dry-run", "--no-input",
    ],
    &[],
)]
#[case::completions(
    &["completions", "--help"],
    &["traces completions <--shell <SHELL>|--list-templates>"],
    &["--shell <SHELL>", "--list-templates"],
    &[],
)]
#[case::completions_alias(
    &["completion", "--help"],
    &["traces completions <--shell <SHELL>|--list-templates>"],
    &["--shell <SHELL>", "--list-templates"],
    &[],
)]
#[case::tracked(
    &["tracked", "--help"],
    &["traces tracked <COMMAND>"],
    &[],
    &["list", "clean", "help"],
)]
fn help_exposes_command_contract(
    #[case] args: &[&str],
    #[case] usages: &[&str],
    #[case] options: &[&str],
    #[case] commands: &[&str],
) {
    let sandbox = Sandbox::new();
    let run = sandbox.run(args);
    run.assert_success();
    run.assert_stderr_eq("");

    // clap derives argv[0] from the executable name, including `.exe` on
    // Windows.
    let stdout = run.stdout.replace("traces.exe", "traces");
    for usage in usages {
        assert!(
            stdout.lines().any(|line| {
                let line = line.trim();
                line.strip_prefix("Usage: ").unwrap_or(line) == *usage
            }),
            "{args:?}: missing usage {usage:?}\n{stdout}",
        );
    }
    for option in std::iter::once("-h, --help").chain(options.iter().copied()) {
        assert!(
            stdout
                .lines()
                .skip_while(|line| line.trim() != "Options:")
                .skip(1)
                .any(|line| {
                    line.trim_start().strip_prefix(option).is_some_and(|rest| {
                        rest.is_empty() || rest.starts_with(char::is_whitespace)
                    })
                }),
            "{args:?}: missing option {option:?}\n{stdout}",
        );
    }
    for command in commands {
        assert!(
            stdout
                .lines()
                .skip_while(|line| line.trim() != "Commands:")
                .skip(1)
                .take_while(|line| {
                    line.is_empty() || line.starts_with(char::is_whitespace)
                })
                .any(|line| line.split_whitespace().next() == Some(*command)),
            "{args:?}: missing command {command:?}\n{stdout}",
        );
    }
}
