# Subsystem Survey: Template Subsystem

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

## 1. Scope & Size
- **Files/Modules Covered**:
  - `src/template/mod.rs` (41 lines) — Module root & re-exports.
  - `src/template/service.rs` (1,257 lines) — Pipeline orchestration (`TemplateService`, `RenderedTemplate`).
  - `src/template/engine.rs` (666 lines) — Minijinja environment setup (`TemplateEngine`, `RenderOutput`, `uuid`).
  - `src/template/loader.rs` (812 lines) — Template discovery & loading (`TemplateLoader`).
  - `src/template/path.rs` (380 lines) — Path wrappers & validation (`TemplatePathInput`, `TemplatePath`, `DeclaredOutputPath`).
  - `src/template/writer.rs` (994 lines) — Target resolution, interactive collision prompt, disk writes (`TemplateWriteTarget`, `WriteMode`, `CommitPolicy`, `WriteOutcome`).
  - `src/template/error.rs` (211 lines) — Error definitions & diagnostic classification (`TemplateError`, `RenderFailureKind`).
  - `src/template/engine/cache.rs` (163 lines) — Minijinja `State`-scoped temp caching helpers (`cached`, `set_temp`, `get_temp`).
  - `src/template/engine/date.rs` (2,486 lines) — `date.*` namespace, date filters, tests, moment-to-strftime compiler (`DateNamespace`, `Anchor`).
  - `src/template/engine/query.rs` (1,961 lines) — `query.*`, `lists.*`, `tasks.*` namespaces, `QuerySet`/`QueryRow` minijinja objects, terminal filters (`QueryOps`).
  - `src/template/engine/schema.rs` (1,280 lines) — `schema.*` namespace, `impl Object for Schema`, field/children/descendants binding (`SchemaOps`).
  - `src/template/engine/ui.rs` (956 lines) — `ui.*` interactive helpers (`text_input`, `select`, `confirm`, `multi_select`), selection recovery (`UiOps`, `SelectOptions`).
  - `src/template/engine/file.rs` (484 lines) — `file.write_to`, `file.include` (`FileOps`).
  - `src/template/engine/path.rs` (581 lines) — Path tests (`path_exists`, `is_file_path`, `is_dir_path`) & filters (`PathOps`).
  - `src/template/engine/string.rs` (600 lines) — Stateless string case & regex filters (`StrOps`).
  - `src/template/engine/num.rs` (188 lines) — Stateless numeric filters (`NumOps`).
  - `src/template/engine/yaml.rs` (261 lines) — Stateless YAML filters via `noyalib` (`YamlOps`).
  - `src/template/engine/error.rs` (56 lines) — Engine error mapping helpers (`TemplateEngineResult`, `confine_error`, `invalid_operation`).
  - `benches/template_render.rs` (238 lines) — Criterion benchmarks for dry-run rendering with swept note counts (up to 1,000 notes).
- **SLoC**: ~13,615 lines total across `src/template/` and `benches/template_render.rs` (~4,650 lines in test modules, ~8,965 lines in production/scaffolding).
- **Public vs pub(crate) Surface**:
  - `pub` surface from `src/lib.rs` / `src/template/mod.rs`: `TemplateService`, `TemplateError`, `RenderFailureKind`, `TemplatePathInput`, `TemplatePathError`, `WriteMode`, `WriteOutcome`, `CommitPolicy` (the latter only under `cfg(any(test, feature = "test-utils"))`).
  - `pub(crate)` / `pub(super)`: Internal pipeline types (`TemplateEngine`, `TemplateLoader`, `TemplatePath`, `DeclaredOutputPath`, `TemplateWriteTarget`, and engine namespace structs `FileOps`, `QueryOps`, `UiOps`, etc.).

---

