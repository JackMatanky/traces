//! Step definitions for note fields, tags, and tasks observed through queries.

use rstest_bdd_macros::{given, when};

use super::{LastRun, Sandbox};
use crate::config_fixture::ConfigFixture;

#[given(r#"another note "{path}" containing:"#)]
fn another_note_containing(sandbox: &Sandbox, path: String, docstring: String) {
    sandbox.write_note(&path, &format!("{}\n", docstring.trim_matches('\n')));
}

#[given(r#"a trusted project with task tag filter "{tag}" and a note "{path}" containing:"#)]
fn project_with_task_tag_filter(
    sandbox: &Sandbox,
    tag: String,
    path: String,
    docstring: String,
) {
    sandbox.write_config_fixture(
        &ConfigFixture::minimal().with_task_tag_filters(&[&tag]),
    );
    sandbox.write_note(&path, &format!("{}\n", docstring.trim_matches('\n')));
    sandbox.run(&["trust"]).assert_success();
}

#[given(r#"a template "{name}" containing:"#)]
fn a_template_containing(sandbox: &Sandbox, name: String, docstring: String) {
    sandbox.write_template(&name, docstring.trim_matches('\n'));
}

#[when(r#"I filter notes by "{filter}""#)]
fn filter_notes(sandbox: &Sandbox, last_run: &mut LastRun, filter: String) {
    last_run.0 = Some(sandbox.run(&["list", "--where", &filter]));
}

#[when(r#"I count tasks with "{filter}""#)]
fn count_tasks(sandbox: &Sandbox, last_run: &mut LastRun, filter: String) {
    last_run.0 = Some(sandbox.run(&["task", "--where", &filter, "--count"]));
}

#[when(r#"I filter tasks by "{filter}""#)]
fn filter_tasks(sandbox: &Sandbox, last_run: &mut LastRun, filter: String) {
    last_run.0 = Some(sandbox.run(&["task", "--where", &filter]));
}

#[when(r#"I list notes with tag "{tag}""#)]
fn list_notes_with_tag(sandbox: &Sandbox, last_run: &mut LastRun, tag: String) {
    last_run.0 = Some(sandbox.run(&["list", "--from", &tag]));
}

#[when(r#"I show the field "{field}" for notes"#)]
fn show_field(sandbox: &Sandbox, last_run: &mut LastRun, field: String) {
    last_run.0 = Some(sandbox.run(&["table", "--column", &field]));
}
