# Precedent Research: Dynamic Values, Coercion Policy, and Schema Validation

**Ticket:** `.scratch/arch-redesign/issues/04-research-values-coercion-validation-precedent.md`  
**Date:** 2026-10-18  
**Scope:** Distillation of transferable precedent from external systems (SQLite, DuckDB, Postgres, Arrow/Polars, `serde_json`, `toml`, `jsonschema`, Alexis King) and local digests (Obsidian Dataview, Metadata Menu, rumdl) informing Traces redesign tickets [09](../issues/09-value-model.md), [10](../issues/10-coercion-policy.md), and [11](../issues/11-schema-note-typing-and-validation.md). Planning-only; zero changes to production code or tests.

---

## Transferable Lessons

### Lesson 1: Parse-Time Ingestion Sniffing vs. Query-Time Re-Sniffing

- **Informs:** Ticket [10] (Coercion policy), Ticket [09] (Value model)
- **Grade:** `VERIFIED`
- **Precedent:**
  - *Obsidian Dataview* parses and coerces types strictly during file ingestion in `parseFrontmatter` (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3678-3689`) and `parseInlineValue` (`id:3163-3175`), executing an ordered parsing cascade: ISO date (`EXPRESSION.date`) -> duration (`EXPRESSION.duration`) -> embed link -> raw string. Once parsed into `PageMetadata.fields`, values are fixed; query execution does not re-sniff or promote strings.
  - *Polars* infers column types at ingestion (`try_parse_dates=True` in CSV/JSON scanners). Once a column is ingested as `Utf8`, filter and sort expressions treat it strictly as text. Re-sniffing strings during query evaluation is explicitly rejected in favor of explicit `cast(pl.Date)` or `.str.to_date()`.
  - *Traces contrast:* Traces currently splits coercion: frontmatter parsing classifies dates via `DateTimeCoercion::Classify` in `impl From<noyalib::Value> for FieldValueRef` (`src/field.rs:772`), but the query layer re-sniffs strings via `TextShape::classify` in `query/sort.rs:462` and `filter.rs:85, 242` (`01 §2`, `02 §3`).
- **Transferability Test:**
  - *Shared:* Single-user synchronous processing; in-memory query evaluation over Markdown notes.
  - *Lacked:* Polars assumes homogeneous columns per DataFrame; Dataview runs in dynamic JavaScript without compile-time type safety.
  - *Traces Constraints (CLI & redb):* Across 20K notes, in-memory query scans must complete in microseconds (redb load floor ~16 ms, query scan ~50–100 μs per `03 §2`). Re-sniffing strings during sort and filter runs regex and calendar parsers $O(N \log N)$ or $O(N)$ times. Classifying at parse time and storing the resolved variant into redb preserves the microsecond query floor.
- **Negative Lesson:** Query-time string sniffing (`TextShape::classify`) creates behavioral divergence: a string sorting as a date in query results renders as raw text in minijinja templates, breaking symmetry between query filtering and template output. Coercion belongs at ingestion or behind explicit query functions.

---

### Lesson 2: Explicit Discriminant Rank vs. Lexicographical Type Strings for Cross-Type Ordering

- **Informs:** Ticket [09] (Value model)
- **Grade:** `VERIFIED`
- **Precedent:**
  - *SQLite* defines an explicit, fixed total order over storage classes (`datatype3.html §4.0`):
    $$\text{NULL} < \text{INTEGER/REAL} < \text{TEXT} < \text{BLOB}$$
  - *Obsidian Dataview* defines cross-type total ordering in `compareValue` (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:5470-5564`) by comparing type strings lexicographically:  
    `if (wrap1.type != wrap2.type) return wrap1.type.localeCompare(wrap2.type);`  
    Because string names dictate order (`"array" < "boolean" < "date" < "duration" < "function" < "html" < "link" < "number" < "object" < "string"`), any duration is arbitrarily "smaller" than any string (`"duration" < "string"`).
  - *Documented Dataview Failure Mode:* In Dataview FAQ (`docs/docs/resources/faq.md:5266-5275`), an unparsed inline expression string compared against a duration (`WHERE duration > dur("10h")`) evaluates to `true` because `"string" > "duration"` alphabetically, yielding nonsensical query results.
  - *Traces contrast:* Traces' `NoteFieldType::rank()` (`src/note/field.rs:209-254`) defines an explicit enum rank: `Null < Bool < Number < Duration < Temporal < Text < Link < List < Object`.
