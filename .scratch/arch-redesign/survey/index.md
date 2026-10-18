# Subsystem Survey: Index (`src/index/`)

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

## 1. Scope & Size
- **Modules/files**: `src/index/` contains 12 Rust source files: `mod.rs` (88 lines), `entry.rs` (375 lines), `service.rs` (1,841 lines), `store.rs` (2,765 lines), `refresh.rs` (683 lines), `delta.rs` (352 lines), `inlinks.rs` (1,358 lines), `trie.rs` (689 lines), `tables.rs` (191 lines), `codec.rs` (207 lines), `sort.rs` (223 lines), `error.rs` (282 lines). Plus `GLOSSARY.md`.
- **SLoC**: Total lines: 9,054 across `src/index/`. Tests embedded in these files account for ~4,800 lines (~53%), leaving production code at ~4,250 lines. `store.rs` alone is 2,765 lines (production ~1,700 lines, tests ~1,065 lines).
- **Public vs pub(crate) surface**:
  - `pub` (at crate root / external): `WorkspaceIndex`, `FileEntry`, `IndexerService`, `IndexError`, `IndexResult`. Under `test-utils` / test cfg only: `InlinkMap`, `RefreshReport`.
  - `pub(crate)` (internal to traces): `RowIndex` (`entry.rs`), `IndexStore` (`store.rs`), `SortedByPath<T>` (`sort.rs`). Everything else is `pub(super)` or private within `src/index/`.

---

## 2. Responsibilities
Verified responsibilities and owning types/functions:
1. **Filesystem Scanning & File Metadata Extraction**: `IndexerService::scan` (`service.rs:324-345`) walks the directory tree skipping `.git`, symlinks, and `.traces/index.redb`, producing path-sorted `Vec<FileMeta>` in parallel using `rayon`.
2. **Note Markdown Parsing & Config Binding**: `IndexerService::parse_note` (`service.rs:261-277`) and `parse_notes` (`service.rs:251-257`) reads Markdown files from disk and calls `crate::parse_markdown` with `MarkdownParserInput` configured with `tasks` and `frontmatter` configs.
3. **In-Memory Snapshot Aggregation**: `WorkspaceIndex::assemble` (`entry.rs:39-45`) merge-joins path-sorted `FileMeta`, path-sorted `Note`, and `InlinkMap` into a contiguous `Box<[FileEntry]>`.
4. **Wikilink Resolution & Inlink Graph Derivation**:
   - `inlinks::LinkResolver` (`inlinks.rs:319-419`) indexes all file paths and basenames.
   - `trie::BaseNameIndex` / `CandidateTrie` (`trie.rs:17-150`) resolves ambiguous basenames via folder distance to Lowest Common Ancestor (LCA); switches from flat scan to arena trie when candidates $\ge 64$.
   - `InlinkMap::with_resolver` (`inlinks.rs:83-99`) maps outlinks to deduplicated target-source edges in parallel via `rayon`, sorting and grouping into target-keyed adjacency lists.
5. **Freshness Assessment & Delta Computation**:
   - `RefreshPlan::collect` (`refresh.rs:281-299`) scans disk and reads stored files concurrently via `rayon::join`.
   - `FileDelta::compute` (`delta.rs:23-65`) performs a 2-way merge-join between current and persisted path-sorted file metadata to determine upserted vs deleted files.
   - `InlinkDelta::compute` (`delta.rs:97-131`) and `InlinkMap::patch_and_diff` (`inlinks.rs:217-270`) compute edge mutations.
6. **Refresh Orchestration & Lifecycle Typestate**:
   - `IndexerService::refresh_with_report` (`service.rs:125-146`) and `refresh_store` (`service.rs:219-238`) coordinate the lifecycle states (`RefreshPlan` $\to$ `RefreshState` [Fresh | Stale] $\to$ `PendingApply` $\to$ `Persisted` | `PersistFailed`).
7. **Storage Schema & redb Management**:
   - `tables::TABLES` (`tables.rs:100-133`) defines 8 redb tables: `files`, `notes`, `links` (multimap), `paths_by_tag` (multimap), `paths_by_file_class` (multimap), `tags_by_path` (multimap), `classes_by_path` (multimap), and `cache_format`.
   - `IndexStore::open` (`store.rs:156-179`) handles schema drift detection and wipe/rebuild recovery via `rebuild_needed` (`store.rs:823-878`).
