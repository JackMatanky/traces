# What is the query core: grammar, IR/plan, rows, output formatting?

Type: grilling
Status: resolved
Blocked by: 6, 13, 18, 19, 24

## Question

Decide the internal architecture of the query subsystem. Decide one item per
bullet:

- Choose one predicate language, or keep separate source and filter grammars.
- Decide whether a logical plan with a pushdown protocol exists, or direct
  interpretation over the settled facet contracts suffices.
- Decide `GroupBy` semantics: real grouping, removal, or the sort alias.
- Compare typed `QueryBuilder` filters with string synthesis. The CLI
  synthesises DSL strings today (`src/cli/task.rs`).
- Define the `QuerySet` laziness contract for templates. Verified lazy: an `Arc`
  base plus an `ExecutionPlan`. Rows clone only at terminal materialization
  (`research/01`, claim 7).
- Select the row representation. `QueryRow` holds `Arc<WorkspaceIndex>` today
  (`src/query/results.rs`).
- Set the ownership of `format.rs` output rendering.
- Preserve diagnostic spans through the template boundary.
- Preserve the `cmp_document_order` correctness invariant for multi-row notes
  (`src/query/results.rs`; research 01, claim 6: it ignores `item_idx`
  today).

Facts: `survey/query.md` (all), `src/query/plan.rs`,
`src/query/grammar/{source,filter}.rs`,
`src/query/results.rs`,
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
- The grouping key needs an equivalence relation. Keep ticket 19's exact
  numeric equality. Never use period-to-point membership as a key. The remaining
  cross-kind key policy is open.
