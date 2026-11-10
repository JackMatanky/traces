# Ticket 27 grilling preparation: the index replay prototype and the fault tests

## Purpose and method

This file prepares the grilling of
[27-index-replay-prototype-and-fault-tests.md](../issues/27-index-replay-prototype-and-fault-tests.md).
It collects the inherited contract, the code facts, the research lessons, the
design questions and the test plan. The grilling decides the design questions.
This ticket is a `task` ticket. It does not change production code. The
maintainer decides adoption after the ticket.

Sources for this file:

- Tickets 16, 21, 23, 24, 25, 26 and 28, and the research of tickets 24 to 26.
- The code: `src/index/store.rs`, `src/index/service.rs`, `src/index/delta.rs`,
  `src/index/fingerprint.rs`, `src/index/tables.rs`, `benches/common/cache.rs`.
- The redb 4.3.0 documentation from the local `.rust-docs/` cache.
- SQLite, CrashMonkey, ALICE and PostgreSQL publications about journals and
  crash testing.
- The digest corpus about cache freshness: Git, Cargo, Mercurial, rustc,
  Obsidian and Dataview.
- The prior probes of this repository: the `dyld-sync-shim` and the `t26-probe`
  crate.

## The inherited contract

| Source | Rule | Reference |
|---|---|---|
| Ticket 16 | Index result equivalence with a fresh parse is Frozen. | 16:79 |
| Ticket 16 | The `index.redb` layout, the refresh strategy and the heal rules are Free. | 16:80 |
| Ticket 16 | Exit codes 0, 1, 2, 130 and `stdout = payload`, `stderr = diagnostics` are Frozen. | 16:81 |
| Ticket 16 | A new file on disk needs a row in the behavior-change table. | 16:80 |
| Ticket 24 | No engine type crosses the port. Bytes are engine-neutral postcard facets. | 24:294-304 |
| Ticket 24 | The writer compares the base identity and the revision inside the write transaction. A mismatch discards the delta. | 24:429-439 |
| Ticket 24 | One transaction replaces facets, task copies, range keys, edges, notices, meta and revision. It deletes all old derived keys first. | 24:481-486 |
| Ticket 24 | Durability and locking stay inside the adapter. The store is a rebuildable cache, not crash-durable. | 24:411-415 |
| Ticket 25 | The racy bit is set when `mtime >= scan_start - 2s`. Unix uses `ctime`; Windows uses size and `mtime`. | 25:246-273 |
| Ticket 25 | A lower epoch or a changed fingerprint means a quiet rebuild. A higher epoch means a fresh parse, one notice, exit 0, no write. | 25:351-373 |
| Ticket 25 | Only a writer that decided to rebuild may delete a store file. | 25:351-373 |
| Ticket 25 | A no-op refresh costs 12 to 65 ms. The journal is the optional branch of this decision. | 25:263-282 |
| Ticket 25 | The `commit_nonce` is dropped. The identity is the incarnation and the revision. | 25 + 27:122 |
| Ticket 26 | The protection window starts when the journal is durable. It ends when the store commit is durable. | 26:44-49 |
| Ticket 26 | Keep the journal until the replay commit with `Durability::Immediate` returns `Ok`. | 26:121-122 |
| Ticket 26 | The journal lives next to the store under `.traces/`. | 26:142 |
| Ticket 26 | The journal content list: store identity, per-file stats with the racy bit and the scan start, facts per changed facet, old derived keys, affected link edges of unchanged files, parse notices, a checksum. | 26:148-158 |
| Ticket 26 | The gate rejects a case only when the measured upper bound on saved work is no greater than the measured lower bound on added cost. | 26:79-80 |
| Ticket 26 | The 4 ms barrier floor came from file and directory sync timing only. The probe did not run kill-and-replay. | 26 Answer |
| Ticket 26 | The claims of 85 percent more write time and 2 times the file size are untested. | 26:216-219 |
| Ticket 28 | A replay changes the revision. Every query result names the incarnation and the revision. | 28 + 27:128-129 |
| Ticket 21 | One writer per canonical root. The generation pair is published after the commit. | 21:89-99 |

