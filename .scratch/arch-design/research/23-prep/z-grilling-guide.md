# Grilling guide for ticket 23

**Superseded.** This guide came before the final decision. Its candidates S1 to
S6 and its recommendations are history. The ticket's `## Answer`, `h4` and `h5`
hold the final design. The "Corrections to the evidence" section stays valid.

This guide helped the grilling of `issues/23-note-document-model.md`. Each
recommendation is a proposal [P]. Each fact has a source. `g-probe-results.md`
holds the measurements. The brief lists known problems in the other prep files.

## Corrections to the evidence

- [V] The "97%" of research 11 is a share of two parts only: note decoding
  (18.48 ms) and link resolution (0.52 ms) for 5,000 notes. The same research
  gives 32.2 ms to 35.2 ms for a whole path-set change
  (`research/11:18,93-108`). Decoding is about half of the whole change. The
  ticket now says this.
- [V] Research 11 estimates `ListItem` at about 128 B after interning. Research
  10 estimates about 96 B (`research/10:80`). No one measured either value.
  `ListItem` is 160 B and `TaskListItem` is 72 B today (`g-probe-results.md`).
- [V] No code in `src/index` compares whole `Note` values. `patch_and_diff`
  compares link edges (`src/index/inlinks.rs:216`). A span in the note cannot
  invalidate other data. The "stale span" argument does not apply to this index.
- [V] The parser copies the fields and tags of task items and of non-task items
  into the note (probe 4). Dataview copies the fields of non-task items only
  (`research/10:24`; the Dataview digest, lines 3603-3604).
- [V] A guard on heading lines gives the same heading text as the second pass.
  The probe checked 2,688 notes without `$` in a heading and found 0 mismatches.
  It found 5 differences in the 23 notes that have `$` in a heading.
- [V] The record size is a text problem, not a span problem. Item text is 93% of
  the `ListItem` bytes. Spans add 4.3% (`g-probe-results.md`).

## Stress test of the candidate designs

Requirements used:

- R1: The LSP gets positions for diagnostics, navigation and completion (ticket
  17).
- R2: An adapter gets a span for each frontmatter key (ticket 20).
- R3: The cold CLI path stays fast. Decoding is 16.6 ms for 2,711 notes.
- R4: Index results equal a fresh parse (ticket 16).
- R5: An unsaved buffer gives a note without disk access (ticket 34).
- R6: Few types and few duplicated invariants.
- R7: Body facts for ticket 36 (heading extent and text slices).

| Design | R1 | R2 | R3 | R4 | R5 | R6 | R7 | Main failure |
|--------|----|----|----|----|----|----|----|--------------|
| S0. Keep: one span-free `Note` | fails | fails | pass | pass | pass | pass | partial | Ticket 17 rejected it for the editor. |
| S1. One `Note` with spans on body nodes, stored | pass | partial | pass | pass | pass | pass | pass | Frontmatter key spans are not free. The record grows 4.3%. |
| S2. One `Note`, spans in memory only (`serde(skip)`) | partial | partial | pass | pass | pass | fails | partial | An index decode gives no spans. The earlier LSP effort withdrew this pattern as a "type-system lie" (`research/20-prep/brief.md:66`). |
| S3. Span-free `Note` and a `NoteDocument` made by reparse | pass | pass | pass | pass | pass | partial | pass | Two types. The LSP must reparse files for spans. |
| S4. One parser makes a `NoteDocument`. A projection makes the `Note`. | pass | pass | partial | pass | pass | fails | pass | Two types and a projection to keep equal. The CLI pays the document cost. [H] |
| S5. Lossless tree (Rowan or Noyalib CST) | pass | pass | fails | pass | pass | fails | pass | +59% parse time (ticket 18). Ticket 17 needs no write-back. |
| S6. Store spans instead of item text | pass | n/a | pass | partial | partial | partial | pass | A query must read the files. Freshness depends on the mtime scan. |

