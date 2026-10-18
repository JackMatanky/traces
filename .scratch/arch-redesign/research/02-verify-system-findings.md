# Research Verification: Index, Query, Template, CLI, and Topology Findings

**Baseline commit**: `0c43149da740fd5d7102a067b8d08ff759ba830b` on `main` (working tree clean for `src/`).  
**Active in-flight worktree**: `.worktrees/config-digest-sections` (`49c5ac39e3c759d68bde58f0751bb8b0ca0b6cad`, branch `config-digest-sections`).  
**Scope**: Factual verification ledger of area survey claims across index, query, template, CLI, and crate topology. Planning-only; zero production code or test changes.

---

## 1. Verification Ledger Summary

| # | Topic | Verdict | Key Finding / Correction |
|---|---|---|---|
| 1 | **Index Store** | `PARTIAL` / `MOVING (07/17)` | 8 tables verified in `tables.rs` (no `lists` table); `IndexStore` mixes 7 distinct responsibility clusters; fail-open only catches incremental persist errors, failing closed on initial `open`; config drift is in-flight via Issue 07/17. |
| 2 | **Two Query Paths** | `VERIFIED` | CLI uses `run_from_store` exclusively; templates use `run` exclusively; both share row-expansion and execution plans, producing identical results for `Not(...)`, class expansion, and folders. |
| 3 | **Query Row & Plan** | `VERIFIED` | `QueryRow` is 48 bytes and clones `Arc<WorkspaceIndex>` per row; `cmp_document_order` defect verified (ignores `item_idx` for list/task rows, returning `Equal` within same note, unpinned by tests); `TextShape::classify` called in both filter and sort; sources and filters use separate ASTs with overlapping semantics. |
| 4 | **FileClassExpander Seam** | `VERIFIED` | Exactly 1 production impl (`SchemaService`), 1 test mock (`RecordingExpander`); zero trait constructor methods; `query` has 0 dependencies on `schema` in production and test. |
| 5 | **Template Engine** | `PARTIAL` | `TemplateEngine::new` is a second composition root; `query.index_cache` and `schema.service_cache` verified; survey claim that chained calls deep-clone rows is **REFUTED** (`QuerySet` clones `Arc` shallowly); root templates re-parsed per render; Moment translation is local (~140 lines) while anchor math delegates to `temporal`; `query` and `schema` modules hold query/AST domain logic. |
| 6 | **CLI Composition** | `PARTIAL` | Verified service constructors; `CliError` has **22 variants** (survey claim of "52+" is **REFUTED**); zero non-CLI production code writes stdout/stderr; `normalize_source_input` touches filesystem; `cli/task.rs` builds filter strings via string concatenation. |
| 7 | **Crate Topology** | `PARTIAL` | Production module DAG is strictly acyclic (0 cycles); survey missed `config -> temporal` and `index -> env_vars`; `traces::testing` has exactly 45 identifiers; `TestProject` is 266 lines (test module is 669 lines); 42 error types verified; "6 layers error propagation" is **REFUTED**; survey cited non-existent files `src/cli/query.rs`/`src/cli/schema.rs`. |
| 8 | **Persisted-Format Coupling** | `VERIFIED` | Only `FileMeta` and `Note` serialized as structured payloads into redb; 16 types carry serde derives solely for redb; zero format/version guards exist beyond the single `CACHE_FORMAT` key (`b"1"`). |

---

## 2. Detailed Claim Verifications

### Claim 1: Index Store Responsibilities, Tables, Codec, and Resilience

