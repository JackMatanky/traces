# Web precedent: coherent reads and generation changes

This record gives external evidence for ticket 24. It does not choose a read
model. Each URL below was fetched. `[V]` marks a fact in the fetched source.
`[H]` marks an unmeasured implication for this project.

## State snapshots and host transitions

### 1. rust-analyzer architecture

Source: <https://rust-analyzer.github.io/book/contributing/architecture.html>

[V] `AnalysisHost` applies changes to the IDE state; `Analysis` is an immutable
snapshot of that state. A Salsa computation cancels when it sees a new revision,
and the IDE boundary converts cancellation to `Result<T, Cancelled>`.

Lesson for ticket 24: **Topic B, generation and host transition.** [H] A host
can give each request one generation and cancel stale requests.

### 2. Salsa revisions

Source: <https://salsa-rs.github.io/salsa/reference/algorithm.html>

[V] Salsa increments the database revision when an input changes. Tracked
functions record dependencies and reuse cached results when those inputs have
not changed.

Lesson for ticket 24: **Topic B, coherent generation.** [H] A revision number
alone does not choose how ticket 24 stores its read model.

### 3. Salsa durability

Source: <https://salsa-rs.github.io/salsa/reference/durability.html>

[V] Salsa durability describes how often an input changes. A tracked function
that uses only high-durability inputs can skip dependency checks after
low-durability inputs change.

Lesson for ticket 24: **Topic B, refresh cost.** [H] Durability is an
invalidation optimization, not evidence that a second query path is needed.

### 4. arc-swap

Source: <https://docs.rs/arc-swap/latest/arc_swap/struct.ArcSwapAny.html>

[V] `ArcSwapAny` reads and replaces a reference-counted pointer atomically
across threads. `load` gives a guard, `load_full` gives an owned pointer, and
`store` replaces the pointer.

Lesson for ticket 24: **Topic B, host transition.** [H] A pointer change can
expose a new immutable generation to later requests. It does not persist the
generation.

### 5. left-right

Source: <https://docs.rs/left-right/latest/left_right/>

[V] `left-right` keeps a reader copy and a writer copy, then flips them on
`publish`. The writer applies each logged operation twice and waits for existing
readers before it updates the old reader copy.

Lesson for ticket 24: **Topic B, host transition.** [H] Two copies and a single
writer add memory and update work. Their cost needs measurement for a large
vault.

## Query planning, rows and the catalog

### 6. DataFusion custom table providers

Source:
<https://datafusion.apache.org/library-user-guide/custom-table-providers.html>

[V] `TableProvider::scan` makes an execution plan during planning; the execution
stream produces record batches later. Projection, filter and limit hints let the
provider reduce the columns or rows that it reads.

Lesson for ticket 24: **Topic A, execution paths; Topic C, rows.**

[H] One row interface can have a store-backed source or a memory-backed source
without two result rules.

### 7. DataFusion filter contract

Source:
<https://docs.rs/datafusion/latest/datafusion/logical_expr/enum.TableProviderFilterPushDown.html>

[V] `Unsupported` does not push a filter to the provider. `Inexact` runs a
second filter after the scan; `Exact` does not run that second filter.

Lesson for ticket 24: **Topic D, secondary tables.** [H] Candidate sets with
false matches need a residual filter. Exact sets need matching query semantics.

### 8. DataFusion catalog

Source:
<https://docs.rs/datafusion/latest/datafusion/catalog/trait.CatalogProvider.html>

[V] A DataFusion catalog names schemas, and a schema names table providers. For
remote metadata, DataFusion first collects table references and places their
metadata in a cached snapshot for planning.

Lesson for ticket 24: **Topic C, rows.** [H] A result row belongs to its
selected table or generation. A catalog name alone does not own its row data.

### 9. Polars lazy optimizations

Source: <https://docs.pola.rs/user-guide/lazy/optimizations/>

