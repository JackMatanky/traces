# D2: Cross-validate ticket 27 perf rows against published competitor data

Research subagent output for ticket 27 pass 2. Goal: validate, tighten, loosen, or flag as
unsupported every latency row the 27 pass-2 clusters proposed for ticket 33, using only
externally citable evidence, then recommend a real vault-scale basis to replace ticket 33's
admitted placeholders (1k/10k/50k "invented" figures, issues/33).

Standing constraint from the user: the LSP must be the top-performer class in the PKM
Markdown space, not merely adequate. All URLs accessed 2026-10-22 unless dated otherwise.

## 1. Source table

| # | Source | URL | Date | What was measured / stated | Credibility |
|---|--------|-----|------|---------------------------|-------------|
| S1 | VS Code issue: folding range hard limit | https://github.com/microsoft/vscode/issues/72994 | 2019-04-28 | VS Code core caps folding ranges per file; hard limit 5000 (issue discussion of folding collapse-all perf) | High (first-party) |
| S2 | lsp-devtools capability matrix | https://lsp-devtools.readthedocs.io/en/latest/guides/capabilities/index.html | undated (tool current 2026) | Documents `foldingRange.rangeLimit = 5000` as the VS Code client-advertised default | High (docs of client defaults) |
| S3 | VS Code default settings | https://code.visualstudio.com/docs/reference/default-settings | undated (product current) | `editor.foldingMaximumRegions` default 5000 | High (first-party) |
| S4 | rust-analyzer config source | https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/config.rs | current main | `workspace_symbol_search_limit` default 128 for `workspace/symbol` | High (source of truth) |
| S5 | gopls workspace_symbol.go | https://go.googlesource.com/tools/+/refs/tags/v0.11.0/internal/lsp/source/workspace_symbol.go | v0.11 | gopls caps `workspace/symbol` at `maxSymbols = 100` | High (first-party source) |
| S6 | clangd manpage | https://manpages.ubuntu.com/manpages/jammy/man1/clangd.1.html | 2021 manpage (stable since) | `--limit-results` default 100 (completion/code complete), `--limit-references` default 1000 | High (first-party manpage) |
| S7 | Helix symbol picker cap | https://github.com/helix-editor/helix/issues/1437 | 2022-01-04, closed by PR #5055 | Helix workspace symbol picker historically showed only the first 128 results; server-side caps discussed (r-a limit config raised to 1048576 in workaround) | High (first-party issue) |
| S8 | Helix per-server request timeout | https://github.com/helix-editor/helix/blob/master/book/src/languages.md | current master | Language-server `timeout` default 20 seconds per request | High (first-party docs) |
| S9 | Zed all-settings page | https://zed.dev/docs/configuring-zed | current | `request_timeout` default 120 seconds (0 = none); `max_buffer_line_length` default 20000 (Zed disables all LSP features for buffers with longer lines); LSP document symbols default off | High (first-party docs) |
| S10 | Neovim LSP docs | https://github.com/neovim/neovim/blob/master/runtime/doc/lsp.txt | current | `buf_request_sync`/`buf_request_all` default timeout 1000ms | High (first-party runtime docs) |
| S11 | Neovim 0.11 release notes | https://github.com/neovim/neovim/blob/master/runtime/doc/news.txt (0.11 section) | 2025 | De-duplicates inlay-hint/diagnostic requests (cancels inflight on new edit); `vim.lsp.foldexpr` caches foldinfo per buffer; 10x speedup vs treesitter foldexpr when no parser | High (first-party) |
| S12 | nvim-treesitter folding hazard | https://github.com/nvim-treesitter/nvim-treesitter/issues/23964 | 2025 | Folding on a 10k-line append took 280ms when the parser was missing (~1ms/line fallback path) | Medium (issue report, plausible) |
| S13 | iwe benchmark (20k docs) | https://github.com/iwe-org/iwe/blob/master/docs/benchmark.md and https://iwe.md/blog/benchmarking-iwe-on-20000-documents | blog 2026-05-03 | On 20000 docs: load plain 705ms, indexed load 958ms; whole-graph queries: filter 12ms, find 15ms, count 18ms, retrieve 7ms, stats 30ms, traverse 48ms (95% CI, criterion) | High (first-party, method documented) |
| S14 | rumdl comparison page | https://rumdl.dev/comparison | Feb 2026 | Cold full-corpus lint of the Rust Book (478 files) 217ms including process startup, ~0.45ms/file amortized; warm/incremental faster | Medium (vendor benchmark, method stated) |
| S15 | VS Code markdown LS unresponsiveness | https://github.com/microsoft/vscode/issues/325595 | 2026-07-13 | Built-in Markdown language server becomes unresponsive on LLVM LangRef.md (single huge Markdown file); supports the parse-free structural-walk design argument | High (first-party issue) |
| S16 | redhat vscode-xml symbols doc | https://github.com/redhat-developer/vscode-xml/blob/main/docs/Symbols.md | current | `xml.symbols.maxItemsComputed` default 5000, with a computed-items cap for outline | High (first-party config doc) |
| S17 | lsp-bench | https://github.com/asyncswap/lsp-bench | current (README verified) | Criterion-style LSP benchmark framework recording p50/p95/mean latency plus resident memory for initialize, documentSymbol, foldingRange, selectionRange, codeLens, semanticTokens/full, documentLink, workspace/symbol, references, and more, over JSON-RPC stdio | High (open tool, method explicit) |
| S18 | rust-analyzer find-references anecdote | https://github.com/rust-lang/rust-analyzer/issues/8879 | 2021-05-19 | Find-references on an external-crate symbol took 30s to over 1 minute; user reports 100x slower than ripgrep (semantic check cost) | Medium (old anecdote, directionally reliable) |
| S19 | rust-analyzer memory thread | https://users.rust-lang.org/t/rust-analyzer-using-13-gib/133914 | 2025-09-14 | 13GB RSS reported as "typical" for a large workspace with ~830 dependencies; multi-GB is normal for r-a on big codebases | Medium (forum, single report but consistent with A-perf issues) |
| S20 | Obsidian large-vault forum threads | https://forum.obsidian.md/t/performance-on-large-vaults/114864 (2026-06-01, 993 notes called "fairly large"); https://forum.obsidian.md/t/graph-view-crashes-with-30k-notes/7039; https://forum.obsidian.md/t/50k-notes-vault-is-lagging/82241; https://forum.obsidian.md/t/how-big-can-a-vault-get/112785 (2026, expects crossing 10k) | 2024 to 2026 | Community vault sizes: ~1000 notes is already "fairly large" for typical users; 13k reported fine, 30k tested OK for graph; 50k vault reported laggy; power users anticipate 10k as a threshold | Medium (self-reports, best available vault-scale data) |
| S21 | Obsidian note-link selector latency | https://forum.obsidian.md/t/slow-performance-with-large-vaults/16633 | 2020 (graveyard) | Link autocomplete took ~4s per keystroke on large vaults (historical failure mode; product has since improved) | Low-Medium (old, archived) |
| S22 | zk large-notebook indexing | https://github.com/zk-org/zk/issues/734 | 2022 to 2024 activity | zk indexing is "quite slow" for large notebooks vs git; maintainer confirms indexing perf is a known pain (3k-note indexing attempts, regressions on small notebooks forced scrap) | Medium (first-party issue) |
| S23 | iwe comparison page (peers) | https://iwe.md/comparison | verified 2026-09-03 (per local research b5) | Marksman and markdown-oxide have no published benchmark suite; iwe is the only PKM Markdown engine with a public 20k-doc corpus benchmark | High (first-party statement about absence) |

