# Architectural Survey: Index Subsystem

## 1. Responsibilities & seams

- **What it owns**:
  - Parallel filesystem scanning (`IndexerService::scan`, `service.rs:325-345`, via `DirTree::descendants`, `dirtree.rs:5`, filtering out `.git`, `index.redb`, symlinks; `FileMeta::from_metadata`, `file.rs:66`).
  - Markdown note parsing with configured frontmatter/task rules (`IndexerService::parse_notes`, `service.rs:251-277`, wrapping `crate::note::parse_markdown`).
  - In-memory immutable snapshot aggregation (`WorkspaceIndex::assemble`, `entry.rs:39-45`, `assemble_internal`, `entry.rs:80-103`, 2-way merge-join of `FileMeta`, `Note`, and `InlinkMap` into `Box<[FileEntry]>`).
  - Wikilink resolution & inbound link graph derivation (`InlinkMap`, `inlinks.rs:58-99`; `LinkResolver`, `inlinks.rs:319-419`; `trie.rs:38-95` folder distance / LCA metric; `CandidateTrie` in `indextree::Arena` at $\ge 64$ candidates).
  - Freshness assessment & delta computation (`RefreshPlan::collect`, `refresh.rs:281-299`; `FileDelta::compute`, `delta.rs:23-65`; `is_paths_unchanged`, `refresh.rs:403-411`; `InlinkDelta::compute`, `delta.rs:97-131`).
  - Refresh orchestration & lifecycle typestate (`refresh.rs:77-160`, `service.rs:125-175`; `RefreshState::Fresh` | `Stale` -> `PendingApply::apply` -> `Persisted` | `PersistFailed`).
  - Durable persistence with redb & postcard (`tables.rs:20-77`, 8 tables: `files`, `notes`, `links`, `paths_by_tag`, `paths_by_file_class`, `tags_by_path`, `classes_by_path`, `cache_format`; `codec.rs:27-55` with buffer reuse; `store.rs:156-179` auto-recreation on corruption/drift; `begin_cache_txn`, `store.rs:1172`, `redb::Durability::None`, `store.rs:1159`).
  - Secondary index extraction: `IndexDimension::visit_values` (`store.rs:1639-1665`), tag segment expansion, frontmatter file-class values; forward/reverse multimap tables with $O(k)$ reverse-index cleanup (`remove_axis_entry`, `store.rs:1362-1376`).
  - Query index point lookups: `paths_with_tag`, `paths_with_file_class`, `paths_in_folder`, `read_notes_batch`, `read_files_batch`, `read_links_for_targets` (`store.rs:191-314`).
- **Public Surface (`src/index/mod.rs:30-40`, `src/lib.rs:155-157`)**:
  - `WorkspaceIndex` (`entry.rs:22`): Immutable in-memory snapshot of `FileEntry`s (`entries`, `entry_at`).
  - `FileEntry` (`entry.rs:136`): 96-byte entry holding `FileMeta`, `Option<Box<Note>>`, `Box<[PathBuf]>` (inlinks).
  - `IndexerService` (`service.rs:35`): Service facade (`build`, `refresh`, `refresh_with_report`, `refresh_store`, `rebuild`, `persist`, `load`).
  - `IndexError` (`error.rs:21`), `IndexResult` (`error.rs:10`). Under `test-utils` / test: `InlinkMap` (`inlinks.rs:58`), `RefreshReport` (`refresh.rs:31`).
  - Internal crate surface (`pub(crate)`): `RowIndex` (`entry.rs:186`), `IndexStore` (`store.rs:136`), `SortedByPath` (`sort.rs:18`).
- **Callers**:
  - `src/cli/mod.rs:365,484`: Cold queries run `refresh_store()` and pass `&IndexStore` to `QueryService::run_from_store`.
  - `src/cli/index.rs:25`: Runs `IndexerService::rebuild()`.
  - `src/query/service.rs:115,131`: `QueryService::run` consumes `WorkspaceIndex`; `run_from_store` queries `IndexStore` multimaps directly.
  - `src/template/engine.rs:135`, `src/template/engine/query.rs:109,228-232`: Holds `Arc<IndexerService>` and invokes `refresh()` once per render, caching `WorkspaceIndex` in minijinja `State`.
