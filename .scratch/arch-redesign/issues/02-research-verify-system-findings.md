# Research: verify index/query/template/cli/topology findings

Type: research
Status: resolved

## Question

The area surveys in `.scratch/arch-redesign/survey/` were written by single-pass subagents and are unverified (see map Notes). Produce a **verification ledger** for every decision-critical claim below: re-check each against the source on `main` (use `codegraph_explore` and direct reads; do NOT trust the survey, glossaries, ADRs, or the older `.design/` reports), and record `VERIFIED` / `PARTIAL` / `REFUTED` / `UNVERIFIABLE` with `file:line` evidence and a corrected one-sentence statement when the survey was wrong or imprecise. Do not propose designs. Do not modify any production code, test, bench, or doc; do not run cargo/mise tasks.

Claims to check (add any adjacent discrepancy you notice, clearly marked as new):

1. **Index store.** `src/index/store.rs` mixes redb lifecycle/recovery, whole-table codec, incremental patching, secondary-index extraction from `Note` (`IndexDimension::visit_values`), and query-backend reads; `query::service` calls `IndexStore` methods directly via `pub(crate)`. List the distinct responsibility clusters and the method set per cluster. Verify the table set in `tables.rs` (survey: 8 tables incl. `cache_format`; ADR-0005 and the LSP map say 3 or 8 with `lists`; note `index-query/17` speaks of a "legacy 7-table store" gaining an 8th `meta` table, so reconcile the counts), the hardcoded `cache_format`, the debug-only assertion in `SortedByPath::assumed_sorted`, and how an index is built or refreshed when the on-disk store is unwritable (fail-open path). **Config-drift detection / `meta` table / `IndexFingerprint` / `RefreshState::Drifted` are IN-FLIGHT user work (`index-query/17`, after `config-service/07`): record their current state on `main` and in `.worktrees/config-digest-sections`, tag the claim `MOVING (07/17)`, and do NOT report their absence as a defect.**
2. **Two query paths.** `QueryService::run` (in-memory `Arc<WorkspaceIndex>`, linear scan in `matched_file_rows`) vs `run_from_store` (redb multimap candidate selection, then a transient `WorkspaceIndex`). Which production callers (`cli`, `template`) use which, and when? Do they share the row-evaluation/filter code or duplicate it? Do they produce identical results for `Not(...)` sources, class expansion, and folder sources?
3. **Query row/plan facts.** Verify `QueryRow`'s composition and approximate size and whether each row clones an `Arc<WorkspaceIndex>`; verify the claimed `cmp_document_order` defect (ignores `item_idx` for list/task rows) with a concrete reasoning trace, and say whether any test pins it; verify `TextShape::classify` callers (filter and sort) and that sources (`--from`) and filters (`--where`) use separate lexers/ASTs, plus the overlap in what they can express (tags, classes, folders).
4. **FileClassExpander seam.** Count implementations and constructors of the `FileClassExpander` trait, who builds the glue (`cli`, `template`), and whether `query` truly has no dependency on `schema` (production and test).
5. **Template engine.** Verify: (a) who constructs `IndexerService`/`QueryService`/`SchemaService` inside `template` (is it a second composition root?) and what `TemplateService` holds; (b) the `minijinja::State` temp-storage caches (`query.index_cache`, `schema.service_cache`); (c) whether `QuerySet::call_method` clones rows (deep) or an `Arc` (shallow) on each chained call; (d) that root templates are re-parsed per render; (e) how much of `template/engine/date.rs` is Moment-token translation / anchor math that has no counterpart in `temporal`, vs thin adaptation; (f) whether `template::engine::query`/`schema` hold domain logic or only adapters.
6. **CLI composition.** Verify where `ConfigService`/`IndexerService`/`QueryService`/`SchemaService`/`TemplateService` are constructed per command (file:line); count `CliError` variants (the two surveys disagree: 22 vs 52+); confirm whether any non-`cli` code writes to stdout/stderr; verify `normalize_source_input` touches the filesystem; confirm `cli/task.rs` builds filter strings.
7. **Crate topology.** Recompute the top-level module → module production dependency matrix mechanically (exclude `#[cfg(test)]` code) and compare with `survey/cross-cutting.md` §5; list every mismatch, every upward edge, and any cycle. Verify the `traces::testing` surface (the survey says exactly 45 identifiers) and which `tests/` and `benches/` files use which items; size of `TestProject` and why subsystems cannot be composed from public types alone. Verify the survey's claim list of "42 error types" and the "6 layers" error-propagation example; verify the `GLOSSARY-MAP.md` mermaid edges vs actual edges (the cross-cutting survey cited non-existent `src/cli/query.rs`/`src/cli/schema.rs`; find the truth).
8. **Persisted-format coupling.** Which types are serialized into redb (`Note`, `FileMeta`, links…) and which carry serde derives *only* because of persistence; what version/format guard exists beyond `cache_format`.

## Output

Write `.scratch/arch-redesign/research/02-verify-system-findings.md` (table-first, dense, <= ~3000 words). On finishing, append an `## Answer` section to THIS ticket file (a 5-8 line summary: how many claims verified/partial/refuted, and the three most consequential corrections) and set `Status: resolved`. Do NOT edit `map.md` (the orchestrator records the decision line).

## Answer

Verification ledger complete in `.scratch/arch-redesign/research/02-verify-system-findings.md` against baseline `0c43149d` (and tracking in-flight worktree `.worktrees/config-digest-sections`).
Across the 8 claims: 4 VERIFIED (Claims 2, 3, 4, 8), 4 PARTIAL (Claims 1, 5, 6, 7; with Claim 1 additionally marked MOVING 07/17), and 0 UNVERIFIABLE.
Three most consequential corrections to the single-pass surveys:

1. `QuerySet` chained calls (`.filter()`, `.sort()`, `.limit()`) clone a shallow `Arc<Vec<QueryRow>>` and append to `ExecutionPlan`; the survey claim of deep row copying on every chained call is REFUTED.
2. `CliError` has exactly 22 enum variants (`src/cli/error.rs:58-231`); the cross-cutting survey's claim of "52+ variants" is REFUTED, and its "6-layer error propagation chain" contains non-existent variants and false wrapping links.
3. The persisted redb store contains 8 tables in `tables.rs` with NO `lists` table (list items are serialized inside `Note`), while fail-open behavior applies strictly to incremental persist failures and fails closed on initial store open.
