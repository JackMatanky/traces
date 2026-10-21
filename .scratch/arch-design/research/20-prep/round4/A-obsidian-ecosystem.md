# Obsidian Ecosystem Precedent: Note Schemas, Typing, and Frontmatter Validation

Topic: Comprehensive audit of note typing, schema definitions, validation,
and property management across the Obsidian ecosystem, community plugins,
static site generators, and user workflows.
Status: Input to Ticket 20 (Schema Note Typing and Validation) stress test.
Deliverable:
`.scratch/arch-design/research/20-prep/round4/A-obsidian-ecosystem.md`
Constraints: Lines <= 80 columns (except tables and URLs); fenced code blocks
use language `text`; no emojis; claims tagged [V] (verified with URL) or [H]
(hypothesis/inference).

---

## 1. Scope

This audit examines 12 real projects, core subsystems, plugins, and tools
spanning the Obsidian and markdown PKM ecosystem to evaluate how note typing,
frontmatter schemas, inheritance, and validation are implemented in practice.

Projects and subsystems examined:

1. Obsidian Core Properties and `types.json` [V: read official documentation,
   forum incident threads, and release changelogs].
2. Obsidian Bases [V: read official syntax specification, release changelog
   v1.9.0/v1.9.10, and forum feature discussions].
3. Metadata Menu (`mdelobelle/metadatamenu`) and Fileclass
   (`mdelobelle/fileclass`) [V: read official documentation, GitHub READMEs,
   positioning guides, and author's architectural redesign notes].
4. Obsidian Dataview (`blacksmithgu/obsidian-dataview`) and Datacore
   (`blacksmithgu/datacore`) [V: read codebase digests, Datacore roadmap, and
   performance architecture notes].
5. Obsidian Templater (`SilentVoid13/Templater`) and Core Templates [V: read
   source digests, modal prompt implementations, and user workflows].
6. Obsidian Linter (`platers/obsidian-linter`) [V: read rule documentation,
   regex parser caveats, and issue tracker reports on comment loss].
7. Breadcrumbs (`michaelpporter/breadcrumbs`, formerly `SkepticMystic`) and
   Juggl (`HEmile/juggl`) [V: read repository READMEs, edge field schemas, and
   graph relationship documentation].
8. Supercharged Links (`mdelobelle/obsidian-supercharged-links`) [V: read
   documentation and CSS custom property data injection patterns].
9. Obsidian Note Schemas (`Aguila1989/obsidian-note-schemas`) [V: read source
   repository README, schema specification, and autofix engine].
10. Metadata Validator (`flowing-abyss/obsidian-metadata-validator`) and
    Metadata Enforcer (`omarish/metadata-enforcer`) [V: read package manifests,
    release announcements, and author post-mortems on JSON Schema].
11. Notion-like Vault Databases: Database Folder (`RafaelGB/obsidian-db-folder`)
    and Projects (`marcusolsson/obsidian-projects`) [V: read repositories,
    column type models, and July 2025 archival notices].
12. Quartz (`jackyzha0/quartz`) and Obsidian Publish [V: read Quartz v4/v5
    `frontmatter.ts` transformer source and Obsidian Publish documentation].

---

## 2. Per-Project Deep Dives

### 2.1 Obsidian Core Properties and `types.json`

- What it is: Introduced in Obsidian v1.4.0 (August 2023), Properties replaced
  raw YAML editing with a structured visual UI driven by a per-vault
  configuration file located at `.obsidian/types.json` [V:
  <https://obsidian.md/help/properties>].
- Typing and validation: Supported types are `text`, `list`, `number`,
  `checkbox`, `date`, and `datetime`. The type registry is strictly vault-global
  and flat. Obsidian infers property types dynamically from values unless a type
  is explicitly configured in `types.json` [V:
  <https://forum.obsidian.md/t/not-all-properties-present-in-types-json/96646>].
- Validation timing: Fully advisory. When a value conflicts with the declared
  vault type (e.g., text found where `number` was configured), a warning icon
  and the tooltip "Type mismatch, expected <Type>" appear [V:
  <https://forum.obsidian.md/t/property-type-and-type-mismatch/113071>]. The
  file is never locked or prevented from saving. Clicking the warning opens a
  modal offering to convert the value or change the vault-wide property type
  [V].
- Storage and parsing: Serialized to YAML frontmatter via `processFrontMatter`.
  The visual editor normalizes YAML, dropping comments and reformatting flow
  lists into multiline block sequences [V:
  <https://forum.obsidian.md/t/some-issues-with-properties-and-bad-yaml/67216>].
- User complaints and regrets:
  1. Flat global namespace: A property named `status` shares the same type and
     autocomplete suggestions vault-wide. A user tracking books (`To Read`) and
     movies (`To Watch`) gets cross-pollinated suggestions or type conflicts [V:
     <https://forum.obsidian.md/t/expand-types-json-with-display-name-and-suggestion-options/101456>].
  2. Sync collisions: External synchronization (Git, Syncthing) frequently
     creates file conflicts on `.obsidian/types.json`
     (`types.sync-conflict-*.json`),
     reverting property types across devices [V:
     <https://forum.obsidian.md/t/i-have-a-lot-of-json-conflict-files-on-my-obsidian-folder/66795>].
  3. Bulk repair absence: Changing a property type in `types.json` leaves
     hundreds
     of existing notes with type mismatch warnings, with no native automated
     migration or batch repair tool [V:
     <https://forum.obsidian.md/t/fix-properties-in-bulk-when-the-property-type-changes/81102>].
- Key lesson: A global property namespace without note-type scoping creates
  severe collision pain for multi-domain vaults. Validation must remain
  non-blocking, but bulk remediation tooling is essential when types change.

### 2.2 Obsidian Bases (`.base` YAML format)

- What it is: A core plugin introduced in Obsidian v1.9.0 (Early Access May
  2025,
  Public v1.9.10 August 2025) providing native database-like tabular views,
  formulas, filtering, and summarization directly over markdown notes [V:
  <https://obsidian.md/changelog/2025-05-21-desktop-v1.9.0/>].
- Architecture and syntax: Stored in plain-text `.base` files using YAML syntax
  [V: https://obsidian.md/help/bases/syntax]. A base defines global or
  view-level `filters` (`and`, `or`, `not`, functions), `formulas` (calculated
  expressions like `note.price * 1.2`), `properties` (column display names),
  `summaries` (aggregations like `Average`, `Sum`, `Earliest`), and `views`
  (table, cards, list).
- Typing and validation: Distinguishes between `note` properties (frontmatter
  keys), `file` properties (`file.ctime`, `file.mtime`, `file.size`, `file.ext`,
  `file.links`), and `formula` properties. Formula outputs are dynamically typed
  based on expressions. Bases does not enforce a write-time schema or validate
  incoming notes; invalid formula expressions or missing properties evaluate to
  `null` or empty cells rather than crashing the view [V].
- Relationship to notes: Read-only query engine with in-cell editing for note
  properties. Bases does not own schemas or note types; it relies on frontmatter
  properties and Obsidian core's property infrastructure [V:
  <https://mdelobelle.github.io/fileclass/positioning/>].
- User complaints and regrets: Users immediately attempted to use Bases views as
  de facto schema linter dashboards (e.g. creating filtered views where
  `status.isEmpty()` or invalid properties are isolated for manual correction)
  because Obsidian lacks native schema validation [V:
  <https://forum.obsidian.md/t/custom-property-restrictions-and-types/107050>].
- Key lesson: Separating the query/view engine (Bases) from the
  schema/validation
  layer allows each to evolve cleanly. Tabular database tools work best as
  consumers of structured frontmatter rather than monolithic schema managers.

### 2.3 Metadata Menu & Fileclass

- What it is: Metadata Menu (`mdelobelle/metadatamenu`) pioneered `fileClass`
  note typing in Obsidian. In late 2024 / 2025, author `mdelobelle` completely
  rebuilt the architecture from scratch into `fileclass`
  (`fronglemog/obsidian-fileclass`),
  a strict frontmatter-only successor powered by Obsidian Bases [V:
  <https://github.com/fronglemog/obsidian-fileclass/blob/main/README.md>;
  <https://mdelobelle.github.io/fileclass/positioning/>].
- Domain model and field types: Supports 26 field types including `Input`,
  `MultiInput`, `Number`, `Boolean`, `Select`, `Cycle`, `Multi`, `Date`,
  `DateTime`, `Time`, `Duration`, `CycleDuration`, `Location`, `Icon`, `Color`,
  `File`, `MultiFile`, `Media`, `MultiMedia`, `Object`, `ObjectList`, `JSON`,
  `YAML`, `Canvas`, `CanvasGroup`, and `CanvasGroupLink` [V:
  <https://mdelobelle.github.io/fileclass/fields/>].
- Inheritance model: Single inheritance via `extends: ParentClass`. Ancestor
  chains are cycle-guarded. Child classes can explicitly drop inherited fields
  using `excludes: [fieldA, fieldB]`. Field order across the inheritance chain
  can be explicitly overridden in the child class via `fieldsOrder` [V:
  <https://mdelobelle.github.io/fileclass/schema/>].
- Multi-class note binding and conflict rule: Notes bind to fileClasses via:
  1. Frontmatter attribute: `fileClass: [ClassA, ClassB]`
  2. Tag match: `mapWithTag` or `tagNames` (including nested tag hierarchies)
  3. Folder path match: `filesPaths`
  4. Bookmark group match: `bookmarksGroups`
  5. Base view query match
  6. Global fallback fileClass [V:
     <https://mdelobelle.github.io/fileclass/schema/>] Crucial conflict rule:
     When multiple classes declare the same field key,
     **the LAST-listed class wins** (`fileClass: [Book, Article]` treats
     `Article` as an overlay on top of `Book`) [V:
     <https://mdelobelle.github.io/fileclass/schema/>].
- Validation timing and gating: Advisory and non-blocking in the UI. A required
  field missing a value turns its property icon red and displays "required" in
  tooltips, but saving, closing, or navigating away is never blocked [V:
  <https://mdelobelle.github.io/fileclass/fields/#required-fields>]. Hard gating
  exists strictly on the programmatic write API (`setValue` rejects empty
  writes on required fields) and CLI (`fileclass validate` exits with non-zero
  status) [V].
- Architectural pivot from Metadata Menu to Fileclass: Metadata Menu coupled
  deeply to Dataview (and Dataview inline fields `key:: value`). This caused
  fragile regex patching of markdown text, race conditions during file saves,
  and severe parse-time slowdowns. Fileclass deliberately dropped Dataview,
  inline fields, and computed fields (`Lookup`, `Formula`), becoming strictly
  frontmatter-only via Obsidian's `processFrontMatter` API and delegating
  queries
  to Bases [V: https://mdelobelle.github.io/fileclass/positioning/].
- Key lesson: Frontmatter-only operations via native AST APIs eliminate data
  corruption. The last-listed class override model matches how users
  conceptualize
  compositional layering. Hard write-gating belongs only to programmatic/agent
  interfaces, not the human editor.

### 2.4 Obsidian Dataview and Datacore

- What it is: Dataview (`blacksmithgu/obsidian-dataview`) is the standard query
  engine for Obsidian notes. Datacore (`blacksmithgu/datacore`) is its planned,
  high-performance successor [V: https://github.com/blacksmithgu/datacore].
- Data ingestion and typing: Schema-free dynamic type inference. Dataview runs
  a Parsimmon parser cascade during background worker sweeps:
  `null` -> array -> Date -> Object -> Number/Boolean -> regex date -> regex
  duration -> embed link -> fallback string [V:
  `obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3650-3693`]. Inline
  fields use `[key:: value]` or `(key:: value)`.
- Gating and validation: Entirely non-gating. No syntax diagnostics are emitted
  for unexpected types; values that fail date/duration parsing remain plain
  strings.
- Regrets and architectural shift: Dataview's inline field parsing caused
  catastrophic performance bottlenecks in large vaults (millions of regex
  evaluations over entire document bodies) [V]. In Datacore, author
  `blacksmithgu` officially disabled inline fields by default, moved to
  throttled background worker pools, switched persistence from IndexedDB to
  LevelDB/LowDB due to hard metadata storage caps, and endorsed standard YAML
  frontmatter and block-level YAML over inline syntax [V:
  <https://github.com/blacksmithgu/datacore/blob/master/ROADMAP.md>].
- Key lesson: In-text inline field parsing (`key:: value`) across note bodies
  is an architectural dead end for large knowledge bases. Structured metadata
  must be confined to frontmatter or explicit code blocks.

### 2.5 Obsidian Templater and Core Templates

- What it is: Templater (`SilentVoid13/Templater`) provides JavaScript-driven
  template execution during note creation, while core Templates performs string
  replacement [V: https://obsidian.md/help/plugins/templates].
- Type concept: No persistent note schema. Typing exists strictly as write-time
  interactive input modals: `tp.system.prompt` (string), `tp.system.suggester`
  (single enum selection), `tp.system.multi_suggester` (multi-select) [V:
  `obsidian_silentvoid13-templater-src-digest.txt:2986-3085`].
- Folder templates: Notes created within a specific folder automatically execute
  that folder's designated template.
- Pitfalls:
  1. Once rendered, notes have zero structural ties to their origin template.
     Editing the template never updates existing notes [V:
     <https://forum.obsidian.md/t/super-fr-enhance-obsidian-with-a-type-system-for-notes-and-database-like-views-metadata-object-oriented-model/46444>].
  2. Unquoted template variables in frontmatter (e.g. `date: {{date:YYYYMMDD}}`)
     cause Obsidian's visual property editor to corrupt the YAML on save,
     converting syntax into broken mapping keys [V:
     <https://forum.obsidian.md/t/the-template-property-is-breaking-when-i-am-using-the-template/114030>].
  3. Race conditions: When Templater and other plugins both write frontmatter
     on note creation, empty write passes frequently wipe out newly generated
     frontmatter [V:
     <https://forum.obsidian.md/t/frontmatter-added-by-templater-is-immediately-overwritten-on-new-note-creation-goal/103207>].
- Key lesson: Templates solve the "creation" problem but fail the "lifecycle"
  problem. Schemas cannot be modeled merely as templates; they must exist as
  persistent definitions that validate existing notes.

### 2.6 Obsidian Linter

- What it is: Linter (`platers/obsidian-linter`) is an automated formatting
  tool executed on save, file change, or command [V:
  <https://github.com/platers/obsidian-linter>].
- Frontmatter capabilities: Reorders YAML keys according to a prioritized
  and alphabetical list (`yaml-key-sort`), formats arrays as inline or
  multiline,
  deduplicates list values, and escapes special characters [V:
  <https://platers.github.io/obsidian-linter/settings/yaml-rules/>].
- Architecture and bugs: YAML rules are implemented primarily via regex and line
  heuristics rather than a lossless YAML AST parser [V:
  <https://platers.github.io/obsidian-linter/settings/yaml-rules/>].
  Consequently:
  1. YAML comments inside values or arrays are stripped or cause sorting
     failures
     [V: https://github.com/platers/obsidian-linter/issues/715].
  2. Blank lines between YAML properties are stripped [V:
     <https://github.com/platers/obsidian-linter/issues/884>].
  3. Conflicts with Obsidian Properties: Linter auto-fixes fight Obsidian's
     native `processFrontMatter` serialization, resulting in churn loops [V:
     <https://github.com/platers/obsidian-linter/issues/1563>].
- Key lesson: Frontmatter reordering and formatting must use an AST/CST parser
  with strict round-trip fidelity. Regex-based YAML manipulation invariably
  causes
  comment destruction and edge-case corruption.

### 2.7 Breadcrumbs and Juggl

- What it is: Breadcrumbs (`michaelpporter/breadcrumbs`, v4.15+) and Juggl
  (`HEmile/juggl`) add typed directed relationships between notes [V:
  <https://github.com/michaelpporter/breadcrumbs/blob/main/README.md>;
  <https://github.com/HEmile/juggl>].
- Relation typing: Breadcrumbs organizes links into directional hierarchies:
  `up`, `down`, `same`, `next`, and `prev`. Users define named Edge Fields
  (e.g., `parent` [up], `child` [down], `sibling` [same]) in settings. Inverse
  relationships and transitive closures are derived automatically [V].
- Storage mechanisms: Reads typed links from frontmatter properties
  (`parent: "[[NoteA]]"`), Dataview inline fields (`parent:: [[NoteA]]`), or
  tags. Juggl renders these typed edges with custom CSS styles, colors, and
  arrows based on property values [V:
  <https://github.com/HEmile/juggl/releases/tag/1.5.0>].
- Validation: Non-gating. If a target note does not exist, it is treated as an
  unresolved node in the directed graph. Breadcrumbs enforces no foreign-key
  cardinality or target-type constraints [V].
- Key lesson: Graph-based navigation benefits immensely from typed edge fields,
  but without note schema validation, links frequently point to notes of the
  wrong class or nonexistent targets.

### 2.8 Supercharged Links

- What it is: Supercharged Links (`mdelobelle/obsidian-supercharged-links`)
  styles internal wikilinks throughout Obsidian based on metadata of the target
  note [V:
  <https://github.com/mdelobelle/obsidian-supercharged-links/blob/master/README.md>].
- Mechanism: Inspects the target note's frontmatter (via metadata cache) and
  injects HTML attributes (`data-link-status="call soon"`,
  `data-link-tags="#person"`)
  and CSS custom properties (`--data-link-priority`) into `<a>` elements across
  the editor, backlinks pane, search, and Bases tables [V].
- Integration with schemas: Works directly with Metadata Menu / Fileclass
  fileClasses, allowing links to display icons, colors, or status badges based
  on the target note's class and property values [V].
- Key lesson: Resolving note types and metadata at link sites provides powerful
  visual feedback. It proves that downstream consumers (LSP, graph, editor)
  need fast, low-overhead access to target note classes and field values.

### 2.9 Obsidian Note Schemas

- What it is: Note Schemas (`Aguila1989/obsidian-note-schemas`) is a dedicated
  frontmatter schema validation and autofix plugin [V:
  <https://github.com/Aguila1989/obsidian-note-schemas/blob/main/README.md>].
- Schema definition: Defined in a single JSON or YAML file
  (`.obsidian/note-schemas.json`). Notes are matched to types via:
  1. Frontmatter attribute: `type: book`
  2. Match rules evaluated in declaration order: `frontmatterType`, `folder`,
     or `tagged` [V].
- Field types & constraints: Supports `string`, `number`, `boolean`, `date`
  (`YYYY-MM-DD`), `enum` (with `values`), and `list` (with optional `values`
  allow-list). Fields support `required: true` and `default` [V].
- Validation timing & diagnostics: Runs on save. Emits advisory diagnostics to
  a dedicated side panel listing violations grouped by note, and displays a
  live status-bar indicator (`valid` / `invalid`). Unknown fields produce
  advisory warnings; missing required fields or type mismatches produce errors
  [V].
- Safe autofix: A dedicated command "Fix all safe issues in this note" inserts
  missing required fields that declare a `default`, and safely coerces obvious
  scalar mistakes (numeric strings -> numbers, `"true"` -> `true`, scalar ->
  single-item list). Unknown fields are strictly preserved and never deleted
  [V].
- Key lesson: A single centralized schema file with folder/tag matching rules,
  advisory side-panel reporting, and conservative non-destructive autofix
  provides an exceptionally clean user experience without blocking edits.

### 2.10 Metadata Validator and Metadata Enforcer

- What they are:
  1. `flowing-abyss/obsidian-metadata-validator`: Folder-tree manifest-driven
     validation using `manifest.md` files placed in a `schemes/` hierarchy
     [V: https://community.obsidian.md/plugins/metadata-validator].
  2. `omarish/metadata-enforcer`: Vault metadata linting and constraint tool
     [V: https://github.com/omarish/metadata-enforcer].
- Architecture and schema evolution:
  - Metadata Validator implements hierarchical inheritance down the folder tree:
    a note inherits the schema of the nearest ancestor `manifest.md` [V].
  - Metadata Enforcer originally used JSON Schema in v1, but **the author
    completely abandoned JSON Schema in v2** [V:
    <https://omarish.com/p/metadata-enforcer-v2>]. The author found JSON Schema
    inflexible and inexpressive for real PKM vaults, replacing it with a
    Django-style Python class DSL (`metadata_enforcer.models`) with class-based
    inheritance, folder routing, and referential constraints across notes [V].
- Key lesson: Standard JSON Schema is overly verbose, awkward for markdown
  frontmatter concepts (wikilinks, folder inheritance, loose coercions), and
  lacks
  native expressiveness for cross-note relational integrity.

### 2.11 Notion-Like Vault Databases (Database Folder & Projects)

- What they are:
  1. Database Folder (`RafaelGB/obsidian-db-folder`): Notion-style folder
     databases [V: https://github.com/RafaelGB/obsidian-db-folder].
  2. Obsidian Projects (`marcusolsson/obsidian-projects`): Multi-view project
     management (table, kanban, gallery, calendar) [V:
     <https://github.com/obsidian-projects/obsidian-projects>].
- Archival and fate: Both repositories were **archived by their maintainers in
  July 2025** (Projects on July 18, 2025; DB Folder on July 28, 2025) [V:
  <https://github.com/marcusolsson/obsidian-projects/releases>;
  <https://github.com/RafaelGB/obsidian-db-folder/releases>].
- Why they were abandoned:
  1. The release of Obsidian Bases (v1.9.0/v1.9.10) in summer 2025 commoditized
     core tabular and database views natively within Obsidian core.
  2. Maintaining complex React table UIs on top of Dataview indexing and ad-hoc
     frontmatter writes created an unsustainable maintenance burden when core
     Obsidian introduced native Properties and Bases [V].
  3. DB Folder suffered persistent metadata corruption reports when writing back
     to frontmatter concurrently with other plugins [V:
     <https://github.com/RafaelGB/obsidian-db-folder/discussions/374>].
- Key lesson: UI-heavy database view plugins that attempt to manage both the
  view layer and storage write-backs become obsolete when the platform provides
  native views. A schema engine should focus on headless validation and data
  integrity rather than building custom UI view frameworks.

### 2.12 Quartz and Obsidian Publish

- What they are: Quartz (`jackyzha0/quartz`, v4/v5) is a fast static-site
  generator for Obsidian vaults [V: https://github.com/jackyzha0/quartz].
  Obsidian Publish is the official hosted publishing service [V:
  <https://obsidian.md/help/publish>].
- Frontmatter processing in Quartz: Uses `gray-matter` with `js-yaml`
  (`JSON_SCHEMA`). In `quartz/plugins/transformers/frontmatter.ts`, it
  implements
  an aggressive compatibility layer [V:
  <https://github.com/jackyzha0/quartz/blob/v4/quartz/plugins/transformers/frontmatter.ts>]:
  1. Alias coalescing: Maps `tags` or `tag`, `aliases` or `alias`, `cssclasses`
     or `cssclass`, `created` or `date`, `modified` or `lastmod`/`updated`.
  2. Soft array coercion: If `tags` or `aliases` is a comma-separated string,
     Quartz automatically splits and trims it into an array [V].
  3. Open-world frontmatter: Unknown frontmatter keys are retained in
     `file.data.frontmatter` and never stripped or rejected [V].
  4. Non-gating: Missing properties fall back to file metadata (e.g. `title`
     falls back to `file.stem`) [V].
- Obsidian Publish constraints: Rigidly enforces that core keys (`tags`,
  `aliases`, `cssclasses`) must be list types; text scalar values are silently
  ignored or break navigation [V:
  <https://obsidian.md/changelog/2025-05-21-desktop-v1.9.0/>].
- Key lesson: External consumers require robust alias coalescing and flexible
  type coercion (e.g. comma-delimited strings to arrays). Strict gating breaks
  static publication pipelines; sensible fallbacks are essential.

---

## 3. Comparison Matrix

| Project / Tool | Field Type Set | Required / Optional | Constraints (min/max/enum/etc) | Inheritance & Conflict Rule | Type Declaration Site | Validation Timing | Open/Closed World | External Value Sources | Schema Format & Version | Diagnostics Surface |
|---|---|---|---|---|---|---|---|---|---|---|
| **Obsidian Properties** | text, list, number, checkbox, date, datetime | All optional; no required concept | None | Flat vault-global namespace; no inheritance | Global `.obsidian/types.json` | Advisory on open/change; non-blocking | Open (any unknown key allowed) | None | JSON (`types.json`), unversioned | Inline warning icon on field row |
| **Obsidian Bases** | note (frontmatter), file (metadata), formula (calculated) | N/A (read/view engine) | Evaluated in formula expressions | None | Defined in `.base` YAML file | Non-gating (evaluates to null/empty) | Open (reads any property) | Vault notes, formula functions | YAML (`.base`), versioned by app | In-cell error display, empty cell fallback |
| **Metadata Menu / Fileclass** | 26 types (Input, Number, Select, Multi, Date, File, Object, ObjectList, Canvas...) | `required: true/false` per field | min, max, step, regex templates, select options | Single inheritance (`extends`); `excludes`; **Last-listed class wins** | Class note frontmatter; mapped via tag, folder, alias | Advisory in UI; gating on programmatic write API & CLI | Open (unknown frontmatter keys preserved) | Notes, folders, Base views, inline lists | YAML in class notes (`version: "2.0"`) | Red field action icons, table column, CLI exit code |
| **Dataview / Datacore** | Dynamic: string, number, date, duration, link, array, object | All optional | None | None | Inferred dynamically from frontmatter/inline | Non-gating (unparseable values stay strings) | Open | Vault files, tasks, lists | None (codeblocks only) | Console logs only; no editor diagnostics |
| **Obsidian Templater** | Interactive modal types (prompt, suggester, multi-suggester) | Gated at prompt time (`throw_on_cancel`) | List choices in suggester | None | Template files; folder template mapping | Write-time modal prompt only | Open | JS scripts, vault queries | Template files (JavaScript/EJS) | Modal abort / Notice popups |
| **Obsidian Linter** | Syntactic: string, array, boolean, date | Configurable rule application | Format rules (YAML array, sorting, quote style) | None | Plugin settings JSON | Advisory or automatic on save | Open | None | Settings JSON | Notice popups on save |
| **Breadcrumbs / Juggl** | Directed edge fields (up, down, same, next, prev) | Optional | Edge direction and named inverse relations | Transitive edge inference; no schema inheritance | Settings + note frontmatter / inline fields | Non-gating | Open | Vault links | Plugin settings JSON | Visual graph styling, matrix view |
| **Supercharged Links** | Target note attributes & tags | Optional | Selector matching (exact, contains) | None | Plugin settings + CSS snippets | Read-only render pass | Open | Target note frontmatter | Settings JSON + CSS | Link DOM attributes (`data-link-*`) |
| **Note Schemas** | string, number, boolean, date, enum, list | `required: true/false`, `default` | Enum values, list allowed values | Single type match (first matching rule wins) | Central file (`note-schemas.json` / YAML) | Advisory on save; non-blocking | Open (unknown fields are warnings) | Predefined schema lists | JSON / YAML schema file | Side panel list, status bar indicator |
| **Metadata Validator** | Frontmatter keys & types | Configurable in manifest | Pattern matching, required fields | Hierarchical folder tree inheritance (`manifest.md`) | `schemes/**/manifest.md` | Advisory on file change | Open | None | Markdown frontmatter | Notice notifications, validation log |
| **Metadata Enforcer** | Python typed fields (CharField, ChoiceField, etc.) | Required / nullable | Django-style field validators, regex, choices | Python class inheritance; sub-folder overrides | External Python models (`models.py`) | Gating via external CLI / pre-commit | Closed per model | External databases, Python logic | Python code DSL (v2; replaced JSON Schema) | CLI stdout/stderr report, exit code |
| **Database Folder / Projects** | text, number, select, multi, date, checkbox | Optional | Select options, date format | None (per-database configuration) | Codeblock YAML / `.dbfolder` settings | Advisory in grid cells | Open | Dataview queries, folders | YAML / JSON | In-cell error formatting |
| **Quartz** | title, tags, aliases, date, description, draft, publish... | Optional (defaults provided) | Fallback coersion (string -> array) | None | Note frontmatter | Non-gating (coerces or ignores) | Open (retains unknown keys) | None | TypeScript transformer plugin | CLI build warnings |

---

## 4. Challenges to Traces' Design (Ticket 20 Answer)

### 4.1 Advisory-Only Validation vs Gating

- Traces Decision: Schemas validate note data as an advisory, on-demand check.
  They never gate the read or index path. Gating is strictly host policy
  (e.g., MCP agent tools may gate writes, while CLI and LSP are advisory) [V:
  `.scratch/arch-design/issues/20-schema-note-typing-and-validation.md`].
- Supporting Evidence:
  - Fileclass author explicitly states: "Nothing is ever blocked. A note that
    violates its class can be saved, left, and come back to — required is a
    statement about your model, not a gate on your typing" [V:
    <https://mdelobelle.github.io/fileclass/fields/#required-fields>].
  - Obsidian Core Properties uses non-blocking advisory warnings. Hard gating on
    save causes immediate user workflow paralysis when drafting thoughts [V:
    <https://forum.obsidian.md/t/obsidian-properties-best-practices-and-why/63891>].
  - Note Schemas displays advisory sidebar and status-bar diagnostics without
    blocking edits [V:
    <https://github.com/Aguila1989/obsidian-note-schemas/blob/main/README.md>].
- Contradicting Evidence:
  - Agent and programmatic write workflows degrade rapidly without gating.
    When Claude Code or automation scripts write bad frontmatter, users
    experience silent metadata rot [V:
    <https://forum.obsidian.md/t/file-ctime-got-reset-when-note-properties-were-edited-using-claude-code-and-cli/111185>].
  - Metadata Enforcer deliberately chose CLI-gating to prevent invalid commits
    in git-backed vaults [V: https://omarish.com/p/metadata-enforcer-v2].
- Recommendation: **Keep Traces' decision**. Pure advisory checks at core with
  host-level gating (MCP tool write gates, git/CI commit gates, LSP advisory
  diagnostics) is the exact consensus pattern proven across the ecosystem.

### 4.2 Multi-Class Conflict Rule: "First Wins" vs "Last Wins"

- Traces Decision: When multiple classes declare the same field, the
  **first-listed class wins** the whole declaration [V:
  `.scratch/arch-design/issues/20-schema-note-typing-and-validation.md`].
- Supporting Evidence:
  - Matches standard programming language precedence (e.g., Python C3 MRO,
    where the first base class in `class C(A, B)` takes precedence).
- Contradicting Evidence:
  - **Metadata Menu and Fileclass explicitly implement "LAST WINS"** [V:
    <https://mdelobelle.github.io/fileclass/schema/>]. The author explains the
    mental model: `fileClass: [Book, Article]` reads as "a Book, with an Article
    layered on top". Users intuitively expect later classes to act as overlays
    or specializations that override base properties [V].
  - CSS cascading rules and configuration stacking (e.g., Docker Compose,
    Kubernetes overlays) consistently treat the last-specified layer as the
    winner.
- Recommendation: **Change to Last-Listed Wins, or make inheritance order
  explicitly match CSS overlay semantics**. If a note declares
  `class: [Entity, Person]`, the user views `Person` as the specialization
  refining `Entity`. If first-wins is retained, `Entity` would silence
  `Person`'s
  stricter field definitions, which contradicts user expectations in PKM.

### 4.3 Required, Min, Max, Step, and Format Constraints

- Traces Decision: Schema checks enforce `required`, `multi`, `min`, `max`,
  `step`, and date/format [V:
  `.scratch/arch-design/issues/20-schema-note-typing-and-validation.md`].
- Supporting Evidence:
  - Community Feature Request #107050 ("Custom Property Restrictions and Types")
    specifically demands min/max for ratings, step constraints, and link target
    restrictions [V:
    <https://forum.obsidian.md/t/custom-property-restrictions-and-types/107050>].
  - Fileclass implements `min`, `max`, and `step` with UI clamping and
    fractional step precision handling (`0.1 + 0.2` clean arithmetic) [V:
    <https://mdelobelle.github.io/fileclass/fields/#number-fields>].
  - Users consistently complain that without `required`, notes are created
    with missing critical fields, breaking query views [V:
    <https://forum.obsidian.md/t/super-fr-enhance-obsidian-with-a-type-system-for-notes-and-database-like-views-metadata-object-oriented-model/46444>].
- Contradicting Evidence:
  - Overly restrictive format constraints (e.g. strict calendar date checks
    rejecting historical BCE dates or partial `YYYY-MM` dates) frustrate users
    [V:
    <https://forum.obsidian.md/t/allow-iso-conforming-partial-dates-in-properties/77364>].
- Recommendation: **Keep Traces' decision**. Bounded ranges, step, and
  presence checks are high-demand features with clear precedent in Fileclass.
  Ensure date checking allows partial dates (`YYYY`, `YYYY-MM`).

### 4.4 Class Declaration Site (Frontmatter vs Folder vs Tag vs Global)

- Traces Decision: A note declares its class(es) in a frontmatter field
  (default `class`, configurable) [V:
  `.scratch/arch-design/issues/20-schema-note-typing-and-validation.md`].
- Supporting Evidence:
  - Frontmatter properties are file-local, surviving file moves across folders
    and renames [V: https://mdelobelle.github.io/fileclass/schema/].
  - Obsidian's flat global `types.json` proved to be a major architectural
    mistake that created namespace pollution across the entire vault [V:
    <https://forum.obsidian.md/t/expand-types-json-with-display-name-and-suggestion-options/101456>].
- Contradicting Evidence:
  - Users intensely dislike having to manually add `class: book` to hundreds of
    notes. Metadata Menu, Fileclass, Note Schemas, and Metadata Validator all
    provide **folder path and tag mapping** so that notes in `Books/` or tagged
    `#book` automatically acquire the schema without frontmatter boilerplate [V:
    <https://mdelobelle.github.io/fileclass/schema/>;
    <https://github.com/Aguila1989/obsidian-note-schemas/blob/main/README.md>;
    <https://community.obsidian.md/plugins/metadata-validator>].
- Recommendation: **Keep frontmatter `class` as the primary binding, but add
  folder and tag schema mapping in Traces configuration**. Forcing explicit
  frontmatter on every single note creates onboarding friction that all leading
  plugins had to solve via folder/tag mapping rules.

### 4.5 Closed World vs Open World (Unknown Fields)

- Traces Decision: Unknown fields are always detected and returned as
  violations with a `closest` match suggestion; hosts decide whether to
  filter or warn [V:
  `.scratch/arch-design/issues/20-schema-note-typing-and-validation.md`].
- Supporting Evidence:
  - Catches typing mistakes (e.g. `authr` instead of `author`).
- Contradicting Evidence:
  - In real Obsidian vaults, notes contain ad-hoc properties from various
    plugins (e.g. `cssclasses`, `publish`, `aliases`, `dg-publish`).
  - Fileclass specifically guarantees that unknown keys are preserved during
    edits and never stripped [V:
    <https://mdelobelle.github.io/fileclass/fields/#nested-fields-object--objectlist>].
  - Note Schemas treats unknown fields as low-severity warnings, never errors
    [V:
    <https://github.com/Aguila1989/obsidian-note-schemas/blob/main/README.md>].
- Recommendation: **Keep Traces' decision**. Generating the violation code
  `unknown-field` at the check layer while letting host policy default to
  ignore or advisory warning matches ecosystem realities.

### 4.6 Things Traces Has Not Considered That Precedents Found Essential

1. **Conservative Non-Destructive Autofix**: Note Schemas found that reporting
   violations without an automated safe fix creates user fatigue. Its autofix
   engine inserts missing required fields that have declared defaults, coerces
   scalar-to-list, and normalizes numeric strings [V:
   <https://github.com/Aguila1989/obsidian-note-schemas/blob/main/README.md>].
   Traces should plan a `traces fix` command with identical safety constraints.
2. **Dependent Link Fields (Filtered Pickers)**: In Fileclass, a link field's
   candidate list can depend on another field in the same note (e.g., selecting
   a `Project` only lists projects belonging to the already-selected `Client`)
   [V:
   <https://mdelobelle.github.io/fileclass/fields/#conditional-candidates-dependent-fields>].
   Traces' tier-2 file checks currently lack dependent-field narrowing.
3. **Lossless Frontmatter Reordering**: When adding new fields to an existing
   note, users strongly care about visual property order matching the schema
   definition. However, rewriting frontmatter easily drops comments and blank
   lines [V: https://platers.github.io/obsidian-linter/settings/yaml-rules/].
   Traces must ensure its write path respects CST preservation.
4. **Field Exclusions (`excludes`) in Inheritance**: Both Metadata Menu and
   Fileclass found that deep inheritance hierarchies often need to drop a
   specific parent field (e.g. `Audiobook` extends `Book` but drops `pages`)
   [V: https://mdelobelle.github.io/fileclass/schema/]. Traces' schema model
   already includes `excludes`—the ecosystem validates that this was a critical
   design inclusion.

---

## 5. Failure Stories and Regrets

- **Obsidian Global Property Namespace (`types.json`)**:
  Obsidian's decision to maintain a single vault-wide type registry without
  note-type scoping resulted in persistent property type pollution (e.g.
  `status`
  clashing between media tracking and tasks) and cross-device sync merge
  conflicts on JSON files [V:
  <https://forum.obsidian.md/t/expand-types-json-with-display-name-and-suggestion-options/101456>;
  <https://forum.obsidian.md/t/i-have-a-lot-of-json-conflict-files-on-my-obsidian-folder/66795>].
- **Dataview Inline Field Performance Collapse**:
  Parsing arbitrary markdown note bodies for regex-based inline fields
  (`[key:: value]`) produced massive lag in large vaults, forcing Datacore to
  disable inline fields by default and re-architect around frontmatter and
  block-level structures [V:
  <https://github.com/blacksmithgu/datacore/blob/master/ROADMAP.md>].
- **Metadata Menu Tight Coupling to Dataview**:
  Metadata Menu originally delegated all field candidate queries, views, and
  evaluations to Dataview. When Dataview stalled and Obsidian released native
  Properties and Bases, the maintainer had to abandon the codebase and rewrite
  a separate plugin (`fileclass`) from scratch [V:
  <https://github.com/fronglemog/obsidian-fileclass/blob/main/README.md>;
  <https://mdelobelle.github.io/fileclass/positioning/>].
- **Metadata Enforcer JSON Schema Abandonment**: Author Omar Bohsali initially
  built `metadata-enforcer` on JSON Schema (v1), only to completely abandon it
  in v2 for a Python class-based DSL because static JSON Schema was too rigid,
  awkward for markdown conventions, and lacked relational cross-note validation
  [V: https://omarish.com/p/metadata-enforcer-v2].
- **Linter YAML Regex Comment Stripping**:
  Obsidian Linter parsed YAML frontmatter using regular expressions rather than
  a
  complete YAML AST parser, causing widespread user complaints about deleted
  comments, stripped blank lines, and corrupted nested objects [V:
  <https://github.com/platers/obsidian-linter/issues/715>].
- **Database Folder and Projects Archival**:
  Both major community database plugins (`obsidian-db-folder` and
  `obsidian-projects`) were abandoned and archived in July 2025 after Obsidian
  introduced native Bases, proving the fragility of building heavy UI view
  layers outside the core application [V:
  <https://github.com/RafaelGB/obsidian-db-folder/releases>;
  <https://github.com/marcusolsson/obsidian-projects/releases>].
- **AI Agent `file.ctime` Corruption**:
  External scripts and AI coding agents (such as Claude Code) editing
  frontmatter
  properties directly on disk without timestamp preservation inadvertently
  reset filesystem `file.ctime`, corrupting timeline queries and vault
  chronologies [V:
  <https://forum.obsidian.md/t/file-ctime-got-reset-when-note-properties-were-edited-using-claude-code-and-cli/111185>].

---

## 6. Transferable Lessons

1. **Decouple Schemas from Storage and Querying**: The cleanest architecture in
   the ecosystem (Fileclass + Bases) treats the schema as a pure definition of
   fields and constraints, delegating querying and views to an independent
   query engine. Traces' design (separating `schema` from `query` and `note`)
   directly mirrors this best practice.
2. **Never Parse Body Text for Schemas**: Confining schemas to frontmatter
   eliminates 90% of the performance, parsing, and data-corruption bugs that
   plagued Dataview, Metadata Menu v1, and regex linters.
3. **Advisory by Default, Gated at Boundaries**: Note authoring is inherently
   messy and exploratory. Gating during normal writing or file saving frustrates
   users. Hard gating must be reserved for automated boundaries: agent write
   tools (MCP), pre-commit hooks, and explicit CI validation passes.
4. **Compositional Overlays Exceed Multiple Inheritance**: Real PKM users
   struggle with multiple inheritance diamonds. Fileclass solved this by
   limiting class inheritance to single `extends` chains, while allowing notes
   to compose multiple classes as an ordered list of overlays where the later
   definition takes precedence.
5. **Autofix Must Be Lossless and Conservative**: Automated repairs should only
   fill declared defaults and perform lossless coercions (e.g. `"123"` ->
   `123`).
   Deleting unknown properties or guessing missing values corrupts user notes
   and destroys trust.
6. **Agent Guardrails Require Timestamp and CST Safety**: When tools or MCP
   agents write frontmatter, they must preserve non-schema keys, maintain key
   ordering, avoid corrupting filesystem creation times (`ctime`), and never
   strip user comments.
