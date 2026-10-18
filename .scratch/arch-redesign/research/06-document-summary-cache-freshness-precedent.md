# Research Precedent: Syntax vs Persisted Summary, Caches, and Freshness

**Ticket:** `.scratch/arch-redesign/issues/06-research-document-summary-cache-freshness-precedent.md`  
**Targets:** Informs ticket 14 (`note-document-model.md`) and ticket 15 (`index-read-model.md`).  
**Status:** Complete  
**Date:** 2026-10-18

---

## Executive Summary: Three Core Architectural Answers

1. **When is mtime+size freshness unsafe, and what mitigation is standard?**
   - **Unsafe Condition:** When a file is modified in-place without changing byte length within the filesystem timestamp resolution window (e.g., 1-second resolution on older/network filesystems, or sub-millisecond/sub-tick modifications on fast filesystems where write completes in the same clock tick as the cache/index write). The file modification timestamp ($t_{\text{mtime}}$) equals or exceeds the index record timestamp ($t_{\text{index}}$), rendering mtime+size identical while contents have mutated ("racy timestamp").
   - **Standard Mitigations:**
     - *Git (`racy-git.txt`, `ce_match_stat`)*: Marks entries where $t_{\text{mtime}} \ge t_{\text{index}}$ as "racily clean" and forces a content hash check (`ce_modified_check_fs()`). When writing the index, it truncates the cached file size of racily clean entries to 0 on disk to ensure subsequent stat checks trigger a size mismatch.
     - *Cargo (`cargo::core::compiler::fingerprint`)*: Touches an `invoked.timestamp` anchor before the build begins, and on completion rewinds the dep-info/anchor mtime back to the build start time to ensure any concurrent modification during execution is strictly newer.
     - *Mercurial dirstate*: Flags ambiguous timestamps where file mtime equals dirstate commit time with a sentinel value (`-1`), forcing hash re-verification on subsequent checks.
     - *Ruff & rumdl (`src/cache.rs`)*: Do not rely on mtime for validity; they key caches strictly on fast content hashes (BLAKE3) plus configuration fingerprints.

2. **Do comparables persist syntax trees or only derived summaries?**
   - **Evidence:** Universally, comparables persist **only derived semantic summaries** (or analysis diagnostics), never lossless concrete syntax trees (CSTs) or full ASTs.
     - *rustc (`rustc-dev-guide/queries/incremental-compilation.md`)*: Persists the dependency graph and hashes of query results, and selectively serializes downstream artifacts (e.g., optimized MIR, object code). It never serializes parse trees across compilations because re-parsing is cheaper than deserializing large AST structures.
     - *rowan / rust-analyzer*: Keeps green/red CSTs purely transient in memory; persistent state consists of salsa memoized semantic queries.
     - *rumdl (`src/cache.rs`)*: Caches only diagnostic summaries (`Vec<LintWarning>`); `ParsedDocument` is transiently allocated during offset iteration and dropped immediately per file.
     - *Obsidian Dataview (`src/data-index/index.ts`, `data-model/markdown.ts`)*: Persists only `PageMetadata` (frontmatter fields, tags, links, task/list metadata) to IndexedDB; markdown ASTs are never stored.
     - *Obsidian Tasks (`src/Obsidian/Cache.ts`)*: Caches only parsed `Task[]` records in memory, deriving them on demand from Obsidian's list item metadata.

3. **Do any comparables use in-memory inverted indexes where scans cost microseconds?**
   - **Evidence:** Yes. Tools serving responsive query interfaces over vaults or workspaces (e.g., Dataview, Obsidian Advanced Metadata Cache) maintain in-memory inverted index maps (`tags`, `links`, `prefix`, `backlinks`) populated from loaded summaries.
     - In Dataview (`FullIndex`), queries execute against in-memory `Map`s in microseconds/milliseconds (<10 ms target); disk/IndexedDB is used strictly as a cold-start warm cache, not as an execution query engine.
     - In Traces (research 03), redb open costs ~16 ms and commit costs ~37 ms, whereas in-memory linear scans and queries cost microseconds (0.4–10 µs per query). Using redb secondary tables (`PATHS_BY_TAG`, `PATHS_BY_FILE_CLASS`) for selective point queries forces disk access and index opens that run orders of magnitude slower than evaluating in-memory summaries.

---

