# What is the query core: grammar, IR/plan, rows, output formatting?

Type: grilling
Status: open
Blocked by: 6, 13, 18, 19, 24

## Question

Decide the internal architecture of the query subsystem. Decide one item per
bullet:

- Choose one predicate language, or keep separate source and filter grammars.
- Decide whether a logical plan with a pushdown protocol exists, or direct
  interpretation over the settled facet contracts suffices.
- Decide `GroupBy` semantics: real grouping, removal, or the sort alias.
- Compare typed `QueryBuilder` filters with string synthesis. The CLI
  synthesises DSL strings today (`src/cli/task.rs:42-64`).
- Define the `QuerySet` laziness contract for templates. Verified lazy: an `Arc`
  base plus an `ExecutionPlan`; rows clone only at terminal materialization
  (`research/01`, claim 7).
- Select the row representation. `QueryRow` holds `Arc<WorkspaceIndex>` today
  (`src/query/results.rs:44,464`).
- Set the ownership of `format.rs` output rendering.
- Preserve diagnostic spans through the template boundary.
- Preserve the `cmp_document_order` correctness invariant for multi-row notes
  (`src/query/results.rs:431`; research 01, claim 6: it ignores `item_idx`
  today).

Facts: `survey/query.md` (all), `src/query/plan.rs`,
`src/query/grammar/{source,filter}.rs`, `src/query/results.rs:44,431,464`,
`src/query/format.rs`.

Inputs: `research/06`, value model, coercion policy, read-model decision.

## Alternatives

Each alternative obeys the settled constraints below. The keep-as-is comparison
stays inside the remaining scope: ticket 24 already removes the dual execution
path, `SourceResolver`, and the whole-index row. Do not re-list those removals
as alternatives.

- Keep two grammars and the transform-list plan. Add the facet candidate
  selection that ticket 24 requires. State what withstands scrutiny.
- Unify the grammars. Extract pushdown candidates with term splitting (research
  06: SQLite `whereSplit`, DataFusion pushdown protocol).
- Use a typed expression builder as the single IR. The DSL becomes one frontend
  of it.
- Keep as is within remaining scope: two grammars, the transform-list plan,
  `GroupBy` as the sort alias, `format.rs` inside query, and CLI string
  synthesis. Compare its cost with the IWE cost model (research 12), not as a
  head-to-head result.

## Settled constraints

These contracts come from resolved tickets. Every alternative must obey them.
This ticket decides inside them.

- One execution plan serves the CLI and hosts (ticket 24). The CLI reads
  candidates from the store. Hosts read a pinned generation.
- Candidates come from facet scans and range keys over the eleven facet tables.
  The four multimap tables are removed (ticket 24). A range key exists only for
  an eligible note-level scalar field, and only when the candidate set provably
  contains every match. Otherwise the executor scans.
- Source selection is the indexable subset of the predicates, not a separate
  language stage (research 06). `All`, `Not`, and a source-less query enumerate
  the `files` paths and narrow with safe predicates. A folder `Path` uses a path
  range. `Tag` uses tag keys. `Class` uses explicit class keys or the bound tag
  and path atoms.
- A predicate moves before `limit`, `flatten`, or a transform only when its
  equivalence is proved for that row mode (ticket 24). This proof is this
  ticket's decision.
- A row owns its matching `Arc<FileEntry>`, not the whole index (ticket 24).
  `WorkspaceIndex` stays total and sorted. Index results must equal a fresh
  parse (ticket 16, frozen row).
- A query runs on one coherent generation and keeps its pinned frame across lazy
  materialization (tickets 21 and 24). Each query result names the generation it
  read as the incarnation and revision pair (ticket 25).
- Class axes stay frontmatter-only. The index stores no derived class. Class
  binding by tag and path expands at query time into `Class`, `Tag` and `Path`
  atoms (tickets 20 and 24).
- Query and template capabilities stay reachable. stdout payload shapes and exit
  codes stay frozen (ticket 16). DSL spelling is migratable through the change
  table.
- The core stays synchronous by default (ticket 17). Evidence vault sizes run
  from 50 to 50,000 notes.

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). The model has no borrowed mirror
enum. Any view enum is query-local. `NoteFieldValueRef` is deleted only after
benchmarking owned against view for `file.name` sort and filter
(`resolve_file_ref` borrows `&str` today). Computed fields (`file.size`,
`list.line`, `list.depth`, `list.parent`) become `Int`. Filters and sorts use
`Int` and `Float` with lossless cross-number comparison (ticket 19). The `i64`
to `f64` loss at the note boundary is not preserved (ticket 16, Not-preserved
list).

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Handed to this
ticket:

- Filters use type brackets. Across brackets `==` is false, and a relational
  comparison does not match. Temporal point/period membership is a filter
  predicate, not a total sort relation. Sorting keeps an explicit total rank
  order. Sort-time sniffing is deleted.
