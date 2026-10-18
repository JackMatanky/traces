# Query Plan Precedent: Predicate Languages, Plans, and Pushdown

## 1. What Each Comparable Actually Does (Verified from Source)

### DataFusion (apache/datafusion)

- **Unified Predicate Representation [V]**: All boolean filters are expressed as `datafusion_expr::Expr` (`datafusion/expr/src/expr.rs:60`). There is no separate "source selection" syntax versus "row filter" syntax.
- **Pushdown Capability Protocol [V]**: `TableSource::supports_filters_pushdown(&self, filters: &[&Expr]) -> Result<Vec<TableProviderFilterPushDown>>` (`datafusion/expr/src/table_source.rs:110-120`) returns a ternary judgment per predicate: `Exact` (storage guarantees exact pruning, filter removed from plan), `Inexact` (storage prunes containers/ranges, but filter retained above scan to check rows), or `Unsupported` (storage ignores filter) (`datafusion/expr/src/table_source.rs:37-49`).
- **Pushdown Optimizer [V]**: In `PushDownFilter` (`datafusion/optimizer/src/push_down_filter.rs:1330-1360`), non-volatile filters are passed to `supports_filters_pushdown`. Exact filters are absorbed into `LogicalPlan::TableScan.filters`, inexact filters are passed into scan AND kept in `LogicalPlan::Filter`, and unsupported filters remain solely in `Filter`.
- **Pruning Statistics [V]**: `PruningPredicate` (`datafusion/pruning/src/pruning_predicate.rs:160-220`) compiles an arbitrary `Expr` into a rewritten statistics expression (min/max/null counts from metadata) to skip entire containers (e.g. Parquet row groups) without reading raw data.

### Polars (pola-rs/polars)

- **Lazy Pipeline Construction [V]**: `LazyFrame` (`crates/polars-lazy/src/frame/mod.rs:48-56`) wraps a `DslPlan` and optimization flags (`OptFlags`). Chaining `.filter()`, `.select()`, `.sort()` produces no work, only building an unoptimized syntax tree.
- **Arena-based IR & Optimization [V]**: Calling `.collect()` lowers `DslPlan` into arena-allocated intermediate representation nodes (`Arena<IR>`, `Arena<AExpr>`) (`crates/polars-plan/src/plans/ir/mod.rs:56-110`).
- **Predicate Pushdown Pass [V]**: `PredicatePushDown` (`crates/polars-plan/src/plans/optimizer/predicate_pushdown/mod.rs:361-425`) traverses IR nodes downward, accumulating predicates in an index map. At `IR::Scan`, it checks `function.allows_predicate_pushdown()`, combines accumulated predicates using `combine_predicates`, and attaches them to `Scan.predicate`. Predicates referencing virtual columns or blocked projections remain as local filter nodes.

### jaq / jq (01mf02/jaq)

- **Generator Stream Transducer [V]**: jaq has no relational plan, no IR, and no optimizer. Every filter is a compiled generator function `(Ctx, Val) -> impl Iterator<Item = Result<Val, Error>>` (`jaq-core/src/lib.rs:25-45`, `jaq-core/src/filter.rs:15-60`).
- **Uniform Filtering Model [V]**: Filtering is not a boolean check on a row struct; it is an iterator that yields 0 items (when false) or 1 item (when true). Projections yield 1 item; unnesting/flattening (`.[]`) yields $N$ items. Composition (`|`) is direct iterator chaining.

### Obsidian Dataview (blacksmithgu/obsidian-dataview)

- **Two-Stage Source vs Where [V]**:
  - `FROM` evaluates candidate paths via metadata indices: `matchingSourcePaths` (`src/data-index/resolver.ts:16-55`) executes tags against `index.tags.getInverse()` (inverted index mapping `#tag` to `Set<string>`), folders against `index.prefix.get()` (trie), links against `metadataCache.resolvedLinks`, and binary operations (`&`, `|`) as set intersection/union. It never inspects page bodies or frontmatter fields.
  - `WHERE` evaluates general expressions row-by-row on deserialized page records (`src/query/engine.ts:56-68`), dropping rows whose evaluated expression is not truthy.
- **True GroupBy Semantics [V]**: `case "group"` (`src/query/engine.ts:121-177`) sorts rows by key, aggregates contiguous matching items, and restructures each row into `{ key: Literal, rows: DataObject[] }`, altering row schema and updating `identMeaning = { type: "group" }`. Subsequent pipeline stages must address fields via `rows.field`.

### Obsidian Tasks (obsidian-tasks-group/obsidian-tasks)

- **Line-Oriented Regex Grammar [V]**: Tasks uses no AST or formal parser (`src/Query/Query.ts:27-80`). Queries are parsed line-by-line via regular expressions into closures: `_filters: Filter[]`, `_sorting: Sorter[]`, `_grouping: Grouper[]`.
- **Global In-Memory Scan [V]**: There is no source selection or index pruning. `applyQueryToTasks` (`src/Query/Query.ts:360-400`) iterates over all vault tasks in memory (`tasks.filter(...)`), applies multi-column sort, applies slice limit, and constructs visual `TaskGroups` for section rendering.

