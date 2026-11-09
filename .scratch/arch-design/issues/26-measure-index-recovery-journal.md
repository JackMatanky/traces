# What does a durable index repair journal cost and save?

Type: task
Status: open
Blocked by: 24, 25

## Question

Measure the cost and the benefit of a durable repair journal for the persisted index.
Do not choose a journal in this task.
The maintainer decides after the measurements.
Ticket [Does the landed index freshness model
stand?](./25-index-freshness-and-invalidation.md) is resolved.
It does not wait for this task.

Make a throwaway prototype.
The redesign has not landed, so no production code has the decided layout.
Use the facet layout, the writer base check and the store identity from these tickets:

- [What is the index read model, and is there one query execution path or
  two?](./24-index-read-model-and-query-execution.md)
- [Does the landed index freshness model
  stand?](./25-index-freshness-and-invalidation.md)

Do not change production index code.
Before you start, run `git diff 207cc3fb..main -- src tests docs`.
Record each change that affects the index.

### Step 1: Define the baseline

Write down what a journal can save before you measure.
Every path must check the full source snapshot, as every refresh does.
A no-op refresh costs 12.0 ms at 50 notes and 65.7 ms at 20,000 notes.
At 50 notes, most of this time is the store open and close. It is not the scan.
No path can avoid these costs.

A journal can save only the parse and the derived-fact work of the changed files.

The journal protects the work only in one window.
The window starts when the journal is durable.
It ends when the store is durable.
Measure the length of this window for each path.
Compare it with the length of the parse that the journal saves.

redb is already crash safe.
It uses two commit slots, checksums and a repair flag.
The journal would sit in front of a transactional store.
Test if it adds protection that the store does not give.

### Gate: Run this check before the full matrix

Start with the best case for the journal.
The best case is the full rebuild at 50,000 notes, on the task-heavy shape.
Measure these values for path 1, path 2 and path 4:

- The redo cost.
- The cost of the `Immediate` commit.
- The journal write and sync cost.
- The length of the window in which only the journal is durable.

Stop if the journal cannot beat both path 1 and path 2 in this case.
In that case, skip the full matrix.
Record the gate result and the break-even in the research asset and in the answer.
Run the full matrix only if the journal passes the gate.

Always report the choice between `Durability::None` and `Durability::Immediate`.
Ticket 24 chose `None`. It did not compare the total cost of open, commit and drop.

### Step 2: Compare these recovery paths

1. **Redo.** Replan from a validated surviving store without a journal.
   This is the model that ticket 25 landed.
2. **Durable commit.** Use redb `Durability::Immediate` for the final store commit.
   Use no journal.
   Research 11 measured 4.01 ms for this commit on an open store.
   Ticket 24 chose `Durability::None`, so this path is the cheapest alternative to a journal.
3. **Host retry.** A long-lived host keeps its published in-memory generation.
   After a failed or uncertain commit, the host does these steps:
   - It drops the handle and the plan.
   - It scans the sources again.
   - It replans from the retained generation.
   - It retries after the input and base checks.

   Tickets 24 and 25 set these rules.
   A cold CLI process cannot use this path. Record that fact.
4. **Journal.** Write the prepared facts to a durable journal before the store write.
   Replay only after the input and base checks.
5. **Side file.** Use this path only for the full rebuild.
   Build the new store in a side file.
   Sync it, then replace the old store.
   Ticket 25 treats a failed replacement as a cache miss for reads.

Test two journal locations:

- Next to the store.
- In the user state directory.

The journal must hold these items:

- The base store identity: engine tag, platform family, format epoch, extractor fingerprint,
  policy fingerprint, incarnation and revision.
- For each changed file: the size, `mtime`, `ctime`, the `racy` bit and the scan start time.
- The replaced facet records of each changed file.
- The old derived keys to delete. Ticket 24 requires this deletion.
- The link edges of unchanged files that a change affects.
- The parse-time notices.
- A checksum.

Read the journal facts only from disk handles.
Never write a fact from an unsaved editor buffer.
Ticket 27 owns the buffer overlay.

### Step 3: Measure these scenarios

Use each scenario with a cold CLI restart and with a long-lived host.

- No change. This is the control.
- A one-note edit.
- A 100-note edit.
- One added, one deleted and one renamed note.
  These change inbound links in other files.
- A task-policy change.
  Ticket 25 makes this a full rebuild.

Use these fixture shapes:

- Rich and realistic. Use the `benches/common` project builders.
- Task-heavy. Ticket 24 measured 37,500 tasks among 487,500 items at 50,000 notes.
  Use this ratio.
