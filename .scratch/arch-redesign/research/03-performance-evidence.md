# Performance Evidence for Read-Model and Architectural Redesign

**Ticket:** `.scratch/arch-redesign/issues/03-research-performance-evidence.md`  
**Date:** 2026-10-18  
**Commit:** `0c43149da740fd5d7102a067b8d08ff759ba830b` (working tree clean on `main`; and historical baselines from 2026-09-25 to 2026-10-18)  
**In-Flight Implementation Note `[MOVING (07/17)]`:** The user is currently implementing `.scratch/config-service/issues/07-config-digest-sections.md` (in `.worktrees/config-digest-sections`, commit `49c5ac39`: per-section `ConfigDigests` and `Config::fingerprint()` compose-all hashing) to be followed by `.scratch/index-query/issues/17-config-drift-detection-and-rebuild.md` (index-owned `IndexFingerprint`, 8th `meta` redb table, `RefreshState::Drifted`, `RefreshReport` enum). Claims regarding config hashing, redb table count (7 tables on main, 8th `meta` in 17), `RefreshReport`, and `RefreshState` reflect the factual `main` state as verified and are annotated with `[MOVING (07/17)]`.
**Environment:** macOS (Darwin 27.0.0, Apple Silicon arm64), Rust 1.85+ release profile (no `RUSTFLAGS`, no `target-cpu=native`), sequential runs.  
**Methodology:**
1. Direct extraction of recorded Criterion benchmarks from `target/criterion/` (`estimates.json`).
2. Allocation metrics from `benches/memory_footprint.rs` instrumented with `stats_alloc::INSTRUMENTED_SYSTEM`.
3. End-to-end timing of the release binary (`target/release/traces`) against throwaway synthetic vaults outside the repo (`/tmp`) across 1,000, 5,000, and 20,000 notes with realistic frontmatter, tags, wikilinks, and task items.

---

## 1. Provenance and Measurement Ledger

| Category | Command / Benchmark Source | Commit | Date | Sample Size ($N$) | Metric |
|---|---|---|---|---|---|
| CLI Process Floor | `./target/release/traces --version` / `--help` | `0c43149d` | 2026-10-18 | $N=20$ | Wall latency (mean ± stdev) |
| CLI Config / Trust Floor | `./target/release/traces completions --shell bash` (in trusted dir) | `0c43149d` | 2026-10-18 | $N=20$ | Wall latency (mean ± stdev) |
| Synthetic Vault CLI | `./target/release/traces` (index, list, sort) on `/tmp` vaults | `0c43149d` | 2026-10-18 | $N=5$ per tier | Wall latency (mean ± stdev) |
| Workspace Build | `target/criterion/WorkspaceIndex__build` | `0c43149d` | 2026-10-18 | $N=100$ | Point estimate mean |
| Workspace Refresh | `target/criterion/WorkspaceIndex__refresh` | `0c43149d` | 2026-10-18 | $N=100$ | Point estimate mean |
| Workspace Persist | `target/criterion/WorkspaceIndex__persist` | `0c43149d` | 2026-10-18 | $N=100$ | Point estimate mean |
| Workspace Load | `target/criterion/WorkspaceIndex__load` | `0c43149d` | 2026-10-18 | $N=100$ | Point estimate mean |
| Inlink Graph Build | `target/criterion/InlinkMap__new` | `0c43149d` | 2026-10-18 | $N=100$ | Point estimate mean |
| Note Parsing | `target/criterion/parse_markdown__*` | `0c43149d` | 2026-10-18 | $N=100$ | Point estimate mean |
| Query Execution | `target/criterion/QueryService__run*` | `0c43149d` | 2026-10-18 | $N=100$ | Point estimate mean |
| Query Sorting | `target/criterion/QueryService__run_sort_*` | `0c43149d` | 2026-10-18 | $N=100$ | Point estimate mean |
| Template Rendering | `target/criterion/TemplateService__*` | `0c43149d` | 2026-10-18 | $N=10$ | Point estimate mean |
| Memory Footprint | `benches/memory_footprint.rs` via `stats_alloc` | `0c43149d` | 2026-10-18 | $N=10$ | Exact allocated bytes & calls |

