# Architecture Survey: Schema Subsystem

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

## 1. Scope & Size
- **Modules / Files Covered**:
  - `src/schema/mod.rs` (52 lines)
  - `src/schema/model.rs` (328 lines)
  - `src/schema/name.rs` (306 lines)
  - `src/schema/raw.rs` (414 lines)
  - `src/schema/builder.rs` (1700 lines)
  - `src/schema/service.rs` (982 lines)
  - `src/schema/error.rs` (488 lines)
  - `src/schema/graph.rs` (509 lines)
  - `src/schema/graph/adjacency.rs` (734 lines)
  - `src/schema/graph/builder.rs` (203 lines)
  - `src/schema/graph/cycle.rs` (395 lines)
  - `src/schema/fields.rs` (464 lines)
  - `src/schema/fields/builder.rs` (886 lines)
  - `src/schema/fields/parser.rs` (918 lines)
  - `src/schema/fields/error.rs` (655 lines)
  - `src/schema/fields/address.rs` (273 lines)
  - `src/schema/fields/select.rs` (2032 lines)
  - `src/schema/fields/file.rs` (298 lines)
  - `src/schema/fields/number.rs` (340 lines)
  - `src/schema/fields/date.rs` (152 lines)
- **SLoC**:
  - Total lines: 12,129 across 20 files.
  - Production code: ~4,757 lines (~39.2%).
  - Test code: ~7,372 lines (~60.8%).
- **Surface**:
  - `pub` surface (`src/schema/mod.rs:35-45`):
    - `Schema` (`pub struct`)
    - `SchemaService` (`pub struct`)
    - `SchemaFieldDef` (re-exported under `#[cfg(any(test, feature = "test-utils"))]`)
    - `SchemaServiceError` (re-exported under `#[cfg(any(test, feature = "test-utils"))]`)
  - `pub(crate)` surface:
    - `SchemaError`, `SchemaFileError` (test-only re-export)
    - `SchemaFileFieldRef`, `SchemaSelectFieldEntry`
    - `SchemaName`, `SchemaNameRef`
    - `RawSchema`, `RawSchemaFieldDef`, `RawSchemaFieldSource`, `RawSchemaFieldType`
    - `warn_schema_construction_diagnostics`
    - `GLOBAL_SCHEMA_NAME` constant (`"global"`)

---

## 2. Responsibilities
Verified responsibilities and owning types/functions:
1. **On-disk Schema TOML parsing & raw deserialization**:
   - `RawSchema`, `RawSchemaFieldDef`, `RawSchemaFieldSource`, `RawSchemaFieldType`, `FieldAddress` in `src/schema/raw.rs:28-256`, `src/schema/fields/address.rs:16-79`.
   - Deserializes `.traces/schemas/<name>.toml` using `serde` with strict `#[serde(deny_unknown_fields)]`.
   - Unparsed type-specific attributes are captured into an untyped `IndexMap<String, FieldValue>` bag in `RawSchemaFieldDef.options`.
2. **Inheritance DAG construction, cycle detection & Kahn linearization**:
   - `SchemaAdjacency` in `src/schema/graph/adjacency.rs:28-87` builds CSR (compressed sparse row) adjacency over dense index integers `DenseIndex` (`u32`), filtering out `global`.
   - `SchemaGraphBuilder` & `SchemaGraph` in `src/schema/graph/builder.rs:21-65`, `src/schema/graph.rs:48-106`.
   - `CycleDetector` in `src/schema/graph/cycle.rs:18-80` performs iterative Tarjan/SCC-style DFS over the Kahn-unvisited subgraph to identify cyclic schemas deterministically in declaration order.
3. **Multi-parent schema field merging & $ref resolution**:
   - `SchemaBuilder` in `src/schema/builder.rs:58-350` drives resolution: resolves `global` first, iterates through topologically sorted schemas, merges parent fields (first-listed wins), applies `excludes`, and invokes field builder.
   - Computes post-merge hierarchy sets (`children`, `descendants`), filtering out branches broken by parent resolution failure (`builder.rs:332-350`).
