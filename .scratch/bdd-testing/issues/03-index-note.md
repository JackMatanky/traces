# 03: Index and note substrate, behavioral discovery

Type: research
Status: resolved

## Question

Which observable behaviors of the index and note substrate constitute *intentional guarantees*, as opposed to incidental implementation behavior?

**Scope**: `src/index/` (redb persistence, refresh and delta logic, fingerprinting, inlink graph, refresh states), `src/note/` (markdown AST, YAML frontmatter, inline fields, links, lists, tasks parsing semantics).

**Method**: breadth-first reading of source, inline tests (208 and 304 `#[test]`s), `tests/integration/persistence.rs`, `tests/integration/refresh.rs`, `tests/e2e/index.rs`, `tests/e2e/tracked.rs`, `src/index/GLOSSARY.md`, `src/note/GLOSSARY.md`, `docs/adr/0009-index-refresh` (and the duplicate-numbered 0009; note the collision), plus `.scratch/arch-design/research/05-freshness-fingerprint-precedent.md` as prior evidence. Note that `.scratch/arch-design/` has open tickets touching incrementality and fingerprinting; record which guarantees those changes would stress (feeds ticket 09).

**Deliverable**: candidate-guarantee inventory in `.scratch/bdd-testing/research/03-index-note.md`, same entry format and citation convention as ticket 01. Distinguish persistence-level guarantees (durability, schema evolution, refresh-state transitions) from parsing-level guarantees (what a note's AST must contain for a given markdown). Flag behaviors guaranteed only by accident of redb/logos internals (refactor-hostile). Max ~400 lines.

## Answer

Findings: [../research/03-index-note.md](../research/03-index-note.md) (362 lines, source-cited; test counts re-verified exact: index 208, note 304).

- **Top pinned guarantees**: refresh-on-read, freshness = full `FileMeta` equality (both directions), path-set change forces link recompute, config drift heals on refresh, fingerprint composition (compile-fails on new settings), `parse_markdown` infallible, task/checkbox exact-match filter, `NoteFieldValue` total order, frozen postcard wire format.
- **Unpinned**: racy-timestamp blind spot (open, arch-design ticket 25 item 4).
- **Refactor-hostile set (12)**: exact redb error strings, hand-bumped `META_VERSION`, fingerprint `syn` source-scan, multimap secondary tables, whole-`Note` postcard shape, `traces::` facade exports, disk-is-source-of-truth; several map directly to open arch-design tickets.
- **Doc contradictions (7)**: slug/callout/footnote glossary specs with zero implementing code; ADR-0009 drift-vs-wipe mismatch; duplicate ADR number 0009 (ambiguous citations); ADR-0009 `proposed` while landed.
- Feeds tickets 07 (invariance classification), 09 (core input: which guarantees the planned refactor stresses).
