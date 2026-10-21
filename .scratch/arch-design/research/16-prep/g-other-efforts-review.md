# Cross-Effort Review: Sibling Wayfinder Decisions Bearing on Ticket 16

- **Topic:** Architectural audit of all sibling efforts under `.scratch/` (excluding `arch-design` and `md-pkm-lsp`), ADRs 0001–0007, and current codebase reality for Ticket 16 (Schema Note Typing and Validation).
- **Scope:** `metadata-schemas`, `note-parse`, `template-service`, `index-query`, `temporal-core`, `config-service`, `prompt-service`, `task-system`, `query-expansion`, `integration-e2e-suite`.
- **Status:** Complete research synthesis.
- **Deliverable:** `.scratch/arch-design/research/16-prep/g-other-efforts-review.md`.

---

## 1. Executive Summary: Settled vs Specified vs Shipped Reality

Across 10 sibling efforts, ADRs 0001–0007, and the current codebase, three systemic patterns emerge:

1. **Schemas are passive metadata, actively consumed only by templates (`.field()`) and queries (`@class*`):** `src/schema` was designed as vault-level metadata about note classes (ADR-0006 [V: `docs/adr/0006-file-class-schemas.md:40-43`]). In production code, schemas NEVER inspect, type, or validate note frontmatter. Note validation was **explicitly deferred** to future LSP and MCP stages [V: `docs/adr/0006:110-111`].
2. **The "Template Pre-Check" does not exist in code and was barred by the No-Declaration Format:** Templates declare no upfront parameters or schema manifests [V: `src/template/GLOSSARY.md:40-45`]. `TemplateService` renders straight to a plain `String` [V: `src/template/service.rs:155-161`]. Rendered text is **never parsed back as a `Note`** before write. A schema pre-check cannot run pre-render; it can only slot in as a post-render inspection of `rendered.content` before `write` [H].
3. **AST spans are almost entirely un-implemented in `src/note/`:** Despite extensive design prose in `md-pkm-lsp` and early `note-parse` tickets, `src/note/` retains **zero byte spans** on frontmatter keys/values, links, tags, or inline fields [V: `src/note/metadata.rs:95-98`, `src/note/links.rs:27-32`, `src/note/parser.rs:331-334`]. Headings and list items store only 1-indexed `SourceLine` [V: `src/note/heading.rs:18-22`, `src/note/lists.rs:44-50`]. `ByteSpan` exists in `src/position.rs:79-138` as a struct, but no note AST node uses it [V].

---

## 2. Per-Effort Findings (10 Sibling Efforts)

### 2.1 `metadata-schemas` (Origin of `src/schema/`)

- **Decided (Resolved):** Schemas define File Class fields via `.traces/schemas/<name>.toml` [V: `metadata-schemas/spec.md:53-55`]. Multiple inheritance DAG uses `extends` (is-a) and `excludes`, linearized via Kahn's topological sort with first-listed-parent wins and cycle errors [V: `docs/adr/0007-field-inheritance-resolution.md:47-53`]. Bounded `$ref` permits overrides pointing only to `#global/<field>` or `#<ancestor>/<field>` [V: `docs/adr/0007:54-61`]. `global.toml` is a reserved, never-required reference pool [V: `docs/adr/0007:57-61`].
- **Field Types & Options:** 6 types: `input`, `select`, `boolean`, `number` (`min`, `max`, `step`), `date` (`format`), `file` (`folders`, `ext`, `class`) [V: `metadata-schemas/issues/07-schema-service-refactor.md:24`].
- **Template Integration:** `schema` namespace binds `schema.get("class").field("prop")` returning selectable values for `ui.select` (plain strings or `{label, value, ...extra}` objects for `select`; index-derived label/value pairs for `file`; `None` for non-list types) [V: `docs/adr/0006:70-77`, `metadata-schemas/issues/03:11-12`, `08:16-20`].
- **Rule on UI Decoupling:** *"The schema supplies values only; the template author picks the interactive `ui.*` function"* [V: `docs/adr/0006:76-77`]. Schemas do not dictate widget choices.
- **`required` and `multi` Intent:** Declared on `SchemaFieldDef`, but **explicitly declared inert**: *"required/multi are declared now but inert until the deferred MCP guardrail stage; LSP and MCP stages are deferred to later phases"* [V: `docs/adr/0006:110-111`].
- **Note Validation Deferral:** **Explicitly Out of Scope** in `metadata-schemas/spec.md:86-87, 89`: "MCP guardrails enforcing required/multi", "A Schema authoring/validation CLI command", and "Editing Notes through Schema output" were excluded. `issues/02:18` deferred checking that `global` is forbidden as a note class because *"no code in this ticket reads Note frontmatter"* [V].
- **Status:** Resolved and merged (`4801e90` on `main`).

