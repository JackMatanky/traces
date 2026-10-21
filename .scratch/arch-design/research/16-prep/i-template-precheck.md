# Template Pre-Check: Schema Validation of Generated Notes

- **Topic:** Code-grounded and precedent-grounded analysis of using schemas as a template pre-check (validating notes produced by templates before writing to disk).
- **Status:** Complete research deliverable for Wayfinder Ticket 16 (Schema Note Typing and Validation).
- **Deliverable:** `.scratch/arch-design/research/16-prep/i-template-precheck.md`.
- **Sources audited:**
  - Codebase: `src/template/` (`service.rs`, `writer.rs`, `engine.rs`, `loader.rs`, `path.rs`, `GLOSSARY.md`, `engine/ui.rs`, `engine/schema.rs`, `engine/query.rs`), `src/cli/template.rs`, `src/dialog/` (`mod.rs`, `terminal.rs`, `preset.rs`), `src/schema/` (`service.rs`, `fields.rs`, `raw.rs`, `types.rs`).
  - ADRs: ADR-0001 (Minijinja with Lazy Interactive Custom Functions), ADR-0002 (Symlink Trust), ADR-0003 (Index-based Selection), ADR-0004 (Centralize CLI Presentation), ADR-0006 (Schemas as File Class Definitions), ADR-0007 (Field Inheritance).
  - Wayfinder designs: `.scratch/md-pkm-lsp/issues/20-schema-and-fileclass-intelligence.md`, `.scratch/md-pkm-lsp/issues/22-template-language-intelligence.md`, `.scratch/template-service/issues/01..04`.
  - Prep research: `.scratch/arch-design/research/16-prep/` (`a-code-inventory.md`, `b-metadatamenu-lessons.md`, `c-validation-precedent.md`, `d-prior-decisions.md`, `brief.md`), `research/03-coercion-validation-precedent.md`.
  - External precedent: Copier/Cookiecutter hooks, Hugo archetypes, Astro Content Collections, Obsidian Metadata Menu (`insertMissingFields`, `postValues`, `validateValue`), Obsidian Templater.

---

## 1. Current Template Pipeline & Write/Execution Model

### 1.1 Step-by-Step Execution Trace

Instantiating a template via `traces template` follows an 8-stage pipeline from CLI dispatch to file persistence [V]:

1. **CLI Argument Parsing (`src/cli/template.rs:34-71`) [V]:**
   - Flags parsed: `name` / `-i` (`Option<PathBuf>`), `--list` (`bool`), `-o`/`--output` (`Option<PathBuf>`), `-f`/`--force` (`bool`), `-n`/`--dry-run` (`bool`), and `--no-input` (`bool`).
   - `--dry-run` conflicts with `-o`/`--output` (`src/cli/template.rs:62`) [V].
2. **Provider Selection (`src/cli/template.rs:168-177`) [V]:**
   - If `--no-input` is passed, `resolve_provider` wraps an empty `PresetDialogProvider::new()` [V].
   - Otherwise, it forwards the injected `TerminalDialogProvider` (which probes `stdin_is_tty()` on prompt calls) [V].
3. **Template Name Resolution (`src/cli/template.rs:179-210`) [V]:**
   - Explicit name is parsed into `TemplatePathInput` (`src/cli/template.rs:204`) [V].
   - If absent, `pick_template` calls `TemplateService::list_available(config)` and prompts via `provider.select("Select a template", &available)` (`src/cli/template.rs:240-254`) [V].
4. **Service & Engine Construction (`src/cli/template.rs:139-144`, `src/template/engine.rs:109-168`) [V]:**
   - `TemplateService::new` constructs `TemplateLoader` (scans `.traces/templates` then global template dirs) and `TemplateEngine` [V].
   - `TemplateEngine::new` configures a minijinja `Environment` with `debug = true` (`src/template/engine.rs:120`) [V].
   - Schemas are loaded eagerly once at construction via `SchemaService::load_verbose(schema_directory)` (`src/template/engine.rs:131-134`) [V].
   - Globals and namespaces are registered: `file`, `query`, `lists`, `tasks`, terminal filters, `path`, `ui` (`UiOps`), `date`, `string`, `num`, `yaml`, `schema` (`SchemaOps`), and function `uuid` (`src/template/engine.rs:137-164`) [V].
