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
| ADR-0009 (index refresh) is implemented but still `status: proposed`; its text says format-version mismatch is a `Drifted` state, while code wipes the store at `IndexStore::open` | `docs/adr/0009-index-refresh.md:5,40-41`; `src/index/store.rs` `rebuild_needed`/`META_VERSION` | Re-verified at main @ `207cc3fb`; ADR status and wording need a human pass (ADR rule: AI-written ADRs stay `proposed`) |
| `Config::hash`, `ConfigDigests::compose`, and the root/templates/schema-directory digests still have no production consumer; the `syn` scan of 6 source files remains | `src/config/model.rs:160`, `src/config/model/digest.rs:42,183-264` | Residual of the landed drift work; ownership asked in the config-lifecycle ticket |
| Scout citation errors in surveys: `src/note/task.rs` (is `src/note/parser/task.rs`), `src/query/source.rs` (is `src/query/grammar/source.rs`) | `survey/config-dialog.md:90`, `survey/cli.md:72` | Corrected by banner; no decision depends on them |
| `RefreshReport` is only logged in production; its count accessors are test-only (`expect(dead_code)`), so a config-drift heal is invisible to the CLI user except at `RUST_LOG=info` | `src/index/refresh.rs:50-59`, `src/index/service.rs:300-316` | Observability of heals is asked in the freshness and workspace-host tickets |
| Divergence register (417 lines, R1-R18) is temporal-core-only by user ruling; it lived in `docs/` beside an older 128-line copy under `.scratch/temporal-core/` | `.scratch/temporal-core/divergence-register.md` | Resolved: the newer docs copy replaced the scratch copy; citations repointed (line numbers unchanged) |
| Without an attached `FileClassExpander`, store queries fall back to exact class match but in-memory filters match nothing; production always attaches one, so it is latent | `src/query/service.rs:402-406`; `src/query/grammar/source.rs:228-229`; `src/cli/mod.rs:478-480` | Replaced by plain class-hierarchy data (ticket 20 Answer); query-side change in the query-core ticket |
| The CLI decides whether to load schemas for a query (`has_classes`), i.e. composition lives in `cli` | `src/cli/mod.rs:474-480` | Asked in the workspace-host ticket (30) |
