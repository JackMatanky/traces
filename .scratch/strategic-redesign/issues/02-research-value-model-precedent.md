# How do mature engines model dynamic values, and what do they pay for it?

Type: research
Status: resolved
Blocked by: none

## Question

Traces carries four dynamic-value enums (`FieldValue`, `FieldValueRef`, `NoteFieldValue`, `NoteFieldValueRef`) with divergent number models (Int/Float vs f64) and different extension variants (Duration/Link) — see `survey/root-primitives.md` H1, `survey/note.md` H1/H8, `survey/cross-cutting.md` H2. How do mature implementations represent one dynamic value type across parsing, validation, sorting, filtering, and rendering?

Study the actual source of: DataFusion `ScalarValue` (datafusion-common), Polars `AnyValue`/`Scalar`, `serde_json::Value` + `toml::Value`/`toml_edit`, `noyalib::Value` (what Traces already parses into), Obsidian Dataview `Literal`/`LiteralType` + comparators (`src/data-model/value.ts`), Metadata Menu field types, SQLite value affinity/storage classes (`sqlite3.h`, `vdbe` comparison), DuckDB `Value`/`LogicalType`. Add comparables you find more applicable.

Focus questions: owned vs borrowed twin types (do any keep a `Ref` mirror and why); where total ordering/equality/hash are defined (on the value, or in a separate comparator/policy); integer vs float separation; how extension types (duration, link, date) are modelled (variants vs typed extension/"logical type"); how deserialization avoids multi-stage conversion (serde `Deserialize` directly vs visitor intermediates); in-memory size tradeoffs of the enum; serialization (postcard-compatibility) constraints.

Informs decision tickets: value model, coercion and comparison policy, schema-note typing and validation, note document model.

Deliverable: `.scratch/strategic-redesign/research/02-value-model-precedent.md` (<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp `get_item_source`, or GitHub raw files) and cite repo path + symbol/line for every claim. Structure: (1) what each comparable actually does (verified from source), (2) the invariants and who owns them, (3) tradeoffs and costs they pay, (4) transferable lessons for Traces, (5) lessons that do NOT transfer and why, (6) what this changes or challenges in the Traces hypotheses named above. Mark facts [V], interpretation [H]. Do not propose the final decision; that belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Resolved by comparative research deliverable: [.scratch/strategic-redesign/research/02-value-model-precedent.md](../research/02-value-model-precedent.md).
