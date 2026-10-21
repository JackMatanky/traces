# Pre-grilling brief: ticket 20 (schema note typing and validation), revision 2

Supersedes revision 1. Inputs: `a`..`e` (round 1) plus round 2:
[f-lsp-effort-review](./f-lsp-effort-review.md),
[g-other-efforts-review](./g-other-efforts-review.md),
[h-arch-design-note-split-review](./h-arch-design-note-split-review.md),
[i-template-precheck](./i-template-precheck.md). Tags: [V] verified in a source
I or a scout read, [H] inference, [P] proposal. Nothing here is a decision; it
is the menu for the grilling.

Status: superseded in part by [ticket 20's
Answer](../../issues/20-schema-note-typing-and-validation.md) and the round-3
[synthesis](./round3/report.md). The consumer-module recommendation (V1 in
section 4 and Q1) is withdrawn; the checker lives in `schema`. The author's
vault facts cited here are one example, not evidence.

## 0. What revision 1 got wrong (and what changed)

1. **It missed the prior LSP wayfinder.** `.scratch/md-pkm-lsp/` (resolved, 34
   tickets) already designed this exact capability: ticket 20 specifies a
   `SchemaValidator` (`validate(&Note,&Schema)->Vec<SchemaDiagnostic>`,
   frontmatter-only, severity tiers, `schema.*` codes, hover, completion,
   go-to-definition, `origin`/`description`/`parent_order` on the schema model);
   ticket 19 the frontmatter-span mechanism; ticket 25 the diagnostics envelope;
   tickets 11/39/40/41 the span model. Arch-design ticket 17 names that effort
   as evidence of intent and explicitly **reopens** its "spans on the shared
   `Note`" decision. I read the ticket 17/18/23 headers but not the LSP effort,
   and I let a scout summarise `d-prior-decisions`. That was a review gap.
2. **Wrong framing of the keep-as-is and "span-free kernel" options.** I treated
   the schema as under-used dead weight (K+ pruning) and recommended a span-free
   kernel with "spans belong to ticket 23". Given your framing, the schema is
   *staged infrastructure for the committed LSP* (ticket 17) and for a
   template/guardrail check. Pruning on caller count is the wrong lens (ticket
   18 already rejected "caller counts alone" as grounds to delete capabilities).
   And a span-blind design leaves the LSP's main schema feature (diagnostics on
   frontmatter) without a home.
3. **Ticket 20 and ticket 23 are both on the frontier and coupled both ways.**
   20 needs to say what a diagnostic points at; 23 needs to know whether the
   editor shape must carry frontmatter spans. Round 1 hid that coupling.

## 1. Ground truth (corrected)

**Schema, today** (`a`, spot-checked): consumed in production only for the
`extends` graph (`@class`), `select` lists and `file` sources via
`schema.get(c).field(f)`. `min/max/step`, `format`, `required`, `multi` are
parsed and unread; ADR-0006 says `required`/`multi` are "inert until the
deferred MCP guardrail stage; LSP and MCP stages are deferred"
(`docs/adr/0006:110-111`). No `traces schema` command exists. The indexer stores
the raw lowercased `class` string and expands inheritance at query time, so
schema edits never invalidate the index (ADR-0009:76-77) [keep this].

**Notes, today:** zero byte spans on any `Note` element: frontmatter keys/values
(`src/note/metadata.rs:56-75`), links, tags, inline fields; `Heading`/`ListItem`
have only a 1-indexed `SourceLine`. `ByteSpan` (8 B, serde-derived),
`BytePos(u32)`, `LineIndex` and `Spanned<T>` already exist in `src/position.rs`
(used by query/task lexers, not by `Note`). `Frontmatter::fields()` is dead
outside tests. The whole `Note` is postcard-serialised into redb `NOTES`
(`src/index/codec.rs`). `template` imports no `note` today (grep: none).

**Span history** (all [V] from `f`/`g`/`h`):

