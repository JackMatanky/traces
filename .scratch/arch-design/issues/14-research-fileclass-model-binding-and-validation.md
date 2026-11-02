# What does Fileclass's source teach about class resolution, typed fields and validation?

Type: research
Status: resolved
Blocked by: none

## Question

Fileclass (`mdelobelle/fileclass`, TypeScript Obsidian plugin) is the nearest
living implementation of class-typed frontmatter that Traces' schema subsystem
resembles. It is the successor to Metadata Menu by the same author. It has 541
unit tests and a separate `fileclass-cli` repo.

Round 4 of the ticket 20 prep verified only its published docs (last-listed class
wins, binding sources, advisory UI). Its source, tests and design notes were not
read.

Metadata Menu was read in tickets 3 and the ticket 20 prep
(`research/20-prep/b-metadatamenu-lessons.md`). This ticket covers what
Fileclass changed and learned, not Metadata Menu again.

Study these areas:

- Class resolution and binding (`src/index`, `src/fileclass`).
- Bindings by alias, tag, path, bookmark group, Base view and global default.
- Precedence and behavior when several classes match.
- Inheritance (`extends`, `excludes`, field order, cycle guards).
- The multi-class overlay rule, its implementation, and its tests.
- The typed-field model with 26 field types and per-type options.
- Candidates and value sources in `src/fields/*`, `candidates.ts`,
  `dateValue.ts`, `dateFormats.ts`, `dateFormatCheck.ts`, `duration.ts`,
  `conditional.ts`, `objectPath.ts`.
- Validation: required fields, per-note validation, severities, and violation
  models (`src/api/fileclassApi.ts`, `filter.ts`).
- Gate checks in the CLI/API `validate` and `setValue`.
- The query cache and invalidation (`src/engine/queryCache.ts`).
- Template adapter (`engine/templateAdapter.ts`) and create-note-with-class
  commands.
- Rename-field migration and insert-missing-fields (`processFrontMatter`).
- Write-back safety and frontmatter round-trip.
- The `demo/` probes, `ROADMAP.md`, `CLAUDE.md`, and docs digest
  (`positioning`, `schema`, `fields`).
- Stated regrets and non-goals: why Lookup/Formula were dropped, and why
  frontmatter-only.

Focus, against decisions taken or open in this map:

- [Schema typing](./20-schema-note-typing-and-validation.md): class binding
  (tag, path, bookmark, global), precedence versus suppression, and conflict
  reporting. Semantics of `required` (blank, empty), min/max/step, select value
  sources, and date format as hint versus check. Violation shape. Re-verify the
  round-4 Fileclass claims against source. Correct any wrong claims.
- [Coercion and comparison](./19-coercion-and-comparison-policy.md) and [value
  model](./18-value-model.md): how typed fields coerce and compare frontmatter
  values (numbers, dates, durations, links, lists, objects).
- [Temporal boundary](./29-temporal-boundary-and-dialects.md): date and
  duration formats, Moment tokens, interval choices.
- [Schema internals](./36-schema-internals-simplification.md): which of the 26
  types and options earn their keep. What an LSP-completion or hover consumer
  needs from a field definition.
- [Body-section schemas](./37-body-section-schemas.md) and [live
  buffers](./27-live-buffer-and-persisted-index-consistency.md): anything
  Fileclass does about body structure or unsaved state.

Informs: tickets 19, 20 (re-test), 29, 35, 36, 37.

Deliverable:
`.scratch/arch-design/research/14-fileclass-model-binding-and-validation.md` (<=
300 lines).
Source: `docs/digests/obsidian_mdelobelle-fileclass-*digest.txt`.

Strategy:

- Use the pre-split tree at `/tmp/fc/` (recreate if missing).
- The main digest has 2.7k files.
- Start from `src/` (src digest), the docs digest, and tests for the named
  areas.
- Avoid `demo/` videos and screenshots beyond the notes named above.
- Cite repo path plus symbol or file-relative line for every claim.
- Mark [V] and [H].
- Do not propose the final decision.

Structure:
(1) what Fileclass actually does.
(2) invariants and owners.
(3) tradeoffs and costs.
(4) transferable lessons.
(5) lessons that do NOT transfer and why (TS and Obsidian specific).
(6) what this changes or challenges in the named Traces decisions, including a
list of round-4 claims about Fileclass now confirmed or corrected.

## Answer

Deliverable: [research 14](../research/14-fileclass-model-binding-and-validation.md),
claims checked by the owner against `/tmp/fc` (resolver, validator) before
recording.

- Sources of binding and `global`: confirmed independently. `global` is the
  first entry of the merge list. It is the lowest-precedence layer for every
  note, as ticket 20 decided. `BindingOrigin` is the same idea as `origin`.
- Precedence: the documented order is frontmatter, tag, path, bookmark, base
  view. The code collects every match and lets the last win a shared key. A
  lower-ranked source overrides a frontmatter class (code-read, untested.
  Intent unknown). It supports ticket 20's decision: an explicit known class
  suppresses bindings, and conflicts are reported, not ordered.
- Validation is weaker than ticket 20's. `step` is UI-only. `[]` satisfies
  `required`. A custom date format accepts any non-empty string. Number and
  boolean checks accept numeric and boolean strings. Ticket 20 keeps its
  stricter rules.
- 28 field types in the AST, about ten structurally distinct. Two are dead
  stubs, three are canvas sync: input to ticket 36. Nested fields are keyed by
  `path\0name`.
- No body validation, no buffer handling. Writes only through the host's
  frontmatter API. Rename-field migration rewrites `fieldsOrder` across
  descendant classes: design pressure only.
- Correction by the owner to the draft: a citation did not hold.
  `resolver.test.ts:49-58` declares no contested key for "test pins path
  overrides frontmatter". The claim is recorded as code-read, untested.

Unresolved: none for the destination.
Downstream: tickets 29 and 36 carry an "Inputs" section. Ticket 20 has a
comment.
