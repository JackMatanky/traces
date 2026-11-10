# Does the index replay prototype survive fault tests, and what does recovery cost?

Type: task
Status: resolved
Blocked by: none

## Question

Ticket [What does a durable index repair journal cost and
save?](./26-measure-index-recovery-journal.md) measured the cost screen. The
external facts journal survived the screen at the loose upper bound. This ticket
builds the real replay prototype and measures the real saved work. Do not change
production code. This ticket carries the full specification that ticket 26 moved
out of its body. The maintainer decides adoption after this ticket.

### A. Replay prototype

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
- Remove the journal only after an error-reporting `Immediate` commit returns
  `Ok`. The commit can apply or replay the target. A checkpoint also qualifies
  after the writer checks that the stored target matches the journal.
- Kill after the journal sync. Reopen and check the replay barrier. Also test
  the normal `Immediate` apply barrier.
- Use a fixture with about 0.75 tasks per note. Count the tasks in it.
  Ticket 26 reports 1 task per note. Check the actual ratio.
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

### B. Fault tests

Apply these tests after the replay prototype works. Separate two failure
classes:

- **Process stop.** Test `SIGKILL`, `abort`, a panic and `Ctrl-C`. The shipped
  release profile uses `panic = "abort"`, so a release panic does not run
  `drop`. Label an unwind build as a non-production control. Record what runs
  `drop` in each case. An out-of-memory kill is an untested limit.
- **System crash or power loss.** The operating system can lose pages that were
  not synced.

Stop the process at each of these points:

- Before the journal write completes. After it completes, before the journal
  sync.
- After the journal sync and before journal publication.
- After journal publication and before the store commit. Reopen and read the
  journal to check that the synced bytes survived.
- Before and after the error-reporting store durability barrier.
- Before the journal removal and after it. Use a named stop point because the
  selected removal has no directory sync. Restore a removed journal from a
  saved image. Check the writer base after the restore.
- In replay, before and after the replay commit. Check that replay is safe to
  repeat.
- In the middle of a full rebuild.

You cannot cut the power. Simulate a system crash through a fault-injecting file
layer and saved file images. Record the method and its limits. Do not claim that
simulation equals power loss.

Test these system-crash cases:

- Lose the last `Durability::None` store commit.
- Keep the journal directory entry. Lose the journal data.
- Lose or keep the store commit while the journal survives.
- Leave the journal after the durable store commit.

Change the input before replay:

- Change a source file that the journal does not list.
- Change a file that the journal lists. Do not keep a stale inbound link, notice
  or range key.
- Add, delete and rename a file.
- Change a file to the same size and `mtime`, inside the racy window. Compute
  the `racy` bit from the original scan start, not the replay time.
- Change the policy.
- Change the parts of the store identity in separate tests. The parts include
  the engine tag, platform family, incarnation and revision.
- Use a lower format epoch and a higher epoch. Use a changed extractor
  fingerprint. Do not replay or delete a journal with a higher epoch.

Discard the whole journal when a source, policy, writer-base or
compatible-identity check fails. Keep a higher-epoch journal untouched. Validate
a copied or restored vault from its store identity, writer base, policy and
complete source snapshot. Do not reject a copy only because its path changed.
An unchanged Unix copy can change inode-change times. A snapshot mismatch then
discards the journal and starts a fresh parse.

Test the storage failures:

- Make the store directory unwritable. Run the command more than once. Make sure
  the journal does not grow or repeat work without a limit.
- Open a read-only vault.
- Fill the disk during the journal write.
- Corrupt or truncate the journal.
- Delete the store but keep the journal. Check the query against a fresh parse.
- Fill the disk after the journal publication and before the store commit.
  Compare this failure with a write that has no journal. Report any new store
  write error as an adoption blocker.
- Place a symlink, a FIFO, an oversized file and a false length at the journal
  path. Do not follow, block or allocate without a bound. A damaged journal
  must not abort a release-profile child.

If an optional journal write, sync or decode fails, ignore its facts and redo
from source files. Do not change the index read and write error rules.

Test the concurrent cases:

- Start a second process while the first holds the store or the journal. Test a
  journal from a live writer and one from a dead writer. redb rejects the
  second open. Measure its actual query fallback. Report a harness retry wait
  separately. Never let two processes replay one journal.
