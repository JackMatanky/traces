//! Step definitions for source selection, filters, and query output.

use rstest_bdd_macros::{given, when};

use super::{LastRun, Sandbox};

#[given(r#"a note "{path}" containing:"#)]
fn another_note(sandbox: &Sandbox, path: String, docstring: String) {
    sandbox.write_note(&path, &format!("{}\n", docstring.trim()));
}

#[given(r#"a template "{path}" containing:"#)]
fn query_template(sandbox: &Sandbox, path: String, docstring: String) {
    sandbox.write_template(&path, &format!("{}\n", docstring.trim()));
}

#[given(r#"a schema "{name}" containing:"#)]
fn query_schema(sandbox: &Sandbox, name: String, docstring: String) {
    sandbox.write_schema(&name, &format!("{}\n", docstring.trim()));
}

#[given("a trusted project with two notes rated nine and one rated three")]
fn three_rated_notes(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("notes/a.md", "---\nrating: 9\n---\n");
    sandbox.write_note("notes/b.md", "---\nrating: 9\n---\n");
    sandbox.write_note("notes/c.md", "---\nrating: 3\n---\n");
    sandbox.run(&["trust"]).assert_success();
}

#[when("I select pages from:")]
fn select_pages(sandbox: &Sandbox, last_run: &mut LastRun, docstring: String) {
    last_run.0 = Some(sandbox.run(&["list", "--from", docstring.trim()]));
}

#[when("I filter pages where:")]
fn filter_pages(sandbox: &Sandbox, last_run: &mut LastRun, docstring: String) {
    last_run.0 = Some(sandbox.run(&["list", "--where", docstring.trim()]));
}
