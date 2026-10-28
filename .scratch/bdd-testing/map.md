# BDD adoption map

## Destination

An implementation-ready **BDD adoption and implementation strategy** for this
workspace, delivered as [`spec.md`](./spec.md). The strategy contains:

- the behavioral domains and contracts that earn executable specifications
- the testing architecture that they live in (BDD vs
  unit/integration/e2e/property)
- the organization of feature files, scenarios, steps and fixtures
- a coverage-gap methodology
- a migration policy for existing tests
- a prioritized adoption roadmap that names a concrete first slice

Resolve every design decision that the strategy needs in a linked ticket. The
map does not cover the implementation of the strategy. The map closes when a
fresh session can start slice 1 with zero open design questions.

## Notes

- **Domain**: Rust workspace `traces` (edition 2024). Framework: `rstest-bdd`.
  Ticket [06: Framework re-validation](issues/06-framework-revalidation.md)
  validated it, with caveats.
- **Skills every session consults**: `research`, `grilling`, `domain-modeling`,
  `prototype`, per ticket type. Research and prototype tickets are AFK; grilling
  tickets are HITL and never answer their own questions.
- **Standing preferences for this effort**:
  - Nextest compatibility is decisive. All four test phases run under
    cargo-nextest.
  - Specs define *what the system guarantees*, never *how it works*.
  - Use idiomatic Rust and minimal abstraction.
  - Build no custom infrastructure beyond adopting an existing crate.
  - Obey the deny-lints (`panic`, `exit`, `print_stdout` and `unwrap` outside
    tests).
  - Obey the machine-enforced layer gates (`check:layers`, `check:visibility`).
    Do not route around them.
  - A bounded exception needs a recorded decision (see ticket 06).
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
  named refactor stress case (ticket 10).
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
  lint. A `build.rs` for new feature files costs a package rebuild (ticket 11
  decides). Ticket 08 decided: fold scenarios into the e2e binary only.
- [07: Boundary tiering, b versus c](issues/07-boundary-tiering.md): option (b)
  wins, with a sharper boundary. Scenarios address the spawned process by
  default. A facade seam is allowed only if a frozen baseline already holds the
  symbol. A spawn must not be able to express the behavior. A six-part rule
  decides which guarantees earn a scenario:
  - observable behavior
  - named channels
  - intent evidence with code agreement
  - no gate or proptest owner
  - hermetic
  - refactor-stable

  For internal seams, promote first. Then keep a unit test that mutation testing
  checks (`mise run test:mutants`). Then retire the pin. Wrapping in
  `test-utils` is forbidden by default. The gate: BDD adoption changes none of
  the three baselines. Each new shipped entry point of the planned workspace
  (core, cli, lsp, mcp) is a new boundary and reopens the rule. Evidence:
  [research/07](research/07-prior-art.md).
- [08: Layer responsibilities](issues/08-layer-responsibilities.md): a six-step
  classification rule (gate, proptest, scenario, e2e, integration, unit).
  Scenarios fold into the e2e binary only. Prototype
  [09](issues/09-bdd-e2e-prototype.md) must confirm this. `check:layers` must
  fail when `git grep` exits with code 2 or higher. Proptests stay inline;
  growth waits for ticket 10. The `integration-e2e-suite` is partially adopted:
  its invariants and gates bind, its per-test classification does not. The
  audits feed ticket 13. Ticket 08 sets six anti-goals. Evidence:
  [research/08-prior-art](research/08-prior-art.md),
  [audits](research/08-audit-e2e.md).
- [09: Prototype BDD inside the e2e binary](issues/09-bdd-e2e-prototype.md): the
  pilot works in `tests/e2e.rs` on macOS with no runner edits. Linux and Windows
  CI stay unverified. Reject `build.rs`: it caused a package rebuild on each
  feature edit. Document `touch` for new feature files. Adoption must add
  `"Zlib"` to `deny.toml`. `check:layers` is not proved: untracked spike files
  were not scanned, and `|| true` hides `git grep` errors. Findings:
  [research/09](research/09-prototype.md).
- [10: Refactor resilience against
  arch-design](issues/10-refactor-resilience.md):
  [research/10](research/10-invariance-map.md) lists 88 guarantees that pass
  conditions 1 and 2 of ticket 07, each with its arch ticket 16 tier. Scenarios
  pin current behavior. Arch-design edits them later, so this map carries no
  arch bookkeeping. A scenario never pins a known defect. Each moving seam gets
  a scenario at the higher boundary, or none. Add table escaping, tag
  containment and `NoteFieldValue` order as proptests. The invariance list sets
  the order of the first slices.
- [11: Feature and step organization](issues/11-feature-step-organization.md):
  features in flat `tests/features/<capability>.feature`.
  - Steps live in `tests/e2e/bdd/mod.rs` and domain-concept files, never one
    file per feature.
  - Use `#[path = "e2e/bdd/mod.rs"]`. The `bdd.rs` form picks up sibling e2e
    files.
  - Use bare `{name}` placeholders with quotes in the pattern.
  - Use two plain fixtures, `Sandbox` and `LastRun`, with no World.
  - Prefer 3 to 5 steps.
  - Ban `Rule:`, `skip!()` and `@ignore`, each with a gate.
  - `check:layers` scans Rust paths for Rust symbols. It does not scan feature
    text, which narrows ticket 08's rule.

  Findings: [research/11](research/11-conventions.md).

## Not yet specified

- Policy for BDD and quality gates beyond layer placement. The tools exist:
  mutarust (`mise run test:mutants`) and `llvm-cov nextest`. Ticket 12 sets the
  thresholds and the scope.
- Linux and Windows CI result for the e2e placement. Only the first adoption
  slice can check it.

## Out of scope

- Executing the pilot slice or any test migration; that is the post-map handoff.
- Work belonging to `.scratch/integration-e2e-suite/`; read-only input here, its
  tickets are not this map's route.
- Building custom framework or infrastructure beyond adopting an existing crate.