## The code today

- `src/index/store.rs:38-43` defines the META keys: `version` = `1` and
  `fingerprint`. A version mismatch clears the store.
- `src/index/tables.rs` defines 8 tables: FILES, NOTES, LINKS, PATHS_BY_TAG,
  PATHS_BY_FILE_CLASS, TAGS_BY_PATH, FILE_CLASSES_BY_PATH, META.
- `src/index/store.rs:351-358` sets `redb::Durability::None` on all write
  transactions. This is the only durability setting in `src`. The comment says:
  the store is a derived cache; a crash loses the last write; a refresh rebuilds
  by a rescan.
- `src/index/store.rs:565-579` reads the stored fingerprint into `Virgin`,
  `Recorded` or `Unreadable`. Callers fail closed.
- `src/index/store.rs:743` `persist` handles `Rebuild` and `Incremental`. An
  empty incremental returns early. A debug assert gates empty passes.
- `src/index/store.rs:894-921` is the commit code that tickets 26 and the e2
  probe cite.
- `src/index/store.rs:360` holds a fault-injection hook that replaces a note row
  with invalid bytes. The fault tests reuse it.
- `src/index/service.rs:206` `prepare_pass` produces
  `RefreshState::Stale(pending)`. `refresh_store` at line 272 applies it. A
  write error has no in-memory fallback.
- `src/index/delta.rs` computes a path-sorted `FileDelta` and an `InlinkDelta`
  in one merge pass.
- `src/index/fingerprint.rs` holds 10 Blake3 digests, a digest per
  index-relevant setting. It excludes the root, template and schema dirs.
- `benches/common/cache.rs:11` sets `INDEX_DB_RELATIVE_PATH` to
  `.traces/index.redb`. `drop_page_cache` is a best-effort hint for a single
  file.
- `src/index/store.rs:165-188` `IndexStore::open` creates the parent directory
  and the database. A `rebuild_needed` result drops the file and recreates it.

## Research lessons

### redb 4.3.0 durability and recovery

Facts from the local redb documentation:

- `Durability::None` commits stay in memory. A later `Immediate` commit makes
  them durable. `Durability::Immediate` guarantees persistence when `commit`
  returns.
- A `commit` error means an atomic but uncertain outcome: the changes are fully
  applied or not applied, but they may already be durable. The database refuses
  further write transactions. A close and a reopen repair the internal state.
- A panic inside a transaction poisons it. The transaction rolls back, but a
  corruption that caused the poison stays in the file.
- `Database::open` detects and recovers crashes, power loss and unclean
  shutdowns without a call. `check_integrity` is the slow explicit repair for
  external damage. It returns `Ok(true)`, `Ok(false)` after a repair, or
  `Corrupted`. Repair makes pending `None` commits durable or rolls them back.
- redb allows a single writer. A second process open fails with
  `DatabaseAlreadyOpen`. A read-only handle cannot coexist with a writable
  handle. Multi-process support is experimental.

Lessons for ticket 27:

- The barrier is a per-transaction `set_durability(Immediate)` on the replay
  transaction. The `None` default of `configure_cache_txn` stays for all other
  writes.
- A failed commit is not a rollback. The recovery checks the incarnation and the
  revision after a reopen, as the ticket requires at 27:121-123.
- The ticket asks for the redb repair time after each unclean stop. Measure the
  implicit open recovery and the explicit `check_integrity` separately.
- The single-writer lock is the tool against two processes that replay the same
  journal. Serialize the validation, the replay and the removal inside one write
  transaction.

### SQLite rollback journal lifecycle

Facts from the SQLite atomic commit document:

- SQLite writes the journal, syncs it and syncs its directory before it writes
  the database pages. It deletes the journal only after the database pages are
  durable, and it syncs the directory after the deletion.
- A journal that survives a crash is a hot journal. The next writer replays it
  before any read or write. This rule needs an exclusive lock during the replay.
- SQLite defends against a garbage-filled journal in two ways: a header page
  count that stays zero until the content is flushed, and per-page checksums
  that a rollback verifies. It keeps the checksums even with `FULL` sync.
