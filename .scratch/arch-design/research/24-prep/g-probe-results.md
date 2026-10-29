# Probe results: query shapes, store locking and refresh cost

This file records measurements for the evidence gate of ticket 24. All numbers
are [V] for the corpus and machine named below. They are not a vault benchmark.
The 50,000-note rows are measured, not extrapolated, but the corpus is
synthetic.

## Summary

- A cold query that has a `Tag` or `Path` source decodes only the matching
  notes. A cold query with no source, a `Not` source or a filter only decodes
  every note. At 50,000 notes the first group takes 117 ms to 127 ms and 35 MB
  to 52 MB. The second group takes 245 ms to 259 ms and 389 MB to 393 MB.
- The ratio between the two groups grows with the vault. At 20,000 notes the
  ratio of peak memory is 9 to 1 (18 MB against 163 MB). Research 11 measured
  2.2 to 1 (17 MB against 39 MB) on lighter notes.
- A second process that opens a held `index.redb` fails at once. The real CLI
  exits with code 1 and the message "Database already open. Cannot acquire
  lock."
- Adding one file to a 50,000-note vault makes the next query take 499 ms and
  395 MB, even for a rare-tag query. The path-set change decodes every note.
- redb has an experimental multi-process mode. A second process then reads while
  a first process holds the store for writing. The mode refuses
  `Durability::None`.

## Method

- Machine: Apple M4 Pro, 12 physical cores, 24 GB, macOS (arm64). This is the
  machine of research 11.
- Code: `main` at `39e96041`. `src/` is equal to `16f67ef0`. Build: `cargo build
  --profile bench --bin traces` with `CARGO_TARGET_DIR` outside the repository.
  The profile has `opt-level = 3` and thin LTO.
- Timing: `/usr/bin/time -l` around the real `traces` binary. Wall time includes
  the process start, the file system scan, the freshness check, the query and
  the output. Peak memory is `maximum resident set size`. The value is the
  median of 5 runs at 1,000 and 5,000 notes. It is the median of 3 runs at
  20,000 and 50,000 notes.
- Index: one `traces index` run before the queries. No file changed between the
  runs of one row, unless the row says so.
- Corpus: a synthetic vault. Each note has about 2,100 B. It has 5 frontmatter
  keys, three headings, nine list items, nine wikilinks and about 0.75 tasks.
  The tags are `common` (all notes), `t0` to `t19` (5% each) and `rare` (5
  notes). The notes sit in 100 folders. A real vault can differ. The
  documentation corpus of ticket 23 has a mean of 11,012 B for each note.
- The generator was a throwaway script. It is not in the repository.

## Cold query by source shape

All runs have an index with no change. Each cell shows wall time in ms and peak
memory in MB.

| Source shape | 1,000 | 5,000 | 20,000 | 50,000 |
|--------------|-------|-------|--------|--------|
| `#rare` (5 notes) | 22 / 6.9 | 31 / 9.4 | 60 / 18.0 | 117 / 35.1 |
| `#t3` (5% of notes) | 24 / 7.4 | 32 / 11.4 | 60 / 25.1 | 124 / 52.5 |
| folder `f3/` (1% of notes) | 24 / 7.8 | 32 / 10.3 | 63 / 20.0 | 124 / 39.1 |
| glob path (no match) | not run | not run | 64 / 18.7 | 127 / 36.4 |
| `#common` (all notes) | 27 / 14.8 | 45 / 46.4 | 108 / 163.2 | 246 / 392.6 |
| no source | 26 / 14.7 | 44 / 45.8 | 106 / 164.9 | 247 / 392.7 |
| `!#rare` (`Not`) | 26 / 14.8 | 43 / 46.1 | 105 / 163.0 | 247 / 390.4 |
| filter only (`status == "open"`) | 27 / 14.7 | 43 / 45.9 | 107 / 162.2 | 245 / 390.6 |
| `traces task`, no source | 26 / 14.7 | 43 / 45.8 | 112 / 162.9 | 259 / 389.1 |

Other numbers from the same runs:

- `traces index` (build from nothing): 102 ms and 18 MB at 1,000 notes. 222 ms
  and 56 MB at 5,000. 797 ms and 196 MB at 20,000. 1,801 ms and 467 MB at
  50,000.
