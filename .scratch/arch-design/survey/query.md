# Architectural Survey: Query Subsystem (`src/query/**`)

## 1. Responsibilities & Seams
- **Core Ownership**: Evaluates declarative source selection (`--from`), row filtering (`--where`), multi-column sorting, grouping (by sort), list/task flattening, and terminal Markdown rendering (`table`, `list`, `task_list`) over indexed notes and list/task items.
- **Public Surface (`pub`)**:
  - `QueryBuilder` (`src/query/builder.rs:55`): declarative specification (`pages`, `lists`, `tasks`).
  - `QueryService` (`src/query/service.rs:57`): execution coordinator over `Arc<WorkspaceIndex>` or `IndexStore`.
  - `QuerySet` (`src/query/results.rs:464`): lazy, memoized result collection supporting method chaining.
  - `QueryRow` (`src/query/results.rs:44`): zero-copy positional row view over `WorkspaceIndex`.
  - `SourceSelector` (`src/query/grammar/source.rs:34`): parsed document-level selector (`All`, `Expr(SourceExpr)`).
  - `QueryError`, `QueryBuilderError`, `QuerySyntaxError`, `QueryDialect`, `QueryResult` (`src/query/error.rs:27-113`).
- **Crate-Internal Surface (`pub(crate)`)**:
  - `QueryMode` (`Pages`, `Lists`, `Tasks`) (`src/query/builder.rs:21`).
  - `ExecutionPlan`, `QueryTransform` (`src/query/plan.rs:33,143`).
  - `FileClassExpander`, `ClassExpansionMode` (`src/query/grammar/source.rs:321,353`).
  - `FieldPath`, `FileField`, `ListField`, `TaskField` (`src/query/grammar/field.rs:37,157,229`).
  - `FilterExpr` (`src/query/grammar/filter.rs:42`), `SortOrder`, `SortDirection` (`src/query/sort.rs:29,267`).
  - `TaskPathStyle` (`src/query/format.rs:16`).
- **Callers & Boundaries**:
  - `src/cli/` (`mod.rs:335-366,470-488`, `list.rs`, `table.rs`, `task.rs`): constructs `QueryBuilder` via CLI flags, executes via `QueryService::run_from_store`.
  - `src/template/engine/query.rs` (`lines 106-239, 309-430`): exposes `query`, `lists`, `tasks` globals in Minijinja; executes `QueryService::run` on render-cached `Arc<WorkspaceIndex>`.
  - `src/file_class_expander.rs` (`lines 15-39`): wires `SchemaService` to `FileClassExpander`.

## 2. Invariants
- **Boolean Precedence (`NOT > AND > OR`)**: Enforced at construction by `BooleanExprParser` recursive-descent tiers (`src/query/grammar/expr.rs:154-213`). Shared across `SourceExpr` and `FilterExpr`.
- **Deterministic Total Order Across Types**: Enforced at runtime by `SortKey::cmp` (`src/query/sort.rs:425-439`) and `NoteFieldValueRef::compare` (`src/note/field.rs:286-312`). Null sorts lowest; rank order: `Null < Bool < Number < Duration < Temporal < Text < Link < List < Object`. Floats normalize `-0.0` to `0.0` and use `total_cmp` (`src/query/sort.rs:432`).
- **Lazy Single Materialization in `QuerySet`**: Enforced at runtime via `std::sync::OnceLock` (`src/query/results.rs:467,484-490`). Chained transforms clone base and append to `ExecutionPlan` in $O(1)$ (`src/query/results.rs:521-529`).
- **Task Formatting Typestate Lack**: Enforced at runtime rather than type system: `render_task_list` returns `Err(QueryError::TaskListRequiresTaskRows)` if `row.task_text()` is `None` (`src/query/format.rs:230-232`).
- **Table Column/Header Parity**: Enforced at runtime in `render_table` (`src/query/format.rs:123-128`), returning `Err(QueryError::TableColumnCountMismatch)`.
- **Task Document Order Comparison Invariant Broken**: `QueryRow::cmp_document_order` compares `self.position.cmp(&other.position)` (`src/query/results.rs:431`). For multiple list/task rows within the same note, `position` (`RowIndex`) is identical; `item_idx` is ignored, violating document item order and making items in the same note equal during template sorting.