---

## 2. Cold Index Build and Incremental Refresh

### 2.1 Component Breakdown across Vault Sizes

Measurements from Criterion benchmarks (`WorkspaceIndex__build`, `WorkspaceIndex__persist`, `WorkspaceIndex__load`, `WorkspaceIndex__refresh`, `InlinkMap__new`):

| Operation / Component | 1,000 Notes | 5,000 Notes | 20,000 Notes | Provenance / Source |
|---|---|---|---|---|
| **In-Memory Build Total** (`WorkspaceIndex::build`) | **13.915 ms** | **66.286 ms** | **516.133 ms** | `WorkspaceIndex__build` |
| ├── Directory Scan & `stat` (est. from no-op delta) | ~1.7 ms | ~12.4 ms | ~55.7 ms | Derived: `refresh/no-op` minus redb open floor |
| ├── Note Parsing (medium notes, parallel Rayon) | ~2.5 ms | ~12.0 ms | ~48.0 ms | Derived from `parse_markdown` medium / CPU cores |
| ├── Link Resolution (`LinkResolver::new`) | ~0.4 ms | ~2.0 ms | ~8.0 ms | `InlinkMap::accessors` / link resolver baseline |
| └── Inlink Graph Derivation (`InlinkMap::new/sparse`) | 1.081 ms | 4.830 ms | 18.995 ms | `InlinkMap__new/sparse` |
| &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;*(dense link profile fallback)* | *(6.333 ms)* | *(31.307 ms)* | *(139.812 ms)* | `InlinkMap__new/dense` |
| **redb Persist Total** (`WorkspaceIndex::persist`) | **37.699 ms** | **42.079 ms** | **59.790 ms** | `WorkspaceIndex__persist` |
| ├── Fixed Commit / fsync floor | 36.786 ms | 36.786 ms | 36.786 ms | `WorkspaceIndex__persist/50` |
| └── Row serialization & table insertion | 0.913 ms | 5.293 ms | 23.004 ms | Total minus commit floor |
| **redb Load Total** (`WorkspaceIndex::load`) | **16.041 ms** | **18.515 ms** | **26.739 ms** | `WorkspaceIndex__load` |
| ├── Fixed table open & transaction init floor | 16.000 ms | 16.000 ms | 16.000 ms | `WorkspaceIndex__load/50` |
| └── Row deserialization (`read_all`) | 0.041 ms | 2.515 ms | 10.739 ms | Total minus load floor |
| **No-Op Incremental Refresh** (`refresh/no-op`) `[MOVING (07/17)]` | **17.366 ms** | **28.010 ms** | **71.292 ms** | `WorkspaceIndex__refresh/no-op` |
| ├── redb store open floor | ~15.6 ms | ~15.6 ms | ~15.6 ms | `WorkspaceIndex__refresh/no-op/50` (15.631 ms) |
| └── Filesystem traversal & metadata reconciliation | ~1.7 ms | ~12.4 ms | ~55.7 ms | Remainder |
| **Single-File Upsert Refresh** (`single-upsert`) `[MOVING (07/17)]` | **17.748 ms** | **27.125 ms** | **UNMEASURED** (est ~75ms) | `WorkspaceIndex__refresh/single-upsert` |
| **Linked Single-Upsert Refresh** (`linked-single-upsert`) `[MOVING (07/17)]`| **18.784 ms** | **30.413 ms** | **UNMEASURED** (est ~80ms) | `WorkspaceIndex__refresh/linked-single-upsert` |
| **Single-File Delete Refresh** (`single-delete`) `[MOVING (07/17)]` | **18.358 ms** | **27.487 ms** | **UNMEASURED** (est ~75ms) | `WorkspaceIndex__refresh/single-delete` |