5. **WriteMode Resolution (`src/cli/template.rs:145`, `src/template/writer.rs:51-57`) [V]:**
   - Evaluated as `WriteMode::from_flags(self.dry_run, self.force)`.
   - Returns `WriteMode::DryRun` if `dry_run == true` (dry-run unconditionally trumps `--force`), else `WriteMode::Commit(CommitPolicy::from_flag(force))` (`CreateNew` or `Overwrite`) [V].
6. **Rendering Stage (`src/template/service.rs:148-162`, `src/template/engine.rs:185-204`) [V]:**
   - `service.render(name)` resolves template source on disk via `loader.find(name)` and `read_template` [V].
   - `engine.render(source, name)` runs minijinja:
     `env.template_from_named_str(name, source)?.render_captured(minijinja::context!())` [V].
   - **Interactive Prompting Mid-Render:** When evaluation hits `{{ ui.text_input(...) }}`, `{{ ui.select(...) }}`, etc., `UiOps` delegates synchronously to `Arc<dyn DialogProvider>` (`src/template/engine/ui.rs:77-133`) [V]. Under `--no-input` or non-TTY, defaults or fallback items return immediately [V].
   - If `file.write_to("path")` was called in the template, it writes into minijinja's temporary state (`WRITE_TO_KEY`) (`src/template/engine/file.rs:36`) [V].
   - Engine returns `RenderOutput { content: String, write_to: Option<DeclaredOutputPath> }` (`src/template/engine.rs:209-214`) [V].
   - Service wraps this in `RenderedTemplate { resolved, content, declared }` (`src/template/service.rs:157-161`) [V].
7. **Write / Commit Stage (`src/template/service.rs:207-226`, `src/template/writer.rs:200-210`) [V]:**
   - `service.write(rendered, output, mode)` branches on `mode` [V]:
     - **`WriteMode::DryRun`:** Returns `Ok(WriteOutcome::Previewed(rendered.content))` immediately without resolving output paths, checking file existence, or touching disk (`src/template/service.rs:213-215`) [V].
     - **`WriteMode::Commit(policy)`:** Builds `TemplateWriteTarget` and executes `target.write(&rendered.content, policy, provider, default)` [V].
       - Output path precedence: explicit CLI `-o` > declared `file.write_to()` > default `Config::output_dir` (`src/template/writer.rs:8-16, 223-233`) [V].
       - Confinement: Non-default paths are confined to `Config::root` via `SafeRelativePath::parse` (`src/template/writer.rs:303-320`) [V].
       - Collision Handling: Under `CommitPolicy::CreateNew`, if `-o` was not given, provider is interactive, and destination exists, prompts interactively in a loop for an alternative relative path (`src/template/writer.rs:259-290`) [V].
       - Physical Write: `commit(&path, content, policy)` creates parent directories (`fs::create_dir_all`) and writes bytes directly to disk via `policy.create_file(path)` and `file.write_all(content.as_bytes())` (`src/template/writer.rs:350-366`) [V].
       - Returns `Ok(WriteOutcome::Written(resolved_path))` (`src/template/service.rs:225`) [V].
8. **CLI Presentation (`src/cli/template.rs:218-225`) [V]:**
   - `WriteOutcome::Written(path)`: prints `wrote <path>` to `stderr` (`src/cli/template.rs:221`) [V].
   - `WriteOutcome::Previewed(content)`: prints raw rendered text to `stdout` (`src/cli/template.rs:223`) [V].

### 1.2 Pipeline Invariants: Parsing, Class Knowledge, and ADR Intent

