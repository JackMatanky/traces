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

## Slice 5: Configuration, tracking and templates

### Row 1: Non-interactive initialization

- **Namespaced row IDs:** 01-G15
- **Behavior:** An `init` command on a non-interactive terminal creates the
  default local configuration and template directory.
- **Former test:** `e2e::init::default_path_init_spawns_and_scaffolds_workspace`
- **Former layer:** `e2e`
- **Defect caught:** Initialization can report success without creating the
  local configuration.
- **Survivor:** Scenario `Non-interactive initialization scaffolds the default
  configuration` in `tests/features/configuration.feature`.
- **Proof:**
  - **Assertion comparison:** The old test asserted success, a stderr
    fragment, the `.traces` directory and the config file. The survivor
    asserts exit zero and the complete stderr path line. It also asserts both
    paths and the default template directory in the config file. It asserts
    the default output directory too.
  - **Kill demonstration:** On the first commit, `Init::run` skipped
    `scaffold_directories` and `write_config_file` while returning success.
    The old test failed because `.traces` did not exist. The scenario failed
    at the same missing-directory assertion. The original production calls
    were restored before the deletion.
  - **Mutation compare:** Not required: the guarded dispatch code is in
    `src/cli/init.rs`. An exploratory scoped run `-f src/config/tracker.rs`
    escaped one `branch/if` mutant that no current test kills, including the
    retained Rust `tracked.rs` pair. That file keeps both of its tests, so the
    deletion rules of section 7 are not engaged. The escape is pre-existing and
    not created by this slice.
  - **Twin re-read:** The retained
    `e2e::workflow::init_trust_index_list_table_task_and_template_chain_through_one_project`
    checks a successful first-run chain. It does not assert the default
    scaffold files or the exact init status line. The `src/cli/init.rs` unit
    tests cover the default config and rollback separately.
  - **Name diff:** Verified
    `cargo nextest list --all-features --run-ignored all` before and after
    deletion. The run removed one test:
    `traces::e2e init::default_path_init_spawns_and_scaffolds_workspace`.
    No other name changed.
- **Successor note for arch-design:** The CLI scaffold contract lives in
  `tests/features/configuration.feature` under the scenario named above.

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

## Slice 4: Index Freshness

### Rows Covered

- **Namespaced row IDs:** 01-G29, 02-G13, 03-P01, 03-P02, 03-P03, 03-P05 (observable half), 03-P08, 03-P20, 03-P21, 03-P22, 03-P25; seams 2, 4, 5
- **Behavior:** Query execution observes filesystem mutations, path set changes, and config drift across spawned CLI invocations.
- **Former tests deleted:**
  1. `e2e::index::content_edit_between_spawns_is_query_visible`
  2. `e2e::index::path_set_changes_between_spawns_prune_ghosts_and_resolve_links` (needs-split)
  3. `e2e::index::newly_created_link_target_resolves_inlinks_after_spawned_refresh`
  4. `e2e::index::list_creates_an_index_store_without_an_explicit_index_command`
  5. `e2e::index::config_edit_heals_the_index_on_the_next_query`
  6. `e2e::query::inlinks_render_across_spawned_queries`
  7. `e2e::template::rendered_output_reenters_the_index`
- **Former layer:** `e2e`
- **Retained Rust tests (not deleted):**
  - `e2e::index::same_size_mtime_restored_edit_reads_the_store_not_a_rebuild` (timing canary; known limitation owned by arch-design ticket 25)
  - `e2e::index::fully_corrupted_index_self_heals_on_the_next_query` (corruption self-heal; out of candidate list)
- **Survivors:** Scenarios in `tests/features/index-freshness.feature`:
  - `An edited task appears in the next query` (replaces `content_edit_between_spawns_is_query_visible`)
  - `Deleting a sibling path resolves an ambiguous wikilink` (replaces deletion half of `path_set_changes_between_spawns_prune_ghosts_and_resolve_links`)
  - `Renaming a note removes the former path and resolves its link` (replaces rename half of `path_set_changes_between_spawns_prune_ghosts_and_resolve_links`)
  - `Adding a link target resolves its inlink on the next query` (replaces `newly_created_link_target_resolves_inlinks_after_spawned_refresh`)
  - `A list query creates the index store when none exists` (replaces `list_creates_an_index_store_without_an_explicit_index_command`)
  - `An index setting edit changes the next query without changing notes` (replaces `config_edit_heals_the_index_on_the_next_query`)
  - `Inlinks render the referencing note path across spawned queries` (replaces `inlinks_render_across_spawned_queries`)
  - `A rendered note appears in the next list query` (replaces `rendered_output_reenters_the_index`)
