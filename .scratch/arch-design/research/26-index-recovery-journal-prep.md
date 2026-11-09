# Research for [What does a durable index repair journal cost and save?](../issues/26-measure-index-recovery-journal.md)

This note records evidence and tentative grilling decisions.
It does not choose a journal.
[V] marks checked evidence. [H] marks a limitation or inference. [P] marks
a candidate or a tentative answer.

The probe in section D is a throwaway program. It is not a benchmark. Its source
is in [26-prep/redb-probe.rs.txt](26-prep/redb-probe.rs.txt). The sync shim is
in [26-prep/dyld-sync-shim.c.txt](26-prep/dyld-sync-shim.c.txt).

The repository code is `main` at `79f393a2`.
The diff since `207cc3fb` includes comments,
tests, one folder-key separator fix and one scanner call.
Check all index-affecting changes again before the measurements.

## Summary

- [V] A `Durability::None` commit survives a normal `Database::drop` in the
  probe. A stop that skips `drop` loses it. The close routine can fail without
  returning an error. It skips its final write during a panic.
- [V] Where the file system keeps its sync promise, redb returns the last
  durable commit after a stop. A journal can save work. It does not make an
  already consistent store more consistent.
- [V] A journal helps after its file and directory become durable.
  The window ends at a durable store commit.
  With `None`, the CLI holds the store through the query before `drop`.
  With `Immediate`, the window ends when that explicit commit succeeds.
- [V] On macOS, Rust `sync_all` and `sync_data` call `F_FULLFSYNC`.
  The file probe took about 4 ms to write and sync a small journal.
  Removal and directory sync took about 3 ms.
  The `Immediate` commit added 0 to 9 ms in order-dependent runs.
  A buffered write took 0.1 ms without encoding or safe publication.
- [H] The sync shim counts `F_FULLFSYNC` and `fsync` calls.
  It does not prove call order or durability.
- [V] The real `traces list` call made 5 `F_FULLFSYNC` calls on `index.redb`,
  even when nothing changed. They span about 14 to 16 ms. See section D5.
- [H] The `rich_realistic` build took 20 to 43 µs for each note.
  Its total includes a scan, not just parsing.
  The 100-note edit has a saved-work upper bound of 4.3 ms.
  The small-file journal probe cost about 7 ms with sync and removal.
  This rules out that measured case only if it bounds a complete replay.
  A buffered journal has unmeasured encoding, publication and cleanup costs.
- [V] The 12.0 ms no-op refresh at 50 notes is mostly a writable store open
  and close. The probe took 3.0 ms to open and 9 to 10 ms to drop.
  Section H records the earlier error that called it a scan cost.
- [H] A read-only open of the real store took 0.03 ms without a sync.
  A complete read-only query is not measured. See section D6.
- [V] The random-time kill tests covered 150 trials.
  The sync shim also killed the real binary before 5 sync calls.
  All next runs matched a fresh store in the tested cases. See D5.
- [V] Repair after an unclean stop depends on the last durable state.
  A clean close followed by a `None` commit and abort took about 7 ms.
  After `Immediate` commits and abort, repair took 33 to 101 ms.
  The journal adds no repair cost. See D3.
- [V] Git does not harden derived data by default.
  The cache tools in research 05 treat a damaged cache as a miss.
  The CLI guide in this repository says to make the program crash-only.
- [P] Use a staged rejection gate. Measure replay, encoding and safe
  cleanup before declaring that a journal can pay.
  Report `None` against `Immediate` without making the durability choice.
- [P] Screen the external journal and the in-store redb journal table.
  If a candidate passes the cost screen, open a follow-on task for the
  replay prototype, the crash tests and the recovery-query checks.

## A. Decision boundaries

- [V] [What counts as the externally observable
  surface?](../issues/16-grilling-compatibility-envelope.md) freezes index
  results against a fresh parse. A journal must obey this rule. The BDD case
  "Deleting the index store preserves query results" pins one consequence. See
  `tests/features/index/index-store.feature:17-25`.
- [V] [What is the index read model?](../issues/24-index-read-model-and-query-execution.md)
  keeps `Durability::None` and says that the store is a rebuildable cache. It
  keeps durability and locking inside the adapter (rule 5). It chose `None`.
  It did not compare the total cost of open, commit and drop with `Immediate`.