- **Where is the final rendered text available?**
  In memory as `RenderedTemplate.content: String` (`src/template/service.rs:159`) [V], inside `WriteOutcome::Previewed(String)` (`src/template/writer.rs:130`) [V], and borrowed as `&str` into `commit(&path, content, policy)` (`src/template/writer.rs:352`) [V].
- **Is it ever parsed back as a `Note`?**
  **NO** [V]. `src/template/` contains zero imports of `Note` or `frontmatter::*` parser [V]. The rendered document is treated strictly as an opaque UTF-8 byte sequence (`file.write_all(content.as_bytes())`) [V].
- **Does anything know the rendered output's `class`?**
  **NO** [V]. Neither `TemplateEngine`, `TemplateService`, `TemplateWriter`, nor `cli::template::Template` knows whether the rendered text contains YAML frontmatter, let alone a `class` key [V]. Even when templates call `schema.get("book")` to populate select lists, that call is an isolated value lookup; the engine never binds the template to that class [V].
- **What does `DryRun` do?**
  Skips output resolution, collision prompting, and disk writes, returning the rendered `content` as `WriteOutcome::Previewed` for stdout printing (`src/template/writer.rs:36-39`, `src/template/service.rs:213-215`, `src/cli/template.rs:223`) [V]. Crucially, dry-run executes all template logic and prompts normally [V].
- **What does the 'No-Declaration Format' mean in ADR-0001 & ADR-0006?**
  `src/template/GLOSSARY.md:40-44` defines *No-Declaration Format* as: *"The design invariant that templates declare no upfront parameter manifests, invoking interactive helpers at point of need during evaluation. Avoid: declared template, template schema, manifest"* [V].
  - In ADR-0001: *"dry-run controls WriteMode (disk writes only)... Architecture naturally extends to MCP mode: AI provides all variables upfront"* [V].
  - In ADR-0006: *"The existing glossary deliberately has a No-Declaration Template Format, so the schema is vault-level metadata about notes, not a template declaration... required/multi are declared now but inert until the deferred MCP guardrail stage; LSP and MCP stages are deferred to later phases"* [V].

---

## 2. Three Senses of 'Template Pre-Check'

We can conceptualize validating a template against a schema at three distinct lifecycle points:

```text
[Template Source] --(S3: Static AST Analysis)--> [minijinja AST]
       |
       v (Evaluation begins)
[Interactive Prompts] --(S2: Pre-Prompt Constraints)--> [User / MCP Input]
       |
       v (Rendering completes)
[Rendered String] --(S1: Post-Render Note Check)--> [Note Parse & SchemaKernel] --> [Disk Write]
```

### 2.1 (S1) Post-Render Check (Realized Note Validation)

- **Mechanism:** Immediately after `self.render(name)?` produces `RenderedTemplate { content, .. }` (`src/template/service.rs:157`) but before `self.write(...)`:
  1. Parse `rendered.content` into frontmatter / `Note` via `noyalib` or the note parser [P].
  2. Read the class attribute using `config.class_field()` (defaulting to `class`) [P].
  3. If a class is present, fetch the schema via `SchemaService::get(&class)` [P].
  4. Run the pure schema validation kernel: `SchemaValidator::validate(&note, &schema)` [P].
  5. Report diagnostics (advisory to stderr, or gate before calling `self.write`) [P].
- **Feasibility:** **High (near-trivial)** [H]. The rendered string is fully resolved, all conditionals evaluated, and all user prompts completed. It leverages the clean existing seam between `service.render()` and `service.write()` (`src/template/service.rs:121`) [V].
- **Cost:** Microseconds ($< 50\,\mu\text{s}$) [H]. Single frontmatter parse + in-memory schema constraint check.
- **Ownership:** `template::service` calling a shared validation kernel (`schema::validate` or `crate::check`) [P].
- **Gating Policy:**
  - *Interactive CLI:* Gating here is hostile: the user just spent 30 seconds answering interactive prompts; aborting the write and discarding rendered text on an advisory mismatch destroys user effort [H]. Policy should be **Advisory** (print warnings to stderr, or prompt "Save anyway?") [P].
  - *MCP / Automated Mode:* Policy should be **Gating** (ADR-0006 guardrail) or return structured diagnostic errors so the agent can correct parameters and re-invoke [P].
