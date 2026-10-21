# Prior LSP effort review (md-pkm-lsp) for arch-design ticket 20

Exhaustive audit of `.scratch/md-pkm-lsp/` (tickets 08, 10, 11, 14, 19, 20, 21, 22, 23, 25, 26, 33, 40, 41; map.md; research files 20, 25, 26, 39, 40) against current codebase and arch-design tickets 16–19.
Tags: [V] verified against cited file:line; [H] inference; [P] proposal for ticket 20.

---

## 1. Decision ledger (resolved vs open vs superseded)

### (a) Schema validation and typing

- **Frontmatter-only runtime validation [V `issues/20:18` (Q1)]**: DECIDED in ticket 20. Validates type-correctness, range/select/date, required, and multi flags on frontmatter fields only. No inline fields (Q20), no file-existence checks (Q15).
- **Advisory severity calibration [V `issues/20:27-30` (Q2, Q6, Q11, Q12)]**: DECIDED in 20, AMENDED by 25 (`issues/25:27, 40`). Missing required = Error; type mismatch = Error; invalid select = Warning; list on non-multi = Warning; unknown class with close match = Warning; unknown field without close match = Information; unknown field with typo = Warning (Q2). SUPERSEDED in part by ticket 19 error-mode principle (advisory, never gating; casts fail to Null).
- **Date validation [V `issues/20:20` (Q16)]**: DECIDED in 20. Syntax parseability only via `chrono`, not `jiff`; ignores schema format string (`YYYY-MM-DD`). CONFLICTS with temporal roadmap and real-vault strftime formats.
- **Select matching [V `issues/20:28` (Q6)]**: DECIDED in 20. Empty list = skip constraint; non-empty list = strict case-sensitive match. SUPERSEDED by ticket 19 adaptive literals (`issues/19:43-45`).

### (b) Completion, hover, and go-to-definition from schema

- **Three-layer completion [V `issues/19:40-51`, `issues/20:37` (Q4)]**: DECIDED in 19/20. Layer 1 (fileClass schema), Layer 2 (global schema), Layer 3 (vault-inferred). FileClass wins on conflicts (`19:52-60`). File-field completion enumerates vault index + overlays, cap 100 (`20:37`).
- **Schema-bound hover [V `issues/19:66-69`, `issues/20:38` (Q5)]**: DECIDED in 19/20, AMENDED by 26 (`issues/26:58`, `issues/19:111`). Renders field name bold (not H2), type, constraints, description, attribution, and suggested-but-unset fields (`Schema::fields()` minus `Note::fields()`).
- **Go-to-definition [V `issues/20:36` (Q3), `issues/26:52-54`]**: DECIDED in 20/26. Jumps to `{schema_dir}/{schema_name}.toml`. For inherited fields, follows `origin` on `SchemaFieldDef`. File-level navigation only in v1 (no line/col in schema TOML).
- **Typo suggestion [V `issues/20:27` (Q2), `issues/21:56`]**: DECIDED. `SchemaService::suggest_field` (canonical match first, then Levenshtein); `suggest_class` proposed (`21:56`).

### (c) Spans on Note/Frontmatter/occurrences & persistence

