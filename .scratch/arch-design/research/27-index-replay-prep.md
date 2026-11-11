# Ticket 27 grilling preparation: the index replay prototype and the fault tests

## Historical plan and reading order

This is the initial design and research plan for ticket 27. The editorial review
date is 2026-11-10; the source does not record the original plan date. The plan
is not a report of completed coverage.

Read [the results](./27-index-replay-prototype.md) for the completed experiments
and their limits.

This plan prepares the grilling of
[27-index-replay-prototype-and-fault-tests.md](../issues/27-index-replay-prototype-and-fault-tests.md).
It separates inherited contracts, production facts, external lessons, DQ
decisions, fault cases, measurements, and result-dependent decisions. The DQ
decisions define the proposed prototype, not journal adoption.

This ticket is a `task` ticket. The initial plan excludes production code
changes. The maintainer decides adoption after the ticket.

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

## Production facts recorded for the initial plan

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
- `src/index/store.rs:360-368` holds a `#[cfg(test)] pub(super)` row-poison
  helper. An external probe cannot call it. Corrupt a test copy through redb.
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

## External research and local injection lessons

The research and risk sections retain uncertainty where strict STE would change
the meaning.

### redb 4.3.0 durability and recovery

Facts from the local redb documentation:

- `Durability::None` commits stay in memory. A later `Immediate` commit makes
  them durable. `Durability::Immediate` guarantees persistence when `commit`
  returns.
- A `commit` error means an atomic but uncertain outcome: the changes are fully
  applied or not applied, but they may already be durable. The database refuses
  further write transactions. A close and a reopen repair the internal state.
- A panic in an unwind build can poison a transaction. The transaction rolls
  back, but the corruption that caused the poison stays in the file. The shipped
  release profile uses `panic = "abort"` and runs no `drop`.
- `Database::open` detects and recovers crashes, power loss and unclean
  shutdowns without a call. `check_integrity` is the slow explicit repair for
  external damage. It returns `Ok(true)`, `Ok(false)` after a repair, or
  `Corrupted`. Repair makes pending `None` commits durable or rolls them back.
- redb allows a single writer. A second process open fails with
  `DatabaseAlreadyOpen`. A read-only handle cannot coexist with a writable
  handle. Multi-process support is experimental.

Lessons for ticket 27:

- Set `Durability::Immediate` on a normal journaled apply or replay. Either
  successful commit can clear its journal. A `None` apply leaves its journal
  until an error-reporting `Immediate` checkpoint or replay. A checkpoint
  qualifies only after the writer checks the stored target against the journal.
- A failed commit is not a rollback. Close the handle. Reopen the store. Read
  the incarnation and revision. Return the existing error. Recheck the base and
  the complete source snapshot in the next command before replay.
- Measure implicit open recovery and explicit `check_integrity` on separate
  saved images. Do not count the second repair as part of the first.
- redb rejects a second process at open while the exclusive writer holds the
  file. It does not wait at `begin_write`. Keep the writer handle before
  publication or replay. Check the base inside the write transaction. Unlink
  only after an `Immediate` commit returns `Ok`.

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

- SQLite syncs its directory after journal removal. This design does not. After
  a successful `Immediate` commit, a returned journal has an old base revision.
  The next writer discards it. Check the returned entry with a saved image.
  Measure the omitted sync as a cost control.
- Put a magic, format version, epoch and checked identity in the header. Check
  the complete body before a commit. A damaged current-format journal causes a
  full discard and refresh. A header with an unknown format stays untouched. Do
  not salvage a record prefix.

### Crash-testing research: CrashMonkey and ALICE

Facts from the OSDI 2018 and SOSP 2021 papers:

- CrashMonkey records the I/O sequence of a workload and replays it to every I/O
  boundary. After each replay it mounts the image and checks the invariants. The
  crash points are bounded by the number of I/O operations, not by time.
- ALICE captures the disk image at every `fsync` boundary inside a library file
  system and checks the same invariants.

Lessons for ticket 27:

- The existing shim kills before a sync call on macOS. It cannot reach a partial
  write, a rename, a post-commit point or an unlink. Add named stop points in
  the throwaway probe. Use a Linux injector for its fault runs.
- Compare the complete recovered query with a fresh parse. Also check a
  test-only replay outcome, the source parse count and the next empty refresh.
  Make a no-replay control fail at a replay stop point.
