# Architecture Survey: Whole-Crate Cross-Cutting Structure

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

## 1. Scope & Size
- **Scope**: Whole-crate topology, module dependency edges, error hierarchies, export tiers in `src/lib.rs`, testing/bench seams, glossary/documentation fidelity, Cargo dependencies, and divergence register alignment.
- **Top-level modules (25)**: `cli`, `config`, `delimiter`, `dialog`, `dirs`, `dirtree`, `env_vars`, `field`, `file`, `file_class_expander`, `file_tracker`, `hash`, `index`, `lexer`, `note`, `path`, `position`, `query`, `schema`, `strsim`, `tag`, `task`, `template`, `temporal`, `yaml`, plus crate roots `lib.rs` and `main.rs`.
- **SLoC**: ~92,521 total lines across 134 source files in `src/`.
  - Heavyweight domain modules: `template` (13,377), `schema` (12,129), `query` (11,429), `temporal` (10,308), `note` (10,171), `index` (9,054), `cli` (8,175), `config` (5,144).
  - Primitives/utilities: `task` (1,919), `field` (1,832), `file_tracker` (1,178), `dialog` (1,147), `lib` (900), `dirtree` (824), `position` (799), `path` (788), `file` (667), `lexer` (657), `tag` (600), `delimiter` (498), `hash` (372), `dirs` (219), `file_class_expander` (87), `strsim` (81), `main` (69), `env_vars` (67), `yaml` (30).
- **Surface**: Binary crate + library facade. `src/lib.rs` exposes only a minimal public API (14 types/functions always pub: `Note`, `ListItem`, `TaskListItem`, `Tag`, `SourceLine`, `TaskStatus`, `FileMeta`, `DialogProvider`, `parse_markdown`, etc.), while relegating 45 internal domain structures to `traces::testing` under `#[cfg(any(test, feature = "test-utils"))]`.

---

## 2. Responsibilities
- **Composition Root & Entrypoint**: `src/main.rs`, `src/cli/` — CLI parsing via `clap`, command dispatch, mapping domain errors into user-facing diagnostics (`CliError`).
- **Workspace State & Trust Management**: `src/config/`, `src/file_tracker.rs`, `src/dirs.rs`, `src/env_vars.rs` — TOML config discovery, workspace root resolution, cryptographic trust tracking via BLAKE3 hashes of config files.
- **Data Primitives & Parsing**:
  - `src/note/`: Markdown AST extraction (via `pulldown-cmark`), YAML frontmatter extraction, inline field and wikilink parsing.
  - `src/field.rs`: Case-insensitive field keys, field values, and dynamic coercions.
  - `src/temporal/`: Date, DateTime, and Duration parsing via `jiff`, `speedate`, and custom grammars.
  - `src/task.rs`: Task status states, priority symbols, and emoji/bracket date recognition.
  - `src/tag.rs`: Validated `#tag/subtag` identifiers.
  - `src/path.rs`: Confinement to project roots via `strict-path`.
- **Indexing & Graph Cache**: `src/index/` — Filesystem scanning, note parsing, inlink reference graph construction, and ACID persistence via `redb`.
- **Query Engine**: `src/query/` — DSL lexing (`logos`), AST parsing, row projection, filtering, sorting, grouping, and rendering (`table`, `list`, `task_list`).
- **Schema Engine**: `src/schema/` — Parsing `.traces/schemas/*.toml`, validating fields, calculating `extends` DAG linearization, resolving `$ref` pointers.
- **Template Instantiation Engine**: `src/template/` — Custom minijinja filters/functions (`ui`, `date`, `file`, `schema`, `query`), interactive prompts (`dialog`), file write policy.
- **Decoupling Glue**: `src/file_class_expander.rs` — Mediates query `@Class*` expansion into schema lookups without direct query-to-schema coupling.

---

