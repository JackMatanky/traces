# Ticket 29 prep: brief and breakdown

This prep gives evidence and candidate designs for the grilling of
`issues/29-query-core-and-plan.md`. This prep decides nothing. The user answers
the grilling questions.

## Layout

This directory follows `research/24-prep`.

- Phase 1 files: `a*` (code), `b*` (prior decisions), `c*` (digests), `d*`
  (crates), `e*` (web precedent).
- Phase 2 files: `g*` (probe), `h*` (stress tests), `z-grilling-guide.md`.

## Rules for every file in this directory

- Tag each claim. `[V]` means you saw it in code, a digest, a crate source or a
  fetched page. Give `path:line` or the URL. `[H]` means a hypothesis. `[P]`
  means a proposal.
- Cite `path:line`. Do not copy a claim from an earlier prep file. Check it
  first.
- Write in STE (ASD-STE100). Use the checker: `python3
  .agents/skills/ste/scripts/check_ste.py check <file>`. The target is 0 errors.
- Use topic names for ticket sections. Do not use ticket line numbers. The
  ticket text can change.
- The repository is at main `0481f5c9`. Check `git diff 207cc3fb..main -- src
  tests docs` before you cite code.
- Do not edit any file outside this directory.

## Breakdown of the ticket into concepts

| id | Concept | Ticket topic | Questions to research |
|----|---------|--------------|-----------------------|
| C1 | Grammar shape | Alternatives 1-2 | One predicate language or two grammars? What does term splitting cost? |
| C2 | Plan and pushdown protocol | Alternatives 2-3 | Does a plan with a pushdown protocol exist, or does direct interpretation suffice? |
| C3 | Pushdown soundness | Inputs on pushdown | Candidate superset rule, row-mode scoping, `limit` and `flatten` equivalence proofs. |
| C4 | GroupBy | Question bullet 3 | Real grouping, removal, or the sort alias? Which key equivalence? |
| C5 | Typed builder versus strings | Question bullet 4 | What does the CLI synthesize today? What would a typed IR change? |
| C6 | QuerySet laziness | Question bullet 5 | Memoized typed errors, generation naming, the overlay attach point. |
| C7 | Row representation | Question bullet 6 | `Arc<FileEntry>` shape, view enum benchmark gate (ticket 18). |
| C8 | Item order and membership | Handoffs to 23 | `item_idx` semantics, the `cmp_document_order` fix, list membership. |
| C9 | Class binding | Ticket 20 inputs | Exclusion atom, folder glob optimization, `Path` scan benchmark. |
| C10 | Output and spans | Question bullets 7-8 | `format.rs` ownership, span preservation at the template boundary. |
| C11 | IR serialization | IWE question | Should the IR have a serde form for LSP and MCP hosts? |
| C12 | Language additions | Ticket 19 inputs | `in (...)`, ranges, relative keywords, null placement, string functions. |
| C13 | Compatibility and measurement | Resolution must name | Change-table rows, benchmark plan, evidence limits. |

## Phases

1. Phase 1: code, prior decisions, digests, crates and web precedent.
2. Phase 2: stress tests of candidate designs, one probe, and the grilling
   guide.

## Files

| File | Content |
|------|---------|
| `a1-query-code.md` | `ExecutionPlan`, `QueryTransform`, `QuerySet`, `QueryRow`, grammar modules, CLI synthesis, template bridge, benches. |
| `b1-prior-decisions.md` | Contracts from tickets 16, 17, 18, 19, 20, 21, 23, 24 and research 06, 12, 13. |
| `c1-digests-query.md` | Dataview DQL, Tasks grammar, zk SQL builder, IWE query document. |
| `d1-crates.md` | DataFusion pushdown protocol, Polars lazy plan, gluesql key encoding, minijinja object limits. |
| `e1-web-plans.md` | Predicate pushdown designs, range-key encodings, planner precedents on the web. |
| `h1-stress-designs.md` | Candidate grammar and plan designs, requirements table, failure modes, inconsistencies. |
| `z-grilling-guide.md` | Order of the questions, evidence, options and proposals. |

## Inconsistencies to watch

1. Research 06 praised a lazy template builder. Research 11 refuted the eager
   clone claim; `QuerySet` chaining is already cheap. Cite the refuted state.
2. The CLI synthesizes `list.completed == false` strings today
   (`src/cli/task.rs:42-64`). The keep-as-is option must state this cost.
3. Ticket 24 gives the candidate-superset rule. This ticket owns the protocol
   detail. Do not re-decide the facet layout.
4. The `cmp_document_order` bug ignores `item_idx` (`src/query/results.rs:431`).
   Name the fix and its test in the resolution inputs.
