//! Query capability: class-expansion source selectors through the spawned
//! CLI, and inlinks rendered from the persisted store.

use super::support::Sandbox;

/// Paired CLI leg for the template composition-root class test (see
/// `tests/integration/template_render.rs`): `@class*` descendant expansion
/// resolves through the schema graph in a spawned `traces list`, and an
/// unknown class degrades to a literal match (no hierarchy, no error).
///
/// Executes vs asserts:
/// - Executes: schemas under `.traces/schemas/`, classed notes, `traces trust`,
///   `traces index`, then `traces list --from` with exact, descendant, and
///   unknown class selectors.
/// - Asserts: `@book*` lists exactly the `sci_fi` and `novel` classed notes
///   (transitive extends), `@book` matches nothing, `@horror` degrades to a
///   clean empty result (no note carries that class) — no error, no panic.
///
/// Defect class: reachability-transfer.
#[test]
fn class_descendant_selector_expands_through_the_schema_graph() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    sandbox.write_schema("book", "");
    sandbox.write_schema("sci_fi", "extends = [\"book\"]");
    sandbox.write_schema("novel", "extends = [\"sci_fi\"]");
    sandbox.write_note("shelf/a.md", "---\nclass: sci_fi\n---\n# A\n");
    sandbox.write_note("shelf/b.md", "---\nclass: novel\n---\n# B\n");
    sandbox.write_note("shelf/c.md", "# C\n");

    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();

    let descendants = sandbox.run(&["list", "--from", "@book*"]);
    descendants.assert_success();
    descendants.assert_never_exit_101();
    descendants.assert_stdout_eq("- shelf/a.md\n- shelf/b.md\n");

    let exact = sandbox.run(&["list", "--from", "@book"]);
    exact.assert_success();
    exact.assert_stdout_eq("");

    let unknown = sandbox.run(&["list", "--from", "@horror"]);
    unknown.assert_success();
    unknown.assert_never_exit_101();
    unknown.assert_stdout_eq("");
}

/// Verifies inlinks render from the persisted store through a spawned query:
/// each CLI invocation is a cold process, so the rendered `inlinks` column
/// proves the cold link read persisted and survived the store round-trip.
///
/// This is the cold-path inlinks residue of the parity differential (the
/// integration comparison in `tests/integration/index_query.rs` cannot reach
/// `QueryRow`'s crate-internal inlinks accessor; a plain in-crate
/// `list("inlinks")` test would near-duplicate the existing in-crate
/// coverage).
///
/// Executes vs asserts:
/// - Executes: linked notes, `traces trust`, `traces index`, then `traces table
///   --column file.name --column inlinks`.
/// - Asserts: the target row's `inlinks` cell carries the linker path.
///
/// Defect class: silent-wrong-answers (cold-path link rows lost or
/// misattributed across the store round-trip).
#[test]
fn inlinks_render_from_the_persisted_store() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("notes/foo.md", "# Foo\n");
    sandbox.write_note("a.md", "see [[notes/foo]]\n");
    sandbox.run(&["index"]).assert_success();

    let table =
        sandbox.run(&["table", "--column", "file.name", "--column", "inlinks"]);
    table.assert_success();
    table.assert_never_exit_101();
    let foo_row = table
        .stdout
        .lines()
        .find(|line| line.starts_with("| foo"))
        .expect("foo row present");
    assert!(
        foo_row.contains("a.md"),
        "inlinks must render the linker; row: {foo_row}"
    );
}
