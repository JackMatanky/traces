//! Golden exact-output baselines for query presentation commands.
//!
//! Pins the exact stdout shape of `traces task --table`, `traces task`,
//! `traces task --count`, and `traces list` against baseline files under
//! `tests/baselines/output/`. Coverage targets the format contract scripts
//! pipe on: a 20-row table with wide columns (including multibyte Unicode
//! cells measured with [`unicode_width`]-style display width), checkbox task
//! lists, count-only output, and bullet lists.
//!
//! # Updating baselines
//!
//! Baselines are checked in and compared byte-for-byte. When a deliberate
//! output change is intended, regenerate every baseline in one pass and
//! review the diff before committing:
//!
//! ```sh
//! TRACES_UPDATE_GOLDEN=1 cargo test --test e2e golden::
//! ```
//!
//! The variable is read only by this module's [`update_golden`] helper; no
//! other e2e test writes to `tests/baselines/`.
//!
//! [`unicode_width`]: https://docs.rs/unicode-width

use std::{
    fs,
    path::{Path, PathBuf},
};

use pretty_assertions::assert_eq;

use super::support::Sandbox;

/// Directory holding checked-in stdout baselines for this module.
const BASELINE_DIR: &str = "tests/baselines/output";

/// Environment variable enabling baseline regeneration.
const UPDATE_ENV: &str = "TRACES_UPDATE_GOLDEN";

/// Compares `actual` against `tests/baselines/output/<name>.txt`, or rewrites
/// the baseline when [`UPDATE_ENV`] is set to `1`.
///
/// The update path lets the parent workflow regenerate all goldens in one
/// test run; it still asserts the file was written so a silent env typo
/// cannot fake a green run.
fn assert_matches_baseline(actual: &str, name: &str) {
    let path = baseline_path(name);
    if std::env::var(UPDATE_ENV).is_ok_and(|value| value == "1") {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create baseline dir");
        }
        fs::write(&path, actual).expect("write baseline");
    }
    assert!(path.is_file(), "missing baseline {}", path.display());
    let expected =
        fs::read_to_string(&path).expect("read baseline").replace("\r\n", "\n");
    assert_eq!(actual, expected, "output baseline: {}", path.display());
}

/// Asserts that `stderr` is `<label> from <root>\n`, comparing the path by
/// filesystem identity so Windows verbatim (`\\?\`) and short (`RUNNER~1`)
/// spellings of the same directory match.
fn assert_summary_from_root(stderr: &str, label: &str, root: &Path) {
    let reported = stderr
        .strip_prefix(&format!("{label} from "))
        .and_then(|rest| rest.strip_suffix('\n'))
        .unwrap_or_default();
    assert!(!reported.is_empty(), "unexpected summary line: {stderr:?}");
    assert_eq!(
        Path::new(reported).canonicalize().expect("canonicalize reported"),
        root.canonicalize().expect("canonicalize root"),
    );
}

/// Resolves `tests/baselines/output/<name>.txt` relative to the crate root.
fn baseline_path(name: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(BASELINE_DIR);
    manifest_dir.join(format!("{name}.txt"))
}

/// Writes the 20-row task fixture used by every baseline test.
///
/// Four notes under `tasks/`: mixed completion states, due dates, priorities
/// (emoji tokens exercising multibyte parsing), and nested indentation. Rows
/// are spread across notes so table `File` widths and task-list suffixes both
/// see more than one width class.
fn write_task_fixture(sandbox: &Sandbox) {
    sandbox.write_note(
        "tasks/alpha.md",
        "- [ ] Alpha item 📅 2026-10-01 🔺\n- [x] Beta item 📅 2026-10-05 \
         🔽\n- [ ] Gamma item with a deliberately long descriptive text 📅 \
         2026-10-09 🔼\n- [ ] Delta 日本語テキスト 📅 2026-10-12 🔺\n- [x] \
         Epsilon 📅 2026-10-15 🔽\n",
    );
    sandbox.write_note(
        "tasks/beta.md",
        "- [ ] Zeta 📅 2026-11-02 🔺\n- [ ] Eta long-name task with wide \
         columns padding check 📅 2026-11-03 🔼\n- [x] Theta 📅 2026-11-04 \
         🔽\n- [ ] Iota item 📅 2026-11-05 🔺\n- [ ] Kappa 📅 2026-11-06 🔽\n",
    );
    sandbox.write_note(
        "tasks/gamma.md",
        "- [ ] Lambda 📅 2026-12-01 🔺\n- [ ] Mu item with a wide Unicode \
         label 東京タワー 📅 2026-12-02 🔼\n- [x] Nu 📅 2026-12-03 🔽\n- [ ] \
         Xi 📅 2026-12-04 🔺\n- [ ] Omicron 📅 2026-12-05 🔽\n",
    );
    sandbox.write_note(
        "tasks/nested.md",
        "- [ ] Parent task\n\x20 - [ ] Child task one 📅 2026-12-20 🔺\n\x20 \
         - [x] Child task two 📅 2026-12-21 🔽\n- [ ] Pi 📅 2026-12-22 🔺\n- \
         [ ] Rho final row with padding 📅 2026-12-23 🔼\n",
    );
}