- Quoted literals adapt per row from pre-parsed candidates. Candidates include
  validated `CalendarPeriod` entries for date-shaped comparisons, arithmetic,
  and the `contains` target. Thread this through `ValueExpr`. Bare `5` tokenizes
  as `Int` and `5.0` as `Float`.
- Membership stays separate from the sort key. A period compares against an
  instant through the instant's local civil date, so a skipped next day does not
  invalidate a real prior day (ticket 21). Period/period equality is the exact
  `(start, unit)` identity (ticket 21, civil periods).
- Grouping needs an equivalence relation for its key. Keep ticket 19's exact
  numeric equality. Never use period-to-point membership as a key. The remaining
  cross-kind key policy is open.
- Open language questions: `in (...)` and ranges, relative keywords (`today`,
  `eom`), existence and type predicates, string functions (`lower`,
  `startswith`, and `string()` only if needed), a truncating date function for
  day matching, and null placement. These are questions, not approved features.
  Ticket 30 owns the relative keywords and the temporal dialect. If the
  resolution adopts any of them, the change table must name the new language.
- The grammar and executor need advisory stderr notices. They notify for rows
  excluded by a bracket mismatch, and for a date-shaped but invalid quoted
  literal (ticket 19). A parse-time NaN-to-Null notice rides the same mechanism
  (ticket 34, coercion inputs). Ticket 34 owns the notice envelope and display.
  stdout and exit codes stay unchanged (ticket 16).

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). Delete
`FileClassExpander`, its crate-root impl, the `&mut ClassExpansionMode`
out-parameter and `Option<Arc<dyn>>`. The query service receives a plain class
hierarchy (keyed by one normalising `ClassName`) from the app layer. It returns
unknown classes as data instead of logging. It behaves the same with or without
a hierarchy attached.

Verified latent hazard: with no expander attached, store queries fall back to
exact match (`src/query/service.rs:402-406`) but in-memory filters match nothing
(`src/query/grammar/source.rs:228-229`, empty class set). Production always
attaches one when classes are used (`src/cli/mod.rs:478-480`,
`src/template/engine/query.rs:121-122`). This gap is a hazard for a new host or
library caller, not a current user-visible bug. Where the hierarchy type lives
is for this ticket and [ticket 35](./35-crate-topology-and-public-surface.md).

Binding (also from ticket 20): expand `@class` (and `@class*`) into `Class | Tag
| Path` atoms from class-owned `[binding]` data (tags with nested matching,
vault-relative globs with `!` exclusions) handed in by the app layer. A note
with a known explicit class uses only that class, so bound members need an
exclusion atom ("has a known explicit class"). Optimise folder-style globs
(`folder/**`) to the `paths_in_folder` lookup and benchmark the remaining `Path`
scan (currently linear, `src/query/service.rs:417-424`).

Binding parity is a completion obligation. The membership resolver and the
validation resolver must agree, tested on the unreduced class set (ticket 20).
Two expansion paths diverged before (findings ledger, class expander row). An
unknown explicit class does not suppress tag-bound membership (ticket 24,
adversarial correction).

## Inputs from the IWE research

See [research 13](../research/13-iwe-query-schema-and-agent-write-gating.md).
IWE queries use a serialisable document (YAML or JSON, MongoDB-style `$`
operators) with four operations (`find`, `count`, `update`, `delete`). The
language has a published JSON Schema (`iwe docs query-schema`, iwe v0.23.0). It
has graph operators with depth bounds. It has no pushdown: evaluation is
in-memory over the whole graph [V].

[H] Question for this ticket: should the query IR have a serde form? This lets
LSP and MCP hosts build queries as data rather than as DSL strings.

Its value semantics converge with [ticket
19](./19-coercion-and-comparison-policy.md) [V:
`crates/liwe/src/query/filter.rs`, `sort.rs`, `collation.rs`]. It uses strict
scalar types, cross-type `==` false, and cross-type ordering undefined for
filters. It uses ICU natural collation, nulls first ascending, and a rank-based
total order for sort.

Also: `update` and `delete` are all-or-nothing across documents, need `expect`
count guards, and accept `dry_run` [V]. Operations `find` (structured
projection) and `retrieve` (agent context assembly with graph expansion and
token budget) are separate. Retrieve is a product feature outside the
destination.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md). The
service acquires a new frame per standalone query. A render/request supplies its
shared frame. Resolve relative expressions and constant temporal operands once,
but keep one pinned calendar capability in a plan where row-dependent coercion
needs it. No evaluator clock or ambient zone.

`QuerySet` retains its creator's plan and frame. Terminal access (including
template iteration, indexing and formatting) returns or propagates a memoized
error instead of silently returning no rows. Keep period membership predicates
distinct from field equality and the total sort key. The period/instant
comparison uses the instant's local civil date, so a skipped next day cannot
invalidate a real prior day.

