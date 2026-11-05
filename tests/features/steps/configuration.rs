//! Configuration scope, discovery, diagnostics, and initialization through the
//! CLI.

use std::fs;

use pretty_assertions::assert_eq;
use rstest_bdd_macros::{given, then, when};

use super::{LastRun, Sandbox};
use crate::{
    fixtures::ConfigFixture,
    support::{Run, assert_reports_same_path},
};

#[then("initialization creates the default configuration files")]
fn initialization_creates_default_configuration_files(
    sandbox: &Sandbox,
    last_run: &LastRun,
) {
    last_run.run().assert_success();
    assert!(sandbox.root().join(".traces").is_dir());
    assert!(sandbox.root().join(".traces/templates").is_dir());
    let config = fs::read_to_string(sandbox.root().join(".traces/config.toml"))
        .expect("init writes a readable config");
    assert!(
        config.contains("directory = \".traces/templates\""),
        "config: {config}"
    );
    assert!(config.contains("output_dir = \".\""), "config: {config}");
}

#[then("stderr reports successful initialization for the project")]
fn stderr_reports_successful_initialization(
    sandbox: &Sandbox,
    last_run: &LastRun,
) {
    let run = last_run.run();
    assert_reports_same_path(
        &run.stderr,
        "initialised traces in ",
        "\n",
        sandbox.root(),
    );
}

#[given("a project already initialized with a custom configuration")]
fn project_already_initialized(sandbox: &Sandbox) {
    sandbox.write_config_fixture(&ConfigFixture::raw(
        "[templates]\ndirectory = \"custom\"\n",
    ));
    fs::create_dir_all(sandbox.root().join("custom"))
        .expect("create custom directory");
}

#[then("the existing configuration is unchanged")]
fn existing_configuration_is_unchanged(sandbox: &Sandbox) {
    assert_eq!(
        fs::read_to_string(sandbox.root().join(".traces/config.toml"))
            .expect("read existing configuration"),
        "[templates]\ndirectory = \"custom\"\n"
    );
    assert!(sandbox.root().join("custom").is_dir());
    assert!(!sandbox.root().join(".traces/templates").exists());
}

fn write_global_config(sandbox: &Sandbox, config: &str, template: &str) {
    let root = sandbox.config_home().join("traces");
    fs::create_dir_all(root.join("global_templates"))
        .expect("create global template directory");
    fs::write(root.join("config.toml"), config).expect("write global config");
    fs::write(root.join("global_templates/daily.md"), template)
        .expect("write global template");
}

#[given(
    "a trusted project with a global template directory and a local output \
     directory"
)]
fn project_with_global_template_and_local_output(sandbox: &Sandbox) {
    sandbox.write_config_fixture(&ConfigFixture::raw(
        "[templates]\noutput_dir = \"notes\"\n",
    ));
    fs::create_dir_all(sandbox.root().join("notes"))
        .expect("create local output directory");
    sandbox.run(&["trust"]).assert_success();
    write_global_config(
        sandbox,
        "[templates]\ndirectory = \"global_templates\"\noutput_dir = \
         \"ignored\"\n",
        "# Global daily\n",
    );
}

#[given("a trusted project with a global task filter and no local task filter")]
fn project_with_global_task_filter(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_note(
        "todo.md",
        "- [ ] global task #global\n- [ ] local task #local\n",
    );
    sandbox.run(&["trust"]).assert_success();
    let global = sandbox.config_home().join("traces");
    fs::create_dir_all(&global).expect("create global config directory");
    fs::write(
        global.join("config.toml"),
        "[tasks]\ntag_filters = [\"global\"]\n",
    )
    .expect("write global task filter");
}

#[given("a trusted project with both local and global task filters")]
fn project_with_local_and_global_task_filters(sandbox: &Sandbox) {
    project_with_global_task_filter(sandbox);
    sandbox.write_config_fixture(
        &ConfigFixture::new().with_task_tag_filters(&["local"]),
    );
    sandbox.run(&["trust"]).assert_success();
}

#[then("the rendered template is written under the local output directory")]
fn rendered_template_is_under_local_output(
    sandbox: &Sandbox,
    last_run: &LastRun,
) {
    last_run.run().assert_success();
    let output = sandbox.root().join("notes/daily.md");
    assert!(output.is_file(), "missing rendered file: {}", output.display());
    assert_eq!(
        fs::read_to_string(output).expect("read rendered file"),
        "# Global daily"
    );
    assert!(!sandbox.root().join("ignored/daily.md").exists());
}

