# Content Systems and Frontmatter Schemas: Precedent Search for Ticket 16

Research date: 2026-10-21
Context: External architectural precedents for markdown frontmatter schemas,
field typing, inheritance/composition, validation timing, and diagnostics.
Scope: Content Management Systems (CMS), Static Site Generators (SSG), and
publishing frameworks.

---

## 1. Scope

This report examines fourteen real-world content systems that define schemas
and validation for markdown frontmatter and structured document metadata:

1. Astro Content Collections & Content Layer [V: read official documentation
   and GitHub issues]
2. Contentlayer and Contentlayer2 [V: read repository discussions and npm specs]
3. Velite [V: read documentation and source configuration]
4. Nuxt Content (v3 Collections) [V: read official documentation and guides]
5. Front Matter CMS (VS Code extension) [V: read official documentation]
6. Decap CMS & Sveltia CMS [V: read official documentation and GitHub issues]
7. Keystatic [V: read documentation and GitHub issues]
8. Sanity Studio & Content Lake [V: read official documentation and guides]
9. Quarto [V: read official documentation, practical guide, and issues]
10. MyST Markdown (mystmd / myst-frontmatter) [V: read documentation and repo]
11. Hugo [V: read official discourse discussions and documentation]
12. Docusaurus [V: read source code and official documentation]
13. Payload CMS [V: read official documentation and versioning guides]
14. Contentful [V: read official documentation and API references]

Systems read and verified against documentation and source during this research
are tagged [V] with URLs. Inferences and background recollections are tagged
[H].

---

## 2. Per-Project Analysis

### 2.1 Astro Content Collections & Content Layer

