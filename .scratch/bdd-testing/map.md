# BDD adoption map

## Destination

An implementation-ready **BDD adoption and implementation strategy** for this
workspace, delivered as [`spec.md`](./spec.md): the behavioral domains and
contracts that earn executable specifications, the testing architecture they
live in (BDD vs unit/integration/e2e/property), the organization of feature
files/scenarios/steps/fixtures, a coverage-gap methodology, a migration policy
for existing tests, and a prioritized adoption roadmap naming a concrete first
slice. Every design decision the strategy rests on must be resolved in a linked
ticket. Implementing the strategy is out of scope: the map closes when a fresh
session could start slice 1 with zero open design questions.

## Notes

- **Domain**: Rust workspace `traces` (edition 2024). Framework: `rstest-bdd`.
  Ticket [06: Framework re-validation](issues/06-framework-revalidation.md)
  validated it, with caveats.
- **Skills every session consults**: `research`, `grilling`, `domain-modeling`,
  `prototype`, per ticket type. Research and prototype tickets are AFK; grilling
  tickets are HITL and never answer their own questions.
- **Standing preferences for this effort**: nextest compatibility is decisive
  (all four test phases run under cargo-nextest). Specs define *what the system
  guarantees*, never *how it works*. Use idiomatic Rust and minimal abstraction.
  Build no custom infra beyond adopting an existing crate. Deny-lints
  (`panic`/`exit`/`print_stdout`/`unwrap` outside tests) and machine-enforced
  layer gates (`check:layers`, `check:visibility`) are constraints. Do not route
  around them. A bounded exception needs a recorded decision (see ticket 06).
- **Maintenance duty**: ticket 06 sets one
  `#[expect(clippy::disallowed_methods)]` on each `mod bdd` declaration. Revisit
  it at every `rstest-bdd` upgrade. The `#[expect]` fails when the lint stops
  appearing. Nobody has filed the upstream fix yet.
- **Standing evidence, source-cited, do not re-do**:
  [research/cucumber-rs.md](research/cucumber-rs.md),
  [research/rstest-bdd.md](research/rstest-bdd.md),
  [research/rust-bdd-crate-comparison.md](research/rust-bdd-crate-comparison.md).
- **Other evidence sources**: `GLOSSARY-MAP.md` plus per-module `GLOSSARY.md`
  files (11 contexts), `docs/adr/` (10 ADRs, including duplicate-numbered
  0009s), the existing test suite's `//! Proves` and `Defect class:`
  conventions, `.mise/tasks/test/` phase definitions. The file
  `docs/refs/cucumber_bdd_rust.md` no longer exists (commit `fa95f85f`).
- **Research findings convention**: findings land in
  `.scratch/bdd-testing/research/` alongside this map, one file per research
  ticket. No throwaway branches: concurrent agents share one worktree; this
  matches the `arch-design` precedent.
- **Coordination**: [`.scratch/arch-design/`](../arch-design/map.md) is the
  named refactor stress case (ticket 09).
  [`.scratch/integration-e2e-suite/`](../integration-e2e-suite/spec.md) is
  read-only input; its spec and audit inform this map and its execution is out
  of scope.

## Decisions so far

<!-- one line per closed ticket: [title](link): gist -->

- [01: Surface workflows, behavioral discovery](issues/01-surface-workflows.md):
  exit/trust/config/dialog contracts mostly pinned; gaps = 130-e2e and "init
  does not grant trust"; ADR-0002/0003 contradictions; TTY/env hostile to
  Gherkin. Findings: [research/01](research/01-surface-workflows.md).
- [02: Query and template pipeline, behavioral
  discovery](issues/02-query-template.md): language surface all pinned (prime
  scenario candidates); plan/memoization internals weak and refactor-sensitive;
  ADR-0008 SourceAtom contradiction confirmed. Findings:
  [research/02](research/02-query-template.md).
- [03: Index and note substrate, behavioral discovery](issues/03-index-note.md):
  refresh/freshness/fingerprint guarantees pinned; racy-timestamp gap unpinned;
  12 refactor-hostile guarantees map onto open arch-design tickets; 7 doc
  contradictions incl. duplicate ADR 0009. Findings:
  [research/03](research/03-index-note.md).
- [04: Schema and temporal, behavioral discovery](issues/04-schema-temporal.md):
  schema isolation/error contracts and DST/calendar rules pinned; cross-domain
  boundaries pinned 2-3 layers; proptests already own algebraic laws; "zero
  outer coverage" premise corrected. Findings:
  [research/04](research/04-schema-temporal.md).
- [05: Cross-cutting contracts, behavioral
  discovery](issues/05-cross-cutting-contracts.md): `test-utils` = 47 not 46 (32
  guarantees / 15 conveniences); exit/stream/layer contracts pinned; `Defect
  class:` taxonomy undocumented and drifting; ADR-0004 `Diagnostic`
  contradiction. Findings:
  [research/05](research/05-cross-cutting-contracts.md).
- [06: Framework re-validation](issues/06-framework-revalidation.md): adopt
  `rstest-bdd` =0.6.0 with caveats. Do not change `clippy.toml`. Put one
  `#[expect(clippy::disallowed_methods)]` on each `mod bdd` declaration, because
  the step wrappers call `catch_unwind`. Bare `{name}` placeholders avoid a
  lint. A `build.rs` for new feature files costs a package rebuild (ticket 10
  decides). You can fold scenarios into the existing integration and e2e
  binaries (ticket 08 decides).

## Not yet specified

- BDD and quality gates beyond layer placement: mutation testing (`mutarust`),
  coverage (`llvm-cov nextest`).
- Prototype in the repo of the 06 caveats that are only inferred:
  `check:layers`, the e2e binary without `test-utils`, Linux and Windows CI, and
  the rebuild cost of edits under a `build.rs`. This item gets sharper after
  ticket 08 fixes where scenarios run.
- Fate of the existing `//! Proves` and `Defect class:` documentation style:
  absorbed into Gherkin, kept alongside, or replaced.
- Scenario density and verbosity norms (how much behavior per scenario, when a
  Scenario Outline beats a rule).
- Fit (or non-fit) of Gherkin for `dialog` interactive and TTY behavior; may
  sharpen once ticket 01 reports.

## Out of scope

- Executing the pilot slice or any test migration; that is the post-map handoff.
- Work belonging to `.scratch/integration-e2e-suite/`; read-only input here, its
  tickets are not this map's route.
- Building custom framework or infrastructure beyond adopting an existing crate.
