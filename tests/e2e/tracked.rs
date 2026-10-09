//! Proves `traces tracked list` and `traces tracked clean` through spawned
//! commands and a real tracked-config store.
//!
//! The tracked-config store is populated by `ConfigService::load` (see
//! `src/cli/tracked.rs`'s module docs), not by `trust`.
//! `Sandbox::trusted_via_cli()` only runs `traces trust`, so every test here
//! runs a load-triggering command (`index`) first to record the local config as
//! a tracked entry.

use super::support::Sandbox;

/// Runs `index` to populate the tracked-config store, then checks `tracked
/// list` prints the project's config path.
///
/// `src/config/store.rs` unit-tests the store logic directly. This is the
/// only test proving `tracked list`'s argv parsing and stdout formatting
/// are actually wired to it.
#[test]
fn list_prints_every_tracked_config_path() {
    let sandbox = Sandbox::trusted_via_cli();
    let index = sandbox.run(&["index"]);
    assert!(index.is_success(), "stderr: {}", index.stderr);

    let list = sandbox.run(&["tracked", "list"]);
    assert!(list.is_success(), "stderr: {}", list.stderr);
    assert!(
        list.stdout.contains(".traces/config.toml"),
        "stdout: {}",
        list.stdout
    );
}

/// Deletes a tracked entry's config file to make it stale, then checks
/// `tracked clean` reports the removal in its structured message.
///
/// The staleness check runs against a real path on a real filesystem after
/// a real deletion — a fixture-backed unit test could fake staleness
/// without proving `clean` actually rechecks disk state. The assertion is
/// the full structured message (count + noun, singular), not a trivia
/// `contains('1')` (ticket 16, §14 P2.24).
#[test]
fn clean_removes_a_stale_tracked_entry_and_reports_the_count() {
    let sandbox = Sandbox::trusted_via_cli();
    let index = sandbox.run(&["index"]);
    assert!(index.is_success(), "stderr: {}", index.stderr);
    std::fs::remove_file(sandbox.root().join(".traces/config.toml"))
        .expect("delete tracked config to make the entry stale");

    let clean = sandbox.run(&["tracked", "clean"]);
    clean.assert_success();
    clean.assert_stderr_eq("removed 1 stale tracked entry\n");
}