- Size of `index.redb`: 2.1 MB, 8.4 MB, 33.7 MB and 67.4 MB.

Reading of the table [H]:

- The part that does not depend on the shape is the floor. At 50,000 notes it is
  about 117 ms. It holds the file system scan, the store open and the path list.
  A shape that decodes all notes adds about 130 ms and 357 MB.
- Each decoded note takes about 7 KB of memory at 50,000 notes (357 MB divided
  by 50,000). The source note has about 2.1 KB. The ratio is larger than the 2
  times of ticket 23, because these notes have many small list items and links.
  Check the ratio on a real vault.
- A `Path` glob uses the path list of the whole store. It decodes only the
  matching notes. A glob with no match costs the same as a rare tag.
- A filter that reads only frontmatter still decodes every note today. The
  record of ticket 23 has frontmatter at 0.5% of the bytes in the documentation
  corpus. A facet split can avoid most of this decode [H].

## Refresh after a file change

Query `#rare`. Each cell shows wall time in ms and peak memory in MB.

| Step | 5,000 | 20,000 | 50,000 |
|------|-------|--------|--------|
| After one edit (same path set) | 49 / 13.0 | 117 / 32.9 | 166 / 65.1 |
| Same query again | 32 / 9.5 | 61 / 18.1 | 135 / 35.1 |
| After one added file (path set changes) | 79 / 45.1 | 251 / 163.8 | 499 / 394.9 |
| Same query again | 31 / 9.5 | 61 / 18.1 | 120 / 35.3 |

- An added file makes the next query decode every note. The memory of a rare-tag
  query then equals the memory of a no-source query.
- This is the path-set change of research 11 (`reconcile` calls
  `read_all_notes`). A read model with a lazy store view does not help here. The
  refresh builds the whole `WorkspaceIndex` first. Ticket 25 owns the refresh
  policy. Ticket 24 owns the read model that the refresh builds.
- A user who adds a note with a template and then queries sees this cost. Both
  steps run in one process in the template engine path.

## A second process and the store

Probe code: a throwaway crate outside the repository, with `redb = "=4.3.0"`.
Process A opens the database and sleeps. Process B opens the same file.

| B call | Result for B | Time |
|--------|--------------|------|
| `Database::create(path)` | `DatabaseError::DatabaseAlreadyOpen` | 171 µs |
| `Database::open(path)` | `DatabaseAlreadyOpen` | not timed |
| `Database::builder().open_read_only(path)` | `DatabaseAlreadyOpen` | not timed |
| the same three calls after A exits | success | create: 4.4 ms |

- B fails at once. It does not wait for the lock.
- The real binary behaves the same. A process held `.traces/index.redb`. Then
  `traces list` printed "failed to index ... Database already open. Cannot
  acquire lock." and exited with code 1. After the holder exited, the same
  command exited with code 0.
- `IndexStore::create_db` has no case for `DatabaseAlreadyOpen`. It becomes
  `StoreError::Redb` (`src/index/store.rs:894-921`). `refresh_store` returns the
  error. `refresh` (template path) logs it and returns the in-memory index.
- `ReadOnlyDatabase` does not help while a writer holds the file. The redb docs
  say: "Multiple processes may open a [`ReadOnlyDatabase`], but it may not be
  opened concurrently with a [`Database`]."

Experimental multi-process mode (crate feature `experimental-multiprocess`,
`ConcurrencyMode::MultiWriter`), checked with a second throwaway crate:

| Step | Result |
|------|--------|
| A holds the store and commits once a second | A commit takes 8 ms to 11 ms (debug build) |
| B opens read only while A holds the store | success in 330 µs |
| B reads while A commits | B sees the newest commit of A |
| B opens a second writer on the same file | success; one commit worked |

- The redb changelog says that, in this mode, commits are always 2-phase and
  `Durability::None` is refused (`CHANGELOG.md` of redb 4.3.0, entry for the
  `experimental-multiprocess` feature). `IndexStore` sets `Durability::None` in
  its write transactions (`src/index/store.rs`, see research 11, section 9). The
  mode is behind an `experimental-` feature flag [H: the API can change].

## What a facet split can save

The figures come from ticket 23 (`research/23-prep/g-probe-results.md`) on the
documentation corpus [V]. They are not from the synthetic vault.

