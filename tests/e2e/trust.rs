//! Proves the trust → untrust → subsequent-command-fails round trip at the
//! process boundary — previously only unit-tested in-process.

use pretty_assertions::assert_eq;

use super::support::Sandbox;

/// Untrusts a trusted project, then checks a later `list` fails with the
/// untrusted diagnostic.
///
/// Needs two real process invocations: the contract is that untrust
/// survives past its own process and is honored on the next config load,
/// which a single-process unit test can't distinguish from a plain store
/// mutation.
#[test]
fn untrust_then_list_fails_with_the_untrusted_diagnostic() {
    let sandbox = Sandbox::trusted_via_cli();
    let untrust = sandbox.run(&["untrust"]);
    assert!(untrust.is_success(), "stderr: {}", untrust.stderr);

    let list = sandbox.run(&["list"]);
    assert!(!list.is_success());
    assert!(
        list.stderr.contains("traces::cli::config_build_untrusted"),
        "stderr: {}",
        list.stderr
    );
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

    // Trust via --all
    let trust_all = sandbox.run(&["trust", "--all"]);
    trust_all.assert_code(0);
    trust_all.assert_never_exit_101();
    trust_all.assert_stdout_eq("");

    // Verify root is in trust list: compare canonicalized filesystem
    // identity, not raw string form, since Windows's `current_dir` can
    // echo back a short (8.3) alias distinct from the sandbox's own
    // un-resolved tempdir path despite both naming the same directory.
    let list_after_trust = sandbox.run(&["trust", "list"]);
    list_after_trust.assert_code(0);
    let reported_root = list_after_trust.stdout.trim_end_matches('\n');
    assert_eq!(
        std::path::Path::new(reported_root)
            .canonicalize()
            .expect("canonicalize reported root"),
        sandbox.root().canonicalize().expect("canonicalize sandbox root"),
        "trust list root mismatch; stdout: {}",
        list_after_trust.stdout
    );

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
