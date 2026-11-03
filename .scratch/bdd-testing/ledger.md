# Migration Ledger

This ledger records deleted tests and their replacement proofs.
The ledger is append-only.
Each slice adds one section.

## Slice 1: Trust Lifecycle Pilot

### Row 1: Revocation Across Process Boundaries

- **Namespaced row IDs:** 01-G4, 01-G23, 01-G24, 05-G3
- **Behavior:** Revoke project trust durably so that a later run fails with
  `traces::cli::config_build_untrusted`.
- **Former test:** `e2e::trust::untrust_then_list_fails_with_the_untrusted_diagnostic`
- **Former layer:** `e2e`
- **Defect caught:** Untrust fails to revoke trust durably across process
  boundaries, which lets subsequent commands execute with untrusted
  configuration.
- **Survivor:** Scenario `Untrust revokes trust for later runs`
  (`tests/features/trust.feature`)
- **Proof:**
  - **Assertion comparison:** The survivor scenario S2 asserts at least what
    the old test asserted. The old test asserted `!list.is_success()` and
    substring `contains("traces::cli::config_build_untrusted")`, with
    unasserted stdout. Scenario S2 asserts exact exit code `1`, exact first
    line of stderr `traces::cli::config_build_untrusted`, and exact empty stdout
    `""`. S2 is strictly stronger.
  - **Kill demonstration:** Production code in `src/cli/untrust.rs` was
    modified so `untrust` omitted the call to `service.untrust(&subject)`,
    keeping the trust grant in the store. Both the old test and scenario S2
    failed when executed against the broken binary. The old test failed with
    `assertion failed: !list.is_success()`. Scenario S2 failed at `Then the exit
    code is 1` with `expected exit code 1, got Some(0)`. Reverting the
    modification restored both tests to passing.
  - **Mutation compare (before deletion):** Scoped run `mise run test:mutants -f
    src/config/trust.rs` generated 20 mutants. Result: 4 killed, 16 skipped, 0
    escaped. Escape set: `[]`.
  - **Mutation compare (after deletion):** Scoped run `mise run test:mutants -f
    src/config/trust.rs` generated 20 mutants. Result: 4 killed, 16 skipped, 0
    escaped. Escape set: `[]`. No new escape appeared. The baseline file
    `mutarust-baseline.json` stayed untouched.
  - **Name diff:** Verified `cargo nextest list --all-features --run-ignored all`
    before and after deletion. Exactly one test was removed:
    `traces::e2e trust::untrust_then_list_fails_with_the_untrusted_diagnostic`.
  - **Twin re-read:** Re-read
    `tests/e2e/trust.rs::trust_all_and_untrust_all_count_asserted_via_trust_list`.
    That test verifies the batch `--all` stream and confirms `trust list`
    reports empty stdout. It does not test the default single-project CLI
    round trip to a failing command. Re-read all tests touching `untrust` in
    `tests/e2e/workflow.rs`, `tests/e2e/config.rs`, `tests/e2e/process_contract.rs`,
    `src/cli/untrust.rs`, `src/config/service.rs`, and `src/config/trust.rs`.
    None is a duplicate twin of the cross-process CLI revocation-then-list
    guarantee.
- **Successor note for arch-design:** Trust revocation CLI behavior is verified
  through BDD feature `tests/features/trust.feature` (`Untrust revokes trust for
  later runs`). Step definitions live in `tests/e2e/bdd/trust.rs`.
- **Evidence note (mutarust behavior on test-only diff):** Executed `mise run
  test:mutants --git-diff --dry-run` with a diff limited to `tests/e2e/trust.rs`.
  The task generated 0 mutants (`Total: 0 mutation(s) would be generated`).
  Labelled as evidence only; never used as proof.

## Pilot Rubric Answers (Slice 1 Close)

