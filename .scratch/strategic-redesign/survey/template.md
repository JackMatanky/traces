# Architectural Survey: Template Subsystem

## 1. Responsibilities & seams

- **What it owns**: Template lookup/resolution across local/global paths, minijinja environment wiring (10 helper namespaces/filter groups), interactive dialog orchestration during render, query/schema reflection inside templates, temporal format translation (Moment to strftime), output path resolution/collision recovery, and atomic root-confined file writing.
- **Public Surface (`src/template/mod.rs:29-35`, `src/lib.rs:168,198-200`)**:
  - `TemplateService` (`service.rs:38`): Main entry point (`new`, `render_to_file`, `list_available`).
  - `TemplatePathError` (`path.rs:196`), `TemplateError` (`error.rs:27`), `RenderFailureKind` (`error.rs:149`).
  - `WriteMode` (`writer.rs:36`), `WriteOutcome` (`writer.rs:125`), `CommitPolicy` (`writer.rs:65`).
  - Internal crate surface (`pub(crate)`/`pub(super)`): `TemplateEngine` (`engine.rs:74`), `TemplateLoader` (`loader.rs:53`), `TemplateWriteTarget` (`writer.rs:140`), `RenderedTemplate` (`service.rs:316`), namespace ops (`FileOps`, `QueryOps`, `UiOps`, `DateNamespace`, `SchemaOps`, `PathOps`, `StrOps`, `NumOps`, `YamlOps`).
- **Callers**:
  - `src/cli/template.rs:140,162,240`: Runs `render_to_file` and `list_available`.
  - `src/cli/completions.rs:102`: Calls `list_available` for shell completion.
  - `benches/template_render.rs:45,100-230`: Sweeps dry-run rendering benchmarks up to 1,000 notes.
- **What callers must know**:
  - Templates follow a No-Declaration model (`GLOSSARY.md:40`): all inputs (`ui.*`) are prompted lazily during rendering. Callers wanting non-interactive runs must pass `PresetDialogProvider`, not rely on `WriteMode::DryRun` (`service.rs:33-37`).
  - `TemplatePathInput::parse` (`path.rs:75`) must validate relative identifiers before lookup.
  - Destination resolution strictly prioritizes: CLI `-o` > template `file.write_to()` > `Config::output_dir` (`writer.rs:227-233`).

## 2. Invariants

- **No-Declaration Dynamic Prompting**: Templates declare no upfront parameters or manifest; interactive prompts (`ui.text_input`, `ui.select`, `ui.confirm`, `ui.multi_select`) are invoked dynamically at evaluation time (`GLOSSARY.md:40-44`, ADR-0001:19-26, `engine/ui.rs:80-130`). Safe code cannot pre-validate parameters statically. Enforced: convention.
- **Root Confinement on Read & Write**: Output destination (`-o` and `file.write_to()`) and inlined files (`file.include()`) MUST NOT escape `Config::root` or traverse unverified symlinks outside root (`engine/file.rs:82-86`, `writer.rs:303-320`). Enforced: runtime via `SafeRelativePath::parse`.
- **Root Confinement Leak in Path Inspection**: Path tests (`path_exists`, `is_file_path`, `is_dir_path`) accept absolute paths as-is without root confinement (`engine/path.rs:161-170`). Enforced: nowhere (intentional leak allows probing host paths).
- **Output Precedence**: Destination resolves as: (1) CLI `-o`, (2) template `file.write_to()`, (3) `Config::output_dir` (`writer.rs:227-233`). Enforced: runtime in `TemplateWriteTarget::target_path`.
- **Atomic Creation under CreateNew**: In `CommitPolicy::CreateNew`, creation uses OS-level `std::fs::File::create_new` (`O_CREAT | O_EXCL`) to guarantee atomic existence check and prevent TOCTOU races (`writer.rs:68-70,102-120`). Enforced: construction.
- **Dry-Run Output Resolution Bypass**: `WriteMode::DryRun` early-returns before resolving output target or running root confinement checks on `file.write_to()` (`service.rs:225-227`). Enforced: runtime match.
- **Render-Scoped Index Refresh & Caching**: `WorkspaceIndex` is refreshed at most once per render across `query.*`, `lists.*`, and `tasks.*`, cached in `minijinja::State` temp storage (`engine/query.rs:211-237`, `engine/cache.rs:61-75`). Enforced: runtime cache wrapper.
- **Eager Schema Construction**: `SchemaService` is constructed eagerly during `TemplateEngine::new` (`engine.rs:131-135`), failing initialization if schema files have parse errors or cycle, even if the template uses no schemas. Enforced: construction.
- **Symlink Rejection in Template Discovery**: Templates cannot be symlinks: `loader.rs` checks `symlink_metadata().is_file()` (`loader.rs:135-136`) and `node.file_type().is_file()` (`loader.rs:190,293`), rejecting symlinks for exact and stem matches. Enforced: runtime discovery.

