# 09: Refactor resilience against arch-design

Type: grilling
Status: open
Blocked by: 02, 03, 07

## Question

Which behavioral guarantees must the future test architecture hold invariant across the refactor(s) planned in `.scratch/arch-design/`? What does that imply for where scenarios may anchor?

**Inputs**:

- [`.scratch/arch-design/map.md`](../../arch-design/map.md) and its open tickets (15 at charting time; query-engine predicate/plan structure, incrementality/fingerprinting especially)
- its decisions-so-far about behavior preservation
- tickets 02 and 03 inventories (query/index guarantees)
- ticket 07's tiering rule

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) Map arch-design's behavior-preserving claims to concrete guarantees in the inventories. Say which guarantees the planned refactor promises to preserve. Say which are currently unstated and therefore at risk.
- (b) Define the equivalence boundaries: the observable contracts that must remain invariant across the refactor. These are the highest-value scenario targets.
- (c) Handle guarantees that anchor to structures the refactor will move. Reclassify them under ticket 07's rule: higher boundary, different layer, or drop from the spec scope.
- (d) Write any coordination note that arch-design's map should carry back. Record the note in the Answer. Writing to their map is allowed as a one-line cross-reference, not a rewrite.

**Answer records**: the invariance list (must-hold guarantees with inventory citations), the reclassified guarantees, and the reciprocity note for arch-design.
