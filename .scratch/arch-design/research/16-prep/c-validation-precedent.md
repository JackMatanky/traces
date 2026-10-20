# External Precedent: Declarative Schema Validation of Semi-Structured & Open-World Data

- **Topic:** External precedent across language ecosystems, databases, query engines, and validation frameworks for Wayfinder Ticket 16 (Schema Note Typing and Validation).
- **Status:** Complete research deliverable.
- **Deliverable:** `.scratch/arch-design/research/16-prep/c-validation-precedent.md`.
- **Relationship to prior prep:** Extends `research/03-coercion-validation-precedent.md` and `research/15-prep/c-external-precedent.md` without duplicating SQLite affinity or basic DuckDB casting tables.

---

## 1. JSON Schema as Practiced & CST Diagnostic Mapping

### What It Does & How It Models Schema vs Data

- **Specification decoupled from data model:** JSON Schema (Draft 2020-12, RFC Draft) validates arbitrary tree structures (`serde_json::Value` in Rust) against independent declarative schema descriptors [V: 2020-12 §4]. Data retains its native structural types (object, array, string, number, boolean, null); the schema overlays semantic assertions (e.g. `type: "string"`, `format: "date"`, `minimum: 0`, `pattern: "..."`).
- **Rust implementation (`jsonschema` crate v0.58.6):**
  - Validation execution: `jsonschema::JSONSchema::validate(&self, instance: &Value)` returns an iterator of `ValidationError<'a>` (`jsonschema/src/error.rs:58`) [V].
  - Fast-path checking: `is_valid(&self, instance: &Value) -> bool` (`src/evaluation/mod.rs`) short-circuits on the first failure without allocating error descriptors [V].
  - Standard output formats (`jsonschema::evaluation`): implements the 2020-12 specification output formats: `FlagOutput` (boolean pass/fail), `ListOutput` (Basic format: flat list of errors with locations), and `HierarchicalOutput` (Detailed format: nested evaluation tree) [V: `jsonschema::FlagOutput`, `ListOutput`, `HierarchicalOutput`].
  - Structured error fields: `ValidationError` exposes `instance_path: &Location` (RFC 6901 JSONPointer path into instance, e.g. `/metadata/due`), `schema_path: &Location` (canonical schema keyword path), `evaluation_path: &Location` (dynamic path including `$ref`), and `kind: &ValidationErrorKind` (`src/error.rs:440-463`) [V].
- **Source spans in `jsonschema`:** `jsonschema` carries **zero source spans** [V]. It operates strictly on `serde_json::Value`, which discards byte offsets during lexical parsing.

### How Language Servers Bridge Schema to CST Source Spans

- **Attachment mechanisms:**
  - `yaml-language-server` (`https://github.com/redhat-developer/yaml-language-server`): associates schemas via glob patterns in user config (`yaml.schemas`), catalog URLs (`SchemaStore`), or in-file modelines (`# yaml-language-server: $schema=<url>`) [V].
  - `taplo` (TOML toolkit / LSP, `https://taplo.tamasfe.dev/`): associates TOML schemas via `.taplo.toml` glob associations or `# taplo:schema=<url>` [V].
- **Span resolution via CST traversal:**
  - In `taplo` (`taplo-common/src/schema/mod.rs:90-148, 680-717`): `Schemas::validate_root` validates JSON DOM via `jsonschema`. When `ValidationError` returns an `instance_path` (e.g. `/author/name`), `taplo` walks the path segments down its Rowan concrete syntax tree (`taplo::dom::Node`), locates the exact syntax token, and extracts its byte span (`rowan::TextRange`), converting it to line/column LSP coordinates [V].
  - In `yaml-language-server`: `vscode-json-languageservice` validates a parsed YAML AST (`yaml-ast-parser`). Error JSONPointers are resolved against AST node offsets to emit `lsp_types::Diagnostic` with precise line/column ranges and configurable severities (`error`, `warning`, `ignore`) [V].

### `$ref`/`allOf` Inheritance Semantics vs Traces `extends` + `$ref`

