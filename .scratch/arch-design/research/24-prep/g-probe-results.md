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

## Not measured

- A real personal vault.
- The decode time of each facet.
- The compact binary temporal encoding of ticket 18.
- The boxed and the unboxed task payload (ticket 23).
- A `Class` source (the vault has no schema).
- The cost of two processes in the multi-process mode with a writer in each.
- Windows and Linux file locking. The probe ran on macOS only.