Notes on the table:

- S1 on R2: body nodes give spans free from `into_offset_iter`. Frontmatter key
  spans need another source (see question 3).
- S3 on R1: a workspace-wide references request needs the link spans of files
  that are not open. S3 reparses each file for it. A note with 1,000 backlinks
  costs about 0.2 s (1,000 times 217 µs) [H].
- LSP start-up: a host that parses every note needs about 217 µs for each note.
  This is 10.9 s for 50,000 notes on one thread. A host that decodes stored
  notes needs about 6.1 µs for each note. This is 0.31 s. These are linear
  extrapolations from the probe [H]. They favour stored spans for a long-lived
  host.
- S6 is not a ticket 23 decision. The text is 93% of the record bytes. Ticket 24
  owns persistence. Send S6 to ticket 24 as an option.
- Ticket 17 and ticket 18 reject S5. Keep the short test of S5 that the ticket
  asks for.

## Questions and options

### Q1. Representation split

Options: S1, S3 or S4. S0, S2 and S5 fail a requirement.

- [P] Ask first: does the LSP host load all notes at start-up, or does it
  reparse on demand? The answer chooses between S1 and S3.
- [P] If the host loads stored notes at start-up, S1 wins on cost and types.
- [P] If the host is short-lived or lazy, S3 wins on a lean record.
- What would change the choice: a measured cost of a `NoteDocument` build, which
  nobody has measured.
- Ticket 18 assumes a split (`NoteLink` is editor-side). If the user picks S1,
  `NoteLink` is not editor-side. Ask the user about this link.

### Q2. Which nodes carry spans, and are they stored

- [P] If S1: spans on `Heading`, `ListItem` and each link occurrence. Two `u32`
  offsets for each (`ByteSpan` exists in `src/position.rs`).
- [P] Keep `line` on `Heading` and `ListItem`, because `list.line` is a query
  field and needs no source text.
- [V] Cost: +4.3% bytes on the probe corpus. A format version change is Free
  (ticket 16).
- What would change the choice: a vault with many short items. The span share
  grows when the items are short. The probe corpus has long items (144 B mean).

### Q3. Frontmatter key and element spans

- [V] A line scan costs 0.5% of the parse time. Noyalib `Spanned<Value>` costs
  +181% to +225% and gives value spans only, with loose end offsets.
- [P] Do not store frontmatter spans. Compute them on demand for an open buffer.
  Use the line scan for top-level keys. Use `Spanned<Value>` for the one block
  of an open buffer when an element span is needed. The cost is about 25 µs for
  one block.
- Failure: the line scan misses quoted keys, complex keys and flow mappings [H].
  Test it against the YAML subset that the parser accepts.

### Q4. Items: children or entities

- [P] Keep items as children of `Note`. Dataview does the same
  (`research/10:27`). A secondary table for task queries is a ticket 24 option.
- [V] A row is rebuilt from the stored `Note` at query time. The pair (note
  index, `item_idx`) is a stable id inside one query. A stored id needs a rule.
- Failure: a consumer that needs an item id across edits (for example a task
  write-back). Ticket 17 sets no write-back.

### Q5. Rollup of item fields and tags

Options:

1. Retain. The parser copies the fields and tags of all items.
2. Align with Dataview. Copy the fields of non-task items only. This changes
   page queries on task fields. It needs a change-table row.
3. Remove the stored copy. Derive the note-level view from `lists` in an
   accessor. Results stay the same. The record loses the duplicate.

- [P] Option 3 keeps the Frozen meaning and removes the duplicate. Its cost is a
  scan of the items for each lookup. Measure it, or cache the view in memory.
- [H] The size of the duplicate is not measured.
- Tags: the probe shows the note tags include the tags of task items and plain
  items. Keep this. It matches the page tags of Obsidian [H].

### Q6. Multi-valued fields and the effective field map

- [V] The code stores all values. The query reads the first value
  (`src/query/results.rs:328`). Ticket 16 sanctions the fix.
