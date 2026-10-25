# 12: Migration and reuse policy

Type: grilling
Status: open
Blocked by: 08

## Question

What happens to the existing ~2,469 tests (2,368 inline unit, ~101 outer) when the new architecture lands: which convert to scenarios, which stay as-is, which are redundant and die?

**Inputs**: ticket 08's classification rule and layer dispositions, the five inventories' pinned/unpinned and Gherkin-suitability verdicts (they say which existing tests already encode guarantees and at what layer), `.scratch/integration-e2e-suite/spec.md` (its consolidation may already have opinions; read-only input per the map's scope ruling).

**Decisions to reach** (HITL, grilling): (a) a conversion rule: an existing test moves to a scenario only if it expresses a guarantee at a tier ticket 07 allows and its layer changes under ticket 08's rule (state the rule, then apply it, never ad hoc); (b) a keep rule (unit tests for internal behavior remain unit tests; do not translate for translation's sake); (c) a deletion/dedup rule: what makes an existing test redundant (scenario proves the same guarantee at a higher boundary, etc.) and the burden of proof for deleting; (d) coverage-preservation guarantee during migration: at no point does the suite prove less than before (how this is verified); (e) scope of migration: does the strategy migrate everything at once or only tests touched by adoption waves (informs ticket 13).

**Answer records**: the three rules (convert/keep/die), the coverage-preservation check, and the migration scope decision.