### 2.2 `note-parse`

- **Decided (Resolved):**
  - Narrowed `BytePos` to `u32` with saturating conversion; relocated `LineIndex` to `src/position.rs` [V: `note-parse/issues/01:9-15`].
  - Frontmatter parser swapped to `noyalib` (0.0.51) under `YAML_CONFIG` (`ParserConfig::serde_yaml_compat()`, YAML 1.2) [V: `note-parse/issues/02:27-30`]. `RawFrontmatter::parse` returns `Result<Frontmatter, FrontmatterParseError>` [V: `02:33`].
  - Frontmatter tag unification: `Note.tags()` unifies body tags with the configured frontmatter tag key (`frontmatter.tags()`, default `"tags"`), with no silent fallbacks to `tag`/`keywords` [V: `note-parse/issues/03:17-19`].
  - Heading node: flat `Heading { level: u8, text: String, line: SourceLine }` on `Note` [V: `note-parse/issues/04:11`].
- **Span Decisions & Evolution (Decided vs Superseded vs Withdrawn):**
  - `note-parse/issues/01:23` originally DECIDED spans would be bare `Range<BytePos>`. **Superseded in code:** `src/position.rs:79-138` landed a concrete `ByteSpan { start: BytePos, end: BytePos }` struct instead [V: `note-parse/issues/01:76`].
  - `note-parse/issues/04:24, 32` explicitly **WITHDREW** occurrence spans (`Spanned<T>` on links/tags/inline fields) from `Note`: the proposal of `#[serde(skip)]` with empty spans was condemned as a *"type-system lie"* where loaded notes carry dead spans [V: `04:32`]. Spans were stripped from headings too (`line: SourceLine` only).
  - Frontmatter spans: `noyalib` was configured for its fast path without spans [V: `note-parse/issues/02:50`]; `Frontmatter` stores `IndexMap<FieldKey, NoteFieldValue>` with **zero spans** [V: `src/note/metadata.rs:95-98`].
- **Status:** Resolved.

### 2.3 `template-service`

- **Decided (Resolved):**
  - Render pipeline: `TemplateService::render_to_file` resolves template, reads source, calls `TemplateEngine::render`, then `TemplateWriter::write` [V: `src/template/service.rs:115-121`].
  - `WriteMode`: enum `DryRun` vs `Commit(CommitPolicy)` (`CreateNew` vs `Overwrite`) [V: `src/template/writer.rs:36-42`, `template-service/issues/03:27-29`]. `DryRun` skips target path resolution and file write, returning `WriteOutcome::Previewed(rendered.content)` [V: `src/template/service.rs:213-214`].
  - Dry-run DOES NOT skip prompts: prompt non-interactivity is delegated to `DialogProvider` (non-TTY fallback or `PresetDialogProvider`), not `WriteMode` [V: `src/template/service.rs:34-37`, `docs/adr/0001:39-42`].
  - No-Declaration Template Format: templates declare no upfront parameter manifests or schemas; helpers execute at point of need [V: `src/template/GLOSSARY.md:40-45`].
- **Schema Validation Status:** Explicitly Out of Scope: *"Metadata Menu-style file class schema validation (post-MVP)"* [V: `template-service/spec.md:81`].
- **Future Seam:** `template-service/spec.md:90` noted multi-pass rendering (`eval_to_state` + `render_captured`) could discover required interactive widgets before render, but left it unimplemented [V].
- **Status:** Resolved.

