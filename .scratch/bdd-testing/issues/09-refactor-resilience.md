# 09: Refactor resilience against arch-design

Type: grilling
Status: open
Blocked by: 02, 03, 07

## Question

Which behavioral guarantees must the future test architecture hold invariant across the refactor(s) planned in `.scratch/arch-design/`, and what does that imply for where scenarios may anchor?

**Inputs**: [`.scratch/arch-design/map.md`](../../arch-design/map.md) and its open tickets (15 at charting time; query-engine predicate/plan structure, incrementality/fingerprinting especially), its decisions-so-far about behavior preservation, tickets 02 and 03 inventories (query/index guarantees), ticket 07's tiering rule.

**Decisions to reach** (HITL, grilling): (a) a mapping from arch-design's behavior-preserving claims to concrete guarantees in the inventories: which ones does the planned refactor promise to preserve, and which are currently unstated and therefore at risk; (b) equivalence boundaries: the observable contracts that must remain invariant across the refactor (these are the highest-value scenario targets); (c) guarantees that anchor to structures the refactor will move: reclassify them (higher boundary, different layer, or drop from the spec scope) under ticket 07's rule; (d) any coordination note arch-design's map should carry back (record it in the Answer; writing to their map is allowed as a one-line cross-reference, not a rewrite).

**Answer records**: the invariance list (must-hold guarantees with inventory citations), the reclassified guarantees, and the reciprocity note for arch-design.