## 3. Verified Observations
- [V] **Dual Lexing/Parsing Architecture**: Lexing uses `logos` (`src/query/grammar/filter.rs:17`, `source.rs:14`). Winnow is NOT used in `src/query/**` (it is isolated to `src/temporal/lexer.rs:7`). Parsing is hand-rolled recursive descent (`src/query/grammar/expr.rs:154`).
- [V] **Expression Tree / Plan vs AST**: There is NO relational algebra or physical query planner. `ExecutionPlan` is a simple `Vec<QueryTransform>` (`src/query/plan.rs:34`). The only optimizations are local algebraic step fusions: adjacent filters fuse into `Filter(a.and(b))` (`src/query/plan.rs:63-80`), adjacent sorts fuse into multi-column `SortOrder` (`plan.rs:84-109`), and `Sort` + `Limit` fuses into `TopK` quickselect (`plan.rs:113-133`).
- [V] **`GroupBy` is an Alias for Sort**: `QueryTransform::GroupBy(path)` does not bucket, partition, or aggregate; it is implemented directly as `SortOrder::single(path, SortDirection::Ascending).sort(rows)` (`src/query/plan.rs:254-258`).
- [V] **Asymmetric Execution Strategy**:
  - `QueryService::run` (in-memory) performs a linear scan over all `0..index.entries().len()` (`src/query/service.rs:323-329`). It cannot use tag or class indexes because `WorkspaceIndex` contains only `entries: Box<[FileEntry]>` (`src/index/entry.rs:23`).
  - `QueryService::run_from_store` queries redb secondary tables (`paths_with_tag`, `paths_with_file_class`, `paths_in_folder`) via `SourceResolver` (`src/query/service.rs:136-137,399-432`), parallel-loads candidate notes via `rayon`, assembles a transient `WorkspaceIndex`, and re-evaluates `is_match` (`src/query/service.rs:162-167`).
- [V] **`BooleanExpr::Not` Fallback in Store**: In `SourceResolver::resolve_expr`, `BooleanExpr::Not(_)` falls back to scanning every path in the workspace (`src/query/service.rs:395`), defeating index pruning.
- [V] **Chained Transform Cache Invalidation**: `QuerySet::push` preserves `self.base` and `self.plan`, resetting `cache: OnceLock::new()` (`src/query/results.rs:527`). Calling `.len()` and then `.filter(...)` re-runs the entire plan from the root `self.base` rather than the cached intermediate rows.
- [V] **Implicit String Type Promotion**: `TextShape::classify` (`src/query/sort.rs:462`) heuristically detects ISO date, datetime, or duration shapes on arbitrary strings. Used during sort-key creation (`SortKey::from_text`, `src/query/sort.rs:394`) and filter literal promotion (`ComparisonExpr::classify_literal`, `src/query/grammar/filter.rs:238-248`), coercing strings to temporal types without schema checks.
- [V] **Direct Markdown Output Coupling**: `src/query/format.rs` (448 lines) hand-rolls GitHub-flavored Markdown tables with `unicode_width` column alignment (`format.rs:158-202`), bulleted lists (`format.rs:205-214`), and checkbox task lists (`format.rs:222-273`) directly inside the query module.
- [V] **Minijinja Diagnostic Loss**: `QuerySyntaxError` contains rich `miette::Diagnostic` source spans (`src/query/error.rs:113-125`). In `src/template/engine/query.rs:299,379`, `QueryError` is converted into `minijinja::Error` via `query_error`, stripping source span labels and rendering only the display string.

## 4. Hypotheses
- **H1: Unify In-Memory Querying with Inverted Index Pruning [H]**
  - *Thesis*: In-memory `QueryService::run` should not linearly scan `WorkspaceIndex` for selective tag or folder filters; maintaining secondary in-memory bitsets or maps (or routing through store indexes) enables sub-millisecond candidate pruning.
  - *Current vs Alt*: Current always does $O(N)$ linear scans in memory (`service.rs:323`). Alt: Inverted index or Roaring bitset over `WorkspaceIndex` entries.
  - *Removable*: Eliminates dual query execution code paths between `run` and `run_from_store`.
  - *Disproof Attempt*: In vaults <= 5,000 files, linear scan over contiguous memory takes <1ms; building/maintaining in-memory secondary indices adds memory and index-refresh latency that exceeds linear scan savings.
  - *Blast Radius*: `WorkspaceIndex`, `QueryService`, `IndexerService`.
