# Architecture Redesign Survey: Query Subsystem

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

**Area**: Query Subsystem (`src/query/**`, `src/file_class_expander.rs`, `benches/query_*.rs`)  
**Author**: SurveyQuery  
**Status**: Completed  
**Date**: 2026-10-18  

---

## 1. Scope & Size

### 1.1 Source Files Covered

| File / Subsystem | Prod SLoC | Test SLoC | Total Lines | Primary Responsibilities |
| :--- | :---: | :---: | :---: | :--- |
| `src/query/mod.rs` | 90 | 49 | 139 | Module entrypoint, doc architecture, re-exports |
| `src/query/builder.rs` | 184 | 298 | 482 | `QueryBuilder`, `QueryMode` (`Pages`, `Lists`, `Tasks`), fluent transforms builder |
| `src/query/service.rs` | 457 | 851 | 1,308 | `QueryService` runner over `Arc<WorkspaceIndex>` and `IndexStore` (`SourceResolver`) |
| `src/query/plan.rs` | 18 | 553 | 571 | `ExecutionPlan`, algebraic fusion (filters, sorts, top-k quickselect) |
| `src/query/results.rs` | 550 | 1,418 | 1,968 | `QueryRow` (zero-copy view), `QuerySet` (lazy memoized collection), field resolution |
| `src/query/value.rs` | 181 | 386 | 567 | `QueryFieldValueRef` (ephemeral borrowed scalar bridge) |
| `src/query/sort.rs` | 170 | 948 | 1,118 | `SortOrder`, `SortTerm`, `SortKey`, `TextShape` heuristics, total order sorting |
| `src/query/format.rs` | 300 | 148 | 448 | Markdown rendering: `Table`, `List`, `TaskList`, cell escaping, `TaskPathStyle` |
| `src/query/error.rs` | 215 | 137 | 352 | `QueryError`, `QueryBuilderError`, `QuerySyntaxError`, `FieldPathError` |
| `src/query/grammar/mod.rs` | 25 | 0 | 25 | Grammar re-exports |
| `src/query/grammar/expr.rs` | 289 | 250 | 539 | Precedence climbing boolean algebra (`not` > `and` > `or`), `AtomParser` trait |
| `src/query/grammar/source.rs` | 681 | 508 | 1,189 | Logos lexer & parser for `--from`: `#tag`, `folder/`, `@Class`, glob matching |
| `src/query/grammar/filter.rs` | 1,233 | 890 | 2,123 | Logos lexer & evaluator for `--where`: arithmetic, date/durations, `contains` |
| `src/query/grammar/field.rs` | 247 | 353 | 600 | Accessor validation: `file.*`, `list.*`, `tags`, `inlinks`, metadata key |
| `src/file_class_expander.rs` | 40 | 47 | 87 | Concrete glue implementing `FileClassExpander` for `SchemaService` |
| `benches/query_*.rs` | 0 | 1,656 | 1,656 | Criterion suites: `query_execution`, `query_parsing`, `query_sort` |
| **Totals** | **4,680** | **8,492** | **13,172** | Subsystem is ~4.7K lines production Rust, ~8.5K lines tests/benches |

### 1.2 Public vs. Pub(crate) Surface

- **External Public (`pub`)**:
  - `traces::query::QueryBuilder` (`src/query/builder.rs:55`)
  - `traces::query::QueryService` (`src/query/service.rs:57`)
  - `traces::query::QuerySet` (`src/query/results.rs:464`)
  - `traces::query::QueryRow` (`src/query/results.rs:44`)
  - `traces::query::SourceSelector` (`src/query/grammar/source.rs:34`)
  - `traces::query::QueryError` (`src/query/error.rs:35`), `QueryBuilderError`, `QuerySyntaxError`, `QueryDialect`, `QueryResult`
