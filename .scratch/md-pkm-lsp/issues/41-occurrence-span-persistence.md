# Occurrence-span persistence: persist `Spanned<T>` spans through redb (amending ticket 11 §8), or keep them transient behind `Option<ByteSpan>`?

Type: grilling
Blocked by: (resolved) 26, 27

## Question

Note-parse ticket 04's original span design — a mandatory `Spanned<T>` with `#[serde(skip)]` on the span plus the doc invariant "store-loaded spans decode 0..0, never trust them" — was withdrawn mid-triage as a type-system lie: correctness by prose across the persist boundary. Two mechanically-verified options remain, recorded with full evidence in [research 40](../research/40-spanned-occurrence-persistence.md):

- **B — persist spans**: plain serde derives on `Spanned<T>`; spans round-trip through the `NOTES` table; zero test churn (the derived span-inclusive `Eq`/`Ord` stay honest, every round-trip assert and both ordering pins pass unmodified); requires amending ticket 11 §8 ("spans stay transient, never in redb").
- **C — honest absence**: `span: Option<ByteSpan>`, `None` after a store load; keeps §8; forces manual value-only `PartialEq`/`Eq`/`Hash` (decoded `None` vs encoded `Some` fails the whole-`Note` round-trip asserts), which contradicts span-inclusive `Ord` — so ordering goes value-only and the two pins (`src/position.rs:853`, `:863`) are rewritten — plus `Option` plumbing at every span consumer.

Decide. The settling evidence: whether any real consumer needs byte ranges from *store-assembled* `Note`s (redb-loaded with no live parse — `StoreSnapshot` already hands `SortedByPath<Note>` to the query side, `src/index/store.rs:38-39`) or whether every range-serving path re-parses the file first (research 39's model: spans recomputed at request time from text the LSP already touches). That hinges on what this ticket's downstream consumers actually demand — [definition/references/hover/rename](26-definition-references-hover-rename.md) and [structural symbols/editor intelligence](27-structural-symbols-and-editor-intelligence.md) — and on what the resolved answers of [LSP persistence/caching](13-lsp-persistence-and-caching-strategy.md) and [live buffer vs filesystem overlay](14-live-buffer-vs-filesystem-overlay.md) say about closed-file serving; re-read those at grilling time.

Also in scope of the decision:

- If B wins: the replacement text for §8 (when are persisted spans trusted? — research 40's draft: valid for the snapshot they rode in, refreshed with every row write).
- If C wins: where the `Option` boundary sits — inside `Spanned<T>`, or plain values in `Note`'s persisted collections with spans attached only in transient views.

Feeds: note-parse ticket 04 (already rewritten Heading-only; spans return as a follow-up ticket once this decides) and md-pkm-lsp ticket 11 §1/§8.

## Inputs from 25 (2026-10-15)

- **Byte-precision closed-file diagnostics need link ranges (25 §4.4)**: even span-persistence option 41-B doesn't give closed-file *link* ranges (`Link` has no position field at all). Byte-precision diagnostics on closed files therefore require either 41's persisted link spans or re-reading file text on cold compute (costed in 25's research). Feeds the 41-B vs 41-C choice: 41-C (plain `Option<ByteSpan>`) leaves closed-file link diagnostics cold-compute-only.
- **blake3 sidecar cache question routes HERE (25 §10.4/11.1)**, not to resolved 13: an optional blake3 sidecar cache (file-hash → computed results, eliminating disk re-read on cold compute) is *not* a redb table, so 13's "no new redb tables" constraint is untouched either way; 41 owns the sidecar-or-not decision (it interacts with span persistence — the sidecar would need to carry spans too).

## Inputs from 26 (2026-10-18)

- **Demand statement**: definition/references/rename consume spans for **closed files** too (not only live-session requests); serving = re-parse on demand from `ContentResolver::text_of` for cold files, resident spans when cached. **26 does not require persisted spans** (41-C acceptable); 41-B is the upgrade path if the re-parse cost lands outside budget.
- **11 §8 amended**: its "only live-editing-session requests need spans" sentence is corrected (see 11's Amended-by-26 block); this ticket's framing inherits the correction.
- **Evidence corrected**: gopls v0.12 (Sept 2023) persists per-identifier cross-reference indexes (go.dev/blog/gopls-scalability), so "no PKM peer persists occurrence indexes" is false as previously stated. The transient-spans justification is now "markdown occurrence scanning is cheap (KB-scale notes) and index maintenance is the measured failure mode elsewhere" (markdown-oxide MAX_INDEXED_LINES, zk ~7-minute cold index), not "nobody persists".
- **Blocking consequence**: under `concurrency_level(1)` a long rename blocks every subsequent request (26's A2 decision: rename is the sole >100ms handler with `$/progress`); this ticket's serving/persistence choice is the structural fix point.

## Inputs from iwe research (2026-10-21)

- (amended 2026-10-21 by iwe research) iwe's rebuild-every-start (~958ms @ 20k docs, indexed load) and raw-markdown `ServerParams.state` handoff corroborate that markdown graph rebuild is cheap and content handoff beats serialized indexes ([tool-iwe](../research/tool-iwe.md) Part 3, Part 4).

## Input from 27 (2026-10-22)

- 27 is resolved and no longer blocks 41. Demand statement: 27's document-scope features need transient spans only (the LSP-side walk produces exact byte spans per request; nothing 27 ships reads persisted spans), so 41-C (honest absence, `span: Option<ByteSpan>`) is acceptable from 27's side; 26's cross-file consumers remain 41's other demand source.
