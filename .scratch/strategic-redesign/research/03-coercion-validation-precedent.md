# Research: Type Coercion, Validation Precedents, and Diagnostic Reporting

Label: `wayfinder:research`. Informs: coercion and comparison policy, schema-note typing and validation, query core.
Context tickets: `survey/schema.md` (H1), `survey/query.md` (H6), `survey/note.md` (H7).

---

## 1. What Each Comparable Actually Does

### SQLite: Storage Classes, Column Affinity, and Comparison Rules

- **Dynamic storage classes on values**: Every SQLite value (`sqlite3_value` / `Mem`) dynamically stores its fundamental storage class: `NULL`, `INTEGER`, `REAL`, `TEXT`, or `BLOB` (`src/vdbeaux.c:4555-4623`) [V].
- **Column affinity**: Table columns carry an affinity (`TEXT`, `NUMERIC`, `INTEGER`, `REAL`, `BLOB`) (`src/vdbe.c:397-428`, `applyAffinity`) [V]. Affinity is applied when storing data into a column or during comparison. When coercing to `NUMERIC`, strings with valid numeric syntax are converted to `INTEGER` or `REAL`; unparseable strings remain unchanged as `TEXT` without raising an error (`vdbe.c:407`, `applyNumericAffinity`) [V].
- **Comparison across storage classes**: Total cross-type ordering is implemented in `sqlite3MemCompare` (`src/vdbeaux.c:4555-4623`): `NULL < NUMERIC (integers & reals sorted numerically) < TEXT (collation sequence) < BLOB (memcmp)` [V]. A numeric is always strictly less than any text (`vdbeaux.c:4599-4610`) [V].
- **Comparison affinity rules**: In expressions comparing two operands, `sqlite3CompareAffinity` (`src/expr.c:350-365`) decides coercion:
  - Column vs un-typed expression (literal): the column's affinity is applied to the literal before comparing [V].
  - Column vs Column: if either has numeric affinity, `NUMERIC` affinity is used; otherwise `BLOB` (no coercion) [V].
  - Expression vs Expression: no affinity conversion occurs; values compare by raw storage class [V].

### DuckDB: Explicit/Implicit Cast Graph and `TRY_CAST`

- **Directed implicit cast cost matrix**: DuckDB computes implicit cast viability via `CastRules::ImplicitCast(from, to)` (`src/function/cast_rules.cpp:18, 54-311`) [V]. It returns an integer cost >= 0 (e.g. Tinyint -> Smallint -> Integer -> Bigint -> Hugeint -> Double; Date -> Timestamp), or -1 if implicit conversion is prohibited [V].
- **String literal protection**: `VARCHAR` is not implicitly cast to numeric or temporal types during binary operations (`src/function/cast_rules.cpp:35-36`, cost = -1) [V]. Overload resolution penalizes or rejects arbitrary string-to-type coercion, preventing unintended promotion.
- **Dual cast semantics (`CAST` vs `TRY_CAST`)**: In `Value::TryCastAs` (`src/common/types/value.cpp:2146-2183`) and `src/function/cast/string_cast.cpp:388-392`, casting takes `bool strict`:
  - `strict = true` (`CAST`): conversion failure throws `ConversionException` [V].
  - `strict = false` (`TRY_CAST`): conversion failure invokes `HandleCastError::AssignError`, writing SQL `NULL` without error [V].

### Polars: Strict vs Non-Strict Casts

- **Series cast options**: Series casting accepts `CastOptions` (`crates/polars-core/src/series/mod.rs:415, 503-506`) [V].
- **Strictness handling**: `CastOptions::NonStrict` converts invalid or out-of-range elements to `null` [V]. `CastOptions::Strict` runs non-strict conversion and then calls `handle_casting_failures(slf.as_ref(), &out)` (`src/series/mod.rs:504-506`), raising `ComputeError` if the null count increased [V].
- **Untyped ingestion**: `Series::from_any_values_and_dtype` (`crates/polars-core/src/series/any_value.rs:91-105`): with `strict = false`, incompatible values degrade to `null`; with `strict = true`, type mismatches return an error immediately [V].

### Obsidian Dataview: Type Inference and `Values.compareValue`

