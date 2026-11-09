# What does a durable index repair journal cost and save?

Type: task
Status: resolved
Blocked by: 24, 25

## Question

Measure the cost and the benefit of a durable repair journal for the persisted index.
Do not choose a journal in this task.
The maintainer decides after the measurements.
Ticket [Does the landed index freshness model
stand?](./25-index-freshness-and-invalidation.md) is resolved.

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
Check the full source snapshot on every path.
Research 11 measured 12.0 ms for a no-op refresh at 50 notes and 65.7 ms at
20,000 notes, on the old store layout.
At 50 notes, most of this time is the writable store open and close.
Do not call it the scan cost.
Treat these numbers as [H] inputs.

Measure a read-only open as a separate control. It can avoid the writable open
and close on an unchanged query. It cannot avoid the source check.

A journal can save some parse and derived-fact work after a failed write.
The saved work depends on the facts in the journal and the work needed to replay them.

The protection window starts after the journal becomes durable.
The window ends when the store commit is durable.

- An external journal becomes durable after its file sync and directory sync.
- An in-store journal becomes durable after its first durable transaction.
- A new store file also needs a parent-directory sync.

Measure the window for the cold CLI and the simulated host.
Measure the time before journal publication separately. A stop there gets no benefit.

redb is already crash safe when the file system obeys its sync calls.
It uses two commit slots, checksums and a repair flag.
Test whether the journal saves work that a redb commit does not save.
Do not count a normal `Database::drop` as proof of durability.
redb does not return a close error, and it skips the final close write during a panic.
Use an error-reporting durability barrier before journal removal.

### Gate: Reject only cases that cannot pay

Start with measured cases for edits and full rebuilds.
Use changed-file counts, vault sizes, note shapes and path-set changes.
Measure the cost of a full rebuild and an incremental update for the same inputs.
Do not add a file-count switch to production code in this task.
Record a crossover only when repeated measurements show one.

Measure these values before you expand a case:

- An upper bound on work that replay can save, including derived-fact work.
- A lower bound on the candidate's normal-write cost.
  For an external journal include encoding, file sync and directory sync.
  For an in-store journal include its first durable transaction.
- The cost of an error-reporting store durability barrier.
- The total open, commit, repair and close costs for `None` and `Immediate`.
- The protection window and the rival's first usable query after a stop.

Reject a case only when a measured upper bound on saved work is no greater than
a measured lower bound on added cost.
Compare the journal with redo and with `Durability::Immediate` separately.
If the bounds overlap, keep the candidate open for complete replay measurement.
Do not use an upper bound on saved work to claim that a journal pays.
Record the gate result and its limits. Do not choose a journal.

If any candidate survives the cost screen, create a follow-on task.
Move the full replay prototype, fault-injecting crash tests, Linux correctness
checks and recovery-query measurements to that task.
Move ticket 36's blocker from this ticket to the follow-on task.
Record a cost-screen result under this ticket's `## Answer`.
Name the unresolved recovery proof. Do not select a journal from costs alone.

Report the `None` and `Immediate` comparison as a separate result.
Ticket 24 chose `None` without a total open, commit and drop comparison.
Ticket 36 owns the durability choice after the needed measurements.

### Step 2: Compare these recovery paths

1. **Redo.** Replan from a validated surviving store without a journal.
   This is the model that ticket 25 landed.
2. **Durable commit.** Use redb `Durability::Immediate` for the final store commit.
   Use no journal. Research 11 measured 4.01 ms for this commit on an open store.
   Measure the total open, commit and drop cost on the decided layout.
   Measure repair after a stop. Do not apply the synthetic-store result to a real vault.
3. **Host retry.** Model a host that keeps its published in-memory generation.
   After a failed or uncertain commit, the host does these steps:
   - Drop the handle and the plan.
   - Scan the sources again.
   - Replan from the retained generation.
   - Retry after the input and base checks.

   Tickets 24 and 25 set these rules.
   A cold CLI process cannot use this path. Label the host model as a simulation.
   Ticket 33 must check the result against the eventual host.
