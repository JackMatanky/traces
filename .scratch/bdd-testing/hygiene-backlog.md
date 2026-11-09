# Hygiene Backlog

This backlog lists test-hygiene and foundation items that sit outside the BDD
adoption slices. The user owns this backlog. It serves spec section 9.8.

## Rules

Each test change in this backlog must use:

- The die rule from spec section 7.
- A ledger entry in `.scratch/bdd-testing/ledger.md` that records the proof and
  the surviving twin before any deletion.

## 1. Misplaced E2E Tests (5 items)

These tests live in `tests/support/fixtures.rs`. They test in-memory data
formatting and filesystem helpers without exercising the CLI binary. They belong
in unit tests:

1. `formats_minimal_configuration`: tests in-memory TOML formatting.
2. `formats_raw_configuration`: tests raw TOML string passthrough.
3. `formats_declared_sections`: tests section serialization.
4. `serializes_special_characters_in_structured_values`: tests TOML escaping.
5. `writes_config_file_to_disk`: tests local file write helpers.

## 2. Misplaced Integration Tests (10 items)

These tests live in `tests/integration/`. They test isolated component logic and
belong inline in unit tests under `src/`:

1. `query.rs::page_query_returns_real_indexed_notes`: single-component page query
   lookup. Twin lives in `src/query/service.rs`.
2. `query.rs::query_tasks_returns_task_level_rows_distinct_from_page_level_query`:
   task row flattening over memory notes. Twin lives in `src/query/service.rs`.
3. `query.rs::query_builder_reuses_one_index_for_page_and_task_queries`: index
   borrow reuse. Belongs in `src/query/service.rs`.
4. `query.rs::accepts_canonical_list_paths_and_rejects_obsolete_task_paths_with_diagnostic`:
   parser diagnostic check. Twin lives in `src/query/grammar/filter.rs`.
5. `query.rs::sorting_an_empty_query_set_returns_empty`: empty set sorting edge
   case. Belongs in `src/query/sort.rs`.
6. `query.rs::sorts_pages_descending_by_text_field`: sorter descending order.
   Twin lives in `src/query/sort.rs`.
7. `query.rs::filters_pages_using_unquoted_iso_date_literals`: grammar literal
   parsing. Twin lives in `src/query/grammar/filter.rs`.
8. `query.rs::filters_pages_handle_null_and_missing_temporal_operands`: null operand
   propagation. Twin lives in `src/query/grammar/filter.rs`.
9. `template.rs::template_shorthands_support_iso_offsets_and_reference_format`:
   template date filter formatting. Twin lives in `src/template/engine/date.rs`.
10. `template.rs::template_filters_support_bucketing_aliases_and_weekday_arg`:
    template bucketing aliases. Twin lives in `src/template/engine/date.rs`.

## 3. Weak Assertions (7 items)

These tests use loose checks that can hide regressions:

1. `tests/e2e/completions.rs::zsh_shell_prints_a_completion_script`: asserts
   only `contains("#compdef traces")`. Needs script structure validation.
2. `tests/e2e/init.rs::default_path_init_spawns_and_scaffolds_workspace`:
   asserts only `contains("initialised...")`. Needs exact scaffold verification.
3. `tests/e2e/query.rs` diagnostic tests (lines 100, 123, 193): assert loose
   substring matches. Need exact diagnostic code checks.
4. `tests/integration/query.rs::query_builder_reuses_one_index_for_page_and_task_queries`:
   asserts only row counts.
5. `tests/integration/query.rs::sorting_an_empty_query_set_returns_empty`:
   asserts only `assert!(result.is_empty())`.
6. `tests/integration/template.rs::template_weekday_rejects_out_of_range_day_index`:
   asserts only `is_err()` without checking the error variant.
7. `tests/integration/template.rs::template_shorthands_reject_malformed_iso_offsets`:
   asserts only `is_err()` without checking the error variant.

## 4. Foundation Gaps (Spec Section 9.9 Item 12)

These infrastructure items harden the repository build and release pipeline:

1. Pin GitHub Actions by full SHA instead of mutable tags in CI workflows.
2. Add Dependabot configuration for automated dependency updates.
3. Add `SECURITY.md` with vulnerability reporting instructions.
4. Add `CHANGELOG.md` alongside version automation from architecture ticket 38.
5. Add fuzzing harness for query and frontmatter parsers.
6. Add automated release workflow.
7. Add `cargo-hack` and `minimal-versions` checks in CI.

## 5. Scheduled Workflow Findings

The scheduled workflows `fuzz.yml`, `soak.yml`, `bench.yml` and `mutation.yml`
had never run before 2026-11-08. Each item follows the die rule from spec
section 7.

1. Fixed: Benchmark smoke overflowed the stack in a debug build. The lexer
   recursed once for each skipped match. The lexers now skip whole runs. The
   smoke run uses the bench profile.
2. Fixed: Fuzz found that `0004000milli` parsed as a duration. The unit list now
   rejects every sub-second spelling.
3. Fixed: all four scheduled workflows have now run once.
4. Open: a debug build still overflows the stack on about 5,000 short lines in
   one note.
5. Open: the mutation run with `-m index` failed. The cause is not known.