## 3. Invariants
- **Root Confinement**: All paths accessed for reading notes, writing templates, or querying MUST be strictly within the workspace project root. Enforced at runtime by `SafeRelativePath` in `src/path.rs:36` using `strict_path::PathBoundary`.
- **Single Process / Sync Engine**: Entire codebase is single-threaded synchronous (except rayon parallelization in index scanning and query execution). No `tokio` runtime.
- **Acyclic Module Graph in Production**: No circular dependencies exist between any top-level modules at compile time.
- **Cryptographic Trust Gate**: Configuration files (`.traces/config.toml`) cannot be loaded or executed without prior user consent recorded in `~/.local/share/traces/trusted.db` (enforced in `src/config/trust.rs`).
- **Frozen Testing Export Tier**: Exactly 45 identifiers in `src/lib.rs:181` are exported under `traces::testing`. Any leak or alteration is pinned by tests in `src/lib.rs:746-900`.
- **Dual Duration Regime**: Duration equality/hashing compares total seconds (`1h 30m == 90m`), but duration arithmetic applies calendar units to local calendar clocks (`src/temporal/duration.rs`, divergence register Entry 3).

---

## 4. State & Lifecycle
- **Process Lifecycle**: Ephemeral CLI commands (`traces list`, `traces template`, `traces index`). Each invocation discovers the workspace root, initializes `ConfigService`, loads `redb` storage, executes, and exits.
- **Persistence**:
  - Workspace index: `.traces/index.redb` (managed by `src/index/store.rs`). Tables: `META`, `FILES`, `INLINKS`, `NOTES`.
  - User-global state: Tracked paths and trusted project hashes stored in SQLite/binary files under XDG state dirs (`src/file_tracker.rs`).
- **Concurrency**:
  - `rayon`: Used in `src/index/service.rs` for parallel Markdown parsing and in `src/query/service.rs` for parallel row projection.
  - Locks: `redb` manages its own internal database locking; no user-space `Arc<RwLock>` or channels across modules.

---

## 5. Dependency Edges & Topology

### Module Dependency Matrix (Production Code Edges)
Generated by mechanical grep over `src/**` excluding comments and test blocks (`#[cfg(test)]`):

| Source Module | Outbound Dependencies (Targets: Reference Count) |
|---|---|
| **cli** | `config:22`, `dialog:5`, `index:2`, `query:6`, `schema:1`, `template:9` |
| **config** | `dirs:3`, `dirtree:1`, `env_vars:1`, `field:4`, `file_tracker:5`, `hash:5`, `path:4`, `tag:2`, `task:5` |
| **delimiter** | *(leaf: 0)* |
| **dialog** | *(leaf: 0)* |
| **dirs** | *(leaf: 0)* |
| **dirtree** | *(leaf: 0)* |
| **env_vars** | *(leaf: 0)* |
| **field** | `temporal:3` |
| **file** | `path:3`, `temporal:2` |
| **file_class_expander** | `query:2`, `schema:1` |
| **file_tracker** | `dirs:1`, `dirtree:2`, `hash:1` |
| **hash** | *(leaf: 0)* |
| **index** | `config:3`, `dirtree:3`, `file:9`, `note:8`, `path:4`, `tag:1` |
| **lexer** | `position:1` |
| **note** | `config:2`, `delimiter:3`, `field:12`, `lexer:1`, `path:2`, `position:8`, `tag:9`, `task:12`, `temporal:8`, `yaml:1` |
| **path** | *(leaf: 0)* |
| **position** | *(leaf: 0)* |
| **query** | `field:2`, `file:1`, `index:4`, `lexer:9`, `note:16`, `path:10`, `position:3`, `strsim:1`, `tag:2`, `task:3`, `temporal:23` |
| **schema** | `dirtree:2`, `field:15`, `file:2`, `path:2`, `strsim:1` |
| **strsim** | *(leaf: 0)* |
| **tag** | *(leaf: 0)* |
| **task** | `delimiter:1`, `temporal:1` |
| **template** | `config:3`, `dialog:7`, `dirtree:2`, `field:1`, `file:2`, `index:4`, `note:1`, `path:7`, `query:19`, `schema:8`, `temporal:25`, `yaml:1` |
| **temporal** | `lexer:2`, `position:2` |
| **yaml** | *(leaf: 0)* |