- Drop the store handle after a failed commit, then let another writer act.
  Check the writer base again before cleanup. A matching target revision alone
  does not identify the writer.

Treat a failed redb commit as an uncertain outcome:

- Drop the handle and reopen the store. Read its incarnation and revision.
- Return the existing write error. Do not retry in the same command.
- In the next command, check the source snapshot and the base before replay.
  If the compatible base changed, discard the journal and refresh from source.
- Keep a higher-epoch journal untouched. Ticket 25 drops the 128-bit
  `commit_nonce`. Do not use it.

### C. Correctness and platforms

- Compare every recovered query against a trusted fresh parse.
- Change the revision at a replay. Each query result names the incarnation and
  the revision. Ticket 28 sets the generation pair.
- Cover all facets and range keys. Reject partial or mixed-policy data.
- Exclude everything under `.traces/` from the source scan. Ticket 25 sets this
  rule.
- Sync the journal file and its parent directory at publication. Measure the
  two syncs separately. On macOS this is `F_FULLFSYNC`. The selected removal
  unlinks without a directory sync. Windows cannot sync a directory. Do not
  claim portability across file systems.
- Run the fault tests on Linux at least once. Record the sync-call differences
  between macOS and Linux.
- Use the matched macOS release-profile runs as the primary latency evidence.
  Ticket 26 used `opt-level = 3`. Keep its numbers as historical bounds.

### D. Recovery queries and decision input

- Measure the first usable query and repeated queries per scenario and size.
- Run the scenarios on the cold CLI and the simulated host. Label the host model
  as a simulation. Ticket 33 validates the host retry path against these
  numbers.
- Measure the no-change control with a read-only store open, both with and
  without a retained journal. A read-only handle cannot coexist with a writable
  handle. A pure read-only operation never opens the journal.
- Run the scenarios from ticket 26 step 3: no change, a one-note edit, a
  100-note edit, an added, deleted and renamed file with inbound links, and a
  policy change that needs a rebuild.
- Use the rich fixture builders and a hub-linked fixture for inbound links.
  Record the actual task count. Use a throwaway implementation of the decided
  eleven-facet layout for the layout claims. Do not use a round-robin row copy.
- Compare full recovered queries at matched stop points, not raw-row reads.
  Also record the test-only replay outcome and the source parse count. Make a
  no-replay control fail the fault tests. Check that the next refresh has an
  empty delta after a replay.
- Use the anchors 50, 5,000 and 50,000 notes. Extend near crossovers.
- Check the untested claims of 85 percent more write time and 2 times the file
  size. Use the decided layout with `lists` and the derived `tasks` facet.
  Measure the task-only scan, the full query rows and a one-note replacement.
- Measure the real saved-work bounds. Count replay persistence, derivation,
  decode, source checks and redb repair. Count the target-state comparison,
  checkpoint and later cleanup of a retained journal against its original
  write. Replace the provisional break-even rates from the screen.
- Report the measured stop-window rates for incremental edits. Calculate a
  break-even rate per phase. Never combine the rates. Do not invent a stop-time
  distribution or a failure rate. If cost and benefit ranges overlap, report
  no proved ranking.
- After measurement, propose a numeric host staleness bound with an explicit
  freshness target. Ticket 33 checks it against the real host.
- Record the method, the results and the limits under an `## Answer` heading.
  Link the research asset from this ticket.
- Hand the result to [ticket
  36](./36-reconcile-decisions-and-audit-coverage.md).
- Ticket 36 owns the `None` versus `Immediate` choice.
- The maintainer decides if the journal enters the specification.

### E. Specify these rules

Specify the detailed journal rules after the prototype survives the fault tests.
Ticket 26 records only the barrier rule.

- Where the journal stores vault-derived facts.
- Who can read the journal, the file mode, and the sync tools or backups that
  copy it. The store holds the same facts today. Compare the exposure with the
  store.
- The journal name. Ticket 25 puts the engine tag in the cache file name.
- The path form in the journal. Ticket 32 owns the path model.
- The behavior-change table row for the journal file. Ticket 16 requires a row
  for a new file on disk.
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
- The error-reporting barrier that permits journal removal. An `Immediate`
  apply or replay of the target qualifies. A checkpoint qualifies only after
  the writer checks the stored target against the journal.