## 3. Verified observations

- [V] Root template AST is not cached: `TemplateEngine::render` compiles `template_from_named_str(name, source)` on every execution (`engine.rs:190-193`).
- [V] Rich domain types are degraded to strings across minijinja boundary: `NoteFieldValue::Date`, `DateTime`, `Duration`, `Link` become flat strings (`engine/query.rs:604-607`), forcing downstream date filters (`date_format`, `date_add`, `is_past`) to re-parse them from scratch via `parse_date_point` (`engine/date.rs:473`).
- [V] QuerySet method chaining clones row vectors: `QuerySet::call_method` executes `let rows = self.as_ref().clone();` on every chained method (`.filter()`, `.sort()`, `.limit()`, `.group_by()`, `.flatten()`) (`engine/query.rs:398-428`), causing `Vec<QueryRow>` allocations on each intermediate stage.
- [V] Regexes compiled on every invocation: `regex_replace` and `regex_match` invoke `Regex::new(pattern)` on every single call without caching (`engine/string.rs:199,213`).
- [V] Moment.js pattern translation runs per filter call: `date_format` calls `pattern_to_strftime`, which executes byte-by-byte `moment_to_strftime` scanning on every invocation unless `%` is present (`engine/date.rs:476,492,537-623`).
- [V] Path inspection tests bypass root confinement: `InspectTarget::resolve` allows absolute paths as-is (`engine/path.rs:162-163`), enabling `path_exists`, `is_file_path`, and `is_dir_path` to probe arbitrary host paths outside `Config::root`.
- [V] DryRun skips output confinement validation: `TemplateService::write` early-returns `WriteOutcome::Previewed` if `mode` is `DryRun` (`service.rs:225-227`), meaning output paths declared via `file.write_to()` are never validated for root escapes during preview.
- [V] Template listing ignores subdirectories: `TemplateLoader::stems_in` only scans direct children of template directories (`loader.rs:283-305`), preventing `list_available` from discovering templates in subfolders even though `find` supports nested paths (`loader.rs:168-171`).
- [V] Select deduplication is $O(N^2)$: `SelectOptions::extract` deduplicates items by scanning `!new_values.contains(&value)` on minijinja `Value`s (`engine/ui.rs:197-200`).
- [V] Schema Object downcasting relies on fragile State temp storage: `impl Object for Schema` relies on `SchemaOps::get_value` having stashed `Arc<SchemaService>` in `State` under `SCHEMA_SERVICE_CACHE_KEY` (`engine/schema.rs:125-129,161-167`), failing with an internal error if accessed without the cache seeded.

## 4. Hypotheses

- **[H1] Typed Minijinja Value Protocol vs String Degradation (Rank 1)**:
  - *Thesis*: Wrapping domain primitives (`DatePoint`, `DateTimeValue`, `DurationValue`, `Link`) in custom minijinja `Object`s instead of stringifying them (`engine/query.rs:604-607`) eliminates repeated string re-parsing in date filters (`engine/date.rs:473`) and preserves domain methods directly on values (e.g. `row.due.format(...)`, `row.link.target`).
  - *Current vs Alt*: Current converts all query values to plain strings. Alternative: implement `Object` for `DatePoint`/`Link` with method dispatch, or keep string conversion.
  - *Removable*: Eliminates repeated calls to `parse_date_point`, `DatePoint::parse`, and string allocations in loops (`benches/template_render.rs:96`).
  - *Disprove*: Benchmark shows string parsing is negligible (<5%) compared to index scan; or `Arc<dyn Object>` dynamic dispatch overhead exceeds parsing short ISO strings.
  - *Blast radius*: `engine/query.rs`, `engine/date.rs`, `note/`.