- **Internal Crate Surface (`pub(crate)` / `pub(super)`)**:
  - `QueryMode` (`Pages`, `Lists`, `Tasks`) (`src/query/builder.rs:21`)
  - `ExecutionPlan`, `QueryTransform` (`src/query/plan.rs:33,143`)
  - `FileClassExpander`, `ClassExpansionMode` (`src/query/grammar/source.rs:353`)
  - `FieldPath`, `FileField`, `ListField`, `TaskField` (`src/query/grammar/field.rs:37,157,229`)
  - `FilterExpr`, `FilterAtom`, `ComparisonExpr`, `ValueExpr` (`src/query/grammar/filter.rs:42,138,196,253`)
  - `SortOrder`, `SortDirection`, `SortKey`, `TextShape` (`src/query/sort.rs:29,267,336,446`)
  - `QueryFieldValueRef` (`src/query/value.rs:18`)
  - `QueryDisplayFormat`, `TaskPathStyle` (`src/query/format.rs:16,29`)

---

## 2. Responsibilities

| Responsibility | Owning Type / Function | Verification (File:Line) |
| :--- | :--- | :--- |
| **Source Selection Parsing** | `SourceSelector::parse`, `SourceExpr::parse` | `src/query/grammar/source.rs:51,116` |
| **Filter Expression Parsing** | `FilterExpr::parse` | `src/query/grammar/filter.rs:54` |
| **Boolean Algebra & Precedence** | `expr::parse_boolean_expr`, `BooleanExpr<A>` | `src/query/grammar/expr.rs:50,274` |
| **Field Path Validation & Typo Hints** | `FieldPath::parse`, `FieldPathError` | `src/query/grammar/field.rs:68`, `src/query/error.rs:179` |
| **Query Specification** | `QueryBuilder` | `src/query/builder.rs:55-184` |
| **Plan Optimization & Algebraic Fusion**| `ExecutionPlan::run`, `fuse_filters`, `fuse_sorts`, `fuse_sort_limit` | `src/query/plan.rs:39-133` |
| **In-Memory Query Execution** | `QueryService::run`, `matched_file_rows` | `src/query/service.rs:104,318` |
| **Store-Backed Candidate Querying** | `QueryService::run_from_store`, `SourceResolver` | `src/query/service.rs:129,343` |
| **Row Instantiation (Pages/Lists/Tasks)** | `QueryService::pages`, `lists`, `tasks`, `item_rows` | `src/query/service.rs:201-313` |
| **Zero-Copy Field Access** | `QueryRow::resolve_ref`, `QueryFieldValueRef` | `src/query/results.rs:189`, `src/query/value.rs:18` |
| **Total Order & Multi-Column Sorting** | `SortOrder::sort`, `SortKeys`, `SortKey::cmp` | `src/query/sort.rs:63,103,137,425` |
| **Top-K Bounded Quickselect** | `QueryTransform::TopK`, `select_nth_unstable_by` | `src/query/plan.rs:279-318` |
| **Lazy Execution & Result Caching** | `QuerySet::rows` (`OnceLock<Arc<Vec<QueryRow>>>`) | `src/query/results.rs:483-491` |
| **Result Row Explosion** | `QueryTransform::Flatten` | `src/query/plan.rs:259-278` |
| **Markdown Output Rendering** | `QueryDisplayFormat::render`, `render_table`, `render_list`, `render_task_list` | `src/query/format.rs:88,118,205,222` |
| **Minijinja Object Interop** | `impl Object for QuerySet`, `impl Object for QueryRow` | `src/template/engine/query.rs:309,517` |

---

## 3. Invariants

1. **Shared Boolean Precedence (`NOT > AND > OR`)**:
   - *Enforced*: `BooleanExprParser` (`src/query/grammar/expr.rs:96-152`) through dedicated recursive descent tiers (`parse_or` calls `parse_and` calls `parse_not` calls `parse_atom`).
   - *Safe bypass*: None. Both `SourceExpr` and `FilterExpr` implement `AtomParser` and share this exact parser.
2. **Deterministic Total Ordering on Dynamic Values**:
   - *Enforced*: `SortKey::cmp` (`src/query/sort.rs:425-439`) and `NoteFieldValueRef::compare` (`src/note/field.rs:340`). Missing or unindexable values sort as `Null`. Unlike non-null variants rank deterministically (`Null < Bool < Number < Duration < Temporal < Text`). Floating numbers use `f64::total_cmp` after `normalize_zero` (`src/query/sort.rs:432`).
   - *Safe bypass*: None.