- **What callers must know**:
  - `WorkspaceIndex` is immutable and detached from the root path (`entry.rs:17-18`); callers query it in-memory via `QueryService`.
  - `IndexerService::refresh` is fail-open (`mod.rs:50-62`): persistence errors are logged and the in-memory index is still returned. In contrast, `refresh_store` (`service.rs:219-237`) propagates persistence errors directly as it has no in-memory fallback.
  - `QueryService::run_from_store` constructs a partial, non-total `WorkspaceIndex` containing only matching candidate files (`query/service.rs:162-166`).

## 2. Invariants

- **`WorkspaceIndex` Path-Ascending Totality**: `WorkspaceIndex::entries` contains exactly one `FileEntry` for every regular file in the project root, strictly sorted ascending by path (`entry.rs:80-103`). Enforced: constructor `assemble` requires `SortedByPath<FileMeta>` and `SortedByPath<Note>`. However, `SortedByPath::assumed_sorted` (`sort.rs:48-57`) only validates via `debug_assert!`; in release builds, callers rely on convention. Split: `scan` (`service.rs:343`), `parse_notes`, and `assemble_internal` (`entry.rs:80-103`). Breach: `query/service.rs:162-166` constructs a partial `WorkspaceIndex` with only query-matched files.
- **`InlinkMap` Canonical Dedup & Sorting**: In `InlinkMap`, every target key has $\ge 1$ source; sources are strictly deduplicated and canonically path-sorted (`inlinks.rs:50-59`). Enforced: construction in `from_flat_edges` (`inlinks.rs:102-116`) and `patch_and_diff` (`inlinks.rs:243-248`). Local to `inlinks.rs`.
- **Wikilink Target Stability Bifurcation**: Inbound link edges for unedited notes remain invariant if and only if the set of indexed paths is strictly unchanged (`refresh.rs:403-411`). Enforced: runtime branch in `refresh.rs:344` (`is_paths_unchanged`). Any path add/delete/rename forces full-vault note deserialization (`store.read_all_notes()`, `refresh.rs:487`) and link re-resolution (`refresh.rs:360-386`). Split: `FileDelta::compute` (`delta.rs:23`), `RefreshPlan::reconcile` (`refresh.rs:338`), and `LinkResolver`.
- **Ephemeral Derived Cache Durability**: `.traces/index.redb` is strictly an ephemeral derived cache; corruption or loss must never cause query failure or data loss (`store.rs:1159-1164`). Enforced: `IndexStore::open` (`store.rs:164-174`) automatically recreates the database on corruption or schema mismatch (`rebuild_needed`, `store.rs:823-878`); `materialize_apply` (`mod.rs:50-62`) catches `PersistFailed` and falls back to in-memory index; `redb::Durability::None` skips per-commit fsync (`store.rs:1159`). Split: `IndexStore::open`, `store.rs:rebuild_needed`, `mod.rs:materialize_apply`.
- **Config Drift Rebuild Invalidation (Invariant Gap)**: Persisted index rows must match active config (tasks, frontmatter, schemas `class_field`). Enforced: nowhere today. Stored version in `CACHE_FORMAT` is hardcoded `b"1"` (`tables.rs:73`, `store.rs:877`). Config changes do not affect `FileDelta`; `service.rs:1749-1780` pins that untouched files serve stale class axes. ADR-0009 and Issue 17 define the atomic `meta` table with `IndexFingerprint`, but remain unimplemented. Split: gap across `config` and `index`.

## 3. Verified observations