- [V] [Does the landed freshness model
  stand?](../issues/25-index-freshness-and-invalidation.md) decouples this
  ticket. It says a crash leaves a no-op refresh of the uncommitted files. It
  drops `commit_nonce` and the unwritable latch. It keeps store identity,
  the `racy` bit and the rule that only a writer that decided to rebuild may
  delete a store file.
- [V] [How should open editor buffers relate to the persisted
  index?](../issues/27-live-buffer-and-persisted-index-consistency.md) says that
  a host holds the store only during a refresh and a persist. A replay must
  change the revision of the store.
- [V] Ticket 22 gives `init` an undo journal. Ticket 20 lists a bounded undo
  journal for agent writes. Both protect vault files. This ticket concerns a
  derived cache. The two kinds must not share code or names.
- [V] [Ticket 35](../issues/35-reconcile-decisions-and-audit-coverage.md) now
  waits for this ticket. The ticket handoff adds the follow-on rule.

## B. What a stop loses

[V] One persist is one write transaction with one commit. See
`src/index/store.rs:781-813` for a rebuild and `:816-860` for an incremental
write. The commit calls are at lines 811 and 858. `configure_cache_txn` sets
`Durability::None` at line 355.

[V] `RefreshPlan::collect` opens the store and scans the vault in parallel. See
`src/index/refresh.rs:332-353`. The store is open for the whole refresh. A stop
at any point leaves the previous durable state.

| Phase | A stop here loses | What redb keeps | A journal can save |
| --- | --- | --- | --- |
| Open and scan | Nothing | Last durable state | Nothing |
| Parse | Parse work | Last durable state | Nothing. No journal exists yet |
| Store write before its durable commit | Parse work | Last durable state | Prepared work after journal publication |
| After `None` commit, before a successful durable close | The commit | Last durable state | Prepared work after journal publication |
| After a successful durable commit | Nothing | The new state | Nothing |

[V] A stop before a durable commit leaves the last durable state.
The next open can need repair. A journal cannot remove the redb repair.

[H] The journal window starts after durable publication, not after the
`None` commit. In the current CLI, `None` leaves the store open during the query.
At 20,000 notes, the shim shows a store open of 67 to 81 ms.
At 50,000 notes, it shows 152 to 198 ms. See D5.

An `Immediate` commit shortens the window after the commit.

[V] A panic after a `None` commit loses the commit.
`close_database` skips its final write while the thread panics.
See redb `src/db.rs:1967`. No code in `src/` calls `process::exit` or
`abort`, so a normal return runs `drop`. This does not prove that the
close write succeeds.

## C. What redb guarantees

[V] A non-durable commit does not reach the disk header. The probe shows that a
stop which skips `drop` loses it. In one process the design keeps the commit in
memory. After a crash the database returns to the last durable commit. It stays
consistent. See redb `docs/design.md:323-334` and `src/transactions.rs:378-385`.

[V] `Database::drop` calls `close_database`. That routine tries a durable
commit that writes the allocator state. A durable commit also persists
earlier non-durable commits. See redb `src/db.rs:1922-1955,1963-2010`.
The routine returns no `Result` and can fail to flush without notifying
the caller.

A reopen can read buffered operating-system pages.
Do not use a reopened revision to prove power-loss durability.

[V] The header holds a `recovery_required` flag. A writable open sets it and
flushes the file. A clean close clears it. After an unclean stop the next open
must repair the store. See `src/db.rs:1672`, `page_manager.rs:1458-1470` and
`header.rs:208-220`. This is why a command that writes nothing still pays about
12 ms for open and close.

[V] redb calls `File::sync_data` for every sync. See
`file_backend/optimized.rs:350`. In Rust std for Apple targets, `sync_data` and
`sync_all` both call `fcntl(F_FULLFSYNC)`. See `os_fsync` and `os_datasync` in
`library/std/src/sys/fs/unix.rs:1266-1286` of nightly-2026-09-09. The shim log
in section D5 confirms it. The 4.01 ms `Immediate` commit in research 11
is therefore a full cache flush.

[V] The repository calls no sync function. A search of `src/` for `sync_all`,
`sync_data` and `fsync` finds no call. The atomic writes in
`src/file_tracker.rs` and `src/template/writer.rs` use a temporary file and a
rename. They do not sync. A journal would add the first sync call to the
repository.

