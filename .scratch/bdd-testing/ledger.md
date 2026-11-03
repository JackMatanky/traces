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
