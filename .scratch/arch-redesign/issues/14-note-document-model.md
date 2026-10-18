# Note document model: AST, spans, persistence boundary, parse pipeline

Type: grilling
Blocked by: 02, 03, 09, 13, 06

## Question

`Note` (`src/note/model.rs`) is simultaneously the parser's output, the in-memory unit of the index, the query layer's row source, and — through serde derives — the serialized record in redb's `NOTES` table. It discards source spans (only line numbers survive), is parsed by a pulldown-cmark pass plus a second pass for headings plus logos scans (reportedly three over list-item text), and takes parser options as borrowed `config` types. The language-server effort needs byte-precise occurrences and live-buffer parsing; the query/index side needs a compact, stable, cheap-to-load semantic summary. Decide whether **one** type can honestly serve all of these, or whether the model separates a *syntactic/analysis* representation (with spans, occurrences, recomputed from text) from a *persisted semantic summary*, plus how parsing is structured (single pass vs several), who owns parser options, and how the infallible-parse and flat-list-slice properties are preserved.

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify core findings](01-research-verify-core-findings.md)'s §6 ledger lands): [note §3, §7, §11](../survey/note.md), [index §6, §11.6](../survey/index.md). Ticket [Performance evidence](03-research-performance-evidence.md) supplies parse-cost and memory facts. Prior LSP decisions to reconcile (not obey blindly): md-pkm-lsp tickets 11 (span model), 40/41 (span persistence: persist vs `Option<ByteSpan>` vs transient), 26 (nav spine and `Occurrence`).

**Verified (ledgers landed 2026-10-18)** — [01 §6](../research/01-verify-core-findings.md): two pulldown-cmark passes (`parser.rs:109`, `:122`, the second with `ENABLE_MATH`); list-item text lexed at least three times (`list.rs:402`, `:420` → `task.rs:33`, plus pulldown-cmark itself); all `ByteSpan`s discarded (only `SourceLine` retained); `Note` is postcard-encoded straight into `NOTES` (`codec.rs:33`). The note GLOSSARY claims of heading slug dedup at parse time, footnote reference/definition nodes, and callout nodes are **refuted** — none exist in code (the LSP map's occurrence/slug designs describe future types). [02 §8](../research/02-verify-system-findings.md): only `FileMeta` and `Note` are structured redb payloads; 16 types carry serde derives solely for redb; the only format guard is `CACHE_FORMAT_VERSION = b"1"` — an encoding drift without a bump makes `read_batch` warn and silently drop rows. Perf ([03 §4–5](../research/03-performance-evidence.md)): a medium note parses in ~10 µs (~0.9 µs per list item, ~0.56 µs per frontmatter field, ~6 µs/KB); parsing is ~5–8% of cold build; second-pass cost is **unseparated** (not measurable without a private probe); resident cost is ~189 B per list item and ~39 KB transient heap per note during build. Conclusion: CPU savings from a single-pass parser are not demonstrated by the data; ownership, spans, and persistence-format coupling are the live issues.

## Alternatives (not exhaustive)

- Single `Note` extended with spans; decide persisted vs transient spans (LSP tickets 40/41).
- Two types: `ParsedNote`/analysis AST (spans, occurrences, never persisted) and `NoteSummary` (persisted/query-facing), with one projection function between them.
- Persist only derived facets (tags, links, headings, tasks) and keep ASTs transient.
- Single-pass event-driven extraction vs the current multi-pass structure; incremental/partial re-parse explicitly rejected or accepted.
- Whether the stored format derives from domain types via serde or from an explicit storage schema with a version.

## Done when

The answer states the type(s) and their consumers, span residency (and its effect on the redb format), the parse pipeline structure and its allocation/complexity profile, ownership of parse options (consistent with [Config ownership](13-config-ownership-and-lifecycle.md)), the heading/footnote/callout/slug scope that belongs to the model vs the LSP resolver, and the amendments required to md-pkm-lsp tickets 11/40/41 if any.