- **Transferability Test:**
  - *Shared:* Heterogeneous metadata properties where different notes hold different types for the same key.
  - *Lacked:* SQLite has 5 storage classes; Traces has 9 PKM types.
  - *Traces Constraints (CLI & redb):* Enum discriminant comparison is a single CPU instruction ($O(1)$ integer compare) that requires zero heap allocations and is deterministic across redb serialization.
- **Negative Lesson:** Lexicographical type-name ordering (Dataview) is an anti-pattern that creates subtle query bugs and unpredictable sorting. Cross-type total ordering must be defined by an explicit numeric discriminant rank. Furthermore, cross-type arithmetic in filters should produce errors or nulls rather than silently succeeding via type-rank comparisons.

---

### Lesson 3: Lossless Numeric Identity vs. Lossy Float Collapse

- **Informs:** Ticket [09] (Value model)
- **Grade:** `VERIFIED`
- **Precedent:**
  - *`serde_json::Value`* encapsulates numbers in an internal `Number` enum preserving `i64`, `u64`, and `f64` representation without collapsing all numbers to floating point.
  - *`toml::Value`* explicitly separates `Value::Integer(i64)` from `Value::Float(f64)`.
  - *SQLite* distinguishes `INTEGER` (signed 1, 2, 3, 4, 6, or 8 bytes) from `REAL` (8-byte IEEE floating point), preserving exact integers up to 64 bits.
  - *Traces contrast:* While `FieldValue` (`src/field.rs:591-592`) distinguishes `Int(i64)` and `Float(f64)`, `NoteFieldValue` (`src/note/field.rs:49`) collapses all numbers to `Number(f64)`. Furthermore, YAML parsing in `FieldValueRef::from(noyalib::Value)` calls `n.as_f64()` unconditionally (`01 §2 Claim 2`), destroying integer precision for values $> 2^{53}$ (e.g. snowflake IDs, large Unix timestamps) and necessitating clippy suppressions (`cast_precision_loss`).
- **Transferability Test:**
  - *Shared:* Dynamic schema values from YAML/TOML/JSON metadata files.
  - *Lacked:* JavaScript/Dataview represents all numbers as IEEE-754 doubles natively.
  - *Traces Constraints (redb & Postcard):* Postcard encodes integer variants compactly using variable-length varints (`1..9` bytes). Storing `Int(i64)` and `Float(f64)` in the core value model reduces redb storage size for small integers compared to fixed 8-byte `f64` and preserves exact bitwise equality for IDs and counters.
- **Negative Lesson:** Collapsing all numeric values into `f64` at the note boundary causes irreversible precision loss on 64-bit integer keys, prevents exact integer modulus/indexing, and creates unnecessary floating-point comparison fuzziness.

---

### Lesson 4: Schema as an Interpretive Overlay vs. Mutating Ingestion

