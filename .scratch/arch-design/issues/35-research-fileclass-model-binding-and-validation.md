# What does Fileclass's source teach about class resolution, typed fields and validation?

Type: research
Status: resolved
Blocked by: none

## Question

Fileclass (`mdelobelle/fileclass`, TypeScript Obsidian plugin; successor to
Metadata Menu by the same author; 541 unit tests; separate `fileclass-cli`
repo) is the nearest living implementation of class-typed frontmatter that
Traces' schema subsystem resembles. Round 4 of the ticket 16 prep verified only
its published docs (last-listed class wins, binding sources, advisory UI). Its
source, tests and design notes were not read. Metadata Menu was read in tickets
03 and the ticket 16 prep (`research/16-prep/b-metadatamenu-lessons.md`); this
ticket covers what Fileclass changed and learned, not Metadata Menu again.

Study: class resolution and binding (`src/index`, `src/fileclass`, bindings by
alias, tag, path, bookmark group, Base view and global default; precedence and
what happens with several); inheritance (`extends`, `excludes`, field order,
cycle guards, the multi-class overlay rule and how it is implemented and
tested); the typed-field model (the 26 field types, per-type options, candidates
and value sources in `src/fields/*`, `candidates.ts`, `dateValue.ts`,
`dateFormats.ts`, `dateFormatCheck.ts`, `duration.ts`, `conditional.ts`,
`objectPath.ts`); validation (required, per-note validation, severities, how
violations are modelled and surfaced; `src/api/fileclassApi.ts`, `filter.ts`;
the CLI/API `validate` and `setValue` gating); the query cache and
invalidation (`src/engine/queryCache.ts`); template adapter
(`engine/templateAdapter.ts`) and create-note-with-class commands; rename-field
migration and insert-missing-fields (write-back safety, frontmatter round-trip,
`processFrontMatter`); the `demo/` probes and `ROADMAP.md`, `CLAUDE.md` and
docs digest (`positioning`, `schema`, `fields`) for stated regrets and
non-goals (why Lookup/Formula were dropped, why frontmatter-only).

Focus, against decisions taken or open in this map:

- [Schema typing](./16-schema-note-typing-and-validation.md): class binding
  (tag, path, bookmark, global), precedence versus suppression, last-wins versus
  order-independent conflict reporting, `required` semantics (blank, empty),
  min/max/step, select value sources, date format as hint versus check,
  violation shape; re-verify the round-4 Fileclass claims against source and
  correct any that were wrong.
- [Coercion and comparison](./15-coercion-and-comparison-policy.md) and
  [value model](./14-value-model.md): how typed fields coerce and compare
  frontmatter values (numbers, dates, durations, links, lists, objects).
- [Temporal boundary](./23-temporal-boundary-and-dialects.md): date and
  duration formats, Moment tokens, interval choices.
- [Schema internals](./31-schema-internals-simplification.md): which of the 26
  types and options earn their keep; what an LSP-completion or hover consumer
  needs from a field definition.
- [Body-section schemas](./32-body-section-schemas.md) and
  [live buffers](./30-live-buffer-and-persisted-index-consistency.md): anything
  Fileclass does about body structure or unsaved state.

Informs: tickets 15, 16 (re-test), 23, 31, 32, 29.

Deliverable: `.scratch/arch-design/research/35-fileclass-model-binding-and-validation.md`
(<= 300 lines). Source: `docs/digests/obsidian_mdelobelle-fileclass-*digest.txt`;
use the pre-split tree at `/tmp/fc/` (recreate if missing). The main digest has
2.7k files; start from `src/` (src digest), the docs digest and tests for the
named areas; avoid `demo/` videos and screenshots beyond the notes named above.
Cite repo path plus symbol or file-relative line for every claim. Structure:
(1) what Fileclass actually does, (2) invariants and owners, (3) tradeoffs and
costs, (4) transferable lessons, (5) lessons that do NOT transfer and why (TS
and Obsidian specific), (6) what this changes or challenges in the named Traces
decisions, including a list of round-4 claims about Fileclass now confirmed or
corrected. Mark [V] and [H]. Do not propose the final decision.

## Answer

Deliverable: [research 35](../research/35-fileclass-model-binding-and-validation.md),
claims checked by the owner against `/tmp/fc` (resolver, validator) before
recording.

- Binding sources and `global`: confirmed independently. `global` is the first
  entry of the merge list, so it is the lowest-precedence layer for every
  note, as ticket 16 decided. `BindingOrigin` is the same idea as `origin`.
- Precedence: the documented order is frontmatter, tag, path, bookmark, base
  view, but the code collects every match and lets the last win a shared key,
  so a lower-ranked source overrides a frontmatter class (code-read, untested;
  intent unknown). It supports ticket 16's decision that an explicit known class
  suppresses bindings and that conflicts are reported, not ordered.
- Validation is weaker than ticket 16's: `step` is UI-only, `[]` satisfies
  `required`, a custom date format accepts any non-empty string, and the
  number and boolean checks accept numeric and boolean strings. Ticket 16 keeps
  its stricter rules.
- 28 field types in the AST, about ten structurally distinct; two are dead
  stubs, three are canvas sync: input to ticket 31. Nested fields are keyed
  by `path\0name`.
- No body validation, no buffer handling; writes only through the host's
  frontmatter API. Rename-field migration rewrites `fieldsOrder` across
  descendant classes: design pressure only.
- Correction by the owner to the draft: its "test pins path overrides
  frontmatter" citation did not hold (`resolver.test.ts:49-58` declares no
  contested key), so the claim is recorded as code-read, untested.

Unresolved: none for the destination. Downstream: tickets 23 and 31 carry an
"Inputs" section; ticket 16 has a comment.
