# How do query engines structure predicate languages, plans, and pushdown?

Type: research
Status: resolved
Blocked by: none

## Question

Traces has two grammars (source `--from`, 1.2k LoC; filter `--where`, 2.1k LoC) with separate lexers/ASTs, an `ExecutionPlan = Vec<QueryTransform>` with local step fusion, a `GroupBy` that is only an ascending sort, a `QuerySet` that is lazy in `query` but cloned eagerly in the template bridge, and two execution strategies (in-memory linear scan vs redb secondary-table pushdown). See `survey/query.md` H1/H3/H5/H7, `survey/template.md` H3, `survey/index.md` H1.

Study: DataFusion logical plan + `PruningPredicate`/filter pushdown + `TableProvider::supports_filters_pushdown`; Polars lazy `LazyFrame`/predicate pushdown; jaq/jq filter model; Obsidian Dataview DQL (`FROM` source vs `WHERE`, true `GROUP BY` semantics, `src/query/engine.ts`); Tasks plugin query grammar; SQLite query planner for index selection; `tantivy` or `roaring` only if they inform in-memory secondary indices.

Focus: is source selection a distinct concept from row filtering or just an indexable subset of predicates; when is a real IR warranted versus direct interpretation at this scale (personal vaults, ~1k-20k notes); how pushdown capability is expressed (trait on the storage vs engine knowing storage); lazy collection semantics exposed to an embedded template language; what `GROUP BY` means in a flat markdown output.

Informs: query core and plan, index read model and query execution, template engine boundary.

Deliverable: `.scratch/arch-design/research/06-query-plan-precedent.md` (<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp `get_item_source`, or GitHub raw files) and cite repo path + symbol/line for every claim. Structure: (1) what each comparable actually does (verified from source), (2) the invariants and who owns them, (3) tradeoffs and costs they pay, (4) transferable lessons for Traces, (5) lessons that do NOT transfer and why, (6) what this changes or challenges in the Traces hypotheses named above. Mark facts [V], interpretation [H]. Do not propose the final decision; that belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Deliverable completed at [.scratch/arch-design/research/06-query-plan-precedent.md](../research/06-query-plan-precedent.md).


> Correction (orchestrator): finding 5 in this research ("eliminate eager `QuerySet` cloning at `engine/query.rs:398`") rests on the scout claim REFUTED by `research/01` claim 7 — chained `QuerySet` methods clone an `Arc` + plan, not rows. The lazy-builder lesson stands only as a pattern; it does not identify a defect.
