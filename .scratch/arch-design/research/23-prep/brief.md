# Ticket 23 prep: brief and breakdown

This prep gives evidence and candidate designs for the grilling of
`issues/23-note-document-model.md`. This prep decides nothing. The user answers
the grilling questions.

## Layout

This directory follows `research/20-prep` and `research/22-prep`.

- Phase 1 files: `a*` (code), `b*` (prior decisions), `c*` (digests), `d*`
  (crates), `e*` (web precedent).
- Phase 2 files: `f*` (candidate design stress tests), `g*` (probe), `h*` (fact
  check) and `z-grilling-guide.md`.

## Rules for every file in this directory

- Tag each claim. `[V]` means you saw it in code, a digest, a crate source or a
  fetched page. Give `path:line` or the URL. `[H]` means a hypothesis. `[P]`
  means a proposal.
- Cite `path:line`. Do not copy a claim from an earlier prep file. Check it
  first.
- The earlier agent files `22-prep/n1`, `n2b`, `n5` and `n6` have errors. Do not
  cite them.
- Write in STE (ASD-STE100). Use the checker: `python3
  .agents/skills/ste/scripts/check_ste.py check <file>`.
- `src/note` has not changed since `207cc3fb`. `src/index`, `src/query` and
  `src/task.rs` changed after `207cc3fb`. Most changes are formatting. About 99
  lines in `src/index/service.rs` are not formatting. Check any claim about
  these files again.

## Breakdown of the ticket into concepts

| id | Concept | Ticket topic | Questions to research |
|----|---------|--------------|-----------------------|
| C1 | Representation split | A | What do precedents split? Who owns each invariant? What does each candidate cost? |
| C2 | Span model | A, B | Which unit? What size? Which nodes? Persisted or transient? |
| C3 | Frontmatter spans | B | Which YAML parser gives marks? What does a line scan cost? |
| C4 | Items: children or entities | C | What row identity do `item_idx`, `list.*` and document order need? |
| C5 | Item-to-note rollup | C | What does Dataview do? Which query results change? |
| C6 | Multi-valued inline fields | C | How do fields merge? Which source wins? |
| C7 | Task status representation | D | What does `ListItem` cost? What do interners offer? |
| C8 | Parser input and `IndexPolicy` | D | Which fields? Borrowed or owned? |
| C9 | Markdown pass count | D | Can one pass give the same heading text? |
| C10 | Footnotes, callouts, slugs | D | Which consumer needs which fact? |
| C11 | Infallibility and notices | D | How does the parser report a notice? |
| C12 | Persistence | A | Store the whole `Note` or extracted facts? Can the store decode lazily? |
| C13 | Live buffer | A | How does an unsaved buffer enter the model? |
| C14 | Body facts for ticket 36 | B | Which heading extents and text slices? |
| C15 | Compatibility of each change | Compatibility rules | Which query results change? |
| C16 | Performance budgets | Evidence gate | What does each operation cost at each vault size? |
| C17 | Naming, visibility, glossary | D | What does the glossary claim that the code lacks? |

## Phases

1. Phase 1: code, prior decisions, digests, crates and web precedent.
2. Phase 2: one stress test for each candidate design, a measured probe, a fact
   check, and the grilling guide.

## Inconsistencies found so far

Each item has a source. The ticket text now covers items 1 to 4.

1. Ticket 18 calls `NoteLink` editor-side. Topic A has not chosen the split. The
   `NoteLink` definition depends on topic A.
2. Ticket 20 says the app layer owns the merge of inline fields. Ticket 20 also
   says ticket 23 handles the merge. Topic C must name one owner.
3. Ticket 16 makes vault-format parsing Frozen and makes the `index.redb` layout
   Free. An observable change must be in the change table. The first version of
   the ticket text did not say this.
4. Ticket 17 says the seven vault sizes are measurement points. The first
   version of the ticket text said "set a budget for each operation across the
   sizes". This was stronger than ticket 17.
5. `src/note/parser.rs:77-78` says the second pass enables "math and footnote
   parsing". `MARKDOWN_OPTIONS` already enables footnotes (`parser.rs:68-70`).
   Only `ENABLE_MATH` differs (`parser.rs:73`). The doc comment is wrong [V].
6. `src/index/entry.rs:230` says `Note` is 240 bytes. The probe measures
   `size_of::<Note>()` as 232 bytes (release build, macOS arm64) [V].
7. `Note::fields()` and `Note::get()` already give frontmatter priority over
   inline fields (`src/note/model.rs:128-173`). `fields()` has a `dead_code`
   expectation outside tests. Topic C decides a question that the code partly
   answers.
8. `b3-open-ticket-demands.md` says spans can bloat the store by 3x to 5x. It
   gives no source (`b3:19`). `e3-web-spans-positions-perf.md` says persisted
   spans inflate note decoding. The probe measures +4.3% bytes for spans on
   headings, items and links (see `g-probe-results.md`). The list item text is
   93% of the `ListItem` bytes. Decoding runs at about 1 ns for each byte.
9. Several agent outputs had no value. I deleted the stub
   `b1-prior-tickets-01-15.md`. I replaced the placeholders `d1` and `d2` and
   the filler file `a1b` with facts that I checked. The agent for research
   tickets 10 to 15 (`B1bResearch10to15R`) wrote no file. Its reply was
   incoherent. Research 10, 11, 12 and 15 still need a careful read.
10. `a2b-query-template-consumers.md` said that `Cargo.toml` has LSP
    dependencies. It has none [V].
11. Line citations of the form `23:NN` in `b2a`, `b2b`, `b3` and `e1` point to
    the version of ticket 23 in commit `47628bd3`. The ticket text changed after
    that commit. Use topic names, not line numbers.
12. The first version of `c2-digests-obsidian.md` said Dataview merges non-task
    item fields into the nearest ancestor task. The Dataview digest shows that
    this code is commented out (digest lines 3606-3614). The active code merges
    non-task item fields into the page (lines 3603-3604). The ticket 23 sentence
    "Dataview rolls up the fields of non-task items" is correct [V].
13. Dataview merges frontmatter values and body values of one key by
    concatenation (digest lines 3391-3393 and 3734-3737). It has no precedence
    rule. `Note::get` gives the frontmatter value first
    (`src/note/model.rs:160-173`). The two behaviours differ. The question
    "which source wins" has no Dataview answer.
14. `e3-web-spans-positions-perf.md` said that Marksman, IWE and rumdl use full
    sync. IWE uses full sync [V]. Marksman reads the sync kind from a setting
    [V]. No evidence exists for rumdl.
15. `e3` and `e1` say a reparse of a note takes less than 50 µs. This is the raw
    `pulldown-cmark` time. The full `parse_markdown` takes about 217 µs for a
    mean 11 KB note [V] (`g-probe-results.md`).
16. Ticket 16 lists the `.first()`-only read of multi-valued fields as a known
    defect. Fixing it is a change-table row. The ticket text lacked this. It has
    it now.
17. Three numbering schemes name the split candidates: the letters A to D in
    `research/20-prep/h-arch-design-note-split-review.md`, the numbers 1 to 5 in
    `e1`, and the bullets of the ticket. Candidates 2 and 3 of `e1` are almost
    the same design.
18. Not done in this prep: research 10, 11, 12 and 15 (no careful read), the
    stress test of each candidate, the fact check and the grilling guide. The
    agent backends failed for most of the session.
