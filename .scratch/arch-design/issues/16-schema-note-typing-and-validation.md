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
for this declaration" as a pure, span-free function over resolved effective
fields. `note` keeps classification (tickets 14 and 15). Gating is a host
policy over stable violation codes, not a property of the check. Which schemas
apply to a note is decided by a class-binding resolver (frontmatter class, tag
and path bindings) plus a `global` layer, both specified below.

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
  with an optional `closest` match; hosts filter). Each violation carries its
  layer (`class` or `global`), its `origin` and a `hint` taken from the field's
  `description`, so an agent can self-correct. Class resolution adds unknown
  class and `schema.class-conflict`; the app layer adds link target missing or
  wrong class (next bullet). Expected sets are bounded (count, truncated flag,
  closest few via `strsim`).
- Tier-2 `file` checks: the check exposes folder, extension and class
  constraints as data; the app layer performs index lookups. No oracle trait.
- `required` is kept: missing means absent, `null`, blank string or empty list;
  `false` and `0` are present. One code, `found` says absent or blank.
- Several classes: inheritance resolves within each class by the existing
  `extends` rule (multiple parents are kept; first-listed parent wins, a child
  redefinition wins over its parents). The resolved class set is reduced to its
  most-derived members, so an ancestor never conflicts with its own
  descendant. Between unrelated classes the rule is order-independent:
  identical declarations (for example through a shared `$ref`) are not a
  conflict; differing declarations produce one `schema.class-conflict` per note
  and class set, and value checks on that field are skipped, while `required`
  and `multi` arity are still checked if either class declares them. Sibling
  parents that collide with no child redefinition are reported the same way.
  Unknown classes are skipped. First-listed wins does not decide class-versus-
  class conflicts.
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

**Global layer:** the reserved `global` schema stays a `$ref` pool and also
becomes the lowest-precedence layer applied to the frontmatter of every note,
classed or not. It is not a class (no `@global` query, never extended); a class
declaration of the same field always wins; `required` in `global` still degrades
to `false`; a class may opt out of a global field with `excludes`. Violations
from this layer carry layer `global`, and the diagnostic adapter derives a
distinct code family `schema.global.*`, so hosts can apply a different policy
(for example hint-level in the LSP, ignored on unclassed notes). Default
severities belong to ticket 27, which is asked to rank the `global` family
below class violations. This changes the documented invariant in
`src/schema/mod.rs:48-51`; the doc comment and the survey are updated with the
specification.

**Class binding:** a note's class set has three sources: its frontmatter class
field, `tags` (nested tags count, as in the existing `Tag` source atom) and
`paths` (vault-relative globs in the existing glob dialect, with `!` exclusion
patterns) declared in a strict, non-inherited `[binding]` table on the class
schema. Bookmarks and Base views are not sources. Rule: a note with one or more
known explicit classes uses only those; bindings are defaults for notes
without a known explicit class (an empty or unknown class value does not
suppress them, so a typo does not silently drop validation). Expansion happens
at query time into the existing `Class`, `Tag` and `Path` atoms plus a
"has a known explicit class" exclusion, so the index and its fingerprint never
learn about bindings. One resolver answers both "which classes does note n
have" and "which notes does class c claim" (with hierarchy expansion for
`@book*`); equivalence between the two is a tested invariant, stated on the
unreduced set. A move or a binding-tag edit changes the class set and is a
class change (see the guardrail). Query cost of path bindings (the `Path` atom
scans all paths today) is for ticket 22 to optimise and benchmark.

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

**Agent guardrail:** no design depends on harness hooks (several harnesses
have none, and each defines its own). The portable guarantee is detect and
feed back; prevention exists only where `traces` owns the write or the harness
is configured to deny native writes. One contract, `check_note(before, after)`
where each side is a path plus content, is exposed as `traces check` (paths,
`--staged`, or `--before <file|->` `--after <file|->` with
`--before-path`/`--after-path` for stdin; git optional) and as MCP tools.
Channels by what the harness provides [P, `research/16-prep/round5/report.md`]:

- *Nothing:* an `AGENTS.md` snippet generated by `traces` (soft) and the exit
  code.
