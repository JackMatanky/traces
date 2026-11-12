# Grilling guide for ticket 29: query core and plan

Purpose: question order, evidence, options, and proposals per decision
bullet. Do not answer for the user. Keep STE rules in the Answer.

## Order

1. Grammar shape (one language or two). The plan shape depends on it.
2. Plan and pushdown protocol. The candidate rules and the per-row-mode
   proof live here.
3. Row type and modes, `item_idx`, `cmp_document_order`.
4. `GroupBy` decision and key equivalence.
5. Membership operation and its separation from equality and sort.
6. `QuerySet` laziness contract, memoized errors, overlay seam.
7. Typed builder versus string synthesis; the IR serde question.
8. `format.rs` ownership and spans.
9. Benchmark plan and change-table rows as the wrap-up.

## Measured and adversarial evidence

Two deeper evidence files back this guide:

1. `g1-probes.md` (shared): the three-table layout costs 33.7 MB at
   50k in one commit; a stage-per-commit build costs the same total
   (files 16.8 MB, text facet +16.8 MB, numeric facet +0 MB from page
   reuse). A folder range returns 7,143 rows in 261.5 µs. A numeric
   range `rating >= 4.5` over order-preserving f64 keys returns 5,000
   rows in 171 µs, with path order inside the key. Inserts cost
   153.1 ms and commit 59.6 ms (medians of 5, three tables). A facet
   rebuild by table clear costs 3.7 ms. Deletes do not shrink the
   file; only compaction reclaims.
2. `h2-redteam.md`: attacks with defenses, including `Not` above an
   Exact atom must classify Inexact, the tag-uniqueness and NUL
   admission questions the encoding decision must answer, and the
   task-row GroupBy equivalence that stays unmeasured. Scope the
   equivalence claim or add it to ticket 36's measurement list.

## Per-bullet evidence and options

### Grammar

Evidence: logos powers both grammars [V]. The boolean precedence module is
shared (`src/query/grammar/expr.rs`) [V]. Source values are paths, globs,
tags, classes. Filter values are literals [V].

Options:

- A. Keep two grammars. Smallest diff; two error surfaces; the candidate
  step reads the source atom only.
- B. Unify into one grammar with two value spaces. One span pipeline; one
  AST; the merged node kinds need one kind per role [H].

Proposal: decide by the IR question (bullet 7). If the resolution adopts a
serde IR for hosts, unification buys less; if the DSL stays the single
surface, two grammars stay cheap. IWE precedent: a serialisable query
document with a published schema (research 13) [V].

### Plan and pushdown

Evidence: ticket 24 settled candidate-superset, facet scans, and range keys
only for eligible note-level scalar fields [V]. Task rows override note
fields (`src/query/results.rs:320-333`) [V]. The transform-order test
proves filters never jump before `limit` [V]. DataFusion ternary protocol
and SQLite `whereSplit` are the precedents (research 06) [V].

Options:

- A. Candidate step from source atom only; residual filter on all
  candidates. No per-predicate pushdown past transforms. This is D1; the
  proofs collapse to one rule.
- B. Per-predicate judgments (Exact-or-scan) plus the per-row-mode proof
  for pushing before `limit` or `flatten`. Pushing before `limit` stays
  forbidden unless the transform is `TopK` with the same key.

Proposal: A first, with B named as the follow-up if measurements show the
facet scan hurts at 50k (21 ms stand-in at 24-prep [V]). Record the proof
obligations either way: the superset gate must read the same value the
filter reads, per row mode.

### Row type

Evidence: rows own `Arc<FileEntry>` (ticket 24) [V]. `cmp_document_order`
ignores `item_idx` (`src/query/results.rs:420-431`) [V]. Ticket 23 asked
for the item order contract [V]. Ticket 28 needs the overlay seam.

Options:

- A. Two fields: position plus `item_idx`, compare position then `item_idx`.
  Fixes the invariant; one rule for ticket 23's order contract.
- B. A synthetic global order across modes. Bigger change, no known need.

