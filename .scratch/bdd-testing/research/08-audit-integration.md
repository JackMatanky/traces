# Research 08: Integration Layer and Test-Utils Facade Audit

This audit examines every test in `tests/integration/*.rs` (persistence, query,
refresh, template_render). It also examines the entry point
`tests/integration.rs` and the `traces::testing` facade (`src/lib.rs:178-212`)
gated by `feature = "test-utils"`. ## 1. Facade Measurement and Usage

- `tests/baselines/public-api-test-utils.txt` declares **47 symbols** under
  `traces::testing::`. `src/lib.rs:182` states "exactly the 46 identifiers".
  That line lags the actual count by 1 after `RefreshReport` became an enum with
  `DriftCause`.
- Breakdown: **32 production types** and **15 test-support helpers /
  constants**.
- **Facade symbols used across `tests/integration/`**:
  - `persistence.rs`: `FileEntry`, `QueryBuilder`, `QueryService`, `QuerySet`,
    `SourceSelector`, `TestProject` (plus root re-exports `DateValue`,
    `SourceLine`, `TaskDateType`, `TaskListItem`, `TaskPriority`,
    `TaskStatusType`).
  - `query.rs`: `IndexerService`, `QueryBuilder`, `QueryBuilderError`,
    `QueryService`, `SourceSelector`, `TaskConfig`, `TestProject` (plus root
    `SourceLine`).
  - `refresh.rs`: `FileEntry`, `QueryBuilder`, `QueryService`, `SourceSelector`,
    `TestProject`.
  - `template_render.rs`: `CommitPolicy`, `TemplatePathInput`,
    `TemplateService`, `TestProject`, `WriteMode`, `WriteOutcome`, `ZoneGuard`
    (plus root `PresetDialogProvider`).
  - Total distinct `traces::testing` symbols used: **13 of 47** (27.7%).
  - Unused by `tests/integration/` (34 symbols): for example, `Blake3Digest`,
    `Blake3PathHash`, `Config`, `ConfigService`, `TrustRequest`, `DriftCause`,
    `IndexSetting`, `InlinkMap`, `RefreshReport`, `WorkspaceIndex`,
    `path_codec`, `QueryResult`, `QueryRow`, `Schema`, `SchemaFieldDef`,
    `SchemaService`, `SchemaServiceError`, `RenderFailureKind`, `DEFAULT_*`,
    `build_test_index`, and other symbols.

## 2. Refactor Stability vs Architecture Design (Tickets 23–36)

- `TestProject`: High churn risk under ticket 30 (workspace host consolidation)
  and ticket 29 (path capability model).
- `QueryService` & `QueryBuilder`: At risk of signature drift under ticket 24
  (index read model) and ticket 26 (query core/plan unification).
- `FileClassExpander` seam (`template_render.rs:606`): Scheduled for deletion in
  tickets 20, 26, and 30.
- `ZoneGuard`: Global thread-state mutation; ticket 21 replaces ambient
  environment with an explicit context frame. ## 3. Systematic Test Audit

### tests/integration/persistence.rs (3 tests)

| Test Function | What It Proves | Nature | Twins & Parity | Refactor Stability | Assertions & Time/Env | Classification |
|---|---|---|---|---|---|---|
| `reloads_flat_list_items_with_metadata_and_hierarchy` | Proves redb `NOTES` serialization round-trips flat list items, tasks, priorities, dates, depths, and line pointers. | Multi-component in-process seam (`note` + `index` store). | Unit twin: `src/index/store.rs` has unit round-trips. | Fragile to redb table format changes (tickets 24, 25). | Strong assertions; no ambient time/env dependence. | KEEP-as-integration |
| `preserves_query_rows_and_list_metadata_across_cold_reload` | Proves redb index without `LISTS` table preserves identical `QuerySet` results after deleting original markdown files from disk. | Differential parity test between in-memory `QuerySet` and cold-reloaded store. | Closest unit twin: `src/query/service.rs:1242`. | Fragile to index cache format changes. | Strong exact row match (`assert_rows_match`). | KEEP-as-integration |
| `refresh_after_corruption_recovery_reports_every_file_upserted_and_nothing_deleted` | Proves `IndexerService::refresh_with_report` detects byte corruption in `.traces/index.redb` and recovers via full rebuild. | In-process filesystem/database fault-injection seam. | Closest e2e twin: `tests/e2e/index.rs:245` (`corrupted_index_database_heals_on_query`). | Fragile to recovery mechanism redesign. | Strong exact count assertion. | KEEP-as-integration |

