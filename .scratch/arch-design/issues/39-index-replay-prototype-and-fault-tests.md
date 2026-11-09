# Does the index replay prototype survive fault tests, and what does recovery cost?

Type: task
Status: open
Blocked by: none

## Question

Ticket [What does a durable index repair journal cost and
save?](./26-measure-index-recovery-journal.md) measured the cost screen.
The external facts journal survived the screen. This ticket builds the real
replay prototype and tests it. Do not change production code.
The maintainer decides adoption after this ticket.

### A. Replay prototype

- Encode parsed facts from the real parser. Do not copy row bytes.
- Record the encode cost. Compare it with the 53.3 ms row-copy bound.
- Derive links, tags, classes and notices from the recorded facts at replay.
- Measure the derivation cost. The screen replayed prebuilt rows only.
- Use the external file, the writer base check and the store identity
  from tickets 24 and 25.
- Remove the journal only after the `Immediate` replay commit returns `Ok`.

### B. Fault tests

- Kill at every sync point of the journal write and the replay commit.
- Simulate a lost last `Durability::None` commit with a surviving journal.
- Test corrupt and truncated journals.
- Test a journal from a higher format epoch. Never replay or delete it.
- Test an unwritable store directory across repeated commands.
- Check that the journal does not grow without bound there.
- Test concurrent live and dead writers. Never replay twice.

### C. Correctness and platforms

- Compare every recovered query against a trusted fresh parse.
- Cover all facets and range keys. Reject partial or mixed-policy data.
- Run the fault tests on Linux at least once.
- Record the sync-call differences between macOS and Linux.
- The macOS numbers in ticket 26 stay the primary latency evidence.

### D. Recovery queries and decision input

- Measure the first usable query and repeated queries per scenario and size.
- Use the anchors 50, 5,000 and 50,000 notes. Extend near crossovers.
- Report the measured stop-window rates for incremental edits.
- Hand the result to [ticket 35](./35-reconcile-decisions-and-audit-coverage.md).
- Ticket 35 owns the `None` versus `Immediate` choice.
- The maintainer decides if the journal enters the specification.

## Inputs from earlier tickets

- [Ticket 26](./26-measure-index-recovery-journal.md) and
  [its measurement](../research/26-index-recovery-journal-measurement.md):
  the screen numbers, the barrier rule, and the kill-sweep method.
- [Ticket 24](./24-index-read-model-and-query-execution.md): the writer base
  check and the facet store shape.
- [Ticket 25](./25-index-freshness-and-invalidation.md): store identity fields
  and the racy bit rule.
