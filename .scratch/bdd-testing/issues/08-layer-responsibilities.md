# 08: Layer responsibilities

Type: grilling
Status: open
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
