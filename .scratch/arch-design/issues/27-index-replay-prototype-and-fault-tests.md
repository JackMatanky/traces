# Does the index replay prototype survive fault tests, and what does recovery cost?

Type: task
Status: resolved
Blocked by: none

## Answer

### Result and adoption scope

The throwaway prototype passed 63 of 63 macOS release-profile cases with redb
4.3.0. The Linux container run passed 62 cases; the guarded `ENOSPC` case stays
ignored by design. The results do not establish every requirement in
sections A–E. The resolved status records the investigation's completion, not
completion of all adoption gates.

Ticket 36 weighs adoption and the `None` versus `Immediate` choice. Ticket 33
gets the freshness data. A numeric host freshness bound remains a handoff
requirement.

The [prototype evidence](../research/27-index-replay-prototype.md) records the
method, measurements and limits. The evidence does not support adoption on its
own.

No journal policy beats no-journal `Immediate` on the measured write path. These
policy cases do not all do equivalent persistence work. Do not infer a general
policy ranking from those cases.

### Re-targeted probe evidence (2026-11-10)

The probe journals and replays the real `.traces/index.redb` layout. It checks
full `QueryRow` accessors against a fresh-parse store. The layout probe's
one-run observations remain in the research asset for method record only.

The replay uses the eight current production redb tables, not the complete
decided eleven-facet layout. The decided identity fields use a modelled header
and sidecar. The production store does not yet carry them.

Notices remain outside the probe because the current parser has no notices data.
Tickets 29 and 34 retain that gate.

The macOS suite covers these cases:

- A weakened incarnation-plus-revision base check wrongly accepts a
  same-revision forked journal with different rows. This proves a hole in that
  probe's identity check. A digest is the tested backstop, not a newly adopted
  ticket-25 contract or the only possible solution.
- The probe handles every decided-identity mutation that it models.
- A read-only vault and a true read-only handle never open the journal.
- `cp -R` reparses. `mv` replays.
- `SIGKILL` and `SIGINT` match the simulated stops.
- The probe rejects a hostile checksum-valid body without a large allocation.

A Linux container run passed the full 62-test re-targeted suite with pinned
redb 4.3.0 (one case ignored by design). Two `SIGKILL`/`SIGINT` cases first
failed there because the image ships no `kill` binary; a
`T27_SIGNAL_HELPER`-aware harness fixes that. The 4 GiB VM setting below is the
original plan, not the actual run configuration.

### Storage-failure observation and blocker

A dedicated `enospc-probe` binary reproduced real `ENOSPC` in a 64 MiB
`tmpfs` container. The writer gets a real catchable error
(`code: 28 StorageFull`) at the failing commit. The loss is what follows: under
`Durability::None`, every row acknowledged before that commit vanished on
reopen (`acknowledged=31 actual=0 missing=31`). Under native `Immediate`, all
earlier acknowledged rows survived on the same filesystem. An orderly close
with no failure kept all 128 `None` rows, so the loss needs the failure
boundary, not a `None` commit alone. A post-`ENOSPC` store keeps a recovery
flag, so `open_read_only` reports `RepairAborted`; the separately labelled
writable reopen is the only working scan.

The journal-induced and no-journal surfaces matched, and both lost
acknowledged `None` rows. The decided reopen-and-classify path has an error to
report, but it cannot restore the lost acknowledged rows.

This observation is an adoption blocker for ticket 36 to weigh. It is scoped to
the tested stack (redb 4.3.0, tmpfs, orderly exit at the failure boundary). It
is not a general redb guarantee, and it is not a power-loss test.

### Measurement evidence and answer (2026-11-10)

The corrected matched harness ran its implemented matrix on Linux with the
release profile, using native redb durability (`Durability::Immediate` commits,
`checkpoint_for_probe` for the flush barrier). Policy comparisons use 20 runs
in a true a-b-a-c-a order at 5,000 and 50,000 notes. All other cases use 20
runs. The 50-note anchor remains outside this matrix.

Linux results at 5k: the a-block medians are 671.9, 658.1 and 659.8 ms, so the
noise band is about 2 percent. Journaled `Immediate` costs 667.3 ms and
journaled `None` costs 662.9 ms against the 663.7 ms no-journal median. At 50k
the no-journal median is 6,686.0 ms. Journaled `Immediate` is 6,705.5 ms
(+0.3 percent). Journaled `None` is 6,620.8 ms (-1 percent). The journal
surcharge sits inside the noise band at both sizes. No policy ranking is
proved.

