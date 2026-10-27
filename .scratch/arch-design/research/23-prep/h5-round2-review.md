# Round 2 review of the D3 decision set

Four scouts stress-tested D3 and the decisions F1 to F10. Their files are in
`.design/2026-10-26-model-note-facts-doc/scouts/` (`r2-*`). This file keeps the
review. Each fact is tagged [V] with a source. Each inference is tagged [H].
Each proposal is tagged [P].

## 1. The main finding: one event loop cannot feed both shapes cheaply

- [V] The parser does not scan source text. It builds transformed buffers. Link
  starts add a synthetic `[` and link ends add `]` (`src/note/parser.rs:411-439`).
- [V] The field scanner and the tag scanner drop their Logos spans
  (`src/note/parser/lexer.rs:19-37`). Task token spans refer to the joined item
  buffer, not to the source (`src/note/parser/task.rs:22-45`).
- [V] Only `Start(Item)` uses its event offset, and it turns the offset into a
  line (`src/note/parser.rs:284-287`). End events carry no offset.
- [H] A document hook inside the facts loop gets block ranges cheaply:
  headings, items, code, quotes, footnotes and full link ranges. It cannot get
  tag, field and link-part ranges without an offset map. The map would touch the
  Frozen parser.
- [H] My earlier probe (+1.4% for 404,753 occurrences) measured only event
  spans. It did not measure this hard part.

Two shapes remain:

| Shape | Description | Strongest argument against |
|---|---|---|
| H1 | Hook in the facts loop for block ranges. Source-slice scans for tags, fields, link parts and frontmatter keys. | The hook spreads through the list, marker, lexer and link handlers. It risks the Frozen parse results. |
| H2 | The facts parser stays as it is. A separate `NoteDoc::scan(src, settings)` walks the bytes with its own offset iterator. Both share one rules module. | A second Markdown pass for the editor. Rules can drift. |

- [P] Choose H2. The existing parser tests are the Frozen contract. H2 leaves
  them untouched. The extra pass costs about 45 microseconds per note (raw pass
  1: 121.8 ms for 2,716 notes) and runs only for an editor request.
- [P] One rules module holds the event options, the link classifier, the tag
  grammar, the field lexer and the slug rule. Both builders call it.
- [P] A differential test compares full ranges, not counts. Each range must
  slice the source to the expected text. Each fact needs one document occurrence.
  The test also checks that `doc` on or off gives the same `Note`.
- [P] A convenience call `parse_with_doc` runs both for an open buffer.

## 2. Other findings and the proposed fix

| Finding | Source | Proposed fix |
|---|---|---|
| Keep the stored note-level tags and inline fields. A derived view loses document order and the borrowed `&[Tag]` slice. | `src/note/parser.rs:488-498`; `src/note/model.rs:175-180` | Retain both. Revisit only with a measured duplicate size and an order-keeping view. |
| Multi-value rule also covers item fields, not only note fields. | `src/query/results.rs:320-333` | Apply one rule to both. Keep a frontmatter `Null` as a present winner. |
| Effective view must not add body fields to the schema check. | ticket 20 lines 52-65 | Keep the check frontmatter-only. |
| `Box<TaskData>` has no measured size or allocation cost. | `g-probe-results.md` | Measure boxed and unboxed after the name `String` leaves. |
| The guard must test the heading source range for `$`, not the cleaned text. | `src/note/parser.rs:106-161` | Add a mixed corpus with setext, escaped and empty math headings. |
| Bulk indexing drops notices. | `src/index/service.rs:328-355` | Collect path-qualified notices beside bulk results. Ticket 31 shows them. |
| A version bump deletes the store at open. 50,000 notes then reparse. Two processes can collide. | `src/index/store.rs:174-185` | The host reports "facts unavailable" during the rebuild. Tickets 25 and 30 own the two-process rule. |
| A background parse can publish ranges of old bytes after a newer buffer arrives. | ticket 34 lines 9-17 | Tag each range set with a source revision. Drop stale results. The open buffer wins. |
| A closed-file edit can race another write. | scout r2 case 4 | Compare the full source snapshot at commit. Abort on a difference. |
| Heading scope for tasks: the last earlier line can pick a heading inside a quote or list. | `src/note/heading.rs:7-45` | Define scope with parser heading ranges. Test it. |
| `BytePos` clamps above 4 GiB. A clamped edit range is wrong. | `src/position.rs:171-213` | Give no ranges for oversize input. |
| Same-file anchors are dropped before resolution. | `src/index/inlinks.rs:362-373` | Add a local heading check for them. |
| Images are not outlinks today. | `src/note/parser.rs:225-293` | State if `NoteDoc` records them. |
| Pipe rows: tables are not enabled, so a wikilink in a row has no cell boundary. | `src/note/parser.rs:67-73` | Use the same options in both builders. Test `\| [[a]] \|`. |

## 3. Design details the alternatives scout proposed

- [P] `NoteDoc` holds per-kind vectors (headings, links, fields, items,
  footnotes, callouts, code). It has no `'a` lifetime. The host owns the text as
  an `Arc<str>` with a version. A host builds its own cursor index.
- [P] Spans are checked `u32` byte spans plus a line-start index. The UTF-16
  conversion runs at the host boundary. Test CRLF, a final empty line, non-BMP
  text and a cursor inside a multi-byte character.
- [P] A small scanner reads frontmatter keys. It scans only the metadata event
  range. It prefers an absent span to a false span. Noyalib `Spanned` is not the
  sole source.
- [P] A link keeps three parts: path, heading and block id. Block targets stay
  unresolved. Do not persist a resolution. Keep every occurrence, even when links
  share one target.
- [P] Ticket 24 gets two options: item text out of line (text is 93% of item
  bytes) and the zk idea "select only the bytes a plan reads". A columnar layout
  and a separate task table are rejected for now.

## 4. Wording that must change in resolved tickets

- Ticket 18: add the stored internal-link fact beside the root `Link` and the
  editor-side `NoteLink`.
- Ticket 19: state that each element of a multi-value field gets parse-time
  classification.
- Ticket 20: replace "the app layer owns any merge" with note-owned effective
  lookup. Keep the frontmatter-only check.
- Ticket 22: drop the "do not move `TaskStatus`" assumption if the task payload
  no longer stores that type.

## 5. Downstream handoffs

24: one facts-only `Note` record; item text option. 25: the version bump rule.
26: item order, `item_idx`, list membership, multi-value item fields. 28: check
the external-link reader. 30: revision gate, background range index, settings
snapshot. 31: notice channel. 32: visibility of the internal-link fact, `NoteLink`
and the buffer parser. 34: buffer wins over disk and background work. 35: the
accessor is not the schema field resolution. 36: section extents and source
slices.

## 6. Not checked

- A working `NoteDoc` builder, so the cost of H2 is unmeasured.
- Memory of a host at 50,000 notes.
- A two-process redb test.
- The scout claims were spot-checked in three places only.