### 2.4 `index-query`

- **Decided (Resolved):**
  - Persistent storage in `redb` (`file_records` and `notes` tables) [V: `index-query/spec.md:104`, `index-query/issues/02:36`].
  - `IndexRecord` combines `FileRecord` + `Note` [V: `index-query/issues/04:21`]. Resolves fields via bare key (`Note::fields()`: frontmatter first, then inline fields) [V: `index-query/issues/05:41`].
  - Source selection via `QuerySource`: `All`, `Tag`, `Folder`, `Class` [V: `index-query/issues/04:20`, `metadata-schemas/issues/05:68`]. Class source expands transitively via `SchemaRegistry::matching_classes` (the `FileClassExpander` seam) [V: `metadata-schemas/issues/05:68`].
  - Inlinks derived in-memory on query from `Note::outlinks()`, never persisted in redb [V: `index-query/issues/10:63`].
- **Status:** Resolved.

### 2.5 `temporal-core`

- **Decided (Resolved):**
  - Parsing & Timezone: Naive datetimes parse as local wall-clock time and store as UTC instants; date-only inputs are civil dates without timezone [V: `temporal-core/divergence-register.md:324`, `spec.md:22, 100`].
  - Formatting Dialect: Core formatting uses `chrono::format::strftime` syntax strictly [V: `divergence-register.md:338`, `spec.md:108`]. `moment.js` tokens (`YYYY`, `MM`, `DD`, etc.) are translated ONLY at the template date filter seam via a scoped translator [V: `divergence-register.md:338`, `temporal-core/review.md:113-116`].
  - Divergence with Schema: Schema `date` field `format` strings (e.g. `"YYYY-MM-DD"`) follow Moment conventions, whereas `[frontmatter]` config defaults use strftime (`"%Y-%m-%dT%H:%M:%S"`) [V: `src/config/raw.rs:49`, `src/schema/fields/date.rs:78`]. `temporal-core` did not unify them.
- **Status:** Tickets 01–04 resolved.

### 2.6 `config-service`

- **Decided (Resolved):**
  - `Config` discovers and merges local (`.traces/config.toml`) and global config [V: `config-service/spec.md:41`].
  - Config digests & fingerprint (`07-config-digest-sections.md`): `Config::digests()` covers `[frontmatter]`, `[schemas]`, `[tasks]`, `[templates]`, `[root]` [V: `config-service/issues/07:16-22`].
  - Critical Invariant: The index fingerprint covers `[schemas].class_field` and `[schemas].directory`, but **does NOT cover `.traces/schemas/*.toml` file contents** [V: `docs/adr/0009-index-refresh.md:76-77`]. Schema file changes never invalidate `redb` [V].
- **Status:** Resolved.

### 2.7 `prompt-service`

- **Decided (Resolved):**
  - Object-safe `DialogProvider` trait separates prompting from template execution (`text`, `confirm`, `select`, `multi_select`) [V: `docs/adr/0001:34-36`, `prompt-service/spec.md:26-30`].
  - Index-based selection: `select` and `multi_select` take display labels and return `usize` / `Vec<usize>` positions; caller recovers the value object by indexing [V: `docs/adr/0003-index-based-dialog-selection.md:60-64`, `prompt-service/issues/03:17, 33-35`].
  - Non-TTY fallback: `TerminalDialogProvider` returns default values / index 0 / empty vec without blocking [V: `docs/adr/0001:39-42`, `prompt-service/issues/03:19, 28`]. `PresetDialogProvider` queues test/MCP inputs [V: `prompt-service/spec.md:32`].
- **Status:** Resolved.

### 2.8 `task-system`

- **Decided (Resolved):**
  - Flattened list model: `Note` stores flat `Box<[ListItem]>` with `depth: u8`, `line: SourceLine`, `parent: Option<SourceLine>`, `is_ordered: bool`, `fields: Option<Box<ListFieldMap>>`, `tags: Box<[Tag]>`, `kind: ListItemType`, `text: ListText` [V: `src/note/lists.rs:44-50`, `task-system/system-design-review.md:383-385`]. Zero byte spans [V].
  - Field Inheritance: List queries inherit parent Note metadata; item-level inline fields override frontmatter and page-level inline fields [V: `task-system/spec.md:123-125, 254`, `task-system/grilling-session.md:610-613`, `task-system/issues/08:40-45`].