- The recovery decision is encoded in the journal header state.
- SQLite assumes that a file deletion is atomic. A partial deletion can corrupt
  the database.

Lessons for ticket 27:

- The journal removal needs a directory sync after the removal. Without it, a
  crash leaves a consumed journal in place. The next open must then decide
  again.
- The cheap decision is the writer base check. The store revision is past the
  journal. The base check then fails, and the code discards the journal.
- The journal needs a magic, a format version and an identity header, so the
  open code can decide before any replay.
- A partial application is never allowed. Ticket 27 rejects partial or
  mixed-policy data. A checksum failure therefore means: discard the whole
  journal and rebuild. Per-record salvage has no value here, unlike in SQLite.

### Crash-testing research: CrashMonkey and ALICE

Facts from the OSDI 2018 and SOSP 2021 papers:

- CrashMonkey records the I/O sequence of a workload and replays it to every I/O
  boundary. After each replay it mounts the image and checks the invariants. The
  crash points are bounded by the number of I/O operations, not by time.
- ALICE captures the disk image at every `fsync` boundary inside a library file
  system and checks the same invariants.

Lessons for ticket 27:

- The stop points of the ticket are already at sync boundaries. The
  `dyld-sync-shim` with `KILL_AT=n` implements this method on macOS. Extend the
  sweep to all sync calls of the prototype, not only one.
- The oracle after any stop point is the same test: the recovered query equals a
  fresh parse. Run it after every crash point and record the outcome class.
- The saved-image method matches ALICE: capture the file images at each sync
  boundary, then reopen from the image. Record the limit: this simulates a
  process stop with clean pages, not a power loss with torn writes and write
  reordering.

### PostgreSQL write-ahead log

Facts from the PostgreSQL internals material:

- Every change goes to the log before the data pages. Recovery replays the log
  from the REDO point. A page LSN stops the replay of an older record, which
  makes the redo idempotent.
- Full-page images after a checkpoint protect against torn page writes.

Lessons for ticket 27:

- Make the replay idempotent by construction instead of with page LSNs: the
  journal records absolute row upserts and explicit key deletions. Replaying the
  same rows twice produces the same state. This satisfies the ticket rule that
  replay is safe to repeat.
- Torn writes inside a journal file are caught by the whole-file checksum. The
  recovery discards the journal and rebuilds. No page-level machinery is needed
  at fact granularity.

### Cache freshness precedents from the digest corpus

- Git treats an entry with `mtime` at or after the index timestamp as racily
  clean and re-checks the content. It truncates the cached size to 0 for
  ambiguous entries.
- Cargo never hashes an `mtime` into a fingerprint. It rewinds the dep-info
  mtime to the build start.
- Mercurial marks an ambiguous timestamp with a sentinel and re-verifies.
- rustc never serializes parse trees. It persists hashes and re-parses, because
  a re-parse is cheaper than a deserialize.
- Obsidian keeps the metadata cache in memory with events. Dataview persists
  only a summary as a cold-start warm cache and rebuilds the indices on load.

Lessons for ticket 27:

- The journal must carry the scan start of the original pass. The replay
  computes the racy bit from the recorded scan start, not from the replay time,
  as the ticket requires at 27:86-87.
- Never put an `mtime` value into an identity key. The store identity holds
  digests and tags, not timestamps.
- The journal holds facts, not parse trees. This matches the rustc rule and the
  ticket rule to encode parsed facts, not row bytes.

### Fault injection on this machine

- `.scratch/arch-design/research/26-prep/dyld-sync-shim.c.txt` holds a
  `DYLD_INSERT_LIBRARIES` interposer. It logs `fsync` and `F_FULLFSYNC` calls
  with a monotonic timestamp and kills the process before the n-th sync when
  `KILL_AT=n` is set.
- `src/index/store.rs:360` corrupts a note row in place through a
  fault-injection hook. The fault tests reuse it.