4. **Per-field validation & parse-don't-validate transformation**:
   - `SchemaFieldBuilder` in `src/schema/fields/builder.rs:74-246`.
   - Evaluates `$ref` bounds (must target `global` or transitive `extends` ancestors).
   - `SchemaFieldParser` in `src/schema/fields/parser.rs:28-120` claims recognized attributes and collects errors.
   - Specialised parsers in `fields/select.rs`, `fields/file.rs`, `fields/number.rs`, `fields/date.rs` convert generic `FieldValue` options into domain types (`SchemaSelectField`, `SchemaFileField`, `SchemaNumberField`, `SchemaDateField`).
5. **External select-values file confinement and caching**:
   - `SelectValuesFileCache` in `src/schema/fields/select.rs:208-289` parses `.toml`/`.json` external values files, strictly confined to the schemas directory using `SafeRelativePath`.
6. **Query & template read-side facade**:
   - `SchemaService` in `src/schema/service.rs:36-202` holds `IndexMap<SchemaName, Arc<Schema>>`.
   - Provides O(1) schema retrieval (`get`), hierarchy queries (`children_of`, `descendants_of`), and class expansion (`matches`).
   - Adapts to `query::FileClassExpander` in `src/file_class_expander.rs:15-50`.

---

## 3. Invariants
- **`global` is reserved and non-inheritable**:
  - Enforced in `SchemaGraphBuilder::new` (`graph/adjacency.rs:53-54`) where `global` is excluded from graph nodes.
  - Enforced in `builder.rs:144-156` where `global`'s own `extends` list is ignored.
  - Safe code outside cannot introduce a cycle involving `global` or make a note inherit `global` as a class.
- **`$ref` addresses are acyclic and bounded**:
  - Enforced at runtime in `SchemaFieldBuilder::resolve_ref` (`fields/builder.rs:252-277`): target must be `global` or in `self.ancestors` (`IndexSet<SchemaName>`).
  - Because `ancestors` comes strictly from the already-linearized acyclic `SchemaGraph`, `$ref` chains cannot loop.
- **Denial of unknown attributes**:
  - Top-level schema attributes denied via `#[serde(deny_unknown_fields)]` on `RawSchema` (`raw.rs:29`).
  - Field definition attributes denied via `RawSchemaFieldDefVisitor` (`raw.rs:133-146`) against `ALLOWED_OPTION_KEYS`.
  - Type-specific field options denied via `SchemaFieldParser::finish` (`fields/parser.rs:125-143`).
- **No ambiguous field canonical names**:
  - Enforced in `SchemaBuilder::reject_ambiguous_field_names` (`builder.rs:306-323`): no two fields in a resolved schema may produce the same `FieldKey::canonicalize(name)`. Violations yield `SchemaError::AmbiguousFieldName`.
- **Note value validation**:
  - **Invariant verified**: The schema subsystem **nowhere** validates note content or field values. No code in `src/schema/` references `src/note/` or `NoteFieldValue`. Schemas validate only *themselves* (their TOML definitions and external values files).
- **Immutability post-construction**:
  - Once constructed via `SchemaService::new` or `load_verbose`, `SchemaService` contains `Arc<Schema>` and exposes only immutable query methods.

---

## 4. State & Lifecycle
- **Owned State**:
  - `SchemaService` owns `IndexMap<SchemaName, Arc<Schema>>` (`service.rs:36-38`).
  - Each `Schema` owns `IndexMap<FieldName, SchemaFieldDef>`, `IndexSet<SchemaName>` for `ancestors`, `children`, and `descendants` (`model.rs:11-20`).
- **Caches & Ephemeral State**:
  - `SelectValuesFileCache` (`fields/select.rs:208-212`) uses `RefCell<IndexMap<PathBuf, Arc<Vec<FieldValue>>>>`. It is ephemeral: instantiated inside `SchemaFieldBuildContext` during `SchemaService::load_verbose`, used across all field builds, and dropped when loading completes (`service.rs:86-90`).
  - `SchemaAdjacency`, `SchemaGraph`, `SchemaBuilder`, `SchemaFieldBuilder` are all transient stack-allocated builders dropped after `load_verbose`.
- **Persistence**:
  - Schemas are read from `.traces/schemas/*.toml` on the filesystem. No schema data is persisted into `redb` (verified in `src/index/entry.rs` and `src/schema/service.rs`).
