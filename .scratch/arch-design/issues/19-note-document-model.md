# What is the note document model: spans, facts, items, multi-value metadata?

Type: grilling
Status: open
Blocked by: 14, 10, 13, 33

## Question

Decide the shape of the parsed `Note` as the unit consumed by
index/query/template and (maybe) edited: byte-span policy; whether list/task
items remain children of `Note` or become separate indexed entities; the
duplication of item fields/tags into note-level collections; multi-valued inline
field semantics (store-all-read-first today); per-task status representation
(owned String vs symbol/id into the status map); single vs dual markdown pass;
how footnotes/callouts/slugs (claimed by the glossary, absent in code) enter the
model; infallibility contract retained.

Facts: `survey/note.md` (all §3), `src/note/model.rs:24-33`,
`src/note/parser.rs:108,121,488-498`, `src/note/lists.rs:41-50`, measured sizes
(`research/11`), judged-sound pieces (MarkerAccumulator, Logos single pass, flat
list hierarchy, infallible parser).

Alternatives: keep zero-span read-mostly model; add compact spans; separate
"facts table" model; lossless tree (expected to be rejected — attempt to refute
that).

Inputs: `research/10`, `research/11`, value model, forward-consumer decision.

Graduates fog: note parser micro-optimisations (status interning, second pass).

## Inputs from the value model decision

See [ticket 14's Answer](./14-value-model.md). The note type may split into an
indexing/query shape and an editor shape. The root `Link` carries `target`,
`display` and `embedded`; `NoteLink` (syntax kind and source span) is the
editor-side occurrence type and is defined here. Runtime values are
`note::NoteFieldValue` with `Int`/`Float`.

## Inputs from the coercion and comparison decision

See [ticket 15's Answer](./15-coercion-and-comparison-policy.md). Inline fields
may now yield `DateTime` (civil when naive). Classification stays at parse and
is never re-sniffed; decide how multi-valued inline fields classify element by
element. Parse-time `NaN` becomes `Null` with a warning. `Link` identity is the
folded target, with `display` and `embedded` as attributes.

## Inputs from the schema typing decision

See [ticket 16's Answer](./16-schema-note-typing-and-validation.md). The schema
check is span-blind and takes a field map; adapters need a span per field key
(and per list element where known) from whichever editor shape is chosen.
Merging inline fields into that map is app-layer business and is not promised
by 16. An MCP agent that edits files directly is checked post hoc; it adds no
write-back requirement here.

Ticket 32 (body-section schemas) is blocked by this ticket: it needs to know
which body facts and spans the note model keeps. Class binding reads a note's
tags and path, both already available.

## Inputs from the IWE and Fileclass research

See [research 33](../research/33-iwe-core-and-hosts-architecture.md) and
[35](../research/35-fileclass-model-binding-and-validation.md). IWE stores raw
text per key beside an arena tree of block nodes; spans are 1-based line ranges
per block node only, with no byte offsets and none on inlines
(`crates/liwe/src/graph.rs`, `model.rs`) [V], enough for its LSP, which uses
full-document sync. Its writer re-synthesises markdown from the tree
(`pulldown-cmark-to-cmark`); the CHANGELOGs record repeated text-loss
regressions and two reversals: normalise-on-write clobbering external edits
(iwe v0.26.0) and a normalise pass touching every file (diwe v0.24.1) [V].
[H] This supports ticket 13's "no general write-back" and says any future writer
must splice spans into the original bytes instead of re-rendering; it does not
by itself refute the lossless-tree alternative. Fileclass has no body model
(frontmatter only, written through the host's `processFrontMatter`) and gives no
input.