- **JSON Schema `$ref` & `allOf`:**
  - `$ref` is an applicative pointer: evaluation jumps to the referenced schema definition [V: 2020-12 §7.7].
  - Composition / inheritance uses `allOf: [ { "$ref": "#/defs/Base" }, { "properties": { ... } } ]`. In JSON Schema, `allOf` is **pure logical conjunction** [V]. Constraints accumulate; child schemas **cannot override or widen** base constraints. If `Base` declares `type: "string"` and child declares `type: "integer"`, `allOf` produces an unsatisfiable schema (`false`) rather than an override [H].
- **Traces `extends` + `$ref` (`src/schema`):**
  - Traces models object-oriented inheritance via `extends = ["base"]` (`src/schema/types.rs`) [V]. Child field definitions explicitly **shadow and override** base fields by name [V].
  - `$ref` in Traces is restricted to pulling reusable field definitions from `.traces/schemas/globals.toml` or other schemas (`$ref = "global.due"`) [V].
- **Advisory vs Gating:** JSON Schema validators and schema LSPs are strictly advisory diagnostics; non-conforming documents format and parse without error [V].
- **Cost:** Compiling a schema is $O(S)$ where $S$ is schema size. Evaluating `is_valid` is $< 1\,\mu\text{s}$ per small JSON instance; collecting all errors via `iter_errors` costs $2\text{--}10\,\mu\text{s}$ per instance due to pointer tracking [H].
- **Lesson for Traces:** Decouple schema validation from parser/storage. Keep note AST untyped and span-preserving; evaluate validation as an independent pass mapping schema error paths to note AST `ByteSpan` tokens [P].
- **What Disproves the Lesson:** If note queries need low-level SIMD columnar filtering or disk-level index sorting, untyped AST storage forces repetitive runtime deserialization and parsing on every query invocation [H].

---

## 2. Frictionless Table Schema: Typed Columns Over Loosely-Typed Text

### What It Does & How It Models Schema vs Data

- **The definitive standard for text-to-type coercion:** Frictionless Table Schema (`https://specs.frictionlessdata.io/table-schema/`) standardizes how to declare types and constraints over inherently string-based tabular data (CSV, TSV, flat text) [V].
- **Physical vs Logical Data Model:**
  - *Physical representation:* raw text on disk (strings in CSV or YAML frontmatter) [V: Table Schema § Concepts].
  - *Logical representation:* typed values (integers, dates, booleans) resulting from applying `type`, `format`, and `missingValues` [V].
  - `missingValues` (`[""]`, `["-"]`, `["NaN"]`): applies to the **physical** representation *before* casting, converting matched strings to logical `null` [V: § Other Properties].
  - `constraints` (`required`, `unique`, `minimum`, `maximum`, `pattern`, `enum`, `minLength`, `maxLength`): evaluate strictly against the **logical** representation *after* type casting [V: § Constraints].
- **Rich Types & Formats:**
  - Date/time types: `date`, `time`, `datetime`, `duration`, `year`, `yearmonth` [V: § Types and Formats]. Supports explicit strptime `<PATTERN>` (e.g. `"%d/%m/%Y"`) and ISO-8601 default [V].
  - Numbers: `bareNumber: false` allows leading/trailing characters (`95%`, `$100`); `decimalChar` and `groupChar` control localization [V].
- **Schema Synchronization (`schema_sync`):**
  - In `frictionless-py` (`v4.framework.frictionlessdata.io`), `detector.schema_sync = True` dynamically matches data columns to schema field descriptors by name rather than positional order, tolerating reordered or missing optional columns in the text source [V].

### Error Taxonomy

Frictionless Framework categorizes errors into a clean, hierarchical taxonomy (`https://v4.framework.frictionlessdata.io/docs/references/errors-reference`) [V]:

1. `type-error`: value cannot be cast to declared type and format (`cell`, `fieldName`, `fieldPosition`, `rowPosition`) [V].
2. `constraint-error`: value fails logical constraint (`minimum`, `maximum`, `pattern`, `enum`) [V].
3. `missing-cell` / `extra-cell`: row has fewer or more fields than expected [V].
4. `unique-error`: duplicate value encountered on a unique-constrained field [V].

