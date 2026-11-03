//! Steps for observable index freshness across spawned commands.

use std::fs;

use pretty_assertions::assert_eq;
use rstest_bdd_macros::{given, then};

use super::{LastRun, Sandbox};
use crate::config_fixture::ConfigFixture;

#[given("an indexed project with the original task")]
fn indexed_original_task(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("a.md", "# A\n\n- [ ] old task\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
    let first = sandbox.run(&["task"]);
    first.assert_success();
    first.assert_stdout_eq("- [ ] old task (a.md)\n");
    // The size changes so this test does not depend on the file system's mtime
    // resolution.
    sandbox.write_note("a.md", "# A\n\n- [ ] rewritten task text\n");
}

#[given("an indexed project with a changed tag")]
fn indexed_changed_tag(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("a.md", "# A\n\n#removed\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
    sandbox.run(&["list", "--from", "#removed"]).assert_stdout_eq("- a.md\n");
    sandbox.write_note("a.md", "# A\n\n#added-and-new\n#added\n");
}

fn indexed_link_targets(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("notes/foo.md", "# Foo\n");
    sandbox.write_note("archive/foo.md", "# Old Foo\n");
    sandbox.write_note("a.md", "links to [[foo]]\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
    sandbox
        .run(&["list"])
        .assert_stdout_eq("- a.md\n- archive/foo.md\n- notes/foo.md\n");
}

#[given("an indexed project with one of two link targets deleted")]
fn one_link_target_deleted(sandbox: &Sandbox) {
    indexed_link_targets(sandbox);
    fs::remove_file(sandbox.root().join("archive/foo.md"))
        .expect("remove ambiguous target");
}

#[given("an indexed project with a link target renamed")]
fn link_target_renamed(sandbox: &Sandbox) {
    indexed_link_targets(sandbox);
    fs::remove_file(sandbox.root().join("archive/foo.md"))
        .expect("remove ambiguous target");
    sandbox.run(&["list"]).assert_stdout_eq("- a.md\n- notes/foo.md\n");
    fs::rename(
        sandbox.root().join("notes/foo.md"),
        sandbox.root().join("archive/foo.md"),
    )
    .expect("rename target");
}

#[given("an indexed project with a newly added link target")]
fn link_target_added(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("a.md", "links to [[new]]\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
    sandbox.write_note("new.md", "# New\n");
}

#[then(r#"the row for "{name}" contains "{source}""#)]
fn row_contains_inlink(last_run: &LastRun, name: String, source: String) {
    let run = last_run.0.as_ref().expect("prior command");
    let row = run
        .stdout
        .lines()
        .find(|line| line.starts_with(&format!("| {name}")))
        .expect("target row must exist");
    assert!(row.contains(&source), "expected {source} in row: {row}");
}

#[then("the project has an index store")]
fn project_has_index_store(sandbox: &Sandbox) {
    assert!(sandbox.root().join(".traces/index.redb").is_file());
}

#[given("an indexed project with its class field changed")]
fn class_field_changed(sandbox: &Sandbox) {
    sandbox.write_config_fixture(&ConfigFixture::minimal());
    sandbox.write_note("a.md", "---\nclass: Book\nkind: Novel\n---\n# A\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
    sandbox.run(&["list", "--from", "@Book"]).assert_stdout_eq("- a.md\n");
    sandbox.write_config_fixture(
        &ConfigFixture::minimal().with_schemas(".traces/schemas", Some("kind")),
    );
    sandbox.run(&["trust"]).assert_success();
}

#[given("an indexed project with a link to foo")]
fn indexed_link_to_foo(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("notes/foo.md", "# Foo\n");
    sandbox.write_note("a.md", "see [[notes/foo]]\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
}

#[given("an indexed project with repeated and empty link sources")]
fn repeated_and_empty_link_sources(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("target.md", "# Target\n");
    sandbox.write_note("a.md", "[[target]] and [[target]]\n");
    sandbox.write_note("z.md", "see [[target]]\n");
    sandbox.write_note("empty.md", "# Empty\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
}

#[given("an indexed project with exact implied and nearby links")]
fn exact_implied_and_nearby_links(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("notes/foo.md", "# Near\n");
    sandbox.write_note("remote/foo.md", "# Far\n");
    sandbox.write_note("notes/a.md", "see [[foo]]\n");
    sandbox.write_note("remote/a.md", "see [[notes/foo]]\n");
    sandbox.write_note("root.md", "see [[remote/foo.md]]\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
}

#[then(r#"the inlinks for "{path}" are "{expected}""#)]
fn inlinks_are(last_run: &LastRun, path: String, expected: String) {
    let run = last_run.0.as_ref().expect("prior command");
    let row = run
        .stdout
        .lines()
        .find(|line| {
            line.split('|').nth(1).is_some_and(|cell| cell.trim() == path)
        })
        .expect("target path row must exist");
    assert_eq!(row.split('|').nth(2).expect("inlinks cell").trim(), expected);
}

#[then(r#"the inlinks for "{path}" are empty"#)]
fn inlinks_empty(last_run: &LastRun, path: String) {
    let run = last_run.0.as_ref().expect("prior command");
    let row = run
        .stdout
        .lines()
        .find(|line| {
            line.split('|').nth(1).is_some_and(|cell| cell.trim() == path)
        })
        .expect("target path row must exist");
    assert_eq!(row.split('|').nth(2).expect("inlinks cell").trim(), "");
}

#[given("a trusted project with a rendered tagged note")]
fn rendered_tagged_note(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_template(
        "daily.md",
        "# Rendered\n\n- [ ] from template #from{{ \"template\" }}",
    );
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["template", "-i", "daily", "--no-input"]).assert_success();
}

#[given("an indexed project whose index store was removed")]
fn store_removed(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("a.md", "# A\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
    sandbox.run(&["list"]).assert_stdout_eq("- a.md\n");
    fs::remove_file(sandbox.root().join(".traces/index.redb"))
        .expect("remove index store");
}

#[given("an indexed project with a persisted list result")]
fn persisted_list_result(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("a.md", "# A\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
    sandbox.run(&["list"]).assert_stdout_eq("- a.md\n");
}

#[given("an indexed project with bare and prefixed frontmatter tags")]
fn indexed_frontmatter_tags(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("a.md", "---\ntags: [project]\n---\n# A\n");
    sandbox.write_note("b.md", "---\ntags: [\"#project\"]\n---\n# B\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
}

#[given("a trusted project that was indexed")]
fn trusted_project_indexed(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
}

#[then("stdout contains the project configuration path")]
fn tracked_config_path(sandbox: &Sandbox, last_run: &LastRun) {
    let run = last_run.0.as_ref().expect("prior command");
    let config_path = sandbox.root().join(".traces/config.toml");
    assert!(
        run.stdout.contains(config_path.to_string_lossy().as_ref()),
        "stdout: {}",
        run.stdout
    );
}

#[given("an indexed project with hidden Git files and a symlink note")]
fn hidden_git_and_link(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("a.md", "# A\n");
    sandbox.write_note(".git/hidden.md", "# Hidden\n");
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        sandbox.root().join("a.md"),
        sandbox.root().join("linked.md"),
    )
    .expect("create symlink note");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(
        sandbox.root().join("a.md"),
        sandbox.root().join("linked.md"),
    )
    .expect("create symlink note");
    assert_eq!(sandbox.run(&["list"]).stdout, "- a.md\n");
}
