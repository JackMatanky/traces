//! Environment isolation observed through spawned commands and sandbox files.

use std::{ffi::OsStr, fs};

use super::support::{Sandbox, plain};

/// Verifies derived state stores materialize inside the sandbox state
/// accessor (files on disk, not just a cleared environment).
///
/// Executes vs asserts:
/// - Executes: `traces trust` and `traces list` in a trusted sandbox.
/// - Asserts: the trusted-configs and tracked-configs stores exist and are
///   populated under [`Sandbox::state_dir`].
///
/// Defect class: assumed-hermeticity (if `TRACES_STATE_DIR` plumbing breaks,
/// the stores land in the host state home and these assertions fail).
#[test]
fn state_stores_materialize_inside_the_sandbox_state_accessor() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["list"]).assert_success();

    for store in ["trusted-configs", "tracked-configs"] {
        let store_root = sandbox.state_dir().join(store);
        assert!(
            store_root.is_dir(),
            "{store} missing under {}",
            sandbox.state_dir().display()
        );
        let entries =
            fs::read_dir(&store_root).expect("read store directory").count();
        assert!(entries > 0, "{store} exists but is empty");
    }
}

/// Verifies global config discovery resolves from the sandbox config-home
/// accessor, by planting a malformed global config that only the child can
/// see.
///
/// Executes vs asserts:
/// - Executes: `traces trust` (clean setup), then `traces list` with a
///   malformed `traces/config.toml` planted under [`Sandbox::config_home`].
/// - Asserts: the load fails and names the global config file — proving the
///   child read config discovery from the sandbox accessor. If the
///   `XDG_CONFIG_HOME` plumbing broke, the child would see no such file and
///   this test fails.
///
/// Defect class: assumed-hermeticity.
#[test]
fn global_config_resolves_from_the_sandbox_config_home_accessor() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();

    let global_dir = sandbox.config_home().join("traces");
    fs::create_dir_all(&global_dir).expect("create global config dir");
    fs::write(global_dir.join("config.toml"), "unknown_key = true\n")
        .expect("write malformed global config");

    let list = sandbox.run(&["list"]);
    assert!(
        !list.is_success(),
        "malformed global config must fail the load; stdout: {}",
        list.stdout
    );
    // The diagnostic code is printed on its own short line (proven not to
    // wrap), and the unknown-field message is our planted content —
    // together they prove the child parsed the global config from the
    // sandbox accessor. (The absolute path itself wraps mid-component and
    // is deliberately not reconstructed; see support module docs.)
    assert!(
        plain(&list.stderr).contains("config_build_config_file_failed"),
        "failure must be a config-file parse failure; stderr: {}",
        list.stderr
    );
    assert!(
        list.stderr.contains("unknown field `unknown_key`"),
        "failure must come from the planted global config; stderr: {}",
        list.stderr
    );
}

/// Negative control for the state accessor: the store must follow an
/// explicit `TRACES_STATE_DIR` override, not the sandbox default.
///
/// Without this control, the accessor assertions in
/// [`state_stores_materialize_inside_the_sandbox_state_accessor`] could pass
/// vacuously if the state plumbing ever broke; here the same assertions flip
/// with the plumbing.
///
/// Executes vs asserts:
/// - Executes: `traces trust` with `TRACES_STATE_DIR` overridden to an
///   alternate directory via [`Sandbox::run_with_envs`].
/// - Asserts: the trusted-configs store lands under the override and the
///   default accessor stays untouched.
///
/// Defect class: assumed-hermeticity.
#[test]
fn state_store_follows_a_state_dir_override_not_the_default_accessor() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    let override_dir = tempfile::tempdir().expect("create override state dir");

    let trust = sandbox.run_with_envs(&["trust"], &[(
        "TRACES_STATE_DIR",
        override_dir.path().as_os_str(),
    )]);
    trust.assert_success();

    assert!(
        override_dir.path().join("trusted-configs").is_dir(),
        "store must land under the override path"
    );
    assert!(
        !sandbox.state_dir().join("trusted-configs").exists(),
        "default accessor must stay empty when the override applies"
    );
}

/// Verifies sandbox commands ignore ambient variables while explicitly opted-in
/// variables reach the child. The test runs in a separate process with
/// `TRACES_IGNORED_DIRS=secret` set in its parent environment.
///
/// Executes vs asserts:
/// - Executes: `traces index` twice, first under the default policy and then
///   with `TRACES_IGNORED_DIRS=secret` opted in via [`Sandbox::run_with_envs`].
/// - Asserts: the default run indexes both notes; the opt-in run indexes only
///   the non-ignored note.
///
/// Defect class: assumed-hermeticity (ambient ignored dirs leaking to the
/// child).
#[test]
fn opt_in_ignored_dirs_prune_only_when_explicitly_declared() {
    if std::env::var_os("TRACES_E2E_AMBIENT_CHILD").is_none() {
        let child = super::support::run_with_ambient_env(
            "env_policy::opt_in_ignored_dirs_prune_only_when_explicitly_declared",
            "TRACES_IGNORED_DIRS",
            "secret",
        );
        let stdout = String::from_utf8_lossy(&child.stdout);
        assert!(
            child.status.success() && stdout.contains("1 passed"),
            "isolated test failed: stdout: {stdout}, stderr: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        return;
    }
    assert_eq!(
        std::env::var_os("TRACES_IGNORED_DIRS").as_deref(),
        Some(OsStr::new("secret")),
        "isolated child must receive the controlled ambient ignored-dir value"
    );
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_note("open.md", "# open\n");
    sandbox.write_note("secret/hidden.md", "# hidden\n");

    let without = sandbox.run(&["index"]);
    without.assert_success();
    assert!(
        without.stderr.contains("indexed 2 file(s)"),
        "stderr: {}",
        without.stderr
    );

    let with = sandbox.run_with_envs(&["index"], &[(
        "TRACES_IGNORED_DIRS",
        OsStr::new("secret"),
    )]);
    with.assert_success();
    assert!(
        with.stderr.contains("indexed 1 file(s)"),
        "stderr: {}",
        with.stderr
    );
}

/// Checks a naive date-time is interpreted in UTC by the spawned binary even
/// when the test process inherits a non-UTC timezone.
///
/// Executes vs asserts:
/// - Executes: `traces template --dry-run` under a separate parent with
///   `TZ=Pacific/Honolulu`.
/// - Asserts: the rendered timestamp is the UTC instant, not Honolulu time.
///
/// Defect class: assumed-hermeticity (ambient timezone shifting date queries).
#[test]
fn ambient_timezone_does_not_shift_naive_datetime_rendering() {
    if std::env::var_os("TRACES_E2E_AMBIENT_CHILD").is_none() {
        let child = super::support::run_with_ambient_env(
            "env_policy::ambient_timezone_does_not_shift_naive_datetime_rendering",
            "TZ",
            "Pacific/Honolulu",
        );
        let stdout = String::from_utf8_lossy(&child.stdout);
        assert!(
            child.status.success() && stdout.contains("1 passed"),
            "isolated test failed: stdout: {stdout}, stderr: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        return;
    }
    assert_eq!(
        std::env::var_os("TZ").as_deref(),
        Some(OsStr::new("Pacific/Honolulu")),
        "isolated child must receive the controlled ambient timezone"
    );
    let sandbox = Sandbox::trusted_via_cli();
    sandbox
        .write_template("clock.md", "{{ '2024-02-03T12:00:00' | timestamp }}");

    let rendered =
        sandbox.run(&["template", "-i", "clock", "--dry-run", "--no-input"]);
    rendered.assert_success();
    rendered.assert_stdout_eq("1706961600");
}
