# Research: fileclass (Obsidian plugin) — digest-synthesis critical record

**Scope**: this is a *digest-synthesis* record, not a live-repo deep dive. A relevance
pre-review judged fileclass **marginal** for the design map — its digests carry enough
design substance, and Metadata Menu (same author, ticket 08) already carries the core
fileClass-completion precedent. One narrow web axis was checked live (maintenance/adoption
facts, §2.3 and §12). Every load-bearing claim below cites the digest file and the
`FILE:` section inside it. Self-reported figures (541 tests, 44 videos) are flagged as such
wherever used.

**Primary corpus**:

- `docs/digests/obsidian_mdelobelle-fileclass-digest.txt` (the full repo: README, source,
  docs site, tests, demo vaults; 2,695 `FILE:` sections — cited below as *main digest*)
- `docs/digests/obsidian_mdelobelle-fileclass-docs-digest.txt` (README + the complete
  `docs/` Hugo site + `tests/e2e/README.md` + triage doc; a **subset** of the main digest,
  not a thin remnant — the docs prose lives here *and* in the main digest)
- `docs/digests/obsidian_mdelobelle-fileclass-src-digest.txt` (README + all of `src/` +
  all of `tests/unit/`; 211 `FILE:` sections; source also duplicated in the main digest)

**Web check (2026-10-21, narrow axis only)**: GitHub repo page for
`mdelobelle/fileclass` fetched directly; one Google search for Metadata Menu's maintenance
status. Details in §2.3 and §12.

---

## 1. What fileclass is

Fileclass is an Obsidian community plugin (id `fileclass`, author mdelobelle, MIT,
v0.2.15 at digest time; `isDesktopOnly: false`) that gives frontmatter **typed, validated
properties** driven by per-class schemas — "a schema and input forms for your frontmatter"
(main digest, `FILE: README.md`; `FILE: manifest.json`). Users define **fileClasses**
(note types) as Markdown notes in a configured class-files folder; each declares typed
fields; notes are bound to classes by frontmatter `fileClass:` key, tag, folder path,
bookmark group, Base view, or a vault-wide default (main digest, `FILE:
docs/content/schema.md`, "Binding a note to fileClass(es)").

Key positioning facts (main digest, `FILE: README.md`; `FILE:
docs/content/positioning.md`):

- **Frontmatter-only by design**: reads via Obsidian's metadata cache, writes via
  `app.fileManager.processFrontMatter`; "note text is never parsed or edited." Users of
  Dataview inline fields (`key:: value`) are told to stay on Metadata Menu.
- **No Dataview dependency** — candidate lists for link/choice fields come from Obsidian
  core **Bases** views (`.base` files) instead of Dataview queries. Schemas and typed input
  work without Bases; query-dependent features degrade gracefully (main digest, `FILE:
  docs/content/_index.md`, Requirements).
- **26 field types** (Input, MultiInput, Number, Boolean, Select, Cycle, Multi,
  Date/DateTime/Time, Duration, CycleDuration, File/MultiFile, Media/MultiMedia, Location,
  Icon, Color, Object/ObjectList, JSON, Canvas/CanvasGroup/CanvasGroupLink) — full type
  table with per-type validation shape in main digest, `FILE: docs/content/fields.md`,
  opening table.
- Surfaces: guided input modals, Obsidian-native Properties-panel buttons, context menus,
  DOM-injected nav indicators, an editable `fileclass-table` Bases view with validation
  columns, a schema canvas, a public plugin API, and a standalone CLI/TUI (separate repo,
  drives the *running* Obsidian app via `obsidian eval`) — main digest, `FILE: README.md`,
  Features; `FILE: docs/content/cli.md`.

## 2. Lineage vs Metadata Menu

### 2.1 Successor relationship (digest-verified)

Fileclass is, by its own manifest description and README, "the successor to Metadata Menu
(same author)" (main digest, `FILE: manifest.json` description; `FILE: README.md`,
"Coming from Metadata Menu"). The pre-review's claim 1 is **verified**:

