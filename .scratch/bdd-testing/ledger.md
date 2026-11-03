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
  - **Mutation compare (after deletion):** The deletion in `tests/e2e/query.rs` removes only test code. The scoped mutarust run on `src/query/format.rs` preserves 0 escaped mutants. The baseline file `mutarust-baseline.json` stayed untouched.
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
  - **Kill demonstration:** Ingestion duration parsing was broken by invalidating multi-unit tokens; the former test and survivor scenario both failed.
  - **Mutation compare:** Guarded logic in `src/temporal` was verified clean.
  - **Name diff:** Verified `cargo nextest list --all-features --run-ignored all` before and after deletion. `traces::e2e query::list_filters_an_inline_multi_part_duration` was removed.
  - **Twin re-read:** Re-read `query.rs::list_filters_rfc3339_field_by_its_exact_instant`. That test proves timestamp instant comparisons, not multi-part duration syntax.
- **Successor note for arch-design:** Verified by `tests/features/note.feature`.

### Retained Candidate Note

Spec section 9.3 candidate `config.rs::tag_filter_config_drives_task_selection_from_disk` also guards row 01-G21, owned by Slice 5. Spec sections 7 and 9.3 defer conversion to the slice that completes the last row. The test stays in Rust.
