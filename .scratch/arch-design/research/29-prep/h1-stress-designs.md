# Stress-test designs for ticket 29: query core, plan, rows

Rules: candidate designs, requirement checks, failure modes, inconsistencies.
Tags [V]/[H]/[P]. Cite `path:line` or ticket lines. Do not preset the Answer.

## Candidate designs

### D1: keep two grammars + transform-list plan, add facet candidate selection

Source grammar stays; filter grammar stays; the plan gains a candidate step
that ticket 24 needs (folder range, tag keys, class atoms, else facet scan).

Checks:

- R-ticket 24: satisfies candidate-superset by construction. Candidates come
  from the source atom only, and the residual filter runs on all candidates.
  The `Inexact` pruning never decides (research 06, DataFusion) [V].
- Failure mode: `All` and `Not` enumerate `files` paths. A filter that
  references a field cannot prune. No range key exists for it. The cost is
  the facet scan, 21 ms stand-in at 24-prep [V].
- Failure mode: two grammars mean two parse error surfaces and two span
  pipelines. Ticket 31 needs spans across both.
- Keep-as-is cost comparison: the `GroupBy` alias stays deceptive (survey,
  findings ledger). Ticket 16 makes spelling migratable. Keeping the alias
  without a rename keeps the lie [H].

### D2: unify grammars with term splitting and pushdown

One grammar; the planner splits terms (SQLite `whereSplit` model), maps each
to a candidate key or a scan, and the residual runs the rest.

Checks:

- R-ticket 24: candidate superset provable per predicate only when the key
  and the filter read the same value. Task rows break this: item metadata
  overrides note fields (`src/query/results.rs:320-333`) [V]. So a field key
  applies to note-mode rows only, and task rows fall back to scan. That is
  the per-row-mode rule ticket 24 demands.
- Failure mode: `contains` on a tag field is not a byte-prefix property.
  Only equality-like predicates get keys. The superset gate decides per
  predicate; unsupported goes to scan. DataFusion names the three judgments
  `Exact`, `Inexact`, `Unsupported`; Traces needs only Exact-or-scan [H].
- Failure mode: push before `limit` and `flatten` needs the per-row-mode
  proof. The `pages(All).limit(2).filter(...)` test proves transforms keep
  their order [V]. So pushdown past `limit` is forbidden unless the limit is
  a `TopK` with the same key. State it as a proof obligation.
- Failure mode: grammar unification is a lexer-level change first. Logos
  already powers both grammars (`src/query/grammar/`) [V]. The boolean
  precedence module is shared (`src/query/grammar/expr.rs`) [V]. But source
  and filter value spaces differ (paths and globs versus literals), so a
  merged AST needs one node kind per role [H].

### D3: typed builder as the single IR, DSL as one frontend

`QueryBuilder` chains become the IR. CLI string synthesis dies. Hosts build
data.

Checks:

- R-ticket 16: stdout shapes stay. DSL spelling is migratable, so replacing
  string synthesis with builder calls is a change-table row [H].
- Matches the IWE serialisable query document precedent (research 13) [V].
  A serde form of the IR is the alternative surface for LSP and MCP.
- Failure mode: `contains` needs target-shape checks that strings do today.
  The builder must validate, or errors move later and lose spans [H].
- Failure mode: `minijinja` `call_method` dispatch builds filter strings
  today (`src/template/engine/query.rs`). A builder IR replaces that
  dispatch with data. Ticket 31 owns that bridge [P].

## GroupBy options

- Remove: sorting plus a `group` marker is honest. Dataview's `GROUP BY`
  restructures rows into `{key, rows}` and changes row identity [V]. If
  Traces keeps flat rows, "group" is a lie.
- Nest: a new row shape breaks `cmp_document_order` and terminal accessors.
  The cost is a second row mode across every consumer.
- Keep alias: document it as `sort`. Cheapest. The survey finding stays on
  record as accepted debt [P].

## Requirement table

| Req | Source | D1 | D2 | D3 |
| --- | --- | --- | --- | --- |
| candidate superset proof | t24 | by source atom | per predicate | per predicate |
| task-row override safety | t24, `results.rs:320-333` | scan fallback | row-mode rule | row-mode rule |
| push past limit proof | t24 | n/a (no push) | forbidden unless TopK | same as D2 |
| spans to template | t31 | both grammars | one pipeline | builder loses spans |
| IR for hosts | t17, research 13 | none | AST serde | builder serde |
| removals | t24 + survey | GroupBy alias | none left | string synthesis |

## Inconsistencies to resolve at grilling

1. Row repr: ticket 24 settled `Arc<FileEntry>` rows. Ticket 29 owns the row
   modes (note, list, task) and `item_idx`. `cmp_document_order` compares
   position only and ignores `item_idx` (`src/query/results.rs:420-431`)
   [V]. The fix is a comparison that breaks ties by `item_idx`. The item
   order contract that ticket 23 requested is the same rule. One decision,
   two tickets cite it.
2. Membership versus equality: ticket 19 hands over membership as a
   predicate distinct from `==`. The grilling must not let a range key
   assume membership is orderable. `CalendarPeriod` equality is the
   `(start, unit)` identity, not an order [V].
3. `QuerySet` overlay attach point: the row type must let ticket 28's
   overlay replace one row without re-planning. The cheapest contract: rows
   are immutable, and the overlay swaps the row list before memoization.
   Ticket 28 owns the swap rule. Ticket 29 must only guarantee the seam.
4. Advisory notices (ticket 19): bracket-mismatch exclusions and invalid
   date literals need stderr notices. Stdout and exit codes stay frozen
   (ticket 16). Ticket 34 owns the envelope. The plan must not emit notices
   directly; it collects them on the result.
