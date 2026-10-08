# 15: Recompute duplication analysis + behavior ledger

**What to build:** After all gap tests land, the duplication analysis is re-run against the
post-gap inventory with the audit's original standard, and every proposed deletion carries a
ledger entry naming its behavior, former layer, defect class, and surviving test. This gate
exists so consolidation can never mask a freshly filled gap; test counts are bookkeeping, not
a criterion.

**Category:** enhancement

**Blocked by:** 04 (the investigation may add a regression test that changes the inventory),
10, 11, 12, 13, 14 (every test-adding gap ticket — the analysis runs over the enlarged suite;
09 reaches it via 11).

**Status:** resolved

- [x] Duplication standard applied unchanged: same layer + same boundary + same observable =
      duplication; vertical redundancy proving a new failure boundary (exit code, stream split,
      cross-process state) = keep
- [x] Post-gap inventory explicitly re-evaluates the known borderline sets against their new
      rivals: the persistence roundtrip pair, the E2E dispatch persistence tests, and the
      task-filter file
- [x] Every proposed deletion gets a ledger row: behavior → former layer → defect class →
      surviving test (named)
- [x] Test-count delta recorded as bookkeeping only; no deletion approved by count
- [x] The golden-path test evaluated but NOT deleted (its replacements — cross-process index
      read, edit-between-spawns — must exist and be identified by number in the ledger before
      any future deletion ticket)
- [x] Ledger incorporates the dispositions already recorded by earlier tickets: 11's unit-owned
      config-stage mapping and 13's schema-field replacement mapping (both promised to land
      here; they are resolved facts, not proposed deletions)
- [x] Output reviewed before ticket 16 executes — this ticket's own gate: the spec mandates the
      recompute and the ledger, but the human checkpoint before mass deletion is **this
      ticket's addition** (proposed here; the maintainer signs off before 16 starts)

## Comments

Resolved 2026-10-09. Recompute run over the post-gap inventory
(lib 3,151 / integration 58 / e2e 61 / doctests 83 — bookkeeping only;
the ticket-03 pins were lib 3,147 / 45 / 27 / 83). No deletion is approved
by count anywhere below.

### Recomputed D1–D12 dispositions (standard applied unchanged)

| ID | Post-gap disposition | Ledger row (behavior → former layer → defect class → surviving test) |
|---|---|---|
| D1 | **Delete** the `TestProject`-based twin | trust round-trip via fixture service → integration (`config_lifecycle.rs::test_project_manages_trust_and_untrust_lifecycle`) → consolidation-masking-gap → `config_lifecycle.rs::trust_then_untrust_round_trips_through_the_public_service_surface` |
| D2 | **Delete** (audit disposition re-verified: unit asserts more) | persist→load identity (counts+paths) → integration (`index_persistence_roundtrip.rs::persist_then_load_recovers_the_same_file_count_and_paths`) → consolidation-masking-gap → in-crate `src/index/service.rs::persistence` roundtrip units + kept `preserves_query_rows_and_list_metadata_across_cold_reload` (unique: re-query after sources deleted). Ticket 12's delta test does not subsume it (different act: edit+refresh) |
| D3 | **Keep**, merge assertions (consolidation refactor for ticket 16, not a deletion) | tasks-vs-lists differential proven twice → integration → consolidation-masking-gap → `index_query.rs` differential owner after merge |
| D4 | **Keep both** (revised from audit) | file-sourced select field → integration keeps row-shape reference; the E2E twin proves schema values-file loading through the spawned child parse (process boundary = new failure boundary). The audit's `--output` repoint concern is moot: ticket 14 now spawns `-o` separately |
| D5 | **Collapse** checkbox variants into the D8 model | task checkbox rendering → E2E (`dispatch.rs::task_prints_a_checkbox_line_per_task`) → consolidation-masking-gap → `dispatch.rs::task_preserves_custom_markers_indents_nested_depth_and_formats_line_numbers` (exact-stdout model) + in-crate render units |
| D6 | **Delete** | task sort ordering via argv → E2E (`dispatch.rs::task_sorting_orders_output_with_asc_and_desc`) → consolidation-masking-gap → in-crate sort units (`SortOrder::parse` + query sort tests); clap conflict wiring stays covered by `unknown_sort_field_reports_a_did_you_mean_suggestion` |
| D7 | **Delete** the `--column` twin | same diagnostic shape twice → E2E (`dispatch.rs::task_table_invalid_column_reports_its_location_and_repair`) → consolidation-masking-gap → `dispatch.rs::task_invalid_where_reports_its_location_and_repair`. Ticket 10's argv-exit-2 table coverage is a different observable and stays |
| D8 | **Keep** (the model — one exact-stdout contract) | — |
| D9 | **Delete** bash + fish twins | shell completion markers → E2E (`dispatch.rs::bash_shell_prints_a_completion_script`, `::fish_shell_prints_a_completion_script`) → consolidation-masking-gap → `dispatch.rs::zsh_shell_prints_a_completion_script` + in-crate shell marker units |
| D10 | **Delete** the remaining integration twin (file goes with D1) | trust/untrust lifecycle at the weakest layer → integration (`config_lifecycle.rs::trust_then_untrust_round_trips_through_the_public_service_surface`) → consolidation-masking-gap → `tests/e2e/process_contract.rs` trust stream contracts + `tests/e2e/untrust.rs` + `tests/e2e/env_policy.rs::state_stores_materialize_inside_the_sandbox_state_accessor` |
| D11 | **Keep both** (audit); ticket 11 adds the stale flavor — third boundary, keep | — |
| D12 | **Partial collapse**: delete `trust_then_index_persists_the_file_index` (subsumed by `list_persists_the_file_index_without_an_explicit_index_command`, the stronger no-explicit-index claim); keep the three stdout/stderr-split representatives (`list_prints…`, `table_renders…`, `task_preserves_custom_markers…` = D8) and the unique-boundary tests (`unknown_sort_field…`, `invalid_filter_expression…`, `task_from_nonexistent…`, `dry_run_prints…`, `render_error…`) | index-file creation via explicit index → E2E (`dispatch.rs::trust_then_index_persists_the_file_index`) → consolidation-masking-gap → `dispatch.rs::list_persists_the_file_index_without_an_explicit_index_command` |