4. **Durable journal.** Write prepared facts beside the store before its write.
   Hold the store writer handle before journal publication or replay.
   Check the writer base while the handle is held. Replay only after the input checks.
   Make the journal durable before the store write:
   - Sync the journal file and its parent directory.
     Sync on macOS uses `F_FULLFSYNC`.
   - Keep the journal until the replay commit with `Durability::Immediate` returns `Ok`.
     That commit is the error-reporting barrier that permits journal removal.
   - Do not use `Database::drop` as the barrier. Its close commit reports no error.
     A reopen that reads the new revision does not prove durability.
5. **Buffered journal control.** Write the prepared facts without a sync.
   Test process stops only. Do not count it as power-loss protection.
6. **Side file.** Use this path only for full rebuilds that pass the gate.
   Build and sync the new store in a side file. Replace the old store and sync
   the directory before claiming that the replacement is durable.
   Ticket 25 treats a failed replacement as a cache miss for reads.
7. **In-store journal table.** Screen this redb candidate beside the external journal.
   Use `Durability::Immediate` for both transactions.
   The transactions do these steps:
   - Record the facts in a first durable transaction.
   - Apply the facets and remove the journal atomically in a second durable transaction.
   Apply these limits:
   - Include the parent-directory durability cost if the store file is new.
   - An unreadable or incompatible store cannot replay this journal.
   - Expand this path only if its cost bounds and failure tests justify replay work.
   - Do not select it from this cost screen.

For an external journal, test only the location next to the store under `.traces/`.
Do not add a user state directory location.
Keep any viable journal inside the redb adapter.
Keep engine types, locking and durability out of the domain store port.
A future engine can use its own recovery method.

The journal must hold these items:

- The base store identity: engine tag, platform family, format epoch, extractor fingerprint,
  policy fingerprint, incarnation and revision.
- For each changed file: the size, `mtime`, `ctime`, the `racy` bit and the scan start time.
- The facts that the replay needs to replace or rebuild each changed facet.
- For incremental replay: the old derived keys to delete.
  Ticket 24 requires the deletion. A full rebuild replaces all old keys.
- The link edges of unchanged files that a change affects.
- The parse-time notices.
- A checksum.

For a full rebuild, measure two payloads before you choose a replay prototype:

- All prepared facets for direct replay.
- Parsed facts plus the work to rebuild keys, links and notices on replay.

Read the journal facts only from disk handles.
Never write a fact from an unsaved editor buffer.
Ticket 28 owns the buffer overlay.

### Step 3: Measure these scenarios

Use each selected scenario with a cold CLI restart and a simulated long-lived host.
Do not call the simulated host a production measurement.

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

The maintainer chose 50, 200, 1,000, 5,000, 10,000, 20,000 and 50,000 notes.
This range comes from the maintainer's vaults and from other Obsidian vaults.

The `benches/common` sweep stops at 20,000 notes.

- Extend the sweep in the prototype.
- Start with 50, 5,000 and 50,000 notes as vault-size anchors.
- Measure one-note, 100-note and 5,000-note edits where the size permits.
- Include a full rebuild at 50,000 notes on the task-heavy shape.
- Expand to 200, 1,000, 10,000 and 20,000 notes near a measured crossover.
- Keep the no-change and path-set controls.
- Do not run every path at every size unless the measured bounds leave that case open.

Measure a complete no-change query with a read-only store open.
Measure a changed query with the read-only-to-writable handover.
Measure a repair-required read-only open and its writable fallback.
At the cost screen, measure the payload size, the candidate's normal-write
cost, the rival's first usable query and each protection window.
Record an upper bound on saved work for each selected size and scenario.
Do not call a bound a complete replay measurement.

For each candidate that survives, the follow-on task measures the recovery
costs. Ticket 27 sections A and D carry the full list.

Report the cost without a failure. Every journaled write pays this cost.
Report the cost and the protection window for the buffered control separately.

A reviewer reported in ticket 25 that the 11-facet layout costs 85% more write time.
The same reviewer reported that it doubles the file size. No one checked these figures.
Check them on the decided layout with `lists` and the derived `tasks` facet.
Measure the task-only scan, the full query rows and a one-note replacement.

### Step 4: Hand the failure tests to the follow-on task

Apply this step only to candidates that survive the cost screen.
Separate two failure classes:

