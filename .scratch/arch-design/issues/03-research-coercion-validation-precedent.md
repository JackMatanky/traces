# Where do mature systems put type coercion and validation, and how do they report violations?

Type: research
Status: resolved
Blocked by: none

## Question

Traces coerces strings heuristically in three places (`src/field.rs:843` ISO-date coercion at deserialization; `src/query/sort.rs:462` `TextShape::classify`; `src/query/grammar/filter.rs:238` literal promotion) and performs zero validation of note values against schemas (`survey/schema.md` H1; `survey/query.md` H6; `survey/note.md` H7). Schemas validate only their own definitions.

Study: SQLite type affinity rules and comparison across storage classes; DuckDB implicit/explicit cast rules and `TRY_CAST`; Polars strict vs non-strict casts; Obsidian Dataview type inference for untyped frontmatter + `dv.compare`; Obsidian Metadata Menu fileClass field validation and fix-up UX; `jsonschema` (Rust) and `schemars`: validation as a separate pass producing structured errors with instance paths; `taplo`/`toml-edit` schema validation diagnostics; Ruff's settings validation; rumdl's config validation.

Focus: is coercion a property of the value, of the query/comparison, or of a schema-bound pass; how do systems make heuristic inference predictable (documented lattice, explicit cast functions, opt-in); how violations are represented (error vs diagnostic vs null) and whether validation is advisory or gating; how advisory validation is consumed by CLI/lints/editors; costs of validating at index time vs query time vs on demand.

Informs: coercion and comparison policy, schema-note typing and validation, query core.

Deliverable: `.scratch/arch-design/research/03-coercion-validation-precedent.md` (<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp `get_item_source`, or GitHub raw files) and cite repo path + symbol/line for every claim. Structure: (1) what each comparable actually does (verified from source), (2) the invariants and who owns them, (3) tradeoffs and costs they pay, (4) transferable lessons for Traces, (5) lessons that do NOT transfer and why, (6) what this changes or challenges in the Traces hypotheses named above. Mark facts [V], interpretation [H]. Do not propose the final decision; that belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Deliverable completed at `.scratch/arch-design/research/03-coercion-validation-precedent.md`.
Researched primary sources across 9 comparables (SQLite, DuckDB, Polars, Obsidian Dataview, Metadata Menu, jsonschema/schemars, taplo/toml-edit, Ruff, and rumdl) citing exact source files and line numbers. Clarified that coercion belongs to query binding/casts (not deserialization sniffing), note validation must be an independent advisory diagnostic pass (the Taplo/jsonschema pattern) rather than an ingestion-blocking gate, and cross-type comparison requires a deterministic partition lattice without ambient timezone lookups.
