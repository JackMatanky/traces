//! Shell-completion capability: the sole E2E representative proves the
//! `completions --shell` argv path reaches a real script on stdout.

use super::support::Sandbox;

/// Checks `completions --shell zsh` prints the `#compdef traces`
/// marker to stdout.
///
/// Sole E2E representative per the ticket 16 ledger (D9): the bash and
/// fish twins are deleted; in-crate shell marker units own the per-shell
/// script shapes.
#[test]
fn zsh_shell_prints_a_completion_script() {
    let sandbox = Sandbox::trusted_via_cli();

    let completions = sandbox.run(&["completions", "--shell", "zsh"]);

    assert!(completions.is_success(), "stderr: {}", completions.stderr);
    assert!(
        completions.stdout.contains("#compdef traces"),
        "stdout: {}",
        completions.stdout
    );
}
