//! Step definitions for configuration tracking through the spawned CLI.

use std::fs;

use rstest_bdd_macros::{given, then};

use super::{LastRun, Sandbox};

#[given("a project whose configuration tracking cannot be saved")]
fn project_with_unwritable_tracking(sandbox: &Sandbox) {
    sandbox.write_config();
    fs::write(sandbox.state_dir().join("tracked-configs"), "")
        .expect("block tracked config store creation");
}

#[then("the project remains untrusted")]
fn project_remains_untrusted(sandbox: &Sandbox) {
    let trusted = sandbox.run(&["trust", "list"]);
    trusted.assert_success();
    trusted.assert_stdout_eq("");
}

#[given("a trusted project whose configuration was indexed")]
fn trusted_project_with_indexed_configuration(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();
}

#[given("a trusted project whose indexed configuration was deleted")]
fn trusted_project_with_deleted_configuration(sandbox: &Sandbox) {
    trusted_project_with_indexed_configuration(sandbox);
    fs::remove_file(sandbox.root().join(".traces/config.toml"))
        .expect("delete tracked configuration");
}

#[then("stdout is the project configuration path")]
fn tracked_config_path_is_stdout(sandbox: &Sandbox, last_run: &LastRun) {
    let run = last_run.0.as_ref().expect("a prior command must have run");
    let path = sandbox
        .root()
        .join(".traces/config.toml")
        .canonicalize()
        .expect("canonical tracked config path");
    run.assert_stdout_eq(&format!("{}\n", path.display()));
}

#[then("no tracked configuration paths remain")]
fn tracked_config_paths_are_empty(sandbox: &Sandbox) {
    let list = sandbox.run(&["tracked", "list"]);
    list.assert_success();
    list.assert_stdout_eq("");
}

#[then("the project configuration path remains tracked")]
fn tracked_config_path_remains(sandbox: &Sandbox) {
    let list = sandbox.run(&["tracked", "list"]);
    list.assert_success();
    let path = sandbox
        .root()
        .join(".traces/config.toml")
        .canonicalize()
        .expect("canonical tracked config path");
    list.assert_stdout_eq(&format!("{}\n", path.display()));
}
