# Does the schema subsystem validate and type note data, and where does that capability live?

Type: grilling
Status: resolved
Blocked by: 18, 19

## Question

`src/schema` (12k LoC; ~4.7k production) models field definitions and
inheritance. It never touches note values, and the index extracts `fileClass`
independently. Should schemas drive validation/typing of note frontmatter and
inline fields? Options include advisory diagnostics, gating, or no checks. If so,
which module owns the validator, the diagnostic type, and the shared typed-field
representation for templates (`ui` prompts) and query?

Facts: `survey/schema.md` §2-3, `src/index/service.rs:1732-1923` (`mod
class_field`; shifted from 1656-1840 by `207cc3fb`), and global schema semantics.
Four field-type enums exist: `RawSchemaFieldType`, `SchemaFieldTypeTag`,
`SchemaFieldType`, and `NoteFieldType`. Other facts include the `SchemaService`
`expect(dead_code)` constructor and the `FileClassExpander` seam. The charting
judged this seam sound, but the Answer overturns it.

Alternatives: keep schema passive (template/prompt metadata only), add an
advisory validation pass producing structured diagnostics, or use schema-typed
ingestion at index time. Include the keep-as-is option and what it means for
12k lines.

Inputs: `survey/schema.md`, `research/03`, value model and coercion decisions,
compat envelope.

Graduates fog: schema internals simplification (see map).

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). Schema options use a schema-local
literal type. Strings stay strings, with no date/duration/link variants.
Native TOML dates read as strings. Select values that name links are interpreted as links
only here, at the consumer boundary, with the shared root `Link` wikilink
parser.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Frontmatter
classification stays schema-free and quote-insensitive. Whether to add a schema
override for date-shaped text is decided here (rejected below). Select strings
naming links are parsed at the schema-check boundary into `Link` values and
compared with the shared Link equality relation (folded target and subpath).
Query-literal adaptation is separate. Coercion failures remain advisory, never
gating.

## Answer

**Decision:** Schemas check note data on demand and report advisory violations.
They do not type values at parse time or gate reads and indexing. Schemas do not
enter the index or its fingerprint. `schema` owns the pure, span-free rule for
checking values against resolved effective fields. `note` keeps classification
(tickets 18 and 19). Each host sets gating policy over stable violation codes;
the check does not gate. The class-binding resolver selects schemas by
frontmatter class, tag and path bindings. The `global` layer also applies, as
specified below.

**Roles of the schema:**

- Value and source provider for templates and LSP completion.
- Advisory frontmatter checker for LSP diagnostics and post-render template checks.
- Hover and go-to-definition metadata.
- Write-path guardrail for an MCP agent.
- Schema definitions never affect parsing, the index or the fingerprint (only the configured class-field name is shared).

**Layering (standing principle, binds tickets 28, 31, 33 and 35):** cli, lsp and
mcp are thin hosts of `core`. `core` holds an app layer of use cases. The app
layer composes domain modules (`note`, `schema`, `query`, `index`, `template`)
over foundation types. Domain modules expose data and pure functions. No
one-implementor trait lets one module call another. Use cases take explicit
inputs, and they do not hold a context object.

**The check:**

- `schema` resolves a class set into effective fields. This is a pure function
  over a schema snapshot. Hosts replace the snapshot wholesale. No cache is
  mandated. `ResolvedFields::check(&FieldMap) -> Vec<Violation>` returns
  violations in deterministic field order, without spans or severity.
- The runtime value type (today `NoteFieldValue`) sits below both `schema` and
  `note`. Schema literals remain a separate type (ticket 18). Ticket 35 decides
  its exact placement. At check time, parse select values that name links with
  the root `Link` wikilink parser (ticket 18). Compare them with ticket 19's
  Link equality, not the query binder's adaptive-literal rule. Do not
  re-implement link equality in the schema checker.