| Part of the stored `Note` | Share of encoded bytes |
|---------------------------|------------------------|
| list items | 77.7% |
| outlinks | 14.1% |
| headings | 5.6% |
| frontmatter | 0.5% |
| tags | 0.03% |
| path, inline fields, framing | 2.1% |

- A facet read of the meta part only (frontmatter, tags, inline fields) decodes
  about 3% of the bytes of a note on this corpus [H].
- A task query needs the items and the heading ranges. It does not need the
  outlinks. The outlinks are 14% of the bytes.
- The inbound link graph needs the outlinks of every note. It does not need the
  items.

## redb cache size

A throwaway copy of the store code set the redb cache size from an environment
variable. The default is 1 GiB. The table shows peak memory in MB at 50,000
notes (wall time in ms).

| Query | Default | 8 MB | 1 MB | 0 |
|-------|---------|------|------|---|
| rare tag | 35.5 (115) | 35.1 (112) | 31.4 (111) | 29.9 (113) |
| no source | 391.6 (224) | 326.3 (217) | 318.7 (271) | 307.7 (696) |

- A small cache saves at most about 17% on a full decode. Decoded notes use
  about 310 MB. No redb setting changes this.
- A cache of 0 makes the full decode 3 times slower.
- An 8 MB cache costs no time here and saves about 65 MB on the worst shape [H].
- Order keys: redb has no filter or sort engine. It gives key order, range
  reads and multimap lookups. Sort by a field in memory after the facet scan is
  not measured.

## Real facet decode at 50,000 notes

A throwaway probe parsed the 50,000 synthetic notes with the real parser. It
stored each note in five redb tables: the whole note, and four facets (meta,
headings, outlinks, items). Each facet is a postcard record keyed by path. The
meta facet holds the frontmatter, the inline fields and the tags. Each run opens
the store, scans one table in key order and decodes every record. Times are
the median of three runs on an Apple M4 Pro with 8 performance cores. Peak
memory is the process maximum resident set.

| Facet | Bytes stored | Sequential scan and decode | Parallel decode | Peak memory (sequential) |
|-------|--------------|----------------------------|-----------------|--------------------------|
| Whole note | 46.3 MB | 140-146 ms | not run | 99 MB (331 MB if all notes stay) |
| Meta | 4.2 MB (85 B each) | 51 ms (7.4 ms is I/O) | 25 ms | 14 MB |
| Headings | 2.5 MB | 14 ms | not run | 11 MB |
| Outlinks | 11.1 MB | 38 ms | 21 ms | 28 MB |
| Items | 27.6 MB | 51 ms | 38 ms | 64 MB (183 MB if all stay) |
| Items and the heading facet of each note that holds a task | not stored | 78 ms | not run | 70 MB |

- The meta decode takes about 7 times more than its I/O. The frontmatter
  parses dates from text. The earlier stand-in probe measured 21 ms. The real
  facet takes 51 ms.
- A sequential meta scan misses the 35 ms gate. A parallel decode meets it. The
  parallel run copies the raw bytes first (4 MB) and decodes them with rayon.
  The first parallel run in a fresh process was slower (170-250 ms) because the
  file was not in the OS cache.
- Holding all decoded notes costs 3 times more memory than a streaming scan:
  331 MB against 99 MB for whole notes. Items alone cost 183 MB against 64 MB.
- A whole-note stream cuts the peak memory from 391 MB (the current path) to
  99 MB. Its scan and decode take 140 ms. The 245-259 ms of the current path
  measures the whole process, so the two times are not equal measures.
- A `task` query that scans every item takes 51 ms to 78 ms. It does not need
  the other facets for notes that hold no task.
- The probe measured a scan and decode only. It did not build rows or sort.
- Cost when a query needs all four facets of every note. A lockstep scan of the
  four tables takes 151-154 ms and 108 MB. A whole-note scan takes 141-146 ms
  and 99 MB. A lookup by path in three tables for each note takes 233 ms. If
  every decoded note stays in memory, the lockstep scan takes 180-185 ms and
  338 MB.
- Tags and fields apart. A tags table (merged body and frontmatter tags, 1.2 MB)
  scans and decodes in 17-25 ms and 8 MB. A fields table (frontmatter and
  inline fields, 3.6 MB) takes 65-83 ms and 13 MB in the same runs. The parallel
  decode takes 14 ms and 32-43 ms. The machine was busy during these runs, so a
  meta scan that took 51 ms earlier took 73-111 ms. Compare only within one
  run set.
