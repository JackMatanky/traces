# e2-web-precedent: cold-start index stores

This file covers the web research for the index read model. Topic names refer to
the ticket sections.

## redb

### Design document

- URL: <https://raw.githubusercontent.com/cberner/redb/v4.3.0/docs/design.md>
  (fetched; matches the pinned crate version 4.3.0 [V: `Cargo.toml:138`,
  `Cargo.lock:1411-1412`]).
- One writer and several readers. redb uses MVCC on copy-on-write B+trees. A
  read transaction gets a private copy of the B+tree root. The store does not
  free a page while a registered read transaction can see it. A commit flips one
  "god byte" between two commit slots. Each slot holds a root page number, an
  XXH3 checksum of the root and of each child page, and a transaction id. A
  crash leaves the older slot valid. The god byte also holds a
  `recovery_required` flag, so the next open repairs the allocator state.
- Savepoints sit on the same MVCC structure. A savepoint registers as a read
  transaction, and it also saves a copy of the page allocator state, about 64 kB
  per 1 GB of values at 4 kB pages. Ephemeral savepoints die at drop. Persistent
  savepoints live in a system table and cross a restart.
- Lesson for ticket 24: D and B. The commit is one atomic root swap, so the
  store is one generation and a read cannot mix two generations. The MVCC
  reader-pins-pages rule warns against a long-lived host that keeps one read
  transaction open across renders: the store grows. Snapshot replacement, not a
  held read transaction, is the design that matches redb.

### Locking and the second process

- URLs: <https://docs.rs/redb/4.3.0/redb/struct.Database.html>,
  <https://docs.rs/redb/4.3.0/redb/enum.DatabaseError.html>,
  <https://docs.rs/redb/4.3.0/redb/struct.ReadOnlyDatabase.html> (fetched).
- `Database::open` and `Database::create` both take an exclusive file lock. When
  a second process opens the held file, the open fails with
  `DatabaseError::DatabaseAlreadyOpen`: "The Database is already open. Cannot
  acquire lock." A live `WriteTransaction` extends the lock past the drop of the
  `Database` value. Dropping `Database` closes it, flushes buffered data, and
  invalidates open read transactions (`StorageError::DatabaseClosed`).
  `ReadOnlyDatabase::open` lets several processes read one file, but not beside
  a writable `Database` handle. [V]
- The multi-process mode that lets several processes share one writable file
  sits behind the `experimental-multiprocess` cargo feature, which redb marks
  unstable. The default mode is one writer per file across all processes. [V:
  <https://raw.githubusercontent.com/cberner/redb/v4.3.0/Cargo.toml:62-65>]
- Lesson for ticket 24: B. The standard outcome for a second CLI process while a
  host holds the store is a failed open, not a queue and not a read-only view.
  The read-only open mode exists but excludes the writable handle. A cold CLI
  that cannot take the lock needs a defined fallback, and the current code maps
  every non-corruption open error to `StoreError::Redb` [V:
  `src/index/store.rs:894-921`].

### Read transactions and multimap tables

- URLs: <https://docs.rs/redb/4.3.0/redb/struct.ReadTransaction.html>,
  <https://docs.rs/redb/4.3.0/redb/trait.ReadableMultimapTable.html>,
  <https://docs.rs/redb/4.3.0/redb/struct.MultimapTable.html>,
  <https://docs.rs/redb/4.3.0/redb/trait.Value.html>,
  <https://docs.rs/redb/4.3.0/redb/struct.AccessGuard.html>,
  <https://docs.rs/redb/4.3.0/redb/struct.Savepoint.html>,
  <https://docs.rs/redb/4.3.0/redb/struct.WriteTransaction.html> (fetched).
- One read transaction gives one snapshot. It opens tables by name and returns
  `MultimapValue` iterators in ascending order. `Savepoint` plus
  `WriteTransaction::restore_savepoint` gives a rollback of one write. [V]
- The `Value` trait forces a `SelfType<'a>` borrowed view, `from_bytes`, and a
  `TypeName`. `AccessGuard::value()` decodes when called, and the guard releases
  at drop. redb therefore supports a lazily decoded value: the bytes stay in the
  page until the code asks for the value. The zero-copy window is the guard, not
  the transaction. [V]
- Lesson for ticket 24: C and D. A row that borrows from the store must keep the
  `AccessGuard` (or an `OwnedAccessGuard`) alive, so the row lifetime is the
  guard lifetime. A lazily decoded facet is possible: decode only the facets the
  query touches, inside one read transaction. Ticket 26 owns the row type; this
  evidence sets the lifetime shape it must work with.

## LMDB / heed reader locks

- URLs: <http://www.lmdb.tech/doc/starting.html>,
  <https://docs.rs/heed/latest/heed/struct.RoTxn.html>,
  <https://docs.rs/heed/latest/heed/struct.Env.html>,
  <https://docs.rs/heed/latest/heed/struct.EnvOpenOptions.html> (fetched; the
  TLS page `lmdb.tech/doc/group__mdb.html` did not open, so the API details
  below rest on the heed docs and the getting-started page).
