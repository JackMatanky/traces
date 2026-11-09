# Research for [What does a durable index repair journal cost and save?](../issues/26-measure-index-recovery-journal.md)

This is preparation for the open ticket. It does not record a decision.
[V] marks checked evidence. [H] marks a limitation or inference. [P] marks an
option or a proposed answer.

The probe in section D is a throwaway program. It is not a benchmark. Its source
is in [26-prep/redb-probe.rs.txt](26-prep/redb-probe.rs.txt). The sync shim is
in [26-prep/dyld-sync-shim.c.txt](26-prep/dyld-sync-shim.c.txt). The repository
code is `main` at `79f393a2`. No source file in `src/` or `tests/` changed
since `207cc3fb` in a way that affects this ticket. The changes are comments,
tests, one folder-key separator fix and one scanner call.

## Summary

- [V] A normal process exit makes a `Durability::None` commit durable.
  `Database::drop` runs a final durable commit. A stop that skips `drop` loses
  the last `None` commit. See sections C and D.
- [V] Where the file system keeps its sync promise, redb shows no torn store.
  After a stop it shows the last durable commit. A journal protects work. It
  does not protect consistency.
- [V] A journal helps in one window. The window starts when the journal is
  durable. It ends when the store is durable. In the CLI the store becomes
  durable at `drop`. The CLI keeps the store open during the query, so the
  window runs from the commit to the end of the command.
- [V] On macOS, Rust `sync_all` and `sync_data` call `F_FULLFSYNC`. A synced
  journal costs about 4 ms to write and 3 ms to remove. `Durability::Immediate`
  adds 0 to 9 ms in the probes. The effect depends on the order of runs. A buffered journal
  costs 0.1 ms and survives a process stop only.
- [V] The real `traces list` call makes 5 `F_FULLFSYNC` calls on `index.redb`,
  even when nothing changed. They span about 14 to 16 ms. A synced journal would
  add at least 3 more. See section D5.
- [V] A parse costs 20 to 43 µs for each note in the `rich_realistic` build
  benchmark. A 100-note edit saves at most 4 ms. A synced journal costs about
  7 ms for each write. A synced journal loses at any rate for edits up to 100 notes.
  A buffered journal pays only above a 5% rate.
- [V] The 12.0 ms no-op refresh at 50 notes is mostly store open and close. The
  probe shows 3.0 ms to open and 9 to 10 ms to drop. The first draft of the
  ticket called this time the scan cost. Section H lists the fix.
- [V] A read-only open of the store costs 0.03 ms and makes no sync call. See
  section D6. This is a larger saving than any recovery design here.
- [H] 150 trials killed the real binary with `SIGKILL` at random times after
  an edit. In 109 trials the process was still running. All 150 next runs
  showed the edit. The test is weak: it does not target a phase. See D5.
- [V] A repair after an unclean stop depends on the last commit. After a clean
  close and a `None` commit it takes 7 ms. After an `Immediate` commit it takes
  33 to 88 ms, because only the close commit saves the allocator state. A
  journal removes neither cost. `Immediate` makes the repair longer.
- [V] Git does not harden derived data by default. The cache tools in research
  05 treat a damaged cache as a miss. The CLI guide in this repository says to
  make the program crash-only.
- [P] Change the gate. Compare the saved work with the added cost for each
  scenario before any prototype. Stop if the saved work is never larger. Report
  the choice between `None` and `Immediate` as a separate result.

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
  waits for this ticket.

## B. What a stop loses

[V] One persist is one write transaction with one commit. See
`src/index/store.rs:781-813` for a rebuild and `:816-860` for an incremental
write. The commit calls are at lines 811 and 858. `configure_cache_txn` sets
`Durability::None` at line 355.

[V] `RefreshPlan::collect` opens the store and scans the vault in parallel. See
`src/index/refresh.rs:332-353`. The store is open for the whole refresh. A stop
at any point leaves the previous durable state.

| Phase | A stop here loses | What redb keeps | A journal could save |
| --- | --- | --- | --- |
| Open and scan | Nothing | Last durable state | Nothing |
| Parse | Parse work | Last durable state | Nothing. No journal exists yet |
| Write transaction | Parse work | Last durable state | Parse work, after the journal sync |
| After `None` commit, before `drop` ends | The commit | Last durable state | Parse work, after the journal sync |
| After `drop` ends | Nothing | The commit | Nothing |

