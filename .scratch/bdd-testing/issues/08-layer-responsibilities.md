# 08: Layer responsibilities

Type: grilling
Status: resolved
Blocked by: 06, 07

## Question

What are the responsibilities and boundaries of each test layer in this repo
after BDD arrives? The layers are:

- behavioral scenarios (rstest-bdd)
- unit tests (`src/` inline)
- integration (`tests/integration/`)
- e2e (`tests/e2e/`)
- property-based tests (`proptest`, currently 4 blocks in `temporal/`)

**Inputs**:

- ticket 06's framework verdict and constraints
- ticket 07's tiering rule
- the five discovery inventories (what each guarantee's natural layer is)
- `.scratch/integration-e2e-suite/spec.md` and `test-arch-audit.md`. Read-only:
  use them to learn what that effort already decided about the integration/e2e
  split.
- `mise.toml` gates (`check:layers`, `check:visibility`) and `.mise/tasks/test/`
  phase wiring

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) Write a classification rule: which class of behavior belongs to which
  layer. A future test author must be able to apply the rule without asking.
- (b) Decide where BDD scenarios physically run: a new nextest phase, folded
  into integration, or the `harness` arrangement per ticket 06.
- (c) Decide the role and expected growth of property tests relative to
  scenarios.
- (d) State what the existing `integration`/`e2e` split becomes: unchanged,
  absorbed, or redefined. Also state where `.scratch/integration-e2e-suite/`'s
  decisions stand as input: accepted, superseded, or partially adopted, with
  reasons.
- (e) Name the anti-goals: the layers this repo will *not* introduce (for
  example, no mock layer, no contract-test framework) and why.

**Answer records**: the classification rule, the phase/gate wiring decision, and
the disposition of integration-e2e-suite's spec.

## Answer

The user and the agent resolved this ticket together. Evidence:
[research/08-prior-art.md](../research/08-prior-art.md),
[research/08-integration-e2e-suite-disposition.md](../research/08-integration-e2e-suite-disposition.md),
[research/08-natural-layers.md](../research/08-natural-layers.md),
[research/08-audit-e2e.md](../research/08-audit-e2e.md),
[research/08-audit-integration.md](../research/08-audit-integration.md),
[research/08-verify-suite-decisions.md](../research/08-verify-suite-decisions.md).
The audits are agent judgments from static reading. Spot checks confirmed two
defects. Use the rest as input to ticket 12. Do not treat it as fact.

### Decision (a): classification rule

Ask these questions in this order. One guarantee can match more than one.

1. A compile-time, visibility or layer rule goes to a **gate**.
2. A law, round-trip, idempotence or total order goes to a **proptest**. The
   proptest owns the law. A scenario may add 5 to 10 user-visible key examples.
3. A spawned process shows it on stdout, a stderr code, the exit status or files
   under the project root. It also passes ticket 07's six conditions. It becomes
   a **scenario**. An ADR, glossary entry, doc claim or workflow comment states
   the rule.
4. A process-boundary mechanism that no stakeholder would state goes to **e2e**.
   Examples: env policy, EPIPE/101, completions, signals.
5. A guarantee that needs multi-module in-process composition or fault injection
   goes to **integration**. Examples: cold-redb parity, corruption recovery.
6. Everything else goes to a **unit test**. Mutation testing guards it only
   after ticket 11 sets thresholds.

### Decision (b): placement

- Fold scenarios into the `e2e` binary only:
  `#[expect(clippy::disallowed_methods)] mod bdd;` in `tests/e2e.rs`.
- Defer a facade-driver binary until a guarantee passes ticket 07's facade test.
  A dedicated `bdd` binary is the fallback.
- Three conditions apply:
  - The decision binds only after prototype ticket 14 passes `mise run verify`
    and a three-OS CI run.
  - `check:layers` must fail when `git grep` exits with code 2 or higher.
  - `check:layers` must scan every path that holds feature files and steps.

### Decision (c): property tests

- Proptests stay inline in `src/`. Commit `proptest-regressions/` files. No CI
  rule for them.
- Growth candidates wait for ticket 09: path codec, note wire format,
  `NoteFieldValue` order, table escaping, tag containment.
- Query-plan idempotence is excluded (arch ticket 26 can rewrite it).

### Decision (d): integration and e2e suite

- The split stays: integration is in-process, e2e is spawn-only.
- `.scratch/integration-e2e-suite/` is **partially adopted**.
  - Binding: layer invariants, hermetic sandbox, baselines and gates, four-phase
    runner, ledger method (its issues 15 and 16).
  - Not binding: its per-test classification and its "implemented" claims.
- Ticket 12 builds a fresh ledger from the audits. The audits found:
  - 5 misplaced e2e unit tests and 10 misplaced integration unit tests
  - 6 duplicates and 7 weak assertions
  - 1 test on a condemned seam
  - 27 e2e tests that are scenario candidates

  Ticket 12 decides whether the 27 candidates convert.
- Hygiene fixes outside this map: the stale "46 identifiers" doc at
  `src/lib.rs:182` (the baseline has 47), and `check:layers` failing silently
  (it was inactive from `7b71ed06` to `938c0748` because a deleted pathspec made
  `git grep` exit 128).

### Decision (e): anti-goals

- No mock or stub framework.
- No PTY harness.
- No `harness = false` runner.
- No combinatorial Scenario Outline suites.
- No contract-test framework.
- No test layer without a gate or ledger that stops it from going silently dead.

Assert diagnostic codes. Never assert miette-wrapped text. This is ticket 07's
channel rule. It is not a separate anti-goal.

### Constraints for other tickets

- Ticket 09: confirm the proptest growth list and the facade seams.
- Ticket 10: wait for ticket 14. Keep `.feature` text free of the `check:layers`
  patterns.
- Ticket 11: set mutation thresholds. Unit tests are guarded only after that.
- Ticket 12: use the audits and the 27 candidates as ledger input.