Lazy errors stay typed. A materialization or zone failure is an error, never a
missing value or a blank cell (tickets 21 and 31). A corrupted or missing
required facet is a complete cache miss with the fresh-parse fallback, not
partial rows (tickets 24 and 25). That fallback exits 0 and prints one stderr
notice (ticket 24 change table).

## Inputs from the note document and index read model decisions

See [ticket 23's Answer](./23-note-document-model.md) and [ticket 24's
Answer](./24-index-read-model-and-query-execution.md).

- This ticket owns the row type, the residual filter, the pushdown protocol and
  membership. A row owns its matching `Arc<FileEntry>`, not the whole index.
- Ticket 24 removes `SourceResolver`. `All`, `Not` and a source-less query
  enumerate `files` paths, then narrow with safe predicates.
- A folder `Path` uses a path range. `Tag` uses tag keys. `Class` uses explicit
  class keys or the bound tag and path atoms.
- Ticket 24 removes the four multimap tables. Pushdown uses facet scans and
  range keys. The range-key encoding is still unmeasured.
- Ticket 24 keeps one execution plan for the CLI and for hosts.
- Ticket 23 asks this ticket for the item order, `item_idx` and the membership
  operation.
- One value is a scalar and several values are a list. This rule covers note
  fields and item fields.

## Inputs on pushdown soundness

These facts bound the pushdown protocol. They come from ticket 24's adversarial
corrections and research 06:

- A candidate set must contain every matching row. The residual filter still
  runs on all candidates (research 06, DataFusion `Inexact`).
- Task rows read item metadata first. Item metadata can override the note value
  (`src/query/results.rs:320-333`). A note-field key cannot safely prune a
  filter on a task row. Scope a range key to a row mode where the key and the
  filter read the same value.
- A facet key writer must read values through the effective accessor that ticket
  23 defines. The current iterator and `Note::get` disagree on inline fields
  (ticket 24, adversarial correction; ticket 23 fixes the accessor).
- The persisted path bytes must sort in the order the folder range needs. Ticket
  32 owns the byte form. Ticket 24 sets the sort-order check at the build and
  load boundaries.

## Handoffs

These are contracts this ticket must honor, not decide here:

- [Ticket 28](./28-live-buffer-and-persisted-index-consistency.md) owns the
  buffer overlay on a pinned generation. The row and plan contracts must give
  the overlay one attach point. Do not design the overlay here.
- [Ticket 32](./32-path-and-capability-model.md) owns the persisted path form
  and root identity. State the sort-order need; do not pick the bytes.
- [Ticket 30](./30-temporal-boundary-and-dialects.md) owns the relative
  keywords, the temporal dialect, and the filename period parser. `FileMeta.day`
  is removed (ticket 24), so filename periods are derived at query time. Wait on
  ticket 30 for that parser and for `date.format`.
- [Ticket 31](./31-template-engine-boundary.md) owns the minijinja bridge, typed
  versus stringified values, and span preservation at callbacks. Keep
  `QueryError` spans intact so tickets 31 and 34 can decide the rest.
- [Ticket 33](./33-workspace-host-and-composition.md) owns composition,
  `normalize_source_input` and task-filter synthesis. The typed-builder versus
  string question decides the core surface. Ticket 33 decides who calls it.
- [Ticket 34](./34-error-and-diagnostic-architecture.md) owns the diagnostic
  envelope, severities and notice display.
- [Ticket 35](./35-crate-topology-and-public-surface.md) owns crate topology and
  visibility. The hierarchy-type module is decided with ticket 35.

## Resolution must name

- The chosen grammar and plan shape, the alternatives considered, the keep-as-is
  comparison within remaining scope, and what withstands scrutiny.
- The pushdown protocol: which predicates are candidates, the candidate superset
  rule, residual filter placement, and the per-row-mode equivalence proof for
  any predicate pushed before `limit`, `flatten` or a transform.
- The row type: row modes for note, list and task rows, `item_idx` semantics,
  the `cmp_document_order` fix, and the item order contract that ticket 23
  requested.
- The `GroupBy` decision and its key equivalence relation under ticket 19's
  rules.
- The membership operation for temporal point/period, distinct from equality and
  from the sort key (tickets 19 and 21).
- The lazy-materialization contract: memoized typed errors, generation naming,
  and the overlay attach point for ticket 28.
- The output-formatting decision for `format.rs`, with the template methods and
  stdout payload shapes kept reachable (ticket 16).
- Every observable behavior change as a change-table row (ticket 16). A language
  addition counts as a decision only when the resolution names it as new
  language.
- The benchmark and measurement plan with sources. Sources: owned versus view
  for `file.name` (ticket 18). The folder glob optimisation and the `Path` scan
  (tickets 20 and 24). The range-key encoding costs (ticket 24). Full row
  materialization on a real vault (ticket 24). The same-stem candidate scale at
  5,000 (ticket 24).
- The owner of each new invariant, and the evidence limits. Unmeasured encodings
  stay marked. IWE numbers stay a cost model, not a head-to-head result (ticket
  24).
