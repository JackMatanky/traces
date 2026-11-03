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