- Multimap against a postcard list for tags. A multimap from path to tag scans
  in 13-14 ms. A postcard list of tags scans in 11-13 ms. The machine was quiet
  in this run, so the tags scan is faster than the 17-25 ms above. The
  multimap has no scan gain. Its gain is the reverse direction (tag to paths).
- One row for each list item. A table keyed by path, a zero byte and the item
  index (450,000 rows) scans in 74 ms. A multimap from path to the index and
  the item bytes scans in 72 ms. One postcard list for each note scans in 56-58
  ms. A row layout costs about 30% more to scan. In exchange, it allows a range
  read of one note or of one item, and a later secondary index on an item field.
  The outlinks row test counted with a debug format, so its 190 ms is not a
  valid result.
- The probe vault is synthetic. Each note has four scalar frontmatter fields,
  three sections and about nine list items. It has no nested frontmatter, no
  body inline fields except on tasks, and no large note. The probe measured the
  table scan and decode only. It ran no `QueryService` query end to end.
- The probe measured today's `Note` shape. Ticket 23 decides a smaller shape
  (no status name in the item) that the code does not have yet. The item sizes
  are therefore an upper bound [H].

## Layouts that fit the maintainer's templates

The maintainer's templates in `docs/vault/` select a folder. The folder is
`"80_insight"`, `"60_library"`, an `OR` of folders, or `-"00_system/..."`. They
filter on frontmatter fields (`file_class`, `subtype`, `type` and dates). They
sort, and they print names or aliases. They read `file.tasks` with `FLATTEN`,
completion dates and a sort. They read `file.inlinks` and `file.outlinks`. They
rarely read `file.lists`. No template reads a full note.

Same 50,000-note probe. Each layout writes the facets with one record for each
note. Median of three runs.

| Layout | Write (encode, insert, commit) | Scan all notes |
|--------|-------------------------------|----------------|
| One whole record | 300-330 ms | 145 ms |
| Facets: fields, tags, headings, outlinks, one lists record | 520-555 ms | lists 54-55 ms |
| Same, with `tasks` apart from the other list items | 550-587 ms | tasks 12 ms; other items 48-50 ms |

- The `tasks` table holds only the tasks of notes that have one, each with its
  item index. A task query reads 12 ms instead of 54 ms.
- Range reads by folder. All facet tables are keyed by path, so a folder source
  is a key range. One folder of 500 notes took 3-10 ms for `tasks` and for
  `fields`, including the open of the store.
- The file size was the same for all three layouts in this run (134,746,112
  bytes), so it is not a valid size comparison.

## Split of the list records and a two-stage read

The templates in `docs/vault/` mostly query in two stages. A source (a folder, a
tag or an exclusion) and note-level filters (`file_class`, `type`, a date
range) pick the notes. Then a `FLATTEN file.tasks` or `FLATTEN file.lists`
reads the items of those notes only. A sort on a field comes last. A few task
queries start with `FROM #task` and filter on `T.completed`.

The probe stored each note's list items in three ways (50,000 notes, 487,500
items, 37,500 tasks, median of three quiet runs):

| Layout | Bytes | Scan and decode |
|--------|-------|-----------------|
| One record for each note with the whole items (as today) | 27.6 MB | 58-60 ms |
| Core record for each note: kind, depth, line, parent, order flag, item fields, item tags | 4.1 MB | 28-30 ms |
| Text record for each note: raw text and clean text | 23.6 MB | 37 ms |

- The text is 85% of the item bytes. A task filter that reads the core record
  only takes half the time.
- Two-stage read: scan `tags`, pick 2,500 notes (5%), then read the core record
  of each picked note by path. The whole read takes 15-16 ms. The same read with
  whole item records takes 16-17 ms. The tags scan alone takes about 12 ms, so
  the point reads of the picked notes cost 3-5 ms.
- The core and text layouts write two records for each note. A row-per-item
  layout writes one row for each item (about 10 for each note).

## Staged filter pipelines (end to end, no row building)

