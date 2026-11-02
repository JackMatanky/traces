# Should schemas also constrain note body sections, and how?

Type: grilling
Status: open
Blocked by: 20, 23, 13, 15

## Question

[Ticket 20's Answer](./20-schema-note-typing-and-validation.md) limits schema
validation to frontmatter. Decide whether schemas should also describe the body.
Decide the body description: required headings, their order and depth, header
patterns, later content blocks, and token budgets. Decide where that capability
lives.

Facts: `Note` already persists headings (level, text, line), lists and tasks.
`Note` does not persist paragraphs, code blocks and tables (`src/note/model.rs`,
`src/note/heading.rs`). Templates already create the body skeleton, so a body
schema must not duplicate or fight them. IWE validates body structure with an
ordered, greedy, non-backtracking section matcher and token budgets
(`crates/iwe/docs/schema.md`, read in round 4). mdschema does something similar
(not read). Diagnostics need body spans, which ticket 23 decides.

Open points:

- Decide which body facts the note model keeps (23).
- Decide the schema format extension (strict TOML, `sections`?).
- Decide whether a heading-only subset (required headings, order, depth) is the
  first step, because it needs only persisted data.
- Decide the violation locator (a field key in 20, a heading path here).
- Decide the interplay with templates.
- Decide whether agents need it (required sections in agent-written notes).

Alternatives:

- Keep frontmatter-only.
- Use the heading-only subset.
- Use full IWE-style sections, blocks and budgets.

Include the keep-as-is option.

Inputs: ticket 20 Answer, `research/20-prep/round4/report.md` and
`F-agent-and-mcp-vault-tools.md`, note document model (23).

## Inputs from the IWE research

See [research 13](../research/13-iwe-query-schema-and-agent-write-gating.md)
(sections 1.3 to 1.4). IWE validates body schemas through an ordered, greedy,
non-backtracking walk over sections. Per-entry constraints include `const`,
`enum`, `pattern`, length and `maxTokens`. Occurrence bounds include
`minContains` and `maxContains` across the node. The walk supports
`additionalSections` and terminal-only wildcards (a non-terminal wildcard is a
schema error, exit 2) [V: `docs/document-schema.md`].

Violations locate by header breadcrumb. Token budgets count with `tiktoken_rs`
`o200k_base` (`crates/diwe/src/tokens.rs`) [V], a model-specific tokenizer and
dependency. IWE has no inheritance. Schemas bind by path glob, so body rules
never compose across a class hierarchy. Traces must decide how body rules
compose. Fileclass has no body validation. The implementation lives in an
external crate; ticket 15 investigates it.

Ticket 15 found the external crate is a poor fit (second dialect, recursive
tree, no spans). The matcher semantics are reusable as a design over the flat
heading slice (about 150 to 250 lines) [H]. No dependency decision is left for
this ticket.

## Inputs from the note document and index read model decisions

See [ticket 23's Answer](./23-note-document-model.md).

- Section extents come from heading lines. Source slices serve token budgets.
- `NoteDoc` owns every range. The schema check is span-free. Adapters attach spans.