- **[H2] Consolidate Moment.js Translation and Temporal Shorthand into `src/temporal/` (Rank 2)**:
  - *Thesis*: Moving the 2,486-line date subsystem (`engine/date.rs`) into `src/temporal/` as a cohesive `FormatDialect` / `CalendarAnchor` component eliminates duplication of temporal concepts and reduces `template::engine` to a shallow minijinja adapter.
  - *Current vs Alt*: Current embeds Moment token tables, ordinal formatting, and anchor math inside `engine/date.rs:147-260,498-623`. Alternative: Move to `temporal::dialect::moment` and `temporal::anchor`, keeping `engine/date.rs` < 200 lines of glue.
  - *Removable*: ~2,000 lines of domain temporal logic inside template engine; allows CLI query / format commands to share Moment-style formatting.
  - *Disprove*: Moment formatting is uniquely template-specific (Obsidian Templater compatibility) and polluting `src/temporal/` (strict RFC/ISO and Jiff based) violates temporal's invariants.
  - *Blast radius*: `src/template/engine/date.rs`, `src/temporal/`.
- **[H3] Zero-Copy Query Builder & Pipeline Pushdown vs Materialized Cloning (Rank 3)**:
  - *Thesis*: Replacing eager `QuerySet` cloning on each chained method (`engine/query.rs:398`) with a builder pipeline that accumulates `.where()`, `.sort()`, `.limit()` before execution enables query plan optimization (filter/sort fusion, top-k limit) and avoids intermediate `Vec<QueryRow>` allocations.
  - *Current vs Alt*: Current executes `service.run(&index, builder)` in `query.from()` producing a materialized `QuerySet` (`engine/query.rs:213`), and every subsequent `.filter()` or `.sort()` clones the entire `Vec<QueryRow>` (`engine/query.rs:398`). Alternative: Return a `QueryPlan` / lazy builder `Object` that executes only upon terminal evaluation (`.table()`, `.list()`, iteration).
  - *Removable*: Cloned `Vec<QueryRow>` allocations (`engine/query.rs:398`); unlocks pre-fetch filter/sort pushdown into store (`.scratch/template-service/module-design-review.md:18-20,29`).
  - *Disprove*: Templates frequently inspect row lengths or iterate directly with `{% for row in query.from().where(...) %}`, making lazy execution require complex iterator adapter wrappers in minijinja.
  - *Blast radius*: `src/template/engine/query.rs`, `src/query/`.
- **[H4] Confine Path Inspection to Project Root (Rank 4)**:
  - *Thesis*: Harmonizing path inspection (`path_exists`, `is_file_path`, `is_dir_path`) with `file.include` and `writer` by strictly confining all path arguments to `Config::root` (`engine/path.rs:162-170`) closes an information leakage vector where templates can stat arbitrary host filesystem paths.
  - *Current vs Alt*: Current allows absolute paths in `InspectTarget::resolve` (`engine/path.rs:162-163`). Alternative: Route all path candidates through `SafeRelativePath::parse(root, ...)`, rejecting absolute paths outside root.
  - *Removable*: Security seam discrepancy between file reading/writing (confined) and file stat (unconfined).
  - *Disprove*: User templates legitimately check for external tool paths (e.g., `"/usr/local/bin/pandoc" is is_file_path`) or global templates before execution.
  - *Blast radius*: `src/template/engine/path.rs`, `src/template/error.rs`.
- **[H5] Validate Output Paths during DryRun (Rank 5)**:
  - *Thesis*: Resolving and validating `file.write_to()` output paths during `WriteMode::DryRun` (`service.rs:225-227`) ensures path confinement errors (`OutputPathEscapesRoot`) and unresolvable targets are caught during preview before committing.
  - *Current vs Alt*: Current returns `Previewed(content)` in `DryRun`, completely skipping `TemplateWriteTarget::target_path` and `confine`. Alternative: Call `target_path` in both modes; only skip disk `commit()`.
  - *Removable*: Discrepancy where a template succeeds under dry-run but crashes on commit.
  - *Disprove*: In dry-run, output file collision prompts would annoy users, or resolving paths when user passes `--dry-run` without full environment setup causes spurious failures.
  - *Blast radius*: `src/template/service.rs`, `src/template/writer.rs`.