- `check` returns kind mismatch, not in select set, out of range, `step`
  mismatch, missing required, `multi` arity and unknown field. It always returns
  unknown fields with an optional `closest` match. Hosts filter them. Each
  violation carries its layer (`class` or `global`), `origin` and a `hint` from
  the field's `description`. This lets an agent correct its input. Class
  resolution adds unknown class and `schema.class-conflict`. The app layer adds
  link target missing or wrong class. For expected sets, return the count and a
  truncated flag, with only the closest few using `strsim`.
- Tier-2 `file` checks: the check exposes folder, extension and class
  constraints as data. The app layer performs index lookups. No oracle trait is
  needed.
- `required` stays: absent, `null`, blank string and empty list mean missing.
  `false` and `0` count as present. One code reports this violation; `found`
  distinguishes absent from blank.
- Several classes:
  - Resolve inheritance within each class by the existing `extends` rule. Keep
    multiple parents: the first-listed parent wins, and a child redefinition
    wins over its parents.
  - Reduce the resolved class set to its most-derived members. An ancestor
    cannot conflict with its descendant.
  - Compare unrelated classes without regard to order. Identical declarations,
    including those shared through `$ref`, do not conflict. Differing
    declarations produce one `schema.class-conflict` per note and class set.
    Skip value checks on the conflicting field, but check `required` and `multi`
    arity if either class declares them.
  - Report colliding sibling parents without a child redefinition in the same
    way. Skip unknown classes. First-listed wins does not settle
    class-versus-class conflicts.
- No declared option is silently ignored. Check `min` and `max`. Check `step`
  exactly for integers and with relative tolerance for floats. `date.format` is
  a write-format hint: the check accepts Date/DateTime and any scalar if the
  format has no date component. This gap remains explicit until ticket 30 fixes
  the dialect.
- Add `origin`, `description` and `parent_order` now. The schema format may
  still change freely. Every field type accepts the new `description` key.
  `suggest_class` waits for the LSP.
- The check takes a field map. Ticket 23 gives the note an effective accessor
  that merges inline fields. The accessor does not feed this check. The check
  stays frontmatter-only.

**Global layer:** The reserved `global` schema remains a `$ref` pool. It also
becomes the lowest-precedence layer on the frontmatter of every note, classed or
not. It is not a class: no `@global` query can name it, and no class extends it.
A class declaration of the same field always wins. `required` in `global` still
degrades to `false`. A class can exclude a global field with `excludes`.
Violations from this layer carry layer `global`. The diagnostic adapter derives
a separate `schema.global.*` code family. Hosts can set a different policy
(for example, hint-level in the LSP or ignored on unclassed notes). Ticket 34
owns default severities and is asked to rank this family below class violations.
This changes the documented invariant in `src/schema/mod.rs:48-51`. The doc
comment and the survey are updated with the specification.

**Class binding:** A note's class set has three sources:

- The frontmatter class field.
- `tags` in a strict, non-inherited `[binding]` table on the class schema.
  Nested tags count, as in the existing `Tag` source atom.
- `paths` in the same table: vault-relative globs in the existing dialect,
  including `!` exclusion patterns.

Bookmarks and Base views are not sources. A note with one or more known explicit
classes uses only those classes. Bindings supply defaults only when there is no
known explicit class. Empty or unknown class values do not suppress bindings, so
a typo does not silently drop validation.

Expand bindings at query time into the existing `Class`, `Tag` and `Path` atoms,
plus a "has a known explicit class" exclusion. The index and its fingerprint
remain unaware of bindings. One resolver answers both "which classes does note n
have" and "which notes does class c claim". The latter includes hierarchy
expansion for `@book*`. Equivalence between these two answers is a tested
invariant on the unreduced class set. A move or a binding-tag edit changes the
class set and counts as a class change (see the guardrail). Ticket 28 owns
optimisation and benchmarking of path-binding queries. The `Path` atom scans all
paths today.

