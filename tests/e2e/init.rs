//! End-to-end tests for `traces init`.
//!
//! Exercises `traces init` through the spawned binary against an isolated
//! sandbox on null stdin, verifying process-level exit status and stderr
//! output.

use super::support::Sandbox;

/// Spawns `traces init` in a fresh sandbox on null stdin.
///
/// On non-TTY stdin (such as `/dev/null` in a spawned child), the terminal
/// dialog provider short-circuits to defaults rather than hanging, scaffolds
/// `.traces/`, and emits `initialised traces in <dir>` to stderr.
///
/// Executes vs asserts:
/// - Executes: spawned `traces init` with stdin piped from `/dev/null`.
/// - Asserts: process exits successfully (exit 0) and stderr contains
///   `initialised traces in`.
///
/// Defect class: unexecuted-dispatch-arm (previously never spawned at process
/// level).
#[test]
fn default_path_init_spawns_and_scaffolds_workspace() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&["init"]);

    assert!(
        run.is_success(),
        "traces init failed (exit status: {:?}): stderr: {}",
        run.status,
        run.stderr
    );
    assert!(
        run.stderr.contains("initialised traces in"),
        "expected 'initialised traces in' in stderr, got: {}",
        run.stderr
    );
    assert!(
        sandbox.root().join(".traces").is_dir(),
        ".traces directory was not created"
    );
    assert!(
        sandbox.root().join(".traces/config.toml").is_file(),
        ".traces/config.toml was not created"
    );
}