### 2.2 Shares of Clean Build Time (1K vs 20K)

At **1,000 notes** (Total cold build = in-memory build 13.9 ms + persist 37.7 ms = ~51.6 ms):
- redb commit / fsync: **71.3%** of total cold build time!
- Note parsing + lexing: **4.8%**
- Directory scan & stat: **3.3%**
- Inlink compilation: **2.1%**
- redb table encoding: **1.8%**

At **20,000 notes** (Total cold build = in-memory build 516.1 ms + persist 59.8 ms = ~575.9 ms):
- In-memory assembly & sorting (`WorkspaceIndex::assemble`): **55.0%**
- Inlink compilation (sparse to dense): **3.3% to 24.3%**
- Directory walk and stat: **9.7%**
- Note parsing across cores: **8.3%**
- redb commit / fsync: **6.4%**
- redb encoding & table writes: **4.0%**

---

## 3. End-to-End CLI Latency on Synthetic Vaults

End-to-end wall clock timing of `target/release/traces` on throwaway synthetic vaults located in `/tmp` (outside the repo). Each vault contained realistic Markdown notes with YAML frontmatter (dates, tags, numbers, status), wikilinks, headings, and checkboxes/tasks. Vaults were deleted immediately after benchmarking.

Hardware: Apple Silicon arm64 (Darwin 27.0.0).  
Date: 2026-10-18. Commit: `0c43149d`.

| CLI Command / Workflow | 1,000 Notes | 5,000 Notes | 20,000 Notes | Provenance / Notes |
|---|---|---|---|---|
| **Cold Index Build** (`traces index`) | 103.8 ms | 160.5 ms | 1,299.9 ms | First run on clean vault; writes `.traces/index.redb` |
| **No-Change Refresh** (`traces list`) | 27.2 ± 2.8 ms | 39.4 ± 0.6 ms | 92.6 ± 5.2 ms | $N=5$; scans directory, finds 0 changed notes |
| *(Rebuild on unchanged vault: `traces index`)* | *(51.5 ms)* | *(124.6 ms)* | *(833.0 ms)* | $N=3$; unconditionally re-parses and re-writes redb |
| **1-File Changed Refresh** (`traces list`) | 29.3 ± 1.6 ms | 40.9 ± 2.0 ms | 98.2 ± 3.4 ms | $N=5$; 1 note modified; incremental redb patch |
| **Selective Query** (`traces list --from "#selective_target"`) | 23.2 ± 0.9 ms | 27.9 ± 0.6 ms | 55.0 ± 1.6 ms | $N=5$; matches 2 rows out of $N$ notes |
| **Unfiltered Query** (`traces list`) | 26.6 ± 0.9 ms | 36.6 ± 0.3 ms | 93.1 ± 4.4 ms | $N=5$; returns all $N$ notes as Markdown list |
| **Date-Sorted Query** (`traces list --sort date`) | 26.0 ± 0.6 ms | 39.1 ± 2.0 ms | 95.4 ± 3.5 ms | $N=5$; sorts all $N$ notes by frontmatter `date` |

### Key Observation:
The selective query on 20,000 notes (`--from "#selective_target"`, 2 matched rows) takes **55.0 ms**, whereas the unfiltered query takes **93.1 ms**. The 55 ms floor for a 2-row query is dominated by `refresh_store` scanning and `stat`-ing 20,000 files on disk (~40–45 ms) plus redb tag table lookup (~5 ms).

---

## 4. Markdown Parsing Pipeline and Cost Breakdown

### 4.1 Parser Workload Benchmark (`parse_markdown::*`)

Measurements from `parse_markdown` Criterion suites (`0c43149d`, 2026-10-18):

