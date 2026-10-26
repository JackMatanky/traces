# What is the note document model: spans, facts, items, multi-value metadata?

Type: grilling Status: open Blocked by: 18, 10, 17, 12

## Question

Decide the shape of the parsed `Note`. Decide the shape of its editor
counterpart. The index, the query and the template use the `Note`. The LSP needs
source positions. Ticket 17 requires no general write-back.

Decide the topics in this order. Topic A frames the other topics.

### A. Representation split

- Decide if one shared `Note` with spans serves the index and the editor.
- Decide if the index `Note` and an editor `NoteDocument` are two types.
- Decide if the index keeps extracted facts and the editor keeps the document.
- Decide if the editor reparses the buffer on demand and the index keeps no
  spans.
- Decide if the index persists spans or if spans stay transient in memory.
- Define `NoteLink` (syntax kind and source span). Decide which type `outlinks`
  holds.
- Decide how the editor shape builds from an unsaved buffer in memory (ticket
  34). Do not promise an incremental reparse.

### B. Spans

- Decide the byte-span policy. Name each node that carries a span: headings,
  items, links and frontmatter.
- Decide how adapters get a span for each frontmatter field key (ticket 20).
  Decide how they get a span for each list element where known.
- Measure the cost of frontmatter spans against the span-free fast path. The
  Noyalib CST is rejected (+59% parse time).
- Decide which body facts and spans the model keeps for ticket 36. Include the
  extent of each heading section. Include the body text slices for token
  budgets.

### C. Items and fields

- Decide if list items and task items stay children of `Note` or become separate
  indexed entities. State the row identity that ticket 26 needs: `item_idx`, the
  `list.*` fields and document order.
- Decide if item fields and item tags stay duplicated in the note-level
  collections. Today the parser copies the fields and tags of task items and of
  non-task items. Dataview copies the fields of non-task items only.
- Decide the semantics of multi-valued inline fields. Today the note stores all
  values and the query reads the first value.
- Decide how a multi-valued inline field classifies each element (ticket 19).
- Decide if the note model offers one effective field map. Decide who owns the
  merge of inline fields into that map: the note model or the app layer (ticket
  20). Decide which source wins when a key exists in both.
- Confirm that the note keeps no raw lexeme. Ticket 20 renders the text view
  from the typed value.

### D. Parser

- Decide the representation of the task status: an owned `String`, or a symbol
  or id. State if the parser still needs the status map.
- Decide the final place of `TaskStatus*` and `TaskListItem` (ticket 22).
- Decide the final field set of the parser input (ticket 22). Add each
  index-time read to `IndexPolicy`. Include the title, aliases and date keys.
- Decide if the parser input is borrowed or owned. Check the decision against a
  long-lived host (ticket 30).
- Decide between a single markdown pass and a dual markdown pass.
- Decide how footnotes, callouts and slugs enter the model. Decide by consumer
  need, not by the glossary. Consumers are LSP navigation and heading-link
  resolution. The code has none of them today.
- Decide the infallibility contract. State how the parser reports a notice, for
  example when `NaN` becomes `Null` (ticket 19). Ticket 31 owns the mechanism
  that shows the notice.

### Compatibility rules

Ticket 16 makes vault-format parsing Frozen. The same files must parse to the
same meaning. Record each observable change in the change table. Give each
change a tier and a migration. An observable change that is not in the table is
a regression.

Examples of changes: fewer item tags in the note-level collections, new
multi-value results, and `NaN` becoming `Null`. Name the query results that
change.

Ticket 16 lists the `.first()`-only read of multi-valued inline fields as a
known defect. Fixing it is a change-table row. It is not a regression.

The `index.redb` layout and versioning are Free (ticket 16). Index results must
equal the results of a fresh parse (Frozen).

### Evidence gate

Decide only after you see the evidence. Use these inputs from `research/11`:

- The second markdown pass takes 20% to 38% of the parse time.
- Status interning removes 10k `String` allocations. Research 11 estimates that
  `ListItem` shrinks from 160 B to about 128 B. Research 10 estimates about 96
  B. No one measured the result.
- Note deserialization takes 97.3% of the sum of two parts: note decoding (18.48
  ms) and link resolution (0.52 ms) for 5,000 notes. A whole path-set change
  takes 32 ms to 35 ms.

Measure these before you decide:

- The record size and the parse cost of added spans.
- The cost of frontmatter spans (topic B).
- The parity of heading text when you remove the second pass.

Ticket 17 names seven vault sizes: 50, 200, 1,000, 5,000, 10,000, 20,000 and
50,000 notes. Use them as measurement points. Choose the range of each parser
operation from evidence. An operation does not need equal optimization at every
size.

### Fixed inputs

Do not reopen these decisions.

- Vault-format parsing is Frozen (ticket 16). The parser stays infallible.
- The Noyalib compat parser stays. The Noyalib CST is rejected for values.
- `Link` and `NoteLink` are separate types. `Link` holds `target`, `display` and
  `embedded`.