- **Concurrency & Mutability**:
  - Entirely single-threaded, synchronous execution. No `rayon`, no OS threads, no `Mutex` or `RwLock`.
  - In minijinja template rendering (`src/template/engine/schema.rs:48-90`, `cache.rs:9-12`), `Arc<SchemaService>` is seeded into minijinja's `State` temp storage on the first `schema.get()` call so `Schema`'s minijinja `Object` implementation can execute hierarchy methods (`children`, `descendants`).

---

## 5. Dependency Edges
- **Outbound `crate::` edges**:
  - `crate::FieldName`, `crate::FieldNameRef`, `crate::FieldNameError` (from `src/field.rs`)
  - `crate::FieldKey` (from `src/field.rs`)
  - `crate::FieldValue` (from `src/field.rs`)
  - `crate::BaseNameRef` (from `src/path.rs`)
  - `crate::DirTree`, `crate::DirTreeError` (from `src/path.rs`)
  - `crate::path::{SafeRelativePath, PathError}` (from `src/path.rs`)
  - `crate::strsim::closest_match` (from `src/strsim.rs`)
  - Outbound to other submodules: **None** (zero dependencies on `crate::note`, `crate::query`, `crate::template`, `crate::index`, or `crate::config`).
- **Inbound `crate::` edges**:
  - `src/cli/mod.rs` & `src/cli/error.rs`: loads `SchemaService`, maps `SchemaError`.
  - `src/file_class_expander.rs`: implements `query::FileClassExpander` for `SchemaService`.
  - `src/template/engine.rs`: loads and stores `Arc<SchemaService>`.
  - `src/template/engine/query.rs`: consumes `Arc<SchemaService>` for query operations.
  - `src/template/engine/schema.rs`: binds `SchemaOps`, `Schema`, `SchemaFieldDef` to minijinja.
- **Cycles & Leaked Internals**:
  - Zero architectural cycles. Clean unidirectional dependency graph: `schema` sits at the foundational layer beside `path` and `field`.
  - Test-only leak: `pub(crate) use error::SchemaFileError` in `src/schema/mod.rs:33-34` exists solely for `template::engine::schema` test pattern-matching.

---

## 6. Interfaces
- **`SchemaService`**:
  - Deep seam. Exposes a clean, narrow public/crate API (`new`, `load_verbose`, `get`, `children_of`, `descendants_of`, `matches`) hiding Kahn linearization, CSR graphs, cycle detection, and attribute parsing.
- **`Schema`**:
  - Moderately deep domain model. Hides field inheritance history, ancestor resolution, and suggestion algorithms (`suggest_field`). Exposes `name`, `field`, `fields`.
- **`SchemaBuilder` vs `SchemaFieldBuilder`**:
  - Distinct responsibilities: `SchemaBuilder` (`src/schema/builder.rs:58`) orchestrates inter-schema relationships (graph ordering, DAG inheritance, `excludes`). `SchemaFieldBuilder` (`src/schema/fields/builder.rs:74`) operates at intra-field level (parsing options, resolving `$ref` coordinates).
  - While clean in separation, both have builder naming and both accumulate diagnostic warnings, creating conceptual symmetry with slightly fragmented error routing.

---

## 7. Algorithms, Data Structures & Perf
- **Compressed Sparse Row (CSR) Graph (`src/schema/graph/adjacency.rs:28-87`)**:
  - Replaces hash probes with contiguous array slices: `DenseIndex(u32)` indices, `names: Vec<SchemaNameRef>`, `child_offsets: Vec<u32>`, `child_targets: Vec<DenseIndex>`.
  - Compact memory footprint, cache-friendly traversal during Kahn's sort and hierarchy calculation.
- **Cycle Detection (`src/schema/graph/cycle.rs:18-80`)**:
  - Iterative strongly-connected-components search over the Kahn-unvisited subgraph. Stack-allocated work vector (`Vec<(DenseIndex, usize)>`), zero recursion, avoids recursion limit panics on adversarial topologies. Runs in $O(V + E)$ time.