[V] A stop in the first two rows leaves a store that needs repair at the next
open. The repair does not need the journal. See section D.

[H] The only phases that the journal can protect are rows three and four. In the
CLI these rows last from the commit to the end of the command. The CLI keeps
the store open during the query. At 20,000 notes the shim shows the store open
for 67 to 81 ms. At 50,000 notes it shows 152 to 198 ms. See section D5.

[V] A panic after the commit loses a `None` commit. `close_database` skips its
final commit while the thread panics. See redb `src/db.rs:1967`. No code in
`src/` calls `process::exit` or `abort`, so a normal return runs `drop`.

## C. What redb guarantees

[V] A non-durable commit does not reach the disk header. The probe shows that a
stop which skips `drop` loses it. In one process the design keeps the commit in
memory. After a crash the database returns to the last durable commit. It stays
consistent. See redb `docs/design.md:323-334` and `src/transactions.rs:378-385`.

[V] `Database::drop` calls `close_database`. This function runs a commit with
the default `Durability::Immediate` that writes the allocator state. A durable
commit also persists earlier non-durable commits. See `src/db.rs:1922-1955` and
`:1963-2010` in redb 4.3.0.

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

[V] A read-only reopen with a writable handle costs 3.0 ms to open and 9 to
10 ms to drop. The sum matches the 12.0 ms no-op refresh at 50 notes in
research 25, section E.

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

[V] Let `O` be the added cost of a journal for each write that has no failure.
Let `B` be the redo cost minus the replay cost for one failure. The journal pays
when the failure rate is more than `O / B` for each write. If `B` is not larger
than `O`, the journal never pays.

[V] Both paths pay the same costs after a stop. These are the store open and
repair, the scan and the commit. See sections B and D3. So `B` is only the parse
time of the changed files minus the decode time of the journal.

[V] The `WorkspaceIndex::build` benchmark covers scan, parse and assembly. It
does not persist. A Criterion quick run on this machine at `79f393a2` gave
medians for the `rich_realistic` shape. They are 8.6 ms at 200 notes, 24.0 ms
at 1,000 and 101.3 ms at 5,000. That is 43, 24 and 20 µs for each note.

[H] These figures include the scan. They also include the speed-up from 12
cores. They suit a large batch. For one note, use the single-thread
`parse_markdown` benchmark: 8 µs for 1 KB, 62 µs for 10 KB and 113 µs for a
large note. Real notes have a mean of 11 KB. So `B` for one note is about
0.1 ms.

[H] Decode cost is not measured. The old `WorkspaceIndex::load` results show
about 0.5 µs for each note above the open floor. They date from 2026-10-18.
Treat `B` as the parse time minus a small decode time.

| Scenario | `B` at most | `O`, synced journal | `O`, buffered journal | Result |
| --- | ---: | ---: | ---: | --- |
| 1-note edit | 0.1 ms | about 7 ms | about 0.1 ms | Synced never pays. Buffered pays only near a 100% rate |
| 100-note edit | 4.3 ms | about 7 ms | about 0.2 ms | Synced never pays. Buffered pays above a 5% rate |
| 5,000-note edit | 101 ms | about 14 ms | about 1.2 ms | Synced pays above a 14% rate. Buffered pays above a 1% rate. Journal size about 6.5 MB |
| Full rebuild, 50,000 notes | about 0.65 s | about 35 to 60 ms | not measured | Break-even about 5% to 9% for each rebuild. The window is 0.87 s [H] |

[H] The `O` values add the synced write (4 ms), the removal (3 ms) and a
payload write. The payload encoding cost is not in the table. A real journal
adds it. This makes `O` larger and the result worse for the journal.

[H] Let `p` be the chance that a stop hits the window of one write. The journal
pays when `p` is more than `O / B`. The window runs from the commit to the end
of the command. It is 15 to 200 ms in the shim traces. Measure it for each
command and size. No data gives a stop rate. Ticket 25 says not to invent one.

