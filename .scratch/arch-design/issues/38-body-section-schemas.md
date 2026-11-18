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

Decisions: Use
[ticket 36's decision-record rules](./36-reconcile-decisions-and-audit-coverage.md#decision-records-and-evidence-gates).
Keep decision evidence separate from implementation gates.

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

## Scope reading

Tag: [P] proposal.

The map ends at a redesign specification. It keeps new product features out of
the destination. Agent workflows around the check are also product features
outside the destination. The map records only their pressure on the core.

Body schemas have an architecture part and a product part.

- Architecture: the violation locator, the type that carries headings into the
  check, the composition rule across classes, and the proof that the index stays
  unchanged. Tickets 20, 28, 33, 34 and 35 freeze records that these items
  touch.
- Product: the section grammar, header patterns, occurrence bounds, block
  rules, token budgets and the tokenizer. No resolved ticket commits these
  features.

[P] Recommend option b. Reason: only the seams cost more to change later, and
the grammar is additive in a Migratable format (ticket 16). Option a stays valid
if the owner has no use for body checks.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). Ticket 20
hands "body-section schemas (required headings, order, depth, later blocks and
token budgets)" to this ticket. It keeps the violation locator open to more than
a frontmatter key.

- The check is advisory. It never gates reads or indexing. Each host sets policy
  over stable violation codes.
- `ResolvedFields::check(&FieldMap) -> Vec<Violation>` is pure and span-free. It
  returns violations in deterministic field order. It has no severity.
- A violation carries a layer (`class` or `global`), an `origin` and a `hint`.
- Schemas never enter the index or its fingerprint.
- The effective field accessor (ticket 23) does not feed the check. The check
  stays frontmatter-only.
- `check_note(before, after)` takes a path and content on each side. It already
  carries the whole body. Body checks need no new content transport. Their input
  type, locator and violation key remain open.
- The delta rule blocks only violations that an edit introduces. The delta key
  is `(field, code, hash(value))`, not a list position. A body violation needs a
  key of the same kind.
- `traces check` uses separate exit codes for a broken schema and for note
  violations.
- The template check runs between `render()` and `write()`. It parses the
  rendered frontmatter, resolves the class and runs the same check. It skips the
  check if the render has no frontmatter or class.
- Class binding by `[binding]` (`tags`, `paths`) is strict and not inherited.
  The `global` layer is the lowest-precedence layer for every note.
- The MCP write tools must cover every edit operation, including the body. If
  they do not, the host must deny native writes. This is design pressure only.
- Round 4 ([report](../research/20-prep/round4/report.md), section 4) says: "If
  body schemas ever arrive, `Violation.field` needs a locator that is not only a
  frontmatter key."
- Round 4 also says that IWE and mdschema validate body structure. The mdschema
  claim is [S].

## Inputs from the compatibility envelope

