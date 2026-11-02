# What does a durable index repair journal cost and save?

Type: task
Status: open
Blocked by: 24

## Question

Measure the recovery journal. Ticket
[Does the landed index freshness model
stand?](./25-index-freshness-and-invalidation.md) is resolved. It no longer
waits for this measurement. This task decides whether to add a journal later.

Use the decided facet layout and the writer base check of ticket 24.
See [What is the index read model, and is there one query execution path or
two?](./24-index-read-model-and-query-execution.md).
Use a throwaway prototype if that layout has not landed.
Do not change production index code.

Compare these recovery paths:

1. Replan from a validated surviving store without a journal.
2. Retry retained work in a long-lived host after input and base checks.
3. Write the prepared facts to a durable journal before the store write.
   Replay only after input and base checks.

Measure a one-note edit, a 100-note edit and a task-policy change that needs a
full reparse. Include a cold CLI restart and a long-lived host.
Use representative vault files and a task-heavy shape.
Test 50, 200, 1,000, 5,000, 10,000, 20,000 and 50,000 notes.

Measure the first usable query, repeated queries, journal write and sync time,
recovery time, disk bytes and peak memory. Report the cost without failure.
Apply the same source-snapshot checks to every recovery path.

Test these failure points:

- Stop the process before the journal sync. Stop the process after the sync.
- Stop the process before the store commit. Stop the process after the commit.
- Stop the process before journal removal.
- Lose the last `Durability::None` store commit.
- Change a source file or the policy before replay.
- Change the store incarnation or revision before replay.
- Make the store directory unwritable.
  Check whether the journal location is still writable.
- Corrupt or truncate the journal.

Check every recovered query against a trusted fresh parse.
Reject partial data and mixed-policy rows.
Record where the journal stores vault-derived facts and who can read them.
Record the journal cleanup rule and the behavior when the journal is absent.
Treat a failed redb commit as an uncertain outcome.
Sync both the journal file and its directory before treating the journal as
durable.

Use release-mode Criterion runs for performance comparisons.
Record the fixture, measurement range, cache state, limitations and results in
a research asset linked from this ticket.
Calculate the failure frequency at which the journal pays for its normal
write cost. Do not assume that failures are common.

Remove the throwaway prototype after the measurements.
Do not choose a journal in this task.
Return the measured trade-offs to the maintainer. Record them in this ticket
and link them from ticket 25.