[P] The gate in ticket 26 starts from the full rebuild. This case has the
largest `B` and the largest `O`. It does not show the best case for the
journal. Test `B > O` for each scenario instead.

## F. Candidate designs

| ID | Design | Protects | Added cost | Verdict |
| --- | --- | --- | --- | --- |
| P1 | Redo. This is the landed model. | Nothing extra | 0 | Baseline. It matches Git, Cargo, Ruff and the crash-only rule |
| P2 | `Durability::Immediate` | A stop after the commit and before `drop` ends. Power loss in that window | 0 to 9 ms, and a longer repair after a stop (88 ms against 7 ms at 270 MB) | The cheapest rival. It needs no new state. It can lose to P1 for a small edit |
| P3 | `None` and a clean `drop` | A normal exit | 0 | Landed. It has two holes: a panic and a stop before `drop` ends |
| P4 | Host retry in memory | A failed or uncertain commit | 0 | Landed in part. `PersistFailed` keeps the update. See `src/index/refresh.rs:156-180` |
| P5 | Buffered journal, no sync | A process stop only | about 0.1 ms and the encoding | Works only for the commit-to-end window. It needs a format, a replay and a cleanup rule |
| P6 | Synced journal | A process stop and power loss | about 7 ms and the encoding | More costly than P2 for the same protection |
| P7 | Journal as a table in the store | Same as P2 | One more commit | P2 in disguise. The store commit already is the journal |
| P8 | Side file, then rename, for a rebuild | A crash in a rebuild | A second file and a sync | The rebuild is one transaction already. A synced rename gives the P2 gain for a rebuild only. Not scored. Measure with the rebuild `O` |
| P9 | Resumable rebuild. Commit each chunk. Mark `META` as incomplete | A crash in a rebuild | 4 ms for each chunk | Rare benefit. It breaks one coherent generation unless the mark works. Defer |
| P10 | Dirty sentinel file | Detection of an unclean stop | One file | No gain. The commit carries `META`, the `racy` bit and the rows together |
| P11 | Read-only open when no file changed | Nothing. It is a speed gain | -12 ms and -5 syncs for a no-change call | Not a recovery design. It removes the cost that a journal would add to. See D6. Pass to tickets 25 and 32 |
| P12 | A flag that a `SIGINT` handler sets. The run stops and drops the store | A `Ctrl-C` in the window | A handler and a cancel check | Not tested. A handler cannot drop a redb handle safely. `SIGKILL` and power loss still skip it |

[V] P7 follows from the redb design. A journal table that is written in a
separate `Immediate` commit is a durable commit. It adds a second transaction
and no new protection.

[V] P10 gives nothing. A lost commit also loses its `META` row and its `racy`
bits, because they share one transaction. The next refresh compares each file
row with the disk. Ticket 25 already requires this check.

[H] P5 has one more cost. The note has no data on the stop distribution. A
stop in the parse leaves no journal, because the journal follows the parse. The
journal helps only after the parse ends.

[H] A journal cannot help when the store is not writable. It needs the store to
replay. A journal in the user state directory adds orphan files when the vault
moves or disappears. Ticket 25 already leaves cleanup of retired cache files
open. Do not add a second location to the matrix.

[H] If a journal passes the gate, build replay on the same `persist` function.
A second write path will drift when the format epoch changes. The journal must
carry the epoch, the extractor fingerprint and the policy fingerprint. It must
carry the base incarnation and revision. After a replay the store revision must
change. A store with a newer revision than the journal base means that the
commit was durable. Discard the journal. Never apply a delta twice.

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
| H2 | Step 1 does not say that a normal exit makes a `None` commit durable. | Add the drop fact and the panic hole to Step 1. |
| H3 | The gate uses the full rebuild as the best case. It is the case with the largest `O` and the largest `B`. | Test `B > O` for each scenario. Run the full matrix only where `B > O`. |
| H4 | Path 2 measures the commit only. The commit is 4 ms in one test and the drop gets shorter. | Measure the sum of open, commit and drop. |
| H5 | Two journal locations are tested. The user state directory has no replay target when the store is unwritable. | Test only the location next to the store. |
| H6 | The power-loss simulation uses file images at sync points. This misses torn tails, garbage and reordering. | If the gate passes, use a fault-injecting file layer. Otherwise state the limit. |
| H7 | The failure list omits a deleted store with a surviving journal. | Add the case. The BDD feature requires that the query result stays correct. |
| H8 | The repair measure names one case. | Measure both regimes: clean close, then stop; and durable commits, then stop. |
| H9 | The process-stop list does not say which stops skip `drop`. | Test `exit`, `abort`, `SIGKILL` and a panic after the commit. |
| H10 | The break-even text does not remove costs that every path pays. | Define `B` as parse time minus decode time. |
| H11 | The Handoffs list said that ticket 35 does not list this ticket. It does now. | Fixed in the ticket. |
| H12 | The window is not the 10 to 20 ms of `drop`. The CLI keeps the store open during the query. | Measure the window for each command and size. |
| H13 | Path 2 ignores the repair after an `Immediate` commit. It is 33 to 88 ms. | Include the repair time in the cost of path 2. |

