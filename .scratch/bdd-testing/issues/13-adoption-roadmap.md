# 13: Adoption roadmap

Type: grilling
Status: open
Blocked by: 09, 11, 12

## Question

In what order should BDD be adopted, and what is the concrete first slice? This
is the last decision ticket. Its answer feeds `spec.md` directly.

**Inputs**:

- tickets 06 (framework verdict and caveats), 07 (tiering rule), 08 (layer
  architecture), 09 (invariance list), 10 (organization and authoring
  conventions), 11 (quality/gap method), and 12 (migration rules)
- the five inventories (which domains carry the highest-value, least-pinned
  guarantees)

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) Write prioritized adoption slices. Each slice names its domain, the
  guarantees it pins, and its dependency on infrastructure. The first slice must
  bootstrap layout, fixtures, and phase wiring.
- (b) Write the pilot slice in enough detail that an implementing session starts
  cold. Name candidate domains, which existing tests it first interacts with,
  and its acceptance criteria.
- (c) Give the sequencing rationale: risk, value of guarantees pinned, and
  learning payback. Early slices should validate the framework verdict from 06
  in real conditions.
- (d) Write the explicit "not now" list: the domains deliberately deferred, with
  reasons. Candidates for the map's fog graduate into this list, not into more
  tickets.

**Answer records**: the ordered slice list, the pilot slice dossier, and the
not-now list. Resolution of this ticket is followed immediately by writing
`spec.md` (that writing is the map's destination act, not a new ticket).