- LMDB keeps POSIX file locks. One process must not open the same environment
  twice. A close of one handle drops the locks of the other. Read transactions
  are cheap but must stay short. An active reader stops the reuse of freed
  pages, so the file grows. heed exposes `clear_stale_readers` for dead reader
  slots.
- On macOS, heed notes a limit of about 10 POSIX semaphores per process. [V]
- Lesson for ticket 24: B. Two engines from the same family agree on one cost: a
  long-held read view grows the file. The precedent pattern is short read
  transactions plus an in-memory snapshot that the host replaces. This is the
  shape the ticket already leans toward for ticket 17.

## SQLite as a derived index (zk) and its locking

- URLs: <https://zk-org.github.io/zk/notes/notebook.html>,
  <https://zk-org.github.io/zk/notes/note-filtering.html>,
  <https://raw.githubusercontent.com/zk-org/zk/main/README.md>,
  <https://www.sqlite.org/lockingv3.html>, <https://www.sqlite.org/wal.html>,
  <https://www.sqlite.org/isolation.html> (fetched).
- zk is the closest precedent: a CLI plus an editor host over one note vault. zk
  keeps one SQLite database at `.zk/notebook.db` and serves search, tags, links
  and dates from it. A plain `zk index` updates the changed notes. A forced run
  reindexes all notes. The DB is derived state: the notes are the truth. [V]
- SQLite rollback mode has five lock states (UNLOCKED, SHARED, RESERVED,
  PENDING, EXCLUSIVE). Rollback mode admits many readers or one writer, never
  both. WAL mode admits readers and one writer at the same time. The reader uses
  a shared-memory wal-index and sees a snapshot from the start of its read
  transaction. A second process on the same file gets SQLITE_BUSY in rollback
  mode or waits in WAL mode. It is never shut out of reads. [V: lockingv3 §3,
  wal §2.2, isolation]
- Lesson for ticket 24: A and B. zk answers with one engine and one path. The
  SQL engine pushes down every query shape, so there is no second execution path
  to keep equal. SQLite chose concurrency that admits the second reader. redb in
  its default mode does not. If ticket 24 keeps redb, the read model must state
  what a CLI does when the host holds the store. SQLite's answer, read the last
  commit and block on write, is not available for free.

## Git index and commit-graph

- URLs: <https://git-scm.com/docs/racy-git>,
  <https://git-scm.com/docs/gitformat-index>,
  <https://git-scm.com/docs/gitformat-commit-graph> (fetched).
