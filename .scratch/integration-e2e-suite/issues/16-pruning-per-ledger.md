# 16: Pruning per ledger

**What to build:** The proven duplicates come out — each deletion executed only because its
ledger row names the surviving test. The suite shrinks by roughly 2% while gaining the stronger
gap tests; the golden path survives; the suite stays green.

**Blocked by:** 15 (the recomputed ledger is the authorization list).

**Category:** enhancement

**Status:** resolved

- [x] Byte-equivalent config pair deduplicated: one surviving test plus the ~5-line
      public-`untrust` smoke retained (the only external caller of the pub `untrust` surface);
      the in-crate fixture duplicate
      `src/lib.rs::tests::manages_trust_and_untrust_lifecycle` deleted per §14 P2.21; the
      surviving integration test(s) flagged for relocation — the spec puts relocation at the
      naming step, so the move itself happens in ticket 19
- [x] Config trust/lifecycle integration **files eliminated** — neither survives in §12's
      target tree; unit + E2E coverage confirmed by ledger rows; the surviving test's final
      home is named by ticket 19's tree instantiation (19 carries the mirroring AC)
- [x] Verified duplicates removed per ledger rows, honoring §9's dispositions faithfully where
      they are keep-side: D2 delete the integration subset but **keep** the unique re-query
      test; D3 **merge assertions, don't delete**; D4 keep the integration test, repoint or
      drop its E2E twin; D5 keep one E2E stdout representative (D6 drops); (D7, D9, D12 live
      in the E2E-collapse AC below — no double-listing)
- [x] E2E collapses: query commands 9→3, completions 3→1, duplicate diagnostics twin removed
- [x] Near-vacuous `tracked clean` assertion strengthened from a `stderr.contains('1')` trivia
      check to a structured message check (§14 P2.24 / §15 T4.6 — assertion fix, not a
      deletion; lands here because it is pruning-phase work per the plan)
- [x] `task_classification` integration file **deleted as redundant** (all three tests'
      assertions unit-twin'd; the disk-config gap moved to E2E in ticket 11) — fold into the
      query composition file ONLY if the ledger invokes the spec §Consolidation
      task-classification conditional (E2E config test failed to land AND a surviving
      composition test keeps a non-twin'd failure boundary; audit §15 T7.4 / §12 R4 origin)
- [x] Visibility freeze: production and test-utils public-api diffs empty; textual pub-token
      diff empty (the test deletion and param drops are shrink-only, rule 5)
- [x] ~74 dead tempdir fixture parameters removed (the `_temp` parameter is ignored by the
      helper; callers create unread directories)
- [x] `golden_path` NOT deleted; ledger row records the deferral and its replacement tests
- [x] `mise run test` green after pruning; ledger finalized with before/after dispositions

## Comments

Resolved in commit `bc196cdc` (merged into `integration-e2e-suite`), under
the ticket 15 maintainer sign-off (approved ledger as written).

### Executed deletions (each ledger-named survivor verified in place)

- D1+D10: `tests/integration/config_lifecycle.rs` deleted (both tests);
  survivors: e2e `untrust.rs` (the pub-`untrust` external smoke),
  `process_contract.rs` trust streams, `env_policy.rs` state-store test.
  The AC's "relocation" clause is superseded by the approved D10 deletion —
  ticket 19 may re-home a lifecycle test if its mirroring AC demands one.
- D2: `persist_then_load_recovers_the_same_file_count_and_paths` deleted;
  survivors: in-crate persistence roundtrip units + kept
  `preserves_query_rows_and_list_metadata_across_cold_reload`.
- D5: `task_prints_a_checkbox_line_per_task` deleted; a plain todo/done
  checkbox pair folded into the D8 exact-stdout model
  (`task_preserves_custom_markers…`).
- D6: `task_sorting_orders_output_with_asc_and_desc` deleted; survivor:
  in-crate sort units + `unknown_sort_field…` clap wiring.
- D7: `task_table_invalid_column_reports_its_location_and_repair` deleted;
  survivor: `task_invalid_where_reports_its_location_and_repair`.
- D9: bash + fish completions tests deleted; survivor: zsh representative +
  in-crate shell marker units.
- D12: `trust_then_index_persists_the_file_index` deleted; survivor:
  `list_persists_the_file_index_without_an_explicit_index_command`
  (doc notes the subsumption).
- `task_tag_filters.rs` deleted (3 tests, unit-twin'd in
  `src/note/parser.rs`, `src/query/service.rs`, `src/query/results.rs`,
  `src/template/engine/query.rs`); the §Consolidation conditional was NOT
  invoked (ticket 11's E2E disk-config test landed), so no fold-in.

### Assertion fix (P2.24)

`tracked.rs::clean_removes_a_stale_tracked_entry_and_reports_the_count`:
`stderr.contains('1')` → exact structured message
`removed 1 stale tracked entry\n` (empirically confirmed singular form).

### Dead tempdir sweep

`rows_for_files`/`rows_for`/`rows_of_three`/`rated_rows` dropped their dead
`temp` parameters; 82 dead caller tempdir creations removed (results.rs 47,
filter.rs 34, sort.rs 7 — the audit counted ~74 pre-ticket-13; integration
deletions shifted the exact count); stale "writes files under temp" docs
corrected to the in-memory reality; two dead `Path` imports dropped. Live
temps (`list_row`/`fs::write`/`IndexerService` callers) untouched.

### D3/D4 dispositions

- D3: the tasks-vs-lists differential double-proved at integration — one
  owner deleted with `task_tag_filters.rs`; the surviving differential lives
  in `index_query.rs` (parity + temporal filter coverage). No deletion
  beyond the file.
- D4: ledger-keep honored — the E2E select-field twin stays (child-parse
  boundary); `--output` argv coverage now exists via ticket 14's spawn.

### P2.21 note

`schema::descendants` unit dup (P2.21) is moot: ticket 13 already deleted
its integration twin, so no duplication remains.

### Sanctioned baseline delta (rule 5)

`tests/baselines/pub-tokens.txt`: 2 lines — the test-support helper
signatures after parameter drops (shrink-only). Production and test-utils
public-api baselines: diff empty (verified).

### After-state counts (bookkeeping)

lib 3,150 / integration 55 / e2e 55 / doctests 83 (CI pin unchanged);
before: lib 3,151 / integration 58 / e2e 61. `mise run test` green;
clippy `-D warnings`; fmt; `check:layers`; `check:visibility` all green.

**Evidence:** audit §5 item 2 (byte-equivalent pair + `src/lib.rs:918` dup), §9 table
(dispositions as corrected above), §7 dead `_temp` fixture pattern (45+22+7 = 74 sites),
§12 (config files eliminated from the target tree; task-classification T7.4 conditional),
§14 P2.21/P2.23/P2.24, §15 T4 (plan);
spec §Consolidation (delete-as-redundant default supersedes the audit's fold-default).
**Defect class:** layer-redundancy (same observable owned at multiple layers).