- Feature: In Astro v2-v4, `src/content/config.ts` defined collections via
  `defineCollection({ schema: z.object({...}) })`. In Astro v5, the Content
  Layer API (`astro:content`) introduced decoupled loaders and schemas [V:
  <https://v4.docs.astro.build/en/reference/modules/astro-content/>].
- Fields, types, and references: Schemas are written using Zod. Cross-collection
  relationships use `reference('collectionName')`, which checks that the string
  ID or slug exists in the target collection [V: ibid].
- Validation timing: Validation runs at build time (`astro build`) and during
  development sync (`astro sync` / dev server reload). It is strictly gating.
  Invalid frontmatter terminates the build process [V:
  <https://v4.docs.astro.build/en/guides/content-collections/>].
- Architectural placement: Schema parsing sits between file ingestion and route
  generation. Markdown frontmatter is parsed with gray-matter, passed through
  Zod, and stored in an internal Vite virtual module cache [V: ibid].
- Storage vs parsing: Content files remain raw Markdown/MDX in the repository.
  Astro generates static TypeScript declaration files (`.astro/types.d.ts`) to
  provide editor autocompletion and static type safety in `.astro` components
  [V: https://v4.docs.astro.build/en/guides/content-collections/].
- Complaints and regressions:
  - Raw Zod error dumps: Users complained that validation errors dumped raw
    serialized Zod objects rather than readable file/line diagnostics [V:
    <https://github.com/withastro/astro/issues/15976>].
  - Dev server crashes: Malformed frontmatter caused the development server to
    crash rather than emitting an advisory warning [V:
    <https://github.com/withastro/astro/issues/13595>].
  - Silent `astro check` failure: Schema mismatches caused `astro check` to exit
    with status 1 without diagnostics unless `astro sync` ran first [V:
    <https://github.com/withastro/astro/issues/12724>].
  - Build blockage on missing references: Broken `reference()` links halt the
    entire site compilation, making partial drafts impossible to maintain in the
    same repository without dummy records [V:
    <https://github.com/withastro/astro/issues/12680>].
- Key lesson: Build-time hard gating with strict schemas creates severe friction
  for content authors. A single typo in an un-rendered draft blocks production
  deployments across the entire workspace.

### 2.2 Contentlayer & Contentlayer2

- Feature: Content SDK converting Markdown/MDX into type-safe JSON data
  artifacts via code generation [V: https://contentlayer.dev/docs].
- Fields and types: Schemas are declared in `contentlayer.config.ts`
  via `defineDocumentType` and `defineNestedType`. Fields specify primitive
  types (`string`, `date`, `number`, `boolean`, `json`, `nested`, `list`).
  Composition is supported via nested types; inheritance is not natively
  supported [V: ibid].
- Validation timing: Gating at build time (`contentlayer build`) and during
  watch mode. Invalid fields abort artifact generation [V: ibid].
- Architectural placement: Runs as a build plugin / CLI pre-processor. It emits
  compiled JSON documents and TypeScript types into `.contentlayer/generated`.
- Storage vs parsing: Storage is standard Markdown with frontmatter. The runtime
  app never parses Markdown; it imports the pre-parsed, pre-validated JSON
  artifacts [V: ibid].
- Complaints and failure story:
  - Tight bundler coupling and abandonment: Contentlayer tightly coupled its
    watcher and esbuild runner into Next.js internals. When Next.js moved to
    Turbopack and updated its App Router, Contentlayer broke repeatedly and was
    ultimately abandoned by its core creators in 2023 [V:
    <https://github.com/contentlayerdev/contentlayer/issues/429>].
  - The community had to fork the project to `contentlayer2`
    (`timlrx/contentlayer2`) to patch compatibility [V:
    <https://github.com/timlrx/contentlayer2/discussions/3>].
- Key lesson: Coupling content schema validation to build-time code generation
  and bundler internals makes the schema engine fragile to toolchain shifts.

### 2.3 Velite

- Feature: Modern, lightweight content-to-data tool designed as an alternative
  to Contentlayer. It uses Zod schemas to convert Markdown/MDX into typed JSON
  [V: https://velite.js.org/].
- Fields, types, and composition: Configured in `velite.config.js`. Uses Zod
  extended with custom schemas (`s.object`, `s.string`, `s.markdown`, `s.file`,
  `s.image`). Reusability is achieved via JavaScript object composition and Zod
  schema chaining (`.extend()`) [V: https://velite.js.org/guide/custom-schema].
- Validation timing and severity: Built-in configurable gating:
  `strict: boolean` (default: `false`).
  "If true, throws an error and terminates the process if any schema validation
  fails. Otherwise, a warning is logged but the process does not terminate" [V:
  <https://velite.js.org/reference/config>].
- Architectural placement: Standalone build-time and watch-time data preparation
  layer. Writes static JSON and TypeScript types to `.velite/` [V: ibid].
- Storage vs parsing: Files remain Markdown on disk.
- Key lesson: Velite explicitly made advisory validation (`strict: false`) the
  default behavior because fatal build errors during content drafting frustrate
  authors. Gating is an opt-in policy.

### 2.4 Nuxt Content (v3 Collections)

- Feature: Git-based content engine for Nuxt with collection definitions in
  `content.config.ts` [V: https://content.nuxt.com/docs/collections/define].
- Fields, types, and indexing: Uses Zod or Valibot for schema definitions.
  Supports SQL database indexing (`indexes: [{ columns: ['category'] }]`)
  generating columns in SQLite, LibSQL, or PostgreSQL [V: ibid].
- Collection assignment: Collections are mapped strictly via file glob patterns
  (`source: 'blog/*.md'`), NOT via frontmatter class tags [V: ibid].
- Multi-collection restriction: A document can belong to only one collection at
  a time: "Currently, a document is designed to be present in only one
  collection
  at a time. If a file is referenced in multiple collections, live reload will
  not work correctly" [V: https://github.com/nuxt/content/issues/2966].
- Validation timing: Runs at content import/sync time before loading into
  SQLite.
- Key lesson: Binding types to file paths simplifies index generation and SQL
  column mapping, but strictly prevents multi-class polymorphism. When a single
  document matches two collections, caching and live-reload break.

### 2.5 Front Matter CMS (VS Code)

- Feature: Headless CMS embedded directly inside VS Code for static site
  generators [V: https://frontmatter.codes/docs/content-creation/content-types].
- Content types and inheritance: Content types are configured in JSON
  (`frontmatter.json` or `.frontmatter/config/taxonomy/contenttypes/*.json`).
  Fields include `string`, `number`, `datetime`, `boolean`, `image`, `file`,
  `choice`, `list`, `taxonomy`, `fields`, `fieldCollection`, `dataFile`.
  Inheritance across content types is NOT implemented via class inheritance.
  Instead, composition is achieved via `fieldCollection` and `fieldGroup` (a
  named group of fields defined once and referenced across content types) [V:
  <https://frontmatter.codes/docs/content-creation/fields>].
  Configuration inheritance exists at the file level via `frontMatter.extends`
  (extending remote or local settings files) [V:
  <https://frontmatter.codes/docs/settings>].
- Assignment to documents: Content types can be assigned in frontmatter via
  `type: <name>` or bound to specific folders via `pageFolders` [V:
  <https://frontmatter.codes/docs/content-creation/content-types>].
- External value sources: Uses `dataFile` fields to populate select choices
  from external JSON/YAML data files (e.g. an authors registry) [V:
  <https://frontmatter.codes/docs/content-creation/fields>].
- Validation timing: Runs in the VS Code editor panel while editing.
  Advisory notifications for required fields; does not block disk writes or file
  saving in the raw markdown editor [V: ibid].
- Key lesson: Real-world editor tooling uses field-group composition
  (`fieldCollection`) rather than multi-class inheritance to share fields across
  content models. External data files (`dataFile`) are essential for shared
  taxonomies.

### 2.6 Decap CMS (formerly Netlify CMS) & Sveltia CMS

- Feature: Git-backed, web-based content management interfaces that read a
  YAML schema (`config.yml`) and commit changes directly to Git [V:
  <https://decapcms.org/docs/widgets/>].
- Schema structure: Collections define a list of `fields` using built-in
  widgets (`string`, `number`, `boolean`, `datetime`, `select`, `relation`,
  `list`, `object`).
- Constraints and validation:
  - `required`: defaults to `true` on all fields (`required: false` to make
    optional) [V: ibid].
  - `pattern`: regex validation with error message `['^[A-Z]', 'Must start
    with capital']`.
  - `relation`: dynamic link to other collections (`collection`, `value_field`,
    `search_fields`).
- Validation timing and gating:
  - In Decap CMS: Validation runs in the browser editor and gates publishing.
    Saving is blocked if a required field is empty or regex fails [V: ibid].
  - In Sveltia CMS (Decap successor): Config is validated at startup on the
    login screen to catch broken relation targets before editing starts [V:
    <https://sveltiacms.app/en/docs/config-basics>].
  - Draft workflow exception: Sveltia CMS explicitly relaxes required field
    checks during draft editing: "Draft entries can be saved with required
    fields left empty without raising UI errors... Required fields are strictly
    enforced as soon as an entry tries to leave the drafting stage" [V:
    <https://sveltiacms.app/en/docs/workflows/editorial>].
- Complaints and regressions:
  - Nested validation bugs: Decap CMS experienced severe regressions where
    nested `object` or `list` widgets failed validation erroneously on save [V:
    <https://github.com/decaporg/decap-cms/issues/7383>].
  - List regex bypass: Regex `pattern` validation was silently ignored on list
    items [V: https://github.com/decaporg/decap-cms/issues/2382].
- Key lesson: A `required: true` default in schema design blocks iterative
  authoring unless drafts are explicitly exempted from gating.

### 2.7 Keystatic

- Feature: TypeScript-first, Git-backed content management tool by Thinkmill
  [V: https://keystatic.com/docs/collections].
- Schema structure: Configured in `keystatic.config.ts`. Defines collections and
  singletons with field helpers: `fields.text`, `fields.number`,
  `fields.relationship`, `fields.object`, `fields.array`, `fields.conditional`,
  `fields.markdoc`.
- Field validation: Configured per field: `validation: { length: { min, max },
  isRequired: true }` [V: https://keystatic.com/docs/fields/text].
- Validation timing: Runs in the Keystatic Admin UI during editing. Gating on UI
  save, but raw Markdown files in Git can bypass it completely [V: ibid].
- Complaints and failure stories:
  - Schema drift between CMS and SSG: When Keystatic's schema and an SSG's
    schema (such as Astro or Nuxt Content) are maintained separately, they
    drift, causing runtime parse crashes [V:
    <https://github.com/mechanistic-org/portfolio/issues/61>].
- Key lesson: When schema definitions are duplicated between an editing UI and
  a build tool, drift is inevitable. The schema engine must serve as a single
  source of truth.

### 2.8 Sanity Studio & Content Lake

- Feature: Cloud content platform decoupling a schemaless document database
  (Content Lake) from a customizable React editor (Sanity Studio) [V:
  <https://www.sanity.io/docs/content-lake/schema-validation-and-the-content-lake>].
- Validation rules and severity levels:
  - Validation rules are built via a chainable `Rule` builder:
    `Rule => Rule.required().min(5).max(100)`.
  - Severity levels: `Rule.error(...)` (blocks publishing in the Studio),
    `Rule.warning(...)` (flags an issue in the Studio but allows publishing),
    and `Rule.info(...)` (informational hint) [V:
    <https://www.sanity.io/docs/studio/validation>].
- Architectural placement and timing:
  - Intentional schemaless storage: The Content Lake does NOT enforce schema
    validation rules!
  - "Validation rules set up on your document types or fields will only run
    within the studio (client-side). They will not run when using this mutation
    API (server-side)" [V: https://www.sanity.io/docs/http-mutations].
  - API mutations (`@sanity/client`, webhooks, scripts) write raw JSON documents
    directly into the Content Lake without checking schema rules [V: ibid].
  - On-demand auditing: The CLI command `sanity documents validate` audits an
    existing dataset against current schema rules on demand [V:
    <https://www.sanity.io/docs/content-lake/schema-and-content-migrations>].
- Inheritance vs composition: Sanity has no class inheritance. Reusability is
  handled entirely through object types and array member definitions [V:
  <https://www.sanity.io/docs/studio/object-type>].
- Key lesson: Sanity's architectural separation of schemaless storage from
  client-side, multi-severity validation rules is identical in philosophy to
  Traces' design: parsing and storage stay schema-free, validation is advisory
  and on-demand, and host interfaces enforce policy.

### 2.9 Quarto

- Feature: Multi-language scientific publishing system based on Pandoc [V:
  <https://quarto.org/>].
- Schema engine: Quarto includes a custom YAML schema validator implemented in
  TypeScript (`src/core/schema/yaml-schema.ts` and
  `src/core/lib/yaml-validation/validator.ts`) [V:
  <https://github.com/quarto-dev/quarto-cli/blob/main/src/core/schema/yaml-schema.ts>].
- Open vs closed schemas:
  - Closed schemas: Used for system options (e.g. `format: html`, `lightbox`).
    Unknown property names trigger errors with fuzzy "Did you mean X?"
    suggestions [V: https://quarto-tdg.org/yaml].
  - Open schemas: Top-level document metadata allows arbitrary unknown keys so
    authors can define custom metadata for templates and shortcodes without
    validation errors [V: ibid].
- Validation timing: Runs at render time (`quarto render`). Gating: invalid
  frontmatter fails the render [V: ibid].
- Diagnostic quality and failures:
  - Union type diagnostics: When an option accepts a union (e.g. `false` or
    `Array<Link>`), Quarto's validator frequently reported confusing messages
    like "has value <http://quarto.org>, which must instead be 'false'" because
    it printed the first branch of the union [V: https://quarto-tdg.org/yaml].
- Key lesson: Highlighting typos with "Did you mean X?" is immensely helpful,
  but strict closed schemas on document frontmatter break user extensibility.
  Distinguishing between closed schema definitions and open metadata passthrough
  is critical.

### 2.10 MyST Markdown (mystmd & myst-frontmatter)

- Feature: Scientific publishing stack for Jupyter Book and web documents [V:
  <https://mystmd.org/>].
- Schema architecture: Frontmatter parsing and validation are isolated in the
  independent npm package `myst-frontmatter` [V:
  <https://www.npmjs.com/package/myst-frontmatter>].
- Scoping: Differentiates between page-level frontmatter (`01-paper.md`) and
  project-level configuration (`myst.yml`). A frontmatter cascade allows page
  values to override project defaults [V:
  <https://mystmd.org/guide/frontmatter>].
- Validation timing: Runs during manifest generation and build. Emits structured
  warnings and errors via diagnostic logging [V:
  <https://github.com/jupyter-book/mystmd/blob/main/packages/myst-cli/src/build/site/manifest.ts>].
- Key lesson: Isolating frontmatter types and validators into a standalone
  library (`myst-frontmatter`) completely decoupled from the rendering engine
  keeps the validator pure and reusable across CLI, LSP, and web tools.

### 2.11 Hugo

- Feature: Fast Go-based static site generator [V: https://gohugo.io/].
- Data model: No formal schema validation engine. Frontmatter is parsed into
  generic Go maps (`.Params`).
- Front matter cascade: Hugo implements inheritance through its `cascade`
  feature in site config or `_index.md`: frontmatter key-value pairs are pushed
  down the content directory tree to matching descendant pages [V:
  <https://gohugo.io/content-management/front-matter/#front-matter-cascade>].
- Archetypes: Template files (`archetypes/default.md`) populated only on
  `hugo new content`. They do not validate existing files [V:
  <https://discourse.gohugo.io/t/make-frontmatter-fields-required-per-archetype-enforce-schema/33498>].
- Complaints and user pain:
  - Silent failures: Hugo users repeatedly request schema validation because
    frontmatter typos (e.g. `alises` instead of `aliases`) fail silently,
    rendering empty strings in templates without warnings [V:
    <https://discourse.gohugo.io/t/how-to-validate-frontmatter/54085>].
  - Template crashes: When templates expect a date and receive a string, Hugo
    panics during HTML generation deep into the build [V:
    <https://discourse.gohugo.io/t/feature-request-frontmatter-validation-type-checking/55900>].
- Key lesson: A schemaless system with zero validation leads to silent data
  rot, missing fields on published pages, and hard-to-debug template failures.

### 2.12 Docusaurus

- Feature: React-based documentation generator [V: https://docusaurus.io/].
- Schema structure: In `@docusaurus/plugin-content-docs`, frontmatter is
  validated using Joi schemas (`validateFrontMatter`) [V:
  <https://github.com/facebook/docusaurus/blob/main/packages/docusaurus-plugin-content-docs/src/frontMatter.ts>].
- Open world model: The Joi schema explicitly enables `.unknown()`: recognized
  fields (`id`, `title`, `tags`, `sidebar_position`) are strictly typed, but
  unrecognized fields are allowed through without errors, typed as
  `DocFrontMatter & Record<string, unknown>` [V: ibid].
- Validation timing: Runs at site build and development startup. Gating for
  malformed known fields, silent passthrough for unknown fields.
- Key lesson: An open-world frontmatter policy (`.unknown()`) prevents the
  schema validator from breaking custom user workflows while maintaining type
  safety for known core fields.

### 2.13 Payload CMS

- Feature: TypeScript-first headless CMS with code-defined schemas [V:
  <https://payloadcms.com/docs/configuration/overview>].
- Schema structure: Defined via field arrays. Field types include `text`,
  `number`, `select`, `relationship`, `upload`, `array`, `group`, `blocks`.
- Drafts and validation bypass:
  - Payload explicitly relaxes validation when `draft: true` is enabled:
    "When a document is saved or created with draft: true... required fields
    are not strictly enforced. This core feature allows content editors to save,
    autosave, or update incomplete documents that are missing mandatory data
    without triggering validation errors" [V:
    <https://payloadcms.com/docs/versions/drafts>].
  - Publishing (`_status: 'published'`) re-enforces full validation [V: ibid].
- Key lesson: Incomplete, in-progress documents MUST be permitted to bypass
  `required` validation; otherwise, autosave and drafting become impossible.

### 2.14 Contentful

- Feature: Enterprise headless CMS with API-first content models [V:
  <https://www.contentful.com/developers/docs/>].
- Constraints: Built-in validations: `required`, `size: { min, max }`,
  `regexp: { pattern }`, `in: [...]` (predefined values), `linkContentType:
  [...]`
  (constrains references to specific target models) [V:
  <https://www.contentful.com/help/fields/available-validations/>].
- Validation timing:
  - Draft creation via the Content Management API (CMA) accepts incomplete
    records without validating required fields [V:
    <https://www.contentful.com/developers/docs/references/content-management-api/entries/create-an-entry/>].
  - Publishing gates: An entry cannot transition from Draft to Published if
    any validation rule is violated [V:
    <https://www.contentful.com/help/content-preview/live-preview-publishing/>].
- Key lesson: Constraining reference targets by content type (`linkContentType`)
  is an industry standard for cross-document link integrity.

---

## 3. Cross-Project Comparison Matrix

| Project | Field Types | Required vs Optional | Constraints (min/max/regex/enum) | Inheritance / Composition | Type Declaration | Validation Timing & Severity | Open vs Closed World | External Values | Diagnostics Surface |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Astro** | Zod types (string, number, date, boolean, array, object) | Required by default in Zod; explicit `.optional()` | Full Zod constraints (`min`, `max`, `regex`, `enum`) | Composition via Zod `.extend()` or object spread; no class inheritance | Folder / glob in `config.ts` | Build time (`astro build` / `sync`); hard gating | Closed by default in Zod object; passthrough requires `.passthrough()` | Can load external JSON/API via custom loaders (v5) | CLI error with serialized Zod issues |
| **Contentlayer** | String, number, boolean, date, json, nested, list | Explicit `required: true / false` | Basic type constraints | `defineNestedType` composition; no inheritance | File path glob in document config | Build & watch time; hard gating | Closed world for declared document type | None built-in | CLI compilation error with document path |
| **Velite** | Zod types + custom `s.markdown()`, `s.image()`, `s.file()` | Required by default; explicit `.optional()` | Full Zod constraints | Composition via Zod chaining; no inheritance | File pattern glob in collection | Build time; advisory warning by default (`strict: false`), gating optional | Closed by default in Zod | Supports custom loaders | CLI formatted diagnostic logs |
| **Nuxt Content v3** | Zod / Valibot types | Required by default; explicit `.optional()` | Full Zod / Valibot constraints | Composition via validator chaining; no inheritance | File pattern glob in collection | Collection sync time (build/dev); gating | Closed per schema | External git repo sources supported | CLI error messages |
| **Front Matter CMS** | String, number, datetime, boolean, image, file, choice, list, taxonomy, fields, dataFile | Optional by default; explicit `required: true` | `numberOptions: { min, max, step }`, `choices: [...]` | Composition via `fieldGroup` and `fieldCollection`; settings `extends` | Frontmatter `type` field or folder binding | Editor UI; advisory notification, never gates raw file edits | Open world (extra frontmatter keys preserved) | `dataFile` loads options from JSON/YAML | VS Code editor notification banner |
| **Decap CMS** | String, number, boolean, datetime, select, relation, list, object | Required by default (`required: false` to make optional) | `pattern: [regex, msg]`, `min`, `max`, `step` | Nested `object` and `list` widgets; no schema inheritance | Folder collection in `config.yml` | Editor UI; gates save/publish in browser UI | Open world (unrecognized frontmatter preserved) | `relation` widget queries other collections | Inline UI field error labels |
| **Sveltia CMS** | Decap-compatible widgets | Required by default (`required: false` to optional) | `pattern`, `min`, `max`, `minlength`, `maxlength` | Nested widgets; no schema inheritance | Folder collection in `config.yml` | Config validated at startup; drafts bypass required; publish gates | Open world | `relation` widget | Modal startup errors; inline UI field warnings |
| **Keystatic** | Text, number, select, relationship, image, file, object, array, blocks, markdoc | Optional by default; `isRequired: true` in validation | `validation: { length: { min, max } }`, options array | Composition via `fields.object` and `fields.blocks`; no inheritance | Folder / path in `keystatic.config.ts` | Editor UI; gates save in UI; files in Git bypass | Open world (preserves untouched YAML) | None built-in | Form validation error text under inputs |
| **Sanity** | String, number, boolean, date, datetime, reference, array, object, document | Optional by default; `Rule.required()` | `Rule.min()`, `.max()`, `.regex()`, custom JS rules | Composition via `object` types and array members; no inheritance | Document `_type` property | Studio: advisory warnings, errors gate publish; Content Lake: schemaless | Schemaless lake; Studio validates configured fields | Cross-dataset references, external data via mutations | Studio banner, list of errors/warnings, CLI audit |
| **Quarto** | YAML scalars, mappings, sequences, formats | System options have explicit required/optional | Range, regex, enum, boolean 1.2 | Metadata merge cascade; no class inheritance | Top-level document frontmatter or `_quarto.yml` | Render time (`quarto render`); gating | Hybrid: closed schemas for options, open for custom metadata | External `_metadata.yml` files | CLI error with line/col, snippets, and "Did you mean?" |
| **MyST Markdown** | Common frontmatter fields (title, authors, tags, exports, etc.) | Standard fields specified in `myst-frontmatter` | Type validation, enum checks, URL checks | Page frontmatter overrides project frontmatter | Markdown frontmatter and `myst.yml` | Build time; structured warnings and errors | Open world (extra keys allowed) | External references via xref | CLI diagnostic log with file and property paths |
| **Hugo** | Generic Go data types (untyped maps) | No schema enforcement | None built-in | Front matter `cascade` by folder hierarchy; archetypes | Markdown frontmatter; archetypes | Never validates frontmatter; template render fails | Open world (all keys accepted) | Data files in `data/` accessible in templates | Go template panic or silent empty string |
| **Docusaurus** | Joi types (string, uri, number, boolean, array) | Optional by default in Joi; explicit `.required()` | Joi regex, uri, enum, range constraints | Plugin options inheritance; no class inheritance | Markdown frontmatter in `docs/` | Build & dev startup; gates on invalid known fields | Open world via Joi `.unknown()` | None built-in | CLI build error with Joi diagnostic path |
| **Payload CMS** | Text, number, select, relationship, upload, array, group, blocks | Optional by default; explicit `required: true` | `min`, `max`, `minLength`, `maxLength`, custom `validate` | Composition via `group` and `blocks`; TypeScript mixins | Collection config; `_status` tracks draft/published | Server API & UI; drafts bypass required; publish gates | Strict per collection | Dynamic relationship lookups | API JSON errors and UI inline warnings |
| **Contentful** | Symbol, text, integer, number, date, boolean, json, reference | Optional by default; explicit `required: true` | `size: { min, max }`, `regexp`, `in: [...]`, `linkContentType` | Reusable content types linked via references; no inheritance | Entry `sys.contentType` | API/Editor: drafts bypass required; publish gates | Closed per content type | References to entries in other content types | API error response with field violation details |

---

## 4. Challenges to Traces' Design

### 4.1 Advisory-Only Validation vs. Gating

- Traces decision: Validation is an advisory, on-demand check. It never gates
  the read or index path, and never enters the index or fingerprint. Gating is
  a host policy over stable violation codes.
- Supporting evidence:
  - Sanity's Content Lake has proven over years that decoupling storage from
    validation prevents database migration gridlock and enables multi-tool
    workflows [V:
    <https://www.sanity.io/docs/content-lake/schema-validation-and-the-content-lake>].
  - Sveltia CMS and Payload CMS explicitly bypass required-field validation for
    drafts and autosaves; enforcing gating during writing halts the authoring
    process [V: <https://sveltiacms.app/en/docs/workflows/editorial>,
    <https://payloadcms.com/docs/versions/drafts>].
  - Velite chose `strict: false` (warnings only) as its default because build
    crashes during content drafting were the top developer complaint with
    Contentlayer [V: https://velite.js.org/reference/config].
- Contradicting evidence:
  - Astro and Quarto gate builds on validation errors because shipping broken
    HTML or PDFs to production is considered worse than failing the build [V:
    <https://v4.docs.astro.build/en/guides/content-collections/>,
    <https://quarto-tdg.org/yaml>].
- Recommendation: KEEP advisory-only in core. However, for MCP agent write
  tools and CLI check commands, establish clear host policies: interactive
  agent edits should block only on high-severity violations (or newly introduced
  violations via Traces' delta rule), while static export commands (Pandoc,
  HTML)
  may optionally gate.

### 4.2 Required Fields in a Note-Taking / PKM System

- Traces decision: Retains `required: bool` (missing means absent, `null`, blank
  string, or empty list; `false` and `0` are present).
- Supporting evidence:
  - Required fields are standard across CMSs (Contentful, Sanity, Decap,
    Payload).
- Contradicting evidence:
  - Every content system that enforced `required` during initial note creation
    or drafting received intense user complaints. Decap CMS's `required: true`
    default is widely cited by developers as frustrating because starting a stub
    note requires filling in every field immediately.
  - Sveltia CMS had to add an explicit exception: draft entries can be saved
    with required fields empty [V:
    <https://sveltiacms.app/en/docs/workflows/editorial>].
  - In a PKM vault, notes frequently begin as quick captures or incomplete
    stubs. If an MCP tool or CLI check gates saving on `required`, quick capture
    is broken.
- Recommendation: KEEP `required` in the schema definition, but ensure that the
  default host policy for interactive note creation treats `missing-required` as
  an advisory warning rather than a blocking error.

### 4.3 Multi-Class Inheritance with "First Wins" Conflict Rule

- Traces decision: Classes support `extends` (multiple parents, first wins). A
  note can name multiple classes in frontmatter (`class = ["book", "article"]`).
  The first-listed class wins the whole declaration on conflict, with an
  advisory `schema.class-conflict` warning.
- Contradicting evidence:
  - Sitecore data templates allowed multiple inheritance with "first wins" based
    on the ordered list of base templates. This caused notorious silent bugs:
    reordering templates in the CMS UI silently altered the winning validation
    rules, default values, and field types [V:
    <https://sitecore.stackexchange.com/questions/1884/what-happens-if-the-same-field-name-is-used-in-two-separate-inherited-data-templ>].
  - Umbraco explicitly banned multiple inheritance collisions: if two composed
    document types declare the same property alias, Umbraco throws a fatal
    `InvalidCompositionException` rather than guessing a winner [V:
    <https://github.com/umbraco/Umbraco-CMS/issues/11202>].
  - Modern content systems (Front Matter CMS, Sanity, Keystatic, Velite,
    Payload) completely avoided multiple class inheritance. They adopted
    **composition** (`fieldGroup`, nested object types, or mixins) where field
    sets are explicitly included [V:
    <https://frontmatter.codes/docs/content-creation/fields>,
    <https://www.sanity.io/docs/studio/object-type>].
- Recommendation: CHANGE or DEFER multiple class inheritance. "First wins" on
  field collisions creates invisible, order-dependent behavior that confuses
  both human users and LLM agents. If a note needs multiple classes, Traces
  should either:
  1. Treat field collisions as a hard schema error (like Umbraco), requiring
     explicit disambiguation; or
  2. Adopt composition (mixins / field groups) rather than class inheritance.

### 4.4 Class Declaration: Frontmatter vs. Folder / Glob / Tag

- Traces decision: Notes declare class(es) in a frontmatter field (default
  `class`, can be a list).
- Supporting evidence:
  - Front Matter CMS supports `type: <name>` in frontmatter so content types
    can move freely across directories [V:
    <https://frontmatter.codes/docs/content-creation/content-types>].
  - Sanity documents declare `_type` inside the document body [V:
    <https://www.sanity.io/docs/studio/document-type>].
  - In PKM vaults, notes are frequently organized by project or status rather
    than strict folder hierarchies. Forcing folder-based typing (like Astro or
    Nuxt Content) prevents cross-cutting classifications.
- Contradicting evidence:
  - Astro, Nuxt Content, Hugo, and Decap CMS bind schemas to folder paths
    (`content/posts/*.md`). This allows instant schema resolution without
    reading file contents or parsing frontmatter.
- Recommendation: KEEP `class` in frontmatter. For a personal knowledge base,
  frontmatter typing is far more flexible than rigid folder hierarchies.
  However, Traces should allow optional folder-based fallback defaults (similar
  to Hugo's `cascade` or Front Matter CMS's `pageFolders`) so that notes in a
  given folder automatically inherit a default class if none is declared.

### 4.5 External Value Sources (Select Lists)

- Traces decision: Select `values` can be external JSON lists of
  `value`/`label`/`link`.
- Supporting evidence:
  - Front Matter CMS includes a first-class `dataFile` field type specifically
    to
    populate choice options from external JSON/YAML files (e.g. an authors list)
    [V: https://frontmatter.codes/docs/content-creation/fields].
  - Contentful and Sanity rely heavily on references to central records rather
    than hardcoded enum strings [V:
    <https://www.sanity.io/docs/studio/reference-type>].
- Recommendation: KEEP external value sources. Decoupling taxonomy values from
  individual schema files prevents schema duplication and supports growing PKM
  vocabularies.

### 4.6 Strict Schema TOML Format vs. Open Note Metadata

- Traces decision: Schemas are strict TOML (unknown keys are errors). Note
  checking produces `unknown field` violations with `closest` match via
  `strsim`, which hosts can filter.
- Supporting evidence:
  - Strict schema definitions prevent typos in configuration keys.
  - Quarto's "Did you mean X?" suggestions for unknown properties proved
    immensely popular with users [V: https://quarto-tdg.org/yaml].
  - Docusaurus uses Joi with `.unknown()`, ensuring that unknown custom fields
    do not break existing documents while known fields are validated [V:
    <https://github.com/facebook/docusaurus/blob/main/packages/docusaurus-plugin-content-docs/src/frontMatter.ts>].
- Recommendation: KEEP strict TOML for schema definitions, and KEEP unknown
  note fields as filterable advisory diagnostics.

### 4.7 Things Traces Has Not Considered That Precedents Found Essential

1. Schema drift between tools: In a markdown vault, Obsidian, Traces,
   frontmatter
   CMSs, and external scripts all touch the same frontmatter. In Keystatic
   (issue #61), schema drift between the CMS and static site generator caused
   silent breakage [V: https://github.com/mechanistic-org/portfolio/issues/61].
   Traces needs a way to validate that schemas themselves match the actual
   frontmatter usage in the vault (a schema health check).
2. Conditional / contextual fields: Front Matter CMS, Keystatic, and Decap CMS
   all discovered that schemas need conditional visibility (e.g. if
   `format == 'book'`, show `isbn`; if `status == 'published'`, require
   `publish_date`) [V: https://keystatic.com/docs/fields/conditional].
3. Union diagnostics: As demonstrated by Quarto, reporting the wrong branch of a
   union type creates deeply confusing diagnostics [V:
   <https://quarto-tdg.org/yaml>]. Traces must report all permitted variants
   when a multi-type check fails.

---

## 5. Failure Stories and Regrets

- Astro Content Collections Raw Zod Dumps:
  Astro emitted serialized Zod errors when validation failed, obscuring the
  offending file and line number and confusing developers [V:
  <https://github.com/withastro/astro/issues/15976>].
- Astro Dev Server Crash on Invalid Frontmatter:
  Invalid frontmatter in a newly drafted file caused the Astro dev server to
  crash outright rather than logging an advisory warning and continuing to serve
  other pages [V: https://github.com/withastro/astro/issues/13595].
- Contentlayer Abandonment and Bundler Coupling:
  Contentlayer coupled its schema watcher directly to esbuild and Next.js
  internals. When Next.js updated its bundler architecture, Contentlayer broke
  repeatedly, leading to project abandonment and requiring a community fork
  [V: <https://github.com/contentlayerdev/contentlayer/issues/429>,
  <https://github.com/timlrx/contentlayer2/discussions/3>].
- Sitecore "First Wins" Multiple Inheritance Silent Corruption:
  Sitecore's use of "first wins" in base template inheritance resulted in
  silent configuration changes when developers reordered template lists in the
  UI, causing subtle production data bugs [V:
  <https://sitecore.stackexchange.com/questions/1884/what-happens-if-the-same-field-name-is-used-in-two-separate-inherited-data-templ>].
- Umbraco Property Collision Lockout:
  Umbraco suffered from subtle data corruption when multiple compositions shared
  field aliases, forcing maintainers to introduce a hard
  `InvalidCompositionException` that rejected collisions at save time [V:
  <https://github.com/umbraco/Umbraco-CMS/issues/11202>].
- Decap CMS Nested Validation Breakage:
  A regression in Decap CMS v3.6.0 broke validation on nested `object` and
  `list`
  widgets, preventing users from saving valid documents [V:
  <https://github.com/decaporg/decap-cms/issues/7383>].
- Quarto Obtuse Union Error Diagnostics:
  Quarto's YAML validator reported that an option "must instead be 'false'" when
  given a URL because it only printed the first branch of a boolean/array union,
  baffling authors [V: https://quarto-tdg.org/yaml].
- Hugo Silent Typo Failures:
  Hugo's total lack of frontmatter validation causes field typos (like `alises`)
  to fail silently, leaving live sites with missing metadata and broken layouts
  [V: https://discourse.gohugo.io/t/how-to-validate-frontmatter/54085].

---

## 6. Transferable Lessons

1. Separate Schemaless Storage from Advisory Validation:
   Sanity, Payload CMS, Sveltia CMS, and Contentful all agree: storage and
   ingestion must remain schemaless. Validation belongs in client/host layers
   (Studio, editor, CLI check) and must not block drafts or raw file writes.
2. Composition Beats Multiple Class Inheritance:
   The CMS and SSG industry overwhelmingly rejected multiple class inheritance
   with "first wins" rules. Composition via named field groups (`fieldGroup`,
   mixins, or Zod `.extend()`) provides predictable, transparent data models
   without hidden shadowing.
3. Multi-Severity Diagnostics (Error vs. Warning vs. Info):
   Sanity's three-tier model (`error`, `warning`, `info`) matches real-world
   authoring needs. Required-field violations during drafting should be
   `warning`, while type corruption (e.g. invalid date format or corrupt ID)
   can be `error`.
4. Open World with Closed Field Sets:
   Like Docusaurus (`.unknown()`) and Quarto (open metadata), systems that
   succeed in personal and scientific publishing validate recognized fields
   strictly while permitting unrecognized frontmatter keys to pass through
   unhindered.
5. External Data Sources for Taxonomies:
   Hardcoding enum values in schema files quickly causes maintenance headaches.
   Front Matter CMS's `dataFile` model demonstrates the value of loading
   selection options from external JSON/YAML registries.
