# Does the schema subsystem validate and type note data, and where does that capability live?

Type: grilling
Status: resolved
Blocked by: 14, 15

## Question

`src/schema` (12k LoC; ~4.7k production) models field definitions and
inheritance but never touches note values; the index extracts `fileClass`
independently. Should schemas drive validation/typing of note frontmatter and
inline fields (advisory diagnostics vs gating vs none), and if so which module
owns the validator, the diagnostic type, and the typed-field representation
shared with templates (`ui` prompts) and query?

Facts: `survey/schema.md` §2-3, `src/index/service.rs:1732-1923` (`mod
class_field`; shifted from 1656-1840 by `207cc3fb`), four field-type enums
(`RawSchemaFieldType`, `SchemaFieldTypeTag`, `SchemaFieldType`,
`NoteFieldType`), `FileClassExpander` seam (judged sound at charting;
overturned, see the Answer), global schema semantics, `SchemaService`
`expect(dead_code)` constructor.

Alternatives: keep schema passive (template/prompt metadata only); add advisory
validation pass producing structured diagnostics; schema-typed ingestion at
index time. Include the keep-as-is option and what it means for 12k lines.

Inputs: `survey/schema.md`, `research/03`, value model and coercion decisions,
compat envelope.

Graduates fog: schema internals simplification (see map).

## Inputs from the value model decision