The probe ran four pipelines on the 50,000-note vault. The times include the
open of the store and a sort of the surviving rows. Median of three quiet
runs. The first run of each set was slower (15-185 ms) because the file was not
in the OS cache. The probe read five tables. `files` has path keys. `fields`
and `tags` hold those facets. `tasks_p` holds the tasks of each note. `classes`
holds the `status` value, as a stand-in for the class field.

| Pipeline | Stages | Rows | Time |
|----------|--------|------|------|
| Folder `f3/` (500 notes), `status` is done, `created` from a date, sort by rating | key range on `files`, point reads of `fields` | 112 | 4.4 ms |
| Tag `t3`, not folder `f5/`, rating at least 3, then tasks of survivors, sort | scan `tags`, point reads of `fields` and `tasks_p` | 2,500 | 18.4 ms |
| No source, `status` is done and rating at least 3, sort | scan `fields` | 6,667 | 48-49 ms |
| No source, class is done (thin `classes` scan), rating at least 3, sort | scan `classes`, point reads of `fields` | 6,667 | 33 ms |

- A source that prunes the notes first (a folder or a tag) keeps the whole
  query under 20 ms at 50,000 notes.
- A field filter with no source scans every `fields` record. It takes about 48
  ms on one thread.
- The thin `classes` scan, with point reads for the 16,667 notes in class done,
  took 33 ms. A thin class table helps this shape.
- The probe made the class from `status`. The vault has no schema, so this is
  not a real class source [H].
- The probe built no rows and ran no `QueryService` code.

## Store comparison on the same pipelines

The same 50,000 notes went into seven stores and an in-memory snapshot. The
candidates and the layout for each store are in `i1-store-candidates-and-designs.md`.
Every store loaded the same facet bytes from one dump of the real parser output.
Every store returned the same rows: 112, 2,500 and 6,667. Each run was a new
process. The time includes the open of the store, the query and a sort. The first
run was discarded. The figure is the median of seven runs. The machine was busy
(load average 15 to 28), so compare the stores within one set only. Versions:
redb 4.3.0, rusqlite 0.41.1 (bundled SQLite), turso 0.9.0-pre.2, heed 0.23.1,
fjall 3.1.13, duckdb 1.20000.1.

Pipelines: p1 is a folder range (500 notes) with a filter on `status` and
`created`. p2 is a tag with an excluded folder, a filter on `rating` and the
tasks of the survivors. p3 is no source and a filter on `status` and `rating`.
Times are in ms.

| Store and layout | p1 | p2 | p3 | Build | Replace one note | File (MB) |
|------------------|----|----|----|-------|------------------|-----------|
| In-memory snapshot (load all, rayon) | 62 | 62 | 67 | none | none | none |
| redb, one table for each facet (scan) | 14 | 26 | 53 | 984 | 18 | 135 (67 without the two range tables) |
| redb with range tables for tags and for status plus rating | not run | 18 | 14 | 984 | 18 | 135 |
| heed (LMDB), one table for each facet (scan) | 1.0 | 10.6 | 36.5 | 323 | 5.5 | 78 |
| heed with range tables | not run | 5.1 | 1.1 | 323 | 5.5 | 78 |
| fjall (LSM), one table for each facet (scan) | 4.7 | 24.5 | 48.9 | 2,344 | 6.9 | 129 |
| fjall with range tables | not run | 11.8 | 5.2 | 2,344 | 6.9 | 129 |
| rusqlite, narrow tables with indexes | 1.5 | 3.7 | 12.1 | 738 | 3.0 | 79 |
| turso, the same narrow tables with indexes | 29.3 | 31.0 | 21.8 | 3,577 | 8.8 | 89 |
| DuckDB, one wide columnar table, no index | 16.9 | 16.8 | 17.8 | 340 | 17.7 | 13 |

Peak memory of one query run: snapshot 145 MB, DuckDB 36 MB, turso 27 MB, the
other stores 11 to 19 MB.

What a second process sees. A holder process kept the store open (a write
transaction for SQLite and turso). A second process then read and wrote.

| Store | Reader beside the holder | Writer beside the holder |
|-------|--------------------------|--------------------------|
| redb | fails: `DatabaseAlreadyOpen` | fails |
| heed (LMDB) | works (the holder held the environment open only) | works |
| fjall | fails: `Locked` | fails |
| rusqlite (WAL) | works | fails with `DatabaseBusy` while the holder writes |
| turso | fails: file locked | fails |