**Class hierarchy as data replaces `FileClassExpander`:** `schema` produces a
plain hierarchy keyed by one normalising `ClassName` (the index stores
lowercase). The app layer passes it to the query service. Return unknown classes
as data, not as a logging side effect. Hosts replace the snapshot wholesale when
schemas change. Delete the trait, crate-root `src/file_class_expander.rs`, the
`&mut ClassExpansionMode` out-parameter, and
`Option<Arc<dyn FileClassExpander>>`. Verified smells include a single
implementation, a consumer-defined trait, and out-parameter mutation of the
query AST. The orphan rule also forbids this implementation in any third crate once
`query` and `schema` are separate crates. Reproduction showed E0117 in a
throwaway workspace, but a newtype compiles.

A latent hazard exists. Without an expander, store queries fall back to exact
match while in-memory filters match nothing. Production always attaches one, so
this is not a current bug. Ticket 20 records the decision and the schema-side
producer. Ticket 28 owns the query-side change. Tickets 28 and 35 decide the
owning module of the hierarchy type.

**Template boundary:** The app layer checks the rendered note between `render()`
and `write()` and reports advisory violations. It parses rendered frontmatter
with the note parser, resolves the class and runs the same check. Skip the check
if the render has no frontmatter or class. Warn on stderr in interactive use.
Run the check under `--dry-run`. Return structured output under MCP. Reject
schema-driven prompts (ADR-0006 still keeps the schema as a source of values)
and static template analysis (frontmatter is raw text to the template AST).

**Agent guardrail:** Do not depend on harness hooks. Several harnesses lack
back. Prevention exists only where `traces` owns the write or the harness denies
native writes. Expose one `check_note(before, after)` contract, where each side
contains a path and content. Expose it through MCP tools and `traces check`:
paths, `--staged`, or `--before <file|->` `--after <file|->` with
`--before-path`/`--after-path` for stdin. Git is optional. Channels depend on
what the harness provides [P, `research/20-prep/round5/report.md`]:

- *Nothing:* an `AGENTS.md` snippet generated by `traces` (soft) and the exit
  code.
- *A shell or post-edit command:* a lint loop (Aider `--lint-cmd` [V]), a git
  pre-commit on staged blobs, CI.
- *A shell:* validated CLI write commands (`traces new`, a field-set command)
  sharing the MCP edit use case, so any agent with a shell has a gated path. It
  is a path, not a barrier.
- *Any `traces` call:* a bounded notice of notes changed since the last check
  that gained violations. It needs a check watermark and the violation keys at
  the last check as the `before`. This forms a second derived store outside the
  index and fingerprint, with unmeasured cost (tickets 24, 25, 34). Without it,
  report all violations of changed notes, no delta.
- *An MCP client:* validated edit and create tools, the only gate, plus a
  dry-run `check_note` tool taking a path and proposed content. A gate is as
  strong as its weakest open write path. The tools must cover every edit
  operation, including the body, or native writes must be denied. Otherwise,
  agents fall back to native edit tools that can change frontmatter. Guards the MCP host runs on its own, against protocol revision 2026-07-28 [V,
  [round-6 addendum](../research/20-prep/round6/I-mcp-server-guards.md)]:
  - Validate before write with the delta rule.
  - A content-hash precondition on edits (the agent must have read the current note).
  - Atomic write, then re-read and re-check.
  - A bounded undo journal.
  - Delete, move and bulk guards (inbound links, class-set change, a per-call cap).
  - Schema-aware input coercion.
  - Violations and a short field summary on every read result. Confirmations use a multi round-trip
  `input_required` result on `tools/call`, which only clients that declare
  `elicitation` can answer. With none, deny. `requestState` must be
  integrity-protected and bound to the request digest and a TTL. A notice of
  notes changed outside the server needs an in-process watcher and baseline.
  This notice is best effort only. The protocol has no sessions, so state
  across calls is a server-minted handle or process memory lost on restart. Also,
  the result block that carries it may or may not reach the model [H]. Features that are unavailable or untrusted:
  - Roots: deprecated. Use server configuration.
  - Logging: deprecated. Per-request opt-in.
  - `subscriptions/ listen`: client opt-in, no known model feed.
  - The Skills extension: no coding harness in the client matrix.
  - Tool annotations: hints, untrusted. Server `instructions` now ride
  `server/discover`, which clients may skip.
