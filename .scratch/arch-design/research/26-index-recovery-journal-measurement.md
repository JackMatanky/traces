# Ticket 26 measurement: index recovery journal cost screen

Status: measured. Machine: Apple M4 Pro, macOS 27.2, APFS, Rust nightly-2026-10-01, redb 4.3.0, repo `main` at `79f393a2`. Probe: throwaway crate at `/tmp/t26-probe`, drives the real engine through `traces::testing`. No `src/` file changed.

## Summary

- [V] Journal writes and removals cost 8 to 76 ms per command at these sizes. The probe measured these costs on the real engine.
- [H] The rebuild saved-work bound is 1,333 ms, but it is loose. A replay still pays the persist and must derive facets from facts. The real saved work is lower. Ticket 29 must measure it.
- [V] The in-store journal table costs more than the external file at every measured size. At 50,000 notes it adds 477 ms over its own baseline apply.
- [H] For incremental edits the journal pays above a 4 to 7 percent stop rate. The bound uses the refresh delta as saved work. The real saved work is lower. Ticket 29 must measure it.
- [H] The screen did not reproduce the 85 percent write and 2 times size claims for a facet layout. The probe emulated a round-robin split, not the decided facet layout. The claim stays untested.
- [V] `Durability::None` commits die on abort; `Immediate` commits survive. This check sampled one stop point per mode on one store copy. `Immediate` cost 4.6 ms more per command in this sample.
- [V] The sampled rebuild kills left an empty or a full store, never partial. The probe sampled 9 stop points of 15. The next open took 17 to 37 ms.

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

The probe replayed prebuilt rows, not a real replay. Treat these numbers as lower bounds. The steps were:

1. Create a fresh store.
2. Replay prebuilt facts rows in one `Immediate` commit.
3. Read one raw row, then drop.

|Size|Journal path|Rival warm open plus read|
|---|---|---|
|50|13.96 ms|3.03 ms|
|5,000|37.65 ms|3.19 ms|
|50,000|145.38 ms|3.03 ms|

- [V] The rival after a stop is not a warm open. It is a full rescan: 1,718 ms at 50,000 notes.
- [H] The journal path must also derive links, range keys and notices from facts, and it must persist. These costs are missing here. A full-query comparison at matched stop points belongs to ticket 29.

## R5: facet layout claim

- [V] The emulation split `notes` rows round-robin across 8 tables with the same bytes. This is not the decided facet layout of ticket 24.
- [V] It wrote each layout in one `Immediate` transaction into fresh files.
- [V] The write times matched at every size.
- [V] At 50,000 notes the split layout took 828.9 ms against 820.8 ms.
- [H] The claims of 85 percent more write time and 2 times size stay untested. This emulation does not test the decided layout.

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

`SIGKILL` before the k-th sync call, k = 1, 2, 3, 4, 5, 6, 8, 12, 15 of 15. Next process reopens 3 times.

|k|Store state|Notes|First open|
|---|---|---|---|
|1|Invalid header|none|reject|
|2 to 12|Empty schema|0|17 to 37 ms|
|15, the last sync|Full|50,000|33.31 ms|

- [V] The store file stayed at 45,056 bytes until the durable commit landed. No partial large file appeared in this sample.
- [V] Every sampled stop left an empty or a full store, never partial. The next open after an empty store cost at most 37 ms.
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

The saved-work bounds are upper bounds. Each one omits replay persist and facet derivation, so the real saved work is lower. The gate rejects a candidate only when even the upper bound never exceeds the added cost.

|Scenario|Rival B|External O|In-store added O|Provisional break-even, external|
|---|---|---|---|---|
|Rebuild, 50,000|1,332.9 ms|76 ms|477 ms|above 5.7 percent|
|Edit 100, 50,000|125 ms|8.0 ms|4.9 ms|above 6.4 percent|
|Edit 5,000, 50,000|898 ms|7.9 ms|not measured|above 0.9 percent|
|No change, 50,000|0 ms|8.0 ms|4.9 ms|never pays|

- [V] No candidate rejects at the upper bound. The bounds are loose, so this is not proof that a journal pays.
- [V] No-change commands save nothing. They never pass at any stop rate.
- [H] The provisional rates assume the loose bounds. A cold CLI stops rarely, and its stop may not land in the window. Ticket 29 must measure the real saved work before anyone uses these rates.
- [P] Keep the external file with the facts payload. Drop the in-store journal table. Decide adoption with ticket 29's replay measurements.

## Limits

- [V] Journal encode here copies real row bytes. A real journal encodes parsed facts, so these costs are lower bounds. [H] The postcard proxy would add tens of percent, not orders of magnitude.
- [V] All numbers are macOS and APFS. Linux, Windows, and network volumes are untested.
- [V] Notes are synthetic. The fixture makes 1 task per note against the specified 0.75, so task-heavy paths are under-tested.
- [V] `F_FULLFSYNC` counts prove call order, not disk durability. No real power-loss test ran.
- [V] The durability check sampled one stop point per mode. The kill sweep sampled 9 of 15 stop points. Both cover a narrow band.
- [V] The recovery replay wrote prebuilt rows. It did not re-derive links, tags, or notices from facts. The follow-on task must measure that derivation.