- [V] Parallel Rayon table writes in a single `redb::WriteTransaction`: `WriteTarget::ALL` runs concurrently via `into_par_iter()` (`store.rs:1197-1200`), and secondary axes write forward/reverse concurrently via `rayon::join` (`store.rs:1209-1216`).
- [V] Bypassing commit fsync via `redb::Durability::None`: `begin_cache_txn` disables durability for index writes (`store.rs:1159-1174`), eliminating disk sync overhead on the derived cache.
- [V] Fixed ~34 ms commit transaction floor: Benches show a ~34 ms fixed commit floor below 1,000 notes before scaling linearly with row volume (`benches/index_store.rs:113-115`).
- [V] 8 redb tables defined in static schema: `files`, `notes`, `links`, `paths_by_tag`, `paths_by_file_class`, `tags_by_path`, `classes_by_path`, `cache_format` (`tables.rs:100-133`).
- [V] Catastrophic cliff on path-set changes: Adding/deleting a single path forces `merge_refreshed_notes` (`refresh.rs:487`) to read and deserialize all notes from redb and re-resolve all vault links (`refresh.rs:361-386`), whereas content-only edits patch links in $O(\text{edited})$ without reading unedited notes (`refresh.rs:344-360`).
- [V] Reverse-index maintenance for tags and file classes: `store.rs:1362-1376` (`remove_axis_entry`) reads the path-keyed reverse index to delete old entries in $O(k)$ where $k$ is that note's tag/class count, avoiding full table scans.
- [V] Selective extraction of file classes: `IndexDimension::visit_values` (`store.rs:1654-1658`) extracts file-class values exclusively from YAML frontmatter (`fm.get_values(key)`), ignoring inline fields (`[class:: value]`).
- [V] Dual query execution pipelines: `QueryService::run` operates on in-memory `WorkspaceIndex` (`query/service.rs:167-168`), while `QueryService::run_from_store` reaches into `IndexStore` internal methods and synthesizes a partial `WorkspaceIndex` (`query/service.rs:131,162-166`).
- [V] Basename disambiguation threshold at 64: `TRIE_THRESHOLD = 64` (`trie.rs:33`) switches from flat linear scan to an `indextree::Arena` folder trie (`CandidateTrie`, `trie.rs:40`) using LCA folder distance (`FolderRef::distance_to`, `trie.rs:80`).
- [V] Pinned config staleness test on main: `service.rs:1749-1780` asserts that changing `class_field` config leaves persisted rows untouched and fresh refresh returns empty report with old axes intact.
- [V] Fail-open memory fallback on persist failure: `mod.rs:50-62` logs a warning on `PersistFailed` and unpacks the in-memory `WorkspaceIndex` so caller queries succeed even when redb is read-only or locked.
- [V] Postcard buffer reuse in row encoding: `codec.rs:27-40` reuses caller-provided `Vec<u8>` buffers across rows during batch writes (`store.rs:1387,1417`).

## 4. Hypotheses

- **[H1] Consolidate Dual Query Execution into Pure WorkspaceIndex (Rank 1)**:
  - *Thesis*: `run_from_store` (`query/service.rs:129-169`) and its 4 redb secondary index tables (`paths_by_tag`, `paths_by_file_class`, `tags_by_path`, `classes_by_path`) should be removed in favor of evaluating all queries over `WorkspaceIndex`.
  - *Current vs Alt*: Current maintains 4 forward/reverse multimap tables in redb, pays $O(k)$ reverse-index updates on every note write (`store.rs:1440-1469`), and constructs a partial `WorkspaceIndex` (`query/service.rs:162-166`). Alternative: redb stores only `files`, `notes`, `links`, `meta`; all queries run over `WorkspaceIndex`. Keep as-is: avoids deserializing all notes on cold CLI queries in massive vaults (20k+ notes).
  - *Removable*: 4 redb tables (`tables.rs:40-70`), `IndexDimensions`, `IndexDimension`, `visit_values`, `remove_axis_entry` (~400 lines in `store.rs`), `SourceResolver` (`query/service.rs:343-400`), `run_from_store` dual path.
  - *Disprove*: Cold CLI point-query benchmark (`traces list "#rare_tag"`) on 20,000 notes: if loading `WorkspaceIndex` takes >100ms or >100MB vs <15ms for `run_from_store`, point-indexing is justified.
  - *Blast radius*: `src/index/store.rs`, `src/index/tables.rs`, `src/query/service.rs`, `src/cli/mod.rs`.