## I. Questions for the grilling, with proposed answers

1. **Is this ticket still a measurement task, or can it close with an analytic
   answer?**
   [P] Keep it a task. Make section E the first step.
   Add a gate: if `B` is never larger than `O`, stop and record the answer.
   The probe shows this on macOS for incremental edits.
   A rebuild at 50,000 notes is open, with a break-even of about 5% to 9% for each rebuild.
2. **Which failures count?**
   [P] Count a process stop, a panic and a host crash for lost work. Count power
   loss for consistency only. redb and `META` already give consistency. Lost
   work in a cache costs one redo.
3. **Is `Durability::Immediate` a required rival?**
   [P] Yes. Measure open, commit and drop together. The A-B-A run shows 0 to 9 ms.
   It closes the gap between the commit and the end of the command, and the
   panic hole. It lengthens the repair after a stop: 88 ms against 7 ms at
   270 MB. Decide it in ticket 24 or 35, not as a journal.
4. **Should the matrix test a user state directory location?**
   [P] No. See section F.
5. **Which sync call does the test use?**
   [P] Use `File::sync_all`. On macOS it is `F_FULLFSYNC`, as in redb. Record
   `fsync` on Linux and `FlushFileBuffers` on Windows. Report the buffered cost
   and the synced cost. Do not use Git's `writeout-only`. It is not durable.
6. **How do we simulate a crash?**
   [P] For the gate, use a child process, a fail point and `abort`. If the gate
   passes, build a fault-injecting file layer as SQLite does. State that file
   images at sync points miss reordering and torn writes.
7. **What does the answer change downstream?**
   [P] Ticket 24: the durability setting and the stale comment. Ticket 32: the
   host must drop the store at the end of each persist. A panic or an early
   exit skips the final commit. Ticket 35: the choice between `None` and
   `Immediate`. Ticket 33: no notice is needed if there is no journal.
8. **What stays open after a no-journal answer?**
   [P] The Linux and Windows sync costs. The rebuild `O` at 50,000 notes. The
   window length for each command and size. The repair time of a 67 MB store
   after an `Immediate` commit.
9. **Should a read-only open be the first change?**
   [P] Pass it to tickets 25 and 32. A no-change call then skips 12 ms and
   5 syncs. See D6. It also avoids the unclean state for a pure query.
10. **What rule must a long-lived host obey?**
   [P] Drop the store after each persist. Never catch a panic while the store
   is open. A held `None` store keeps every commit non-durable. A caught panic
   loses them. Ticket 32 owns the rule.

## J. Remaining gaps

[H] The redb probe ran on macOS only. It used a synthetic table, not the index
layout. Cases ran 20 times each. Only the `None` and `Immediate` comparison has
repeat runs, and it shows noise as large as the effect.

[H] The real-binary tests ran on macOS. The kill tests use a 1,000-note vault
for phases and a 50,000-note vault for a rebuild. The 10 phase cases and 3
rebuild cases passed. A journal under test would need its own fail points.

[H] The rebuild `O` is an estimate. It adds 3 ms of encoding from the codec
benchmark. It adds a write and sync of about 30 MB. It adds 4 ms for the
directory and the removal. The journal size is a guess. The decode time of a
journal is not measured.

[H] Linux and Windows behavior is not measured. The sync call, the cost, the
directory sync and the repair time can differ.

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