- **Interaction with Dialog / DryRun:**
  - `--dry-run`: Runs identically! Validates the preview string, prints diagnostics to stderr, and prints preview to stdout [P].
  - `--no-input`: Validates whether the default values uphold the schema [P].
- **Kernel Interface:** Synchronous, pure function: `fn validate_note(note: &Note, schema: &Schema) -> Vec<SchemaDiagnostic>` [P]. Does **not** require spans (`spans: None`) [P].

### 2.2 (S2) Pre-Prompt Constraint (Schema-Driven Prompting)

- **Mechanism:** Constrain interactive prompts during render so that invalid values cannot be entered [P].
- **Existing `ui.*` methods (`src/template/engine/ui.rs:34`) [V]:**
  - `ui.text_input(label, default)`: freeform text via `provider.text(label, default)`.
  - `ui.select(label, items, kwargs)`: single choice via `provider.select(label, &opts.labels)`.
  - `ui.multi_select(label, items, kwargs)`: multi choice via `provider.multi_select(label, &opts.labels)`.
  - `ui.confirm(label)`: boolean via `provider.confirm(label, default)`.
- **What already works:** Select fields. When templates write `schema.get("book").field("status") | ui.select("Status")`, `Schema::field` returns the valid select options directly to `ui.select` (`src/template/engine/schema.rs:180-204`) [V].
- **What a full schema-driven prompt system would need:**
  - `Number`: min, max, step bounds. `DialogProvider` has no numeric prompt [V]; inquire supports `CustomType<f64>`, but `DialogProvider` only provides `text` (`src/dialog/mod.rs:165`) [V].
  - `Date`: date picker or regex format validation (`dateFormat`). `DialogProvider` has no date method [V].
  - `File`: filtered note picker. Currently requires manual composition: `query.from(schema.get("c").field("f")) | ui.select(...)` (`ADR-0006`, `src/template/engine/schema.rs:186-190`) [V].
- **Feasibility:** **Low for automatic inference; Medium for explicit template helpers** [H]. Because Traces mandates the *No-Declaration Format* (`src/template/GLOSSARY.md:40`), templates do not declare upfront what class they represent. The engine cannot know that `ui.text_input("Rating")` corresponds to `rating: Number(min=1, max=5)` in `Book.toml` unless the author writes `ui.schema_input("book", "rating")` or passes schema metadata into `ui.*` [H]. Furthermore, adding typed prompt methods to `DialogProvider` bloats an object-safe trait intentionally kept to five minimal primitives (ADR-0001, ADR-0003) [V].
- **Cost:** High development and API churn; breaks simplicity of `DialogProvider` [H].
- **Ownership:** `dialog` trait + `template::engine::ui` [P].
- **Gating Policy:** Gating at point of entry (terminal prompt refuses invalid input until corrected) [V].
- **Interaction with Dialog / DryRun:** Under `--no-input`, defaults must satisfy constraints or render errors out [H].

### 2.3 (S3) Static Template Analysis (Compile-Time AST Check)

- **Mechanism:** Parse the template source into a minijinja AST before execution, inspect what frontmatter fields and class it will write, and verify them against the schema [P].
- **What the LSP Effort Decided (Ticket 22):**
  - Ticket 22 grounded: *"templates cannot be safely executed for analysis purposes... The `ui.*` namespace's 'No-Declaration Format' means templates call interactive prompts synchronously mid-render, blocking on a live DialogProvider... There is no existing static-analysis capability"* (`.scratch/md-pkm-lsp/issues/22-template-language-intelligence.md:9`) [V].
  - Ticket 22 adopted minijinja's `machinery::parse` behind `unstable_machinery` (`src/template/analysis.rs`) to inspect `Stmt::Template` for syntax errors, undeclared variables, helper namespaces, and include/extends targets [V].
