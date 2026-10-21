# Scout E: Validation Result Model and Checker API

<!-- rumdl-disable MD013 -->

Superseded where it differs from [ticket 16's
Answer](../../../issues/16-schema-note-typing-and-validation.md): severities
belong to ticket 27, and the delta key omits the element index. Its sketches
assume `NoteFieldValue: Hash`, which ticket 15 rules out.

## 1. Sub-problem Restated

Ticket 16 must decide whether and how the schema subsystem validates note
data. While the validation kernel logic evaluates whether frontmatter fields
conform to schemas, this sub-problem (E) focuses on the **result model** and
**checker API** produced by that evaluation.

This result model must serve four distinct hosts without impedance mismatch:

1. **LSP Diagnostics**: Language Server Protocol
   `textDocument/publishDiagnostics` and `textDocument/diagnostic` (pull)
   needing exact byte/line ranges, codes, severity calibration, descriptions,
   and quick-fix edits.
2. **CLI / Hook JSON**: Command-line linter (`traces check`) and pre-commit
   hooks outputting structured JSON or human-readable terminal reports, often
   filtering by baseline/delta to prevent noisy blocking of legacy notes.
3. **Template Post-render Warning**: Advisory non-blocking stderr notice when
   rendered note frontmatter violates schema constraints.
4. **MCP Guardrail (`isError`)**: Model Context Protocol tool execution result
   returning machine-actionable feedback (`isError: true`) enabling an LLM
   agent to self-correct invalid field writes, while projecting schema rules
   into JSON Schema `inputSchema`.

Critical domain pressures include:

- **Delta Rule**: Ability to isolate and block *only newly introduced*
  violations in dirty git working copies or incremental edits, ignoring
  pre-existing debt.
- **Bounded 'Expected'**: Handling large enum value sets (e.g. `job_titles.json`
  with 3,734 values) without blowing up diagnostic payload sizes or LLM token
  windows.
- **Severity Ownership**: Clarifying whether severity belongs inside the raw
  validation violation or in the diagnostic presentation envelope owned by
  Ticket 27.

---

## 2. What Traces Does Today

Today, Traces has zero schema validation execution or result models:

- `src/schema/model.rs:11-20`: Schema holds resolved fields (`SchemaFieldDef`),
  ancestors, children, and descendants. No validation method exists.
- `src/schema/fields.rs:52-57`: `SchemaFieldDef` stores `kind`, `required`,
  `multi`. Properties like `min`, `max`, `step`, and `format` are parsed into
  AST definitions but completely unread in production (`brief.md:39-41`).
- `src/note/metadata.rs:95-98`: Frontmatter stores
  `IndexMap<FieldKey, NoteFieldValue>`. It possesses zero byte spans.
- `src/cli/error.rs:14, 39-52`: 1,800 lines of error translation matching
  subsystem errors to `miette::Diagnostic`. ADR-0004 strictly keeps `miette`
  out of domain libraries.
- `examples/.traces/schemas/dir_contact.toml:16-17`: Select field referencing
  external JSON file `values/job_titles.json`, containing 3,734 values.
- `src/strsim.rs`: Local wrapper around crate `strsim` (0.11.1) providing
  `closest_match` used for suggesting schema field typos.

Prior LSP planning (`.scratch/md-pkm-lsp/issues/20:101-118` [V]) drafted an
initial `SchemaDiagnostic` with `SchemaDiagnosticKind`:

```rust
pub struct SchemaDiagnostic {
    pub field: FieldName,
    pub range: Option<Range<BytePos>>,
    pub kind: SchemaDiagnosticKind,
    pub message: String,
}
```

However, this coupled diagnostic presentation (message, range) directly into the
validation outcome and assumed a hardcoded severity tier (`Error` for missing
required or type mismatch) that directly contradicted Ticket 15's advisory
principle (`brief.md:78-86` [V]).

---

## 3. Precedents: Real Systems and What Failed/Succeeded

### 3.1 `jsonschema` (Rust crate)

- **Model** [V, crates.io `jsonschema` 0.28 /
  github.com/Stranger6667/jsonschema-rs]: Validation produces
  `Box<dyn Iterator<Item = ValidationError>>`. Each `ValidationError` exposes
  `instance_path: JSONPointer`, `schema_path: JSONPointer`,
  `kind: ValidationErrorKind`, and lazy `Display`.