- *An LSP client:* diagnostics. Feedback only (`didChange` cannot deny,
  `willSaveWaitUntil` cannot abort a save).
- *Hooks, sandboxes, deny rules:* documented recipes, not features. Claude
  Code's pre-write hook does carry the full `content` for whole-file writes [V].
  The earlier claim that hooks never see the resulting file is withdrawn.
  Shell writes remain invisible to hooks.
- *Placement:* the only harness-independent prevention is to keep the writer
  away from the canonical store, which is a workflow, not a filter [H;
  consistent with every surveyed tool, none of which prevents raw writes]. Use
  an isolated working copy (git worktree or branch, or APFS/Btrfs clone elsewhere)
  or an `_inbox/` folder. A gated `traces merge` or `promote` promotes notes by
  running `check_note` on the destination path (bindings need it).
- *Working copy isolation:* isolation holds only where the writer is launched
  inside the copy. Same-user processes can still reach the canonical vault.
  Multi-user editing is a product feature outside the map's destination. The
  design pressure on core is that `check_note` must run over a change set with a
  base to detect concurrent edits.
- *Instruction files:* a per-folder `AGENTS.md` snippet generated from the
  `[binding].paths` of each class steers writers where they work (loaded lazily
  by Claude Code [V], others unknown [S]). Soft, never relied on.

Give the agent MCP tool input schemas and a schema resource up front. Do not
project large enums or strftime formats. Check those after the call. The delta
rule blocks only violations introduced by the edit. Key violations by
`(field, code, hash(value))`, not list position.

Ask the user before a class-set change, including one caused by a move or a
binding-tag edit. Also ask before setting an unknown class. Deny either change
if no confirmation channel exists. Host policy maps each code to ignore, warn or
reject. The MCP crate renders JSON Schema and `isError` results. Core has no
protocol types.

`traces check` uses separate exit codes for a broken schema and note violations.
A broken schema must not read as "notes are fine". Notes with no class, no
binding and no global field are unvalidated. MCP create tools should demand a
class or bound location. Under `--dry-run`, the template check has no output
path, so path bindings do not apply.

Rejected as a design basis or default:

- Hook-based gating.
- A staging directory without a launch convention (an instruction).
- A watcher daemon as a standalone product. The daemon has no channel to the agent; inside the MCP server it is an advisory notice.
- Auto-revert (destroys data).
- ACP, FUSE or WebDAV interception (custom client, kernel extension, fragile mounts).
- Git clean filters (break `checkout` and `diff`).
- `reference-transaction` hooks (see commit hashes only).
- YAML-comment or `$schema` hints in notes (Obsidian strips comments [V]; `$schema` pollutes every note).

A loopback NFS mount gates atomic-rename writers on macOS without root [V, one
experiment in round 6]. It does not gate in-place writes (a shell `>` exits 0
while the server rejects). It is bypassed through the backing directory, and
hangs processes if the daemon dies. At most, keep it as a documented recipe.

**MCP edits:** The MCP host, not core, owns how an MCP edit tool applies a
lossless frontmatter change to a note. Ticket 20 adds no write-back or span
requirement to ticket 23 (ticket 17 holds). Raise edit-range needs when MCP is
specified.

**Date-shaped text:** no parse-time override, no quote convention. `input` accepts any scalar. Consumers get a text view rendered from the typed
value, under a tested round-trip invariant. Canonical forms round-trip, and
normalisation is documented. Links already normalise display. A `string()` cast
arrives only if a real use appears (ticket 19).

