//! Exercises template rendering and writes through the spawned CLI.

use pretty_assertions::assert_eq;

use super::support::Sandbox;

/// Renders without `--dry-run` and checks the rendered file lands on disk
/// at the default output path.
///
/// Covers a default-path write; overwrite and redirected writes are tested
/// separately below.
#[test]
fn writes_the_rendered_file_to_the_default_output_path() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_template("daily.md", "# {{ \"Today\" }}\n");

    let template = sandbox.run(&["template", "-i", "daily", "--no-input"]);

    assert!(template.is_success(), "stderr: {}", template.stderr);
    let written = sandbox.root().join("daily.md");
    assert!(written.is_file());
    assert_eq!(
        std::fs::read_to_string(written).expect("read written file"),
        "# Today"
    );
}

/// Verifies overwrite refusal is non-destructive at process level: spawning
/// a render over an existing output file fails with the
/// `traces::cli::template::output_exists` diagnostic and leaves the target
/// byte-identical. Then spawns the `-f` (force) and `-o` (output override)
/// argv paths, which in-crate tests only exercise by constructing the struct
/// directly.
///
/// Executes vs asserts:
/// - Executes: `traces template -i daily --no-input` over an existing file,
///   then the same with `-f`, then with `-o elsewhere.md`.
/// - Asserts: refusal exits 1 with the diagnostic and the file is untouched;
///   `-f` exits 0 and replaces the content; `-o` exits 0 and writes to the
///   override path.
///
/// Defect class: destructive-overwrite.
#[test]
fn overwrite_refusal_is_non_destructive_and_force_and_output_paths_spawn() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_template("daily.md", "new content\n");
    std::fs::write(sandbox.root().join("daily.md"), "old content\n")
        .expect("seed existing output");

    let refusal = sandbox.run(&["template", "-i", "daily", "--no-input"]);
    refusal.assert_code(1);
    refusal.assert_never_exit_101();
    assert!(
        refusal.stderr.contains("traces::cli::template::output_exists"),
        "stderr: {}",
        refusal.stderr
    );
    assert_eq!(
        std::fs::read_to_string(sandbox.root().join("daily.md"))
            .expect("read untouched output"),
        "old content\n",
        "refusal must leave the target byte-identical"
    );

    let forced = sandbox.run(&["template", "-i", "daily", "--no-input", "-f"]);
    forced.assert_success();
    forced.assert_never_exit_101();
    assert_eq!(
        std::fs::read_to_string(sandbox.root().join("daily.md"))
            .expect("read overwritten output"),
        "new content"
    );

    let redirected = sandbox.run(&[
        "template",
        "-i",
        "daily",
        "--no-input",
        "-o",
        "elsewhere.md",
    ]);
    redirected.assert_success();
    redirected.assert_never_exit_101();
    assert_eq!(
        std::fs::read_to_string(sandbox.root().join("elsewhere.md"))
            .expect("read redirected output"),
        "new content"
    );
}

/// Verifies the real terminal dialog path is deterministic on null stdin
/// WITHOUT `--no-input`: that flag swaps in the preset dialog provider, but
/// the contract under test is the genuine `TerminalDialogProvider`
/// short-circuiting to defaults on a non-TTY stdin (audit §14 P1.16).
///
/// The spawn is bounded by the std-only watchdog in
/// [`Sandbox::run_with_watchdog`], so a hang is killed and reported instead
/// of wedging the suite; no new dependency or ticket-18 timeout is needed.
///
/// Executes vs asserts:
/// - Executes: `traces template -i daily -f` (interactive path, null stdin)
///   under the watchdog.
/// - Asserts: exit 0 and the default-path render exactly (`anon`), not merely
///   termination.
///
/// Defect class: nondeterministic-TTY.
#[test]
fn non_tty_dialog_short_circuits_to_defaults_without_hanging() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_template(
        "daily.md",
        "{{ ui.text_input(\"name\", \"anon\") }}\n",
    );

    let rendered = sandbox
        .run_with_watchdog(
            &["template", "-i", "daily", "-f"],
            std::time::Duration::from_secs(15),
        )
        .expect("non-TTY render finished within the watchdog window");

    rendered.assert_success();
    rendered.assert_never_exit_101();
    assert_eq!(
        std::fs::read_to_string(sandbox.root().join("daily.md"))
            .expect("read rendered file"),
        "anon",
        "non-TTY stdin must render the prompt defaults exactly"
    );
}