- The git index stores per-file stat data (ctime, mtime, size) and compares it
  to the working tree. The racy-git problem: a file modified within the same
  timestamp tick as the index write looks clean but is not. Git answers with a
  rule. An entry whose mtime is not older than the index file's own mtime is
  "racily clean"; git then compares content, not only stat data. The racy-clean
  fallback is slow (2.22 s against 0.14 s in the doc's example). Git also zeroes
  cached sizes for racily clean entries on the next write to retire them. [V:
  racy-git]
- The index file ends with a checksum over its content. Optional extensions
  carry their own length and signature. Unknown ones with an upper-case first
  byte are ignored. The `TREE` extension caches directory-to-tree answers and
  marks stale nodes with entry_count -1, so a rebuild of the tree is
  incremental. The `link` (split index) mode keeps the bulk of the entries in a
  shared index file. It records only the changes (delete and replace bitmaps) on
  top. [V: gitformat-index]
- The commit-graph is a derived, rebuildable acceleration structure in chunks
  (OID fanout, OID lookup, commit data, optional bloom filters). A chain of
  files extends it; the `BASE` chunk lists the base graphs. A hash mismatch or a
  wrong hash algorithm means git ignores the file with a message, not an error.
  [V: gitformat-commit-graph]
- Lesson for ticket 24: B and D. Git treats its derived index as disposable.
  Every consumer must accept a missing, stale or corrupt cache and fall back to
  a full walk. The racy-timestamp rule is the precedent for ticket 25's racy
  timestamps. The checksum-then-ignore rule is the precedent for the Frozen rule
  that a lost store heals by a rebuild. The split index and the commit-graph
  chain also show one known safe layout. A small delta layer sits on top of a
  full base layer for a derived store.

## ruff and rumdl on-disk caches

- URLs:
  <https://raw.githubusercontent.com/astral-sh/ruff/main/crates/ruff/src/cache.rs>,
  <https://github.com/astral-sh/ruff/issues/12284>,
  <https://raw.githubusercontent.com/rvben/rumdl/main/src/cache.rs>,
  <https://docs.astral.sh/ruff/configuration/> (fetched).
- ruff's cache key is a hash of the package root plus the settings, then the
  Ruff version selects a directory: `.ruff_cache/<VERSION>/<key>`. The per-file
  key is the file's mtime and permission mode. The cache serialises with rkyv.
  On read failure (missing, unparsable, wrong root) ruff warns and continues
  with an empty cache. On write, ruff writes a temporary file and renames it
  over the target. A rename failure on Windows (`PermissionDenied`, the
  multi-process race) is logged, not raised. The run's results are already
  computed, so the cache write is non-fatal (issue 12284, fixed in PR 12302
  "Make cache-write failures non-fatal"). Other write failures still return an
  error from `persist` [V: cache.rs lines 157-190]. rumdl copies the ruff
  design. The key is (content hash, config hash, rules hash, version). Storage
  is one JSON file per entry under `.rumdl_cache/<version>/`. Writes go through
  a pid+counter temp file and a rename. A failed write logs at debug level and
  never fails the run. rumdl source: cache.rs lines 595-604, "Write to cache
  (log errors but don't fail - cache is optional)" [V].
- rumdl adds an explicit `CacheMissReason` enum (missing, unreadable, invalid,
  file/config/rules/version changed) so a cache miss is diagnosable instead of
  silent. [V]
- Lesson for ticket 24: B. The standard rule for a derived cache write that
  fails is fail open. The tool logs the failure, keeps the computed results, and
  loses only the caching benefit. ruff's issue 12284 is the named precedent. A
  multi-process cache-write race made the whole run exit 2, and the fix demoted
  the failure to a log line. But ruff and rumdl are linters whose result is
  already in memory when the write fails. The store pushdown in this repo
  computes rows from the store after a write. The analogous choice is therefore
  which consumer sees the failure. The current code splits by consumer:
  `refresh` fails open and `refresh_store` fails closed. This matches ruff's
  split by consumer. The rumdl `CacheMissReason` enum is the pattern for making
  a corrupt or drifted store diagnosable instead of silent.

## Cargo registry index cache

- URLs: <https://doc.rust-lang.org/cargo/reference/registry-index.html>,
  <https://doc.rust-lang.org/stable/nightly-rustc/cargo/sources/registry/index/cache/struct.SummariesCache.html>
  (fetched).
- The registry index is one text file per crate. Each line is one version's
  JSON. Cargo then builds a binary `.cache` file beside it (`SummariesCache`).
  The file has a three-version header (cache version, index schema version,
  index file version) and then (semver, 0, JSON blob, 0) pairs. The extracted
  semver lets Cargo match versions without parsing the JSON. The header versions
  let a newer Cargo regenerate rather than misread an older cache. The cache is
  documented as internal and unstable. [V]
- Lesson for ticket 24: D and B. Cargo splits "what the store keeps" from "what
  the format needs". The version travels in the header, so a schema change is a
  cache version bump and a regeneration, not a migration. The header carries
  three independent version numbers. Schema drift and cache-format drift are
  different events. If ticket 24 keeps a `meta` table, this is the precedent for
  one design. The meta table stores the format version and the parser identity,
  and a rebuild follows a mismatch (ticket 25 owns the rule).

## rkyv, zerocopy, postcard: lazy decode tradeoffs

- URLs: <https://docs.rs/rkyv/latest/rkyv/>,
  <https://docs.rs/rkyv/latest/rkyv/trait.Archive.html>,
  <https://docs.rs/rkyv/latest/rkyv/validation/index.html>,
  <https://docs.rs/zerocopy/latest/zerocopy/>,
  <https://docs.rs/postcard/latest/postcard/>,
  <https://postcard.jamesmunns.com/wire-format.html> (fetched).
- rkyv keeps data in its archived form. `rkyv::access` validates pointers with
  bytecheck, `access_unchecked` skips the check, and the archived view needs no
  deserialisation. Its compatibility rule is strict: data stays readable only
  while the schema, the format-control features (endianness, alignment, pointer
  width) and the rkyv semver all match. A schema change makes the old bytes
  unreadable, not partly readable. zerocopy is the stricter cousin. A type
  derives `FromBytes`/`TryFromBytes`, and the compiler proves the transmute.
  There is no partial decode of an older layout at all. [V]
- postcard is a compact serde format with a stable v1 wire format. It is
  explicitly "NOT a self-describing format". Sender and receiver must share the
  schema. Backwards/forwards compatibility between schema revisions is outside
  the format's scope, on the user. Integers are varints, so records are compact,
  but every read decodes into owned Rust values. There is no validated zero-copy
  access mode. [V]
- Lesson for ticket 24: D. The current store (postcard bytes in redb values)
  pays a full decode per row. The lazily decoded alternative is rkyv-style
  archived access. Its price is a hard schema-freeze rule and a validation pass
  on open (or unsafe access). A facet split with per-facet decode is the middle
  path: postcard stays, and "what loads when" is the lever instead of the
  encoder. Ticket 23's fact that a decoded note takes about twice its encoded
  size is the measurement that decides if the change pays.

## Roaring bitmaps and FST for inverted path sets

- URLs:
  <https://docs.rs/roaring/latest/roaring/bitmap/struct.RoaringBitmap.html>,
  <https://docs.rs/fst/latest/fst/> (fetched).
- `RoaringBitmap` is a compressed set of u32 with union, intersect, difference,
  `intersection_len`, range iteration, rank and select, and a standard
  serialised format with `serialize_into` / `deserialize_from`. Multi-way set
  operations run in one pass over many bitmaps. [V]
- `fst` stores ordered string sets/maps as finite state transducers. Build once,
  then search with automata or ranges. The set can stay on disk behind a memory
  map. Set operations (union, intersection, difference) run across many stored
  sets with memory proportional to the stream count, not the key count. Keys
  must be inserted in lexicographic order. [V]
- Lesson for ticket 24: D and F. If the four multimap tables shrink, the
  in-memory alternative the load can rebuild has first-class library support:
  `RoaringBitmap` over a stable per-path number, or `fst::Set` over the paths
  themselves. Both make "build in memory at load" cheap because the serialised
  forms are designed for it. This is evidence for the Dataview-pattern
  alternative (store `files`, `notes`, `meta` only, rebuild the inverted sets at
  load), which the ticket lists as an option.

## Columnar versus row storage for a small metadata store

- URLs: <https://duckdb.org/docs/2.0/guides/performance/indexing>,
  <https://parquet.apache.org/docs/overview/motivation/> (fetched. The
  `internals/storage` page redirects and its markdown fetch returned the
  navigation shell. The zonemap and row-group numbers above come from the
  indexing guide and the search-result summary, not the storage page.)
- DuckDB stores tables in row groups. Inside a group each column is contiguous.
  Zone maps (min/max per column per group) let a predicate skip whole groups.
  Ordered data makes the skipping strong; random data makes it weak. Exact
  lookups get an ART index, which is a separate structure the user creates.
  Parquet's motivation page states the general case. Columnar representation
  buys compression and efficient scans of few columns over many rows. [V]
- Lesson for ticket 24: D and A. Columnar pays off when a query touches few
  fields of many rows and rows are many (DuckDB's row groups are about 122,880
  rows). The vault sizes here are 50 to 50,000 notes. The read model is a full
  row per file. Research 11 already shows both paths are fast at 20k. Columnar
  facts would buy scan speed the evidence gate does not ask for, at the cost of
  a second layout to keep equal. The evidence supports row storage with
  per-facet decode, not a columnar rewrite.

## Measured context from the main agent

- A second process that opens a held redb file fails at once with
  `DatabaseAlreadyOpen` [V: measurement by the main agent]. This matches the
  documented lock behavior above.
- redb 4.3 has an `experimental-multiprocess` mode [V: measurement by the main
  agent; feature listed in the redb `Cargo.toml` lines 62-65].

## Cross-cutting answers the ticket asks for

- What a second process sees when it opens a held redb database: a failed open
  with `DatabaseAlreadyOpen` on every platform redb supports locking on. The
  read-only open mode refuses to run beside a writable handle. The only
  multi-writer escape is an unstable, non-default cargo feature. A cold CLI must
  therefore treat "store held" as a normal, expected state. The current mapping
  of all open errors to `StoreError::Redb` hides it [V:
  `src/index/store.rs:894-921`, ticket Facts].
- The standard rule for a failed cache write: fail open. ruff, rumdl and git all
  treat the derived store as optional. A failed write is logged (at warn or
  debug level) and the run keeps its computed results. A corrupt or stale read
  falls back to a full recompute. The only failure class the precedents keep
  fatal is a failure that corrupts the primary data, which a derived store
  cannot do. The ticket's split rule (refresh open, `refresh_store` closed) is
  stricter than every precedent found. It needs a consumer-level argument, not a
  storage-level one. The pushdown path fails before it has results; the refresh
  path fails after them.