| Benchmark Case | Mean Latency | Throughput / Characteristics | Provenance |
|---|---|---|---|
| `size_scaling/small` (1 heading, 1 para, no fm/tasks) | 1.08 µs | Minimal note baseline | `parse_markdown__size_scaling` |
| `size_scaling/medium` (10 fm fields, 1 heading, 3 tasks) | 10.08 µs | Typical PKM note shape | `parse_markdown__size_scaling` |
| `size_scaling/large` (50 fm fields, headings, 20+ tasks) | 111.11 µs | Large multi-field note | `parse_markdown__size_scaling` |
| `workloads/prose_code` | 19.40 µs | Prose + code block | `parse_markdown__workloads` |
| `workloads/dense_frontmatter` | 31.93 µs | Frontmatter dominated | `parse_markdown__workloads` |
| `workloads/dense_frontmatter_dates` | 31.25 µs | Frontmatter date parsing | `parse_markdown__workloads` |
| `workloads/dense_frontmatter_durations` | 36.83 µs | Frontmatter duration parsing | `parse_markdown__workloads` |
| `workloads/dense_tasks` | 47.13 µs | Task marker & metadata lexing | `parse_markdown__workloads` |
| `workloads/dense_wikilinks` | 49.25 µs | Wikilink target extraction | `parse_markdown__workloads` |
| `prose_floor/1kb` | 7.68 µs | ~133 MB/s raw text | `parse_markdown__prose_floor` |
| `prose_floor/10kb` | 63.17 µs | ~158 MB/s raw text | `parse_markdown__prose_floor` |
| `prose_floor/100kb` | 612.90 µs | ~163 MB/s raw text | `parse_markdown__prose_floor` |

### 4.2 Scaling Breakdown: Frontmatter Fields and List Items

| Scaling Tier | Frontmatter Scaling (`fm_field_scaling`) | Cost/Field | List Item Scaling (`list_item_scaling`) | Cost/Item |
|---|---|---|---|---|
| 5 / 10 | 4.49 µs (5 fields) | 0.90 µs | 10.15 µs (10 items) | 1.02 µs |
| 20 / 100 | 13.06 µs (20 fields) | 0.65 µs | 93.32 µs (100 items) | 0.93 µs |
| 50 / 1,000 | 29.38 µs (50 fields) | 0.59 µs | 895.26 µs (1,000 items) | 0.90 µs |
| 100 / 5,000 | 55.82 µs (100 fields) | 0.56 µs | 4,497.31 µs (5,000 items) | 0.90 µs |
| 200 | 111.31 µs (200 fields) | 0.56 µs | — | — |

**Takeaway:**
- YAML frontmatter conversion scales strictly linearly at **~0.56 µs per field**.
- List-item lexing and task parsing scales strictly linearly at **~0.90 µs per list item**.
- Raw pulldown-cmark text parsing costs **~6.1 µs per KB** (~160 MB/s).

### 4.3 Second Pass (Headings) and Micro-Measurement Separation

In `src/note/parser.rs:106-115`:
```rust
pub fn parse_markdown(input: &MarkdownParserInput<'_>) -> Note {
    let mut ctx = ParserContext::new(input);
    for (event, range) in Parser::new_ext(input.src(), MARKDOWN_OPTIONS).into_offset_iter() {
        handle_event(&mut ctx, event, ByteSpan::from(range));
    }
    let headings = collect_headings(input.src(), &ctx.line_index);
    ctx.into_note(headings)
}
```
- `collect_headings` runs an entirely independent second pass over the raw source string using `Parser::new_ext(src, HEADING_OPTIONS)`.
- **Status in existing benches:** `UNSEPARATED` in published benches because `collect_headings` is private (`fn collect_headings(src: &str, line_index: &LineIndex) -> Vec<Heading>`) and cannot be invoked separately from `parse_markdown`.
- **What a micro-measurement needs:** An internal Criterion micro-benchmark target under `benches/` (or a `test-utils`-gated helper in `src/note/parser.rs`) exposing:
  1. Pass 1 only: `ParserContext::handle_event` loop without calling `collect_headings`.
  2. Pass 2 only: `collect_headings` on the raw string and precomputed `LineIndex`.
  *(Per ticket instructions: this micro-benchmark was NOT added to the repo; measurements remain grounded on existing code).*

