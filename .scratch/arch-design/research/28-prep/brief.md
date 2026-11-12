# Ticket 28 prep: brief and breakdown

this prep gives evidence and candidate designs for the grilling of
`issues/28-live-buffer-and-persisted-index-consistency.md`. this prep decides
nothing. the user answers the grilling questions.

## layout

this directory follows `research/24-prep`.

- phase 1 files: `a*` (code), `b*` (prior decisions), `c*` (digests), `d*`
  (crates), `e*` (web precedent).
- phase 2 files: `g*` (probe), `h*` (stress tests), `z-grilling-guide.md`.

## rules for every file in this directory

- tag each claim. `[v]` means you saw it in code, a digest, a crate source or a
  fetched page. give `path:line` or the url. `[h]` means a hypothesis. `[p]`
  means a proposal.
- cite `path:line`. do not copy a claim from an earlier prep file. check it
  first.
- write in ste (asd-ste100). use the checker: `python3
  .agents/skills/ste/scripts/check_ste.py check <file>`. the target is 0 errors.
- use topic names for ticket sections. do not use ticket line numbers. the
  ticket text can change.
- the repository is at main `0481f5c9`. check `git diff 207cc3fb..main -- src
  tests docs` before you cite code.
- do not edit any file outside this directory.

## Breakdown of the ticket into concepts

| id | Concept | Ticket topic | Questions to research |
|----|---------|--------------|-----------------------|
| C1 | Overlay truth and scope | Group 1 | Which reads observe unsaved content? What divergence does a CLI accept? Where does the overlay live? |
| C2 | Overlay on a pinned generation | Group 2 | What does a request pin? Which generation owns an overlay entry? What pair does a result name? |
| C3 | Edits, parse and positions | Group 3 | Which revision does a request see? What triggers a parse? How do UTF-16 conversions stay correct? |
| C4 | Save and close | Group 4 | What joins a save to the store? What happens at `didClose` without save? |
| C5 | Deletion and rename | Group 5 | What is the overlay rule for a gone path? What key survives a rename? |
| C6 | External change | Group 6 | Does the host see external edits to an open key? What does it report? |
| C7 | Freshness and the staleness bound | Group 7 | Does an overlay query check sources? What does the bound mean after a replay? |
| C8 | Open-buffer schema check | Group 8 | Which content feeds `parse_with_doc`? Which revisions trigger a check? |
| C9 | Multi-process limits | Group 9 | What do the store lock and the replay revision force on the overlay? |
| C10 | Compatibility | Change table | Which observable change needs a row? Is a host answer a new surface? |
| C11 | Evidence needs | Evidence gate | What costs are unmeasured: parse per keystroke, debounce, memory? |

## Phases

1. Phase 1: code, prior decisions, digests, crates and web precedent.
2. Phase 2: stress tests of candidate designs, one probe, and the grilling
   guide.

## Files

| File | Content |
|------|---------|
| `a1-buffer-code.md` | Host seams, `WorkspaceIndex` readers, template engine index cache, `parse_markdown` input shape, tests. |
| `b1-prior-decisions.md` | Contracts from tickets 16, 17, 20, 21, 23, 24, 25, 26, 27 and the md-pkm-lsp map. |
| `c1-digests-lsp.md` | markdown-oxide, marksman, rumdl, zk, vscode-markdown-languageservice buffer and overlay facts. |
| `d1-crates.md` | rust-analyzer VFS and salsa shapes, ropey, notify, tower-lsp-server, redb concurrency. |
| `e1-web-buffer-models.md` | LSP sync models, editor truth conventions, save and close semantics on the web. |
| `h1-stress-designs.md` | Candidate overlay designs, requirements, failure table, inconsistencies. |
| `z-grilling-guide.md` | Order of the questions, evidence, options and proposals. |

## Inconsistencies to watch

1. The md-pkm-lsp map predates tickets 23 to 25. Its `RefreshPlan` single-file
   idea and its redb assumptions can conflict with the settled read model. Check
   each md-pkm-lsp claim against those tickets before you cite it.
2. Ticket 23 says the host owns the debounce. Ticket 33 owns the wiring. This
   prep states semantics only, never host internals.
3. The redb multiprocess mode is experimental and refuses `Durability::None`. Do
   not treat it as an available option without a source.