- **Topological Linearization & Ancestor Closure**:
  - Linear Kahn sort runs in $O(V + E)$.
  - Hierarchy sets (`ancestors`, `descendants`, `children`) are precomputed once at construction and stored as `IndexSet<SchemaName>` inside each `Schema`. Read queries (`descendants_of`, `matches`) are fast index lookups and set unions.
- **Select File Parsing & Confinement**:
  - Path confinement checked via `SafeRelativePath::parse`. File content read from disk (`std::fs::read_to_string`) and parsed via `toml` or `serde_json`.
  - Caching via `RefCell<ValuesFileCacheMap>` prevents duplicate I/O when multiple select fields reference the same values file.
- **Allocations**:
  - `RawSchemaFieldDef.options` captures all type-specific options into an `IndexMap<String, FieldValue>` before typed parsing, allocating map entries and `FieldValue` variants for every key in TOML.
  - Strings are frequently cloned during inheritance (`fields.entry(...).or_insert_with(|| field.clone())` in `builder.rs:251`).

---

## 8. Error Topology
- **Error Proliferation**:
  - System splits errors across two main layers:
    1. Schema loading/graph errors (`src/schema/error.rs`): `SchemaError`, `SchemaFileError`, `SchemaNameError`, `SchemaWarning`.
    2. Field parsing/building errors (`src/schema/fields/error.rs`): `SchemaFieldBuilderError`, `SchemaFieldParserError`, `SchemaSelectFieldValueError`, `SchemaSelectFieldEntryError`, `SchemaSelectFieldFileError`.
  - Total of 9 error/warning enums across ~1,143 lines of error definitions.
- **Conversions & Context**:
  - `SchemaFieldBuilderError` boxes inside `SchemaError::FieldBuilder(Box<SchemaFieldBuilderError>)` (`error.rs:73`).
  - In bare `$ref` overrides, field errors convert into `SchemaWarning::DegradedOverride` (`fields/builder.rs:141`), preserving the base field definition while logging/accumulating the warning.
  - Multi-error batching: `SchemaFieldBuilderError::Parser(Vec<SchemaFieldParserError>)` accumulates all option mismatches for a field rather than bailing on the first.
  - In `service.rs:262-275`, `SchemaConstruction` preserves two parallel streams of non-fatal diagnostics: `warnings: Vec<SchemaWarning>` and `failures: Vec<SchemaFailure>`.

---

## 9. Duplicated & Conflicting Concepts
- **Field Type Taxonomy Drift (`RawSchemaFieldType` vs `SchemaFieldTypeTag` vs `SchemaFieldType` vs `NoteFieldType`)**:
  1. `RawSchemaFieldType` (`raw.rs:204`): Serde wire enum (`input`, `select`, `boolean`, `number`, `date`, `file`).
  2. `SchemaFieldTypeTag` (`fields.rs:241`): Diagnostic enum mirroring the 6 kinds without payloads.
  3. `SchemaFieldType` (`fields.rs:150`): Domain enum carrying resolved option structs (`Select(Arc<SchemaSelectField>)`, `Number(SchemaNumberField)`, etc.).
  4. `NoteFieldType` (`src/note/field.rs:209`): Note frontmatter/body value taxonomy (`Null`, `Bool`, `Number`, `Duration`, `Temporal`, `Text`, `Link`, `List`, `Object`).
  - **Conceptual Conflict**: `SchemaFieldType` models schema author constraints (e.g. `select`, `file`, `date`), whereas `NoteFieldType` models parsed Markdown/YAML primitives (e.g. `Text`, `Link`, `Temporal`, `List`). The two taxonomies have no direct mapping or validation bridge.
- **Select Parsing Dual-Track in `fields/select.rs`**:
  - `parse_inline_value_objects` vs `parse_file_object_entries` vs `parse_file_bare_entries`: ~2,000 lines in `select.rs` to handle combinations of inline strings, inline objects, file bare strings, and file objects with selectors (`value`, `label`, `order`).

---