[V] Polars can put predicates at scan level and select only the columns needed
at scan level. Its lazy optimizer also moves slice operations toward the scan
when that is valid.

Lesson for ticket 24: **Topic A, query plans; Topic D, facets.** [H] Scan
pruning can save work without changing the final row relation.

## Persistent search generations

### 10. Tantivy search example

Source: <https://tantivy-search.github.io/examples/basic_search.html>

[V] A long-lived Tantivy reader can reload after a commit, while each request
gets a searcher. A searcher points to an immutable index snapshot; using one
searcher for related queries keeps their index version equal.

Lesson for ticket 24: **Topic B, host transition.** [H] A held generation can
serve one request while a later request gets the new generation.

### 11. Tantivy segment ownership

Source: <https://docs.rs/tantivy/latest/tantivy/struct.Searcher.html>

[V] A Tantivy `Searcher` holds segment readers and keeps their segments alive
until the searcher is destroyed. Its `generation` identifies the version of the
held snapshot.

Lesson for ticket 24: **Topic B, generation lifetime.** [H] A request guard can
control when old generation data becomes free.

### 12. Lucene searcher manager

Source:
<https://lucene.apache.org/core/9_12_1/core/org/apache/lucene/search/SearcherManager.html>

[V] Lucene's `SearcherManager` shares searchers across threads and can open new
searchers during refresh. It closes an old searcher only after its users release
it.

Lesson for ticket 24: **Topic B, host transition.** [H] A host needs an explicit
lifetime rule when it replaces a generation.

## Database read transactions

### 13. LMDB transactions

Source: <https://lmdb.readthedocs.io/en/release/>

[V] LMDB supports concurrent readers and one writer, even across processes on
the same host. Its named databases share a transaction, and memory mapping
permits zero-copy lookup and iteration.

Lesson for ticket 24: **Topic B, store reads; Topic C, rows.** [H] Borrowed
database bytes need a live read transaction. Zero-copy bytes do not make a
decoded `Note` borrowable.

### 14. redb design

Source: <https://raw.githubusercontent.com/cberner/redb/master/docs/design.md>

[V] redb uses MVCC and copy-on-write structures for one writer and multiple
concurrent readers. It gives a read transaction its own root, and it retains
pages that active readers can still reach.

Lesson for ticket 24: **Topic B, writer admission and generation.** [H] A long
read transaction can retain old pages. This source does not establish the
project's second-process open behavior.

### 15. SQLite WAL

Source: <https://www.sqlite.org/wal.html>

[V] A SQLite WAL reader fixes an end mark at the start of its read transaction.
It reads only page versions before that mark, while a writer can append later
commits; long reads can hold back checkpoints.

Lesson for ticket 24: **Topic B, coherent store reads.** [H] One read
transaction can cover the query's related tables, but its lifetime has storage
costs.

## Cache update events and Rust row ownership

### 16. Obsidian metadata cache API

Source:
<https://raw.githubusercontent.com/obsidianmd/obsidian-api/master/obsidian.d.ts#L6446-L6533>

[V] `changed` reports an indexed file and its new cache, but it does not report
a rename. `deleted` carries a best-effort old cache; `resolve` reports link
resolution for one file; `resolved` reports completion across files.

Lesson for ticket 24: **Topic B, host transition; Topic D, inbound links.**

[H] These separate events do not establish a single atomic generation for cache
facts and resolved links.

### 17. Rust `Arc` ownership

Source: <https://doc.rust-lang.org/std/sync/struct.Arc.html>

[V] `Arc::clone` adds an atomic reference count to the same allocation; it does
not copy the inner value. `Arc::new` allocates the inner value, and atomic
reference counting costs more than ordinary memory access.

Lesson for ticket 24: **Topic C, row ownership.** [H] Each clone and drop of a
row-owned `Arc<Snapshot>` changes a reference count. Borrowing from a held
snapshot avoids that per-row count change. [H] Fully owned rows can outlive the
snapshot, but they copy selected data. Their allocation and memory costs need
measurement.
