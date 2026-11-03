# Migration Ledger

This ledger records deleted tests and their replacement proofs.
The ledger is append-only.
Each slice adds one section.

## Slice 1: Trust Lifecycle Pilot

### Row 1: Revocation Across Process Boundaries

- **Namespaced row IDs:** 01-G4, 01-G23, 01-G24, 05-G3
- **Behavior:** Revoke project trust durably so that a later run fails with `traces::cli::config_build_untrusted`.
- **Former test:** `e2e::trust::untrust_then_list_fails_with_the_untrusted_diagnostic`
- **Former layer:** `e2e`
- **Defect caught:** Untrust fails to revoke trust durably across process boundaries, which lets subsequent commands execute with untrusted configuration.
- **Survivor:** Scenario `Untrust revokes trust for later runs` (`tests/features/trust.feature`)
- **Proof:**
  - **Assertion comparison:** The survivor scenario S2 asserts at least what the old test asserted. The old test asserted `!list.is_success()` and substring `contains("traces::cli::config_build_untrusted")`, with unasserted stdout. Scenario S2 asserts exact exit code `1`, exact first line of stderr `traces::cli::config_build_untrusted`, and exact empty stdout `""`. S2 is strictly stronger.
  - **Kill demonstration:** Production code in `src/cli/untrust.rs` was modified so `untrust` omitted the call to `service.untrust(&subject)`, keeping the trust grant in the store. Both the old test and scenario S2 failed when executed against the broken binary. The old test failed with `assertion failed: !list.is_success()`. Scenario S2 failed at `Then the exit code is 1` with `expected exit code 1, got Some(0)`. Reverting the modification restored both tests to passing.
  - **Mutation compare (before deletion):** Scoped run `mise run test:mutants -f src/config/trust.rs` generated 20 mutants. Result: 4 killed, 16 skipped, 0 escaped. Escape set: `[]`.
  - **Twin re-read:** Re-read `tests/e2e/trust.rs::trust_all_and_untrust_all_count_asserted_via_trust_list`. That test verifies the batch `--all` stream and confirms `trust list` reports empty stdout. It does not test the default single-project CLI round trip to a failing command. Re-read all tests touching `untrust` in `tests/e2e/workflow.rs`, `tests/e2e/config.rs`, `tests/e2e/process_contract.rs`, `src/cli/untrust.rs`, `src/config/service.rs`, and `src/config/trust.rs`. None is a duplicate twin of the cross-process CLI revocation-then-list guarantee.
- **Successor note for arch-design:** Trust revocation CLI behavior is verified through BDD feature `tests/features/trust.feature` (`Untrust revokes trust for later runs`). Step definitions live in `tests/e2e/bdd/trust.rs`.
- **Evidence note (mutarust behavior on test-only diff):** Executed `mise run test:mutants --git-diff --dry-run` with a diff limited to `tests/e2e/trust.rs`. The task generated 0 mutants (`Total: 0 mutation(s) would be generated`). Labelled as evidence only; never used as proof.
