# Slice Survey: Schema Subsystem

> **Survey baseline: `2dc3250d`. Re-verification note (main @ `207cc3fb`).** Only `src/index/service.rs` line numbers moved (fileClass tests are `mod class_field`, `service.rs:1732`). `src/schema` is untouched. Authoritative corrections: `reverification.md`.

> **Superseded in part by ticket 16 (checked in round 5, `research/20-prep/round5/R2-spec-claims.md` section 6.2).** The praise for the `FileClassExpander` seam (sections 1, 8) is overturned; `global` is no longer only a `$ref` pool (sections 2, 8) but also the lowest-precedence layer for every note; "note value validation is absent" (section 2, hypothesis H1 in section 4) is resolved by an advisory check; "first-listed wins" (section 1) no longer decides class-versus-class conflicts (it still decides `extends` parents); query-typing fog (section 7) is resolved (schema never types values at parse or query time). See [ticket 16's Answer](../issues/20-schema-note-typing-and-validation.md).

## 1. Responsibilities & seams

- **Schema TOML deserialization & raw validation**: Parses `.traces/schemas/<name>.toml` via `serde` (`src/schema/raw.rs:28-40`). Collects type-specific options into untyped `IndexMap<String, FieldValue>` bags (`src/schema/raw.rs:59-60`).
- **Inheritance DAG & cycle detection**: Deduplicates parent declarations, builds CSR graph (`src/schema/graph/adjacency.rs:28-87`), detects cycles via Tarjan-style DFS (`src/schema/graph/cycle.rs:18-80`), and linearizes inheritance order via Kahn's topological sort (`src/schema/graph/builder.rs:27-71`).
- **Field resolution & merging**: Merges multi-parent fields (first-listed wins), applies `excludes`, and resolves `$ref` overrides bounded to `global` or transitive ancestors (`src/schema/builder.rs:125-295`). Enforces unique canonical field identities (`src/schema/builder.rs:306-323`).
- **External select values cache**: Confines and caches external `.toml`/`.json` select option tables within the schemas directory (`src/schema/fields/select.rs:208-289`).
- **Public & crate surface**:
  - `pub`: `SchemaService` (`src/schema/service.rs:36`), `Schema` (`src/schema/model.rs:11`), `GLOBAL_SCHEMA_NAME` (`src/schema/mod.rs:52`). Under test/test-utils: `SchemaFieldDef`, `SchemaServiceError`.
  - `pub(crate)`: `SchemaError`, `SchemaWarning`, `SchemaConstruction`, `RawSchema*`, `SchemaName*`, `SchemaField*`.
- **Callers & caller assumptions**:
  - `TemplateService` (`src/template/engine.rs:131-134`): Calls `SchemaService::load_verbose` at construction. Assumes service is immutable and thread-safe for the engine's lifetime.
  - `QueryService` (`src/file_class_expander.rs:15-39`): Consumes `SchemaService` via the `FileClassExpander` trait to expand `@class` queries to children/descendants.
  - `Cli` (`src/cli/mod.rs:490-502`): Calls `load_verbose` to service schema inspect commands and passes it to query/template pipelines.
  - Callers must know: `SchemaService` is read-only post-construction; directory reload is all-or-nothing; schemas validate only their own definitions, never note contents.

## 2. Invariants

- **`global` is reserved and non-inheritable**: Enforced at construction (`src/schema/graph/adjacency.rs:53-54`, `src/schema/builder.rs:144-156`). `global` cannot be extended, cannot participate in Kahn sorting or cycles, and `required = true` on `global` fields degrades to `false` with `SchemaWarning::StrayGlobalRequired` (`src/schema/fields/builder.rs:152-154`).
- **`$ref` targets are acyclic and ancestor-bounded**: Enforced at runtime in `SchemaFieldBuilder::resolve_ref` (`src/schema/fields/builder.rs:252-277`). A `$ref` can only target `global` or a transitive `extends` ancestor in `self.ancestors`. Arbitrary cross-hierarchy references are rejected (`RefOutOfBounds`).
- **Canonical field name uniqueness**: Enforced at construction in `SchemaBuilder::reject_ambiguous_field_names` (`src/schema/builder.rs:306-323`). No two effective fields on a schema may produce the same `FieldKey::canonicalize(name)`; prevents ambiguous note frontmatter matching.
- **Topological evaluation order**: Enforced by Kahn's sort (`src/schema/graph/builder.rs:88-125`). Parents always resolve before children, guaranteeing base definitions exist for field inheritance.
- **Graceful degradation for non-fatal defects**:
  - Missing `extends` target emits `MissingExtendsTarget` warning, skips parent, keeps own fields (`src/schema/graph/adjacency.rs:73`, `src/schema/builder.rs:241-248`).
  - Failed parent resolution emits `ParentFailedToResolve` warning, child resolves without parent's fields (`src/schema/builder.rs:242-247`).
  - Bare `$ref` override with invalid options emits `DegradedOverride` warning and falls back to base field options (`src/schema/fields/builder.rs:139-145`).
- **Note value validation is absent**: Enforced nowhere / convention. The schema subsystem contains zero checks against note frontmatter or body values (`src/schema/` never references `src/note/`). Notes are completely unvalidated by schemas.
- **Post-construction immutability**: Enforced by type construction (`src/schema/service.rs:36-38`). `SchemaService` exposes only immutable references (`&Arc<Schema>`) and query iterators.

## 3. Verified observations [V]

- [V] `src/schema` contains 12,129 total lines across 20 files: 4,756 lines production code (39.2%), 7,373 lines unit test code (60.8%) (`src/schema/**`).
- [V] `select.rs` alone comprises 2,032 lines (616 prod, 1,416 test), representing 16.8% of the entire subsystem (`src/schema/fields/select.rs:1-2032`).
- [V] `src/schema/` has zero import dependencies on `src/note/`, `src/query/`, `src/template/`, or `src/index/` (`src/schema/mod.rs:1-52`).
- [V] `SchemaService::new` is marked `expect(dead_code)` in non-test builds (`src/schema/service.rs:52-60`); production loads exclusively through `load_verbose` (`src/schema/service.rs:83-90`, `src/template/engine.rs:132`).
- [V] Minijinja templates access `Schema` methods (`children`, `descendants`) via a hidden `State` temp-cache hack (`"schema.service_cache"`), because `Schema` has no reference to `SchemaService` (`src/template/engine/schema.rs:82-90`, `161-166`).
- [V] Field options use an untyped intermediate representation: `RawSchemaFieldDef.options` is `IndexMap<String, FieldValue>` (`src/schema/raw.rs:60`), parsed imperatively via `SchemaFieldParser` (`src/schema/fields/parser.rs:28-143`).
- [V] Four distinct field type enums coexist across the codebase: `RawSchemaFieldType` (`src/schema/raw.rs:204`), `SchemaFieldTypeTag` (`src/schema/fields.rs:241`), `SchemaFieldType` (`src/schema/fields.rs:150`), and `NoteFieldType` (`src/note/field.rs:209`).
- [V] Cycle detection uses an iterative Tarjan/SCC-style DFS over dense `u32` indices, guaranteeing $O(V+E)$ execution without stack overflow (`src/schema/graph/cycle.rs:18-80`).
- [V] Schema CSR adjacency uses contiguous array slices (`child_offsets: Vec<u32>`, `child_targets: Vec<DenseIndex>`) (`src/schema/graph/adjacency.rs:42-47`).
- [V] Five nested error enums govern field parsing: `SchemaFieldBuilderError`, `SchemaFieldParserError`, `SchemaSelectFieldValueError`, `SchemaSelectFieldEntryError`, `SchemaSelectFieldFileError` (`src/schema/fields/error.rs:32-232`).
- [V] Notes index their `fileClass` into Redb multimap `paths_by_file_class` purely by raw string extraction without consulting `SchemaService` (`src/index/service.rs:1656-1840`).

## 4. Hypotheses [H]

- **[H1] Schema subsystem is an isolated metadata compiler, leaving note data completely untyped and unvalidated.**
  - *Thesis*: 12k lines model schema authoring constraints, but Traces never validates note frontmatter or coerces types against schemas. Runtime validation is 0% implemented in product code.
  - *Current*: `schema` models schemas; `note` models notes; `index` indexes raw string classes; zero validation exists.
  - *Alternative*: Introduce `validate(note: &Note, schema: &Schema) -> Vec<SchemaDiagnostic>` in a dedicated validation layer (`schema::validate` or `analysis::schema`).
  - *Alternative (Keep as-is)*: Keep schema as a pure passive metadata provider; let note frontmatter remain unconstrained open data (Obsidian Dataview style).
  - *Disproof*: If users require flexible schemaless notes where schemas serve only as optional autocomplete hints for templates, strict validation would break valid vaults.
  - *Blast radius*: Broad if note indexing fails on schema violations; narrow if validation is advisory (LSP/linting diagnostics).
- **[H2] External select values processing (`fields/select.rs`, 2,032 lines) is accidental complexity disproportionate to value.**
  - *Thesis*: Supporting external TOML, JSON, dynamic key selectors (`value`, `label`, `order`), bare strings vs objects, float total ordering, and file cache confinement accounts for >16% of schema LoC and 5 nested error types, but templates rarely need more than static inline lists.
  - *Current*: Custom format dispatch, path confinement checking (`SafeRelativePath`), caching (`SelectValuesFileCache`), and 3 parsing tracks in `select.rs`.
  - *Alternative*: Restrict select fields to inline string/object lists. If external data is needed, ingest it as standard notes or structured vault data.
  - *Alternative (Keep as-is)*: Preserve external data feeds so large taxonomies (ISO codes, clients) do not bloat `.toml` schema files.
  - *Disproof*: Identify critical user workflows depending on external shared JSON/TOML dictionaries.
  - *Removable*: Deletes ~1,500 lines across `select.rs`, `SelectValuesFileCache`, and 3 error enums.
  - *Blast radius*: Narrow (confined to `select` field parsing and external values file fixtures).
- **[H3] Untyped intermediate option parsing (`parser.rs`, 918 lines) reimplements serde in custom imperative code.**
  - *Thesis*: Collecting options into `IndexMap<String, FieldValue>` and manually claiming keys via `SchemaFieldParser` reimplements serde's deserializer with hand-rolled type mismatch reporting.
  - *Current*: Custom visitor collects unparsed keys; `SchemaFieldParser` tracks `claimed: HashSet<&'static str>` and collects errors (`src/schema/fields/parser.rs:28-53`).
  - *Alternative*: Use serde untagged/internally tagged enums directly on `RawSchemaFieldDef` for each field kind.
  - *Alternative (Keep as-is)*: Keep manual parser to support Traces' graceful degradation model (bare `$ref` overrides degrade invalid keys to warnings while keeping valid keys; serde cannot easily do partial degradation).
  - *Disproof*: Demonstrate that serde custom visitor/derive produces inscrutable error messages or cannot support key-level degradation.
  - *Removable*: Deletes ~600 lines of `parser.rs` and manual extractor methods.
  - *Blast radius*: Medium (`raw.rs`, `fields/builder.rs`, `fields/parser.rs`).
- **[H4] Four parallel field-kind taxonomies cause unnecessary cognitive fragmentation.**
  - *Thesis*: `RawSchemaFieldType`, `SchemaFieldTypeTag`, `SchemaFieldType`, and `NoteFieldType` duplicate the concept of field types across 4 distinct enums without direct mapping.
  - *Current*: `RawSchemaFieldType` for serde wire; `SchemaFieldTypeTag` for diagnostics; `SchemaFieldType` for resolved options; `NoteFieldType` for note values.
  - *Alternative*: Collapse `RawSchemaFieldType` and `SchemaFieldTypeTag` into one enum; build a clean mapping from `NoteFieldType` to `SchemaFieldType`.
  - *Alternative (Keep as-is)*: Keep them decoupled to avoid leaking payload options into diagnostic messages or serde wire contracts.
  - *Disproof*: If wire representation, diagnostic tags, and domain options have divergent evolution paths.
  - *Removable*: Eliminates 1 enum definition and dozens of trivial `From` match conversions.
  - *Blast radius*: Small (internal to `src/schema/fields/`).
- **[H5] Five-layer nested error hierarchy in `fields/error.rs` adds gratuitous boilerplate.**
  - *Thesis*: 5 nested error enums across 655 lines for field parsing are over-engineered.
  - *Current*: `SchemaFieldParserError` -> `SchemaSelectFieldValueError` -> `SchemaSelectFieldEntryError` -> `SchemaSelectFieldFileError` (`src/schema/fields/error.rs:32-232`).
  - *Alternative*: Flatten into a unified `SchemaDiagnostic` struct with code, path, and message.
  - *Alternative (Keep as-is)*: Retain fine-grained types for programmatic matching in tests.
  - *Disproof*: If downstream callers (LSP, CLI) need to match exhaustively on exact failure causes rather than display formatted messages.
  - *Removable*: Deletes ~350 lines of boilerplate error enum variants and `From`/`Display` impls.
  - *Blast radius*: Medium (unit tests in `error.rs` and `fields/error.rs`).
- **[H6] Detached `Schema` value object forces template engine context cache hack.**
  - *Thesis*: `Schema` contains no reference to `SchemaService`, forcing template engine to stash `Arc<SchemaService>` in Minijinja `State` temp storage on `schema.get()` (`src/template/engine/schema.rs:82-90`).
  - *Current*: `Schema` holds only local fields and ancestors; hierarchy queries re-fetch service from Minijinja state dynamically.
  - *Alternative*: Return a lightweight `SchemaHandle<'a>` bound to `&'a SchemaService`, or store a shared `Arc<SchemaHierarchy>` inside `Schema`.
  - *Alternative (Keep as-is)*: Keep `Schema` pure and detached, preserving trivial cloning, `PartialEq`, and reference-cycle freedom.
  - *Disproof*: If storing `Arc` in `Schema` creates reference cycles or complicates serialization/testing.
  - *Removable*: Deletes `SCHEMA_SERVICE_CACHE_KEY`, `cached_service`, and temp-state plumbing in `template/engine/schema.rs`.
  - *Blast radius*: Small (`src/schema/model.rs`, `src/template/engine/schema.rs`).
- **[H7] Whole-directory rebuild on schema change is optimal and withstands scrutiny (keep as is).**
  - *Thesis*: Incremental DAG mutation for schemas is premature optimization. Even with 100 schemas, directory load and Kahn sort take <1ms in memory.
  - *Current*: Whole directory reload via `SchemaService::load_verbose` on engine/CLI init.
  - *Alternative*: Incremental file watcher invalidation and fine-grained DAG mutation.
  - *Disproof*: Benchmarking shows full reload of 1,000 synthetic schemas in memory takes <5ms. Incremental synchronization would add lock contention and cache invalidation bugs for zero perceptible benefit.
  - *Blast radius*: Zero (retains current architecture).
- **[H8] CSR adjacency representation is over-engineered for PKM schema counts, but harmless.**
  - *Thesis*: CSR graph (`adjacency.rs`, 734 lines) with dense integer indexing is intended for graphs with $10^5+$ nodes; vaults have 5–50 schemas. A standard `IndexMap<SchemaName, Vec<SchemaName>>` would do identical work in 150 lines.
  - *Current*: Dense `u32` CSR slices (`child_offsets`, `child_targets`) in `graph/adjacency.rs:42-47`.
  - *Alternative*: Replace CSR with standard adjacency list maps.
  - *Alternative (Keep as-is)*: Keep CSR because it is completely tested, deterministic, allocation-free during traversal, and already working.
  - *Disproof*: Refactoring CSR saves only ~400 lines but risks subtle regressions in deterministic traversal.
  - *Blast radius*: Internal to `src/schema/graph/`.

## 5. Cross-slice couplings

- **With `field.rs`**: Strongly coupled to root primitive types: `FieldName` (`src/field.rs:45`), `FieldNameRef` (`:216`), `FieldKey` (`:273`), `FieldValue` (`:595`), `FieldValueRef` (`:716`). Schema canonicalization explicitly delegates to `FieldKey::canonicalize` (`src/schema/builder.rs:312`).
- **With `path.rs`**: Consumes `DirTree::children` (`src/schema/service.rs:223`), `SafeRelativePath` (`src/schema/fields/select.rs:22`), and `PathError` (`src/schema/fields/error.rs:211`).
- **With `query` (Clean seam)**: Completely decoupled via `src/file_class_expander.rs:15-39`, which implements `query::FileClassExpander` for `SchemaService`. Neither `query` nor `schema` imports the other.
- **With `template` (Tight wiring)**:
  - `TemplateEngine::new` eagerly loads `SchemaService` (`src/template/engine.rs:131-134`).
  - Minijinja engine exposes `SchemaOps` and binds `Schema`, `SchemaFieldDef`, and `SchemaFileFieldRef` directly into template objects (`src/template/engine/schema.rs:69-72`).
  - Runtime service-seeding hack: `schema.rs:82-90` stashes `Arc<SchemaService>` in render state temp-storage.
- **With `index` (Total absence of coupling)**:
  - `IndexerService` parses note frontmatter and populates `paths_by_file_class` Redb table without referencing schemas (`src/index/service.rs:1656-1840`).
- **With `note` (Zero bridge)**:
  - No connection exists between `SchemaFieldDef` and `NoteFieldValue` or `NoteFieldType`. Schema field rules are completely ignored during note parsing and storage.

## 6. Comparables worth studying

- **Obsidian Metadata Menu / Dataview** (`obsidian-metadata-menu`, `obsidian-dataview`): Informs [H1] (runtime note validation). Metadata Menu implements fileClass inheritance and frontmatter field validation as optional UI linting, leaving Markdown text permissive.
- **DuckDB / SQLite schema catalogs** (`duckdb/src/catalog`): Informs [H1], [H4] (catalog vs runtime row validation). Catalogs provide metadata for query planning and row binding; row validation occurs strictly at execution boundaries.
- **`jsonschema` / `schemars`** (`crates.io/crates/schemars`): Informs [H3] (declarative option parsing vs imperative reflection). Explores compiling schema constraints from serde data structures instead of imperative key visitors.
- **rustc graph & cycle detection** (`compiler/rustc_data_structures/src/graph`): Informs [H7], [H8] (graph representations). Rustc uses dense integer indices and CSR-like structures for dependency graphs; demonstrates clean separation between graph traversal and AST nodes.
- **Typst object model** (`typst/crates/typst-eval`): Informs [H6] (environment-bound values). Typst passes contextual engine handles through evaluators rather than stashing engines in dynamic thread/render temp state.

## 7. Open decisions

- `ticket:schema-note-validator`: Where should runtime note validation live (`schema::validate` vs `note::validate` vs external `analysis::schema`), and should it be advisory (LSP/lint) or enforcing (indexing reject)?
- `ticket:schema-select-simplification`: Should external JSON/TOML select values files be eliminated in favor of inline declarations, or retained for large dictionaries?
- `ticket:schema-field-serde-refactor`: Can `RawSchemaFieldDef.options` migrate to strongly-typed serde enum tables while preserving graceful degradation warnings on bare `$ref` overrides?
- `ticket:schema-error-flattening`: Should the 5-layer nested error hierarchy in `fields/error.rs` be collapsed into a flat diagnostic model?
- `ticket:schema-template-handle`: Should `Schema` be bound to `SchemaService` via a handle type (`SchemaHandle`) to eliminate the Minijinja `State` temp-storage hack?
- Fog: How should query execution behave when a note's frontmatter value violates its schema's field type (e.g. string where number is expected) — coerce, treat as null, or yield an execution error?

## 8. Withstands scrutiny

- **Kahn's linearization & bounded `$ref` invariant**: Requiring `$ref` to target only `global` or transitive `extends` ancestors (`src/schema/fields/builder.rs:252-277`) guarantees acyclic reference resolution by construction. Coupled with Kahn's topological sort, field merging is strictly deterministic and robust against adversarial topologies.
- **Ambiguous field name rejection**: Enforcing `FieldKey::canonicalize` uniqueness across effective fields (`src/schema/builder.rs:306-323`) prevents silent frontmatter shadowing across case/separator variations (`status` vs `Status` vs `status_`).
- **FileClassExpander seam**: Decoupling `schema` and `query` via a 39-line adapter trait (`src/file_class_expander.rs:15-39`) is a model deep interface. Neither crate-level module imports the other.
- **All-or-nothing directory reload over incremental DAG mutation**: Full reload of <50 schemas executes in <1ms. Avoiding incremental DAG cache invalidation eliminates an entire class of synchronization bugs at zero performance cost.
- **Global schema isolation**: Treating `global` as a flat, non-inheritable `$ref` pool (`src/schema/builder.rs:144-156`) prevents cyclic inheritance bugs and cleanly separates shared defaults from note file classes.