- **Status:** Resolved.

### 2.9 `query-expansion`

- **Status: Needs-triage (Open / Unimplemented) [V: `query-expansion/spec.md:1`].**
- **Proposal:** Proposes `file.date` periodic note extraction from filename or frontmatter [V: `spec.md:76`], `TAGS` and `INLINKS` redb multimap tables [V: `spec.md:77`], and `any(field, expr)`/`all(field, expr)` list predicates [V: `spec.md:95`]. No implementation exists in code.

### 2.10 `integration-e2e-suite`

- **Decided (Resolved):**
  - Frozen Facade & Redundancy Pruning: Deleted test-only `schema_field_resolution.rs` integration test, transferring coverage to template `@class` tests and in-crate schema graph units [V: `integration-e2e-suite/issues/13:34-36, 80-97`, `15:94-97`].
  - Config Staleness Finding: Changing `class_field` causes false negatives on stale index axes until explicit `traces index` rebuild [V: `integration-e2e-suite/issues/04:21, 60-64, 82-84`].
  - Dry-Run E2E Pin: Pinned that `template --dry-run` writes nothing to disk and emits rendered content to stdout [V: `integration-e2e-suite/issues/14:14-16`, `integration-e2e-suite/research/C-e2e.md:39`].
- **Status:** Resolved.

---

## 3. `metadata-schemas` in Depth: Origins and Intent

### 3.1 Why Were the Fields and Options Designed?

`metadata-schemas` ported the Obsidian Metadata Menu plugin's fileClass model into Traces TOML files [V: `metadata-schemas/spec.md:13-19`, `docs/adr/0006:31-38`]:

- **`required` & `multi`:** Designed to specify whether a field must exist on notes of that class and whether it accepts multiple values [V: `spec.md:25`]. But because Traces templates query values dynamically rather than generating a GUI form, ADR-0006 explicitly ruled them **inert** [V: `docs/adr/0006:110-111`].
- **`number min`, `max`, `step`:** Direct port of Metadata Menu number settings (which configure numeric sliders/steppers in Obsidian). In `src/schema/fields/number.rs:12-16`, they parse into `Option<f64>`. In `src/schema/fields/number.rs:23, 39`, their accessors are annotated `#[cfg(test)]`. **They have zero production readers** [V: `a-code-inventory.md:89`].
- **`date format`:** Port of Metadata Menu date display formatting. In `src/schema/fields/date.rs:12-14`, it parses into `Option<String>`. In `src/schema/fields/date.rs:21`, the `format()` accessor is annotated `#[cfg(test)]`. **Zero production readers** [V].
- **External Select Values (`issues/08`):** Sourced from Metadata Menu's `ValuesListNotePath`. Created to support large static vocabularies (e.g. 750 countries, 420 industries) in external TOML/JSON files without bloating `global.toml` [V: `metadata-schemas/issues/08:52-65`].
- **`$ref` & `global`:** Solved multiple-inheritance reuse without diamond collisions: `$ref` is acyclic by construction (bounded to `#global/<field>` or `#<ancestor>/<field>`) [V: `docs/adr/0007:54-61`]. `global.toml` provides a shared reference pool whose fields can never be required globally [V: `docs/adr/0007:57-61`].

### 3.2 Intended Consumers vs Deferral of Note Validation

The design intended three consumers:

1. **Templates:** Sourcing option lists for `ui.select` (shipped) [V: `docs/adr/0006:70-77`].
2. **Queries / Tasks:** Transitive `@class*` source selection (shipped) [V: `docs/adr/0006:79-80`].
3. **LSP completions & MCP guardrails:** Future editor and agent guardrails (deferred) [V: `docs/adr/0006:94-95, 110-111`].

**Explicit Statements on Deferral:**