- **Advisory vs Gating:** In Frictionless, validation is an analysis report (`frictionless validate`); it reports tables of errors with row/column coordinates without mutating or destroying data files [V].
- **Cost:** Casting strings to logical types on read costs $50\text{--}300\,\text{ns}$ per cell in Rust [H].
- **Lesson for Traces:** Notes with frontmatter are identical to CSV rows: raw physical text parsed into logical fields. Traces should adopt Frictionless's two-stage pipeline: (1) physical missing-value conversion (`""` -> `Null`), (2) logical casting & constraint checking [P]. Field errors should distinguish between `type_error` (unparseable date) and `constraint_error` (valid date, but before `minimum`) [P].
- **What Disproves the Lesson:** Frictionless assumes fixed columnar schemas where all rows share the same table descriptor. Markdown vaults are heterogeneous: different notes declare different `fileClass`es or lack schemas entirely; columnar assumptions break on deep nested frontmatter objects [H].

---

## 3. Python & Rust Validation Paradigms & Diagnostic Representations

### Pydantic v2: Lax vs Strict Modes & `ValidationError` Structure

- **Lax vs Strict (`pydantic.dev/docs/validation/2.11/errors/errors/`):**
  - *Lax mode (default):* Pydantic coerces compatible shapes (`"42"` -> `42`, `"true"` -> `True`, `42.0` -> `42`) to accommodate JSON/HTTP boundaries [V].
  - *Strict mode (`strict=True`):* forbids implicit coercion; `"42"` fails type-check for `int` (`int_type`) [V].
- **`ValidationError.errors()` structure:** returns a list of `ErrorDetails` dictionaries (`src/pydantic_core`):
  - `loc: tuple[str | int, ...]`: path to the field (e.g. `("author", "address", 0, "zip")`) [V].
  - `msg: str`: human-readable explanation [V].
  - `type: str`: machine-readable error discriminator (e.g. `int_parsing`, `string_too_short`, `missing`) [V].
  - `input: Any`: the raw un-coerced value provided [V].
  - `ctx: dict[str, Any]`: optional variables used in formatting the message [V].
  - `url: str`: link to documentation [V].

### Rust Struct Validation: Deserialization vs Separate Pass

- **Serde "Deserialize into Typed":**
  - Standard Serde (`serde::Deserialize`) is all-or-nothing: deserialization fails at the first unparseable field via `serde::de::Error::custom`, aborting the parse [V: `serde` docs].
  - `#[serde(deny_unknown_fields)]`: strictly gates unknown keys; unusable for open-world frontmatter where plugins and users add arbitrary keys [V].
  - `serde_valid`: derives validation checks directly into deserialization; still forces an all-or-nothing boundary before an AST can exist [V].
- **Rust Independent Validation Crates:**
  - `garde` (`v0.23.0`): validates existing Rust structs. `garde::Report` (`src/error.rs:23`) stores a flat list of `(Path, Error)` pairs [V: `garde::error::Report`]. Path is a compact list of field names and indices [V].
  - `validator` (`v0.21.0`): `ValidationErrors` (`src/types.rs:50`) stores a hierarchical tree: `HashMap<Cow<'static, str>, ValidationErrorsKind>`, with `Struct(Box<ValidationErrors>)` and `List(BTreeMap<usize, Box<ValidationErrors>>)` [V: `validator::ValidationErrors`].
  - `schemars` (`v1.2.3`): generates JSON Schema from Rust structs via derive macro; decouples schema publication from runtime validation [V].

### Diagnostic Crates & `lsp_types::Diagnostic`

