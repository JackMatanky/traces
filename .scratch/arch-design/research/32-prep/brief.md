# Ticket 32 prep: brief and breakdown

This prep gives evidence and candidate designs for the grilling of
`issues/32-path-and-capability-model.md`. This prep decides nothing. The user
answers the grilling questions.

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
- Write in STE (ASD-STE100). Use the checker:
  `python3 .agents/skills/ste/scripts/check_ste.py check <file>`. The target is
  0 errors.
- Use topic names for ticket sections. Do not use ticket line numbers. The
  ticket text can change.
- The repository is at main `0481f5c9`. Check
  `git diff 207cc3fb..main -- src tests docs` before you cite code.
- Do not edit any file outside this directory.

## Breakdown of the ticket into concepts

| id | Concept | Ticket topic | Questions to research |
|----|---------|--------------|-----------------------|
| C1 | Confinement owner | Question bullet 1 | Capability `ProjectRoot`, newtype propagation, or a boundary fs facade? |
| C2 | TOCTOU stance | Question bullet 4 | Accept the window per boundary, or close it? Note reads, inspection, output. |
| C3 | Inspection and DryRun | Question bullet 5 | Make template path tests and preview obey the writer rules? |
| C4 | Persisted path form | Question bullet 6 | OS bytes versus UTF-8, separator rule, case rule, NFC and NFD rule. |
| C5 | Store and journal portability | Ticket 25 inputs | Drop the platform family? What format epoch follows? |
| C6 | Sort order of the form | Ticket 24 inputs | Which byte order do folder ranges and order checks need? |
| C7 | Traversal and tracker paths | Question bullet 7 | `DirTree` and `file_tracker` usage; state-home store types. |
| C8 | Root identity | Ticket 21 and 22 inputs | Canonical identity versus portable path versus confined capability. |
| C9 | Symlinked `.traces` | Open question F11 | May `.traces` be a symlink? Decide or hand back with an owner. |
| C10 | Compatibility and glossary | Ticket 16 inputs | Change-table rows; the "Avoid: relative path" glossary conflict. |

## Phases

1. Phase 1: code, prior decisions, digests, crates and web precedent.
2. Phase 2: stress tests of candidate designs, one probe, and the grilling
   guide.

## Files

| File | Content |
|------|---------|
| `a1-path-code.md` | `src/path.rs`, codec, `SortedByPath` order users, template writer and inspection, tracker and discovery. |
| `b1-prior-decisions.md` | Contracts from tickets 16, 20, 21, 22, 24, 25, 26, 27 and research 09. |
| `c1-digests-paths.md` | IWE key model, git path handling, editor URI variants, vault tools. |
| `d1-crates.md` | strict-path, cap-std, relative-path, camino, typed-path, gitoxide guards. |
| `e1-web-path-models.md` | Path equivalence, NFC and NFD file systems, key encodings, symlink policies on the web. |
| `h1-stress-designs.md` | Candidate confinement designs, requirement table, edge cases, inconsistencies. |
| `z-grilling-guide.md` | Order of the questions, evidence, options and proposals. |

## Inconsistencies to watch

1. The glossary says "Avoid: relative path" (`src/GLOSSARY.md:18-20`) but the
   code ships `RelativePath` (`src/path.rs:64`). A decision that keeps the type
   must correct the glossary.
2. Research 09 says `SafeRelativePath` discards the canonicalized path
   (`src/path.rs:190`). Check that line before you cite it; the file can drift.
3. The Deno advisory GHSA-8xpq-cjcf-3wh9 shows a real NFC and case bypass on
   APFS. Use it as evidence for the Unicode rule, not as a decided design.
4. Ticket 24 needs byte order for folder ranges. Ticket 25 drops the platform
   family only when the form is portable. Keep both contracts in one answer.