## 10. Doc, Glossary & ADR Drift
- **ADR-0006 Claim Verification**:
  - Claim: "Deny unknown fields via `#[serde(deny_unknown_fields)]` on both RawSchema and RawSchemaFieldDef" (ADR-0006:62).
    - **Drift**: `RawSchemaFieldDef` cannot use `#[serde(deny_unknown_fields)]` directly because type-specific options are collected into `options: IndexMap<String, FieldValue>` via a custom visitor (`raw.rs:115-185`). Unknown top-level field keys are denied via `ALLOWED_OPTION_KEYS` check in `RawSchemaFieldDefVisitor`, not direct serde derive.
  - Claim: "A missing extends target degrades to exact match with a warning" (ADR-0006:86, ADR-0007:51).
    - **Verified**: Implemented via `SchemaWarning::MissingExtendsTarget` (`graph/adjacency.rs:73`, `service.rs:191-200`).
- **ADR-0007 Claim Verification**:
  - Claim: "Linearizes the class DAG with Kahn's topological sort; cycles are hard errors... own fields override all parents; among parents the first-listed wins" (ADR-0007:50-52).
    - **Verified**: Implemented in `graph/builder.rs` and `builder.rs:241-256`.
  - Claim: "A stray `required = true` is ignored with a warn log" (ADR-0007:60).
    - **Verified**: Handled in `fields/builder.rs:152-154` yielding `SchemaWarning::StrayGlobalRequired`.
- **GLOSSARY.md Drift**:
  - `src/schema/GLOSSARY.md:69-73` defines "File Field Filter" as restricting targets by "folder, file extension, or File Class". Verified in `SchemaFileField` (`folders`, `ext`, `class`).

---

## 11. Candidate Architectural Questions

1. **Where should runtime note validation live, and should it belong to the `schema` subsystem?**
   - *Claim*: The schema subsystem currently defines field types, ranges, options, and required/multi flags, but provides zero validation of note frontmatter/body values. Planned LSP diagnostics (ticket 20) require validating note metadata against schema definitions. This validator belongs as a pure function `validate(&Frontmatter, &Schema) -> Vec<SchemaDiagnostic>` inside `src/schema/`.
   - *Evidence*: `src/schema/` has zero references to `src/note/` (`grep` verified). `NoteFieldValue` has its own taxonomy (`NoteFieldType`) distinct from `SchemaFieldType`.
   - *Counter-argument*: `schema` is currently pure metadata about file classes and depends only on primitive types (`FieldName`, `FieldValue`). Pulling note models into `schema` creates an inward dependency from `schema` to `note` (or forces a new bridging crate/module). Keeping `schema` strictly about schema definitions and placing validation in an external service (or `note::validate` / `analysis::schema`) keeps `schema` small and independent of note AST representations.
   - *Open Decision*: Does `src/schema` own the validator (`schema::validate`), or does an external analysis/engine layer own it while `src/schema` remains a passive metadata registry?
   - *Touches*: `schema`, `note`, LSP analysis host (`.scratch/md-pkm-lsp/issues/20`), CLI linting.

2. **Should the five-layer error hierarchy in `schema::fields::error` be flattened?**
   - *Claim*: `SchemaFieldParserError` -> `SchemaSelectFieldValueError` -> `SchemaSelectFieldEntryError` -> `SchemaSelectFieldFileError` represents needless nested error abstraction for a load-time config parser, creating hundreds of lines of boilerplate and verbose conversion matches.
   - *Evidence*: `src/schema/fields/error.rs` is 655 lines with 5 distinct error enums. Most variants are string-formatting wrappers that convert into either a hard `SchemaError::FieldBuilder` or a `SchemaWarning::DegradedOverride`.
   - *Counter-argument*: The hierarchy preserves exact error provenance (whether an invalid selector occurred in an inline list vs an external JSON vs an external TOML file, at what path), which is valuable for precise user diagnostics in CLI error reporting.
   - *Open Decision*: Retain fine-grained nested error types for select values vs flatten into a single diagnostic struct or flat error enum.
   - *Touches*: `schema::fields::error`, `schema::error`, CLI diagnostics.