- **[H6] Reusable Pre-Compiled Template AST Cache (Rank 6)**:
  - *Thesis*: Caching minijinja compiled template ASTs (`engine.rs:190-193`) keyed by template path and content hash eliminates template re-parsing during batch note generation and benchmark sweeps.
  - *Current vs Alt*: Current parses template string anew on every `render()` call. Alternative: Use `Environment::add_template` / compiled bytecode or keep string parsing.
  - *Removable*: Re-parsing overhead in multi-template workflows and Criterion benchmarks (`benches/template_render.rs:96-98`).
  - *Disprove*: In CLI execution, traces is a single-shot process that renders exactly one template per invocation; template parse time (<50μs) is <1% of runtime compared to index refresh (~1-10ms), so caching adds memory and cache invalidation complexity for no perceptible user benefit.
  - *Blast radius*: `src/template/engine.rs`, `src/template/service.rs`.
- **[H7] Lazy Schema Loading in TemplateEngine (Rank 7)**:
  - *Thesis*: Deferring `SchemaService` loading from `TemplateEngine::new` (`engine.rs:131-135`) to the first `schema.get()` or `@Class` query call prevents schema parse errors from breaking unrelated template renders.
  - *Current vs Alt*: Current unconditionally loads and validates all schemas upfront, failing `TemplateService::new` with `TemplateError::SchemaLoad` even for templates that only use `date` or `ui`. Alternative: Wrap `SchemaService` in `OnceCell` / lazy loader.
  - *Removable*: Hard dependency between template engine initialization and schema registry health.
  - *Disprove*: Failing early at service startup surfaces schema syntax errors immediately before any user interaction, preventing partial execution where user answers prompts only for the render to fail later.
  - *Blast radius*: `src/template/engine.rs`, `src/template/service.rs`.
- **[H8] Regex and Pattern Caching in Template Helpers (Rank 8)**:
  - *Thesis*: Adding a thread-local or small LRU cache for compiled `Regex` (`engine/string.rs:199,213`) and compiled Moment patterns (`engine/date.rs:495`) avoids recompiling identical patterns inside template loops (`{% for ... %}`).
  - *Current vs Alt*: Current recompiles on every call. Alternative: Caching regex/moment formats.
  - *Removable*: Redundant string parsing and NFA compilation in loops.
  - *Disprove*: Template loops rarely execute >100 iterations in PKM; Criterion benchmarks show date report runs in microseconds, and regex caching introduces cache locking/contention.
  - *Blast radius*: `src/template/engine/string.rs`, `src/template/engine/date.rs`.

## 5. Cross-slice couplings

- `template -> temporal`: Uses `DatePoint`, `DateTimeValue`, `DurationValue`, `TimeUnit`, `DateUnit`, `PeriodEdge`, `CalendarPeriod`, `CalendarError`, `DateError`, `local_zone()`. Leaks Moment dialect compilation (`engine/date.rs:498-623`) and anchor calculations (`engine/date.rs:147-260`) in template engine rather than core temporal.
- `template -> query`: Uses `QueryBuilder`, `QueryService`, `QuerySet`, `QueryRow`, `SourceSelector`, `SourceAtom`, `FieldPath`, `FileField`, `ListField`, `TaskPathStyle`. Re-implements minijinja `Object` wrappers for `QuerySet`/`QueryRow` (`engine/query.rs:309,517`). Degrades rich domain query values (`Date`, `DateTime`, `Duration`, `Link`) to flat strings (`engine/query.rs:604-607`).
- `template -> schema`: Uses `Schema`, `SchemaService`, `SchemaError`. Loads schema registry eagerly in `TemplateEngine::new` (`engine.rs:131-135`). Implements `Object for Schema` directly in `template::engine::schema` (`engine/schema.rs:170`), and accesses `SchemaService` via downcast from `minijinja::State` temp storage (`engine/schema.rs:161-167`).
- `template -> dialog`: Uses `DialogProvider`, `DialogError`. Direct dependency in `TemplateService` and `UiOps` (`engine/ui.rs:45-47`). Interactive collision reprompting in `writer.rs:270-278`.
- `template -> index`: Uses `IndexerService`, `WorkspaceIndex`, `IndexError`. Render-scoped caching of refreshed `WorkspaceIndex` inside `minijinja::State` (`engine/query.rs:211-237`).
- `template -> config / path`: Uses `Config`, `SafeRelativePath`, `RelativePath`, `PathError`, `DirTree`. Enforces root confinement on outputs and file includes.