Recovery saves real work. At 5k, replay takes about 300 ms against about
330 ms for parse fallback (8 to 10 percent saved). At 50k, replay takes about
2,940 ms against about 3,220 ms (8.2 to 10.7 percent saved across the
edit and links scenarios). A policy rebuild dominates both paths at
about 5.7 s at 50k.

The source-check placement comparison now uses equal work. Placement A (check
before hold) totals 551.2 ms with a 217.8 ms hold at 50k. Placement B (check
inside hold) totals 560.9 ms with a 558.3 ms hold. The totals match; B holds
the writer lock 2.56 times longer. Stable versus all-racy snapshots differ by
under 5 ms at 20 runs each (2,997.9 versus 3,002.6 ms at 50k). A pure
read-only no-change pass costs the same with and without a retained journal
(216.4 versus 215.6 ms digest). First usable queries take 9 to 101 ms by
size; repeated queries are cheaper.

Earlier macOS numbers below keep their record. The Linux run supersedes them
as the primary evidence for the corrected protocol.

- Journal encode is about 0.3 ms. The journaled `Immediate` cost was reported as
  sync cost, not encode: about 490 ms at 5k and 5.5 s at 50k. These figures
  describe the reported phase costs, not the complete normal-write surcharge.
  The harness uses external fsync as a surrogate. It does not establish native
  redb `Immediate` behavior.
- The reported retained-journal no-change refresh takes 4,315.3 ms versus
  6,329.0 ms at 50k. Its case name implies read-only access, but the measurement
  enters writable recovery. A pure read-only operation never opens the journal.
  The reported interpretation shifts cost to the next write. The control does
  not prove a pure read-only benefit.
- The original placement interpretation reports a 9 to 14 percent total-time
  reduction for placement B. The writer hold grows to nearly the whole parse.
  The research table's 5k values, 207.0 versus 149.3 ms, do not support the
  stated percentage range. Treat that interpretation as reported, not a
  universal winner or a substitute for the measured ranges.
- The original stable/all-racy claim says the difference is less than 4 ms at
  both sizes. Its own 5k values are 159.1 versus 132.6 ms, which contradict that
  claim. The 50k values are 1,953.0 versus 1,949.0 ms. These are one-run
  observations, not evidence that hashing costs are equivalent.
- At 50k, clean replay is 4,264.3 versus 8,061.0 ms for parse fallback. This
  saves about 47 percent of that fallback in the reported comparison. Decode
  plus replay runs about 60 times the historical 53.3 ms comparison. The old
  answer calls this a "write budget" and rejects replay on that basis. That
  inference is unsupported: 53.3 ms is ticket 26's row-copy/encode comparison,
  not a universal recovery-latency limit. Compare recovery against the matched
  fallback, with all costs counted as section D specifies.

### Requirements that the evidence does not close

The original conclusion says correctness gates closed except the Linux run and
torn-write ordering. The corrected protocol closed the Linux run, the 20-run
repetition rule, the equal-work placement comparison and native durability
evidence. These requirements remain visible:

- Torn-write ordering needs a block-level injector. Saved images and process
  stops do not establish arbitrary power-loss ordering.
- The full decided eleven-facet layout, notices, and the 85-percent/2-times
  layout claims remain unproved.
- Whole-blob versus streaming framed facts and incremental rebuild savings
  remain unproved.
- The 50-note anchor sits outside the measured matrix.
- The stop-window rates, per-phase break-even rates and the numeric host
  freshness proposal remain requirements below.

No new experiment is part of this document revision beyond the corrected Linux
protocol and `ENOSPC` probe reported above. The provisional protocol below is a
design for the throwaway prototype, not proof that every rule was implemented
or checked.

## Follow-on purpose and boundaries

Ticket [What does a durable index repair journal cost and
save?](./26-measure-index-recovery-journal.md) measured the cost screen. The
external facts journal survived the screen at the loose upper bound. This ticket
builds the real replay prototype and measures the real saved work. Do not change
production code. This ticket carries the full specification that ticket 26 moved
out of its body. The maintainer decides adoption after this ticket.