| Item | Status |
|---|---|
| md-pkm-lsp 11: spans on the shared AST, no parallel model; transient, never in redb | resolved 2026-09, **reopened by arch 13** |
| md-pkm-lsp 19: frontmatter spans via `noyalib` `Spanned<T>` as a **separate scan output consumed by LSP handlers**, "no new struct on `Frontmatter`" | resolved; contradicts 20 Q7 below |
| md-pkm-lsp 20 Q7: `Frontmatter { fields, #[serde(skip)] spans }` | resolved, then the `serde(skip)` pattern was **withdrawn** as a "type-system lie" (note-parse 04; research 40) |
| md-pkm-lsp 41 / research 40: persist spans in redb (B) vs `Option<ByteSpan>` (C) | **open** |
| arch 17: LSP committed; zero-span design rejected; shared vs separate editor/index representation "neither selected" | resolved; question handed to 23 |
| arch 18: `Link` (target/display/embedded, root) vs `NoteLink` (syntax kind, **span**, editor-side); Noyalib CST rejected (+59% parse, `span_at(path)` unsound for dotted keys/`!!str`/block scalars) | resolved |
| arch 23: byte-span policy, split vs shared | **open, unblocked** |

Technical fact I checked: Noyalib 0.0.53 builds a `(Value, SpanTree)` on every
parse (`parser::parse_exactly_one`, `de.rs:217`), and exposes `Spanned<T>`
(`spanned.rs`). So frontmatter key/value ranges are plausibly obtainable without
the CST (which ticket 18 rejected), at the cost of the `SpanContext` map.
**Not measured** (note-parse 02:50 says the current config uses the span-free
fast path). That one probe would settle ticket 23's frontmatter-span cost
question.

**Prior validator design** (md-pkm-lsp 20, audited in `f` §2). Still sound: pure
`validate` over a parsed note, frontmatter-only first (inline fields deferred:
no spans, multi-valued semantics), advisory, `source: "traces"` + `schema.*`
codes with per-code overrides and an `enabled` gate (ticket 29), schema-TOML
spans forbidden in v1 (go-to-def lands on the file). Stale or contradicted:
`TypeMismatch`/`MissingRequired` as **Error** (ticket 19: advisory); strict
string match for select (ticket 19: adaptive literals, `Link` matches via folded
target); `chrono`-ISO-only date check that ignores `format` (the author's vault
declares strftime `%Y`, `%H:%M`, `%Y-%m-%dT%H:%M`; `temporal-core` also says
strftime core, Moment only at the template filter seam; the schema tests use
Moment `YYYY-MM-DD`: an unresolved dialect conflict, ticket 27); `Number(f64)`
(ticket 18: `Int`/`Float`);
`SchemaValidator`/`SchemaDiagnostic`/`origin`/`description` all `pub` (ticket 16
visibility rule); `serde_yaml` premises. Not built: no `validate.rs`, no
`origin`, no `description`, no `parent_order`, no `Frontmatter.spans`.

**What the LSP effort expects of the schema beyond diagnostics** (`f` §4):
select/file value completion, three-layer field completion (schema > global >
vault-inferred), hover (type, constraints, `description`, origin chain),
go-to-definition to the schema file, typo suggestion (`suggest_field` exists;
`suggest_class` proposed). These are the real reason the dead options
(`min/max/step`, `format`, `required`, `multi`) are worth keeping: hover and
completion read them.

**Author's vault** (`examples/.traces/schemas`): `required` 0 uses; `min/max` 0;
`step` 3; `format` 14 (strftime, incl. `%H:%M`, `%Y`); `multi` 14; select 28,
file 18, external select lists 5 uses (292 to 3,734 entries, one with
`[[target|Display]]` link entries).

## 2. The seam that makes 20 and 23 independent [P]

Three layers; only the middle one is ticket 20's design:

```text
schema data
  (effective fields, origin, description, parent order,
   select values, file source)             --> [kernel]
note frontmatter
  (field map: IndexMap<FieldKey, NoteFieldValue>)
                                           --> [kernel] -> Vec<Violation>
Violation { field: FieldKey, element: Option<usize>, kind,
            expected, found }
  --> adapter -> Diagnostic { code, severity, span?, related }
```

- The kernel reads only the field map, which is the same `NoteFieldValue` map in
  **any** note shape (ticket 18 made it the single runtime value). It never sees
  spans, so it runs identically on an index-shape note, an editor-shape note,
  and freshly rendered template text.
- The adapter is where shapes differ: the editor shape supplies
  `field_span(key)` (and `element_span(key, i)`); the index shape or a
  post-render check yields `span: None` (file/field-level diagnostic).