- **Ingestion-time heuristic inference**: `parseFrontmatter` (`src/data-index/resolver.ts`, digest lines 3650-3688) recursively inspects untyped frontmatter strings: tests `EXPRESSION.date.parse`, then `EXPRESSION.duration.parse`, then `EXPRESSION.embedLink.parse`, falling back to raw string only if all fail [V].
- **Universal total order**: `Values.compareValue` (`src/data-model/value.ts`, digest lines 5470-5564) implements total ordering across any two arbitrary values [V].
- **Mismatched type ordering**: When two values have different types (`wrap1.type != wrap2.type`), Dataview falls back to alphabetical string comparison of their internal type names: `wrap1.type.localeCompare(wrap2.type)` (`digest:5487`) [V]. Thus `"array" < "boolean" < "date" < "duration" < "link" < "number" < "object" < "string"` [V].

### Obsidian Metadata Menu: fileClass Field Validation & Advisory UX

- **Field-level validation**: Validates note field values against `fileClass` definitions using `validateValue(type: FieldType)(fieldVM)` (`src/fields/Fields.ts`, digest lines 6751-6775) [V].
- **Advisory index and read queries**: `FieldInfo.getInfos()` (`src/fields/Field.ts`, digest lines 2570-2586) returns `isValid: !fieldVM.value || validateValue(fieldVM.type)(fieldVM)` as diagnostic metadata [V]. Notes with invalid fields are never rejected by the index or hidden from views [V].
- **Modal write-gating**: Gating occurs strictly at the interactive UI boundary: modal form submission (`save()` in `src/fields/models/Number.ts:10533`) checks `validateValue`, blocking write-back and showing user errors if invalid [V]. Raw markdown edits remain unconstrained.

### jsonschema (Rust) & schemars: Decoupled Validation with Instance Paths

- **Decoupled validation pass**: `jsonschema::JSONSchema::validate(&self, instance: &Value)` executes an out-of-band validation pass against an existing `serde_json::Value` tree, returning an iterator of `ValidationError` [V].
- **Structured path reporting**: `ValidationErrorRepr` (`jsonschema/src/error.rs:62-75`) captures `instance_path: Location` (RFC 6901 JSONPointer path into data), `schema_path: Location` (keyword location in schema), and `kind: ValidationErrorKind` [V].
- **Generation vs Validation separation**: `schemars` generates schema definitions from Rust structs via derive `JsonSchema`; validation logic is entirely decoupled from schema generation [V].

### taplo / toml-edit: CST Spans and Advisory Schema Diagnostics

- **Independent parsing and validation**: `taplo::parser::parse` constructs CST and DOM (`taplo::dom::Node`) with rowan byte spans (`TextRange`) without requiring a schema [V].
- **Mapping validation errors to source spans**: `Schemas::validate_root` validates DOM JSON via `jsonschema`. `NodeValidationError::new` traverses `error.instance_path` across the CST `dom::Node` hierarchy to resolve the exact AST node and byte span (`taplo-common/src/schema/mod.rs:90-148, 680-717`) [V].
- **Advisory LSP & CLI diagnostics**: `taplo-lsp/src/diagnostics.rs:308-322` publishes these as LSP diagnostics (`DiagnosticSeverity::ERROR`). Neither formatting nor parsing is halted by schema non-conformance [V].

### Ruff: Settings Validation and Schema Export

- **Strict configuration gating**: Configuration structs derive `#[serde(deny_unknown_fields)]` (`crates/ruff_workspace/src/options.rs:50, 534, 695`), strictly failing deserialization on unknown options [V].
- **Advisory rule validation**: Unknown rule selectors are warned or gated behind preview flags (`crates/ruff_workspace/src/configuration.rs:109-111`) [V].
- **Schema emission**: Options structs derive `schemars::JsonSchema` (`options.rs:51`) to publish schemas for editor autocompletion and diagnostics [V].

### rumdl: Sourced Config Validation and Typestate Warnings