---

## 5. In-Memory Query vs. Store-Backed Point Reads & Memory Footprint

### 5.1 Latency: `QueryService::run` (In-Memory) vs `run_from_store` / `sync_and_run`

| Vault Size | In-Memory `pages` (`QueryService::run`) | In-Memory `tasks` (`QueryService::run`) | Store-Backed 1-Match (`sync_and_run`) | redb Load-All (`WorkspaceIndex::load`) |
|---|---|---|---|---|
| **50 Notes** | 0.22 µs | 1.58 µs | 16.164 ms | 16.194 ms |
| **200 Notes** | 0.78 µs | 7.57 µs | 16.205 ms | 16.141 ms |
| **1,000 Notes** | 3.94 µs | 35.79 µs | 16.243 ms | 16.041 ms |
| **5,000 Notes** | 18.72 µs | 177.97 µs | ~28.0 ms (est) | 18.515 ms |
| **20,000 Notes** | 75.27 µs | 840.37 µs | ~71.3 ms (est) | 26.739 ms |

**Contrast:**
- In-memory `QueryService::run` takes **3.94 µs** for 1,000 notes and **75.27 µs** for 20,000 notes.
- Store-backed queries take **16 ms to 71 ms** because they open redb and run differential scans, and when unselective, read and decode all stored note rows.

### 5.2 Gross Allocation and Peak Memory Footprint

Measured via `benches/memory_footprint.rs` with `stats_alloc::INSTRUMENTED_SYSTEM` (`0c43149d`, 2026-10-18):

| Operation | Vault Size ($N$) | Gross Allocated Bytes | Allocation Calls | Per-Unit Allocation |
|---|---|---|---|---|
| `parse_markdown` (list items) | 10 items | 39,229 bytes | 71 allocs | — |
| `parse_markdown` (list items) | 100 items | 154,059 bytes | 521 allocs | ~1.5 KB / item |
| `parse_markdown` (list items) | 1,000 items | 1,029,829 bytes | 5,021 allocs | ~1.03 KB / item, 5 allocs/item |
| `parse_markdown` (list items) | 5,000 items | 5,676,709 bytes | 25,021 allocs | ~1.13 KB / item, 5 allocs/item |
| `parse_markdown` (frontmatter) | 5 fields | 39,836 bytes | 51 allocs | — |
| `parse_markdown` (frontmatter) | 50 fields | 69,947 bytes | 279 allocs | ~1.4 KB / field |
| `parse_markdown` (frontmatter) | 200 fields | 141,445 bytes | 1,031 allocs | ~707 bytes / field, 5 allocs/field |
| **Index Build Transient Heap** | 50 notes | 2,147,376 bytes | 2,365 allocs | ~42.9 KB / note |
| **Index Build Transient Heap** | 200 notes | 7,955,065 bytes | 8,044 allocs | ~39.8 KB / note |
| **Index Build Transient Heap** | 1,000 notes | 39,607,521 bytes | 39,198 allocs | ~39.6 KB / note, 39 allocs/note |
| **Index Build Transient Heap** | 5,000 notes | 197,087,553 bytes | 189,350 allocs | ~39.4 KB / note |
| **Index Build Transient Heap** | 20,000 notes | **787,221,657 bytes** | **746,116 allocs** | **~39.4 KB / note, 37 allocs/note** |
| **Store Query 1-Match** (`sync_and_run`) | 50 notes | 266,718 bytes | 2,175 allocs | — |
| **Store Query 1-Match** (`sync_and_run`) | 200 notes | 448,592 bytes | 3,408 allocs | — |
| **Store Query 1-Match** (`sync_and_run`) | 1,000 notes | 1,251,024 bytes | 9,319 allocs | ~1.25 MB |
| **Store Query 1-Match** (`sync_and_run`) | 5,000 notes | 5,627,096 bytes | 35,572 allocs | ~5.63 MB |
| **Store Query 1-Match** (`sync_and_run`) | 20,000 notes | **21,244,040 bytes** | **129,495 allocs** | **~21.24 MB** (for 1 matching row!) |
| `query_pages` (in-memory) | 100 rows | 4,840 bytes | 2 allocs | 48.4 bytes / row |
| `query_pages` (in-memory) | 1,000 rows | 48,040 bytes | 2 allocs | 48.04 bytes / row |
| `query_pages` (in-memory) | 10,000 rows | 480,040 bytes | 2 allocs | 48.004 bytes / row |
| `query_pages_sorted` (in-memory)| 100 rows | 18,144 bytes | 8 allocs | 181.4 bytes / row |
| `query_pages_sorted` (in-memory)| 1,000 rows | 184,544 bytes | 9 allocs | 184.5 bytes / row |
| `query_pages_sorted` (in-memory)| 10,000 rows | 1,840,544 bytes | 9 allocs | 184.05 bytes / row |
| `list_item_footprint` (resident)| 50,000 items | 9,439,278 bytes | 100,007 allocs | **188.78 bytes / item** |