- Save file images at the injection boundaries. The method does not simulate all
  power-loss write reorderings or torn pages. State the limit for each
  system-crash case.

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

- `.scratch/arch-design/research/26-prep/dyld-sync-shim.c.txt` interposes
  `fsync` and `fcntl(F_FULLFSYNC)` on macOS. `KILL_AT=n` stops before the n-th
  sync. It logs on stderr, misses non-sync boundaries and forwards variadic
  `fcntl` calls without checking the argument form. Do not reuse it unchanged
  for the complete suite.
- `src/index/store.rs:366` is a private test-only row-poison helper. The
  external probe mirrors the table definition and changes a test copy with redb.
- `SIGKILL`, `abort`, `Ctrl-C` with its default handler and a release-profile
  panic run no `drop`. A separate unwind test build can exercise a `drop` path.
  Label that build as non-production. An out-of-memory kill stays an untested
  limit.

## Initial implementation plan and risks

The initial plan uses the public surface and `traces::testing`, in the style of
`t26-probe`. It calls for a throwaway crate, no production code changes, and a
shadow store for the decided eleven-facet layout. The results describe the later
scratch-branch helpers and distinguish the two probe identities.

At planning time, `traces::testing` has 47 identifiers under the `test-utils`
feature (`src/lib.rs:177-211`). This is a historical surface count, not a
restriction on later scratch-branch helpers. It does not expose store `META`.

- Mirror the table definitions for direct redb checks.
- Use the exported `IndexerService`, `TestProject` and `QueryService` for real
  fixtures and query checks.
- The `poison_note_row` helper at `src/index/store.rs:366` is
  `#[cfg(test)] pub(super)`. Use a test copy and direct redb for corrupt rows.
  Do not treat the helper as an external probe API.
- Reject special files, symlinks and implausible lengths before decode. Use a
  nonblocking, no-follow open for the journal on supported systems. A checksum
  does not prevent a malicious FIFO from blocking at `open`.
- Use named failpoints beyond the old macOS sync shim. Keep logs out of child
  stderr. The shim's `fcntl` forwarding reads a variadic argument without
  checking its command. Do not reuse that part unchanged.
- Treat saved file images as a limited crash simulation. The initial Linux VM
  plan uses VM-local storage and 2 GiB of memory. The completed Linux run uses 4
  GiB, as the results explain. redb defaults to a 1 GiB cache. Ticket 33
  measured 467 MB for a pinned 50,000-note generation. Report any case that
  cannot fit. Do not silently omit it.

## DQ decisions for the planned prototype

These decisions define the initial prototype contract, not an adopted journal.
The fault and measurement matrices below specify the planned evidence. The
results asset identifies completed work, changed probe surfaces, and open gates
for tickets 27 and 36.

### DQ1. Publication

- Use a fixed temp name next to `index.redb.journal` under `.traces/`.
- Create the temp file with mode `0600` while you hold the store writer handle.
- Write the temp file. Sync the temp file.
- Rename it to the final name. Sync the parent directory.
- Do not replace an unresolved final journal.

The directory sync does not have a Windows equivalent in this plan.

### DQ2. Removal and normal writes

Compare these paths against no-journal `Immediate`:

- A journaled normal apply with `Durability::Immediate` clears its journal after
  that commit returns `Ok`.
- A journaled normal apply with `Durability::None` keeps its journal. Resolve it
  with an error-reporting `Immediate` checkpoint or replay before the next
  publication. Check that the stored target matches the journal under the writer
  handle before a checkpoint. Do not clear it on `Database::drop`.

Charge the later checkpoint and cleanup to the original write.

- Unlink the journal without a directory sync after the qualifying commit. A
  crash can bring the entry back. The writer base check then rejects it.
- Check that state from a saved image.
- Measure the omitted directory sync as a control.

### DQ3. Format and damage

Use a fixed prefix with magic, header version and format epoch.

- Check the header before you trust its lengths or identity.
- Check the full body before the commit. Bound the file size and every decode
  allocation.
- Compare a single postcard blob with framed postcard facts and a global
  checksum. Framing gives streaming and bounded memory, not partial replay.
- Discard all facts on a checksum or decode failure. Preserve an unknown header
  instead of treating it as a damaged current-format journal.

### DQ4. Scope and rebuild

Journal incremental writes in the prototype. Screen both full-rebuild payloads
from ticket 26 before choosing the rebuild candidate.