/// Verifies the feedback loop: a rendered note written into the project is
/// picked up by the next spawned query (writer → fs → refresh → query).
/// Write units test path resolution only; index units read pre-existing
/// files — this closes the loop at process level.
///
/// Executes vs asserts:
/// - Executes: `traces template -i daily --no-input` writing a tagged note into
///   the project root, then `traces list --from "#fromtemplate"`.
/// - Asserts: the rendered file is query-visible through its tag.
///
/// Defect class: untested-feedback-loop.
#[test]
fn rendered_output_reenters_the_index() {
    let sandbox = Sandbox::trusted_via_cli();
    // The tag is assembled dynamically so the template file itself does not
    // carry the literal `#fromtemplate` tag (it is inside the project and
    // indexed too); only the rendered output does.
    sandbox.write_template(
        "daily.md",
        "# Rendered\n\n- [ ] from template #from{{ \"template\" }}",
    );

    let template = sandbox.run(&["template", "-i", "daily", "--no-input"]);
    template.assert_success();
    template.assert_never_exit_101();

    let listed = sandbox.run(&["list", "--from", "#fromtemplate"]);
    listed.assert_success();
    listed.assert_stdout_eq("- daily.md\n");
}
#[test]
fn renders_file_sourced_select_field_in_e2e_template() {
    let sandbox = Sandbox::trusted_via_cli();

    // Write values file DTO
    sandbox.write_schema_value(
        "categories.toml",
        "[[entries]]\nid = \"rust\"\ntitle = \"Rust Programming\"\n",
    );

    // Write schema file referencing values file
    sandbox.write_schema(
        "topic",
        r#"
        [fields.category]
        type = "select"
        values = { path = "values/categories.toml", value = "id", label = "title" }
        "#,
    );

    // Write template referencing the schema select field
    sandbox.write_template(
        "topic_note.md",
        "Category: {{ schema.get('topic').field('category')[0].label }} ({{ \
         schema.get('topic').field('category')[0].value }})",
    );

    // Run the template generation command
    let run = sandbox.run(&["template", "-i", "topic_note", "--no-input"]);

    assert!(run.is_success(), "stderr: {}", run.stderr);
    let written = sandbox.root().join("topic_note.md");
    assert!(written.is_file());
    assert_eq!(
        std::fs::read_to_string(written).expect("read topic note"),
        "Category: Rust Programming (rust)"
    );
}

// Relocated from the dissolved `dispatch.rs` (ticket 19): dry-run and
// render-error contracts join the write-path capability file.

/// Checks `--dry-run` prints rendered content to stdout without writing a
/// file.
///
/// A negative filesystem assertion: only a real process against a real disk
/// can prove absence; a mock can't.
#[test]
fn dry_run_prints_rendered_content_to_stdout_without_writing() {
    let sandbox = Sandbox::trusted_via_cli();
    // Notes live under `notes/`, scoped away from `templates/`:
    // `WorkspaceIndex` indexes every markdown file under the project root,
    // including the template file itself, so an unscoped `query.from()`
    // here would also count `report.md`.
    sandbox.write_note("notes/a.md", "# A\n");
    sandbox.write_note("notes/b.md", "# B\n");
    sandbox.write_template(
        "report.md",
        "{{ query.from(\"notes/\") | length }} notes",
    );

    let template =
        sandbox.run(&["template", "-i", "report", "--dry-run", "--no-input"]);

    assert!(template.is_success(), "stderr: {}", template.stderr);
    assert_eq!(template.stdout, "2 notes");
    assert!(!sandbox.root().join("report.md").exists());
}

/// Checks a render error surfaces the stable `render_query_failed`
/// diagnostic code.
///
/// That code is the contract external tooling greps stderr for. Proves it
/// survives the full CLI error path, not just that the error value
/// constructs correctly. Does not reassert the exact source location: see
/// the trailing comment inside this test for why and where that's covered
/// instead.
#[test]
fn render_error_reports_a_stable_diagnostic_code() {
    let sandbox = Sandbox::trusted_via_cli();
    sandbox.write_template(
        "broken.md",
        "line one\n{{ query.from().sort(\"nope.bad\") }}\n",
    );

    let template =
        sandbox.run(&["template", "-i", "broken", "--dry-run", "--no-input"]);

    assert!(!template.is_success());
    assert!(template.stdout.is_empty(), "stdout: {}", template.stdout);
    assert!(
        template.stderr.contains("traces::cli::template::render_query_failed"),
        "stderr: {}",
        template.stderr
    );
    // The exact `broken.md:2:<col>` location this error carries is verified
    // unit-level against `minijinja::Error` directly, in `src/cli/error.rs`'s
    // `location::line_column_returns_the_1_based_char_column` and
    // `location::render_error_location_reports_name_line_and_column_for_a_real_render_error`
    // tests, and end-to-end (in-process) in `src/cli/mod.rs`'s
    // `query_workflows::template_render_errors_identify_the_failing_template_and_line_through_cli_dispatch`,
    // which asserts the full `report.md:2:15` string. Not reasserted here:
    // Miette line-wraps long causal chains across lines with a `│`
    // continuation glyph that can land inside a path with no original
    // whitespace there, so reconstructing it from captured stderr text is
    // not reliable (see module docs).
}
