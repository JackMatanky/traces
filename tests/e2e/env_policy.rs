//! Environment-policy contracts: the spawned child environment is
//! default-deny with an explicit ownership table, and derived state/config
//! artifacts provably land inside the sandbox's accessor paths.
//!
//! `env_clear()` alone proves nothing; these tests assert real policy
//! structure and real files in real accessor paths.

use std::{ffi::OsStr, fs};

use pretty_assertions::assert_eq;

use super::support::{Sandbox, plain};

/// Verifies the spawned child receives exactly the declared policy table —
/// no more, no less.
///
/// Executes vs asserts:
/// - Executes: the sandbox command builder for `traces --version` (no spawn
///   needed; the policy is inspectable on the built `Command`).
/// - Asserts: the applied child environment equals
///   [`Sandbox::env_policy_entries`] entry for entry, and no opt-in variable
///   (`TRACES_CEILING_DIRS`, `TRACES_IGNORED_DIRS`) is pre-declared.
///
/// Defect class: assumed-hermeticity (a builder regression that passes host
/// variables through "just in case" would surface here).
#[test]
fn spawned_child_receives_exactly_the_declared_policy_table() {
    let sandbox = Sandbox::new();

    let mut declared = sandbox.env_policy_entries();
    declared.sort();

    let mut applied: Vec<(String, std::ffi::OsString)> = sandbox
        .command(&["--version"])
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.unwrap_or_default().to_owned(),
            )
        })
        .collect();
    applied.sort();

    assert_eq!(
        applied, declared,
        "child environment must equal the declared policy table"
    );
    for opt_in in ["TRACES_CEILING_DIRS", "TRACES_IGNORED_DIRS"] {
        assert!(
            !declared.iter().any(|(key, _)| key == opt_in),
            "{opt_in} must be opt-in, never pre-declared"
        );
    }
}

/// Verifies the policy's sandbox-owned entries carry the sandbox accessor
/// values, never host values.
///
/// Executes vs asserts:
/// - Executes: the policy table construction for a fresh sandbox.
/// - Asserts: `TRACES_STATE_DIR` equals the state accessor, `XDG_CONFIG_HOME`
///   equals the config-home accessor, and `TZ` is pinned to `UTC`.
///
/// Defect class: assumed-hermeticity (a table entry silently re-anchored to
/// a host path would surface here).
#[test]
fn policy_owns_state_config_and_timezone_entries() {
    let sandbox = Sandbox::new();
    let entries = sandbox.env_policy_entries();
    let get = |key: &str| {
        entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone())
    };

    assert_eq!(
        get("TRACES_STATE_DIR").as_deref(),
        Some(sandbox.state_dir().as_os_str()),
        "state store root must be the sandbox accessor"
    );
    assert_eq!(
        get("XDG_CONFIG_HOME").as_deref(),
        Some(sandbox.config_home().as_os_str()),
        "config home must be the sandbox accessor"
    );
    assert_eq!(
        get("TZ").as_deref(),
        Some(OsStr::new("UTC")),
        "child timezone must be pinned to UTC"
    );
}

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

/// Verifies opt-in environment variables reach the child only when the test
/// declares them explicitly.
///
/// Executes vs asserts:
/// - Executes: `traces index` twice — once under the default policy, once with
///   `TRACES_IGNORED_DIRS=secret` opted in via [`Sandbox::run_with_envs`].
/// - Asserts: the default run indexes both notes; the opt-in run indexes only
///   the non-ignored note.
///
/// Defect class: assumed-hermeticity (an opt-in variable leaking into the
/// default table would flip the first assertion).
#[test]
fn opt_in_ignored_dirs_prune_only_when_explicitly_declared() {
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