- Use the direct-facet payload as a cost control. Derive the candidate facets
  from recorded parsed facts at replay.
- Run a kill case inside a rebuild with and without the journal. Compare full
  costs on the same fixture and release profile.
- The older 1,333 ms, 1,718 ms and 1,801 ms figures came from different runs. Do
  not combine them into a measured benefit.

### DQ5. Changed sources

- Check the complete path set against the original source snapshot.
- Combine the base store file records with the journal changes.
- Use the original scan start for the racy rule.
- Record content digests for racy files, including unchanged files. Recheck these
  digests. Get the digest from the parser's read buffer for a changed file.
- Discard the whole journal if any source, policy, identity or writer-base check
  fails. Do not re-parse a single changed file inside the replay.

A moved root alone does not invalidate the journal. A Unix copy can change
inode-change times. Discard the journal on that metadata mismatch. Parse fresh.

Report both stable and all-racy fixture costs. A future-dated `mtime` stays racy
under ticket 25.

### DQ6. Concurrency and lock time

redb rejects a second process at `open` while a writer holds the file. It does
not wait at `begin_write`.

- Record the actual query fallback and its notice.
- Report harness retry wait separately.
- The writer checks the base inside its write transaction before replay.
- Measure source validation both before and during the writer-handle hold.
- Include any extra read-only to writable handover cost.

The store lock does not freeze source files.

### DQ7. A failed commit

Treat a failed commit as an uncertain outcome:

- Drop the handle. Reopen the store. Check the incarnation and revision.
- Return the existing write error. Do not retry in the same command.
- Check the complete source snapshot and the base in the next command. Replay
  if the original base remains valid.
- Discard a compatible journal otherwise. Refresh from source. A target
  revision alone does not prove whose commit won.
- Preserve a higher-epoch journal.

### DQ8. Name and cleanup

Use `index.redb.journal` and a fixed temp name for this engine.

- Remove the safe current-engine temp, stale journal and orphan journal while
  you hold the store handle. Resolve the final name before another rename.
- Preserve a higher-epoch or unreadable-version journal. Skip the optional
  journal while that final name stays occupied.
- Ticket 36 records the open owner decision for retired-engine files. Limit the
  current engine to a final journal and a temp file.

### DQ9. Adapter ownership

Keep journal ownership, durability and locking in the redb adapter. Add no
domain port operation. Use the current platform-bound store path form.

Ticket 32 owns any portable form. A future engine can use its own log.

Journal facts can reach backup and sync tools that copy `.traces/`. Mode `0600`
can prevent a different-user backup tool from reading the journal. The store
currently inherits the process `umask`.

### DQ10. Notices and exits

Replay and normal discard are quiet. A damaged optional journal alone does not
produce a notice. Keep the existing one-notice read fallback when the store
itself is unusable.

A higher-epoch store produces its existing notice and exit 0 after a successful
fresh parse. Ticket 34 owns the text. An optional journal failure does not
change the store write error rules.

### DQ11. Readers

A pure read-only operation does not open the journal. A query command that
enters writable refresh can validate and replay it before publishing a query.

- Run a case with a read-only vault, a journal and zero journal writes.
- Measure the no-change read-only control with and without a retained journal.

### DQ12. Decision input

- Measure parse-only redo and the saved work from each path. The screen's 125 ms
  edit bound includes unknown work.
- Profile scan, parse, link reads, reconcile and persistence separately.
- Include the replay persist and source validation.
- Compare no-journal `None`, no-journal `Immediate`, both journaled normal-write
  paths and the buffered control.
- Match stop times, fixture, profile and cache state.

An overlap between cost and benefit intervals proves no ranking. Use the ticket
26 rejection gate only when the saved-work upper bound does not exceed the
added-cost lower bound.

Ticket 27 proposes a host staleness bound after measuring validation costs and
stating a freshness target. Ticket 33 checks that bound against the real host.

## Planned fault matrix

Each row specifies a requirement, not a claim of completed coverage. Each row
checks full query equivalence with a fresh parse. Also check the test-only
outcome, parse count, revision, exit code and notice stream.