- **Feasibility for Schema Validation:** **Zero / Fatally Flawed** [H].
  1. *Frontmatter is not a template language construct:* In minijinja, frontmatter is raw template body text. To minijinja's AST, `--- \n title: {{ ui.text_input("Title") }} \n ---` is just an alternating sequence of `Stmt::EmitRaw` and `Stmt::EmitExpr` [H].
  2. *Evaluating frontmatter requires execution:* Frontmatter fields may be conditionally rendered (`{% if condition %}tag: value{% endif %}`) or generated in loops (`{% for k, v in items %}`) [H].
  3. *Unrendered templates are invalid YAML:* A YAML parser cannot parse raw template source containing unescaped Jinja tags like `date: {{ date.now() }}` [H].
  4. *Unknown class target:* Static analysis cannot know what `class` a note will have unless hardcoded in raw text [H].
- **Cost:** Extremely high complexity for negligible, fragile static heuristic coverage [H].
- **Ownership:** `template::analysis` (LSP only) [V].
- **Conclusion:** Static analysis works for syntax and variable references (Ticket 22), but **cannot** perform schema conformance pre-checks [H].

### 2.4 Comparison Matrix Across the Three Senses

| Dimension | (S1) Post-Render Check | (S2) Pre-Prompt Constraint | (S3) Static AST Analysis |
| :--- | :--- | :--- | :--- |
| **Lifecycle Phase** | After render, before write | During interactive render | Before render (parse time) |
| **Feasibility** | **High** (natural pipeline seam) | **Medium** (requires explicit helper calls) | **Infeasible** (frontmatter unparsed in AST) |
| **Input Data** | Rendered `String` -> parsed `Note` | Prompt arguments + `FieldDef` | Minijinja `machinery::ast::Stmt` |
| **Class Detection** | Exact (reads frontmatter key) | Manual (must be passed in template) | Unknown / heuristic only |
| **Error Mode** | Advisory (CLI) / Gating (MCP) | Interactive prompt retry / validation | Advisory syntax diagnostic |
| **Needs Spans?** | No (`None` or byte offset) | No (terminal cursor) | Yes (AST node byte spans) |
| **Dialog Impact** | Zero (prompts already finished) | Requires `DialogProvider` changes | Zero (prompts never executed) |
| **Dry-Run Support** | Full (checks stdout preview) | Full (checks prompt defaults) | Full (no execution) |

---

## 3. Relationship to LSP Validator & Error-Mode Policy

### 3.1 Kernel Sharing: Reusing `SchemaValidator`

The post-render check (S1) and the LSP validator should use the **identical validation kernel** [P].

- Ticket 20 (`.scratch/md-pkm-lsp/issues/20-schema-and-fileclass-intelligence.md:57`) specified:

  ```rust
  pub struct SchemaValidator;
  impl SchemaValidator {
      pub fn validate(note: &Note, schema: &Schema) -> Vec<SchemaDiagnostic>;
  }
  ```

- Because a post-rendered template output is parsed into a `Note`, `SchemaValidator::validate` can be called directly without any template-specific validation code [P].

### 3.2 Does It Need Spans?

**NO** [V].

- In the LSP (Ticket 20 & 25), diagnostics require `range: Range<BytePos>` to underline erroneous text in an active editor buffer [V].
- In CLI template execution, the output is not displayed in an editor; error output is printed to stderr [V].
- Ticket 20 explicitly designed `range` as optional (`.scratch/md-pkm-lsp/issues/20-schema-and-fileclass-intelligence.md:105`):
  `pub range: Option<Range<BytePos>>` [V].
- When validating a freshly rendered note in CLI/MCP, `range` is simply `None` (or line 1). The diagnostic identifies the offending field by `field: FieldName` and `message: String` [P].

### 3.3 Does It Need `file`-Field Index Membership?

**NO** [V].