- ADR-0006 line 110–111 explicitly stated: *"required/multi are declared now but inert until the deferred MCP guardrail stage; LSP and MCP stages are deferred to later phases."* [V]
- `metadata-schemas/spec.md:86-87` explicitly listed under Out of Scope: *"MCP guardrails enforcing required/multi"* and *"A Schema authoring/validation CLI command."* [V]
- `research/03:79-80` established that Markdown notes must parse and index regardless of schema conformance; note validation is an on-demand advisory pass, never an index gate [V].

### 3.3 Intended Meaning of `date.format` and `step`

- **`date.format`:** Intended as a display and parse format pattern. In `src/schema/fields/date.rs:78`, unit tests use Moment syntax (`"YYYY-MM-DD"`). In `src/config/raw.rs:49`, `date_created` defaults to strftime (`"%Y-%m-%dT%H:%M:%S"`). In `md-pkm-lsp/issues/20:20`, review confirmed that schema format strings were Moment-style and conflicted with chrono, leading to the decision that validation should check only ISO-8601 parseability, not format compliance [V].
- **`step`:** In `src/schema/fields/number.rs:79` and `fields/parser.rs:870-877`, `step` is validated to be a positive finite float (`step > 0.0`). Its intended meaning is the discrete quantization increment (e.g. integer step `1.0` or half `0.5`). It is never evaluated against notes today [V].

---

## 4. The Template Pre-Check Idea

### 4.1 Prior Design of Validating Template Output

- **ADR-0006 "MCP guardrail stage":** Envisioned that when an AI agent instantiates templates via MCP (where no human is present to answer `ui.*` prompts), schemas would act as guardrails enforcing `required` and `multi` constraints on the agent-provided parameters [V: `docs/adr/0006:110-111`].
- **The "No-Declaration Format" Barrier:** `src/template/GLOSSARY.md:40-45` establishes as a design invariant that templates declare NO upfront parameter manifests or schemas. Helpers are invoked dynamically during evaluation. Therefore, a template's schema requirements **cannot be determined prior to rendering** without evaluating the template [H].
- **Multi-pass Rendering Proposal:** `template-service/spec.md:90` contemplated using MiniJinja's `eval_to_state` + `render_captured` to discover interactive prompts in pass 1, prompt in batch, and render in pass 2. This was deferred and never implemented [V].
- **`RenderFailureKind`:** Defined in `src/template/error.rs:18-42` (`Syntax`, `UndefinedVariable`, `CustomFunction`, `Execution`, `Other`). It classifies minijinja engine failures during evaluation; it has no concept of post-render schema validation [V].

### 4.2 Current Code Inventory in `src/template/`

1. **Render Pipeline (`src/template/service.rs:115-162`):**
   - `render_to_file(name, output, mode)` calls `let rendered = self.render(name)?`, then `self.write(rendered, output, mode)` [V: `service.rs:115-121`].
   - `render()` returns `RenderedTemplate { resolved, content: String, declared: Option<DeclaredOutputPath> }` [V: `service.rs:157-161`].
2. **Where Frontmatter of Rendered Output Lives:**
   - Frontmatter is NOT parsed, structured, or exposed in `RenderedTemplate`. It exists purely as raw text inside `rendered.content: String` [V: `service.rs:159`].
3. **What `DryRun` Does (`src/template/service.rs:213-215`, `src/template/writer.rs:36-42`):**
   - `WriteMode::DryRun` causes `write()` to early-return `Ok(WriteOutcome::Previewed(rendered.content))` immediately [V: `service.rs:213-214`].
   - It bypasses output path resolution, path confinement checks, overwrite guards, and all disk I/O [V: `service.rs:188-196`].
   - `DryRun` does NOT suppress `ui.*` interactive prompts (handled by `DialogProvider`) [V: `service.rs:34-37`].
4. **Is Rendered Text Parsed Back as a Note Anywhere?**
   - **NO.** Zero calls to `parse_markdown`, `Note`, `RawFrontmatter`, or `yaml::parse` exist in `src/template/` [V]. The string returned by Minijinja is written directly to disk via `fs::File::create_new` / `write_all` [V: `src/template/writer.rs:344-360`].
