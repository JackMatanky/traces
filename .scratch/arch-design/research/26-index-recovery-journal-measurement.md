# Ticket 26 measurement: index recovery journal cost screen

Status: measured. Machine: Apple M4 Pro, macOS 27.2, APFS, Rust nightly-2026-10-01, redb 4.3.0, repo `main` at `79f393a2`. Probe: throwaway crate at `/tmp/t26-probe`, drives the real engine through `traces::testing`. No `src/` file changed.

## Summary

- [V] The external journal never rejects at the gate. For a 50,000-note rebuild it costs 76 ms (facts) against about 1,333 ms of saved work.
- [V] The in-store journal table adds 477 ms to every rebuild. It is dominated by the external file (76 ms) for the same protection.
- [V] For incremental edits the journal pays above a 4 to 7 percent stop rate. The stop must land inside a 125 ms window. A cold CLI sits far below that rate.
- [V] The 85 percent write and 2 times size claims for a facet layout did not reproduce. Measured ratio: 1.01 write, 1.000 bytes.
- [V] `Durability::None` commits die on abort; re-verified on the real store. `Immediate` adds 4.6 ms per command and makes the next open 20 times slower after an abrupt stop.
- [V] A kill anywhere before the last sync of a rebuild leaves an empty store. The next open takes 17 to 37 ms and no partial store ever appears.

## Method

1. Build synthetic vaults at 50, 5,000, and 50,000 notes.
2. Use the ticket fixture shape: nested paths, 4 classes, tags, 10 items per note.
3. Make every 13th item a task. Add one outbound link per note.
4. Harvest every row byte from the real store with the `rows` command. Rebuild each payload shape from those bytes.
5. Time each candidate cost with 7 repeats. Report medians. One benchmark at a time, release mode, no `RUSTFLAGS`.
6. Inject stops with `SIGKILL` at the k-th `F_FULLFSYNC` through the `dyld` shim. Reopen and count rows.
7. Compare recovered state against row counts 0 and n. Any other count fails.

Marks: [V] measured here. [H] interpretation. [P] proposal.

## R1: rebuild cost (the rival that a journal saves)

|Size|Scan plus parse|Persist|Total|
|---|---|---|---|
|50|3.4 ms|42.0 ms|45.4 ms|
|5,000|78.8 ms|118.5 ms|197.3 ms|
|50,000|726.9 ms|606.0 ms|1,332.9 ms|

- [V] The 50,000-note store holds 1,102,000 rows in 8 tables and measures 67,375,104 bytes. The `notes` table alone is 28.4 MB with a median row of 572 bytes.

## R2: external journal cost per command

Median of 7: encode, then write, `sync_all`, and parent-directory sync. Remove plus directory sync is separate.

|Payload|n=50|n=5,000|n=50,000|
|---|---|---|---|
|Facets bytes|71.9 KB|7.49 MB|76.6 MB|
|Facets O|4.69 ms|6.07 ms|36.09 ms|
|Facets encode|0.6 ms|18.0 ms|173.2 ms|
|Facts bytes|32.2 KB|3.40 MB|34.9 MB|
|Facts O|4.25 ms|4.90 ms|18.83 ms|
|Facts encode|0.2 ms|5.8 ms|53.3 ms|
|Edit100 bytes|28.7 KB|60.8 KB|62.5 KB|
|Edit100 O|4.08 ms|3.96 ms|4.01 ms|
|Remove plus dir sync|3.9 to 5.8 ms|3.9 to 5.8 ms|3.9 to 5.8 ms|

- [V] A journal file sync plus directory sync has a 4 ms floor on APFS. The cost grows to about 25 ms at 76 MB.

## R3: in-store journal table cost

Two `Immediate` transactions: the first records facts into a `journal` table, the second applies facet rows and clears it. Baseline is one `Immediate` transaction that applies the same rows.

|Mode|n=50|n=5,000|n=50,000|
|---|---|---|---|
|Facets journal|11.12 ms|378.64 ms|4,219.14 ms|
|Facets baseline|4.02 ms|91.94 ms|800.81 ms|
|Facts journal|3.71 ms|100.76 ms|603.25 ms|
|Facts baseline|2.98 ms|25.19 ms|126.17 ms|
|Edit100 journal|10.31 ms|10.31 ms|11.94 ms|
|Edit100 baseline|5.31 ms|5.98 ms|7.00 ms|

- [V] At 50,000 notes the rebuild journal adds 477 ms over its own baseline apply. The external file adds 76 ms total. Same protection, 6 times cheaper.
- [H] The second write transaction pays most of the cost because it rewrites pages the first transaction already touched.

## R4: recovery cost after a stop

First usable query after a stop, measured as:

1. Create a fresh store.
2. Replay the facts in one `Immediate` commit.
3. Run one read, then drop.

