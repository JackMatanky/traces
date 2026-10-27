# Demands of the open tickets on ticket 23

This file lists what tickets 24 to 36 need from the note model. Line citations
of the form `23:NN` point to the version of ticket 23 in commit `47628bd3`. The
ticket text changed after that commit. Use the topic names, not the line
numbers. Line citations to the other tickets are from the agent. I did not check
them.

## Summary

- [V] Ticket 24 lists the note document model in its inputs
  (`24-index-read-model-and-query-execution.md:50-51`). It needs to know if the
  index stores the whole `Note` or extracted facts.
- [V] Ticket 25 needs the format version and the parser version. It needs the
  cost of note deserialization on a path-set change
  (`25-index-freshness-and-invalidation.md:26-39`).
- [V] Ticket 26 needs row identity: `item_idx`, the `list.*` fields and document
  order. It needs the multi-value semantics (`26-query-core-and-plan.md:17-18`).
- [V] Ticket 28 needs stable `NoteFieldValue` values at the template boundary
  (`28-template-engine-boundary.md:11-16`).
- [V] Ticket 30 needs a borrowed or owned parser input for a long-lived host
  (`30-workspace-host-and-composition.md:48-52`).
- [V] Ticket 31 needs the infallibility contract and a channel for parser
  notices (`31-error-and-diagnostic-architecture.md:30-37`).
- [V] Ticket 32 needs the visibility and the place of `Note`, `Link`, `NoteLink`
  and `TaskListItem` (`32-crate-topology-and-public-surface.md:75-80`).
- [V] Ticket 33 checks the invariants of ticket 23 again
  (`33-reconcile-decisions-and-audit-coverage.md:9-17`).
- [V] Ticket 34 is blocked by ticket 23. It needs a way to build the editor
  shape from a buffer in memory
  (`34-live-buffer-and-persisted-index-consistency.md:3-17`).
- [V] Ticket 35 needs to know if the note gives one merged field map
  (`35-schema-internals-simplification.md:9-16`).
- [V] Ticket 36 is blocked by ticket 23. Its question says: "Decide which body
  facts the note model keeps (23)" (`36-body-section-schemas.md:3-5,25`).
- [P] Ticket 23 settles these decisions now: the representation split, the span
  policy, the item row identity, the multi-value semantics, the notice channel
  and the status representation.
- [P] Ticket 23 leaves these decisions to later tickets: the redb table layout
  (24), the invalidation mechanism (25), the query plan (26), the diagnostic
  presentation (31), the crate boundaries (32) and the section schema syntax
  (36).

## Table: ticket, demand and topic

| Ticket | Demand on ticket 23 | Topic |
|---|---|---|
| 24 | Whole `Note` blob or extracted facts. Lazy decode or a snapshot in memory. | A |
| 25 | Parser version and format version. Note deserialization is about half of a path-set change. | A, D |
| 26 | Row identity for multi-row notes. First value or all values of a field. | C |
| 28 | `NoteFieldValue` at the template boundary without stringification. | C, D |
| 30 | Borrowed or owned parser input. Single or dual pass. | D |
| 31 | Infallibility contract. Channel for notices such as `NaN` to `Null`. Byte spans for diagnostics. | B, D |
| 32 | Visibility and place of `Note`, `NoteLink`, `Link`, `TaskListItem`. | A, D |
| 33 | Consistency of invariants. Removal of duplicated fields. | all |
| 34 | The editor shape from an unsaved buffer. Reparse on demand. | A |
| 35 | One merged field map, or frontmatter only. | C |
| 36 | Heading extent, text slices for token budgets, block kinds. | B |

## Decisions to settle now

1. Representation split (C1, C12).
   - Ticket 24 cannot decide the persisted format before this decision.
   - A shared `Note` with spans forces ticket 24 to persist or strip the spans.
   - A split lets ticket 24 design a compact record and lets ticket 34 reparse
     the buffer.
