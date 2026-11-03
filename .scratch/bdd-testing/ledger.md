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
