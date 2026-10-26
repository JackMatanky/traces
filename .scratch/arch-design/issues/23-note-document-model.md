# What is the note document model: spans, facts, items, multi-value metadata?

Type: grilling Status: open Blocked by: 18, 10, 17, 12

## Question

Decide the shape of the parsed `Note` and of its editor counterpart. The index,
the query and the template consume the `Note`. The LSP needs source positions.
Ticket 17 requires no general write-back.

Decide the topics in this order. Topic A frames the other topics.

### A. Representation split

- Decide if one shared span-aware `Note` serves the index and the editor.
- Decide if the index `Note` and an editor `NoteDocument` are two types.
- Decide if the index keeps extracted facts and the editor keeps the document.
- Decide if the editor reparses the buffer on demand and the index persists no
  spans.
- Decide if spans are persisted in the index or transient in memory.
- Define `NoteLink` (syntax kind and source span). Decide which type `outlinks`
  holds.
- Decide how the editor shape builds from an unsaved in-memory buffer (ticket
  34). Do not promise an incremental reparse.

### B. Spans

- Decide the byte-span policy. Name each node that carries a span: headings,
  items, links, frontmatter.
- Decide how adapters get a span for each frontmatter field key, and for each
  list element where known (ticket 20).
- Measure the cost of frontmatter spans against the span-free fast path. The
  Noyalib CST is rejected (+59% parse time).
- Decide which body facts and spans the model keeps for ticket 36. Include the
  extent of each heading section and the body text slices for token budgets.

### C. Items and fields

- Decide whether list and task items remain children of `Note` or become
  separate indexed entities. State the row identity that ticket 26 needs
  (`item_idx`, `list.*` fields, document order).
- Decide if item fields and tags stay duplicated in note-level collections.
  Dataview rolls up non-task item fields only.
- Decide the multi-valued inline field semantics. Today the note stores all
  values and the query reads the first value.
- Decide how multi-valued inline fields classify element by element (ticket 19).
- Decide if the note model offers one effective field map. State how inline
  fields merge into it, and which source wins on a key collision (ticket 20).
- Confirm that the note keeps no raw lexeme. Ticket 20 renders the text view
  from the typed value.

### D. Parser

- Decide the per-task status representation: an owned `String`, or a symbol or
  id. State whether the parser still needs the status map.
- Decide the final place of `TaskStatus*` and `TaskListItem` (ticket 22).
- Decide the final field set of the parser input (ticket 22). Add each
  index-time read to `IndexPolicy`. Include the title, aliases and date keys.
- Decide if the parser input is borrowed or owned. Check it against a long-lived
  host (ticket 30).
- Decide between a single and a dual markdown pass.
- Decide how footnotes, callouts and slugs enter the model. Decide by consumer
  need (LSP navigation, heading-link resolution), not by the glossary. The code
  has none of them today.
- Decide the infallibility contract. State how the parser reports a warning, for
  example `NaN` becoming `Null` (ticket 19). Ticket 31 owns the mechanism that
  shows the warning.
- Keep path-derived facts, such as the filename period, out of `Note` (ticket
  21).

### Classify each change

Vault-format parsing is Frozen (ticket 16). Classify each parse-semantic change
as a row of the change table. Examples: fewer item tags in note-level
collections, multi-value results, `NaN` becoming `Null`. Name the query results
that change.

### Evidence gate

Decide only after the evidence. Use these inputs from `research/11`:

- The second markdown pass takes 20% to 38% of parse time.
- Status interning removes 10k `String` allocations. `ListItem` shrinks from 160
  B to about 128 B.
- Note deserialization is 97% of the cost of a path-set change.

Measure these before you decide:

- The record size and the parse cost of adding spans.
- The cost of frontmatter spans (topic B).
- The parity of heading text when the second pass is removed.

Set operation-specific budgets across the seven vault sizes (ticket 17).

### Fixed inputs

Do not reopen these decisions.

- Vault-format parsing is Frozen. The parser stays infallible.
- The Noyalib compat parser stays. The Noyalib CST is rejected for values.
- `Link` and `NoteLink` are separate types. `Link` is `target`, `display` and
  `embedded`.
- `NoteFieldValue` keeps its 80 B size test.
- The schema check is span-free (ticket 20). Adapters attach spans.
- The editor needs no general write-back. A future writer splices spans into the
  original bytes and never re-renders the tree.
- The zero-span design is rejected for the editor (ticket 17). It remains an
  option for the index shape only.

Facts: `survey/note.md` (all §3), `src/note/model.rs:24-33`,
`src/note/parser.rs:108,121,488-498`, `src/note/lists.rs:41-50`. Measured sizes
come from `research/11`. The judged-sound pieces are MarkerAccumulator, Logos
single pass, flat list hierarchy and infallible parser.

Alternatives (decide the split first, then the span policy inside each):

- Keep the zero-span read-mostly model for the index only. Add a separate editor
  shape.
- Use one shared `Note` with compact spans.
- Use a span-free `Note` plus an editor `NoteDocument`.
- Use extracted facts for the index plus a `NoteDocument` for the editor.
- Use a span-free `Note` and reparse the buffer on demand (Taplo and rumdl
  pattern).
- Use the lossless tree. Expect to reject it. Try to refute that expectation.
- Include the keep-as-is option for every topic.

Inputs: `research/10`, `research/11`, value model, forward-consumer decision.

### Resolution must name

- The alternatives considered, including the retain option.
- The owner of each invariant.
- What becomes removable.
- What remains unresolved.
- The downstream tickets that change: 24, 25, 26, 28, 32, 34, 35 and 36.

This ticket graduates these fog items: note parser internals (status interning,
second pass, `ListItem` footprint, footnote, callout and slug modelling).

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). The note type may split into an
indexing/query shape and an editor shape. The root `Link` carries `target`,
`display` and `embedded`; `NoteLink` (syntax kind and source span) is the
editor-side occurrence type and is defined here. Runtime values are
`note::NoteFieldValue` with `Int`/`Float`.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Inline fields
may now yield `DateTime` (civil when naive). Classification stays at parse and
is never re-sniffed; decide how multi-valued inline fields classify element by
element. Parse-time `NaN` becomes `Null` with a warning. `Link` identity is the
folded target, with `display` and `embedded` as attributes.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The schema
check is span-blind and takes a field map; adapters need a span per field key
(and per list element where known) from whichever editor shape is chosen.
Merging inline fields into that map is app-layer business and is not promised by
20. An MCP agent that edits files directly is checked post hoc; it adds no
    write-back requirement here.

Ticket 36 (body-section schemas) is blocked by this ticket: it needs to know
which body facts and spans the note model keeps. Class binding reads a note's
tags and path, both already available.

## Inputs from the IWE and Fileclass research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md) and
[14](../research/14-fileclass-model-binding-and-validation.md). IWE stores raw
text per key beside an arena tree of block nodes; spans are 1-based line ranges
per block node only, with no byte offsets and none on inlines
(`crates/liwe/src/graph.rs`, `model.rs`) [V], enough for its LSP, which uses
full-document sync. Its writer re-synthesises markdown from the tree
(`pulldown-cmark-to-cmark`); the CHANGELOGs record repeated text-loss
regressions and two reversals: normalise-on-write clobbering external edits (iwe
v0.26.0) and a normalise pass touching every file (diwe v0.24.1) [V]. [H] This
supports ticket 17's "no general write-back" and says any future writer must
splice spans into the original bytes instead of re-rendering; it does not by
itself refute the lossless-tree alternative. Fileclass has no body model
(frontmatter only, written through the host's `processFrontMatter`) and gives no
input.