**Proposed deletion total: 9 tests + 1 empty file** (`config_lifecycle.rs`)
— consistent with the audit's "≈10 E2E + ≈3 integration" net.

### New-ticket overlap review (all KEEP, recorded for honesty)

- `env_policy::global_config_resolves_from_the_sandbox_config_home_accessor`
  vs `config_capability::malformed_global_config_fails_parse_for_trusted_
  local`: same layer + observable, **different defect classes**
  (assumed-hermeticity plumbing-break detector vs trust-before-parse stage
  leg). Standard's vertical-redundancy clause applies; keep both.
- Ticket 12 `content_edit_between_spawns` vs ticket 14
  `rendered_output_reenters_the_index`: different acts (edit existing vs
  template writer feedback loop). Keep.
- Ticket 10 process contracts vs dispatch diagnostics: numeric/exit/stream
  boundary vs diagnostic-text boundary. Keep.
- Ticket 04's freshness regression (in-crate) vs ticket 12's integration
  delta test: different layers and acts (Fresh-arm contract vs refresh
  delta). Keep.

### Golden path: evaluated, NOT deleted

`golden_path.rs` stays. Its replacements, identified for any future deletion
ticket: ticket 08 (spawned init: `init.rs::default_path_init_spawns_and_
scaffolds_workspace`), ticket 10 (exit codes + stream splits:
`process_contract.rs`), ticket 12 (edit-between-spawns + ghost pruning:
`index.rs`; cold inlinks: `query.rs`), ticket 14 (template write family:
`template_write.rs`).

### Resolved dispositions incorporated (facts, not proposals)

- **Ticket 11** unit-owned config-stage mapping: MissingBaseline
  (`src/config/file.rs` + `service.rs` trust_status units), merge
  (`src/config/builder.rs` merge units), field-key validation
  (`src/config/service.rs` InvalidFieldKey units) — stay in-crate.
- **Ticket 13** schema-field replacement mapping: deleted
  `schema_field_resolution.rs` → template-class expansion through
  `TemplateService` + in-crate `src/schema/graph/*` and service/builder
  twins.
- **Ticket 04** ruling: fresh-refresh regression pinned in-crate
  (`fresh_refresh_serves_persisted_class_axes_unchanged…`); staleness E2E
  recipe forbidden as assertion everywhere (incl. ticket 12).
- **Ticket 12** corruption ruling: table-level self-heal asserted at process
  level; header-destruction boundary documented, not pinned (wipe-and-
  recreate candidate recorded for a future product spec).

### ⛔ Human checkpoint (this ticket's gate)

The maintainer signs off on this ledger before ticket 16 executes. Ticket 16
may prune only rows marked **Delete** above; **Keep/Collapse** rows are
refactors at most.

**Sign-off (2026-10-09):** maintainer approved the ledger as written. Ticket
16 is mandated to delete exactly the 9 proposed tests plus the empty
`config_lifecycle.rs` file (D1, D2, D5-collapse, D6, D7, D9, D10, D12-
partial), each with its ledger-named survivor in place. Keep/Collapse rows:
refactors only, no deletions.

**Evidence:** audit §9 (duplication table D1–D12), §14 P2 (recompute note); spec §Consolidation.
**Defect class:** consolidation-masking-gap (the failure mode this gate prevents).
