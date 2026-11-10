# Does the index replay prototype survive fault tests, and what does recovery cost?

Type: task
Status: open
Blocked by: none

## Question

Ticket [What does a durable index repair journal cost and
save?](./26-measure-index-recovery-journal.md) measured the cost screen.
The external facts journal survived the screen at the loose upper bound.
This ticket builds the real replay prototype and measures the real saved
work. Do not change production code. This ticket carries the full
specification that ticket 26 moved out of its body.
The maintainer decides adoption after this ticket.

### A. Replay prototype

- Encode parsed facts from the real parser. Do not copy the row bytes.
- Record the encode cost. Compare it with the 53.3 ms row-copy bound.
- Derive links, tags, classes, range keys and notices from the recorded
  facts at replay.
- Measure the derivation cost. The screen replayed prebuilt rows only.
- Carry the full journal content list from ticket 26. It adds the old derived
  keys and the link edges of unchanged files that a change affects.
- Delete the old derived keys that the journal records. Ticket 24 requires
  the deletion.
- Count the replay persist in the saved-work bound. The screen bound counted
  the rival persist as saved, which a replay still pays.
- Use the external file, the writer base check and the store identity
  from tickets 24 and 25.
- Remove the journal only after the `Immediate` replay commit returns `Ok`.
- Kill at a journal sync, then reopen, replay, and check the removal rule.
  This proves the barrier across the whole flow.
- Use the ticket fixture task ratio of about 0.75 tasks per note.
  The screen fixture made 1 task per note and under-tested task paths.
- Measure the journal decode and replay times.
- Measure the store open, commit, barrier and close times.
- Measure the redb repair time after each class of unclean stop.
- Measure the journal bytes by facet, the store size before and after,
  and peak memory.
- Measure the store-writer lock hold time and the time another process waits.
- Report the cost without a failure. Every journaled write pays this cost.
- Report the buffered-control cost and window separately.

### B. Fault tests

Apply these tests after the replay prototype works. Separate two failure
classes:

- **Process stop.** `SIGKILL`, `abort`, a panic and `Ctrl-C` can leave
  different states. Record whether a handler runs `drop`.
  An out-of-memory kill is an untested limit.
- **System crash or power loss.** The operating system can lose pages that
  were not synced.

Stop the process at each of these points:

- Before the journal write completes. After it completes, before the journal sync.
- After the journal sync and before journal publication.
- After journal publication and before the store commit.
  Reopen and read the journal to check that the synced bytes survived.
- Before and after the error-reporting store durability barrier.
- Before the journal removal and after it, before the directory sync.
- In replay, before and after the replay commit.
  Check that replay is safe to repeat.
- In the middle of a full rebuild.

You cannot cut the power. Simulate a system crash through a fault-injecting
file layer and saved file images. Record the method and its limits. Do not
claim that simulation equals power loss.

Test these system-crash cases:

- Lose the last `Durability::None` store commit.
- Keep the journal directory entry. Lose the journal data.
- Lose or keep the store commit while the journal survives.
- Leave the journal after the durable store commit.

Change the input before replay:

- Change a source file that the journal does not list.
- Change a file that the journal lists.
  Do not keep a stale inbound link, notice or range key.
- Add, delete and rename a file.
- Change a file to the same size and `mtime`, inside the racy window.
  Compute the `racy` bit from the original scan start, not the replay time.
- Change the policy.
- Change the parts of the store identity in separate tests.
  The parts include the engine tag, platform family, incarnation and revision.
- Use a lower format epoch and a higher epoch.
  Use a changed extractor fingerprint.
  Do not replay or delete a journal with a higher epoch.

Discard the whole journal when a source, policy, writer-base or
compatible-identity check fails. Keep a higher-epoch journal untouched.
Validate a copied or restored vault from its store identity, writer base,
policy and complete source snapshot. Do not reject a copy only because its
path changed.

Test the storage failures:

- Make the store directory unwritable. Run the command more than once.
  Make sure the journal does not grow or repeat work without a limit.
- Open a read-only vault.
- Fill the disk during the journal write.
- Corrupt or truncate the journal.
- Delete the store but keep the journal. Check the query against a fresh parse.

If an optional journal write, sync or decode fails, ignore its facts and
redo from source files. Do not change the index read and write error rules.

Test the concurrent cases:

- Start a second process while the first holds the store or the journal.
  Test a journal from a live writer and one from a dead writer.
  Never let two processes replay one journal.
- Drop the store handle before a durability check, then let another writer
  act. Check the writer base again before cleanup.

Treat a failed redb commit as an uncertain outcome. After it, check the
store incarnation and revision. Ticket 25 drops the 128-bit `commit_nonce`.
Do not use it.

### C. Correctness and platforms

- Compare every recovered query against a trusted fresh parse.
- Change the revision at a replay. Each query result names the incarnation
  and the revision. Ticket 28 sets the generation pair.