3. **Lazy Single Execution in QuerySet**:
   - *Enforced*: `QuerySet::rows()` uses `std::sync::OnceLock` (`src/query/results.rs:483-491`). Multiple calls to `.len()`, `.iter()`, `.table()`, or `.get()` evaluate `ExecutionPlan::run` at most once.
   - *Safe bypass*: Calling a mutating transform method (e.g. `.filter(...)`) returns a new `QuerySet` with a cleared `OnceLock` (`src/query/results.rs:524-529`), deferring evaluation of the extended pipeline.
4. **Task Formatting Requires Task Rows**:
   - *Enforced*: Runtime guard in `QueryDisplayFormat::render_task_list` (`src/query/format.rs:230-232`) returning `Err(QueryError::TaskListRequiresTaskRows)` if `row.task_text()` returns `None`.
   - *Safe bypass*: Type system allows calling `.task_list(...)` on a `QuerySet` created in `Pages` mode; invariant is enforced at runtime, not typestate.
5. **Table Header/Column Count Parity**:
   - *Enforced*: Runtime check in `QueryDisplayFormat::render_table` (`src/query/format.rs:123-128`) returning `Err(QueryError::TableColumnCountMismatch)`.
   - *Safe bypass*: Unchecked at builder time; only enforced at materialization.
6. **Task Document Ordering Tie-Breaking Bug in Document Order**:
   - *Observation*: `QueryRow::cmp_document_order` compares only `self.position.cmp(&other.position)` (`src/query/results.rs:431`). For task/list items from the same note, `position` is identical (`RowIndex` of the note in `WorkspaceIndex`). It fails to compare `item_idx`!
   - *Impact*: In minijinja, sorting rows by document order (or using `custom_cmp` via `src/template/engine/query.rs:540`) considers all list items in the same note equal, destabilizing or collapsing order.

---

## 4. State & Lifecycle

- **Owned State**:
  - `QueryBuilder`: Plain inert struct holding `QueryMode`, `SourceSelector`, and `ExecutionPlan` (`src/query/builder.rs:54-59`).
  - `QuerySet`: Holds `base: Arc<Vec<QueryRow>>`, `plan: ExecutionPlan`, and `cache: OnceLock<Arc<Vec<QueryRow>>>` (`src/query/results.rs:464-468`).
  - `QueryRow`: Holds `index: Arc<WorkspaceIndex>`, `position: RowIndex`, `flattened: Vec<(FieldPath, NoteFieldValue)>`, `kind: RowType` (`src/query/results.rs:44-50`).
  - `QueryService`: Holds `class_field: String` and `class_expander: Option<Arc<dyn FileClassExpander>>` (`src/query/service.rs:57-60`).
- **Concurrency & Parallelism**:
  - The query engine is synchronous.
  - Parallelism is used strictly via `rayon` in `QueryService::run_from_store` (`src/query/service.rs:138-158`) to batch-read notes, files, and links from `IndexStore` concurrently.
  - In-memory querying (`QueryService::run`, `src/query/service.rs:104-113`) is entirely single-threaded: iterates `0..index.entries().len()` on the caller thread.
- **Allocation & Caching Lifecycle**:
  - `QuerySet` acts like an immutable, copy-on-write pipeline. Transforms append `QueryTransform` to `ExecutionPlan` in $O(1)$ time without copying rows (`src/query/results.rs:521-529`).
  - Materialization clones `base` (`base.to_vec()`) and runs the fused plan once, memoizing the output `Arc<Vec<QueryRow>>` in `OnceLock`.

---

## 5. Dependency Edges