- Hub-linked. Use it only for the inbound-link checks.

Test 50, 200, 1,000, 5,000, 10,000, 20,000 and 50,000 notes.
The maintainer chose this range.
It comes from the maintainer's vaults and from other Obsidian vaults.
The `benches/common` sweep stops at 20,000 notes.
Extend it in the prototype.

Measure these values for each path, scenario and size:

- The first usable query after a restart.
- Repeated queries after the recovery.
- The journal write time, the journal sync time and the directory sync time.
- The journal removal time.
- The store open time, with the redb repair time after an unclean stop.
- The recovery time.
- The disk bytes of the journal, and the store size before and after.
- The peak memory.
- The number of file reads and parses during recovery.
- The length of the window in which only the journal is durable.

Report the cost without a failure.
This is the cost that every user pays on every write.
Report the journal bytes for each facet.

A reviewer reported in ticket 25 that the 11-facet layout costs 85% more write time.
The same reviewer reported that it doubles the file size.
No one checked these figures.
Check them on the layout that you use.
Measure both the `lists` and the `tasks` layouts that ticket 24 leaves open.

### Step 4: Test these failure classes

Separate two failure classes:

- **Process stop.** Examples are `SIGKILL`, a panic, `Ctrl-C` and an out-of-memory kill.
  The operating system keeps the pages that the process wrote.
- **System crash or power loss.** The operating system loses the pages that you did not sync.

Test the process stop with a child process and a fail point.
You cannot cut the power.
Simulate a system crash from the file images at each sync point.
Record the simulation method and its limit.
Do not claim that the simulation equals real power loss.

Stop the process at each of these points:

- Before the journal write ends. After it ends, before the journal sync.
- After the journal sync.
- Before the store commit. After the store commit.
- Before the journal removal.
- Replay: stop before the replay commit. Replay: stop after the replay commit.
  The replay must be safe to repeat.
- Full rebuild: stop in the middle of the rebuild.

Test these system-crash cases:

- Lose the last `Durability::None` store commit.
- Keep the journal directory entry. Lose the journal data.

Change the input before the replay:

- Change one source file that the journal does not list.
- Change one file that the journal lists.
  The replay must not keep a stale inbound link, a stale notice or a stale range key.
  Discard the whole journal or recompute the cross-file facts.
- Add, delete and rename a file.
- Change a file to the same size and `mtime`, inside the racy window.
  The replay must compute the `racy` bit from the original scan start. It must not use the replay time.
- Change the policy.
- Change each part of the store identity.
  The parts are the engine tag, the platform family, the incarnation and the revision.
- Use a lower format epoch and a higher format epoch.
  Use a changed extractor fingerprint.
  Do not replay a journal with a higher epoch. Do not delete it.
  Ticket 25 sets the same rule for the store.

Test the storage failures:

- Make the store directory unwritable.
  Check whether the journal location is still writable.
  Run the command more than once.
  Make sure the journal does not grow and the cost does not repeat without a limit.
- Open a read-only vault.
- Fill the disk during the journal write.
- Corrupt the journal. Truncate the journal.

Test the concurrent and copy cases:

- Start a second process while the first process holds the store or the journal.
  Test a journal from a live writer and a journal from a dead writer.
  Two processes must never replay one journal.
- Copy or restore the vault with its journal to another path or machine.
  The replay must reject the journal.

Treat a failed redb commit as an uncertain outcome.
After it, check the store incarnation and the revision.
Ticket 25 drops the 128-bit `commit_nonce`. Do not use it.

## Check every result

Check every recovered query against a trusted fresh parse.
Check every facet: `files`, `fields`, `tags`, `classes`, `headings`, `outlinks`, `links`,
`tasks`, `lists`, `notices` and the range keys.
Reject partial data and mixed-policy rows.
Make sure a replay does not skip a journaled file without a decision.

Check that nothing under `.traces/` enters the source scan.
If a journal is outside `.traces/`, check that the scan excludes it.
Ticket 25 sets this rule.

Sync both the journal file and its directory before you treat the journal as durable.
Record the sync call that you use on each platform.
On macOS, check if the call flushes the disk cache.
Windows cannot sync a directory. Record how the journal replaces this step.
Do not claim that a result on one platform holds on another.

## Record these rules

- Where the journal stores vault-derived facts.
- Who can read the journal, the file mode and the sync tools or backups that copy it.
  The store holds the same facts today. Compare the exposure with the store.