- **AST extension over parallel index [V `issues/11:31-35`, `research/39:15-18`]**: DECIDED in 11. `Range<BytePos>` fields added to `Link`, `Tag`, inline fields, and new `Heading` AST node. Parallel LSP span index explicitly rejected.
- **Frontmatter spans [V `issues/11:60-63`, `issues/19:34-38`, `issues/20:45` (Q7)]**: DECIDED in 19/20. Ticket 20 proposed parallel map `Frontmatter { fields, #[serde(skip)] spans: IndexMap<FieldKey, ByteSpan> }`. Reconciled in 19 to `noyalib` `Spanned<T>` single-pass extraction relative to frontmatter block.
- **Span persistence (transient vs redb) [V `issues/11:68-71`, `issues/40:8-25`, `issues/41:1-32`]**: OPEN (ticket 41). Ticket 11 §8 decided spans stay transient, never in redb. Note-parse 04 Option A (mandatory span + `serde(skip)` + 0..0 doc invariant) was WITHDRAWN as a type-system lie (`40:8-16`). Option B (persist spans, amend 11 §8) vs Option C (`span: Option<ByteSpan>`, honest `None` on store decode, value-only Eq/Ord test churn) remain OPEN in ticket 41. Ticket 26 AMENDED 11 §8: closed-file serving re-parses on demand from `ContentResolver::text_of`, so 26 does not require persisted spans (41-C acceptable, 41-B upgrade path; `issues/26:78`, `issues/41:20-24`).
- **Occurrence navigation spine [V `issues/26:50-54`, `research/26:268`]**: DECIDED in 26. `Occurrence { kind: OccurrenceKind, span: ByteSpan }` with payload-in-variant (`FieldKey(FieldKey)`, `Wikilink(Link)`, etc.).

### (d) Diagnostics envelope

- **Transport & push-gate [V `issues/25:20-22` (Q1-R, Q2-R)]**: DECIDED in 25. Document-level pull is Phase A primary (`textDocument/diagnostic`); push (`publishDiagnostics`) is fallback for pull-incapable clients. Push is gated off when client pulls. Phase B adds workspace pull with bounded ~10MB warm-up cache (`issues/33:122`).
- **Debounce & publication lane [V `issues/25:23-25` (Q3-R), `issues/22:48-52` (Q19)]**: DECIDED in 25. T1 content/index = 100ms; T2 per-document publish = ~50ms adaptive clamp `[50ms, compute, 500ms]` (clangd shape); T3 workspace event-driven. Generalizes ticket 22's β single-lane publisher (FIFO `Compute/Clear/Opened`, coalesced per URI) to ALL diagnostics. No handler sleep (`issues/14:42`).
- **Namespace & rule registry [V `issues/25:28, 40-41` (Q6-R, Q15-R2)]**: DECIDED in 25. Scheme C: single `source: "traces"`, hierarchical `<feature>.<rule>` codes (`schema.*`, `tasks.*`, `links.*`, `query.*`, `template.*`), `DiagnosticRule` enum with `code()`, `default_severity()`, `const ALL`. Dotted fragment URL `codeDescription.href`.
- **Severity overrides & configuration [V `issues/25:29, 39` (Q7-R), `issues/20:56` (Q14)]**: DECIDED in 25. Registry holds defaults; overrides via editor-sourced O1 map (`schema.<rule>`, vocabulary `hint|info|warn|error|off`). Project config `[schemas.diagnostics]` is an **emission gate only** (`enabled: bool`), NOT a per-source severity table (`issues/25:39, 155`).
- **Aggregation [V `issues/25:26` (Q4-R)]**: DECIDED in 25. No Pipeline abstraction; plain fold functions over protocol-free diagnostics; `lsp_types` conversion only at boundary.

### (e) Live-buffer handling

- **Transient replacement overlay [V `issues/14:21-27`]**: DECIDED in 14. Editor buffers temporarily override `WorkspaceIndex` entries while open. `RwLock<HashMap<Uri, BufferState>>` with `content: String, version: i32`.
- **On-demand parsing [V `issues/14:23`, `issues/33:39`]**: DECIDED in 14. `BufferState` stores NO `parsed_note` field. Content parsed on-demand via `parse_markdown` (~0.02ms/10KB). Validation runs on demand against the on-demand parsed `Note`, NOT cached per keystroke.
- **Serving closed files [V `issues/26:78`, `issues/11:80`, `issues/41:20-24`]**: DECIDED in 26. `ContentResolver::text_of` re-parses cold files on demand from disk; no spans stored in redb.

---

## 2. SchemaValidator design & critical audit

### The prior design in full (ticket 20 / research 20)