[H] The comment at `src/index/store.rs:1317-1323` says that skipping the fsync
removes the fixed cost that dominates a full persist. Research 11 refuted this
claim. The fixed cost is the store open and close. Fix the comment when the
redesign lands.

[V] Both persist arms use one transaction. A crash in the middle of a rebuild
leaves the previous durable store. A side file adds no atomicity to a rebuild.

## D. Probe results

Machine: Apple M4 Pro, macOS 27.2, APFS data volume. Build: release,
`redb = "=4.3.0"`, Rust 1.98.0. Each timing is the median of 20 runs. There is
no A-B-A control and no warm-up study. The run-to-run noise was large. Research
11 measured 11.4 ms for `create`. This probe measured 20 ms. Treat each number
as indicative.

### D1. Which commit survives a stop

The probe writes 1,000 rows with `Immediate`. It then commits one row and ends
the process in one of three ways. A second process opens the file.

| Commit | End of process | Last commit survives | Next open |
| --- | --- | --- | ---: |
| `None` | normal `drop` | yes | 5.6 ms |
| `None` | `process::exit(0)` | no | 27.7 ms |
| `None` | `process::abort()` | no | 20.7 ms |
| `Immediate` | normal `drop` | yes | 10.3 ms |
| `Immediate` | `process::exit(0)` | yes | 19.0 ms |
| `Immediate` | `process::abort()` | yes | 23.9 ms |

[V] The open times come from one run. They show that an unclean open costs
more than a clean one. They do not give a stable ratio.

### D2. Cost of open, commit and drop

| Case | Commit | Drop | Open, commit and drop |
| --- | ---: | ---: | ---: |
| New file, `None` | 0.03 ms | 10.70 ms | 30.30 ms |
| New file, `Immediate` | 3.94 ms | 9.08 ms | 32.90 ms |
| Existing file, `None` | 0.02 ms | 17.61 ms | 21.04 ms |
| Existing file, `Immediate` | 7.03 ms | 13.03 ms | 23.12 ms |
| Existing file, no write | not run | 9.0 to 9.8 ms | open 3.0 ms |

[V] `Immediate` added 2.1 to 2.6 ms in the first run. Four more runs gave
deltas of 0.1 to 3.2 ms. An A-B-A run alternated `None`, `Immediate` and `None`
on one file, 30 times each. Three repeats gave 25.0 to 25.7 ms, 29.0 to 30.1 ms
and 20.1 to 20.8 ms. The two `None` groups differ by 4.5 ms because a run
depends on the state that the run before it leaves. So the cost of `Immediate`
is 0 to 9 ms and the data cannot narrow it. The drop after an `Immediate` commit
is shorter. The `Builder` of redb has no option that changes the sync count.
It has `set_cache_size` and `set_repair_callback` only.

[V] A writable reopen takes 3.0 ms to open and 9 to 10 ms to drop.
The sum matches the 12.0 ms no-op refresh at 50 notes in research 25,
section E.

### D3. Repair time after an unclean stop

| File | Case | Next open |
| --- | --- | ---: |
| 34 MB, 100,000 rows | clean close, then `None` commit, then abort | 6.8 ms |
| 270 MB, 500,000 rows | clean close, then `None` commit, then abort | 6.7 ms |
| 34 MB, 100,000 rows | `Immediate` commits, abort, no close | 33.1 ms |
| 270 MB, 500,000 rows | `Immediate` commits, abort, no close | 101.0 ms |
| 34 MB or 270 MB | clean reopen | 3.0 to 4.4 ms |

[V] Two repair regimes exist. After a clean close, redb has a saved allocator
state and repairs fast. Without a clean close, the repair walks the file. The
cost grows with the file. The index store at 50,000 notes is 67 MB in the
current layout and 135 MB with two range tables. See
`24-prep/g-probe-results.md:69,345-350`.

[V] The redb repair is the same for a journal path and for a redo path. A
journal cannot remove it.

### D4. Cost of a journal file

| Size | Write only | Write and sync | Write, sync and directory sync |
| ---: | ---: | ---: | ---: |
| 1 KB | 0.10 ms | 2.88 ms | 3.94 ms |
| 100 KB | 0.11 ms | 3.28 ms | 3.99 ms |
| 10 MB | 1.10 ms | 10.27 ms | 7.93 ms |
| 100 MB | 10.76 ms | 68.31 ms | 43.47 ms |