- **[H2] Inverted Outlink Stem Index to Avoid Whole-Vault Deserialization on Path Deletions (Rank 2)**:
  - *Thesis*: An inverted index of unresolved wikilink stems would allow incremental refresh on path-set changes (file adds/deletions/renames) to re-resolve only notes whose targets could match the affected stem, eliminating the full-vault note deserialization cliff (`refresh.rs:361-386,487`).
  - *Current vs Alt*: Current: `is_paths_unchanged` is an all-or-nothing boolean; deleting any single file forces reading all notes from redb and re-resolving all links across the vault. Alternative: check if the added/deleted path's basename matches any existing note's unresolved outlinks; if none, patch in $O(1)$ like content edits. Keep as-is: full re-resolution on path changes is simple and robust.
  - *Removable*: Eliminates the latency cliff where single-file deletion takes orders of magnitude longer than single-file edit (`benches/index_refresh.rs:117-121`).
  - *Disprove*: Measure frequency of file creation/deletion vs edit. If deletions are rare or full link compilation for 5,000 notes takes <15ms (`benches/index_inlinks.rs`), added index state is unjustified.
  - *Blast radius*: `src/index/refresh.rs`, `src/index/inlinks.rs`, `src/index/store.rs`.
- **[H3] Atomic In-Store Config Drift Invalidation per ADR-0009 / Issue 17 (Rank 3)**:
  - *Thesis*: Storing an exhaustive `IndexFingerprint` (10 framed config digests) in an atomic redb `meta` table (renamed from `cache_format`) provides transactional drift detection and heals stale secondary indexes automatically without external companion files.
  - *Current vs Alt*: Current: no drift detection; stale axes pinned in tests (`service.rs:1749-1780`). Alternative: rename `cache_format` to `meta`, write fingerprint in same write txn as rows, auto-rebuild on drift. Keep as-is: requires manual `traces index` after config edits.
  - *Removable*: Eliminates operator-facing staleness bugs, removes warn-only mitigations from issue 04, deletes `fresh_refresh_serves_persisted_class_axes_unchanged_...` test.
  - *Disprove*: Proving `IndexFingerprint` computation or comparison adds measurable latency to `prepare_pass` (it compares 10 32-byte digests in memory, <1 µs).
  - *Blast radius*: `src/index/service.rs`, `src/index/store.rs`, `src/index/refresh.rs`, `src/index/tables.rs`.
- **[H4] Eliminate CandidateTrie and Indextree Dependency (Rank 4)**:
  - *Thesis*: Basenames with $\ge 64$ candidate paths in PKM vaults are vanishingly rare. The `CandidateTrie` construction (`trie.rs:40`) and `indextree::Arena` dependency add ~500 lines of complex tree indexing without practical benefit over flat LCA scans.
  - *Current vs Alt*: Current: switches to `CandidateTrie` at 64 candidates (`trie.rs:33`). Alternative: always use `nearest_flat` (`trie.rs:65-95`) with direct LCA folder distance. Keep as-is: already implemented, tested, and benchmarks show flat scaling up to 128 candidates (`benches/index_inlinks.rs:60`).
  - *Removable*: ~500 lines in `src/index/trie.rs`, `indextree` crate dependency in `Cargo.toml`.
  - *Disprove*: Benchmark `nearest_flat` vs `CandidateTrie` on 128 same-stem candidates. If flat scan takes >1ms for 100 links, trie is justified.
  - *Blast radius*: `src/index/trie.rs`, `Cargo.toml`.
