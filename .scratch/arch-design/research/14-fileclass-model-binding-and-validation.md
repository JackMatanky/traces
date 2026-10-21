# Research: Fileclass Model, Binding, and Validation

- Ticket: 14-research-fileclass-model-binding-and-validation.md
- Repo: `mdelobelle/fileclass` (TypeScript Obsidian plugin; Metadata Menu successor)
- Sources: `/tmp/fc/` (src, tests, docs/content, demo/ROADMAP.md, CLAUDE.md)
- Context: Informs Traces schema redesign (tickets 18, 19, 20, 27, 34, 35, 36)
- Tags: [V] verified from source/tests, [H] architectural interpretation

---

## 1. What Fileclass Actually Does

### Class Definition & Inheritance

Fileclass stores schemas in Markdown notes within a designated folder
(`settings.classFilesPath`) [V: src/schema/fileClass.ts:145]. Schema
configuration lives entirely in YAML frontmatter [V: docs/content/schema.md:13]:

- `extends`: Single inheritance link to parent class [V:
  src/schema/inheritance.ts:13]. Multiple parents are unsupported [V:
  src/schema/inheritance.ts:19]. Ancestor chains are cycle-guarded via a
  visited set in `computeAncestors`, terminating on cycles or missing parents
  [V: src/schema/inheritance.ts:20].
- `excludes`: String array of inherited field names dropped down the chain;
  exclusions accumulate walking towards ancestors in `resolveInheritedFields`
  [V: src/schema/inheritance.ts:54].
- `fields`: Array of `{ name, id, type, options, path }` [V:
  src/schema/field.ts:53]. Nested fields encode ancestry via `____` separated
  IDs in `path` [V: src/schema/field.ts:8,59].
- Field resolution runs root-to-leaf: ancestors first, child last [V:
  src/schema/inheritance.ts:64]. Redefinitions override ancestor definitions in
  place, keeping the ancestor's sequence slot [V:
  src/schema/inheritance.ts:80]. Identity key is `${field.path}\0${field.name}`
  [V: src/schema/inheritance.ts:99].
- `fieldsOrder`: Explicit ordering list stored on the class note [V:
  src/schema/fieldOrder.ts:10]. Supports level-qualified names
  (`editions.format`) and falls back to legacy field IDs for Metadata Menu
  migration [V: src/schema/fieldOrder.ts:28,42].

### Note-to-Class Binding & Multi-Class Resolution

`resolveBinding` matches notes against candidate classes in `FileBinding` [V:
src/schema/resolver.ts:200].

- Matching sources in `collectBoundNames` [V: src/schema/resolver.ts:107-147]:
  1. Frontmatter alias (`innerNames`, e.g. `fileClass: [Book, Article]`)
  2. Tags (`tagAncestry` includes tag hierarchy `#author/french` -> `#author`,
     folded to lowercase)
  3. Folder path prefix (`folderPath.startsWith(prefix)`)
  4. Bookmark groups (`bookmarkBindings`)
  5. Base view membership (`baseViewNames`)
- Global fileClass: Prepended as baseline (`[global, ...own]`) [V:
  src/schema/resolver.ts:204]. If note has no own classes, source is `"global"`;
  otherwise `"fileClass"` [V: src/schema/resolver.ts:211].
- Multi-class collision rule: `mergeFields` loops `for (const name of names)`
  and inserts fields into a Map keyed by `fieldKey` [V:
  src/schema/resolver.ts:172-180]. Last encountered class overwrites earlier
  ones [V: src/schema/resolver.ts:163]. Winner takes loser's position [V:
  src/schema/resolver.ts:169].
- Discrepancy between documentation and code [V by code reading; not pinned
  by any test]: docs say sources "are combined in this priority order"
  (`frontmatter > tag > path > bookmark > base view > preset`)
  [docs/content/schema.md:134-152; src/schema/resolver.ts:103-106].
  `collectBoundNames` pushes every match into `names` in that order, with no
  suppression [src/schema/resolver.ts:119-145], and `mergeFields` lets the
  last name win a shared key [src/schema/resolver.ts:172-180]. So on a
  collision the lowest-ranked source (a base view, then bookmark, path, tag)
  overrides a frontmatter class. Whether that is intended ("priority" as a
  label order only) is unknown [H]; `tests/unit/resolver.test.ts:49-58` pins
  the order and de-duplication but declares no contested key, and `:71-78`
  pins last-wins only within a frontmatter list.

### Typed Field Model & Options