- An index makes the biggest difference. A filter with no source takes 12 ms
  with SQLite indexes and 1.1 ms with a range table in LMDB. The scan layouts
  take 36 to 53 ms. The two scan stores that open fast (heed, fjall) stay near
  36 to 49 ms for p3.
- The redb open costs about 13 ms on this file. Every redb query pays it. LMDB
  opens in about 1 ms.
- SQLite in WAL mode serves readers beside a writer. redb, fjall and turso do
  not. The second-process problem of topic B has no cost in SQLite and in LMDB.
- Turso was the slowest to build (3.6 s) and had a fixed cost of 20 to 30 ms
  for each run. It locks the file even for a reader. It is a pre-release.
- DuckDB scanned in 17 ms for every shape and stored the data in 13 MB. A
  single-note replace took 18 ms. Its build took 6.5 minutes to compile.
- fjall needed a flush of the memtables after the bulk load. Without the flush,
  every open replayed the journal and took 700 ms.
- A SQL index prunes only. Values that are lists or objects, `contains` on a
  list, regular expressions and dates parsed from text still need the residual
  filter on the decoded `fields` record. The probe did not include this
  residual step for the SQL stores. For the key-value stores p2 and p3 included
  it. Add about 5 to 15 ms for the decode of the survivors [H].
- The SQL layout indexed only the scalar fields `status`, `rating` and
  `created`. A general field filter needs one row in `field` for each scalar
  value of each note. That increases the write cost and the file size [H].
- The probe is synthetic. It does not use a real vault, a cold disk cache, or
  Linux or Windows locking.

## Rust-native stores: surrealkv, turbokv and sled

The maintainer wants Rust-native crates only. This removes rusqlite (C SQLite),
heed (C LMDB) and DuckDB (C++). The probe then ran three more stores:
surrealkv 1.1.1, turbokv 0.6.0 and sled 1.0.0-alpha.124. They used the same
pipelines, the same dump and the same range tables as redb and fjall. Median of
five runs, in ms. The machine
was busy, so compare the rows of this table only.

| Store | p1 | p2 | p3 | p2 with range table | p3 with range table | Build | Replace one note | File (MB) |
|-------|----|----|----|---------------------|---------------------|-------|------------------|-----------|
| redb | 13 | 26 | 52 | 17 | 14 | 995 | 19 | 135 |
| fjall | 4.8 | 25.5 | 53 | 11.6 | **5.2** | 2,448 | **9.0** | 129 |
| surrealkv | 20 | 35 | 58 | 25 | 19 | 1,012 | 50 | 63 |
| turbokv | 97 | 117 | 138 | 106 | 97 | 530 | 120 | 86 |
| sled | 141 | 156 | 176 | 139 | 143 | 754 | 161 | 4.3 |
| turso (SQL) | 29 | 31 | 22 | n/a | n/a | 3,531 | 8.7 | 89 |

- All three new stores returned the same rows as the other stores.
- surrealkv has no fixed open cost as large as turbokv. It opens in about 17
  ms. It ranks below redb and fjall in this probe. A replace takes 50 ms.
- turbokv has a fixed open cost of 95 ms with the `durable` preset. The `fast`
  preset lowers it to 55 ms. The open cost makes every cold query slower than
  redb. The probe did not tune its options beyond this.
- sled has a fixed open cost of about 130 ms. Its file is 4.3 MB. I did not
  check why it is so small.
- Every new store locks its directory. A second process failed to open it for a
  read and for a write: surrealkv (`LOCK` file), turbokv (`DirectoryLocked`)
  and sled (`WouldBlock`).
- surrealkv and turbokv use one keyspace, so the probe stored each table as a
  key prefix. The write path used chunks of 2,000 notes. surrealkv committed
  with `Durability::Immediate`. turbokv used the `durable` preset.
- The probe used the default options of each crate. A tuned run can change the
  open cost of turbokv and sled [H].

## Check of the surrealkv README claim

The surrealkv README reports large gains over redb. Its benchmark uses 500,000
keys, 48 threads and 128 clients on a 32-core server. Its figures are
operations for each second of a long-running process. The probe above runs one
short process with one reader. A second probe opened each store once and
timed repeated queries. It also timed the get of the `fields` record of every
note on one thread. surrealkv used one shared read-only transaction.

