# Research: precedent for syntax vs persisted summary, caches, and freshness

Type: research
Status: resolved

## Question

Gather transferable lessons for tickets [Note document model](14-note-document-model.md) (AST vs persisted summary, spans, parse pipeline) and [Index read model](15-index-read-model.md) (role of redb, storage engine vs index semantics, freshness, overlay). Measured context (research 03): redb floors (commit ≈ 37 ms, open ≈ 16 ms), directory scan + `stat` ≈ 40–45 ms at 20K notes, in-memory queries in µs, parsing 5–8% of cold build. Config-drift detection via an index-owned fingerprint is fixed in-flight work; do not research it, but note how comparables key caches on content + configuration.

**General/Rust comparables**: rustc incremental compilation and cargo fingerprints (what is cached, what key invalidates it); ruff's per-file cache (content hash + settings key); git index and the "racy timestamp" problem (is mtime+size freshness safe, and what do mature tools add); Mercurial dirstate; Bazel/Buck2/watchman file-change detection; ripgrep/`ignore` parallel directory walking (the dominant 40–45 ms cost); lossless syntax trees vs semantic summaries (`rowan`, `toml::Spanned`/`toml_edit`, pulldown-cmark/comrak offset iteration); embedded KV caches (redb, sled, LMDB) used as derived caches and their open/commit cost behavior.

**Local digests** (`docs/digests/`; very large — **search/grep first, read only matched ranges**; `*-src-digest.txt` = source, `*-docs-digest.txt` = docs, unsuffixed = whole repo):

- `rumdl-src` digest: its on-disk cache design (keys, invalidation, content/config hashing, parallel file processing) and how parsed structures are reused across rules.
- `obsidian_blacksmithgu-obsidian-dataview-src-digest.txt`: its metadata cache/index of pages, tasks and links (what it stores per file, incremental update, memory model) and the documented large-vault performance failures (search `-docs-` for performance guidance).
- `obsidian_obsidian-tasks-src-digest.txt` only for how task extraction is cached/updated per file.

## Deliverable

`.scratch/arch-redesign/research/06-document-summary-cache-freshness-precedent.md`: at most ~8 lessons tagged to [Note document model](14-note-document-model.md)/[Index read model](15-index-read-model.md), graded `VERIFIED` (source, cite), `ECOSYSTEM-CONSENSUS`, or `INFERENCE`, each with a **transferability test** (single-user synchronous CLI; ≤ ~20K notes; redb warm cache; Note currently the serde-persisted record; a future live-buffer overlay) and **negative lessons**. Specifically answer from evidence: when is mtime+size freshness unsafe and what mitigation is standard; do comparables persist syntax trees or only derived summaries; do any use in-memory inverted indexes where scans cost microseconds. Do not recommend a design; do not modify production code or docs. Append an `## Answer` (5–8 lines) to THIS ticket and set `Status: resolved`. Do NOT edit `map.md`.

## Exact digest file names (`docs/digests/`)

`lsp_rvben-rumdl-src-digest.txt` (rumdl cache and parallel processing; ignore LSP code), `lsp_rvben-rumdl-benches-digest.txt`, `obsidian_blacksmithgu-obsidian-dataview-src-digest.txt` and `-docs-digest.txt`, `obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt` (and the whole-repo `obsidian_mnaoumov-obsidian-advanced-metadata-cache-digest.txt` only if needed — a dedicated metadata-cache design, high relevance here), `obsidian_obsidian-tasks-src-digest.txt`. Also `rust_zoni-obsidian-export-digest.txt` / `rust_cryptogladi-obsidian-parser-digest.txt` only if a Rust parse-to-summary example is needed (whole-repo digests; search before reading).

## Answer

Precedent research across git, cargo, rustc, rumdl, ruff, and obsidian plugins is documented in `.scratch/arch-redesign/research/06-document-summary-cache-freshness-precedent.md`:

1. **Freshness & Racy Timestamps**: `mtime + size` freshness is unsafe when files are modified in-place without length changes within the filesystem clock resolution ($t_{\text{mtime}} \ge t_{\text{index}}$). Standard mitigations are git's "racily clean" check (falling back to content hashing when timestamps coincide, plus truncating cached size to 0 in index records) or cargo's `invoked.timestamp` anchor rewinding. Neither hashes mtime into persistent cache keys.
2. **Persistence Boundary**: Comparables universally persist only derived semantic summaries (Dataview's `PageMetadata`, rustc query results, rumdl warnings). Concrete syntax trees and byte-precise spans remain transient in memory and are re-parsed on demand (~10 µs per note) when needed for analysis or LSP.
3. **Read Model Architecture**: Tools serving responsive queries (Dataview, Advanced Metadata Cache) maintain in-memory inverted indexes evaluating in microseconds (<10 ms). Using an embedded ACID store (redb with 16 ms open and 37 ms commit floors) as a live query engine with secondary multimap tables adds latency without benefit on ≤20K vaults where the dataset fits in tens of megabytes.
