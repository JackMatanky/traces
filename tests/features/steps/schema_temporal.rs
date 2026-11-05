//! Project fixtures for schema selection and temporal behavior.

use pretty_assertions::assert_eq;
use rstest_bdd_macros::{given, then};

use super::Sandbox;
use crate::fixtures::ConfigFixture;

#[given("a trusted library with book, science fiction, and novel classes")]
fn book_class_library(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_schema("book", "");
    sandbox.write_schema("sci_fi", "extends = [\"book\"]");
    sandbox.write_schema("novel", "extends = [\"sci_fi\"]");
    sandbox.write_note("shelf/book.md", "---\nclass: book\n---\n");
    sandbox.write_note("shelf/novel.md", "---\nclass: novel\n---\n");
    sandbox.write_note("shelf/other.md", "# Other\n");
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted library with kind as its file class field")]
fn library_with_kind_field(sandbox: &Sandbox) {
    sandbox.write_config_fixture(
        &ConfigFixture::new()
            .with_templates("templates")
            .with_schemas(".traces/schemas", Some("kind")),
    );
    sandbox.write_note("novel.md", "---\nkind: Novel\n---\n");
    // The unclassed note must stay out of the @Novel match.
    sandbox.write_note("plain.md", "# Plain\n");
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted project with a file-sourced category field")]
fn project_with_categories(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_schema_value(
        "categories.toml",
        "[[entries]]\nid = \"rust\"\ntitle = \"Rust \
         Programming\"\n\n[[entries]]\nid = \"python\"\ntitle = \"Python \
         Programming\"\n",
    );
    sandbox.write_schema(
        "topic",
        "[fields.category]\ntype = \"select\"\nvalues = { path = \
         \"values/categories.toml\", value = \"id\", label = \"title\" }\n",
    );
    sandbox.write_template(
        "topic_note.md",
        "Category: {{ schema.get('topic').field('category')[0].label }} ({{ \
         schema.get('topic').field('category')[0].value }}); {{ \
         schema.get('topic').field('category')[1].label }} ({{ \
         schema.get('topic').field('category')[1].value }})",
    );
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted project with two offset-dated notes one nanosecond apart")]
fn offset_dated_notes(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note(
        "matching.md",
        "---\ndue: 2026-07-29T14:30:00.123456789+02:00\n---\n",
    );
    sandbox.write_note(
        "different.md",
        "---\ndue: 2026-07-29T14:30:00.123456788+02:00\n---\n",
    );
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted project with a date, a different date, and an undated note")]
fn date_notes(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("due.md", "---\ndue: 2026-01-01\n---\n");
    sandbox.write_note("later.md", "---\ndue: 2026-01-02\n---\n");
    sandbox.write_note("undated.md", "# Untimed\n");
    sandbox.run(&["trust"]).assert_success();
}

#[given(r#"a trusted project with a template "{name}" containing:"#)]
fn project_with_template(sandbox: &Sandbox, name: String, docstring: String) {
    sandbox.write_config();
    sandbox.write_template(&name, docstring.trim());
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted project with two month-end notes and a calendar template")]
fn month_end_project(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("notes/jan.md", "---\nwhen: 2026-01-31\n---\n");
    sandbox.write_note("notes/feb.md", "---\nwhen: 2026-02-15\n---\n");
    sandbox.write_template(
        "calendar.md",
        r#"{{ query.from("notes/").where('when + dur("1 month") == "2026-02-28"') | length }}|{{ "2026-01-31" | date_add(1, unit="months") }}"#,
    );
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted library with a cycle in its file classes")]
fn cyclic_classes(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_schema("book", "extends = [\"novel\"]");
    sandbox.write_schema("novel", "extends = [\"book\"]");
    sandbox.write_note("book.md", "---\nclass: book\n---\n");
    sandbox.run(&["trust"]).assert_success();
}

#[given(
    "a trusted library with one invalid file class and one valid file class"
)]
fn partially_invalid_classes(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_schema(
        "broken",
        "[fields.status]\ntype = \"input\"\n[fields.Status]\ntype = \"input\"",
    );
    sandbox.write_schema(
        "book",
        "[fields.status]\ntype = \"select\"\nvalues = [\"draft\"]",
    );
    sandbox.write_note("book.md", "---\nclass: book\n---\n");
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted library with a template that names an unknown schema")]
fn missing_schema_template(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_template("missing.md", "{{ schema.get('missing') }}");
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted library with a template that names an unknown schema field")]
fn missing_schema_field_template(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_schema("book", "[fields.status]\ntype = \"input\"");
    sandbox.write_note("book.md", "---\nclass: book\n---\n");
    sandbox.write_template(
        "missing_field.md",
        "{{ schema.get('book').field('missing_field') }}",
    );
    sandbox.run(&["trust"]).assert_success();
}

#[given(
    "a trusted library with a template that queries an unknown schema class"
)]
fn unknown_schema_class_template(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note("horror.md", "---\nclass: horror\n---\n");
    // An unclassed note must stay out of the literal fallback match.
    sandbox.write_note("plain.md", "# Plain\n");
    sandbox.write_template(
        "unknown_class.md",
        "{{ query.from(\"@horror\") | length }}",
    );
    sandbox.run(&["trust"]).assert_success();
}

#[given(
    "a trusted project with a calendar comparison across the spring clock \
     change"
)]
fn spring_calendar_comparison(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note(
        "notes/spring.md",
        "---\nstart: 2026-03-07T12:00:00-05:00\nend: \
         2026-03-08T12:00:00-04:00\n---\n",
    );
    sandbox.write_template(
        "spring.md",
        r#"{{ query.from("notes/").where('dur("1 day") == dur("24 hours")') | length }}|{{ query.from("notes/").where('date_add(start,1,"days") == end') | length }}|{{ query.from("notes/").where('date_add(start,24,"hours") == end') | length }}"#,
    );
    sandbox.write_template(
        "boundaries.md",
        r#"{{ query.from("notes/").where('date_diff(start,end,"days") == 1') | length }}|{{ query.from("notes/").where('date_diff(start,end,"hours") == 23') | length }}"#,
    );
    sandbox.run(&["trust"]).assert_success();
}

#[then(r#"file "{path}" is:"#)]
fn file_is(sandbox: &Sandbox, path: String, docstring: String) {
    let actual = std::fs::read_to_string(sandbox.root().join(path))
        .expect("rendered file exists");
    assert_eq!(actual, docstring.trim());
}