### Fan-Out Ranking (High to Low)
1. `template` (out = 12 modules)
2. `query` (out = 11 modules)
3. `note` (out = 10 modules)
4. `config` (out = 9 modules)
5. `cli` (out = 6 modules)
6. `index` (out = 6 modules)
7. `schema` (out = 5 modules)
8. `file_tracker` (out = 3 modules)
9. `file_class_expander`, `temporal`, `task`, `file` (out = 2 modules each)
10. `lexer`, `field` (out = 1 module each)

### Fan-In Ranking (High to Low)
1. `path` (in = 7 modules: `config`, `file`, `index`, `note`, `query`, `schema`, `template`)
2. `temporal` (in = 6 modules: `field`, `file`, `note`, `query`, `task`, `template`)
3. `dirtree` (in = 5 modules: `config`, `file_tracker`, `index`, `schema`, `template`)
4. `field` (in = 5 modules: `config`, `note`, `query`, `schema`, `template`)
5. `config` (in = 4 modules: `cli`, `index`, `note`, `template`)
6. `file` (in = 4 modules: `index`, `query`, `schema`, `template`)
7. `position` (in = 4 modules: `lexer`, `note`, `query`, `temporal`)
8. `tag` (in = 4 modules: `config`, `index`, `note`, `query`)
9. `index` (in = 3 modules: `cli`, `query`, `template`)
10. `lexer` (in = 3 modules: `note`, `query`, `temporal`)
11. `note` (in = 3 modules: `index`, `query`, `template`)
12. `query` (in = 3 modules: `cli`, `file_class_expander`, `template`)
13. `schema` (in = 3 modules: `cli`, `file_class_expander`, `template`)
14. `task` (in = 3 modules: `config`, `note`, `query`)
15. `dialog` (in = 2 modules: `cli`, `template`)
16. `dirs` (in = 2 modules: `config`, `file_tracker`)
17. `hash` (in = 2 modules: `config`, `file_tracker`)
18. `strsim` (in = 2 modules: `query`, `schema`)
19. `yaml` (in = 2 modules: `note`, `template`)
20. `delimiter` (in = 2 modules: `note`, `task`)
21. `env_vars` (in = 1 module: `config`)
22. `file_tracker` (in = 1 module: `config`)
23. `template` (in = 1 module: `cli`)
24. `cli`, `file_class_expander` (in = 0)

### Upward Edges & Architectural Breaches
1. **`note -> config`** (`src/note/parser/input.rs:9`): `MarkdownParserInput` takes `&FrontmatterConfig` and `&TaskConfig`. Lower-level AST parser directly depends on the high-level configuration domain model.
2. **`index -> config`** (`src/index/service.rs:21`): `IndexerService` takes `&Config` and `&FrontmatterConfig`.
3. **`template -> query`** (`src/template/engine/query.rs:76`): Template engine directly embeds and invokes `QueryBuilder` and `QueryService`.
4. **`template -> schema`** (`src/template/engine/schema.rs:43`): Template engine directly embeds `SchemaService`.

### Cycle Analysis
- **Module Granularity**: **0 cycles** in production code. The DAG is strictly acyclic.
- **Type Granularity**: No mutual recursive type definitions across module boundaries.
- **Test-Only Inversions**:
  - `index -> query` (`src/index/service.rs:376`): Index service tests import `QueryService` to verify that indexed data is queryable.
  - `schema -> query` (`src/schema/service.rs:343`): Schema tests import query symbols.
  - `cli -> task` / `cli -> hash` / `cli -> dirtree`: CLI unit tests bypass the command boundary and test against internal domain structures.

---

## 6. Interfaces & Seams
- **Deep Seams**:
  - `DialogProvider` (`src/dialog/mod.rs`): Narrow trait (`prompt_text`, `prompt_select`, `prompt_confirm`) hiding terminal I/O (`inquire`) or test presets. Used cleanly by CLI and template engine.
  - `redb Store` (`src/index/store.rs`): Fully encapsulates database transactions, table handles, and serialization behind `IndexStore::read` and `IndexStore::write`.
  - `StrictPath` / `SafeRelativePath` (`src/path.rs`): Deep abstraction wrapping path boundaries and normalizing separators.