### SQLite (sqlite/sqlite)

- **Unified WHERE Analysis [V]**: SQLite parses a single `WHERE` expression. `sqlite3WhereSplit` (`src/whereexpr.c:50-80`) breaks top-level `AND` terms into a flat `WhereClause` array.
- **Constraint Index Matching vs Residual Filtering [V]**: `exprAnalyze` inspects each `WhereTerm` against available indices. The query planner (`src/where.c:1-60`) builds index search loops for matched terms; unmatched terms become residual filter checks executed against rows emitted by the index.

## 2. Invariants and Who Owns Them

- **Filtering Soundness [V]**: DataFusion owns this in the optimizer (`PushDownFilter`). If storage answers `Inexact`, the optimizer guarantees correctness by keeping an identical filter node in the plan. Storage is never trusted to be exact unless it explicitly signs `Exact`.
- **Query Spec Immutability [V]**: In Polars, `LazyFrame` mutations return a new `LazyFrame` referencing an immutable plan or new AST node. In Traces, `QueryTransform` operations mutate or consume `ExecutionPlan`, but intermediate evaluation in templates eagerly clones `QuerySet` (`src/template/engine/query.rs:398`).
- **Path Resolution Confinement [V]**: Dataview enforces that all candidate generation in `matchingSourcePaths` resolves strictly to normalized vault paths before page data is fetched (`src/data-index/resolver.ts:127-142`).
- **Grouping Row Identity [V]**: Dataview maintains the invariant that grouping changes the row cardinality and row type (`src/query/engine.ts:175`). In Traces, `GroupBy` is an invariant violation: it claims to group but silently acts as a sort (`src/query/plan.rs:254-257`).

## 3. Tradeoffs and Costs They Pay

- **DataFusion / Polars**:
  - *Cost*: Enormous conceptual and memory footprint. Polars maintains separate `DslPlan`, `IR` arena, expression arena, and multi-pass optimizer (`crates/polars-plan/src/plans/optimizer/mod.rs`).
  - *Tradeoff*: Justified for GB-TB datasets with distributed partitions, but compilation and planning take microseconds to milliseconds—unacceptable overhead if a personal vault query in Traces takes only 300µs total.
- **Dataview**:
  - *Cost*: Two distinct mental models (`FROM` source vs `WHERE` expressions). Users cannot use general field expressions in `FROM` (e.g. `FROM status = "active"` is illegal; only tags, folders, links).
  - *Tradeoff*: Massive speed boost for vault startup: candidate file sets are pruned via in-memory inverted sets in <1ms without parsing markdown files.
- **jaq / jq**:
  - *Cost*: Pure linear evaluation without indices or pushdown. No ability to skip scanning unneeded files.
  - *Tradeoff*: Extreme elegance, streaming execution, zero AST optimization passes, no plan compilation.
- **Obsidian Tasks**:
  - *Cost*: Zero query optimization, no index utilization, no boolean `OR` without complex regexes.
  - *Tradeoff*: Zero parser complexity; instantaneous evaluation for vaults under 10k tasks.

## 4. Transferable Lessons for Traces

1. **Storage Pushdown Protocol via Capabilities [H]**:
   - Traces currently tightly couples `QueryService` to redb tables via `SourceResolver` (`src/query/service.rs:136-168`).
   - *Transferable*: Adopt DataFusion's principle of querying storage capabilities (`Exact`, `Inexact`, `Unsupported`). Whether evaluating against an in-memory `WorkspaceIndex` or redb, query execution can pass candidate predicates down to an index prune method. Unhandled or inexact predicates fall back to row-level evaluation.
2. **Unified Expression Grammar with Term Splitting [H]**:
   - SQLite and DataFusion disprove the need for separate `--from` (1,189 LoC) and `--where` (2,123 LoC) grammars.
   - *Transferable*: A single expression language where `#tag`, `folder("...")`, and `link("[[...]]")` are simply built-in predicates or field checks. The query planner splits top-level `AND` conjuncts (like SQLite `whereSplit`): terms matching tag/folder/class indices are pruned via bitsets or redb keys; remaining terms run as row filters.
3. **Lazy Query Builder in Template Engine [H]**:
   - Polars' `LazyFrame` demonstrates that builder methods (`.filter()`, `.sort()`, `.limit()`) should not execute eagerly.
   - *Transferable*: In `src/template/engine/query.rs`, `query.from()` should return a lazy `QueryBuilderObject` wrapping a `QuerySpec`. Chained calls `.filter()`, `.sort()`, `.limit()` simply update fields in `QuerySpec` (or push AST nodes). Execution occurs only upon terminal methods (`.table()`, `.list()`, `.task_list()`, or `into_iter()`). This eliminates eager `QuerySet` cloning (`src/template/engine/query.rs:398`) and enables Top-K sort-limit fusion across template boundaries.