- [V] `Note::get` and `Note::fields()` give the frontmatter value first and skip
  an inline key that the frontmatter has (`src/note/model.rs:128-173`). Dataview
  concatenates both sources (the Dataview digest, lines 3391-3393).
- Options for a key in both sources: frontmatter wins (today); concatenate
  (Dataview); keep two maps and let the app layer choose.
- [P] Keep two maps in the note. Offer the effective view as an accessor. The
  schema check and ticket 35 keep the difference between authored frontmatter
  and body fields.
- Options for several values: always a list; scalar for one and a list for
  several (Dataview). [P] A uniform list avoids two shapes.
- Ask the user who owns the merge: the note model or the app layer.

### Q7. Task status representation

- [V] `TaskStatus` is `symbol: char`, `name: String`, `kind`
  (`src/task.rs:50-55`). The parser needs the kind to compute `fully_complete`.
- [P] Store the symbol and the kind. Resolve the name from the status map when a
  consumer needs it. The parser keeps the map for the kind only.
- Failure: a consumer that prints the name without the map. Ticket 22 puts the
  map in the parser input, so the parser input stays small.
- [H] The `ListItem` size after this change is not measured.

### Q8. Parser input

- [P] Pass the settings by reference to `parse`. Do not store a reference in any
  type. A long-lived host owns the settings in an `Arc`. This avoids lifetime
  contagion (the red-team concern of ticket 22) [H].
- Every index-time read goes into `IndexPolicy` (ticket 22).

### Q9. One markdown pass or two

- [V] The guard gives exact parity on the corpus. The raw second pass costs 21%
  of the full parse time.
- [P] Collect heading text in pass 1. Run pass 2 only when a heading line has a
  `$`. Add a differential test as a permanent test. It protects the two tests
  that pin math behaviour (`src/note/parser.rs:725-748,772-785`).
- [H] The end-to-end saving is not measured. Measure it before the decision.

### Q10. Footnotes, callouts and slugs

- [V] `pulldown-cmark` emits footnote events already (`ENABLE_FOOTNOTES` is on).
  It makes no slugs. `ENABLE_GFM` gives five alert kinds only.
- [P] Do not store slugs. Derive them from the heading text on demand, as rumdl
  does (`research/10:38`). Delete the footnote and callout entries from the
  glossary until a consumer needs them. This follows the rule to decide by
  consumer need.
- Failure: heading-link resolution (`[[note#heading]]`) needs a rule for
  duplicate headings. Markdown-oxide resolves dash and space forms (`e2`).

### Q11. Infallibility and notices

- [V] The parser has one known notice today: `NaN` becomes `Null`.
- Options: return `(Note, Vec<Notice>)`; keep notices inside the `Note`; log
  only.
- [P] Return the notices from the parser call. Do not store them. A stored
  notice would repeat on every query. Ticket 19 says notices are advisory.
- Ticket 31 owns how a notice shows.

### Q12. Body facts for ticket 36

- [P] Keep headings with level, text, line and span. Derive the section extent
  from the next heading of the same or a lower level. Do not store paragraphs,
  code blocks or tables. Take text slices from the source on demand.
- Failure: a body schema that checks a paragraph count needs more facts. Ask
  ticket 36 for its list before it blocks.

### Q13. Change-table rows

Ask the user to approve each row (ticket 16):

- Removal or change of the task-field rollup (Q5).
- Multi-value results (Q6).
- `NaN` to `Null` with a notice (Q11).
- Any change of heading text (Q9). The guard should change none.

## Not checked

- The cost of building a `NoteDocument` (S4).
- The end-to-end gain of the guarded pass.
- The size of the rollup duplicate.
- The sizes after status interning.
- The probe corpus is the repository documentation, not a personal vault.
- Research 12 and 15 (read only through the ticket summaries).
- The extrapolations for LSP start-up and for backlink reparse.