### tests/integration/refresh.rs (2 tests)

| Test Function | What It Proves | Nature | Twins & Parity | Refactor Stability | Assertions & Time/Env | Classification |
|---|---|---|---|---|---|---|
| `refresh_persists_a_query_visible_delta_observed_without_mutation` | Proves note content edit followed by `refresh_with_report` persists changes observable by a fresh `load()` and query. | In-process seam (`index` store + `query`). | E2E twin: `tests/e2e/index.rs:19` (`content_edit_between_spawns_is_query_visible`). | Fragile to index refresh model (ticket 25). | Strong assertions on tag visibility and task text. | KEEP-as-integration |
| `refresh_with_added_and_deleted_paths_rebuilds_links_and_merges_notes` | Proves adding, deleting, and updating files during refresh preserves path sort order and updates backlink graph. | In-process multi-component seam (`index` + `note` link resolver). | E2E twin: `tests/e2e/index.rs:52` covers path-set changes via CLI. | High risk: stem/inlink cache redesign (tickets 24, 25). | Strong path and inlink assertions. | KEEP-as-integration |

### tests/integration/query.rs (18 tests)

| Test Function | What It Proves | Nature | Twins & Parity | Refactor Stability | Assertions & Time/Env | Classification |
|---|---|---|---|---|---|---|
| `page_query_returns_real_indexed_notes` | Proves `QueryBuilder::pages` returns all notes from an index. | Boundary test across index + query. | Unit twin: `src/query/service.rs:635` (`returns_all_files_in_sorted_order`). | Stable. | Moderate: path comparison only. | MISPLACED-unit |
| `sorts_pages_by_a_typed_date_frontmatter_field` | Proves sorting pages by date parses ISO strings and RFC3339 datetimes to common order. | Cross-module seam (`note` frontmatter + `temporal` + `query`). | E2E twin: `tests/e2e/query.rs:385` filters RFC3339. | High risk: ticket 19 (coercion/comparison) and 21 (ambient zone). | Strong exact order assertion. | KEEP-as-integration |
| `sorts_tasks_by_priority_severity_rank_with_nulls_first` | Proves `list.priority` sorts by severity rank, not alphabetical, nulls first. | Cross-module seam (`task` priority + `query` sorter). | Unit twin: `src/query/sort.rs` unit tests. | Stable. | Strong: tests both asc and desc orders. | KEEP-as-integration |
| `query_tasks_returns_task_level_rows_distinct_from_page_level_query` | Proves task query flattens multiple tasks per note into discrete rows. | Single-component query execution test over dummy notes. | Unit twin: `src/query/service.rs:1175`. | Stable. | Strong: checks completion bools. | MISPLACED-unit |
| `query_builder_reuses_one_index_for_page_and_task_queries` | Proves borrowing `Arc<WorkspaceIndex>` across successive queries works without consumption. | Pure API lifetime/borrow test. | Unit twin: can live in `src/query/service.rs`. | High risk: query execution redesign (ticket 26). | Weak: only counts rows. | MISPLACED-unit |
| `evaluates_query_modes_distinguishing_lists_and_tasks_with_structural_metadata` | Proves `QueryBuilder::lists` returns all list items while `tasks` returns only tagged tasks. | Cross-module seam (`config` task tags + `note` + `query`). | E2E twin: `tests/e2e/query.rs:219` (`task_preserves_custom_markers...`). | High risk: ticket 18 (value model) and ticket 23 (note model). | Strong: verifies depth, line, parent pointers. | KEEP-as-integration |
| `accepts_canonical_list_paths_and_rejects_obsolete_task_paths_with_diagnostic` | Proves `list.<field>` filter works and `task.<field>` returns `did you mean` error. | Diagnostic contract test. | Unit twin: `src/query/grammar/filter.rs`. E2E twin: `tests/e2e/query.rs:193`. | High risk: ticket 31 (diagnostic architecture). | Strong: asserts error string. | MISPLACED-unit |
| `inherits_note_frontmatter_on_list_rows_with_inline_field_override` | Proves list items inherit page frontmatter but allow inline field overrides. | Cross-module seam (`note` metadata + inline parser + `query` row resolution). | Closest unit: `src/query/service.rs:1025`. | High risk: ticket 23 (note document model). | Strong row-count and text assertions. | KEEP-as-integration |
| `sort_then_limit_returns_top_k_rows` | Proves sort + limit returns top k rows. | Query engine algorithmic test. | Exact unit duplicate: `src/query/builder.rs:230` (`sort_then_limit_matches_full_sort_order`). | Stable. | Moderate: path array assertion. | DUPLICATE |
| `sorting_an_empty_query_set_returns_empty` | Proves sorting an empty result set returns empty without panic. | Unit edge case. | Unit twin: belongs in `src/query/sort.rs`. | Stable. | Weak: `assert!(result.is_empty())`. | MISPLACED-unit |
| `sorts_pages_descending_by_text_field` | Proves descending text sort orders Z to A. | Query sorter unit test. | Unit twin: `src/query/sort.rs`. | High risk: ticket 19 (Unicode collation). | Moderate. | MISPLACED-unit |
| `filters_pages_using_temporal_arithmetic_and_duration_scaling` | Proves temporal filter expressions (`date ± duration`, `duration * N`). | Cross-module seam (`temporal` arithmetic + `query` grammar/eval). | Unit twin: `src/query/grammar/filter.rs:1871-1926`. | High risk: ticket 19 and 27. | Strong: tests 4 arithmetic combinations. | KEEP-as-integration |
| `filters_pages_using_unquoted_iso_date_literals` | Proves unquoted `YYYY-MM-DD` parses as date literal, not subtraction. | Grammar regression test. | Unit twin: `src/query/grammar/filter.rs`. | High risk: ticket 19 (quoted literal adaptation). | Strong exact row match. | MISPLACED-unit |
| `filters_pages_using_temporal_registry_functions` | Proves `date_add`, `date_diff`, and `date_component` functions evaluate correctly. | Cross-module seam (`temporal` + `query` evaluation). | Unit twin: `src/query/grammar/filter.rs:1895-1943`. | High risk: ticket 27 (temporal dialects). | Strong: covers 4 functions. | KEEP-as-integration |
| `filters_pages_using_temporal_bucketing_helpers` | Proves `sow`, `eow`, `som`, `eom`, `soy`, `eoy` helpers in query filter. | Cross-module seam (`temporal` + `query`). | Unit twin: `src/query/grammar/filter.rs`. | High risk: ticket 27. | Strong table assertion over 6 expressions. | KEEP-as-integration |
| `filters_pages_reject_static_invalid_temporal_arguments` | Proves query builder rejects invalid temporal arguments at parse time. | Pure query grammar parser test. | Unit duplicate: `src/query/grammar/filter.rs:1965-1975`. | High risk: ticket 31. | Weak: only checks `matches!(err, Syntax(_))`. | DUPLICATE |
| `filters_pages_handle_null_and_missing_temporal_operands` | Proves null or missing temporal operands propagate as null. | Query evaluation unit test. | Unit twin: `src/query/grammar/filter.rs:1941`. | High risk: ticket 19 (null placement). | Strong: checks both empty inequality and null equality. | MISPLACED-unit |
| `store_backed_and_in_memory_queries_agree_across_representative_selectors` | Differential parity: in-memory `run` and redb `sync_and_run` yield identical `QuerySet` across 7 selectors. | True integration parity test across index, store, and query engine. | No unit twin; essential storage fidelity proof. | Fragile to read-model changes (ticket 24). | Strong: asserts full `assert_eq!(from_store, from_memory)`. | KEEP-as-integration |