8. **Row-Level Persistence & Deserialization**:
   - `codec::encode_row` / `decode_row` (`codec.rs:27-55`) serializes/deserializes using `postcard`.
   - `IndexStore::persist` (`store.rs:654-681`) applies full rebuild or incremental writes (`apply_rebuild` / `apply_incremental`) with disabled durability (`redb::Durability::None`, `store.rs:1159`).
9. **Query Index Lookups**:
   - `IndexStore::paths_with_tag`, `paths_with_file_class`, `paths_in_folder` (`store.rs:266-309`) allow `QueryService` to query specific paths directly from redb multimaps without loading the full index.

---

## 3. Invariants
1. **WorkspaceIndex Entry Ordering & Totality**:
   - *Invariant*: `WorkspaceIndex::entries` contains exactly one `FileEntry` for every regular file in the project, strictly sorted in ascending order by `FileMeta::path` (`entry.rs:80-103`).
   - *Enforcement*: Constructor `WorkspaceIndex::assemble` takes `SortedByPath<FileMeta>` and `SortedByPath<Note>` (which enforce path ordering via type or debug assert in `sort.rs:48-57`), then merge-joins them in a single pass.
   - *Safe violation*: Safe code cannot violate this directly because `assemble` is `pub(crate)` and requires `SortedByPath`. However, `SortedByPath::assumed_sorted` only debug-asserts ordering; in release builds, out-of-order inputs would produce broken binary searches in `WorkspaceIndex::entry_at`.
2. **Inlink Map Canonical Normalization**:
   - *Invariant*: In `InlinkMap`, every target key has at least one inbound source; sources are strictly deduplicated and canonically path-sorted (`inlinks.rs:50-59`).
   - *Enforcement*: Built through `InlinkMap::from_flat_edges` (`inlinks.rs:102-116`) or `patch_and_diff` (`inlinks.rs:217-270`). `patch_and_diff` cleans up self-cancelled deletes/inserts and maintains sorted order via binary search insertion.
   - *Safe violation*: Cannot be violated outside module; `from_raw` is crate-private (`pub(super)`).
3. **Content-Only Delta Fast-Path vs Path-Set Change Invalidation**:
   - *Invariant*: Inbound link edges for unedited notes remain unchanged if and only if no paths were added or removed (`RefreshPlan::is_paths_unchanged`, `refresh.rs:403-411`).
   - *Enforcement*: `RefreshPlan::reconcile` branches on `is_paths_unchanged`. If any path changed (add, delete, rename), it forces a complete recompute of all inlinks using the fresh `LinkResolver` (`refresh.rs:360-386`), because an unedited note's ambiguous wikilink target could resolve differently when candidate paths appear/disappear.
   - *Safe violation*: Invariant correctly guarded at runtime.
4. **Ephemeral Derived Cache Durability**:
   - *Invariant*: The redb database at `.traces/index.redb` is strictly an ephemeral derived cache, not authoritative state (`store.rs:1159-1164`).
   - *Enforcement*: Rebuilds run with `Durability::None`. Corrupted or incompatible databases trigger automatic wipe-and-rebuild at `IndexStore::open` (`store.rs:165-174`, `809-815`).
   - *Safe violation*: Cannot be violated; corruption recovers cleanly.
5. **Config Drift vs Freshness (The Invariant Gap / Drift)**:
   - *Invariant claimed in ADR 9 / issue 17*: Config changes (frontmatter mappings, task statuses, schemas class_field) must invalidate the index on next refresh.
   - *Code reality*: **NOT ENFORCED TODAY**. `RefreshPlan::collect` only diffs file metadata (`size`, `modified_at`). The proposed 8th meta table for `IndexFingerprint` does not exist; `CACHE_FORMAT` version is hardcoded `b"1"`. Modifying `traces.toml` leaves `.traces/index.redb` marked "fresh", serving stale axes until `traces index` is run.

---

## 4. State & Lifecycle
- **Owned State**:
  - In-Memory: `WorkspaceIndex` owns `Box<[FileEntry]>`. Each `FileEntry` owns `FileMeta`, `Option<Box<Note>>`, and `Box<[PathBuf]>` (inlinks).
  - On-Disk: `IndexStore` owns a `redb::Database` handle and `PathBuf`.
  - Service: `IndexerService` owns `root: PathBuf`, `tasks: TaskConfig`, `frontmatter: FrontmatterConfig`, `class_field: String`.
- **Caches & Persistence**:
  - Persistent store: `.traces/index.redb`. Contains 8 tables (2 to-one value tables `files`, `notes`; 5 multimap tables `links`, `paths_by_tag`, `paths_by_file_class`, `tags_by_path`, `classes_by_path`; 1 marker table `cache_format`).
  - Rows stored as `postcard` byte blobs.