5. **Where a Template Schema Check Could Slot In:**
   - In `TemplateService::render_to_file` (or `write`), immediately after `let rendered = self.render(name)?` [V: `service.rs:121`].
   - At this point, `rendered.content` is complete. A validator can parse the frontmatter (via `RawFrontmatter::new` / `yaml::parse`), read `class`, resolve the schema via `SchemaService`, and evaluate constraints [H].
   - **Behavior in `DryRun`:** The check can execute during dry-run preview without touching disk, returning schema diagnostics alongside `WriteOutcome::Previewed` [H].
   - **Behavior in `Commit`:** Can either gate the write (`TemplateError::SchemaValidation`) or log advisory stderr diagnostics before proceeding to write [H].

---

## 5. Note Parsing & Frontmatter Span Reality Check

Verifying prior designs against current tree (`src/position.rs`, `src/note/metadata.rs`, `src/note/parser.rs`, `src/note/lists.rs`, `src/note/heading.rs`, `src/yaml.rs`):

| Component / Concept | Prior Design / Specified Shape | Implemented in Tree Today? | Exact Code Status & Location |
|---|---|---|---|
| `BytePos` | `u32`-backed source offset | **YES [V]** | `src/position.rs:45-73`: `pub(crate) struct BytePos(u32);` with `saturating_from(usize)`. |
| `ByteSpan` | `Range<BytePos>` (ticket 01) vs struct (position refactor) | **YES (as struct) [V]** | `src/position.rs:79-138`: `pub(crate) struct ByteSpan { start: BytePos, end: BytePos }`. |
| `Spanned<T>` | Value paired with `ByteSpan` | **YES (partially used) [V]** | `src/position.rs:358-470`: `pub(crate) struct Spanned<T> { value: T, span: ByteSpan }`. Used in query lexer, task item scanner, and date/duration prefix parsing; **NOT used in `Note` AST**. |
| `LineIndex` | Precomputed line starts for offset→line conversion | **YES [V]** | `src/position.rs:188-290`: `LineIndex`. Used in `src/note/parser.rs:337, 360` to populate `SourceLine` on list items and headings. |
| **Frontmatter Spans** | `#[serde(skip)] spans: IndexMap<FieldKey, ByteSpan>` (`md-pkm-lsp#20`) | **NO [V]** | `src/note/metadata.rs:95-98`: `Frontmatter { fields: IndexMap<FieldKey, NoteFieldValue> }`. **ZERO spans.** |
| **Link Spans** | `Spanned<Link>` (`md-pkm-lsp#11`) | **NO [V]** | `src/note/links.rs:27-32`: `Link { target: String, text: String, kind: LinkType }`. **ZERO spans.** |
| **Tag Spans** | Occurrence spans for tags | **NO [V]** | `src/tag.rs:25-28`: `Tag { full: String, segments: Box<[String]> }`. **ZERO spans.** |
| **Inline Field Spans** | `Spanned<InlineField>` | **NO [V]** | `src/note/parser.rs:332`: `inline_fields: IndexMap<FieldKey, Vec<NoteFieldValue>>`. **ZERO spans.** |
| **Heading Spans** | Byte span on `Heading` | **NO [V]** | `src/note/heading.rs:18-22`: `Heading { level: u8, text: String, line: SourceLine }`. **Line only, ZERO byte spans.** |
| **List Item Spans** | Byte span on `ListItem` | **NO [V]** | `src/note/lists.rs:44-50`: `ListItem { depth: u8, line: SourceLine, parent: Option<SourceLine>, ... }`. **Line only, ZERO byte spans.** |

**Summary:** `src/note/` parser retains **ZERO byte spans** on any element of a note. Spans in `src/position.rs` serve only transient query and task token parsing. Every proposal to attach byte spans to `Note` or `Frontmatter` was either explicitly withdrawn (`note-parse#04:24, 32`) or never implemented (`md-pkm-lsp#20`).

---

## 6. Cross-Effort Contradictions & Architectural Tensions

