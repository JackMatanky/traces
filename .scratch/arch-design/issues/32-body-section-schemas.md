# Should schemas also constrain note body sections, and how?

Type: grilling
Status: open
Blocked by: 16, 19

## Question

[Ticket 16's Answer](./16-schema-note-typing-and-validation.md) limits schema
validation to frontmatter. Decide whether schemas should also describe the
body: required headings, their order and depth, header patterns, and later
content blocks and token budgets, and where that capability lives.

Facts: headings (level, text, line), lists and tasks are already persisted in
`Note`; paragraphs, code blocks and tables are not (`src/note/model.rs`,
`src/note/heading.rs`). Templates already create the body skeleton, so a body
schema must not duplicate or fight them. IWE validates body structure with an
ordered, greedy, non-backtracking section matcher and token budgets
(`crates/iwe/docs/schema.md`, read in round 4); mdschema does something similar
(not read). Diagnostics need body spans, which ticket 19 decides.

Open points: which body facts the note model keeps (19); the schema format
extension (strict TOML, `sections`?); whether a heading-only subset (required
headings, order, depth) is the first step, since it needs only persisted data;
the violation locator (a field key in 16, a heading path here); interplay with
templates; whether agents need it (required sections in agent-written notes).

Alternatives: keep frontmatter-only; heading-only subset; full IWE-style
sections, blocks and budgets. Include the keep-as-is option.

Inputs: ticket 16 Answer, `research/16-prep/round4/report.md` and
`F-agent-and-mcp-vault-tools.md`, note document model (19).
