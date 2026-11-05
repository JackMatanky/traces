//! Platform state-directory fallback chain, observed through spawned
//! processes.
//!
//! The sandbox owns `TRACES_STATE_DIR`, so the default policy never exercises
//! the fallbacks (`XDG_STATE_HOME`, `HOME`, `LOCALAPPDATA`). Each test here
//! removes `TRACES_STATE_DIR` from the child with
//! [`Sandbox::run_with_envs_removing`] while `HOME` (`USERPROFILE` and
//! `LOCALAPPDATA` on Windows) stays pointed at the sandbox, so nothing can
//! reach the real home directory.

use std::path::Path;

use super::support::Sandbox;

/// Asserts `traces trust` created the trust store directly under
/// `state_home/traces` and nowhere under the sandbox state directory.
fn assert_store_lands_under(state_home: &Path, sandbox: &Sandbox) {
    let store = state_home.join("traces").join("trusted-configs");
    assert!(store.is_dir(), "trust store must land at {}", store.display());
    assert!(
        !sandbox.state_dir().join("traces").exists()
            && !sandbox.state_dir().join("trusted-configs").exists(),
        "no store may land under the sandbox default"
    );
}

/// `XDG_STATE_HOME` places the store at `$XDG_STATE_HOME/traces` when
/// `TRACES_STATE_DIR` is absent.
///
/// Executes vs asserts:
/// - Executes: `traces trust` with `TRACES_STATE_DIR` removed and
///   `XDG_STATE_HOME` set to a fresh directory.
/// - Asserts: exit 0 and the trusted-configs store under
///   `$XDG_STATE_HOME/traces`.
///
/// Defect class: assumed-hermeticity.
#[cfg(not(windows))]
#[test]
fn places_store_under_xdg_state_home_without_state_dir_override() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    let xdg = tempfile::tempdir().expect("create xdg state home");

    let trust = sandbox.run_with_envs_removing(
        &["trust"],
        &[("XDG_STATE_HOME", xdg.path().as_os_str())],
        &["TRACES_STATE_DIR"],
    );

    trust.assert_success();
    assert_store_lands_under(xdg.path(), &sandbox);
}

/// `HOME` alone places the store at the platform default below `$HOME`:
/// `.local/state` on Linux, `Library/Application Support` on macOS.
///
/// Executes vs asserts:
/// - Executes: `traces trust` with `TRACES_STATE_DIR` removed and no
///   `XDG_STATE_HOME`; the sandbox `HOME` is a fresh directory.
/// - Asserts: exit 0 and the trusted-configs store under the platform default
///   below that `HOME`.
///
/// Defect class: assumed-hermeticity.
#[cfg(unix)]
#[test]
fn places_store_under_home_default_without_any_state_override() {
    let sandbox = Sandbox::new();
    sandbox.write_config();

    let trust = sandbox.run_with_envs_removing(&["trust"], &[], &[
        "TRACES_STATE_DIR",
        "XDG_STATE_HOME",
    ]);

    trust.assert_success();
    let default_state = if cfg!(target_os = "macos") {
        sandbox.state_dir().join("Library").join("Application Support")
    } else {
        sandbox.state_dir().join(".local").join("state")
    };
    let store = default_state.join("traces").join("trusted-configs");
    assert!(store.is_dir(), "trust store must land at {}", store.display());
}

/// `LOCALAPPDATA` places the store at `%LOCALAPPDATA%\traces` when
/// `TRACES_STATE_DIR` is absent.
#[cfg(windows)]
#[test]
fn places_store_under_local_app_data_without_state_dir_override() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    let local = tempfile::tempdir().expect("create local app data");

    let trust = sandbox.run_with_envs_removing(
        &["trust"],
        &[("LOCALAPPDATA", local.path().as_os_str())],
        &["TRACES_STATE_DIR"],
    );

    trust.assert_success();
    assert_store_lands_under(local.path(), &sandbox);
}

/// `USERPROFILE` places the store at `%USERPROFILE%\AppData\Local\traces`
/// when both `TRACES_STATE_DIR` and `LOCALAPPDATA` are absent.
#[cfg(windows)]
#[test]
fn places_store_under_user_profile_default_without_any_state_override() {
    let sandbox = Sandbox::new();
    sandbox.write_config();

    let trust = sandbox.run_with_envs_removing(&["trust"], &[], &[
        "TRACES_STATE_DIR",
        "LOCALAPPDATA",
    ]);

    trust.assert_success();
    let store = sandbox
        .state_dir()
        .join("AppData")
        .join("Local")
        .join("traces")
        .join("trusted-configs");
    assert!(store.is_dir(), "trust store must land at {}", store.display());
}