- `NoteFieldValue` keeps its 80 B size test.
- The schema check is span-free (ticket 20). Adapters attach spans.
- The editor needs no general write-back. A future writer splices spans into the
  original bytes. It never re-renders the tree.
- The zero-span design is rejected for the editor (ticket 17). It stays an
  option for the index shape only.
- Path-derived facts, such as the filename period, stay out of `Note` (ticket
  21).

### Facts

- `survey/note.md` (all of §3).
- `src/note/model.rs:24-33`.
- `src/note/parser.rs:108,121,488-498`.
- `src/note/lists.rs:41-50`.
- Measured sizes come from `research/11`.
- Sound parts: `MarkerAccumulator`, the Logos single pass, the flat list
  hierarchy and the infallible parser.

### Alternatives

Decide the split first. Then decide the span policy inside each alternative.

- Keep the zero-span read-mostly model for the index only. Add a separate editor
  shape.
- Use one shared `Note` with compact spans.
- Use a span-free `Note` and an editor `NoteDocument`.
- Use extracted facts for the index and a `NoteDocument` for the editor.
- Use a span-free `Note` and reparse the buffer on demand (Taplo and rumdl
  pattern).
- Use the lossless tree. Expect to reject it. Try to refute that expectation.
- Include the keep-as-is option for every topic.

### Sources

Read `research/10`, `research/11` and the `Inputs from …` sections below.

### Resolution must name

- The alternatives considered, including the retain option.
- The owner of each invariant.
- What becomes removable.
- What remains unresolved.
- The downstream tickets that change: 24, 25, 26, 28, 30, 31, 32, 34, 35 and 36.

This ticket graduates these fog items: note parser internals (status interning,
second pass, `ListItem` footprint, and footnote, callout and slug modelling).

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). The note type can split into an
index shape and an editor shape. The root `Link` holds `target`, `display` and
`embedded`. `NoteLink` holds the syntax kind and the source span. Ticket 18
calls `NoteLink` the editor-side occurrence type. This ticket defines
`NoteLink`.

Ticket 18 assumes the split. Topic A has not chosen the split. Decide topic A
before you define `NoteLink`. Runtime values are `note::NoteFieldValue` with
`Int` and `Float`.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md).

- Inline fields can now yield `DateTime` (civil when naive).
- The parser classifies a value at parse time. No code classifies it again.
- Decide how a multi-valued inline field classifies each element.
- A parse-time `NaN` becomes `Null`. The parser reports this case.
- `Link` identity is the folded target. `display` and `embedded` are attributes.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md).

- The schema check is span-blind. It takes a field map.
- Adapters need a span for each field key. They need a span for each list
  element where known. The chosen editor shape must give these spans.
- Ticket 20 says the app layer owns the merge of inline fields into the field
  map. Ticket 20 also says ticket 23 handles it. Ticket 20 promises no merge.
  Topic C decides the owner.
- An MCP agent that edits files directly is checked after the edit. It adds no
  write-back requirement here.
- Class binding reads the tags and the path of a note. Both are available.
- Ticket 36 (body-section schemas) is blocked by this ticket. It needs to know
  which body facts and spans the note model keeps.

## Inputs from the config ownership decision

See [ticket 22's Answer](./22-config-ownership-and-lifecycle.md).

- `note` defines its own parser-input type. The type holds the status map, the
  tag filters and the tags field name. `note` imports nothing from `config`.
- Build the parser input one time for each indexing run. The parser borrows it.
  Do not clone it for each file.
- The parser-input fields are provisional. This ticket decides the final field
  set.
- Ticket 22 does not move `TaskStatus`. This ticket decides the final place of
  `TaskStatus*` and `TaskListItem`.
- `index` owns `IndexPolicy`. `IndexPolicy::fingerprint()` is the only source of
  the fingerprint. Ticket 22 removed `title`, `aliases` and the date keys from
  the fingerprint, because no production code reads them.
- An index-time read must add its key to `IndexPolicy`.

## Inputs from the IWE and Fileclass research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md) and
[14](../research/14-fileclass-model-binding-and-validation.md).

- IWE stores raw text for each key beside an arena tree of block nodes [V].
- IWE spans are 1-based line ranges. They exist on block nodes only. IWE has no
  byte offsets and no spans on inline nodes [V] (`crates/liwe/src/graph.rs`,
  `model.rs`).
- This is enough for the IWE LSP, which uses full-document sync.
- The IWE writer re-synthesises markdown from the tree
  (`pulldown-cmark-to-cmark`) [V].
- The CHANGELOGs record repeated text-loss regressions [V].
- The CHANGELOGs record two reversals [V]: normalise-on-write overwrote external
  edits (iwe v0.26.0), and a normalise pass touched every file (diwe v0.24.1).
- [H] This supports ticket 17's "no general write-back".
- [H] A future writer must splice spans into the original bytes. It must not
  re-render the tree.
- [H] This does not refute the lossless-tree alternative.
- Fileclass has no body model. It has frontmatter only. It writes through the
  host's `processFrontMatter`. It gives no input.