- **Concurrency & Parallelism**:
  - Concurrency model is multi-threaded synchronous via `rayon`.
  - No async runtimes or background daemon threads.
  - File scanning, file metadata inspection (`stat` syscalls), Markdown parsing, and inlink resolving run across Rayon threadpools (`rayon::join` and `par_iter`).
  - redb transactions: `begin_read()` produces `ReadTransaction` allowing concurrent readers. `begin_write()` takes redb's single writer lock. Write transactions run parallel table writes (`store.rs:700`, `1197`, `1209`) across independent tables within the single transaction.

---

## 5. Dependency Edges
- **Outbound `crate::` edges**:
  - `crate::file::{FileFormat, FileMeta}`: fundamental currency for scanned entries.
  - `crate::note::{Note, LinkTarget, LinkType, ListItem, ListItemType, NoteFieldValue, Tag}`: parsed note domain representation.
  - `crate::path::{FolderRef, HasPath, BaseNameRef, PathError, display_portable}`: path abstractions and distance metrics.
  - `crate::config::{Config, FrontmatterConfig, TaskConfig, SchemasConfig}`: parser configurations.
  - `crate::DirTreeError`: scan error propagation.
  - `crate::parse_markdown`, `MarkdownParserInput`: note parsing entry point.
- **Inbound `crate::` edges**:
  - `cli::index`: calls `IndexerService::from(config).rebuild()`.
  - `cli::mod` (`refresh_store`): `IndexerService::from(config).refresh_store()`.
  - `query::service`: calls `IndexStore::paths_with_tag`, `paths_with_file_class`, `paths_in_folder`, `read_notes_batch`, `read_files_batch`, `read_links_for_targets`. Also queries against in-memory `WorkspaceIndex`.
  - `template::engine`: holds `Arc<IndexerService>` and accesses cached `WorkspaceIndex`.
  - `lib.rs`: exports `WorkspaceIndex`, `IndexerService`, `FileEntry`.
- **Anomalies / Boundary Leaks**:
  - `store.rs` knows domain-specific note indexing rules: `IndexDimensions::for_class_field` (`store.rs:1582`) and `IndexDimension::visit_values` (`store.rs:1639-1665`) reach deep into `Note::tags` and `Note::frontmatter` / `inline_fields` to build the tag and file-class secondary index tables. Storage is not agnostic to query axes.
  - `query::service` bypasses `IndexerService` and reaches directly into `IndexStore` internal methods (`paths_with_tag`, `paths_with_file_class`, `paths_in_folder`, etc.) via `pub(crate)` visibility.

---

## 6. Interfaces: Deep vs. Shallow
- **`IndexerService`**: Deep seam. Callers invoke `build()`, `refresh()`, or `refresh_store()`. Beneath this simple facade lies parallel directory traversal, delta computation, parallel markdown parsing, trie-based link graph compilation, and redb transaction orchestration.
- **`WorkspaceIndex`**: Deep data interface. Once assembled, callers (`QueryService`, CLI formatters) treat it as an immutable, path-sorted snapshot. Hides internal assembly mechanics.
- **`IndexStore`**: Extremely wide/shallow surface (~2.7K lines). It acts simultaneously as:
  1. redb connection manager / recovery engine (`open`, `rebuild_needed`).
  2. Whole-table serializer / deserializer (`read_all`, `write_all_parallel`).
  3. Incremental table patcher (`apply_diff_deletions`, `apply_diff_upserts`, `apply_modified_notes`, `apply_inlink_delta`).
  4. Secondary index extractor for tags and file classes (`IndexDimension::visit_values`).
  5. Query execution store backend (`paths_with_tag`, `paths_with_file_class`, `paths_in_folder`, `read_notes_batch`, `read_links_for_targets`).
  It exposes ~15 distinct methods across different operational scopes.

---

## 7. Algorithms, Data Structures & Perf
- **Scan & Delta**:
  - Disk scan: $O(N)$ filesystem walk using `rayon` for concurrent `stat` syscalls.
  - `FileDelta::compute` (`delta.rs:23-65`): $O(N)$ two-pointer merge-join over two path-sorted slices. Minimal allocations (pushes only changed/added/deleted `FileMeta`).