- **Shallow / Leaky Seams**:
  - `FileClassExpander` (`src/file_class_expander.rs`): Declared in GLOSSARY-MAP as decoupling `Query` from `Schema`. In reality, it is an 87-line pass-through struct that implements `query::FileClassExpander` by taking `&SchemaService` and calling `schema.expand_classes(...)`. Both `cli` and `template` must instantiate this glue manually.
  - `IndexerService` (`src/index/service.rs`): Thin orchestrator around `redb` and `Note::parse`.
  - `QueryService` (`src/query/service.rs`): Passthrough wrapper that builds `QuerySet` over `WorkspaceIndex`.

---

## 7. Algorithms, Data Structures & Performance Hotspots
- **Repeated Serialization / Codecs**:
  - `postcard`: Used in `src/index/store.rs` for compact binary serialization of `Note`, `FileMeta`, and inlinks into `redb`.
  - `noyalib`: YAML parser used in `src/yaml.rs` and `src/note/metadata.rs`. YAML values are parsed into `noyalib::Value`, then immediately mapped into `FieldValueRef` and `IndexMap<FieldKey, FieldValue>`.
- **String Hashing & Canonicalization**:
  - `FieldKey` (`src/field.rs`): Uses case-insensitive comparison (`unicase` semantics via manual lowercase ASCII/Unicode). Implements `indexmap::Equivalent<FieldKey>` via `FieldKeyRef<'a>` to avoid heap allocation during lookup in `IndexMap<FieldKey, FieldValue>`.
- **Trie Index for Inlinks**:
  - `src/index/trie.rs`: Implements `BaseNameIndex` using an `indextree::Arena` and `rustc_hash::FxHashMap` for fast folder-distance basename resolution of wikilinks.
- **Allocation Hotspots**:
  - `QueryRow` evaluation: Materializing rows into `QueryRow` clones `NoteFieldValue` unless carefully projected.
  - Template AST rendering: `minijinja` constructs dynamic value environments, converting `FieldValue` to `minijinja::Value` on each helper call.

---

## 8. Error Topology

### Error Types Inventory
The codebase defines **42 error types** via `thiserror::Error` and `miette::Diagnostic`:
- **CLI**: `CliError` (central diagnostic hub with 52+ variants), `ConfigScaffoldError`.
- **Config**: `ConfigError`, `ConfigDiscoveryError`, `ConfigReadError`, `ConfigTrustError`, `ConfigValueError`.
- **Dialog**: `DialogError`.
- **Dirtree**: `DirTreeError`.
- **Field**: `FieldNameError`, `FieldKeyError`.
- **File**: `FileNameError`.
- **File Tracker**: `FilePathTrackerError`.
- **Hash**: `HashError`.
- **Index**: `IndexError`, `StoreError`.
- **Lexer**: `LexError`.
- **Note**: `FrontmatterParseError`.
- **Path**: `PathError`.
- **Position**: `PositionError`.
- **Query**: `QueryError`, `QueryBuilderError`, `QuerySyntaxError`, `FieldPathError`.
- **Schema**: `SchemaError`, `SchemaServiceError`, `SchemaFileError`, `SchemaNameError`, `RawFieldDefError`, `SchemaFieldBuilderError`, `SchemaFieldParserError`, `SchemaSelectFieldValueError`, `SchemaSelectFieldEntryError`, `SchemaSelectFieldFileError`, `FieldAddressError`.
- **Tag**: `TagError`.
- **Task**: `TaskError`.
- **Template**: `TemplateError`, `TemplatePathError`.
- **Temporal**: `DateError`, `DurationError`, `CalendarError`.

### Error Layering & Conversions
- **Error Propagation Depth**: Primitive errors cross up to **6 layers** of nesting before reaching `CliError`:
  1. `FieldNameError` (`src/field.rs`)
  2. $\to$ `FieldAddressError` (`src/schema/fields/address.rs`)
  3. $\to$ `SchemaFieldBuilderError` (`src/schema/fields/error.rs`)
  4. $\to$ `SchemaError` (`src/schema/error.rs`)
  5. $\to$ `TemplateError::SchemaLoad` (`src/template/error.rs`)
  6. $\to$ `CliError::TemplateRender` (`src/cli/error.rs`)
