# 10: Refactor resilience against arch-design

Type: grilling
Status: open
Blocked by: 01, 02, 03, 04, 05, 07, 08

## Question

Which behavioral guarantees must stay invariant across the refactors that
`.scratch/arch-design/` plans? Where can scenarios anchor as a result?

## State of arch-design

Arch-design is still moving. Do not wait for it to finish.

- Arch tickets 01 to 22 are resolved. Their decisions bind this ticket.
- Arch tickets 23 to 36 are open. They can still move seams.
- For each seam that an open arch ticket can move, record that ticket.
- For each such seam, record a trigger to revisit the decision.

## Inputs

- [`.scratch/arch-design/map.md`](../../arch-design/map.md) and its tickets.
  The map lists decisions for tickets 01 to 22.
- [Arch ticket 16](../../arch-design/issues/16-grilling-compatibility-envelope.md).
  It sorts each surface into Frozen, Migratable, Free or Not preserved. Use its
  table as the oracle for decision (a).
- Arch tickets 18 to 22. They change behavior on purpose. Read each ticket
  itself. Do not rely on the map gist.
- The five inventories from tickets 01 to 05: `research/01` to `research/05`.
  Use their pinned flags, their intent verdicts and their contradiction lists.
- Ticket 07: the six-condition rule, the exception classes and the internal-seam
  policy. Rule 6 lists the moving seams. This ticket owns that list.
- Ticket 08: the six-step classification rule and its constraints for this
  ticket. Decision (c) lists the proptest growth candidates.
- `research/03` section C (items 1 to 12) and `research/02` G14 to G18. Both
  are the provisional list of moving seams.
- Run `git diff 207cc3fb..main -- src tests docs` before you start. Arch-design
  recorded that baseline. Report any drift that touches a guarantee here.

## Decisions to reach

HITL, grilling. Do not answer for the user. Reach these decisions:

- (a) Map every guarantee in the five inventories to a tier of arch ticket 16.
  Say which guarantees the refactor promises to keep. Say which guarantees no
  arch ticket states. These guarantees are at risk.
- (b) Define the equivalence boundaries. These are the Frozen contracts that
  ticket 07's channels can observe. They are the highest-value scenario targets.
  Examples: exit codes, payload shapes, trust gate, vault parsing, index
  equivalence with a fresh parse.
- (c) Decide how scenarios hold Migratable surfaces. Examples: diagnostic codes,
  flag spellings, template and query spelling. Arch ticket 16 allows a change
  through the spec's change table. Ticket 07 makes diagnostic codes a channel.
  State whether such a scenario is allowed. State the tag that marks it. Ticket
  11 uses this tag.
- (d) List each resolved arch decision that changes behavior on purpose. Examples
  are tickets 18, 19, 20, 21 and 22, and the three known defects. Name each
  scenario candidate or pinned guarantee that the change inverts or removes.
  Record the open ticket 25 item 4 canary inversion.
- (e) Refine ticket 07's moving-seam list. For each seam, name the arch ticket
  that moves it. Then choose one disposition under ticket 07's rule: a higher
  boundary, a different layer under ticket 08's rule, or drop from the spec.
- (f) Settle the constraints from ticket 08 and the workspace split:
  - Confirm or change the proptest growth list: path codec, note wire format,
    `NoteFieldValue` order, table escaping and tag containment.
  - Confirm or change the facade seams: `RefreshReport`, `DriftCause`,
    `InlinkMap` and `load()`. Arch tickets 25 and 32 can re-home them.
  - State what the workspace split does to ticket 07's gate. The gate applies to
    each crate after the split.
- (g) Write the note that arch-design's map must carry back. Add it to their
  map as a one-line cross-reference, not a rewrite. Record the note in the
  Answer. The note names the owner who checks the invariance list when an arch
  ticket resolves.

## Boundaries

- Do not resolve a doc/code contradiction here. Ticket 12 decision (f) owns each
  disposition.
- Flag each contradiction that touches an invariance guarantee. Examples:
  ADR-0002 trust, ADR-0008 `SourceAtom`, and ADR-0009 format version.
- Do not write scenarios. Do not choose the layout of feature files. Ticket 11
  owns both.
- Do not decide which existing tests convert. Ticket 13 owns that decision.

## Answer records

- The invariance list: each must-hold guarantee, its arch ticket 16 tier, and its
  inventory citation.
- The reclassified guarantees, with the disposition and the moving arch ticket
  for each.
- The list of deliberate behavior changes and the guarantees that they affect.
- The tag rule for scenarios on Migratable surfaces.
- The confirmed proptest list and the confirmed facade seams.
- The reciprocity note for arch-design.
- Constraints for tickets 11, 12, 13 and 14.