- **Module**: `src/schema/validate.rs` (new file, `issues/20:133`).
- **Types [V `issues/20:101-118`]**:

  ```rust
  pub struct SchemaDiagnostic {
      pub field: FieldName,
      pub range: Option<Range<BytePos>>,
      pub kind: SchemaDiagnosticKind,
      pub message: String,
  }
  pub enum SchemaDiagnosticKind {
      UnknownClass { class: String },
      UnknownField { field: String, suggestion: Option<String> },
      TypeMismatch { expected: String, actual: String },
      MissingRequired { field: String },
      InvalidSelect { field: String, options: Vec<String> },
      ListOnNonMulti { field: String },
      InvalidDate { field: String },
  }
  pub struct SchemaValidator;
  impl SchemaValidator {
      pub fn validate(note: &Note, schema: &Schema) -> Vec<SchemaDiagnostic>;
  }
  ```

- **Severity tiers [V `issues/20:27-30`]**: Error (MissingRequired, TypeMismatch); Warning (InvalidSelect, ListOnNonMulti, UnknownField with typo, UnknownClass); Information (UnknownField without typo).
- **Q1–Q20 summary**: Q1 frontmatter-only; Q2 two-tier unknown field (canonical match skips); Q3 go-to-def to schema TOML; Q4 file-field completion cap 100; Q5 hover with metadata/suggested-unset; Q6 three-tier select (empty=skip); Q7 frontmatter spans parallel map; Q8 `origin: Option<SchemaName>` on `SchemaFieldDef`; Q9 timing on-open/change/save; Q10/13/17/19 `description: Option<String>` on field defs; Q11 enforce `multi: false`; Q12 single source + code; Q14 config emission gate; Q15 file fields type-only; Q16 date ISO-parseability via chrono; Q18 standalone validator fn; Q20 defer inline fields.

### Critical audit against arch-design decisions 16–19

1. **Value model conflict (ticket 18) [V `issues/18:31-37`]**:
   - Ticket 18 established `NoteFieldValue::Int(i64)` and `Float(f64)`, schema-local literal type for schema options (strings stay strings, native TOML dates stay strings), and NO `FieldValue` in schema.
   - Ticket 20's validator assumed `SchemaFieldType::Number` checked against a single `Number(f64)` or `FieldValue::Number`. With `Int` vs `Float`, schema needs to know whether `number` allows both or distinguishes integers (e.g. `step: 1.0` vs integer counts).
   - Ticket 18 decided select options that name links are interpreted as links ONLY at consumer boundary with shared wikilink parser. Ticket 20 Q6 strictly checked string equality against `select_values()` — this would falsely flag `[[target]]` note fields as `InvalidSelect`!
2. **Coercion & adaptive literals conflict (ticket 19) [V `issues/19:41-46`]**:
   - Ticket 19 established adaptive literals: a select string matches a note `Link` value by adopting the folded link target. Ticket 20's `validate` had zero concept of adaptive literal matching.
   - Ticket 19 established the error-mode principle: coercion failures are advisory, never gating; filters never error. Ticket 20 calibrated `TypeMismatch` as hard Error; under ticket 19, mismatched types fail to Null or produce advisory diagnostics, never abort processing.
   - Frontmatter classification is schema-free and quote-insensitive. Ticket 20 decides whether schema can override date inference back to text. Ticket 20 had no mechanism for schema overrides during parsing — it only evaluated post-parse `NoteFieldValue`.
3. **Compatibility envelope visibility rule (ticket 16) [V `issues/16:16`]**:
   - Ticket 16 mandates reducing `pub`; `pub` only for outward-facing components.
   - Ticket 20 made `SchemaValidator`, `SchemaDiagnostic`, `SchemaFieldDef.origin`, `SchemaFieldDef.description`, and `Frontmatter.spans` completely `pub`. If schema validation is internal to core/LSP, leaking these types into the public crate API violates ticket 16.
