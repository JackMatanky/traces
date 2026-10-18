# Tactical findings ledger (outside the map's destination)

Scout findings that are local defects or mechanical cleanups: fixable without an architectural decision. They are
NOT on the route. Status is **scout-reported, unverified** until `research/01` runs; fix them through ordinary issues.
Anything that turns out to depend on a map decision is promoted back into the owning ticket.

| Finding | Evidence (scout) | Notes |
| --- | --- | --- |
| `QueryRow::cmp_document_order` ignores `item_idx`, so rows of the same note compare equal | `src/query/results.rs:431` (query survey H2) | Correctness bug; the ordering *invariant* is also asked in the query-core ticket |
| `traces list \| head` panics (EPIPE, exit 101/134) on unbuffered `print!` | `src/cli/{table,list,task,template}.rs`; `.scratch/integration-e2e-suite/issues/05-epipe-exit101-decision.md` | Process contract; stdout-writer ownership is asked in the workspace-host ticket |
| `speedate` has one call site; `winnow` has one call site | `src/temporal/date.rs:296`; `src/temporal/lexer.rs:7` | Dependency pruning; ordered after the temporal-boundary decision |
| `GroupBy` is an ascending-sort alias | `src/query/plan.rs:254-258` | Semantics asked in the query-core ticket |
| Template path inspection accepts unconfined absolute paths; `DryRun` skips target confinement | `src/template/engine/path.rs:162`; `src/template/service.rs:225` | Invariant ownership asked in the path-and-capability ticket; point fixes may precede it |
| `SortedByPath::assumed_sorted` checks ordering only under `debug_assert!` | `src/index/sort.rs:48-57` | Invariant strengthening; folds into the read-model ticket |
| Glossary drift: note GLOSSARY slugs/footnotes/callouts; `src/GLOSSARY.md` Occurrence/NavTarget/Resolution/Ambiguity; "Avoid: relative path" vs `RelativePath` | `src/note/GLOSSARY.md:45-105`; `src/GLOSSARY.md:18-20,72-105` | Docs correction through `domain-modeling` |
| Two ADRs numbered 0009 | `docs/adr/0009-index-refresh.md`, `docs/adr/0009-filter-date-literals.md` | Renumber |
| `lib.rs` is 74% test fixtures | `src/lib.rs:230-892` | Layout; the public-surface question lives in the crate-topology ticket |
| `DialogProvider::select` returns index 0 silently when non-interactive; `DialogError::NotInteractive` unreachable | `src/dialog/terminal.rs:84`, `src/dialog/preset.rs:223` | Contract asked in the workspace-host ticket |
| `[templates].directory` has no default while `[schemas].directory` does | `src/config/builder.rs:76-82,141-150` | Config default asymmetry |