- **Compiler/CLI Diagnostic Crates:**
  - `miette` (`v7.6.0`, `src/protocol.rs:20-70`): rich terminal error reporting. `Diagnostic` trait defines `code`, `severity` (`Advice`, `Warning`, `Error`), `help`, `url`, `source_code`, `labels` (`LabeledSpan` with byte offsets and highlights), and `related` diagnostics [V].
  - `codespan-reporting` / `ariadne`: lower-level ASCII/ANSI formatting; emits compiler-style reports with primary and secondary byte spans [V].
- **LSP Standard Diagnostic (`lsp_types::Diagnostic`, v0.97.0, `src/lib.rs:356-398`):**
  - `range: Range`: start/end line and UTF-16 character offsets [V].
  - `severity: Option<DiagnosticSeverity>`: 1 (Error), 2 (Warning), 3 (Information), 4 (Hint) [V].
  - `code: Option<NumberOrString>`: machine-readable identifier (e.g. `"traces::schema::type_error"`) [V].
  - `code_description: Option<CodeDescription>`: URL for the error code (`href: Uri`) [V].
  - `source: Option<String>`: origin linter/engine (`"traces"`) [V].
  - `message: String`: human-readable description [V].
  - `related_information: Option<Vec<DiagnosticRelatedInformation>>`: links secondary locations (e.g. "field defined here in schema.toml") [V].
  - `tags: Option<Vec<DiagnosticTag>>`: Unnecessary (dimmed) or Deprecated (strikethrough) [V].
  - `data: Option<serde_json::Value>`: preserved between diagnostic emission and `textDocument/codeAction` for quick fixes [V].

### The Least-Regret Diagnostic Shape for Traces (CLI + LSP + MCP)

```rust
pub struct Diagnostic {
    pub file_path: PathBuf,
    pub span: Option<ByteSpan>,       // byte offset range into file
    pub severity: DiagnosticSeverity, // Error, Warning, Info, Hint
    pub code: &'static str,           // stable string identifier
    pub message: String,              // human description
    pub help: Option<String>,         // actionable suggestion
    pub related: Vec<RelatedLocation>,// e.g. schema definition site
    pub fix: Option<SuggestedFix>,    // edit payload for LSP CodeAction / MCP
}
```

- **Why least-regret [P]:**
  1. CLI: renders cleanly via `miette` or `codespan-reporting` using `span` and `help`.
  2. LSP: converts losslessly to `lsp_types::Diagnostic` (`span` -> `Range`, `severity` -> `DiagnosticSeverity`, `related` -> `related_information`).
  3. MCP: serializes directly to JSON for coding agents without losing structure.
- **Lesson for Traces:** Never use Serde deserialization to validate notes. Deserialize into an open AST; run validation as an out-of-band pass emitting the unified `Diagnostic` shape [P].
- **What Disproves the Lesson:** If Traces never implements an LSP or interactive fixer, a rich `Diagnostic` struct is dead weight compared to standard `anyhow::Result` / `eprintln!` [H].

---

## 4. Data Warehousing & Gradual Validation: Write-Path Gating vs Read-Path Tolerance

### dbt Tests & Data Contracts

- **dbt tests (`https://docs.getdbt.com/reference/resource-configs/severity`):**
  - Decoupled SQL-based assertions executed against existing warehouse tables [V].
  - `severity: warn | error`: configurable thresholds (`warn_if: ">10"`, `error_if: ">100"`) [V].
  - `store_failures: true`: writes violating records into an audit schema table (`dbt_test__audit`) for forensic inspection without halting data transformations [V: `store_failures` docs].
- **dbt Model Contracts (`contract: { enforced: true }`, `https://docs.getdbt.com/docs/mesh/govern/model-contracts`):**
  - **Write-path gating:** dbt compiles the contract into physical DDL (`CREATE TABLE ...`); if the transformation query outputs a column with a mismatched datatype, the build fails and never materializes the table [V].

### Database Constraints: Write Time vs Read Time

