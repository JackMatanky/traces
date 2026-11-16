# Which scout claims survive adversarial verification?

Type: research
Status: resolved
Blocked by: none

## Question

The ten breadth-first scouts (`survey/*.md`) made claims that later decisions
will lean on. Which claims stay true after an adversarial re-check against the
code (not docs and comments)? What are the corrected citations?

1. `field::FieldValueRef` coerces ISO strings to temporal values at
   deserialization (`src/field.rs`). Frontmatter takes a 3-stage conversion
   `noyalib::Value -> FieldValueRef -> NoteFieldValue` (`src/note/metadata.rs`).
   Four value enums exist: `FieldValue`, `FieldValueRef`, `NoteFieldValue`,
   `NoteFieldValueRef`.
2. `NoteFieldValueRef::compare` promotes Date to DateTime through the ambient
   local zone (`src/note/field.rs`), diverging from `NoteFieldValue`'s derived
   `PartialEq`.
3. `ConfigDigests`/`Config::hash` have no production consumer
   (`src/config/model/digest.rs`, `src/config/model.rs`); config drift is pinned
   as stale (`src/index/service.rs`).
4. `QueryService::run_from_store` builds a partial `WorkspaceIndex` that breaks
   its documented totality (`src/query/service.rs`). `BooleanExpr::Not` falls
   back to a full path scan. In-memory `run` is a linear scan.
5. A single path add/delete forces reading every note from redb and re-resolving
   all links (`src/index/refresh.rs`); content-only edits do not.
6. `QueryRow::cmp_document_order` ignores `item_idx` (`src/query/results.rs`).
7. Template: path inspection accepts unconfined absolute paths
   (`src/template/engine/path.rs`). `WriteMode::DryRun` skips target confinement
   (`src/template/service.rs`). Query values are stringified at the minijinja
   boundary (`src/template/engine/query.rs`). Every chained `QuerySet` method
   clones the row vec. But `query::QuerySet` itself is lazy/memoized.
8. `src/schema` never reads note values; the index extracts `fileClass` from
   frontmatter with no schema involvement (`src/index/service.rs`).
9. `parse_markdown` runs pulldown-cmark twice (`src/note/parser.rs`). Byte spans
   are discarded. List-item fields/tags are duplicated into
   `Note.inline_fields`/`Note.tags` (`src/note/parser.rs`). Multi-valued inline
   fields are stored, but only `first()` is ever read.
10. Documentation drift: `src/note/GLOSSARY.md` slugs/footnotes/callouts,
    `src/GLOSSARY.md` Occurrence/NavTarget/Resolution/Ambiguity, and "Avoid:
    relative path" vs `RelativePath`. Two ADRs are numbered 0009.
11. `speedate` and `winnow` each have exactly one call site
    (`src/temporal/date.rs`, `src/temporal/lexer.rs`); `indextree` is used only
    by the 64-candidate trie (`src/index/trie.rs`).
12. CLI owns domain logic: scaffolding (`src/cli/init.rs`), filesystem-probing
    `normalize_source_input` (`src/cli/mod.rs`), DSL string synthesis
    (`src/cli/task.rs`); unbuffered `print!` panics on EPIPE.
13. The production module DAG is acyclic (10 tiers). `note -> config` is an
    upward edge (`src/note/parser/input.rs`). `file_tracker` has exactly two
    callers (`src/config/tracker.rs`, `src/config/trust.rs`). The edge table in
    `survey/cross-cutting.md` is accurate.
14. For each existing ADR (0001-0009), record which candidate redesign areas it
    constrains. Also record whether its stated invariant is still true in code.

Use codegraph + reading; do not run benchmarks (separate ticket). Source
surveys: all files under `.scratch/arch-design/survey/`.

## Output

For every numbered claim: verdict (confirmed / refuted / partially, with
correction), exact current file and symbol, and any new adjacent fact
discovered. Also list any *new* architectural facts found while checking.

Deliverable: `.scratch/arch-design/research/01-verify-load-bearing-claims.md`
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

Deliverable completed:
`.scratch/arch-design/research/01-verify-load-bearing-claims.md`. All 14 scout
claims and ADRs 0001–0009 were adversarially checked against production code: 12
confirmed, 2 partially confirmed with corrections. Claim 1: default
deserialization is passthrough; coercion needs explicit classify. Claim 7:
QuerySet method chaining does NOT clone the row vec, only Arc and ExecutionPlan.
ADR-0009 drift detection was checked as unbuilt at `2dc3250d`. **Superseded**:
it landed in main @ `207cc3fb` (see `reverification.md`). Four new architectural
facts documented.
