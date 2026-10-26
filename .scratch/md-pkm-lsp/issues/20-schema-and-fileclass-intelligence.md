# Schema- and File-Class-aware language intelligence

Type: grilling
Status: resolved
Blocked by: 08, 19
Research: `20-schema-fileclass-intelligence.md` (includes stress test results in Part VIII), `20-designs.md`

## Question

Grounding: `SchemaService` (`src/schema/service.rs`) is a load-once registry
over `.traces/schemas/*.toml`, resolved via Kahn's-topological-sort inheritance
(`extends`/`excludes`/`$ref`) into `IndexMap<SchemaName, Arc<Schema>>`, exposing
`.get()`, `.fields()`, `.field()`, `.children_of()`, `.descendants_of()`,
`.matches()`. **Critically: there is no runtime validator today.** Schema
validation happens only when schema *definitions* are parsed, never against
actual Note field values. File-Class binding lives outside `schema/`, in
`src/file_class_expander.rs`, bridging to `query::grammar::source`. Informed by
Metadata Menu research (ticket 08).

## Decisions (Q1 to Q20)

### Validation Scope & Behavior

| Q | Decision | Rationale |
|---|----------|-----------|
| **Q1** | The runtime validator is scoped to **frontmatter only** (type-correctness + range/select/date + required + multi enforcement). No inline fields, no file-existence checks. | Matches the Dataview/Metadata Menu precedent. `NoteFieldValue::Null` on required fields is correctly detected as missing. Object values on non-Object types are correctly rejected. |
| **Q15** | File fields: type + multi only (no file-existence check). | The pure function model is preserved: no filesystem access is needed. Type checking (is the value a `Link`?) is cheap and pure. |
| **Q16** | Date fields: syntax validation only (ISO-8601 parseability). | Schema format strings (`YYYY-MM-DD`) do not match chrono format (`%Y-%m-%d`). Use the existing chrono parser (already in codebase), not jiff. Validate parseability, not format compliance. |
| **Q20** | Frontmatter only; inline fields are deferred to a follow-up. | Inline fields have different semantics (lists expected, no byte offsets). Frontmatter-only is the right initial boundary. |

### Severity & Diagnostics

| Q | Decision | Rationale |
|---|----------|-----------|
| **Q2** | Two-tier severity: **Information** (no typo match via `suggest_field()`) / **Warning** (typo match). **Fix:** check the canonical match first: if `FieldKey::is_match(note_field, schema_field)` is true for any schema field, skip the diagnostic entirely (no Warning, no Information). Only non-canonical unknowns enter the two-tier logic. | Prevents a false positive on casing matches (e.g., `Status` vs `status`). Uses the existing `suggest_field()`, which does canonical + Levenshtein matching. |
| **Q6** | Three-tier select validation: **strict** (a non-empty list means the value must be in the set) / **skip** (an empty list means no constraint) / **N/A** (not select). An invalid select = **Warning** (not Error). | An empty `values` list meaning skip is the key insight. Matches the Metadata Menu precedent: options are completion suggestions, not enforcement. |
| **Q11** | Enforce the `multi` flag: a Warning if a list value appears on a `multi: false` field. | No enforcement exists today, so this is net-new. Warning is appropriate for likely-mistake YAML. |
| **Q12** | A single `source: "traces-schema"` with a `code` field for differentiation (`"type-mismatch"`, `"invalid-select"`, `"missing-required"`, `"unknown-field"`, `"unknown-class"`). Add `relatedInformation` pointing to the source schema for inherited fields. | Multiple source strings are unconventional in LSP. A single source + code enables editor-level filtering. |

### Intelligence Features

| Q | Decision | Rationale |
|---|----------|-----------|
| **Q3** | Go-to-definition: jump directly to the originating schema. Schema name to file path: `{config.resolved_schema_directory()}/{schema_name}.toml`. For inherited fields, follow `origin` on `SchemaFieldDef`. The resolution chain is shown in hover text via `parent_order`. | SchemaService does not store a directory, so reconstruct it from Config (O(1)). `$ref` follows a single hop. No source positions in schema TOML in v1 (navigate to the file, not the line). |
| **Q4** | File-field completion: enumerate from the vault index + DocumentStore overlays, cap 100, `is_incomplete: true`. Sort by relevance (recently opened, then same folder, then alphabetical). Skip `_`-prefixed and `.`-prefixed files. | `SchemaFileFieldRef` + `file_field_source()` already encode the filter logic. Merge indexed files with unsaved buffers from DocumentStore. |
| **Q5** | Hover: field list + metadata (kind/required/multi) + description + suggested-but-unset fields (the set difference of schema keys vs note keys) + ancestor chain. | Schema resolution: read the class field, then `SchemaService::get()`. Suggested-but-unset = `Schema::fields()` keys minus `Note::fields()` keys. |
| **Q9** | Timing: on-open + debounced-change (300ms) + on-save. Re-validate only on frontmatter changes, not body edits. | The 300ms debounce is the industry standard (rust-analyzer, yaml-language-server, json-language-server); it is provisional, since ticket 25 owns the final debounce model (reconciled 2026-09-23). |

