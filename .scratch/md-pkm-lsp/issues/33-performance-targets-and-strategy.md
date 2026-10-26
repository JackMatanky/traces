# Performance targets & strategy

Type: grilling
Blocked by: 10, 12, 13
Status: resolved

## Question

Grounding (reconciled 2026-09-23): the existing performance signals are the decomposed `benches/index_*.rs` suite. `index_lifecycle.rs` was split into `index_refresh.rs`, `index_build.rs`, `index_store.rs`, `index_inlinks.rs`, etc. `benches/index_codec.rs` benchmarks `postcard` (de)serialization cost per read/write. Metadata scan and note parsing are already `rayon`-parallel (`src/index/service.rs`). Query source-resolution is still a linear scan of the `WorkspaceIndex` (`src/query/service.rs:132-139`). It is not an inverted index. The merge-join delta is single-threaded. No numbers exist today for interactive per-keystroke LSP latency, large-workspace (10k+ note) scale, or memory footprint.

Once the analysis-host (10), concurrency (12), and persistence/caching (13) decisions are made, set concrete, falsifiable targets and the strategy to hit them:

- Set initial full-workspace indexing time targets at stated workspace sizes. **The "1k / 10k / 50k notes" figures in this ticket's own earlier draft were illustrative placeholders I invented while charting the map, not evidence of actual or expected Traces vault sizes.** Before setting a falsifiable target, find or establish a real basis for target scale: ask the user directly, check for any existing user/vault-size data, or look at what comparable PKM tools cite as "large vault" in their own docs/issue trackers. Markdown Oxide's `MAX_INDEXED_LINES = 10_000`-per-file cap and any vault-size discussion in the zk/Marksman research are a starting point, not a substitute. Then decide whether the existing linear-scan query approach (scan/parse are already `rayon`-parallel; `rayon` is already a dependency, reconciled 2026-09-23) needs an inverted index (tag→files, folder→files) to hit the target, or whether the current approach is provably sufficient at target scale.
- Set the incremental single-edit (`didChange`) latency budget (the interactive-feel threshold, typically sub-100ms for completion) and decide what specifically must stay off that hot path. For example: full-tree redb diffing must not run per keystroke, which ties directly to ticket 13's per-keystroke-vs-per-save invalidation granularity decision.
- Set interactive request latency budgets per capability. Hover, completion, and definition should be near-instant; workspace-wide rename/diagnostics may tolerate higher latency, potentially with `$/progress` reporting per ticket 12.
- Set the memory budget for large PKM workspaces. Decide whether the full parsed `Note` AST remains viable at 50k+ notes. `WorkspaceIndex` keeps this AST resident for every file today. Decide whether LSP mode needs a different residency policy. For example, use a lazily-loaded/evictable parsed-note cache instead of the CLI's always-full-index model.
- Decide which existing benchmarks get extended for LSP scenarios. For example, add `benches/lsp_incremental.rs`. Decide which scenarios need a new benchmark harness. For example, use simulated keystroke-latency benchmarks.

## Answer

### Scale basis

Target vault sizes come from `benches/common/mod.rs:63`: `WORKSPACE_FILE_COUNTS` = [50, 200, 1_000, 5_000, 20_000] (reconciled 2026-09-23; the earlier-drafted longer list was stale), with extrapolation to 50K/100K via `bench-model` script. The 20K anchor is the primary design target; 50K/100K are extrapolation checkpoints, not hard commitments. Markdown files average 5-20KB; typical PKM vaults range 100-5,000 notes; power users reach 10K-20K. No studied PKM LSP published vault-size benchmarks at planning time (Traces will set the standard). However, iwe (2026-10-21 survey) now publishes criterion graph-load numbers (20k docs ~705ms plain / ~958ms indexed, M3 Pro) but explicitly not LSP latency ("We don't benchmark iwes"), harvestable as an external comparator against the cold <2s target ([tool-iwe](../research/tool-iwe.md) Part 4). (amended 2026-10-21 by iwe/fileclass research) One-line contrast: iwe's full-rebuild-inline (stall-not-staleness) index model differs from Traces' stale-result-discard design ([tool-iwe](../research/tool-iwe.md) Part 4).

### 1. Startup targets (cold & warm)