- Scenarios also cover 03-P20, 03-P21, 03-P22, and 03-P25.
- **Proof:**
  - **Assertion comparison:**
    - `content_edit_between_spawns_is_query_visible`: Asserted `task` output `- [ ] old task (a.md)\n` then `- [ ] new task (a.md)\n` and never exit 101. Survivor `An edited task appears in the next query` asserts exact exit codes 0 and exact stdout strings `- [ ] old task (a.md)\n` and `- [ ] new task (a.md)\n`.
    - `index.rs::path_set_changes_between_spawns_prune_ghosts_and_resolve_links` (needs-split): The old test asserted list and inlink rows across a deletion and a rename. Survivors `Deleting a sibling path resolves an ambiguous wikilink` and `Renaming a note removes the former path and resolves its link` assert the same exact outputs.
    - `newly_created_link_target_resolves_inlinks_after_spawned_refresh`: Asserted table inlinks contain `a.md`. Survivor `Adding a link target resolves its inlink on the next query` asserts table row for `new` contains inlink `a.md`.
    - `list_creates_an_index_store_without_an_explicit_index_command`: Asserted list success, stdout `- a.md\n`, and `.traces/index.redb` exists. Survivor `A list query creates the index store when none exists` asserts exit code 0, exact stdout `- a.md\n`, and index store file exists.
    - `config_edit_heals_the_index_on_the_next_query`: The old test asserted `@Book` before the edit and `@Novel` after the edit. Survivor `An index setting edit changes the next query without changing notes` asserts `- a.md\n` for `@Book`. After the edit and re-trust, it asserts `- a.md\n` for `@Novel` and empty stdout for `@Book`.
    - `inlinks_render_across_spawned_queries`: Asserted table inlink for foo contains `a.md`. Survivor `Inlinks render the referencing note path across spawned queries` asserts table row for `foo` contains inlink `a.md`.
    - `rendered_output_reenters_the_index`: Asserted template run success, list by `#fromtemplate` outputs `- daily.md\n`. Survivor `A rendered note appears in the next list query` asserts exit code 0 for template execution and exact stdout `- daily.md\n` for list.
  - **Kill demonstration:** Production code in `src/index/fingerprint.rs::drift` was modified to return `None`. The former test and the survivor scenario both failed against the broken code. Restoration of the code made both tests pass.
  - **Mutation compare (before deletion):** Scoped run `mise run test:mutants -f src/index/fingerprint.rs --match drift --workers 1 --timeout 1200` generated 5 mutants. Result: 2 killed, 3 skipped, 0 escaped. Escape set: `[]`. Mutation score: 100.00%.
  - **Mutation compare (after deletion):** Scoped run `mise run test:mutants -f src/index/fingerprint.rs --match drift --workers 1 --timeout 1200` generated 5 mutants. Result: 2 killed, 3 skipped, 0 escaped. Escape set: `[]`. Mutation score: 100.00%. No new escape appeared. Baseline file `mutarust-baseline.json` stayed untouched.
  - **Open:** The other six deleted tests have no individual kill demonstration or mutation compare recorded. The grouped `fingerprint.rs::drift` demonstration cannot be shown to fail every one of them.
  - **Name diff:** Verified `cargo nextest list --all-features --run-ignored all` before and after deletion. Exactly 7 tests were removed:
    - `traces::e2e index::config_edit_heals_the_index_on_the_next_query`
    - `traces::e2e index::content_edit_between_spawns_is_query_visible`
    - `traces::e2e index::list_creates_an_index_store_without_an_explicit_index_command`
    - `traces::e2e index::newly_created_link_target_resolves_inlinks_after_spawned_refresh`
    - `traces::e2e index::path_set_changes_between_spawns_prune_ghosts_and_resolve_links`
    - `traces::e2e query::inlinks_render_across_spawned_queries`
    - `traces::e2e template::rendered_output_reenters_the_index`
  - **Twin re-read:**
    - Re-read all tests in `tests/e2e/index.rs`, `tests/e2e/query.rs`, `tests/e2e/template.rs`, `tests/integration/refresh.rs`, and `tests/integration/inlinks.rs`.
    - Integration tests in `tests/integration/refresh.rs` exercise in-process `IndexerService` APIs directly and remain in place.
    - `tests/e2e/index.rs::same_size_mtime_restored_edit_reads_the_store_not_a_rebuild` remains in place as the arch ticket 25 timing canary.
    - No twin duplicate remains across e2e or integration suites for the deleted process-level freshness and drift checks.
