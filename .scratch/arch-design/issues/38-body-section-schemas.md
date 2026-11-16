# Should schemas also constrain note body sections, and how?

Type: grilling
Status: open
Blocked by: 13, 15, 16, 17, 20, 22, 23, 24, 25, 28

Every blocker is resolved. Open tickets 31, 33, 34, 35 and 37 use the Answer of
this ticket. They do not block it. See "Order against open tickets".

## Question

[Ticket 20's Answer](./20-schema-note-typing-and-validation.md) limits schema
checks to frontmatter. Decide whether schemas also describe the body. Decide the
body description. Decide where that capability lives.

Decide scope first (decision 1). Decisions 2 to 5, 7 and 10 to 12 cover
architecture. They change contracts that other tickets fix.
Decisions 6, 8 and 9 add schema grammar and product surface.

Open points:

1. Decide the scope.
   - Option a: keep schemas frontmatter-only and record the pressure.
   - Option b: decide only the seams (decisions 2 to 5, 7 and 10 to 12).
     Put the grammar in the fog of the map.
   - Option c: decide the seams and the grammar.
2. Decide which body facts the check reads. Ticket 23 fixed the facts that exist
   (see the inputs below). Decide if the check reads stored headings only, or
   also `NoteDoc` ranges and source text.
3. Decide the type that carries headings into the check. `schema` cannot import
   `note` (ticket 20). Ticket 35 places the type. This ticket names its fields.
4. Decide how the check treats a heading that the parser finds inside a quote or
   a list item. The stored `Heading` has no container flag.
5. Decide the violation locator. Ticket 20 uses a field key. Decide the body
   locator (a heading path?) and the delta key for a body violation. Decide
   what a "missing section" violation points to.
6. Decide the schema format extension: the key name, the strict TOML shape, and
   flat or nested entries.
7. Decide how body rules compose. Cover `extends`, several classes, the `global`
   layer, `$ref` and the class-owned `[binding]` table.
8. Decide whether a heading-only subset is the first step. The subset has
   required headings, order, depth (level), header patterns and occurrence
   bounds. Prove whether stored headings supply all needed facts.
9. Decide whether later content blocks and token budgets are in scope. If yes,
   decide the token counter and the source of block ranges.
10. Decide the interplay with templates. A template creates the body skeleton.
    The schema must not duplicate it or fight it.
11. Decide the reporting of broken body rules. Decide the violation code family.
    Decide which broken rules are schema errors (exit code 2 in ticket 20).
12. Decide whether the index needs a new persisted fact. If yes, name its facet
    and the format epoch bump (tickets 24 and 25).

Record agent needs as design pressure only. Do not decide an agent workflow
here. See "Inputs from the forward-consumer decision".

Facts: `Note` persists headings. A stored heading holds a level, markup-stripped
display text and a start line (`src/note/model.rs`, `src/note/heading.rs`).
`Note` persists list items. A task is a list item (`Note::tasks` filters
`Note::lists`). `Note` does not persist paragraphs, code blocks or tables.
Ticket 23 confirms this. It stores no paragraph, code block or table. `NoteDoc`
holds code ranges but no paragraph or table range. Templates create the body
skeleton. A body schema must not duplicate or fight them. IWE checks body
structure with an ordered, greedy, non-backtracking section matcher and token
budgets (`docs/document-schema.md` in IWE, see the IWE inputs below). mdschema
does something similar. No one read it. [S] marks an unreviewed source claim.
Diagnostics need body locations. Ticket 23 decides how adapters get them.

Alternatives:

- Keep frontmatter-only.
- Decide the seams only and record the grammar as fog.
- Use the heading-only subset.
- Use the full IWE-style sections, blocks and budgets.

Include the keep-as-is option. Keep-as-is means that a `sections` key stays an
unknown-key error in strict TOML. It also means that the `Violation` locator
stays a field key.

The Answer must name these items (map rule): the alternatives, the invariant
owner, what becomes removable, what stays unresolved and the affected tickets.

Evidence the owner needs:

- The scope reading below.
- The cost of a vault-wide body check from stored headings and from `NoteDoc`.
  The `NoteDoc` prototype scan took 29% of parse time [H] (ticket 23). No one
  measured a real builder.
- The effect of an extra heading field on the extractor fingerprint (ticket 25).

Inputs: ticket 20 Answer, `research/20-prep/round4/report.md` and
`F-agent-and-mcp-vault-tools.md`, note document model (23).

## Inputs from the IWE research

See [research 13](../research/13-iwe-query-schema-and-agent-write-gating.md)
(sections 1.3 to 1.4). The `crates/iwe/docs/schema.md` file is stale. Round 4
read it. See the ticket 20 comment of 2026-10-21. Use `docs/document-schema.md`
in IWE and `liwe/src/schema/document.rs`.

Violations locate by header breadcrumb. Token budgets count with `tiktoken_rs`
`o200k_base` (`crates/diwe/src/tokens.rs`) [V]. This is a model-specific
tokenizer and dependency. Research 13 calls the bias unneeded [H]. IWE has no
inheritance. Schemas bind by path glob, so body rules never compose across a
class hierarchy. Traces must decide how body rules compose. IWE composes
overlapping schemas order-free, like `allOf`. IWE has no delta rule. An existing
violation blocks every edit [V].

Ticket 15 found the external crate is a poor fit (second dialect, recursive
tree, no spans). The matcher semantics are reusable as a design over the flat
heading slice (about 150 to 250 lines) [H]. No dependency decision is left for
this ticket.

Ticket 15 says a native matcher "reports the heading line directly". That holds
only for a heading that exists. A missing required heading has no line.

## Inputs from the Fileclass research

See [research 14](../research/14-fileclass-model-binding-and-validation.md).
