# 11: Coverage-gap methodology

Type: grilling
Status: open
Blocked by: 08

## Question

How do you find gaps in behavioral coverage and judge the quality of specifications? Do not chase vanity metrics (test counts, line coverage as a goal).

**Inputs**:

- ticket 08's layer classification
- the five inventories' pinned/unpinned flags (unpinned guarantees are gap candidates)
- existing measurement machinery (`llvm-cov nextest`, `mutarust` mutation testing, `docs/refs/mutation_testing.md`, `docs/refs/quality_gates_*`)
- the map's fog item on quality gates

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) Define what "covered" means per layer: a guarantee has an enforcing test of the right kind, not just an exercise.
- (b) Choose a practical gap-detection method a session can run: review cadence, mapping guarantees to tests, and tooling if any. Keep it lightweight.
- (c) State the scenario-quality criteria: what makes a specification good (precise observable contract, refactoring resilience, readable by a domain speaker, failure attribution). Say how review judges quality.
- (d) Name the explicit anti-metrics: the numbers this effort refuses to optimize (scenario count, coverage percentage targets) and why.
- (e) Decide the disposition of mutation testing and coverage tooling relative to scenarios: unchanged, extended, or explicitly out.

**Answer records**: the coverage definition, the gap-detection procedure, the quality rubric, and the anti-metrics list.