- **Successor note for arch-design:** Scenarios in `tests/features/index-freshness.feature` check index freshness, link updates, and drift recovery.

## Slice 8: Spawned-Signal Exit 130 (Dropped)

No scenario was written and no test was deleted. The user decided
(2026-11-03) to drop the slice before the signal dev-dependency landed.

### Spike Results (Entry Condition 9.7)

- **Platform gating.** `scenarios!` generates one test for every feature file
  in the directory it scans, and a missing step fails at run time with
  `Step not found` (spec section 2). Cfg-gating the step definitions alone
  would leave Windows generating a test whose steps do not exist. The clean
  gate is a second `#[cfg(unix)] scenarios!("tests/features/unix", ...)`
  call over a separate `tests/features/unix/` directory. The `check:layers`
  feature-text checks and the sixth gate check scan `tests/features/`
  recursively, so the nested directory stays visible to them. Windows
  behavior is [INFERENCE]; CI was never run for this layout.
- **Exit 130 mechanics.** `src/main.rs` returns `ExitCode::from(130)` only
  for `CommandOutcome::Aborted(UserAbort::Interrupted)`. That outcome is
  produced only when a dialog error chain holds
  `DialogError::UserInterrupted`. `inquire` reports that error
  (`OperationInterrupted`) only when the prompt runs on a real TTY.
  `src/dialog/terminal.rs` guards every prompt with `stdin_is_tty` and
  answers from a fallback table otherwise. A piped-stdin child that receives
  SIGINT dies by signal, so `ExitStatus::code()` is `None`, and no assertion
  of a numeric 130 is possible without a pseudoterminal.
- **Readiness.** Deterministic prompt readiness (poll an observable, never
  `sleep`) requires reading the prompt output, which again needs a
  pseudoterminal. Spec section 4 bans a PTY harness.
- **Dependency.** `rustix` is a dev-dependency (1.1.5, `fs` feature). Its
  `process` feature would cover `kill`; its `pty` feature would cover the
  pseudoterminal. Both stay unadded.

### Decision

Known limitation. The 130 pin stays in the unit layer:
`src/main.rs::ctrl_c_abort_exits_130` (outcome to exit-code mapping) and
`src/cli/mod.rs::user_interrupted_during_template_picker_returns_aborted_interrupted`
(interrupt mapping through `CliError::user_abort`). Spec sections 9.3, 9.7,
9.9 item 13, 10 and 12 record the decision. Research/10 row 01-G1 notes the
130 part as pinned; the class-level map row names the same pin.

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
  stays in this run. Slice 5 has since covered 01-G21 (2026-11-03), so the
  die-rule blocker is gone. Its deletion still waits for a scoped mutation
  compare, like the other two tests. Spec section 7: a test whose rows span
  slices dies in the slice that completes its last row.
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

## Slice 7: Note-Model Rules Through Query Output

### Row 1: Task Custom Markers, Indentation Depth, and Clickable Coordinates

- **Namespaced row IDs:** 03-N04, 03-N09, 05-task
- **Behavior:** Preserve task status markers, outline indentation, nesting depth, and clickable source coordinates in task query output.
- **Former test:** `e2e::query::task_preserves_custom_markers_indents_nested_depth_and_formats_line_numbers`
- **Former layer:** `e2e`
- **Defect caught:** Task formatting drops non-standard markers or flattens hierarchy in spawned output.
- **Survivors:**
  - Scenario `Task output keeps all known and unknown markers at their nesting depth` (`tests/features/note.feature`)
  - Scenario `Task coordinates keep every known and unknown marker's source line` (`tests/features/note.feature`)