- **Inlink Graph Derivation & Ambiguity Handling**:
  - `LinkResolver::new` (`inlinks.rs:326-345`): $O(N)$ pass over all files, indexing basenames into an `FxHashMap<BaseNameRef, BaseNameIndex>`.
  - `trie::BaseNameIndex` (`trie.rs:38-44`): If candidates $< 64$, flat linear scan ($O(K)$ where $K$ is candidate count). If $K \ge 64$, builds `CandidateTrie` in an `indextree::Arena` ($O(K \cdot \text{depth})$), answering queries in $O(\text{depth})$ using Lowest Common Ancestor distance.
  - Parallel resolution: Notes resolved across `rayon::par_iter()`, producing flat $(target, source)$ edges, followed by `par_sort_unstable()` and `dedup()`.
  - Benchmarks (`benches/index_inlinks.rs`): Inlink compilation scales linearly with edge count ($E \log E$ sort); ambiguous same-stem lookups stay strictly bounded by candidate depth.
- **Incremental Refresh Fast Path**:
  - When $paths$ are unchanged (`is_paths_unchanged`, `refresh.rs:403-411`), `InlinkMap::patch_and_diff` (`inlinks.rs:217-270`) mutates the existing inlink map in-place: $O(\text{edited\_notes} \cdot \text{links})$, completely bypassing whole-workspace link re-resolution.
  - Tag/class index updates (`store.rs:1440-1469`): Uses reverse index (`tags_by_path`, `classes_by_path`) to delete only previous values for modified notes in $O(k)$ time ($k$ = previous values on that note), avoiding table scans.
- **Allocation & Copying**:
  - `WorkspaceIndex` packs entries into a boxed slice `Box<[FileEntry]>`. `FileEntry` boxes `Note` (`Option<Box<Note>>`) to keep entry size to 96 bytes.
  - Serialization uses `postcard` with buffer reuse (`encode_row`, `codec.rs:30-39`).
  - redb iteration in `IndexStore::read_all` decodes full table values into memory sequentially in `decode_table_rows` (`store.rs:1061-1076`).

---

## 8. Error Topology
- **Error Types**:
  - `IndexError` (`error.rs:21-45`): Public enum wrapping `Store(StoreError)`, `Scan(DirTreeError)`, `Path(PathError)`, `NoteParse { path, source: io::Error }`, `Inspect { path, source: io::Error }`.
  - `StoreError` (`error.rs:49-79`): Internal enum (`pub(crate)`) wrapping `Io`, `Redb(Box<redb::Error>)`, `Serialize { path, source: postcard::Error }`, `Deserialize { path, source: postcard::Error }`.
- **Error Conversion & Context**:
  - Transparent conversions (`#[from]`) preserve error chains.
  - File paths are explicitly attached to `NoteParse`, `Inspect`, `Io`, `Serialize`, `Deserialize`, and `Redb` (via `wrap_redb_error`).
- **Resilience / Fail-Open Semantics**:
  - In `IndexerService::refresh` (`service.rs:125-146`), if persisting incremental updates to redb fails (`refresh::PersistFailed`), the service logs a warning and successfully returns the refreshed in-memory `WorkspaceIndex` (`mod.rs:50-62`, `materialize_apply`). The user query succeeds even if the disk cache is unwritable.
  - In `IndexStore::read_batch` (`store.rs:901-909`), corrupted individual note/file rows in redb trigger a `tracing::warn!` and are skipped rather than crashing the batch query.
  - In `IndexStore::open` (`store.rs:165-174`), corrupted redb files or schema mismatches trigger complete automatic database recreation.

---

## 9. Duplicated or Conflicting Concepts
1. **Three-Table vs. Eight-Table Model Drift**:
   - The original architecture decision ADR-0005 (`docs/adr/0005-template-query.md:45`) called for "Eight tables" (`files`, `notes`, `links`, `lists`, `paths_by_tag`, `paths_by_file_class`, `tags_by_path`, `file_classes_by_path`).
   - The LSP design notes (`.scratch/md-pkm-lsp/map.md:53`) refer to a "three `redb` tables (`FILES`/`NOTES`/`LINKS`)" model.
   - The actual implementation in `tables.rs` has **8 tables**, but NOT the 8 in ADR-0005: `lists` was dropped (list items are serialized directly inside `Note` via `postcard`), and `cache_format` was added (`tables.rs:100-133`).
