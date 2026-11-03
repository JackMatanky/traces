# 10: Refactor resilience against arch-design

Type: grilling
Status: resolved
Blocked by: 01, 02, 03, 04, 05, 07, 08

## Question

Which guarantees can scenarios pin without anchoring to a seam that
`.scratch/arch-design/` plans to rebuild?

## Scope

- BDD implementation comes first. Scenarios pin current behavior.
- Arch-design runs later. It finds and edits the affected scenarios and tests.
- This ticket gives arch-design no bookkeeping. It adds no tag, no row id, no
  revisit marker and no note in the arch map.
- The ticket only keeps scenarios off moving seams, so that they stay stable.

## Inputs

- [Arch ticket 16](../../arch-design/issues/16-grilling-compatibility-envelope.md).
  It sorts each surface into Frozen, Migratable, Free or Not preserved.
- The five inventories from tickets 01 to 05: `research/01` to `research/05`.
- Ticket 07: the six-condition rule. Condition 6 lists the moving seams. This
  ticket owns that list.
- Ticket 08: the six-step classification rule and its constraints for this
  ticket.
- `research/03` section C (items 1 to 12) and `research/02` G14 to G18. Both
  are the provisional list of moving seams.

## Decisions to reach

HITL, grilling. Do not answer for the user. Reach these decisions:

- (a) Write the invariance list. It holds the Frozen and Migratable guarantees
  that pass ticket 07 conditions 1 and 2. It also holds the guarantees that arch
  ticket 16 does not list. Each entry
  cites its inventory item and its arch ticket 16 tier. Map the other guarantees
  at class level.
- (b) Say that a scenario never pins a known defect. Arch ticket 16 names three:
  the closed-pipe panic, `i64` to `f64` loss, and `.first()`-only multi-valued
  inline fields.
- (c) Refine ticket 07's moving-seam list. For each seam, choose one
  disposition: a scenario at the higher boundary that the seam feeds, or no
  scenario.
- (d) Confirm or change the proptest growth list from ticket 08: path codec, note
  wire format, `NoteFieldValue` order, table escaping and tag containment.
- (e) Confirm the facade seams from ticket 08: `RefreshReport`, `DriftCause`,
  `InlinkMap` and `load()`.

## Boundaries

- Do not resolve a doc/code contradiction here. Ticket 12 decision (f) owns each
  disposition. Flag each contradiction that touches the invariance list.
- Do not write scenarios. Do not choose the layout of feature files. Ticket 11
  owns both.
- Do not decide which existing tests convert. Ticket 13 owns that decision.
- Do not decide the migration scope. Tickets 13 and 14 own it.

## Answer records

- The invariance list, with inventory citations and arch ticket 16 tiers.
- The seam dispositions.
- The confirmed proptest list and facade seams.
- Constraint for ticket 14: the invariance list sets the order of the first
  slices.

## Answer

Resolved. The user confirmed this text.

The full lists are in [research/10-invariance-map.md](../research/10-invariance-map.md).

- **(a) Invariance list.** The map has 88 rows for guarantees that pass ticket 07
  conditions 1 and 2. Three rows repeat items across inventories. Each row has
  its inventory citation and its arch ticket 16 tier: F, M, F+M or D. D means
  that arch ticket 16 does not list the surface. Treat D as Frozen. The map
  holds the other guarantees at class level.
- **(b) Known defects.** A scenario never pins the closed-pipe panic, the `i64`
  to `f64` loss, or the `.first()`-only multi-valued inline fields.
- **(c) Seam dispositions.** The map gives one disposition for each of the 12
  seams in `research/03` section C and for G14 to G18. Each is a scenario at the
  higher boundary, or no scenario.
- **(d) Proptests.** Add table escaping, tag containment and `NoteFieldValue`
  order now. Keep path codec and note wire format on the list. Do not add the
  schema DAG sort. Query-plan idempotence stays excluded.
- **(e) Facade seams.** `RefreshReport`, `DriftCause`, `InlinkMap` and `load()`
  get no scenario. They stay integration-test seams.
- **Flagged contradictions.** Five contradictions touch the list. Ticket 12
  decision (f) owns them. They are in the map.
- **Constraint for ticket 14.** The invariance list sets the order of the first
  slices.