- **Informs:** Ticket [10] (Coercion policy), Ticket [11] (Schema and note typing)
- **Grade:** `VERIFIED`
- **Precedent:**
  - *Obsidian Metadata Menu* separates raw file parsing from schema interpretation (`obsidian_mdelobelle-metadatamenu-src-digest.txt:2540-2603`). The note parser reads raw metadata into `ExistingField` (preserving strings as written). The schema subsystem (`fileClass`) then acts as an on-demand interpretive overlay: `fieldVM = fieldValueManager(...)`.
  - *Validation separation:* In Metadata Menu, validation is a non-blocking property on field info: `isValid: !fieldVM.value || validateValue(fieldVM.type)(fieldVM)` (`id:2580`). An invalid date or select value does NOT cause file rejection or parse abort; it remains visible, queries can inspect it, and the UI highlights non-conformance.
  - *Display formatting:* `displayValue` and `valueString` (`id:12251-12290`) apply the field's declared options (such as `dateFormat: "YYYY-MM-DD"`) dynamically at render time.
- **Transferability Test:**
  - *Shared:* Markdown notes are human-written documents with arbitrary frontmatter; schemas are optional, versioned, or user-defined.
  - *Lacked:* Metadata Menu relies on Obsidian's interactive UI and event loop.
  - *Traces Constraints (Vaults to 20K & Redb):* In a 20K vault, notes frequently violate schema definitions (partial notes, in-progress edits, legacy frontmatter). If schema validation is fused into note ingestion such that non-conforming notes fail parsing, users lose access to entire notes in queries and templates.
- **Negative Lesson:** Forcing note parsing to fail on schema validation errors destroys fault tolerance in text-based PKM vaults. Schemas must interpret and validate values as an overlay; the underlying note document model must remain capable of holding unrefined/untyped values.

---

### Lesson 5: Decentralized Schema Option Definitions with Centralized Validation Typestate

- **Informs:** Ticket [11] (Schema and note typing)
- **Grade:** `VERIFIED`
- **Precedent:**
  - *rumdl* implements a typed-options-per-owner pattern (`lsp_rvben-rumdl-src-digest.txt:9560-9585`, `9800-9832`, `25055-25124`). Each rule implements `config_schema() -> Option<(String, toml::Value)>`, declaring its expected keys and types via helper macros (`impl_default_config!`).
  - *Centralized Registry & Typestate:* The central `RuleRegistry` queries rules for their schemas and runs a unified validator (`validate_config_sourced_impl`). Config loading uses a typestate transition:  
    `SourcedConfig<ConfigLoaded>` $\xrightarrow{\text{validate}}$ `SourcedConfig<ConfigValidated>` $\rightarrow$ `Config`.
  - The central engine checks unknown keys and validates value types against defaults (`toml_value_type_matches`), while individual rules own their domain semantics.
- **Transferability Test:**
  - *Shared:* Single-user synchronous CLI; TOML-based configuration and schema declarations; strict DAG architecture.
  - *Lacked:* rumdl validates tool configuration options, whereas Traces validates note instance metadata against user-authored note schemas.
  - *Traces Constraints (Module Graph):* `src/schema/` currently has zero references to `note` types (`01 §4`). Keeping schema definitions pure and using a separate validator/typestate pass prevents circular dependencies between `note`, `schema`, and `query`.
- **Negative Lesson:** Hardcoding validation rules or field-type mappings inside the central parser couples every new schema feature directly to the parser. Conversely, letting schemas validate arbitrary note ASTs directly couples `schema` to `note`. A central validator operating over a shared value model preserves module isolation.

---

### Lesson 6: Dynamic Diagnostic Validation vs. "Parse, Don't Validate" Construction

- **Informs:** Ticket [11] (Schema and note typing), Ticket [09] (Value model)
- **Grade:** `ECOSYSTEM-CONSENSUS`
- **Precedent:**
  - *Alexis King ("Parse, don't validate", 2019):* In type-driven design, parsing transforms unrefined input into a typed representation where illegal states are unrepresentable ($A \rightarrow \text{Result}<B, E>$), whereas validation merely checks boolean conditions ($A \rightarrow \text{Result}<(), E>$), leaving consumers to handle unrefined data.
  - *`jsonschema` / `boon` API:* Both crates validate dynamic data (`&serde_json::Value`) against a compiled schema, returning a collection of `ValidationError` diagnostics without transforming or constructing new types.
  - *`serde_valid` / `validator`:* Performs declarative validation on already-deserialized, strongly-typed structs.
