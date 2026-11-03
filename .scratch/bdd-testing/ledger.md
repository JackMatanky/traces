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