### Model Extensions

| Q | Decision | Rationale |
|---|----------|-----------|
| **Q7** | Frontmatter spans: a parallel map `Frontmatter { fields, #[serde(skip)] spans }`. `ByteSpan = Range<BytePos>`. A `from_with_spans(raw, source, base_offset)` constructor. **Phase 1:** a stub (empty spans). **Phase 2:** populated from `noyalib` `Spanned<T>` byte ranges (ticket 19's mechanism; reconciled 2026-09-23, replacing the originally-planned line-offset re-scan). | Zero callers break. `#[serde(skip)]` preserves the postcard roundtrip. Manual `PartialEq` ignores `spans`. Offset tracking comes from `noyalib` once ticket 19 lands; Phase 1 is a stub until then. |
| **Q8** | Origin: `origin: Option<SchemaName>` on `SchemaFieldDef`. Stamped in `resolve_own_fields` after `build()`. Manual `PartialEq` excluding `origin`. | Propagation: inherited fields keep the ancestor's origin via `Clone`. The `$ref` origin is "declared by", not "defined by" (acceptable: the target is derivable from the raw schema). About 32 bytes/field. |
| **Q10** | Description: `Option<String>` on `SchemaFieldDef` and `RawSchemaFieldDef`. Add it to `ALLOWED_OPTION_KEYS` and the visitor in `raw.rs`. Handle it like `required`/`multi` (a field-level attribute). | It must be on both the resolved and raw types for TOML deserialization. |
| **Q13** | Description inherits with the field through the schema hierarchy. | Same pattern as `required`/`multi`: the child overrides if declared, else inherits via `field.clone()`. |
| **Q17** | Description: an inline string in TOML. | Consistent with `required`/`multi` syntax. |
| **Q19** | Description: optional in schema TOML. | `Option<String>`: an absent value means no description shows in hover. |

### Architecture

| Q | Decision | Rationale |
|---|----------|-----------|
| **Q14** | Config: `[schemas.diagnostics]` with `enabled` + `debounce_ms`. Per-source severity comes via editor LSP settings, not project config. | Follows the existing config nesting pattern. Minimal project config; the editor controls diagnostic display. |
| **Q18** | The validator is a standalone `SchemaValidator` struct: `validate(&Note, &Schema) -> Vec<SchemaDiagnostic>`. A pure function, reusable by a future CLI. | No LSP backend exists yet, so it must be a library function. `SchemaDiagnostic` carries `field: FieldName`, `range: Option<Range<BytePos>>`, `kind: SchemaDiagnosticKind`, `message: String`. |

### Fileclass precedents (2026-10-21)

- **Q6 corroboration**: fileclass's `validateInList`/`options` treats an
  empty/absent list as unconstrained, an independent production match for 20's
  read of the incumbent semantics
  ([tool-fileclass](../research/tool-fileclass.md)).
- **Q11 reverse case (recorded, not decided)**: 20's matrix addresses
  list-on-non-multi but not the reverse (a scalar-valued field on a multi-valued
  fileclass). fileclass's measured calibration rule (Multi/MultiInput lenient,
  MultiFile/MultiMedia strict, "flag only what queries can observe") is input
  for the implementation spec ([tool-fileclass](../research/tool-fileclass.md)).
- The deferred `schema-file.*` diagnostic codes now have a shipped reference
  design: fileclass's vault-wide definition-side audit (ERROR/WARNING taxonomy,
  fingerprinted once-per-problem logging, sessionly+rename triggers)
  ([tool-fileclass](../research/tool-fileclass.md)).
- Multi-class collision order (last-frontmatter-declared-wins) is specified by
  the incumbent plugin but missing from 20's resolution matrix; it is flagged
  for the implementation spec ([tool-fileclass](../research/tool-fileclass.md)).

The decision is unchanged; the block is precedent-recording only.

## Model Change Summary

### `Frontmatter` (src/note/metadata.rs)

```rust
pub struct Frontmatter {
    fields: IndexMap<FieldKey, NoteFieldValue>,
    #[serde(skip)]
    spans: IndexMap<FieldKey, ByteSpan>,  // Phase 2: populated from noyalib Spanned byte ranges (ticket 19)
}
// New: span_of(), span_of_key(), from_with_spans(), get_values_with_spans()
// Manual PartialEq ignoring spans
```

### `SchemaFieldDef` (src/schema/fields.rs)