| Scenario | Target | Strategy |
|----------|--------|----------|
| Cold start (no redb cache) | **<2s for 20K files** | `rayon` parallel scan + parse; parallelism breakeven at ~1K items for parsing workloads |
| Warm start (redb exists) | **<500ms for 20K files** | Load redb index -> filesystem diff -> reparse only changed files (existing `refresh()` path) |
| Warm start (1K files) | **<100ms** | Same path, smaller scan |

**Why parallelize**: Markdown Oxide already uses `rayon::par_iter` for startup. rust-analyzer's parallel VFS prototype cut scan from 46s to 6-20s. pulldown-cmark parse is CPU-bound and independent per file, which is ideal for work-stealing. The existing `IndexerService::scan` metadata walk and `parse_notes` are already `rayon`-parallel (`src/index/service.rs`; reconciled 2026-09-23). Remaining cold-start work is verifying against the <2s@20K target (with `with_min_len` tuned to 2-4× CPU core count if the parallelism breakeven under-delivers), not introducing parallelism from scratch.

### 2. Incremental single-edit (didChange) latency

| Operation | Target | Strategy |
|-----------|--------|----------|
| Single-file reparse + index update | **<10ms** | pulldown-cmark: 0.20ms/48KB; extraction overhead on top; full re-parse per edit (universal pattern: no incremental parsing within markdown documents) |
| Structural diff (links/headings/tags) | **<1ms** | Compare extracted structural data against previous WorkspaceIndex entry; skip cascade if unchanged |
| Backlink recomputation | **<5ms** | Update inlink multimap for changed file + its neighbors; O(link-count), not O(vault-size) |
| Debounce window | **100ms** (provisional: ticket 25 owns final tiers) | Matches rumdl; avoids re-indexing on every keystroke |
| Full-tree redb diff | **Never on hot path** | Ticket 13: expand `RefreshPlan` with single-file constructor; `didChange` bypasses `RefreshPlan::collect` entirely |

### 3. Interactive request latency (p95)

| Capability | Target | Stretch | Strategy |
|------------|--------|---------|----------|
| Hover | **<10ms** | <5ms | In-memory hash-map lookup; metadata always resident |
| Completion | **<20ms** | <10ms | Wiki-link + heading completion from inverted index; no linear scan |
| Go-to-definition | **<5ms** | <2ms | O(1) inverted index lookup (Marksman achieves 1.5-3µs) |
| Find-references | **<20ms** | <5ms | Inverted index: file→outgoing-links multimap; O(refs), not O(vault) |
| Document symbols | **<10ms** | <5ms | Headings from in-memory index |
| Diagnostics (per-file) | **<20ms** | <10ms | Re-parse + lint one file; debounced, not per-keystroke |
| Rename | **<100ms** | <30ms | Find refs + apply edits; may use `$/progress` for large workspaces |