4. **Ticket 17 reopening of spans [V `issues/17:17-18`]**:
   - Ticket 17 explicitly reopened whether `Note` should split into an editor representation (with spans) and an indexed representation (spanless).
   - Ticket 20 hardcoded spans directly into `Frontmatter` via `#[serde(skip)] spans`, which was later caught in ticket 40 as part of the Option A type-system lie.

### Stale assumptions & discrepancies

- **`chrono` vs `jiff` [V `issues/20:20`, `research/20:71`]**: Research 20 recommended `jiff 0.2.37` (`research/20:71`), but Q16 picked `chrono` solely because "chrono is already in codebase" (`issues/20:20`). Traces is actively moving toward `jiff`/temporal-core; ticket 20's chrono ISO-only syntax assumption is stale.
- **`serde_yaml` [V `issues/19:28`, `issues/20:45`]**: Ticket 20 treated `serde_yaml`'s lack of spans as a blocker requiring line-offset re-scanning. In the current tree, `noyalib` is ALREADY the parser (`src/note/metadata.rs:58`), but `RawFrontmatter::parse` discards `Spanned<T>` and drops all spans.
- **`FieldValueRef` [V `src/note/metadata.rs:71`]**: Current code still references `FieldValueRef`, but ticket 18 eliminated the borrowed mirror.
- **`required` semantics [V `issues/20:18`]**: Q1 treats `NoteFieldValue::Null` as missing. But in YAML frontmatter, an explicit `key: null` or `key: ~` parses as `NoteFieldValue::Null`, whereas an omitted key is absent from the map. Both are reported as `MissingRequired`, but have different diagnostics in LSP tooling.
- **Date `format` in real vaults [V `brief.md:41`, `issues/20:20`]**: Q16 claimed schema format strings (`YYYY-MM-DD`) don't match chrono formats (`%Y-%m-%d`) and should be ignored. But in the author's real vault (`examples/.traces/schemas/`), 14 schemas explicitly declare strftime formats (`%Y-%m-%d`, `%Y-%m-%dT%H:%M`, `%H:%M`, `%Y`). Q16 completely ignored actual vault practice!

### Current tree verification (all [V])

- `Frontmatter` has spans? **NO.** `src/note/metadata.rs:95-98` defines `Frontmatter { fields: IndexMap<FieldKey, NoteFieldValue> }` with derived `PartialEq, Deserialize, Serialize`. Zero span fields or methods exist.
- `SchemaFieldDef.origin` or `description` exist? **NO.** `src/schema/fields.rs:52-57` defines `SchemaFieldDef { kind, required, multi }`. `src/schema/raw.rs:18-21` does not list `"description"` in `ALLOWED_OPTION_KEYS`.
- `src/schema/validate.rs` exists? **NO.** `ls src/schema/` confirms no `validate.rs` exists.

---

## 3. Span decision history & representation split

### History of tickets 11, 39, 40, 41

1. **Ticket 11 & Research 39 [V `issues/11:31-75`, `research/39:15-30`]**: Rejected parallel span index. Added `Range<BytePos>` to AST nodes. Narrowed `BytePos` to `u32`. Decided transient spans (never in redb, §8).
2. **Note-Parse 04 & Option A withdrawal [V `issues/40:8-16`, `issues/41:7-9`]**: Attempted mandatory `Spanned<T>` with `#[serde(skip)]` and a doc comment "spans from store decode as 0..0, never trust them." Withdrawn mid-triage as a type-system lie: type promises presence, store delivers dead data.
3. **Ticket 40 & 41 (Option B vs Option C) [V `issues/40:17-45`, `issues/41:10-32`]**:
   - **Option B (persist spans)**: Derive serde on `Spanned<T>`. Spans roundtrip through redb `NOTES` table. Zero test churn. Amends 11 §8. Wire cost: 8 bytes per occurrence.
   - **Option C (honest absence)**: `span: Option<ByteSpan>`. Keeps 11 §8. But decoded `None` breaks whole-`Note` round-trip equality asserts (`assert_eq!(decoded, note)` in `src/note/model.rs:589`), forcing manual value-only `Eq`/`Ord` and rewriting test pins (`src/position.rs:853, 863`), plus `Option` unwrapping at every consumer.
   - **Ticket 26 serving model amendment [V `issues/26:78`, `issues/41:20-24`]**: Definition/references/rename consume spans for closed files by re-parsing on demand from `ContentResolver::text_of`. Persisted spans are NOT required. 41-C is acceptable; 41-B is an optional upgrade path. Ticket 41 remains OPEN.

