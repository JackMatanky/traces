# How do incremental systems fingerprint inputs and decide staleness?

Type: research
Status: resolved
Blocked by: none

## Question

Traces' persistent index is a redb cache with a hardcoded format byte, content/stat-based file deltas, and an unimplemented config-drift fingerprint (ADR-0009 `docs/adr/0009-index-refresh.md`, `.scratch/index-query/issues/17-config-drift-detection-and-rebuild.md`); path-set changes trigger whole-vault re-resolution while content edits patch locally (`src/index/refresh.rs:344-386`). See `survey/index.md` H2/H3, `survey/config-dialog.md` H1.

Study: Cargo fingerprints (`src/cargo/core/compiler/fingerprint/`, `DepFingerprint`, `StaleItem`, env/config/flag inputs); rustc incremental (`rustc_query_system/src/dep_graph`, red-green marking, `Fingerprint`, `DepNode`); Git index (`gix-index` stat cache, racy-git, `gix-dir`); Mercurial dirstate (v2) ; rumdl's cache (key derivation including config hash); Ruff's cache (`ruff_cache`, `CacheKey` trait); Obsidian Dataview and advanced-metadata-cache (how link resolution is recomputed on rename/delete); `watchman`/`notify` only if they inform path-set invalidation.

Focus: how the full set of inputs affecting a derived artifact is enumerated and kept honest (derive/trait like Ruff `CacheKey` vs hand-listed digests); granularity of invalidation (whole cache vs per-axis vs per-entry); how link/reference graphs are invalidated by structural changes (rename/add/delete) without whole-graph recompute; atomicity of fingerprint + data; failure/corruption policy.

Informs: index freshness and invalidation, index read model, config ownership and lifecycle.

Deliverable: `.scratch/strategic-redesign/research/05-freshness-fingerprint-precedent.md` (<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp `get_item_source`, or GitHub raw files) and cite repo path + symbol/line for every claim. Structure: (1) what each comparable actually does (verified from source), (2) the invariants and who owns them, (3) tradeoffs and costs they pay, (4) transferable lessons for Traces, (5) lessons that do NOT transfer and why, (6) what this changes or challenges in the Traces hypotheses named above. Mark facts [V], interpretation [H]. Do not propose the final decision; that belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Research complete. Deliverable recorded at [.scratch/strategic-redesign/research/05-freshness-fingerprint-precedent.md](../research/05-freshness-fingerprint-precedent.md).