| Scenario | Injection | Expected path | Extra check |
|---|---|---|---|
| Stop during the temp write | Named partial-write stop | Redo | No temp facts enter the store |
| Stop after write, before file sync | Named stop | Redo | Writer cleans the temp file |
| Stop after file sync, before rename | Named stop | Redo | Final journal is absent |
| Stop after rename, before directory sync | Saved entry-present and entry-absent images | Replay or redo | Present entry has synced data |
| Stop after directory sync, before apply | Named stop | Replay | Journaled files are not reparsed |
| Stop before the normal `Immediate` barrier | Named stop | Replay or discard | Match the store revision after repair |
| Stop after the normal `Immediate` barrier | Named stop | Stale discard | The target revision is durable |
| Stop before unlink | Named stop | Stale discard | No second replay |
| Stop after unlink | Named stop and returned-entry image | No journal or stale discard | No removal directory sync |
| Stop inside replay commit | Named stop or sync interposer | Replay or discard | Full result and revision after repair |
| Stop inside a full rebuild | Named stop | Replay candidate or redo control | No partial store query |
| Lose a prior `None` base commit | Saved old-base image | Discard and redo | Do not apply against the wrong base |
| Keep the journal entry. Lose the journal data. | Saved image or body truncation | Discard and redo | Header and body checks fail closed |
| Lose the store commit. Keep the journal. | Saved old-store image | Replay | Full query and an empty next refresh |
| Keep the store commit and journal | Saved image | Stale discard | No duplicate revision advance |
| Change an unlisted source file | Edit before replay | Discard and redo | Check inbound links, notices and keys |
| Change a listed source file | Edit before replay | Discard and redo | No local re-parse inside replay |
| Add, delete or rename a source file | Edit before replay | Discard and redo | Check inbound links and path set |
| Keep size and `mtime`. Change the racy content. | Edit the content. Restore `mtime`. | Discard and redo | Original scan start and digest decide |
| Change the policy | Change fingerprint input | Discard and rebuild | No mixed-policy rows |
| Change each store identity part | Edit one field per run | Discard or preserve by epoch | No foreign base accepted |
| Use a lower or higher store epoch | Edit store `META` | Rebuild or fresh parse | Higher store gets one notice |
| Use a higher or unknown journal header | Edit journal prefix | Normal source path | Preserve the final journal |
| Copy or move the root | Create separate test roots | Replay or discard by snapshot | Root pathname alone is not a cause |
| Fork a store at the same incarnation and revision | Change affected rows in a copy | Reject or report protocol failure | Full query must equal fresh parse |
| Make the store directory unwritable | Repeat the command | Existing store error rule | At most a final journal and a temp |
| Make the vault read-only | Read-only mount or mode | Read-only query or fresh parse | Zero journal writes on a pure read |
| Fill the disk during the journal write | Inject ENOSPC | Normal source path | No growing temp files |
| Fill the disk after journal publication | Inject store-commit ENOSPC | Existing write error | Compare a no-journal write |
| Corrupt, truncate or lie about journal length | File surgery | Discard or preserve unknown | No panic or unbounded allocation |
| Plant a symlink or FIFO at the journal name | File surgery | Normal source path | No symlink following or blocking at open |
| Delete the store. Keep the current journal. | Remove the store in a test copy | Discard and redo | Check the full query |
| Start a second process with a live writer | Child process | Actual query fallback | Open error, not a transaction wait |
| Start a second process after writer death | Child process | One replay | Harness retry wait is separate |
| Run two `None` journaled writes | Stop before each checkpoint | Replay or redo by base | Resolve the first journal before the second |
| Drop the handle after a failed commit | Second writer acts | Discard or next-command replay | No same-command retry |

The probe needs named stop points for non-sync boundaries. The old macOS shim
does not provide them.

- Keep injector logs separate from child stderr.
- Use a Linux injector for the Linux fault run.

Add controls that disable replay, the base check and the source check. These
controls must fail the relevant rows.

Saved file images model selected lost writes. They do not prove behavior under
real power loss or arbitrary write reordering.

## Planned measurement matrix and decision rules

Use the shipped `release` profile for matched runs. The repo sets
`opt-level = "z"`, `panic = "abort"`, fat LTO and one codegen unit. The default
`bench` profile uses `opt-level = 3`.

Force Criterion to use the shipped profile.

The ticket 26 throwaway probe also used `opt-level = 3`. Keep its numbers as
historical bounds, not as a matched gate input. Run benchmarks in series,
without `RUSTFLAGS`.