## A. Replay prototype

### Payload and saved-work accounting

- Encode parsed facts from the real parser. Do not copy the row bytes.
- Record the encode cost. Compare it with the 53.3 ms row-copy bound.
- Derive links, tags, classes, range keys and notices from the recorded facts at
  replay.
- Measure the derivation cost. The screen replayed prebuilt rows only.
- Carry the full journal content list from ticket 26. It adds the old derived
  keys and the link edges of unchanged files that a change affects.
- Delete the old derived keys that the journal records. Ticket 24 requires the
  deletion.
- Count the replay persist in the saved-work bound. The screen bound counted the
  rival persist as saved, which a replay still pays.
- Use the external file, the writer base check and the store identity from
  tickets 24 and 25.

### Durability and measured costs

- Remove the journal only after an error-reporting `Immediate` commit returns
  `Ok`. The commit can apply or replay the target. A checkpoint also qualifies
  after the writer checks that the stored target matches the journal.
- Check the durability barriers:
  - Kill after the journal sync.
  - Reopen the store.
  - Check the replay barrier.
  - Run a test of the normal `Immediate` apply barrier.
- Use a fixture with about 0.75 tasks per note. Count the tasks in it. Ticket 26
  reports 1 task per note. Check the actual ratio.
- Measure the journal decode and replay times.
- Measure the store open, commit, barrier and close times.
- Measure the redb repair time after each class of unclean stop.
- Measure the journal bytes by facet, the store size before and after, and peak
  memory.
- Measure the store-handle hold time. A second process gets an open error while
  redb holds the file. Measure its actual query fallback. Report harness retry
  time separately.
- Report the cost without a failure. Every journaled write pays this cost.
- Report the buffered-control cost and window separately.

## B. Fault tests

Run these tests after the replay prototype works. Separate two failure
classes:

- **Process stop.** Run tests with `SIGKILL`, `abort`, a panic and `Ctrl-C`. The
  shipped release profile uses `panic = "abort"`, so a release panic does not
  run `drop`. Label an unwind build as a non-production control. Record what
  runs `drop` in each case. An out-of-memory kill is an untested limit.
- **System crash or power loss.** The operating system can lose pages that were
  not synced.

### Process-stop locations

Stop the process at each of these points:

- Before the journal write completes. After it completes, before the journal
  sync.
- After the journal sync and before journal publication.
- After journal publication and before the store commit. Reopen the store. Read
  the journal to check that the synced bytes survived.
- Before and after the error-reporting store durability barrier.
- Before the journal removal and after it. Use a named stop point because the
  selected removal has no directory sync. Restore a removed journal from a saved
  image. Check the writer base after the restore.
- In replay, before and after the replay commit. Check that replay is safe to
  repeat.
- In the middle of a full rebuild.

### System-crash simulation

You cannot cut the power. Simulate a system crash through a fault-injecting file
layer and saved file images. Record the method and its limits. Do not claim that
simulation equals power loss.

Run these system-crash cases:

- Lose the last `Durability::None` store commit.
- Keep the journal directory entry. Lose the journal data.
- Lose or keep the store commit while the journal survives.
- Leave the journal after the durable store commit.

### Source and identity changes

Change the input before replay:

- Change a source file that the journal does not list.
- Change a file that the journal lists. Do not keep a stale inbound link, notice
  or range key.
- Add a file. Delete a file. Rename a file.
- Change a file to the same size and `mtime`, inside the racy window. Compute
  the `racy` bit from the original scan start, not the replay time.
- Change the policy.
- Change the parts of the store identity in separate tests. The parts include
  the engine tag, platform family, incarnation and revision.
- Use a lower format epoch and a higher epoch. Use a changed extractor
  fingerprint. Do not replay a journal with a higher epoch. Do not delete it.

Discard the whole journal when a source, policy, writer-base or
compatible-identity check fails. Keep a higher-epoch journal untouched.

Validate a copied or restored vault from its store identity, writer base, policy
and complete source snapshot. Do not reject a copy only because its path
changed. An unchanged Unix copy can change inode-change times. A snapshot
mismatch then discards the journal and starts a fresh parse.

### Storage and hostile-journal failures

Run tests with these storage failures:

- Make the store directory unwritable. Run the command more than once. Make sure
  the journal does not grow or repeat work without a limit.
- Open a read-only vault.
- Fill the disk during the journal write.
- Corrupt or truncate the journal.
- Delete the store but keep the journal. Check the query against a fresh parse.
- Fill the disk after the journal publication and before the store commit.
  Compare this failure with a write that has no journal. Report any new store
  write error as an adoption blocker.
- Run the hostile-journal cases:
  - Place a symlink, a FIFO, an oversized file and a false length at the journal
    path.
  - Do not follow a symlink.
  - Do not block.
  - Do not allocate without a bound.

A damaged journal must not abort a release-profile child.

Ignore the journal facts if an optional journal write, sync or decode fails.
Redo from source files. Do not change the index read and write error rules.

### Concurrent writers

Run these concurrent cases:

- Start a second process while the first holds the store or the journal:
  - Run a case with a journal from a live writer.
  - Run another case with a journal from a dead writer.
  - Measure its actual query fallback.
  - Report a harness retry wait separately.
  - Never let two processes replay one journal.

  redb rejects the second open.
- Drop the store handle after a failed commit. Let another writer act after you
  drop the handle. Check the writer base again before cleanup. A matching target
  revision alone does not identify the writer.

### Uncertain commit

Treat a failed redb commit as an uncertain outcome:

- Drop the handle. Reopen the store. Read its incarnation and revision.
- Return the existing write error. Do not retry in the same command.
- Check the source snapshot and the base before replay in the next command.
  Discard the journal if the compatible base changed. Refresh from source in
  that case.
- Keep a higher-epoch journal untouched. Ticket 25 drops the 128-bit
  `commit_nonce`. Do not use it.

## C. Correctness and platforms

- Compare every recovered query against a trusted fresh parse.
- Change the revision at a replay. Each query result names the incarnation and
  the revision. Ticket 28 sets the generation pair.
- Cover all facets and range keys. Reject partial or mixed-policy data.
- Exclude everything under `.traces/` from the source scan. Ticket 25 sets this
  rule.
- Sync the journal file at publication. Sync its parent directory. Measure the
  two syncs separately. On macOS this is `F_FULLFSYNC`. The selected removal
  unlinks without a directory sync. Windows cannot sync a directory. Do not
  claim portability across file systems.
- Run the fault tests on Linux at least once. Record the sync-call differences
  between macOS and Linux.
- Use the matched macOS release-profile runs as the primary latency evidence.
  Ticket 26 used `opt-level = 3`. Keep its numbers as historical bounds.

## D. Recovery queries and decision input

### Query scenarios and controls

- Measure the first usable query and repeated queries per scenario and size.
- Run the scenarios on the cold CLI and the simulated host. Label the host model
  as a simulation. Ticket 33 validates the host retry path against these
  numbers.
- Measure the no-change control with a read-only store open, both with and
  without a retained journal. A read-only handle cannot coexist with a writable
  handle. A pure read-only operation never opens the journal.
- Run the scenarios from ticket 26 step 3:
  - No change.
  - A one-note edit.
  - A 100-note edit.
  - An added, deleted and renamed file with inbound links.
  - A policy change that needs a rebuild.
- Use the rich fixture builders and a hub-linked fixture for inbound links:
  - Record the actual task count.
  - Use a throwaway implementation of the decided eleven-facet layout for the
    layout claims.
  - Do not use a round-robin row copy.
- Compare full recovered queries at matched stop points, not raw-row reads:
  - Record the test-only replay outcome and the source parse count.
  - Make a no-replay control fail the fault tests.
  - Check that the next refresh has an empty delta after a replay.
- Use the anchors 50, 5,000 and 50,000 notes. Extend near crossovers.
- Check the untested claims of 85 percent more write time and 2 times the file
  size. Use the decided layout with `lists` and the derived `tasks` facet.
  Measure the task-only scan, the full query rows and a one-note replacement.

### Cost accounting and handoffs

- Measure the real saved-work bounds:
  - Count replay persistence, derivation, decode, source checks and redb repair.
  - Count the target-state comparison, checkpoint and later cleanup of a retained
    journal against its original write.
  - Replace the provisional break-even rates from the screen.
