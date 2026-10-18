# Research: precedent for dynamic values, coercion, and schema validation

Type: research
Status: resolved

## Question

Gather transferable lessons for tickets [Value model](09-value-model.md) (one value tree or several), [Coercion policy](10-coercion-policy.md) (where untyped text acquires a type), and [Schema and note typing](11-schema-note-typing-and-validation.md) (who validates values against a schema). Not an LSP or PKM survey: choose comparables by the question.

**General/Rust comparables** (read source or authoritative docs; use rust-docs-mcp for crate APIs and current versions): SQLite type affinity and cross-type ordering (Traces' `NoteFieldType` rank resembles it), implicit-cast rules in SQLite/DuckDB/Postgres; Arrow/Polars dtype system and casting; `serde_json::Value`, `toml`/`toml_edit` datetime and `Spanned` handling, `serde_yaml`-successors (`noyalib` is in use); `jsonschema`/`boon`/`serde_valid` for validation-as-separate-pass vs typed-construction; "parse, don't validate" applied to dynamic data.

**Local digests** (`docs/digests/`; files are very large — **search/grep first, then read only the matched ranges**; `*-src-digest.txt` is source code, `*-docs-digest.txt` is documentation, the unsuffixed `*-digest.txt` is a whole-repo digest; inspect a file's header/structure before choosing):

- `obsidian_blacksmithgu-obsidian-dataview-src-digest.txt`: its value/type model (Luxon `DateTime`/`Duration`, `Link`, `Literal` union), comparison/ordering across kinds, implicit string-to-date/duration inference, how frontmatter and inline fields are normalized, and its documented failure modes. Use `-docs-` only for user-visible typing rules.
- `obsidian_mdelobelle-metadatamenu-src-digest.txt`: how a field's declared type (fileClass) drives interpretation, validation, and display; where validation lives relative to parsing.
- `rumdl-src` digest: how config/option values are typed and validated per rule (a typed-options-per-owner pattern), as a comparison for schema-owned vs central validation.

## Deliverable

`.scratch/arch-redesign/research/04-values-coercion-validation-precedent.md`: at most ~8 lessons, each tagged to the ticket(s) it informs and graded `VERIFIED` (seen in source, cite file/section), `ECOSYSTEM-CONSENSUS` (stated by authoritative docs/practitioners, cite), or `INFERENCE`. For each lesson state the **transferability test**: which Traces constraints the exemplar shares or lacks (single-user synchronous CLI; vaults to ~20K notes; values persisted via serde to redb; template and query consumers). Include **negative lessons** (patterns that do not suit Traces and why). Do not recommend a design. Do not modify production code or docs. Append an `## Answer` (5–8 lines) to THIS ticket and set `Status: resolved`. Do NOT edit `map.md`.

## Exact digest file names (`docs/digests/`)

`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt` (source), `obsidian_blacksmithgu-obsidian-dataview-docs-digest.txt` (docs), `obsidian_mdelobelle-metadatamenu-src-digest.txt`, `obsidian_mdelobelle-metadatamenu-docs-digest.txt`, `lsp_rvben-rumdl-src-digest.txt` (rumdl source; the `lsp_` prefix is a directory-grouping artifact — rumdl is used here only as a Rust config/options comparable, not as an LSP).

## Answer

The precedent research is complete in `.scratch/arch-redesign/research/04-values-coercion-validation-precedent.md`. Eight lessons cover: (1) ingestion-time coercion cascades vs. avoiding query re-sniffing (Dataview/Polars); (2) explicit numeric discriminant rank vs. Dataview's broken alphabetical type ordering (SQLite/Dataview); (3) lossless integer/float preservation (`serde_json`/TOML) preventing 64-bit precision loss; (4) schema as a non-blocking interpretive overlay (Metadata Menu) rather than failing note parsing; (5) typed options per owner with central validation typestate (rumdl); (6) diagnostic validation separating from typed parsing to avoid dropping malformed notes (Alexis King/`jsonschema`); (7) native civil temporal representation over zero-heuristics (TOML/`jiff`); and (8) unifying multi-tree conversion hops (`FieldValue`/`NoteFieldValue`). Each lesson includes transferability tests against Traces' 20K synchronous CLI and redb constraints alongside explicit negative lessons.
