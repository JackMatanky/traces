# Research: Freshness, Invalidation, and Fingerprinting Precedents

> **Re-verification note (main @ `207cc3fb`).** The Traces-side premise below ("unimplemented config-drift fingerprint") is outdated: `207cc3fb` implemented ADR-0009. Lesson 1 (type-owned completeness by exhaustive destructuring) and Lesson 3 (fingerprint in the redb `meta` table, same transaction as the rows) were realized almost exactly as recommended (`src/index/fingerprint.rs`, `src/index/store.rs`), which is convergent confirmation, not new evidence. Lesson 2 (inverted unresolved-stem index) stays superseded by `research/11` (97% of the cliff is note deserialization). Lesson 4 (racy-timestamp guard) is **still open** (`src/index/delta.rs` untouched; no `racy` handling in `src/index`). Differences from the recommendation: the landed code keeps `ConfigDigests` as the source and derives the index fingerprint from it (rather than deleting `digest.rs`), and the `syn` source scan still exists.

**Ticket:** `.scratch/strategic-redesign/issues/05-research-freshness-fingerprint-precedent.md`  
**Deliverable:** `.scratch/strategic-redesign/research/05-freshness-fingerprint-precedent.md`  
**Status:** Complete

---

## 1. What Each Comparable Actually Does (Source-Verified)

### Cargo Fingerprints

- **[V] Composite Struct & Two-Tier File Cache**: Cargo models build unit freshness as `Fingerprint` (`src/compiler/fingerprint/mod.rs:639-688`), containing 11 explicit input components: `rustc` version hash, `features` (sorted string), `declared_features`, `target` hash, `profile` hash, `path` (workspace-relative or absolute), `deps: Vec<DepFingerprint>`, `local: Mutex<Vec<LocalFingerprint>>` (mtimes/checksums from dep-info and build script outputs), `rustflags`, `config: u64`, and `compile_kind: u64`.
- **[V] Dependency Propagation**: Dependency edges are stored in `DepFingerprint` (`mod.rs:596-612`), capturing package ID, extern crate name, public exposure, and recursive `Arc<Fingerprint>`. Stale dependencies propagate upstream via `FsStatus::StaleDepFingerprint` or `FsStatus::StaleDependency` (`mod.rs:705-714, 1308-1311`).
- **[V] Two-File On-Disk Protocol & Fail-Open Recovery**: Persisted to `<loc>` (raw hex of `hash_u64()`) and `<loc>.json` (full serialized `Fingerprint` struct) (`mod.rs:1979-1995`). `_compare_old_fingerprint` (`mod.rs:2070-2095`) fast-paths equality against `<loc>` and `fs_status.up_to_date()`. On mismatch, it parses `<loc>.json` and performs structural field diffing to populate `DirtyReason` (`dirty_reason.rs:319-322`). On I/O or JSON decode failure, it falls back to `FingerprintComparison::Dirty { reason: DirtyReason::FreshBuild }` (`mod.rs:2064-2066`); corruption never aborts Cargo.

### rustc Incremental (`rustc_middle::dep_graph`, `rustc_incremental`)

- **[V] Red-Green Marking & Change Cutoff**: rustc tracks query execution in a dependency graph where each query invocation is a `DepNode` (`kind: DepKind, key_fingerprint: PackedFingerprint`) with a 128-bit `Fingerprint` (`rustc_middle/src/dep_graph/graph.rs:165-205`). In a subsequent session, `try_mark_green` (`graph.rs:909-1035`) checks parents in the previous graph: if all parents can be marked green, the node is marked green without evaluation. If an un-green parent is forced and re-evaluated, its new value fingerprint is compared against `prev_value_fingerprint_of` (`graph.rs:704-706`); if identical, the parent is colored green and recomputation terminates (change cutoff).
- **[V] CLI Argument Gating & Private Session Directories**: `load_dep_graph` (`rustc_incremental/src/persist/load.rs:35-103`) opens `dep-graph.bin` and checks `prev_commandline_args_hash == sess.opts.dep_tracking_hash(false)`. If any CLI flag affecting code generation differs, or if file headers or decoding fail, rustc logs `completely ignoring cache` and returns `LoadResult::DataOutOfDate`, triggering a total rebuild. Writes go to an isolated directory `s-<pid>-<random>` (`fs.rs:11-83`), staging `dep-graph.part.bin` before renaming to `dep-graph.bin`, using OS `flock` locks on `.lock` files to prevent concurrent worker interference and govern garbage collection.

### Git Index (`gix-index`, `gix-dir`) & Mercurial/Sapling `treestate`

