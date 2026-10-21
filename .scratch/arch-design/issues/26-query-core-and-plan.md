# What is the query core: grammar, IR/plan, rows, output formatting?

Type: grilling
Status: open
Blocked by: 6, 18, 19, 24, 13

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

See [ticket 18's Answer](./18-value-model.md). The model has no borrowed
mirror enum; any view enum is query-local. `NoteFieldValueRef` is deleted only
after benchmarking owned vs view for `file.name` sort and filter
(`resolve_file_ref` borrows `&str` today). Computed fields (`file.size`,
`list.line`, `list.depth`, `list.parent`) become `Int`.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Handed to
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

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). Delete
`FileClassExpander`, its crate-root impl, the `&mut ClassExpansionMode`
out-parameter and `Option<Arc<dyn>>`. The query service receives a plain class
hierarchy (keyed by one normalising `ClassName`) from the app layer, returns
unknown classes as data instead of logging, and behaves the same with or
without a hierarchy attached. Verified latent hazard: with no expander
attached, store queries fall back to exact match (`src/query/service.rs:402-406`)
but in-memory filters match nothing (`src/query/grammar/source.rs:228-229`,
empty class set). Production always attaches one when classes are used
(`src/cli/mod.rs:478-480`, `src/template/engine/query.rs:121-122`), so this is
a hazard for a new host or library caller, not a current user-visible bug.
Where the hierarchy type lives is for
this ticket and 32.

Binding (also from 20): expand `@class` (and `@class*`) into `Class | Tag |
Path` atoms from class-owned `[binding]` data (tags with nested matching,
vault-relative globs with `!` exclusions) handed in by the app layer. A note
with a known explicit class uses only that class, so bound members need an
exclusion atom ("has a known explicit class"). Optimise folder-style globs
(`folder/**`) to the `paths_in_folder` lookup and benchmark the remaining
`Path` scan (currently linear, `src/query/service.rs:417-424`). Membership and
the validation resolver must agree, tested on the unreduced class set.

## Inputs from the IWE research

See [research 13](../research/13-iwe-query-schema-and-agent-write-gating.md).
IWE's query is a serialisable document (YAML or JSON, MongoDB-style `$`
operators) with four operations (`find`, `count`, `update`, `delete`), a
published JSON Schema for the language (`iwe docs query-schema`, iwe v0.23.0),
graph operators with depth bounds, and no pushdown: evaluation is in-memory
over the whole graph [V]. [H] Question for this ticket: should the query IR
have a serde form so that LSP and MCP hosts build queries as data rather than
as DSL strings? Its value semantics converge with [ticket
19](./19-coercion-and-comparison-policy.md) (strict scalar types, cross-type
`==` false, cross-type ordering undefined for filters, ICU natural collation,
nulls first ascending, a rank-based total order for sort) [V:
`crates/liwe/src/query/filter.rs`, `sort.rs`, `collation.rs`]. Also: `update`
and `delete` are all-or-nothing across documents, require `expect` count guards
and accept `dry_run` [V]; `find` (structured projection) and `retrieve` (agent
context assembly with graph expansion and token budget) are separate
operations. Retrieve is a product feature outside the destination.