- *A shell or post-edit command:* a lint loop (Aider `--lint-cmd` [V]), a git
  pre-commit on staged blobs, CI.
- *A shell:* validated CLI write commands (`traces new`, a field-set command)
  sharing the MCP edit use case, so any agent with a shell has a gated path;
  it is a path, not a barrier.
- *Any `traces` call:* a bounded notice of notes changed since the last check
  that gained violations. It needs a check watermark and the violation keys at
  the last check as the `before`; a second derived store outside the index and
  fingerprint, with unmeasured cost (tickets 20, 21, 27). Without it, report
  all violations of changed notes, no delta.
- *An MCP client:* validated edit and create tools, the only gate, plus a
  dry-run `check_note` tool taking a path and proposed content. A gate is as
  strong as its weakest open write path: the tools must cover every edit
  operation, body included, or native writes must be denied, otherwise agents
  fall back to native edit tools that can change frontmatter.
- *An LSP client:* diagnostics; feedback only (`didChange` cannot deny,
  `willSaveWaitUntil` cannot abort a save).
- *Hooks, sandboxes, deny rules:* documented recipes, not features. Claude
  Code's pre-write hook does carry the full `content` for whole-file writes
  [V], so the earlier claim that hooks never see the resulting file is
  withdrawn; shell writes remain invisible to hooks.
- *Placement:* the only harness-independent prevention is to keep the writer
  away from the canonical store, which is a workflow, not a filter [H;
  consistent with every surveyed tool, none of which prevents raw writes]. An
  isolated working copy (git worktree or branch; APFS/Btrfs clone elsewhere)
  or an `_inbox/` folder, promoted by a gated `traces merge` or `promote` that
  runs `check_note` on the destination path (bindings need it). It holds only
  where the writer is launched inside the copy; same-user processes can still
  reach the canonical vault. A product feature outside the map's destination;
  the design pressure on core is that `check_note` must run over a change set
  with a base to detect concurrent edits.
- *Instruction files:* a per-folder `AGENTS.md` snippet generated from the
  `[binding].paths` of each class steers writers where they work (loaded
  lazily by Claude Code [V], others unknown [S]); soft, never relied on.

Constraints given up front: MCP tool input schemas and a schema resource (large
enums and strftime formats are not projected and are checked after the call).
The delta rule blocks only violations introduced by the edit, keyed `(field,
code, hash(value))`, not by list position. A change of the class set, including
one caused by a move or an edit of a binding tag, asks the user and denies when
no confirmation channel exists; so does setting an unknown class. Host policy
maps each code to ignore, warn or reject. The MCP crate renders JSON Schema and
`isError` results; core has no protocol types. `traces check` exits with a
distinct code for a broken schema (it never reads as "notes are fine") and
another for note violations. Notes with no class, no binding and no global
field are unvalidated; MCP create tools should require a class or a bound
location. Under `--dry-run` the template check has no output path, so path
bindings do not apply there. Rejected as a design basis or default: hook-based
gating; a staging directory without a launch convention (an instruction); a
watcher daemon (no channel to the agent); auto-revert (destroys data); ACP,
FUSE or WebDAV interception (custom client, kernel extension, fragile mounts);
git clean filters (break `checkout` and `diff`); `reference-transaction` hooks
(see commit hashes only); YAML-comment or `$schema` hints in notes (Obsidian
strips comments [V]; `$schema` pollutes every note). A loopback NFS mount
gates atomic-rename writers on macOS without root [V, one experiment in
round 6] but not in-place writes (a shell `>` exits 0 while the server
rejects), is bypassed through the backing directory, and hangs processes if
the daemon dies; at most a documented recipe.

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
- *Last-listed wins (Fileclass), first-listed wins, conjunction of all classes
  (IWE, JSON Schema `allOf`), single class only:* order-dependent rules were
  rejected as the cross-class rule (reordering a class list must not flip
  results, nothing is masked silently); single-class only is rejected because
  Fileclass, Metadata Menu, IWE and Logseq all support several [V].
- *JSON Schema as the schema format:* rejected (IWE uses it for validation
  only; Traces schemas also drive prompts, completion, file links and
  label/value mapping); JSON Schema remains an export for MCP.