[V] Remove a 1 KB file and sync the directory: 3.05 ms.

[H] The two sync columns for 10 MB and 100 MB show noise. Their order is not
reliable. Use the range 8 to 10 ms for 10 MB and 40 to 70 ms for 100 MB.

[H] The probe does not show a Linux or a Windows result. It does not test
`F_BARRIERFSYNC` or any weaker flush. Do not use these numbers for other
platforms.

### D5. The real binary

[V] Method: a release build of `traces` at `79f393a2`. A 1,000-note vault with
one task, one tag and one link in each note. A `DYLD_INSERT_LIBRARIES` shim logs
each `fcntl(F_FULLFSYNC)` and `fsync` call. `dtruss` and `fs_usage` need
`sudo`, which was not available.

| Command | `F_FULLFSYNC` calls | Span of the calls |
| --- | ---: | ---: |
| `traces list`, no change | 5 | 14.0 to 16.5 ms |
| `traces list` after a one-note edit | 5 | 13.6 ms |

[V] All calls hit `index.redb`. A one-note edit adds no sync call. The
`Durability::None` commit adds none. The five calls come from the open, the
close commit and the flag change. A synced journal adds at least three calls:
the file, the directory and the removal.

[V] Kill test steps:

- Append one task to a note.
- Start `traces list`. Wait 0 to 30 ms. Send `SIGKILL`.
- Run `traces task` and check that the new task shows.

[V] In 150 trials the signal hit a live process 109 times. All 150 results
showed the new task. The next run took 27.9 ms at the median, 33.8 ms at the
95th percentile and 36.2 ms at most. A run with no kill and no edit took
22.1 ms. A stop costs the next run about 6 ms at the median and 14 ms at most.

[H] The vault is small. The test does not hit a chosen phase. It shows no
case where the redo path gives a wrong result. It does not show that no such
case exists.

[V] Phase-targeted kills: the shim sends `SIGKILL` just before the n-th sync
call of the real binary on a 1,000-note vault. Calls 1 to 5 cover the open and
every step of the close.

- Each case followed an edit.
- Each case ran once for an incremental write and once for a rebuild.
- All 10 next runs showed the edit and equal results from a fresh store.
- The next run took 26 to 31 ms after an incremental stop.
- The next run took 61 to 68 ms after a stop in a rebuild.

[V] A rebuild of a 50,000-note vault (store 33.7 MB, synthetic notes) made 15
sync calls in 1,552 ms. Calls 2 to 6 came in the first 19 ms. The parse ended at
about 651 ms. Calls 7 to 15 came from 651 ms to 1,516 ms. So the window after
the parse is about 0.87 s of a 1.5 s command. Kills before calls 2, 3 and 5
gave a correct next run of 1.5 to 1.6 s each, which is a full redo.

### D6. Read-only open

[V] `Builder::open_read_only` returns a `ReadOnlyDatabase`. In the probe it
opened the real `index.redb` in 0.03 ms and dropped it in 0.01 ms. A shim
log shows no sync call. The writable open takes 3.0 ms and its drop takes 9
to 10 ms.

[V] A read-only open fails with `RepairAborted` if the store needs repair. See
redb `page_manager.rs:877-879`. A caller must fall back to a writable open then.

[H] A command that finds no change could use only a read-only open. It would
skip the 12 ms of open and close and the 5 sync calls. A command that finds a
change opens the store for writing. The sequence of read-only open, drop and
writable open costs 14.5 ms. A read-only open fails with "Database already
open" while a writable handle is held, so one process cannot hold both. Ticket
25 lists a read-only vault as open. Pass this result to
tickets 25 and 32. It is not part of this ticket's decision.

## E. Cost model and break-even

[V] Let `O` be the extra cost of a journal on each normal journaled write.
At a matched stop point, let `B` be the rival's first usable query time
minus the replay query time. Compare redo and `Immediate` separately.
For one phase with positive `O` and `B`, its conditional break-even is `O / B`.
The recovery benefit changes with the stop point.
Do not combine phases without an observed stop-time distribution.

[H] Shared costs include the source scan and part of the store open.
Different paths can pay different repair, lock and barrier costs.
Do not subtract these costs without a path-pair measurement.
Parse time minus decode time is an upper bound for a direct replay.
A parsed-fact replay also reconstructs derived keys, links and notices.
These bounds do not measure a complete recovery query.

