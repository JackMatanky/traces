# Ticket 27: replay prototype and fault evidence

Status: investigation complete. This is not an adoption result.
Ticket 27 is resolved. Ticket 36 weighs adoption.

## Method and artifact

The throwaway replay crate lives on the scratch worktree branch
`scratch/t27-probe-worktree`, at
`.scratch/arch-design/research/27-probe/replay/` under
`.worktrees/t27-probe/`. The layout probe and its older results are in
[the layout probe](./27-probe/layout/). Production code changes are
limited to that scratch branch, gated behind the `test-utils` feature.
The changes expose `IndexStore` and the index database path constant.
They also add two gated test helpers on `IndexStore`, a gated
`QueryService` pass-through, and a gated `WorkspaceIndex` constructor
from facts. No production behavior changed.

The replay probe journals and replays the **real** `.traces/index.redb`
layout: the eight production redb tables, postcard rows, and real
persistence through `WorkspaceIndex::for_probe_facts` and
`IndexerService::persist`. Verification compares full `QueryRow`
accessors against a fresh-parse store through
`run_from_store_for_probe`. Decided-identity fields are incarnation,
revision, epoch, policy, and extractor fingerprint. The probe models
them in the journal header and a sidecar file. The production store does
not carry them yet. The notices facet is out of scope. The parser is
infallible and the codebase has no notices data. This gate stays open
for tickets 29 and 34.

It exercises publication, source checks, replay, and process stops. The
layout crate derives eleven named facets from parsed fixture notes. Run
commands and known limits are in [the replay
instructions](./27-probe/replay/README.md) and the layout probe's stderr
output.

Host: Apple M4 Pro, macOS 27.2, APFS, arm64. Source commit `51af072a`.
Nightly Rust 1.101.0 (2026-09-30). Release builds use `opt-level = "z"`,
fat LTO, one codegen unit, and `panic = "abort"`. No `RUSTFLAGS` were
set. The Linux runs used a Podman Fedora aarch64 VM with 4 GiB RAM. They
are not native-Linux latency evidence. These tests model process stops,
not arbitrary power-loss reorderings.

## Correctness results

The macOS release-profile suite passed 42 of 42 tests with redb 4.3.0
(about 10 seconds). A Linux run passed 25 of 25 with pinned redb 4.3.0
(2.12 seconds) after the VM memory rose to 4 GiB. The re-targeted
42-test suite has not yet run there. The suite covers, beyond the
original 25 cases:

- The fork hole. With the base check weakened to incarnation plus
  revision only (`PROBE_WEAK_BASE=1`), a journal from a divergent store
  with the same revision is wrongly accepted, and verification fails.
  This is direct evidence that the ticket-25 contract needs the digest
  backstop.
- Every decided-identity part mutates and is handled: policy drift,
  incarnation, revision, epoch up (discard and reparse), epoch down
  (preserve and serve a fresh query), extractor fingerprint.
- Read-only vault and a true read-only redb handle never open or apply
  the journal.
- A Unix `cp -R` discards and reparses. A `mv` replays. An edit racing
  the final source check fails safe.
- `SIGKILL` and `SIGINT` at a hold failpoint behave like the simulated
  stops. Repair timing and `check_integrity` output are printed without
  time assertions.
- A hostile but checksum-valid body is rejected by a length guard
  without a large allocation.
- The mid-rebuild stop rolls back inside a single redb transaction. This
  is a tautology before incremental rebuilds exist and proves nothing
  about a multi-transaction rebuild.

Fact hashing canonicalizes postcard rows through ordered JSON before
blake3, because postcard map bytes are not canonical. Recovery checks
the target digest after applying changes, not before.

Real `ENOSPC` was exercised in a container with a 64 MiB `tmpfs`. The
result changes the decided error model. redb 4.3.0 mmaps the store. With
`Durability::None`, an `ENOSPC` commit reports success. The file stays
sparse, the new rows are silently lost, and queries serve the stale
state. With a hard file-size cap, the process dies by `SIGXFSZ` with no
catchable error.

- The journal-induced failure surface matches the no-journal surface.
  There is no new failure class.
- Both surfaces are silent loss. The decided reopen-and-classify path
  has no `ENOSPC` error to classify on this stack.
- This silent loss is itself an adoption blocker for ticket 36 to weigh.

## Matched measurements (macOS, release profile)

Protocol: 20 runs per policy case, A-B-A interleaving, no-op controls
(drift clean). Placement and scenario cases take 5 runs. Execution is
serial. The warmup run is discarded. Report medians and ranges.
Fixture: 5,000 and 50,000 notes, one-note edit as the comparison work.

### Journal policy comparison (work = one-note edit)

| Case | n | Median ms | Range ms | Journal B | Notes |
|---|---:|---:|---|---:|---|
| no-journal `Immediate`, 5k | 20 | 480.4 | 454.9–578.2 | 0 | parse 398.3, commit+durability 82.3 |
| journaled `Immediate`, 5k | 20 | 1,106.0 | 1,036.9–1,226.6 | 324,452 | commit+durability 489.7, sync 12.6 |
| journaled deferred `None`, 5k | 20 | 628.4 | 565.7–712.8 | 324,452 | commit+durability 0.0, sync 8.2 |
| no-journal `Immediate`, 50k | 20 | 5,462.5 | 5,258.1–5,929.6 | 0 | parse 4,804.3, commit+durability 655.2 |
| journaled `Immediate`, 50k | 20 | 12,205.6 | 12,002.6–13,312.3 | 3,289,454 | commit+durability 5,530.3, sync 13.8 |
| journaled deferred `None`, 50k | 20 | 6,716.7 | 6,549.6–6,850.1 | 3,289,454 | commit+durability 0.0, sync 10.4 |