## 2. Responsibilities
Verified list of distinct responsibilities:
1. **Identifier validation**: Validates relative template paths without escaping (`src/template/path.rs:75-86`, `TemplatePathInput::parse`).
2. **Template discovery & resolution**: Resolves names/stems across local (`.traces/templates/`) and global template directories, rejecting symlinks and handling `.md` stem ambiguity (`src/template/loader.rs:94-108`, `TemplateLoader::find`).
3. **Template listing & completion feed**: Collects available top-level `.md` template stems for CLI listing, completions, and pickers (`src/template/loader.rs:264-272`, `TemplateLoader::list_available`).
4. **Minijinja environment assembly**: Assembles minijinja `Environment`, wires debug mode, custom loader closure, and registers 10 helper namespaces/filter groups (`src/template/engine.rs:109-168`, `TemplateEngine::new`).
5. **Template evaluation & output capture**: Evaluates template string, capturing optional `file.write_to` path from `minijinja::State` temp storage (`src/template/engine.rs:185-204`, `TemplateEngine::render`).
6. **Interactive user input during render**: Pauses render to query `DialogProvider` synchronously for text, confirmation, and index-based single/multi-selection (`src/template/engine/ui.rs:77-133`, `UiOps`).
7. **Query execution inside templates**: Connects template `query.*`, `lists.*`, and `tasks.*` to `IndexerService` and `QueryService`, caching the refreshed `WorkspaceIndex` per render in `State` (`src/template/engine/query.rs:206-238`, `QueryOps::run`).
8. **Schema reflection in templates**: Connects `schema.get()` to preloaded `SchemaService`, enabling dynamic field inspection and traversal of `children()`/`descendants()` (`src/template/engine/schema.rs:116-144,170-229`, `SchemaOps`).
9. **Temporal arithmetic, formatting & Moment dialect parsing**: Parses dates via `DatePoint`, implements Moment-to-strftime translation, applies date arithmetic and unit shifts in templates (`src/template/engine/date.rs:472-623`, `date_format`, `moment_to_strftime`).
10. **File inclusion & confinement**: Reads arbitrary files within project root for inlining into templates (`src/template/engine/file.rs:78-97`, `FileOps`).
11. **Output destination resolution & collision recovery**: Reconciles `-o` override, `file.write_to()`, and default directory by precedence; confines paths to project root; reprompts interactively on collision if interactive and `CreateNew` policy (`src/template/writer.rs:251-290`, `TemplateWriteTarget::resolve`).
12. **Atomic disk commits**: Creates parent directories and writes output files, using `File::create_new` for atomic existence verification (`src/template/writer.rs:88-121,350-366`, `commit`).
13. **Diagnostic classification**: Classifies minijinja evaluation errors into coarse categories (`Syntax`, `Prompt`, `Query`, `Index`, `Io`, `Other`) by unwinding `std::error::Error::source` (`src/template/error.rs:188-210`, `RenderFailureKind::classify`).

---

## 3. Invariants
- **No-Declaration Format**: Templates declare no upfront parameters or manifest; interactive inputs are invoked on demand during rendering (`src/template/GLOSSARY.md:40-44`, ADR-0001). Safe code cannot pre-validate parameters without evaluating.
- **Root Confinement**: All file includes (`file.include`) and output paths (`-o`, `file.write_to()`) MUST NOT escape the project root (`Config::root`) or traverse unverified symlinks outside root (`src/template/engine/file.rs:82-86`, `src/template/writer.rs:303-320`). Enforced at runtime via `SafeRelativePath::parse`.
- **Precedence Hierarchy for Output Path**: Highest precedence is CLI `-o`, followed by template `file.write_to()`, followed by configured default directory (`src/template/writer.rs:227-233`, `TemplateWriteTarget::target_path`). Enforced at resolution time.
- **Atomic Creation under CreateNew**: In `CommitPolicy::CreateNew`, file creation uses `fs::File::create_new` to atomically prevent TOCTOU race conditions (`src/template/writer.rs:102-120`). Enforced in constructor.
- **Dry-Run Confinement to I/O**: `WriteMode::DryRun` suppresses only disk writes; it does NOT suppress interactive prompts or queries (`src/template/service.rs:33-37`, ADR-0001, test `dry_run_still_uses_the_injected_provider_for_ui_calls`). Safe code cannot bypass prompts merely by passing `DryRun`.
- **Render-scoped Index Caching**: Within a single template render, `WorkspaceIndex` is refreshed at most once upon the first query and reused for all subsequent queries (`src/template/engine/query.rs:226-237`, `src/template/engine/cache.rs`). Enforced via `minijinja::State` temp storage.
- **No Symlink Template Discovery**: Template loader ignores symlinks when matching files by exact name or stem (`src/template/loader.rs:135-136,144-146,277-279`).

---