- **Context Loss**:
  - Several errors convert via `#[from] Box<serde_json::Error>` or `#[from] std::io::Error`, discarding precise span and semantic file context.
  - `TemplateError` erases specific minijinja internal errors into `TemplateError::Render(String)`.

---

## 9. Export Tiers in `src/lib.rs` & Test Seams

### The Export Tiers
1. **Always Public (`pub`)**: 14 items (`Note`, `ListItem`, `TaskListItem`, `Tag`, `SourceLine`, `TaskStatus`, `FileMeta`, `DialogProvider`, `parse_markdown`, etc.).
2. **Crate-Internal (`pub(crate)`)**: ~58 items across `config`, `field`, `file_tracker`, `index`, `query`, `schema`, `template`.
3. **Frozen Test Tier (`traces::testing`)**: Exactly 45 identifiers exported under `#[cfg(any(test, feature = "test-utils"))]`:
   - Config: `Config`, `ConfigService`, `TaskConfig`, `TrustRequest`.
   - File & Hash: `FileFormat`, `Blake3Digest`, `Blake3PathHash`.
   - Index: `FileEntry`, `IndexerService`, `InlinkMap`, `RefreshReport`, `WorkspaceIndex`.
   - Path: `path::codec as path_codec`.
   - Query: `QueryBuilder`, `QueryBuilderError`, `QueryResult`, `QueryRow`, `QueryService`, `QuerySet`, `SourceSelector`.
   - Schema: `Schema`, `SchemaFieldDef`, `SchemaService`, `SchemaServiceError`.
   - Template: `CommitPolicy`, `RenderFailureKind`, `TemplatePathInput`, `TemplateService`, `WriteMode`, `WriteOutcome`.
   - Temporal: `ZoneGuard`.
   - Fixtures: `TestProject`, `build_test_index`, `create_trusted_project`, `fixture_service`, `parse_note`, `parse_note_str`, `parse_tag`, `resolve_safe_path`, `write_note`, `write_schema`, `write_template`, and directory constants.

### Structural Awkwardness of Testing
- **Evidence**:
  - Integration tests in `tests/integration/` and benchmarks in `benches/` **cannot function without `traces::testing`**. Cargo manifest specifies `required-features = ["test-utils"]` for all 11 benchmarks and `integration`.
  - `TestProject` in `src/lib.rs:279` is 600+ lines of fixture orchestration solely to construct mock services, temporary directories, and in-memory indexes because subsystems cannot be easily composed or tested from public types alone.
  - Crate visibility tests (`src/lib.rs:69-107`) use compile-fail doctests specifically to prevent internal types from leaking to public consumers, while needing a dedicated side-door (`testing`) to make integration testing possible.

---

## 10. External Crates & Functional Overlaps

Analysis of `Cargo.toml` dependencies vs internal implementations:
1. **`strsim` crate vs `src/strsim.rs`**:
   - `src/strsim.rs` is an 81-line wrapper around `strsim::levenshtein`. Redundant abstraction level; could be a simple utility function in `field` or `schema`.
2. **Parser overlapping**:
   - `logos`: Used in `src/lexer.rs`, `src/query/grammar/lexer.rs`, and `src/temporal/lexer.rs`.
   - `winnow`: Used in `src/temporal/lexer.rs` for combinator parsing alongside logos.
   - `speedate`: Used in `src/temporal/date.rs:296` for fast RFC 3339 byte parsing.
   - `pulldown-cmark`: Used in `src/note/parser.rs` for Markdown.
   - *Verdict*: 4 distinct parsing engines across the crate (`logos`, `winnow`, `speedate`, `pulldown-cmark`).
3. **`noyalib` vs `serde_json` vs `toml`**:
   - `noyalib` is used for YAML 1.2 frontmatter parsing and YAML template serialization (`src/yaml.rs`, `src/template/engine/yaml.rs`).
   - `serde_json` is used for JSON select values (`src/schema/fields/select.rs`) and schema field addresses.
   - `toml` is used for config and schema definition parsing.
   - `field.rs` has separate conversion paths for `From<noyalib::Value>` and `TryFrom<serde_json::Value>`, producing duplicate AST mapping logic.