- **Typestate validation**: `SourcedConfig<ConfigLoaded>` transitions to `SourcedConfig<ConfigValidated>` via `validate_config_sourced` (`lsp_rvben-rumdl-src-digest.txt:24820-25275`) [V].
- **Advisory diagnostics with suggestions**: Validates rules and unknown keys against `RuleRegistry`, producing `Vec<ConfigValidationWarning>` with Levenshtein suggestions (`suggest_similar_key`, `digest:24832-24858`) [V].
- **Non-blocking consumption**: CLI prints warnings to stderr (`digest:12938`) and continues; LSP logs warnings (`digest:36161`) without failing execution [V].

---

## 2. Invariants and Invariant Owners

| Invariant | Owner | Enforcement Mechanism |
|---|---|---|
| **Value Representation Truthfulness** | Parser / Deserializer | Values reflect source syntax (string is string; number is number). No lossy guessing during raw ingest. |
| **Cross-Type Total Ordering** | Value Comparator | Deterministic type partition hierarchy (e.g. SQLite `NULL < Number < Text < Blob`). No local-timezone leaks or arbitrary string name sorting. |
| **Predictable Coercion Lattice** | Query Binder / Expression Engine | Documented widening rules (DuckDB `CastRules`). Implicit string-to-temporal promotion forbidden; explicit casts (`CAST`, `TRY_CAST`) required. |
| **Document Open-World Tolerance** | Note Parser & Indexer | Notes are user text. Markdown documents MUST parse and index regardless of schema conformance. Zero schema-gating at ingestion. |
| **Schema Conformance Reporting** | Analysis / Diagnostic Service | Independent pass evaluating parsed note AST against `Schema`. Emits structured diagnostics with AST byte spans. |

---

## 3. Tradeoffs and Costs Paid

- **Heuristic Sniffing vs Schema Discipline**:
  - *Heuristic parsing* (Dataview, Traces `TextShape::classify`) allows zero-config date sorting, but silently mutates strings looking like dates/durations (e.g. `title: 2026-07-29` or `time: 1h30m`) and creates ordering anomalies when non-dates sort chronologically [H].
  - *Explicit / Schema coercion* requires query functions (`date(due)`) or schema declarations, but guarantees predictability and preserves string semantics [H].
- **Validation Timing (Index-time vs Query-time vs On-Demand)**:
  - *Index-time validation*: Validating every note against schemas during indexing creates heavy coupling between `index` and `schema`, invalidates the index on schema edits, and slows down file-watcher reindexing [H].
  - *Query-time validation*: Re-checking note values on every query execution adds per-row evaluation overhead and pollutes query execution with diagnostic concerns [H].
  - *On-demand validation* (Taplo / rumdl pattern): Notes index unconditionally into storage; an independent analysis pass validates notes on demand (LSP background diagnostics, `traces check` CLI command), keeping storage fast and invariant-free [H].
- **Error Modes (Gating vs Diagnostic vs Null)**:
  - *Gating errors* (`deny_unknown_fields`, Polars `strict=True`) protect configuration integrity, but break user workflows if applied to open markdown notes [H].
  - *Null coercion* (`TRY_CAST`, Polars `strict=False`) protects query pipelines from crashing on malformed data, converting unparseable values to `null` [H].
  - *Advisory diagnostics* (Taplo, Metadata Menu) preserve full visibility of errors without degrading document availability or searchability [H].

---

## 4. Transferable Lessons for Traces

1. **Coercion belongs to the query binder and explicit functions, not deserialization or sorting [H]**:
   - Traces currently coerces strings to dates at three separate points: `FieldStringValue::classify` (`src/field.rs:843`), `SortKey::from_text` via `TextShape::classify` (`src/query/sort.rs:462`), and `ComparisonExpr::classify_literal` (`src/query/grammar/filter.rs:238`) [V].
   - Following DuckDB and Polars, frontmatter and inline fields should remain truthful strings unless coerced by a known schema field type or explicit query function (`date(field)`).
2. **Dual cast semantics for query filtering and aggregation [H]**:
   - Following DuckDB (`CAST` vs `TRY_CAST`) and Polars (`strict=true|false`), query expressions filtering on typed fields should support non-strict casting (converting unparseable frontmatter strings to `null` so queries continue executing) alongside strict casting when requested.
3. **Deterministic total ordering across disparate types [H]**:
   - SQLite proves that a fixed, static rank hierarchy (`Null < Bool < Number < Temporal < String < Link < List < Map`) provides consistent sorting without cross-type heuristic sniffing [H].
   - Dataview's approach of comparing type names alphabetically (`"date" < "duration" < "number" < "string"`) is an anti-pattern that creates counter-intuitive ordering [H].