- Cover all facets and range keys. Reject partial or mixed-policy data.
- Exclude everything under `.traces/` from the source scan.
  Ticket 25 sets this rule.
- Sync the journal file and its parent directory. Measure the two syncs
  separately. On macOS this is `F_FULLFSYNC`. Windows cannot sync a
  directory. Do not claim portability across file systems.
- Run the fault tests on Linux at least once.
- Record the sync-call differences between macOS and Linux.
- The macOS numbers in ticket 26 stay the primary latency evidence.

### D. Recovery queries and decision input

- Measure the first usable query and repeated queries per scenario and size.
- Run the scenarios on the cold CLI and the simulated host. Label the host
  model as a simulation. Ticket 33 validates the host retry path against
  these numbers.
- Measure the no-change control with a read-only store open. A read-only
  handle cannot coexist with a writable handle.
- Run the scenarios from ticket 26 step 3: no change, a one-note edit,
  a 100-note edit, an added, deleted and renamed file with inbound links,
  and a policy change that needs a rebuild.
- Use the rich fixture builders and a hub-linked fixture for inbound links.
- Compare full recovered queries at matched stop points, not raw-row reads.
- Use the anchors 50, 5,000 and 50,000 notes. Extend near crossovers.
- Check the untested claims of 85 percent more write time and 2 times the
  file size. Use the decided layout with `lists` and the derived `tasks`
  facet. Measure the task-only scan, the full query rows and a one-note
  replacement.
- Measure the real saved-work bounds. Replace the provisional break-even
  rates from the screen.
- Report the measured stop-window rates for incremental edits.
- Calculate a break-even rate per phase. Never combine the rates. Do not
  invent a stop-time distribution or a failure rate.
- Record the method, the results and the limits under an `## Answer`
  heading. Link the research asset from this ticket.
- Hand the result to [ticket 36](./36-reconcile-decisions-and-audit-coverage.md).
- Ticket 36 owns the `None` versus `Immediate` choice.
- The maintainer decides if the journal enters the specification.

### E. Specify these rules

Specify the detailed journal rules after the prototype survives the fault
tests. Ticket 26 records only the barrier rule.

- Where the journal stores vault-derived facts.
- Who can read the journal, the file mode, and the sync tools or backups
  that copy it. The store holds the same facts today.
  Compare the exposure with the store.
- The journal name. Ticket 25 puts the engine tag in the cache file name.
- The path form in the journal. Ticket 32 owns the path model.
- The behavior-change table row for the journal file. Ticket 16 requires a
  row for a new file on disk.
- The cleanup rule for a stale journal, an orphan journal and a journal of a
  retired engine. Ticket 25 leaves the removal of retired cache files open.
- The behavior when the journal is absent.
- What each path reports after a replay, a discard or a failure.
  Ticket 34 owns the notices.
- Whether the redb adapter can own the journal without a new domain port
  operation. Ticket 24 keeps durability and locking inside each adapter.
- Whether another engine already has a log such as SQLite WAL.
- The new states and failure modes that the journal adds.
- The error-reporting barrier that permits journal removal.
  Ticket 26 fixes the barrier as the `Immediate` replay commit.
- The behavior when journal publication fails or leaves an orphan.

## Method

Use release mode. Do not set `RUSTFLAGS`. Use Criterion for steady-state
timings. Run measurements in series. Do not run measurements in parallel.

Use a child-process harness for cold starts and failure tests. Run a cold
case at least 20 times. Report the median and the range. Use an A-B-A
control to measure noise. Do not rank paths when the ranges overlap.

Use `benches/common/cache.rs` (`drop_page_cache`) for the cold-cache state.
On macOS it is a hint for a single file. It is not a full purge. Record the
cache state and the limit of the method.

Measure latency on macOS/APFS. Use a Podman Linux VM for the Linux
correctness checks on VM-local storage. Do not use host bind mounts.

The VM has 2 GiB of memory. Keep the tests within that limit. Do not treat a
VM result as native-Linux latency. Ticket 26 sets the same method rules.

Record these facts in the research asset:

- The fixture and the seed.
- The measurement range.
- The cache state.
- The Git commit, the operating system, the file system, the disk, the CPU
  and the tool versions.
- The research asset name: `../research/27-index-replay-prototype.md`.

Link the asset from this ticket.

## Inputs from earlier tickets

- [Ticket 26](./26-measure-index-recovery-journal.md) and
  [its measurement](../research/26-index-recovery-journal-measurement.md):
  the screen numbers, the barrier rule, and the kill-sweep method.
- [Ticket 24](./24-index-read-model-and-query-execution.md): the writer base
  check and the facet store shape.
- [Ticket 25](./25-index-freshness-and-invalidation.md): store identity fields
  and the racy bit rule.
- [Ticket 28](./28-live-buffer-and-persisted-index-consistency.md): the
  generation pair and the revision rule at a replay.
- [Ticket 33](./33-workspace-host-and-composition.md): the host retry path
  that validates against these numbers.