- **Process stop.** `SIGKILL`, `abort`, a panic and `Ctrl-C` can leave different states.
  An out-of-memory kill is an untested limit. Do not exhaust the workstation.
- **System crash or power loss.** The operating system can lose pages that were not synced.
  A lost `Durability::None` commit costs redo work and can leave a store to repair.

You cannot cut the power. Simulate a system crash through a fault-injecting
file layer and saved file images. Record the method and its limits. Do not
claim that simulation equals power loss.

Ticket 27 section B carries the full fault-test specification.
It covers the stop points, the system-crash cases, the input-change cases,
the storage failures, the concurrency cases and the uncertain-commit rule.

## Check every result

At the cost screen, check each measured rival query against a fresh parse.
The follow-on task keeps every recovered query equal to a fresh parse.
Ticket 27 section C carries the full facet list and the rejection rules.

Check that nothing under `.traces/` enters the source scan.
Ticket 25 sets this rule.

Use `File::sync_all` on the journal file and sync its parent directory before
you treat the journal as durable. Measure the two calls separately.
On macOS, `sync_all` and `sync_data` both use `F_FULLFSYNC`.
Check the calls in the cost-screen prototype.
Do not infer a Windows directory guarantee.
Do not claim that a result on one file system holds on another.

## Record these rules

Record the gate's input data and limits in this ticket. The follow-on task
records the detailed journal rules. Ticket 27 section E carries the list.

## Method

Use release mode.
Run one benchmark at a time.
Do not set `RUSTFLAGS`.
Use Criterion for steady-state timings.

Use a harness that starts child processes for cold starts and failure tests.
Run each cold case at least 20 times. Report the median and the range.
Use an A-B-A control, as research 11 did, to measure noise.
Do not rank paths when the ranges overlap.

Use `benches/common/cache.rs` (`drop_page_cache`) for the cold-cache state.
On macOS this is a hint for one file. It is not a full purge.
Record the cache state of each run and the limit of the method.

Measure latency on macOS/APFS. Use a Podman Linux VM for separate correctness
checks on VM-local storage. Do not use host bind mounts for Linux storage checks.
The existing `applehv` VM has 2 GiB of memory. Keep its tests within that limit.

Do not treat a Linux VM result as native-Linux latency or a Windows result.
Do not claim that a crash simulation equals a real power loss.

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

For each selected size, scenario, failure class and path pair, compare matched
stop points on the same source snapshot. Report a conditional break-even by phase.
Align each stop by elapsed time from the start of the write.
Record whether each path has made its store commit durable at that time.

- `O` is the measured extra cost of the journal on a write without failure.
- `B` is the rival's first usable query time minus the journal's after that stop.
- Include barrier, repair, decode and cleanup costs that differ between paths.
- Calculate `O / B` only for a phase with positive `O` and `B`.
- For a phase with no saved work, report no break-even benefit.

For a single phase, report the conditional rate as "one eligible stop in that
phase for each N journaled writes". Do not combine rates from different phases.
Report each protection window and the unprotected parse time.
Compare redo and `Immediate` separately.

Report normal-write cost and recovery-query time on separate axes.
Do not invent a stop-time distribution or a failure rate.
State that the real rate is unknown. Record inconclusive ranges.

## Finish the task

Remove the cost-screen prototype after the measurements.
Keep the Git commit and the commands in the research asset, so a reader can repeat the runs.
Record the cost bounds and measured rivals under an `## Answer` heading.
Link the answer from ticket 25. Return the trade-offs to the maintainer.

If no candidate survives, record the rejection evidence and remaining limits.
If a candidate survives, create the follow-on task before resolving this ticket.
Move the full-replay and failure-test requirements to the follow-on task.
Change ticket 36's blocker to the follow-on task.
Do not report recovery correctness or a journal choice from a cost-screen result.

The answer must name these items:

- The paths that you compared, with the retained path 1 as the baseline.
- The proposed journal invariant and its owner. Name none if all candidates fail.
- The code that the journal makes removable. Name none if there is none.
- What stays unresolved.
- The downstream tickets that the result changes.
- If a candidate survived: the follow-on task and the changed ticket 36 blocker.

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
- [Ticket 28](./28-live-buffer-and-persisted-index-consistency.md): a query result names the
  generation pair, the incarnation and the revision. A replay must change the revision.