- **SQLite STRICT tables (v3.37.0+):** `CREATE TABLE t(x INT) STRICT;` disables type affinity; inserting text into `INT` throws `SQLITE_CONSTRAINT_DATATYPE` [V]. Without STRICT, SQLite silently converts strings to numbers or falls back to text storage [V: research 03].
- **DuckDB / PostgreSQL Constraints:** Write-time operations (`INSERT`) strictly reject constraint violations. On read-time external data (e.g. `read_csv`), DuckDB provides `ignore_errors = true` or `TRY_CAST` to convert unparseable values to `NULL` without aborting the query [V: DuckDB docs].

### Cue, Dhall, and Nickel: Gradual Configuration Validation

- **Dhall:** Total functional language; strictly typed upfront. Zero partial/invalid values admitted [V: Dhall spec].
- **Cue:** Value lattice where types and values inhabit the same domain. Constraints unify gradually (`&`); incomplete or partial records are valid until final export [V: Cue spec].
- **Nickel:** Gradual typing with **lazy contracts** [V: Nickel docs]. Type checking occurs at static boundaries; dynamic contracts (`Record`, `Array`) evaluate lazily when fields are accessed at runtime.

### The Invariant: Write Path vs Read Path

| Path | Ownership | Optimal Policy | Consequence of Inversion |
|---|---|---|---|
| **Write Path** (`traces new`, `traces template`, UI modal) | Traces owns output | **Gating:** refuse to create/write malformed files. | Users write broken files and expect the tool to clean them up. |
| **Config Path** (`.traces/config.toml`, `.traces/schemas/*.toml`) | Traces owns syntax | **Gating:** fail fast on typos (`deny_unknown_fields`). | Typos in schemas silently corrupt downstream indexing. |
| **Read / Index Path** (Markdown vault on disk) | User / external editors own data | **Advisory:** parse unconditionally; emit diagnostics. | Gating bricks the vault: an external typo locks user out of searching 10,000 valid notes. |

- **Lesson for Traces:** Never gate note indexing on schema compliance. Gating belongs strictly to interactive commands (`traces new`, template execution) and config files. Ingestion must remain 100% open-world [P].
- **What Disproves the Lesson:** If users intentionally treat Traces as a strict headless database where invalid records must never be queried, advisory indexing violates ACID expectations [H].

---

## 5. Ingestion-Time vs Read-Time Typing in Open-World Search Engines

### Elasticsearch: Dynamic Mapping, Conflicts, and `ignore_malformed`

- **Dynamic mapping & conflicts (`https://www.elastic.co/guide/en/elasticsearch/reference/current/mapping.html`):**
  - Elasticsearch infers field datatypes dynamically from the first ingested document [V].
  - **Mapping conflict:** if doc 1 has `{"age": 30}` (inferred as `long`), and doc 2 arrives with `{"age": "thirty"}`, Elasticsearch throws `mapper_parsing_exception` and **rejects the entire document** [V].
- **Remediation parameters:**
  - `coerce: true` (default): attempts coercion (e.g. `"30"` -> `30`) [V: `coerce` docs].
  - `ignore_malformed: true`: if a field value cannot be parsed into its declared type, the exception is swallowed; the malformed field is dropped from search indexing, but the rest of the document is indexed successfully [V: `ignore_malformed` docs].
  - `_ignored` field: Elasticsearch records the names of malformed ignored fields in an internal metadata array (`_ignored`) for diagnostic auditing [V].

### MongoDB: `$jsonSchema` Validation Actions and Levels

- **MongoDB Schema Validation (`https://www.mongodb.com/docs/manual/core/schema-validation/`):**
  - `validationAction: "error"` (default): rejects non-conforming writes [V].
  - `validationAction: "warn"`: permits non-conforming writes, but writes a violation entry into the `mongod` diagnostic log [V].
  - `validationLevel: "strict"`: validates all inserts and updates [V].
  - `validationLevel: "moderate"`: validates inserts and updates to documents that *already conform* to the schema, but permits updates to pre-existing non-conforming documents without blocking [V]. This prevents legacy data lock-out when schemas evolve [H].

### Apache Solr Schemaless Mode

