# Probe results shared by tickets 28, 29, 32

Machine: local arm64 macOS, release build, redb 4.5.0 (cargo registry
version; the repo pins 4.3.0, costs should not differ). Probe source lives
at /tmp/traces-probe (throwaway). Method: 2 warmup passes, then medians of
9 timed runs (store writes: medians of 5 cold runs on fresh files). Tag [V].

## Sort order (ticket 32)

- Full sort of 50k path strings: `Path::cmp` 34.7 ms; raw byte sort
  (`OsStr::as_encoded_bytes`) 4.8 ms. A 7x gap. The release-check figures
  (1.7 ms `Path` vs 0.12 ms `OsStr`) repeat the shape on a smaller set.
  The check is cheap either way; a full sort in `Path` space is not.
- Byte order diverges from `Path` order: `notes/note.md` sorts before
  `notes-x.md` in `Path` order [V], and after it in raw byte order [V].
- The folder range convention survives byte space: for range
  `[area-004/, area-0040)` (last byte `/` 0x2F incremented to `0` 0x30),
  `partition_point` over sorted strings includes the folder children and
  excludes the sibling `area-004-x.md` [V]. One wrong convention does not:
  an end bound of `prefix + \x01` excludes every child, because `/` (0x2F)
  sorts after 0x01 [V].

## Overlay costs (ticket 28)

- `Arc` generation handout over 50k rows: below 0.1 µs (median ran under
  the timer floor). The snapshot is free; only the small overlay costs.
- Overlay build of 10 buffered entries (`HashMap<String, u32>`): 1.0 µs,
  one-time per query.
- Overlay miss-check across a 50k-row result (each row probes the map
  before falling back to the generation): 0.69 ms per full pass. The
  cost is per-row `String` materialization, not the map probe. A design
  that keys the overlay by path bytes and checks without allocation would
  sit well under this. Compare: cold query by shape 117 to 127 ms at 50k
  (24-prep guide), so even the naive pass is about 0.6% of a query.

## Range-key store costs (tickets 24, 29)

Three redb tables: `files` (path bytes key, 96-byte value), one text
field table, and one numeric field table. The text key is tag 0 +
value `done` + NUL + path bytes. The numeric key is tag 1 + 8-byte
order-preserving f64 + NUL + path bytes. The f64 transform: set the
sign bit for positives, invert all bits for negatives; unsigned
big-endian byte order then equals numeric order. NaN has no row;
ticket 19 excludes it (Null, never matches).

| Measure | 5k | 50k |
| --- | --- | --- |
| inserts, 3 tables (median) | 18.3 ms | 153.1 ms |
| commit (median) | 9.6 ms | 59.6 ms |
| db size | 4.2 MB | 33.7 MB |
| folder range over one value: rows / time | 714 / 25.9 µs | 7,143 / 261.5 µs |
| numeric range `rating >= 4.5`: hits / time | 500 / 17.0 µs | 5,000 / 171.0 µs |
| clear one facet table (`retain(.., false)`) | 0.32 ms | 3.7 ms |
| db size after clear | unchanged | unchanged |

Verified in the same run [V]: rows of the folder range and the numeric
range come out in ascending path-byte order inside one value prefix.
The first numeric match decodes to exactly 4.5 [V].

Earlier single-run figures used two tables and no warmup: write+commit
9.1 ms @5k, 66.9 ms @50k, 2.1/16.8 MB. Those runs were
commit-dominated cold runs; the medians above replace them.

Lessons for the grill:

1. The three-table layout costs 33.7 MB at 50k in one commit [V]. A
   stage-per-commit build (sizes probe, medians of 3) costs the same
   total [V]. Measured stages: files 16.8 MB, text facet +16.8 MB,
   numeric facet +0 MB (its commit reuses freed pages) [V]. Plan on
   the total, not a per-table cost. The 24-prep figure (67 to 135 MB
   with three range tables) stays the worse case for large text
   values [V].
2. Deletes do not shrink the file: the freed pages stay in the freelist.
   The compact path (ticket 36 territory) is the only reclaim [V].
3. Facet rebuild by table clear is milliseconds, not a full rewrite [V].
4. Range-key prefix `tag + value + NUL + path` supports equality (exact
   key), folder scans (prefix range), and numeric comparisons (ordered
   f64 bytes) in one table layout; path order falls out of the key [V].
5. Numeric encoding is 8 bytes plus tag and NUL; no arithmetic at scan
   time, one transform at write time. The ticket 24 open question
   "exact range-key encoding + costs" now has a measured candidate [V].

## Normalization and candidate costs (tickets 32, 28)

Corpus: 50,000 paths, 90% ASCII, 5% already-NFC accents, 5% NFD
(decomposed) accents (norm probe, medians of 9).

| Measure | 50k pass | Per path |
| --- | --- | --- |
| NFC check only (ASCII fast path, verify the rest) | 1,288.7 µs | 26 ns |
| NFC full normalize | 16,423.0 µs | 328 ns |
| Lowercase fold | 1,625.2 µs | 33 ns |
| Candidate list materialize (50k) | 95.0 µs | — |
| Candidate materialize + 10 overlay inserts | 159.2 µs | — |

Lessons for the grill:

1. The check-then-normalize-when-needed shape is affordable at every
   reconcile site: 1.3 ms per 50k pass, 8 times cheaper than a
   normalize-everything pass [V].
2. Write-time NFC normalize adds about 16 ms to a 50k full build,
   under 1% of the 1,801 ms build measured in 24-prep [V].
3. The fold-for-links duty costs 33 ns per path; a full 50k fold pass
   costs 1.6 ms [V].
4. The overlay-aware candidate step costs about 64 µs for 10 overlay
   paths over a 50k candidate list (159.2 minus 95.0 µs), under 0.1%
   of a 117 to 127 ms cold query [V].