- **H2: Fix Document Order Comparison Bug in `QueryRow` [H]**
  - *Thesis*: `QueryRow::cmp_document_order` must break ties between rows in the same note by comparing `item_idx` when `kind` is `RowType::List`.
  - *Current vs Alt*: Current: `self.position.cmp(&other.position)` (`results.rs:431`). Alt: `self.position.cmp(&other.position).then_with(|| self.item_idx().cmp(&other.item_idx()))`.
  - *Removable*: Removes non-deterministic task/list ordering bugs in templates and Minijinja custom comparison (`template/engine/query.rs:540`).
  - *Disproof Attempt*: Check if existing tests rely on list rows comparing equal.
  - *Blast Radius*: Isolated to `QueryRow::cmp_document_order` (`src/query/results.rs:427-432`).
- **H3: Consolidate Grammars: Source DSL (`--from`) and Filter DSL (`--where`) [H]**
  - *Thesis*: The separate lexers and ASTs in `source.rs` (1,189 lines) and `filter.rs` (2,123 lines) duplicate boolean parsing, string unquoting, and tag matching logic. Unifying into a single predicate language simplifies parser maintenance.
  - *Current vs Alt*: Keep as is: source operates on files/notes, filter operates on rows/fields. Alt: Single expression engine where `#tag` and `file.folder` are standard predicates.
  - *Removable*: Deduplicates `SourceToken`/`FilterToken` lexers, separate ASTs, and redundant `contains(tags, "#tag")` vs `#tag` logic.
  - *Disproof Attempt*: Source expressions must map directly to indexed storage keys (`paths_with_tag`, `paths_with_file_class`) before row materialization. Unifying them into a general row filter forces complex pushdown analysis to extract indexable predicates.
  - *Blast Radius*: High. Touches CLI flags, Template `.from()`, `QueryBuilder`.
- **H4: Decouple Markdown Output Formatting from `QuerySet` [H]**
  - *Thesis*: Markdown table, list, and task-list rendering (`src/query/format.rs`) belongs in a presentation/formatter module, not embedded directly on `QuerySet`.
  - *Current vs Alt*: Keep formatting methods on `QuerySet` for template ergonomics (`rows.table(...)`) vs extracting a standalone `Formatter` service.
  - *Removable*: Removes 450 lines of ASCII string layout, cell escaping, and `unicode_width` calculations from the core query engine.
  - *Disproof Attempt*: Minijinja ergonomics: template authors expect `.table(...)` as an immediate chaining method on query objects; moving it requires wrapper proxy objects.
  - *Blast Radius*: Moderate: `QuerySet`, `src/query/format.rs`, CLI commands (`table.rs`, `list.rs`, `task.rs`), `src/template/engine/query.rs`.
- **H5: Eliminate Transient `WorkspaceIndex::assemble` in `run_from_store` [H]**
  - *Thesis*: `QueryService::run_from_store` should stream rows directly from deserialized store records instead of allocating an intermediate `WorkspaceIndex` (`src/query/service.rs:162`).
  - *Current vs Alt*: Current constructs `WorkspaceIndex` solely so `QueryRow` can hold `Arc<WorkspaceIndex>`. Alt: Refactor `QueryRow` to decouple from whole-index ownership (e.g. lightweight record view or store reference).
  - *Removable*: Avoids intermediate `WorkspaceIndex::assemble` allocations, `SortedByPath` sorting, and link table synthesis.
  - *Disproof Attempt*: Zero-copy borrowing of fields (`QueryFieldValueRef`) requires note data to live in a stable backing store for the lifetime of query rows.
  - *Blast Radius*: `QueryRow`, `QuerySet`, `WorkspaceIndex`.
- **H6: Explicit vs Implicit Type Coercion (`TextShape`) [H]**
  - *Thesis*: Implicit promotion of strings to dates/durations via `TextShape::classify` causes surprising ordering and comparison semantics for non-temporal string fields that look like ISO dates.
  - *Current vs Alt*: Keep heuristic classification vs requiring schema annotations or explicit conversion functions (`date(field)`).
  - *Removable*: Simplifies `SortKey` generation and filter evaluation; removes heuristic sniffing.
  - *Disproof Attempt*: Markdown frontmatter is un-typed by nature; users write `due: 2026-07-29` without schemas and expect automatic chronological sorting.
  - *Blast Radius*: `src/query/sort.rs`, `src/query/grammar/filter.rs`.
- **H7: Clarify or Deprecate `GroupBy` [H]**
  - *Thesis*: `QueryTransform::GroupBy` is deceptive because it only performs ascending sorting without aggregation or grouped row structures (`src/query/plan.rs:254`). It should either implement real grouping or be deprecated in favor of `sort`.
  - *Current vs Alt*: Keep as sort alias for Dataview compatibility vs introduce genuine grouping/projection.
  - *Removable*: Eliminates misleading AST transform variant.
  - *Disproof Attempt*: Dataview users expect `group by` to cluster rows visually; in flat markdown lists/tables, sorting by group key is often sufficient when paired with template grouping filters.
  - *Blast Radius*: `ExecutionPlan`, `QuerySet::group_by`, Template query engine.