- Ticket 20 Q15 explicitly settled this: *"File fields: type + multi only (no file-existence check). Pure function model preserved — no filesystem access needed. Type checking (is value a Link?) is cheap and pure"* [V].
- Even though `TemplateEngine` already instantiates `IndexerService::from(config)` (`src/template/engine.rs:135`) for `query.*` expressions, keeping the validation kernel pure (taking only `&Note` and `&Schema`) ensures it remains ultra-fast, deterministic, and decoupled from index state [P].

### 3.4 Error Mode: Ticket 15 Advisory vs ADR-0006 MCP Gating

There is an apparent tension between two foundational decisions:

1. **Ticket 15's Error-Mode Principle:** *"Coercion failures follow the error-mode principle: advisory, never gating"* (`.scratch/arch-design/issues/15-coercion-and-comparison-policy.md:45-46`) [V].
2. **ADR-0006's MCP Guardrail Stage:** *"required/multi are declared now but inert until the deferred MCP guardrail stage; LSP and MCP stages are deferred to later phases... one field definition is shared across templates, queries, and future LSP completions/MCP guardrails"* (`ADR-0006: Consequences`) [V].

#### Reconciliation Policy

The divergence resolves cleanly across consumer modalities [P]:

- **Interactive Human CLI (`traces template`):** Follows **Ticket 15 (Advisory)**.
  - Aborting a write after prompts complete discards human effort [H].
  - Behavior: `commit()` proceeds to write the file; any validation diagnostics are printed to `stderr` as warnings (e.g. `warning: field 'status' is not a valid option for schema 'book'`) [P]. An optional `--strict` flag can turn warnings into an abort [P].
- **Non-Interactive / MCP / Agent Mode (`traces template --no-input` or MCP tool call):** Follows **ADR-0006 (Gating)**.
  - An AI agent generating notes from templates must not silently pollute the vault with schema-violating notes [H].
  - Behavior: The tool call refuses to commit the write, returning a structured JSON error detailing the missing required fields or type mismatches, enabling the model to self-correct [P].

---

## 4. Precedent in Comparable Tools

How comparable tools validate generated documents against schemas before writing:

### 4.1 Copier & Cookiecutter

- **Copier (`copier.yml`):**
  - *Prompt-level validation (S2):* Supports question-level validators (`validator: "{% if ... %}message{% endif %}"`) [`copier.readthedocs.io/en/stable/configuring`]. Invalid answers are rejected interactively at the prompt; execution cannot proceed until input is valid [V].
  - *Post-generation tasks:* Copier provides `_tasks` (shell scripts executed after copying). If a task exits non-zero, Copier aborts and rolls back changes [V].
- **Cookiecutter (`cookiecutter.json`):**
  - *Pre-generation hooks (`hooks/pre_gen_project.py`):* Executes *after* prompt collection but *before* filesystem writes. Inspects cookiecutter context variables. If the script exits with non-zero code, generation terminates immediately and writes nothing (Gating) [V].
  - *Post-generation hooks (`hooks/post_gen_project.py`):* Runs after files are written to run linters or cleanup [V].

### 4.2 Hugo Archetypes & `hugo check`

- **Archetype generation (`hugo new <path>`):** Hugo archetypes are Go templates evaluated to create new content notes [`gohugo.io/content-management/archetypes/`].
- **Validation behavior:** Hugo performs **zero schema validation** during `hugo new` [V]. The rendered frontmatter is written unconditionally, regardless of whether required fields are present or types match [V].
- **Build-time check:** Hugo validates frontmatter during site build (`hugo build`), where missing archetype fields cause build failures or render empty strings [V]. `hugo check` was an early command (now deprecated) that only checked syntax, not schema conformance [V].

### 4.3 Astro Content Collections