- **[V] Stat Cache & Racy-Git Invalidation**: `gix-index` stores `Stat` per file: `ctime`, `mtime` (seconds + nanoseconds), `size`, `dev`, `ino`, `uid`, `gid`, and mode (`gix-index/src/entry/stat.rs:86-106`). `Stat::is_racy` (`entry/stat.rs:11-30`) flags an entry as racy if `stat.mtime >= index.timestamp`. Racy entries cannot rely on stat equality on subsequent runs because sub-second edits within the same filesystem timestamp tick would go undetected; racy files must be re-read and content-hashed.
- **[V] Path-Sorted Flat Index vs Hierarchical Tree**: Git index entries are stored in a contiguous, slash-separated byte-path-sorted array (`State::entries`, `gix-index/src/lib.rs:117-136`). Lookups and directory prefix scans use $O(\log N)$ binary search (`entry_index_by_path`, `prefixed_entries_range`, `access/mod.rs:377-395`). In contrast, Sapling/Mercurial dirstate-v2 (`eden/scm/lib/treestate/src/tree.rs:29-65`) models a prefix tree with `NodeEntry::Directory` and `NodeEntry::File`. Directory nodes store `AggregatedState` (`union` and `intersection` of `StateFlags`); if a directory's subtree contains zero dirty or untracked flags, status checks skip traversing the directory in $O(1)$.

### Ruff (`ruff_cache`, `ruff`)

- **[V] `#[derive(CacheKey)]` Compile-Time Honesty**: Ruff enforces cache key completeness via `pub trait CacheKey { fn cache_key(&self, state: &mut CacheKeyHasher); }` (`crates/ruff_cache/src/cache_key.rs:74-85`) and a derive macro (`crates/ruff_macros/src/cache_key.rs:126-129`). Any struct contributing to cache identity must implement `CacheKey`; adding a field without `#[cache_key(ignore)]` forces a compile-time implementation.
- **[V] Two-Level Key Hierarchy & Tempfile Atomicity**: Ruff separates global settings from per-file state (`crates/ruff/src/cache.rs:31-37, 94-133, 365-371`). The global cache key is `cache_key(&package_root, settings)` hashing canonical package root and the entire `Settings` struct into a directory path `.ruff_cache/{VERSION}/{key}`. Different settings naturally resolve to distinct directories. Inside, per-file entries (`FileCacheKey { file_last_modified, file_permissions_mode }`) index lint diagnostics. Writes serialize via `rkyv::to_bytes` into a sibling `tempfile_in`, followed by atomic rename (`temp_file.persist(&self.path)`, `cache.rs:164-197`). Corrupt cache files emit `warn_user!` and fall back to `Cache::empty()`.

### rumdl (`cache.rs`, `workspace_index.rs`)

- **[V] Multi-Axis Hash Filenames & Fine-Grained Reverse Invalidation**: rumdl lints files using a cache key composed of file content hash, config hash, rules hash, and dependency fingerprint (`rumdl/src/cache.rs:6-8, 233-280`). `hash_config` hashes serialized JSON of `Config`; comments mandate `BTreeMap`/`IndexMap` over `HashMap` to prevent `RandomState` iteration drift (`cache.rs:247-252`). Entries are stored at `.rumdl_cache/{VERSION}/{file_hash}_{short_rules_hash}.json`.
- **[V] Reverse Outlink Graph Maintenance**: For cross-file rules, `WorkspaceIndex` (`rumdl/src/workspace_index.rs:429-444, 821-921`) stores `files: HashMap<PathBuf, FileIndex>` and `reverse_deps: HashMap<PathBuf, HashSet<PathBuf>>` (target -> set of sources linking to it). On `update_file(path, index)` or `remove_file(path)`, it clears old reverse deps for `path` as source in $O(k)$ where $k$ is `path`'s outlinks (`workspace_index.rs:898-906`), registers new targets, and uses `get_dependents(target)` to re-lint only affected dependents when a file changes or is deleted.

### Obsidian Dataview & `advanced-metadata-cache`

- **[V] Inverted Stem Index for Path Changes**: Obsidian Dataview maintains an in-memory `IndexMap` (`dataview/src/data-index/index.ts:36-40, 480-523`) with forward `map` and reverse `invMap`, updating in $O(k)$ on rename/delete. The `advanced-metadata-cache` plugin (`advanced-metadata-cache/src/modules/backlinks/backlink-cache-component.ts:85-96, 259-275, 396-427`) maintains `backlinksMap`, `linksMap`, `resolvedBasenameMap` (`lowered_basename -> Set<sourcePath>`), and `unresolvedBasenameMap` (`lowered_stem -> Set<sourcePath>`).
- **[V] Sub-Graph Re-Resolution on Add/Delete/Rename**: When files are created, renamed, or deleted, `updateRelatedLinks(fileNames)` queries `getSourcePathsToReResolve(fileNames)` (`backlink-cache-component.ts:259-275`): it looks up the affected file names in `resolvedBasenameMap` and `unresolvedBasenameMap`, and queues *only* those specific source files for re-resolution. Notes that do not reference the added/deleted/renamed basename are never re-evaluated.