### 5.1 Outbound Module Edges (`use crate::...`)
- `crate::index::{FileEntry, IndexResult, IndexStore, RowIndex, SortedByPath, WorkspaceIndex}` (`src/query/service.rs:26`, `results.rs:27`)
- `crate::note::{ListItem, ListItemType, Note, NoteFieldValue, NoteFieldValueRef}` (`src/query/service.rs:25`, `results.rs:28`, `value.rs:14`, `sort.rs:22`)
- `crate::file::FileMeta` (`src/query/results.rs:26`)
- `crate::temporal::{CalendarError, CalendarPeriod, DatePoint, DateUnit, PeriodEdge}` (`src/query/grammar/filter.rs:34`, `sort.rs:23`)
- `crate::{DateTimeValue, DateValue, DurationValue, FieldKey, LexError, SourceLine, Spanned, SpannedTokenStream, Tag, TaskListItem, TaskPriority, TaskStatusSymbol, TimeUnit, TokenSpec}`
- `crate::strsim::closest_match` (`src/query/grammar/field.rs:24`)

### 5.2 Inbound Module Edges
- `crate::cli` -> `QueryService`, `QueryBuilder`, `SourceSelector`, `QuerySet`, `SortArgs` (`src/cli/mod.rs:335-366,470-488`, `src/cli/task.rs:88,124`)
- `crate::template::engine::query` -> `QueryService`, `QueryBuilder`, `QuerySet`, `QueryRow`, `SourceSelector` (`src/template/engine/query.rs:132,212,309,517`)
- `src/file_class_expander.rs` -> `QueryService`, `FileClassExpander`, `ClassExpansionMode` (`src/file_class_expander.rs:10-39`)

### 5.3 Test-Only Leaks / Gaps
- Multiple test helpers across `builder.rs`, `plan.rs`, `results.rs`, `sort.rs`, `service.rs`, and `format.rs` instantiate `IndexerService::for_tests(...)` directly to build an in-memory `WorkspaceIndex`.
- `QuerySet::r#where` (`src/query/results.rs:552`), `QuerySet::sort` (`results.rs:566`), and `QuerySet::order` (`results.rs:596`) are marked `#[cfg(test)]`.

---

## 6. Interfaces: Deep vs. Shallow

- **`QueryBuilder` / `QuerySet`**: **Deep seam**.
  - Callers (`cli`, `template::engine`) interact through a concise fluent API (`pages()`, `filter()`, `sort()`, `limit()`, `table()`).
  - Hides: logos lexing, AST construction, boolean precedence climbing, algebraic filter/sort/top-k fusion, and lazy memoized evaluation.
- **`FileClassExpander`**: **Shallow seam**.
  - `pub(crate) trait FileClassExpander: Send + Sync` (`src/query/grammar/source.rs:353`) defines a single method: `fn expand(&self, classes: &[String], mode: &mut ClassExpansionMode)`.
  - Implemented only once by `SchemaService` in `src/file_class_expander.rs`.
  - Is it real decoupling or ceremony?
    - *Evidence*: `query` does not depend on `schema`, and `schema` does not depend on `query`. The decoupling prevents a cyclic dependency between the query engine and schema inheritance engine. However, `FileClassExpander` is coupled to `ClassExpansionMode` (which is defined in `query/grammar/source.rs`).
- **`QueryDisplayFormat`**: **Shallow pass-through**.
  - Encapsulates `table`, `list`, and `task_list` string generation. While it implements a high-performance custom table formatter (`src/query/format.rs:105-117`), rendering Markdown strings directly inside the core query engine mixes data querying with terminal presentation.

---

## 7. Algorithms, Data Structures & Performance

1. **Top-K Quickselect Optimization**:
   - `fuse_sort_limit` rewrites `Sort` followed by `Limit(n)` into `QueryTransform::TopK` (`src/query/plan.rs:113-132`).
   - Implemented via `select_nth_unstable_by` ($O(n)$) followed by sorting the truncated $k$ prefix ($O(k \log k)$) (`src/query/plan.rs:303-305`).
   - Benchmarks in `benches/query_sort.rs:216-240` monitor this optimization against full sorting ($O(n \log n)$).
2. **Sort Key Materialization Matrix**:
   - `SortOrder::keys_for` (`src/query/sort.rs:63-77`) computes a row-major flat buffer of precomputed `SortKey`s with fixed stride $M$ (number of sort terms).
   - During sorting, comparator compares `keys.get(a_idx)` and `keys.get(b_idx)` (`src/query/sort.rs:144`), preventing redundant field lookups and conversions during the $O(n \log n)$ sort.