### What 'separate editor and indexing representations' means for validation

- **Two representations [H/P]**:
  - `IndexedNote` (index/store shape): Optimized for storage, queries, and bulk scans. Contains raw values, no byte spans, compact postcard serialization, lives in redb and `WorkspaceIndex`.
  - `EditorNote` (buffer/LSP shape): Carries byte spans on frontmatter keys/values, inline fields, links, and headings. Constructed on-demand from live buffer text or cold disk text via `ContentResolver::text_of`.
- **Could `SchemaValidator` run on both? [H/P]**:
  - Yes, cleanly: decouple validation evaluation from diagnostic reporting.
  - Core validation: `fn validate_fields(fields: &IndexMap<FieldKey, NoteFieldValue>, schema: &Schema) -> Vec<ValidationViolation>`. Pure value-level check.
  - Diagnostics mapper:
    - For `EditorNote` (with spans): maps violation field keys to exact `ByteSpan` (or LSP `Range`).
    - For `IndexedNote` (spanless): maps violations to document-level or line-level diagnostics (`range: None`).
- **Which shape does the LSP hold for open buffers? [V `issues/14:23`]**:
  - Ticket 14 decided the overlay holds `BufferState { content: String, version: i32 }`.
  - The LSP does NOT hold pre-parsed notes in the overlay! It holds raw text strings. On-demand parsing parses `content` into the `EditorNote` shape with spans when an LSP request arrives.
- **Is the validator needed for indexed (non-open) notes at all? [V `issues/25:20-21`, `issues/33:83, 122`]**:
  - **Phase A**: No. Ticket 25 Q1-R established open-document pull/push only. Non-open notes are never validated.
  - **Phase B (Workspace diagnostics)**: Ticket 25 Q1-R and 33 add workspace pull with a bounded ~10MB warm-up cache. For closed notes, workspace diagnostics either:
    1. Parse cold on demand via `ContentResolver::text_of` (per ticket 26 serving model), materialize diagnostics with byte ranges into the ~10MB pull cache, and drop the AST; OR
    2. Run value-only validation against the resident/cached metadata, emitting range-less diagnostics.

---

## 4. What LSP expected of schema BEYOND validation

| Feature | Schema data needed | Exists today? | Which arch-design ticket owns it |
|---|---|---|---|
| **Select value completion** | `SchemaFieldDef::select_values()` | **Yes** [V `src/schema/fields/select.rs:40`] | Ticket 18 (value model literal type), Ticket 20 (typing/completion) |
| **File-field completion** | `SchemaFileFieldRef` (filters: `folders`, `ext`, `class`) + index lookup | **Yes** [V `src/schema/fields/file.rs:42-53`], no completion handler | Ticket 20 (schema query filter), Ticket 34 (LSP completion) |
| **Hover on FileClass / field** | `Schema::fields()`, `origin`, `description`, `parent_order` | **Partial**: `fields()` exists; `origin`, `description`, `parent_order` do NOT [V `src/schema/fields.rs:52`] | Ticket 20 (schema field metadata & inheritance) |
| **Go-to-definition** | Path `{schema_dir}/{name}.toml`, `origin: Option<SchemaName>` | **Partial**: config path exists; `origin` does NOT [V `src/schema/fields.rs:52`] | Ticket 20 (schema metadata) |
| **Typo suggestions (`suggest_field`)** | Canonical match + Levenshtein distance against schema fields | **Yes** [V `src/schema/service.rs:252` (`suggest_field`)] | Ticket 20 (visibility / service facade) |
| **Typo suggestions (`suggest_class`)** | Fuzzy match over schema registry names | **No** (proposed in `issues/21:56`) | Ticket 20 (service helper) |
| **Inheritance chain display** | `parent_order: Vec<SchemaName>` (ordered topological resolution) | **No** (`extends: Vec<SchemaName>` exists, but not resolved order) | Ticket 20 (schema model) |
| **Field provenance (`origin`)** | `origin: Option<SchemaName>` on `SchemaFieldDef` | **No** [V `src/schema/fields.rs:52`] | Ticket 20 (schema builder stamping) |
| **Field documentation (`description`)** | `description: Option<String>` on resolved & raw field defs | **No** [V `src/schema/raw.rs:51`] | Ticket 20 (raw & resolved schema AST) |