---

## 2. Invariants and Invariant Ownership

| Invariant | Owner | Enforcement Mechanism |
|---|---|---|
| **Compile-Time Input Honesty** | Type System (`CacheKey` trait) | Ruff's `#[derive(CacheKey)]` rejects un-keyed fields at compile time; rustc's query system requires `HashStable` for query keys and results. Cargo uses explicit struct fields (`Fingerprint`). |
| **Change Cutoff / Propagation Ceiling** | Query Engine / DepGraph | rustc `try_mark_green` stops red propagation if a parent query recomputes to the same value fingerprint (`prev_value_fingerprint_of == new_fingerprint`). |
| **Stat Cache Race Prevention** | Index Stat Engine | Git `is_racy` marks entries where `mtime >= index.timestamp` dirty, forcing content hashing to eliminate sub-second modification blind spots. |
| **Atomic Cache Mutation** | Storage / Filesystem Seam | Ruff, rumdl, and rustc write whole payloads to temporary files in the same directory/filesystem before executing an atomic rename (`rename` / `persist`). redb achieves equivalent atomicity via multi-table ACID transactions. |
| **Fail-Open Ephemerality** | Cache Facade | Cargo, Ruff, rustc, and rumdl treat disk corruption, missing files, or decoder errors as cache misses (clearing cache or falling back to cold execution); corruption never panics or aborts user commands. |
| **Localized Link Re-Resolution** | Inverted Stem Index | `advanced-metadata-cache` ensures that path-set changes only re-evaluate notes registered in `resolvedBasenameMap` or `unresolvedBasenameMap` for the affected stems. |

---

## 3. Tradeoffs and Costs Paid

- **[V] Structural Hashing Overhead**: rustc pays runtime memory for `PreviousDepGraph` (several MBs per compilation) and CPU time to compute 128-bit SipHash/BLAKE3 value fingerprints across every intermediate query result (`graph.rs:495-496`).
- **[V] Stat Cache False Positives**: Git's racy-git check incurs periodic unneeded content reads on clean files immediately after commits because any file touched in the commit's second tick is flagged racy.
- **[V] Inverted Index Memory vs Deserialization Latency**: `advanced-metadata-cache` and rumdl retain memory for reverse dependency sets (`resolvedBasenameMap`, `unresolvedBasenameMap`, `reverse_deps`) across the entire workspace to save disk deserialization and parsing on structural modifications.
- **[V] Coarse-Grained Rebuild Clashing**: Cargo and rustc wipe whole-unit or whole-crate caches when top-level environment variables or CLI flags change (`load.rs:95-102`), discarding reusable per-file work when global flags drift.

---

## 4. Transferable Lessons for Traces

1. **[H] Derive-Based or Exhaustively Destructured Fingerprinting Beats AST Test Scanners**: Ruff's `CacheKey` demonstrates that compile-time fail-closed freshness should be governed by Rust's type system (a `CacheKey` trait or exhaustive destructuring in the constructor). Traces' current approach in config-service/07—a 412-line `digest.rs` coupled to an out-of-band `syn` AST visitor in `tests/`—is high maintenance. A simple `IndexFingerprint` struct holding the exact tuple consumed by `IndexerService` (`(TaskConfig, FrontmatterConfig, class_field)`) with an exhaustive constructor gives identical fail-closed guarantees without AST parsing.
2. **[H] Inverted Unresolved Stem Index Eliminates the Path-Set Deletion Cliff (Validates H2)**: `advanced-metadata-cache`'s `resolvedBasenameMap` + `unresolvedBasenameMap` pattern (`backlink-cache-component.ts:259-275`) is directly transferable to Traces' `src/index/refresh.rs:344-386`. By persisting an inverted index of unresolved wikilink stems (`Table<Stem, Box<[Path]>>`) alongside existing inbound links:
   - When a note is deleted: only notes linking to it lose their edge ($O(\text{inlinks})$). Unaffected notes are untouched.
   - When a note is added: only notes with unresolved links matching the new note's basename are re-resolved ($O(\text{unresolved\_stems})$).
   - When a note is renamed: treated as delete + add.
   - **Result**: Eliminates `refresh.rs:361-386`'s full-vault note deserialization cliff (`merge_refreshed_notes`), reducing path mutations to the same sub-5ms latency as content edits.