A TOML-style single versus double quote convention is rejected [S]. Noyalib
exposes no per-scalar quote style outside the CST (+59% parse time). Also,
Obsidian's property editor rewrites quote style on save. Quoted numbers and versions already stay text and leading-zero values
are strings (`src/note/metadata.rs:319`). Only date, datetime and

**Alternatives considered:**

- *Keep schema passive:* rejected. It leaves `required`, `multi`, `min`, `max`,
  `step` and `format` inert, blocks tickets 34, 35 and 27 and fails the
  LSP-readiness commitment of ticket 17.
- *Checker in a consumer module above `schema` and `note`* (the earlier
  recommendation): withdrawn. It puts the rule outside its invariant owner and
  repeats the `FileClassExpander` shape.
- *`schema` imports `note`; `note` imports `schema`:* rejected (note must parse
  without schemas, ADR-0009).
- *Small value-view trait, compiled generic validator, merged vault-model
  module:* rejected. They need a glue implementation or inverted dependency. Flat scalar schemas do not justify a compiled engine (blast radius).
- *Index-time typing and query-bind typing:* rejected by tickets 16 and 19 and
  ADR-0009.
- *Optional link-oracle trait for tier-2 checks:* withdrawn (one-implementor
  trait).
- *Binding the agent through the LSP:* rejected (advisory by protocol).
- *Default severities in the check:* not adopted. Ticket 34 owns severities.
- *Last-listed wins (Fileclass), first-listed wins, conjunction of all classes
  (IWE, JSON Schema `allOf`), single class only:* order-dependent rules were
  rejected as the cross-class rule. Reordering a class list must not flip
  results, and nothing is masked silently. Single-class only is rejected because
  Fileclass, Metadata Menu, IWE and Logseq all support several classes [V].
- *JSON Schema as the schema format:* rejected. IWE uses it for validation only. Traces schemas also drive prompts, completion, file links and label/value mapping. JSON Schema remains an export for MCP.
- *Binding only by frontmatter class* (the first Answer) and *binding stored in
  the index:* rejected. Binding by tag and path is conventional in Metadata
  Menu, Fileclass and IWE, and query-time expansion keeps the index schema-free.
  Union and priority-ordered combinations of explicit and bound classes (Metadata
  Menu, Fileclass, IWE) were considered. Suppression was chosen because bindings
  cover notes without a class. This choice creates the fewest false conflicts
  and keeps one simple rule. Fallback if the query exclusion proves costly:
  union with source priority.
- *Global as a reference pool only, or applied only to classed notes:* rejected;
  applying it to every note types shared fields like `due` in unclassed notes
  without a binding mechanism.

**Invariant owners:**

- Validity of a value for a declaration (`schema`).
- Classification (note parser, tickets 18 and 19).
- Select and link equality (value-semantics module, ticket 19).
- Class-name normalisation (one `ClassName`).
- Class binding resolution, class-change detection and the delta (core app layer).
- Policy per code (each host).
- Precedence of the `global` layer (`schema`).

**Becomes removable:**

- `FileClassExpander` and `src/file_class_expander.rs`.
- The `ClassExpansionMode` out-parameter.
- The `Option<Arc<dyn>>` field and the unknown class logging side effect.
- The test-only `SchemaService::new`.
- The `#[cfg(test)]` gating on `min`, `max`, `step` and `format` accessors (they become production).

Not removable: `required`, `multi`, `min`, `max`, `step`, `format`.