## Lessons for Ticket 14 and Ticket 15

### Lesson 1: Racy Timestamp Vulnerability in File-Stat Caches (`VERIFIED`)

- **Tags:** [15]
- **Grade:** `VERIFIED` (Git technical docs: `racy-git.adoc`; Cargo `fingerprint/mod.rs:248-256`; Mercurial `mercurial/dirstate.py`)
- **Precedent:**
  Relying solely on filesystem `(mtime, size)` to detect file freshness suffers from the classic "racy timestamp" problem: if a file is edited, saved, and re-edited without altering its size within the resolution of the filesystem clock, $t_{\text{mtime}}$ does not advance past the cache's recorded timestamp.
  - *Git*: If `st_mtime` is identical to (or newer than) the index file timestamp, git marks the entry as racily clean and computes its object SHA-1 hash to verify content. When updating the index, git truncates cached `st_size` to 0 for ambiguous entries to force an invalidation on next run.
  - *Cargo*: Cargo creates `invoked.timestamp` before build execution starts and rewinds the dep-info mtime to that start time upon completion. Cargo explicitly documents: *"We strive to never include a modification time inside a Fingerprint to get hashed into an actual value... In practice this causes issues with common applications like Docker [which zero nanoseconds]... Instead we only track paths which are checked dynamically."*
- **Transferability Test for Traces:**
  - *Context:* Traces is a synchronous CLI operating on local markdown vaults up to 20K notes. In automated scripts, git checkouts, or fast editor autosaves, notes can be modified within the same millisecond or sub-second window.
  - *Application:* If Traces continues using `mtime + size` in `FileMeta` for quick change detection, it must not treat `stat.mtime == cached.mtime` as clean when the file mtime matches the store's last refresh timestamp. Either: (a) treat matching timestamps as potentially dirty and verify content hash (BLAKE3), or (b) rewind the index timestamp to invocation start.
- **Negative Lesson:**
  Never hash filesystem `mtime` directly into persistent cache keys or fingerprints. Changes in filesystem timestamp granularity (e.g., containers, network drives, git checkouts) invalidate every cache key globally without any content change.

---

### Lesson 2: Concrete Syntax Trees Are Transient; Only Derived Summaries Persist (`VERIFIED`)

- **Tags:** [14], [15]
- **Grade:** `VERIFIED` (rustc-dev-guide `incremental-compilation.html`; rumdl `src/cache.rs:426-445`, `src/parsed_document.rs`; Dataview `src/data-model/markdown.ts:4861-4886`)
- **Precedent:**
  Across compilers (rustc), linters (Ruff, rumdl), and PKM indexes (Dataview, Advanced Metadata Cache), **concrete syntax trees and token streams are never serialized to disk**.
  - *rustc*: Parses source to AST in memory; serializes only query dependency graphs and compact downstream query results (HIR/MIR/dep-graph). It does not serialize the raw AST because parsing is faster than deserializing large pointer-heavy trees.
  - *rumdl*: Parses markdown using `pulldown-cmark::Parser::into_offset_iter()` into a transient `ParsedDocument` containing borrowed events and byte spans. It runs lint rules against this transient structure, discards it, and writes only `Vec<LintWarning>` to disk cache.
  - *Dataview*: Extracts `PageMetadata` (record of frontmatter values, tag sets, link targets, list items) and serializes only this summary into IndexedDB. The markdown AST is discarded immediately after worker parsing.
- **Transferability Test for Traces:**
  - *Context:* Traces research 03 shows `parse_markdown` takes ~10 µs for a medium note and ~1 µs for a small note. Parsing 20,000 notes on multiple Rayon cores takes ~48 ms. In redb, serializing and deserializing rich ASTs or span-heavy trees would drastically inflate redb database size and deserialization latency (currently 10.7 ms for 20K minimal `Note` records).
  - *Application (hypothesis for Ticket 14, not a decision):* Ticket 14 should evaluate splitting the in-memory syntactic document model (`ParsedNote` / analysis AST with byte-precise spans and occurrences for LSP) from the persisted read-model summary (`NoteSummary` / metadata facets: frontmatter, tags, links, headings, tasks). The index store should persist only `NoteSummary`. If LSP needs exact byte spans for a note, it parses that single note in memory in ~10 µs.