3. **Linear Scan in `QueryService::run`**:
   - In-memory execution scans every entry in `WorkspaceIndex` sequentially (`src/query/service.rs:323-329`):
     ```rust
     (0..index.entries().len())
         .map(RowIndex::new)
         .filter(|&position| source.is_match(index.entry_at(position), &self.class_field))
         .map(|position| QueryRow::new(index, position))
     ```
   - For an index of 50,000 notes, an unfiltered query evaluates every note; even for `#tag` queries, in-memory execution does NOT use an inverted tag index — it scans all notes!
4. **Store-Backed Candidate Optimization**:
   - Conversely, `QueryService::run_from_store` (`src/query/service.rs:129-169`) uses `SourceResolver` to query `IndexStore` tables (`paths_with_tag`, `paths_with_file_class`, `paths_in_folder`).
   - *Asymmetry*: In-memory queries do a full linear scan of `WorkspaceIndex`; cold-store queries use indexed lookups.
5. **Memory Overhead of `QueryRow`**:
   - `QueryRow` is sized at up to 112 bytes (`src/query/results.rs:788-793`). It contains `Arc<WorkspaceIndex>` (16 bytes), `RowIndex` (4 bytes), `flattened: Vec<(FieldPath, NoteFieldValue)>` (24 bytes), `kind: RowType` (8 bytes) + padding.
   - For 100,000 list/task items, rows alone consume >11 MB, and each row clones an `Arc<WorkspaceIndex>`.

---

## 8. Error Topology

- **Type Hierarchy**:
  - `QueryError` (`src/query/error.rs:35`): Unified enum with `Builder`, `Syntax`, `FieldPath`, `TaskListRequiresTaskRows`, `TableColumnCountMismatch`.
  - `QueryBuilderError` (`src/query/error.rs:91`): Separates syntax/fieldpath errors from runtime formatting.
  - `QuerySyntaxError` (`src/query/error.rs:113`): Contains `miette::Diagnostic` with `#[source_code]`, `#[label]`, and exact `SourceSpan`.
  - `FieldPathError` (`src/query/error.rs:179`): Formats unknown field accessors with edit-distance suggestion hints via `strsim::closest_match`.
- **Context Loss / Conversion Points**:
  - In `QueryService::run_from_store`, errors from `IndexStore` are returned as `IndexResult<QuerySet>`, whereas `QueryService::run` returns a raw `QuerySet` (panics are avoided by design; invalid filter/sort syntax is caught at builder time).
  - In `template/engine/query.rs:379`, `QueryError` is converted into generic `minijinja::Error` (`ErrorKind::InvalidOperation`), stripping miette spans and source code annotations before returning to the template runner!

---

## 9. Duplicated / Conflicting Concepts

1. **`TextShape` Heuristics vs. Schema Typing**:
   - `TextShape::classify` (`src/query/sort.rs:462`) heuristically sniffs free text to see if it looks like a `DateTime`, `Date`, or `Duration`.
   - Used during sorting (`SortKey::from_text`) and filtering (`ComparisonExpr::classify_literal`, `src/query/grammar/filter.rs:238`).
   - If a user has a string field containing `"2026-07-29"`, the query engine automatically promotes it to a `Date`, changing comparison and sort semantics implicitly without consulting the note's schema.
2. **`QueryFieldValueRef` vs. `NoteFieldValueRef`**:
   - `NoteFieldValueRef` (`src/note/field.rs:179`) represents borrowed frontmatter/inline field values.
   - `QueryFieldValueRef` (`src/query/value.rs:18`) wraps `NoteFieldValueRef` but adds `Tags(&'a [Tag])`, `Inlinks(&'a [PathBuf])`, and `Owned(NoteFieldValue)`.
   - This bifurcation exists because `tags` and `inlinks` are synthetic pseudo-fields not stored in the note's frontmatter map.
