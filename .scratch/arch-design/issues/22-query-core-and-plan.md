# What is the query core: grammar, IR/plan, rows, output formatting?

Type: grilling
Status: open
Blocked by: 6, 14, 15, 20

## Question

Decide the query subsystem's internal architecture: one predicate language vs
separate source and filter grammars; whether a logical plan with pushdown
capability exists or direct interpretation suffices; `GroupBy` semantics (real
grouping or removed); typed `QueryBuilder` filters vs string synthesis (CLI
synthesises DSL strings, `src/cli/task.rs:42-64`); `QuerySet` laziness contract
exposed to templates (verified lazy: `Arc` base + `ExecutionPlan`, rows cloned
only at terminal materialization, `research/01` claim 7); row representation
(`QueryRow` holds `Arc<WorkspaceIndex>`); `format.rs` output rendering
ownership; diagnostics spans through the template boundary; `cmp_document_order`
correctness invariant for multi-row notes.

Facts: `survey/query.md` all, `src/query/plan.rs`,
`src/query/grammar/{source,filter}.rs`, `src/query/results.rs:44,431,464`,
`src/query/format.rs`.

Alternatives: keep two grammars + simple plan (state what withstands); unify
with pushdown extraction; typed expression builder as the single IR with DSL as
a frontend. Include keep-as-is.

Inputs: `research/06`, value model, coercion policy, read-model decision.
