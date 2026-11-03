//! Step definitions for template selection, rendering, and confined output.

use std::{fs, time::Duration};

use pretty_assertions::assert_eq;
use rstest_bdd_macros::{given, then, when};

use super::{LastRun, Sandbox};

#[given("a trusted project without templates")]
fn trusted_project_without_templates(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted project with a daily template")]
fn daily_template(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
    sandbox.write_template("daily.md", "# {{ \"Today\" }}");
}

#[given(
    "a trusted project with a daily template that asks for different prompt \
     answers"
)]
fn daily_template_with_prompt_defaults(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
    sandbox.write_template(
        "daily.md",
        "{{ ui.text_input(\"name\", \"anon\") }}|{{ ui.text_input(\"note\") \
         }}|{{ ui.confirm(\"proceed\") }}|{{ ui.select(\"tone\", [\"calm\", \
         \"loud\"]) }}|{{ ui.multi_select(\"tags\", [\"daily\"]) | length }}",
    );
}

#[when("I preview the template without a terminal")]
fn preview_without_terminal(sandbox: &Sandbox, last_run: &mut LastRun) {
    last_run.0 = Some(
        sandbox
            .run_with_watchdog(
                &["template", "-i", "daily", "--dry-run"],
                Duration::from_secs(15),
            )
            .expect("non-TTY render finished within the watchdog window"),
    );
}

#[given("a trusted project with a daily template and an existing output file")]
fn daily_template_with_existing_output(sandbox: &Sandbox) {
    daily_template(sandbox);
    sandbox.write_note("daily.md", "keep me");
}

#[given("a trusted project with local and global daily templates")]
fn local_and_global_daily_templates(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
    sandbox.write_template("daily.txt", "local");
    let global = sandbox.config_home().join("traces");
    fs::create_dir_all(global.join("global_templates"))
        .expect("create global template directory");
    fs::write(
        global.join("config.toml"),
        "[templates]\ndirectory = \"global_templates\"\n",
    )
    .expect("write global config");
    fs::write(global.join("global_templates/daily.md"), "global")
        .expect("write global template");
}

#[given("a trusted project with ambiguous daily templates")]
fn ambiguous_daily_templates(sandbox: &Sandbox) {
    daily_template(sandbox);
    sandbox.write_template("daily.txt", "another daily template");
}

#[given("a trusted project with a daily template that declares an output path")]
fn daily_template_with_declared_path(sandbox: &Sandbox) {
    daily_template(sandbox);
    sandbox.write_template(
        "daily.md",
        "{{ file.write_to(\"declared.md\") }}# Declared",
    );
}

#[given(
    "a trusted project with a daily template that declares a path outside the \
     project root"
)]
fn daily_template_with_escaping_path(sandbox: &Sandbox) {
    daily_template(sandbox);
    let outside = format!(
        "{}-escaped.md",
        sandbox
            .root()
            .file_name()
            .expect("project directory name")
            .to_string_lossy(),
    );
    sandbox.write_template(
        "daily.md",
        &format!("{{{{ file.write_to(\"../{outside}\") }}}}# Secret"),
    );
}

#[given("a trusted project with a template that ignores a missing include")]
fn template_with_missing_include(sandbox: &Sandbox) {
    daily_template(sandbox);
    sandbox.write_template(
        "daily.md",
        "before{% include \"missing.md\" ignore missing %}after",
    );
}

#[given("a trusted project with a template that ignores an unsafe include")]
fn template_with_unsafe_include(sandbox: &Sandbox) {
    daily_template(sandbox);
    sandbox.write_template(
        "daily.md",
        "before{% include \"../outside.md\" ignore missing %}after",
    );
}

#[then(r#"the rendered file "{path}" exists with content "{expected}""#)]
fn rendered_file_has_content(
    sandbox: &Sandbox,
    path: String,
    expected: String,
) {
    let file = sandbox.root().join(path);
    assert!(file.is_file(), "expected rendered file {}", file.display());
    assert_eq!(fs::read_to_string(file).expect("read rendered file"), expected);
}

#[then(r#"the existing file "{path}" still contains "{expected}""#)]
fn existing_file_unchanged(sandbox: &Sandbox, path: String, expected: String) {
    let file = sandbox.root().join(path);
    assert_eq!(fs::read_to_string(file).expect("read existing file"), expected);
}

#[then(r#"no output file "{path}" exists"#)]
fn no_output_file(sandbox: &Sandbox, path: String) {
    assert!(!sandbox.root().join(path).exists());
}

#[then("the escaped output file is not created")]
fn escaped_output_file_is_absent(sandbox: &Sandbox) {
    let outside = format!(
        "{}-escaped.md",
        sandbox
            .root()
            .file_name()
            .expect("project directory name")
            .to_string_lossy(),
    );
    assert!(!sandbox.root().join("..").join(outside).exists());
}