- Report the measured stop-window rates for incremental edits:
  - Calculate a break-even rate per phase.
  - Never combine the rates.
  - Do not invent a stop-time distribution or a failure rate.
  - Report no proved ranking if cost and benefit ranges overlap.
- Propose a numeric host staleness bound with an explicit freshness target after
  measurement. Ticket 33 checks it against the real host.
- Record the method, the results and the limits under an `## Answer` heading.
  Link the research asset from this ticket.
- Hand the result to [ticket
  36](./36-reconcile-decisions-and-audit-coverage.md).
- Ticket 36 owns the `None` versus `Immediate` choice.
- The maintainer decides if the journal enters the specification.

## E. Specify these rules

Specify the detailed journal rules after the prototype survives the fault tests.
Ticket 26 records only the barrier rule.

### Location, exposure and file identity

- Where the journal stores vault-derived facts.
- Who can read the journal, the file mode, and the sync tools or backups that
  copy it. The store holds the same facts today. Compare the exposure with the
  store.
- The journal name. Ticket 25 puts the engine tag in the cache file name.
- The path form in the journal. Ticket 32 owns the path model.
- The behavior-change table row for the journal file. Ticket 16 requires a row
  for a new file on disk.

### Cleanup, reporting and ownership

- The cleanup rule for a stale journal and an orphan journal. Ticket 25 leaves
  the removal of retired cache files open. Hand the retired-engine decision to
  ticket 36. Do not delete a journal with a higher epoch.
- The behavior when the journal is absent.
- What each path reports after a replay, a discard or a failure. Ticket 34 owns
  the notices.
- Whether the redb adapter can own the journal without a new domain port
  operation. Ticket 24 keeps durability and locking inside each adapter.
- Whether another engine already has a log such as SQLite WAL.
- The new states and failure modes that the journal adds.
- The error-reporting barrier that permits journal removal. An `Immediate` apply
  or replay of the target qualifies. A checkpoint qualifies only after the
  writer checks the stored target against the journal.
- The behavior when journal publication fails or leaves an orphan.

## Method

- Use the shipped release profile for matched timings: `opt-level = "z"` and
  `panic = "abort"`.
- Do not set `RUSTFLAGS`.
- Run Criterion with that profile, not the default `bench` profile with
  `opt-level = 3`.
- Run measurements in series. Do not run measurements in parallel.
- Use a child-process harness for cold starts and failure tests.
- Run a cold case at least 20 times. Report the median and the range.
- Use an A-B-A control to measure noise.
- Do not rank paths when the ranges overlap.

Use `benches/common/cache.rs` (`drop_page_cache`) for the cold-cache state. On
macOS it is a hint for a single file. It is not a full purge. Record the cache
state and the limit of the method.

Measure latency on macOS/APFS. Use a Podman Linux VM for the Linux correctness
checks on VM-local storage. Do not use host bind mounts.

The original plan specifies a VM with 2 GiB of memory. The actual Linux evidence
above uses 4 GiB.

redb defaults to a 1 GiB cache. Ticket 33 measured 467 MB for a pinned
50,000-note generation. Report peak RSS for every phase. Record a failed
50,000-note run as a limit. Do not treat a VM result as native-Linux latency.

Record these facts in the research asset:

- The fixture and the seed.
- The measurement range.
- The cache state.
- The Git commit, the operating system, the file system, the disk, the CPU and
  the tool versions.
- The research asset name: `../research/27-index-replay-prototype.md`.

Link the asset from this ticket.

## Inputs from earlier tickets

- [Ticket 26](./26-measure-index-recovery-journal.md) and [its
  measurement](../research/26-index-recovery-journal-measurement.md): the screen
  numbers, the barrier rule, and the kill-sweep method.
- [Ticket 24](./24-index-read-model-and-query-execution.md): the writer base
  check and the facet store shape.
- [Ticket 25](./25-index-freshness-and-invalidation.md): store identity fields
  and the racy bit rule.
- [Ticket 28](./28-live-buffer-and-persisted-index-consistency.md): the
  generation pair and the revision rule at a replay.
- [Ticket 33](./33-workspace-host-and-composition.md): the host retry path that
  validates against these numbers.

## Provisional grilling decisions (2026-11-10)

These decisions configure the throwaway prototype. They do not prove that the
journal is safe or faster. Do not change production code. The maintainer decides
adoption after the fault tests and measurements.