- **Question 1: Does the BDD framework execute across all target operating
  systems in CI and local stress testing?**
  **Answer: Yes.**
  - **Evidence:** Draft PR 3 CI run
    (<https://github.com/JackMatanky/traces/actions/runs/41626080102>) passed
    across all three operating systems (Ubuntu, macOS, Windows).
  - Scenarios S1 (`Init does not grant trust`) and S2 (`Untrust revokes trust for
    later runs`) passed 100 of 100 iterations under local stress testing.

- **Question 2: Does the deletion and replacement procedure preserve coverage
  and prove that scenarios catch defects?**
  **Answer: Yes.**
  - **Evidence:** The kill demonstration in `src/cli/untrust.rs` broke both the
    former test and scenario S2 (`expected exit code 1, got Some(0)`).
  - The mutation compare on `src/config/trust.rs` produced 0 new escapes before
    and after deletion.
  - The sixth gate check resolves all pilot pointers (01-G4, 01-G17, 01-G23,
    01-G24, 05-G3) against `tests/features/`.

## Slice 2: Process Contract

### Deletions Note

No existing tests were deleted in this slice. Spec section 9.3 lists no candidate
tests for slice 2. Existing e2e tests in `tests/e2e/process_contract.rs` and
`tests/e2e/workflow.rs` stay in place. All seven scenarios added in
`tests/features/process.feature` supply new BDD executable specifications for the
process contract without removing existing Rust tests.

### Pilot Rubric Answer (Slice 2 Close)

- **Question 3: Can a second cold session write one more scenario from this
  spec alone, with no framework-source read and no fixture beyond `Sandbox` and
  `LastRun`?**
  **Answer: Yes.**
  - **Evidence:** A fresh session wrote the scenario `Missing template exits
    with code one and reports template not found diagnostic` in
    `tests/features/process.feature`. It passed on the first run.
  - The session read no `rstest-bdd` source. It used only `Sandbox` and
    `LastRun`.
  - One gap: the spec does not list the diagnostic code strings. The session
    read `src/cli/error.rs` to find `traces::cli::template::not_found`.
    Authors of later slices must expect to read the code for code strings.

## Slice 3: Query Surface

### Row 1: Page Table Columns

- **Namespaced row IDs:** 01-G30, 02-G4, 02-G8.
- **Behavior:** The table command renders one Markdown row for each page.
- **Former test:** `e2e::query::table_renders_a_markdown_table_with_one_row_per_page`.
- **Former layer:** `e2e`.
- **Defect caught:** The table command can ignore requested columns.
- **Survivor:** `A page table renders one Markdown row per note` in
  `tests/features/query.feature`.
- **Proof:**
  - **Assertion comparison:** The old test checked exit success. It checked
    that stdout contained the header, `dune` and `9`. The scenario checks
    exit code 0 and the exact Markdown table on stdout.
  - **Kill demonstration:** A change to `src/cli/table.rs` made both requested
    columns read `file.path`. The old test failed because stdout had no `9`.
    The scenario failed because its table cells changed. Both tests ran
    against the changed code before this test was removed. The change was
    undone.
  - **Mutation compare:** Not required. The guarded connection between CLI
    columns and table output is in `src/cli/table.rs`.
  - **Twin re-read:** `tests/e2e/query.rs` still has the list output test.
    `src/cli/table.rs` still has unit tests for table output and columns.
    These tests do not check two CLI column flags through a spawned process.
- **Successor note for arch-design:** The scenario is the CLI table contract.
  Keep the in-process tests for field validation and escaping.

### Row 2: Default and Requested Task Table Columns

- **Namespaced row IDs:** 01-G30, 02-G4, 02-G8.
- **Behavior:** The task command renders the default task table or the
  requested task columns.
- **Former test:** `e2e::query::task_table_renders_default_and_custom_columns`.
- **Former layer:** `e2e`.
- **Defect caught:** The task table flag can render a task list instead.
- **Survivors:** `A task table displays the default columns and task states`
  and `A task table displays requested columns` in `tests/features/query.feature`.
- **Proof:**
  - **Assertion comparison:** The old test checked exit success for both
    commands. It checked that stdout contained five default headers, both
    tasks, `highest` and `low`. It checked that the requested table contained
    both field headers, `Alpha item` and `highest`. The two scenarios check
    exit code 0 and the exact stdout for each table.
  - **Kill demonstration:** A change to `src/cli/task.rs` reversed the table
    selection condition. The old test failed because stdout had no table
    header. Both scenarios failed because stdout was a task list. All three
    ran against the changed code before this test was removed. The change
    was undone.
  - **Mutation compare:** Not required. The guarded table flag and column
    selection are in `src/cli/task.rs`.
  - **Twin re-read:** `tests/e2e/query.rs` still has the task count test.
    `src/cli/task.rs` has unit tests for the table arguments. Neither checks
    both task table forms through a spawned process.
- **Successor note for arch-design:** Both scenarios pin the CLI table
  output. Keep the in-process tests for task fields and format errors.

### Row 3: Existing Markdown Path

- **Namespaced row IDs:** 02-G1, 02-G2.
- **Behavior:** An existing Markdown path selects the tasks and the count
  from only that note.
- **Former test:** `e2e::query::task_from_bare_markdown_path_selects_only_that_note`.
- **Former layer:** `e2e`.
- **Defect caught:** A task query can ignore the given source path.
- **Survivors:** `A bare Markdown path selects only tasks from that note`
  and `A bare Markdown path limits task counts to one note` in
  `tests/features/query.feature`.
- **Proof:**
  - **Assertion comparison:** The old test checked exit success and that
    stdout contained both selected tasks but no task from `other.md`.
    It also checked exit success, exact stdout `2\n` and empty stderr for
    the count command. The first scenario checks exit code 0 and exact
    task stdout. The second checks exact count stdout and empty stderr.
  - **Kill demonstration:** A change to `src/cli/mod.rs` ignored the CLI
    source argument. The old test and both scenarios failed: the result
    included the task from `other.md`. All three ran against the changed
    code before this test was removed. The change was undone.
  - **Mutation compare:** Not required. The guarded CLI source argument
    is parsed in `src/cli/mod.rs`.
  - **Twin re-read:** The task count test in `tests/e2e/query.rs` selects
    all notes. It does not check an existing source path.
- **Successor note for arch-design:** The scenarios pin the CLI path and
  count behavior. Keep source grammar tests in `src/query/`.

### Row 4: Missing Markdown Path

- **Namespaced row IDs:** 02-G1, 02-G2.
- **Behavior:** A missing Markdown path matches no task and does not
  cause an error.
- **Former test:** `e2e::query::task_from_nonexistent_markdown_path_matches_nothing_without_erroring`.
- **Former layer:** `e2e`.
- **Defect caught:** A missing source path can select an unrelated task.
- **Survivor:** `A missing Markdown path gives no task rows` in
  `tests/features/query.feature`.
- **Proof:**
  - **Assertion comparison:** The old test checked exit success, empty
    stdout and a `0 task(s)` substring in stderr. The scenario checks
    exit code 0, exact empty stdout and the exact count prefix with the
    project root path in stderr.
  - **Kill demonstration:** The same change to `src/cli/mod.rs` ignored
    the CLI source argument. Both the old test and the scenario failed
    because stdout had a task from `todo.md`. They ran against the changed
    code before this test was removed. The change was undone.
  - **Mutation compare:** Not required. The guarded CLI source argument
    is parsed in `src/cli/mod.rs`.
  - **Twin re-read:** The task count test in `tests/e2e/query.rs` selects
    all notes. It does not check a missing source path.
- **Successor note for arch-design:** The scenario pins the empty-result
  CLI contract. The source parser remains covered by in-process tests.

### Name Inventory

The `cargo nextest list --all-features --run-ignored all` list was read
before and after deletion. The four former `traces::e2e query::` names above
were removed. No other old e2e name was removed. The two renamed unit tests
in `src/cli/list.rs` and `src/cli/table.rs` now say `with_explicit_ascending`.
The rename does not close 02-G6. The sort flag scenarios close the gap.

### Retained Tests and Review

`tests/e2e/query.rs::class_descendant_selector_expands_through_the_schema_graph`
remains. It also tests schema graph expansion, which this slice does not
replace. The e2e tests for typo suggestions and source spans remain. The
scenarios check stable diagnostic codes. They do not check Miette text.
The list and task count e2e tests remain because they check distinct stream
and count behavior.

The review of `tests/features/query.feature` used the section 6.4 checklist.
Each scenario states one process-level result with one principal `When` and
three to five steps. The notes, tasks, fields and paths are concrete. The
assertions check exact output, exit codes or diagnostic codes. The title of
each scenario identifies the behavior if an assertion fails.

## Slice 6: Schema and Temporal at the Boundary

### Scenarios Added

The slice adds 18 scenarios in `tests/features/schema-temporal.feature`.
The rows are 04-S3, 04-S4, 04-S7, 04-T2, 04-T4, 04-T6 to 04-T8, and
04-B1 to 04-B7.

- 04-T2 shares "An offset-dated note matches the same instant to a
  nanosecond" with 04-B5; the scenario also pins 04-T2 because the note value
  carries a `+02:00` offset that must normalize to UTC.
- 04-B1 has one new scenario beside the Rust test
  `tests/e2e/query.rs::class_descendant_selector_expands_through_the_schema_graph`,
  which stays (spec section 7).
- 04-B6 shares one scenario: query filter and template `date_add` agree on the
  same month-end clamp (`1|2026-02-28`).

New step definitions live in `tests/e2e/bdd/process.rs` (`in time zone`,
`I list notes with filter:`) and `tests/e2e/bdd/schema_temporal.rs`
(project fixtures and `file "{path}" contains:`).

### Deletions Note

No candidate test was deleted in this run of slice 6. The slice's scenario
work is complete and passing. The deletion proofs are not complete:
the scoped mutation compares did not finish in this run.

- `config.rs::class_field_config_drives_class_selection_from_disk`
  stays for a die-rule reason: the candidate table maps it to 01-G21
  (slice 5, still `gap`) and 04-B2 (this slice). Spec section 7: a test
  whose rows span slices dies in the slice that completes its last row.
- `query.rs::list_filters_rfc3339_field_by_its_exact_instant` and
  `template.rs::renders_file_sourced_select_field_in_e2e_template` stay
  because their required mutation compares (before and after deletion,
  `-f src/temporal/datetime.rs` and
  `-f src/template/engine/schema.rs`) are unfinished. One before-run
  (`src/temporal/datetime.rs`) was started and stopped mid-run; the
  `src/template/engine/schema.rs` before-run never started. Spec section 7:
  if a proof cannot be completed, do not delete; record why. A later run
  must do the two before-runs, the deletions, the two after-runs, the name
  diff, and then complete the ledger rows below.

### Kill Demonstrations (recorded pre-deletion, changes reverted)

- `src/config/model/schemas.rs:42` `class_field_name()` returned the
  hardcoded `"hardcoded_class"`. The old test
  `class_field_config_drives_class_selection_from_disk` and the scenario
  `A configured file class field selects the matching note` both failed
  with empty stdout where `- a.md\n` / `- novel.md` were expected.
- `src/template/engine/schema.rs:264` `select_entry_value` inserted the
  key `"unmatched_label"` instead of `"label"`. The old test
  `renders_file_sourced_select_field_in_e2e_template` and the scenario
  `A template reads the labels and values of a file-sourced select field`
  both failed with `Category:  (rust)` versus
  `Category: Rust Programming (rust)`.
- `src/temporal/datetime.rs:492` `DateTimeShape::parse_any` used
  `Offset::UTC` instead of the parsed offset for RFC 3339 input. The old
  test `list_filters_rfc3339_field_by_its_exact_instant` and the scenario
  `An offset-dated note matches the same instant to a nanosecond` both
  failed with empty stdout where `- matching.md\n` was expected.

All three demonstrations ran on the pre-deletion tree (commit `102253b8`),
where the old tests were still present, and were then reverted.

### Mutation Compare Ledger (slice 6)

Scoped runs used `-f <file>` with `--timeout 1200 --workers 2
--no-diffs`. One compare completed. The `src/config/model/schemas.rs`
before-run ran at commit `102253b8`. It produced 3 mutants, 3 killed,
0 escaped, 0 errored. The compare is moot for deletion while
`class_field_config` stays. It records a clean escape set for slice 5's
later die. The `datetime.rs` before-run was stopped mid-run. The
`schema.rs` before-run and the after-runs did not start.
`mutarust-baseline.json` stayed untouched.

| File | Before | After |
|---|---|---|
| `src/config/model/schemas.rs` | 0 escapes (done) | n/a, test retained |
| `src/temporal/datetime.rs` | not completed | not started |
| `src/template/engine/schema.rs` | not started | not started |

The assertion comparisons and twin re-reads below stay valid for the
later conversion run; they were checked against the old tests at
commit `102253b8`.

### Row 1: Exact Instant Filtering Through Ingestion (conversion pending)

- **Namespaced row IDs:** 04-T2, 04-B5.
- **Behavior:** An RFC3339 note value with a `+02:00` offset normalizes
  to UTC at ingestion and matches a `Z`-spelled filter literal to the
  nanosecond in a spawned `list --where`.
- **Former test (planned):** `traces::e2e
  query::list_filters_rfc3339_field_by_its_exact_instant`.
- **Former layer:** `e2e`.
- **Defect caught:** Offset normalization or nanosecond precision could
  be lost between note ingestion and filter comparison.
- **Survivor:** `An offset-dated note matches the same instant to a
  nanosecond` in `tests/features/schema-temporal.feature`.
- **Proof status:** Kill demonstration done (above). Assertion
  comparison done. The old test asserted exit success and exact stdout
  `- matching.md\n` for the filter
  `due == "2026-07-29T12:30:00.123456789Z"`. It used two `+02:00` notes
  one nanosecond apart. The scenario states the same Given notes, the
  same filter, exit code 0 and the exact stdout docstring. The survivor
  asserts everything the old test asserted. Twin re-read done: the
  datetime unit tests pin the parser directly and
  `tests/integration/query.rs::filters_pages_using_unquoted_iso_date_literals`
  filters in-process. Neither twin runs a `+02:00` note value against a
  `Z` literal through a spawned process. Mutation compare not done, so
  the deletion is deferred.
- **Successor note for arch-design:** The scenario is the offset-to-UTC
  ingestion contract at the CLI boundary. Parser shapes stay
  unit-pinned.

### Row 2: File-Sourced Select Field in a Spawned Template Render (conversion pending)

- **Namespaced row IDs:** 04-B3.
- **Behavior:** A `select` field whose values come from a
  `values/*.toml` file resolves `{value, label}` objects through the
  template `schema` namespace and renders the label and value.
- **Former test (planned):** `traces::e2e
  template::renders_file_sourced_select_field_in_e2e_template`.
- **Former layer:** `e2e`.
- **Defect caught:** The values file could fail to confine, parse, or
  expose `value`/`label` through `schema.get(...).field(...)`.
- **Survivor:** `A template reads the labels and values of a file-sourced
  select field` in `tests/features/schema-temporal.feature`.
- **Proof status:** Kill demonstration done (above). Assertion
  comparison done. The old test asserted success and that the written
  `topic_note.md` equals `Category: Rust Programming (rust)`. The
  scenario states the same Given, command, exit code 0. It asserts the
  written file content exactly through `And file "topic_note.md"
  contains:` (equality, not substring). Twin re-read done:
  `tests/integration/template_render.rs::renders_a_file_sourced_select_field_in_template_rendering`
  pins the same fixture and rendered string in-process.
  `src/schema/fields/select.rs` unit tests keep the confinement pins.
  Mutation compare not done, so the deletion is deferred.
- **Successor note for arch-design:** The scenario is the CLI template
  contract for file-sourced select fields. In-process pins stay.

### Slice 6 Name Inventory

Not recorded for deletion because no test was deleted.
`cargo nextest list --all-features --run-ignored all` was captured
before the planned deletion at
`/tmp/nextest-names-before-slice6.txt` (3978 names). A later conversion
run must diff this list after deleting the two tests.