/// `traces task --table` renders a 20-row Markdown table with Unicode-wide
/// cells padded by display width.
#[test]
fn task_table_renders_twenty_unicode_wide_rows() {
    let sandbox = Sandbox::trusted_via_cli();
    write_task_fixture(&sandbox);
    sandbox.run(&["index"]).assert_success();

    let task = sandbox.run(&["task", "--table"]);
    task.assert_success();
    task.assert_never_exit_101();
    assert_summary_from_root(&task.stderr, "20 task(s)", sandbox.root());
    assert_matches_baseline(&task.stdout, "task-table-20-rows");
}

/// `traces task` renders checkbox lines with `(path)` suffixes for all 20
/// rows, including nested indentation.
#[test]
fn task_list_renders_twenty_checkbox_rows_with_paths() {
    let sandbox = Sandbox::trusted_via_cli();
    write_task_fixture(&sandbox);
    sandbox.run(&["index"]).assert_success();

    let task = sandbox.run(&["task"]);
    task.assert_success();
    task.assert_never_exit_101();
    assert_summary_from_root(&task.stderr, "20 task(s)", sandbox.root());
    assert_matches_baseline(&task.stdout, "task-list-20-rows");
}

/// `traces task --count` writes only the row total to stdout and nothing to
/// stderr.
#[test]
fn task_count_renders_only_the_total() {
    let sandbox = Sandbox::trusted_via_cli();
    write_task_fixture(&sandbox);
    sandbox.run(&["index"]).assert_success();

    let count = sandbox.run(&["task", "--count"]);
    count.assert_success();
    count.assert_never_exit_101();
    count.assert_stderr_eq("");
    assert_matches_baseline(&count.stdout, "task-count-20-rows");
}

/// `traces list` renders one bullet per page for a 20-note workspace.
#[test]
fn list_renders_twenty_bullets_in_default_order() {
    let sandbox = Sandbox::trusted_via_cli();
    for index in 0..20 {
        sandbox.write_note(&format!("notes/item-{index:02}.md"), "# Item\n");
    }
    sandbox.run(&["index"]).assert_success();

    let list = sandbox.run(&["list"]);
    list.assert_success();
    list.assert_never_exit_101();
    assert_summary_from_root(&list.stderr, "20 page(s)", sandbox.root());
    assert_matches_baseline(&list.stdout, "list-20-rows");
}

/// `traces table` renders a 20-row page table with Unicode-wide and long
/// cells padded by display width.
#[test]
fn page_table_renders_twenty_unicode_wide_rows() {
    let sandbox = Sandbox::trusted_via_cli();
    let labels = ["東京タワー", "naïve café", "Zürich"];
    for (index, label) in (0..20).zip(labels.iter().cycle()) {
        sandbox.write_note(
            &format!("pages/page-{index:02}.md"),
            &format!(
                "---\nrating: {rating}\ntitle: {label} {index:02} with a \
                 deliberately long wide title cell\n---\n# Page\n",
                rating = index * 7 % 13,
            ),
        );
    }
    sandbox.run(&["index"]).assert_success();

    let table = sandbox.run(&[
        "table",
        "--column",
        "file.name",
        "--column",
        "title",
        "--column",
        "rating",
    ]);
    table.assert_success();
    table.assert_never_exit_101();
    assert_matches_baseline(&table.stdout, "page-table-20-rows");
}