- **Proof:**
  - **Assertion comparison:** The former test asserted exact stdout strings for plain task listing and coordinate task listing. The survivor scenarios assert the identical exact stdout strings. The survivor assertions are equal in strength.
  - **Kill demonstration:** Production code in `src/query/format.rs` was broken by substituting empty task description text. The former test and both survivor scenarios failed with stdout diff mismatches. Restoring the code restored all tests to passing.
  - **Mutation compare (before deletion):** Scoped run `mise run test:mutants -f src/query/format.rs --exec-timeout 900` generated 60 mutants. Result: 11 killed, 0 escaped, 40 errored, 9 skipped. Mutation score: 100.00%. Escape set: `[]`.
  - **Mutation compare (after deletion):** No after-deletion escape-ID set or scoped run output is recorded. An after-run cannot be established from the deletion alone.
  - **Open:** No before/after escape-ID sets for `src/query/format.rs` were recorded. The earlier 0-escape result does not establish a deletion compare; no pair was rerun here.
  - **Name diff:** Verified `cargo nextest list --all-features --run-ignored all` before and after deletion. `traces::e2e query::task_preserves_custom_markers_indents_nested_depth_and_formats_line_numbers` was removed.
  - **Twin re-read:** Re-read `task_table_renders_default_and_custom_columns` and `task_count_prints_only_the_match_total` in `tests/e2e/query.rs`. Neither asserts hierarchical indentation or custom marker characters.
- **Successor note for arch-design:** Verified by `tests/features/note.feature`.

### Row 2: Task Filter Shortcuts Narrow Output Accurately

- **Namespaced row IDs:** 02-G5, 03-N10
- **Behavior:** Filter task output through command-line shortcuts `--todo`, `--done`, and `--status <char>`.
- **Former test:** `e2e::query::task_filter_shortcuts_narrow_output_accurately`
- **Former layer:** `e2e`
- **Defect caught:** Filter shortcuts omit matching tasks or include excluded tasks across process boundaries.
- **Survivors:**
  - Scenario `The todo shortcut includes every incomplete task` (`tests/features/note.feature`)
  - Scenario `The done shortcut includes only completed tasks` (`tests/features/note.feature`)
  - Scenario `The status shortcut matches the exact task marker` (`tests/features/note.feature`)
- **Proof:**
  - **Assertion comparison:** The former test used substring `contains` checks on stdout. The three key-example survivor scenarios assert exact stdout strings. The survivor assertions are strictly stronger.
  - **Kill demonstration:** Production code in `src/cli/task.rs` was broken by reversing the `--todo` filter expression. The former test and the survivor scenario failed. Restoring the code restored both tests.
  - **Mutation compare:** The guarded code lives in `src/cli/task.rs`. `mutarust.yml` excludes `src/cli`. The kill demonstration serves as primary proof per spec section 7.
  - **Name diff:** Verified `cargo nextest list --all-features --run-ignored all` before and after deletion. `traces::e2e query::task_filter_shortcuts_narrow_output_accurately` was removed.
  - **Twin re-read:** Re-read `src/cli/task.rs` unit tests for filter arguments. Those tests check CLI argument parsing only, not end-to-end task execution.
- **Successor note for arch-design:** Verified by key-example scenarios in `tests/features/note.feature`.

### Row 3: List Filters Multi-Part Inline Duration

- **Namespaced row IDs:** 03-N07, 04-B5
- **Behavior:** Ingest multi-part inline durations from note markdown and evaluate comparisons in query filters.
- **Former test:** `e2e::query::list_filters_an_inline_multi_part_duration`
- **Former layer:** `e2e`
- **Defect caught:** Ingestion treats multi-part duration syntax as unparsed text, breaking query equality comparisons.
- **Survivor:** Scenario `Inline durations compare by elapsed time` (`tests/features/note.feature`)
- **Proof:**
  - **Assertion comparison:** The former test asserted exit code 0 and stdout `- matching.md\n`. The survivor scenario asserts exit code 0 and the identical stdout `- matching.md\n`.
  - **Kill demonstration:** The slice 7 record says invalidating multi-unit duration tokens made both tests fail. It does not name a file or symbol. Commit `2ff6f32c` does not identify either.
  - **Mutation compare:** No before/after escape sets for `src/temporal` were recorded for this deletion. The claim that the guarded logic was clean has no recorded compare.
  - **Name diff:** Verified `cargo nextest list --all-features --run-ignored all` before and after deletion. `traces::e2e query::list_filters_an_inline_multi_part_duration` was removed.
  - **Twin re-read:** Re-read `query.rs::list_filters_rfc3339_field_by_its_exact_instant`. That test proves timestamp instant comparisons, not multi-part duration syntax.