- **Schema definition (`src/content.config.ts`):** Uses Zod schemas via `defineCollection({ schema: z.object({...}) })` [`docs.astro.build/en/guides/content-collections/`].
- **Validation behavior:** Astro does not provide a built-in note generation CLI (`astro new`). Validation occurs at **build / dev-sync time** (`astro sync` / `astro build`) [V].
- **Gating:** Validation is a **strict hard gate** [V]. If any markdown file has invalid frontmatter types or missing required fields, `astro build` fails with an explicit Zod error listing line, file, and constraint [V].

### 4.4 Obsidian Metadata Menu

- **Audit findings (`b-metadatamenu-lessons.md` §2.1-2.4) [V]:**
  - *Zero write gating:* The programmatic write API `postValues()` does **zero type checking, zero schema checks, and zero enum checks** before writing to disk (`src/commands/postValues.ts:3375-3404`) [V].
  - *Missing fields helper (`insertMissingFields`):* `insertMissingFields.ts` scans a note against its `fileClass` and appends missing fields with blank/default values [V]. It is an **assistive editing action**, never a gate [V].
  - *Interactive Modal Gating:* Validation exists exclusively within the interactive UI modal (`valueModal.save()`), where invalid numbers or unparseable dates block the "Save" button in the GUI (`Number.ts:10532`) [V].
  - *Analysis API (`fileFields`):* Reports `isValid: boolean`, but empty fields are unconditionally considered valid (`!fieldVM.value || validateValue(...)`) (`src/commands/fileFields.ts:2580`) [V].

### 4.5 Summary of Precedents

| Tool | Scaffolding / Template Pre-Check | Error Policy | Primary Validation Timing |
| :--- | :--- | :--- | :--- |
| **Copier** | Prompt validator (`validator`) + post-task | Gating | Interactive prompt + Post-gen hook |
| **Cookiecutter** | `pre_gen_project.py` hook | Gating | Pre-write context hook |
| **Hugo** | None | No validation | Build time (`hugo build`) |
| **Astro** | None (no scaffolding tool) | Gating | Build / sync time (`astro check`) |
| **Metadata Menu** | None on write; `insertMissingFields` appends blanks | Advisory / Assistive | Interactive UI modal only |

**Key Takeaway:** Generator tools either validate *at the prompt* (Copier S2) or validate context *before file emission* (Cookiecutter pre-gen hook). Tools that deal with open markdown vaults (Hugo, Obsidian Metadata Menu) never block template emission; they defer validation to advisory analysis or build-time checks.

---

## 5. Architectural Placement & Dependency Options

Where should the template pre-check live in Traces?

### 5.1 Option A: `template` Depends on `schema::validate` (Direct Dependency)

```text
[cli] --> [template] --> [schema::validate]
```

- **Mechanism:** `TemplateService::write` (or a helper inside `service.rs`) parses the rendered note, calls `schema::validate::validate(&note, &schema)`, and handles advisory/gating outcomes [P].
- **Dependency Direction:** `src/template/` already depends on `src/schema/` (`src/template/engine.rs:60`, `src/template/engine/schema.rs:69-72`) [V]. Calling `schema::validate` adds no new dependency edge [H].
- **Workspace Split Steer (Ticket 34):** In a multi-crate workspace split (`traces-schema`, `traces-template`), `traces-template` naturally depends on `traces-schema`. This direction is clean and acyclic [H].

### 5.2 Option B: Trait Injection (`dyn NoteValidator` into `TemplateService`)

```text
[cli] --wires--> [TemplateService(Arc<dyn NoteValidator>)]
                      |
                      v
             [SchemaValidator impl]
```

- **Mechanism:** `TemplateService` defines an object-safe trait:

  ```rust
  pub trait NoteValidator: Send + Sync {
      fn validate(&self, content: &str) -> Vec<TemplateValidationDiagnostic>;
  }
  ```

  `cli` constructs `SchemaValidator` and injects it into `TemplateService::new` [P].
- **Evaluation:** Decouples `template` from `schema`, but adds unnecessary indirection and boilerplate for an engine that already imports `SchemaService` [H]. Violates the Ponytail principle ("no abstraction or wrapper nobody asked for") [H].