### tests/integration/template_render.rs (14 tests)

| Test Function | What It Proves | Nature | Twins & Parity | Refactor Stability | Assertions & Time/Env | Classification |
|---|---|---|---|---|---|---|
| `renders_a_query_over_real_indexed_notes_and_writes_the_result` | Proves template engine evaluates query global and writes rendered output to disk. | Multi-component in-process seam (`template` + `index` + `query` + disk I/O). | CLI e2e twin: `tests/e2e/template.rs:13`. | High risk: ticket 28 (template engine boundary). | Strong exact content check. | KEEP-as-integration |
| `renders_a_file_sourced_select_field_in_template_rendering` | Proves template resolves schema select field backed by external TOML file. | In-process seam driven through facade instead of CLI. | Exact duplicate of e2e: `tests/e2e/template.rs:163` (`renders_file_sourced_select_field_in_e2e_template`). | High risk: ticket 20 (schema typing) and 28. | Strong exact string check. | DUPLICATE |
| `renders_tasks_and_lists_pipelines_with_transforms_and_formatters` | Proves template `tasks.from()` and `lists.from()` pipelines with `where`, `sort`, `limit`, `task_list`, and `table`. | Multi-component in-process seam (`template` minijinja bridge + `query` + `task`). | E2E has partial equivalents. | High risk: ticket 28 (template bridge) and 26. | Strong assertions on counts, task list, and table. | KEEP-as-integration |
| `tasks_pipeline_rejects_obsolete_task_field_syntax_with_diagnostic` | Proves template `tasks.from().where('task.x == y')` bubbles query diagnostic through minijinja. | In-process error propagation across template/query boundary. | Parity twin with `tests/integration/query.rs:253`. | High risk: ticket 31 (diagnostics). | Strong: checks nested error source string. | KEEP-as-integration |
| `lists_pipeline_rejects_task_list_formatter_on_non_task_rows` | Proves template `lists.from().task_list()` rejects page rows. | In-process error propagation across template/query boundary. | Unit twin: `src/query/engine.rs`. | High risk: ticket 31. | Strong: checks error message. | KEEP-as-integration |
| `query_date_arithmetic_agrees_with_template_calendar_shift` | Parity differential: query date math agrees with template `date_add` filter for month clipping (`2026-01-31` + 1mo = `2026-02-28`). | In-process temporal parity seam (`query` arithmetic vs `template` date filter). | No CLI twin. | High risk: ticket 27 (temporal boundary) and 28. | Strong exact rendered string check. | KEEP-as-integration |
| `template_shorthands_support_iso_offsets_and_reference_format` | Proves `date.now` with ISO duration offsets and `date.sow`/`date.eow` reference formats. | Template date filter unit test. | Unit twin: `src/template/engine/date.rs`. | High risk: ticket 27 and 28. | Strong exact multi-token string. | MISPLACED-unit |
| `template_filters_support_bucketing_aliases_and_weekday_arg` | Proves template filters `sow`, `eow`, `som`, `eom`, `soy`, `eoy`, `weekday(n)`. | Template date filter unit test. | Unit twin: `src/template/engine/date.rs`. | High risk: ticket 27 and 28. | Strong table assertion. | MISPLACED-unit |
| `template_weekday_rejects_out_of_range_day_index` | Proves `weekday(n)` rejects out-of-range day numbers (0, 8, -1, 99). | Template filter unit test. | Unit twin: `src/template/engine/date.rs:280`. | High risk: ticket 28. | Weak: only asserts `is_err()`. | WEAK |
| `file_day_resolves_in_query_with_filename_precedence` | Proves `file.day` exposes date in query filters within templates. | Cross-module seam (`template` query + `note` path/title parser). | Unit twin in `src/query/`. | High risk: ticket 21 (civil `CalendarPeriod`). | Strong count assertion. | KEEP-as-integration |
| `template_shorthands_reject_malformed_iso_offsets` | Proves `date.now` rejects invalid ISO duration strings like `P1Q2D`. | Template filter unit test. | Unit twin: `src/template/engine/date.rs`. | High risk: ticket 27 and 28. | Weak: only asserts `is_err()`. | WEAK |
| `file_day_persists_at_index_time_across_query_time_zone_changes` | Proves `file.day` does not change value when queried under UTC+14 vs UTC-12. | In-process seam test with time-zone injection via `ZoneGuard`. | No e2e equivalent (E2E pins `TZ=UTC`). | High risk: ticket 21 replaces `ZoneGuard` with execution context. | Strong: checks both time zones. Ambient env dependent (`ZoneGuard`). | KEEP-as-integration |
| `template_seam_renders_local_clock_under_injected_tz` | Proves `date.now("%Y")` renders local clock year under injected TZ. | Seam test checking `ZoneGuard` affects template clock. | Unit twin: `src/template/engine/date.rs:1372`. | High risk: ticket 21. | Weak assertion (`assert_eq!(content.len(), 4)`). Ambient env dependent (`ZoneGuard`). | WEAK |
| `renders_class_expanded_source_selectors_through_the_schema_graph` | Proves `@class*` transitive inheritance resolves in template queries via `FileClassExpander`. | In-process seam between `schema` DAG, `index`, `query`, and `template`. | Twin in E2E: `tests/e2e/query.rs:30`. | High risk: `FileClassExpander` seam is deleted in ticket 20. | Strong exact output check (`2-1-0-1-1`). | KEEP-as-integration |
| `index_cache_does_not_persist_across_independent_renders` | Proves reused `TemplateService` observes filesystem edits between consecutive renders. | In-process stateful lifecycle and caching seam test. | No unit or e2e twin (E2E spawns fresh processes). | High risk: ticket 28 and 30. | Strong two-stage assertion. | KEEP-as-integration |