---

## 5. Architectural conflicts & invalid assumptions

1. **Schema `FieldValue` vs schema-local literal type [V `issues/18:31-37`]**:
   - Prior LSP effort assumed schema options used the general `FieldValue` type (dates, durations, numbers).
   - Ticket 18 replaced schema `FieldValue` with a schema-local literal type: strings stay strings, native TOML dates stay strings. Schema does NOT own date/duration parsing.
2. **Select-link adaptive literal matching [V `issues/19:43-45`]**:
   - Prior LSP effort (ticket 20 Q6) assumed select values are strictly strings and matched case-sensitively against string note values.
   - Ticket 19 established adaptive literals: a select string matches a note `Link` value by folding the link target. Mismatching string vs Link is NOT an error.
3. **Error-mode principle vs hard Error severity [V `issues/19:46`, `issues/20:27-30`]**:
   - Prior LSP effort classified `TypeMismatch` as hard Error.
   - Ticket 19 established the error-mode principle: filters never error, explicit casts fail to Null, and type coercion failures produce advisory diagnostics, never gating or hard errors.
4. **Visibility rules & crate boundary [V `issues/16:16`]**:
   - Prior LSP effort casually made internal schema types `pub` (`SchemaValidator`, `SchemaDiagnostic`, `origin`, `description`).
   - Ticket 16 mandates reducing `pub`; `pub` only for outward-facing components. Schema validation types must be scoped to `pub(crate)` or private within the schema/LSP boundary.
5. **YAML parsing & spans [V `issues/19:28-38`, `src/note/metadata.rs:58`]**:
   - Prior LSP effort assumed replacing `serde_yaml` with `noyalib` would immediately yield spans via `Spanned<T>`.
   - Current tree already uses `noyalib`, but `RawFrontmatter::parse` strips all positions and produces plain `IndexMap<FieldKey, NoteFieldValue>`.
6. **Workspace split & sync core [V `map.md:27`, `issues/10:27-33`]**:
   - Prior LSP effort assumed single-crate synchronous core with `Arc<WorkspaceIndex>` swap cell and `tower-lsp-server` transport loop.
   - Arch-design tickets 16–17 establish that LSP is a committed consumer, but core remains synchronous and independent.

---

## 6. Templates and schema in prior effort

### Static analysis only, never execution

Ticket 22 established that templates CANNOT be executed for LSP analysis because `ui.*` interactive prompts block synchronously on `DialogProvider`:
> *"Templates cannot be safely executed for analysis purposes... ui.* namespace's 'No-Declaration Format' means templates call interactive prompts synchronously mid-render, blocking on a live DialogProvider... There is no existing static-analysis capability: no code path extracts 'which helpers/variables does this template reference' without a full execution pass"* [V `issues/22:9-11`].
> *"Static analysis: machinery::parse behind an unstable_machinery feature gate, exact-pinned =2.24.0... Placement: shared, engine-adjacent — src/template/analysis.rs... Diagnostics (phase 1): syntax + name-resolution + typo + undefined-variable"* [V `issues/22:21-25, 39-44`].

