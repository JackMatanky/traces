//! Config capability: disk-form TOML driving spawned behavior, the four
//! process-level failure stages, and the global-config leg via child env.

use std::fs;

use super::{
    common::ConfigFixture,
    support::{Sandbox, plain},
};

/// Verifies `[tasks] tag_filters` from the on-disk TOML drives task
/// selection in a spawned process.
///
/// Executes vs asserts:
/// - Executes: `traces trust`, `traces index`, `traces task` — the child parses
///   the config file from disk.
/// - Asserts: only the `#todo` item is a Task; `#other` is not (under no filter
///   both status-marked items would classify as Tasks).
///
/// Defect class: disk-config-untested (all prior config coverage was
/// fixture-computed in-process).
#[test]
fn tag_filter_config_drives_task_selection_from_disk() {
    let sandbox = Sandbox::new();
    sandbox.write_config_fixture(
        &ConfigFixture::new()
            .with_templates("templates")
            .with_task_tag_filters(&["todo"]),
    );
    sandbox.write_note("a.md", "# A\n\n- [ ] one #todo\n- [ ] two #other\n");

    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();

    let task = sandbox.run(&["task"]);
    task.assert_success();
    task.assert_never_exit_101();
    task.assert_stdout_eq("- [ ] one (a.md)\n");
}

/// Verifies `[schemas] class_field` from the on-disk TOML drives class
/// selection in a spawned process.
///
/// Executes vs asserts:
/// - Executes: `traces trust`, `traces index`, then `list --from` class queries
///   — the child indexes and matches under the configured field.
/// - Asserts: the `kind: Novel` value matches `@Novel` (and the default
///   `class`-field value would not exist here).
///
/// Defect class: disk-config-untested.
#[test]
fn class_field_config_drives_class_selection_from_disk() {
    let sandbox = Sandbox::new();
    sandbox.write_config_fixture(
        &ConfigFixture::new()
            .with_templates("templates")
            .with_schemas(".traces/schemas", Some("kind")),
    );
    sandbox.write_note("a.md", "---\nkind: Novel\n---\n# A\n");

    sandbox.run(&["trust"]).assert_success();
    sandbox.run(&["index"]).assert_success();

    let novel = sandbox.run(&["list", "--from", "@Novel"]);
    novel.assert_success();
    novel.assert_stdout_eq("- a.md\n");

    let book = sandbox.run(&["list", "--from", "@Book"]);
    book.assert_success();
    book.assert_stdout_eq("");
}

/// Verifies `[templates] directory` from the on-disk TOML drives template
/// discovery in a spawned process.
///
/// Executes vs asserts:
/// - Executes: `traces trust`, then `completions --list-templates` with the
///   template placed only under the configured (non-default) directory.
/// - Asserts: the configured directory's template is listed.
///
/// Defect class: disk-config-untested.
#[test]
fn templates_directory_config_drives_template_listing_from_disk() {
    let sandbox = Sandbox::new();
    sandbox
        .write_config_fixture(&ConfigFixture::new().with_templates("custom"));
    fs::create_dir_all(sandbox.root().join("custom"))
        .expect("create configured templates dir");
    fs::write(sandbox.root().join("custom/alpha.md"), "custom body\n")
        .expect("write custom template");

    sandbox.run(&["trust"]).assert_success();

    let list = sandbox.run(&["completions", "--list-templates"]);
    list.assert_success();
    list.assert_stdout_eq("alpha\n");
}

/// Verifies the discovery stage: a project with no local config file fails
/// before any trust or parse work, with the discovery diagnostic.
///
/// Executes vs asserts:
/// - Executes: `traces list` in a sandbox with no `.traces/config.toml`.
/// - Asserts: exit code 1, never 101, diagnostic code
///   `traces::cli::config_discovery_failed`, empty stdout.
///
/// Defect class: unwitnessed-contract (failure-stage table).
#[test]
fn missing_local_config_fails_at_the_discovery_stage() {
    let sandbox = Sandbox::new();

    let list = sandbox.run(&["list"]);
    list.assert_code(1);
    list.assert_never_exit_101();
    list.assert_stdout_eq("");
    assert!(
        plain(&list.stderr).contains("traces::cli::config_discovery_failed"),
        "stderr: {}",
        list.stderr
    );
}

/// Verifies the stale-trust stage: editing the config after trust fails the
/// load with `config_build_untrusted` and the Stale flavor.
///
/// Executes vs asserts:
/// - Executes: `traces trust`, then rewrites `.traces/config.toml` with
///   different bytes, then `traces list`.
/// - Asserts: exit code 1, `traces::cli::config_build_untrusted`, and the
///   `untrusted: Stale` flavor (not the never-trusted flavor).
///
/// Defect class: unwitnessed-contract (failure-stage table).
#[test]
fn edited_config_fails_at_the_trust_stage_as_stale() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();

    sandbox.write_config_fixture(&ConfigFixture::raw(
        "[templates]\ndirectory = \"templates\"\n\n# edited after trust\n",
    ));

    let list = sandbox.run(&["list"]);
    list.assert_code(1);
    list.assert_never_exit_101();
    assert!(
        plain(&list.stderr).contains("traces::cli::config_build_untrusted"),
        "stderr: {}",
        list.stderr
    );
    assert!(
        list.stderr.contains("untrusted: Stale"),
        "stale flavor expected; stderr: {}",
        list.stderr
    );
}