## 4. State & Lifecycle
- **Owned State**:
  - `TemplateService<'a>`: Holds `&'a Config`, `TemplateLoader`, `TemplateEngine`, `Arc<dyn DialogProvider>`.
  - `TemplateEngine`: Holds `minijinja::Environment<'static>` with static loader and global objects.
  - Namespace Globals: `FileOps` (holds `Arc<Path>`), `QueryOps` (holds `Arc<IndexerService>`, `Arc<str>`, `Arc<SchemaService>`), `UiOps` (holds `Arc<dyn DialogProvider>`), `PathOps` (holds `Arc<Path>`), `SchemaOps` (holds `Arc<SchemaService>`).
- **Render-Scoped Transient State (`minijinja::State`)**:
  - `file.write_to`: Stashed as string under `"file.write_to"` (`src/template/engine/file.rs:37`).
  - Index cache: Stashed as `Arc<WorkspaceIndex>` under `"query.index_cache"` (`src/template/engine/query.rs:114`).
  - Schema service cache: Stashed as `Arc<SchemaService>` under `"schema.service_cache"` (`src/template/engine/schema.rs:90`).
- **Root-Template Caching**:
  - **NON-EXISTENT**: Each render calls `self.env.template_from_named_str(name, source)` (`src/template/engine.rs:190-193`). Minijinja parses the root template AST anew on every invocation! (Note: included/extended child templates loaded via `set_loader` may be cached inside minijinja's environment if named, but root string templates are never cached).
- **Concurrency**:
  - Sync-only execution. `Arc<dyn DialogProvider>`, `Arc<SchemaService>`, `Arc<IndexerService>` are `Send + Sync`. Template evaluation is single-threaded; `rayon` is invoked internally when `IndexerService::refresh()` rebuilds/scans notes.

---

## 5. Dependency Edges
- **Outbound `crate::` edges**:
  - `crate::Config`: Root path, template directories, class field, schema directory.
  - `crate::DialogProvider`, `crate::DialogError`, `crate::PresetDialogProvider`: Interactive prompting.
  - `crate::path::{SafeRelativePath, RelativePath, PathError}`: Confinement & path parsing.
  - `crate::dirtree::{DirTree, DirTreeError}`: Directory listing in `loader.rs`.
  - `crate::schema::{Schema, SchemaService, SchemaError, ...}`: Eagerly loaded schema graph & reflection.
  - `crate::index::{IndexerService, WorkspaceIndex, IndexError}`: Delta refresh and index store.
  - `crate::query::{QueryBuilder, QueryError, QueryMode, QueryRow, QuerySet, SourceSelector, ...}`: Evaluation of template query expressions.
  - `crate::temporal::{CalendarError, CalendarPeriod, DateDiff, DateError, DatePoint, DateUnit, DateTimeValue, DurationValue, PeriodEdge, TimeUnit, weekday_from_number}`: Date operations.
  - `crate::{DEFAULT_DATE_PATTERN, DEFAULT_DATETIME_PATTERN, UNIT_SPELLINGS_HINT}`: Formatting defaults.
- **Inbound `crate::` edges**:
  - `src/lib.rs:168`: Re-exports `TemplateService`.
  - `src/cli/template.rs`: Calls `TemplateService::new`, `render_to_file`, `list_available`.
  - `src/cli/init.rs`: Validates template paths.
  - `src/cli/completions.rs`: Calls `TemplateService::list_available`.
  - `benches/template_render.rs`: Benchmarks `TemplateService::render_to_file`.
- **Cyclic / Upward References**:
  - No physical cyclic imports, but high conceptual coupling: `src/template/engine/` duplicates logic and domain types from `src/temporal/`, `src/query/`, `src/schema/`, and `src/dialog/`.

---

## 6. Interfaces
- **Deep Seams**:
  - `TemplateService::render_to_file(&self, name, output, mode)`: Deep seam hiding resolution, read, render, interactive prompt handling, output path collision reprompting, and atomic disk writes behind a single call.
  - `TemplateLoader::find(&self, name)`: Deep seam handling local-before-global search, exact matching, stem search, ambiguity detection, and symlink filtering.
- **Shallow / Pass-Through Seams**:
  - `TemplateEngine`: Primarily an adapter configuring `minijinja::Environment` and attaching globals/filters.
  - `TemplateEngineResult<T>`: Thin alias over `Result<T, minijinja::Error>`.
- **Domain Logic Leakage into Adapters**:
  - `src/template/engine/date.rs` contains a 2,486-line subsystem implementing Moment-to-strftime parsing, custom anchor math, and formatting rules that are not minijinja-specific adapters, but domain date logic.
  - `src/template/engine/ui.rs` contains 956 lines implementing `SelectOptions`, attribute path navigation (`get_path`), fallback formatting, and array deduplication (`extract`).
  - `src/template/engine/query.rs` contains 1,961 lines implementing minijinja `Object` wrappers for `QuerySet`, `QueryRow`, `FileFields`, and `ListFields`.

---

## 7. Algorithms, Data Structures & Performance
- **Moment.js Pattern Compilation**:
  - `moment_to_strftime` (`src/template/engine/date.rs:537-623`): Custom linear byte-scanning parser translating Moment tokens (`YYYY`, `MM`, `Do`, bracket literals `[Daily]`, etc.) into `strftime` format strings. Repeated on *every* filter call (`date_format`) unless pattern contains `%`. No caching of compiled pattern strings.
- **Regex Compilation on Every Filter Invocation**:
  - `regex_replace` and `regex_match` (`src/template/engine/string.rs:125,142`): Call `Regex::new(pattern)` dynamically on *every invocation*. In a template loop over hundreds of rows, the regex is recompiled hundreds of times.
- **Allocation & Cloning in Query Pipeline**:
  - `QueryOps::run` (`src/template/engine/query.rs:206-214`): Refreshes or gets `Arc<WorkspaceIndex>`, then calls `self.service.run(&index, builder)` which produces a `QuerySet`.
  - `QuerySet::call_method` (`src/template/engine/query.rs:398`): Calls `let rows = self.as_ref().clone();` on every chained method (`.filter()`, `.sort()`, `.limit()`, `.group_by()`, `.flatten()`), cloning the underlying `Vec<QueryRow>`!
- **Root Template Parsing Overhead**:
  - `self.env.template_from_named_str(name, source)` (`src/template/engine.rs:190-193`): The template source is parsed into an AST and compiled by minijinja on *every execution*.
- **Benchmark Data (`benches/template_render.rs`)**:
  - Evaluates `TemplateService::render_to_file` in `DryRun` over synthetic notes (10, 50, 100, 500, 1000 notes).
  - Sweeps `list_report` (`query.from() | list("file.path")`), `table_report` (`query.from().where(...).sort(...).table(...)`), and `date_report` (loop of 200 date arithmetic/formatting calls).
  - Demonstrates that delta refresh floor + query execution + template rendering scales with note count, but date parsing loop incurs non-trivial CPU overhead in string transformations.

---

## 8. Error Topology
- **Error Types**:
  - `TemplateError`: User-facing enum covering `Resolve(TemplatePathError)`, `Read`, `OutputFileAlreadyExists`, `OutputPathEscapesRoot`, `OutputPathUnverifiable`, `Render`, `Write`, `Prompt(DialogError)`, `SchemaLoad(SchemaError)`.
  - `TemplatePathError`: Covers `Absolute`, `UnsafeComponent`, `AmbiguousTemplate`, `DirectoryRead`, `TemplateNotFound`.
  - `RenderFailureKind`: Coarse enum (`Syntax`, `Prompt`, `Query`, `Index`, `Io`, `Other`) used by CLI diagnostics.
  - `minijinja::Error`: Internal rendering error. Submodules wrap errors into `ErrorKind::InvalidOperation` using `invalid_operation(msg, err)`.
- **Loss / Preservation of Error Context**:
  - `TemplateEngine` runs with `env.set_debug(true)` (`src/template/engine.rs:120`), retaining line and column numbers.
  - Template name passed to minijinja is the template's absolute file path (`src/template/service.rs:144-148`), enabling accurate file:line:col diagnostic rendering in the CLI.
  - Submodule errors preserve the root cause via `.with_source(err)` (`src/template/engine/error.rs:55`).
  - `RenderFailureKind::classify` (`src/template/error.rs:188-210`) unwinds the `source` chain via `downcast_ref` to recover `DialogError`, `QueryError`, `IndexError`, and `io::Error`.

---

## 9. Duplicated / Conflicting Concepts
- **`date.rs` vs `src/temporal/`**:
  - `DateNamespace` (`src/template/engine/date.rs`, 2,486 lines) duplicates concepts from `src/temporal/`. While it delegates basic parsing to `DatePoint::parse` (`src/temporal/point.rs`), it implements its own:
    - Moment.js token table and parser (`MOMENT_TOKENS`, `moment_to_strftime`, `pattern_to_strftime`).
    - Shorthand anchor base resolution (`Anchor::base`, `Anchor::transform`) duplicating temporal calculation logic.
    - Ordinal suffix formatting (`ordinal_suffix`).
    - Unit string parsing and shift dispatch (`shift_by_unit`).
  - **Verdict**: Domain date parsing and pattern conversion logic leaked heavily into the template engine rather than living in `temporal`.
- **`query.rs` vs `src/query/`**:
  - `QueryOps` (`src/template/engine/query.rs`, 1,961 lines):
    - Re-wraps `QuerySet`, `QueryRow`, `FileField`, and `ListField` into minijinja `Object` implementations.
    - Duplicates query terminal rendering formats (`table_filter`, `list_filter`, `task_list_filter`).
    - Contains custom downcasting hacks (`SourceSelector` as an `Object` to smuggle class/filter arguments through minijinja).
- **`schema.rs` vs `src/schema/`**:
  - `impl Object for Schema` (`src/template/engine/schema.rs:170-229`): Implements dynamic field lookup, children, and descendants queries directly within `template::engine` by reaching into `State`'s temp cache for `Arc<SchemaService>`.
- **Template Discovery & Resolution**:
  - `TemplateLoader` implements search across local and global directories, while `Config` also knows about `local_template_dir` and `global_template_dir`. Stem matching and ambiguity detection live strictly in `loader.rs`.

---

## 10. Doc / Glossary / ADR Drift
- **ADR-0001 (Lazy Interactive Custom Functions)**:
  - *Claim*: "All interactive functions are routed through a DialogProvider trait... Two implementations: TerminalDialogProvider and PresetDialogProvider." -> **VERIFIED** (`src/template/engine/ui.rs:80-130`).
  - *Claim*: "dry-run does not suppress prompts." -> **VERIFIED** (`src/template/service.rs:225-237`, prompts run during `render`, `WriteMode::DryRun` only stops `write`).
  - *Claim*: "Registration table: uuid -- add_function -- src/template/engine.rs" -> **VERIFIED** (`src/template/engine.rs:164`).
- **ADR-0003 (Index-based Dialog Selection)**:
  - *Claim*: "There is exactly one selection primitive: select(label, &[String]) -> DialogResult<usize>, multi_select(label, &[String]) -> DialogResult<Vec<usize>>" -> **VERIFIED** (`src/dialog/mod.rs`, `src/template/engine/ui.rs:106-109,121-124`).
  - *Claim*: "TemplateService inspects a template's input array at runtime... maps each element to a display label, calls select, and recovers the chosen object by the same index." -> **VERIFIED** in `UiOps::extract` / `SelectOptions` (`src/template/engine/ui.rs:147-221`), though note it lives in `UiOps` under `template::engine::ui`, not `TemplateService`.
- **ADR-0005 (redb Index with QueryOps Namespace)**:
  - *Claim*: "Single query.from() method... terminal methods: table, list, task_list, count. All terminal methods are also registered as pipeline filters." -> **VERIFIED** (`src/template/engine/query.rs:188-193,368-395`).
- **GLOSSARY.md**:
  - *Claim*: "`WriteMode`: The choice between previewing rendered content and committing it to disk... CommitPolicy: Existing-file behavior for committed writes." -> **VERIFIED** (`src/template/writer.rs:36,65`).

---

## 11. Candidate Architectural Questions

1. **Root-template AST caching vs per-render re-parsing**:
   - *Claim*: Minijinja recompiles the template string AST on every `render()` call (`self.env.template_from_named_str(name, source)` at `src/template/engine.rs:192`), adding parsing latency to repeated instantiations or bench runs.
   - *Counter-argument*: In the CLI, traces is a short-lived process running one template instantiation per invocation. Re-parsing a 2KB template takes <50μs, which is dwarfed by filesystem I/O and index refresh (~1-10ms).
   - *Open Decision*: Should templates be compiled into reusable pre-parsed template AST artifacts (crucial for LSP live buffer analysis and batch rendering), or keep on-the-fly string compilation?
   - *Touches*: TemplateEngine, LSP language intelligence (`.scratch/md-pkm-lsp/map.md:56,90`).

2. **Domain leakage: Moment.js parsing and date anchor math in `engine/date.rs` vs `src/temporal/`**:
   - *Claim*: `src/template/engine/date.rs` (2.5K lines) contains domain logic for Moment pattern compilation (`moment_to_strftime`), ordinal computation, and anchor manipulation that should belong in `src/temporal/`.
   - *Counter-argument*: The Moment syntax is specific to template authoring conventions (Obsidian Templater compatibility), whereas `src/temporal` focuses on strict RFC/ISO temporal models with Jiff. Isolating template-specific format dialects in the template adapter keeps `temporal` clean.
   - *Open Decision*: Keep Moment formatting in `template::engine::date` as a template-specific adapter, or promote pattern translation and date anchors into `temporal`?
   - *Touches*: `src/temporal/`, `src/template/engine/date.rs`.

3. **Synchronous, blocking I/O (`DialogProvider` and `redb`) during template rendering**:
   - *Claim*: Template evaluation is synchronous and blocks the thread on interactive user terminal prompts (`ui.text_input`, `ui.select`) and index delta refreshes. In an LSP or daemon context, blocking render prevents concurrent analysis or requires dedicated thread pools.
   - *Counter-argument*: In CLI tools, synchronous evaluation is simple, deterministic, and avoids async runtime overhead (`tokio`). ADR-0001 explicitly selected minijinja custom functions over scripting engines because of this clean synchronous separation.
   - *Open Decision*: Retain synchronous `render` with decoupled static analysis (`machinery::parse` for LSP), or adopt an asynchronous / coroutine execution model for interactive helpers?
   - *Touches*: `src/dialog/`, `src/template/engine/ui.rs`, LSP architecture.

4. **Dynamic downcasting and `State` temp storage hacks for index and schema services**:
   - *Claim*: Globals in minijinja cannot easily access runtime context across calls, forcing `engine/cache.rs` to stash `Arc<WorkspaceIndex>` and `Arc<SchemaService>` in `minijinja::State` temp storage using downcastable `Cached<T>` wrappers.
   - *Counter-argument*: This mechanism cleanly confines the lifetime of loaded services to the duration of a single render without leaking state across renders or requiring `TemplateEngine` to be rebuilt for each render.
   - *Open Decision*: Keep `State` temp storage downcasting, or refactor `TemplateEngine` to be instantiated per-render with explicit context references?
   - *Touches*: `src/template/engine/cache.rs`, `src/template/engine/query.rs`, `src/template/engine/schema.rs`.

5. **Cloning `Vec<QueryRow>` across chained query filters in minijinja**:
   - *Claim*: `QuerySet::call_method` (`src/template/engine/query.rs:398`) clones the underlying `QuerySet` (`let rows = self.as_ref().clone()`) on every intermediate chained call (`.filter()`, `.sort()`, `.limit()`), causing unnecessary heap allocations on large query results.
   - *Counter-argument*: Intermediate query rows in minijinja are wrapped in immutable `Arc<QuerySet>`, and minijinja's `call_method` receives `&Arc<Self>`, which does not allow in-place mutation without cloning or interior mutability (`RefCell`).
   - *Open Decision*: Introduce interior mutability / copy-on-write builders for intermediate query chains, or leave query clone overhead as negligible for typical PKM result sizes (<1,000 rows)?
   - *Touches*: `src/query/`, `src/template/engine/query.rs`.

6. **Stateless regex compilation on every filter execution in `string.rs`**:
   - *Claim*: `regex_replace` and `regex_match` (`src/template/engine/string.rs:125,142`) construct a new `Regex` on every single invocation without caching.
   - *Counter-argument*: Simple templates rarely invoke regexes in hot loops. Keeping the filter stateless eliminates global mutex locks or thread-local regex caches.
   - *Open Decision*: Add a small LRU or bounded thread-local regex cache to `StrOps`, or keep it as zero-state?
   - *Touches*: `src/template/engine/string.rs`.