[V] The `WorkspaceIndex::build` benchmark includes scan, parse and assembly.
It does not persist. A Criterion quick run used `79f393a2`.
It gave 8.6 ms at 200 notes, 24.0 ms at 1,000 and 101.3 ms at 5,000.
That is 43, 24 and 20 µs for each note on the `rich_realistic` shape.

[H] These times include the scan and a speed-up from 12 cores.
For one note, the single-thread `parse_markdown` probe took 8 µs at 1 KB,
62 µs at 10 KB and 113 µs for a large note.
Real notes have a mean of 11 KB. The one-note saved-work bound is about
0.1 ms. The old `WorkspaceIndex::load` results date from 2026-10-18.
They show about 0.5 µs for each note above the open floor.
Journal decode and complete replay are not measured.

| Scenario | Saved work at most | Synced journal estimate | Buffered estimate | Limit |
| --- | ---: | ---: | ---: | --- |
| 1-note edit | 0.1 ms | about 7 ms | about 0.1 ms | The synced case cannot pay with these bounds |
| 100-note edit | 4.3 ms | about 7 ms | about 0.2 ms | The synced case cannot pay with these bounds |
| 5,000-note edit | 101 ms | about 14 ms | about 1.2 ms | The 14% rate is only a bound for one favorable stop phase |
| Full rebuild, 50,000 notes | about 0.65 s | about 35 to 60 ms | not measured | The 5% to 9% bounds use a guessed 30 MB journal and favorable stop phases |

[H] The estimates omit complete encoding, publication, replay and the
error-reporting barrier. The barrier is the `Immediate` replay commit.
The rebuild estimate uses the
synthetic old layout, not the decided task-heavy facets.
The journal cost is a partial estimate, not a measured lower bound.
Do not label a case viable because an upper bound on `B` exceeds `O`.
The ticket rejects a case only when a measured upper bound on `B` does not
exceed a measured lower bound on `O`.

[H] A stop before journal publication has no replay benefit.
Path timings differ. At the same elapsed stop time, a rival can already be durable.
In that phase, a journal cannot claim saved redo work.
Measure the window for each path from the same start point.
The stop-time distribution remains unknown.

[P] Stage the matrix. Measure representative edits and rebuilds, then
expand near an observed crossover. Do not add a file-count switch to
production code before the measurements.

## F. Candidate designs

| ID | Design | Protects | Added cost | Verdict |
| --- | --- | --- | --- | --- |
| P1 | Redo. This is the landed model. | Nothing extra | 0 | Baseline. It matches Git, Cargo, Ruff and the crash-only rule |
| P2 | `Durability::Immediate` | A stop after its commit, before `drop`, and power loss in that window | 0 to 9 ms in the probe; repair took 88 ms against 7 ms at 270 MB | It does not save work if a stop occurs during its store write |
| P3 | `None` and a clean `drop` | A normal exit without a close error | 0 | A panic, early stop or hidden close error can leave the last commit non-durable |
| P4 | Host retry in memory | A failed or uncertain commit | 0 | `PersistFailed` keeps an update today. A production host does not exist. See `src/index/refresh.rs:156-180` |
| P5 | Buffered journal, no sync | A process stop only | about 0.1 ms for a small write, before encoding and publication | Measure safe publication, replay and cleanup; do not call it durable |
| P6 | External synced journal | A stop after journal sync, before the store's durable barrier | At least the journal sync, publication and removal; the barrier is the `Immediate` replay commit | Compare journal plus `Immediate` with P2 during the store write |
| P7 | Journal table in redb | A stop after a first durable journal transaction, before the facet transaction | Two durable transactions and extra rows [H] | A second durable transaction can apply facets and remove the journal atomically [H] |
| P8 | Side file, then rename, for a rebuild | A stop during a rebuild | A second file, file sync, rename and directory sync | Compare repair, bytes and first usable query only for a viable rebuild |
| P9 | Resumable rebuild. Commit each chunk. Mark `META` as incomplete | A crash in a rebuild | 4 ms for each chunk | Rare benefit. It breaks one coherent generation unless the mark works. Defer |
| P10 | Dirty sentinel file | Detection of an unclean stop | One file | No gain. The commit carries `META`, the `racy` bit and the rows together |
| P11 | Read-only open when no file changed | Nothing. It is a speed control | -12 ms and -5 syncs in the handle probe | Measure a complete query and hand the result to ticket 32 |
| P12 | A flag that a `SIGINT` handler sets. The run stops and drops the store | A `Ctrl-C` in the window | A handler and a cancel check | Not tested. A handler cannot drop a redb handle safely. `SIGKILL` and power loss still skip it |

