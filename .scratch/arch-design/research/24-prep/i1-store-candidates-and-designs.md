# Store candidates and the design that suits each

This note supports the grilling of
`issues/24-index-read-model-and-query-execution.md`. It lists the stores that
the comparison measures. It says how a user of each store would lay out the data.
A fair comparison needs this. A SQL store with a key-value layout, or a
key-value store with a SQL layout, shows the layout, not the engine.

Tags: [V] verified in a source or a run. [H] hypothesis.

## Candidates

Versions come from `cargo search` on 2026-10-29. Sources: `docs.turso.tech`,
`github.com/tursodatabase/turso` (README and `COMPAT.md`), `docs.rs/rusqlite`,
`lib.rs`, the fjall 3.0 release post and `sqlite.org/wal.html`.

| Store | Version | Kind | Decision |
|-------|---------|------|----------|
| redb | 4.3.0 (4.4.0 exists) | Pure Rust B-tree, copy-on-write | Include. It is the current store. |
| rusqlite with bundled SQLite | 0.41.1 | C SQLite, the reference SQL engine | Include. It is the standard SQL baseline [V]. |
| turso | 0.9.0-pre.2 | Rust rewrite of SQLite, async API | Include. The maintainer asked for it. It is a pre-release [V]. |
| heed (LMDB) | 0.23.1 | Memory-mapped B-tree, zero-copy reads | Include. LMDB is the usual fast read baseline [H]. |
| fjall | 3.1.13 | Pure Rust LSM tree | Include. It is the write-optimised Rust option [V]. |
| DuckDB | duckdb 1.20000.1 | Columnar, vectorised | Include as the scan-speed outlier. Its build is slow [V]. |
| In-memory snapshot | none | Load all facets, then filter | Include. This is the no-store cold cost. |
| libsql | 0.10.0 | C SQLite fork | Exclude. Same engine family as rusqlite. Turso itself points new work to the `turso` crate [V]. |
| RocksDB | 0.25.0 | C++ LSM | Exclude. Heavy C++ build. fjall covers the LSM case [H]. |
| canopydb | 0.2.5 | Young Rust B-tree | Exclude. It is a young crate [H]. |
| sled | none | Rust LSM-like | Exclude. It is not maintained [H]. |
| sqlx | 0.9.0 | Async SQL toolkit | Exclude. It is a layer over SQLite, not an engine [V]. |

## Facts that change the design

- Turso reports SQLite 3.50.4 compatibility. It supports `CREATE INDEX`, `JOIN`
  and `ORDER BY`. `GENERATED` supports virtual columns only. `WITHOUT ROWID` is
  experimental and cannot update or delete. `PRAGMA mmap_size` is missing.
  `locking_mode` supports `EXCLUSIVE` only. Multi-process WAL coordination is
  experimental [V: `COMPAT.md`].
- The `turso` crate is async. A probe needs a Tokio runtime [V].
- SQLite in WAL mode admits readers beside one writer. It answers the second
  process problem of ticket 24 without a custom rule [V: `sqlite.org/wal.html`].
- LMDB admits many readers and one writer across processes through a lock file
  [H]. A read transaction pins a snapshot, as in redb.
- fjall holds the directory with one process [H].
- zk is the closest precedent. It keeps a SQLite database for a note vault. It
  stores note rows, link rows and tag (collection) rows, and it filters in SQL
  [V: `c1-digests-lsp-zk.md`, finding 5].

## How each store would hold this data

The workload has four parts. The first part is a source: a folder, a tag, a
class or an exclusion. The second part is note-level filters on fields. The
third part reads tasks or lists for the surviving notes. The fourth part is a
sort.

### redb (key-value, today)

One table for each facet, keyed by the note path. A value is a postcard record.
A folder source is a key range. A filter on fields decodes a record. A tag
filter scans the `tags` table. The probes so far use this layout.

### SQLite and Turso (relational)

A SQL user would not store one blob for each facet and scan it. The user would
make narrow tables and let the engine use indexes. The layout has two kinds of
tables. Pruning tables let the engine find notes. Blob columns hold the exact
facet for rows that the query returns.

