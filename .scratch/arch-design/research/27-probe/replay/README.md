# Ticket 27 replay/fault probe (throwaway)

This isolated Cargo crate harvests real parsed `Note` and `FileMeta` facts with
`traces::testing::IndexerService::build`. It leaves production code and the
production `index.redb` unchanged. In a temporary vault, it writes
`.traces/replay-shadow.redb` and `.traces/index.redb.journal`. The journal holds
postcard-encoded parsed facts, not copied redb rows. Replay decodes facts,
resolves inbound links through `InlinkMap`, and regenerates shadow facets.
The shadow store is not the ticket-24 decided layout.

Run these commands **in series** from the repository root, without `RUSTFLAGS`:

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

`stage ROOT none` retains a deferred journal. `checkpoint ROOT` verifies
the stored identity, revision, source snapshot, and target row hash under
the writer handle. It removes the journal only after its `Immediate`
checkpoint commits successfully. `stage ROOT no-journal` is the unjournaled
`Immediate` control. `recover ROOT no-replay` is a negative control.
`query ROOT` reads shadow page count when the store is available. If another
process holds it, `query` parses the sources and prints fresh query counts
without opening the journal. `hold ROOT` retains a store handle until stdin
closes; it is for the concurrent-process test.

`PROBE_STOP` accepts `before_write`, `partial_write`, `before_file_sync`,
`after_file_sync`, `before_dir_sync`, `after_publication`, `before_commit`,
`after_commit`, `before_unlink`, `after_unlink`, `before_replay`,
`after_replay_commit`, `before_checkpoint_commit`, and
`after_checkpoint_commit`. A stop aborts the child. Child stderr includes
`probe_stop=<name>`. `PROBE_FAIL` accepts `journal_write`,
`journal_file_sync`, `journal_dir_sync`, or `store_commit`. These inject
an error string; they do not exhaust real storage. A failed optional
journal publication uses an unjournaled `Immediate` store write. An
injected store-commit error remains a store-write error with either path.

Tests assert outcomes but make no latency claim. The 12-note fixture has
9 tasks, plus one hub note with no tasks. `stage` prints parse, encode,
commit, journal-byte, and changed-fact observations. `recover` prints its
outcome, source parse count, replay time, and revision. Each printed time
is a single observation, not a benchmark result.

The saved-image test copies a precommit redb file back after an ordinary
`None` apply and checks stored facets against fresh parser facts. This
simulates one lost store commit. It does **not** simulate power loss,
torn pages, or arbitrary write reordering. Other process-stop tests call
`std::process::abort` at named points. On macOS, the probe requests
`F_FULLFSYNC` for the journal and parent directory. If the directory
rejects that request, stderr reports an `fsync` fallback. On Linux, it
calls `sync_all`. Journal removal has no directory sync.

The 25-case release suite passed on macOS/APFS with redb 4.3.0.
It also passed in the Linux VM with redb 4.5.0. A redb 4.3.0
rebuild was killed while compiling `traces` in the 2 GiB VM, so
the pinned-version Linux run remains unverified. For a Podman check,
copy the repository into the container at `/work/traces`, not a bind
mount, and run the same command with `--locked` from `/work/traces`.
The container needs Rust/Cargo, `mkfifo`, and enough memory to build.
Podman timings do not represent native-Linux latency.

## Limits that prevent adoption

- The source snapshot checks all regular files with digests, sizes, mtimes,
  Unix ctimes, original scan start, and racy bits. It rejects non-UTF8
  paths and symlink sources. It does not prove atomicity against edits
  concurrent with the final snapshot check.
- The writer base check hashes the entire shadow store. It detects tested
  same-revision forks, but differs in cost and contract from the proposed
  adapter.
- The shadow has a file-size range control, not full numeric/date range
  keys. Public test-utils cannot expose parser notices, so the `notices`
  facet remains empty. Each commit rebuilds all eleven shadow facets.
  This cannot measure incremental deletion or replay savings.
- `verify` compares all stored facet entries against a fresh parse. It
  compares projected page/list/task paths and task text against production
  `QueryService`. The public API cannot construct a production
  `WorkspaceIndex` from replayed facts. **This does not establish full query
  equivalence** or all eleven decided facet semantics.
- Policy drift, all format/identity mutations, a true read-only redb
  handle, retries in a real host, limited-disk failures, and full-rebuild
  timing remain untested. The concurrent-process test exercises one
  exclusive holder and a source-query fallback. Unknown/higher journal
  headers preserve the file and run production `QueryService` on a
  fresh parse; this command prints query counts only, with no
  persistent generation.
- The 64 MiB file bound rejects oversized reads and a false *header*
  length before body allocation. A hostile checksum-valid postcard
  body still needs a bounded allocation-aware decoder. Optional
  publication failure uses a fallback, but actual ENOSPC during or
  after publication remains untested. The injected store-commit error
  happens before redb's commit, not when disk space runs out mid-commit.
  A real journal-induced store-write error would block adoption.
  A redb commit error returns without in-command retry; reopened
  uncertain-commit outcomes and repair timing remain untested.

Use the rest of ticket 27's fault and measurement matrix before proposing
adoption.