## 5. Cross-Slice Couplings
- **Index Coupling**: `QueryService` and `QueryRow` depend intimately on `WorkspaceIndex` and `IndexStore` (`src/query/service.rs:26-27`). `QueryRow` stores `Arc<WorkspaceIndex>` directly (`results.rs:45`), tying result row lifetimes to the entire index.
- **Note & Field Value Coupling**: Query field resolution bridges `NoteFieldValueRef` (`src/note/field.rs:179`) via `QueryFieldValueRef` (`src/query/value.rs:18`) to inject synthetic pseudo-fields (`tags`, `inlinks`). Total ordering in `SortKey` mirrors `NoteFieldType::rank` (`src/note/field.rs:238`).
- **Temporal Coupling**: Filter arithmetic and function calls depend on `src/temporal/` types (`DatePoint`, `DurationValue`, `CalendarPeriod`, `PeriodEdge`) (`src/query/grammar/filter.rs:34-37`). Period-boundary function names (`sow`, `eom`) share `PERIOD_EDGE_LOOKUP` with template filters (`filter.rs:514`).
- **Schema & `FileClassExpander` Seam**: `FileClassExpander` (`src/query/grammar/source.rs:353`) decouples `query` from `schema`. The concrete implementation is in `src/file_class_expander.rs:15`, preventing circular dependencies between schema inheritance and query selection.
- **Template Engine Object Bridge**: `src/template/engine/query.rs` bridges `QuerySet` and `QueryRow` into Minijinja's `Object` system, forwarding method calls and translating errors.

## 6. Comparables Worth Studying
- **Obsidian Dataview (DQL)**: Informs H3 and H7. Dataview separates source selection (`FROM`) from row filtering (`WHERE`), but provides true `GROUP BY` (creating `rows` sub-arrays).
- **DuckDB / DataFusion (Filter Pushdown & Inverted Indexes)**: Informs H1 and H3. Pushdown optimizer analyzes unified boolean expressions, extracting indexable scan keys (e.g. tag/path bounds) while executing remaining predicates as row filters.
- **Jaq / JQ (Stream Transformation & Lazy Evaluation)**: Informs H5 and H7. Zero-allocation row generators and streaming filters without intermediate index assembly.
- **Polars / Arrow (Projection & Array-backed Rows)**: Informs H5. Columnar or lightweight row views instead of 48-byte structs carrying `Arc<WorkspaceIndex>` clones.

## 7. Open Decisions
- **`decision-query-in-memory-pruning`**: Should `WorkspaceIndex` maintain secondary bitsets for tags/classes to accelerate in-memory source evaluation, or does linear scan remain fast enough for all realistic vault sizes?
- **`decision-query-dsl-unification`**: Should `--from` source selectors and `--where` filter expressions be consolidated into a single relational predicate AST with pushdown optimization?
- **`decision-query-group-by-semantics`**: Should `group_by` implement genuine multi-row bucketing/nesting, or be formally renamed/aliased to sort?
- **Fog**: The boundary between template query expressions and Minijinja's native filter pipeline (e.g. `{{ query.from(...).where(...) }}` vs `{{ query.from(...) | where(...) }}`) introduces cognitive duplication that requires design consensus.

## 8. Withstands Scrutiny
- **`FileClassExpander` Seam (`src/query/grammar/source.rs:353`)**: Evaluated adversarially as potentially superfluous indirection. **Sound**: It cleanly decouples `query` from `schema`, avoiding mutual dependencies while allowing File Class hierarchy expansion (`@Book*`) to leverage the schema DAG.
- **Top-K Quickselect Optimization (`src/query/plan.rs:113,279`)**: Evaluated adversarially. **Sound**: Automatically fusing adjacent `Sort` and `Limit` into `select_nth_unstable_by` reduces $O(N \log N)$ sorting to $O(N + K \log K)$, validated by benchmarks in `benches/query_sort.rs:216`.
- **Precomputed `SortKeys` Matrix (`src/query/sort.rs:63-77`)**: Evaluated adversarially. **Sound**: Flattening sort keys into a row-major buffer with fixed stride avoids repeated field resolution and string normalization during comparator evaluation.