- *Binding only by frontmatter class* (the first Answer) and *binding stored in
  the index:* rejected; binding by tag and path is conventional in Metadata
  Menu, Fileclass and IWE, and query-time expansion keeps the index
  schema-free. Union and priority-ordered combination of explicit and bound
  classes (Metadata Menu, Fileclass, IWE) were considered; suppression was
  chosen because bindings exist to cover notes without a class, it creates the
  fewest false conflicts and it keeps one simple rule. Fallback if the query
  exclusion proves costly: union with source priority.
- *Global as a reference pool only, or applied only to classed notes:*
  rejected; applying it to every note types shared fields like `due` in
  unclassed notes without a binding mechanism.

**Invariant owners:** validity of a value for a declaration (`schema`);
classification (note parser, tickets 14 and 15); select and link equality
(value-semantics module, ticket 15); class-name normalisation (one `ClassName`);
class binding resolution, class-change detection and the delta (core app
layer); policy per code (each host); the `global` layer's precedence (`schema`).

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
its consumer-module recommendation is superseded here), the round-3
[synthesis](../research/16-prep/round3/report.md) and the round-4
[external precedent](../research/16-prep/round4/report.md). Verified in round
4: Fileclass last-wins and class binding sources, Metadata Menu's combined
mapping, Logseq multi-parent `extends`, IWE's design, Umbraco's rejection of
conflicting compositions. Corrected: the earlier attribution of first-listed
wins to Metadata Menu was wrong, and the arXiv paper cited for "agents ignore
advisory warnings" does not show that, so the guardrail rests on protocol
arguments and not on that paper. Scout E's default severities and
element-indexed delta key are superseded by this Answer.
Round 5 and 6 ([claim checks and guardrail channels](../research/16-prep/round5/report.md),
[harness-independent guardrails](../research/16-prep/round6/report.md)) read
the remaining scout claims at the source (LDAP, CUE, Kubernetes pruning, Plone,
Tana, SilverBullet, Metadata Enforcer, Obsidian rewriting verified; Dendron
"fatal" path binding, Logseq "broken trees" and Letta/Graphiti delta
validation refuted), withdrew the hook-payload claim, and found no filter-level
prevention that is both preventive and portable. Per-harness capability cells
are unaudited beyond Claude Code and Aider.

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
- Body-section schemas (required headings, order, depth, later blocks and
  token budgets): [ticket 32](./32-body-section-schemas.md). 16 keeps the
  violation locator open to more than a frontmatter key.
- Query-side expansion of bindings, the "has a known explicit class" atom and
  the path-scan optimisation: [query core](./22-query-core-and-plan.md).
- Rank of the `schema.global.*` family and the schema-error exit code:
  [diagnostics](./27-error-and-diagnostic-architecture.md).
- `[binding]` table and `global` semantics in the schema format and internals:
  [ticket 31](./31-schema-internals-simplification.md).
- Notice-on-read baseline store, CLI write commands, change-set check and the
  session/inbox workflow: [read model](./20-index-read-model-and-query-execution.md),
  [freshness](./21-index-freshness-and-invalidation.md),
  [diagnostics](./27-error-and-diagnostic-architecture.md),
  [workspace host](./26-workspace-host-and-composition.md).

## TODOs

Glossary and ADR updates deferred until the specification is written (do not
edit the glossaries or ADRs yet; they describe the current code):

- TODO `src/schema/GLOSSARY.md`: rewrite *Global Schema* (a `$ref` pool that is
  also the lowest-precedence layer for every note; still never a File Class);
  add *Class Binding* (tags, paths, `!` exclusions; defaults for notes without
  a known explicit class), *Class Set* (unreduced vs most-derived),
  *Class Conflict*, *Violation* (layer, origin, hint) with a Not-this list
  against *Diagnostic*.
- TODO `src/GLOSSARY.md` and `GLOSSARY-MAP.md`: add *Core* and *App Layer* and
  the consumer (cli, lsp, mcp) vocabulary when ticket 28 names them.
- TODO `docs/adr/0006`: its "`required`/`multi` inert until the MCP guardrail
  stage" wording is superseded (they are checked advisorily; gating is host
  policy); add the layering principle and host-policy model as a new ADR
  (status `proposed`, per the ADR rule).