*Note: `template_render.rs` contains 14 test functions, but `template_weekday_rejects_out_of_range_day_index` expands to 4 test cases via `rstest`, yielding 17 runnable test items.*

---

## 4. Synthesis: Does a Distinct Integration Layer Still Earn Its Keep?

**Yes, but strictly as an in-process multi-component seam and storage fidelity layer. It is not a place for unit tests or spawned-CLI substitutes.**

The in-process integration layer proves guarantees that unit tests and E2E cannot prove:

1. **Redb persistence and recovery invariance**: Direct corruption of the
   on-disk database file and check of recovery through wipe-and-rebuild
   (`persistence.rs:249`) cannot run as a unit test without disk I/O. A spawned
   binary cannot inspect the structured memory state.
2. **Cold-process memory-vs-storage parity differentials**: Comparing `QuerySet`
   row equality between in-memory structures and store-reloaded structures after
   deleting original Markdown files from disk (`persistence.rs:116`,
   `query.rs:667`) guarantees zero codec drift and zero secondary-index
   divergence without reparsing Markdown. E2E tests observe only serialized
   stdout text.
3. **Multi-render in-process cache isolation**: Verifying that a long-lived
   service instance (like `TemplateService`) does not leak cached state across
   independent renders when files change on disk (`template_render.rs:670`). E2E
   spawns a new process for every command and cannot detect in-process caching
   defects.