- **Negative Lesson:**
  Do not persist full ASTs, syntax nodes, or byte-offset span trees in redb. Persisting syntax trees creates immense serde overhead, couples database schema to parser internals, and costs more in deserialization than re-parsing on demand.

---

### Lesson 3: In-Memory Inverted Indexing Beats Embedded Storage Engine Queries (`VERIFIED`)

- **Tags:** [15]
- **Grade:** `VERIFIED` (Dataview `src/data-index/index.ts:4043-4053`, `docs-digest:574-575`; Advanced Metadata Cache `README.md:100-104`; Traces research 03)
- **Precedent:**
  In PKM tools managing vaults up to 100K notes:
  - *Dataview*: Stores primary summaries in `pages: Map<string, PageMetadata>` and builds in-memory inverted indices (`tags: ValueCaseInsensitiveIndexMap`, `links: IndexMap`, `prefix: PrefixIndex`). All DQL and JS queries execute purely in memory against these maps (<10 ms target). IndexedDB is strictly a backing store for startup acceleration.
  - *Advanced Metadata Cache*: Implements in-memory backlink and name indices (`title-index.ts`, `backlink-cache-component.ts`) because querying un-indexed disk files or rescanning on every interaction creates unacceptable UI lag.
  - *Traces Research 03 Findings:* In Traces, in-memory queries take microseconds ($0.4\text{--}10\ \mu\text{s}$). Meanwhile, opening redb has a hard fixed floor of ~16 ms, and point-reading secondary tables (`PATHS_BY_TAG`, `NOTES`) on a 20K vault takes 55 ms.
- **Transferability Test for Traces:**
  - *Context:* Traces CLI queries currently have two paths (`run` vs `run_from_store`). CLI uses `run_from_store`, which point-queries redb tables (`PATHS_BY_TAG`, `PATHS_BY_FILE_CLASS`), incurring table open, B-tree lookups, and batch note deserializations.
  - *Application:* Using redb as a live query engine with secondary multimap tables provides negative architectural value. Loading `NoteSummary` records into an in-memory index structure (with in-memory tag/link inverted maps) allows all queries (CLI and templates) to share a single in-memory execution engine operating in microseconds.
- **Negative Lesson:**
  Do not maintain secondary index tables (`PATHS_BY_TAG`, `TAGS_BY_PATH`, `PATHS_BY_FILE_CLASS`, `FILE_CLASSES_BY_PATH`) in an embedded ACID KV store like redb. Updating secondary tables in redb adds 4–23 ms to write transactions and requires multimap maintenance, while providing zero speed advantage over in-memory index scans on 20K vaults.

---

### Lesson 4: Cache Keying on Content Hash + Config Fingerprint (`VERIFIED`)

- **Tags:** [14], [15]
- **Grade:** `VERIFIED` (rumdl `src/cache.rs:327, 546-573`; Ruff cache architecture; Cargo `fingerprint/mod.rs:73-97`)
- **Precedent:**
  Mature tools use a deterministic multi-part cache key:
  $$\text{Key} = \text{Hash}(\text{FileContent}) + \text{Hash}(\text{Configuration}) + \text{Hash}(\text{Rules/Schema}) + \text{ToolVersion}$$
  - *rumdl*: Computes `file_hash = blake3(content)`, `config_hash = blake3(json(config))`, and `rules_hash = blake3(sorted(rule_names))`. It explicitly notes: *"The hash must be stable across repeated loads... Never use HashMap in a serialized config field — Rust's RandomState randomizes iteration per-instance and breaks this invariant. Use BTreeMap or IndexMap."*
  - *Cargo*: Incorporates rustc version, profile, package source relative path, and compile flags into its unit fingerprint.
- **Transferability Test for Traces:**
  - *Context:* Traces is currently landing config-drift detection via `ConfigDigests` and `IndexFingerprint` in the redb `meta` table (Issues 07/17).
  - *Application (hypothesis for Ticket 15, not a decision):* Consistent with rumdl and Cargo, any cache entry or index generation should bind to the configuration fingerprint and codec version. If the configuration (e.g. tag syntax, frontmatter keys) drifts, the index knows immediately without rescanning individual notes.
- **Negative Lesson:**
  Never hash non-deterministic structures (such as standard `std::collections::HashMap` or raw memory addresses) into configuration or content fingerprints.

---

### Lesson 5: Parallel Directory Walking Amortizes Filesystem Metadata Cost (`VERIFIED`)