AST defines 28 enum strings in `FIELD_TYPES` [V: src/schema/field.ts:15-44]:

- 22 supported input types (`SUPPORTED_INPUT_TYPES`) [V:
  src/fields/support.ts:18-38]: `Input`, `Number`, `Date`, `DateTime`, `Time`,
  `Boolean`, `Select`, `Cycle`, `Multi`, `MultiInput`, `Duration`,
  `CycleDuration`, `Location`, `Icon`, `Color`, `File`, `MultiFile`, `Media`,
  `MultiMedia`, `Object`, `ObjectList`, `JSON`, `YAML`.
- 3 canvas types: `Canvas`, `CanvasGroup`, `CanvasGroupLink` (read-only AST
  sync from `.canvas` files, no input modals) [V:
  src/fields/canvas/canvasEngine.ts:1-20].
- 2 dead stubs: `Formula`, `Lookup` (unsupported, delegated to Bases; retained
  only to parse legacy Metadata Menu files without error) [V:
  demo/ROADMAP.md:15; docs/content/positioning.md:42].
- Thin types: `Cycle` is identical to `Select` in storage and validation [V:
  src/fields/validate.ts:150]; `CycleDuration` is array of `Duration` strings
  [V: src/fields/validate.ts:183]; `DateTime`/`Time` differ from `Date` only by
  regex/format defaults [V: src/fields/dateFormats.ts:23-25].
- Value sources: `ValuesList` (inline array or index map), `ValuesListNotePath`
  (line-by-line note read), `ValuesFromBase` (rows from `.base` view),
  `ValuesFromDVQuery` (ignored legacy Dataview stub) [V:
  src/fields/options.ts:78-82; src/fields/valuesIo.ts:28-57].
- Dependent candidates: `conditional.ts` compiles dependent pickers into Bases
  filter formulas (`matchFormula`:
  `${left}.isTruthy() && ${right}.isTruthy() && (${compare})`) [V:
  src/fields/conditional.ts:96-103].

### Validation Semantics & Gating

- `validateField` checks raw frontmatter values [V: src/fields/validate.ts:120].
- Required: `isEmpty(v)` is strictly `v == null || v === ""` [V:
  src/fields/validate.ts:58]. `0` and `false` are valid [V]. Flaw: empty lists
  `[]` evaluate to `isEmpty([]) === false` and pass `isRequired` [V:
  src/fields/validate.ts:58, 125].
- Number: validates `min` and `max` [V: src/fields/validate.ts:71-73]. `step` is
  NOT validated; `stepNumber` is exclusively a UI increment helper in modals [V:
  src/fields/numberStep.ts:51-61].
- Dates: Custom format bypasses format validation (any non-empty string is
  valid) [V: src/fields/validate.ts:97-99]. Wikilinks accepted [V:
  src/fields/validate.ts:95]. Only standard ISO formats are regex-checked [V:
  src/fields/validate.ts:62-64]. `dateFormatCheck.ts` lints Moment tokens, not
  date values [V: src/fields/dateFormatCheck.ts:55].
- Duration: Validates RFC 5545 duration string `P[nW]|P[nD][T[nH][nM][nS]]` [V:
  src/fields/duration.ts:20,46]. Years and months forbidden [V:
  src/fields/duration.ts:5].
- Coercion: Scalars accepted for `MultiInput` and `Multi` [V:
  src/fields/validate.ts:133,153]; `MultiFile` strictly requires array to
  satisfy Bases reverse-link queries [V: src/fields/validate.ts:175].
- Gating: UI is advisory (red indicator, tooltip) [V: docs/content/ui.md:120].
  API `setValue` hard gates (`!result.ok` aborts write) [V:
  src/api/fileclassApi.ts:526]. CLI `validate` exits 1 on violations [V:
  src/api/fileclassApi.ts:508; docs/content/cli.md:98].

### Write Safety, Cache & Invalidation

- Frontmatter-only: reads via `metadataCache`, writes exclusively via
  `processFrontMatter` [V: CLAUDE.md:15]. Never modifies markdown body text [V].
- In-place mutation: `objectPath.ts` mutates JS object in place, preserving
  unmapped keys [V: src/engine/objectPath.ts:11-13].
- Race-free insertion: `insertMissingFields` determines missing fields inside
  the `processFrontMatter` callback from live YAML, avoiding stale cache
  overwrites after template execution [V:
  src/commands/insertMissingFields.ts:51-64].
