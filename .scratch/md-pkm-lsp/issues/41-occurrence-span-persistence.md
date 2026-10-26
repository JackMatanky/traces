# Occurrence-span persistence: persist `Spanned<T>` spans through redb (amending ticket 11 §8), or keep them transient behind `Option<ByteSpan>`?

Type: grilling
Status: open
Blocked by: (resolved) 26, 27
Assigned: agent

## Question

Note-parse ticket 04's original span design was withdrawn mid-triage as a
type-system lie: correctness by prose across the persist boundary. That design
used a mandatory `Spanned<T>` with `#[serde(skip)]` on the span, plus the doc
invariant "store-loaded spans decode 0..0, never trust them". Two
mechanically-checked options remain. [research
40](../research/40-spanned-occurrence-persistence.md) records them with full
evidence:

- **B, persist spans**: use plain serde derives on `Spanned<T>`. Spans
  round-trip through the `NOTES` table. This causes zero test churn: the derived
  span-inclusive `Eq`/`Ord` stay honest, and every round-trip assert and both
  ordering pins pass unmodified. It needs an amendment to ticket 11 §8 ("spans
  stay transient, never in redb").
- **C, honest absence**: use `span: Option<ByteSpan>`, which is `None` after a
  store load. This keeps §8. It forces manual value-only
  `PartialEq`/`Eq`/`Hash`, because a decoded `None` against an encoded `Some`
  fails the whole-`Note` round-trip asserts. That contradicts span-inclusive
  `Ord`, so ordering goes value-only and the two pins (`src/position.rs:853`,
  `:863`) are rewritten. It also adds `Option` plumbing at every span consumer.

Decide. The settling evidence: does any real consumer need byte ranges from
*store-assembled* `Note`s? Those are redb-loaded with no live parse;
`StoreSnapshot` already hands `SortedByPath<Note>` to the query side
(`src/index/store.rs:38-39`). Or does every range-serving path re-parse the file
first? Research 39's model says spans recompute at request time from text the
LSP already touches. That hinges on what this ticket's downstream consumers
demand:
[definition/references/hover/rename](26-definition-references-hover-rename.md)
and [structural symbols/editor
intelligence](27-structural-symbols-and-editor-intelligence.md). It also hinges
on what the resolved answers of [LSP
persistence/caching](13-lsp-persistence-and-caching-strategy.md) and [live
buffer vs filesystem overlay](14-live-buffer-vs-filesystem-overlay.md) say about
closed-file serving. Re-read those at grilling time.

Also in scope of the decision:

- If B wins: write the replacement text for §8. When are persisted spans
  trusted? Research 40's draft: they are valid for the snapshot they rode in,
  and every row write refreshes them.
- If C wins: decide where the `Option` boundary sits. Put it inside
  `Spanned<T>`, or keep plain values in `Note`'s persisted collections and
  attach spans only in transient views.

Feeds: note-parse ticket 04 (already rewritten Heading-only; spans return as a
follow-up ticket once this decides) and md-pkm-lsp ticket 11 §1/§8.

## Inputs from 25 (2026-10-15)

- **Byte-precision closed-file diagnostics need link ranges (25 §4.4)**: even
  option 41-B does not give closed-file *link* ranges, because `Link` has no
  position field at all. Byte-precision diagnostics on closed files need either
  41's persisted link spans or a re-read of file text on cold compute. 25's
  research costed that re-read. This feeds the 41-B vs 41-C
  choice: 41-C (plain `Option<ByteSpan>`) leaves closed-file link diagnostics
  cold-compute-only.
- **The blake3 sidecar cache question routes HERE (25 §10.4/11.1)**, not to
  resolved 13. An optional blake3 sidecar cache maps a file hash to computed
  results. It eliminates disk re-read on cold compute. It is *not* a redb table.
  So 13's "no new redb tables" constraint is untouched either way. 41 owns the
  sidecar-or-not decision, and it interacts with span persistence: the sidecar
  would need to carry spans too.

## Inputs from 26 (2026-10-18)

- **Demand statement**: definition/references/rename consume spans for **closed
  files** too, not only live-session requests. Serving means re-parse on demand
  from `ContentResolver::text_of` for cold files, and resident spans when
  cached. **26 does not need persisted spans** (41-C is acceptable). 41-B is the
  upgrade path if the re-parse cost lands outside budget.
- **11 §8 amended**: its "only live-editing-session requests need spans"
  sentence is corrected (see 11's Amended-by-26 block). This ticket's framing
  inherits the correction.
- **Evidence corrected**: gopls v0.12 (Sept 2023) persists per-identifier
  cross-reference indexes (go.dev/blog/gopls-scalability), so "no PKM peer
  persists occurrence indexes" is false as previously stated. The
  transient-spans justification is now "markdown occurrence scanning is cheap
  (KB-scale notes) and index maintenance is the measured failure mode elsewhere"
  (markdown-oxide MAX_INDEXED_LINES, zk ~7-minute cold index), not "nobody
  persists".
- **Blocking consequence**: under `concurrency_level(1)` a long rename blocks
  every subsequent request (26's A2 decision: rename is the sole >100ms handler
  with `$/progress`). This ticket's serving/persistence choice is the structural
  fix point.

## Inputs from iwe research (2026-10-21)

- (amended 2026-10-21 by iwe research) iwe's rebuild-every-start (~958ms @ 20k
  docs, indexed load) corroborates that a markdown graph rebuild is cheap. Its
  raw-markdown `ServerParams.state` handoff corroborates that content handoff beats
  serialized indexes ([tool-iwe](../research/tool-iwe.md) Part 3, Part 4).

## Input from 27 (2026-10-22)

- 27 is resolved and no longer blocks 41. Demand statement: 27's document-scope
  features need transient spans only. The LSP-side walk produces exact byte
  spans per request, and nothing 27 ships reads persisted spans. So 41-C (honest
  absence, `span: Option<ByteSpan>`) is acceptable from 27's side. 26's
  cross-file consumers remain 41's other demand source.
