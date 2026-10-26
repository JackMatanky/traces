# 11: Coverage-gap methodology

Type: grilling
Status: open
Blocked by: 08

## Question

How do you find gaps in behavioral coverage and judge the quality of
specifications? Do not chase vanity metrics (test counts, line coverage as a
goal).

**Inputs**:

- ticket 08's layer classification
- the five inventories' pinned/unpinned flags (unpinned guarantees are gap
  candidates)
- existing measurement machinery (`llvm-cov nextest`, `mutarust` mutation
  testing, `docs/refs/mutation_testing.md`, `docs/refs/quality_gates_*`)
- the map's fog item on quality gates
- the doc/code contradictions that the five inventories list: research/01
  section 2, research/02 section 3, research/03 section D and research/05
  section 5. Rule 3 of ticket 07 needs a recorded resolution for each
  contradiction before a scenario can use the guarantee.

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) Define what "covered" means per layer: a guarantee has an enforcing test
  of the right kind, not just an exercise.
- (b) Choose a practical gap-detection method a session can run: review cadence,
  mapping guarantees to tests, and tooling if any. Keep it lightweight.
- (c) State the scenario-quality criteria: what makes a specification good
  (precise observable contract, refactoring resilience, readable by a domain
  speaker, failure attribution). Say how review judges quality.
- (d) Name the explicit anti-metrics: the numbers this effort refuses to
  optimize (scenario count, coverage percentage targets) and why.
- (e) Decide the disposition of mutation testing and coverage tooling relative
  to scenarios: unchanged, extended, or explicitly out.
- (f) Triage the contradictions. For each contradiction that touches a scenario
  candidate, record one disposition: the doc wins, the code wins, or drop the
  guarantee. Start with these cases:
  - ADR-0002: a moved directory is "stale" in the ADR, but the code reports it
    as untrusted.
  - ADR-0008: the ADR names `SourceAtom` variants that do not exist.
  - The glossary claims about slugs, callouts and footnotes. No code
    implements them. Drop them, do not specify them.

  Ignore contradictions that touch no scenario candidate, such as stale line
  citations.

**Answer records**: the coverage definition, the gap-detection procedure, the
quality rubric, the anti-metrics list, and the contradiction dispositions.