**Evidence note:** the author's own vault was treated as one example, not as
authority. Decisions rest on code, ADRs, tickets, protocol facts and external
precedent. Items tagged [S] come from scout reports not re-read. No benchmark
was run. Drift since `207cc3fb` touched `query/service.rs`, `query/grammar/source.rs`,
`template/service.rs` and `template/engine/schema.rs` only in doc comments and
the minijinja per-render state API used by the template `Schema` handle. The
seams cited here still exist. These seams include the `FileClassExpander` trait
and the `render()`/`write()` split. Research:
[brief](../research/20-prep/brief.md) (revision 2 plus MCP section; its
consumer-module recommendation is superseded here), the round-3
[synthesis](../research/20-prep/round3/report.md) and the round-4 [external
precedent](../research/20-prep/round4/report.md). Verified in round 4: Fileclass
last-wins and class binding sources, Metadata Menu's combined mapping, Logseq
multi-parent `extends`, IWE's design, Umbraco's rejection of conflicting
compositions. Corrected: the earlier attribution of first-listed wins to Metadata Menu was
wrong. The arXiv paper cited for "agents ignore advisory warnings" does not show
that claim. The guardrail rests on protocol arguments and not on that paper. Scout E's default severities and element-indexed delta key
are superseded by this Answer. Rounds 5 and 6 read the remaining scout claims at the source ([claim checks and
guardrail channels](../research/20-prep/round5/report.md), [harness-independent
guardrails](../research/20-prep/round6/report.md)). LDAP, CUE, Kubernetes
pruning, Plone, Tana, SilverBullet, Metadata Enforcer, and Obsidian rewriting
were verified. Dendron "fatal" path binding, Logseq "broken trees", and
Letta/Graphiti delta validation were refuted. These rounds withdrew the
hook-payload claim, and found no filter-level prevention that is both preventive
and portable. Per-harness capability cells are unaudited beyond Claude Code and
Aider.

**Remains unresolved (handed off):**

- Value-type placement below `schema`, hierarchy type module, visibility, crate
  placement of the app layer: [crate
  topology](./35-crate-topology-and-public-surface.md).
- Query-side change replacing the expander: [query
  core](./28-query-core-and-plan.md).
- Field-key spans for adapters, and inline-field merge: [note document
  model](./23-note-document-model.md).
- `date.format` dialect and the date-format check: [temporal
  boundary](./30-temporal-boundary-and-dialects.md).
- Template service wiring of the post-render check: [template
  boundary](./31-template-engine-boundary.md).
- Diagnostic envelope, severities, per-code overrides and the registry: [error
  and diagnostic architecture](./34-error-and-diagnostic-architecture.md).
- Host composition and use-case layer: [workspace
  host](./33-workspace-host-and-composition.md). Open-buffer checks: [live
  buffer](./27-live-buffer-and-persisted-index-consistency.md).
- Re-test of the "provisionally withstands scrutiny" claim for the
  `FileClassExpander` seam (overturned here):
  [reconciliation](./36-reconcile-decisions-and-audit-coverage.md).
- Schema internals (graduated fog): see ticket 37.
- How MCP edit tools apply note changes, if they need edit ranges: raised when
  MCP is specified (ticket 33 or later).
- Body-section schemas (required headings, order, depth, later blocks and token
  budgets): [ticket 38](./38-body-section-schemas.md). 20 keeps the violation
  locator open to more than a frontmatter key.
- Query-side expansion of bindings, the "has a known explicit class" atom and
  the path-scan optimisation: [query core](./28-query-core-and-plan.md).
- Rank of the `schema.global.*` family and the schema-error exit code:
  [diagnostics](./34-error-and-diagnostic-architecture.md).
- `[binding]` table and `global` semantics in the schema format and internals:
  [ticket 37](./37-schema-internals-simplification.md).
- Notice-on-read baseline store, CLI write commands, change-set check and the
  session/inbox workflow: [read
  model](./24-index-read-model-and-query-execution.md),
  [freshness](./25-index-freshness-and-invalidation.md),
  [diagnostics](./34-error-and-diagnostic-architecture.md), [workspace
  host](./33-workspace-host-and-composition.md).

## TODOs

Glossary and ADR updates deferred until the specification is written (do not
edit the glossaries or ADRs yet; they describe the current code):

- TODO `src/schema/GLOSSARY.md`: rewrite *Global Schema* (a `$ref` pool that is
  also the lowest-precedence layer for every note; still never a File Class).
  Add *Class Binding* (tags, paths, `!` exclusions; defaults for notes without a
  known explicit class). Add *Class Set* (unreduced vs most-derived), *Class
  Conflict*, and *Violation* (layer, origin, hint) with a Not-this list against
  *Diagnostic*.