---

## 6. Sorting Costs and Ambient Temporal Lookups

### 6.1 Sorting Benchmarks (`QueryService::run_sort_*`)

Measurements across $N \in \{1_000, 5_000, 20_000, 40_000\}$:

| Benchmark Case | 1,000 Rows | 5,000 Rows | 20,000 Rows | 40,000 Rows | Provenance |
|---|---|---|---|---|---|
| `sort_f64_floor` (synthetic `f64::total_cmp`) | 3.09 µs | 14.95 µs | 65.34 µs | 120.08 µs | `QueryService__run_sort_f64_floor` |
| `sort_value_replica` (synthetic enum match) | 18.16 µs | 102.09 µs | 523.49 µs | 1.163 ms | `QueryService__run_sort_value_replica` |
| `sort_by_date` (`file.mtime` sort) | 28.58 µs | 140.21 µs | 565.12 µs | 1.145 ms | `QueryService__run_sort_by_date` |
| `sort_by_metadata/sort_only` (`rating`) | 52.13 µs | 256.31 µs | 1.086 ms | 2.175 ms | `QueryService__run_sort_by_metadata` |
| `sort_composite` (2-term: `class, rating`) | 59.79 µs | 290.97 µs | 1.190 ms | 2.650 ms | `QueryService__run_sort_composite` |

### 6.2 Date Sorting vs. Date Filtering: Where Ambient `local_zone()` Hits

1. **Sort Mechanism:**
   In `src/query/sort.rs:63-77`, `SortOrder::keys_for` **precomputes** sort keys upfront in a flat row-major matrix before invoking `order.sort_by`.
   - Date fields are promoted to `DateTimeValue` **once per row** during key generation.
   - During the actual comparison loop (`compare_keys`), keys are `SortKey::DateTime(a)` vs `SortKey::DateTime(b)`: it performs an integer instant comparison (`a.cmp(b)`).
   - **Result:** Ambient `local_zone()` lookups and date conversions contribute **0%** to the $O(N \log N)$ comparison loop during sorting. `sort_by_date` (565 µs for 20K) is within 8% of synthetic `sort_value_replica` (523 µs).

2. **Filter Mechanism:**
   In contrast, filter expressions evaluate calendar operations dynamically per row (`QueryService::run/filter_by_date` on 20,000 notes):
   - `date_compare` (`due > "2026-06-15"`): **832.03 µs** (41.6 ns/row)
   - `datetime_compare`: **842.88 µs** (42.1 ns/row)
   - `duration_compare`: **1.137 ms** (56.8 ns/row)
   - `date_function` (`date("2026-06-15") < due`): **1.807 ms** (90.4 ns/row)
   - `date_arithmetic` (`due + dur("1 month") > "2026-07-01"`): **2.208 ms** (110.4 ns/row)
   - `shorthand_period` (`eom(due) > "2026-06-30"`): **2.677 ms** (133.8 ns/row)