- **Status**: `PARTIAL` / `MOVING (07/17)`
- **Evidence**:
  - `src/index/store.rs:141-1700`: `IndexStore` mixes 7 distinct responsibility clusters across ~1,560 lines of production code:
    1. *Lifecycle & DB Management*: `open` (156), `create_db` (790), `rebuild_needed` (823), `is_rebuild_trigger` (768), `wrap_redb_error` (754).
    2. *Transaction Management*: `begin_read` (323), `begin_write` (333), `commit` (338), `configure_cache_txn` (343), `begin_cache_txn` (1172).
    3. *Table Access & Codec Primitives*: `open_table_for_read` (378), `open_multimap_for_read` (391), `open_multimap_for_write` (1267), `read_table` (426), `write_table` (576), `write_links` (616), `write_cache_format` (598), `delete_tables` (1184).
    4. *Whole-Table Bulk Reads*: `read_all` (453), `read_all_notes` (490), `read_all_files` (502), `read_all_links` (514), `decode_table_rows` (1061), `collect_multimap_links` (1080), `resolve_link_entry` (1115), `collect_source_paths` (1138), `path_by_key` (404).
    5. *Incremental Row Patching*: `persist` (654), `apply_rebuild` (684), `apply_incremental` (712), `apply_diff_deletions` (1300), `delete_files_and_notes` (1314), `delete_index_entries_for_paths` (1337), `remove_axis_entry` (1362), `apply_diff_upserts` (1379), `apply_modified_notes` (1395), `upsert_notes` (1409), `upsert_tags_and_classes` (1424), `upsert_index_axis` (1445), `apply_inlink_delta` (1471), `upsert_row` (1499).
    6. *Secondary Axis Extraction*: `write_all_parallel` (1192), `write_axes_parallel` (1203), `write_index_by_value` (1224), `write_index_by_path` (1246), `collect_multimap_values` (1281), `IndexDimensions::for_class_field` (1582), `IndexDimension::visit_values` (1639-1665), `with_lowercased` (1671).
    7. *Query-Backend Reads*: `read_notes_batch` (191), `read_files_batch` (206), `read_links_for_targets` (225), `paths_with_tag` (266), `paths_with_file_class` (289), `paths_in_folder` (305), `read_batch` (882), `collect_batch` (914), `collect_stored_paths` (923), `paths_from_multimap` (941), `collect_folder_paths` (958), `collect_all_folder_paths` (977), `collect_prefixed_folder_paths` (998), `collect_sorted_path_keys` (1016).
  - `src/query/service.rs:140, 147, 152, 359, 376, 395, 402, 410, 415, 424`: `QueryService` calls `IndexStore` directly via `pub(crate)` visibility.
  - `src/index/tables.rs:100-133`: Exactly **8 redb tables** are defined:
    1. `FILES` (`TableDefinition`, required wipe)
    2. `NOTES` (`TableDefinition`, required wipe)
    3. `LINKS` (`MultimapTableDefinition`, required wipe)
    4. `PATHS_BY_TAG` (`MultimapTableDefinition`, best-effort wipe)
    5. `PATHS_BY_FILE_CLASS` (`MultimapTableDefinition`, best-effort wipe)
    6. `TAGS_BY_PATH` (`MultimapTableDefinition`, best-effort wipe)
    7. `classes_by_path` (`FILE_CLASSES_BY_PATH`, `MultimapTableDefinition`, best-effort wipe)
    8. `cache_format` (`CACHE_FORMAT`, `TableDefinition`, best-effort wipe)
    *Correction to ADR-0005 and LSP map*: There is NO `lists` table. ADR-0005 proposed `lists` as a table, but list items were embedded directly into `Note` (`src/note/model.rs`). The LSP map (.scratch/md-pkm-lsp/map.md:53) claims "three redb tables", which is wrong.
  - `src/index/store.rs:37-38`: `const CACHE_FORMAT_VERSION: &[u8] = b"1";` is hardcoded.
  - `src/index/sort.rs:48-56`: `SortedByPath::assumed_sorted` asserts order using `debug_assert!`, which compiles to a no-op in release mode (`#[cfg(not(debug_assertions))]`).
  - *Config drift status (`MOVING (07/17)`)*: On `main` (`0c43149d`), config drift detection (ADR-0009) is unimplemented; `RefreshPlan::collect` (`src/index/refresh.rs:291-292`) only diffs file metadata. In the active worktree `.worktrees/config-digest-sections` (`49c5ac39`), Issue 07 has landed per-section digests (`ea636465`), which will be wired into index-owned `IndexFingerprint` and an 8th `meta` table in Issue 17.
  - *Fail-open semantics*: `IndexerService::refresh` (`src/index/service.rs:138-144` and `src/index/mod.rs:50-62`) handles incremental persist failures (`PersistFailed`) by logging `tracing::warn!` and returning the refreshed in-memory `WorkspaceIndex` via `failed.into_index()`. However, if the store fails to *open* initially (`src/index/service.rs:163`, `RefreshPlan::collect`), `prepare_pass` fails closed and returns `Err(IndexError::Store)`. Additionally, `IndexerService::refresh_store` (`service.rs:234`) propagates `PersistFailed` as an error because it has no in-memory store fallback.

---

### Claim 2: Two Query Paths (`run` vs `run_from_store`)

