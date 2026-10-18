# Schema and note typing: who owns validation of values against a schema?

Type: grilling
Blocked by: 09, 10, 04

## Question

The schema subsystem defines field types (`SchemaFieldType`: input/select/boolean/number/date/file), ranges, options, `required`, and `multi`, with a sophisticated inheritance DAG — but the surveys report **no code that validates a note's values against a schema**, and `SchemaFieldType` has no relationship to `NoteFieldType`. `schema` is reported not to import `note` at all, and `SchemaDateField` stores an unvalidated `format` string. The LSP map assumes a standalone `SchemaValidator` (`validate(&Note, &Schema) -> Vec<SchemaDiagnostic>`, md-pkm-lsp ticket 20) and Metadata-Menu-style completion. Decide: what *is* a field's semantic type, where is conformance of a value to that type checked, and which module owns that bridge — without making `schema` and `note` mutually dependent.

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify core findings](01-research-verify-core-findings.md) lands): [schema §9/§11.1](../survey/schema.md), [note §9.2](../survey/note.md), [temporal §10.3–4](../survey/temporal.md) (LSP ticket 20 cites a chrono parser that no longer exists in `Cargo.toml`).

**Verified (ledgers landed 2026-10-18)** — [01 §4](../research/01-verify-core-findings.md): `src/schema/` has zero references to `note`; no runtime validator of note values against a schema exists in `cli`, `template`, or `index`; `SchemaDateField.format` is dead configuration (its only accessor is `#[cfg(test)]`, `schema/fields/date.rs:20`); taxonomies are `RawSchemaFieldType` (6), `SchemaFieldTypeTag` (6), `SchemaFieldType` (6, mapped 1:1 among themselves) vs `NoteFieldType` (9, used only for sort ranking) with **zero** mapping between schema and note types. `Cargo.toml` has `jiff` and no `chrono` (md-pkm-lsp ticket 20's chrono claim is stale). `FieldValue` is the only type shared by schema and the value layer.

## Alternatives (not exhaustive)

- `schema` owns a pure validator over the shared value model (schema depends on the value module, not on `note`).
- A separate analysis/validation component depends on both and is the only place the two meet; `schema` stays a passive registry.
- One field-type taxonomy shared by schema and values (single source of truth) vs two taxonomies with a total mapping function.
- Validation as part of the typed-interpretation layer from [Value model](09-value-model.md)/[Coercion policy](10-coercion-policy.md) (a value is *interpreted against* its declared type, yielding a typed value or a diagnostic).

## Done when

The answer states the taxonomy (one or two, and the mapping), the validator's owner and signature-level shape, dependency direction between `schema`, `note`, and the value module, severity/diagnostic ownership as it concerns architecture (not LSP presentation), and what "schema internals" (map fog) can now be specified.