- **Open:** The slice 7 mutation compare for `src/query/format.rs` and `src/temporal` is not recorded as a before/after pair.
- **Successor note for arch-design:** Verified by `tests/features/note.feature`.

### Retained slice 5 candidates

The three spec section 9.3 candidates `config.rs::templates_directory_config_drives_template_listing_from_disk`, `config.rs::global_templates_directory_changes_template_listing`, and `template.rs::writes_the_rendered_file_to_the_default_output_path` still exist in `tests/e2e/config.rs` and `tests/e2e/template.rs`. The ledger does not record an individual kill demonstration or before/after mutation compare for any of the three. Their deletion proof is incomplete, so they remain Rust tests.

### Retained Candidate Note

Spec section 9.3 candidate `config.rs::tag_filter_config_drives_task_selection_from_disk` also guards row 01-G21, owned by Slice 5. Spec sections 7 and 9.3 defer conversion to the slice that completes the last row. The test stays in Rust.

## Deletion Wave (slices 5, 6 and 7 leftovers)

Date: 2026-11-04. This wave finishes the deletions that earlier slices
deferred. The proof of spec section 7 applies to each test. Kill
demonstrations ran on a commit before the deletion commit. The name list
before and after the deletion shows exactly the four names in Rows 1 to 4 removed.
The two `tracked.rs` tests are not removed (see Retained Tests).

### Mutation Compare

Scoped runs used `-f <file>`, `--timeout 1200`, `--workers 2` and
`--no-diffs`. The before-run used the tree before the deletion. The after-run
used the tree after the deletion. `mutarust-baseline.json` stayed untouched.

| File | Before | After | Result |
|---|---|---|---|
| `src/temporal/datetime.rs` | 89 killed, 49 escaped, 27 skipped | 89 killed, 49 escaped, 27 skipped | Same escape IDs. No new escape. |
| `src/template/engine/schema.rs` | 14 killed, 2 escaped, 9 skipped | 14 killed, 2 escaped, 9 skipped | Same escape IDs. No new escape. |
| `src/config/model/tasks.rs` | 10 killed, 3 escaped, 6 skipped | 10 killed, 3 escaped, 6 skipped | Same escape IDs. No new escape. |
| `src/config/model/schemas.rs` | 3 killed, 0 escaped | 3 killed, 0 escaped | No escape. |
| `src/config/tracker.rs` | 1 killed, 0 escaped | 0 killed, 1 escaped | New escape: mutant `1bc082ba551c7449b7fa4cea638f0ec3` (`branch/if`). The proof fails. |
The IDs in `/tmp/mut-before-A.log` and `/tmp/mut-after-A.log` are equal as sets (49 IDs). The IDs in `/tmp/mut-before-B.log` and `/tmp/mut-after-B.log` are equal as sets (2 IDs). The IDs in `/tmp/mut-before-C.log` and `/tmp/mut-after-C.log` are equal as sets (3 IDs). A is `src/temporal/datetime.rs`, B is `src/template/engine/schema.rs`, and C is `src/config/model/tasks.rs`. Each count and ID-set equality was read directly from the named logs.

The `tracker.rs` run executed after the deletion of the two `tracked.rs`
tests. Mutant `1bc082ba...` was killed before and escaped after. No scenario
kills it. The two `tracked.rs` tests stay (see Retained Tests).

### Row 1: Configured Task Tag Filters From Disk

- **Namespaced row IDs:** 01-G21, 03-N10.
- **Behavior:** `[tasks] tag_filters` in the on-disk config selects only the
  tagged checklist items as tasks.
- **Former test:** `e2e::config::tag_filter_config_drives_task_selection_from_disk`.
- **Former layer:** `e2e`.
- **Defect caught:** The configured tag filter is not applied, so an untagged
  checklist item reads as a task.
