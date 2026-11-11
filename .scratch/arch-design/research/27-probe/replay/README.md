# Ticket 27 replay/fault probe (throwaway)

## Current version and retained history

This branch contains the real-layout replay probe. It uses the eight
production tables in `.traces/index.redb`, not the decided eleven-facet
layout. The macOS release-profile fault suite passes 42 of 42 cases.

The source remains on branch `scratch/t27-probe-worktree`. The branch adds
probe access through `test-utils`. It does not adopt a production journal.
The main-repository research asset records the results and remaining limits:
`.scratch/arch-design/research/27-index-replay-prototype.md`.

The sections below retain the earlier shadow-probe record. They explain the
original 25-case suite and its limits. They do not describe the current
implementation. In particular, `stage ROOT none` is the old deferred command.
The current command is `stage ROOT deferred`.

## Run the current fault suite

- Run the commands from this worktree's repository root.
- Run them in series.
- Do not set `RUSTFLAGS`.
- Use a new `/tmp/t27-example` directory for the example.

```sh
cargo test \
  --manifest-path .scratch/arch-design/research/27-probe/replay/Cargo.toml \
  --release --test fault -- --nocapture --test-threads=1
cargo run \
  --manifest-path .scratch/arch-design/research/27-probe/replay/Cargo.toml \
  --release -- seed /tmp/t27-example 12
PROBE_STOP=after_publication cargo run \
  --manifest-path .scratch/arch-design/research/27-probe/replay/Cargo.toml \
  --release -- stage /tmp/t27-example
cargo run \
  --manifest-path .scratch/arch-design/research/27-probe/replay/Cargo.toml \
  --release -- recover /tmp/t27-example
cargo run \
  --manifest-path .scratch/arch-design/research/27-probe/replay/Cargo.toml \
  --release -- verify /tmp/t27-example
```

## Current pause, panic, and weak-base controls

The current code adds these controls. The historical stops section lists only
the original names.

- `PROBE_PAUSE=<name>` parks the process at a named point. Send `SIGCONT` to
  resume. The pause points are `after_publication`,
  `before_checkpoint_commit`, and `replay`.
- `PROBE_PANIC=<name>` makes the stop at `<name>` panic. Use it when the
  crash must show as a panic, not as an abort.
- The stop `before_rebuild_commit` is new. It sits before the source-rebuild
  persist call.
- The durability of the journal publish path is now native redb
  `Durability::None`. The checkpoint makes the data durable. The fallback
  path uses native `Durability::Immediate`.
- `PROBE_WEAK_BASE=1` weakens only the base checks. The replay still checks
  the target fact hash. A mismatch returns an error and keeps the journal.
- `PROBE_DB_RO=1` opens the store with a true read-only redb handle. It
  prints table counts only.

## Current measurement harness

- `measure ROOT NOTES RUNS` runs serial samples. Each sample starts a new
  process with a fresh fixture.
- `NOTES` must be at least 9. The policy order is `a`, `b`, `a`, `c`, `a`.
  Policy `a` is the unjournaled native `Immediate` write. Policy `b` adds a
  journal publish. Policy `c` defers the commit to the checkpoint.
- The run also measures source-check placements A and B with the same work,
  stable and racy snapshots, true read-only digest controls, and five
  recovery and five parse-fallback scenarios.
- After each fixture is ready, the harness applies the per-file cache hint
  from `benches/common/cache.rs`. The hint is best-effort. This is not a
  cold-storage measurement.
- `measure-case ROOT CASE` runs one prepared case. It is an internal command.
- Each sample prints its phase components on one `components` line. This run
  does not print an aggregate table. Summarize the samples yourself.
- Known limits: the model has the current eight tables, not eleven facets.
  redb does not separate its write and sync costs, so the `redb_sync_split`
  field is always `unavailable`. `verify` compares selected query accessors,
  not every field.

## Historical shadow probe: purpose and commands

The isolated Cargo crate originally collected parsed `Note` and `FileMeta`
facts through `traces::testing::IndexerService::build`. It left production
code and the production `index.redb` unchanged.

In a temporary vault, it wrote `.traces/replay-shadow.redb` and
`.traces/index.redb.journal`. The journal contained postcard-encoded parsed
facts, not copied redb rows. Replay decoded those facts, resolved inbound
links through `InlinkMap`, and rebuilt the shadow facets. That shadow store
did not implement the ticket-24 decided layout.

| Historical command | Behavior |
|---|---|
| `stage ROOT none` | Retains a deferred journal. |
| `checkpoint ROOT` | Checks the stored identity, revision, source snapshot, and target row hash under the writer handle. Removes the journal only after its `Immediate` checkpoint succeeds. |
| `stage ROOT no-journal` | Runs the unjournaled `Immediate` control. |
| `recover ROOT no-replay` | Runs the negative control. |
| `query ROOT` | Reads the shadow page count when the store is available. If another process holds the store, `query` parses the sources and prints fresh query counts without opening the journal. |
| `hold ROOT` | Retains a store handle until stdin closes for the concurrent-process case. |

