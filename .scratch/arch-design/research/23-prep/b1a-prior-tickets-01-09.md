# Facts from tickets 01 to 09 that matter for ticket 23

This file lists the claims of the research tickets 01 to 09 and of
`findings-ledger.md`, `reverification.md` and `mission.md`, grouped by fact. The
first version repeated some facts in ten sections. This version lists each fact
once, with its concepts. Citations come from the agent. I did not check every
line. I checked the items marked "checked now".

Legend: BINDS means the fact limits ticket 23. INFORMS means it adds context.

## Summary

- [V] `parse_markdown` runs `pulldown-cmark` twice. The `Note` stores no byte
  span (`issues/01:19-20`; `reverification.md:91-95`). Checked now: the parser
  receives a `ByteSpan` for every event and drops it
  (`src/note/parser.rs:108-111`).
- [V] The parser copies the fields and the tags of list items into the
  note-level collections (`issues/01:19`). Checked now
  (`src/note/parser.rs:488-498`).
- [V] The code stores all values of a multi-valued inline field. The query reads
  only the first value (`issues/01:19-20`; `reverification.md:93-95,188`).
  Checked now (`src/query/results.rs:328`).
- [V] `QueryRow::cmp_document_order` ignores `item_idx` (`issues/01:16-17`;
  `findings-ledger.md:11-12`).
- [V] A path add or a path delete reads every note from redb and resolves all
  links again. A content-only edit does not (`issues/01:15`;
  `reverification.md:19-20,65-67`). Checked now: `a2a-index-persistence.md`.
- [V] A format-version mismatch wipes the store at open. An unreadable
  fingerprint fails closed (`reverification.md:20-25`). Checked now
  (`src/index/store.rs:38-43`).
- [V] The glossary has entries for slugs, footnotes and callouts. The code has
  none of them (`issues/01:20-21`; `findings-ledger.md:16-18`;
  `reverification.md:98-103`).
- [V] `src/GLOSSARY.md` defines `Occurrence`, `NavTarget`, `Resolution` and
  `Ambiguity`. None of them exists in `src/`.
- [V] The glossary entry "Avoid: relative path" conflicts with the core type
  `RelativePath` in `src/path.rs`.

## By concept

### C1, C3, C8: value pipeline

- [V] Four value enums exist: `FieldValue`, `FieldValueRef`, `NoteFieldValue`
  and `NoteFieldValueRef` (`issues/01:11-12,19-20`; `research/02:35-40,72-85`).
  BINDS: a new representation must not add another copy of the value type.
- [V] Frontmatter conversion has three stages: `noyalib::Value -> FieldValueRef
  -> NoteFieldValue` (`issues/01:11,19`; `reverification.md:36-40,89-95`).
- [V] The default deserialization of `FieldValueRef` is a pass-through. Coercion
  needs an explicit classify step (`DateTimeCoercion::Classify`)
  (`reverification.md:36-38`).

### C2, C3, C14: spans

- [V] The note stores no span for a heading, an item, a link or a frontmatter
  key (`reverification.md:91-95`). BINDS: no consumer can assume that a span
  exists today.
- [V] Taplo maps `jsonschema` errors back to CST nodes and byte spans
  (`research/03:54-57`). INFORMS.

### C4, C5, C6: items and fields

- [V] Item fields and tags are duplicated into `Note.inline_fields` and
  `Note.tags`. A query result changes if the duplicate is removed
  (`issues/01:19`).
- [V] Elements after the first value of a multi-valued field are unreachable
  through the note or the query (`reverification.md:93-95`).
- [V] `src/schema` never reads note values. The index extracts `fileClass`
  without the schema (`issues/01:18`; `issues/03:9-14`;
  `reverification.md:85-88`).

### C11, C13: validation and notices

- [V] Notes are open-world at ingestion. Schema validation is advisory and
  separate from ingestion (`research/03:79-81,92-93`; `issues/03:21-22`). BINDS.