- **File format unchanged**: "Your fileClass notes are read as-is… the Metadata Menu
  format is unchanged" (README; `FILE: docs/content/schema.md`: "The file format is
  Metadata Menu's, unchanged"). Legacy Dataview-era option keys (`dvQueryString`,
  `customRendering`) are ignored without crashing; Lookup and Formula are out of scope
  (read-only if present), with Bases views as the replacement for reverse relations and
  computed columns (`FILE: docs/content/positioning.md`, "What changes on migration").
- **Maintenance mode + don't-run-both**: "Metadata Menu still works, but **don't run both
  at the same time**. It is in maintenance mode and won't receive further features"
  (README, "Coming from Metadata Menu").
- **Port-not-fork**: the repo's own `CLAUDE.md` names the local Metadata Menu build as
  "Reference implementation to port behavior from… port semantics, not code style; that
  codebase carries dataview-era baggage this plugin must not inherit" (main digest, `FILE:
  CLAUDE.md`).

### 2.2 Completion lineage (pre-review claim 1, second half — verified with a nuance)

Fileclass's editor completion is one `EditorSuggest`: `FrontmatterValueSuggest extends
EditorSuggest<ValueSuggestion>`, registered via `registerEditorSuggest` (main digest,
`FILE: src/ui/frontmatterSuggest.ts`; registration in `FILE: main.ts`). The file header
states it explicitly: "Read from Metadata Menu's own suggester, which gates on exactly
those three [Select/Cycle/Multi]." This is the same `EditorSuggest`-in-frontmatter pattern
ticket 08/21 Part VII already recorded for Metadata Menu's `ValueSuggest`. Differences
worth noting:

- It is a **value-only** suggester (choice lists for Select/Cycle/Multi). No frontmatter
  *key-name* suggester exists in the digest; types without a value list (Number, Date,
  Object…) are deliberately routed to the "manage field at the cursor" command instead
  (main digest, `FILE: src/commands/manageFieldAtCursor.ts` header: "Deliberately narrow,
  as Metadata Menu's equivalent was: a key the schema does not declare gets **nothing**").
- The trigger is a careful caret-inside-frontmatter-value scan
  (`frontmatterValueAt` from `FILE: src/io/frontmatterCaret.ts`), gated on a per-setting
  flag (`enableFrontmatterSuggest`), with list-item de-duplication of already-taken values
  and a "nothing matched → offer the full list anyway" repair behavior (measured from a
  real-vault report) — `FILE: src/ui/frontmatterSuggest.ts`, `onTrigger`/`getSuggestions`.

### 2.3 Maintenance/adoption facts (web-checked, 2026-10-21 — clearly labeled)

- Live GitHub page (`github.com/mdelobelle/fileclass`): **76 stars, 4 forks, 28 open
  issues, 2 open PRs, 217 commits**. Young, small, actively developed.
- The live README fetched today is **textually identical** to the digest's README (same
  "541 unit tests", same "44 of them" videos line) — the digests are **not stale** on the
  prose surface as of 2026-10-21.
- Release history in-digest: `FILE: versions.json` lists 23 published versions
  (`0.0.1`…`0.2.15`, minAppVersion `1.12.7`); the repo ships its own release-download
  tracking script (`FILE: tools/release-stats.sh`), i.e. the maintainer actively monitors
  adoption. No release *dates* are in the digest, so cadence cannot be quantified from it.
- Metadata Menu's own status: web search confirms "Metadata Menu is officially in
  maintenance mode" and remains available; the summary put MM at "700+ stars" (approximate,
  from a synthesized search result, not a fetched repo page — treat as directional).

**Design-map weight**: fileclass is a *small but actively-maintained* successor to a
mainstream plugin (~700+ stars). Its patterns are current-generation signal; its raw
adoption numbers are an order of magnitude below Markdown Oxide/Marksman-class tools. It
is relevant here as the **most current production example** of frontmatter-schema
semantics, not as an adopted standard.

## 3. Architecture & data model (digest)

- **Schema storage**: one Markdown note per class under a single class-files folder; name
  = filename; frontmatter declares `extends`, `excludes`, `mapWithTag`, `tagNames`,
  `filesPaths`, `relatedViews`, `icon`, `version`, `fields[]` (`{name, id, type, options,
  path}` — `path` = parent field ids joined by `____` for nesting), plus `fieldsOrder`,
  `newNotes`, `baseFile` (main digest, `FILE: docs/content/schema.md`). Unknown field
  types load as `Input` and are "reported as a non-fatal error rather than crashing the
  index."
- **Inheritance**: single-parent chain, **cycle-guarded** ("a self-reference or loop
  simply stops"), resolved root→leaf, **de-duplicated by field name** (nearest declaration
  wins, keeping the ancestor's position); `excludes` accumulates down the chain (schema.md,
  "Inheritance"; `FILE: src/schema/inheritance.ts`). Note the contrast with Traces'
  Kahn's-topo-sort over a DAG — fileclass forbids diamonds by construction (one parent).
- **Binding precedence** (a note can carry several classes): frontmatter `fileClass:` key
  > tag match (including *nested* tags, most-specific first) > path > bookmark group >
  Base view > preset fields; a vault-wide global class is *lowest* precedence; among two
  frontmatter-declared classes, **the last one wins** on a colliding key (schema.md,
  "Binding a note to fileClass(es)").
- **Index**: `FileclassIndex` — "slim successor of Metadata Menu's FieldIndex" — registry,
  ancestors, resolved fields, file→class binding maps; rebuild debounced on
  metadataCache `resolved` + class-file changes; fires `fileclass:indexed` (main digest,
  `FILE: src/schema/fileclassIndex.ts` header). Frontmatter-only reads; "no dataview, no
  IndexedDB."
- **Writes**: every user action is a single `processFrontMatter` write, values addressed
  by name-path, unknown keys preserved (main digest, `FILE: src/io/write.ts` header,
  "ARCHITECTURE.md §4, D2, D5"). One measured exception — see §7.

## 4. Editor/UI surfaces (what exists instead of LSP features)

- **Value suggester** (§2.2) — the only editor completion surface.
- **DOM-injected nav indicators**: `FieldIndicator` injects a clickable icon next to a
  note's name in the tab header, file explorer, and bookmarks; explicitly labeled in its
  own header as "the fragile DOM-injection boundary: it is isolated here, every surface is
  behind a settings flag, each pass is wrapped so a missed selector no-ops rather than
  throwing… Core features never depend on it." It uses MutationObservers on nav panes and
  undocumented workspace-leaf internals reached "through minimal structural casts" (main
  digest, `FILE: src/ui/indicator/fieldIndicator.ts`). A parallel `LinkIndicator` decorates
  internal links in reading view/backlinks/Bases (`FILE: main.ts` plugin fields).
- **Hover exists only as Obsidian-native popovers**, not field-schema hover: the
  note-fields modal listens to the undocumented `hover-link` workspace event to show a
  per-link popover (fileClass wrench + open-fields action) — `FILE:
  src/ui/noteFieldsModal.ts` (hover-link handling); value previews on hover in tables are
  display-only type-appropriate cues (color swatch/icon glyph/media thumb) — `FILE:
  src/ui/valuePreview.ts` header. Nothing anywhere attaches schema metadata to a hover of
  a frontmatter key.
- **The schema canvas**: a rendered `.canvas` model of the class graph (inheritance, base
  dependencies, claimed folders/tags), including struck-through dead tags — main digest,
  `FILE: docs/content/views.md`, "The schema canvas"; `FILE: src/views/schemaCanvas.ts`.

## 5. The validation engine (`src/fields/validate.ts`) — the design-substance core

This is the single most valuable file for the map. `validateField(field, value,
allowedValues)` is a pure, unit-tested, per-value function (main digest, `FILE:
src/fields/validate.ts`; tests in `FILE: tests/unit/validate.test.ts`). Its documented
contract, verified line-by-line:

1. **Empty is always valid unless `required`**: "Empty values
   (undefined/null/\"\") are always valid — Fileclass fields are optional unless a later
   constraint says otherwise." `required: true` in field options flips empty → violation
   (`isRequired`, `validateField` first line). *Corroborates ticket 19/20's
   required-field handling; matches Dataview/Metadata Menu permissiveness.*
2. **Empty allowed-values list = unconstrained**: `validateInList` returns VALID when
   `allowed.length === 0` ("an empty list means 'unconstrained'", file header). **This is
   an independent production corroboration of ticket 20's Q6** ("empty values list = skip
   validation, not 'must be empty'" — research/20 Part VIII weakness #4).
3. **No file-existence checking for note values**: `File`/`Media` accept any string;
   `MultiFile`/`MultiMedia` accept any array of anything. **Corroborates ticket 20's Q15**
   (type + shape only, pure function). *Important distinction the pre-review blurred:
   definition-side paths ARE existence-checked by the schema audit (§6) — values-notes,
   `.base` files, canvases, claimed folders, `extends` targets. What is never checked is
   whether a note's File-field value points at a real note.*
4. **Custom date formats are unchecked**: `validateDatePattern` — "A custom format can't be
   checked without a date library; accept non-empty." Only the three built-in shapes get
   regex checks (`YYYY-MM-DD`, `YYYY-MM-DD[ T]HH:mm[:ss]`, `HH:mm`); insert-as-link
   wikilinks are accepted as-is via `WIKILINK_RE`. **Corroborates ticket 20's Q16
   (syntax-only)**, with a nuance: fileclass's gap is a *dependency* gap (no date lib in
   an Obsidian plugin bundle), whereas Traces already has `chrono` and Q16's
   ISO-parseability check is strictly stronger than fileclass's behavior.
5. **Type-dependent scalar-vs-list shape checking — the pre-review's claim 3 detail,
   verified, with a measured rationale that is itself design substance**:
   - `MultiInput` and `Multi` **accept a scalar** where a list is declared (`asItems`
     wraps it), justified in-code: "Measured: `contains`, `containsAny` and `==` all match
     `themes: Ecology` exactly as they match `themes: [Ecology]`, so nothing that queries
     a vault can tell the two apart — and a vault coming from Metadata Menu is full of
     the scalar form… Flagging it was noise about a difference that costs nothing."
   - `MultiFile`/`MultiMedia` **reject a scalar** — "Here the form **is** visible" — with
     a second measurement: a reverse-view filter
     `contributors.contains(this.file.asLink())` "returns the list one and skips the
     scalar… the note vanishes from the very view meant to list it," and the diagnostic
     message names that consequence.
   This is leniency/strictness calibrated by *downstream query semantics*, not by YAML
   aesthetics — a concrete, evidence-backed precedent for how ticket 20's multi-enforcement
   (Q11: Warning for list-on-non-multi) could reason about the reverse case (scalar-on-
   multi) severity. Fileclass chose *not to flag* the case Bases cannot observe.
6. Other type checks: Number (finite, min/max), Boolean (bool or "true"/"false"), Object
   must be a plain mapping (flagging stale scalar leftovers from before a field became a
   group), ObjectList array-of-mappings, JSON text must parse if stored as string,
   Duration RFC-5545, Location "lat,lon" in range, Icon plausibility, Color CSS-shape.
   Unlisted/later-wave types are unconstrained (`default: VALID`).

## 6. Validation surfaces & the definition-side schema audit

### 6.1 Value-validation surfaces (one engine, three consumers)

The same `validateField` backs: the `fileclass-table` view's **`valid`/`errors`
validation columns** (✓/✗ per note + full messages; covers *all* root fields, not just
visible columns, and every note the class claims including folder-bound ones with no
frontmatter — main digest, `FILE: docs/content/views.md`, "Validation columns"); the CLI
`fileclass validate` (exit code 1 on any violation, CI-friendly — `FILE:
docs/content/cli.md`); and the API's `validate(scope?)` returning `Violation[]` (`FILE:
src/api/fileclassApi.ts`).

**Correction to the pre-review's "two-tier `validate()`" claim**: the `Violation` *type*
declares `severity: "error" | "warning"`, but the shipped `validate()` implementation
currently emits `severity: "error"` unconditionally (`FILE: src/api/fileclassApi.ts`,
`validate()` body). The API is two-tier-*ready*; the live two-tier (ERROR vs WARNING)
exists in the **definition-side** audit (§6.2), not in value validation. "Nothing is ever
blocked" for required violations (fields.md, "Required fields") — violations surface, but
`setValue`/`set-where` refuse invalid *writes*, and `set-where` is dry-run by default with
no-op skipping (`{changed, skipped, errors}` report).

### 6.2 The vault-wide schema-integrity sweep (pre-review claim 3 — verified, and new to the map)

`src/schema/schemaAudit.ts` (+ runner `schemaAuditRun.ts`) is a pure, dependency-injected
audit of **schema definitions against the vault** (main digest, both files; user-facing
description in `FILE: docs/content/schema.md`, "The sweep" and "The log"):

- **Finding kinds, two severities**: `missing-path` (values-note / `.base` / canvas /
  class base that no longer resolves) = ERROR; `missing-folder` (claimed folder gone) =
  ERROR; `missing-parent` (`extends` names an unknown class) = ERROR ("Inheritance fails
  whole: every field of the parent is missing, and nothing says so"); `dead-tag` (tag
  contains whitespace so the index skips it — "measured, #149") = WARNING;
  `unknown-exclude` (`excludes` names a field no ancestor declares — checked against the
  **whole ancestor chain**, after "a user hit this on a three-deep chain") = WARNING.
- **Trigger**: once per session after the first index build, on demand via command, and
  on rename events — deliberately *not* on every index rebuild ("a sweep on each would log
  the same broken path forty times while somebody edits a schema"). Motivation: renames
  that happen while the plugin is off / via file manager / via sync never deliver an
  event, so the rename-warning alone is insufficient (`schemaAudit.ts` header, issue #159).
- **Log hygiene — fingerprinted findings**: each problem gets a `kind|class|field|value`
  fingerprint; it is logged **once** when it appears, again only if it returns after being
  fixed, and its disappearance is itself logged as INFO `schema.resolved` — "the file
  reads as a record of what happened rather than a snapshot of what is wrong." Rotating
  retention (default 500 live entries, 5 archives). Three levels with a stated rule:
  ERROR = "Fileclass cannot do what a definition told it"; WARNING = "a definition that
  will never do anything, silently"; INFO = cross-file writes Fileclass performed
  (schema.md, "The log").
- **Presentation**: a dedicated log *window* (filterable, actionable links to each
  class's schema) rather than a note — ".log rather than a note on purpose: every
  markdown file in the class folder is read as a fileClass."

This is a complete, production-shipped example of **definition-side (schema-file)
diagnostics** — exactly the category ticket 20 deferred (research/20 line 156: "the
optional v2 range-less schema-file diagnostic is deferred… the watch trigger that feeds it
is owed to 31 (schema-dir `didChangeWatchedFiles`)"; ticket 31 confirms: "Schema-dir
`didChangeWatchedFiles` trigger… also lands here").

Adjacent extras, same file family: `strayFrontmatter.ts` detects YAML-looking blocks
displaced below line 1 (a real production incident: a template's stray backtick produced
double-frontmatter on every new note) — conservative detection, "never repairs" (main
digest, `FILE: src/schema/strayFrontmatter.ts`).

## 7. The live-buffer write lesson (directly relevant to the map's standing constraint)

`frontmatterSuggest.ts`'s `selectSuggestion` is the **one** surface that writes through
the live editor buffer instead of `processFrontMatter`, and the in-code rationale is a
first-hand measurement (main digest, `FILE: src/ui/frontmatterSuggest.ts`):

> "While a note is open with unsaved keystrokes — which is exactly the moment a suggester
> fires — the editor is the document. `processFrontMatter` reads the file from disk,
> writes it back, and the buffer is flushed over it a moment later: the chosen value
> disappeared, and clearing the typed text first only made the field end up empty."

To compensate, the plugin hand-rolls the YAML scalar quoting `processFrontMatter` would
have done (`yamlScalar`, `inlineListToBlock` from `FILE: src/io/frontmatterCaret.ts`).
This is an independent, production-measured corroboration of the map's standing
constraint ("unsaved editor buffers must be reflected in live language intelligence
without requiring disk writes") and of ticket 14's transient-replacement model — and a
concrete instance of the *write-side* variant of the hazard (disk write clobbered by
buffer flush), which ticket 14 covers but which until now had no external worked example.

## 8. Zero-LSP-surface counterexample value (pre-review claim 2 — verified)

Across the entire source digest (211 files) there is **no diagnostics publishing, no
schema hover provider, no go-to-definition, no references, no rename** — no LSP-shaped
surface of any kind. The word "diagnostic" does not occur in `src/`. A schema-focused
plugin with (per README, self-reported) 541 unit tests — and in any case a large vitest
suite; the digest's `tests/unit/` region alone contains ~862 `it(`/`test(` definitions
(§12 caveat) — routes every piece of validation intelligence to **non-LSP channels**:

| Intelligence | Where fileclass puts it |
|---|---|
| Per-note value violations | Table `valid`/`errors` columns; red required-icon in fields modal + Properties panel; CLI exit code; API `validate()` |
| Definition-side integrity | Sessionly sweep → fingerprinted schema log + summary Notice |
| Value completion | `EditorSuggest` popover |
| "What is this field" | Note-fields modal, schema editor, DOM indicators — never hover |
| Navigation to schema | Explicit modal links/wrench buttons — never go-to-definition |

This **strengthens ticket 08's recorded claim** ("none of the four provide real
LSP-shaped intelligence… zero diagnostics/hover/definition precedent") into a stronger
form: even the *successor* plugin, rebuilt in 2026 with a deliberate quality push
(strict TypeScript, hundreds of tests, its own CLI/API surface), still ships zero
LSP-shaped surfaces — the PKM-plugin ecosystem's answer to schema intelligence remains
modals, tables, logs, and popovers. Traces' schema diagnostics/hover/definition remain
genuinely new ground in this ecosystem, not catch-up. (Structural cause, obvious but
worth stating: Obsidian plugins are not language servers; there is no protocol surface to
publish to. Fileclass's table/CLI/API triad is what "diagnostics" looks like without a
protocol boundary — which is precisely the gap Traces' LSP fills.)

## 9. What is new vs. duplicated vs. the map

**Already held (duplicated — no map change needed)**:

- fileClass-scoped `EditorSuggest` value completion as the MM/fileclass pattern — ticket
  08 Answer; research/21 Part VII (Metadata Menu row + §6.1); research/20 §6.1.
- "Zero diagnostics/hover/definition precedent" among the four Obsidian plugins — ticket
  08; research/20's precedent table ("Modal UI only… No diagnostics, no hover, no
  go-to-definition").
- fileClass inheritance mirroring Traces' Schema module Kahn's-sort — research/20 §6.1
  (with the DAG-vs-single-parent nuance in §3 above as the only refinement).
- Empty-allowed-values = unconstrained — already *decided* as Q6; fileclass is
  corroboration, not new input.
- Frontmatter-only validation boundary — already Q1/Q20.

**New substance the digests add (candidate map input)**:

1. **A shipped definition-side schema-integrity audit** (§6.2) — the exact deferred
   category (schema-file diagnostics, ticket 20 → ticket 31's schema-dir watch), with
   production-refined severity taxonomy, once-per-problem fingerprinting, resolved-lines,
   and a "sweep, don't watch-everything" trigger model.
2. **Leniency calibrated by query visibility** (§5.5) — the Multi/MultiInput-lenient vs
   MultiFile/MultiMedia-strict split with measured Bases-semantics evidence; direct input
   for how ticket 20's Q11 multi-enforcement should treat the scalar-on-multi case.
3. **Live-buffer write clobbering, measured in production** (§7) — external
   corroboration for ticket 14's model and the map's standing constraint.
4. **Custom-date-format acceptance gap** (§5.4) — Q16's syntax-only choice is what a
   production plugin actually ships; Traces can do strictly better (chrono parseability).
5. **Platform-limitation datapoint**: fileclass wanted validity as a Bases filter and
   couldn't get one — "Bases lets a plugin register a **view** and nothing else, so
   validity is not a property its own Sort and Filter menus can see"; restating the check
   as a base formula was rejected because resolved allowed-values would "answer a
   slightly different question and the two would drift" (main digest, `FILE:
   docs/content/views.md`, "Only the rows that need attention"). A small but honest
   example of the same drift hazard Traces avoids by owning validation in one engine.
6. **Multi-class collision semantics**: last-frontmatter-declared-class-wins per key,
   global default class lowest precedence (§3) — adjacent to ticket 20's resolution
   matrix (type/required/multi/exclude), which does not currently specify multi-class
   collision order.

## 10. Pre-review claim verification summary

| # | Claim | Verdict |
|---|---|---|
| 1 | Fileclass is MM's active successor (v0.2.15 manifest/README); MM in maintenance mode; don't run both; same `EditorSuggest`-in-frontmatter completion pattern | **Verified** (README, manifest, positioning.md, frontmatterSuggest.ts header). Nuance: value-only suggester; no key-name completion. |
| 2 | 541 passing tests, zero LSP-shaped surfaces; validation via table view, CLI, `validate()` API, DOM-injected nav icons | **Verified in substance**. "541" is a self-reported README figure my digest count does not reproduce (~862 test definitions in `tests/unit/` alone — see §12); "zero LSP surfaces" is exact; all four alternative channels confirmed. |
| 3 | Vault-wide schema-integrity sweep (missing paths, dead tags, dangling `extends`); type-dependent scalar-vs-list shape checking (lenient Multi/MultiInput, strict MultiFile/MultiMedia) | **Verified** (schemaAudit.ts, schema.md, views.md, validate.ts) with measured justifications in-code. |
| — | Empty allowed values = unconstrained (Q6) | **Verified** (`validateInList`). |
| — | Date custom formats unchecked (Q16) | **Verified** (`validateDatePattern`), with the dependency-gap nuance. |
| — | No file-existence checking (Q15) | **Verified for note values**; definition-side paths ARE checked (§5.3 distinction). |
| — | `validate() -> Violation[]` two-tier shared CLI/API | **Corrected**: type is two-tier-ready (`"error" \| "warning"`); implementation emits only `"error"`. Real two-tier lives in the definition-side audit. |
| — | Writes go through live editor buffer when note open | **Verified** — with the important nuance that this is the *sole* editor-buffer write surface; everywhere else is `processFrontMatter` (which is exactly why the one exception needed a measured justification). |
| — | (Prompt's digest-gap note) "docs-digest is thin; docs content lives in main digest" | **Corrected**: docs-digest holds the *complete* docs site prose (all 10 pages + layouts); it is a subset duplicate of the main digest, not thin. The main digest is the full-repo superset. |

## 11. Implications for tickets 08 / 20 / 31 (implications only — no decisions)

**Ticket 08 (recorded via research/21 Part VII)**: its "zero diagnostics/hover/definition
precedent" claim can be optionally strengthened with the fileclass datapoint — the
MM-successor, rebuilt 2026 with a quality mandate, still ships none; its validation
delivery triad (table columns / CLI exit code / API) is the ecosystem's non-LSP answer.
Nothing in 08's recorded findings needs revision.

**Ticket 20 (schema/fileClass intelligence, resolved)**: fileclass is independent
production corroboration of Q6 (empty values = unconstrained), Q15 (no note-value file
existence), Q16 (syntax-only dates), and the frontmatter-only boundary (Q1/Q20) — all
settled decisions hold. Two items the digests put on the table for the eventual
implementation spec (not reopened here): (a) the §5.5 scalar-vs-list severity evidence
for Q11's multi enforcement — fileclass's measured "flag only what queries can observe"
rule is a defensible calibration point; (b) multi-class collision order (last-wins) is
specified in the incumbent plugin but not in 20's resolution matrix. The §6.2 audit is a
worked reference design for the v2 `schema-file.*` diagnostics 20 explicitly deferred.

**Ticket 31 (config integration; holds the schema-dir `didChangeWatchedFiles` trigger)**:
the schema audit's trigger model (once-per-session + on-demand + event-driven, explicitly
*not* per-rebuild) and its fingerprint/dedup/rotation log hygiene are a production-tested
shape for what schema-dir watching should produce — findings that don't drown the log
when a user edits a schema note repeatedly. Also relevant to 25's diagnostics
architecture by contrast: fileclass persists definition-side findings across sessions in
a file with dedup, whereas 25's model is ephemeral per-document publish — the two tiers
(value vs definition) plausibly want different lifetimes.

## 12. Caveats & limitations

- **Digest-synthesis scope**: no live source archaeology beyond the README fetch; if a
  specific `validate.ts`/`schemaAudit.ts` behavior becomes load-bearing for a ticket's
  exact wording, spot-check the live repo (the README was verified byte-equivalent on
  2026-10-21, which is evidence of currency but not a guarantee for every file).
- **Self-reported figures**: "541 unit tests" and "44 videos" are README claims. My count
  of line-start `it(`/`test(` in the digest's `tests/unit/` region is ~862 — the README
  number may be stale, differently scoped, or rounded; treat "hundreds of unit tests" as
  the supported claim and "541" as unverified precision. `docs/data/videos.json` contains
  47 entries vs. the README's 44 — consistent with a slightly stale prose figure.
- **Digest gaps**: no star counts, release dates, issue traffic, or download numbers in
  any digest (the repo *has* a release-stats script, but its output isn't committed).
  Web-checked separately (§2.3): 76 stars / 4 forks / 28 issues / 217 commits
  (2026-10-21); MM "maintenance mode" web-confirmed; MM's "~700+ stars" is from a
  synthesized search summary, not a fetched page — directional only.
- **Docs-digest characterization corrected** (§10): it is a complete docs-prose subset,
  not thin; the main digest duplicates everything.
- **The `.claude/docs/ARCHITECTURE.md` referenced throughout CLAUDE.md is not in the
  digest** (only `TRIAGE.md` is) — design-decision IDs (D1–D8, §7, §10, §19.4) are
  cited in source headers but their text is unavailable; claims relying on them here rest
  on the source headers' own summaries.
- **CLI/TUI repo not digested**: `mdelobelle/fileclass-cli` is a separate repository;
  its implementation is unseen — CLI behavior here is documented from `docs/content/cli.md`
  only.
- **Bases internals** are behind the external `obsidian-bases-adapter` package (its own
  repo, not digested); all Bases-interaction claims are at the Fileclass API boundary.