3. **[H] Single-Transaction redb Meta Table Matches Atomic Rename**: Ruff, rustc, and rumdl use temporary files and atomic `rename()` because their caches are file-based. Traces stores its index in an embedded redb database. Renaming `cache_format` to `meta` and storing `IndexFingerprint` in the same `redb::WriteTransaction` that writes `notes` and `links` achieves perfect atomicity with zero temporary-file management.
4. **[H] Racy-Git Timestamp Guard for Stat Comparison**: Traces' `FileDelta::compute` (`src/index/delta.rs:23-65`) compares `mtime` and `size` without checking if `mtime >= last_refresh_timestamp`. On filesystems with coarse (1-second) timestamps, a script modifying two notes within the same second as `traces` execution can cause the second edit to be silently skipped. Adopting Git's `is_racy` rule (read content if `mtime >= store_timestamp`) closes this latent staleness window.

---

## 5. Lessons That Do NOT Transfer and Why

- **[H] Fine-Grained Query DepGraph (rustc / Salsa)**: rustc's `DepNode` and red-green tracking is designed for multi-stage compilers where type checking, borrow checking, and LLVM IR generation form a deep DAG with significant change cutoff leverage. Traces' index pipeline is shallow (scan -> parse markdown -> resolve links -> persist). Maintaining a query dep-graph for note indexing would add massive memory overhead without intermediate cutoff benefits.
- **[H] Directory-Level Stat Aggregation (Mercurial Treestate)**: Mercurial's `AggregatedState` prefix tree accelerates `hg status` in repositories with 500,000+ files where entire subtrees remain untouched for months. Traces vaults are personal PKM repositories (typically 500 to 20,000 files) where a flat Rayon parallel directory walk (`DirTree::descendants`, `dirtree.rs:5`) scans the entire vault in 5–15 ms. Adding a prefix tree disk structure is unjustified complexity.
- **[H] Directory-Per-Hash Cache Layout (Ruff / rumdl)**: Ruff and rumdl isolate different configs into separate directories (`.ruff_cache/{version}/{hash}/`). This works because their cache entries are independent loose files. Traces stores a unified relational graph with reverse indices in a single redb database (`index.redb`); maintaining multiple concurrent database files per config hash would bloat disk usage and fragment workspace state.

---

## 6. Impact on Traces Hypotheses

### [H3] In-Store Config Drift Invalidation (ADR-0009 / Issue 17) — Strongly Confirmed, Design Simplified

- **Status**: [V] confirmed that config drift invalidation is universal across Cargo, rustc, Ruff, and rumdl.
- **Design Impact**: Challenges the speculative complexity of `ConfigDigests` (412 lines + `syn` AST test in `config/model/digest.rs`). Following Ruff's lesson, Traces should delete the `syn` AST visitor and replace `ConfigDigests` with an index-owned `IndexFingerprint` constructed by exhaustive destructuring of the config sub-models actually consumed by `IndexerService` (`TaskConfig`, `FrontmatterConfig`, and `class_field`). Storing this fingerprint in redb's `meta` table (renamed from `cache_format`) enables instant fail-closed drift detection and auto-rebuild.

### [H2] Inverted Outlink Stem Index to Avoid Full-Vault Deserialization — Strongly Confirmed & Parity-Proven

- **Status**: [V] confirmed by `advanced-metadata-cache` (`backlink-cache-component.ts:259-275`) and rumdl (`workspace_index.rs:897-921`).
- **Design Impact**: Overturns the premise in `src/index/refresh.rs:344` that path-set changes require total re-resolution. Maintaining a redb table of unresolved stems (`unresolved_stems: Table<&str, &[Path]>`) allows additions and deletions to re-resolve only the small subset of notes referencing the affected stems, completely eliminating the path-deletion performance cliff.

### [H1] Consolidate Dual Query Execution into Pure WorkspaceIndex — Confirmed

- **Status**: [V] confirmed by Git (`gix-index`) and Dataview (`dataview/src/data-index/index.ts:36-40`).
- **Design Impact**: Git and Dataview maintain flat, contiguous in-memory structures and evaluate queries in-memory. Traces' secondary redb tables (`paths_by_tag`, `paths_by_file_class`, `tags_by_path`, `classes_by_path`) exist solely to support `run_from_store` partial scans, paying $O(k)$ reverse updates on every write. Removing them in favor of loading `WorkspaceIndex` in memory simplifies storage without violating freshness invariants.