- The behavior when journal publication fails or leaves an orphan.

## Method

Use the shipped release profile for matched timings: `opt-level = "z"` and
`panic = "abort"`. Do not set `RUSTFLAGS`. Run Criterion with that profile,
not the default `bench` profile with `opt-level = 3`. Run measurements in
series. Do not run measurements in parallel.

Use a child-process harness for cold starts and failure tests. Run a cold case
at least 20 times. Report the median and the range. Use an A-B-A control to
measure noise. Do not rank paths when the ranges overlap.

Use `benches/common/cache.rs` (`drop_page_cache`) for the cold-cache state. On
macOS it is a hint for a single file. It is not a full purge. Record the cache
state and the limit of the method.

Measure latency on macOS/APFS. Use a Podman Linux VM for the Linux correctness
checks on VM-local storage. Do not use host bind mounts.

The VM has 2 GiB of memory. redb defaults to a 1 GiB cache. Ticket 33
measured 467 MB for a pinned 50,000-note generation. Report peak RSS for
every phase. Record a failed 50,000-note run as a limit. Do not treat a VM
result as native-Linux latency.

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

### Protocol and ownership

- Compare the journaled `Immediate` normal write, a deferred `None` normal
  write, and a no-journal `Immediate` write. Keep the buffered control separate.
- Keep a single deferred journal until an error-reporting `Immediate`
  checkpoint or replay. Before a checkpoint, compare the stored target
  against the journal under the writer handle. Resolve the journal before
  the next publication. Do not use a silent database close as the barrier.
- Use a fixed temp name beside the engine-specific journal name under
  `.traces/`. Hold the store writer handle before publication or replay.
  Sync the temp file, rename it, and sync the parent directory.
- Make the header hold a magic, version, identity, size and a checked header
  checksum. Check the full body with a checksum. Reject bad sizes before
  allocation. Compare a whole postcard blob with streaming framed facts for
  full rebuilds. Do not apply a partial journal.
- Screen both full-rebuild payloads from ticket 26. Direct facets are a cost
  control only. The candidate replay derives facets from parsed facts.
- Check the complete source path set and metadata against the original scan
  start. Record content digests for racy files, including unchanged files.
  Discard the whole journal on any failed source or identity check.
- Recheck the store incarnation and revision inside the replay transaction.
  Delete old derived keys before upserts. Apply all facets and the new revision
  in one transaction.
- Unlink only after the qualifying `Immediate` commit returns `Ok`. Do not
  sync the directory after unlink. A returned stale journal must fail the
  writer base check.
- A pure read-only operation does not open the journal. A command that enters
  writable refresh can replay before it publishes a query.
- The redb adapter owns the journal without a new domain port. Use the store's
  current platform-bound path form. Ticket 32 owns a portable path form.
- Create the temp journal with owner-only permissions (`0600`). A backup or
  sync tool that runs as another user can omit it. Compare that exposure with
  the existing store file.

### Failure decisions

- A current-engine writer cleans its safe stale and orphan files after the
  identity checks. Leave retired-engine files for the ticket 36 decision.
  Preserve a higher-epoch or unreadable-version journal. If the final name
  stays occupied, skip optional journaling and use the normal source path.
- An optional journal failure does not change the store error rules. A replay
  and a normal journal discard are quiet. Journal corruption alone gives no
  notice. The existing notice applies to an actual cache-read fallback or a
  higher-epoch store. Ticket 34 owns the text.
- On a failed commit, close and reopen before reading the identity. Return the
  existing error. Do not retry in the same command. The next writer checks the
  source and the base again. A matching target revision alone does not prove
  that this writer committed.
- redb rejects a second process at open while a writer holds the file. Measure
  the real query fallback. Label any harness retry wait separately.
- A copy does not fail merely because its root path changed. A changed
  inode-change time on a Unix copy can fail the source snapshot check. Discard
  and parse fresh in that case.

### Required proof before adoption

- Add named failpoints for partial writes, syncs, rename, commit and unlink.
  The old macOS sync shim alone cannot reach these stop points. Keep injector
  logs out of the child's diagnostic stream. Use a Linux injector for the
  Linux fault run. State that saved images do not equal real power loss.