4. **Timezone injection across shared domains**: Testing that date arithmetic in
   queries and templates aligns under arbitrary non-UTC time zones
   (`query_date_arithmetic_agrees_with_template_calendar_shift`,
   `file_day_persists...`) without spawning sub-shells.

However, **approximately 40% of the existing tests are misplaced**:
single-expression parser checks, string formatting tests, and redundant
sort/limit tests belong inline in `src/`.

---

## 5. Classification Summary & Top 10 Defects

### Classification Counts (37 test functions / 40 runnable test cases)

- **KEEP-as-integration**: 23 (57.5%)
- **MISPLACED-unit**: 10 (25.0%)
- **DUPLICATE**: 4 (10.0%)
- **WEAK**: 3 (7.5%) *(Note: 3 tests classified as WEAK also overlap with
  MISPLACED-unit).*

### Top 10 Defects and Fragilities

1. **Dead Facade Weight**: `traces::testing` exposes 47 symbols, but the
   integration suite uses only 13 (27.7%). The remaining 34 symbols
   unnecessarily expand the public test-utils API baseline.
2. **Direct Duplicate of E2E**:
   `renders_a_file_sourced_select_field_in_template_rendering`
   (`template_render.rs:60`) is an exact copy of
   `renders_file_sourced_select_field_in_e2e_template`
   (`tests/e2e/template.rs:163`). Both test identical TOML and template data.
3. **Direct Duplicate of Unit Builder**: `sort_then_limit_returns_top_k_rows`
   (`query.rs:366`) duplicates `src/query/builder.rs:230` without exercising any
   new seam.
4. **Duplicate Grammar Error Test**:
   `filters_pages_reject_static_invalid_temporal_arguments` (`query.rs:598`)
   duplicates parser tests in `src/query/grammar/filter.rs:1965` with weaker
   assertions.
5. **Condemned Seam Coverage**:
   `renders_class_expanded_source_selectors_through_the_schema_graph`
   (`template_render.rs:606`) tests `FileClassExpander`. Architectural redesign
   tickets 20 and 26 schedule this interface for deletion.
6. **Thread-Unsafe Global Environment Mutation**: `ZoneGuard::set`
   (`template_render.rs:552`) mutates global process timezone state, creating
   risks for parallel execution.
7. **Vacuous Assertion**: `template_seam_renders_local_clock_under_injected_tz`
   (`template_render.rs:551`) asserts only `content.len() == 4`. That assertion
   passes for any 4-character string (for example `"2026"`, `"1970"`, `"None"`,
   `"fail"`).
8. **Uninformative `is_err()` Checks**:
   `template_weekday_rejects_out_of_range_day_index` (`template_render.rs:422`)
   and `template_shorthands_reject_malformed_iso_offsets`
   (`template_render.rs:474`) assert only `.is_err()`. They ignore the failure
   variant and diagnostic hint.
9. **Misplaced Single-Module Unit Tests in Integration**:
   `page_query_returns_real_indexed_notes`,
   `query_tasks_returns_task_level_rows...`, and
   `sorting_an_empty_query_set_returns_empty` in `query.rs` do not exercise
   cross-module seams. They belong in `src/query/`.
10. **Facade Documentation Lag**: `src/lib.rs:182` still documents "exactly the
    46 identifiers" despite the baseline requiring 47 symbols since the
    `RefreshReport` drift refactor.