4. **`indexmap` vs `rustc-hash`**:
   - `indexmap` is used everywhere preservation of insertion order or deterministic iteration is needed (`IndexMap<FieldKey, FieldValue>`, schemas, notes).
   - `rustc-hash` (`FxHashMap`, `FxBuildHasher`) is used in `src/index/inlinks.rs`, `src/index/store.rs`, and `src/index/trie.rs` for raw speed.
   - Justified bifurcation (order-sensitive frontmatter/schema vs performance-critical index internals).
5. **`strict-path` vs `src/path.rs`**:
   - `src/path.rs` wraps `strict-path` types (`PathBoundary`, `StrictPathError`) and defines `SafeRelativePath`. Good usage of an external invariant crate.
6. **`walkdir` vs `src/dirtree.rs`**:
   - `src/dirtree.rs` is an 824-line wrapper around `walkdir::WalkDir` that wraps `DirEntry` in `DirNode` and converts errors to `DirTreeError`. High weight for wrapping an iterator.
7. **`blake3` vs `src/hash.rs`**:
   - `src/hash.rs` provides `Blake3Digest` and `Blake3Hasher` newtypes around `blake3::Hash` and `blake3::Hasher`. Appropriate domain newtyping.

---

## 11. Documentation & Glossary Drift

Verified against actual source code:
1. **GLOSSARY-MAP.md Mermaid Diagram**:
   - *Claim*: `CLI --> Template`, `Template --> Query`, `Template --> Schema`.
   - *Reality*: CLI *also* depends directly on `Query` and `Schema` (`src/cli/query.rs`, `src/cli/schema.rs`).
   - *Claim*: `Query -. FileClassExpander .-> Schema` (Query decoupled from Schema).
   - *Reality*: While Query does not import Schema, `file_class_expander.rs` is an awkward crate-level glue module that imports both `query` and `schema`, and `Template` imports both `query` and `schema` directly, bypassing any decoupling.
2. **src/GLOSSARY.md Duration Definition**:
   - *Claim*: "An elapsed duration carrying both its total seconds magnitude and written source components... Compares and hashes canonically by total seconds, so 1h 30m and 90m are equal."
   - *Reality*: Duration redesign (Refinement 3, divergence register R4/R5) removed `DurationSeconds` and source component preservation. Durations are now integer nanoseconds (`DurationValue`), rendered canonically via `Display`, not source-preserving.
3. **src/GLOSSARY.md Navigation & Resolution**:
   - *Claim*: Defines `Occurrence`, `NavTarget`, `Resolution`, `Ambiguity` under Core Navigation.
   - *Reality*: These types do not exist in `src/` yet; they are design artifacts from `.scratch/md-pkm-lsp/map.md`. Documented as Core concepts in `src/GLOSSARY.md` despite being unimplemented in the production crate.

---

## 12. Divergence Register Summary (`docs/divergence-register.md`)
- **Temporal Behavior Changes (`R1` to `R18`)**:
  - `R1`: One-step calendar application (largest unit first).
  - `R2`: Repeated units in durations are parse errors (`12h 12h`).
  - `R3`/`R4`/`R5`: Duration grammar narrowed to Jiff friendly/ISO; canonical spelling output; integer nanoseconds backing.
  - `R6`: Rejecting dates without zero-padding (`2026-1-1` rejected).
  - `R7`: Calendar-aware day differences across DST transitions.
  - `R8`: Fold/gap handling pinned.
  - `R12`: `ZoneGuard` thread-local override replaces `unsafe` `TzGuard`.
  - `R18`: RFC 3339 leap seconds rejected.
- **External Parity Divergences**:
  - Entry 1: UTC storage of instants (Dataview/Templater store local offsets).
  - Entry 2: Space-separated datetime strings accepted in frontmatter.
  - Entry 3: Dual regime for duration identity (Luxon parity: `1d == 24h` for equality, but calendar-aware for date addition).
  - Entry 4: Explicit `NoteFieldType` sort ranking (`Null < Bool < Number < String < Date < DateTime < Duration < Link < List < Object`).

---

## 13. Candidate Architectural Questions

