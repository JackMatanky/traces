# Research 02: Query & template pipeline: candidate-guarantee inventory

Scope per ticket: `src/query/` (source-selection DSL, filter semantics, planning,
memoized rows, error semantics) and `src/template/` (resolution order, minijinja
namespaces, confined writes, render errors). Method: breadth-first reading of all
32 source files, inline test modules, the four integration/e2e suites, both
GLOSSARYs, ADR-0001/0005/0008/0009.

**Citation convention** (matches `.scratch/bdd-testing/` style): `file:line` = repo
source; `ADR-000N:Lx-Ly` = `docs/adr/000N-*.md` line range; test names are cited
as `fn_name @ file:line`. Verdicts are `intentional` (documented/glossary'd/ADR'd
**and** test-shaped), `incidental` (nothing specifies it), `uncertain` (docs claim
it, code status unverified or weakly implemented). Entry format: Contract
(observable) → Class → Verdict+Evidence → Pinned → Gherkin-suitability.

## 0. Survey claims checked

| Ticket claim | Finding |
|---|---|
| `src/query/` has 317 `#[test]`s, `src/template/` 362 | **Both confirmed exact** (per-file `rg -c '#\[test\]'` sums; heaviest files `engine/query.rs` 64, `sort.rs` 43, `engine/date.rs` 54, `results.rs` 72) |
| Test-file counts | `tests/integration/query.rs` 18, `integration/template_render.rs` 15, `tests/e2e/query.rs` 15, `e2e/template.rs` 7 (`#[test]`/`#[rstest]` mix) |
| Both GLOSSARYs exist | Yes: `src/query/GLOSSARY.md` (75 lines), `src/template/GLOSSARY.md` (73), mapped from `GLOSSARY-MAP.md:23-28` |

## 1. User-visible language surface (strong scenario candidates)