The mode question: note, list, task rows share one struct with a mode tag,
or three types behind one enum. One struct is today's shape (`kind:
Page|List`, `src/query/results.rs:37-43`) [V]. Keep one struct; the enum
already exists.

### GroupBy

Options: remove (sort plus marker), nest (new row shape; breaks terminal
accessors), keep alias documented as sort. Dataview's `GROUP BY` restructures
rows into `{key, rows}` [V]; Traces rows stay flat, so nesting is the only
honest grouping, and it is the most expensive. The alias is accepted debt
only if documented [P].

Key equivalence: ticket 19's exact numeric equality for cross-kind keys;
period membership is never a key [V].

### Membership

Evidence: ticket 19 handed membership as a predicate distinct from `==`.
`CalendarPeriod` equality is the `(start, unit)` identity [V]. The
period-instant comparison uses the instant's local civil date (ticket 21)
[V].

Proposal: a `member_of` predicate in the IR. Never a range key (periods do
not order). Advisory notice when a bracket mismatch excludes rows (ticket
19). Envelope to ticket 34.

### QuerySet laziness

Evidence: verified lazy, `Arc` base + plan, memoized at terminals (research
01 claim 7) [V]. Ticket 21 demands memoized typed errors [V]. A corrupted
facet means a complete miss with fresh-parse fallback, exit 0 + one notice
(ticket 24) [V].

Options for the seam:

- A. Overlay swaps the row list before memoization; rows immutable. Ticket
  28 owns the swap rule; ticket 29 guarantees only the seam.
- B. Overlay inside the plan. Rejected: it drags buffer semantics into the
  plan (ticket 28 owns them).

### Builder versus strings; IR serde

Evidence: CLI synthesizes strings (`src/cli/task.rs:42-64`) [V]. IWE hosts
build query documents as data [V]. Ticket 16 makes DSL spelling migratable
[V]. Ticket 33 owns who calls the core.

Options:

- A. Keep string synthesis. Cheapest; spans come free from the parser.
- B. Typed builder as core surface; DSL becomes one frontend. Matches the
  IWE precedent; costs a serde form and builder validation.
- C. Both: builder in core, synthesis stays a thin CLI helper.

Spans are the hidden cost of B: string parsing gives spans; builders do not
[H]. Ticket 31 needs spans at the template boundary. That is the argument
for C or for keeping A.

### format.rs

Options: keep inside query (today), move to an output module owned with
ticket 35. Template methods and stdout payload shapes stay reachable either
way (ticket 16) [V]. Decide with ticket 35; this ticket only records the
preference.

## Requirement checks to run against the chosen design

1. Candidate superset per predicate or per source atom, stated.
2. Task-row override safety: note-field keys never prune task filters.
3. Push-past-limit proof obligation named or the push forbidden.
4. `cmp_document_order` fix named with `item_idx`.
5. Memoized typed errors at every terminal; no silent empty results.
6. Overlay seam exists; no buffer semantics in the plan.
7. Notices collected on the result; stdout and exit codes unchanged.
8. Every language addition named as new language in the change table.

## Cross-ticket consistency checks

1. 29-24: no re-decided facet layout; range-key encoding stays unmeasured
   until the benchmark plan runs.
2. 29-28: the seam contract matches the swap-before-memoization shape.
3. 29-32: folder ranges assume the persisted order of ticket 32; state the
   need, not the bytes.
4. 29-31: spans preserved to the boundary; `QueryError` keeps locations.
5. 29-30: relative keywords and the filename period parser wait on ticket
   30; `FileMeta.day` removal stands.
6. 29-19: the value model rules (brackets, NaN to Null, adaptive literals)
   are settled inputs, not questions.

## Benchmark plan skeleton for the Answer

- Owned versus view for `file.name` sort and filter (ticket 18 gate).
- Folder glob optimization versus the linear `Path` scan
  (`src/query/service.rs:417-424`).
- Range-key encoding write and read cost at 5k and 50k (ticket 24 open
  item; prototype tables grew redb 67 to 135 MB [V]).
- Full row materialization on a real vault (ticket 24).
- Same-stem candidate scale at 5,000 (trie 224 ms versus flat 412-436 ms
  [V]).