### 5.3 Option C: Orchestrator / Pipeline Composition (Seam-Based Check)

```text
[cli / orchestrator]
  |-- 1. template_service.render(name) -> RenderedTemplate
  |-- 2. validator.validate(&rendered.content) -> Vec<Diagnostic>
  \-- 3. template_service.write(rendered, output, mode) -> WriteOutcome
```

- **Mechanism:** The caller orchestrates rendering, checking, and writing separately [P].
- **Critical Code Fact:** `TemplateService` **already** exposes this exact seam!
  `src/template/service.rs:126-130`:
  *"Performs the resolution and rendering steps of [`Self::render_to_file`], allowing callers to inspect rendered content before invoking [`Self::write`]. Separating render from write prevents re-executing interactive `ui.*` prompt calls if the rendered result is evaluated multiple times"* [V].
- **Evaluation:** Cleanest architecture [P]. `TemplateService` remains focused on rendering and writing. The CLI or MCP orchestrator calls `render()`, runs validation diagnostics via `SchemaValidator`, and decides whether to proceed with `write()`, prompt the user, or abort [P].

### 5.4 What Can Be Simplified or Deleted in `template/engine/schema.rs`?

If schemas drive a unified validation kernel and value model (Tickets 14 & 16):

- `template/engine/schema.rs` currently implements custom `select_entry_value` conversion (`src/template/engine/schema.rs:258-266`) and `FieldValue` conversions (`src/template/engine/schema.rs:67`) [V].
- With Ticket 14's unified value model (`NoteFieldValue`), schema select entries and note values share one representation [V].
- `template/engine/schema.rs` is freed from any validation responsibility: it remains purely a minijinja data provider (`schema.get(name).field(f)`), while all validation logic consolidates in `schema::validate` [P].

---

## 6. Not Verified

1. **Memory overhead of parsing large rendered templates:** We did not benchmark parsing a 50KB rendered markdown document via `noyalib` in memory [H]. However, typical template outputs are 1–5KB ($< 100\,\mu\text{s}$) [H].
2. **Behavior of `inquire` on non-standard terminals:** Whether inquire terminal prompts cleanly support custom numeric validation closures without glitching when stdin is redirected was not verified in this harness [H].
3. **External select file performance on write checks:** The author's vault contains select lists of up to 3,734 entries (`brief.md:45`) [V]. Validating set membership against a 3,700-entry `BTreeSet` takes $< 1\,\mu\text{s}$, but loading the cache file if cold was not benchmarked [H].

---

## 7. What This Changes for Ticket 16

1. **Template Pre-Check is firmly (S1) Post-Render Check:**
   Static template analysis (S3) is impossible because frontmatter is unparsed text inside minijinja AST; pre-prompt constraint (S2) is blocked by the *No-Declaration Format* and minimalist `DialogProvider`. The only viable "template pre-check" is validating the rendered note text immediately before writing [P].
2. **Reuse the LSP `SchemaValidator` Kernel Unaltered:**
   No separate template validator is needed. The pure `SchemaValidator::validate(&Note, &Schema)` designed in Ticket 20 serves both the LSP and the template pre-check [P].
3. **Spans are Optional:**
   Confirms Ticket 20's design of `range: Option<Range<BytePos>>`. Template checks emit diagnostics with `range = None`, reporting field-level issues to `stderr` or MCP without needing editor coordinate mapping [P].
4. **Dual Failure Policy Resolves Ticket 15 vs ADR-0006:**
   - Interactive CLI (`traces template`): Advisory (warn on stderr, write file). Preserves user prompt effort [P].
   - Non-interactive / MCP (`traces template --no-input`): Gating guardrail. Fails write and returns structured error payload to agent [P].
5. **No Architectural Coupling Needed:**
   Because `TemplateService` already separates `render()` from `write()` (`src/template/service.rs:121`), validation can be composed at the orchestration layer without entangling `TemplateWriter` with schema validation logic [P].