### Historical injected stops and errors

`PROBE_STOP` aborts the child. Child stderr includes `probe_stop=<name>`.
The original stop names are:

- Stage: `before_write`, `partial_write`, `before_file_sync`,
  `after_file_sync`, `before_dir_sync`, `after_publication`,
  `before_commit`, `after_commit`, `after_checkpoint`, `before_unlink`,
  `after_unlink`
- Recovery: `mid_replay`, `before_commit`, `after_replay_commit`,
  `before_checkpoint_commit`, `after_checkpoint_commit`,
  `before_rebuild_commit`

`after_commit` is before the durability barrier: the store commit is not
yet durable, and recovery replays the retained journal. `after_checkpoint`
is after the barrier: the commit is durable, the journal is stale, and
recovery discards it and reparses.

`PROBE_FAIL` accepts `journal_write`, `journal_file_sync`,
`journal_dir_sync`, or `store_commit`. These inject an error string. They do
not exhaust storage. Failed optional publication uses an unjournaled
`Immediate` store write. An injected store-commit error remains a
store-write error on either path.

### Historical fixture, output, and crash model

The original assertions establish outcomes, not latency. The 12-note fixture
contains 9 tasks and one extra hub note with no tasks.

- `stage` prints parse, encode, commit, checkpoint, journal-byte,
  changed-fact, durability, and publication values.
- `recover` prints its outcome, store reopen, base snapshot, decode,
  replay or parse time, and revision.
- Each printed time is one observation, not a benchmark result.

The saved-image case restores a precommit redb file after an ordinary `None`
apply. It compares the stored facets with fresh parser facts. This models
one lost store commit, not power loss, torn pages, or arbitrary write
reordering. The other process-stop cases call `std::process::abort` at named
points.

On macOS, the probe requests `F_FULLFSYNC` for the journal and its parent
directory. If the directory rejects the request, stderr reports an `fsync`
fallback. On Linux, the probe calls `sync_all`. Journal removal has no
directory sync.

### Historical platform results

The 25-case release suite passed on macOS/APFS with redb 4.3.0. It also
passed in the Linux VM with redb 4.5.0. The redb 4.3.0 build received a kill
while compiling `traces` in the 2 GiB VM. At that stage, the pinned Linux
run remained unconfirmed. The research asset records the later successful
pinned run after the memory increase.

- Copy the repository into `/work/traces` for a Podman check.
- Do not use a bind mount.
- Run the same command with `--locked` from `/work/traces`.

The container needs Rust/Cargo, `mkfifo`, and enough memory for the build.
Podman timings do not represent native-Linux latency.

## Historical limits and the adoption boundary

These limits describe the original shadow probe. Later results do not turn
that earlier probe into evidence for the decided layout.

| Area | Original limit |
|---|---|
| Source snapshot | Checks regular-file digests, sizes, mtimes, Unix ctimes, the original scan start, and racy bits. Rejects non-UTF8 paths and symlink sources. Does not prove atomicity against edits concurrent with the final check. |
| Writer base | Hashes the entire shadow store. Detects the checked same-revision forks, but differs from the proposed adapter's cost and contract. |
| Facets | Has a file-size range control, not full numeric/date range keys. Public test-utils cannot expose parser notices. The `notices` facet stays empty. Each commit rebuilds all eleven shadow facets. This cannot measure incremental deletion or replay savings. |
| Query equivalence | `verify` compares the full assembled entries (file metadata, note facts, inbound links) from the replayed store with a fresh parse of the same root, and compares projected page/list/task rows with production `QueryService`. These checks do not establish all eleven decided facet semantics or range-key behavior. |
| Missing cases | Policy drift, all format/identity mutations, real-host retries, limited-disk failures, and full-rebuild timings remained untested. `PROBE_DB_RO` opens a true `redb::ReadOnlyDatabase`, but only for digest controls. |
| Concurrency | Covers one exclusive holder and a source-query fallback. |
| Unknown/higher headers | Preserves the journal file. Runs production `QueryService` on a fresh parse, printing query counts only, without a persistent generation. |
| Decode bounds | The 64 MiB file bound rejects oversized reads and false header lengths before body allocation. A hostile checksum-valid postcard body still needs an allocation-aware bounded decoder. |
| Storage failures | Optional publication has a fallback. Actual ENOSPC during or after publication remained untested. The injected error occurs before redb's commit, not during a full-disk commit. A new journal-induced store-write error blocks adoption. |
| Commit uncertainty | A redb error returns without an in-command retry. The original probe did not check reopened uncertain outcomes or measure repair time. |

- Use ticket 27's full fault and measurement matrix before proposing adoption.
- Keep the current real-layout results separate from this historical record.