- Solr detects field types dynamically and mutates `managed-schema.xml`. When conflicting types occur, Solr either fails or automatically broadens the field to a multi-valued string type [V: Solr Reference Guide].
- **Advisory vs Gating:** Both Elasticsearch (`ignore_malformed: true`) and MongoDB (`validationAction: "warn"`, `validationLevel: "moderate"`) prove that production document engines operating over semi-structured data *must* provide non-blocking escape hatches to prevent ingest rejection [V].
- **Lesson for Traces:** If Traces ever adopts schema-driven typing at index time, it **must** implement Elasticsearch's `ignore_malformed` rule: unparseable fields fall back to raw string storage (or `_ignored` list), keeping the note fully searchable [P].
- **What Disproves the Lesson:** `ignore_malformed` creates invisible data loss: a query `WHERE due < 2026-10-20` silently ignores a note with `due: "2026-10-200"` because it was dropped from the date index [H].

---

## 6. Incremental Validation, Invalidation Topology, and Cost Model

### Invalidation Topology: Note Mutation vs Schema Mutation

| Event | Scope of Invalidation | Recomputation Required |
|---|---|---|
| **Note file updated** (`note.md`) | 1 document | Re-parse note frontmatter AST (~$0.1\,\text{ms}$). Run validator against cached schema (~$2\,\mu\text{s}$). Update cached diagnostics for this file. **Zero index B-tree re-layouts.** |
| **Schema file updated** (`schema.toml`) | All notes inheriting schema | Recompile schema graph ($< 1\,\text{ms}$). If validation is decoupled: invalidate diagnostic cache; re-validate on next `check` or active LSP buffer. If typing was baked into index: **massive re-index storm** (rewrite Redb tables for thousands of notes). |
| **Global schema pool updated** | Vault-wide | Recompile CSR graph. Decoupled validation re-evaluates lazily; index remains stable. |

### Cost Model Sketch for Validating $N$ Notes with $M$ Fields

- **Parameters:**
  - $N = 10,000$ notes (typical upper-mid PKM vault).
  - $M = 15$ frontmatter fields per note. Total field evaluations = $150,000$.
- **Per-field validation cost (in Rust):**
  - Hash lookup for field rule in compiled schema: ~$15\,\text{ns}$ (or ~$5\,\text{ns}$ with `rustc-hash::FxHashMap` / perfect hash).
  - Variant / type matching (e.g. check if `NoteFieldValue` matches `SchemaFieldType`): ~$10\,\text{ns}$.
  - Constraint check (e.g. regex pattern match or range check): ~$50\text{--}150\,\text{ns}$.
  - Total cost per field: ~$\mathbf{100\,\text{ns}}$.
- **Per-note validation cost ($M = 15$ fields):**
  - $15 \times 100\,\text{ns} \approx \mathbf{1.5\,\mu\text{s}}$ per note.
- **Whole-vault validation cost ($N = 10,000$ notes):**
  - Single-core sequential evaluation: $10,000 \times 1.5\,\mu\text{s} = \mathbf{15\,\text{ms}}$!
  - Parallel evaluation with `rayon` (8 cores): $\mathbf{\approx 2.5\,\text{ms}}$!
- **Disk I/O comparison:**
  - Reading and parsing 10,000 markdown files from SSD: $200\text{--}800\,\text{ms}$.
  - Parsing YAML/frontmatter AST: $50\text{--}150\,\text{ms}$.
  - Pure schema validation over already-parsed ASTs represents **$< 5\%$** of the total execution budget [H].

### Lazy vs Precomputed Validation

- **CLI (`traces check` / `doctor`):** Whole-vault validation runs on demand in ~15 ms. Precomputing or persisting validation diagnostics to disk is unnecessary complexity [P].
- **LSP (`textDocument/publishDiagnostics`):** Compute diagnostics **strictly on the active/open buffer** ($< 2\,\mu\text{s}$). The language server does not need background whole-vault diagnostic loops; workspace diagnostics (`workspace/diagnostic`) can run on demand [P].
- **Query execution (`traces query`):** Queries only filter/project specific fields (e.g. `WHERE due < '2026-10-20'`). Validating all 15 fields on every row during a query is waste. Query binding should type-check only referenced fields [P].