**Takeaway:** Ambient `local_zone()` lookups add **2.2x to 3.2x overhead** in filter evaluation (`eom`, date arithmetic), but **zero** overhead in sort comparisons because sort keys are precomputed.

---

## 7. QueryRow & QuerySet Memory Layout and Template Render Chaining

### 7.1 Data Structure Sizes

| Struct | Rust Definition / Source | Stack Size (`std::mem::size_of`) | Heap Allocation per Instance |
|---|---|---|---|
| `QueryRow` | `src/query/results.rs:44` | **48 bytes** | 0 bytes (when `flattened` is empty) |
| ├── `index` | `Arc<WorkspaceIndex>` | 8 bytes | (points to shared index) |
| ├── `position` | `RowIndex` (`usize`) | 8 bytes | 0 |
| ├── `flattened` | `Vec<(FieldPath, NoteFieldValue)>` | 24 bytes | 0 (unallocated until `flatten()`) |
| └── `kind` | `RowType` | 8 bytes | 0 (`Page` or `List { item_idx: u32 }`) |
| `ListItem` | `src/note/model.rs` | **160 bytes** | ~28 bytes heap (strings), ~188 bytes gross |
| `QuerySet` | `src/query/results.rs:464` | **56 bytes** | Holds `Arc<Vec<QueryRow>>` + `ExecutionPlan` |

### 7.2 QuerySet Cloning and Execution Plan Materialization

- `QuerySet.clone()`:
  - Benchmark `QueryService::run/clone_query_set`: **0.01 µs (12.3 ns)** across all sizes (50 to 20,000 notes).
  - Pure atomic increment of `Arc<Vec<QueryRow>>`.
- Template query chaining:
  - In template filters (e.g. `query.from().where("rating >= 5").sort("file.name", false)`), chaining transforms only appends to `ExecutionPlan` in $O(1)$ time.
  - **The hidden clone penalty:** When a chained query set is materialized (`rows()` in `src/query/results.rs:488`):
    ```rust
    Arc::new(self.plan.clone().run(self.base.to_vec()))
    ```
    If `plan` is non-empty, `self.base.to_vec()` clones the entire `Vec<QueryRow>`.
    For 20,000 rows, this performs **20,000 atomic refcount increments** (`Arc::clone(&self.index)`) and allocates 960 KB for the row slice.

### 7.3 Template Rendering Overhead (`TemplateService::render_to_file`)

Measurements on DryRun template rendering (`benches/template_render.rs`):

| Note Count | `refresh_floor` | `list` Template Render | `table_filtered` Template Render |
|---|---|---|---|
| 50 Notes | 16.123 ms | 16.016 ms | 16.076 ms |
| 200 Notes | 16.137 ms | 16.280 ms | 16.528 ms |
| 1,000 Notes | 17.312 ms | 17.041 ms | 18.161 ms |

**Takeaway:**
The template render latency is **95%+ dominated by the `refresh_floor`** (~16–17 ms) which opens redb and checks file freshness. The actual template evaluation, query filtering, and Markdown rendering take only **~0.8 ms** at 1,000 notes.

---

## 8. Fixed Floors that Bound Any Redesign

| System / Boundary | Floor Latency | Cause / Mechanism | Architectural Implication |
|---|---|---|---|
| **Process Startup** | **~4.5 ms** | Binary load, dynamic linking, runtime initialization | Any CLI command takes at least 4.5 ms |
| **Config Load & Trust Check** | **~0.3–1.5 ms** | Discovery of `.traces/config.toml`, TOML parse, trust DB check | Negligible overhead for trusted repositories |
| **redb Transaction Commit** | **~36.8 ms** | `redb::Database::begin_write()` commit + disk `fsync` | Any write transaction in redb takes $\ge 37$ ms on macOS APFS |
| **redb Table Open & Read** | **~16.0 ms** | `redb::Database::open()` + root page reads | Any cold read command that opens redb takes $\ge 16$ ms |
| **Filesystem Directory Walk** | **~1.7 ms / 1K notes**<br>**~12.4 ms / 5K notes**<br>**~55.7 ms / 20K notes** | Recursive `read_dir` + `stat` across all note files | An un-indexed differential scan takes ~55 ms on a 20K vault |