- **Transferability Test:**
  - *Shared:* Traces requires both diagnostics for human feedback (CLI check/doctor, LSP diagnostics) and typed values for engine execution (query filters, sorting, date math).
  - *Lacked:* Traditional "parse, don't validate" works best when invalid inputs can be rejected outright (e.g. HTTP APIs returning 400 Bad Request).
  - *Traces Constraints (PKM Vaults & Queries):* In Traces, invalid frontmatter cannot abort the CLI. If an unrefined note cannot be represented, `traces list` or `traces query` cannot operate on the note. Therefore, Traces requires a two-layer model:
    1. A generic, unrefined dynamic value tree stored in the note model (resilient to schema errors).
    2. A schema-guided interpretation pass ("parsing" in the Alexis King sense) that attempts to construct typed domain projections (`TypedValue`), returning diagnostics when interpretation fails without dropping the underlying raw value.
- **Negative Lesson:** Applying pure "parse, don't validate" directly to note ingestion creates an all-or-nothing cliff: a note with a single invalid date field cannot be parsed into a `Note`, breaking vault-wide indexation. Diagnostic validation must remain separable from note ingestion.

---

### Lesson 7: Native Temporal Representation vs. Heuristic Loss and Dead Formatting

- **Informs:** Ticket [09] (Value model), Ticket [10] (Coercion policy)
- **Grade:** `VERIFIED`
- **Precedent:**
  - *TOML 1.1 Specification & `toml::value::Datetime`:* Natively distinguishes four temporal forms: Offset Date-Time, Local Date-Time, Local Date, and Local Time. It preserves the exact syntax provided by the author.
  - *`jiff`:* Distinguishes `civil::Date`, `civil::Time`, `civil::DateTime`, and `Zoned`. Preserves exact calendar units without forcing UTC conversion or ambient timezone reads.
  - *Dataview Heuristic Flaw:* Dataview converts all dates to Luxon `DateTime` (`value.ts:5426-5434`). At render time, it guesses whether the value was originally a date or datetime via zero-checking:  
    `if (second == 0 && hour == 0 && minute == 0) return toFormat(defaultDateFormat);`  
    Consequently, any datetime occurring at exact midnight (`2026-10-18T00:00:00`) is rendered as a bare date (`2026-10-18`), erasing user time data.
  - *Traces contrast:* In Traces, `SchemaDateField.format` is dead configuration (`01 §4 Claim 4`, getter is `#[cfg(test)]`), and `DatePoint` unconditionally drops written precision (month/year) upon conversion to `NoteFieldValue` (`01 §2 Claim 2`).
- **Transferability Test:**
  - *Shared:* Users expect dates, datetimes, and durations to be parsed accurately from frontmatter strings.
  - *Lacked:* Luxon runs in JS; `jiff` is high-performance Rust with zero-allocation civil date types.
  - *Traces Constraints (Ambient Zone & Sort):* Ticket `01 §3` showed `local_zone()` is reached at 24 non-test sites; research 03 measured that sort keys are precomputed once per row, so the ambient lookup is not in the sort compare loop (whether cross-kind `NoteFieldValueRef::compare` reaches it per comparison is unchecked). Native civil temporal representations (`Date` vs `DateTime`) allow deterministic sorting and eliminate ambient clock access.