### Protocol and ownership (design)

- Compare the journaled `Immediate` normal write, a deferred `None` normal
  write, and a no-journal `Immediate` write. Keep the buffered control separate.
- Keep a single deferred journal until an error-reporting `Immediate` checkpoint
  or replay:
  - Compare the stored target against the journal under the writer handle before
    a checkpoint.
  - Resolve the journal before the next publication.
  - Do not use a silent database close as the barrier.
- Use a fixed temp name beside the engine-specific journal name under
  `.traces/`:
  - Hold the store writer handle before publication or replay.
  - Sync the temp file.
  - Rename it.
  - Sync the parent directory.
- Make the header hold a magic, version, identity, size and a checked header
  checksum:
  - Check the full body with a checksum.
  - Reject bad sizes before allocation.
  - Compare a whole postcard blob with streaming framed facts for full rebuilds.
  - Do not apply a partial journal.
- Screen both full-rebuild payloads from ticket 26. Direct facets are a cost
  control only. The candidate replay derives facets from parsed facts.
- Check the complete source path set and metadata against the original scan
  start. Record content digests for racy files, including unchanged files.
  Discard the whole journal on any failed source or identity check.
- Recheck the store incarnation and revision inside the replay transaction.
  Delete old derived keys before upserts. Apply all facets and the new revision
  in one transaction.
- Unlink only after the qualifying `Immediate` commit returns `Ok`. Do not sync
  the directory after unlink. A returned stale journal must fail the writer base
  check.
- A pure read-only operation does not open the journal. A command that enters
  writable refresh can replay before it publishes a query.
- The redb adapter owns the journal without a new domain port. Use the store's
  current platform-bound path form. Ticket 32 owns a portable path form.
- Create the temp journal with owner-only permissions (`0600`). A backup or sync
  tool that runs as another user can omit it. Compare that exposure with the
  existing store file.

### Failure decisions (design)

- A current-engine writer cleans its safe stale and orphan files after the
  identity checks. Leave retired-engine files for the ticket 36 decision.
  Preserve a higher-epoch or unreadable-version journal. Skip optional journaling
  if the final name stays occupied. Use the normal source path in that case.
- An optional journal failure does not change the store error rules. A replay
  and a normal journal discard are quiet. Journal corruption alone gives no
  notice. The existing notice applies to an actual cache-read fallback or a
  higher-epoch store. Ticket 34 owns the text.
- Close the store after a failed commit. Reopen it before reading the identity.
  - Return the existing error. Do not retry in the same command.
  - The next writer checks the source and the base again. A matching target
    revision alone does not prove that this writer committed.
- redb rejects a second process at open while a writer holds the file. Measure
  the real query fallback. Label any harness retry wait separately.
- A copy does not fail merely because its root path changed. A changed
  inode-change time on a Unix copy can fail the source snapshot check. Discard
  the journal in that case. Parse fresh.

### Required proof before adoption

- Add named failpoints for partial writes, syncs, rename, commit and unlink:
  - Keep injector logs out of the child's diagnostic stream.
  - Use a Linux injector for the Linux fault run.
  - State that saved images do not equal real power loss.

  The old macOS sync shim alone cannot reach these stop points.
- Check the full query rows, the test-only replay outcome, the parse count and
  the next empty refresh. A no-replay control must fail the replay rows.
- Run a test with a same-incarnation and same-revision store fork with different
  affected rows. Report a failed identity proof as a protocol failure. Do not
  restore the dropped `commit_nonce` without a separate decision.
- Reject a symlink, FIFO, oversized file or false journal length:
  - Do not open a symlink target.
  - Do not wait on a FIFO.
  - Do not abort.
  - Use release-profile child processes.

  `src/index/store.rs:366` is `#[cfg(test)]`, not a probe API.
- Run a test with a journal that uses space needed by the later store write.
  Compare its error with a no-journal write. Treat a new store-write failure as
  an adoption blocker, not as an optional journal error.
- Run tests with stable and all-racy 50,000-note fixtures:
  - Measure both source-check placements against total latency and writer hold
    time.
  - Keep every cost in its matched release-profile comparison.
  - Report no proved winner if the ranges overlap.

  Ticket 36 decides adoption.