### Did the prior effort have a template schema pre-check?

**NO.** Ticket 22 had absolutely NO template schema pre-check (i.e. verifying whether a template's generated frontmatter matches a bound schema).

- Template diagnostics were strictly limited to: (1) Jinja syntax errors, (2) unknown helper/namespace names, (3) filter/test typos, and (4) undeclared variables via `undeclared_variables(false)` [V `issues/22:39-44`].
- Render-time and schema-related errors were explicitly excluded:
  > *"Render-only classes (query/schema/dialog/arg failures, error.rs:85-98) out of static scope"* [V `research/22:223-224`].
- The only template-schema interaction was:
  1. Template engine construction eagerly calling `SchemaService::load_verbose` (`issues/22:301-309`).
  2. Construction failure handling (Q23(a′)): if the schema directory fails to load, degrade to an empty schema registry so non-schema template features still work [V `issues/22:53-62, 434-450`].
  3. Authoring completion/hover for `schema.get()` helper calls [V `issues/22:161`].
- Quote confirming no schema diagnostics channel existed:
  > *"The named channel does not exist. 20's diagnostics are per-note frontmatter validation (SchemaValidator::validate(&Note, &Schema), issues/20:57, kinds 20:109-117) — no schema-TOML/construction-failure class anywhere in 20 or any ticket"* [V `research/22:409-411`].

---

## Not verified

- Whether `noyalib`'s CST or event stream exposes character-accurate spans for deeply nested YAML block scalars in Traces frontmatter (spot-checked API docs only; no test harness in tree).
- Whether any external MCP client expects workspace-wide schema diagnostics pushed on initialization without capability negotiation.
- Precise CPU cost of full-reparse on 50KB+ markdown notes during rapid typing (bench flagged anomaly at `benches/note_parsing.rs:470`, unmeasured with live spans).

---

## What this changes for ticket 20

1. **SchemaValidator is a pure value validator; spans are an adapter layer [P]**:
   Do NOT bake spans into `Schema` or `Frontmatter`. Define `SchemaValidator::validate(fields: &IndexMap<FieldKey, NoteFieldValue>, schema: &Schema) -> Vec<ValidationIssue>`. The LSP (or CLI) maps field keys to spans using whatever representation is in scope (`EditorNote` or on-demand AST scan). This cleanly supports both editor buffers and spanless indexed notes without type pollution.
2. **Schema does NOT own note typing or coercion [P]**:
   Frontmatter parsing stays schema-free (per ticket 19). Schema validation is an on-demand advisory pass producing diagnostics (per Research 03 and ticket 20). Coercion failures are advisory warnings, never gating.
3. **Adaptive literal matching must be honored in validation [P]**:
   Select validation cannot be a naive string set lookup; it must allow `Link` values whose folded target matches a select option, per ticket 19.
4. **Template schema pre-check is completely novel [P]**:
   Prior art has zero precedent for checking templates against schemas. If ticket 20 introduces template validation against schemas (e.g., verifying template frontmatter skeleton matches `fileClass`), it must be designed from scratch and cannot rely on runtime rendering.
5. **Metadata extensions (`origin`, `description`, `parent_order`) belong to schema AST [P]**:
   If LSP go-to-def and hover are supported, `SchemaFieldDef` and `RawSchemaFieldDef` need `origin: Option<SchemaName>` and `description: Option<String>`, with manual `PartialEq` preserving semantic identity.
6. **Diagnostics follow Scheme C [V `issues/25:28`]**:
   Single source `"traces"`, hierarchical `schema.*` codes, registry-driven severities, overridable via editor-sourced O1 map. `[schemas.diagnostics]` is strictly an emission gate (`enabled: bool`).
