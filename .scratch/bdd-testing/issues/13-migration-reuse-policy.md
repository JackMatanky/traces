# 13: Migration and reuse policy

Type: grilling
Status: open
Blocked by: 08

## Question

The repo has about 2,469 tests today (2,368 inline unit, ~101 outer). What
happens to them when the new architecture lands? Which convert to scenarios,
which stay as-is, and which are redundant and die?

**Inputs**:

- ticket 08's classification rule and layer dispositions
- the five inventories' pinned/unpinned and Gherkin-suitability verdicts (they
  say which existing tests already encode guarantees and at what layer)
- `.scratch/integration-e2e-suite/spec.md` (its consolidation may already have
  opinions; read-only input per the map's scope ruling)

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) State the conversion rule: an existing test moves to a scenario only if it
  meets two conditions. It expresses a guarantee at a tier ticket 07 allows. Its
  layer changes under ticket 08's rule. State the rule first, then apply it.
  Never go ad hoc.
- (b) State the keep rule: unit tests for internal behavior remain unit tests.
  Do not translate for translation's sake.
- (c) State the deletion/dedup rule: say what makes an existing test redundant
  (a scenario proves the same guarantee at a higher boundary, etc.). Also state
  the burden of proof for deleting.
- (d) Give the coverage-preservation guarantee during migration: at no point
  does the suite prove less than before. Say how the session checks this.
- (e) Decide the scope of migration: migrate everything at once, or only tests
  touched by adoption waves? This informs ticket 14.

**Answer records**: the three rules (convert/keep/die), the
coverage-preservation check, and the migration scope decision.