```rust
pub struct SchemaFieldDef {
    kind: SchemaFieldType,
    required: bool,
    multi: bool,
    origin: Option<SchemaName>,      // provenance tracking
    description: Option<String>,     // field description for hover
}
// Manual PartialEq comparing kind, required, multi only
// origin() and description() accessors
```

### `RawSchemaFieldDef` (src/schema/raw.rs)

```rust
// Add: description: Option<String>
// Add "description" to ALLOWED_OPTION_KEYS
// Handle in visitor alongside required/multi
```

### `SchemasConfig` (src/config/model.rs)

```rust
// Add: diagnostics: DiagnosticsConfig
pub struct DiagnosticsConfig {
    enabled: bool,        // default: true
    debounce_ms: u64,     // default: 300
}
```

### New: `SchemaDiagnostic` (src/schema/validate.rs)

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
```

## Files Needing Changes

| File | Change |
|------|--------|
| `src/note/metadata.rs` | Add the `ByteSpan` alias, the `spans` field, and new methods. Manual `PartialEq`. |
| `src/note/parser.rs` | Track `metadata_offset`. Call `from_with_spans()`. |
| `src/schema/fields.rs` | Add the `origin`, `description` fields, accessors, and manual `PartialEq`. |
| `src/schema/raw.rs` | Add `description` to `ALLOWED_OPTION_KEYS` and the visitor. Add it to `RawSchemaFieldDef`. |
| `src/schema/builder.rs` | Stamp `origin` in `resolve_own_fields`. Propagate `description`. |
| `src/schema/model.rs` | Add `parent_order: Vec<SchemaName>` for the resolution chain. |
| `src/config/raw.rs` | Add `RawDiagnosticsConfig`. |
| `src/config/model.rs` | Add `DiagnosticsConfig`. Extend `SchemasConfig`. |
| `src/config/builder.rs` | Resolve the diagnostics config. |
| `src/schema/validate.rs` | **New file.** `SchemaValidator`, `SchemaDiagnostic`, validation logic. |

## Prerequisites

1. **Ticket 19** (noyalib YAML parsing) gives per-field byte spans, which give
diagnostic underlines.
2. **Ticket 11** (BytePos/Bridgebyte) gives the span infrastructure.
3. **Schema system stable**: `SchemaService`, `Schema`, `SchemaFieldDef` are all
   tested.

## Remaining Weaknesses

| # | Severity | Weakness | Mitigation |
|---|----------|----------|------------|
| 1 | Resolved | YAML offset tracking (serde_yaml lacks spans) | Reconciled 2026-09-23: `noyalib` `Spanned<T>` provides byte ranges (ticket 19); Phase 1 still ships an empty stub until 19 lands. |
| 2 | High | Multi-root schema name collisions | Defer to ticket 30. |
| 3 | Medium | `$ref` origin is "declared by", not "defined by" | Acceptable: the target is derivable from the raw schema. |
| 4 | Medium | No ordered resolution chain on Schema | Add `parent_order: Vec<SchemaName>`. Low effort. |
| 5 | Medium | Overlay files not in completion | Merge DocumentStore overlays. Low effort. |
| 6 | Medium | Manual `PartialEq` on both Frontmatter and SchemaFieldDef | Document the convention. Add size assertions. Low effort. |

### Amended by 25 (2026-10-15)

- **`source` and debounce (25 Q3-R/Q6-R/Q15-R2)**: `source` moves from
  `"traces-schema"` to the unified Scheme C: `source: "traces"` + hierarchical
  `<feature>.<rule>` codes (schema rules live under `schema.*`);
  `relatedInformation` for inherited fields stands.
  `[schemas.diagnostics].debounce_ms` default 300 becomes the T2 value
  (**~50ms** typical, adaptive clamp `[50ms, compute, 500ms]`); the key survives
  as an optional override.
- **Per-source severity (20:56, Q14) becomes per-code O1 keys (25 Q15-R2)**:
  severity overrides are per diagnostic code in the editor-sourced O1 override
  map (`schema.<rule>` keys, vocabulary `hint|info|warn|error|off`). These are
  not per-source settings. `[schemas.diagnostics]` remains an **emission gate**
  (`enabled`); severity resolution is the registry's job (orthogonal, per Q7-R).
- **22:64 receipt**: the optional v2 range-less schema-file diagnostic is
  deferred with a named vehicle. It lands behind 20's `[schemas.diagnostics]`
  gate as `schema-file.*` registry codes, consumed via the same per-code
  override keys; the watch trigger that feeds it is owed to 31 (schema-dir
  `didChangeWatchedFiles`). (amended 2026-10-21 by fileclass research:
  fileclass's audit is the worked reference design for the deferred diagnostic.
  Its trigger model (sessionly + on-demand + rename, explicitly not
  per-rebuild) informs 31's watch,
  [tool-fileclass](../research/tool-fileclass.md).)