|Size|Journal path|Rival warm open plus read|
|---|---|---|
|50|13.96 ms|3.03 ms|
|5,000|37.65 ms|3.19 ms|
|50,000|145.38 ms|3.03 ms|

- [V] The rival after a stop is not a warm open. It is a full rescan: 1,718 ms at 50,000 notes. The journal recovery at 145 ms wins by about 1.57 s.

## R5: facet layout claim

- [V] The emulation split `notes` into 8 facet tables with the same bytes.
- [V] It wrote each layout in one `Immediate` transaction into fresh files.
- [V] The write times matched at every size.
- [V] At 50,000 notes the split layout took 828.9 ms against 820.8 ms.
- [V] The 85 percent write and 2 times size claims did not reproduce in this emulation.

## R6: commit durability on the real store

30 repeats of open, one row insert, commit, drop on a copy of the 50,000-note store.

|Order|Median commit cycle|
|---|---|
|`None`|13.60 ms|
|`Immediate`|18.16 ms|
|`None` again|14.04 ms|

- [V] `Immediate` adds about 4.6 ms per command on an open store. This matches research 11.
- [V] After `SIGABRT` with no close, a `None` commit is gone and an `Immediate` commit survives. Re-verified by row counts.

## R7: next open after an abrupt stop

Child aborts without close. Parent reopens the 50,000-note store.

|Child mode|First open|Warm open|
|---|---|---|
|Commit, close|16.84 ms|16.02 ms|
|`None` commit, no close|18.39 ms|18.41 ms|
|`Immediate` commit, no close|347.09 ms|14.84 ms|

- [V] An `Immediate` commit without close makes the next open 20 times slower at this store size. The repair replays its non-durable tail.
- [H] A cold CLI that closes cleanly never pays this. It pays it after a crash only if it used `Immediate`.

## R8: kill sweep during a 50,000-note rebuild

`SIGKILL` before the k-th sync call. Next process reopens 3 times.

|k|Store state|Notes|First open|
|---|---|---|---|
|1|Invalid header|none|reject|
|2 to 12|Empty schema|0|17 to 37 ms|
|15, the last sync|Full|50,000|33.31 ms|

- [V] The store file stays at 45,056 bytes until the durable commit lands. No partial 60 MB file ever appears.
- [V] The outcome is empty or full, never partial. The next open after an empty store costs at most 37 ms.
- [H] All syncs before the last one are redb internals. The engine gains nothing from syncing earlier in a rebuild.

## R9: incremental rival on the real engine

`refresh_with_report` on a copy of the 50,000-note vault with its store restored.

|Changed notes|Refresh time|
|---|---|
|0|384.7 ms|
|100|509.7 ms|
|5,000|1,282.6 ms|

- [V] The no-change refresh costs 385 ms. Open, repair check, and a 50,000-file scan dominate it. A journal cannot remove this cost.
- [V] The marginal cost of 100 changed notes is about 125 ms. This is the upper bound on saved work for that edit.

## G1: gate evaluation

Saved-work upper bound is the marginal rival cost. Added cost is the journal O per command. The gate rejects a candidate only when saved never exceeds added.

|Scenario|Rival B|External O|In-store added O|Break-even stop rate, external|
|---|---|---|---|---|
|Rebuild, 50,000|1,332.9 ms|76 ms|477 ms|5.7 percent|
|Edit 100, 50,000|125 ms|8.0 ms|4.9 ms|6.4 percent|
|Edit 5,000, 50,000|898 ms|7.9 ms|not measured|0.9 percent|
|No change, 50,000|0 ms|8.0 ms|4.9 ms|never pays|

- [V] No candidate rejects on rebuild: 76 ms against 1,333 ms leaves a wide margin.
- [V] No candidate rejects on large edits. The window is the 125 ms tail after journal publication.
- [H] A cold CLI stops rarely and its stop is unlikely to land in that tail. Expected saving per command sits below the 8 ms cost. Only a host that reloads often can cross the rate.
- [P] Keep the external file with the facts payload. Drop the in-store journal table. Decide the incremental rate question with real stop data in the follow-on task.

## Limits

- [V] Journal encode here copies real row bytes. A real journal encodes parsed facts, so these costs are lower bounds. [H] The postcard proxy would add tens of percent, not orders of magnitude.
- [V] All numbers are macOS and APFS. Linux, Windows, and network volumes are untested.
- [V] Notes are synthetic. Real vault notes vary in size and frontmatter shape.
- [V] `F_FULLFSYNC` counts prove call order, not disk durability. No real power-loss test ran.
- [V] The recovery replay wrote prebuilt rows. It did not re-derive links, tags, or notices from facts. The follow-on task must measure that derivation.
