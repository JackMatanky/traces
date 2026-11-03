//! Index capability: refresh deltas observed through spawned processes —
//! content edits, path-set changes, store-read divergence, and corruption
//! self-heal.

use std::fs;

use super::support::Sandbox;

/// Known limitation (arch-design ticket 25): a same-size content edit with
/// the mtime restored is invisible to a spawned query while metadata
/// fingerprints match. An explicit `traces index` rebuild sees the edit.
/// This timing canary remains a Rust e2e test, not a BDD scenario.
///
/// Executes vs asserts:
/// - Executes: `traces index`, `traces task`, a same-length note edit with the
///   original mtime restored, `traces task` again, then `traces index` +
///   `traces task`.
/// - Asserts: the post-edit query still shows the OLD text (store reload); the
///   post-rebuild query shows the NEW text.
///
/// Defect class: unobservable-reload.
#[test]
fn same_size_mtime_restored_edit_reads_the_store_not_a_rebuild() {
    let sandbox = Sandbox::trusted_via_cli();
    let note = sandbox.root().join("a.md");
    fs::write(&note, "# A\n\n- [ ] alpha task\n").expect("write note");
    let original_mtime = fs::metadata(&note)
        .expect("read note metadata")
        .modified()
        .expect("read note mtime");

    sandbox.run(&["index"]).assert_success();
    let before = sandbox.run(&["task"]);
    before.assert_success();
    before.assert_stdout_eq("- [ ] alpha task (a.md)\n");

    // Same byte length, same mtime: metadata fingerprints stay unchanged.
    fs::write(&note, "# A\n\n- [ ] omega task\n").expect("rewrite note");
    fs::File::options()
        .write(true)
        .open(&note)
        .expect("open note to restore mtime")
        .set_times(fs::FileTimes::new().set_modified(original_mtime))
        .expect("restore original mtime");

    let reloaded = sandbox.run(&["task"]);
    reloaded.assert_success();
    reloaded.assert_never_exit_101();
    reloaded.assert_stdout_eq("- [ ] alpha task (a.md)\n");

    let rebuilt_index = sandbox.run(&["index"]);
    rebuilt_index.assert_success();
    let rebuilt = sandbox.run(&["task"]);
    rebuilt.assert_success();
    rebuilt.assert_stdout_eq("- [ ] omega task (a.md)\n");
}

/// Verifies corruption self-heal at process level: replacing the entire index
/// with garbage makes the next spawned query exit 0 with correct output, a
/// rebuilt store, and no panic.
///
/// Executes vs asserts:
/// - Executes: `traces index`, overwrite `.traces/index.redb` with garbage,
///   then `traces list`, then a further edit + `traces index` + `traces list`
///   to prove the store accepts writes.
/// - Asserts: exit 0, never 101, exact pre-corruption output, and the post-heal
///   edit query-visible.
///
/// Defect class: panic-on-corruption.
#[test]
fn fully_corrupted_index_self_heals_on_the_next_query() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "# A\n");
    sandbox.write_note("b.md", "# B\n");
    sandbox.run(&["index"]).assert_success();

    let db = sandbox.root().join(".traces/index.redb");
    fs::write(&db, b"garbage").expect("replace index with garbage");

    let healed = sandbox.run(&["list"]);
    healed.assert_success();
    healed.assert_never_exit_101();
    healed.assert_stdout_eq("- a.md\n- b.md\n");
    assert!(db.is_file(), "query must restore the index file");
    redb::Database::open(&db).expect("query must restore a valid database");

    // The healed store accepts writes again.
    sandbox.write_note("c.md", "# C\n");
    sandbox.run(&["index"]).assert_success();
    let grown = sandbox.run(&["list"]);
    grown.assert_success();
    grown.assert_stdout_eq("- a.md\n- b.md\n- c.md\n");
}