- **Strengths**: Zero allocation on clean paths via lazy evaluation
  (`validate()` returns `Result<(), ValidationError>` while `iter_errors()`
  returns an iterator). Rich structured enum for error kinds (`Type`, `Enum`,
  `Minimum`, `Required`).
- **Regrets / Pitfalls**: In early versions, formatting messages eagerly
  caused massive benchmark slowdowns. Enum value mismatches in `enum` validation
  embedded all allowed values directly into `ValidationErrorKind::Enum {
  allowed: Vec<Value> }`, causing OOM and huge memory allocations on schemas
  with tens of thousands of enum variants.

### 3.2 Ajv (JSON Schema Validator, JavaScript/TypeScript)

- **Model** [V, ajv.js.org/api.html#error-objects]: Produces `ErrorObject`:
  `keyword: string`, `instancePath: string`, `schemaPath: string`,
  `params: Record<string, any>`, `message?: string`.
- **Options**: `allErrors: boolean` (fail-fast vs collect-all).
- **Regrets / Pitfalls**: Large enum sets with `enum` keyword:
  `params.allowedValues` contains the complete array of allowed strings. When
  validated against huge taxonomies, serializing `errors` to JSON easily
  exceeded Node.js string length limits or bloated HTTP responses. Ajv
  introduced custom keyword definitions and `discriminator` support specifically
  to bypass linear enum checking.

### 3.3 Kubernetes API Server `field.Error` & `field.ErrorList`

- **Model** [V, k8s.io/apimachinery/pkg/util/validation/field]:
  Structured field errors: `Type` (`TypeInvalid`, `TypeNotFound`,
  `TypeRequired`, `TypeDuplicate`, `TypeForbidden`, `TypeUnsupported`), `Field:
  string`, `BadValue: interface{}`, `Detail: string`.
- **Handling Bounded Enums**: Specifically for `TypeUnsupported`, Kubernetes
  defines:

  ```go
  func NotSupported(field *Path, value interface{}, validValues []string) *Error
  ```

  If `validValues` exceeds a small threshold (e.g. > 10 items), standard K8s API
  conventions mandate truncating the slice in the error text
  (`"supported values: [foo, bar, baz, ... (3,731 omitted)]"`) or omitting the
  full list and pointing to the OpenAPI discovery schema instead.
- **Strengths**: Extremely uniform taxonomy. Easy for clients to
  programmatically inspect error type without parsing string regexes.

### 3.4 Ruff & Rust-Analyzer Diagnostics & Fixes

- **Model** [V, ruff diagnostic crate / rust-analyzer `ide_diagnostics`]:
  Separation of rule definition from diagnostic rendering. Ruff diagnostics
  carry: `code: Rule`, `range: TextRange`, `message: String`,
  `fix: Option<Fix>`.
- **Fix Representation**: A `Fix` is not free-form text; it is an atomic
  replacement `Applicability` (`Automatic`, `Suggested`, `Manual`) and `Edit`
  (`TextEdit` with replacement byte range and text).
- **Baselines / Delta**: Ruff supports `--baseline-file` and git diff filtering.
  Ruff computes SARIF/JSON diagnostic hashes based on `(rule_code, file_path,
  content_hash_of_line)` so edits that shift line numbers do not invalidate
  baseline suppressions.

### 3.5 Buf (`buf breaking` and `protovalidate`)

- **Model** [V, buf.build/docs/breaking]:
  Protovalidate returns `Violation` protobufs: `field_path: string`,
  `constraint_id: string`, `message: string`, `rule_value: string`.
- **Delta Rule**: `buf breaking --against .git#branch=main`. Compares current
  descriptor against baseline descriptor. The rule engine works on semantic AST
  diffs, not string diffs: a violation is emitted *if and only if* the change
  breaks backwards compatibility between baseline and target.

### 3.6 SARIF (Static Analysis Results Interchange Format)

- **Model** [V, OASIS SARIF 2.1.0 standard]:
  Industry standard for static analysis reporting. Introduces
  `partialFingerprints` and `baselineState` (`unchanged`, `new`, `absent`,
  `modified`).
- **Fingerprinting**: `partialFingerprints.logicalLocationKey` or
  `contextHash` allows diff engines (e.g. GitHub Code Scanning, Reviewdog) to
  track whether a violation existed prior to the current commit without relying
  on unstable line numbers.

---

## 4. Design Dimensions & Candidate Solutions

### 4.1 Taxonomy of Validation Violations

A schema violation should be a pure value-level domain struct completely
independent of syntax spans or terminal colors:

```rust
pub struct ValidationViolation {
    pub field: FieldKey,
    pub element: Option<usize>,
    pub kind: ViolationKind,
}

pub enum ViolationKind {
    MissingRequired,
    UnknownField { suggestion: Option<String> },
    UnknownClass { class: String, suggestion: Option<String> },
    TypeMismatch { expected: ExpectedType, found: FoundType },
    NotInSet { found: String, candidates: ValueCandidates },
    OutOfRange { bound: RangeBound, found: f64 },
    MultiArityViolation { found_count: usize },
    InvalidDatePattern { expected_format: String, found: String },
    LinkTargetInvalid { target: String, reason: LinkErrorReason },
}
```

Stable rule codes map cleanly to dot-notated LSP / SARIF codes:

- `schema.missing-required`
- `schema.unknown-field`
- `schema.unknown-class`
- `schema.type-mismatch`
- `schema.not-in-set`
- `schema.out-of-range`
- `schema.multi-arity`
- `schema.invalid-date`
- `schema.invalid-link`

### 4.2 Bounding 'Expected' for Enums (3,734 Values Problem)

When validating a field against `job_titles.json` (3,734 values), emitting all
options crashes CLI output, pollutes LSP hovers, and exhausts LLM token limits
in MCP tool responses.

We evaluate three mechanisms:

1. **Truncated Prefix (`count + truncated flag`)**:
   Return the first $k$ items (e.g. 5) plus `total: 3734`.
2. **Top-$k$ Nearest Matches via `strsim`**:
   Compute Levenshtein / Jaro-Winkler distance between `found` and the 3,734
   candidates; return the top 3-5 closest candidates.
3. **Hybrid Model (`ValueCandidates`)**:
   Store total count, whether truncated, and the closest matches:

   ```rust
   pub enum ValueCandidates {
       SmallSet(Vec<String>), // <= 8 items: list all
       LargeSet {
           total_count: usize,
           closest: Vec<String>, // top 3 via strsim
           sample: Vec<String>,  // first 3 alphabetically
       },
   }
   ```

   *Strongest approach*: In an LLM MCP self-correction loop or interactive LSP
   quick-fix, showing "Doctor" when the user wrote "Docter" is actionable;
   dumping 3,734 entries is useless.

### 4.3 Fix Suggestions Representation

How should quick-fixes be represented?

- **Option 1: Freeform text suggestion** (`suggestion: Option<String>`).
  Simplest, but requires the host to figure out how to patch the file.
- **Option 2: Value Replacement AST**
  (`FixAction::ReplaceValue(NoteFieldValue)`). Host transforms this into a
  frontmatter YAML patch.
- **Option 3: TextEdit with Range** (LSP/Ruff style).
  Requires the violation model to know byte spans, which breaks the separation
  between the spanless index model and editor model.
- **Recommendation**: The kernel violation produces
  `FixSuggestion::Replace(String)` or `FixSuggestion::RenameField(FieldKey)`.
  The host adapter (LSP or CLI `--fix`) joins the suggestion with frontmatter
  spans to produce concrete `TextEdit`s.

### 4.4 Delta / Baseline Computation

How to block only newly introduced violations in CLI/hooks/MCP?

- **Keying Identity**: A violation identity key is formed by:
  `ViolationKey = (file_path, field_key, rule_code, element_index)`
- **Algorithm**:
  1. Baseline run: Calculate violations for base commit / clean file:
     $V_{\text{base}}$.
  2. Target run: Calculate violations for current working copy:
     $V_{\text{current}}$.
  3. Delta: $V_{\text{new}} = V_{\text{current}} \setminus V_{\text{base}}$.
- **Edge cases**: If a field value changes from one invalid state to another
  (e.g. typo `"doctr"` $\to$ typo `"doctur"`), keying by `(field, code)` would
  consider it existing debt. Including a hash of `found` distinguishes value
  mutations:
  `Fingerprint = (field_key, rule_code, element_index, hash(found_value))`

### 4.5 MCP Tool Projection (`inputSchema`) & Limitations

Under MCP, tools expose a JSON Schema (`inputSchema`). We can project schemas
into JSON Schema, but impedance mismatches exist:

- **Projectable**:
  - `input` $\to$ `{"type": "string"}`
  - `number` with `min/max` $\to$
    `{"type": "number", "minimum": X, "maximum": Y}`
  - `boolean` $\to$ `{"type": "boolean"}`
  - `multi: true` $\to$ `{"type": "array", "items": ...}`
  - Small `select` $\to$ `{"type": "string", "enum": [...]}`
- **Cannot be projected (must be deferred to validation runtime)**:
  - Huge `select` (3,734 values): Embedding 3,734 strings into tool definition
    drastically inflates tool registration prompt tokens. Best practice: project
    as
    `{"type": "string", "description": "Select from job titles catalog (3,734 values)"}`
    and validate post-call.
  - Date `format` strings (`%Y-%m-%d` or Moment formats): JSON Schema only
    supports standard formats (`date`, `date-time`). Custom strftime patterns
    cannot be validated by JSON Schema regex without complex translation.
  - Wikilink targets (`[[note]]`): JSON Schema has no concept of PKM links.
  - Dynamic file sources (`values = { path = ... }`).

### 4.6 Severity Ownership: Ticket 16 vs Ticket 27

- **Ticket 16 Scope**: Owns the domain check and result model:
  `ValidationViolation { field, element, kind }`. Violations state objective
  facts (e.g. "field X is missing", "value Y is not in enum").
- **Ticket 27 Scope**: Owns the diagnostic envelope and severity mapping:
  `Diagnostic { code, severity, span, message, help, fix }`.
- **Default Severity**: Ticket 16 specifies a *recommended default severity*
  associated with each rule code (e.g. `schema.missing-required` $\to$
  `Warning`), while Ticket 27 defines configuration overrides
  (`[schemas.diagnostics]` or host policies) where MCP can treat it as
  `isError: true` while LSP treats it as `Information`.

---

## 5. Candidate Solutions Menu

### Candidate 1: Keep-As-Is (No Structured Result Model)

- **Description**: No unified `ValidationViolation` struct. Template engine
  prints hardcoded strings to stderr. LSP re-implements an ad hoc validation
  pass directly inside LSP handlers emitting `lsp_types::Diagnostic`. MCP tool
  runs separate JSON Schema checks.
- **Strongest Argument Against**: Massive duplication and drift across four
  hosts. If select value coercion or link matching rules change (Ticket 15),
  every host breaks or diverges.

### Candidate 2: Monolithic LSP Diagnostic Envelope

- **Description**: The validation function directly returns
  `Vec<SchemaDiagnostic>` carrying spans, severity, formatted strings, and
  protocol codes.
- **Strongest Argument Against**: Violates separation of concerns. Requires the
  validator to depend on byte spans and string formatting, preventing it from
  running cleanly on span-free index notes, raw template data, or headless MCP
  inputs.

### Candidate 3: Two-Tier Decoupled Result Architecture (Recommended)

- **Description**:
  1. *Tier 1 (Kernel)*: Pure semantic validator:
     `fn validate_frontmatter(fields: &IndexMap<FieldKey, NoteFieldValue>, schema: &Schema) -> Vec<ValidationViolation>`
     Returns structured, span-free, severity-free `ValidationViolation`s with
     bounded candidates.
  2. *Tier 2 (Host Adapters)*:
     - LSP Adapter joins violations with frontmatter spans $\to$ `Diagnostic`.
     - CLI Adapter formats `miette` reports or computes delta vs git base.
     - Template Adapter prints concise stderr advisory warnings.
     - MCP Adapter formats `isError: true` with actionable self-correction
       hints.
- **Strongest Argument Against**: Requires maintaining adapter transformations
  for each host and coordinating between Ticket 16 (violations) and Ticket 27
  (diagnostics).

### Candidate 4: Unconventional A — Lazy Violation Stream / Iterator

- **Description**:
  `validator.validate(&note, &schema)` returns an iterator:
  `impl Iterator<Item = ValidationViolation>`.
  Allows early termination on the first error for MCP fail-fast guardrails,
  while LSP and CLI consume the full iterator.
- **Strongest Argument Against**: Frontmatter has an average of 5-15 fields;
  eager `Vec<ValidationViolation>` has negligible allocation cost (~100 ns).
  Iterator complexity across lifetime boundaries adds zero real performance
  benefit for note frontmatter.

### Candidate 5: Unconventional B — Violations as Queryable Relational Rows

- **Description**:
  Treat validation violations as a virtual table in the Traces query engine:
  `FROM notes WHERE schema.violations.count > 0` or
  `SELECT path, schema.violations FROM notes`.
  The result model implements the internal query tuple trait.
- **Strongest Argument Against**: Violates ADR-0009 (index never depends on
  schema). Evaluating schema violations inside the query engine would couple
  schema changes to index scans and complicate query execution.

---

## 6. Detailed Analysis of Recommended Solution (Candidate 3)

### 6.1 The Data Models

```rust
// In src/schema/validate/model.rs (owned by Ticket 16)

#[derive(Clone, Debug, PartialEq)]
pub struct ValidationViolation {
    pub field: FieldKey,
    pub element: Option<usize>,
    pub kind: ViolationKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViolationKind {
    MissingRequired,
    UnknownField { suggestion: Option<String> },
    UnknownClass { class: String, suggestion: Option<String> },
    TypeMismatch {
        expected: SchemaFieldType,
        found: NoteFieldType,
    },
    NotInSet {
        found: String,
        candidates: BoundedCandidates,
    },
    OutOfRange {
        bound: RangeBound,
        found: f64,
    },
    MultiArityViolation {
        expected_single: bool,
    },
    InvalidDatePattern {
        pattern: String,
        found: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedCandidates {
    pub total: usize,
    pub closest: Vec<String>, // Top 3 via strsim
    pub truncated: bool,
}
```

### 6.2 Delta Rule Implementation

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ViolationFingerprint {
    pub field: FieldKey,
    pub rule_code: &'static str,
    pub element: Option<usize>,
    pub value_hash: u64,
}

impl ValidationViolation {
    pub fn fingerprint(&self, val: Option<&NoteFieldValue>) -> ViolationFingerprint {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        if let Some(v) = val {
            v.hash(&mut hasher);
        }
        ViolationFingerprint {
            field: self.field.clone(),
            rule_code: self.kind.rule_code(),
            element: self.element,
            value_hash: hasher.finish(),
        }
    }
}
```

When running `traces check --delta` or git pre-commit hooks:

1. Validate base note: $F_{\text{base}} = \{v.\text{fingerprint}()\}$.
2. Validate current note: $V_{\text{curr}}$.
3. Filter:
   $V_{\text{report}} = \{ v \in V_{\text{curr}} \mid v.\text{fingerprint}() \notin F_{\text{base}} \}$.

---

## 7. What Ticket 16 Must Pin vs Leave for Ticket 27

| Concern | Owned by Ticket 16 | Owned by Ticket 27 |
|---|---|---|
| **Violation Struct & Enum** | Exact `ValidationViolation` & `ViolationKind` | Ignored (consumes them) |
| **Rule Codes** | Stable string codes (`schema.*`) | Diagnostic code registry & URLs |
| **Bounded Candidates** | `BoundedCandidates` (top-k `strsim`) | Terminal formatting / truncation display |
| **Severity** | Default recommended severity per rule | User severity overrides & envelope |
| **Spans & Positions** | None (pure field keys & element indices) | Exact `ByteSpan` / line-col mapping |
| **Fix Representation** | Semantic suggestion (`Replace(String)`) | TextEdit byte patching & protocol emission |
| **Host Emission** | Kernel API `validate_frontmatter()` | CLI formatting, LSP push/pull, SARIF export |

---

## 8. Cross-Cutting Lessons Transferable to Other Sub-problems

1. **To Sub-problem B (LSP Seam)**: Do not leak byte spans into the validation
   kernel. The LSP must look up spans using field keys from the violation model.
2. **To Sub-problem C (MCP Guardrail)**: Large select lists (like 3,734 job
   titles) cannot be registered in JSON Schema `enum`. Project them as
   unconstrained strings in `inputSchema` and let the validation kernel return
   bounded `strsim` hints upon violation.
3. **To Sub-problem D (Class Expander & Seam Structure)**: The checker is a pure
   function taking `&IndexMap<FieldKey, NoteFieldValue>` and `&Schema`. It does
   not require complicated trait indirection or synthetic intermediate wrapper
   classes.