4. **Clarify GroupBy as Heading Partitioning or Aggregate Projection [H]**:
   - Dataview and Tasks show that grouping in personal PKM outputs is either (a) structural nesting into `{ key, rows }` (Dataview) or (b) visual sectioning with markdown headers per group (Tasks).
   - *Transferable*: Traces' current `QueryTransform::GroupBy` that merely sorts rows ascending (`src/query/plan.rs:254-257`) is deceptive. It must either be removed (aliased to `sort`), or made to produce distinct group sections in output formatters (`format.rs`).

## 5. Lessons That Do NOT Transfer and Why

1. **Heavy Relational IR / Multi-Pass Optimizer (DataFusion/Polars) [H]**:
   - Personal vaults contain 1,000 to 20,000 notes. Full query execution in memory takes 200µs to 2ms.
   - Allocating arena-based IRs, expression trees, and running multi-phase optimizer passes (projection pushdown, type coercion, CSE) would take more CPU time than the scan itself.
   - *Why it fails*: Scale mismatch. Traces needs an algebraic AST simplifier, not a distributed relational planner.
2. **Pure Generator Model (jaq) [H]**:
   - jaq evaluates everything as an unindexed stream.
   - *Why it fails*: Traces cannot afford to deserialize 20,000 notes from disk or redb on every CLI command just to find 3 notes tagged `#urgent`. Candidate pruning before row materialization is load-bearing.
3. **Line-Oriented Regex Pipeline (Tasks) [H]**:
   - Obsidian Tasks lacks boolean combinations (`A and (B or C)`), arithmetic expressions, and field comparisons.
   - *Why it fails*: Traces CLI queries and templates require rich expressions (`file.mtime > date("now") - duration("7d")`), which cannot be cleanly expressed with line regexes.

## 6. What This Changes or Challenges in Traces Hypotheses

### Challenges H3 (`survey/query.md`): Unify `--from` and `--where` Grammars [H]

- *Survey Proposal*: Consolidate `source.rs` and `filter.rs` into a single predicate language.
- *Finding*: Upheld and strengthened by SQLite and DataFusion [V]. SQLite demonstrates that users write one `WHERE` clause, and the planner splits indexable terms from residual filter terms. Dataview separated `FROM` only as an artifact of Obsidian's API constraints. Unifying into a single expression AST eliminates ~1.5k LoC of duplicate boolean and token logic.

### Challenges H1 (`survey/query.md`) & H1 (`survey/index.md`): In-Memory Pruning vs Secondary redb Tables [H]

- *Survey Question*: Should `WorkspaceIndex` maintain secondary bitsets for tags/classes, or does linear scan suffice?
- *Finding*: Dataview achieves sub-millisecond in-memory pruning across 10k notes using simple in-memory `Map<Tag, Set<Path>>` (`src/data-index/resolver.ts:25`). Roaring bitmaps or `HashSet<Path>` over in-memory entries avoid full 20k-entry scans.
- *Pushdown Insight*: DataFusion's `TableProviderFilterPushDown` shows that storage (whether redb or `WorkspaceIndex`) can provide candidate pruning without knowing query execution internals. If redb stores inverted tables, it returns candidate paths (`Inexact`); `QueryService` loads only those notes and runs residual filters. This removes the need to assemble a transient `WorkspaceIndex` during `run_from_store`.

### Challenges H5 (`survey/query.md`): Eliminate Transient `WorkspaceIndex::assemble` [H]

- *Survey Proposal*: Stream rows directly from deserialized store records instead of allocating intermediate `WorkspaceIndex`.
- *Finding*: Upheld [V]. Traces currently constructs `WorkspaceIndex` (`src/query/service.rs:162-166`) solely because `QueryRow` holds an `Arc<WorkspaceIndex>`. In Polars and DataFusion, row/batch references do not own the catalog/table provider. Decoupling `QueryRow` from `Arc<WorkspaceIndex>` allows direct row streaming from redb without synthesis.

### Challenges H7 (`survey/query.md`): Clarify or Deprecate `GroupBy` [H]

- *Survey Proposal*: Clarify or deprecate `QueryTransform::GroupBy`.
- *Finding*: Strongly challenged and clarified [V]. In Dataview (`src/query/engine.ts:145-175`), `GROUP BY` restructures rows into an array under a key. In Tasks (`src/Query/Query.ts:385`), grouping creates visual section headings. Traces' current behavior (`SortOrder::single(path, Ascending).sort(rows)`) is neither; it is literally `Sort`.
- *Implication*: Either rename/deprecate `group_by` to `sort`, or define its semantics in formatters (`format.rs`) to emit markdown headings/sub-tables per group.

### Challenges H3 (`survey/template.md`): Zero-Copy Query Builder & Pipeline Pushdown [H]

- *Survey Proposal*: Replace eager `QuerySet` cloning with a lazy builder pipeline in minijinja.
- *Finding*: Directly validated by Polars' `LazyFrame` and DataFusion's `DataFrame` [V]. When `query.from(...)` is called in a template, it should construct a lightweight builder object. Method chaining (`.filter()`, `.sort()`, `.limit()`) mutates the builder state. Only terminal formatting (`.table()`, `.list()`) or iteration triggers query execution. This completely eliminates intermediate `Vec<QueryRow>` allocations (`src/template/engine/query.rs:398`).
