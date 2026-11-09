# How do mature engines model dynamic values, and what do they pay for it?

Type: research
Status: resolved
Blocked by: none

## Question

Traces carries four dynamic-value enums (`FieldValue`, `FieldValueRef`,
`NoteFieldValue`, `NoteFieldValueRef`) with divergent number models (Int/Float
vs f64). The extension variants differ (Duration/Link). See
`survey/root-primitives.md` H1, `survey/note.md` H1/H8,
`survey/cross-cutting.md` H2. How do mature implementations represent one
dynamic value type across parsing, validation, sorting, filtering, and
rendering?

Study the actual source of:

- DataFusion `ScalarValue` (datafusion-common)
- Polars `AnyValue`/`Scalar`
- `serde_json::Value` plus `toml::Value`/`toml_edit`
- `noyalib::Value` (what Traces already parses into)
- Obsidian Dataview `Literal`/`LiteralType` plus comparators
  (`src/data-model/value.ts`)
- Metadata Menu field types
- SQLite value affinity/storage classes (`sqlite3.h`, `vdbe` comparison)
- DuckDB `Value`/`LogicalType`

Add comparables you find more applicable.

Focus questions:

- Owned vs borrowed twin types: do any keep a `Ref` mirror, and why?
- Where are total ordering/equality/hash defined: on the value, or in a separate
  comparator/policy?
- Integer vs float separation.
- How are extension types (duration, link, date) modelled: variants vs typed
  extension/"logical type"?
- How does deserialization avoid multi-stage conversion: serde `Deserialize`
  directly vs visitor intermediates?
- What are the in-memory size tradeoffs of the enum?
- What serialization (postcard-compatibility) constraints apply?

Informs decision tickets: value model, coercion and comparison policy,
schema-note typing and validation, note document model.

Deliverable: `.scratch/arch-design/research/02-value-model-precedent.md` (<= 300
lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp
`get_item_source`, or GitHub raw files) and cite repo path + symbol/line for
every claim. Use this structure:

1. What each comparable actually does (checked from source).
2. The invariants and who owns them.
3. The tradeoffs and costs they pay.
4. Transferable lessons for Traces.
5. Lessons that do NOT transfer, and why.
6. What this changes or challenges in the Traces hypotheses named above.

Mark facts [V], interpretation [H]. Do not propose the final decision; that
belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Resolved by comparative research deliverable: [.scratch/arch-design/research/02-value-model-precedent.md](../research/02-value-model-precedent.md).
