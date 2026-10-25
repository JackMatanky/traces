# 04: Schema and temporal, behavioral discovery

Type: research
Status: resolved

## Question

Which observable behaviors of the schema and temporal domains constitute
*intentional guarantees*, as opposed to incidental implementation behavior?

**Scope**: `src/schema/` (File-Class schema registry, field inheritance DAG,
class graph, validation and conflict resolution), `src/temporal/` (date,
datetime, duration, point parsing; zone and calendar arithmetic).

**Method**: read source and inline tests breadth-first (311 and 176 `#[test]`s,
plus the repo's only 4 `proptest!` blocks in `temporal/`). Read
`src/schema/GLOSSARY.md`, `src/temporal/GLOSSARY.md`, `docs/adr/0006`,
`docs/adr/0007`, `docs/adr/0009-filter-date-literals`, and the
`.scratch/temporal-core/` spec and divergence-register as prior evidence. Note:
neither domain is referenced from `tests/` today (zero outer-layer coverage), so
the discovery must judge from source, inline tests, and docs alone. Also record
which temporal guarantees are already well covered by property tests and
therefore may not need Gherkin.

**Deliverable**: candidate-guarantee inventory in
`.scratch/bdd-testing/research/04-schema-temporal.md`, same entry format and
citation convention as ticket 01. Highlight cross-domain contracts (schema
values that flow into query/template; temporal literals inside filters) since
those boundary guarantees are prime scenario candidates. Max ~400 lines.

## Answer

Findings: [../research/04-schema-temporal.md](../research/04-schema-temporal.md)
(262 lines, source-cited).

- **High-value pinned guarantees**: schema resolve-once/`Arc` sharing,
  per-schema failure isolation, structural-ref hard-error vs predicate-ref
  degrade, clock doctrine + DST fold/gap rules, dual duration regime (nominal vs
  calendar), one-step compound date shifts, calendar-diff counts local
  boundaries, unquoted ISO literal parity (ADR-0009 filter literals).
- **Cross-domain boundaries pinned at 2-3 layers**: `@book`/`@book+`/`@book*`
  expansion (`file_class_expander.rs`, integration, e2e); `class_field` config
  binding; `schema.get` hard-error contract; shared `TextShape::classify` in
  filter+sort; one-calendar-owner `dur()` vs `date_add` agreement.
- **Unpinned**: `required`/`multi` inert-declaration claim;
  `UNIT_SPELLINGS_HINT` no-drift claim.
- **Correction to ticket premise**: schema/temporal have no `use traces::schema`
  outer tests, but >=10 seam tests pin boundary behavior (ticket premise "zero
  outer-layer coverage" was too strong).
- **Proptest verdict**: the 4 blocks cover algebraic round-trip laws; scenarios
  still add value for DST fixtures, grammar reject tables, and error-message
  contracts.
- Feeds tickets 07 (boundary guarantees as scenario candidates), 04-of-map fog
  (property-test layer role).