## 6. Comparables worth studying

- **Obsidian Templater (JavaScript plugin)**: Dynamic evaluation model with `<% tp.file.* %>`, `<% tp.date.* %>`, `<% tp.system.prompt() %>`. Moment.js native integration. Informs H1 & H2 on handling user prompts and date dialects.
- **minijinja (Mitsuhiko / GitHub)**: `minijinja::value::Object`, `Value::from_object`, `State::set_temp`, custom loaders. Informs H1, H3, H6 on how minijinja objects can expose getters and methods without string conversion.
- **cargo-generate (Rust CLI)**: Project scaffolding tool. Pre-collects template variables via manifest/prompts before rendering liquid templates; contrasts with Traces' No-Declaration lazy dynamic prompt model (ADR-0001). Informs H1 & Open Decisions.
- **DataFusion / jaq (Rust)**: Streaming query execution and expression evaluation; informs H3 on pushdown of filters and sorts into the query engine before materialization.
- **Jiff (BurntSushi / Rust)**: Modern civil and zoned datetime library. Informs H2 on cleanly handling format strings, civil calendars, and strftime compilation.

## 7. Open decisions

- `decision(template): typed-value-protocol-vs-string-degradation`: Should `NoteFieldValue` maintain typed `Object` wrappers in minijinja or remain flattened to strings?
- `decision(template): move-moment-and-anchor-to-temporal`: Should Moment.js pattern translation and calendar anchor logic migrate from `engine/date.rs` to `src/temporal/`?
- `decision(template): query-pipeline-pushdown`: Should `query.from()` return a lazy query builder object in minijinja to enable plan fusion and sort-limit pushdown, or keep eager `QuerySet` execution?
- `decision(template): path-inspection-root-confinement`: Should `is_file_path`, `path_exists`, and `is_dir_path` be strictly confined to `Config::root`, removing the unconfined absolute path loophole?
- `decision(template): dry-run-output-path-validation`: Should `WriteMode::DryRun` validate output paths and check for root escapes instead of bypassing `TemplateWriteTarget`?
- `decision(template): recursive-template-directory-listing`: Should `TemplateLoader::list_available` discover templates recursively in subdirectories to match `TemplateLoader::find`?
- `decision(template): lazy-vs-eager-schema-loading`: Should `SchemaService` loading be deferred until first use or remain an eager requirement of `TemplateEngine::new`?
- *Preserved fog*: How LSP language intelligence (e.g. autocompletion for `record.file.*` and schema fields in `.md` template buffers) will integrate with minijinja's runtime-only `Object` reflection without a static AST analyzer.

## 8. Withstands scrutiny

- **Separation of Render and Write stages via `RenderedTemplate` (`service.rs:160-224`)**: Separating `render` and `write` ensures that interactive prompts run exactly once, while output target resolution, collision prompt, and atomic disk writes happen in a decoupled second phase. This prevents re-prompting if write fails or needs collision recovery.
- **Atomic file creation via `File::create_new` under `CreateNew` (`writer.rs:105`)**: Using OS-level atomic creation (`O_CREAT | O_EXCL`) avoids TOCTOU vulnerabilities and prevents accidental data loss without holding file locks.
- **Render-scoped caching in `minijinja::State` temp storage (`engine/query.rs:230`, `engine/cache.rs:61-75`)**: Scoping index refresh to the lifetime of a single render prevents stale index caching across CLI service reuse while ensuring multiple queries within one template pay for at most one delta scan.
- **Diagnostic mapping via absolute path naming (`service.rs:276`, `engine.rs:120`)**: Passing the template's absolute file path to minijinja as the template name combined with `env.set_debug(true)` preserves exact source file, line, and column numbers for CLI error reporting.