- **Tags:** [15]
- **Grade:** `VERIFIED` (`ignore::WalkBuilder`, `ripgrep`, rumdl `src/check_runner.rs:1788-1817`, Traces research 03)
- **Precedent:**
  In research 03, directory scan and `stat` across 20,000 files in a single thread takes 40–55 ms, representing over 90% of a no-op refresh and ~55 ms of a 2-row selective query floor.
  - *ripgrep / `ignore` crate*: Parallelizes directory traversal with a thread pool (`WalkBuilder::threads(n).build_parallel()`). Directory traversal reads `d_type` directly from `readdir(3)` directory entries (avoiding `stat` for non-files/subdirectories).
  - *rumdl*: Collects file lists and runs Rayon parallel iterators (`file_tasks.par_iter()`) across available CPU cores.
- **Transferability Test for Traces:**
  - *Context:* Traces runs on multi-core systems (Apple Silicon arm64, x86_64). File system traversal and `stat` calls are I/O-bound and OS-scheduling bound.
  - *Application:* Traces' `refresh_store` directory walk can be parallelized using Rayon or `ignore::WalkBuilder`. Parallel statting across 8–10 cores reduces the 40–45 ms floor to <10 ms on 20K vaults, removing the primary bottleneck of CLI execution.
- **Negative Lesson:**
  Do not perform sequential `std::fs::metadata` calls across 20,000 paths on the main thread when checking freshness.

---

### Lesson 6: Embedded KV Storage Engine vs Derived Cache Semantics (`VERIFIED`)