1. **Date Format Dialect Clash (`metadata-schemas` vs `temporal-core` vs `config-service`):**
   - `metadata-schemas` fixture tests use Moment-style `"YYYY-MM-DD"` [V: `src/schema/fields/date.rs:78`].
   - `config-service` defaults `[frontmatter]` date formats to strftime `"%Y-%m-%dT%H:%M:%S"` [V: `src/config/raw.rs:49`].
   - `temporal-core` decreed that core date formatting strictly uses `chrono::format::strftime`, limiting Moment tokens strictly to template filters [V: `temporal-core/divergence-register.md:338`].
   - `md-pkm-lsp#20:20` flagged that schema format strings contradict chrono and ruled that schema date validation must check only ISO-8601 parseability, not format compliance [V].
2. **Span Shape Drift (`note-parse#01` vs `src/position.rs`):**
   - `note-parse/issues/01:23` formally decided: spans MUST be bare `Range<BytePos>`, never a wrapper or struct.
   - `src/position.rs:79-138` landed a concrete `ByteSpan` struct with `new(start, end)`, directly contradicting `01:23` [V: `note-parse/issues/01:76`].
3. **`#[serde(skip)]` Spans Lie (`md-pkm-lsp#20` vs `note-parse#04`):**
   - `md-pkm-lsp/issues/20:45, 61-70` proposed adding `#[serde(skip)] spans: IndexMap<FieldKey, ByteSpan>` to `Frontmatter`.
   - `note-parse/issues/04:24, 32` condemned this exact pattern as a *"type-system lie"* where loaded notes carry dummy spans, withdrawing it from `Heading` and occurrence nodes [V].
4. **Index Invalidation Trap (`metadata-schemas` vs `config-service` / `index-query`):**
   - ADR-0009 and `config-service/issues/07:18` deliberately exclude schema TOML file contents from the index fingerprint [V: `docs/adr/0009:76-77`].
   - If Ticket 16 introduces index-time schema typing, changing any `.traces/schemas/*.toml` file will NOT invalidate `redb` unless the fingerprint architecture is altered [H].

---

## 7. Not Verified

1. **Whether wild user vaults use external select files:** Cannot verify if users rely on external `.json`/`.toml` select files beyond `examples/` fixtures [H].
2. **Performance of `noyalib` with span tracking enabled:** Have not benchmarked whether configuring `noyalib` for span tracking would regress frontmatter parsing throughput [H].
3. **Exact MCP server wire protocol requirements:** The "deferred MCP guardrail stage" in ADR-0006 has no formal protocol specification in the repo [H].

---

## 8. What This Changes for Ticket 16

1. **Keep Schema Passive from Note Ingestion (Hard Barrier):** `metadata-schemas`, `index-query`, and ADR-0009 firmly establish that schemas never type notes at index time. Schema changes do not invalidate the index fingerprint. Note ingestion remains schema-free and open-world tolerant [V].
2. **Template Pre-Check Must Be Post-Render:** Because of the No-Declaration Template Format, a schema pre-check cannot inspect a template before evaluation. It can only inspect the frontmatter of `rendered.content` after Minijinja runs, right before `write()` [V].
3. **Validation Cannot Expect AST Byte Spans Today:** Because `src/note/` stores zero byte spans, any validator operating on `&Note` can only emit line-level or field-key diagnostics. Byte-accurate diagnostics require either a separate editor-facing AST (splitting `Note`) or an on-demand CST pass (Taplo/jsonschema pattern) [H].
4. **`date.format`, `step`, and `min`/`max` are Dead Code or Inert:** Any option for Ticket 16 that promises to validate `number.step` or `date.format` must either implement new parsing/validation infrastructure or formally retire those fields from `src/schema/` [V].
5. **Validation Must Be an On-Demand Advisory Pass:** Consistently with `research/03`, `md-pkm-lsp#20`, and `metadata-schemas`, note validation must be an advisory analysis pass (`validate(&Note, &Schema) -> Vec<SchemaDiagnostic>`), reusable by the CLI (`traces check`), future LSP, and the post-render template writer [P].