[H] P7 needs a complete two-transaction prototype before it earns a cost or
recovery claim. First, a durable redb transaction stores the prepared facts.
Second, another durable transaction writes facets and removes the journal.
If the second commit fails, the first transaction must remain replayable.
An unreadable or incompatible store cannot supply an in-store journal.

If the store file is new, check its parent-directory durability.
The two redb transactions alone do not prove that a new file name survives
power loss.
Test that case as redo or a side-file rebuild.

[V] P10 gives nothing. A lost commit also loses its `META` row and its `racy`
bits, because they share one transaction. The next refresh compares each file
row with the disk. Ticket 25 already requires this check.

[H] P5 has one more cost. The note has no data on the stop distribution. A
stop in the parse leaves no journal, because the journal follows the parse. The
journal helps only after the parse ends.

[H] A journal cannot help when the store is not writable. It needs the store
to replay. A journal in the user state directory adds orphan files when
the vault moves. Do not add a second location to the matrix.

[H] A replay needs a complete source check, the policy and the writer base.
Use the same facet write path for normal writes and replay.
After replay, change the store revision.
If the store has a newer revision, discard an obsolete journal.
Do not apply the same delta twice.

Do not reject a moved vault only because its root path changed.
Ticket 25 excludes that path from store identity.

[H] A journal needs no new dependency for the pieces. `blake3`, `postcard` and
`uuid` are production dependencies. The Rust standard library has `File::lock`
for exclusion. `tempfile` is a development dependency only. A temporary file
and a rename in production code need a new production dependency or hand-written
code.

## G. Precedents