2. **Duplicate Inlink Resolution**:
   - `InlinkMap::new` (`inlinks.rs:77-80`) builds its own internal `LinkResolver`.
   - `IndexerService::build` (`service.rs:76-81`) builds a `LinkResolver` concurrently with note parsing, then calls `InlinkMap::with_resolver`.
   - `inlinks::resolve_edges_for` (`inlinks.rs:303-316`) constructs yet another `LinkResolver` for delta patching.
3. **Dual Query Entry Paths**:
   - `QueryService::run` operates on in-memory `Arc<WorkspaceIndex>`.
   - `QueryService::run_from_store` (`query/service.rs:115-185`) executes queries directly against `IndexStore`, reading indexed tags/classes, performing point-reads on `NOTES`/`FILES`, and synthesizing a transient `WorkspaceIndex`. Two entirely separate evaluation engines for the same queries.

---

## 10. Doc, Glossary & ADR Drift
1. **ADR-0009 & Issue 17 (Config Drift) vs Code**:
   - *Claim*: ADR-0009 ("Refresh-driven Index Rebuilds: Freshness Sources and Drift as First-Class Refresh State") and Issue 17 define `IndexFingerprint`, `IndexSetting`, an 8th `meta` table, and `RefreshReport::ConfigDrift`.
   - *Reality*: ADR-0009 is marked `status: proposed`. Issue 17 is `ready-for-agent` (blocked by config-service/07). In the actual code, `IndexStore` has no `meta` table and no config hash checking. Changing `traces.toml` does NOT trigger an automatic index rebuild.
2. **ADR-0005 vs Code (Lists Table)**:
   - *Claim*: ADR-0005 specified a dedicated `lists` redb table for list items and tasks.
   - *Reality*: There is no `lists` table in `tables.rs`. `Note` serializes its own `Vec<ListItem>` directly in `NOTES`.
3. **GLOSSARY.md vs Code**:
   - `src/index/GLOSSARY.md` claims:
     - `RefreshState`: "The state from comparing current project files with the persisted index: `Fresh` when the opened store remains current, or `Stale` when an Index Update awaits persistence." (Matches code `refresh.rs:77-85`).
     - But ADR-0009 proposed expanding `RefreshState` to include `Drifted`. The glossary reflects the current code, not the proposed ADR.
4. **.scratch/md-pkm-lsp/map.md vs Code**:
   - *Claim*: Map line 53 claims `redb` has "three redb tables (`FILES`/`NOTES`/`LINKS`, no raw text, no spans)".
   - *Reality*: `redb` has 8 tables in `tables.rs`.

---

## 11. Candidate Architectural Questions

### 1. Should `IndexStore` be split into storage persistence and query indexing seams?
- **Claim**: `store.rs` is 2,765 lines and conflates low-level redb engine/serialization management (`rebuild_needed`, `create_db`, `codec`) with high-level schema indexing (`IndexDimensions`, `visit_values`, `paths_with_tag`). This leaks storage internals into `query::service` and ties redb to specific `Note` properties.
- **Evidence**: `store.rs:1574-1665` defines `IndexDimension`, extracting tag segments and `fileClass` keys from `Note`. `query/service.rs:131, 344` takes `&IndexStore` directly to perform multimap index lookups and batch reads.
- **Counter-argument**: Keeping them together in `store.rs` avoids an extra abstraction layer. The secondary indexes exist specifically to accelerate `redb` queries; separating them into a "generic storage" vs "index view" layer adds boilerplate without changing execution semantics.
- **Decision remaining**: Whether to split `store.rs` into `storage/redb.rs` (generic key-value/multimap engine) and `index/query_index.rs` (tag/class index extraction and lookups), or keep it unified.
- **Touches**: `query`, `index`.

### 2. Should `redb` secondary indexes (tags, file classes) be removed in favor of in-memory indexing on `WorkspaceIndex`?
- **Claim**: Maintaining 4 forward/reverse multimap tables in redb (`paths_by_tag`, `paths_by_file_class`, `tags_by_path`, `classes_by_path`) adds enormous write complexity to incremental refreshes (`store.rs:1340-1469`) and persistence transactions, while `WorkspaceIndex` already loads all notes into memory for `traces list`/`table`/`task` queries anyway.
- **Evidence**: `IndexerService::refresh` loads all notes and files into `WorkspaceIndex`. `QueryService::run_from_store` is only used when bypassing `WorkspaceIndex`, but cold CLI queries already refresh the store first. In benches (`benches/index_store.rs:113`), redb transaction commit floor is ~34ms; incremental updates pay reverse-index deletion costs on every touched note.
- **Counter-argument**: In large vaults (e.g. 20,000+ files), `run_from_store` allows point-reading only the 10 files matching `#project` without deserializing all 20,000 notes into memory.
- **Decision remaining**: Is cold CLI query memory reduction on 20K vaults worth the 4 multimap tables, reverse-index maintenance, and complex incremental diff logic in redb, or should `WorkspaceIndex` be the sole indexing substrate?
- **Touches**: `index`, `query`, `cli`.