- Failure classes and `drop` behavior: `SIGKILL` runs no handler and no `drop`.
  `abort` runs no unwind and no `drop`. A panic unwinds and runs `drop` for live
  values. `Ctrl-C` with the default handler runs no `drop`. An out-of-memory
  kill is an untested limit, as the ticket says.

## Design questions for the grilling

### DQ1. The publication step

The stop-point list separates the journal sync from the journal publication.
Three designs exist:

| Option | Method | Note |
|---|---|---|
| A | Fixed name, create, fill, sync, use | No distinct publication step. A crash mid-fill leaves a visible partial file. The checksum catches it. |
| B | Temp file, sync, rename, sync the directory | The rename is the publication. A partial temp file is never visible under the real name. An orphan temp file needs a cleanup rule. |
| C | Fill, sync, flip a committed flag in the header, sync again | Two syncs of one file. No rename. No directory sync for publication. |

Recommendation: B on macOS and Linux. The ticket already requires a directory
sync for the journal file. B gives the cleanest match to the stop-point list and
the cleanest orphan rule. Record the Windows limit: no directory sync.

### DQ2. The removal rule

The removal sequence:

- Remove the journal after the `Immediate` commit returns `Ok`.
- Sync the directory after the removal.
- Measure the cost of the removal sync per window.

An alternative comes from the SQLite recovery path. Truncate the journal to
zero, or zero its header, and sync the file. The journal is then not hot, and no
directory sync is needed for that decision. The deletion happens later and is
cosmetic. This trades a directory sync for a file sync and removes the
hot-journal ambiguity.

Defense in depth: the writer base check inside the replay transaction also
catches a consumed journal. The store revision is past the journal revision. The
base check then fails, and the code discards the journal.

### DQ3. The journal format

| Option | Format | Consequence |
|---|---|---|
| A | One postcard blob, a header with magic, version, identity, lengths and a whole-body Blake3 checksum | Any damage means: discard and rebuild. No partial replay. |
| B | Framed records with per-record checksums | A salvage of a prefix is possible, but a partial application is forbidden, so the salvage has no value. |

Recommendation: A. It satisfies the checksum item of the content list, it is the
smallest encode, and the discard rule is already the ticket rule. The header
encodes the recovery decision, as SQLite does.

### DQ4. The journal scope

Journal the incremental windows. Do not journal a full rebuild. The rival facts:

- A rebuild repeats after a kill. The measured cost is about 1,801 ms at 50,000
  notes including persistence.
- A kill inside the rebuild transaction rolls back to the previous consistent
  store state through the redb recovery.
- The stop point in the middle of a full rebuild stays in the test plan with
  this recovery path.

Measure the rebuild-restart cost as the rival of a journaled rebuild.

### DQ5. Replay against changed inputs

The replay re-stats all journaled files and compares with the recorded stats
under the racy rule of the recorded scan start. A changed file inside the
journal gets a fresh parse of that file before the apply. An unlisted change
does not block the replay, because the journal carries the edge deletions of
unchanged files. A divergence that the local repair cannot absorb discards the
journal and runs a normal refresh. This matches the rule to validate a copied
vault from the complete source snapshot.

### DQ6. Concurrency

Validate and replay inside one write transaction. redb serializes writers, so a
second process blocks on `begin_write`, and its base check fails after the first
writer commits. The ticket rule that two processes never replay one journal
follows from the lock. Measure the lock hold time and the wait time of the
second process, as the ticket asks at 27:42.

### DQ7. The failed-commit protocol

On a commit error: drop the handle, reopen the database, read the incarnation
and the revision. If the revision is at or past the journal target, remove the
journal. Otherwise replay again. Never treat the error as a rollback.

### DQ8. The name and the cleanup

Name the journal after the store file: `index.redb.journal`. The engine tag
question of ticket 25 applies to the store name family. Cleanup rules to
specify:

- A journal without a store file is an orphan. A writer deletes it.
- A journal of a retired engine follows the ticket 25 rule for retired cache
  files.
- A journal with a higher epoch stays untouched.

### DQ9. Adapter ownership

The redb adapter owns the journal file. No new domain port operation is needed:
the domain sees only a persist result. A future engine with its own journal,
such as SQLite, keeps the rule inside its adapter. This answers the ticket
question at 27:187-189.