- **[H5] Decouple Markdown Parsing Dialect from IndexerService (Rank 5)**:
  - *Thesis*: `IndexerService` stores `TaskConfig`, `FrontmatterConfig`, and `class_field` (`service.rs:36-40`) solely to construct `MarkdownParserInput` (`service.rs:270-275`), conflating index storage with parser configuration.
  - *Current vs Alt*: Current embeds parser configs inside `IndexerService`. Alternative: factor a `NoteParserConfig` / `NoteEngine` shared across CLI, LSP, and indexer. Keep as-is: `IndexerService` is currently the only batch consumer.
  - *Removable*: Removes dialect configuration fields and imports from `IndexerService`.
  - *Disprove*: Finding that no other subsystem (e.g. LSP or standalone parser) needs independent batch note parsing.
  - *Blast radius*: `src/index/service.rs`, `src/note/`.
- **[H6] Release-Mode Invariant Enforcement for SortedByPath (Rank 6)**:
  - *Thesis*: `SortedByPath::assumed_sorted` (`sort.rs:48-57`) only validates ordering via `debug_assert!`, leaving release builds susceptible to silent binary search failures if an unsorted slice is passed.
  - *Current vs Alt*: Current: debug-only assertion. Alternative: verify ordering in release via `windows(2).all()` or enforce at type construction through `SortedByPath::sorted`. Keep as-is: avoids redundant $O(N)$ pass after `par_sort_unstable_by`.
  - *Removable*: Eliminates latent silent corruption in release builds.
  - *Disprove*: Profiling shows `windows(2).all()` check on 20k paths adds measurable overhead to `IndexerService::scan`.
  - *Blast radius*: `src/index/sort.rs`.
- **[H7] Factor IndexStore into StorageEngine and DomainRepository (Rank 7)**:
  - *Thesis*: `store.rs` (2,765 lines) conflates low-level redb engine management (recovery, transactions, durability, postcard codec) with high-level domain indexing (`visit_values`, `paths_with_tag`).
  - *Current vs Alt*: Current: monolithic `IndexStore`. Alternative: split into `StorageEngine` (generic key-value / multimap table runner) and `IndexRepository`. Keep as-is: avoids transaction lifetime boilerplate across layers.
  - *Removable*: Decouples storage mechanics from note AST semantics.
  - *Disprove*: Showing that redb `WriteTransaction` borrowing rules make layered abstraction overly verbose or inefficient.
  - *Blast radius*: `src/index/store.rs`.

## 5. Cross-slice couplings

- **Index ↔ Note**: `IndexerService::parse_note` (`service.rs:261-277`) calls `crate::note::{parse_markdown, MarkdownParserInput}`. `IndexDimension::visit_values` (`store.rs:1639-1665`) traverses `Note::tags` and `Note::frontmatter` (`get_values`), inspecting YAML frontmatter keys directly while omitting inline fields. `Note` and its inner AST types are serialized directly into redb `NOTES` via postcard (`codec.rs:27-55`).
- **Index ↔ Config**: `IndexerService::from(&Config)` (`service.rs:42-52`) unpacks `tasks`, `frontmatter`, `class_field`. ADR-0009 / Issue 17 couples index invalidation to `Config::digests()` and `ConfigDigests`. In contrast to `src/file_tracker.rs` (which uses external companion hash files for trust tracking, `file_tracker.rs:30`), the index fingerprint lives inside redb (`docs/adr/0009-index-refresh.md:58-60`).
- **Index ↔ Query**: `QueryService::run` consumes `WorkspaceIndex` (`query/service.rs:167`). `QueryService::run_from_store` reaches directly into `IndexStore` internal methods (`paths_with_tag`, `paths_with_file_class`, `paths_in_folder`, `read_notes_batch`, `read_files_batch`, `read_links_for_targets`) and synthesizes a partial `WorkspaceIndex` (`query/service.rs:162-166`).
- **Index ↔ CLI**: `cli::mod::run_query_builder_from_store` (`cli/mod.rs:470-488`) orchestrates `refresh_store` + `run_from_store`. `cli::index` (`cli/index.rs:25`) calls `IndexerService::rebuild()`.
- **Index ↔ Template**: `TemplateEngine` holds `Arc<IndexerService>` (`template/engine.rs:135`) and refreshes `WorkspaceIndex` per render, caching it in minijinja `State` (`template/engine/query.rs:228-232`).
- **Index ↔ Path/FS**: Traversal delegates to `DirTree::descendants` (`dirtree.rs:5`, `service.rs:327`) returning `DirNode` and converting to `FileMeta` (`file.rs:66`).