- The journal name. Ticket 25 puts the engine tag in the cache file name.
- The cleanup rule for a stale journal, an orphan journal and a journal of a retired engine.
  Ticket 25 leaves the removal of retired cache files open.
- The behavior when the journal is absent.
- What each path reports to the user after a replay, a discard or a failure.
  Ticket 33 owns the notices.
- Whether the journal fits inside the store adapter.
  Ticket 24 keeps durability and locking inside the adapter.
  Record if it needs a new port operation.
- Whether the journal works with any engine, or duplicates the log of an engine such as SQLite WAL.
- The new states and failure modes that the journal adds.

## Method

Use release mode.
Run one benchmark at a time.
Do not set `RUSTFLAGS`.
Use Criterion for steady-state timings.

Use a harness that starts child processes for cold starts and for the failure tests.
Run each cold case at least 20 times.
Report the median and the range.
Use an A-B-A control, as research 11 did, to measure the noise.

Use `benches/common/cache.rs` (`drop_page_cache`) for the cold-cache state.
On macOS this is a hint for one file. It is not a full purge.
Record the cache state of each run and the limit of the method.

Record these facts in a research asset:

- The fixture and the fixture seed.
- The measurement range.
- The cache state.
- The Git commit, the operating system, the file system, the disk, the CPU, and the
  versions of Rust, `redb` and `postcard`.
- The limits.
- The results in Markdown tables, which you also print to the console.

The research asset is `../research/26-index-recovery-journal-measurement.md`.
Link it from this ticket.

## Calculate the break-even

For each size, scenario and path pair, calculate the failure rate at which the journal pays for itself.

- `O` is the extra cost of the journal for each write without a failure.
- `B` is the redo cost minus the replay cost for one failure.
- The journal pays when the failure rate is more than `O / B` for each write.

Report the rate as "one failure for each N writes".
Report it against path 1 and against path 2.
Report it for each failure class.
Do not assume that failures are common.
Do not invent a failure rate.
State that the real rate is unknown.

## Finish the task

Remove the throwaway prototype after the measurements.
Keep the Git commit and the commands in the research asset, so that a reader can repeat the runs.
Record the measured trade-offs in this ticket, under an `## Answer` heading.
Link the answer from ticket 25.
Return the trade-offs to the maintainer.

The answer must name these items:

- The paths that you compared, with the retained path 1 as the baseline.
- The invariant that the journal needs, and its owner.
- The code that the journal makes removable. Name none if there is none.
- What stays unresolved.
- The downstream tickets that the result changes.

## Inputs from earlier tickets

These sections are facts and open points. Treat them as fallible.

- [Research 11](../research/11-performance-evidence.md): `Database::create` costs 11.4 ms and
  `drop` costs 12.0 ms. A `Durability::None` commit on an open store costs 18 µs.
  An `Immediate` commit costs 4.01 ms.
  Ticket 25 notes that earlier data suggests a journal adds more fixed latency than it saves.
  Treat this as a hypothesis. Try to disprove it.
- [Ticket 24](./24-index-read-model-and-query-execution.md): a host opens the store only
  to commit a batch. Each batch pays one open, about 13 ms.
  The journal must add its cost to this open cost.
  The store is a derived cache. It is not crash-durable data.
- [Ticket 25](./25-index-freshness-and-invalidation.md): after an uncertain commit, the
  host drops the handle and the plan. The store has no unwritable latch.
  Each persist tries the write-open again. Crash recovery is a no-op refresh of the
  uncommitted files. An unwritable store can repeat the same work [H].
- [Ticket 27](./27-live-buffer-and-persisted-index-consistency.md): a query result names the
  generation pair, the incarnation and the revision. A replay must change the revision.
- Tickets 20 and 32 describe undo journals for agent writes.
  Ticket 22 describes one for `init`. They are different.
  They protect vault files. This journal protects a derived cache.
- Ticket 16 freezes index results that equal a fresh parse. The journal must obey this rule.
- [Preparation notes](../research/26-index-recovery-journal-prep.md): source facts,
  probe results on macOS, a proposed gate and the faults found in this ticket.
  Treat them as fallible.

## Handoffs

- Ticket 24: the adapter and port result.
- Ticket 25: link the measurement.
- Ticket 27: the generation pair after a replay.
- Ticket 31: the path form in the journal.
- Ticket 32: the host persist rule and the staleness bound.
- Ticket 33: the notices for a replay or a discard.
- Ticket 16: a change-table row for each new file on disk.
- Ticket 35: it waits for this ticket.
  The maintainer decides if the result must enter the specification, or if the journal is out of scope.
