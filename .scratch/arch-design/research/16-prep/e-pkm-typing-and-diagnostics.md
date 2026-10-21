# PKM Note Typing, Schema Enforcement & Diagnostic Precedent

- **Topic:** Survey of note metadata typing, schema definition, violation handling, and diagnostic reporting across PKM tools, markdown language servers, static site generators, and databases.
- **Deliverable:** `.scratch/arch-design/research/16-prep/e-pkm-typing-and-diagnostics.md`.
- **Target Issue:** Ticket 16 (Schema Note Typing and Validation) for Wayfinder milestone.
- **Constraints Applied (Tickets 12–15):** Core is synchronous CLI by default (T13); LSP is next committed consumer (T13); note value model has two owned types (`NoteFieldValue` vs schema-local literal) (T14); select values matching wikilinks use adaptive-literal rule (T14/T15); frontmatter classification is schema-free (T15); coercion failures and validation diagnostics are advisory, never gating (T15/Res03); CLI stdout/stderr split and exit codes frozen (T12).
- **Tags:** `[V]` verified in digest/code/official doc, `[H]` inference, `[P]` proposal for Traces.

---

## 1. Local Digest Deep Dive: PKM Tools, Parsers & Language Servers

### 1.1 Obsidian Dataview: Dynamic Inference on Ingest (No Schema)

