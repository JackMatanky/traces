# How do query engines structure predicate languages, plans, and pushdown?

Type: research
Status: resolved
Blocked by: none

## Question

Traces has two grammars (source `--from`, 1.2k LoC; filter `--where`, 2.1k LoC)
with separate lexers/ASTs. An `ExecutionPlan` is a `Vec<QueryTransform>` with
local step fusion. A `GroupBy` is only an ascending sort. A `QuerySet` is lazy
in `query` but cloned eagerly in the template bridge. Two execution strategies
exist: in-memory linear scan vs redb secondary-table pushdown. See
`survey/query.md` H1/H3/H5/H7, `survey/template.md` H3, `survey/index.md` H1.

Study these comparables:

- DataFusion logical plan + `PruningPredicate`/filter pushdown +
  `TableProvider::supports_filters_pushdown`
- Polars lazy `LazyFrame`/predicate pushdown
- jaq/jq filter model
- Obsidian Dataview DQL (`FROM` source vs `WHERE`, true `GROUP BY` semantics,
  `src/query/engine.ts`)
- Tasks plugin query grammar
- SQLite query planner for index selection
- `tantivy` or `roaring`, only if they inform in-memory secondary indices

Focus:

- Is source selection a distinct concept, or an indexable subset of predicates?
- When is a real IR warranted versus direct interpretation at this scale
  (personal vaults, ~1k-20k notes)?
- How is pushdown capability expressed: trait on the storage, or engine knowing
  storage?
- What lazy collection semantics are exposed to an embedded template language?
- What does `GROUP BY` mean in a flat markdown output?

Informs: query core and plan, index read model and query execution, template
engine boundary.

Deliverable: `.scratch/arch-design/research/06-query-plan-precedent.md` (<= 300
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

Deliverable completed at
[.scratch/arch-design/research/06-query-plan-precedent.md](../research/06-query-plan-precedent.md).

> Correction (orchestrator): finding 5 in this research rests on a scout claim
> that `research/01` claim 7 REFUTED. The claim said "eliminate eager `QuerySet`
> cloning at `engine/query.rs:398`". In truth, chained `QuerySet` methods clone
> an `Arc` + plan, not rows. The lazy-builder lesson stands only as a pattern.
> It does not identify a defect.