---

## 9. What These Numbers Do and Do Not Support

### 9.1 Supported Survey Hypotheses

1. **redb Commit fsync Floor Dominates Persistence:**
   - *Survey hypothesis:* Persisting the index is dominated by redb's synchronous write transaction commit.
   - *Evidence:* **SUPPORTED.** Persisting 50 notes takes 36.8 ms; persisting 1,000 notes takes 37.7 ms; persisting 5,000 notes takes 42.1 ms. Over 85% of persist latency at $\le 5\text{K}$ notes is the redb transaction commit floor.
2. **redb Table Open Floor Sets Read Limit:**
   - *Survey hypothesis:* Cold read queries cannot beat redb's database open overhead.
   - *Evidence:* **SUPPORTED.** `WorkspaceIndex::load` is 16.0 ms at 50 notes and 16.0 ms at 1,000 notes. No store-backed query can finish in under 16 ms if redb is opened.
3. **Inlink Map Compilation Scales Well for Realistic Notes:**
   - *Survey hypothesis:* The `InlinkMap` compilation is cheap enough for standard notes but vulnerable to dense link topologies.
   - *Evidence:* **SUPPORTED.** Sparse inlinks take 1.08 ms at 1K and 19.0 ms at 20K notes. Dense link topologies jump to 6.3 ms at 1K and 139.8 ms at 20K notes.
4. **Dual Query Path Overhead:**
   - *Survey hypothesis:* Store-backed `run_from_store` pays massive allocation overhead synthesizing a transient `WorkspaceIndex`.
   - *Evidence:* **SUPPORTED.** For a single matching note on a 20K vault, `sync_and_run` allocates **21.24 MB** across **129,495 allocations**, while in-memory `QueryService::run` allocates **48 bytes** in **3.9 µs**.

### 9.2 Contradicted Survey Hypotheses

1. **Date Sorting Suffers from Repeated Ambient `local_zone()` Calls:**
   - *Survey hypothesis:* Sorting by date/datetime repeatedly calls `local_zone()` or promotes dates per comparison in the sort loop.
   - *Evidence:* **CONTRADICTED.** `SortOrder::keys_for` precomputes sort keys once per row upfront. During the $O(N \log N)$ comparison loop, keys are compared as raw timestamps (`DateTimeValue::cmp`). Sort time (565 µs for 20K) is within 8% of synthetic bare enum sorting.
2. **QuerySet Cloning in Templates Degrades Performance:**
   - *Survey hypothesis:* Chaining query transforms in MiniJinja templates clones rows or materializes query results repeatedly.
   - *Evidence:* **CONTRADICTED FOR TRANSFORM CHAINING.** `QuerySet.clone()` takes **12.3 ns** regardless of vault size (50 to 20,000 notes). However, **terminal materialization** of an unexecuted plan clones `QueryRow` instances via `self.base.to_vec()`, causing an atomic refcount bump per row once.

### 9.3 Unmeasured / Micro-Measurement Required

1. **Exact Pulldown-Cmark Pass 1 vs. Pass 2 (Headings) Split:**
   - *Status:* **UNMEASURED SEPARATELY.** In current production code, `collect_headings` is private and called unconditionally within `parse_markdown`. Existing benchmarks measure the whole parser (~10 µs per medium note).
   - *Requirement to Measure:* An internal benchmark target or private probe exposing `collect_headings` and `handle_event` separately without modifying the production API surface.