## 6. Comparables worth studying

- **rustc incremental compilation (`rustc_incremental` / `rustc_query_system`)**: Informs H2 (fine-grained dependency edge invalidation) and H3 (fingerprinting and redb meta table). Rustc hashes query inputs and tracks dependency graph edges; when a file changes, only dependent queries re-execute.
- **Cargo fingerprints (`cargo::core::compiler::fingerprint`)**: Informs H3 (config drift detection). Cargo computes a fingerprint over toolchain, env vars, and config flags, invalidating build artifacts when flags change without source edits.
- **Obsidian Dataview / advanced-metadata-cache**: Informs H1 (pure in-memory index vs secondary tables) and H2 (vault link resolution caching). Dataview keeps the entire metadata index in memory; cold startup re-indexes from SQLite cache.
- **gitoxide / `gix-index` (Git dircache)**: Informs H6 (`SortedByPath`) and delta computation (`FileDelta`). Git's index maintains path-sorted entries with stat cache (mtime, size, ctime, dev/ino) for instant two-pointer delta reconciliation.
- **ripgrep (`ignore` crate directory walk)**: Informs `IndexerService::scan` parallel directory traversal and ignore filtering.

## 7. Open decisions

- **Sharp Candidate Tickets**:
  1. *Consolidate query execution*: Benchmark and evaluate memory/latency of pure `WorkspaceIndex` querying vs redb secondary tables; deprecate `run_from_store` if viable.
  2. *Implement ADR-0009*: Add `IndexFingerprint` in `meta` redb table for automatic config-drift rebuild on refresh.
  3. *Evaluate CandidateTrie removal*: Benchmark 128-candidate flat scan vs indextree folder trie to decide if `indextree` dependency can be dropped.
  4. *Inverted unresolved-link stem index*: Prevent whole-vault re-read and link re-resolution on single path deletion/addition.
  5. *Decouple Markdown dialect config*: Extract `NoteParserConfig` from `IndexerService`.
- **Preserved Fog**:
  - *LSP multi-file buffer overlay vs persistent index*: How to expose unpersisted keystroke updates to inlinks and tags without redb writes. Does `RefreshState` need an `Ephemeral(IndexUpdate)` state?
  - *File-class extraction in inline fields vs frontmatter*: Currently `IndexDimension::visit_values` only looks in frontmatter (`fm.get_values(key)`), while note queries support inline fields (`[class:: Book]`). Should index secondary tables index inline fields too, or does H1 make this moot?

## 8. Withstands scrutiny

- **`redb` with `Durability::None` + `postcard`**: Ephemeral derived cache design with disabled commit fsync (`store.rs:1159`) and automatic wipe-and-rebuild recovery on corruption or schema mismatch (`store.rs:164-174,823-878`) is exceptionally fast (~34ms commit floor) and robust.
- **Parallel Rayon table writes in a single txn**: `store.rs:1197-1200` concurrent writes across disjoint tables under one transaction maximize multi-core throughput without database locking bottlenecks.
- **Content-only delta fast path**: `inlinks.rs:217-270` (`patch_and_diff`) in-place graph mutation completely bypasses full-vault re-resolution when paths are unchanged.
- **LCA folder distance metric**: `FolderRef::of(a).distance_to(FolderRef::of(b))` (`trie.rs:80`) provides deterministic Obsidian-compatible wikilink disambiguation with correct tie handling (`tied = true` -> unresolved).
