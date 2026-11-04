//! Process-boundary recovery and simultaneous index writers.

use std::fs;

use super::support::Sandbox;

/// Two `traces index` processes race on one store. Both must exit 0 (or a
/// typed `traces::cli::index::failed` diagnostic, never a redb panic or exit
/// 101), the store must stay a valid redb database, and a following `list`
/// must render both notes. Spawning both before waiting acts as the barrier.
///
/// Defect class: concurrent-writer corruption.
#[test]
fn indexes_from_two_processes_leave_a_queryable_store() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "# A\n");
    sandbox.write_note("b.md", "# B\n");

    for run in sandbox.run_twice_concurrently(&["index"]) {
        let stderr = &run.stderr;
        run.assert_never_exit_101();
        assert!(
            !stderr.contains("panicked at") && !stderr.contains("redb::"),
            "raw database panic: {stderr}"
        );
        assert!(
            run.is_success() || stderr.contains("traces::cli::index::failed"),
            "index must succeed or report a typed error: {stderr}"
        );
    }

    let db = sandbox.root().join(".traces/index.redb");
    redb::Database::open(&db)
        .expect("concurrent writers leave a valid database");
    let listed = sandbox.run(&["list"]);
    listed.assert_success();
    listed.assert_stdout_eq("- a.md\n- b.md\n");
}

/// A garbage trust companion hash must classify the config as stale (exit 1,
/// `untrusted: stale`), never panic; a following `trust` grant rewrites the
/// hash and restores exit-0 queries.
///
/// Defect class: panic-on-corruption.
#[test]
fn rejects_a_corrupt_trust_companion_until_the_next_grant() {
    let sandbox = Sandbox::trusted_via_cli();
    let trust_dir = sandbox.state_dir().join("trusted-configs");
    let companion = fs::read_dir(&trust_dir)
        .expect("read trust store")
        .map(|entry| entry.expect("read store entry").path())
        .find(|path| {
            path.extension().is_some_and(|extension| extension == "hash")
        })
        .expect("trust grant writes companion hash");
    fs::write(&companion, "garbage").expect("corrupt companion");

    let stale = sandbox.run(&["list"]);
    stale.assert_code(1);
    stale.assert_never_exit_101();
    let flattened =
        stale.stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flattened.contains("untrusted: stale"),
        "corrupt companion must classify as stale: {}",
        stale.stderr
    );

    sandbox.run(&["trust"]).assert_success();
    let healed = sandbox.run(&["list"]);
    healed.assert_success();
    assert_ne!(
        fs::read_to_string(&companion).expect("read replacement hash"),
        "garbage",
        "next grant must replace the corrupt hash"
    );
}