See [ticket 14's Answer](./14-value-model.md). Schema options use a
schema-local literal type (strings stay strings, no date/duration/link
variants; native TOML dates read as strings). Select values that name links
are interpreted as links only here, at the consumer boundary, with the shared
root `Link` wikilink parser.

## Inputs from the coercion and comparison decision

See [ticket 15's Answer](./15-coercion-and-comparison-policy.md). Frontmatter
classification stays schema-free and quote-insensitive; the schema override that
keeps a date-shaped value as text is decided here. Select strings match note
`Link` values through the adaptive-literal rule (a string adopts the folded
link target), so no separate matching rule is needed. Coercion failures follow
the error-mode principle: advisory, never gating.

## Answer

**Decision:** schemas validate note data as an advisory, on-demand check. They
never type values at parse time, never gate the read or index path, and never
enter the index or its fingerprint. `schema` owns the rule "is this value valid
for this declaration" as a pure, span-free function over a resolved set of
effective fields. `note` keeps classification (tickets 14 and 15). Gating is a
host policy over stable violation codes, not a property of the check.

**Roles of the schema:** (a) value and source provider for templates and LSP
completion; (b) advisory frontmatter checker (LSP diagnostics, post-render
template check); (c) hover and go-to-definition metadata; (d) write-path
guardrail for an MCP agent; (e) schema definitions never affect parsing, the
index or the fingerprint (only the configured class-field name is shared).

**Layering (standing principle, binds tickets 22, 24, 26 and 28):**
consumers (cli, lsp,
mcp) are thin hosts of `core`. `core` holds an app layer of use cases that
composes domain modules (`note`, `schema`, `query`, `index`, `template`) over
foundation types. Domain modules expose data and pure functions; the app layer
composes them. No one-implementor trait exists to let one module call another.
Use cases are functions with explicit inputs, not a held context object.

**The check:**

- `schema` resolves a class set into effective fields (pure function over a
  schema snapshot; hosts replace the snapshot wholesale; no cache is mandated).
  `ResolvedFields::check(&FieldMap) -> Vec<Violation>`, deterministic field
  order, no span, no severity.
- The runtime value type (today `NoteFieldValue`) must sit below both `schema`
  and `note`; schema literals stay a separate type (ticket 14 holds). Its exact
  placement is handed to ticket 28. Select values that name links are parsed
  with the root `Link` wikilink parser at the check (ticket 14) and compared by
  ticket 15's adaptive equality, defined with the value type, not
  re-implemented.
- Kinds from `check`: kind mismatch, not in select set, out of range, `step`
  mismatch, missing required, `multi` arity, and unknown field (always returned
  with an optional `closest` match; hosts filter). Class resolution adds
  unknown class and `schema.class-conflict`; the app layer adds link target
  missing or wrong class (next bullet). Expected sets are bounded (count,
  truncated flag, closest few via `strsim`).
- Tier-2 `file` checks: the check exposes folder, extension and class
  constraints as data; the app layer performs index lookups. No oracle trait.
- `required` is kept: missing means absent, `null`, blank string or empty list;
  `false` and `0` are present. One code, `found` says absent or blank.
- Several classes: inheritance resolves within each class by the existing
  `extends` rule (first-listed parent wins); then the first-listed class wins
  the whole declaration; unknown classes are skipped; one advisory
  `schema.class-conflict` per note and field when types differ.
- Option semantics, with the principle that no declared option is silently
  ignored: `min`/`max` checked; `step` checked (exact for integers, relative
  tolerance for floats); `date.format` is a write-format hint and the check
  accepts Date/DateTime, plus any scalar when the format has no date component;
  that gap stays explicit until ticket 23 fixes the dialect.
- Model additions: `origin`, `description` and `parent_order` now (the schema
  format may still change freely; `description` is a new key accepted by every
  field type). `suggest_class` waits for the LSP.
- The check takes a field map; merging inline fields into it is the app layer's
  business and is not promised here (ticket 19).

**Class hierarchy as data replaces `FileClassExpander`:** `schema` produces a
plain hierarchy keyed by a single normalising `ClassName` (the index stores
lowercase), and the app layer hands it to the query service. Unknown classes
come back as data instead of a logging side effect. Hosts swap the snapshot
wholesale when schemas change. Delete the trait, the crate-root
`src/file_class_expander.rs`, the `&mut ClassExpansionMode` out-parameter and
`Option<Arc<dyn FileClassExpander>>`. Verified smells: single implementation,
consumer-defined trait, out-parameter mutation of the query AST, and the
orphan rule forbidding this impl in any third crate once `query` and `schema`
are separate crates (reproduced: E0117 in a throwaway workspace; a newtype
compiles). A latent hazard also exists: without an expander, store queries fall
back to exact match while in-memory filters match nothing; production always
attaches one, so it is not a current bug. 16 records the decision and the
schema-side producer;
ticket 22 owns the query-side change; the owning module of the hierarchy type
is for tickets 22 and 28.

**Template boundary:** an advisory post-render check in the app layer between
`render()` and `write()`. It parses the rendered frontmatter with the note
parser, resolves the class and runs the same check. It is skipped when the
render has no frontmatter or class, warns on stderr in interactive use, runs
under `--dry-run`, and returns structured output under MCP. Schema-driven
prompts (ADR-0006 keeps the schema as a source of values) and static analysis
of templates (frontmatter is raw text to the template AST) are rejected.

**Agent guardrail:** the LSP is feedback only: `didChange` cannot deny and
`willSaveWaitUntil` cannot abort a save. Layers: constraints given to the agent
up front (MCP tool input schemas, a schema resource; large enums and strftime
formats are not projected and are checked after the call); a blocking check
through one core use case `check_note(before, after)`, exposed as `traces check
--before <file|-> --after <file|->` (git is optional sugar) and as MCP tools;
advisory LSP diagnostics; a commit or CI backstop. Direct file edits get
detection and feedback only, because a pre-write hook sees the edit instruction
and not the resulting file [H, from tool documentation, not tested]. True gating
exists only where the tool owns the write (MCP edit tools). The delta rule
blocks only violations introduced by the edit, keyed `(field, code,
hash(value))`, not by list position. Changing a note's class, or setting an
unknown class, asks the user and denies when no confirmation channel exists.
Host policy maps each code to ignore, warn or reject. The MCP crate renders
JSON Schema and `isError` results; core has no protocol types.

**MCP edits:** how an MCP edit tool applies a change to a note (a lossless
frontmatter edit) belongs to the MCP host, not core. 16 puts no write-back or
span requirement on ticket 19 (ticket 13 holds); if MCP tools need edit
ranges, that is raised when MCP is specified.

**Date-shaped text:** no parse-time override, no quote convention. `input`
accepts any scalar and consumers get a text view rendered from the typed value,
under a tested round-trip invariant (canonical forms round-trip, normalisation
documented; links already normalise display). A `string()` cast arrives only if
a real use appears (ticket 15). A TOML-style single versus double quote
convention is rejected [S]: Noyalib exposes no per-scalar quote style outside
the CST (+59% parse time), and Obsidian's property editor rewrites quote style
on save. Quoted numbers and versions already stay text and leading-zero values
are strings (`src/note/metadata.rs:319`); only date, datetime and
duration-shaped strings are promoted regardless of quotes.

**Alternatives considered:**

- *Keep schema passive:* rejected. It leaves `required`, `multi`, `min`, `max`,
  `step` and `format` inert, blocks tickets 27, 28 and 30 and fails the
  LSP-readiness commitment of ticket 13.
- *Checker in a consumer module above `schema` and `note`* (the earlier
  recommendation): withdrawn. It puts the rule outside its invariant owner and
  repeats the `FileClassExpander` shape.
- *`schema` imports `note`; `note` imports `schema`:* rejected (note must parse
  without schemas, ADR-0009).
- *Small value-view trait, compiled generic validator, merged vault-model
  module:* rejected (need a glue impl or inverted dependency; flat scalar
  schemas do not justify a compiled engine; blast radius).
- *Index-time typing and query-bind typing:* rejected by tickets 12 and 15 and
  ADR-0009.
- *Optional link-oracle trait for tier-2 checks:* withdrawn (one-implementor
  trait).
- *Binding the agent through the LSP:* rejected (advisory by protocol).
- *Default severities in the check:* not adopted; ticket 27 owns severities.

**Invariant owners:** validity of a value for a declaration (`schema`);
classification (note parser, tickets 14 and 15); select and link equality
(value-semantics module, ticket 15); class-name normalisation (one `ClassName`);
class resolution, delta and class-change detection (core app layer); policy per
code (each host).

**Becomes removable:** `FileClassExpander`, `src/file_class_expander.rs`, the
`ClassExpansionMode` out-parameter, the `Option<Arc<dyn>>` field and the unknown
class logging side effect; the test-only `SchemaService::new`; the
`#[cfg(test)]` gating on `min`, `max`, `step` and `format` accessors (they become
production). Not removable: `required`, `multi`, `min`, `max`, `step`, `format`.

**Evidence note:** the author's own vault was treated as one example, not as
authority. Decisions rest on code, ADRs, tickets, protocol facts and external
precedent. Items tagged [S] come from scout reports not re-read; no benchmark
was run. Drift since `207cc3fb` touched `query/service.rs`,
`query/grammar/source.rs`, `template/service.rs` and
`template/engine/schema.rs` only in doc comments and the minijinja per-render
state API used by the template `Schema` handle; the seams cited here (the
`FileClassExpander` trait, the `render()`/`write()` split) still exist.
Research: [brief](../research/16-prep/brief.md) (revision 2 plus MCP section;
its consumer-module recommendation is superseded here) and the round-3 scouts
and [synthesis](../research/16-prep/round3/report.md). Scout E's default
severities and element-indexed delta key are superseded by this Answer.

**Remains unresolved (handed off):**

- Value-type placement below `schema`, hierarchy type module, visibility, crate
  placement of the app layer: [crate topology](./28-crate-topology-and-public-surface.md).
- Query-side change replacing the expander: [query core](./22-query-core-and-plan.md).
- Field-key spans for adapters; inline-field merge:
  [note document model](./19-note-document-model.md).
- `date.format` dialect and the date-format check: [temporal
  boundary](./23-temporal-boundary-and-dialects.md).
- Template service wiring of the post-render check: [template
  boundary](./24-template-engine-boundary.md).
- Diagnostic envelope, severities, per-code overrides and the registry: [error
  and diagnostic architecture](./27-error-and-diagnostic-architecture.md).
- Host composition and use-case layer: [workspace
  host](./26-workspace-host-and-composition.md); open-buffer checks:
  [live buffer](./30-live-buffer-and-persisted-index-consistency.md).
- Re-test of the "provisionally withstands scrutiny" claim for the
  `FileClassExpander` seam (overturned here): [reconciliation](./29-reconcile-decisions-and-audit-coverage.md).
- Schema internals (graduated fog): see ticket 31.
- How MCP edit tools apply note changes, if they need edit ranges: raised when
  MCP is specified (ticket 26 or later).
