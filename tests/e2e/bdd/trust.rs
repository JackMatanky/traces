//! Step definitions for the trust and project lifecycle.

use rstest_bdd_macros::given;

use super::Sandbox;

#[given("a project initialised with init")]
fn a_project_initialised_with_init(sandbox: &Sandbox) {
    let run = sandbox.run(&["init"]);
    run.assert_success();
}

#[given("a trusted project that was then untrusted")]
fn a_trusted_project_that_was_then_untrusted(sandbox: &Sandbox) {
    sandbox.write_config();
    let trust = sandbox.run(&["trust"]);
    trust.assert_success();
    let untrust = sandbox.run(&["untrust"]);
    untrust.assert_success();
}

#[given("an unconfigured project")]
fn an_unconfigured_project(_sandbox: &Sandbox) {}

#[given("a trusted project")]
fn a_trusted_project(sandbox: &Sandbox) {
    sandbox.write_config();
    let trust = sandbox.run(&["trust"]);
    trust.assert_success();
}

#[given(r#"a trusted project with a template "{name}""#)]
fn a_trusted_project_with_template(sandbox: &Sandbox, name: String) {
    sandbox.write_config();
    sandbox.write_template(&name, "template content\n");
    let trust = sandbox.run(&["trust"]);
    trust.assert_success();
}

#[given(r#"a trusted project with a note "{path}" containing:"#)]
fn a_trusted_project_with_note_containing(
    sandbox: &Sandbox,
    path: String,
    docstring: String,
) {
    sandbox.write_config();
    let content = format!("{}\n", docstring.trim_matches('\n'));
    sandbox.write_note(&path, &content);
    let trust = sandbox.run(&["trust"]);
    trust.assert_success();
}
