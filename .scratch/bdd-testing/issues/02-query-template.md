# 02: Query and template pipeline, behavioral discovery

Type: research
Status: resolved

## Question

Which observable behaviors of the query and template pipeline constitute
*intentional guarantees*, as opposed to incidental implementation behavior?

**Scope**: `src/query/` (source-selection DSL, grammar and filter semantics,
planning, memoized result rows, error semantics for malformed queries),
`src/template/` (template resolution order, minijinja namespace functions,
confined file writing, render errors).

**Method**: read source and inline tests breadth-first (the two heaviest
modules: 317 and 362 `#[test]`s). Read `tests/integration/query.rs`,
`tests/integration/template_render.rs`, `tests/e2e/query.rs`,
`tests/e2e/template.rs`, `src/query/GLOSSARY.md`, `src/template/GLOSSARY.md`,
`docs/adr/0005`, `docs/adr/0008`, `docs/adr/0001`. Question which DSL/render
behaviors are specified guarantees (documented, glossary'd, ADR'd) versus
whatever the parser happens to do. Flag contradictions between docs and code.

**Deliverable**: candidate-guarantee inventory in
`.scratch/bdd-testing/research/02-query-template.md`. Use the same entry format
and citation convention as ticket 01: guarantee as observable contract, class,
intentional/incidental/uncertain with evidence, pinned/unpinned,
Gherkin-suitability. Note especially which query semantics are *user-visible
language surface* (strong scenario candidates), versus internal plan
optimizations (refactor-sensitive, poor scenario candidates). Max ~400 lines.

## Answer

Findings: [../research/02-query-template.md](../research/02-query-template.md)
(392 lines, source-cited; spot-verified: `SourceAtom` contradiction confirmed at
`src/query/grammar/source.rs:190` vs `docs/adr/0008:77`).

- **User-visible language surface** (all pinned, prime scenario candidates):
  unknown-class degrade, output precedence + root confinement,
  `--dry-run`/`--force`, stable diagnostic codes, filter semantics (date
  literals, left-to-right arithmetic, null ordering), sort defaults, terminal
  renderer parity.
- **Internal/plan-sensitive** (weak scenario candidates): plan fusion/TopK
  (equivalence only), memoization O(1) (unpinned), row budget, store-vs-memory
  agreement.
- **Doc contradictions**: ADR-0008 names nonexistent `Folder`/`Glob` SourceAtom
  variants (code: `Tag | Path | Class`); ADR-0005 promises a nonexistent
  terminal-filter error; ADR-0001 stale line cites; ADR-0009 `proposed` while
  fully wired; GLOSSARY resolution-order ambiguity.
- **Gherkin-hostile**: perf claims unobservable, plan internals
  refactor-sensitive, miette wrapping, injected-timezone requirement,
  platform-specific paths.
- Feeds tickets 07 (language surface vs internals), 09 (plan/memoization
  refactor-sensitive), 11 (unpinned = gap candidates).