```sql
note(id INTEGER PRIMARY KEY, path TEXT UNIQUE NOT NULL,
     fields BLOB, headings BLOB, outlinks BLOB, lists BLOB)
tag(note_id INTEGER, tag TEXT, PRIMARY KEY (tag, note_id))
field(note_id INTEGER, key TEXT, txt TEXT, num REAL,
      PRIMARY KEY (key, txt, note_id))        -- plus an index on (key, num)
task(note_id INTEGER, idx INTEGER, status TEXT, due TEXT, body BLOB,
     PRIMARY KEY (note_id, idx))
```

- `field` holds each scalar frontmatter value. A list or an object value stays
  in the `fields` blob. A query on such a value is a residual filter [H].
- The `path` index serves a folder source as a range (`path >= 'f3/' AND path <
  'f30'`). A `UNIQUE` index replaces `WITHOUT ROWID`, because Turso does not
  support `WITHOUT ROWID` for updates.
- A write of one note deletes the old rows of that note and inserts the new
  rows in one transaction. This is how zk updates a note [V].
- Settings for SQLite: WAL, `synchronous = NORMAL`, a prepared statement cache,
  one transaction for a bulk load, and the index build after the load [V: the
  SQLite tuning pages found in the search].

### heed (LMDB)

The same layout as redb. LMDB calls a table a named database. A value is a
slice that points into the memory map, so a read copies nothing until the
decode. A user would size the map once and open many named databases [H].

### fjall (LSM)

A keyspace for each facet, keyed by the path. A folder source is a prefix scan.
An LSM tree writes fast and reads a range through sorted runs. A user would
tune the block cache and the value separation threshold [V: fjall 3.0 post].

### DuckDB (columnar)

A table with typed columns for the hot scalar fields (status, rating, date,
tags as a list column). Blob columns hold the facets. The engine scans a column
without reading the others. A user would load with the appender API and let the
engine scan, with no secondary indexes [H].

### In-memory snapshot

Read all facet records from one store, decode in parallel with rayon, and run
the filter over memory. This costs what a cold CLI pays when it has no
store-side pruning.

## Fairness rules

1. All stores load the same data: the 50,000 synthetic notes and the same
   facet bytes. The facet bytes come from the real parser and postcard.
2. Every store answers the same pipelines and returns the same row set. The
   probe checks the row counts against the redb result.
3. A store uses its native strengths. SQL stores filter in SQL with indexes. Key-value
   stores use ranges and point reads. The snapshot filters in memory.
4. The time includes opening the store, the query and a sort of the result. It
   does not include process start.
5. Each run is a new process. The first run after a build is discarded.
6. Report the median of five runs, the file size, the peak memory, the bulk
   build time and the time to replace one note.

## What this comparison cannot show

- It uses a synthetic vault of small notes with scalar frontmatter.
- It runs on macOS with a warm OS cache.
- It measures no `QueryService` code and builds no rows.
- Turso is a pre-release. A later version can change its numbers.

## Crates read after the first round

The first round did not read these crates. The sources are the README files in
the local cargo registry (2026-10-29). I did not run any of them.

| Crate | Version | Facts from the README | Decision |
|-------|---------|-----------------------|----------|
| surrealkv | 1.1.1 | LSM tree with an async commit pipeline on Tokio. Snapshot isolation with non-blocking readers. The authors warn that the on-disk formats are not frozen. The vendor benchmark ranks LMDB first for range scans [V]. | Not run. It is an LSM tree like fjall. I did not find its process-locking rule [H]. |
| turbokv | 0.6.0 | Async LSM store. One open handle owns the directory. Its Bloom-filter format needs hardware AES, so the build needs target-feature flags. The benchmark compares it with fjall and redb only [V]. | Not run. Young crate with build flags. |
| native_db | 0.8.2 | Typed models with primary and secondary indexes. It is built on redb (version 1.5.1 in the lock). The API is not stable [V]. | Not run. It is redb with managed indexes, so it keeps the redb open cost and lock [H]. |
| sled | 1.0.0-alpha.124 | Pre-release rewrite. | Not run. The first round called sled unmaintained. The registry shows a pre-release, so that claim was too strong. |
| surrealdb | 4.0.0-beta.1 | Distributed document-graph database. | Not run. It is much larger than a local cache needs. |
| gluesql, polodb_core, persy, nebari | various | Not read. | Not run. |

These crates do not change the earlier finding. The structural points are the
ones that decided the first round: an index or a range table, the open cost,
and the lock model. None of the crates above is a Rust-native store with
multi-process readers that I could verify [H].