[V] Git: [the `core.fsync`
setting](https://git-scm.com/docs/git-config#Documentation/git-config.txt-corefsync)
defaults to `committed,-loose-object`. This list does not include `index`,
`commit-graph` or `pack-metadata`. Add the `derived-metadata` group by hand to
harden them. Git says that items which are not hardened may be lost after an unclean
system shutdown. The default `core.fsyncMethod` on macOS is `writeout-only`.
Git avoids a full cache flush on macOS by default.

[V] SQLite: [Atomic Commit in
SQLite](https://www.sqlite.org/atomiccommit.html) syncs the rollback journal
twice. The first sync writes the content. The second sync writes the page count
in the header. SQLite syncs the directory with the journal. It warns that
`fullfsync` on macOS is "profoundly slow" and slows other disk use. It warns
that a moved, renamed or deleted hot journal breaks recovery. Its tests use a
crash VFS that simulates partial sector writes, garbage and reordering.

[H] SQLite's journal protects an authoritative store. Its undo journal restores
old pages. A redo journal for a rebuildable cache has no such precedent in
this research. The cache tools in research 05 (Cargo, Ruff, rustc, rumdl) treat
a damaged cache as a miss.

[V] Pillai and others, [All File Systems Are Not Created
Equal](https://www.usenix.org/conference/osdi14/technical-sessions/presentation/pillai),
found 60 crash vulnerabilities in 11 widely used systems. The systems include
SQLite, LMDB, Git and Mercurial. The correctness of an update protocol depends
on file system behavior that varies between systems. A hand-written journal
carries this risk. redb carries it already.

[V] Candea and Fox, [Crash-Only
Software](https://www.usenix.org/events/hotos03/tech/full_papers/candea/candea_html/index.html),
say that crash-only programs have one way to stop and one way to start. The
stop is a crash. The start is recovery. The CLI guide in this repository
repeats the rule. See `docs/refs/cli_guide.md:807-810,869-870`. The landed
design fits this rule. A cache that rebuilds on a miss has one recovery path.

[V] Dataview keeps its index cache in IndexedDB on the local device. Its
author chose it over a synced file because a synced cache would break across
devices. It reuses the cache on first indexing and has no journal. See
[issue 1064](https://github.com/blacksmithgu/obsidian-dataview/issues/1064).
Logseq users report a crash during a re-index of a large graph in
[issue 6154](https://github.com/logseq/logseq/issues/6154). The fix there was a
rebuild, not a log.

[V] The `fjall` crate has three persist modes: `Buffer`, `SyncData` and
`SyncAll`. `Buffer` writes to the operating system and survives a process stop.
redb has only `None` and `Immediate`. A redb `None` commit does not survive a
process stop. The gap between the two engines is the case that a buffered
journal would fill.

[V] LMDB documents `MDB_NOSYNC`: a system crash may undo the final
transactions, but integrity is kept if the file system preserves write order.
This is the same trade as redb `None`. RocksDB tolerates a damaged log tail at
replay in its default modes. The wiki says that it cannot tell a damaged tail
from an incomplete write. A journal here would need a rule for the tail.

## H. Faults in the ticket text

| ID | Fault in ticket 26 | Proposed change |
| --- | --- | --- |
| H1 | Step 1 called the 12.0 ms and 65.7 ms times "this scan". They are no-op refresh times. At 50 notes they are mostly store open and close. | Fixed in the ticket. |
| H2 | Step 1 did not separate a successful normal close from an I/O failure or panic. | Describe the successful probe and the unreported close error. |
| H3 | The full rebuild has both a large `O` and a large `B`; it is not the sole best case. | Stage the matrix. Reject a case only when its measured `B` upper bound does not exceed the measured `O` lower bound. |
| H4 | Path 2 measures the commit only. The commit is 4 ms in one test and the drop gets shorter. | Measure the sum of open, commit and drop. |
| H5 | Two journal locations are tested. The user state directory has no replay target when the store is unwritable. | Test only the location next to the store. |
| H6 | File images alone miss torn tails, garbage and reordering. | Use fail points first. Simulate power loss with a fault-injecting file layer only for candidates that pass the cost gate. |
| H7 | The failure list omits a deleted store with a surviving journal. | Add the case. The BDD feature requires that the query result stays correct. |
| H8 | The repair measure names one case. | Measure both regimes: clean close, then stop; and durable commits, then stop. |
| H9 | The stop list did not separate `SIGKILL`, `abort`, a panic and `Ctrl-C`. | Test every phase with `SIGKILL`, and test the other modes at representative boundaries. Do not force a real OOM. |
| H10 | Parse time minus decode time is only a saved-work bound. | Measure the complete replay and first usable query against each rival. |
| H11 | The Handoffs list said that ticket 35 does not list this ticket. It does now. | Fixed in the ticket. |
| H12 | The window is not the 10 to 20 ms of `drop`. The CLI keeps the store open during the query. | Measure the window for each command and size. |
| H13 | Path 2 ignores the repair after an `Immediate` commit. It is 33 to 88 ms. | Include the repair time in the cost of path 2. |
| H14 | `Database::drop` does not return a failed close write. | Keep a journal until the `Immediate` replay commit returns `Ok`. |
| H15 | A copied store and journal can carry matching identities. | Check the source snapshot and the writer base. Do not reject a copy by path alone. |
| H16 | P7 called a journal table a duplicate of `Immediate`. | Test its two-transaction design. It can protect prepared work during the store write. |
| H17 | The 14% and 5% to 9% rates use saved-work bounds and incomplete journal costs. | Mark them as bounds, not profitable failure rates. |
| H18 | One failure rate for an entire path hides different stop-phase benefits. | Compare matched elapsed stop points and report a conditional rate for each beneficial phase. |

## I. Tentative grilling decisions

[P] These answers are tentative. They replace the earlier proposals in this
note. Keep the ticket open until its measurements are complete.

### I1. Scope and performance

- Keep ticket 26 as a measurement task. Do not select a journal.
- Keep incremental source updates and rebuilds on policy or identity drift.
  Measure a cost crossover, but do not add a file-count switch now.
- Use a staged rejection gate, not a full-rebuild-first gate.
  Measure changed counts, total notes, note shapes and path-set changes.
  Expand around cases whose measured cost bounds overlap.
- Report normal-write cost and the first usable query after a stop.
  Compare redo, `Immediate`, a journal and a conditional rebuild side file.
- Measure two rebuild payload bounds on the decided task-heavy facets.
  One carries prepared facets. One carries parsed facts and rebuilds derived data.
- Report a conditional eligible-stop rate for each matched phase.
  Measure each window. Do not combine rates without a stop distribution.
- If a candidate passes the gate, open a follow-on task.
  Move to it the replay prototype, the crash tests, the Linux checks
  and the recovery-query checks. Rewire ticket 35 to that follow-on task.
  Do not resolve ticket 26 from costs alone.

### I2. Durability and recovery

- Count process stops and power loss as possible redo events.
  Check store consistency separately. Keep file-system sync limits explicit.
- Use `File::sync_all` for an external journal and sync its parent directory.
  Measure each call. Do not assume Windows has the same guarantee.
- Use a buffered journal only as a process-stop control.
  Include encoding, safe publication, replay and cleanup in its cost.
- Use fail points for process stops.
  Test `SIGKILL` at each phase and other stop modes at key boundaries.
  Simulate power loss only for candidates that survive the cost gate.
- Keep a durable journal until the `Immediate` replay commit returns `Ok`.
  Do not use `Database::drop` or a reopened revision as that proof.
  The journal-side sync uses `File::sync_all` plus a parent-directory sync.
  Compare journal plus `Immediate` with `Immediate` alone during store writes.
- When a journal fails, discard its facts and use the existing redo path.
  Do not make an optional cache aid a reason to reject a valid source update.
- When a source, policy, writer-base or compatible-identity check fails,
  discard the whole journal. Keep a higher-epoch journal untouched.
- Check a copied vault by facts, not by its absolute path.
  Ticket 25 excludes the root from store identity.

### I3. Ownership and platform limits

- Hold the store writer handle before journal publication or replay.
  Use the adapter's lock and check the writer base under it.
- Keep the domain store port engine-neutral.
  Keep any justified journal inside the redb adapter.
  Let another engine decide whether its own log is sufficient.
- Measure a complete read-only query, handover and repair fallback here.
  Pass the result to ticket 32. Do not change production index code.
- Simulate the host lifecycle in a throwaway harness.
  Ticket 32 must check the result against a production host later.
  After a panic, discard the handle and plan before revalidation.
- Measure latency on macOS/APFS.
  Use Podman's Linux VM for correctness checks on VM-local storage.
  Do not claim native-Linux speed, Windows behavior or real power-loss proof.

[P] Add P7, the in-store redb journal table, to the staged cost screen.
The first durable transaction records facts.
The second durable transaction applies facets and removes the journal.
An unreadable or incompatible store cannot replay it.
Check directory durability when the store file is new.
The two-transaction cost and failure behavior are not measured.

## J. Remaining gaps

[H] The redb probe ran on macOS only. It used a synthetic table, not the index
layout. Cases ran 20 times each. Only the `None` and `Immediate` comparison has
repeat runs, and it shows noise as large as the effect.

[H] The real-binary tests ran on macOS. The kill tests use a 1,000-note vault
for phases and a 50,000-note vault for a rebuild. The 10 phase cases and 3
rebuild cases passed. A journal under test would need its own fail points.

[H] The rebuild `O` is an estimate from the old layout.
It adds 3 ms of encoding from the codec benchmark.
It adds a write and sync of about 30 MB.
It adds 4 ms for the directory and removal.
The 11-facet task-heavy payload can have a different size.
Neither payload's decode and full replay time is measured.

[H] Linux and Windows behavior is not measured.
The existing Podman `applehv` machine is stopped and has 2 GiB of memory.
Plan Linux correctness checks on VM-local storage.
Do not use a Linux VM result as a native-Linux latency result.

[H] Network file systems, FAT volumes and case-insensitive volumes affect the
baseline too. redb needs honest sync calls and file locks. The store open
repairs corruption and invalid-data errors only. A stale lock or another I/O
error after a killed writer stops the command. This is not measured.

[H] Windows is not analyzed. `FlushFileBuffers` has no directory sync. The
store open removes the file and creates it again. A second handle can block
that on Windows [INFERENCE]. The mapping of `SIGKILL` and `Ctrl-C` is unknown.

[H] A long-lived host has no row in the tables. A held store keeps every `None`
commit non-durable. The work lost grows with the number of batches. Ticket 32
owns this. See question 10.

[H] The ticket asks who can read the journal and which tools copy it. The store
already holds the same facts in `.traces/index.redb`. A move of the store to
the user state directory would change read-only vaults and sync-tool exposure
for every path. That is outside this ticket.
