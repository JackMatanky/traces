# 02: Honesty — fix self-validating assertions & overclaiming docs

**What to build:** Every claim the suite makes matches what it executes. The store-edit test
fails if the command under test does nothing, and no doc comment asserts a boundary the test
doesn't run. Everything after this ticket builds on true claims.

**Blocked by:** 01 (visibility baselines — every code-changing ticket diffs against them).

**Category:** bug

**Status:** resolved

- [x] `src/cli/mod.rs::query_workflows::table_reflects_a_note_edit…` asserts through a
      non-refreshing load (`IndexerService::load()` — already exported, precedent in
      `index_persistence_roundtrip`), not `refresh_store()`; a no-op `Cli::run` now fails it
- [x] Same-shape sibling `indexing_then_page_and_task_queries…` either fixed the same way or
      its doc scoped to what it actually runs
- [x] All audit §11 **test-doc** rows rewritten to executed behavior (all four dispatch rows —
      persistence ×2, trust-coverage claim, exit-code claim — plus `golden_path` ×3: spawns
      count, index-read claim, trust-step claim; `support.rs` parallelism sentence + stale
      `src/cwd.rs` path; `create_trusted_project` templates-dir doc divergence — **doc truth
      only** here, ticket 17 re-verifies after its fixture reshape; `index_persistence_roundtrip`
      stale promotion pointer — §11's final `.mise` task-doc row is ticket 18's; AC1 resolves
      §11's `src/cli/mod.rs:1278` caveat)
- [x] No behavior changed in any test other than the two assertion fixes (this is a truth pass,
      not a refactor)
- [x] `mise run test` green and **no test renamed or removed** (verify by diffing the test-name
      list before/after — counts are bookkeeping, not a gate, per spec §Consolidation)

## Comments

Resolved in commit `1d66d93c` (merged in `integration-e2e-suite`).

- `table_reflects_a_note_edit_made_between_two_cli_invocations_with_no_explicit_index_command` uses `IndexerService::load()` instead of `refresh_store()`; verified that a no-op second `Cli::run` causes the test to fail.
- `indexing_then_page_and_task_queries_observe_the_same_project_state` updated to use `IndexerService::load()` on all three assertions and doc updated.
- All audit §11 doc claims corrected in `dispatch.rs`, `golden_path.rs`, `support.rs`, `src/lib.rs`, and `index_persistence_roundtrip.rs`.
- Test inventory list diff confirmed completely empty (0 tests added, 0 tests removed, 0 tests renamed).

**Evidence:** audit §11 table; §14 P0 items 1–2; test counts 3,127 + 76 at audit HEAD as
context only. Defect examples:
`src/cli/mod.rs::query_workflows::table_reflects_a_note_edit…` (assertion path re-syncs from
disk), `tests/e2e/dispatch.rs:17,:38` (file-existence only).
**Defect class:** self-validating assertion; claim/execution mismatch.
