//! Index capability: refresh deltas observed through spawned processes —
//! content edits, path-set changes, store-read divergence, and corruption
//! self-heal.

use std::fs;

use super::support::Sandbox;

/// Verifies a content edit between two spawned queries is query-visible.
///
/// Executes vs asserts:
/// - Executes: `traces index`, then `traces task`, then a note edit, then
///   `traces task` again — each spawn is a fresh process reading the store.
/// - Asserts: the first run shows the original task text; the second shows the
///   edited text (the query path re-parses the changed note).
///
/// Defect class: stale-secondary-index.
#[test]
fn content_edit_between_spawns_is_query_visible() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "# A\n\n- [ ] old task\n");
    sandbox.run(&["index"]).assert_success();

    let first = sandbox.run(&["task"]);
    first.assert_success();
    first.assert_stdout_eq("- [ ] old task (a.md)\n");

    sandbox.write_note("a.md", "# A\n\n- [ ] new task\n");

    let second = sandbox.run(&["task"]);
    second.assert_success();
    second.assert_never_exit_101();
    second.assert_stdout_eq("- [ ] new task (a.md)\n");
}

/// Verifies the path-set refresh branch between spawns: a deleted file's
/// ghost row vanishes and wikilinks re-resolve through the distinct
/// full-recompute path.
///
/// `[[foo]]` is ambiguous while `notes/foo.md` and `archive/foo.md` both
/// exist, so it resolves to nothing; deleting `archive/foo.md` must make
/// the same link resolve to `notes/foo.md` on the next spawned query —
/// without an explicit re-index in between.
///
/// Executes vs asserts:
/// - Executes: `traces index`, `traces list` + `traces table` (baseline), a
///   file deletion, then `traces list` + `traces table` again.
/// - Asserts: the deleted path disappears from `list`; the surviving target's
///   `inlinks` column shows the previously-ambiguous linker.
///
/// Defect class: stale-secondary-index (ghost rows / unresolved links
/// surviving a path-set change).
#[test]
fn path_set_changes_between_spawns_prune_ghosts_and_resolve_links() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("notes/foo.md", "# Foo\n");
    sandbox.write_note("archive/foo.md", "# Old Foo\n");
    sandbox.write_note("a.md", "links to [[foo]]\n");
    sandbox.run(&["index"]).assert_success();

    let baseline = sandbox.run(&["list"]);
    baseline.assert_success();
    baseline.assert_stdout_eq("- a.md\n- archive/foo.md\n- notes/foo.md\n");

    fs::remove_file(sandbox.root().join("archive/foo.md"))
        .expect("delete archive/foo.md");

    let listed = sandbox.run(&["list"]);
    listed.assert_success();
    listed.assert_never_exit_101();
    listed.assert_stdout_eq("- a.md\n- notes/foo.md\n");

    let table =
        sandbox.run(&["table", "--column", "file.name", "--column", "inlinks"]);
    table.assert_success();
    let foo_row = table
        .stdout
        .lines()
        .find(|line| line.starts_with("| foo"))
        .expect("surviving foo row");
    assert!(
        foo_row.contains("a.md"),
        "deleted sibling must re-resolve the wikilink; row: {foo_row}"
    );
}

/// Divergence canary: a same-size content edit with the mtime restored is
/// invisible to a spawned query (metadata fingerprints match, so the store
/// is reloaded) but visible after an explicit `traces index` rebuild —
/// proving the first query genuinely read the store instead of silently
/// rebuilding.
///
/// This is the canonical recipe (portable); it must never be reused to pin
/// the config-staleness behavior (ticket 04 forbids that recipe).
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
    filetime::set_file_mtime(
        &note,
        filetime::FileTime::from_system_time(original_mtime),
    )
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

/// Verifies corruption self-heal at process level: table-level corruption
/// (redb magic preserved) makes the next spawned query exit 0 with correct
/// output, a rebuilt store, and no panic.
///
/// The corruption arrangement preserves the redb file magic and trashes
/// everything after it — the damage class the store recovers from by
/// treating the tables as empty and re-upserting every file (the same
/// contract the integration persistence file owns at service level).
/// Header-level destruction (a wholly garbage file) is a different damage
/// class the store does not recover from; per the ticket-12 ruling that
/// boundary is documented, not pinned.
///
/// Executes vs asserts:
/// - Executes: `traces index`, byte-level table corruption of
///   `.traces/index.redb`, then `traces list`, then a further edit + `traces
///   index` + `traces list` to prove the store accepts writes.
/// - Asserts: exit 0, never 101, exact pre-corruption output, and the post-heal
///   edit query-visible.
///
/// Defect class: panic-on-corruption.
#[test]
fn table_level_corruption_self_heals_on_the_next_query() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "# A\n");
    sandbox.write_note("b.md", "# B\n");
    sandbox.run(&["index"]).assert_success();

    let db = sandbox.root().join(".traces/index.redb");
    let mut bytes = fs::read(&db).expect("read valid index file");
    bytes
        .get_mut(9..)
        .expect("index file longer than the redb magic number")
        .fill(0xFF);
    fs::write(&db, bytes).expect("corrupt index tables");

    let healed = sandbox.run(&["list"]);
    healed.assert_success();
    healed.assert_never_exit_101();
    healed.assert_stdout_eq("- a.md\n- b.md\n");

    // The healed store accepts writes again.
    sandbox.write_note("c.md", "# C\n");
    sandbox.run(&["index"]).assert_success();
    let grown = sandbox.run(&["list"]);
    grown.assert_success();
    grown.assert_stdout_eq("- a.md\n- b.md\n- c.md\n");
}

// Relocated from the dissolved `dispatch.rs` (ticket 19, ledger D12):
// the no-explicit-index persistence claim is the stronger of the pair.
/// Trusts a project, writes a note, and runs `list` with **no** prior
/// `index` call — then checks that `.traces/index.redb` exists on disk.
///
/// Verifies that the `list` dispatch path invokes persistence on its
/// initial cold path, leaving the index file present on disk.
///
/// Subsumes the deleted `trust_then_index_persists_the_file_index`
/// (ticket 16, ledger D12): the no-explicit-index claim is the stronger
/// of the pair.
#[test]
fn list_persists_the_file_index_without_an_explicit_index_command() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "# A\n");

    let list = sandbox.run(&["list"]);

    assert!(list.is_success(), "stderr: {}", list.stderr);
    assert!(sandbox.root().join(".traces/index.redb").is_file());
}