3. **Double Evaluation: `SourceSelector` vs. `FilterExpr`**:
   - Both support tag matching: `--from '#book'` uses `SourceAtom::Tag` (`src/query/grammar/source.rs:191`), while `--where 'contains(tags, "#book")'` or `--where 'file.tags == ...'` uses `FilterFunction::Contains` (`src/query/grammar/filter.rs:156`).
   - They use different tokenizers, different ASTs, and different matching implementations for the same conceptual check.

---

## 10. Doc, Glossary & ADR Drift

1. **ADR-0008 (Source Expression Grammar)**:
   - *Claim*: `@Class` atoms match notes tagged with child classes if sci-fi extends book (ADR-0008:70-73).
   - *Verified*: Verified in `SourceAtom::Class` (`src/query/grammar/source.rs:224-230`) and `FileClassExpander` (`src/file_class_expander.rs:15-38`). However, `SourceResolver::resolve_expr` for `BooleanExpr::Not(_)` falls back to scanning every path in the folder (`src/query/service.rs:395`), ignoring inverted class indexes.
2. **ADR-0009 (Filter Date Literals)**:
   - *Claim*: Number-only arithmetic (`1 + 2 == 3`) fails loudly at query-build time (`ADR-0009:37-38`).
   - *Verified*: Verified in `src/query/grammar/filter.rs:301-315` (`ValueExpr::is_number_only`) and tested in unit tests.
3. **GLOSSARY.md Drift**:
   - *Claim*: "Execution Plan: The ordered transform sequence a query applies to rows, fused where possible... and executed lazily on first read" (`src/query/GLOSSARY.md:38`).
   - *Verified*: Holds true. But note that `QueryBuilder` also executes initial row materialization eagerly when `QueryService::run` is called; only transforms added to `QuerySet` are executed lazily on first read of `QuerySet`.
4. **Planned LSP Map Drift (`.scratch/md-pkm-lsp/map.md:34`)**:
   - *Claim*: Cites linear scan in `QueryService::execute` / `run_from_store` at line 318-329.
   - *Verified*: In `src/query/service.rs`, `matched_file_rows` is indeed at lines 318-329. The in-memory scan is $O(N)$ across all entries in `WorkspaceIndex`.

---

## 11. Candidate Architectural Questions

### 1. In-Memory Inverted Index vs. Linear WorkspaceIndex Scan
- **Hypothesis**: `QueryService::run` should utilize inverted index lookups (tags, classes, folders) in memory rather than linearly scanning all `WorkspaceIndex::entries()`.
- **Evidence**: `QueryService::matched_file_rows` (`src/query/service.rs:323-329`) iterates over `0..index.entries().len()` on every query. On a 20,000-note vault, a selective query like `--from "#urgent"` examines all 20,000 entries. In contrast, `run_from_store` (`src/query/service.rs:136-137`) uses `store.paths_with_tag(...)` for instant index lookups.
- **Counter-argument**: In-memory `WorkspaceIndex::entries()` is a contiguous, cache-friendly slice (`SortedByPath<FileEntry>`). For typical vaults (1K–5K notes), a linear pass evaluating tag bitflags or slice lookups runs in <1ms without the memory overhead of maintaining secondary inverted multimaps in memory.
- **Decision Open**: Should `WorkspaceIndex` maintain in-memory inverted indices for tags/classes, or should queries against large vaults route through `IndexStore` / an explicit indexing structure?
- **Touches**: Query, Index, LSP.

### 2. Dual Grammar Duplication: Source Expressions (`--from`) vs. Filter Predicates (`--where`)
- **Hypothesis**: The separate grammars for `--from` and `--where` should be unified into a single expression AST and evaluation engine.
- **Evidence**: Currently, `source.rs` (1,189 lines) and `filter.rs` (2,123 lines) have separate logos tokenizers, separate ASTs (`SourceAtom` vs `FilterAtom`), and separate boolean evaluation paths, despite both expressing boolean combinations of note attributes (tags, folders, classes).
- **Counter-argument**: Source selection operates at the document level (determining *which files to load/parse* into rows), whereas filter expressions operate at the row level (evaluating properties of notes, list items, and tasks). Conflating them would complicate document-level index pruning.
- **Decision Open**: Keep source selection as a dedicated index-filtering DSL and filter predicates as a row-filtering DSL, or unify them into a single relational/predicate algebra?
- **Touches**: Query, Template, CLI.