---

## 7. Shortlist of Candidate Design Shapes for Traces

| Design Shape | Validator Owner | Diagnostic Type | Typed-Field Representation | Contradicting Precedent | Adversarial Failure Mode for 1k-20k Vault |
|---|---|---|---|---|---|
| **Shape 1: Decoupled Advisory Analysis Service** *(Taplo / Metadata Menu / LSP model)* | Independent `AnalysisService` or `LintService` | Canonical `Diagnostic` struct (byte span, severity, code, message, help) | Unchanged `NoteFieldValue` in AST and index (untyped/text-preserving). Type coercion deferred to query binder. | **SQLite STRICT / dbt contracts:** allowing malformed data into index causes silent query bugs downstream. | Users write `due: 2026-02-31`; query sort behaves inconsistently because the invalid date was not normalized or rejected at index time. |
| **Shape 2: Schema-Informed Ingest with `ignore_malformed`** *(Elasticsearch / Solr model)* | `IndexService` (ingestion pipeline) | Ingestion diagnostics stored in index metadata (`_ignored`) | Schema-typed `NoteFieldValue` stored directly in Redb tables; malformed fields fall back to text. | **Taplo / Obsidian:** coupling index to schemas creates invalidation storms on schema edits and breaks out-of-order indexing. | Editing `.traces/schemas/base.toml` triggers re-indexing of 15,000 notes on disk, locking Redb and causing 3-second lag in CLI/LSP. |
| **Shape 3: Passive Schema** *(Keep-as-Is; Template / Prompt metadata only)* | No note validator (UI prompt validator only) | None | Schema types exist only in `src/schema` for UI prompt widgets. Notes remain completely untyped dynamic values. | **All precedents (Taplo, Frictionless, Pydantic, dbt):** users who author schemas expect editor diagnostics and CLI checks. | 12k LoC in `src/schema` remains dead code to note validation; users get zero feedback when frontmatter typos accumulate. |
| **Shape 4: Hybrid: Query-Bind Typing + Advisory Linter** *(DuckDB / Polars model)* | Query Binder owns runtime type coercion; `AnalysisService` owns advisory diagnostics | Canonical `Diagnostic` for linter; `EvaluationError` / `Null` for queries | Truthful raw values in storage. Query planner coerces fields based on schema at query bind time. | **Traditional RDBMS (Postgres):** dynamic dictionary lookups at query time are slower than statically typed columnar arrays. | Small CPU overhead during query planning to look up schema types for projected columns. |

---

## Not verified

- Whether `lsp_types` v0.97.0 `CodeAction` payload integration with `Diagnostic.data` requires custom serialization shims in current `tower-lsp` / `async-lsp` setups.
- Real-world benchmark of `jsonschema` crate performance vs hand-rolled struct matchers on large YAML frontmatter objects.
- Exact memory footprint of caching compiled schema CSR graphs in a long-running LSP daemon process.

---

## Questions this raises for the grilling

1. **Validator Ownership Seam:** Does validation live in `src/schema` (making schema depend on `Note`), in `src/note` (making note depend on `Schema`), or in a separate consumer module (e.g. `src/analysis` or `src/lint`) that imports both?
2. **Schema Invalidation Penalty:** If schemas drive index storage typing (Shape 2), are we willing to accept whole-vault re-indexing every time a user edits a `.traces/schemas/*.toml` file?
3. **The Unschematized Note Experience:** Since most markdown notes in a vault do not declare a `fileClass`, how does query filtering behave on fields that are typed in *some* notes (via schema) and un-schematized in others? Does the query engine use the schema as a global type hint, or strictly per-note?
4. **Diagnostic Storage:** Should `traces check` diagnostics be purely ephemeral (recomputed on demand in ~15 ms), or stored in a Redb table for fast status queries?
5. **Interactive Gating vs Passive Acceptance:** Should `traces new` refuse to save a note if user input violates schema constraints, even though raw markdown edits outside Traces are accepted without hindrance?