- **Tags:** [15]
- **Grade:** `VERIFIED` (redb architecture docs; LMDB design docs; Traces research 03: §2.1)
- **Precedent:**
  Embedded key-value engines (redb, SQLite, LMDB) have distinct transaction cost profiles:
  - *redb*: Implements copy-on-write B-trees with strict ACID durability. A commit with `fsync` has a hard hardware floor of 36–38 ms on Apple Silicon SSDs (accounting for 71.3% of Traces' cold build on 1,000 notes). Database open (`Database::open`) takes 15–16 ms to read file headers, check allocator state, and prepare transaction contexts.
  - *LMDB*: Uses memory-mapped files (`mmap`). Read transactions incur near-zero open overhead (no parsing or memory allocation), but write transactions still pay fsync penalties unless durability is weakened (`MDB_NOSYNC`).
- **Transferability Test for Traces:**
  - *Context:* Traces uses redb for `.traces/index.redb`. The index is a **derived, rebuildable cache**, not an authoritative system of record (the markdown files are authoritative).
  - *Application:* Because the index is derived, paying synchronous `fsync` overhead on every single incremental update or CLI query is unnecessary. Traces can:
    1. Treat redb strictly as a warm-start serialization cache for primary records (`NOTES` and `FILES`), loading them into an in-memory `WorkspaceIndex` at startup.
    2. Eliminate secondary index tables from redb, cutting row insertion overhead.
    3. If redb fails to open or is corrupt, fail open by rebuilding directly into memory without aborting CLI queries.
- **Negative Lesson:**
  Never treat an embedded ACID KV store as an in-process relational database for high-frequency CLI operations when the entire data model fits in tens of megabytes of RAM.

---

### Lesson 7: On-Demand Event-Driven Parsing for Localized Span Resolution (`VERIFIED`)

- **Tags:** [14]
- **Grade:** `VERIFIED` (rumdl `ParsedDocument`; `pulldown-cmark::Parser::into_offset_iter`; Traces research 03: §4.1)
- **Precedent:**
  In markdown tooling, pull-parsers like `pulldown-cmark` do not allocate full AST nodes unless requested.
  - *rumdl*: Uses `Parser::new_with_broken_link_callback(...).into_offset_iter()`, inspecting `(Event, Range<usize>)` tuples in a single pass. It extracts only link targets, code block bounds, and heading spans, avoiding duplicate parser passes.
  - *Traces Verification Ledger 01 (§6)*: Traces currently runs `pulldown-cmark` twice (once in `parse_markdown` without math, once with math) and lexes list-item text three times (`list.rs:402`, `:420`, `task.rs:33`). All byte spans are subsequently discarded.
- **Transferability Test for Traces:**
  - *Context:* Traces notes parse in ~10 µs. Language server features (definition, hover, occurrences) operate on one active buffer at a time, not 20,000 notes simultaneously.
  - *Application (hypothesis for Ticket 14, not a decision):* Ticket 14 could eliminate the double-parsing pass in `note/parser.rs` by configuring `pulldown-cmark` once with all required options and extracting semantic summaries and spans in a single event pass. When the LSP needs precise byte spans for an open note, it parses the note on demand in memory.
- **Negative Lesson:**
  Never run multiple parsing passes over the same document text with differing configuration options when a single superset event pass can collect all required facets.

---

### Lesson 8: Live-Buffer Overlay via Layered In-Memory Snapshots (`VERIFIED`)

- **Tags:** [15]
- **Grade:** `VERIFIED` (rust-analyzer `Vfs` / Salsa snapshot design; Obsidian Tasks `Cache.ts:10137-10175`; Dataview `FullIndex:4180-4252`)
- **Precedent:**
  How do live hosts (LSP, IDE plugins) reconcile unsaved buffers with persistent disk caches?
  - *rust-analyzer*: The virtual file system (`vfs`) holds file contents in memory. Unsaved editor buffers overlay disk contents in memory. Query execution runs against Salsa database snapshots without touching disk or invalidating unrelated files.
  - *Obsidian Dataview & Tasks*: In-memory cache maintains `pages: Map<string, PageMetadata>` or `tasks: Task[]`. When a file changes or is edited in memory, the listener removes old entries for that specific path and inserts new parsed entries (`this.pages.set(file.path, meta)`), followed by an event trigger (`this.touch()`). It never triggers a global cache re-read.
- **Transferability Test for Traces:**
  - *Context:* Ticket 15 requires an in-memory mutable/overlayable read model for LSP support that does not write to redb while typing.
  - *Application:* An immutable `Arc<WorkspaceIndex>` snapshot can be paired with an ephemeral in-memory overlay map:
    $$\text{ReadModel} = \text{Overlay}(\text{UnsavedBuffers}) \cup \text{Snapshot}(\text{WorkspaceIndex})$$
    When an LSP client sends `textDocument/didChange`, Traces parses the changed buffer in ~10 µs, places the resulting `NoteSummary` in the overlay, and answers queries instantaneously without acquiring redb locks or touching disk.
- **Negative Lesson:**
  Do not serialize unsaved editor buffer changes to disk or an embedded database during interactive typing sessions.

---

## Provenance Ledger

| Lesson | Comparable / Source | Reference / Citation | Verification Grade |
|---|---|---|---|
| 1. Racy Timestamps | Git, Cargo, Mercurial | Git `Documentation/technical/racy-git.adoc`; Cargo `fingerprint/mod.rs:248-256`; Mercurial `mercurial/dirstate.py` | `VERIFIED` |
| 2. Syntax Trees vs Summaries | rustc, rowan, rumdl, Dataview | rustc-dev-guide `incremental-compilation.html`; rumdl `src/cache.rs:426-445`; Dataview `src/data-model/markdown.ts:4861` | `VERIFIED` |
| 3. In-Memory Inverted Index | Obsidian Dataview, Advanced Metadata Cache | Dataview `src/data-index/index.ts:4043-4053`; Advanced Metadata Cache `README.md:100-104`; Traces research 03 | `VERIFIED` |
| 4. Cache Keys & Fingerprints | rumdl, Ruff, Cargo | rumdl `src/cache.rs:546-573`; Cargo `fingerprint/mod.rs:73-97` | `VERIFIED` |
| 5. Parallel Directory Walking | ripgrep, ignore crate, rumdl | `ignore::WalkBuilder`; rumdl `src/check_runner.rs:1788-1817`; Traces research 03 | `VERIFIED` |
| 6. Embedded KV Cost Behavior | redb, LMDB | redb architecture docs; Traces research 03 (§2.1 Criterion baselines) | `VERIFIED` |
| 7. Single-Pass Event Parsing | pulldown-cmark, rumdl | rumdl `src/parsed_document.rs:27598-27651`; Traces research 01 (§6) & 03 (§4.1) | `VERIFIED` |
| 8. Live-Buffer Overlay | rust-analyzer, Dataview, Tasks | Dataview `FullIndex::finish`: 4238; Tasks `Cache.ts:10137`; rust-analyzer Vfs design | `VERIFIED` |
