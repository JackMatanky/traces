//! Index capability: refresh deltas observed through spawned processes —
//! content edits, path-set changes, store-read divergence, and corruption
//! self-heal.

use std::fs;

use super::{config_fixture::ConfigFixture, support::Sandbox};

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

/// Verifies path-set refresh between spawns after a deletion and a rename.
///
/// `[[foo]]` is ambiguous while `notes/foo.md` and `archive/foo.md` both
/// exist, so it resolves to nothing; deleting `archive/foo.md` must make
/// the same link resolve to `notes/foo.md` on the next spawned query —
/// without an explicit re-index in between.
///
/// Executes vs asserts:
/// - Executes: `traces index`, then `traces list` before and after a deletion
///   and a rename, and `traces table` after each path-set change.
/// - Asserts: old paths disappear and the current target's `inlinks` column
///   shows the previously-ambiguous linker without another explicit index.
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
    fs::rename(
        sandbox.root().join("notes/foo.md"),
        sandbox.root().join("archive/foo.md"),
    )
    .expect("rename surviving foo");
    let renamed = sandbox.run(&["list"]);
    renamed.assert_success();
    renamed.assert_never_exit_101();
    renamed.assert_stdout_eq("- a.md\n- archive/foo.md\n");

    let renamed_table =
        sandbox.run(&["table", "--column", "file.name", "--column", "inlinks"]);
    renamed_table.assert_success();
    let renamed_row = renamed_table
        .stdout
        .lines()
        .find(|line| line.starts_with("| foo"))
        .expect("renamed foo row");
    assert!(
        renamed_row.contains("a.md"),
        "renamed target must retain the resolved wikilink; row: {renamed_row}"
    );
}

/// A target added after indexing must acquire inlinks on the next query.
#[test]
fn newly_created_link_target_resolves_inlinks_after_spawned_refresh() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "links to [[new]]\n");
    sandbox.run(&["index"]).assert_success();
    sandbox.write_note("new.md", "# New\n");

    let table =
        sandbox.run(&["table", "--column", "file.name", "--column", "inlinks"]);
    table.assert_success();
    let new_row = table
        .stdout
        .lines()
        .find(|line| line.starts_with("| new"))
        .expect("new target row");
    assert!(
        new_row.contains("a.md"),
        "new target must have its inlink: {new_row}"
    );
}

/// Divergence canary: a same-size content edit with the mtime restored is
/// invisible to a spawned query (metadata fingerprints match, so the store
/// is reloaded) but visible after an explicit `traces index` rebuild —
/// proving the first query genuinely read the store instead of silently
/// rebuilding.
///
/// This recipe stays about metadata freshness; config drift has its own canary,
/// [`config_edit_heals_the_index_on_the_next_query`].
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

/// Trusts a project, writes a note, and runs `list` with **no** prior
/// `index` call, then checks both the result and the on-disk store.
///
/// Verifies the initial cold `list` path builds an index for the note and
/// creates `.traces/index.redb`.
#[test]
fn list_creates_an_index_store_without_an_explicit_index_command() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("a.md", "# A\n");

    let list = sandbox.run(&["list"]);

    assert!(list.is_success(), "stderr: {}", list.stderr);
    list.assert_stdout_eq("- a.md\n");
    assert!(sandbox.root().join(".traces/index.redb").is_file());
}

/// Verifies an index-relevant config edit heals the index on the next spawned
/// query, with no `traces index` in between.
///
/// Executes vs asserts:
/// - Executes: `traces trust`, `traces index` and class queries under
///   `class_field = "class"`, then an edit to `class_field = "kind"`, a
///   re-trust, and the same class queries again.
/// - Asserts: before the edit `@Book` matches and `@Novel` does not; after it
///   `@Novel` matches and `@Book` does not, though no note file changed.
///
/// Defect class: stale-secondary-index (config drift).
#[test]
fn config_edit_heals_the_index_on_the_next_query() {
    let sandbox = Sandbox::new();
    sandbox.write_config_fixture(
        &ConfigFixture::new().with_templates("templates"),
    );
    sandbox.write_note("a.md", "---\nclass: Book\nkind: Novel\n---\n# A\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();

    let book_before = sandbox.run(&["list", "--from", "@Book"]);
    book_before.assert_success();
    book_before.assert_stdout_eq("- a.md\n");

    sandbox.write_config_fixture(
        &ConfigFixture::new()
            .with_templates("templates")
            .with_schemas(".traces/schemas", Some("kind")),
    );
    sandbox.run(&["trust"]).assert_success();

    let novel = sandbox.run(&["list", "--from", "@Novel"]);
    novel.assert_success();
    novel.assert_stdout_eq("- a.md\n");
    let book_after = sandbox.run(&["list", "--from", "@Book"]);
    book_after.assert_success();
    book_after.assert_stdout_eq("");
}
