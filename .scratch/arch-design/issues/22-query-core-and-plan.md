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

## Inputs from the value model decision

See [ticket 14's Answer](./14-value-model.md). The model has no borrowed
mirror enum; any view enum is query-local. `NoteFieldValueRef` is deleted only
after benchmarking owned vs view for `file.name` sort and filter
(`resolve_file_ref` borrows `&str` today). Computed fields (`file.size`,
`list.line`, `list.depth`, `list.parent`) become `Int`.

## Inputs from the coercion and comparison decision

See [ticket 15's Answer](./15-coercion-and-comparison-policy.md). Handed to
this ticket:

- Filters use type brackets (cross-kind `==` false, relational no match); sort
  keeps a total rank order and is built with an explicit context; sort-time
  sniffing is deleted.
- Quoted literals adapt per row from pre-parsed candidates, including inside
  arithmetic and the `contains` target; thread this through `ValueExpr`. Bare
  `5` must tokenize as `Int` and `5.0` as `Float`.
- Not yet decided here: `in (...)` and ranges, relative keywords (`today`,
  `eom`), existence and type predicates, string functions (`lower`,
  `startswith`; `string()` only if a use appears), a truncating date function
  for day matching, null placement, and the `GroupBy` key relation.
- Two advisory stderr notices are required of the grammar and executor (rows
  excluded by a bracket mismatch; a date-shaped but invalid quoted literal).
