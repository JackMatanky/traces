# Where do mature systems put type coercion and validation, and how do they report violations?

Type: research
Status: resolved
Blocked by: none

## Question

Traces coerces strings heuristically in three places:

- `src/field.rs:843`: ISO-date coercion at deserialization.
- `src/query/sort.rs:462`: `TextShape::classify`.
- `src/query/grammar/filter.rs:238`: literal promotion.

Traces performs zero validation of note values against schemas
(`survey/schema.md` H1, `survey/query.md` H6, `survey/note.md` H7). Schemas
validate only their own definitions.

Study these comparables:

- SQLite type affinity rules and comparison across storage classes
- DuckDB implicit/explicit cast rules and `TRY_CAST`
- Polars strict vs non-strict casts
- Obsidian Dataview type inference for untyped frontmatter, plus `dv.compare`
- Obsidian Metadata Menu fileClass field validation and repair UX
- `jsonschema` (Rust) and `schemars`: validation as a separate pass producing
  structured errors with instance paths
- `taplo`/`toml-edit` schema validation diagnostics
- Ruff's settings validation
- rumdl's config validation

Focus:

- Is coercion a property of the value, of the query/comparison, or of a
  schema-bound pass?
- How do systems make heuristic inference predictable: documented lattice,
  explicit cast functions, opt-in?
- How are violations represented: error vs diagnostic vs null?
- Is validation advisory or gating?
- How do CLIs, lints and editors consume advisory validation?
- What are the costs of validating at index time, at query time, and on demand?

Informs: coercion and comparison policy, schema-note typing and validation,
query core.

Deliverable: `.scratch/arch-design/research/03-coercion-validation-precedent.md`
(<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp
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

Deliverable completed at
`.scratch/arch-design/research/03-coercion-validation-precedent.md`. Research
covered 9 comparables with exact source files and line numbers: SQLite, DuckDB,
Polars, Obsidian Dataview, Metadata Menu, jsonschema/schemars, taplo/toml-edit,
Ruff, and rumdl. It clarifies three points:

- Coercion belongs to query binding and casts, not to deserialization sniffing.
- Note validation must be an independent advisory diagnostic pass (the
  Taplo/jsonschema pattern), not an ingestion-blocking gate.
- Cross-type comparison needs a deterministic partition lattice without ambient
  timezone lookups.