- TODO `src/GLOSSARY.md` and `GLOSSARY-MAP.md`: add *Core* and *App Layer* and
  the consumer (cli, lsp, mcp) vocabulary when ticket 35 names them.
- TODO `docs/adr/0006`: its "`required`/`multi` inert until the MCP guardrail
  stage" wording is superseded. They are checked advisorily, and gating is host
  policy. Add the layering principle and host-policy model as a new ADR (status
  `proposed`, per the ADR rule).

## Comments

2026-10-21, re-test against [research
12](../research/12-iwe-core-and-hosts-architecture.md),
[13](../research/13-iwe-query-schema-and-agent-write-gating.md) and
[14](../research/14-fileclass-model-binding-and-validation.md) (IWE and
Fileclass were read from source for the first time). **No decision is
overturned.** Confirmed or refined:

- *Suppression and conflict reporting.* Fileclass's code collects every bound
  class and lets the last win a shared key. A lower-ranked source (path,
  bookmark, base view) overrides a frontmatter class although its docs list
  frontmatter first (`src/schema/resolver.ts:119-180`; code-read, no test,
  intent unknown). This is the failure the suppression rule and the one-per-note
  `schema.class-conflict` avoid.
- *`global` layer.* Fileclass prepends its global class to every note's merge
  list, so it is the lowest-precedence layer for classed and unclassed notes
  (`resolveBinding`, `src/schema/resolver.ts:200-219`) [V]. Same design.
- *`origin`.* Fileclass keeps a `BindingOrigin` per class (frontmatter, tag,
  path, bookmark, global) [V], the same idea as the violation `origin`.
- *Delta rule.* IWE's gate, `validate_pending_documents`, validates each pending
  `(key, content)` pair in isolation in a throwaway graph, with no base to
  compare (`crates/diwe/src/schema.rs:255-312`) [V]. Any violation in a touched
  document blocks the write. It is the same shape as `check_note`, minus the
  delta. The block-on-pre-existing-violation consequence is [H].
- *Other gates.* IWE's MCP schema gate is always on; "strict" means mandatory
  `expect` count guards on `iwe_query` mutations (`docs/mcp.md:99`), and every
  write tool accepts `dry_run`. Multi-document changes are all-or-nothing.
  `expect` guards are a candidate for the MCP host's bulk guards (ticket 33).
- *Stricter than both.* Fileclass never validates `step`, lets `[]` satisfy
  `required` (`validate.ts:58`) and accepts any non-empty string for a field
  with a custom date format. IWE has no `step`. The Answer's rules stand.
- *Binding placement.* IWE binds in the host config (`[schemas.<name>].match`,
  globs with gitignore-style `!`, last match wins), has no inheritance and
  composes overlapping schemas order-free. The Answer keeps class-owned
  `[binding]`. Ticket 37 should state how `!` ordering works in the existing
  glob dialect.
- *Design-time checks.* IWE exits 2 for unreachable schema entries and Fileclass
  strikes through a tag that can never bind. A "binding that can never match"
  check fits the schema-error exit code.

*Corrections to earlier claims about these tools* (details in the ledgers of
research 13 and 14):

- IWE does not bind by a frontmatter `type` (round-4 file D).
- Its "strict mode" is not the gate's on switch.
- Fields with a reserved prefix (`_`, `$`, `.`, `#`, `@`) are validated since iwe 0.22.0. Withdraw the "[V] IWE ignores reserved-prefix fields" row in the round-4 report. The doc it read (`crates/iwe/docs/schema.md:83`) is stale. The source test is `liwe/src/schema/document.rs` `builds_section_tree_with_levels_and_keeps_prefixed_frontmatter`.
- Fileclass has 28 field types in its AST (22 take input), not 26.
- Its documented binding priority does not match its code.
