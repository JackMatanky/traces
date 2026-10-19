# What is the note document model: spans, facts, items, multi-value metadata?

Type: grilling
Status: open
Blocked by: 14, 10, 13

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