/// Verifies the parse stage is reachable AFTER trust: `trust` hashes the
/// malformed bytes without parsing them, so trusting the malformed config
/// first succeeds and the next load fails at parse.
///
/// The naive arrangement (trust a valid config, then corrupt it) halts at
/// the stale-trust stage instead — that path is covered by
/// [`edited_config_fails_at_the_trust_stage_as_stale`].
///
/// Executes vs asserts:
/// - Executes: `traces trust` over malformed TOML, then `traces list`.
/// - Asserts: trust exits 0 (hash-without-parse), the load exits 1 with
///   `traces::cli::config_build_config_file_failed`.
///
/// Defect class: trust-before-parse confusion.
#[test]
fn malformed_config_trusted_first_fails_at_the_parse_stage() {
    let sandbox = Sandbox::new();
    sandbox.write_config_fixture(&ConfigFixture::raw("unknown_key = true\n"));

    // `trust` records the content baseline without parsing: the malformed
    // bytes are trusted as-is.
    sandbox.run(&["trust"]).assert_success();

    let list = sandbox.run(&["list"]);
    list.assert_code(1);
    list.assert_never_exit_101();
    assert!(
        plain(&list.stderr)
            .contains("traces::cli::config_build_config_file_failed"),
        "stderr: {}",
        list.stderr
    );
}

/// Verifies the build-stage field-key validation: config that parses cleanly
/// but carries an empty field key fails after trust, with the field-key
/// diagnostic.
///
/// Executes vs asserts:
/// - Executes: `traces trust` over syntactically-valid TOML with `class_field =
///   ""`, then `traces list`.
/// - Asserts: trust exits 0 (the key validates during the build stage, not at
///   parse), the load exits 1 with
///   `traces::cli::config_build_invalid_field_key`.
///
/// Defect class: trust-before-parse confusion (stage conflation).
#[test]
fn invalid_field_key_fails_at_the_build_stage() {
    let sandbox = Sandbox::new();
    sandbox.write_config_fixture(&ConfigFixture::raw(
        "[templates]\ndirectory = \"templates\"\n\n[schemas]\nclass_field = \
         \"\"\n",
    ));

    sandbox.run(&["trust"]).assert_success();

    let list = sandbox.run(&["list"]);
    list.assert_code(1);
    list.assert_never_exit_101();
    assert!(
        plain(&list.stderr)
            .contains("traces::cli::config_build_invalid_field_key"),
        "stderr: {}",
        list.stderr
    );
}

/// Verifies the global-config positive leg: a global `[templates] directory`
/// written to the child's config home changes what a spawned
/// `completions --list-templates` renders.
///
/// Executes vs asserts:
/// - Executes: local trust, then a global config plus global template file
///   planted under [`Sandbox::config_home`], then `completions
///   --list-templates`.
/// - Asserts: both the local and the global template are listed — the global
///   layer contributed its configured directory. If the sandbox env policy
///   (ticket 09) dropped `XDG_CONFIG_HOME`, the global config would be
///   invisible and this test fails.
///
/// Defect class: disk-config-untested (global leg never spawn-covered).
#[test]
fn global_templates_directory_changes_template_listing() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
    sandbox.write_template("local-only.md", "local body\n");

    let global_dir = sandbox.config_home().join("traces");
    fs::create_dir_all(global_dir.join("global_templates"))
        .expect("create global templates dir");
    fs::write(
        global_dir.join("config.toml"),
        "[templates]\ndirectory = \"global_templates\"\n",
    )
    .expect("write global config");
    fs::write(
        global_dir.join("global_templates/from-global.md"),
        "global body\n",
    )
    .expect("write global template");

    let list = sandbox.run(&["completions", "--list-templates"]);
    list.assert_success();
    assert!(list.stdout.contains("local-only"), "stdout: {}", list.stdout);
    assert!(
        list.stdout.contains("from-global"),
        "global template missing from listing; stdout: {}",
        list.stdout
    );
}

/// Verifies the global-config parse leg: a trusted valid local config plus a
/// malformed global config fails the load at the parse stage.
///
/// Executes vs asserts:
/// - Executes: local trust, then a malformed global config planted under
///   [`Sandbox::config_home`], then `traces list`.
/// - Asserts: exit code 1 with `traces::cli::config_build_config_file_failed`
///   carrying the planted content's unknown-field message.
///
/// Defect class: trust-before-parse confusion (the local trust baseline
/// cannot protect the global layer).
#[test]
fn malformed_global_config_fails_parse_for_trusted_local() {
    let sandbox = Sandbox::new();
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();

    let global_dir = sandbox.config_home().join("traces");
    fs::create_dir_all(&global_dir).expect("create global config dir");
    fs::write(global_dir.join("config.toml"), "unknown_key = true\n")
        .expect("write malformed global config");

    let list = sandbox.run(&["list"]);
    list.assert_code(1);
    list.assert_never_exit_101();
    assert!(
        plain(&list.stderr)
            .contains("traces::cli::config_build_config_file_failed"),
        "stderr: {}",
        list.stderr
    );
    assert!(
        list.stderr.contains("unknown field `unknown_key`"),
        "failure must come from the planted global config; stderr: {}",
        list.stderr
    );
}