- Tickets 20 and 33 describe undo journals for agent writes.
  Ticket 22 describes one for `init`. They are different.
  They protect vault files. This journal protects a derived cache.
- Ticket 16 freezes index results that equal a fresh parse. The journal must obey this rule.
- [Preparation notes](../research/26-index-recovery-journal-prep.md): source facts,
  macOS probes and estimates from the old store layout.
  The note records tentative grilling decisions. Treat all estimates as fallible.

## Answer

Measured. Full numbers live in
[the cost screen](../research/26-index-recovery-journal-measurement.md).
Marks use the map rules.

### Gate verdicts

- The saved-work bounds below are upper bounds. A replay still pays the
  persist, and it must derive facets from facts. The real saved work is
  lower. Ticket 27 measures it.
- Rebuild, 50,000 notes: the external facts journal costs 76 ms per command.
  The loose saved-work upper bound is 1,332.9 ms. No rejection at the bound.
  This is not proof that a journal pays.
- The in-store journal table costs more than the external file at
  every measured size. At 50,000 notes it adds 477 ms over its own baseline
  apply. [P] Drop the in-store table.
- Edit 100 notes: the loose saved-work bound is 125 ms. The external journal
  costs 8.0 ms per command. No rejection at the bound. [H] It pays only above
  a 4 to 7 percent stop rate inside the 125 ms window.
- No-change commands save nothing and pay 8.0 ms. They never pass.
- Verdict: no candidate rejects, so no journal is chosen here.
  The screen sets provisional break-even rates. Ticket 27 must measure the
  real saved work before anyone uses the rates.

### Measured facts that bound the decision

- [V] The sampled rebuild kills left an empty or a full store, never
  partial. The probe sampled 9 of 15 stop points. The next open costs 17 to
  37 ms.
- [V] `Durability::None` commits die on abort. `Immediate` commits
  survive. This check sampled one stop point per mode on one store copy.
  `Immediate` cost 4.6 ms more per command in this sample.
- [V] The journal publication barrier is specified and costed. A synced
  journal file plus directory sync has a 4 ms floor. The probe did not run
  the kill-and-replay test. Ticket 27 must prove the barrier end to end.
- [H] The claims of 85 percent more write time and 2 times size for a facet
  layout stay untested. The probe emulated a round-robin split, not the
  decided facet layout of ticket 24.
- [V] The no-change refresh costs 385 ms at 50,000 notes. Open and scan
  dominate it. A journal cannot remove this cost.
- [H] Journal encode costs here copy real row bytes, so they are lower
  bounds. A facts encode from the real parser adds tens of percent.

### Handoffs

- [Ticket 27](./27-index-replay-prototype-and-fault-tests.md) owns the rest:
  the real replay prototype with derivation, fault-injection crash tests,
  and the barrier end-to-end proof.
  It also owns Linux correctness checks, full-query recovery measurements,
  and the real saved-work bounds behind the break-even rates.
- Ticket 36: its blocker moved from this ticket to ticket 27.
  The `None` versus `Immediate` choice stays with ticket 36.
- Ticket 25: the read-only-open control finding (0.03 ms, no syncs) passes
  there. A read-only handle cannot coexist with a writable handle.
- Ticket 33: the simulated-host retry path validates against these numbers.
  After a panic, discard the handle and the plan. Validate before publication.
- Ticket 24: preserve the engine-neutral store port and adapter-owned durability.
- Ticket 28: the generation pair after a replay. A replay must change the revision.
- Ticket 32: the path form in the journal.
- Ticket 34: the notices for a replay or a discard.
- Ticket 16: a change-table row for each new file on disk.

## Code delta

None. No production code changed. The probe is a throwaway crate that drives
the engine through `traces::testing`. Its source is recorded in
`research/26-prep/`. The screen ran at release mode, one benchmark at a time,
with no `RUSTFLAGS`.

Drift check: `git diff 207cc3fb..main -- src tests docs` shows 197 files.
The index changed in 13 files (+1,003, -576), including `store.rs`,
`refresh.rs` and `service.rs`. The probe measured `main` at `79f393a2`,
so the landed drift is inside these numbers. They are not stale.