- [H] The agent proposes `parse -> facts, and a separate Vec<Warning>`. No
  evidence in tickets 01 to 09 supports it. It is a proposal for ticket 23.

### C12, C15, C16: persistence and compatibility

- [V] `IndexFingerprint` is built by an exhaustive destructure of the config
  sub-models. It is stored in the redb `meta` table, in the same transaction as
  the rows. Drift wins over staleness (`reverification.md:20-25`;
  `issues/01:13-14`).
- [V] ADR-0009 is implemented: refresh-driven drift detection. The
  format-version mismatch wipes the store at open (`reverification.md:20-25`).
- [V] The merge of the earlier work added no detectable cost to refresh or to a
  cold query. The cost of the drift heal is not measured
  (`reverification.md:69-83`).

### C9, C10, C17: passes, glossary and names

- [V] The two passes are at `src/note/parser.rs:108,121` (`issues/01:19`).
- [V] The code has no slug generation and no callout extraction. Do not treat
  slugs, footnotes and callouts as facts of the model.

## Constraints that bind ticket 23

| Concept | Constraint | Evidence |
|---------|-----------|----------|
| C2, C14 | The `Note` stores no byte span today. A body-section schema cannot assume spans exist. | `reverification.md:91-93` |
| C4, C6 | Multi-valued inline fields are stored but read through `.first()`. A consumer of all values needs a model and query change. | `reverification.md:93-95` |
| C4 | `cmp_document_order` ignores `item_idx`. Item identity must not rely on the comparator. | `issues/01:16-17`; `findings-ledger.md:11-12` |
| C11 | Ingestion is not gated by schema validation. A separate analysis pass checks notes. | `research/03:79-81,92-93` |
| C16 | A path add or delete decodes every note. A design must keep this cost in mind. | `issues/01:15`; `reverification.md:65-67` |
| C15 | A drift in config heals by refresh. A format-version mismatch wipes the store. | `reverification.md:20-25` |
| C12, C17 | Glossary entries without code are not model facts. | `issues/01:20-21`; `reverification.md:98-103` |

## Claims of dubious quality

| Claim | Problem | Source |
|-------|---------|--------|
| Default coercion happens at deserialization for ISO strings. | Partly refuted. The default is a pass-through. | `issues/01:11`; `reverification.md:36-38` |
| `QuerySet` chaining clones the row vector. | Refuted. Only the `Arc` and the plan are cloned. | `issues/01:17`; `reverification.md:75-82`; `findings-ledger.md:14` |
| Template row cloning affects the `QuerySet` contract. | Only partly tied to the claim. The wider rule is the stringification at the template boundary. | `issues/01:17`; `reverification.md:75-82` |

## Not checked

- Tickets 01 to 09 give no evidence for C7 (task status), C13 (live buffer) or
  the borrowed parser input (C8).
- The line numbers in `reverification.md` and `findings-ledger.md`.
- Tickets 10 to 15. The agent for these tickets wrote no file. Research 10, 11,
  12 and 15 still need a careful read.

## Inconsistencies found while rewriting

- The first version says "[V] verified: cite `path:line` (only from provided ...
  files above)". The agent had no files above the legend. The legend is a
  leftover of the prompt.
- The first version has a C7 entry: "[H] Task status should be handled as an
  item-level entity with span-preserving placement". No evidence supports it. I
  removed it.
- The first version tags "Notes/index unconditionally parse and store" as [V]
  evidence for C13 (live buffer). The fact does not concern a live buffer. I
  removed the link.
- The first version lists "Any claim that expects byte spans in stored Note
  facts" as a refuted claim. No ticket makes this claim. I removed the row.
- The first version has about ten entries that say "the note stores zero byte
  spans". I merged them.
- The first version says `cmp_document_order` ignores `item_idx`.
  `a2b-query-template-consumers.md` says tied rows keep their order, because the
  sort is stable. Both can be true: the comparator does not break a tie between
  two items of one note, and the stable sort keeps their relative order. [H]
  Check `QueryRow::cmp_document_order` and the sort before ticket 26 decides row
  identity.
