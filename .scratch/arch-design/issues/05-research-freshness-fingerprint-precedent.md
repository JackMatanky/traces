# How do incremental systems fingerprint inputs and decide staleness?

Type: research
Status: resolved
Blocked by: none

## Question

Traces' persistent index is a redb cache with a hardcoded format byte. File
deltas use content and stat data. The config-drift fingerprint was unimplemented
when charted. It has since landed (`207cc3fb`) (ADR-0009
`docs/adr/0009-index-refresh.md`,
`.scratch/index-query/issues/17-config-drift-detection-and-rebuild.md`).
Path-set changes trigger whole-vault re-resolution. Content edits patch locally
(`src/index/refresh.rs:344-386`). See `survey/index.md` H2/H3,
`survey/config-dialog.md` H1.

Study these comparables:

- Cargo fingerprints (`src/cargo/core/compiler/fingerprint/`, `DepFingerprint`,
  `StaleItem`, env/config/flag inputs)
- rustc incremental (`rustc_query_system/src/dep_graph`, red-green marking,
  `Fingerprint`, `DepNode`)
- Git index (`gix-index` stat cache, racy-git, `gix-dir`)
- Mercurial dirstate (v2)
- rumdl's cache (key derivation including config hash)
- Ruff's cache (`ruff_cache`, `CacheKey` trait)
- Obsidian Dataview and advanced-metadata-cache (how link resolution is
  recomputed on rename/delete)
- `watchman`/`notify`, only if they inform path-set invalidation

Focus:

- How is the full input set enumerated and kept honest: derive/trait like Ruff
  `CacheKey` vs hand-listed digests?
- What is the invalidation granularity: whole cache, per axis, or per entry?
- How are link/reference graphs invalidated by rename, add, or delete without
  whole-graph recompute?
- How is the fingerprint made atomic with its data?
- What is the failure and corruption policy?

Informs: index freshness and invalidation, index read model, config ownership
and lifecycle.

Deliverable:
`.scratch/arch-design/research/05-freshness-fingerprint-precedent.md` (<= 300
lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp
`get_item_source`, or GitHub raw files) and cite repo path + symbol/line for
every claim. Use this structure:

1. What each comparable actually does (checked from source).
2. The invariants and who owns them.
3. The tradeoffs and costs they pay.
4. Transferable lessons for Traces.
5. Lessons that do NOT transfer, and why.
6. What this changes or challenges in the Traces hypotheses named above.

Mark facts [V], interpretation [H]. Do not propose the final decision; that
belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Research complete. Deliverable recorded at
[.scratch/arch-design/research/05-freshness-fingerprint-precedent.md](../research/05-freshness-fingerprint-precedent.md).