- The open language questions are: `in (...)` and ranges, relative keywords
  (`today`, `eom`), existence and type predicates, string functions (`lower`,
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
exact match (`src/query/service.rs`) but in-memory filters match nothing
(`src/query/grammar/source.rs`, empty class set). Production always
attaches one when classes are used (`src/cli/mod.rs`,
`src/template/engine/query.rs`). This gap is a hazard for a new host or
library caller, not a current user-visible bug. Where the hierarchy type lives
is for this ticket and [ticket 35](./35-crate-topology-and-public-surface.md).

Binding (also from ticket 20): expand `@class` (and `@class*`) into `Class | Tag
| Path` atoms from class-owned `[binding]` data (tags with nested matching,
vault-relative globs with `!` exclusions) handed in by the app layer. A note
with a known explicit class uses only that class, so bound members need an
exclusion atom ("has a known explicit class"). Optimise folder-style globs
(`folder/**`) to the `paths_in_folder` lookup and benchmark the remaining `Path`
scan (currently linear, `src/query/service.rs`).

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
  (`src/query/results.rs`). A note-field key cannot safely prune a
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
  and root identity. State the sort-order need. Do not pick the bytes.
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

## Answer

### Decisions recorded in the grilling session (2026-11-14 and 2026-11-15)

The maintainer confirmed every decision in this section. Two red-team rounds
and three audits tested the decisions before the maintainer confirmed them.

The project is pre-release. The `.redb` layout is Free, and the maintainer can
wipe it at will. Ticket 16 freezes stdout payload shapes, exit codes, and
template and query capabilities. DSL spelling stays migratable through the
change table.

Evidence files:

- [g1-probes.md](../research/29-prep/g1-probes.md): range-key store costs,
  sort order, normalization and candidate costs.
- [h2-redteam.md](../research/29-prep/h2-redteam.md): the first red-team
  round.
- [h3-redteam-round2.md](../research/29-prep/h3-redteam-round2.md): the
  second red-team round, the three audits, the grouping precedent, the
  Nushell lessons, and the list of reversals.
- [g2-pushdown-ux.md](../research/29-prep/g2-pushdown-ux.md): end-to-end
  pushdown measurements and query shapes.
- [z-grilling-guide.md](../research/29-prep/z-grilling-guide.md) and
  [h1-stress-designs.md](../research/29-prep/h1-stress-designs.md): the
  first-round options.

Marks: [V] means measured, run, or read in a cited source. [H] means an
inference. [P] means a target design, including an adopted proposal.
The [V] marks on probe results name the original evidence authors, not a new
measurement.
Chosen rules, change-table targets, invariant owners and handoffs below are [P]
unless marked otherwise.

#### Decision index

- Sections 1 and 2: the predicate IR, filter stages and pushdown scope.
- Section 3: grouping, keys, labels, counts and the CLI surface.
- Section 4: the typed builder and CLI synthesis.
- Section 5: pinned generations, memoization and lazy errors.
- Sections 6 and 9: row ownership, row identity and document order.
- Section 7: output rendering and per-group widths.
- Section 8: spans, clause ids and notices.
- Section 10: the reserved membership node.
- Later sections: change rows, measurements, invariant owners, evidence limits
  and handoffs.

#### Row terms used in this Answer

[V] Query modes are `Pages`, `Lists` and `Tasks` (`src/query/builder.rs`).
The row tag has only `RowType::Page` and `RowType::List { item_idx }`
(`src/query/results.rs`).
Pages mode creates a Page row for each matching file, including attachments
(`src/query/service.rs`).
Lists mode selects every list item. Tasks mode selects only items for which
`item.kind().is_task()` is true (`src/query/service.rs`).
Both item modes create the same List row tag (`src/query/service.rs`).
`item_idx` addresses `Note::lists()` (`src/query/results.rs`).
Thus, "Task rows" means rows selected by Tasks mode, not a third row tag.
"List rows" means rows selected by Lists mode, including its task items.
[P] The target keeps one row struct and the existing row-kind distinction
(section 6).
The exact Rust API stays with ticket 35.

#### Overrides by the maintainer that shape this ticket

1. Pushdown scope. The first implementation pushes Exact range keys for `==`
   and for `<`, `<=`, `>` and `>=`. The literal must be an UNQUOTED numeric,
   temporal or bool single-bracket literal. The first scope also covers
   And-intersection and source-plus-filter. There is no text range table at
   first. `Or`, `Not`, `!=`, quoted literals, `contains()`, function-wrapped
   atoms and arithmetic scan. This Answer states the FULL soundness protocol
   (Exact, Inexact, Unsupported; rules R1-R7), so later cases are additive.
   Gate: add a case only when a measured one-shot query exceeds 100 ms on a
   vault of 20,000 notes or fewer.
2. Flat task output depth. The depth fix applies to BOTH flat and grouped
   output. Depth uses active preceding ancestors in the same output list or
   group. Orphans start at zero. This touches frozen stdout.
   It is NOT on ticket 16's not-preserved list.
   The change table records it as an intentional defect fix.
   The maintainer acknowledged it.

Two more overrides belong to other tickets. The overlay path-set change is one
full re-resolve (ticket 28). Admission matches `.traces` case-insensitively
and nothing else (ticket 32). This ticket does not repeat them.

### 1. One predicate language, or separate source and filter grammars

Alternatives:

- Keep two grammars and the transform-list plan. Add the facet candidate
  selection that ticket 24 needs. This is the retain option.
- Unify the grammars into one text grammar. Extract pushdown candidates by
  term splitting (SQLite `whereSplit`, DataFusion pushdown, research 06).
- Use a typed expression builder as the single IR. The DSL becomes one
  frontend of it.
- Keep as is within the remaining scope: two grammars, the transform-list
  plan, `GroupBy` as the sort alias, `format.rs` inside query, and CLI
  string synthesis. The cost compares with the IWE cost model (research 12).
  It is not a head-to-head result. IWE evaluates in memory over the whole
  graph and has no pushdown [V, research 13].

Chosen rule and invariant owner (query core, this ticket):

- Both grammars stay as frontends. Both lower to ONE predicate IR. The IR is
  the filter AST extended with `Path`, `Tag` and `Class` source atoms.
- The IR has `PathAtom::{Literal, Folder, Glob}`. Today
  `SourceAtom::Path(GlobPattern)` is a regex glob over `display_portable`
  (`src/query/grammar/source.rs`, ADR-0008). `file.path ==` is byte
  equality. The planner has a separate candidate rule for each form.
- Every atom carries `Option<Span>` plus a clause id. Section 8 gives the
  details.
- Class names and depth are immutable in the IR. The resolved class set lives
  in the plan. Today `visit_atoms_mut` changes `ClassExpansionMode` in place
  (`source.rs`). `with_children` changes it in templates
  (`src/template/engine/query.rs`).
- One IR is not one predicate. The source predicate runs first. Each filter
  stage keeps its plan position.
- A Filter may move before a Sort or before another Filter. A Filter never
  moves before a Limit.
- A Filter may move before a Flatten only if it does not read the flattened
  path.
- TopK by key is allowed only with the same key. It starts with numeric and
  temporal keys only. Three conditions apply. The key order must equal the
  comparator order. Descending ties must keep stage input order. A byte order
  must not stand in for a collated string order.
- The descending tie order needs care [V]. The executor sorts stably with a
  reversed compare (`src/query/sort.rs`). A reverse index scan would
  reverse the ties.

What withstands scrutiny in the keep-as-is option:

- The two frontends stay. Both use `logos`. They share the boolean-expression
  module `src/query/grammar/expr.rs` [V, z-grilling-guide.md]. Source values
  are paths, globs, tags and classes. Filter values are literals. A merged AST
  needs one node kind per role [H, h1]. One IR gives the benefit without a
  lexer merge.
- These parts do not withstand scrutiny: the `GroupBy` sort alias (section 3),
  CLI string synthesis (section 4), the in-place class mutation, the
  regex-only path atom, and atoms without spans.

Hidden DSL differences that this Answer records [V]:

- Filter tokens parse bare words as `f64` or Duration first.
  `--where 'file.name == inf'` fails to lex. `--from inf.md` works.
- `file.name == "1d"` returns 0 for `1d.md` today. The quoted literal becomes a
  Duration. Ticket 19's raw-text rule fixes this.
- A source `Path` atom is a case-sensitive regex glob. `file.path ==` is byte
  equality.
- Spans exist today only inside `QuerySyntaxError`
  (`src/query/error.rs`).

ADR duties (record rows, change no other ticket):

- ADR-0008: `SourceAtom::Path(GlobPattern)` and "no introspection
  of parsed expressions" change.
- ADR-0010: "quoted and unquoted forms must mean the same thing"
  is false after ticket 19. The pushdown rule relies on the difference. Add
  "superseded in part".
- ADR-0005: the eight tables and the `group_by` chain wording are superseded by
  tickets 24 and 29.
- ADR-0009: superseded by ticket 25.

Serde form of the IR (IWE question [H]): deferred. IWE ships a serialisable
query document with a published JSON Schema [V, research 13]. No host code
exists in Traces. The typed builder is the data form that a Rust host can use.
A serde form is a named later option.

New language: this ticket adopts no new DSL syntax. The open language
questions of ticket 19 stay open. They are `in (...)` and ranges, relative
keywords, existence and type predicates, string functions and a truncating
date function. Ticket 30 owns the relative keywords and the temporal dialect.
`--group-by` is a CLI flag and not DSL syntax.

Code made removable: the in-place class mutation (`source.rs`,
`src/template/engine/query.rs`). The regex-only `SourceAtom::Path`
(`source.rs`).

Unresolved: the `member_of` spelling and the filename-period parser wait on
ticket 30. Ticket 31 owns span preservation at the template callbacks.
Ticket 33 decides who calls the core. Ticket 34 owns the diagnostic envelope.

### 2. Logical plan with a pushdown protocol, or direct interpretation

Alternatives:

- Direct interpretation over the facet contracts. The candidate step reads the
  source atom only. The residual filter runs on all candidates. This is
  option A of the first round [z-grilling-guide.md].
- A logical plan with a pushdown protocol: per-predicate Exact, Inexact and
  Unsupported (DataFusion `Inexact`, SQLite `whereSplit`, research 06).
- Keep as is: the transform-list plan plus the candidate step that ticket 24
  needs.
- A reduced protocol: Exact for equality on a scalar note-level field only
  (AuditSimplicity item 15). Defer ranges, Not, Or and temporal probes.

Chosen rule (invariant owner: the query planner):

- A plan with a protocol exists. Each pushed predicate gets one judgment:
  Exact, Inexact or Unsupported.
- The residual filter ALWAYS runs on every candidate. A candidate set must
  contain every matching row.
- The Answer states the FULL protocol (R1-R7 below). The first implementation
  ships a subset. Later cases then stay additive.

Protocol definitions (confirmed consistency correction):

Exact means that the candidate set equals the matching set for the predicate,
row mode and pinned frame.
Inexact means that the candidate set contains every matching row and can contain
extra rows.
Unsupported means that the planner cannot produce a proved candidate set.
A lossy encoding is Inexact unless a predicate-specific proof establishes
equality of the sets.
Closed bounds prevent omissions on monotone encodings. Closed bounds alone do
not establish Exact.
R6 may complement only a set that satisfies the Exact definition.

Protocol rules:

- R1. A range key is Exact only with a matching-set equality proof and all of
  these conditions:
  - The literal is an unquoted token of bracket B. B is bare Int or Float, bare
    Date or DateTime, bare Duration, or bare bool.
  - The operator is `==`, `<`, `<=`, `>` or `>=`.
  - The left side is a plain field path. It has no arithmetic, call or
    field-versus-field form.
  - The field is a note-level scalar on a Pages row.
  - The key writer reads values through the effective accessor of ticket 23.
    Frontmatter wins over inline, including frontmatter `null`. Several inline
    values form a list, not several scalar keys.
  - The key covers all of bracket B, including period-valued rows.
- R2. Under ticket 19, `==` is false across brackets. Relational operators do
  not match across brackets. No other kind can match.
- R3. Any quoted literal is Unsupported and scans. A later optimization may
  probe a union of brackets. The counterexamples are in the list below.
- R4. `!=`, `null`, missing, and a `Not` of anything not provably complete are
  never keys. In the probe, `rating != 5` returns 10 of 12 rows and `rating ==
  null` returns 6 [V].
- R5. Closed bounds on a lossy monotone encoding prove a superset, not Exact.
  Temporal probes need operator-specific bounds for Date, civil DateTime,
  instant and CalendarPeriod values under the pinned frame. Point-period
  matching uses membership bounds. Period-period comparison uses normalized
  `(start, unit)` identity and order. An unproved mapping is Unsupported.
- R6. `Not(Exact S)` is `Exact(U \ S)`. U is ALL `files` rows, attachments
  included. In Pages mode `img.png` is a row (`src/query/service.rs`).
  `Not(Inexact)` and `Not(Unsupported)` are Unsupported.
- R7. `And` intersects supported children. It is Exact only if every child is
  Exact. A supported child with an Unsupported child makes it Inexact. With no
  supported child, scan the full input.
- `Or` unions supported children. It is Exact only if all children are Exact.
  Any Unsupported branch makes the `Or` Unsupported.
- Empty And and Or use the universe and empty-set identities respectively.

Reading hazard for R6 [V]: "Not over Exact is the same S flagged Inexact" is
unsound. `not (rating > 5)` matches rows outside S.

Counterexamples for R3 and R1 (ten rows in
[h3-redteam-round2.md](../research/29-prep/h3-redteam-round2.md), section 1):

- `code == '012'` matches the String `"012"` by raw text. The feature file
  `tests/features/note/note-frontmatter.feature` pins this [V].
- `rating == "5"` matches String `"5"` rows.
- `due == "2026-07-29"` matches an inline String.
- A day literal matches period-valued rows and civil DateTimes.
- `project == "alpha"` matches the Link `[[Alpha]]` by folded key.
- `spent == 90m` matches `1h 30m`.

Ticket 19 is not implemented. The counterexamples that use adaptive literals
or periods come from the spec text plus today's behavior. They did not run
[H].

First implementation scope (override 1):

- R1 applies only to proved Exact numeric, temporal and bool probes. An unproved
  or merely Inexact encoding scans in this first scope.
- The scope includes And-intersection and source-plus-filter.
- R6 `Not`, `Or`, text range tables and quoted literals wait. These cases scan.
  The residual filter still runs; it cannot recover an omitted candidate.
- The 100 ms gate on a vault of 20,000 notes or fewer is necessary, not
  sufficient. Add a case only after repeatable whole-process benefit is
  measured.
- Compare the same query, frame and generation with the case enabled and
  disabled. Include diagnostic cost, candidate reduction, write cost and cache
  size.

Rule for Task and List rows (`file.*` has no item override):

- Item metadata overrides note values on Task and List rows
  (`src/query/results.rs`).
- `file.*` fields have no item override (`src/query/results.rs`). Range
  and
  equality keys on `file.folder`, `file.path` and `file.name` MAY prune Task
  and List rows.
- Metadata fields and `tags` cannot prune Task and List rows.
  [V] Item tags resolve separately (`src/query/results.rs`).
  Other fields prune Pages rows only.
- Filename-derived `file.date` keys stay Unsupported until ticket 30 defines
  the filename-period parser. `file.day` is retired.

Per-field bracket metadata and notices:

- Store global bracket totals in `meta` in the same transaction as derived
  facts. Delete old derived facts before writing replacements and totals.
- Totals describe the generation, not an arbitrary query's input.
- Each atom's exact mismatch total uses its logical filter-stage input. The
  notice describes incompatible brackets there, not an optimizer's visited rows.
- Compute notices independently of short-circuit evaluation and predicate
  movement. Use an exact diagnostic pass when metadata cannot prove the scoped
  result.
- Derived generations use effective buffer values for their diagnostic metadata.
  Disk-only totals cannot establish their notices.
- Compare scan and pushed notices for source restrictions, And, Limit, row modes
  and buffer edits. Include this work in the optimization benefit check.

Source selection stays "the indexable subset of the predicates" (ticket 24):

- `All`, `Not` and a source-less query enumerate `files` paths. They narrow with
  safe predicates.
- A folder `Path` literal uses the path range of ticket 32.
- A derived query scans its effective generation unless its candidate index
  includes every replacement and addition. A persisted-only index cannot prune
  it.
- Candidate intersections use one common path/row-identity order or membership
  sets. Value-index traversal order is not path order.
- `Tag` uses tag keys.
- `Class` uses explicit class keys, or the bound tag and path atoms.

Equivalence rule for moving a predicate (invariant owner: the planner):

- Pages rows: the key and the residual filter read the same effective note
  value. The candidate set is a superset.
- Task and List rows: only the `file.*` rule above lets a key prune.
- A Filter before a Sort or another Filter keeps the same row set [H]. A stable
  sort of a subset keeps the relative order of the rows.
- A Filter before a Limit changes the row set. It stays forbidden.
- A Filter before a Flatten is allowed only if it does not read the flattened
  path.

Evidence (full tables in
[g2-pushdown-ux.md](../research/29-prep/g2-pushdown-ux.md) and
[g1-probes.md](../research/29-prep/g1-probes.md)):

- Whole-process wall time of a scan: 28 / 48 / 204 ms at 1k / 5k / 50k notes
  [V]. With perfect pushdown (a 0-row range query) the floor is 26 / 39 / 140
  ms. Separate floor runs measured scans at 29 / 48 / 205 ms. Those runs give
  savings of 3 / 9 / 65 ms (31 ms at 20k).
- A scan crosses 100 ms near 19k notes. The floor crosses 100 ms near 33k.
- The floor is mostly the freshness walk: about 110 ms of 140 ms at 50k. Plain
  `find` over the same files takes 125 ms and `fd` takes 60 ms.
- Every scan costs the same from 0.1% to 80% selectivity. Row decode, assemble
  and drop make up 87-90% of the avoidable cost. Filter evaluation is about 10
  ms.
- Templates load the whole generation (about 50 ms at 5k, 255 ms at 50k for the
  first query, then about 3 ms per extra query). Store pushdown does not apply
  there. A host pays no walk per query.
- Real shapes: of 45 distinct queries in the author's vault, 21 are AND-only and
  21 have a range. A plain field-versus-literal `or` is 2 of 47 branches. `!=`
  and Not return 75-80% of rows. Quoted text equality is the most common
  equality. The decided rule excludes it at first.
- Store costs (50k): a numeric range `rating >= 4.5` returns 5,000 rows in 171
  us. A folder range returns 7,143 rows in 261.5 us. The first match decodes
  4.5. Path order inside one value holds. Clearing a facet takes 3.7 ms. Deletes
  do not shrink the file. The total is 33.7 MB for either build pattern. A
  numeric range table adds about 0 MB at 50k. A text facet table adds 16.8 MB.
- Machine: macOS arm64, shipped `opt-level = "z"` binary, synthetic vaults,
  medians of 11, busy machine.

Attacks that no design survives [V, h2-redteam.md]:

- Arbitrary user grammar extensions: any fixed IR admits a query that the
  planner cannot prove. The facet scan is the ceiling, not a gap.
- Cross-facet joins (link targets as filterable rows) are out of scope by ticket
  24's settled shape. Nobody re-opens them here.

Code made removable: the full-store `SourceResolver` read path and the `Not`
full scan (`src/query/service.rs`), after ticket 24 removes them. The four
multimap tables (ticket 24). This ticket owns the protocol that replaces them.

Unresolved: range-key encode and read costs on a real vault. The range-key
encoding of task keys. Task-row group equivalence (ticket 36).

### 3. `GroupBy`: real grouping, removal, or the sort alias

Alternatives:

- Keep the sort alias and document it as a sort. The first round called this
  accepted debt [P, h1]. The alias hides the fact that no grouping happens. This
  is the retain option.
- Remove the `group_by` method. Offer a sort plus a marker.
- Nest groups as plan rows with a second row shape (Dataview `{key, rows}`).
  Later stages would then see group rows. Every consumer would need a second row
  mode.
- Build a distinct grouped result after the plan runs. This is the chosen
  option.
- For the key: one key, a repeatable flat composite key, or nested keys.

Chosen rule (invariant owner: the query core):

- A DISTINCT `GroupedQuerySet` exists. It is built after filter, sort and limit.
  The order of the Tasks plugin is the model: filter, sort, limit, group, `limit
  groups`.
- It is an ordered list of `{key, rows}`. Each `rows` is a normal `QuerySet`.
- There is NO plan stage after grouping.
- The Rust type has no `sort`, `limit`, `where` or `count` method.
- The template (minijinja) object returns a TYPED error for those names. The
  error names the replacement. Templates dispatch dynamically
  (`src/template/engine/query.rs`), so the type cannot hide the names.
- `take_groups(n)` exists only when a template needs it.
- ONE key now. A repeatable flat composite key is a named later extension.
  Nested group levels are rejected. They make `rows` a list of groups, not a
  `QuerySet`.
- The plan variant `QueryTransform::GroupBy` is deleted (`src/query/plan.rs`).
  It is a sort alias today.
- `QuerySet::group_by` is replaced (`src/query/results.rs`).
- The template method `group_by` stays. It now returns groups.

Key equality uses ticket 19's exact numeric equality. Period membership never
defines a key:

- Numbers use the exact number line. Int against Float is exact. No compare uses
  `i64 as f64`. `-0.0` is normalized at key build. NaN is already Null.
- Temporal keys use three classes. Civil keys are `(date, time)`, with Date as
  midnight. Instant keys are epoch nanoseconds. Period keys are the normalized
  `(start, unit)`. The classes stay apart and order Civil, Instant, Period.
- A mixed civil and instant comparator is not transitive across a DST gap [V].
  The probe used `America/New_York 2026-03-08`. The cycle is civil 02:30 < civil
  03:00 < instant 07:15Z < civil 02:30. `sort_by` panicked in 15 of 200 runs.
- A Date and a midnight civil DateTime are `==`-equal under ticket 19 . They
  group together.
- A CalendarPeriod groups by exact `(start, unit)` identity. Membership never
  defines its equality. The final review lifts the earlier ban on period keys.
- Link keys use `ComparisonKey`, then target plus subpath. Ticket 19 sorts Links
  with the collator on the target only. Equality uses the folded target plus the
  subpath. `[[A#x]]` and `[[A#y]]` would merge under a target-only sort.
- Strings use raw bytes. Collator Equal plus the byte tiebreak equals byte
  equality, so this is safe.
- ONE grouping code path serves every key type. Hash-group on the canonical key.
  Then sort only the distinct keys: numeric order, temporal class order,
  structural order or the collator.
- Canonicalize every existing value kind. Fan out only the outer queried list. A
  nested list becomes one ordered structural sequence key.
- Object keys use byte-sorted property names and recursively canonical values.
  Insertion order never matters. Compare sorted name lists, then their values.
- Reuse scalar normalization at every leaf. Raw structural `PartialEq` alone
  does not establish exact Int/Float group equivalence.
- Tags project String elements. Inlinks project Link elements.
- Structural labels render canonical property and element order
  deterministically. Templates keep the complete typed key.

Measured at 50k rows (RedTeam29, replica comparators, real `jiff` and
`icu_collator`) [V]:

- Sort plus split: 5-7 ms for numbers and bytes. 50-210 ms with ICU.
- Hash-group for numbers and byte keys: 2-8 ms. With ICU, it takes 132 ms at
  50,000 text groups.
- A naive Int-Float comparator through `as f64` breaks transitivity. It gives
  563 violating triples per 2M values. It gives 315 groups against 29 true
  classes.
- Today's f64-only key merges ints above 2^53. `big: ...993` renders as
  `...992`.

Required invariant tests (owner: the query core test suite):

- Group equality is equality of canonical `GroupKey` values. It is reflexive,
  symmetric and transitive.
- Scalar filter equality agrees within each grouping domain.
- Date and civil-midnight DateTime values share a key when their civil values
  agree.
- Civil values and instants remain separate group classes, even when a filter
  relates them through the pinned zone.
- Membership matching never defines group equality. Test the documented
  exceptions separately from scalar equality.
- Transitivity holds, and `sort_by` never panics, over a mixed pool. The pool
  holds these values:
  - Int and Float near 2^53 and 2^63, signed zero and infinity.
  - Date, civil DateTime across a DST gap and fold, instants and periods.
  - Duration `1mo` against `30d`, and Link case and subpath.
  - Strings in NFC and NFD, `a10` against `a010`, Bool and Null.
  - Nested lists, and objects with the same properties in another order.
- Equivalence classes are contiguous after sorting.
- No compare uses `i64 as f64`.
- `-0.0` is normalized at key build.

Fan-out and counts:

- Fan out the outer list only. Deduplicate its canonical keys before adding a
  row. Each row belongs at most once to each group.
- Preserve pre-group row order inside each group.
- Missing, Null, NaN normalized to Null and an empty outer list use the null
  group. Empty nested lists and empty objects remain distinct structural keys.
- Today `SortKey::from_value_ref` turns lists, objects, Tags and Inlinks into
  Null (`src/query/sort.rs`). A new group-key projection is needed.
- R is the set of rows after filter, sort and limit. D is the number of rows in
  R.
- Membership total M is the sum of distinct canonical memberships per row. An
  empty outer list contributes one null membership. Group sizes sum to M, not D.
- Template accessors: `len` is the group count. `row_count` is the distinct row
  count. `entry_count` is the membership count.
- Rows carry a PRE-GROUP ORDINAL and a flatten ordinal. Distinct counts and
  dedup use the ordinal.
- Do not use the `QueryRow::eq` method for dedup. It deep-compares the whole
  `FileEntry` (`results.rs`).
- `(note position, item_idx)` alone collides after `flatten` (`results.rs`,
  `plan.rs`).

Group order and field rules:

- Group order uses ticket 19's rank. Null stays first in both directions.
  Reverse only non-null key order for descending groups.
- Bare `--group-by FIELD` is ascending, so headings read 2024, 2025.
- A `-` prefix is descending. This is the `+` and `-` grammar of
  `SortOrder::parse`.
- The asymmetry is deliberate. A bare `--sort FIELD` is DESCENDING today
  (`src/cli/mod.rs`, `src/query/GLOSSARY.md`). `--asc` and `--desc` need
  `--sort`. The help text says so.
- `--sort` orders the rows inside each group.
- `--group-by` takes exactly one term. A comma is an error.
- An absent note field groups under the null group.
- An unknown or malformed `file.*` field is an error with a `help:` line. Today
  `--sort nosuch` exits 0 and does nothing.

Heading label and form:

- The label is the canonical spelling of the key class. For strings and Links it
  is the byte-minimal raw spelling among the class members. For numbers it is
  the normalized decimal (`5`, not `5.0`). Dates use ISO. Durations use the
  canonical form.
- The label is NEVER "the first row's key". That key changes when a file is
  renamed.
- Null prints `(none)`. Normalize the display label first.
- For a non-null label equal to `(none)` or starting with `\`, add one leading
  backslash. Apply output escaping exactly once after this marker.
- The heading level is fixed at `##`. The help text says it can collide when you
  paste it under a note heading.
- For headings, fold `\n \r U+2028 U+2029 U+0085 \v \f` to spaces and trim
  before adding the marker. Then escape Markdown delimiters.
- Render remaining C0, DEL, C1 and Unicode directional formatting controls as
  visible escapes before output. Preserve ordinary RTL letters and combining
  marks.
- Markdown escaping covers `` \ ` * _ [ ] < > & ~ $ { } ^ ! ``. Escape a
  trailing run of `#` after a space or at the start. Do not escape `- + = |`.
- Cover ESC, backspace, carriage return, bidi formatting controls, Persian,
  Hebrew and leading-backslash sentinel strings in output tests.
- A property test uses pulldown-cmark 0.13 (GFM, math, heading attributes,
  wikilinks). The naive heading fails 64,587 of 400k keys. Fold plus minimal
  escape fails none [V].

CLI surface:

- `--group-by FIELD` exists on `list`, `table` and `task`. A shared flattened
  clap args struct holds it, beside `SortArgs` (`src/cli/mod.rs`).
- The output is one markdown heading per group. Then the existing renderer draws
  that group. `task --table` renders one table per group with its own header.
- `--count` with `--group-by` prints `n<TAB>key` per group. Encode TAB, carriage
  return, newline, remaining line separators and backslash. Apply the same
  non-null marker after label normalization.
- WITHOUT `--group-by`, `--count` stays a bare number. That shape is frozen.
- `--count` is NEW on `list` and `table`. Today only `task` has it, and `list
  --count` exits 2. The help text says the output form changes with
  `--group-by`.
- `--count` wins over `--table`. It keeps the stderr summary off, as flat `task
  --count` does.
- The stderr summary keeps the house style: `D page(s) in G group(s) from ROOT`.
  Tasks print `D task(s)`. The existing form is `N page(s) from ROOT`
  (`src/cli/list.rs`). An `(M listings)` clause appears only when M is not D.
- An empty grouped result exits 0 and prints nothing.
- There is no `--format` flag yet. Printed labels are presentation, not typed
  serialization. Number `5` and String `5` can share a label. Templates retain
  lossless typed keys. Do not promise machine round-tripping.

Template migration:

- Old `group_by(x)` equals `sort(x, false)`. Templates default `sort` to
  DESCENDING (`src/template/engine/query.rs`).
- A template that reads a group as a row fails loudly for `{{ n.file.name }}`.
  The value is undefined.
- The change is silent for `{{ n.file }}` (prints empty), `{% if n.file %}`
  (false) and `{{ g | length }}` (the group count). The change row says so.

Uses to update:

- `src/query/plan.rs`.
- `src/query/results.rs` and the tests at `results.rs`.
- `src/template/engine/query.rs`. The test
  `group_by_clusters_equal_values_ascending` expects `"b a "`.
- `src/query/GLOSSARY.md` and `docs/adr/0005-template-query.md`.
- `tests/baselines/pub-tokens.txt`.
- No `.feature` file, e2e test, example, bench or `docs/vault` template uses
  `group_by`. This search cannot cover user templates outside the repo [H].

Precedent (the full comparison is in
[h3-redteam-round2.md](../research/29-prep/h3-redteam-round2.md), section 12):

- Dataview `GROUP BY` yields `{key, rows}` and sorts keys ascending with nulls
  first [V].
- Tasks groups last, with headings: filter, sort, limit, group, `limit groups`
  [V].
- jq `group_by` returns arrays sorted by key. pandas returns a `GroupBy` object.
  SQL collapses the rows [V].
- Nushell `--to-table` keeps key types, so `1` and `1.0` are separate groups. It
  is new in 0.117.0. Traces differs: ticket 19 uses exact numeric equality [V].

Lazy errors through minijinja: `Object::get_value` returns `Option`, and
`enumerate` returns an `Enumerator` (`src/template/engine/query.rs`).
Neither can fail. Ticket 21's memoized typed error has no channel for
`GroupedQuerySet`. Ticket 31 owns the handoff.

Code made removable: `QueryTransform::GroupBy`, the sort alias in
`QueryTransform::apply`, and `QuerySet::group_by`. The tests that assert the
sort-alias behavior change.

Unresolved: task-row group equivalence and measurements (ticket 36).
A repeatable composite key, `take_groups(n)` and `--format` remain outside
the current surface. All existing value kinds now have group-key rules.

### 4. Typed `QueryBuilder` filters, or string synthesis

Alternatives:

- Keep string synthesis. It is cheapest, and the parser gives spans for free.
  This is the retain option.
- Make the typed builder the core surface. The DSL becomes one frontend.
- Keep both: the builder in the core, and string synthesis as a thin CLI helper
  (option C of the first round).

Chosen rule (owner: the query core builder):

- The typed builder is the core surface. The DSL is one frontend.
- CLI synthesis moves to the builder. This fixes `traces task --status é` and
  `traces task --status ✓`, which return 0 tasks today [V].
  `char::escape_default` makes `\u{e9}`, and the DSL does not decode it
  (`src/cli/task.rs`). Change row 13.
- Ticket 33 decides who calls the core. Tell ticket 33 that ticket 29 settled
  synthesis. A second decision there would duplicate this one.
- The builder must validate the target shapes that the strings check today.
  Errors must not lose spans [H].
- Serde form of the IR is deferred (section 1).

Code made removable: the string formatting in `TaskFilterArgs::status_filter`
(`src/cli/task.rs`). `refresh_query` (`src/cli/mod.rs`)
stays unchanged for `--where` strings.

Unresolved: who calls the builder (ticket 33). The template bridge (ticket 31).

### 5. The `QuerySet` laziness contract for templates

Facts [V, research 01 claim 7]: a `QuerySet` is an `Arc` base plus an
`ExecutionPlan`. Rows clone only at terminal materialization.

Alternatives:

- The overlay swaps the row list before memoization. Rows are immutable.
  Ticket 28 owns the swap rule (option A of the first round).
- The overlay lives inside the plan. The first round rejected this. It drags
  buffer semantics into the plan.
- Keep the memo per `QuerySet` instance. This is the retain option. It has a
  defect: `#[derive(Clone)]` copies `OnceLock` state, and `push` resets it. A
  sibling chain then recomputes the plan.
- Key the memo on the base id only (AuditSimplicity item 21). The maintainer
  did not adopt this.
- The seam is the pinned generation (chosen).

Chosen rule (invariant owner: the query core). Ticket 28 owns the derived
generation:

- `QuerySet` and `GroupedQuerySet` read one pinned generation. It is persisted
  or derived. They know NOTHING about buffers.
- The seam is "any pinned generation". Ticket 28 owns the derived generation.
- Each immutable query family owns a shared `Arc` memo. Clones share it. A
  changed plan, frame or generation gets a distinct memo identity.
- Keep no process-global memo registry. The memo retains the generation and the
  clause table for cached failures. Equal predicates with different origins do
  not share diagnostic errors.
- The grouped memo also includes its field and direction.
- Each result names `(incarnation, revision, derived_id)` within its host
  lifetime. Cross-process consumers need a host-session identity as well.
- Relative keywords (`today`, `eom`, ticket 30) bind to the pinned frame at plan
  time. A standalone query acquires a new frame. A render or request supplies
  its shared frame (ticket 21).
- Every terminal returns or propagates a memoized typed error. The terminals are
  iteration, indexing, formatting and grouping. No terminal returns an empty
  result for a failure (tickets 21 and 31).
- A corrupt required facet is a complete cache miss. The query falls back to a
  fresh parse, exits 0 and prints one notice (ticket 24).

Code made removable: the copy of `OnceLock` memo state in
`#[derive(Clone)]` and the memo reset in `push`.

Unresolved: the fallible-terminal channel for minijinja (ticket 31). The
worker that builds the derived generation (ticket 33). The heavy-note entry
clone and the single-thread rebuild cost (ticket 28).

### 6. The row representation

Alternatives:

- One row struct with the existing row-kind tag. This is the retain option.
- Three row types behind one enum.
- A synthetic global order across the row modes. No consumer needs it.
- A row that holds `Arc<WorkspaceIndex>` plus a `RowIndex` (today:
  `src/query/results.rs`), against a row that owns its
  `Arc<FileEntry>`.

Chosen rule (invariant owner: the query core):

- [P] One row struct with the existing row-kind tag. See the row terms above.
  [V] The current tags are `RowType::Page` and `RowType::List { item_idx }`
  (`src/query/results.rs`).
- There is no synthetic global order.
- Document order is `(note position, item_idx)`, compared in that order. The
  flatten ordinal follows. `item_idx` is the index into `Note::lists` (ticket
  24). Task and list rows both carry it.
- This is the item order contract that ticket 23 asked for. It fixes
  `cmp_document_order` (`results.rs`), which ignores `item_idx`.
- A row owns its matching `Arc<FileEntry>` (ticket 24). Today this is not built.
  `WorkspaceIndex.entries` is `Box<[FileEntry]>` (`src/index/entry.rs`).
  `QueryRow` holds `Arc<WorkspaceIndex>` plus a `RowIndex`.
- A row carries a pre-group ordinal and a flatten ordinal (section 3). Distinct
  counts and dedup use the ordinal.
- Task display depth uses the active preceding ancestor stack in the emitted
  list or group. An ancestor appearing later is not a display parent.
- Emit an orphan at depth zero. Do not reorder rows after an explicit sort. Keep
  source `list.depth` unchanged. Apply this rule to flat and grouped output.
- Today's renderer uses `row.depth()` (`src/query/format.rs`).
- Evidence [V, binary]: `traces task --todo` with a done parent prints the
  orphaned child at depth 1. Sorting does the same. Change row 12 records the
  fix. It touches frozen stdout and is an intentional defect fix. The maintainer
  acknowledged it.

Code made removable: the whole-index borrow in `QueryRow` (`results.rs`). The
use of `QueryRow::eq` for row identity (`results.rs`).

Unresolved: the exact Rust shape of the row API (ticket 35). The cold CLI builds
`FileEntry` rows for survivors only (ticket 24). Task-row group equivalence
(ticket 36).

### 7. Ownership of `format.rs` output rendering

Alternatives:

- Keep `format.rs` inside the query module. This is the retain option.
- Move rendering to an output module. Ticket 35 picks the path.
- Keep two Markdown paths, as Nushell does (`to md` and `table --theme
  markdown`).
- Shared column widths for grouped tables, against per-group widths.

Chosen rule (invariant owner: the output module). Ticket 35 owns the module
path:

- The render code moves to an output module. `src/query/GLOSSARY.md` and
  `GLOSSARY-MAP.md` change their module descriptions.
- The pipeline has three steps. Typed rows or groups (an ITERATOR) go to a plain
  cell grid with no domain types. ONE markdown renderer draws the grid.
- The query core returns typed rows and groups and no text.
- Widths are PER-GROUP. The renderer streams groups, so its grid holds one group
  at a time. This bounds incremental render-grid memory, not memory for hash
  grouping or retained rows.
- Flat tables still need global widths, which take two passes. Only grouped
  output streams.
- Keep the existing cell escaping and its property tests (`format.rs`).
- The zero-column table form `"||\n"` (`format.rs`) needs a grouped equivalent.
  An empty grouped result prints nothing and exits 0.
- There is no terminal-width planning. The output is markdown, not a terminal
  table.
- Template methods and stdout payload shapes stay reachable (ticket 16).

Measured (100 groups x 500 rows x 4 columns, medians of 11) [V]:

- Per-group widths: 21.54 ms, 8.59 MiB peak.
- Shared widths: 21.00 ms, 17.10 MiB.
- Shared widths, two passes: 27.23 ms, 8.50 MiB.
- One 184-character outlier in one group: shared output grows 2.7x (12,208 KiB
  against 4,503 KiB). All rows pad to the outlier.

Nushell lessons (details in
[h3-redteam-round2.md](../research/29-prep/h3-redteam-round2.md), section 13)
[H]:

- Copy these:
  - A cell grid between row-to-text and drawing.
  - Separate symbols for missing and null.
  - Escape `|` and newlines in Markdown cells.
  - Validate group-by names before execution.
  - Typed pushdown results with a conservative `None` fallback
    (`project_output_columns` returns `Option`).
- Do not copy these items:
  - Loose epsilon `==` next to a `total_cmp` sort.
  - String-keyed group records.
  - Null groups dropped in record mode.
  - A hard abort on a key closure error.
  - "Couldn't fit" printed as success stdout.
  - Spans on every value. Nushell is removing them.
  - Two Markdown paths.
  - `to md --pretty` padding by bytes.

Code made removable: the render code inside the query module, once the output
module exists. The flat `render` entry point that takes `&[QueryRow]`
(`format.rs`) changes to take an iterator.

Unresolved: the module path and the Rust API of `GroupedQuerySet` (ticket 35).
The EPIPE writer contract shares a location with the output module. Tickets 33
and 35 name it once.

### 8. Diagnostic spans through the template boundary

Alternatives:

- Spans only inside `QuerySyntaxError` (today). This is the retain option.
- A span on every atom, with a clause id (chosen).
- A span on every value, as Nushell does. Nushell is removing them.

Chosen rule (invariant owner: the IR builder):

- Every atom carries `Option<Span>` plus a clause id.
- The builder owns a clause table `(id, Arc<str>)`. The span is a byte range
  into that clause.
- Typed builder calls carry `None`.
- A repeatable `--where` fuses into one And, so a clause id is needed.
- Spans exist today only inside `QuerySyntaxError` (`src/query/error.rs`). The
  evaluator has no spans, and `FieldPathError` carries none.
- A notice after the parse needs an atom span and a clause id. Ticket 19 and
  ticket 34 notices need them.
- `QueryError` keeps its spans intact. Tickets 31 and 34 decide the rest.

Notices:

- A bracket mismatch, an invalid date-shaped quoted literal and a NaN-to-Null
  conversion ride one mechanism on stderr. Ticket 34 owns the envelope and the
  text.
- Dedup key: one notice per (document or path, kind) until the state changes or
  the buffer closes.
- Error shape: miette with one `help:` line that names the replacement. This is
  the house style. `grep` finds no `warning:` text in `src/`.
- Exact mismatch notices use each atom's logical stage input. Global bracket
  totals are a shortcut only when they prove that scoped result (section 2).

Unresolved: how spans survive the minijinja callbacks (ticket 31). The envelope,
the severity and the display (ticket 34). Whether the plan collects notices on
the result instead of printing them [P, z-grilling-guide.md] (ticket 34).

Code made removable: none. The span carrier is new.

### 9. The `cmp_document_order` invariant for multi-row notes

Alternatives:

- Keep the position-only compare. This is the retain option. It ignores
  `item_idx` (`src/query/results.rs`).
- Compare position, then `item_idx` (chosen).
- A synthetic global order across modes. Rejected, as in section 6.

Chosen rule (invariant owner: the query core):

- Document order is `(note position, item_idx)`, compared in that order.
- The flatten ordinal breaks ties between rows that `flatten` creates from one
  row.
- Each Sort and TopK preserves stage input order for equal keys. Document order
  supplies that input order only before earlier stages reorder rows. [V] The
  current Sort is stable (`src/query/sort.rs`). TopK uses the stage input
  ordinal (`src/query/plan.rs`). A re-sort at the presentation edge is not
  enough.
- The default note order is the path order of ticket 32: the key is the path
  with `/` replaced by `0x00`. Stdout order stays the same as today. No change
  row is needed for order.

Required tests:

- `cmp_document_order` compares two list items in one note by `item_idx`.
- `flatten` of one multi-value field keeps a stable order through the ordinal.
- The unit test `index/service.rs` keeps passing: it expects `["notes/note.md",
  "notes-x.md"]`.

Code made removable: the position-only `cmp_document_order` (`results.rs`).

Unresolved: none for this ticket. Ticket 23 uses this contract.

### 10. Membership operation for temporal point and period

Alternatives:

- A `member_of` predicate in the IR with a frontend now (first round).
- `member_of` as an IR node name only, with no frontend and no implementation
  (chosen).
- Express every membership relation through ordinary key order. This is unsound.
  Period identity order is not point-period membership.

Chosen rule (invariant owner: the query core). Ticket 30 owns the spelling:

- `member_of` is reserved as an IR node name only. It has no implementation and
  no frontend. The spelling waits on ticket 30.
- Its relation is never a range key and never a group key.
- Membership stays separate from equality and from the sort key (tickets 19 and
  21). A period compares against an instant through the local civil date of the
  instant. A skipped next day does not invalidate a real prior day.
- Period and period equality is the exact `(start, unit)` identity (ticket 21).
- The advisory bracket-mismatch notice rides the notice mechanism (section 8).
  The envelope belongs to ticket 34.

Unresolved: the spelling and the frontend (ticket 30). The first-round notice
for rows with a Null date (h2) is not adopted here. Tickets 30 and 34 decide it.

Code made removable: none. The node name is new.

### Change-table rows (ticket 16 categories)

These rows come from the consolidated list of the grilling session. This
ticket owns rows 9-15 and 20. It records row 17 for ticket 19.
Row 16 belongs to ticket 28. Rows 1-8, 18, 19 and 21 belong to ticket 32.

| Row | Change | Category | Note |
| --- | --- | --- | --- |
| 9 | `--group-by`; `--count` on `list` and `table`; `n<TAB>key` lines; `##` headings; `(none)` label | New surface | Bare `--count` stays frozen. |
| 10 | stderr `D page(s) in G group(s) from ROOT` | Free | Only under `--group-by`. |
| 11 | Template `group_by(x)` returns groups, not sorted rows. Old `group_by(x)` equals `sort(x, false)`. | Migratable | The capability stays. Reads of a group as a row fail loudly or change silently (section 3). |
| 12 | Task display depth uses active preceding ancestors in flat and grouped output. Orphans start at zero; explicit sort order stays unchanged. | Frozen payload, intentional defect repair | The maintainer confirms the emitted-stack rule. Source `list.depth` stays unchanged. |
| 13 | `task --status é` returns tasks | Free | Defect fix. |
| 14 | Typed builder. DSL spelling unchanged. | Free | |
| 15 | Bracket-mismatch advisory notice | New surface | stderr only. |
| 17 | The quoted and unquoted divergence of ADR-0010 is fixed by ticket 19 | Migratable | Already a ticket 19 row. |
| 20 | An empty grouped result exits 0 | Frozen | Consistent with flat zero rows. |

### Benchmark and measurement plan

Measured here (single machine, macOS arm64) [V]:

- Range-key sizes and scans (`g1-probes.md`).
- Sort cost: 31.6 ms for `Vec<PathBuf>.sort()`, 4.5 ms for stored NUL keys,
  about 10 ms for a String comparator at opt-level 3.
- Normalization costs (`g1-probes.md`, ticket 32).
- Original overlay prototype: full rebuild 113.8 ms at 50k, plus 59.3 ms gather;
  12.7 ms at 5k. These are not amended resolution-context lifecycle
  measurements. A new exact-base/effective-path context can need another full
  resolve.
- The grouping costs (section 3) and grouped render costs (section 7).
- End-to-end pushdown UX (`g2-pushdown-ux.md`).

Still open (ticket 36 or the implementation):

- Owned against view for `file.name` sort and filter (the ticket 18 gate).
  `NoteFieldValueRef` goes only after this benchmark.
- Folder glob against the linear `Path` scan (`src/query/service.rs`).
- Range-key encode and read costs on a real vault.
- Full row materialization on a real vault.
- Same-stem candidates at 5,000. RELABEL the "trie 224 ms against flat 412-436
  ms" figure of ticket 24. The trie build alone takes 2.0-2.3 ms. The 224 ms
  figure is probably the whole `InlinkMap::new`. Keep the trie decision.
- Single-thread full rebuild cost and heavy-note entry clone (ticket 28).
- Windows and Linux file locking.
- Task-row group equivalence (ticket 36 list).
- The NFC dependency choice (ticket 32).
- A recheck of the sort and group numbers under the shipped `opt-level = "z"`.

Probe code stays throwaway (`/tmp/traces-probe`, `/tmp/rt29-probe-src`). The
`.worktrees/rt29` worktree is removed. Ticket 35 decides a durable bench home.

### Owners of the new invariants

| Invariant | Owner |
| --- | --- |
| A candidate set holds every matching row, and the residual filter always runs | The query planner (this ticket) |
| The key writer reads values through the effective accessor | The index key writer (tickets 23 and 24) |
| Per-field bracket counts change in the same write transaction | The store (ticket 24) |
| The folder-range end bound has a pinned increment rule with a test | The store and ticket 32 |
| Group equality agrees with filter `==` within each grouping domain | The query core test suite |
| `heading_escape` keeps every heading well formed | The output module |
| Distinct counts and dedup use the pre-group ordinal | The query core |
| The memo key is `(plan, frame, generation triple)` | The query core |
| The derived generation is always built from the current base | Ticket 28 |
| Every atom carries `Option<Span>` and a clause id | The IR builder |
| One notice per (document or path, kind) | Ticket 34 |

### Adversarial review and required corrections

Two red-team rounds and three audits tested these decisions. The evidence is in
[h2-redteam.md](../research/29-prep/h2-redteam.md) and
[h3-redteam-round2.md](../research/29-prep/h3-redteam-round2.md). They changed
the decisions in these ways:

- Pushdown scope narrowed. The first round proposed a candidate step from the
  source atom only. The final set states the full protocol. The first
  implementation pushes only unquoted range keys, And-intersection and
  source-plus-filter.
- The range-key rule narrowed (the Q5 correction). Quoted literals, `!=`, `null`
  and missing never use a range key. `Or` scans in the first implementation. The
  full protocol can support `Or` later (R7). The counterexample table has ten
  rows.
- R6 replaced "Not over Exact is Inexact". `Not(Exact S)` is `Exact(U \ S)`.
- Task and List rows may prune on `file.*` keys. The first round banned all note
  keys for Task rows.
- Bracket-mismatch counts moved into the `meta` facet. A residual-only notice
  would vanish on a pruned run.
- `GroupBy` is real grouping. The first round kept the alias as accepted debt.
- Sort-then-split became hash-group (the Q26 correction). One code path serves
  every key type.
- Temporal group keys use three classes: civil, instant and period. The mixed
  civil and instant comparator has a cycle and panics.
- Link group keys use `ComparisonKey`, then target plus subpath.
- Row identity gained a pre-group ordinal and a flatten ordinal (the Q7
  correction).
- Task depth applies to flat and grouped output (the Q31 correction).
- Plain byte order became the NUL-form key (the Q10 correction, ticket 32). No
  change row for order is needed.
- Group order uses the `+` and `-` prefix grammar (the Q6b correction).
- The group heading shows a canonical spelling, not the first row's key.
- The heading uses `heading_escape`. The naive heading failed 64,587 of 400k
  keys.
- `--count` lines use `n<TAB>key`, not `key: n`. The summary keeps `from ROOT`.
- Shared widths became per-group widths (the Q27 correction).
- `member_of` shrank to an IR node name (AuditSimplicity item 20).
- The overlay seam moved from "swap rows before memoization" to "any pinned
  generation".
- The memo moved behind a shared `Arc`.
- The builder is the core surface, and CLI synthesis moves to it (first-round
  option C rejected).
- `format.rs` moved from "record a preference" to a decision (output module).
- The maintainer rejected three audit proposals: Exact equality only for the
  first pushdown, compile-time absence of grouped-set methods, and a
  base-id-only memo key. Templates dispatch dynamically, so the template adapter
  still needs typed errors.

### Evidence limits

- Ticket 19 semantics are not implemented. The range-key counterexamples that
  rely on adaptive literals or periods come from the spec text plus today's
  behavior. They did not run [H].
- The comparator probes use replica comparators. They use real `jiff` and real
  `icu_collator` with the numeric-ordering and tertiary-strength settings of
  ticket 19. They do not use the real `SortKey`.
- The minijinja probe used a map-shaped group, not the real `GroupedQuerySet`.
- Windows `Path::cmp` order is not checked.
- The UX data is synthetic and comes from a busy machine.
- `docs/vault` queries are Dataview and Tasks queries. The mapping to Traces
  syntax is by inference [H].
- The sort numbers come from one auditor at opt-level 3 and z. The red team
  measured 31.6 ms and 4.5 ms.
- Nushell behavior comes from source reading. No `nu` binary ran.
- Range-key encode and read costs, task keys and full row materialization on a
  real vault stay unmeasured. They carry no [V] mark.

### Resolution and handoffs

The grammar and plan shape: two frontends lower to ONE predicate IR. A plan with
a pushdown protocol runs on one pinned generation. The residual filter always
runs. The first pushdown ships unquoted range keys only.

The alternatives that the Answer weighs:

- Keep two grammars and the transform-list plan (retain).
- Unify the grammars with term splitting.
- Use a typed builder as the single IR.
- Keep as is within the remaining scope. The IWE cost model is the only
  comparison. It is not a head-to-head result.

What withstands scrutiny: the two frontends, the shared boolean-expression
module, filter-move rules that never cross a Limit, the residual filter, and
one mode-tagged row struct. What does not: the `GroupBy` sort alias, CLI string
synthesis, in-place class mutation, spanless atoms, the whole-index row, and
position-only document order.

Handoffs, by ticket. Other tickets keep their own decisions. These items record
the grilling-time duties.
Their presence does not assert that each duty remains undone.

- Ticket 24:
  - Supersession rows for the platform family (rule 3 and the store identity
    text). Record the release order check by NUL-form key (from ticket 32).
  - Relabel the "224 ms trie" figure (section on the measurement plan).
  - Store per-field bracket counts in the `meta` facet, in the same write
    transaction as the facet change.
  - `FileEntry` shares `Note` and `inlinks` by `Arc`. The `LinkResolver` is
    owned.
  - The spine clone cost.
- Ticket 25: the case-variant `.traces` scan test, the scan-epoch proof, the
  rule that a row without `ctime` read on Unix is stale, and the wording for a
  newer epoch. These come from tickets 28 and 32.
- Tickets 26 and 27: the path form equals the store form. Record the platform
  family supersession.
- Ticket 28:
  - The seam is "any pinned generation". `QuerySet` knows nothing about buffers.
  - The memo key is `(plan, frame, generation triple)`. The triple holds the
    base pair and the `derived_id`.
  - A new overlay state is a new derived generation. It gives rows that own
    `Arc<FileEntry>`.
  - A path-set change does ONE full re-resolve: 113.8 ms at 50k (about 173 ms
    with the note gather) and 12.7 ms at 5k. It runs off the request thread. The
    change is latest-wins. Queries keep the previous derived generation until
    the new one publishes.
  - The existing refresh uses `patch_and_diff` only when `is_paths_unchanged`
    (`src/index/refresh.rs`).
- Ticket 30:
  - Relative keywords (`today`, `eom`) and key bounds bind under the pinned
    frame at plan time.
  - The `member_of` spelling.
  - The filename-period parser. Filename-derived `file.date` keys stay
    Unsupported until then. `file.day` is retired.
  - `date.format`.
- Ticket 31:
  - A fallible-terminal channel for minijinja (`src/template/engine/query.rs`).
  - The group value form, and a template adapter that returns `{key, rows}`.
    Ticket 16 freezes the capability.
  - Span preservation at the callbacks.
  - Blocked-by repair: 31 waits on 7, 16, 18, 28 and 29.
- Ticket 32:
  - The sort and range key is the path with `/` replaced by `0x00`. The folder
    range is `[prefix\0, prefix\x01)`.
  - A path-literal or folder source atom that misses the exact listed bytes uses
    the comparison-key map. A unique match rewrites the atom to the listed
    spelling at planning. Several matches return the UNION of candidates with no
    diagnostic. Globs are never rewritten. String comparisons such as
    `file.folder == "x"` stay exact.
  - Glob atoms keep pattern text. Literal and folder atoms keep typed input
    until planning resolves them to listed `RelativePath` values.
- Ticket 33:
  - Worker wiring: the latest-wins rule and the single owner.
  - The LSP re-push rule and the number of the staleness bound.
  - The symlinked `.traces` case (F11) and the ban on a host-global resolver
    cache.
  - Ticket 29 settled synthesis: the typed builder replaces CLI string
    synthesis. Ticket 33 decides who calls the core.
  - `normalize_source_input` moves into core and resolves through the fold map.
    Ticket 33 decides who calls it.
  - The EPIPE writer contract shares a location with the output module.
  - Blocked-by repair: 33 gains 28 and 32.
- Ticket 34:
  - Notices: skipped names (printed in quoted octal), bracket mismatch (clause
    id and atom span), divergence, ghost row, and fold-map union (none).
  - A lint code for Windows device names.
  - The dedup key, the envelope and the `help:` line shape.
  - Blocked-by repair: 34 waits on 8, 12, 13, 20, 28, 29 and 31.
- Ticket 35:
  - The output module path.
  - The Rust API of `GroupedQuerySet` and the row API.
  - `RelativePath` visibility and the public API break.
  - A durable bench home.
  - Type-based enforcement of byte order.
  - Blocked-by repair: 35 waits on 8, 12, 16, 22, 33 and 34.
- Ticket 36: task-row group equivalence, durable benches, Windows and Linux
  locking, and the `opt-level = "z"` recheck.

ADR duties: ADR-0005, ADR-0008, ADR-0009 and ADR-0010 (section 1).

Glossary duties:

- `src/GLOSSARY.md` (the Ambiguity entry is already consistent).
- `src/query/GLOSSARY.md` (`group_by` becomes a terminal that returns a Grouped
  Query Set).
- `src/query/GLOSSARY.md` (Sort Order defaults to descending, while `--group-by`
  is ascending).
- `src/query/GLOSSARY.md` and `GLOSSARY-MAP.md` (output rendering moves).
- `src/index/GLOSSARY.md` (Index Fingerprint is stale per ticket 25).

The map duties below were done on 2026-11-16.

Map duties (`.scratch/arch-design/map.md`): add "Decisions so far" lines for
tickets 28, 29 and 32. Update the entries that name the platform family in the
identity. Graduate the fog item "Persistence portability". Keep "Output
formats" open, because `--format` is deferred. Record the Blocked-by repairs.

Observations for tickets 24, 25 and 33. They are not decided here
([g2-pushdown-ux.md](../research/29-prep/g2-pushdown-ux.md)):

- Skip the `WorkspaceIndex` drop at process exit. It costs 22-31 ms at 50k on
  scans.
- Skip the redb commit when nothing changed. It costs about 10-11 ms at every
  size. That is a third of a 1k query.
- The freshness walk is the largest lever: about 110 ms at 50k. Plain `find`
  over the same files takes 125 ms. `fd` takes 60 ms.
- The template path loads the whole generation per run: about 50 ms at 5k and
  255 ms at 50k. Its decode is single-threaded.
- Stale state after one edit costs +26 / +21 / +47 ms at 1k / 5k / 50k.

## Code delta

The code does not implement this ticket. This table lists what the refactor
changes. The locations come from the code reads of the grilling session
(2026-11-14 and 2026-11-15).

| Area | Today (file and symbol) | Target (ticket decision) | Depends on |
| --- | --- | --- | --- |
| Predicate IR | Separate parsers `src/query/grammar/{filter,source,expr}.rs`. `SourceAtom::Path(GlobPattern)` at `source.rs`. | One IR with `PathAtom::{Literal, Folder, Glob}` and `Tag`, `Class` atoms. Both grammars lower to it. | Ticket 19, ticket 30 |
| Class expansion | `visit_atoms_mut` changes `ClassExpansionMode` in place (`source.rs`). `with_children` does the same in templates (`src/template/engine/query.rs`). | Class names and depth are immutable in the IR. The plan holds the resolved set. | Ticket 20 |
| Spans | Spans only in `QuerySyntaxError` (`src/query/error.rs`). | `Option<Span>` on every atom plus a clause id. A builder-owned clause table. | Ticket 31, ticket 34 |
| Filter moves and TopK | `fuse_sort_limit` fuses a Sort and a Limit into `TopK` (`src/query/plan.rs`). | A Filter never moves before a Limit. TopK needs the same key, and numeric or temporal keys at first. | None |
| Candidate selection | `SourceResolver` scans the full store for `All`, `Not` and empty sources (`src/query/service.rs`). | Candidate step with Exact, Inexact and Unsupported. Residual filter always runs. | Ticket 24, ticket 32 |
| Folder and glob paths | Linear `Path` scan (`src/query/service.rs`). | Folder `Path` literal uses the path range. A benchmark covers the remaining glob scan. | Ticket 24, ticket 32 |
| Bracket counts | No counts exist. | Per-field bracket counts in the `meta` facet. | Ticket 24 |
| Row type | `QueryRow` holds `Arc<WorkspaceIndex>` plus `RowIndex` (`src/query/results.rs`). `entries` is `Box<[FileEntry]>` (`src/index/entry.rs`). | The row owns its matching `Arc<FileEntry>`. | Ticket 24, ticket 28 |
| Document order | `cmp_document_order` ignores `item_idx` (`results.rs`). | `(note position, item_idx)` plus the flatten ordinal. | Ticket 23 |
| Row identity | `QueryRow::eq` deep-compares the entry (`results.rs`). | Pre-group ordinal and flatten ordinal. | None |
| Task depth | `render_task_list` indents by `row.depth()` (`src/query/format.rs`). | Active preceding ancestors determine display depth. Orphans start at zero; sort order stays. | Change row 12 |
| Grouping | `QueryTransform::GroupBy` is a sort alias (`plan.rs`). `QuerySet::group_by` (`results.rs`). | Distinct `GroupedQuerySet`. Hash-group. No plan stage after grouping. | Ticket 19, ticket 21 |
| Group key | `SortKey::from_value_ref` turns lists, objects, Tags and Inlinks into Null (`src/query/sort.rs`). | Recursive canonical keys and outer-list fan-out. Temporal subclasses: Civil, Instant, Period. | Ticket 19 |
| Template `group_by` | Dispatch in `src/template/engine/query.rs`, and its test. | The object returns groups. Typed errors name the replacement. | Ticket 31 |
| Lazy errors in minijinja | `get_value` returns `Option`. `enumerate` returns an `Enumerator` (`src/template/engine/query.rs`). | A fallible-terminal channel. | Ticket 31 |
| Memo | `#[derive(Clone)]` copies `OnceLock` state. `push` resets it (`results.rs`). | Shared `Arc` memo keyed by `(plan, frame, generation triple)`. | Ticket 28 |
| CLI flags | `SortArgs` only (`src/cli/mod.rs`). `--count` only on `task` (`task.rs`). | A shared group args struct beside `SortArgs`. `--count` on `list` and `table`. | Ticket 33, ticket 35 |
| Count and summary | `N page(s) from ROOT` (`src/cli/list.rs`, `table.rs`). `N task(s)` (`task.rs`). | `D page(s) in G group(s) from ROOT` under `--group-by`. | Change row 10 |
| CLI synthesis | `TaskFilterArgs::status_filter` formats a string (`src/cli/task.rs`). `escape_default`. | Typed builder calls. | Ticket 33 |
| Rendering | `render` takes `&[QueryRow]` (`src/query/format.rs`). The flat width pass and the empty-table form `"||\n"`. | Output module with a cell grid and one renderer. Per-group widths for groups. | Ticket 35 |
| Default sort order | `Path::cmp` order. | NUL-form key (ticket 32). Stdout order is unchanged. | Ticket 32 |
| Docs and baselines | `tests/baselines/pub-tokens.txt`. Help baselines `tests/baselines/help/{list,table,task}.txt`. Tests `src/query/results.rs`. | Update baselines, tests, glossaries and ADRs. | None |

Not in code:

- The `GroupedQuerySet` type and the output module.
- The pushdown protocol and the per-field bracket counts.
- The conformance tests for group equality and the heading property test.
- Range-key measurement on a real vault.
- Row ownership of `Arc<FileEntry>` (ticket 24).

Unclear in the ticket:

- "Pushdown candidates" for task and list keys. The ticket 24 encoding for task
  range keys stays unmeasured.
- Whether a template needs `take_groups(n)`.
- The Rust shape of `GroupedQuerySet` and of the row API (ticket 35).
- The channel for lazy errors in minijinja (ticket 31).
- Task-row group equivalence (ticket 36).

### Final review amendments (2026-11-16)

The maintainer confirms recursive group keys, exact scoped mismatch notices,
emitted-ancestor task depth and safe display labels.

Group equality is a canonical equivalence relation. It is not every filter
matching relation. Civil values and instants remain separate; period membership
never partitions groups.

The review's numeric counterexample uses `9007199254740992` and
`9007199254740993`. Both map to the same f64 value. That probe is a superset,
not Exact. Complementing it can lose a valid row.

Required regressions cover that complement, nested keys, object insertion order,
duplicate outer-list keys, scoped notices and child-before-parent task output.
Check terminal safety separately from Markdown structure. These requirements
do not claim that the target implementation already exists.