| Store | Point gets of 50,000 records, one thread | p2 warm (ms) | p3 warm (ms) | p2 with range table, warm (ms) | p3 with range table, warm (ms) | Open (ms) |
|-------|------------------------------------------|--------------|--------------|--------------------------------|--------------------------------|-----------|
| redb | 2.58 M ops/s | 11.0 | 37.4 | 4.3 | 0.79 | 3 to 8 |
| fjall | 1.09 M ops/s | 19.6 | 43.1 | 8.4 | 1.44 | 3 to 9 |
| surrealkv | 0.90 M ops/s | 18.6 | 37.6 | 8.3 | 0.90 | 13 to 23 |

- The probe did not reproduce a surrealkv advantage. With the open cost
  removed, redb was equal or faster on every pipeline.
- The README gives redb 779,000 point reads for each second on 48 threads. One
  redb thread in this probe did 2.58 million. The README figure is therefore
  limited by its harness, not by redb. The README ranking is not comparable
  with this probe.
- The README measures concurrent throughput. The cold CLI and the first host
  consumers need low latency for one reader. A host with many parallel readers
  may see a different result. The probe did not test that [H].
- Run-to-run noise was large. Several runs of the same query showed a first
  call of 12 ms and of 72 ms for redb on a busy machine. Compare only the
  medians.

## Review of the surrealkv benchmark and a scan test

Sources: the surrealkv README and the adapters of `surrealdb/crud-bench`
(`redb.rs`, `fjall.rs`, `surrealkv.rs`, read on 2026-10-29).

Raw scans on the probe data (warm, no decode, median of repeated runs):

| Store | All 50,000 `fields` values (3.6 MB) | All 50,000 keys | 100 folder ranges (50,000 keys) |
|-------|-------------------------------------|-----------------|----------------------------------|
| redb | 1.9 ms | 3.3 ms | 3.2 ms |
| surrealkv | 4.2 ms | 4.3 ms | 4.6 ms |
| fjall | 7.3 ms | 7.9 ms | 8.2 ms |

- The scan test supports the README direction for surrealkv against fjall.
  surrealkv is 1.7 to 1.9 times faster than fjall. The README gives 6.75 times
  for scans of the first 100 keys.
- redb is the fastest of the three on raw scans in this probe.

What the crud-bench adapters do:

- The redb adapter declares its table as `TableDefinition<&[u8], Vec<u8>>`. A
  `Vec<u8>` value type makes redb allocate and copy every value that it reads.
  A separate test on 50,000 records of 71 bytes measured a full scan at 6.6 ms
  with `Vec<u8>` and 1.4 ms with `&[u8]`. A scan of the first 100 keys took
  13.4 us with `Vec<u8>` and 2.9 us with `&[u8]`. The README redb scan and
  point-read figures are therefore about 4 to 5 times too low for redb [H].
- The redb adapter runs each operation on a blocking thread pool
  (`affinitypool::spawn_local`). The surrealkv adapter awaits in place. The
  thread hop adds a fixed cost to each short scan.
- The fjall adapter uses `OptimisticTxDatabase`. This is a transactional
  wrapper with conflict tracking. It also sets a 64 KiB block size and
  value separation at 4 KiB. The probe above used the plain `Database`.
- Every adapter opens a new transaction and iterator for each scan, and the
  headline scan is `select * limit 100`. This measures the cost of setup for a
  scan, not the throughput of a long scan.
- The write figures compare different durability. redb uses `Durability::None`,
  surrealkv uses `Durability::Eventual` and fjall uses `PersistMode::Buffer`.
  The synchronous table has no figure for fjall, redb or LMDB. The README says
  the run was too slow to finish.
- The machine is a 32-core server with 128 GB of memory and 48 to 128
  concurrent clients. The probe uses one reader on a laptop.

Conclusion: the README ranking is real for its workload. Part of the redb gap
comes from the adapter value type. The probe data does not show a surrealkv
advantage over redb on scans.

## Not measured

- A real personal vault.
- The decode time of a `Class` source facet.
- The compact binary temporal encoding of ticket 18.
- The boxed and the unboxed task payload (ticket 23).
- A `Class` source (the vault has no schema).
- The cost of two processes in the multi-process mode with a writer in each.
- Windows and Linux file locking. The probe ran on macOS only.