- Frontmatter reordering: `reorderFrontmatter` snapshots entries, deletes keys,
  and reassigns in target order, leaving values by reference [V:
  src/io/reorderFrontmatter.ts:45-58]. Integer-like keys (`2024`) cannot be
  positioned due to YAML parser sorting [V: src/schema/reorder.ts:26,95].
- Rename migration: `migrateRenamedField` scans vault via `notesCarrying`, opens
  confirmation modal, updates frontmatter across notes, and rewrites
  `fieldsOrder` in declaring and descendant classes [V:
  src/commands/renameFieldMigration.ts:37-87,119].
- Query cache: `queryCache.ts` memoizes promise-based Base scans keyed by
  `(basePath, subKey)`, invalidated on vault modify events for `.base` files [V:
  src/engine/queryCache.ts:21-52].

---

## 2. Invariants and Owners

- Pure Schema Domain: Modules `schema/`, `engine/objectPath.ts`,
  `fields/validate.ts`, `fields/duration.ts`, `fields/numberStep.ts` have zero
  Obsidian imports and run in pure Vitest [V: vitest.config.ts].
- Single-Inheritance Acyclic Chain: `computeAncestors` enforces a single parent
  chain and aborts on cycles or unknown parents [V:
  src/schema/inheritance.ts:20].
- Identity by Name and Level: Collisions and overrides are keyed by
  `${field.path}\0${field.name}`, ensuring nested fields (`editions.publisher`)
  never collide with root fields (`publisher`) [V:
  src/schema/inheritance.ts:99; src/schema/resolver.ts:157].
- Write Isolation: All writes happen through single `processFrontMatter` calls,
  preserving unknown frontmatter properties [V: src/io/write.ts:2-6].
- Template Sequence Order: Templates apply first via raw modify/Templater; class
  frontmatter fields merge second via `processFrontMatter` [V:
  docs/content/schema.md:261-265].

---

## 3. Tradeoffs and Costs

- External View Delegation: Delegating queries and candidate lists to Obsidian
  Bases eliminated Dataview baggage but coupled relationship resolution to
  Obsidian UI runtime [V: docs/content/positioning.md:23-31].
- Binding order versus override order: see section 1; the code lets a lower
  ranked binding source override a frontmatter class on a collision, and no
  test covers it [V for code, H for intent].
- Weak Validation for Custom Dates: Deferring to custom Moment formats forces the
  validator to accept any non-empty string, degrading schema validation to a
  presence check [V: src/fields/validate.ts:97-99].
- Incomplete Required Check: `isEmpty` does not test array length, permitting
  empty lists `[]` on required multi-fields [V: src/fields/validate.ts:58].
- No Body Validation: Strict frontmatter boundary leaves note bodies, headings,
  and code blocks completely unvalidated (addressed only by heuristic
  `strayFrontmatter` detection) [V: src/schema/strayFrontmatter.ts:45].

---

## 4. Transferable Lessons for Traces

- Frontmatter In-Place Mutation: Preserving unknown keys by modifying the parsed
  AST/map in place and round-tripping scalars by reference prevents YAML
  formatting loss [V/H].
- In-Place Inheritance Override: Overriding fields at the ancestor's position
  rather than appending to the end preserves logical document layout [V/H:
  src/schema/inheritance.ts:80].
- In-Write Missing Field Check: Deciding missing fields inside the write callback
  against live frontmatter prevents race conditions with template expansion
  [V/H: src/commands/insertMissingFields.ts:51].
- Level-Qualified Keys: Qualifying field identity by nesting level
  (`path\0name`) avoids false collisions between object properties and root
  properties [V/H: src/schema/inheritance.ts:99].
- Descendant Field Order Tracking: Renaming a field must update `fieldsOrder`
  across all descendant classes, not just the declaring class [V/H:
  src/commands/renameFieldMigration.ts:119].
- Explicit Origin Provenance: Tracking why a class applies (`BindingOrigin`:
  `#tag`, `/path`, `*bookmark`, `frontmatter`, `global`) provides transparent
  diagnostics [V/H: src/schema/resolver.ts:52-85].

---

## 5. Lessons That Do NOT Transfer and Why

- Last-Wins Resolution: In Traces' multi-consumer environment (CLI, LSP, MCP),
  silent last-wins creates non-deterministic collisions across unrelated
  classes. Traces' order-independent `schema.class-conflict` error is strictly
  safer [H].
- Unchecked Custom Date Formats: Fileclass abandons date validation when custom
  formats are used. Traces can parse and validate structured temporal formats
  using a dedicated parser crate [H: ticket 27].
