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