**Inverted index design** (ticket 13's "in-memory mutable index" made concrete):

```
wiki_link_index: HashMap<String, Vec<PathBuf>>   // file stem/title → paths
tag_index: HashMap<String, Vec<PathBuf>>          // tag → files
file_class_index: HashMap<String, Vec<PathBuf>>   // file class → files
outgoing_link_index: HashMap<PathBuf, Vec<Link>>  // file → its outgoing links
```

Metadata (paths, tags, links, headings, file-classes, frontmatter field names) is always resident: ~100 bytes/file × 20K = ~2MB. AST bodies go in an LRU cache. Schema definitions (field names, types, constraints per FileClass) are small and static, so they are always resident and never evicted.

### 4. Memory budget

| Scenario | Budget | Strategy |
|----------|--------|----------|
| Idle (vault indexed) | **<100MB** | Metadata always resident; AST bodies in LRU cache |
| Active editing (10 open buffers) | **<200MB** | Buffer content + index + diagnostics + schema definitions |
| Hard cap | **<500MB** | Leave room for VS Code + other extensions in 3GB host |
| Per-file cap | **10,000 lines** | Match Markdown Oxide's cap; reject/lazy-load beyond |

**Memory model**: an evictable LRU cache with three tiers:

| Tier | Contents | Residency | Size estimate |
|------|----------|-----------|---------------|
| **Always resident** | File paths, tags, links, headings, file-classes, frontmatter field names, schema definitions | Never evicted | ~100 bytes/file × 20K = ~2MB + schema KBs |
| **LRU cache** | Parsed Note ASTs (full body) | Evicted under memory pressure; re-parse on demand from redb/disk | Configurable max (e.g. 256 entries) |
| **Transient** | Editor buffer snapshots, diagnostic results, in-flight query results | Dropped on completion | Bounded by concurrent requests |

**Degradation modes** (perl-lsp-rs pattern):

- **Normal**: full LRU cache, background indexing active
- **Warning** (>300MB): stop background indexing, reduce LRU capacity
- **Critical** (>400MB): evict LRU entries aggressively, serve only from always-resident metadata

**Frontmatter/schema intelligence impact**: Schema definitions (`HashMap<FileClass, SchemaDefinition>`) are small (field names + types + constraints per class) and always resident. Schema-aware completion/validation is a hash-map lookup + type check: O(1), negligible memory and compute cost. This is the major differentiator: Traces can do field-name completion, type-aware value completion, and schema validation for frontmatter, because the schema index is always in memory and always cheap to query. No other Markdown/PKM LSP does this.

### 5. Benchmark plan

**Extend existing** (`benches/index_refresh.rs`, the former `benches/index_lifecycle.rs`, decomposed into `index_*.rs` files; reconciled 2026-09-23):

- Add `WorkspaceIndex::refresh/single-file-didchange` group: reparse one file + update in-memory index + recompute affected backlinks at each workspace size
- Add `WorkspaceIndex::refresh/parallel-cold-start` group: the baseline is the current parallel scan/parse (`rayon`) plus the single-threaded merge-join delta; check the <2s@20K target rather than parallelizing from scratch (reconciled 2026-09-23)
- Extend `WorkspaceIndex::build` to track peak memory via `jemalloc` stats or `std::mem::size_of_val`

**New file** (`benches/lsp_latency.rs`):

- **Interactive latency group**: simulated hover/completion/gotodef/find-refs at 1K/5K/10K/20K files; pure in-memory lookups against the inverted index
- **Memory footprint group**: measure `WorkspaceIndex` memory at each workspace size; track LRU cache hit/miss rates
- **Debounce effectiveness group**: simulate 100ms keystroke bursts; measure how debounce reduces reparse count
- **Schema intelligence group**: frontmatter completion/validation latency with 5/20/50/100 field schemas

### Summary

| Decision | Resolution | Key reasoning |
|----------|------------|---------------|
| Cold-start strategy | Parallelize with `rayon`, <2s at 20K | Markdown Oxide/rust-analyzer precedent; parsing is CPU-bound and independent |
| Query strategy | Inverted index now, <5ms gotodef | Linear scan of 20K entries is fast but not O(1); inverted index eliminates it entirely |
| Memory model | Evictable LRU with 3 tiers | Best balance of performance (metadata always resident) and system strain (ASTs evictable); schema definitions always resident for frontmatter intelligence |
| Benchmark plan | Extend index_refresh.rs + new lsp_latency.rs (reconciled 2026-09-23: `index_lifecycle.rs` decomposed) | Both refresh-path additions and interactive latency need dedicated benchmarks |

### Amended by 23 (2026-10-04)

- **Task rows -> always-resident tier** (or 33 prices vault-wide refill): full reload = 15.7-19.6ms baseline / 21.9-28.9ms current at every vault size, full rebuild = 406ms@20K. Both are unusable in-request; per-note refill alone is fine (typical 0.07-1ms). ASTs are scheduled for a 256-entry LRU (`### 5`/tier notes), so a future workspace-wide task walk would pay re-parse per evicted file unless task rows are resident. Today's `WorkspaceIndex::assemble` keeps all ASTs resident; this amends the *plan*, not current code.
- **Missing `workspace/symbol` latency row added**: proposed row, number unfilled. Derive it from `bench_query_tasks_density` at the 20K-file design anchor with a stated mean→p95 factor (measured resident query: 0.665ms@60K rows, 1.77ms@100K, 18.08ms@1M, resident-only, parse excluded).
- **Tension surfaced per `map.md:33`**: measured `parse_markdown__line_density/4000` (51.2KB fixture, 4,000-*character* lines, not 4,000 lines) = 18.6ms, above the <10ms single-file reparse target; the bench itself flags this as an unexplained anomaly (`benches/note_parsing.rs:470-476`). Root cause is owed to the implementation spec before it counts as a settled conflict. Realistic 4,000-line notes ≈2-6ms (under target).

### Amended by 25 (2026-10-15)

- **Transient diagnostics tier -> bounded ~10MB workspace cache (25 Q1-R)**: the "diagnostic results" transient-tier line gains a bounded (~10MB) workspace cache serving pull; workspace-scoped, conditional on open ticket 30 (multi-root).
- **Diagnostics latency budgets (25 perf review)**: document pull **warm p95 ≤10ms**, **cold p95 ≤100ms**, 300ms absolute ceiling; workspace first pull is served as streamed partials with a ~50ms flush throttle (ty #19657); warm-up 1.5-2s @20K stands, now bracketed by external data (rumdl 0.45ms/file incl. startup; 53-rule lint 1.46ms/file upper). Owed benches stand: diagnostics rule-eval bench, full redb scan+decode wall time, warm-up wall time @1K/5K/20K in `lsp_latency.rs`.

### Amended by 26 (2026-10-18)

- **Hover row tightened**: <2ms target / <1ms stretch (this supersedes the table's <10ms/<5ms hover row; SOTA warm markdown hover measures <1ms). The real test is edit-to-hover over the T1-coalesced snapshot.
- **Rename row stays binding, restated byte-tiered**: ≤100ms @ ≤1MB total bytes, ≤1s @ ≤10MB (file-count tiers rejected as the wrong unit); workDone `$/progress` is implemented for rename (see 12's block). prepareRename <5ms (locate-class, measured ~3ms class).
- **Find-refs row is read as "≤20ms for ≤5,000 results"**, measured over the merged occurrence-resolved union path with the per-candidate-file scan as the dominant term.
- **Benches owed to 33**: the >5,000-refs cap case, index-maintenance (edit -> index -> query, where markdown LSPs measurably fall over), the pathological 51KB single-line locate micro-bench, and edit-to-hover.
- **Unverified figure flagged**: the "Marksman 1.5-3us goto definition" citation has no primary source (traced to issue #179 with no measurements); remove or re-source it.

## Inputs from 27 (2026-10-22)

- Corrected perf rows for 27's nine features (replaces per-cluster drafts; evidence: 27 pass 2 Part XII.3 + [d2-benchmarks](../research/27-pass2/d2-benchmarks.md)): doc symbols <10 target / <5 stretch headings-only, <10 / n-a with tasks (re-measured; served per-document from ContentResolver, NOT the in-memory index as 33:53 currently says); workspace/symbol <50ms stated at the 10k tier, cap 128, derived per 33:117's method; folding 30/10 honoring client rangeLimit 5000; selection 10/5; documentLink ≤5 warm / ≤10 cold (UX-mandated, nvim gx is synchronous); CodeLens discovery 10/3, resolve 20/5; documentHighlight 10/3 (b4's original; pass-2 Part XI's "<=5ms" was a drift); semantic tokens 10/5 with refresh coalesced at ≥100ms.
- REFERENCE_CEILING (26's named constant) is proposed to change to two-tier, clangd-shaped: default soft cap 1000 per response with visible truncation, hard ceiling 5000 configurable (clangd --limit-references default 1000; r-a 30-60s anecdote; local 2.93s Neovim stall at ~100 refs). The 33:129 find-refs row basis ("≤20ms for ≤5,000 results") must be re-derived against the 1000 default (5000 remains the hard ceiling), and 26's "cap fed to 27/29" wording is superseded by this two-tier model, pending 26's owner confirmation.
- Vault-scale basis replacing the admitted placeholders: 100 smoke / 1k typical-large / 10k power-user sustained / 50k stress-degradation-only, plus the 20k iwe cross-tool anchor (filter 12ms @ 20k docs). No Markdown PKM peer publishes per-request LSP latency: adopt an lsp-bench-shaped criterion suite (p50/p95 + RSS) before 33 accepts the rows; the 2MB@20k memory claim is equally unsupported and must be bench-reported.