- Delegation to Host GUI: Fileclass relies on Obsidian Bases for foreign keys,
  lookups, and reverse relations. Traces must own relationship queries within
  its core query engine [H].
- Dynamic String Coercion: Fileclass's `filter.ts` coerces all types to strings
  for comparison (`equals`, `contains`). Traces uses a typed `NoteFieldValue`
  and strict adaptive equality [H: tickets 18, 19].
- Frontmatter-Only Restriction: While frontmatter isolation simplifies write
  safety, Traces requires body section schemas (ticket 36) for structured notes
  [H].

---

## 6. Impact on Traces Decisions and Round-4 Ledger

### Impact on Traces Decisions

- Ticket 20 (Schema Typing): Affirms global layer as lowest-precedence baseline,
  `excludes` accumulating towards ancestors, in-place inheritance ordering, and
  `origin` provenance tracking. Refines `required` (Traces must keep its
  stricter check that catches `[]`). Affirms that `date.format` is a write hint
  rather than strict check unless supported by a native temporal engine.
  Confirms that explicit frontmatter classes MUST suppress path/tag bindings to
  avoid Fileclass's inverted precedence bug.
- Ticket 19 & 18 (Coercion & Value Model): Fileclass accepts scalars for
  multi-text fields (`MultiInput`) but forbids them for relational multi-links
  (`MultiFile`). Traces should retain explicit list types in AST while allowing
  scalar coercion in queries.
- Ticket 35 (Schema Internals): Prune thin/redundant types. Of Fileclass's 28
  types, only ~10 represent unique structural data types; `Cycle`,
  `CycleDuration`, `Location`, `Icon`, `Color` are UI presentation concerns.
- Ticket 34 & 36 (Live Buffers & Body Schemas): Fileclass has zero live buffer
  synchronization or body validation; provides no precedent for tickets 34/36.

### Round-4 Claims Verification Ledger

| Round-4 Claim | Status | Source Evidence & Correction |
|---|---|---|
| Last-listed class wins | CONFIRMED [V] | `src/schema/resolver.ts:172-180`. `mergeFields` overwrites earlier classes on identical `${field.path}\0${field.name}`. |
| Single inheritance via `extends`; cycle-guarded | CONFIRMED [V] | `src/schema/inheritance.ts:13-25`. Single parent link; visited set terminates cycles. |
| `excludes` drops inherited fields | CONFIRMED [V] | `src/schema/inheritance.ts:54-59`. Accumulates down chain towards ancestors. |
| Binding priority: frontmatter > tag > path > bookmark > global | CORRECTED [V code, H intent] | `src/schema/resolver.ts:119-145, 172-180`. The docs list this priority, but all matches are collected without suppression and last-wins applies, so a path or bookmark class overrides a frontmatter class on a shared key. Global is correctly placed first as baseline. Untested. |
| Advisory in UI, hard gating on write API (`setValue`) and CLI (`validate` exits 1) | CONFIRMED [V] | `src/api/fileclassApi.ts:506-515, 526-530`; `docs/content/cli.md:98, 105`. `setValue` aborts on validation failure; CLI exits 1. |
| 26 field types supported | CORRECTED [V] | `src/schema/field.ts:15-44`; `src/fields/support.ts:18-38`. AST has 28 types; `Formula` and `Lookup` are dead stubs; 3 `Canvas` types lack input modals; 22 types are in `SUPPORTED_INPUT_TYPES`. |
| Numeric `step` validated | CORRECTED [V] | `src/fields/validate.ts:66-75`; `src/fields/numberStep.ts:51-61`. `step` is never validated in `validateField` (only min/max); it is purely a UI increment helper in modals. |
| `required` checks presence | CORRECTED [V] | `src/fields/validate.ts:58-60, 125`. `isEmpty` checks `null` and `""`. Numbers (`0`) and booleans (`false`) pass, but empty array `[]` erroneously passes `isRequired`. |
| Custom date formats validated | CORRECTED [V] | `src/fields/validate.ts:97-100`. Custom format bypasses validation (accepts any non-empty string). `dateFormatCheck.ts` only lints Moment tokens. |
| Frontmatter-only; unknown keys preserved | CONFIRMED [V] | `CLAUDE.md:15`; `src/io/write.ts:18-33`; `src/engine/objectPath.ts:11-13`. Writes use `processFrontMatter`; unknown keys preserved in-place. |