3. **Is the 2K-line `fields/select.rs` over-engineered for select option extraction?**
   - *Claim*: `select.rs` is 2,032 lines (over 16% of the entire schema subsystem), supporting inline bare strings, inline objects, external TOML, external JSON, dynamic key selectors (`value`, `label`, `order`), and float total ordering. This complexity exceeds what template authoring and metadata menus practically require.
   - *Evidence*: `src/schema/fields/select.rs` spans 2,032 lines with 1,416 lines of tests. It implements its own caching layer (`SelectValuesFileCache`), custom file format dispatch, and multi-field selector resolution.
   - *Counter-argument*: External JSON/TOML data feeds allow users to maintain large enumerated option lists (e.g. ISO codes, client lists, project taxonomies) outside note frontmatter without bloating template files. The implementation is pure, well-tested, and isolated behind `SchemaSelectField`.
   - *Open Decision*: Keep rich external select file resolution as-is or restrict select fields to static inline declarations.
   - *Touches*: `schema::fields::select`, template rendering, LSP completion.

4. **Should schema resolution support incremental update rather than full-directory reload?**
   - *Claim*: In long-running LSP sessions, editing a single `.traces/schemas/book.toml` currently triggers a full directory re-scan, re-parse of all schemas, rebuilding of the CSR graph, Kahn's sort, and re-merging of all fields via `SchemaService::load_verbose`. An incremental or fine-grained invalidation model is needed.
   - *Evidence*: `SchemaService::new` (`service.rs:62-64`) and `load_verbose` (`service.rs:83-90`) read the entire directory via `DirTree::children`. `SchemaService` has no update or reload method for an individual schema.
   - *Counter-argument*: A vault rarely has more than a few dozen schemas. A full reload takes <1ms in memory (single-pass TOML parsing + linear CSR sort). Kahn's sort over a graph of 50 nodes is negligible. Incremental DAG mutation introduces cache coherence bugs for zero measurable user latency gain.
   - *Verdict*: **Keep as-is** (re-load entire directory on schema file change).
   - *Touches*: `schema::service`, LSP file watcher.

5. **Should `RawSchemaFieldDef.options` use untyped `FieldValue` or strongly-typed TOML structs per field kind?**
   - *Claim*: Deserializing options into a generic `IndexMap<String, FieldValue>` and then validating with a manual `SchemaFieldParser` reimplements serde's job in custom imperative code across ~900 lines in `parser.rs`. Using serde internally tagged/untagged enums would eliminate hundreds of lines of manual key claiming.
   - *Evidence*: `src/schema/fields/parser.rs` is 918 lines devoted to `raw_value`, `string`, `string_list`, `f64`, `claimed: HashSet<&'static str>`, and tracking type mismatches.
   - *Counter-argument*: Serde's derived error messages for untagged/tagged enums are notoriously inscrutable and fail to support Traces' graceful degradation model (where bare `$ref` overrides degrade invalid keys to warnings while keeping valid keys). The manual parser produces exact "field #book/status of type select has no attribute 'min'" diagnostics.
   - *Open Decision*: Keep custom imperative parser for graceful degradation and tailored diagnostics vs refactor to serde-based typed options.
   - *Touches*: `schema::raw`, `schema::fields::parser`, `schema::fields::builder`.

6. **Should `Schema` hold an `Arc` reference to its `SchemaService` or remain a passive value object?**
   - *Claim*: Minijinja templates call `.children()` and `.descendants()` on `Schema`. Because `Schema` does not reference `SchemaService`, minijinja has to stash an `Arc<SchemaService>` in renderer temp state via a hidden key (`"schema.service_cache"` in `src/template/engine/schema.rs:90`) during `schema.get()` and re-fetch it dynamically. Giving `Schema` an `Arc<SchemaHierarchy>` or having `SchemaService` return a handle would eliminate this temp-cache hack.
   - *Evidence*: `src/template/engine/schema.rs:48-53` explicitly documents this workaround: "`.field()`/`.children()`/`.descendants()` instead re-fetch the render's `Arc<SchemaService>` from `State`'s temp storage on demand".
   - *Counter-argument*: Keeping `Schema` pure and decoupled from `SchemaService` avoids reference cycles and makes `Schema` trivially cloneable, comparable (`PartialEq`), and thread-safe without lifetime parameters.
   - *Open Decision*: Retain detached value object with engine-level context seeding vs attach a shared hierarchy reference to resolved `Schema` handles.
   - *Touches*: `schema::model`, `template::engine::schema`.