- **Status**: `VERIFIED`
- **Evidence**:
  - `src/query/service.rs:104-113` (`run`): Accepts `&Arc<WorkspaceIndex>` and `QueryBuilder`. Evaluates candidate rows in memory via `self.rows_for(mode, index, &source)` and runs plan transforms via `plan.run(rows)`.
  - `src/query/service.rs:129-169` (`run_from_store`): Accepts `&IndexStore` and `QueryBuilder`. Queries multimap tables via `SourceResolver::resolve(&source)`, batch-point-reads matching notes, files, and links from redb, constructs a transient `WorkspaceIndex` (`service.rs:162-166`), and executes `self.rows_for(mode, &index, &source)` and `plan.run(rows)`.
  - *Production Callers*:
    - `cli`: `list::List`, `table::Table`, and `task::Task` call `src/cli/mod.rs:365` (`refresh_query`) $\to$ `src/cli/mod.rs:484` (`service.run_from_store(store, builder)`). CLI production code **never** invokes `QueryService::run`.
    - `template`: `QueryOps::run` (`src/template/engine/query.rs:213`) invokes `self.service.run(&index, builder)` against `self.cached_index(state)`. Templates **never** invoke `run_from_store`.
  - *Code Sharing*: Both paths share row instantiation (`rows_for`, `src/query/service.rs:201`), row matching (`matched_file_rows`, `service.rs:318`), item expansion (`item_rows`, `service.rs:286`), and plan transformation (`ExecutionPlan::run`, `src/query/plan.rs:40`).
  - *Result Parity*:
    - `Not(...)`: Identical. `SourceResolver::resolve_expr` (`service.rs:395`) falls back to `self.store.paths_in_folder(Path::new(""))` (every indexed path), loads all notes into the transient index, and lets `matched_file_rows` filter out non-matching rows via `source.is_match(...)`.
    - Class expansion: Identical. Both call `self.resolve_source_classes(&mut source)` (`service.rs:110, 135`) before resolution and matching.
    - Folder sources: Identical. `SourceResolver::resolve_atom` (`service.rs:424-430`) fetches all paths in the vault and filters by regex `pattern.is_match(p)`. `matched_file_rows` applies the exact same predicate in memory.

---

### Claim 3: Query Row, Document Order Defect, and Lexer/AST Facts

