//! Step definitions for note fields, tags, and tasks observed through queries.

use rstest_bdd_macros::{given, then, when};

use super::{LastRun, Sandbox};
use crate::fixtures::ConfigFixture;

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

#[when(r#"I filter notes by "{filter}""#)]
fn filter_notes(sandbox: &Sandbox, last_run: &mut LastRun, filter: String) {
    last_run.set(sandbox.run(&["list", "--where", &filter]));
}

#[when(r#"I count tasks with "{filter}""#)]
fn count_tasks(sandbox: &Sandbox, last_run: &mut LastRun, filter: String) {
    last_run.set(sandbox.run(&["task", "--where", &filter, "--count"]));
}

#[when(r#"I filter tasks by "{filter}""#)]
fn filter_tasks(sandbox: &Sandbox, last_run: &mut LastRun, filter: String) {
    last_run.set(sandbox.run(&["task", "--where", &filter]));
}

#[when(r#"I list notes with tag "{tag}""#)]
fn list_notes_with_tag(sandbox: &Sandbox, last_run: &mut LastRun, tag: String) {
    last_run.set(sandbox.run(&["list", "--from", &tag]));
}

#[when(r#"I show the field "{field}" for notes"#)]
fn show_field(sandbox: &Sandbox, last_run: &mut LastRun, field: String) {
    last_run.set(sandbox.run(&["table", "--column", &field]));
}

#[then(r#"filtering notes by "{filter}" yields no results"#)]
fn no_notes_match(sandbox: &Sandbox, filter: String) {
    let run = sandbox.run(&["list", "--where", &filter]);
    run.assert_success();
    run.assert_stdout_eq("");
}

#[then(r#"filtering tasks by "{filter}" yields no results"#)]
fn no_tasks_match(sandbox: &Sandbox, filter: String) {
    let run = sandbox.run(&["task", "--where", &filter]);
    run.assert_success();
    run.assert_stdout_eq("");
}