### DQ10. Notices and exits

A replay success is silent. A journal discard causes a quiet rebuild with no
notice, under the ticket 25 open rules. Only a higher-epoch store produces the
one notice and exit 0. Ticket 34 owns the notice texts. This ticket specifies
the states and the mapping.

### DQ11. Readers and the journal

Readers never replay. A reader that sees a journal treats the store as possibly
stale, which the cache rules already allow. The no-change control uses a
read-only open, as the ticket requires at 27:146-147.

### DQ12. The unmeasured rivals

The saved-work bound needs the parse-only fallback measured, because a journal
discard pays a rescan that the current numbers do not split. Add this
measurement to the plan. It also fills the open gap of ticket 25 at 25:431-432.

## Test plan

Each row: scenario, injection, oracle, expected outcome.

| Scenario | Injection | Oracle | Outcome |
|---|---|---|---|
| Kill before the journal write completes | Shim `KILL_AT` at the first sync, or kill in the temp window | Fresh parse equals store | Nothing to replay; quiet |
| Kill after the write, before the journal sync | Shim before the file sync | Fresh parse equals store | Temp file orphan; cleanup |
| Kill after the journal sync, before publication | Shim between sync and rename | Journal bytes survive a reopen | Replay or discard |
| Kill after publication, before the store commit | Shim after the rename sync | Journal valid, identity matches | Replay, revision advances |
| Kill before and after the barrier | Shim around the `Immediate` commit | Incarnation and revision consistent | Remove or replay-again rule |
| Kill before and after the journal removal | Shim around the remove and the directory sync | No double replay | Base check catches a stale journal |
| Kill inside the replay commit | Shim during the commit | Equals fresh parse | Uncertain-outcome protocol |
| Kill in the middle of a full rebuild | Shim at a sync inside the rebuild | Equals fresh parse | redb rollback; rebuild repeats |
| Lose the last `None` commit | Image before the `None` commit | Stale store, then rescan | Cache rule holds |
| Keep the directory entry, lose the journal data | Image with the entry only, or truncate the body | Checksum fails | Discard and rebuild |
| Store commit lost, journal survives | Image before the commit, journal durable | Replay produces the target state | Replay wins |
| Journal left after a durable commit | Image after the commit, before the removal | Base check fails | Quiet discard |
| Corrupt or truncate the journal | File surgery, or the store hook pattern | Checksum or length fails | Discard and rebuild |
| Delete the store, keep the journal | Remove the store file | Query equals fresh parse | Orphan journal deleted; rescan |
| Unwritable store directory | Directory mode change, run twice | No unbounded journal growth | Bounded retries; fail open |
| Read-only vault | Read-only mount or file mode | Exit rules of ticket 16 hold | Fresh parse or read-only open |
| A changed unlisted file | Edit before the replay | Edges of unchanged files still correct | Replay holds |
| A changed listed file | Edit before the replay | No stale link, notice or range key | Local re-parse or discard |
| Add, delete, rename | File surgery before the replay | Equals fresh parse | Replay or discard |
| Same size and `mtime`, inside the racy window | Touch with a frozen `mtime` | Racy bit from the recorded scan start | Re-check content |
| A changed policy | Change a fingerprint input | No mixed-policy rows | Discard and rebuild |
| Each identity part changed separately | Engine tag, platform, incarnation, revision | No retag from another identity | Discard and rebuild |
| Lower epoch, higher epoch, changed fingerprint | Meta edits | Ticket 25 rules hold | Rebuild, or fresh parse with a notice |
| A higher-epoch journal | Journal meta edit | The journal stays untouched | Keep |
| A second process during a replay | Start a writer against a live or dead holder | One replay only | Lock serializes; base check |
| Handle dropped before a durability check | Drop, then a second writer acts | Writer base checked before cleanup | No wrong cleanup |

## Measurement plan