- **Status**: `VERIFIED`
- **Evidence**:
  - `QueryRow` layout (`src/query/results.rs:44-50`):
    - `index: Arc<WorkspaceIndex>` (8 bytes)
    - `position: RowIndex` (8 bytes)
    - `flattened: Vec<(FieldPath, NoteFieldValue)>` (24 bytes)
    - `kind: RowType` (enum with `Page` or `List { item_idx: u32 }`, 8 bytes)
    - **Total size**: 48 bytes on 64-bit systems.
  - Every `QueryRow` clones `Arc<WorkspaceIndex>` on creation (`src/query/results.rs:57`, `src/query/service.rs:308`).
  - `cmp_document_order` defect (`src/query/results.rs:426-432`):

    ```rust
    pub(crate) fn cmp_document_order(&self, other: &Self) -> std::cmp::Ordering {
        self.position.cmp(&other.position)
    }
    ```

    - *Reasoning Trace*: The docstring at line 421 claims it orders "document order for note rows, and item order within their note for list and task rows". In reality, the body compares *only* `self.position` (the note's index). If two rows come from the same note (same `position`) but have different `item_idx` (e.g. task 0 vs task 1), `cmp_document_order` returns `Ordering::Equal`.
    - *Test Pinning*: **No test in the repository pins this function** or asserts order between multiple list/task rows within the same note. It is exposed to MiniJinja via `custom_cmp` (`src/template/engine/query.rs:539`).
  - `TextShape::classify` (`src/query/sort.rs:462`):
    - Called by sort: `KeyShape::from_text` (`src/query/sort.rs:394`).
    - Called by filter: `tokenize_literal` (`src/query/grammar/filter.rs:85`) and `comparison_rhs_value` (`src/query/grammar/filter.rs:242`).
  - Separate Lexers and ASTs:
    - `--from`: `SourceLexer` / `SourceToken` (`src/query/grammar/source.rs:358-450`), AST: `SourceSelector` / `SourceAtom` (`source.rs:53-233`).
    - `--where`: `FilterLexer` / `FilterToken` (`src/query/grammar/filter.rs:30-120`), AST: `FilterExpr` / `FilterAtom` (`filter.rs:125-235`).
    - Overlap: Both express tags (`#tag` vs `contains(tags, '#tag')`), classes (`@Class` vs `class = 'Class'`), and folders (`dir/` vs `startswith(file.path, 'dir/')`), but evaluate them at different pipeline stages (index candidate lookup vs row predicate evaluation).

---

### Claim 4: FileClassExpander Seam

- **Status**: `VERIFIED`
- **Evidence**:
  - `src/query/grammar/source.rs:353`: Trait definition:

    ```rust
    pub(crate) trait FileClassExpander: Send + Sync {
        fn expand(&self, classes: &[String], mode: &mut ClassExpansionMode);
    }
    ```

  - Trait Implementations: Exactly **2 implementations** exist across the repo:
    1. Production: `impl FileClassExpander for SchemaService` in root bridge module `src/file_class_expander.rs:15`.
    2. Test: `impl FileClassExpander for RecordingExpander` in `src/query/grammar/source.rs:1164`.
  - Trait Constructors: **0 constructor methods** on the trait itself.
  - Glue Construction:
    - `cli`: `src/cli/mod.rs:479-480`: constructs `Arc<SchemaService>` and passes it to `QueryService::with_class_expander`.
    - `template`: `src/template/engine/query.rs:132-133`: receives `Arc<SchemaService>` and passes it to `QueryService::with_class_expander`.
  - `query` module isolation: `query` has **zero dependencies on `schema`** across both production code and test modules (0 occurrences of the word `schema` in `src/query/`).

---

### Claim 5: Template Engine Composition, Caching, and Domain Logic

- **Status**: `PARTIAL`
- **Evidence**:
  - *Composition Root*: `TemplateEngine::new` (`src/template/engine.rs:109-168`) is a second composition root. It independently constructs:
    1. `SchemaService` (`engine.rs:132-134`)
    2. `IndexerService` (`engine.rs:135`)
    3. Three `QueryService` instances (`engine.rs:138-155`: page, list, task)
    `TemplateService` (`src/template/service.rs:38-43`) holds `config: &'a Config`, `loader: TemplateLoader`, `engine: TemplateEngine`, `provider: Arc<dyn DialogProvider>`.
  - *MiniJinja State Caches*:
    - `query.index_cache` (`INDEX_CACHE_KEY`, `src/template/engine/query.rs:100`): Caches `Arc<WorkspaceIndex>` per render. Refreshed via `indexer.refresh().map(Arc::new)` on first `.from()` call and stored via `cache::cached` (`cache.rs:61`).
    - `schema.service_cache` (`SCHEMA_SERVICE_CACHE_KEY`, `src/template/engine/schema.rs:90`): Caches `Arc<SchemaService>` in state temp-storage on `schema.get()` (`schema.rs:125`). Used by subsequent `.children()` and `.descendants()` calls (`schema.rs:243`).
  - *Row Cloning on Chained Calls (REFUTATION)*:
    - The survey claimed chained calls deep-clone rows. This is **REFUTED**.
    - In `src/template/engine/query.rs:398`: `let rows = self.as_ref().clone();` clones `QuerySet`.
    - `QuerySet` (`src/query/results.rs:464-468`) stores `base: Arc<Vec<QueryRow>>`.
    - Chained calls (`.filter()`, `.sort()`, `.limit()`, `.flatten()`) invoke `QuerySet::push` (`results.rs:521`), which clones only the `Arc` and appends transforms to `ExecutionPlan`. Rows are materialized lazily via `rows()` (`results.rs:483-490`).
  - *Root Template Re-Parsing*:
    - `TemplateEngine::render` (`src/template/engine.rs:190-193`) calls `self.env.template_from_named_str(name, source)?`. Root templates are re-parsed into MiniJinja bytecode on every single render call.
  - *Moment-Token Translation vs Anchor Math in `date.rs`*:
    - `src/template/engine/date.rs` is 2,486 lines.
    - Moment-token translation (`moment_to_strftime`, `MOMENT_TOKENS`, `ordinal_suffix`, lines 482-623: ~140 lines of logic) has no counterpart in `temporal`.
    - In contrast, calendar anchor math (`start_of`, `end_of`, `shift`, `with_weekday`, `DateDiff`) delegates directly to `temporal::DatePoint` methods (`src/temporal/point.rs:347, 364, 437`).
  - *Domain Logic in Template Modules*:
    - `template::engine::query`: Contains AST rewriting logic (`set_class_depth`, `src/template/engine/query.rs:499-515`) modifying `SourceAtom::Class` expansion depth, plus markdown formatting pipelines (`table_filter`, `list_filter`, `task_list_filter`).
    - `template::engine::schema`: Contains query AST synthesis logic (`file_field_source`, `src/template/engine/schema.rs:277-309`) translating schema file-field filter definitions into `SourceSelector` expressions.
    - Neither module is merely an adapter; both contain query domain logic.

---

### Claim 6: CLI Composition, Diagnostics, and I/O Hygiene

- **Status**: `PARTIAL`
- **Evidence**:
  - *Service Construction Locations*:
    - `ConfigService`: `src/cli/mod.rs:227` (`Cli::run(&ConfigService::new(), provider)`)
    - `IndexerService`: `src/cli/index.rs:41` (`IndexerService::from(config).rebuild()`), `src/cli/mod.rs:344` (`IndexerService::from(config).refresh_store()`)
    - `QueryService`: `src/cli/mod.rs:477` (`QueryService::new(...)`)
    - `SchemaService`: `src/cli/mod.rs:494` (`SchemaService::load_verbose(...)`)
    - `TemplateService`: `src/cli/template.rs:140` (`TemplateService::new(config, ...)`)
  - *`CliError` Variant Count (REFUTATION)*:
    - The cross-cutting survey claimed `CliError` has "52+ variants". This is **REFUTED**.
    - `src/cli/error.rs:58-231` defines **exactly 22 enum variants**: `CurrentDirectory`, `ConfigLoad`, `TrustTargetResolve`, `Trust`, `Untrust`, `TrustShow`, `TrustList`, `TrustClean`, `TrackedList`, `TrackedClean`, `InitPrompt`, `InitAlreadyInitialized`, `InitInvalidTemplateDir`, `InitScaffold`, `InitConfigWrite`, `Index`, `SchemaQuery`, `Query`, `TemplateInstantiate`, `NoTemplates`, `TemplatePicker`, `NoCommand`.
  - *Stdout / Stderr Writing*:
    - No non-CLI production code writes directly to stdout or stderr. All non-CLI logging routes through `tracing` macros (`warn!`, `debug!`, `info!`).
    - Direct `print!`, `println!`, and `eprintln!` calls occur exclusively in `src/main.rs:31` and `src/cli/*.rs` (with one test-only helper in `src/dialog/terminal.rs:126`).
  - *Filesystem Access in `normalize_source_input`*:
    - `src/cli/mod.rs:451-452, 463`: `candidate.is_file()` and `root.join(path).is_file()` perform filesystem `stat` syscalls during input argument normalization to auto-append `.md` and quote spaced paths.
  - *Filter String Synthesis in `cli/task.rs`*:
    - `src/cli/task.rs:46, 55, 58`: Synthesizes query filter strings via `format!("list.status_symbol == \"{}\"", ...)` and hardcoded strings `"list.completed == false"` / `"list.completed == true"`.

---

### Claim 7: Crate Topology, Test Seams, and Error Layering

- **Status**: `PARTIAL`
- **Evidence**:
  - *Mechanically Recomputed Top-Level Production Dependency Matrix*:

    ```text
    cli: config, dialog, index, query, schema, template
    config: dirs, dirtree, env_vars, field, file_tracker, hash, path, tag, task, temporal
    delimiter: (leaf: 0)
    dialog: (leaf: 0)
    dirs: (leaf: 0)
    dirtree: (leaf: 0)
    env_vars: (leaf: 0)
    field: temporal
    file: path, temporal
    file_class_expander: query, schema
    file_tracker: dirs, dirtree, hash
    hash: (leaf: 0)
    index: config, dirtree, env_vars, file, note, path, tag
    lexer: position
    note: config, delimiter, field, lexer, path, position, tag, task, temporal, yaml
    path: (leaf: 0)
    position: (leaf: 0)
    query: field, file, index, lexer, note, path, position, strsim, tag, task, temporal
    schema: dirtree, field, file, path, strsim
    strsim: (leaf: 0)
    tag: (leaf: 0)
    task: delimiter, temporal
    template: config, dialog, dirtree, field, file, index, note, path, query, schema, temporal, yaml
    temporal: lexer, position
    yaml: (leaf: 0)
    ```

    - *Survey Discrepancies*: `survey/cross-cutting.md` §5 omitted `config -> temporal` (via `DEFAULT_DATETIME_PATTERN`, `src/config/model.rs:925, 954`) and `index -> env_vars` (via `env_vars::is_ignored_dir`, `src/index/service.rs:328`).
  - *Cycles and Upward Edges*:
    - Cycles: Exactly **0 cycles** in production code. The topology is strictly a DAG.
    - Upward Edges: `note -> config` (`note/parser/input.rs`), `index -> config` (`index/service.rs`), `template -> query` (`template/engine/query.rs`), `template -> schema` (`template/engine/schema.rs`), `query -> index` (`query/service.rs`).
  - *`traces::testing` Surface*:
    - `src/lib.rs:184-209`: Contains **exactly 45 identifiers**, matching the comment at line 181.
    - Used extensively across `tests/integration/` (`persistence.rs`, `query.rs`, `refresh.rs`, `template_render.rs`) and all 11 `benches/`.
    - *Subsystem composition limitation*: Public crate root (`src/lib.rs:1-176`) exports only leaf domain models (`Note`, `ListItem`, `FileMeta`, `Tag`, `DialogProvider`, `temporal`); all services (`ConfigService`, `IndexerService`, `QueryService`, `SchemaService`, `TemplateService`) and index structures are internal, requiring `test-utils` for any external composition.
    - `TestProject` (`src/lib.rs:279-544`) is **266 lines** of code (the survey's claim of "600+ lines" conflated `TestProject` with the entire `test_support` module, which is 669 lines).
  - *Error Types Inventory & 6-Layer Chain (REFUTATION)*:
    - 42 total error types verified across `src/`.
    - The survey's "6-layer error propagation example" (`FieldNameError` $\to$ `FieldAddressError` $\to$ `SchemaFieldBuilderError` $\to$ `SchemaError` $\to$ `TemplateError::SchemaLoad` $\to$ `CliError::TemplateRender`) is **REFUTED**:
      1. `FieldAddressError` is never converted to `SchemaFieldBuilderError`; it fails during serde TOML deserialization into `SchemaFileError::Toml`.
      2. `CliError::TemplateRender` does not exist; the CLI variant is `CliError::TemplateInstantiate`.
  - *`GLOSSARY-MAP.md` Drift*:
    - Mermaid diagram (`GLOSSARY-MAP.md:33-52`) omits the direct edge `CLI --> Schema` (active in code at `src/cli/mod.rs:494`).
    - The cross-cutting survey cited non-existent files `src/cli/query.rs` and `src/cli/schema.rs` to prove direct coupling; both integrations actually reside in `src/cli/mod.rs`.

---

### Claim 8: Persisted-Format Coupling & redb Serialized Types

- **Status**: `VERIFIED`
- **Evidence**:
  - *Serialized Types*:
    - Only **2 types** are serialized as structured binary payloads via `postcard`:
      1. `FileMeta` (`src/file.rs:16`) into table `FILES`.
      2. `Note` (`src/note/model.rs:25`) into table `NOTES`.
    - All other redb tables (`LINKS`, `PATHS_BY_TAG`, `PATHS_BY_FILE_CLASS`, `TAGS_BY_PATH`, `FILE_CLASSES_BY_PATH`, `CACHE_FORMAT`) store raw UTF-8 path or name bytes (`&[u8]`).
  - *Types Carrying Serde Solely for Persistence*:
    - 16 types in `src/note/`, `src/file.rs`, `src/task.rs`, and `src/position.rs` derive `Serialize, Deserialize` exclusively for redb row persistence: `FileMeta`, `FileFormat`, `Note`, `Heading`, `ListItem`, `ListItemType`, `ListText`, `LinkTarget`, `LinkType`, `NoteFrontmatter`, `TaskDate`, `TaskDateType`, `TaskPriority`, `TaskStatus`, `TaskStatusSymbol`, `TaskStatusType`, `SourceLine`.
  - *Format Guards Beyond `cache_format`*:
    - **None exist**. Rows have no framing magic bytes, type version tags, or checksums.
    - If `postcard` encoding drifts without bumping `CACHE_FORMAT_VERSION` (`b"1"`), decoding errors cause `IndexStore::read_batch` (`src/index/store.rs:901-909`) to log a warning and silently drop the corrupted note from queries.

---

### What was skipped or not checked

- Did not modify production code, tests, benches, or documentation.
- Did not run cargo check, test, bench, or mise tasks.
- Verified in-flight branch `config-digest-sections` read-only without modifying `.worktrees/`.
- Risk: In-flight Issue 17 will alter table counts and drift detection once landed on main.