### 3. Keep as-is: Is the `InlinkMap` LCA-distance trie hybrid algorithm optimal?
- **Claim**: The current trie-switching strategy (flat scan for $<64$ candidates, `CandidateTrie` folder-tree LCA for $\ge 64$ candidates, `trie.rs:33`) should be kept as-is.
- **Evidence**: Benches in `benches/index_inlinks.rs` show sub-millisecond link compilation for dense graphs and flat scaling across deep folder hierarchies. Memory overhead is zero for the 99% of basenames with 1 candidate.
- **Counter-argument**: A single `CandidateTrie` construction uses `indextree::Arena` allocations and is only used during build/refresh. A pure hash map of `(basename, folder) -> target` could be faster and simpler.
- **Decision remaining**: Keep as-is; the current hybrid design is verified sound, performant, and well-tested.
- **Touches**: `index`.

### 4. How should the LSP's live buffer overlay interact with `RefreshPlan` and incremental persistence?
- **Claim**: In an LSP session, typing in an open buffer must not write to `.traces/index.redb` on every keystroke, but queries and diagnostics need updated inlinks. `RefreshPlan` and `PendingApply` currently couple memory materialization with redb persistence.
- **Evidence**: `refresh.rs:105-127` couples `apply(dimensions)` to `IndexStore::persist`. While `materialize_apply` can handle `PersistFailed`, there is no explicit first-class "in-memory update without persistence" typestate.
- **Counter-argument**: `PendingApply::into_index` (`refresh.rs:135-141`) already exists and materializes an in-memory `WorkspaceIndex` from an unpersisted `IndexUpdate`! It is currently used only for fail-open recovery, but could be promoted to a first-class operation.
- **Decision remaining**: Should `RefreshState` explicitly support `Ephemeral(IndexUpdate)` for live LSP buffers alongside `Fresh` and `Stale(PendingApply)`, or should the LSP manage an overlay layer on top of `WorkspaceIndex` without touching `RefreshPlan`?
- **Touches**: `index`, `md-pkm-lsp`.

### 5. Where should Config Drift detection (ADR-0009 / Issue 17) live: in `RefreshPlan` or at the CLI/Service boundary?
- **Claim**: Checking config fingerprint inside `RefreshPlan::collect` (`redb` meta table) tightly couples index storage to config section hashes. Instead, `IndexerService::open` or the caller could store the fingerprint in an external companion file or check it prior to opening the store.
- **Evidence**: ADR-0009 explicitly decided *against* companion files, mandating atomic redb meta table updates (`docs/adr/0009-index-refresh.md:58-60`). However, implementation has stalled because it requires an 8th table schema migration and cross-crate dependency coordination (`ConfigDigests`).
- **Counter-argument**: Having `RefreshPlan` detect config drift guarantees that no query path can ever read an index built with different frontmatter or schema class settings. Putting it outside redb creates TOCTOU race conditions and non-atomic cache state.
- **Decision remaining**: Should ADR-0009 be implemented exactly as planned with the 8th `meta` table in redb, or should config invalidation be handled at a higher layer?
- **Touches**: `config`, `index`, `cli`.

### 6. Who should own Note parsing and Markdown dialect configuration?
- **Claim**: `IndexerService` currently owns `TaskConfig`, `FrontmatterConfig`, and `class_field` (`service.rs:35-40`) and directly invokes `parse_markdown`. This means any tool parsing a note needs to duplicate this configuration or construct an `IndexerService`.
- **Evidence**: `IndexerService::parse_note` (`service.rs:261-277`) constructs `MarkdownParserInput`. If the LSP needs to re-parse a single buffer in memory, it either must hold `IndexerService` or re-instantiate parser inputs with identical configs.
- **Counter-argument**: `IndexerService` is already the natural owner of batch note parsing across the workspace. It derives from `&Config` cleanly via `From<&Config>`.
- **Decision remaining**: Should note parsing configuration be factored into a dedicated `NoteParserService` / `NoteEngine` shared by both `IndexerService` and the LSP, or remain embedded in `IndexerService`?
- **Touches**: `note`, `index`, `config`, `md-pkm-lsp`.