- **Typing Mechanism:** No schema definition exists. Field typing is performed dynamically on ingest during markdown parsing (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3650-3693`) [V].
- **Frontmatter Inference Cascade (`parseFrontmatter`):**
  1. `null` / empty $\to$ `null` [V:3652].
  2. Array $\to$ recursively mapped array [V:3655].
  3. JS/YAML `Date` $\to$ Luxon `DateTime.fromJSDate(value)` [V:3662].
  4. Object $\to$ recursively parsed string-literal dictionary [V:3666].
  5. Number / Boolean $\to$ primitive number/boolean [V:3674-3677].
  6. String $\to$ eager parser cascade: tries `EXPRESSION.date.parse(value)` (ISO/Luxon format) $\to$ `EXPRESSION.duration.parse(value)` $\to$ `EXPRESSION.embedLink.parse(value)` $\to$ fallback to raw string [V:3678-3689].
- **Inline Field Parsing (`parseInlineValue`):** Strips `[key:: value]` or `(key:: value)` and runs `EXPRESSION.inlineField.parse(value)` (`src/data-import/markdown-file.ts:3163-3175`) [V]. Empty text maps to `null` [V:3167].
- **Gating vs Advisory & Diagnostics:** Completely non-gating. No validation errors or diagnostics are ever emitted for unexpected types or syntax; unparseable strings simply stay strings [V].
- **Cost:** Parsing every string with 3 Parsec-style Parsimmon parsers per frontmatter entry runs during background index worker passes [V:3678-3689].

### 1.2 Obsidian Templater: UI-Driven Input Modals (Write-Time Only)

- **Typing Concept:** No data schema or validation. Types exist strictly as prompt modalities during interactive template execution (`obsidian_silentvoid13-templater-src-digest.txt:2986-3075`) [V].
- **Prompt Types:**
  - `tp.system.prompt(text, default, throw_on_cancel, multiline)` $\to$ returns `Promise<string | null>` via `PromptModal` (`Modal`) [V:2986-3018, 3302].
  - `tp.system.suggester(text_items, items, throw_on_cancel, placeholder)` $\to$ returns `Promise<T>` via `SuggesterModal<T>` (`FuzzySuggestModal<T>`) [V:3021-3059, 3401].
  - `tp.system.multi_suggester(text_items, items)` $\to$ returns `Promise<T[]>` via `MultiSuggesterModal<T>` [V:3062-3085].
- **Gating vs Advisory:** No validation of resulting note content; errors occur only if the user cancels a modal configured with `throw_on_cancel: true` or if template JS throws [V:3013].

### 1.3 Obsidian Advanced Metadata Cache: Untyped Reference Extraction

- **Frontmatter Link Handling:** Scans note frontmatter for `[[links]]` and exposes them via Obsidian's `CachedMetadata.frontmatterLinks: ReferenceCache[]` (`obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt:9592-9603`) [V].
- **Typing & Validation:** Links carry byte/line offsets (`ReferenceCache.position`) and raw target strings, but zero type semantics [V:9592]. Canvas references are mapped to synthetic `frontmatterLinks` to satisfy consumers expecting file metadata [V:9596-9600]. No schema or diagnostic validation exists [V].

### 1.4 Obsidian Linter: AST/Regex Auto-Fix Formatting

- **Mechanism:** Text-modifying formatter running on file save, file change, or manual command (`obsidian_platers-obsidian-linter-digest.txt:8039-8051`) [V].
- **Frontmatter Rules:** Applies syntactic transforms: `yaml-key-sort` (priority keys + alphabetical order), `format-yaml-array` (inline vs multi-line), `escape-yaml-special-characters`, `dedupe-yaml-array-values` [V:111-124].
- **Parsing Caveat:** YAML keys and values are parsed via regex heuristics rather than a full YAML AST parser (`docs/additional-info/rule-types/yaml.md:246-247`) [V]. Comments and blank lines inside YAML can be dropped [V:248-249].
- **Diagnostics & Severity:** Zero diagnostic reporting (no Error/Warning/Info severity levels). Rules directly mutate markdown text on disk. If frontmatter cannot be parsed, transformations are aborted to avoid data corruption [H].
- **Suppression:** Granular per-file suppression via `disabled rules: [yaml-key-sort]` in YAML frontmatter or inline HTML comments `<!-- linter-disable -->` [V:7981-8016].

### 1.5 Obsidian Tasks: Hardcoded Domain Fields

- **Data Model:** Fixed, closed schema hardcoded into the `Task` class (`obsidian_obsidian-tasks-src-digest.txt:25140-25203`) [V].
- **Fields & Types:**
  - Dates: `startDate`, `scheduledDate`, `dueDate`, `doneDate`, `cancelledDate`, `createdDate` (typed as `moment.Moment | null`, parsed via ISO regex `\d{4}-\d{2}-\d{2}`) [V:4919, 26008, 26473-26476].
  - Enum/Scalar: `status: Status`, `priority: Priority` (Highest, High, Medium, None, Low, Lowest), `recurrence: Recurrence | null`, `dependsOn: string[]`, `id: string` [V:25142, 25948-25950].
- **Dual Serialization Syntax:**
  1. Dataview format: `[due:: 2026-10-20]`, `[priority:: high]` [V:26460-26479].
  2. Emoji format: `📅 2026-10-20`, `🛫 2026-10-20`, `⏳ 2026-10-20`, `🔺`, `🔁` [V:26612-26631].
- **Gating vs Advisory:** Non-gating on read; unparseable lines or invalid dates are silently ignored or treated as plain description text [V]. Input validation occurs strictly inside the interactive `TaskModal` UI [V:11430].

### 1.6 zk: Minimal CLI Frontmatter Handling

- **Data Model:** Notes parse YAML frontmatter via standard Go `yaml.Unmarshal` into an untyped dictionary `Metadata map[string]any` (`zk-src-digest.txt:6139, 6345-6360`) [V].
- **Schema Concept:** Zero user schema definition. A minimal set of hardcoded keys are recognized: `title`, `tags`/`keywords`, `aliases` [V:1738-1746].
- **Configurable Keys:** Config file (`.zk/config.toml`) permits mapping custom frontmatter key names for dates: `[format.markdown.frontmatter] creation-date-key = "created"`, `modification-date-key = "changed"` (`zk-docs-digest.txt:1759-1763`) [V].
- **Validation:** None. Values in `Metadata` are exposed dynamically to Tera/Go template variables (`{{metadata.field}}`) without type checking or lint warnings [V:1747-1750].

### 1.7 LSP Ecosystem: Diagnostics Models & Recomputation

#### rumdl (Rust Markdown Linter & LSP)

- **Diagnostic Type (`LintWarning`):** Carries `message: String`, 1-indexed `line`, `column` (in UTF-8 characters, not byte offsets), `end_line`, `end_column`, `severity: Severity` (`Error`, `Warning`, `Info`), `rule_name: Option<String>` (e.g. `"MD013"`), and `fix: Option<Fix>` (`lsp_rvben-rumdl-src-digest.txt:9344-9361`) [V].
- **Atomic Multi-Span Fixes (`Fix`):** `Fix` holds `range: Range<usize>`, `replacement: String`, and `additional_edits: Vec<Fix>` [V:9385-9393]. Crucial for PKM edits where changing an inline link or footnote requires an atomic paired edit at the end of the file [V:9365-9370].
- **Scope & Scheduling:**
  - `CrossFileScope`: Rules declare `None` (single file, 99%) or `Workspace` (cross-file reference validation like MD051) [V:9475-9482].
  - On `did_change` (per keystroke): runs only single-file fast rules; skips external tools and cross-file passes [V:37825, 39148].
  - On `did_open` / full index update: executes `republish_diagnostics` across open documents, explicitly verifying that document version has not advanced during computation [V:39162-39183].
- **Suppression:** Inline HTML comments `<!-- rumdl-disable MD013 -->` tracked via `SuppressionReport` [V:9484-9508].

#### Marksman (F# Markdown Language Server)

- **Diagnostic Model (`Diag.fs`):** Strongly-typed union `Entry` with numeric error codes:
  - `AmbiguousLink(Element, Syms.Ref, Dest[])` $\to$ Code `"1"`, uses `DiagnosticRelatedInformation` to point to duplicate target locations (`lsp_artempyanykh-marksman-digest.txt:6645-6655, 6761-6766`) [V].
  - `BrokenLink(Element, Syms.Ref)` $\to$ Code `"2"` [V:6653, 6780].
  - `NonBreakableWhitespace(Lsp.Range)` $\to$ Code `"3"` [V:6654, 6803].
- **Severity Policy:** Asymmetric severity based on link syntax: Wiki-links (`WL`) resolve to `DiagnosticSeverity.Error`, while standard Markdown links (`ML`) resolve to `DiagnosticSeverity.Warning` [V:6752-6759, 6781-6788].
- **Execution:** Stateless evaluation per document or folder (`checkFolder`); resolves cross-file references against in-memory CST symbol index [V:6718-6729].

#### markdown-oxide (Rust PKM Language Server)

- **Unresolved Link Diagnostics:** Detects dead wikilinks and unresolved references across the vault (`lsp_feel-ix-343-markdown-oxide-src-digest.txt:1417-1442`) [V].
- **Severity Choice:** Uses `DiagnosticSeverity::INFORMATION` [V:1476]. Why: in PKM vaults and Zettelkasten systems, uncreated notes ("ghost notes") are an intended workflow step, so treating missing links as Errors or Warnings creates intolerable visual clutter [H].
- **Configurability:** Single master toggle `settings.unresolved_diagnostics: bool` (default `true`) [V:1172, 1450]. Directly emits `tower_lsp::lsp_types::Diagnostic` without an internal diagnostic domain model [V:1460].

#### vscode-markdown-languageservice (TypeScript)

- **Configuration Hierarchy (`DiagnosticOptions`):** Independent fine-grained severity controls per diagnostic category (`lsp_microsoft-vscode-markdown-languageservice-digest.txt:3003-3038`) [V]:
  - `validateReferences`, `validateFragmentLinks`, `validateFileLinks`, `validateMarkdownFileLinkFragments`, `validateUnusedLinkDefinitions`, `validateDuplicateLinkDefinitions`: each accepts `DiagnosticLevel` (`'error' | 'warning' | 'hint' | 'ignore'`) [V:2982-2998].
  - `ignoreLinks: readonly string[]` (glob array of ignored target paths) [V:3037].
- **Diagnostic Codes:** Distinct strings (`link.no-such-file`, `link.no-such-header-in-file`, `link.no-such-reference`, `link.unused-definition`) [V:3053-3071].
- **Recomputation Modes:** Dual architecture:
  1. `DiagnosticComputer`: Stateless, evaluates whole document on demand (`computeDiagnostics`) [V:1683-1688].
  2. `DiagnosticsManager` (`IPullDiagnosticsManager`): Stateful, implements LSP 3.17 Pull Diagnostics, tracking file watchers and incrementally dirtying cached diagnostics on workspace changes [V:1691-1700, 1799].

### 1.8 Rust Obsidian Parsers & Vim Tooling

- **`cryptogladi-obsidian-parser`:** Generic over property type `Note<T = DefaultProperties>` where `DefaultProperties = HashMap<String, serde_yml::Value>` (`rust_cryptogladi-obsidian-parser-digest.txt:350, 802`) [V]. Allows callers to either retain dynamic YAML values or deserialize into strongly typed Rust structs at compile time [H].
- **`zoni-obsidian-export`:** Frontmatter is an untyped alias `type Frontmatter = serde_yaml::Mapping` (`rust_zoni-obsidian-export-digest.txt:1898`) [V]. Read and written verbatim without validation [V].
- **`mcky-obsidian-cli`:** Converts `serde_yaml::Value` into `serde_json::Value` on read; no schema layer [V].
- **`obsidian-nvim`:** Validates a closed set of recognized keys: `id` (string|number), `aliases` (string|list), `tags` (string|list) (`vim_obsidian-nvim-digest.txt:935-943`) [V]. Contract: *"Invalid types are ignored with a warning. Unrecognized fields are left untouched as metadata."* [V:935, 943].

---

## 2. External Precedent Survey: PKM, Static Site Generators & Rich Documents

### 2.1 Obsidian Properties & Obsidian Bases

- **Obsidian Properties (v1.4+):**
  - Types: `Text`, `List`, `Number`, `Checkbox`, `Date` (`YYYY-MM-DD`), `Date & time` (`YYYY-MM-DDTHH:mm:ss`), `Tags` (`https://help.obsidian.md/properties`) [V].
  - Global Schema Registry: Stored in `.obsidian/types.json` as a vault-wide map `{"<prop_name>": "<type>"}` [V]. All notes sharing a property name share that type vault-wide [V].
  - Type Mismatch Handling: Strictly advisory. If a note's frontmatter has `cost: "high"` when `cost` is declared `Number`, Obsidian displays an orange caution triangle with tooltip `"Type mismatch, expected Number"` (`https://forum.obsidian.md/t/property-type-and-type-mismatch/113071`) [V].
  - Gating vs Advisory: Never gates reading, indexing, or saving. In Source Mode, raw YAML remains untouched. Clicking the warning triangle offers UI actions: convert property type vault-wide or edit value [V].
- **Obsidian Bases (v1.10+, `.base` files):**
  - Database views (table, board, list) over markdown notes (`https://help.obsidian.md/bases/formulas`) [V].
  - Property Sources: `note` (YAML frontmatter), `file` (ctime, mtime, size), and `formula` (computed expressions) (`TypeScript API: BasesPropertyType`) [V].
  - Formulas evaluate dynamically and project typed values (string, number, boolean, date, list) without mutating frontmatter [V].

### 2.2 Logseq: File Graph vs DB Graph

- **File Graph Mode:** Properties are raw text (`key:: value` or Org `:PROPERTIES:` drawers). Untyped, unvalidated, schema-less (`logseq/libs/guides/db_properties_guide.md:16-24`) [V].
- **DB Graph Mode:**
  - 2-layer schema architecture: (1) Property definitions (`PropertySchema`), (2) Entity values in SQLite/Datascript [V:28-44].
  - `PropertySchema`: `type` (`'default' | 'number' | 'node' | 'date' | 'checkbox' | 'url'`), `cardinality` (`'one' | 'many'`), `hide: bool`, `public: bool` [V:48-62].
  - Tag Classes & Inheritance: Tags act as classes. `Editor.createTag('ZoteroItem', { tagProperties: [...] })` associates schemas with tags [V:147-168]. `Editor.addTagExtends('ZoteroItem', 'Reference')` provides schema inheritance [V:186-188].
  - Enforcement: In DB mode, property values are structured entities; invalid inputs are rejected or coerced by the UI input widget before DB write [V:80-110].

### 2.3 Dendron: Hierarchical Schemas & Dot-Pattern Matching

- **Schema Files:** YAML files named `{name}.schema.yml` in vault (`https://docs.dendron.so/notes/DxucQoYEzu9VnBoYuFiqN`) [V].
- **Hierarchical Matching:** Schemas declare trees with `id`, `parent`, and `children` containing `pattern` (glob or regex matching dot-hierarchies, e.g. `pattern: "[0-9]{4}"`) (`dendron.topic.schema.tutorial.first-schema.md:120-145`) [V].
- **Templates & Namespaces:** Matching a note hierarchy (e.g. `daily.journal.2026.10.20`) automatically attaches a `template: <noteId>` on creation [V:140-145]. `namespace: true` allows arbitrary unvalidated child notes [V].
- **Gating vs Advisory:** Strictly advisory. Schemas drive lookup autocomplete and note creation templates. Notes that diverge from the schema hierarchy are fully readable, searchable, and editable; no gating or build failures occur [V]. Malformed schema YAML triggers a startup warning banner [V].

### 2.4 Anytype & Capacities: Object-Oriented Typing

- **Anytype:** Local-first CRDT database. Notes are Objects of a specific `Object Type` (e.g. Task, Book, Note). Properties are `Relations` (Text, Number, Date, Select, Relation to Object) (`https://doc.anytype.io/anytype-docs/data-model/relations`) [V].
- **Capacities:** Object-based PKM. Objects belong to an Object Type; each Type defines a fixed set of property definitions.
- **Enforcement:** Strongly typed at creation and edit time via UI widgets (date pickers, numeric inputs, relation pickers). Writing invalid types is prevented by the UI; in the underlying CRDT store, invalid payloads fail schema decoding [H].

### 2.5 Tana: Supertags & First-Class Fields

- **Supertags as Classes:** Adding `#book` or `#meeting` to a node attaches defined Fields (`https://tana.inc/docs/supertags`) [V].
- **Field Model:** Fields are themselves first-class nodes with types (Plain text, Date, Number, Checkbox, Instance of Supertag, Options) [V].
- **Inheritance & Multi-Tagging:** Supertags support inheritance (`#novel` extends `#book`). A node can have multiple supertags (`#project` and `#urgent`). If two supertags declare fields with identical IDs, they collapse into one; if they share only a display name, both fields render under their respective supertag headers [V].
- **Enforcement:** Structural UI enforcement: typing drives the input modal/dropdown. The underlying node graph stores entity references, preventing type mismatches at the data layer [H].

### 2.6 Notion & Airtable: Columnar Typing & Write Coercion

- **Typing Model:** Strict column types declared per database table (Text, Number, Select, Multi-select, Date, Checkbox, Relation).
- **Coercion on Write:**
  - Interactive/Paste: Pasting string `"123"` into a Number column coerces to `123`; pasting unparseable `"abc"` leaves the cell empty or clears it [V].
  - REST API: Writing a string into a Number property or invalid date string returns an immediate `400 Bad Request` with structured JSON error (`validation_error`) [V]. Gating at the API boundary!

### 2.7 Static Site Generators: Build-Time Gating

- **Astro Content Collections (`src/content.config.ts`):**
  - Uses Zod schemas: `defineCollection({ schema: z.object({ title: z.string(), date: z.coerce.date(), tags: z.array(z.string()).default([]) }) })` (`https://docs.astro.build/en/guides/content-collections/`) [V].
  - Build Gating: Hard gating. If any Markdown/MDX frontmatter fails the schema, `astro build` aborts with a detailed `ZodError` showing file path, frontmatter key path, expected vs received type [V].
  - Reconciling YAML: Relies explicitly on `z.coerce.date()`, `z.coerce.number()`, `z.coerce.boolean()` to bridge YAML string parsing with typed JavaScript primitives [V].
  - Editor Experience: The Astro VS Code extension runs Zod validation in-memory and surfaces LSP diagnostics on frontmatter fields [V].
- **Quarto:** Uses internal YAML schemas (in Deno) to validate document frontmatter; violations trigger render-time warning or fatal exit codes depending on configuration [V].

### 2.8 Markdown Frontmatter Linting: `remark-lint-frontmatter-schema`

- **Specification:** Validates Markdown YAML frontmatter against standard JSON Schema (Draft 7 / 2020-12 via Ajv) (`https://github.com/JulianCataldo/remark-lint-frontmatter-schema`) [V].
- **Association:** In-file via `'$schema': ./schemas/post.schema.yaml` in frontmatter, or globally via glob mapping in `.remarkrc` [V]. Supports schema composition via `$ref` and `allOf` [V].
- **CST Mapping & Diagnostics:** Maps JSON Schema validation error paths (`instancePath`) back to AST source tokens, providing exact line/column spans for IDE squiggles [V].
- **Execution Mode:** Advisory in editor (VS Code `vscode-remark`), but configurable as a build-breaking gate in CI / Astro / Gatsby pipelines [V].

---

## 3. Comparative Alignment Matrix

| Project / Tool | Schema Concept | Where Declared | Gating? | Violation Handling | Drives UI? | Diagnostic Shape |
|---|---|---|---|---|---|---|
| **Obsidian Dataview** [V] | None (dynamic inference) | N/A | No | Fallback to raw string | No | None (silent) |
| **Obsidian Templater** [V] | Interactive prompt types | Per template call | No | Throws on cancel | Yes (modals) | JS runtime exception |
| **Obsidian Tasks** [V] | Hardcoded domain schema | Code / Settings | No | Silent ignore / plain text | Yes (TaskModal) | None on file read |
| **zk** [V] | Minimal key aliases | `.zk/config.toml` | No | Untyped `map[string]any` | No | None |
| **rumdl** [V] | Markdown lint rules | `rumdl.toml` / inline | No (advisory) | Reports warning/fix | No (LSP) | `LintWarning` (line, col, sev, fix, rule) |
| **Marksman** [V] | Link/heading consistency | Workspace code | No (advisory) | Flag broken/ambiguous | No (LSP) | `Entry` (code "1"/"2", WL=Err, ML=Warn) |
| **markdown-oxide** [V] | Vault reference index | Settings toggle | No (advisory) | Informational diagnostic | No (LSP) | LSP `Diagnostic` (`Severity::INFORMATION`) |
| **VS Code Markdown** [V] | Configurable link rules | `settings.json` | No (advisory) | Flag broken links | No (LSP) | LSP `Diagnostic` (per-rule level/code) |
| **obsidian-nvim** [V] | Special key subset | Lua config | No (advisory) | Warns, preserves metadata | No | Lua warning log |
| **Obsidian Properties** [V] | Global property types | `.obsidian/types.json` | No (advisory) | Orange caution icon | Yes (inputs) | UI tooltip ("Type mismatch") |
| **Obsidian Bases** [V] | Views & formula types | `.base` file | No (advisory) | Evaluates dynamically | Yes (grid/table)| Dynamic formula error |
| **Logseq (DB Mode)** [V] | `PropertySchema` + Tags | SQLite / Tag entities | Yes (on DB write)| Rejects/coerces in UI | Yes (inputs) | DB constraint error |
| **Dendron** [V] | Hierarchy + templates | `*.schema.yml` | No (advisory) | Ignores non-conforming | Yes (lookup) | Startup "schema malformed" log |
| **Anytype / Capacities** [V]| Object Types & Relations | Workspace DB | Yes (on edit) | UI prevents invalid input | Yes (strict) | Input validation message |
| **Tana** [V] | Supertags & Fields | Node graph | Yes (on edit) | Constrained by field type | Yes (strict) | UI input constraint |
| **Notion / Airtable** [V] | Columnar database schema | Database settings | Yes (API write) | Coerces paste; 400 on API | Yes (strict) | API `validation_error` (400) |
| **Astro Collections** [V] | Zod Schema | `content.config.ts` | **Yes (Build gate)**| Aborts build with error | No (LSP IDE) | `ZodError` (file, path, expected) |
| **remark-lint-schema** [V] | JSON Schema (Draft 7) | `$schema` or `.remarkrc`| Opt-in gate | IDE squiggles / CI exit 1 | No (LSP) | remark-message (line, col, path) |
| **Traces (Today)** [V] | Passive `FileClass` TOML | `.traces/schemas/*.toml`| No (passive) | Schema never touches notes| No | None |

---

## 4. Synthesis: Cross-Cutting Architectural Patterns

### (a) Gating vs Advisory: The Open-World Divide

- **The Empirical Law:** Across the entire survey, **every tool operating over an open-world personal markdown vault (Obsidian, Dataview, Dendron, zk, Marksman, markdown-oxide, obsidian-nvim) is strictly ADVISORY** [V].
- **Why Gating Fails in PKM:**
  1. Notes are authored outside the tool (vim, Obsidian mobile, git sync, web clippers, scripts). Gating ingest on a schema violation either rejects the file (hiding user notes from search/query) or aborts execution, destroying trust [H].
  2. Incremental drafting is non-linear: a user frequently creates a note with `status: draft`, leaving `due:` blank or set to a placeholder string `"next week"` before formalizing it [H].
- **Where Gating Exists:** Gating appears only in:
  - *Build-time compilers* (Astro, Quarto, CI linters), where broken frontmatter produces broken production websites [V].
  - *Database-backed systems* (Logseq DB, Anytype, Airtable, Notion), where writes pass through structured transactional API boundaries [V].
- **Conclusion for Traces:** Traces core indexing and querying must remain **strictly non-gating** [P: Ticket 15 / Res03]. Schema validation belongs in an on-demand verification command (`traces check` / `traces doctor`) and background LSP diagnostics [P].

### (b) Declaration Topology: Global vs Per-Class vs Per-Note

1. **Global Property Registry (Obsidian `.obsidian/types.json`):**
   - *Design:* Property names are globally unique: `status` is always `Select` vault-wide [V].
   - *Failure Mode:* Severe namespace collision. If `#book` wants `author: "string"` and `#article` wants `author: "[[Link]]"`, or if two unrelated templates use `rating` as a number vs a text grade, Obsidian forces a global type conflict, producing permanent orange warning icons across hundreds of notes [V].
2. **Per-Class / Tag Schema (Dendron, Tana, Logseq DB, Metadata Menu, Traces `fileClass`):**
   - *Design:* Properties are scoped to a class/schema (`.traces/schemas/book.toml`) [V]. Notes bind to classes via `fileClass: book` or tags [V].
   - *Advantage:* Allows contextual typing (`rating` can be integer on `book`, float on `benchmark`). Matches human mental models of entity types [H].
3. **Per-Note `$schema` (remark-lint, JSON Schema):**
   - *Design:* Note frontmatter declares `$schema: ./schemas/book.json` [V].
   - *Failure Mode:* High authoring friction; users dislike littering note frontmatter with schema boilerplate [H].

### (c) Violation Handling: Coerce, Ignore, Flag, or Refuse?

- **Ignore (Dataview, Tasks, zk):** Silently treat invalid data as raw strings or null. Terrible for user feedback; queries silently drop notes due to type mismatch [H].
- **Refuse (Notion API, Astro build):** Unacceptable for a local markdown CLI reader [H].
- **Coerce (Notion paste, Astro `z.coerce`):** Can cause catastrophic data loss if applied destructively. Coercing `"2026-W42"` or `"circa 1920"` to null or a corrupted date destroys user intent [H].
- **Flag / Advisory Diagnostic (Obsidian Properties, rumdl, Marksman, remark-lint):** The gold standard. Read the value as an uncorrupted raw representation (`NoteFieldValue::String` or `Raw`), allow queries to inspect it via fallback semantics, but surface a structured diagnostic pointing to the exact span with the expected vs received type [P].

### (d) Writing (UI/Prompts) vs Checking (Validation): Dual-Role Tensions

- In Obsidian Templater, types exist **only for writing** (rendering input modals) [V].
- In `remark-lint-frontmatter-schema`, types exist **only for checking** (post-hoc validation) [V].
- In Tana, Logseq DB, and Obsidian Properties, types serve **both**: the schema drives the input widget during note creation/editing *and* validates existing data [V].
- **Lesson for Traces:** Ticket 16's schema subsystem must explicitly separate the **Field Definition** (which provides metadata for `ui` interactive prompts in `traces new`) from the **Field Validator** (which checks existing note values) [P]. Sharing a single enum for both confuses prompt options with evaluation constraints [H].

### (e) Reconciling YAML Inference with Declared Types

- **The Date vs String Ambiguity:** YAML 1.2 parsers automatically convert unquoted `2026-10-20` into native timestamps. If a schema declares `isbn: string`, an unquoted numeric ISBN `9780132350884` is parsed by YAML as a number. Conversely, if a schema declares `title: string`, YAML might parse `2001: A Space Odyssey` as a mapping or error unless quoted [H].
- **How Others Solve It:**
  - *Dataview:* Evaluates regex cascades over stringified representations (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3678-3689`) [V].
  - *Astro / Zod:* Applies `z.coerce` to explicitly bridge parsed types to schema types [V].
  - *Obsidian Properties:* Normalizes values when saved through the UI by quoting strings containing YAML special characters [V].
- **Traces Decision (Ticket 15):** Frontmatter parsing is schema-free and quote-insensitive; classification into `NoteFieldValue` happens without looking at schemas [V: Ticket 15]. The validator therefore inspects the resulting `NoteFieldValue` against the schema field definition: if schema expects `String` but value is `Date(2026-10-20)`, the validator checks whether the date can losslessly format as string without diagnostic error [P].

### (f) Multi-Class & Inheritance Conflict Resolution

- **Dendron:** Hierarchical patterns. If multiple schemas match a note path, the last loaded schema wins completely (no field merging) [V].
- **Tana:** Set-union of fields across all attached supertags. Identical field IDs merge; distinct fields with identical names render separately under distinct headers [V].
- **Logseq DB:** Tag inheritance via `tagExtends`. Child tags inherit parent tag properties; child property definitions shadow parent properties [V].
- **Traces Seam:** `FileClassExpander` in `src/schema` implements topological sort and field inheritance via `extends = [...]` (`src/schema/types.rs`) [V]. Child schemas override parent fields by key name [V]. If a note specifies multiple classes (`fileClass: [book, project]`), Traces needs an unambiguous merge order: left-to-right class precedence where earlier classes shadow later classes [P].

---

## 5. Ranked Lessons for Traces to Steal & Traps to Avoid

### Lessons to Steal

#### 1. Decoupled Advisory Diagnostic Pass with Precise Source Spans

- **The Pattern (rumdl, taplo, remark-lint):** Validate notes via an independent, read-only pass that maps schema errors to exact CST byte spans (`Range<usize>` / `line:col`) [V].
- **How Traces Steals It:** `src/schema` remains a passive metadata definition crate. A validation runner (e.g. `src/schema/validate.rs` or `src/diagnostics`) accepts a `&Note` and `&ResolvedSchema`, compares `NoteFieldValue` against `SchemaField`, and emits `Vec<NoteDiagnostic>` [P].
- **Adversarial Qualification (Why it might not transfer):** Traces' note parser currently uses `serde_yaml` for frontmatter, which completely strips byte spans from dictionary keys and values [V: `c-validation-precedent.md:19`]. Emitting precise squiggles for frontmatter fields requires either AST token-offset mapping (like `taplo`/`yaml-ast-parser`) or a dedicated span-preserving YAML parser [H].

#### 2. Fine-Grained Severity Toggles with Informational Default for Unresolved References

- **The Pattern (markdown-oxide, VS Code Markdown):** Default broken wikilinks to `DiagnosticSeverity::Information` or `Hint`, with per-rule configuration (`error`, `warning`, `info`, `ignore`) in config [V].
- **How Traces Steals It:** In `.traces/config.toml`, allow configuring diagnostic severities per rule code (e.g. `[diagnostics] broken-link = "info"`, `schema-type-mismatch = "warning"`). Never fail CLI queries on warnings [P].
- **Adversarial Qualification:** Config complexity bloat. In a small CLI tool, exposing dozens of severity configuration knobs before having an active LSP editor client adds maintenance weight with zero immediate user benefit [H].

#### 3. Atomic Multi-Span Fix Bundles

- **The Pattern (rumdl `Fix::with_additional_edits`):** A diagnostic fix carries a primary edit and `additional_edits: Vec<Fix>` applied atomically [V:9385-9415].
- **How Traces Steals It:** When validating frontmatter, a fix that converts a malformed inline field into frontmatter or adds a missing `fileClass` requires coordinated edits across different lines of the document [P].
- **Adversarial Qualification:** Over-engineering if Traces only provides CLI read queries and `traces new`. Fix-its are only useful if Traces implements an interactive autofix CLI (`traces fix`) or an LSP Code Action provider [H].

### Traps to Avoid

#### 1. The Obsidian Global Property Registry Trap

- **The Trap:** Storing property types in a single vault-wide registry (`types.json`) keyed by property name [V].
- **Why It Fails:** Users inevitably reuse common property names (`status`, `type`, `date`, `rating`, `author`) with different meanings and types across different note classes, leading to unresolvable vault-wide type conflicts [V].
- **Traces Defense:** Keep property definitions strictly bound to schemas (`.traces/schemas/<name>.toml`). Property types are evaluated only in the context of the note's declared `fileClass` [P].

#### 2. The Build-Gating Trap (Astro / Strict Compilers)

- **The Trap:** Refusing to parse, index, or query a note whose frontmatter violates schema constraints [V].
- **Why It Fails:** Destroys the primary value of a local-first markdown tool. A single syntax typo in a note written on mobile causes `traces list` or `traces query` to crash or hide files, infuriating the user [H].
- **Traces Defense:** Core parsing and indexing must remain 100% schema-free and non-gating [V: Ticket 15]. Schema validation is strictly opt-in via `traces check` or LSP [P].

#### 3. The Destructive Auto-Formatting Trap (Obsidian Linter Regexes)

- **The Trap:** Attempting to format or coerce frontmatter values using regex string replacement without a full YAML AST [V:244-250].
- **Why It Fails:** Strips YAML comments, corrupts multi-line strings, mangles block scalar indicators (`|`, `>`), and drops unknown fields [V:248-249].
- **Traces Defense:** Never modify markdown frontmatter unless executing an explicit user command with AST round-tripping [P].

---

## 6. Not Verified

- Whether Obsidian Bases (`.base` files) store formula definitions in YAML frontmatter or separate JSON files in private vaults [H].
- The exact performance benchmark of `jsonschema` vs manual Rust match validation on 10,000 markdown notes in a cold CLI run [H].
- Whether Logseq DB mode plans to support file-based markdown sync with bi-directional schema validation in 2026/2027 [H].

---

## 7. Questions This Raises for the Grilling

1. **Span Tracking Seam:** Does Traces currently have byte-level spans for individual frontmatter keys and values, or does `serde_yaml` discard them? If spans are missing, can schema diagnostics only point to the whole frontmatter block (`1:1`), or do we need a span-preserving frontmatter parser?
2. **Schema Ingestion Seam:** Should `traces index` store schema-typed fields in the index SQLite database, or should the index store only raw `NoteFieldValue` and evaluate schema types lazily on query/check?
3. **Multi-Class Precedence:** When a note declares multiple classes in frontmatter (`fileClass: [book, project]`), what is the authoritative merge order if both define the same field with incompatible types?
4. **Interactive `traces new` Gating:** Should `traces new` interactively reject invalid inputs (acting like Tana/Notion UI), even though existing note validation is strictly advisory?
5. **Diagnostic Ownership:** Should the diagnostic type (`NoteDiagnostic`) live in `src/schema`, in `src/note`, or in a new shared `src/diagnostics` crate/module alongside future LSP link diagnostics?