### 3. QueryRow Fat Representation vs. Indexed Lightweight Projection
- **Hypothesis**: `QueryRow`'s internal structure (`Arc<WorkspaceIndex>`, `RowIndex`, `Vec<(FieldPath, NoteFieldValue)>`, `RowType`) is too heavy (112 bytes) and causes excessive pointer chasing and reallocation overhead.
- **Evidence**: `QueryRow` is 112 bytes (`src/query/results.rs:788-793`). In list/task queries with 50K items, 50K `Arc` clones are created. Flattening adds a heap-allocated `Vec` per row (`results.rs:48`). Furthermore, `QueryRow::cmp_document_order` (`results.rs:431`) has a bug where it only compares note `position` and ignores `item_idx`, breaking stable ordering for list items.
- **Counter-argument**: Storing `Arc<WorkspaceIndex>` and `RowIndex` allows zero-copy borrowing of note fields (`QueryFieldValueRef`), avoiding copying note strings into query results until formatting.
- **Decision Open**: Refactor `QueryRow` into a lightweight 16-byte index pair `(RowIndex, u32)` that borrows `WorkspaceIndex` from the parent `QuerySet` rather than storing an `Arc` in every row, and fix document order comparison to include `item_idx`.
- **Touches**: Query, Template.

### 4. Rendering Responsibility: Subsystem Coupling vs. Standalone Formatters
- **Hypothesis**: Markdown table, list, and task rendering (`src/query/format.rs`, 448 lines) should be separated from query evaluation and moved to an output/presentation formatting layer.
- **Evidence**: `QuerySet` directly owns `.table()`, `.list()`, and `.task_list()` (`src/query/results.rs:647-683`), which formats GFM tables, padded columns, and Unicode width strings. This couples query data materialization to ASCII/Markdown rendering.
- **Counter-argument**: Having `.table()` and `.list()` on `QuerySet` provides an extremely ergonomic API for templates (`{{ pages.from("#book").table(...) }}`) and CLI commands, avoiding intermediate wrapper objects.
- **Decision Open**: Keep formatting methods directly on `QuerySet` or extract rendering into an orthogonal formatting trait/module?
- **Touches**: Query, Template, CLI.

### 5. `FileClassExpander` Seam: Essential Decoupling vs. Unnecessary Abstraction
- **Hypothesis**: The `FileClassExpander` trait in `src/query/grammar/source.rs` and its implementation in `src/file_class_expander.rs` is an artificial seam that can be simplified.
- **Evidence**: `FileClassExpander` has exactly one production implementation (`SchemaService`). It exists solely to avoid having `query` import `schema` or vice versa. But `QueryService` requires `Arc<dyn FileClassExpander>` (`src/query/service.rs:59`), incurring dynamic dispatch.
- **Counter-argument**: It strictly preserves a clean directed acyclic graph: `query` and `schema` are completely independent sibling modules. Neither depends on the other. Removing the seam would create a tight coupling or cyclic dependency.
- **Decision Open**: Verdict: **Keep as-is**. The seam is clean, narrow (1 method), and successfully prevents module coupling.
- **Touches**: Query, Schema.

### 6. Heuristic Type Sniffing (`TextShape`) vs. Strict Typing
- **Hypothesis**: Automatic classification of arbitrary strings into `Date`, `DateTime`, or `Duration` (`TextShape::classify`) creates unpredictable query comparison and sort behavior.
- **Evidence**: `TextShape::classify` (`src/query/sort.rs:462`) is invoked transparently on all string fields during sorting and literal classification. A string property that happens to match an ISO date syntax is sorted as a temporal instant rather than lexicographically, violating the principle of least astonishment for unstructured metadata.
- **Counter-argument**: Frontmatter values often lack explicit type annotations. Users expect dates written as `due: 2026-07-29` to sort chronologically and compare with date literals without manual casting functions.
- **Decision Open**: Maintain implicit `TextShape` coercion, or make type promotion explicit via query functions (e.g., `date(field)`)?
- **Touches**: Query, Schema, Temporal, Note.