4. **Advisory validation via Instance Path traversal to AST Spans (The Taplo Pattern) [H]**:
   - Traces should adopt the Taplo architecture: the note parser builds an AST (`Note`) preserving byte spans (`ByteSpan`); `SchemaService` provides field rules; an independent `AnalysisService` runs `validate(note, schema)` producing diagnostics with exact source spans for CLI (`traces check`) and LSP (`textDocument/publishDiagnostics`) [H].
5. **Serde gating for tool config vs open tolerance for notes [H]**:
   - Ruff's `#[serde(deny_unknown_fields)]` and rumdl's `validate_config_sourced` are ideal for `.traces/config.toml` and `.traces/schemas/*.toml` (fail fast on user typos) [H].
   - Frontmatter and inline fields in notes must remain open-world, using purely advisory validation [H].

---

## 5. Lessons That Do NOT Transfer and Why

1. **Dataview's Ingestion-Time Heuristic Sniffing**:
   - Dataview mutates frontmatter strings into luxon `DateTime` at parse time (`parseFrontmatter`). This prevents distinguishing between an intentional ISO string (e.g. an identifier or title) and a date, and forces downstream consumers to handle mixed types [H]. Traces should not adopt eager string classification.
2. **Dataview's Type-Tag Alphabetical Sort**:
   - In Dataview `Values.compareValue`, `wrap1.type.localeCompare(wrap2.type)` orders types alphabetically [V]. This makes `false` (boolean) sort before `1970-01-01` (date), which sorts before `100` (number), which sorts before `"a"` (string). Traces needs a semantic total order, not stringified JavaScript type tag sorting [H].
3. **Ruff's Gating Deserialization for Document Data**:
   - Ruff's `deny_unknown_fields` strictly errors on unknown fields [V]. Applying this to note frontmatter would cause notes with foreign or un-schematized YAML keys to fail parsing or disappear from the index.
4. **Direct Database Schema Constraints**:
   - Relational engines (Postgres, SQLite) reject rows violating table schemas upon `INSERT`. In PKM vaults, notes are created in external editors without database transactions; refusing to index an invalid note leaves the index out of sync with disk [H].

---

## 6. What This Changes or Challenges in the Traces Hypotheses

- **`survey/schema.md` [H1] (Schema subsystem is an isolated compiler; note data untyped and unvalidated)**:
  - *Affirmed & Refined [V/H]*: H1 correctly observes that 12k lines of schema logic never validate note contents. However, comparative precedent soundly refutes putting validation in the indexer or note parser.
  - *Adjustment*: Validation must not gate note ingestion. Instead, introduce an independent `SchemaValidator` or `AnalysisService` operating on `&Note` and `&Schema`, producing `Vec<SchemaDiagnostic>` with AST spans (the Taplo/Metadata Menu model) [H].
- **`survey/query.md` [H6] (Explicit vs Implicit Type Coercion / `TextShape::classify`)**:
  - *Affirmed & Refined [V/H]*: H6 argues that implicit promotion via `TextShape::classify` causes surprising ordering. The research confirms that DuckDB, Polars, and SQLite strictly avoid ambient string sniffing during comparisons.
  - *Adjustment*: Eliminate `TextShape::classify` from `SortKey` and `ComparisonExpr`. Replace with:
    1. Query functions for explicit conversion (`date()`, `dur()`) [H].
    2. Schema-informed field typing at query bind time (if a schema defines `due` as `Date`, the query knows to compare as Date) [H].
    3. Deterministic cross-type sort order where strings sort strictly after temporaries/numbers [H].
- **`survey/note.md` [H7] (Ambient Timezone Leak in Temporal Comparison)**:
  - *Affirmed [V/H]*: Comparing `Date` and `DateTime` by invoking local system timezone (`src/note/field.rs:297-300`) violates purity. SQLite and DuckDB require deterministic comparison: either convert `Date` to UTC midnight or partition `Date` and `DateTime` in the comparison lattice [H].

---

*Deliverable for issue #03: where mature systems put type coercion and validation.*