| Measurement | Method | Decision served |
|---|---|---|
| Real parser-fact encode and journal bytes by facet | Criterion, release, serial | Compare with the historical 53.3 ms row-copy bound |
| Whole blob against streaming frames | Same parsed facts and fixture | Encode, decode, RSS and total replay time |
| Derive changed facets, affected edges and notices | Time each step at replay | Count work the screen omitted |
| Full source check and racy digests | Stable and all-racy fixtures | Price strict replay validation |
| Validation before or during writer hold | Same snapshot and output | Total latency, handover and lock hold |
| Open, apply, target-state check, checkpoint, close, unlink | Cold child and Criterion | Every no-failure cost, including deferred work |
| File and parent-directory sync | Timed prototype calls | Publication cost, not just syscall counts |
| Removal directory sync control | Separate timed branch | Price the omitted cleanup sync |
| redb repair after each stop class | Separate saved images for open and integrity check | Do not double-count repair |
| Journal bytes, store size and peak RSS | Per facet and per phase | Initial 2 GiB VM limit and layout claims |
| Writer-handle hold and other-process query | Actual concurrent children | Fallback latency and lock cost |
| Harness acquisition retry | Separate timed control | Do not call it production wait |
| No-change with no journal and a retained journal | Read-only open, no writer held | Measure the cold query control |
| First usable and repeated queries | Cold CLI and simulated host | Compare with a trusted fresh parse |
| No-journal `None` and `Immediate` writes | Same fixture and phase | Check if a journal adds value |
| Buffered journal control | Process stops only | Do not claim power-loss protection |
| Task scan, assembled rows and one-note replace | Decided eleven-facet shadow layout | Check the 85 percent and 2 times claims |
| Parse-only redo and each persist stage | Time separately | Calculate real saved work |
| Journal-induced store ENOSPC | A no-journal control on the same free space | Check for a new user-visible failure |

- Use 50, 5,000 and 50,000-note anchors.
- Extend near measured crossovers.
- Check the fixture count against about 0.75 tasks per note.
- Use a hub-linked shape for inbound links.

Run cold cases at least 20 times with an A-B-A control. Report the median and
range. Do not rank overlapping ranges.

Let `O` be the extra normal-write journal cost for each matched stop phase.
Include any later checkpoint and cleanup. Let `B` be the rival first usable
query time minus the journal first usable query time. Include source checks,
redb repair, decode, derivation and replay persist.

- Report `O / B` only where both are positive. A touch that invalidates the
  snapshot has no journal benefit.
- Reject a candidate only when the saved-work upper bound does not exceed the
  added-cost lower bound. An overlap proves neither a win nor a rejection.
- Do not invent a failure rate or a stop-time distribution.

## Cross-ticket contracts and delivery plan

- Ticket 26 now names an `Immediate` normal apply or replay as a removal
  barrier. Its release-panic handoff now says that the process aborts.
- Ticket 24 decides eleven facets. The production snapshot above has eight
  tables. The initial plan calls for a throwaway layout probe of the decided
  shape. The results distinguish that incomplete shadow from the real-layout
  replay probe.
- Ticket 25 keeps racy files racy from the original scan start. The journal adds
  digests for that source check, but adds no `commit_nonce`.
- Ticket 28 makes replay advance the revision. A forked store with matching
  incarnation and revision but different affected rows needs a fault test.
- Ticket 32 owns portable path bytes. The journal uses current store bytes until
  that decision changes the format.
- Ticket 33 checks the proposed staleness bound after ticket 27 measures the
  cost. A numeric bound needs an explicit freshness target.
- Ticket 34 owns notice text. A damaged optional journal alone does not become a
  store-read failure.
- Ticket 36 decides adoption and the owner for retired-engine file cleanup.
- The initial delivery plan names `../research/27-index-replay-prototype.md` as
  the results asset. That asset now exists. Ticket 27 links it under
  `## Answer`.

## Result-dependent decisions from the initial plan

The questions below remain the plan's decision checklist. The results asset
states which evidence exists and which gates remain open.

- Whether either journal path beats the no-journal `Immediate` path at a
  measured stop phase.
- Whether whole-blob or streaming replay wins at a large rebuild size.
- Whether a full-rebuild facts journal survives the real saved-work gate.
- Whether the same-revision fork test exposes a base-identity hole. A failed
  proof blocks adoption until the identity contract changes.
- Whether the journal makes a store write fail after it uses available space. A
  new failure blocks adoption.
- Which numerical host staleness bound fits the measured cost and the stated
  freshness target.