Key readings:

- Journal encode (postcard plus blake3) is about 0.3 ms at both sizes.
  This sits far under the 53.3 ms ticket-26 write budget. Encoding is
  not a cost concern.
- The journaled `Immediate` surcharge is not encode. It is journal-file
  sync, store-file fsync, and directory fsync: about 490 ms at 5k and
  5.5 s at 50k.
- The deferred journal pays almost nothing in the write path but still
  parses the current state up front. Parse dominates everywhere: about
  400 ms at 5k and 4.8 to 6.7 s at 50k.
- No policy dominates. The no-journal `Immediate` path wins the write
  path at both sizes. A retained journal wins the read-only no-change
  refresh at 50k (4,315.3 ms versus 6,329.0 ms without). The retained
  journal shifts cost from reads to the next write.
- Store sizes at 50k: no-journal 76,754,944 B, journaled `Immediate`
  51,236,864 B, deferred 33,689,600 B. The no-journal store is bigger
  because a full rebuild writes all rows. Peak RSS at 50k is about
  462 MB with a journal and 281 to 339 MB without.

### Source-check placement (5 runs)

| Placement | n | Total median ms | Total range ms | Writer-hold median ms |
|---|---:|---:|---|---:|
| A: snapshot before hold, 5k | 5 | 207.0 | 170.0–243.0 | 33.2 |
| B: snapshot inside hold, 5k | 5 | 149.3 | 145.9–197.7 | 134.1 |
| A, 50k | 5 | 2,134.7 | 2,117.0–2,159.9 | 199.8 |
| B, 50k | 5 | 1,939.8 | 1,921.2–1,965.7 | 1,920.7 |

Placement B cuts total wall time by 9 to 14 percent by overlapping parse
with the store digest. The writer hold grows to nearly the whole parse.
Placement A keeps the hold small at a 9 to 14 percent total-time cost.
This is a trade-off, not a winner.

### Snapshot hash: stable versus all-racy (one run each)

Stable and all-racy differ by less than 4 ms at both sizes. The 5k pair
is 159.1 versus 132.6 ms. The 50k pair is 1,953.0 versus 1,949.0 ms.
Fixture mtime changes add no measurable hash cost.

### Recovery cost and scenarios (5 runs each, medians)

At 5k: clean replay 424.3 ms versus parse-only fallback 781.0 ms, so the
saved-work bound is 356.7 ms, about 46 percent of the fallback. The
first usable query takes 11.5 ms and a repeated query 6.9 ms. Scenario
totals: no-change 480.5, one-note edit 471.0, hundred-note edit 442.3,
add/delete/rename with inbound links 475.4, policy-change rebuild 474.4
(journal 1.54 MB).

At 50k, scenario totals use 5 runs each. The table gives median, range,
and journal bytes:

| Scenario | Median ms | Range ms | Journal B |
|---|---:|---|---:|
| no-change | 4,266.5 | 4,249.1–4,359.1 | 3,289,265 |
| one-note edit | 4,349.1 | 4,251.2–4,382.0 | 3,289,360 |
| hundred-note edit | 4,284.2 | 4,277.2–4,300.4 | 3,298,825 |
| links add/delete/rename | 4,275.5 | 4,267.7–4,296.0 | 3,289,559 |
| policy-change rebuild | 4,333.4 | 4,325.9–4,355.0 | 15,821,983 |

The parse-only fallback at 50k takes a median of 8,061.0 ms. The
saved-work bound is therefore 3,796.7 ms, about 47 percent of the
fallback. Read-only no-change access takes 6,329.0 ms without a journal
and 4,315.3 ms with one retained. At 50k the first usable query takes
about 63.0 ms and a repeated query about 54.5 ms. Decode plus replay
takes about 3.2 s. That is roughly 60 times the 53.3 ms write budget.
Replay itself does not fit the budget at this size. The harness peak
RSS at 50k is about 488 MB.

## Layout observations (superseded for sizing)

The layout-probe numbers below predate the re-target. They are one
serial run per anchor. They are **not** medians, A-B-A measurements, or
a matched durability comparison. The shadow omits timestamps, most range
keys, notices, and full query rows:

| Notes | Tasks | Control store (`None`) | Shadow store (`Immediate`) | Control persist | Shadow persist | Process peak RSS |
|---:|---:|---:|---:|---:|---:|---:|
| 50 | 37 | 1,056,768 B | 1,056,768 B | 44.160 ms | 6.027 ms | 6,373,376 B |
| 5,000 | 3,750 | 8,425,472 B | 16,846,848 B | 199.748 ms | 285.830 ms | 78,446,592 B |
| 50,000 | 37,500 | 67,375,104 B | 134,746,112 B | 1,054.827 ms | 2,865.841 ms | 712,179,712 B |

The matched measurement tables land here when the running harness
finishes.

## Evidence not established

The prototype has not proved a Linux run of the full 42-test suite. It
has not proved the 85-percent and 2x cost claims, whole-blob versus
streaming payloads, or incremental rebuild savings. Those gates need the
real implementation owned by ticket 16 and decided by ticket 36.
Torn-write ordering needs a block-level injector and was not tested.
The matched measurements now exist. Any numeric host freshness bound
goes to ticket 33 with this data. Ticket 36 must not adopt the journal
from this evidence alone.