- **Survivor:** `Configured tag filters exclude untagged checklist items from tasks`
  in `tests/features/note.feature`. Slice 5 covers 01-G21 with
  `Local and global configuration fields combine independently`.
- **Proof:** Assertion comparison: the old test asserted exit 0 and the exact
  stdout `- [ ] one (a.md)\n`. The scenario asserts exit 0 and the same exact
  stdout, with an extra `#todo/active` note. Kill demonstration:
  `TaskConfig::tag_filters` returned an empty list. The old test and the
  scenario both failed. Mutation compare: `src/config/model/tasks.rs`, no new
  escape. Twin re-read: the parser unit tests pin the filter in process and
  never read a config file from disk.
- **Successor note for arch-design:** None.

### Row 2: Configured File Class Field

- **Namespaced row IDs:** 04-B2, 01-G21.
- **Behavior:** `[schemas] class_field` in the on-disk config selects the
  notes that carry that field.
- **Former test:** `e2e::config::class_field_config_drives_class_selection_from_disk`.
- **Former layer:** `e2e`.
- **Defect caught:** The class field name is ignored, so `@Novel` matches
  nothing.
- **Survivors:** `A configured file class field selects the matching note` and
  `A configured file class field does not invent another class` in
  `tests/features/schema-temporal.feature`.
- **Proof:** Assertion comparison: the old test asserted exact stdout
  `- a.md\n` for `@Novel` and empty stdout for `@Book`. The two scenarios
  assert the same, as separate key examples. Kill demonstration:
  `class_field_name()` returned a fixed `"hardcoded_class"`. The old test and
  the scenario both failed. Mutation compare: `src/config/model/schemas.rs`,
  3 killed before and after. Twin re-read: slice 6 section above.
- **Successor note for arch-design:** None.

### Row 3: Exact Instant Filtering Through Ingestion

- **Namespaced row IDs:** 04-T2, 04-B5.
- **Behavior:** An RFC 3339 value with a `+02:00` offset matches a
  `Z`-spelled filter literal to the nanosecond.
- **Former test:** `e2e::query::list_filters_rfc3339_field_by_its_exact_instant`.
- **Former layer:** `e2e`.
- **Defect caught:** Offset normalization or nanosecond precision is lost
  between ingestion and filter comparison.
- **Survivor:** `An offset-dated note matches the same instant to a nanosecond`
  in `tests/features/schema-temporal.feature`.
- **Proof:** Assertion comparison and twin re-read: slice 6 section above,
  Row 1. Kill demonstration: `parse_any` used `Offset::UTC`. Both failed.
  Mutation compare: `src/temporal/datetime.rs`, same 49 escapes before and
  after.
- **Successor note for arch-design:** None.

### Row 4: File-Sourced Select Field in a Spawned Template Render

- **Namespaced row IDs:** 04-B3.
- **Behavior:** A `select` field with values from a file exposes `value` and
  `label` to the template.
- **Former test:** `e2e::template::renders_file_sourced_select_field_in_e2e_template`.
- **Former layer:** `e2e`.
- **Defect caught:** The values file is not read or the label is lost.
- **Survivor:** `A template reads the labels and values of a file-sourced select field`
  in `tests/features/schema-temporal.feature`.
- **Proof:** Assertion comparison and twin re-read: slice 6 section above,
  Row 2. Kill demonstration: `select_entry_value` inserted `unmatched_label`
  instead of `label`. Both failed. Mutation compare:
  `src/template/engine/schema.rs`, same 2 escapes before and after.
- **Successor note for arch-design:** None.

### Retained Tests

- `e2e::tracked::list_prints_every_tracked_config_path` and
  `e2e::tracked::clean_removes_a_stale_tracked_entry_and_reports_the_count`
  are restored and stay. The kill demonstration passed: breaking
  `TrackerStore::list` or `TrackerStore::clean` fails the old test and the
  scenario. The mutation compare did not pass. After the deletion, mutant
  `1bc082ba551c7449b7fa4cea638f0ec3` in `src/config/tracker.rs` escaped. The
  scenario Givens run `trust`, which already records the path, so no scenario
  pins that `index` records the configuration as a tracked path. Spec
  section 7 says a new escape fails the proof. Rows 03-P25 and
  05-file_tracker are `pinned-elsewhere` in research/10. A later change can
  add a scenario that kills the mutant. Then the two tests can die.