- This differs from revision 1 in an important way: spans are
  **a requirement 20 places on 23**, not an afterthought. Requirements (R1-R5)
  for whichever editor shape 23 picks: R1 key span and value span per
  frontmatter key; R2 per-element spans for list values (a multi-select with one
  bad element); R3 the frontmatter block base offset; R4 duplicate keys resolve
  as the typed map does (last wins); R5 UTF-8 `ByteSpan` only, conversion to
  protocol positions stays at the boundary (md-pkm-lsp 26's `convert`).
  Inline-field spans are a later requirement (Q20 deferral).
- Shapes for 23 (`h` §3), with what 20 consumes: (A) one shared span-bearing
  `Note`: validator takes `&Note`; costs redb record size and the 18.5 ms
  deserialize path (`research/11`; the "+15-25%" figure in `h` is [H], not
  measured). (B) `NoteFacts` (index, span-free) + `NoteDocument` (editor, spans,
  `NoteLink`) from one parser: validator takes the field map plus spans from
  `NoteDocument`. (C) lossless tree: rejected by research 10/ticket 18. (D)
  shared span-free `Note` + on-demand reparse of the open buffer: frontmatter
  spans re-derived per request, kernel unchanged. Under every shape the kernel
  is the same; only the adapter changes.
- Avoid a trait for the "target" (as `h` proposes, `NoteSourceTarget`): one
  implementor per shape is not a seam yet (Rule of Three); pass the field map
  plus an optional span lookup.

## 3. Template pre-check (`i`, verified against `src/template`)

- Today: `TemplateService::render()` returns
  `RenderedTemplate { content: String, .. }` and `write()` commits it; the seam
  is already documented (`service.rs:126-130`). Rendered text is never parsed
  back; nothing knows its class; `--dry-run` runs prompts and returns the text.
- **S1 post-render check** is the viable form: parse the rendered frontmatter,
  read `class_field`, resolve the schema, run the kernel, report. Near-free (the
  rendered text is 1-5 KB), needs no spans, works for `--dry-run` and
  `--no-input` (checks that defaults satisfy the schema). Needs a
  frontmatter-parse helper callable from `template` (new `template` to `note`
  edge).
- **S2 schema-constrained prompts** conflicts with ADR-0006 ("schema supplies
  values only; the author picks `ui.*`"), the No-Declaration Format, and a
  deliberately minimal `DialogProvider`; it is also what would finally give
  `min/max/step/format` a runtime reader.
- **S3 static analysis** of a template cannot see frontmatter (it is raw text to
  the minijinja AST); md-pkm-lsp 22 limits static analysis to syntax and names.
  Infeasible for schema conformance.
- Policy: advisory to stderr for interactive use (aborting after the user
  answered prompts discards effort); gating is a **host policy**, not a kernel
  property, which is how ADR-0006's MCP guardrail fits ticket 19's advisory
  principle (the MCP host chooses to refuse and return structured violations).
  Precedent: Cookiecutter pre-gen hook gates; Copier validates at the prompt;
  Hugo/Metadata Menu do not gate.
- Reuses the same kernel as the LSP: no second validator.

## 3b. MCP guardrail (added after review)

- Stated intent already exists. ADR-0006 says the schema is shared with
  "future LSP completions/MCP guardrails" (`docs/adr/0006:37-38,94-95`) and that
  `required`/`multi` are inert "until the deferred MCP guardrail stage"
  (`:110-111`). ADR-0001 builds `PresetDialogProvider` for MCP ("AI provides all
  variables upfront", `docs/adr/0001:36-37,51`). Ticket 17: MCP is a likely
  near-term consumer; make its seams easy, do not build it; general note
  write-back is out of scope. So MCP is a documented third consumer and the
  first real consumer of `required`/`multi`.
- MCP spec facts [V, modelcontextprotocol.io spec 2025-11-25, Tools]: a tool
  declares `inputSchema` as JSON Schema (2020-12 by default); servers MUST
  validate tool inputs; validation failures such as "date in wrong format, value
  out of range" are returned as a tool result with `isError: true` and
  actionable text, and clients SHOULD pass that to the model "to enable
  self-correction"; malformed requests are separate protocol errors.
- So a guardrail has two halves. **G1, before the call:** describe what is
  allowed by projecting schema fields into the tool's input schema (select to
  enum, number to min/max, `multi` to array, `required` to required, `format`
  and `description` to hints, `file` to a candidate lookup). This is a
  projection of the same field data hover and completion read, not a validator.
  **G2, after the call:** validate the agent's proposed values and return
  structured violations the agent can act on.
- Consequences for the kernel [P]:
  1. **Gating is legitimate on a write path.** The precedent table in `c` §4
     (dbt contracts, Notion/Airtable API 400s, Cookiecutter pre-gen hook; Astro
     at build) gates where the tool owns the output, and advises on the read
     path. Ticket 19's "advisory, never gating" covers read, index and query.
     Reconcile by keeping the kernel policy-free: it emits violations with
     stable codes, and each host supplies a policy `code -> ignore | warn |
     reject`. Ticket 29's per-code override map is the same mechanism; `reject`
     is a host-side outcome, not a kernel one.
  2. **The input is a proposed field map, not a parsed note.** Values arrive as
     JSON. Option (a): render the proposed frontmatter to YAML, parse it with
     the note parser, then run the kernel, so what is validated is exactly what
     will be re-read (one classification path, consistent with ticket 19).
     Option (b): schema-directed JSON to `NoteFieldValue` coercion, a second
     classifier that contradicts "classified once at parse". Lean (a). Side
     effect on Q4: classification is quote-insensitive, so an agent cannot write
     a text-valued date-shaped `input` field at all; accept any scalar for
     `input`.
  3. **Violations must be machine-actionable and serialisable:** field path,
     stable code, expected (bounded: a 3,734-entry select cannot be echoed back,
     so return the closest few via the existing
     `closest_match`/`suggest_field`), found. The same type feeds the LSP
     adapter; MCP-specific rendering (`isError` text, JSON Schema) stays at the
     MCP boundary (ticket 17 invariant).
  4. **Unknown-field policy differs by host:** Hint in the LSP, possibly reject
     for an agent that should not invent keys. Per-host policy, not a kernel
     flag.
  5. **`file` source membership matters more:** it stops hallucinated or
     out-of-folder links. The MCP host has the index, so the tier-2 oracle is
     justified rather than optional.
  6. **No spans needed.** Field paths suffice, which again argues for a
     span-blind kernel with span-bearing adapters.
  7. **Same call path as the template check:** S1 post-render and G2 are the
     same kernel call; only the host policy differs.
- ADR-0006's "schema supplies values only; the author picks `ui.*`" governs
  interactive prompts. It does not conflict with G1, which targets a tool input
  schema, not a dialog.
- Scope question for the user: what may the agent enter? (i) template variables
  and the rendered note (ADR-0001), (ii) frontmatter of new notes, or (iii)
  edits to existing notes, which ticket 17 puts out of scope.

## 4. Option space, reframed

| Option | Shape | Verdict basis |
|---|---|---|
| **K. Keep passive** | Defer everything to the LSP ticket | Leaves `origin`/`description`/`parent_order`, the kernel and its module undecided, which blocks 31, 32, 34; fails ticket 17's "ready for the LSP" |
| **V1. Kernel in a consumer module** above `schema` and `note` (field-map in, violations out) + adapters | Recommended shape [P] | Real edges: `schema` imports no `note` and vice versa; the model already has a bridge precedent (`file_class_expander.rs`); fits the workspace-split steer; keeps `schema` free of the value enum |
| **V2. Kernel as methods on `SchemaFieldDef`** | `schema` imports `note::NoteFieldValue` | Smallest diff; makes `schema` depend on the note crate in a split |
| **V3. In `note`** | `note` imports schema types | Same, other direction |
| **I. Index-time typing** | Coerce at parse | Contradicts 16 (fresh-parse equivalence), 19 (advisory), `research/03`, ADR-0009 fingerprint scope |
| **Q. Query-bind typing** | Schema as type hints in the planner | Superseded by 19's adaptive literals |
| **W. Guardrail as host policy over V1** | Gate on write (MCP host, optionally template host); G1 projects schema fields into tool input schemas at the MCP boundary | A policy plus a projection, not a separate validator |

Pruning (`K+`) survives only as an *internal* simplification ticket for what the
LSP/guardrail do not need (CSR graph, imperative `parser.rs`, five error enums,
dead `SchemaService::new`), to be sequenced after the seam is decided;
`min/max/step/format/required/multi` stay because hover, completion and the
kernel read them.

## 5. Questions in dependency order

### Q1 (spine). Confirm the roles and the seam

Roles of the schema: (a) author-time value/source provider (templates now, LSP
completion later), (b) advisory frontmatter checker (LSP diagnostics,
post-render template check, optional CLI), (c) metadata for hover/go-to-def, (d)
write-path guardrail for MCP (G1 projection and G2 validation, gating by host
policy), (e) never an ingestion or index input. Options: V1 field-map kernel +
adapters [lean]; V2/V3 as above; K. Stress: do you want a CLI exposure
(`traces check`) decided here? Product features are out of this map's scope; the
kernel is designed so each consumer can exist.

### Q2. Spans requirement and the interface to ticket 23

Accept R1-R5 as the contract on whichever editor shape 23 chooses; the kernel
stays span-blind. Alternatively let 20 commit to shape B (`NoteFacts` +
`NoteDocument`), which pre-empts 23. Lean: state requirements, do not pick the
shape. Ask: is a file-level diagnostic acceptable for the post-render template
check and the CLI (yes in `i`), and does the LSP need element-level spans for
multi-valued fields from day one? Also resolve whether the probe on Noyalib
`Spanned<T>` cost should run before 23.

### Q3. Violation taxonomy (reconciled with 18/19/20)

| Check | Needs | Lean |
|---|---|---|
| kind mismatch (`input` accepts any scalar; `boolean` Bool; `number` Int/Float; `date` Date/DateTime or text per `format`) | value | advisory Warning |
| select membership (value or folded `Link` target; empty list = no constraint per 20 Q6) | list + ticket 19 rule | Warning |
| `multi` (scalar == one-element list; >1 on `multi=false`) | value | Warning |
| `required` / missing | key presence; `null` vs absent differ | decide in Q5 |
| number `min/max` (`step`: undecided meaning) | value | Warning, if kept |
| file link source (`folders`/`ext`/`class`) | link resolution + class lookup (index) | tier 2, via oracle; prior Q15 said type-only |
| unknown field | never an error; 20 Q2 two-tier typo hint | Hint/Information |
| unknown class | exists today (`warn_unknown_classes`) | keep |
Severity vocabulary and overrides are ticket 29's registry model; severities are not "Error" for type problems (ticket 19).

### Q4. The "schema override" for date-shaped text (ticket 19 handoff)

Facts: classification is schema-free and quote-insensitive (`title: 2025-01-15`
is `Date`, `2001` is `Int`, `4h` is `Duration`). Note `h`'s suggestion that a
`type = "string"` field overrides promotion cannot be done at parse without
coupling note to schema; and the schema type is `input`, not `string`. Options:
(a) moot: `input` accepts any scalar, text view at the consumer; (b) explicit
`string()` cast in query/template; (c) Information diagnostic; (d) parse-time
(rejected). Lean (a)+(b). Ask whether any real note needs the original text of a
date-shaped title.

### Q5. Fate of `required`

0 uses in your vault; Metadata Menu has none; ADR-0006 declared it inert until
the MCP guardrail stage; the LSP design wants it. The MCP guardrail is now the
named first consumer, so delete is hard to justify. Options: keep and implement
as `schema.missing-required` (advisory in the LSP, reject in MCP by host
policy); keep inert; delete. Lean keep. Decide `null` vs absent.

### Q6. Meaning of `date.format` and `number.step`

`format` is strftime in your vault and Moment in the schema's own tests and
temporal's template seam. Options: write-format hint only (read by a future
prompt/renderer and by hover); also a read contract (parse the scalar's text
with it; needs lexeme, absent for `Int`/`Date`); delete. Lean: hint, checked
only for kind (+ text parse where the format has no date variant in the note
model). Ticket 27 owns the dialect.

### Q7. Several classes on one note

`class: [a, b]` stores two strings. Effective fields: union by canonical
`FieldKey`, first-listed wins (consistent with `extends`), plus a Hint when
classes disagree on a field's type; matches the LSP effort's "fileClass wins"
matrix (19). Metadata Menu's id-based dedupe is the cautionary tale.

### Q8. Schema model additions the LSP needs

`origin` (provenance), `description`, `parent_order` (resolution chain),
`suggest_class`. These are schema-subsystem changes that 20 can authorise now
(they were designed in md-pkm-lsp 20 Q3/Q8/Q10) subject to visibility
(`pub(crate)`) and manual `PartialEq` caveats. Ask whether to adopt as designed
or revisit (e.g. `origin` as a side map instead of a field).

### Q9. External select values

Capability stays (vault evidence, 3.7k link entries, and the LSP's
completion/validation both read them); implementation simplification is deferred
to the specification. Ask whether those lists are also notes (then a `file`
field could replace them).

### Q10. Template boundary and graduated internals

S1 post-render check as consumer of the kernel; ADR-0006 values-only stance
unchanged (S2 out); `Schema` handle contract to 28. Internals to sequence after
the seam: collapse `RawSchemaFieldType`/`SchemaFieldTypeTag`; delete
`SchemaService::new`; error enums with ticket 31 (authoring errors fail/degrade
at load; note violations never mix); CSR and `parser.rs` untouched by any
option.

### Q11. MCP guardrail requirements on 20

Decide what 20 guarantees so ticket 17's "make MCP seams easy" holds without
building MCP: (1) a policy-free kernel with stable codes and a host policy map
(`ignore | warn | reject`); (2) validate by render-and-reparse (option a above)
rather than a second classifier; (3) violations carry bounded `expected` and a
suggestion; (4) schema field data is available as a neutral descriptor
(kind, allowed values source, bounds, `multi`, `required`, `description`) so
hover, completion and an MCP input-schema projection read one thing; JSON Schema
rendering lives at the MCP boundary. Ask: scope of what the agent may enter
(template variables, new-note frontmatter, existing-note edits out of scope),
and whether unknown fields should reject for agents.

## 6. Handoffs 20 must make explicit

| To | 20 pins down |
|---|---|
| 23 | R1-R5 span requirements; kernel span-blind; no schema input to parsing; frontmatter lexeme only if Q6 picks a read contract |
| 24/25 | Schema never enters the index or fingerprint; class stays a raw string |
| 26 | No schema-informed query binder; select/link equality reuses ticket 19 |
| 27 | `date.format` meaning and dialect |
| 28 | S1 check wiring; `Schema` handle; ADR-0006 |
| 29 | External select survives or not |
| 30 | Who composes `SchemaService` + note parse + kernel for each consumer |
| 31 | Violation to Diagnostic envelope: reuse md-pkm-lsp 25 Scheme C (`source "traces"`, `schema.*`, registry, emission gate) or revisit |
| 32 | Kernel module sits above `schema` and `note` |
| 34 | Kernel runs on an in-memory parsed buffer without persistence |
| 30/32 (MCP) | Policy map and host-supplied reject outcome live at the host; kernel crate has no MCP or protocol types; descriptor and violation types serialisable |

## 7. Proposed grilling order

1. Q1 roles (LSP, template check, MCP guardrail) and the seam (and what to adopt
   wholesale from md-pkm-lsp 20/25).
2. Q2 spans requirements, handoff to 23.
3. Q3 taxonomy and severities with Q5 `required`.
4. Q4 date-as-text, Q6 `format`/`step`.
5. Q7 multi-class, Q8 schema model additions.
6. Q9 external select, Q10 template boundary and internals.

## 8. Not verified

- Noyalib `Spanned<T>` cost/behaviour for top-level frontmatter keys
  (unmeasured; the one probe that would settle 19's cost question).
- The "+15-25% record size" and "-10-20% deserialize" numbers in `h` are
  estimates; `research/11` gives only the 18.48 ms / 97.3% breakdown.
- Whether `survey/cross-cutting.md`'s tier numbers are exact (the edges are
  verified: no `schema`<->`note` imports).
- Any validation cost; no benchmark exists.
- `i`'s claim that S3 is "fatally flawed" is an argument, not a probe.
- Round-1 files `a`..`e` still contain the errors listed in revision 1's section
  2 (serde_yaml, SQLite, `types.rs`, "-3.5k LoC"); treat `brief.md`, `f`..`i` as
  the corrected view.
- MCP facts come from the 2025-11-25 spec page only; no MCP SDK or server was
  inspected, and no Traces MCP design exists in the repo beyond ADR mentions.
- Whether MCP hosts will be allowed to call the kernel on a proposed field map
  before writing (render-and-reparse needs the note parser and a YAML writer in
  the guardrail path; the YAML writer is not located or verified).