| Measurement | Method | Reference |
|---|---|---|
| Encode cost with real parser facts | Criterion, release, serial | 27:20 |
| Derivation cost at replay | Criterion | 27:23 |
| Decode and replay times | Criterion and cold runs | 27:37 |
| Store open, commit, barrier, close | Cold child runs, median and range, 20 runs or more | 27:38 |
| File sync and directory sync, separately | Shim log timestamps | 27:133-134 |
| redb repair after each stop class | Open recovery and `check_integrity` | 27:39 |
| Journal bytes per facet, store sizes, peak memory | Probe output | 27:40-41 |
| Lock hold and second-process wait | Timestamps around the write lock | 27:42 |
| Cost without a failure, and the buffered control | Criterion | 27:43-44 |
| First usable query and repeated queries per scenario and size | Cold child harness | 27:142 |
| 85 percent write time and 2 times file size | Decided layout with `lists` and `tasks` | 27:154-157 |
| Real saved-work bounds, per-phase break-even | Compare with the screen bounds | 27:158-162 |
| Parse-only fallback | Rebuild minus persistence split | DQ12, 25:431-432 |

Anchors: 50, 5,000 and 50,000 notes, with a 0.75 task-per-note ratio and a
hub-linked fixture. Extend near the crossovers. The gate compares the measured
upper bound on saved work with the measured lower bound on added cost.

## Prototype shape and risks

- The prototype lives in a throwaway crate on the public surface and
  `traces::testing`, in the style of the `t26-probe` crate. It moves into the
  redb adapter only if the maintainer adopts it.
- `traces::testing` is frozen at 47 identifiers under the `test-utils` feature
  (`src/lib.rs:177-211`). The list has no store or META access. The probe
  mirrors the table definitions and opens `.traces/index.redb` with redb
  directly, as the ticket 26 probes did. The service-level helpers
  `IndexerService`, `TestProject`, `build_test_index` and `QueryService` stay
  the primary path for fixtures and queries.
- Risk: the shim kills at sync boundaries only. Torn writes inside a `write`
  call are simulated with file surgery. Record this limit.
- Risk: the system-crash simulation is a process stop with clean pages. It is
  not a power loss. The ticket forbids the claim of equality, and the research
  asset must repeat the limit.
- The Linux run uses the Podman VM with VM-local storage and 2 GiB of memory. It
  checks correctness, not latency.

## Consistency checks against the ticket text

- The barrier definition is consistent: the `Immediate` replay commit is the
  barrier, and it permits the removal.
- The stop-point list implies a distinct publication step. DQ1 resolves the
  design to match it.
- The None-commit-loss case applies to the normal incremental path and to
  rebuilds, not to the journaled path, whose commit is `Immediate`. The test
  plan states this scope.
- The ticket drops the `commit_nonce`. The identity pair is the incarnation and
  the revision. No test uses a nonce.
- The research asset `../research/27-index-replay-prototype.md` does not exist
  yet. This ticket creates it and links it under an `## Answer` heading.
- Ticket 33 states that the staleness bound is set with this ticket. The
  measurements here, mainly the no-change control and the refresh costs, are the
  input. Ticket 33 owns the bound decision.
- The Linux VM has 2 GiB. Ticket 33 reports 467 MB for a pinned generation at
  50,000 notes. The simulated-host runs in the VM must fit both.
- Ticket 36 owns the `None` versus `Immediate` choice. This ticket supplies the
  measured delta between the two settings on the replay transaction.

## Open questions for the grilling

1. Publication design: temp-and-rename against a header flag. DQ1.
2. Removal sync: pay the directory sync after the removal, or rely on the base
   check alone. DQ2.
3. Journal format: whole-file checksum against framed records. DQ3.
4. Journal scope: incremental only, or also rebuilds. DQ4.
5. Replay against changed inputs: local re-parse against a full discard. DQ5.
6. Cleanup rules for orphan, stale and retired journals. DQ8.
7. The notice mapping handoff to ticket 34. DQ10.
8. The scope of the Linux run: correctness only, or a latency spot check.
9. The removal design: unlink with a directory sync, or truncate-first with a
   later cosmetic delete. DQ2.
10. The staleness bound input that ticket 33 needs from these measurements.
