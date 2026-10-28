# Ticket 24 prep: brief and breakdown

This prep gives evidence and candidate designs for the grilling of
`issues/24-index-read-model-and-query-execution.md`. This prep decides nothing.
The user answers the grilling questions.

## Layout

This directory follows `research/23-prep`.

- Phase 1 files: `a*` (code), `b*` (prior decisions), `c*` (digests), `d*`
  (crates), `e*` (web precedent).
- Phase 2 files: `g*` (probe), `h*` (stress tests of candidate designs) and
  `z-grilling-guide.md`.

## Rules for every file in this directory

- Tag each claim. `[V]` means you saw it in code, a digest, a crate source or a
  fetched page. Give `path:line` or the URL. `[H]` means a hypothesis. `[P]`
  means a proposal.
- Cite `path:line`. Do not copy a claim from an earlier prep file. Check it
  first. Earlier prep files have errors (see the `research/23-prep/brief.md`
  list).
- Write in STE (ASD-STE100). Use the checker: `python3
  .agents/skills/ste/scripts/check_ste.py check <file>`. The target is 0 errors.
  Replace each word that the checker flags.
- Use topic names for the ticket sections. Do not use line numbers of the
  ticket. The ticket text can change.
- The repository is at `main` (`39e96041`). `src/` equals `16f67ef0`.
  `src/index` and `src/query` changed after `207cc3fb`. The only non-comment
  change is the `DirTree` `.walk()` seam in `scan`, plus one test. Check any
  claim about these files again.
- Do not edit any file outside `research/24-prep/`.

## Breakdown of the ticket into concepts

| id | Concept | Ticket topic | Questions to research |
|----|---------|--------------|-----------------------|
| C1 | Consumers and one or two execution paths | A | What does each consumer need from the index? What do precedents do when a store and a memory path both exist? |
| C2 | Plan for each source shape | A | What does each shape cost today? Which shape needs a secondary table? |
| C3 | Read model shape | B | Immutable snapshot, lazy store-backed view or columnar facts: what does each cost and who owns each invariant? |
| C4 | Generation coherence and host transition | B | How does a long-lived host move to a new generation? What do rust-analyzer, salsa, `arc-swap` and IWE do? |
| C5 | Failed-persist rule | B | Fail open or fail closed? What do precedent caches do? |
| C6 | Writer admission and store locking | B | What does redb do when a second process opens a held store? |
| C7 | Row borrow source | C | Which type does a row borrow from? What do Polars and DataFusion do? |
| C8 | Persistence granularity | D | Whole `Note`, independent facets or extracted facts? Which encoding? |
| C9 | Secondary tables | D | Which tables stay? Which query needs each table? Can the load build them in memory? |
| C10 | Inbound link graph | D | Persist the graph or compute it in memory? |
| C11 | `FileMeta` and temporal encoding | D | Which fields stay? Does the store keep the zone-derived `day`? Is a compact encoding worth it? |
| C12 | Notices | D | Does the store keep parser notices? |
| C13 | "Has a known explicit class" lookup | D | Where does the lookup live? |
| C14 | Class axes over inline fields | D | Does the index class axis cover inline fields? |
| C15 | Totality and partial index | E | Can a type enforce one entry for each file, sorted by path? |
| C16 | `CandidateTrie` and `indextree` | F | Which query needs them? What replaces them? |
| C17 | Store engine and repository split | F | Does one type hold both jobs? |
| C18 | Memory and budgets | Evidence gate | What does each design cost at 50,000 notes? |
| C19 | Surface for a CLI crate | Compatibility | Which read-model items must be `pub`? |
| C20 | Compatibility of each change | Compatibility | Which observable result changes? Which change-table row follows? |

## Phases

1. Phase 1: code, prior decisions, digests, crates and web precedent.
2. Phase 2: one stress test for each candidate design, a measured probe, and the
   grilling guide.

## Files

| File | Content |
|------|---------|
| `a1-index-query-code.md` | Call sites, tables, `FileMeta` readers, notices, order users, tests, narrow `pub` items. |
| `b1-prior-tickets-01-15.md` | Items from tickets and research 01 to 15. Tickets 16 to 23 were reviewed by the main agent. |
| `c1-digests-lsp-zk.md` | zk, marksman, markdown-oxide, rumdl, vscode-markdown-languageservice, obsidian-parser, Basalt. |
| `c2-digests-obsidian.md` | Dataview, Tasks, Advanced Metadata Cache, Metadata Menu, Fileclass. |
| `e1-web-snapshots-generations.md` | Snapshots, generations, pushdown and row ownership on the web. |
| `e2-web-persisted-index-stores.md` | redb, LMDB, SQLite, git, ruff, rumdl, Cargo, rkyv, roaring, DuckDB. |
| `g-probe-results.md` | Cold query by source shape up to 50,000 notes, refresh after a change, a second process, facet shares. |
| `h1-stress-test-designs.md` | Requirements, candidate designs, rules, row sources, store contents, totality, trie, engine split. |
| `z-grilling-guide.md` | Order of the questions, evidence, options and proposals for each question. |

Crate research used `rust-docs-mcp` for redb 4.3.0, postcard 1.1.3, indextree
4.9.2, rkyv 0.8.18 and salsa 0.28.5. The redb source in the local registry gave
the lock behaviour and the multi-process feature.

## Inconsistencies found and where each one stands

1. The ticket evidence gate left out the `CandidateTrie` measurement of research
   11 and the new probe. Fixed in the ticket.
2. The ticket says the four multimap tables exist for the pushdown. This is
   true. Two of them have no query reader. They keep the forward tables in step
   (`a1`, section D). Added to the ticket Facts.
3. Research 11 advises a held `IndexStore` handle. A held handle makes every
   other `traces` process fail (`g-probe-results.md`). Added to the ticket
   Facts.
4. Research 11 measured a memory ratio of 2.2 to 1 on 20,000 notes. The probe
   gives 9 to 1 on denser notes. The ticket now says the ratio depends on the
   content.
5. The ticket said that the bulk index path collects notices. No code collects
   them today. Fixed in the ticket.
6. The ticket Fixed inputs said the store "persists one facts-only `Note`".
   Topic D asks if the store keeps whole notes or facets. Fixed in the ticket.
7. `IndexerService::load` has no production caller. Added to the ticket Facts.
8. `b1` cites ticket line numbers. They go stale when the ticket changes. Not
   fixed. Use topic names.
9. A path-set change decodes every note even on the store path. The boundary
   with ticket 25 did not say that the facet split changes this. Fixed in the
   ticket.
10. Ticket 20 records the class expander hazard and says production always
    attaches an expander. The ticket fact lacked this qualifier. Fixed in the
    ticket.
11. The first draft of `e2` called a current rule "the ticket's proposal".
    Fixed.