Negative findings (searched, not found): no published per-request latency numbers for
`textDocument/documentSymbol`, `foldingRange`, `selectionRange`, `documentLink`, `codeLens`,
`semanticTokens`, or `workspace/symbol` from marksman, zk, vscode-markdown-languageservice,
markdown-oxide, or iwe. No iwe memory/RSS benchmark table (https://iwe.md/memory and
/memory-benchmark both 404; benchmark.md has no RSS numbers). No documented per-request LSP
timeout in VS Code (vscode-languageclient applies none by default; only initialize/connect
timeouts exist, see https://github.com/microsoft/vscode-docs/blob/main/api/language-extensions/language-server-extension-guide.md).
markdown-oxide's MAX_INDEXED_LINES (cited in issues/33) could not be verified on the web in
this pass; it remains a local-source claim.

## 2. Row-by-row verdicts

Verdict vocabulary: TOP-CLASS (evidence supports keeping, and it leads the field), TIGHTEN
(evidence supports a stricter number), LOOSEN (evidence suggests the number is unrealistic or
diverges from the correct source-of-truth row), UNSUPPORTED (no external datapoint exists;
the row is a self-imposed target that must be bench-validated in code).

| Row (source of the row) | Our target | Best-in-class evidence | Verdict | Revision |
|---|---|---|---|---|
| `documentSymbol` headings-only warm (b1 D10, b1 :225) | < 5ms p95 | No peer publishes ms figures. iwe whole-vault ops at 20k docs are 7ms (retrieve) to 48ms (traverse) [S13]; our row is a single-file resident metadata walk, strictly cheaper. VS Code's built-in Markdown LS hangs on one huge file [S15], i.e. the field's default is not a perf benchmark to beat on this row | TOP-CLASS (keep, self-imposed) | Keep < 5ms. No competitor number exists to tighten against; the iwe class check (whole-graph ops under 50ms at 20k) makes a 5ms single-doc walk conservative, not aggressive |
| `documentSymbol` with tasks warm (b1 :226, ticket 33 existing <10ms row) | < 10ms p95 | Same as above; heading walk plus cached-AST task walk | TOP-CLASS (keep) | Keep < 10ms; it is already ticket 33's row, and no external evidence forces movement |
| `documentSymbol` cold (b1 :227) | measure, no row yet | rumdl shows ~0.45ms/file all-in amortized including startup for a 478-file corpus [S14]; iwe indexed load 958ms for 20k docs [S13] implies ~0.05ms/doc steady-state amortized | Keep "measure first" | Retain as a measurement task, not a target. Both anchors say cold cost is dominated by parse, which is what the row must isolate |
| `workspace/symbol` cap 128 @ 20k notes (b1 :228) | < 50ms p95, cap 128 | Server caps: r-a 128 [S4], gopls 100 [S5], clangd 100 [S6]. Client cap: Helix picker 128 [S7]. No ms latency published anywhere; iwe filter over 20k docs is 12ms [S13] | TOP-CLASS on the cap (keep 128); UNSUPPORTED on the ms (keep, bench-gated) | Keep cap 128 (dead center of the 100 to 128 industry band, and Helix proves pickers rely on it). Keep < 50ms as a self-imposed stretch: it is roughly 4x an iwe whole-graph filter on 40x less work per hit, plausible but unproven; must be validated by a bench sibling of `bench_query_tasks_density` before 33 accepts it |
| `workspace/symbol` empty query early-exit at 128 (b1 :230) | same row | Helix sends one empty-query request and filters client-side [S7 discussion]; r-a/gopls caps make empty-query cost bounded [S4][S5] | TOP-CLASS (keep) | Keep. Deterministic early-exit is strictly better than the peers' return-then-filter shape |
| `foldingRange` (b2 :138) | < 30ms target, < 10ms stretch | VS Code hard-caps folding at 5000 ranges [S1][S3] and advertises `rangeLimit = 5000` [S2]. Pathological fallback (no parser) cost ~1ms/line [S12] means a 30ms budget fails only for ~30k-line files, which our AST walk will not hit (single parse reused). No peer ms numbers published | TOP-CLASS (keep) | Keep 30/10. Also keep honoring `rangeLimit` with priority truncation; the 5000 default is the ecosystem contract [S1][S2] |
| `selectionRange` (b2 :139) | < 10ms target, < 5ms stretch | Position-local walk with early exit; MS markdown LS smart-select is tiered innermost-first (local b2 evidence, ms-digest). No ms numbers published anywhere | UNSUPPORTED (keep, by-construction) | Keep 10/5. Cost is O(depth) from a resident tree; the only external check is that MS ships the same feature without range caps or timeouts |
| `documentLink` warm (b3 :326) | <= 5ms warm, <= 10ms cold | Neovim `gx` issues documentLink synchronously per keypress (local b3 evidence); Neovim's own sync-request timeout is 1000ms [S10], so a slow server is survivable but sticky. No peer ms numbers; MS/zk have no link cap (local b3 evidence) | UNSUPPORTED (keep, UX-mandated) | Keep 5/10. The argument is UX (synchronous keypress path), not competitor benchmarking; it cannot be tightened externally |
| `documentLink/resolve` (b3 :327) | n/a v1, future <= 2ms | VS Code resolves links lazily to `command:` or `file://...#L...` URIs (local b3 evidence) | Keep deferred | No change |
| `textDocument/codeLens` discovery (b4 :294) | < 10ms target, < 3ms stretch | No PKM Markdown peer ships codeLens at all (b4 finding); VS Code's own provide/resolve debounce precedent is 250ms (local b4, research/26); rust-analyzer's viewport-gated resolve model is the de-facto client pattern | UNSUPPORTED (keep, no benchmark class exists) | Keep 10/3. We are defining the class, so the rows are self-imposed; instrument them or the numbers are decoration |
| `codeLens/resolve` per heading (b4 :295) | < 20ms target, < 5ms stretch | Same: RA-style viewport-gated resolve; no peer baseline | UNSUPPORTED (keep) | Keep 20/5 |
| Eager-all count materialization (b4 :296) | amortized into index cold (< 3s / 1000 files), incremental edit <= 10ms | iwe indexed load 958ms for 20k docs [S13]; zk's known indexing pain on multi-thousand-note notebooks [S22] says index-time budgets must be explicit | TOP-CLASS (keep) | Keep. The iwe number (20k in under 1s) is actually stricter than ours (1k in 3s); do not advertise our cold budget as leading until a bench compares like for like |
| `workspace/codeLens/refresh` emission (b4 :298) | <= 1 per 250ms per graph change, 0 during typing | Neovim 0.11 cancels and de-duplicates inflight requests on edit [S11]; iwe uses a 100ms internal delay (local b4 evidence); spec requires "absolute care" | TOP-CLASS (keep) | Keep. Our 0-during-typing rule is stricter than every cited client behavior |
| `textDocument/documentHighlight` (b4 :297) | b4 says < 10ms target / < 3ms stretch; consolidated 27 Part XI item 9 drifted to "<= 5ms" | Document-local span match; no peer numbers | LOOSEN (internal consistency fix) | Adopt b4's original < 10ms target / < 3ms stretch as the single source of truth; the "<= 5ms" variant in the consolidated file was a drift and sits between b4's target and stretch, satisfying no budget. Change the consolidated doc to match b4 |
| `inlayHint` visible range (b4 :299, Option E only) | < 30ms target, < 10ms stretch | Zed's client-side debounce dominates UX at 700ms edit / 50ms scroll (local b4, Zed settings); Helix exposes `inlay-hints-length-limit` but no timing [docs.helix-editor.com/master/editor.html] | TOP-CLASS (keep, conditional on Option E) | Keep; row stands only if 27's inlay option is chosen |
| `semanticTokens/full` warm (b6 :145) | <= 10ms target, <= 5ms stretch | Neovim ships semantic tokens on by default with a 200ms client debounce (local b6, protocol.lua); our emission is index-walk-only on an already-parsed snapshot, so 100ms debounce [ticket 25] plus <= 5ms emission is tighter than the nvim pipeline's client-side budget | TOP-CLASS (keep) | Keep 10/5. Our refresh floor of >= 100ms coalesced [b6 :147] is 2x tighter than Neovim's 200ms debounce and errors are swallowed, matching nvim/Zed cancel-and-restart behavior [S11] |
| Emission after `didChange` (b6 :146) | rides 25's 100ms debounce, <= 5ms at request time | Neovim cancels inflight semantic-token work on edit [S11]; VS Code has no request timeout to protect a slow server (negative finding, section 1) | TOP-CLASS (keep) | Keep. Zero server work until post-settle is the correct shape in every editor sampled |
| Deferred `full/delta` (b6 :148) | <= 2ms if full regresses | rust-analyzer `diff_tokens` pattern (local b6) | Keep deferred | No change |
| `SYMBOL_CEILING` 1k soft / 5k hard per doc (ticket 33, b3 :331) | 5000 hard cap on symbols per document | redhat vscode-xml `xml.symbols.maxItemsComputed` default 5000 [S16]; VS Code folding 5000 [S1][S3]; MS folding rangeLimit 5000 [S2] | TOP-CLASS (keep 5k hard) | Keep. 5k is the ecosystem's hard-outline cap (two independent first-party precedents). Keep 1k soft as our truncation-with-log threshold; no peer documents a soft tier, so that half is ours |
| Workspace symbol cap 128 (ticket 33) | 128 | r-a 128 [S4], gopls 100 [S5], clangd 100 [S6], Helix picker 128 [S7] | TOP-CLASS (keep) | Keep 128 |
| `REFERENCE_CEILING` 5000 (ticket 33, b3 :331) | 5000 references per response | clangd `--limit-references` default is 1000 [S6]; r-a find-references on huge symbol sets historically 30 to 60s [S18]; local research/26 recorded a 2.93s bufload stall at ~100 refs in Neovim | LOOSEN default, keep hard ceiling | Two-tier, clangd-shaped: default soft cap 1000 per response with truncation notice, hard ceiling 5000 configurable. Our flat 5000 default is 5x the only published peer default and sits in the stall class the local bufload evidence recorded |
| Vault scale basis (ticket 33 placeholders) | 1k / 10k / 50k notes, admitted invented | Community evidence [S20]: ~1000 notes is already "fairly large" (Obsidian forum, 2026); 13k fine, 30k tested OK, 50k laggy; zk indexing pain confirmed at multi-thousand-note scale [S22]; iwe's public suite is 20k docs [S13] | TIGHTEN (basis must be evidence-based) | See section 3 for the recommended basis |

## 3. Vault-scale basis recommendation for ticket 33

Replace the admitted placeholders with this evidence-anchored ladder:

| Tier | Note count | Role | Evidence |
|---|---|---|---|
| Small | 100 | smoke bench, CI | smallest tier that exercises workspace indexing at all |
| Typical large | 1000 | primary interactive target | Obsidian users call ~993 notes "fairly large" [S20 #114864, 2026] |
| Power user | 10000 | sustained-target tier | 13k reported fine, 30k OK [S20]; zk's indexing pain appears in this band [S22] |
| Stress | 50000 | degradation-reporting tier, no hard guarantee | 50k vault reported laggy in Obsidian [S20 #82241] |
| Anchor for cross-tool comparison | 20000 documents | single published peer corpus | iwe's public benchmark suite [S13] |

Rationale: (a) our headline latency rows (< 50ms workspace/symbol) should be stated at the
10k tier, because that is where real power-user vaults sit and where every cited engine
starts to hurt; (b) the 20k tier exists purely so our benches are comparable to the only
public competitor numbers in the category [S13]; (c) 50k is a degradation-reporting tier
because even Obsidian (not an LSP) goes laggy there [S20], so promising interactive latency
at 50k would be a claim no engine in this category makes; (d) note-size matters as much as
note-count: iwe's 20k corpus is one-doc-per-stub style, while a real vault averages
longer notes, so benches should report bytes-indexed alongside note-count.

Also carry forward from local research (ticket 13, redb warm-cache context): cold index
cost is a separate row family from warm-query latency, and zk [S22] plus iwe [S13] both
show the industry pain is concentrated in index/cold paths, not warm queries. Our warm rows
are therefore where top-class is claimable; cold rows should be reported, not promised.

## 4. Ceiling verdicts

1. `SYMBOL_CEILING` (1k soft / 5k hard per doc): TOP-CLASS, keep as-is. Two independent
   first-party 5000 defaults (vscode-xml [S16], VS Code folding [S1][S3][S2]) make 5k the
   ecosystem hard cap for structural responses.
2. Workspace symbol cap 128: TOP-CLASS, keep. Industry band is 100 to 128 [S4][S5][S6][S7];
   128 is the top of the band and Helix's picker design depends on it.
3. `REFERENCE_CEILING`: LOOSEN the default to 1000 (clangd's published default [S6]) with
   hard ceiling 5000 remaining configurable. Evidence: a 30 to 60s find-references
   experience on broad symbols [S18] and a measured 2.93s Neovim stall at ~100 refs
   (local research/26). Truncation must be visible to the user, matching clangd behavior.
4. No `LINK_CEILING` in v1 (b3 proposal): keep. No peer caps links [S23 and local b3];
   per-doc link counts are tens, and truncating something no competitor truncates would be
   invented strictness.
5. Folding `rangeLimit` 5000 honored by priority: TOP-CLASS, keep. It is literally the
   client-advertised default [S2]; silent truncation beyond it is MS behavior and is fine.

## 5. Gaps and the instrument-in-code recommendation

Gaps found (all of these are why several rows above are UNSUPPORTED):

1. No published per-request LSP latency numbers from any Markdown PKM peer for any of the
   eight feature families in ticket 27. iwe is the only publisher and it measures
   whole-graph CLI queries, not LSP methods [S13][S23]. Consequence: every millisecond
   target in 27 is self-imposed until measured in our own benches.
2. No RSS or memory benchmarks published by iwe (memory-benchmark page 404; benchmark.md
   has no RSS table). rust-analyzer's multi-GB RSS on large workspaces [S19] is the only
   first-party scale-warning datapoint; our own claim of ~2MB resident metadata at 20k
   notes (local b6) is therefore also self-imposed and must be bench-reported.
3. VS Code has no documented per-request LSP timeout (negative finding, section 1), unlike
   Neovim 1000ms [S10], Helix 20s [S8], and Zed 120s [S9]. Client-tolerance arguments for
   VS Code cannot cite a number; our latency rows should instead be justified against
   keystroke-path UX (gx, outline-on-open), which is what b3 already does.
4. markdown-oxide MAX_INDEXED_LINES (cited in issues/33) remains unverifiable on the web
   this pass; either verify against the crate source or drop the citation from 33.
5. Zed's `max_buffer_line_length = 20000` (all LSP features disabled beyond it) [S9] and
   VS Code's hang on huge single files [S15] are the two documented "giant file" failure
   modes in the ecosystem. Our parse-free structural walks avoid both, but we have no row
   stating a max-file-size degradation policy; recommend 33 adds one (degrade gracefully
   above N MB, mirroring Zed's explicit-disable shape rather than VS Code's hang).

Instrument-in-code recommendation (this is the only path to converting UNSUPPORTED rows):

- Adopt the feature coverage of lsp-bench [S17] as the checklist: it already measures
  documentSymbol, foldingRange, selectionRange, codeLens, semanticTokens/full,
  documentLink, workspace/symbol, and references, each with p50/p95/mean plus RSS, which
  is exactly the row format ticket 33 uses. Our benches should report the same statistics
  so any future peer run of lsp-bench against our server is apples-to-apples.
- Minimum bench set before 33 accepts the rows: `bench_document_symbol_warm` (headings and
  tasks sub-rows), `bench_workspace_symbol_20k` (cap-128 early-exit, empty query),
  `bench_folding_range` at 500 and 5000 ranges (rangeLimit truncation path),
  `bench_selection_range` (worst-case depth), `bench_document_link_warm`,
  `bench_code_lens_discovery_and_resolve`, `bench_semantic_tokens_full`, plus a
  `bench_reference_search` at the new 1000 default cap. Each should print RSS, because the
  memory claim (2MB at 20k notes) is currently as unsupported as the latency claims.
- Report at the 1k/10k/50k tiers from section 3, with the 20k tier additionally reported for
  iwe comparability [S13].

Net effect on ticket 33's row table: one row moves (documentHighlight, LOOSEN to b4's
< 10/< 3), one ceiling changes default (`REFERENCE_CEILING` soft default 5000 to 1000,
hard 5000 retained), the vault-scale basis is replaced with an evidence ladder (1k typical
large, 10k power user, 50k stress, 20k cross-tool anchor), and every other row stands as
TOP-CLASS or bench-gated UNSUPPORTED with no competitor number in the category capable of
tightening or loosening it.