### G1: Source-expression grammar: atoms, boolean ops, precedence
- **Contract**: `from()` accepts `` #tag `` (exact **and** nested sub-tag: `#book`
  matches `#book/fiction`), `folder/` (compiles to `folder/**`), `@Class` with
  `*` descendant / `+` child sigils, quoted path globs (`*`, `**`, `/`-anchored,
  `\` normalized), combined with `and`/`or`/`not`/`&&`/`||`/parens under
  precedence `NOT > AND > OR`.
- **Class**: user-visible language surface.
- **Verdict: intentional**: ADR-0008:59-77 specifies grammar + precedence;
  `src/query/GLOSSARY.md:10-14` defines Source Expression; matching rules are
  doc-commented at `src/query/grammar/source.rs:179-201` (tag containment via
  `is_contained_in`, `source.rs:221`; folder glob at `source.rs:565-577`;
  glob semantics `source.rs:248-305`; class modes `source.rs:310-330`); shared
  precedence parser `src/query/grammar/expr.rs:1-11`.
- **Pinned**: yes; `parses_precedence_grouping_and_repeated_negation @
  source.rs:776`, `parses_grouped_expression_overriding_precedence @ expr.rs:399`,
  `matches_boolean_combinations_of_tags_and_paths @ source.rs:974`, e2e
  `class_descendant_selector_expands_through_the_schema_graph @ tests/e2e/query.rs:30`.
- **Gherkin**: **good**: every atom/precedence case is a Given-When-Then over
  fixture notes; exact-match vs nested-tag and `@class` vs `@class*` are the
  highest-value scenarios.

### G2: `from()` input dispatch (no-arg / string / Schema file field)
- **Contract**: `.from()` → every indexed note; `.from("")`/blank string → same
  (`source.rs:51-57`); `.from("#book")` → parsed DSL; a Schema `file` field value
  (`SourceSelector`) is used as-is and can be widened by the `| with_children` /
  `| with_descendants` filters.
- **Class**: user-visible language surface.
- **Verdict: intentional**: ADR-0008:51-57 (three input types + downcast
  smuggling); `resolve_from_arg @ src/template/engine/query.rs:280-303`; filters
  registered `query.rs:182-183`.
- **Pinned**: yes; `renders_a_file_sourced_select_field_in_template_rendering @
  integration/template_render.rs:60`,
  `renders_class_expanded_source_selectors_through_the_schema_graph @ :606`.
- **Gherkin**: **good**.

### G3: Unknown class selector degrades to empty, never errors/panics
- **Contract**: `@horror` with no such class matches nothing, exit success, no
  exit-101; `@book` (exact) matches nothing when only child classes exist while
  `@book*` transitively matches them.
- **Class**: user-visible language surface (CLI contract).
- **Verdict: intentional**: explicit spec in the test's own doc comment
  `tests/e2e/query.rs:12-56` ("unknown class degrades to a literal match … no
  error, no panic"); backed by `ClassExpansionMode` design `source.rs:310-330`,
  ADR-0008:70-73.
- **Pinned**: yes; `class_descendant_selector_expands_through_the_schema_graph
  @ tests/e2e/query.rs:30` (incl. `assert_never_exit_101`).
- **Gherkin**: **excellent**: pure observable CLI behavior.

### G4: Query-mode row granularity (`pages` / `lists` / `tasks`)
- **Contract**: `query` = one row per note; `lists` = one row per list item
  (plain bullets, checkboxes, tasks) in document order; `tasks` = one row per
  task item only. Task/page accessors return `None`/`null` on the wrong row kind.
- **Class**: user-visible language surface.
- **Verdict: intentional**: `src/query/GLOSSARY.md:22-26` (Query Mode),
  `builder.rs:19-27`, `src/query/mod.rs:16-19`, `src/template/engine/query.rs:22-28`;
  wrong-kind accessors documented `results.rs:87-130`.
- **Pinned**: yes; `evaluates_query_modes_distinguishing_lists_and_tasks_with_structural_metadata
  @ integration/query.rs:175`, `query_tasks_returns_task_level_rows_distinct_from_page_level_query
  @ integration/query.rs:123`, `task_fields_resolve_to_null_on_page_level_records
  @ results.rs:1216`.
- **Gherkin**: **good**.

### G5: Filter-expression semantics (comparisons, `contains`, temporal functions)
- **Contract**: `== != < <= > >=` with type coercion; `contains(field, target)`
  does tag-prefix matching; date±duration, date−date, duration×number; `dur`,
  `date_add`, `date_diff`, `date_component` + registry/bucketing helpers;
  arithmetic groups **left-to-right** (no precedence); null/invalid rows never
  satisfy ordered comparisons; statically invalid arguments fail at parse time;
  bare ISO date literals lex as dates and equal their quoted form; number-only
  arithmetic (`1 + 2 == 3`) is a build error.
- **Class**: user-visible language surface.
- **Verdict: intentional**: `src/query/grammar/filter.rs:1-18` (module spec),
  ADR-0009:26-38 (bare dates + number-arithmetic rejection), `mod.rs:31-36`.
  Note: **ADR-0009 status is `proposed`** (ADR-0009:3), so its guarantees are
  decided-but-not-yet-accepted.
- **Pinned**: yes; `filters_pages_using_temporal_arithmetic_and_duration_scaling
  @ integration/query.rs:435`, `…_unquoted_iso_date_literals @ :486`,
  `…_temporal_registry_functions @ :513`, `…_temporal_bucketing_helpers @ :572`,
  `…_reject_static_invalid_temporal_arguments @ :598`,
  `…_handle_null_and_missing_temporal_operands @ :615`,
  `rejects_arithmetic_on_number_literals_alone @ filter.rs:1482`,
  `list_filters_an_inline_multi_part_duration @ tests/e2e/query.rs:409`.
- **Gherkin**: **good** for date/null/comparison scenarios; arithmetic-order and
  null-semantics cases need table-driven Examples.

### G6: Sort default direction and CLI sort flags
- **Contract**: sorting defaults to **descending** everywhere (template
  `.sort("f")` with no arg, CLI `--sort` with no prefix); `+`/`-` prefixes and
  `--asc`/`--desc` override per segment; `--asc` and `--desc` conflict and require
  `--sort`.
- **Class**: user-visible language surface.
- **Verdict: intentional**: `GLOSSARY.md:43-48` ("defaulting to descending"),
  `engine/query.rs:341,400-411`, `src/cli/mod.rs:278-318`, ADR-0005:80-84.
- **Pinned**: yes; `defaults_to_descending_without_flags @ src/cli/mod.rs:843`,
  `sorts_pages_descending_by_text_field @ integration/query.rs:412`, flag-conflict
  parsing tests `cli/mod.rs:843-881`.
- **Gherkin**: **good**.

### G7: Null placement (Dataview-compatible)
- **Contract**: missing/null sorts **first ascending, last descending** (the
  Dataview default); a `NullPlacement` seam exists for future `nulls first/last`
  grammar but no grammar exposes it yet.
- **Class**: user-visible language surface (output ordering), with an internal
  seam deliberately left unpinned by syntax.
- **Verdict: intentional**: `src/query/sort.rs:283-305` (Dataview rationale +
  seam rationale), `sort.rs:535`.
- **Pinned**: partially; unit `null_placement_seam_controls_null_ordering_in_compare_keys
  @ sort.rs:759` + `auto_placement_puts_null_first_ascending_and_last_descending
  @ sort.rs:810`; integration `sorts_tasks_by_priority_severity_rank_with_nulls_first
  @ integration/query.rs:74`. The unpinnable *seam* (Future variants) is by design.
- **Gherkin**: **medium**: only observable through whole-result ordering.

### G8: Terminal renderers: dual form (method + pipeline filter), parity, output rules
- **Contract**: `table`, `list`, `task_list`, `count` are terminal: reachable as
  `rows.table(...)` and `rows | table(...)`, both calling the same `QuerySet`
  method; `count` is infallible (`QuerySet::len`); `table` escapes `|` in cells
  and headers and collapses newlines; header/column length mismatch is an error;
  `task_list` on page-level rows is an error; `task_list` path styles
  `Suffix`/`Coordinates`/`None` render `({path})`, `({path}:{line})` (1-indexed),
  or nothing.
- **Class**: user-visible language surface (rendered output + errors).
- **Verdict: intentional**: `engine/query.rs:37-43` ("Both forms call the same
  QuerySet method"), ADR-0005:67-69, `GLOSSARY.md:69-75` (Task Path Style,
  `traces task -l` → `src/cli/task.rs:104-106,132`), error variants
  `src/query/error.rs:45-63`.
- **Pinned**: yes, strongly; parity ×4: `table/list/task_list/count_call_method_and_filter_forms_render_identically
  @ engine/query.rs:1008,1032,1053,1073`; escaping
  `…_escapes_pipe_characters_so_cell_values_cannot_break_table_rows @ results.rs:1508`
  (+1519, +1528); `rejects_a_headers_columns_length_mismatch @ results.rs:1553`;
  `rejects_page_level_records_with_no_task_fields @ results.rs:1719`;
  `renders_line_number_coordinates_with_line_numbers_flag @ src/cli/task.rs:390`;
  e2e `task_table_renders_default_and_custom_columns @ e2e/query.rs:281`.
- **Gherkin**: **good**: byte-exact markdown assertions, but that *is* the
  contract; keep fixtures tiny.

### G9: Diagnostics contract: spans, typo suggestions, stable CLI codes
- **Contract**: malformed source/filter expressions produce miette diagnostics
  with exact source spans (`invalid filter expression`); bad field paths produce
  ``did you mean `file.name`?`` edit-distance hints; `task.<f>` paths are rejected
  in favor of `list.<f>`; CLI failure codes are stable strings
  (`traces::cli::query::failed`, `traces::cli::template::render_query_failed`,
  `traces::cli::template::output_exists`); render errors carry template name +
  1-based line and column.
- **Class**: user-visible language surface.
- **Verdict: intentional**: `src/query/error.rs:1-18,107-125` (spans/labels),
  `error.rs:166-196` (`FieldPathError` suggestion), `field.rs:68-143` (typo
  threshold + `task.` rejection with `list.` suggestion), `cli/error.rs:751-770`
  + tests `cli/error.rs:1579,1588`, ADR-0008:94-98 (miette spans "addressed").
- **Pinned**: yes; `unknown_sort_field_reports_a_did_you_mean_suggestion @
  tests/e2e/query.rs:100`, `invalid_filter_expression_reports_its_location_and_repair
  @ e2e/query.rs:123`, `task_invalid_where_reports_its_location_and_repair @
  e2e/query.rs:193`, `accepts_canonical_list_paths_and_rejects_obsolete_task_paths_with_diagnostic
  @ integration/query.rs:253`, `tasks_pipeline_rejects_obsolete_task_field_syntax_with_diagnostic
  @ template_render.rs:234`, `render_error_reports_a_stable_diagnostic_code @
  tests/e2e/template.rs:240`.
- **Gherkin**: **good**, caveat: never assert raw miette-wrapped stderr for long
  causal chains (line-wrap brittleness documented at `tests/e2e/template.rs:261-266`);
  assert short substrings after `plain()`.

### G10: Template resolution order + validation-before-search
- **Contract**: lookup runs **per directory, local before global**, each
  directory trying exact relative path first, then extensionless stem match
  (ambiguity → `AmbiguousTemplate` error listing candidates); symlinks never
  match; absolute/`..`/bare-`.` inputs fail validation **before** any directory is
  searched (`Absolute`/`UnsafeComponent`), distinct from `TemplateNotFound` only
  after all directories miss; the same loader backs `-i` and
  `{% include %}`/`{% extends %}`; a missing include honors `ignore missing`.
- **Class**: user-visible language surface (template names users type).
- **Verdict: intentional**: `src/template/loader.rs:1-27,73-108,110-206`,
  `GLOSSARY.md:22-32`, `src/template/path.rs:41-86`.
- **Pinned**: unit-heavy (32 tests in `loader.rs`, 8 in `path.rs`: ambiguity,
  symlink, validation); no dedicated e2e for cross-directory precedence found.
- **Gherkin**: **medium-good**: good scenario material (local overrides global;
  `../evil` rejected), but fixture-heavy (needs local+global dirs).

### G11: Output-path precedence + root confinement
- **Contract**: destination resolves `--output` > `file.write_to()` > configured
  default output dir; both runtime candidates are confined to the project root
  (`OutputPathEscapesRoot` / `OutputPathUnverifiable` on escape); default output
  filename is the template path with extension forced to `md`; dry-run returns
  content **without resolving/validating** the output path.
- **Class**: user-visible language surface **and** security boundary.
- **Verdict: intentional**: `src/template/writer.rs:1-20,133-230`,
  `writer.rs:303` (`confine`), `src/template/path.rs:29-39,141-154`,
  `GLOSSARY.md:56-61`, `service.rs:85-90`.
- **Pinned**: yes; `confines_a_candidate_in_a_not_yet_created_subdirectory @
  writer.rs:477`, e2e `overwrite_refusal_is_non_destructive_and_force_and_output_paths_spawn
  @ e2e/template.rs:44` (`-f`, `-o` argv), `…_writes_the_rendered_file_to_the_default_output_path
  @ e2e/template.rs:13`.
- **Gherkin**: **excellent**: escape-root and precedence cases are canonical
  Given-When-Then.

### G12: Write mode vs prompts: independent axes; overwrite policy
- **Contract**: `--dry-run` suppresses **disk writes only**: `ui.*` prompts and
  index refresh still execute; non-TTY falls to defaults without hanging;
  `--no-input` swaps in an empty preset provider; commit writes refuse an existing
  target atomically (`create_new`, non-destructive) unless `--force`.
- **Class**: user-visible language surface.
- **Verdict: intentional**: ADR-0001:39-42 ("dry-run does not suppress
  prompts"), `writer.rs:31-57,60-121`, `service.rs:27-37,46-50`,
  `GLOSSARY.md:63-73` (Write Mode, Commit Policy).
- **Pinned**: yes; `dry_run_still_uses_the_injected_provider_for_ui_calls @
  src/template/service.rs:1141` (ADR-0001:91 cites stale `:1152`, see C3),
  `non_tty_dialog_short_circuits_to_defaults_without_hanging @ e2e/template.rs:108`,
  `dry_run_prints_rendered_content_to_stdout_without_writing @ e2e/template.rs:210`.
- **Gherkin**: **excellent**.

### G13: CLI stream contract and index freshness round-trips
- **Contract**: `traces list` writes matches to stdout and the count line to
  stderr (scripts pipe on this); a rendered template's output re-enters the index
  for later queries; each render refreshes the index once and **never** reuses a
  cache across renders.
- **Class**: user-visible language surface (CLI/process contract).
- **Verdict: intentional**: the stdout/stderr split is explicitly declared a
  guarantee in `tests/e2e/query.rs:145-148`; freshness docs `engine/query.rs:18-20`
  and ADR-0005:58-62 (lazy refresh).
- **Pinned**: yes; `…list_prints_matching_pages_to_stdout_and_a_count_to_stderr
  @ e2e/query.rs:148`, `rendered_output_reenters_the_index @ e2e/template.rs:144`,
  `index_cache_does_not_persist_across_independent_renders @ integration/template_render.rs:670`.
- **Gherkin**: **good** (multi-step: render → index → query).

## 2. Internal / plan-sensitive (weak scenario candidates)

### G14: Plan fusion & TopK optimization (mechanism) / equivalence (contract)
- **Contract (mechanism)**: adjacent filters fuse to one `And`; adjacent sorts
  fuse; sort+limit rewrites to `TopK` quickselect; optimizations are idempotent.
  `plan.rs:8-18,28-41,111-133`; ADR-0005:91-93.
- **Contract (observable)**: fused/chained plans must yield **identical results**
  to the unfused form, including tie-break stability (TopK ties broken by input
  index, `plan.rs:294-300`).
- **Class**: mechanism = internal/plan-sensitive; equivalence = observable.
- **Verdict: intentional for equivalence**: pinned by
  `chained_filters_match_one_combined_filter_expression @ results.rs:1786`,
  `chained_sort_then_limit_matches_full_sort_order_for_tied_keys @ results.rs:1808`,
  `sort_then_limit_returns_top_k_rows @ integration/query.rs:366`. **Idempotence
  claim** (`plan.rs:30-31`) is unit-only. Fusion itself is *incidental* to users.
- **Pinned**: equivalence yes; mechanism internals unit-only.
- **Gherkin**: **poor for mechanism, good for equivalence**: write scenarios as
  "same input, chained vs single-call, same output"; never assert plan shape.

### G15: QuerySet memoization, O(1) chaining, branch isolation
- **Contract**: transform methods append to a pending plan in O(1); reads flush
  the plan once and cache rows; cloning shares memoized rows; branching from a
  cloned base never cross-contaminates. `GLOSSARY.md:60-67`, `results.rs:456-483`.
- **Class**: internal (the *isolation* half is observable).
- **Verdict: intentional-but-implementation-flavored**: note the GLOSSARY
  elevates `O(1)`/`shares the memoized rows` into domain language, i.e. even the
  glossary is asserting performance internals; only isolation is user-observable.
- **Pinned**: isolation yes (`branching_from_the_same_base_does_not_cross_contaminate
  @ results.rs:1858`); O(1)/shared-rows no (nothing measures them).
- **Gherkin**: **poor** for memoization/O(1) (unobservable); medium for
  branch-isolation (Given chained-and-cloned sets, assert outputs diverge).

### G16: Row representation: zero-allocation views, size budget, document order
- **Contract**: `QueryRow` is a positional view borrowing from the index; list
  rows keep document order; row size stays within a budget. `results.rs:1-13,39-50`,
  `cmp_document_order @ results.rs:427`.
- **Class**: internal/perf.
- **Verdict: intentional internally**: pinned by `query_row_stays_within_its_size_budget
  @ results.rs:787`; document order pinned observably by
  `renders_a_checkbox_line_per_task_in_document_order @ src/cli/task.rs:253`.
- **Gherkin**: **poor** (size budget), **good** for document-ordering only.

### G17: Store-backed vs in-memory execution agreement
- **Contract**: queries executed against persisted redb tables and against the
  in-memory index agree across representative selectors. `service.rs:1-10`.
- **Class**: internal seam (architecture-level invariant).
- **Verdict: intentional**: pinned by `store_backed_and_in_memory_queries_agree…
  @ integration/query.rs:667`.
- **Gherkin**: **poor**: invisible at CLI level; belongs in Rust integration
  tests, not scenarios.

### G18: Include-loader soft-fail and symlink/dedup quirks
- **Contract**: missing include → `Ok(None)` (so `ignore missing` works) but
  ambiguity/invalid-identifier/directory-read errors are hard failures
  (`loader.rs:208-220`); directory dedup is string-equality only, no
  canonicalization, so a symlinked twin dir is searched twice (`loader.rs:110-122`).
- **Class**: internal/exotic edge behavior (arguably user-visible for includes).
- **Verdict: intentional** for the soft/hard split (documented rationale in
  `loader.rs:213-216`); the no-canonicalization quirk is documented-as-known
  (`loader.rs:113-115`) → intentional-but-accepted-limitation.
- **Pinned**: unit-level within `loader.rs` (32 tests); no e2e include scenarios found.
- **Gherkin**: **poor** (platform/symlink variance; keep to unit tests).

## 3. Docs-vs-code contradictions

- **C1 (real semantic mismatch): ADR-0008 names `SourceAtom` variants that do not
  exist.** ADR-0008:75-77: "where `SourceAtom` is `Tag`, `Folder`, `Class`, or
  `Glob`". Code: `pub(crate) enum SourceAtom { Tag, Path(GlobPattern), Class }` at
  `src/query/grammar/source.rs:190-201`: `Folder` and `Glob` were unified into
  `Path`. Behavior matches the prose (folder/glob both work), the *type names* in
  the ADR are wrong. Fix the ADR or accept as drift.
- **C2 (weaker claim than ADR states): terminal-then-filter "clear runtime
  error".** ADR-0005:71-74: "Attempting to sequence a terminal filter before
  another filter produces a clear runtime error with template name, line, and
  column." Code has **no dedicated error**: terminal filters take `&QuerySet`
  (`engine/query.rs:447-472`), so misuse degrades to a generic minijinja
  argument-type/`UnknownMethod` error. Location/line/column is true only
  incidentally (all render errors carry it). No test asserts this claim anywhere
  (`rg terminal tests/` → none). Classify as **uncertain/unverified**.
- **C3 (stale line citations): ADR-0001 and ADR-0008 line numbers drifted.**
  ADR-0001:76 `src/dialog/mod.rs:35` → trait at `mod.rs:45`; ADR-0001:86
  `engine.rs:144` (ui wiring) → ui registers at `engine.rs:158`; ADR-0001:91
  `service.rs:1152` → test at `:1141`; ADR-0001:85 `ui.rs:34` → **correct**;
  ADR-0008:52 `query.rs:262-278` (`from()` dispatch) → `resolve_from_arg` now at
  `:280`. Citation drift only, not behavior drift.
- **C4 (ambiguous glossary vs loader order): cross-dimension precedence.**
  `GLOSSARY.md:28-32`: "first as exact paths, then in local template directories,
  then in global directories". Code order is **per-directory**: local-exact →
  local-stem → global-exact → global-stem (`loader.rs:98-105`). Consequence: a
  local *stem* match beats a *global exact-path* match, which the glossary
  phrasing does not obviously allow. Loader module docs are authoritative and
  explicit (`loader.rs:3-8`); the glossary sentence should be reworded.
- **C5 (status mismatch, minor): ADR-0009 is `proposed` while fully wired.**
  ADR-0009:3 status `proposed`, yet its behavior is implemented
  (`filter.rs:54-115` date-lexing, `filter.rs:1081-1090` number-only rejection)
  and integration-pinned (`integration/query.rs:486`). Also **duplicate ADR
  number**: `docs/adr/0009-filter-date-literals.md` and `docs/adr/0009-index-refresh.md`.
- **C6 (checked, no contradiction)**: registration table ADR-0001:63-74 matches
  actual modules (`file/query/tasks/ui/date/schema` objects + `uuid` @
  `engine.rs:164` + `string/num/yaml` filters); ui's four functions ADR-0001:33
  = `ui.rs:34`; `traces task -l` glossary = `cli/task.rs:104-106`; glob support
  claimed by `mod.rs:28-30` and `GLOSSARY.md:10-14` really exists
  (`source.rs:565-577` + quoted globs); ADR-0005:64-69 three input types =
  `resolve_from_arg`.

## 4. Hostile to Gherkin-style testing

- **H1: Performance claims (O(1), zero-allocation, quickselect, capacity
  overshoot `service.rs:221-223`)**: unobservable in scenarios. Assert only
  output equivalence; keep them in criterion benches, never Given-When-Then.
- **H2: Plan internals (fusion order, TopK rewrite, idempotence)**: refactor-
  sensitive; scenarios that inspect *how* the plan ran will break on harmless
  changes. Pin only chained-vs-single output equality (results.rs:1786/1808).
- **H3: Byte-exact markdown renderers** (table pipes, newline collapsing,
  checkbox symbols): brittle to formatting tweaks, but escaping rules
  (`results.rs:1508-1528`) are genuine security-adjacent contracts worth pinning
  with tiny fixtures.
- **H4: miette-wrapped stderr**: line-wraps with `│` continuations can split
  paths mid-token (`tests/e2e/template.rs:261-266`). Use `plain()` + short
  substrings + stable codes (`traces::cli::*`), never full-stderr equality.
- **H5: Time/locale dependence**: temporal filter semantics use the local zone
  (`filter.rs:386-411`, `ZoneGuard @ filter.rs:1340`) and render tests inject TZ
  (`template_render.rs:551`). Scenarios touching dates need the same injected-zone
  fixture or they'll flake.
- **H6: Filesystem variance**: symlinks never matching (`loader.rs:124-146`),
  no path canonicalization (`loader.rs:110-122`), Windows separator normalization
  in globs (`source.rs:260-266`): platform-specific; leave to unit tests, not
  cross-platform scenarios.
- **H7: Interactive/dialog paths**: require PresetDialogProvider/non-TTY
  conventions (`--no-input`, null stdin, ADR-0001:39-42); scenarios must spawn
  the real CLI (as `tests/e2e/template.rs:108`) or they test a mock.
- **H8: Index refresh timing**: freshness spans two commands (index/render →
  query); each scenario needs explicit `traces index`/render-order steps
  (`tests/e2e/query.rs:30`, `e2e/template.rs:144`).

---

## 5. Bottom line for ticket 03+ (scenario selection)

Highest-value, pinned, user-visible scenario seeds: **G3** (class-selector
degradation), **G11** (output precedence + root escape), **G12** (dry-run vs
prompts vs overwrite), **G9** (diagnostics/codes), **G5** (date literals + null
semantics), **G6** (sort defaults), **G8** (terminal parity/escaping), **G13**
(stream split + index round-trip). Exclude from Gherkin: **G14 mechanism, G15
memoization, G16 size budget, G17 store-vs-memory, G18 symlink quirks** (all
Rust-level, each already pinned by unit/integration tests).