### 1. Leaf-to-Domain Inversion (`note -> config`)
- **Claim**: Lower-level primitives (`note`, `task`, `field`) should not import high-level application configuration (`config`). `MarkdownParserInput` taking `FrontmatterConfig` and `TaskConfig` couples AST parsing to user configuration models.
- **Evidence**: `src/note/parser/input.rs:9` imports `crate::config::{FrontmatterConfig, TaskConfig}`.
- **Counter-argument**: Passing configuration structs directly avoids destructuring 10+ parser options at every callsite. The parser needs to know delimiters and custom task status symbols.
- **Decision Open**: Should `note` define its own parameter options struct, or should `FrontmatterConfig` live in a shared primitives layer below both `config` and `note`?
- **Touches**: `note`, `config`, `cli`, `index`.

### 2. Testing Seam Bifurcation (`traces::testing` & `test-utils`)
- **Claim**: The 45-identifier `traces::testing` module and `test-utils` feature indicate that the crate's public API is too narrow and uncomposable, forcing integration tests and benchmarks to rely on pseudo-private internals.
- **Evidence**: All 11 benchmarks in `Cargo.toml:56-115` and `tests/integration` require `test-utils`. `TestProject` in `src/lib.rs` is 600 lines of fixture code.
- **Counter-argument**: A CLI tool's public API is the CLI interface itself; keeping library APIs private or `pub(crate)` prevents external consumers from depending on unstable PKM data structures while allowing internal optimization.
- **Decision Open**: Should `traces` be split into domain crates (`traces-core`, `traces-index`, `traces-query`, `traces-cli`), making domain interfaces naturally public across workspace crates without synthetic `testing` gates?
- **Touches**: `lib.rs`, `Cargo.toml`, all integration tests and benchmarks.

### 3. God-Module Template Engine (`template` Fan-Out)
- **Claim**: `src/template/` has the highest fan-out in the repository (imports 12 modules), coupling template rendering directly to `query`, `schema`, `dialog`, `dirtree`, `index`, and `config`.
- **Evidence**: `template` depends on almost every domain service in the codebase, acting as an alternative composition root rather than a rendering engine.
- **Counter-argument**: Minijinja templates expose namespaces for all PKM operations (`tp.query`, `tp.schema`, `tp.ui`, `tp.file`). It is inherently an integration environment for personal workflows.
- **Decision Open**: Should the template engine be decoupled into a pure rendering spine with an injected context/provider interface (e.g., dynamic capability registration), or kept as a unified facade?
- **Touches**: `template`, `query`, `schema`, `dialog`, `index`.

### 4. Fragmented Parsing Stack (4 Independent Parser Engines)
- **Claim**: Using `logos`, `winnow`, `speedate`, and `pulldown-cmark` across different modules creates redundant parsing mechanisms, increases binary size, and complicates diagnostic span reporting.
- **Evidence**: `src/temporal/lexer.rs` mixes `logos` with `winnow`; `src/temporal/date.rs` uses `speedate`; `src/query/` uses `logos`; `src/note/` uses `pulldown-cmark`.
- **Counter-argument**: Each engine is best-in-class for its exact niche: `logos` for tokenization, `winnow` for combinator grammar, `speedate` for zero-allocation date validation, `pulldown-cmark` for CommonMark conformance.
- **Decision Open**: Can temporal and query grammars be unified under a single parsing framework (e.g. `winnow` throughout), simplifying error/span models for the planned PKM LSP?
- **Touches**: `temporal`, `query`, `lexer`, `Cargo.toml`.

### 5. `FileClassExpander` Decoupling Illusion
- **Claim**: The `FileClassExpander` seam creates architectural friction without real decoupling benefits.
- **Evidence**: Only 87 lines (`src/file_class_expander.rs`); both `cli` and `template` must manually instantiate `SchemaService`, wrap it in `FileClassExpander`, and pass it to `QueryService`.
- **Counter-argument**: It strictly prevents `query` from having a compile-time dependency on `schema`, allowing queries to run on basic files/tags without loading the schema engine.
- **Decision Open**: Keep as-is (strict DAG isolation between query and schema) or formalize as part of a unified `WorkspaceContext`?
- **Touches**: `query`, `schema`, `file_class_expander`, `cli`, `template`.