- Check the full query rows, the test-only replay outcome, the parse count and
  the next empty refresh. A no-replay control must fail the replay rows.
- Test a same-incarnation and same-revision store fork with different affected
  rows. Report a failed identity proof as a protocol failure. Do not restore
  the dropped `commit_nonce` without a separate decision.
- Reject a symlink, FIFO, oversized file or false journal length. Do not
  open a symlink target, wait on a FIFO or abort. Use release-profile child
  processes. `src/index/store.rs:366` is `#[cfg(test)]`, not a probe API.
- Test a journal that uses the space that the later store write needs. Compare
  its error with a no-journal write. Treat a new store-write failure as an
  adoption blocker, not as an optional journal error.
- Test stable and all-racy 50,000-note fixtures. Measure both source-check
  placements against total latency and writer hold time. Keep every cost in
  its matched release-profile comparison. If the ranges overlap, report no
  proved winner. Ticket 36 decides adoption.

## Comments

### Re-targeted probe evidence (2026-11-10)

The [prototype evidence](../research/27-index-replay-prototype.md) now
records the re-targeted probe. The probe journals and replays the real
`.traces/index.redb` layout and verifies full `QueryRow` accessors
against a fresh-parse store. The macOS release suite passed 42 of 42
tests with redb 4.3.0. A Linux run passed the earlier 25 tests with
pinned redb 4.3.0 after the VM memory rose to 4 GiB.

New evidence closes several grilling gates:

- The fork hole is proved. With the base check weakened to incarnation
  plus revision, a same-revision forked journal is wrongly accepted.
  The ticket-25 contract needs the digest backstop.
- Every decided-identity mutation is handled. Read-only vault and handle
  never open the journal. `cp -R` reparses and `mv` replays.
- `SIGKILL` and `SIGINT` match the simulated stops. A hostile
  checksum-valid body is rejected without a large allocation.
- Real `ENOSPC` in a 64 MiB `tmpfs` container produced no catchable
  error. redb reports success while the store stays sparse and loses the
  new rows silently. The journal-induced and no-journal surfaces match,
  but both are silent loss. This is a new adoption blocker for
  ticket 36 to weigh.

The layout probe's one-run observations stay in the research asset for
method record only.

### Measurement evidence and answer (2026-11-10)

The matched harness ran the full matrix on macOS with the release
profile. Policy comparisons take 20 A-B-A runs at 5,000 and 50,000
notes. Placement and scenario cases take 5 runs. No-op controls were
clean. Highlights:

- Journal encode is about 0.3 ms, far under the 53.3 ms budget. The
  journaled `Immediate` surcharge is sync cost, not encode: about 490 ms
  at 5k and 5.5 s at 50k.
- No policy dominates. No-journal `Immediate` wins the write path. A
  retained journal wins the 50k read-only no-change refresh (4,315.3 ms
  versus 6,329.0 ms) by shifting cost to the next write.
- Placement B cuts total time 9 to 14 percent but grows the writer hold
  to nearly the whole parse. Stable and all-racy snapshot hashing
  differs by less than 4 ms.
- At 50k, clean replay saves about 47 percent of the parse fallback
  (4,264.3 versus 8,061.0 ms), but decode plus replay itself runs about
  60 times the 53.3 ms write budget.

The correctness gates closed except two recorded limits: the Linux run
of the full 42-test suite, and torn-write ordering, which needs a
block-level injector. Real `ENOSPC` under `Durability::None` is silent
sparse-write loss with no catchable error on both surfaces, a new
adoption blocker. The numeric freshness bound goes to ticket 33 with
this data.

## Answer

The prototype survives the fault suite: 42 of 42 release-profile tests
on macOS, including the fork-hole proof, identity mutations, signal
stops, hostile-body rejection, and copy-versus-move. Recovery costs are
measured end to end. The evidence does not support adoption. Replay
decode exceeds the write
budget about 60-fold at 50,000 notes. No journal policy beats
no-journal `Immediate` on the write path. Real `ENOSPC` is silent loss
on this stack. Ticket 36 weighs these results;
ticket 33 gets the freshness data. The two open limits are the Linux
42-test run and torn-write ordering, both marked in the research asset.