2. Span unit and span lifetime (C2, C3).
   - The unit sets the data type (`TextRange`, `u32` or `usize`).
   - [P] A transient span never enters the store and never becomes stale. The
     probe measures +4.3% bytes for persisted spans (`g-probe-results.md`). This
     weakens the case against persisted spans.
3. Item entity status and row identity (C4, C5). Ticket 26 needs it.
4. Multi-value semantics (C6). Today the parser keeps all values and the query
   reads the first value.
5. Infallibility and the notice channel (C11). Ticket 31 owns the presentation.
   Ticket 23 owns the function signature, for example `(Note, Vec<NoteNotice>)`.
6. Status representation and place (C7). Ticket 32 and ticket 24 need it.
   Research 11 says interning cuts `ListItem` from 160 B to 128 B and removes
   10k allocations. The probe confirms 160 B for `ListItem`.

## Decisions to leave open

1. The redb table layout and the pushdown of secondary indexes (ticket 24).
2. The invalidation mechanism and the fingerprint granularity (ticket 25).
3. The query intermediate representation (ticket 26).
4. The diagnostic severity, registry and presentation (ticket 31).
5. The crate boundaries and packaging (ticket 32).
6. The body section schema syntax and the token counter (ticket 36).

## Risks of an early answer

1. A monolithic `Note` with spans in the store.
   - Risk: ticket 24 decodes span data that queries never read. Ticket 25
     invalidates rows on whitespace edits.
   - [P] Keep spans transient or lazy.
2. A lossless tree.
   - Risk: the Noyalib CST costs +59% parse time (ticket 18). A stored tree
     costs memory across 20k to 50k notes (research 11).
   - Ticket 17 sets no general write-back. A writer can splice into the original
     bytes.
3. Token counting in the parser.
   - Risk: the `note` module depends on a tokenizer crate such as `tiktoken-rs`.
   - [P] Record text slices or offsets only. Ticket 36 counts tokens.
4. An early merge of inline fields into the frontmatter map.
   - Risk: tickets 20 and 35 lose the difference between authored frontmatter
     and body fields. Ticket 26 loses the provenance of a collision.
   - [P] Keep both maps. Offer an effective view as an accessor.
5. A parser input that holds a reference to the status map.
   - Risk: a long-lived host (ticket 30, ticket 34) needs the whole workspace
     config to reparse one buffer.
   - [P] Parse the task marker as a `char` or a symbol. Resolve the name later.

## Lessons for ticket 23

- C12: Keep the index record lean. Research 11 says note decoding takes 97.3% of
  the sum of decoding (18.48 ms) and link resolution (0.52 ms). A whole
  path-set change takes 32 ms to 35 ms. The probe shows the text, not the
  spans, is the bulk of the bytes.
- C1: Spans serve the LSP, the diagnostics (ticket 31) and the schema adapters
  (ticket 20). None of these runs on the cold query path [P].
- C7: Interning the status removes allocations and shrinks `ListItem`.
- C11: The parser never returns `Err`. It returns a valid note and a list of
  notices [P].

## Not checked

- A benchmark of zero-copy decoding against postcard.
- The memory cost of heading byte extents on vaults over 50k notes.
- The line citations to tickets 24 to 36.

## Inconsistencies found while rewriting

- The first version tagged every summary bullet and every lesson [V]. Bullets
  that state what ticket 23 "must settle" and every "lesson" are proposals. I
  tagged them [P].
- The first version says spans "bloat redb index storage by 3-5x". It gives no
  source. The probe measures +4.3% (`g-probe-results.md`). The claim has no
  support.
- The first version cites IWE (research 12) as a precedent for "plain queryable
  data" with the editor tree separate. The ticket 23 input says IWE keeps line
  spans on block nodes inside one arena tree. IWE does not show a split. [H]
- The first version says "Ticket 23 must settle ... that spans are transient".
  The user decides this. A recommendation is not a requirement.
- The line citations `23:NN` point to an earlier text of the ticket. They are
  stale.
- The first version says the cold query budget is "17-39 MB". It gives no
  citation. I did not check it in research 11.