See [ticket 16's Answer](./16-grilling-compatibility-envelope.md).

- The `config.toml` and schema TOML formats are Migratable. A new schema key is
  a change-table row, not a regression.
- A schema without the new key must behave as before. Prove that property for
  each selected extension.
- Diagnostic codes are Migratable. Record the selected body-check capability and
  its observable changes in the specification change table.
- The schema TOML format uses `#[serde(deny_unknown_fields)]`
  (`src/schema/raw.rs`) [V]. A binary without the key rejects a file that has
  it. The user is the only consumer, so no external reader exists.

## Inputs from the forward-consumer decision

See [ticket 17's
Answer](./17-grilling-forward-consumers-and-design-pressure.md).

- The LSP is the next committed consumer. Support editor features that need
  source positions, including diagnostics. Body diagnostics fall in this class.
- MCP is likely, not committed. Do not build MCP seams for the body check
  speculatively.
- General note write-back is not a requirement. A body schema must not need it.
- The map keeps agent workflows outside the destination (see "Not yet specified"
  in the map). Items such as "required sections in agent-written notes" are
  design pressure only. The core already supports them through `check_note`.

## Inputs from the config ownership decision

See [ticket 22's Answer](./22-config-ownership-and-lifecycle.md), "Handoffs".
Ticket 22 hands this ticket one item.

- The schema files own class `[binding]` (ticket 20). Ticket 22 adds no config
  key for it. `FieldName` stays a leaf.
- [P] Put body rules in schema files, not `config.toml`. Ticket 22 does not
  decide a body grammar. A heading matcher must not reuse `FieldName`.
- `IndexPolicy` does not change. The index reads no schema (ticket 20).

## Inputs from the note document and index read model decisions

See [ticket 23's Answer](./23-note-document-model.md) and [ticket 24's
Answer](./24-index-read-model-and-query-execution.md).

State these as facts, not as open questions.

- `Note` stays the one persisted facts type. It has no byte span. It keeps
  headings (level, text, start line), list items, links, tags and fields.
- `Note` stores no paragraph, code block or table. Ticket 23 decided this for
  this ticket.
- `NoteDoc` is a flat set of ranges plus a line index. The index never stores
  it. It holds heading ranges and section extents, code ranges, item spans,
  callouts, footnotes and frontmatter key spans. It holds no paragraph or table
  range.
- Ticket 23 says: "Ticket 38 reads section extents from the heading lines. It
  reads text slices from the source." The code delta row says: "Compute section
  extents from heading lines for token budgets and body checks."
- `NoteDoc` builds from bytes in hand (`NoteDoc::scan`, `parse_with_doc`). A
  check over many closed files must read each file.
- The schema check is span-free. `NoteDoc` owns every range. Adapters attach
  spans.
- A stored position never drives an edit.
- The store keeps 11 facets, including `headings`. Facets are independent. A
  query loads only the facets that it needs.
- The parser has no heading slug. A slug derives from heading text when needed.
  Duplicates get `-1`, `-2`.
- A change of the stored `Note` changes the extractor fingerprint or bumps the
  format epoch (ticket 25).

Facts from the code that ticket 23 does not mention:

- `collect_headings` in `src/note/parser.rs` keeps headings inside block quotes
  and list items. The parser test
  `collects_headings_inside_blockquotes_and_list_items_without_losing_metadata`
  pins this [V].
- Ticket 23 and ticket 24 say that a heading inside a quote or a list does not
  count (for the heading above a task). The stored `Heading` has no field that
  tells the two cases apart [V].
- The stored heading text holds only text and code events. Math is dropped. A
  heading that holds only math has the text `""`. An empty heading stays stored
  [V: `src/note/parser.rs`].

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- The extractor fingerprint hashes the encoded bytes of one probe record for
  each facet, built from an embedded corpus. Any new stored heading field
  changes the fingerprint.
- A test fails until a person runs `mise run index:bless`. That task adds one to
  the format epoch. A lower epoch rebuilds the store. A higher epoch is a read
  miss with one notice.
- Schema files stay outside the store identity. A schema change does not rebuild
  the store.
- A heading-only check that reads the stored `headings` facet adds no fact. It
  needs no epoch bump.

## Inputs from the live-buffer decision

See [ticket 28's Answer](./28-live-buffer-and-persisted-index-consistency.md),
"Group 8".

- `check_note(before, after)` runs on the debounced parse output of the buffer.
  It never runs on each keystroke.
- The host maps violations to protocol diagnostics. Core has no protocol type.
- Diagnostics are feedback only. `didChange` cannot deny an edit.
- An open buffer yields a `Note` and a `NoteDoc` from `parse_with_doc`. A body
  check on a buffer needs no persisted data.
- Ticket 28 does not name body checks. The debounce number belongs to ticket 33.

## Inputs from the query core decision

See [ticket 29's Answer](./29-query-core-and-plan.md).

- The query core has no heading predicate or heading source. Do not add a
  heading source for body schemas.
- A report such as "notes that lack a required section" is a check result, not a
  query.
- Ticket 29 is not a blocker.

## Inputs from the open tickets

These tickets are open. Each consumes the Answer of this ticket.

- [Ticket 31, template boundary](./31-template-engine-boundary.md): ticket 31
  decides the wiring of the post-render check. A body check in that use case
  needs the input and locator decisions above. ADR-0006 keeps the schema as a
  source of values, not of prompts. Ticket 20 rejects schema-driven prompts and
  static template analysis. Do not infer a schema-to-skeleton feature from those
  capabilities.
- [Ticket 33, workspace host](./33-workspace-host-and-composition.md): the host
  keeps a compact heading range index. It keeps a document only for an open
  buffer. The change-set check (`check_note` over a change set with a base)
  takes content, so the body is already in it.
- [Ticket 34, diagnostics](./34-error-and-diagnostic-architecture.md): ticket 34
  owns the envelope, severities and the registry. It has no body or locator
  input today. Its code families are `schema.global.*` and the class codes.
- [Ticket 35, crate topology](./35-crate-topology-and-public-surface.md): ticket
  35 places the heading input type that decision 3 names.
- [Ticket 37, schema internals](./37-schema-internals-simplification.md): ticket
  37 decides imperative option parsing against serde, the error enums and the
  class hierarchy. A new `sections` key changes all three. Ticket 37 does not
  list this ticket.

Order against open tickets: resolve this ticket before ticket 34, or let ticket
34 absorb the locator decision. Ticket 34 is not blocked by this ticket today.

## Inputs from the IWE research

See [research 13](../research/13-iwe-query-schema-and-agent-write-gating.md)
(sections 1.3 to 1.4). The `crates/iwe/docs/schema.md` file is stale. Round 4
read it. See the ticket 20 comment of 2026-10-21. Use `docs/document-schema.md`
in IWE and `liwe/src/schema/document.rs`.

IWE validates body schemas through an ordered, greedy, non-backtracking walk
over sections. Per-entry constraints include `const`, `enum`, `pattern`, length
and `maxTokens`. Occurrence bounds include `minContains` and `maxContains`
across the node. The walk supports `additionalSections` and terminal-only
wildcards (a non-terminal wildcard is a schema error, exit 2) [V:
`docs/document-schema.md`]. Two entries with the same match identity are a
load-time schema error [V].

Violations locate by header breadcrumb. Token budgets count with `tiktoken_rs`
`o200k_base` (`crates/diwe/src/tokens.rs`) [V]. This is a model-specific
tokenizer and dependency. Research 13 calls the bias unneeded [H]. IWE has no
inheritance. Schemas bind by path glob, so body rules never compose across a
class hierarchy. Traces must decide how body rules compose. IWE composes
overlapping schemas order-free, like `allOf`. IWE has no delta rule. An existing
violation blocks every edit [V].

Ticket 15 found that the external crate is a poor fit. It has a second dialect,
a recursive tree and no spans. The matcher semantics are reusable as a design
over the flat heading slice (about 150 to 250 lines) [H]. No dependency decision
is left for this ticket. `regex` is already a dependency (`Cargo.toml`) [V]. A
header pattern needs no new crate.

Ticket 15 says a native matcher "reports the heading line directly". That holds
only for a heading that exists. A missing required heading has no line.

## Inputs from the Fileclass research

See [research 14](../research/14-fileclass-model-binding-and-validation.md).

- Fileclass has no body validation. It validates frontmatter only. It writes
  through `processFrontMatter`.
- Research 14 says "Traces requires body section schemas (ticket 38) for
  structured notes" [H]. This is a hypothesis, not evidence. Decision 1 stays
  open.

## Inputs from the agent-tool research

See `research/20-prep/round4/F-agent-and-mcp-vault-tools.md` (section 2.3).

- The Obsidian Local REST API patches one heading, block or frontmatter field.
  Whole-file writes by agents caused data loss there [V].
- This supports section-level patch tools in a future MCP host. It is product
  surface outside the destination. It adds no requirement to this ticket.

## Checks before resolution

- Keep the existing frontmatter-only contract unless the user accepts an
  extension. Do not adopt the seams-only proposal through this review.
- Record each earlier rule that the selected extension changes. Give ticket 36
  the exact supersession list.
- If the user selects the seams only, name the remaining body-check decisions.
  Do not call an unfinished grammar ready for implementation.
- Give tickets 31 and 33 the check input and scheduling requirements. Do not
  wait for a numeric debounce interval to define a logical locator.
- Give ticket 34 the locator and body-code requirements before it fixes the
  diagnostic envelope. Give tickets 35 and 37 the type and schema changes.
- Keep a missing-section locator valid without inventing a source range. Let an
  adapter choose a file-level or nearby source location.
- Check repeated heading text, duplicate sections, nested headings and headings
  inside quotes or list items. Keep deterministic violation order.
- Check the delta key when a section moves or its body text changes. Distinguish
  an existing violation from a new violation.
- Check class inheritance, several classes and the `global` layer without
  silently applying field precedence to section order.
- State whether the extension changes ticket 20's post-render skip rule for
  missing frontmatter or classes. That rule currently remains in force.
- Keep byte spans in `NoteDoc`. Keep stored facts and the schema checker
  span-free.
- Prove an unchanged extractor fingerprint if the extension adds no stored fact.
  Otherwise name the epoch change and corpus case.

Tickets 34 and 37 do not list this ticket as a blocker. Their owners must
consume its Answer before fixing affected contracts, or record the exclusion.