#[given("a nested project with outer, nearest, and global templates")]
fn nested_project_with_multiple_configurations(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.write_template("outer.md", "outer\n");
    let nested = sandbox.root().join("nested");
    fs::create_dir_all(nested.join(".traces"))
        .expect("create inner config directory");
    fs::create_dir_all(nested.join("templates"))
        .expect("create inner template directory");
    fs::create_dir_all(nested.join("leaf"))
        .expect("create nested working directory");
    fs::write(
        nested.join(".traces/config.toml"),
        "[templates]\ndirectory = \"templates\"\n",
    )
    .expect("write inner config");
    fs::write(nested.join("templates/nearest.md"), "near\n")
        .expect("write inner template");
    sandbox.run(&["trust", "nested"]).assert_success();
    let global = sandbox.config_home().join("traces");
    fs::create_dir_all(global.join("global_templates"))
        .expect("create global templates");
    fs::write(
        global.join("config.toml"),
        "[templates]\ndirectory = \"global_templates\"\n",
    )
    .expect("write global config");
    fs::write(global.join("global_templates/shared.md"), "shared\n")
        .expect("write global template");
}

#[when("I list templates from the nested project")]
fn list_templates_from_nested_project(
    sandbox: &Sandbox,
    last_run: &mut LastRun,
) {
    let output = sandbox
        .command(&["completions", "--list-templates"])
        .current_dir(sandbox.root().join("nested/leaf"))
        .output()
        .expect("spawn template listing");
    last_run.set(Run {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    });
}

#[then("only the nearest local and global templates are listed")]
fn only_nearest_local_and_global_templates_are_listed(last_run: &LastRun) {
    let run = last_run.run();
    run.assert_success();
    run.assert_stdout_eq("nearest\nshared\n");
}

#[given("an untrusted project configuration")]
fn untrusted_project_configuration(sandbox: &Sandbox) {
    sandbox.write_config();
}

#[given("a malformed project configuration that is trusted")]
fn malformed_trusted_project_configuration(sandbox: &Sandbox) {
    sandbox.write_config_fixture(&ConfigFixture::raw("unknown_key = true\n"));
    sandbox.run(&["trust"]).assert_success();
}

#[given("a trusted project whose configuration was changed to malformed TOML")]
fn changed_trusted_configuration(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
    sandbox.write_config_fixture(&ConfigFixture::raw("unknown_key = true\n"));
}

#[given("a trusted local project with malformed global configuration")]
fn malformed_global_configuration(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
    let global = sandbox.config_home().join("traces");
    fs::create_dir_all(&global).expect("create global config directory");
    fs::write(global.join("config.toml"), "unknown_key = true\n")
        .expect("write malformed global config");
}

#[given("a project configuration with an invalid field key that is trusted")]
fn trusted_project_with_invalid_field_key(sandbox: &Sandbox) {
    sandbox.write_config_fixture(&ConfigFixture::raw(
        "[schemas]\nclass_field = \"\"\n",
    ));
    sandbox.run(&["trust"]).assert_success();
}

#[given(
    "a project configuration that writes a section as an array and is trusted"
)]
fn trusted_project_with_array_section(sandbox: &Sandbox) {
    sandbox.write_config_fixture(&ConfigFixture::raw("templates = [\"x\"]\n"));
    sandbox.run(&["trust"]).assert_success();
}

#[then(r#"stderr contains diagnostic code "{code}""#)]
fn stderr_contains_diagnostic_code(last_run: &LastRun, code: String) {
    let run = last_run.run();
    assert_eq!(
        run.stderr.lines().next().unwrap_or(""),
        code,
        "unexpected first diagnostic line; stderr: {}",
        run.stderr
    );
}

#[given("a trusted local configuration at the project root")]
fn trusted_configuration_at_discovery_ceiling(sandbox: &Sandbox) {
    sandbox.write_config();
    sandbox.run(&["trust"]).assert_success();
}

#[when("I run \"traces list\" with the project root as the discovery ceiling")]
fn list_with_ceiling(sandbox: &Sandbox, last_run: &mut LastRun) {
    // The ceiling must equal the working directory the process reports. POSIX
    // resolves symlinks there (macOS `/var` -> `/private/var`), so
    // canonicalize. On Windows, `canonicalize` returns a `\\?\` verbatim path
    // that never equals the plain (possibly 8.3 short) `current_dir` form.
    let ceiling = if cfg!(windows) {
        sandbox.root().to_path_buf()
    } else {
        sandbox.root().canonicalize().expect("canonical project root")
    };
    last_run.set(sandbox.run_with_envs(&["list"], &[(
        "TRACES_CEILING_DIRS",
        ceiling.as_os_str(),
    )]));
}
