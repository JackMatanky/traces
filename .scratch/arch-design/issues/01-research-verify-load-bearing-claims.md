# Which scout claims survive adversarial verification?

Type: research
Status: resolved
Blocked by: none

## Question

The ten breadth-first scouts (`survey/*.md`) made claims that later decisions will lean on. Which of the following survive adversarial re-verification against the code (not docs/comments), and what are the corrected citations?

1. `field::FieldValueRef` coerces ISO strings to temporal values at deserialization (`src/field.rs:843`); frontmatter takes a 3-stage conversion `noyalib::Value -> FieldValueRef -> NoteFieldValue` (`src/note/metadata.rs:71-72`); four value enums exist (`FieldValue`, `FieldValueRef`, `NoteFieldValue`, `NoteFieldValueRef`).
2. `NoteFieldValueRef::compare` promotes Date to DateTime through the ambient local zone (`src/note/field.rs:297-300`), diverging from `NoteFieldValue`'s derived `PartialEq`.
3. `ConfigDigests`/`Config::hash` have no production consumer (`src/config/model/digest.rs`, `src/config/model.rs:148-155`); config drift is pinned as stale (`src/index/service.rs:1749-1780`).
4. `QueryService::run_from_store` builds a partial `WorkspaceIndex` that breaks its documented totality (`src/query/service.rs:162-166`); `BooleanExpr::Not` falls back to a full path scan (`:395`); in-memory `run` is a linear scan (`:323`).
5. A single path add/delete forces reading every note from redb and re-resolving all links (`src/index/refresh.rs:361-386,487`); content-only edits do not.
6. `QueryRow::cmp_document_order` ignores `item_idx` (`src/query/results.rs:431`).
7. Template: path inspection accepts unconfined absolute paths (`src/template/engine/path.rs:162`); `WriteMode::DryRun` skips target confinement (`src/template/service.rs:225`); query values are stringified at the minijinja boundary (`src/template/engine/query.rs:604`); every chained `QuerySet` method clones the row vec (`:398`) while `query::QuerySet` itself is lazy/memoized.
8. `src/schema` never reads note values; the index extracts `fileClass` from frontmatter with no schema involvement (`src/index/service.rs:1656-1840`).
9. `parse_markdown` runs pulldown-cmark twice (`src/note/parser.rs:108,121`); byte spans are discarded; list-item fields/tags are duplicated into `Note.inline_fields`/`Note.tags` (`src/note/parser.rs:488-498`); multi-valued inline fields are stored but only `first()` is ever read.
10. Documentation drift: `src/note/GLOSSARY.md` slugs/footnotes/callouts, `src/GLOSSARY.md` Occurrence/NavTarget/Resolution/Ambiguity, and "Avoid: relative path" vs `RelativePath`; two ADRs numbered 0009.
11. `speedate` and `winnow` each have exactly one call site (`src/temporal/date.rs:296`, `src/temporal/lexer.rs:7`); `indextree` is used only by the 64-candidate trie (`src/index/trie.rs`).
12. CLI owns domain logic: scaffolding (`src/cli/init.rs:52-83`), filesystem-probing `normalize_source_input` (`src/cli/mod.rs:436-468`), DSL string synthesis (`src/cli/task.rs:42-64`); unbuffered `print!` panics on EPIPE.
13. The production module DAG is acyclic (10 tiers), `note -> config` is an upward edge (`src/note/parser/input.rs:9`), `file_tracker` has exactly two callers (`src/config/tracker.rs`, `src/config/trust.rs`); the edge table in `survey/cross-cutting.md` is accurate.
14. For each existing ADR (0001-0009) record which candidate redesign areas it constrains and whether the ADR's stated invariant is still true in code.

Use codegraph + reading; do not run benchmarks (separate ticket). Source surveys: all files under `.scratch/arch-design/survey/`.

## Output

For every numbered claim: verdict (confirmed / refuted / partially, with correction), exact current `path:line`, and any new adjacent fact discovered. Also list any *new* architectural facts found while verifying.

Deliverable: `.scratch/arch-design/research/01-verify-load-bearing-claims.md` (<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp `get_item_source`, or GitHub raw files) and cite repo path + symbol/line for every claim. Structure: (1) what each comparable actually does (verified from source), (2) the invariants and who owns them, (3) tradeoffs and costs they pay, (4) transferable lessons for Traces, (5) lessons that do NOT transfer and why, (6) what this changes or challenges in the Traces hypotheses named above. Mark facts [V], interpretation [H]. Do not propose the final decision; that belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Deliverable completed: `.scratch/arch-design/research/01-verify-load-bearing-claims.md`.
All 14 scout claims and ADRs 0001–0009 were adversarially verified against production code: 12 confirmed, 2 partially confirmed with corrections (Claim 1: default deserialization is passthrough, coercion requires explicit classify; Claim 7: QuerySet method chaining does NOT clone the row vec, only Arc and ExecutionPlan). ADR-0009 drift detection was verified unbuilt at `2dc3250d`; **superseded**: it landed in main @ `207cc3fb` (see `reverification.md`). Four new architectural facts documented.