- **Negative Lesson:** Heuristically guessing date vs datetime based on zeroed time fields (Dataview) or dropping written calendar precision (Traces' current `DatePoint` collapse) causes silent data loss. Temporal variants must be explicitly distinguished in the value enum.

---

### Lesson 8: Unified Dynamic Value Tree vs. Redundant Multi-Tree Conversion Chains

- **Informs:** Ticket [09] (Value model)
- **Grade:** `VERIFIED`
- **Precedent:**
  - *`serde_json::Value`* serves as the single dynamic value representation across JSON, YAML (via `serde_yaml`/`serde-yml`), and downstream toolchains, avoiding redundant intermediate conversion trees.
  - *DuckDB / Arrow:* Columnar data uses unified physical vector representations with logical type overlays, avoiding copying between subsystem-specific representations.
  - *Traces contrast:* Traces currently maintains multiple parallel value representations:
    1. `FieldValue` / `FieldValueRef` (`src/field.rs`): generic, holds `Int`/`Float`, used by schema options.
    2. `NoteFieldValue` / `NoteFieldValueRef` (`src/note/field.rs`): Markdown-specific, holds `Duration` and `Link`, collapses `Number(f64)`, owns total order.
    3. `QueryFieldValueRef` (`src/query/value.rs`): wraps note field value and adds query-specific projections (`Tags`, `Inlinks`, `Owned`).
    - Note frontmatter load traverses `noyalib::Value -> FieldValueRef -> NoteFieldValue` (`01 §2`), and template evaluation converts `NoteFieldValue -> minijinja::Value` (`src/template/engine/query.rs:598`). `FieldStringValue` has zero external consumers.
- **Transferability Test:**
  - *Shared:* Both schemas and notes need dynamic key-value storage.
  - *Lacked:* General-purpose libraries like `serde_json` lack domain PKM primitives (`Link`, `Duration`).
  - *Traces Constraints (redb & Benchmarks):* Criterion benchmarks (`03 §2`) show clean index build parses thousands of notes in parallel. Multiplying intermediate AST hops during note ingestion wastes CPU cycles and allocations on conversion glue.
- **Negative Lesson:** Creating separate value trees (`FieldValue` vs `NoteFieldValue`) to isolate schemas from Markdown-specific types (like `Link` or `Duration`) produces maintenance overhead, conversion boilerplate, and lossy intermediate hops without architectural benefit. A single core dynamic value tree with optional domain variants satisfies all consumers.

---

## Provenance Ledger

| Lesson | Exemplar System | Source File / Reference | Finding Grade | Informs Ticket |
|---|---|---|---|---|
| 1 | Obsidian Dataview | `docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3163-3175, 3678-3689` | `VERIFIED` | [10], [09] |
| 1 | Polars / DuckDB | Polars Ingestion Docs (`try_parse_dates`), DuckDB Cast Lattice (`sql/expressions/cast`) | `ECOSYSTEM-CONSENSUS` | [10] |
| 2 | SQLite | SQLite Documentation `datatype3.html §4.0` (Comparison Expressions) | `ECOSYSTEM-CONSENSUS` | [09] |
| 2 | Obsidian Dataview | `docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:5470-5564`, `docs-digest:5266-5275` | `VERIFIED` | [09] |
| 3 | `serde_json`, `toml`, SQLite | `serde_json::Value` (Number), `toml::Value` (Integer/Float), SQLite storage classes | `VERIFIED` | [09] |
| 4 | Obsidian Metadata Menu | `docs/digests/obsidian_mdelobelle-metadatamenu-src-digest.txt:2540-2603, 12251-12290` | `VERIFIED` | [10], [11] |
| 5 | rumdl | `docs/digests/lsp_rvben-rumdl-src-digest.txt:9560-9585, 9800-9832, 25055-25124` | `VERIFIED` | [11] |
| 6 | Alexis King / `jsonschema` | Alexis King ("Parse, don't validate", 2019); `jsonschema::validate` (`src/lib.rs:1445`) | `ECOSYSTEM-CONSENSUS` | [11], [09] |
| 7 | TOML 1.1 / Dataview | TOML 1.1 Datetime Spec; Dataview `obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:5426-5434` | `VERIFIED` | [09], [10] |
| 8 | Traces Core Verification | `.scratch/arch-redesign/research/01-verify-core-findings.md:Claim 2 (53-99)` | `VERIFIED` | [09] |
