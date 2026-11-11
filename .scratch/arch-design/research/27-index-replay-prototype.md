# Ticket 27: replay prototype and fault evidence

Status: investigation complete. This is not an adoption result.
Ticket 27 is resolved. Ticket 36 weighs adoption.

## Findings and limits

The real-layout replay probe passes 63 of 63 macOS cases with redb 4.3.0. The
Linux container run passes 62 of those cases (one ignored by design). These are
process-stop results, not proof against arbitrary power-loss reorderings.

The corrected matched protocol (native redb durability, 20 runs per case,
a-b-a-c-a order, equal-work placements) ran on Linux. The journal surcharge
sits inside the A-B-A noise band at 5k and 50k. Replay saves 89 ms, about
20 percent, against parse fallback at 5k. Under the colder second-session
protocol the 50k replay saving is not measurable. A later section records
the cache-state difference that explains the two sessions.

A dedicated `ENOSPC` probe shows real, scoped behavior. The writer gets a
catchable `StorageFull` error. Acknowledged `Durability::None` rows vanish on
reopen at the failure boundary. `Immediate` rows survive. This stays an
adoption blocker for ticket 36 to weigh.

Four distinctions control interpretation:

| Evidence | What it establishes | What it does not establish |
|---|---|---|
| Eight-table replay probe | Real production layout, persistence, and query comparison | The decided eleven-facet implementation or notices |
| Eleven-facet layout shadow | Historical layout observations from fixture facts | Complete timestamps, range keys, notices, or full query rows |
| Corrected policy timing cases | Native-durability costs at matched work on VM storage | Native-Linux or production-host latency, or an adoption result |
| Fault cases and saved images | Selected process-stop and damage behavior | Arbitrary torn writes, power-loss ordering, or a full completed plan matrix |
| `ENOSPC` probe | Real errno-level failure, row-survival split by durability, tmpfs and orderly exit | Power-loss ordering, other redb versions, or other filesystems |

Two original interpretations need correction, not removal:

- The source calls 53.3 ms a ticket-26 “write budget.” This historical
  ROW-COPY/ENCODE comparison is not a recovery latency limit.
- The macOS harness labeled the journaled-`Immediate` surcharge as sync cost
  from an external fsync surrogate. The corrected protocol measures native
  redb durability directly and finds no measurable surcharge.

The research and risk sections retain uncertainty where strict STE would change
the meaning. The [initial plan](./27-index-replay-prep.md) remains the
requirements checklist, not a completed coverage claim.

## Method and probe identities

### Real-layout replay probe and branch provenance

The throwaway replay crate lives in the main repository at
`.scratch/arch-design/research/27-probe/replay/`. The branch
`scratch/t27-probe-worktree` held the working copy during the runs and is
now removed. The probe instructions are in
[the replay probe](./27-probe/replay/README.md).

The layout probe and its older results are in
[the layout probe](./27-probe/layout/).

The probe needs the production `test-utils` seams. Those seams lived only on
the scratch branch. They are:

- The public `IndexStore` and the index database path constant.
- Two gated test helpers on `IndexStore`.
- A gated `QueryService` pass-through.
- A gated `WorkspaceIndex` constructor from facts.
- Four native-durability seams: `rebuild_for_probe`,
  `checkpoint_for_probe`, `database_for_probe`, `persist_into_for_probe`.

The main repository does not carry these seams. A future probe run needs
them again; ticket 35 decides the durable version. No production behavior
changed.

The replay probe journals and replays the real `.traces/index.redb` layout:

- The eight production redb tables and postcard rows.
- Real persistence through `WorkspaceIndex::for_probe_facts` and
  `IndexerService::persist`.
- Full `QueryRow` accessor comparisons against a fresh-parse store through
  `run_from_store_for_probe`.

The journal header and a sidecar model incarnation, revision, epoch, policy, and
extractor fingerprint. The production store does not carry those
decided-identity fields yet.

The notices facet is out of scope: the parser is infallible, and the codebase
has no notices data. This gate stays open for tickets 29 and 34.

Fact hashing canonicalizes postcard rows through ordered JSON before blake3
because postcard map bytes are not canonical. Recovery checks the target digest
after applying changes, not before.

### Incomplete eleven-facet layout shadow

The layout crate derives eleven named facets from parsed fixture notes. It is
not the real-layout replay crate. Its shadow omits timestamps, most range keys,
notices, and full query rows. The historical layout table below therefore cannot
size the complete decided implementation.

### Environment and measurement protocol

| Item | Recorded setup |
|---|---|
| Host | Apple M4 Pro, macOS 27.2, APFS, arm64 |
| Source | Commit `51af072a`, worktree branch `scratch/t27-probe-worktree` |
| Compiler | Nightly Rust 1.101.0 (2026-09-30) |
| Release profile | `opt-level = "z"`, fat LTO, one codegen unit, `panic = "abort"` |
| Compiler flags | No `RUSTFLAGS` set |
| Corrected protocol host | Podman rust:1.96-slim container (aarch64), pinned redb 4.3.0, nightly-2026-10-01; VM storage, not native-Linux latency evidence |
| Execution | Serial; fresh child process per sample |
| Policy cases | 20 runs per block, order a-b-a-c-a (a = 60 runs); native `Durability::Immediate` commits |
| Other cases | 20 runs per case |
| Reported statistics | Medians and ranges |
| Fixtures | 5,000 and 50,000 notes; one-note edit as the policy comparison work |

The `Immediate` labels below retain the source's case names. The external
store-file fsync and directory fsync surrogate does not prove native redb
`Durability::Immediate` behavior.

The deferred path reports write-path cost without proving that later checkpoint
and cleanup costs are fully charged. Do not treat these measurements as the
complete matched protocol from the initial plan.

The reported no-op controls are “drift clean”; that label is not proof of a
cold-cache protocol.

## Fault evidence

### Completed test coverage

| Run | Result | Duration | Limit |
|---|---|---|---|
| macOS, release profile, redb 4.3.0 | 63 of 63 passed | 9.4 seconds | Process-stop evidence |
| Linux container, re-targeted suite, pinned redb 4.3.0 | 62 passed, 1 ignored by design | 8.1 seconds | `SIGKILL`/`SIGINT` cases need `T27_SIGNAL_HELPER` when the image ships no `kill` binary |

The suite extends the original 25 cases with these checks:

| Case | Reported evidence | Interpretation or limit |
|---|---|---|
| Same-revision fork | `PROBE_WEAK_BASE=1` weakens the base check to incarnation plus revision. The probe wrongly accepts a divergent journal, and verification fails. | Direct evidence for a digest backstop, not an adopted ticket-25 identity contract |
| Decided identity | Policy drift, incarnation, revision, epoch up, epoch down, and extractor fingerprint mutate | Epoch up discards and reparses; epoch down preserves and serves a fresh query |
| Pure read-only access | A read-only vault and a true read-only redb handle never open or apply the journal | Distinct from the writable refresh measurement below |
| Root and source changes | Unix `cp -R` discards and reparses; `mv` replays; an edit racing the final source check fails safe | Root pathname alone does not decide replay |
| Process stops | `SIGKILL` and `SIGINT` at a hold failpoint behave like simulated stops | Repair timing and `check_integrity` output have no time assertions |
| Hostile valid checksum | A length guard rejects the body without a large allocation | Does not establish all malicious-input cases |
| Mid-rebuild stop | A single redb transaction rolls back | Tautological before incremental rebuilds; proves nothing about a multi-transaction rebuild |

### ENOSPC and file-size failure observations

A dedicated `enospc-probe` binary tested real `ENOSPC` on a 64 MiB `tmpfs`
(container). It used native `Durability::None` and `Immediate` commits with a
1 MiB cache and 1 MiB rows. The receipt is an external synchronized file. The
pristine-copy checker opens read-only first.

The failing commit gives the writer a real catchable error
(`redb::Error::Io` errno 28, `StorageFull`). The loss follows that error. Under
`Durability::None`, every row acknowledged before the failing commit was
absent after reopen (none-0: `acknowledged=31 actual=0`; none-16: `23` rows
the same). Under native `Immediate`, all earlier acknowledged rows survived on
the same filesystem (immediate-0: 31 of 31; immediate-16: 23 of 23). An
orderly close with no failure kept all 128 `None` rows on a roomy control
filesystem. The loss therefore needs the failure boundary, not a `None` commit
alone. A retained journal shifts only the exhaustion threshold (31 to 23 rows
at 16 MiB).

A post-`ENOSPC` store keeps a recovery flag, so `open_read_only` reports
`RepairAborted` even with free space. The separately labelled writable reopen
is the only working scan; `check_integrity` stayed clean in all cases.
A hard file-size cap instead kills the process with `SIGXFSZ`.

| Case | Writer exit | Acknowledged rows | Survived on reopen |
|---|---|---|---|
| none-0 | 3, errno 28 | 31 | No (0 of 31) |
| none-16 | 3, errno 28 | 23 | No (0 of 23) |
| immediate-0 | 3, errno 28 | 31 | Yes (31 of 31) |
| immediate-16 | 3, errno 28 | 23 | Yes (23 of 23) |
| control (none, roomy fs) | 0 | 128 | Yes (128 of 128) |

The journal-induced and no-journal surfaces matched: both lose acknowledged
`None` rows at the failure boundary. The decided reopen-and-classify path has
an error to report, but it cannot restore the lost acknowledged rows.

Keep these observations within the tested stack: redb 4.3.0, tmpfs, orderly
exit at the failure boundary. This is not a power-loss test and not a general
redb guarantee.

## Corrected protocol measurements (Linux container, release profile, native durability)

### Journal policy comparison, 20 runs per block, order a-b-a-c-a

Work is the one-note edit. Policy a is the no-journal `Immediate` write.
Policy b is the journaled `Immediate` write. Policy c is the journaled
deferred `None` write with a later native checkpoint charge.

| Policy | 5k median ms | 5k range ms | 50k median ms | 50k range ms | Journal |
|---|---:|---|---:|---|---:|
| a, no-journal `Immediate` | 663.7 (a blocks 671.9/658.1/659.8) | 637.1–790.1 | 6,686.0 | 6,475.4–8,115.3 | 0 B |
| b, journaled `Immediate` | 667.3 | 642.2–713.4 | 6,705.5 | 6,579.1–8,305.6 | 317 KB / 3.3 MB |
| c, journaled `None` + checkpoint | 662.9 | 643.5–689.5 | 6,620.8 | 6,531.4–6,984.8 | 317 KB / 3.3 MB |

At 5k the a-block drift is about 2 percent. Policy b adds +3.6 ms and policy c
subtracts 0.8 ms against the a median. At 50k, policy b adds +19.5 ms
(0.3 percent) and policy c subtracts 65.2 ms (1 percent). The a-block range is
wide, so both deltas sit inside the noise band. The journal surcharge on a
cold full publish is not measurable with this protocol. No policy ranking is
proved.

Phase medians at 50k: parse about 1,080 ms, validation about 4.45 s,
native commit 618 to 669 ms, checkpoint about 26 ms (c only). Peak RSS is
319 to 338 MB at 50k.

### Recovery: replay versus parse fallback (20 runs each)

| Scenario | 5k replay ms | 5k fallback ms | 50k replay ms | 50k fallback ms | 50k saved |
|---|---:|---:|---:|---:|---:|
| no-change | 300.4 | 328.2 | 2,926.6 | 3,187.2 | 8.2 percent |
| one-note edit | 305.2 | 330.1 | 2,945.3 | 3,222.6 | 8.6 percent |
| hundred-note edit | 299.0 | 331.4 | 2,938.7 | 3,291.3 | 10.7 percent |
| links add/delete/rename | 301.9 | 330.5 | 2,941.3 | 3,218.9 | 8.6 percent |
| policy rebuild | 563.6 | 594.3 | 5,689.5 | 5,879.6 | 3.2 percent |

Replay verifies against a fresh parse and full assembled entries at every run.
Every recovered outcome is `replayed`, and every fallback outcome is `parsed`.
A source change during replay discards the journal and parses. The
class-change fallback case shows this in 20 of 20 runs.

This table ran on warmer pages than the second session. The second-session
section supersedes it for recovery claims.

### Source-check placement with equal work (20 runs each)

| Placement | 50k total median ms | 50k hold median ms |
|---|---:|---:|
| A: check before hold | 551.2 | 217.8 |
| B: check inside hold | 560.9 | 558.3 |

The totals match within noise. Placement B holds the writer lock 2.56 times
longer at 50k (217.8 to 558.3 ms) and 2.2 times longer at 5k (27.4 to
60.2 ms). Equal work removes the earlier unequal-comparison caveat.

### Snapshot, read-only and query costs (20 runs each)

| Case | 5k median ms | 50k median ms |
|---|---:|---:|
| Snapshot, stable fixture | 305.6 | 2,997.9 |
| Snapshot, all-racy fixture | 302.3 | 3,002.6 |
| Read-only no-change digest, no journal | 21.6 | 216.4 |
| Read-only no-change digest, retained journal | 22.0 | 215.6 |
| First usable query | 8.7 to 10.2 | 90.2 to 101.9 |
| Repeated query | 7.8 to 8.6 | about 80 |

Stable versus all-racy differs by under 5 ms at both sizes with 20 runs each.
The retained journal costs nothing on a pure read-only no-change pass.

The macOS tables below keep their record as the earlier-generation harness
output. The Linux tables above supersede them for the corrected protocol. The
Linux tables use native redb durability, a true a-b-a-c-a order, 20 runs
everywhere, equal-work placements, and real stable/racy snapshots. The
second-session section later in this file supersedes the first-session
recovery table and its phase medians.

## Anchor, stop-window and second-session runs

A second corrected session ran on a fresh Linux container. Each sample now
carries its own best-effort cache hint. In this session the 50k parse phase
runs 3.8 to 4.3 s. The first session's parse phase ran about 1.1 s, so that
session served warmer pages.

Do not compare the two sessions' totals. The first session's recovery table
shows warm-cache behavior. This section shows the colder protocol. It
supersedes the first session for recovery claims.

### 50-note anchor (full case list, 20 runs per case)

| Case | Median ms | Range ms | Note |
|---|---:|---|---|
| Policy a, no-journal `Immediate` | 10.4 | 9.6–25.7 | n = 60 |
| Policy b, journaled `Immediate` | 12.1 | 10.5–23.9 | journal 3,701 B |
| Policy c, journaled `None` + checkpoint | 12.4 | 10.2–18.9 | |
| Recovery, replay, one-note edit | 6.10 | 5.77–8.21 | |
| Recovery, parse fallback, one-note edit | 7.13 | | |

At 50 notes every recovery is a few milliseconds. Parse does not dominate.
The journal surcharge sits inside the noise band. Replay saves about 1 ms.

### Second-session policy phases (20 runs per block, a-b-a-c-a)

| Phase, median ms | 5k | 50k |
|---|---:|---:|
| Base store scan | 50.5 | 470.2 |
| Parse sources | 124.3 | 3,752.1 |
| Validate derived facts | 480.0 | 5,292.4 |
| Journal encode | 1.4 | 15.3 |
| Publish write plus file sync | 9.9 | 12.4 |
| Publish directory sync | 0.5 | 1.3 |
| Native commit | 79.5 to 89.8 | 718 to 773 |
| Checkpoint (c only) | 5.2 | 33.6 |
| Journal removal plus directory sync | 2.6 | 22.7 |

The 5k a-block drift is about 3 percent. The 50k a-block drift is about
12 percent and trends down. Total medians are 757.2 (a), 770.1 (b), and
786.3 ms (c) at 5k. At 50k the
totals are 10,349.5 (a), 10,824.1 (b), and 10,065.1 ms (c). Policy deltas sit
inside this drift, so no ranking is proved. The
measured per-phase journal overhead is about 14 ms at 5k and 52 ms at 50k.

### Stop-window recovery (20 runs each)

| Stop class | 5k outcome | 5k median ms | 50k outcome | 50k median ms |
|---|---|---:|---|---:|
| Before the journal exists | parsed | 437.6 | parsed | 5,988.1 |
| After journal publish, before commit | replayed | 348.4 | replayed | 6,163.2 |
| After commit, journal retained | unchanged | 435.5 | unchanged | 7,333.7 |

The 5k ranges are 385.9–562.6, 299.5–462.6, and 407.0–509.7 ms. The 50k
ranges are 4,680.7–7,304.6, 4,480.8–6,951.0, and 6,728.8–8,052.6 ms. At 5k
replay saves 89 ms against parse. At 50k the replay median is 175 ms slower
than parse and the ranges overlap. The 50k replay saving is not measurable
under this protocol. The post-commit stop pays a source compare, not a
persist, and its cost matches the ordinary refresh cost.

### Per-phase break-even

The break-even test never combines phases. Each phase carries its own
overhead, and each phase realizes its own saving:

- Encode plus publish phases at 5k carry 11.9 ms of overhead. A stop there
  saves 89.2 ms. The break-even point is one stop per 7.5 writes in the
  12 percent replayable window.
- The removal phase charges 2.6 ms at 5k and saves nothing. This cost never
  breaks even.
- The pre-publish phases charge no overhead and save nothing.
- At 50k the encode plus publish phases carry 29.1 ms of overhead. The
  measured saving is zero, and the point estimate favors parse by 175 ms.
  The break-even point is not reached.
- The replayable window spans encode start to commit end: 91 ms of a 770 ms
  write at 5k, and 802 ms of a 10,824 ms write at 50k.

### Proposed host staleness bound

The freshness target stays: a command must not serve an index older than the
last successful publish. The proposed bound is: after an interrupted write,
the next command must produce a queryable store within one completed publish
cycle of the same host. Measured recovery plus first query: 361 ms with
replay and 450 ms with parse at 5k, against a 770 ms publish cycle. Measured:
6.2 to 6.3 s at 50k against a 10.8 s publish cycle. The bound holds at both
sizes on VM storage. Ticket 33 must re-measure on real hosts.

First usable query after recovery runs 12.1 to 12.9 ms at 5k and 121 to
128 ms at 50k.

## Reported measurements (macOS, release profile)

### Journal policy comparison (work = one-note edit)

| Case | n | Median ms | Range ms | Journal B | Notes |
|---|---:|---:|---|---:|---|
| no-journal `Immediate`, 5k | 20 | 480.4 | 454.9–578.2 | 0 | parse 398.3, commit+durability 82.3 |
| journaled `Immediate`, 5k | 20 | 1,106.0 | 1,036.9–1,226.6 | 324,452 | commit+durability 489.7, sync 12.6 |
| journaled deferred `None`, 5k | 20 | 628.4 | 565.7–712.8 | 324,452 | commit+durability 0.0, sync 8.2 |
| no-journal `Immediate`, 50k | 20 | 5,462.5 | 5,258.1–5,929.6 | 0 | parse 4,804.3, commit+durability 655.2 |
| journaled `Immediate`, 50k | 20 | 12,205.6 | 12,002.6–13,312.3 | 3,289,454 | commit+durability 5,530.3, sync 13.8 |
| journaled deferred `None`, 50k | 20 | 6,716.7 | 6,549.6–6,850.1 | 3,289,454 | commit+durability 0.0, sync 10.4 |

| Other reported quantity | Value | Interpretation limit |
|---|---|---|
| Journal encode, postcard plus blake3 | About 0.3 ms at both sizes | Below historical 53.3 ms ROW-COPY/ENCODE comparison; not a recovery budget |
| Journaled `Immediate` sync-related surcharge | About 490 ms at 5k; 5.5 s at 50k | Source attributes it to journal-file sync, store-file fsync, and directory fsync, not encode |
| Parse time across paths | About 400 ms at 5k; 4.8 to 6.7 s at 50k | Deferred path still parses current state up front |
| 50k no-journal store | 76,754,944 B | Full rebuild writes all rows |
| 50k journaled `Immediate` store | 51,236,864 B | Different work prevents a universal sizing comparison |
| 50k deferred store | 33,689,600 B | Different work prevents a universal sizing comparison |
| 50k peak RSS | About 462 MB with a journal; 281 to 339 MB without | Harness and path-specific observations |

The source concludes that encoding is not a cost concern and that no policy
dominates. It reports the no-journal `Immediate` path as the write-path winner
at both sizes. It describes deferred cost as almost nothing in the write path,
despite the larger reported total.

These are reported interpretations, not a complete cost-benefit proof. The
source also says a retained journal shifts cost from reads to the next write.
The timing table does not remove the initial plan's checkpoint-and-cleanup
accounting requirement.

### Source-check placement (5 runs)

| Placement | n | Total median ms | Total range ms | Writer-hold median ms |
|---|---:|---:|---|---:|
| A: snapshot before hold, 5k | 5 | 207.0 | 170.0–243.0 | 33.2 |
| B: snapshot inside hold, 5k | 5 | 149.3 | 145.9–197.7 | 134.1 |
| A, 50k | 5 | 2,134.7 | 2,117.0–2,159.9 | 199.8 |
| B, 50k | 5 | 1,939.8 | 1,921.2–1,965.7 | 1,920.7 |

The original interpretation says B cuts wall time by 9 to 14 percent by
overlapping parse with the store digest. It says A keeps the hold small at a 9
to 14 percent total-time cost.

The 5k totals do not support that percentage range. The 5k ranges overlap, so
these measurements do not establish a performance ranking. Retain the
measurements rather than adopting the reported percentage range as established.

The hold under B grows to nearly the whole parse. This remains a total-time
versus writer-hold trade-off, not a winner.

### Snapshot hash: stable versus all-racy (one run each)

| Fixture | Stable ms | All-racy ms |
|---|---:|---:|
| 5k | 159.1 | 132.6 |
| 50k | 1,953.0 | 1,949.0 |

The original report claims a difference below 4 ms at both sizes and no
measurable cost from fixture mtime changes. The 5k pair contradicts the first
claim; the 50k pair differs by 4.0 ms at the reported precision. One run per
case does not establish the second claim or a timing distribution.

### Recovery cost and scenarios (5 runs each, medians)

| Recovery quantity | 5k | 50k |
|---|---:|---:|
| Clean replay, reported comparison | 424.3 ms | 4,264.3 ms |
| Parse-only fallback | 781.0 ms | 8,061.0 ms |
| Reported saved-work bound | 356.7 ms, about 46 percent | 3,796.7 ms, about 47 percent |
| First usable query | 11.5 ms | About 63.0 ms |
| Repeated query | 6.9 ms | About 54.5 ms |
| Decode plus replay | Not separately reported | About 3.2 s |
| Harness peak RSS | Not separately reported | About 488 MB |

Clean replay and no-change scenario totals have distinct reported values at both
sizes. Ticket 27 reports 4,264.3 ms for 50k clean replay. The scenario table
reports 4,266.5 ms for 50k no-change. The source does not establish that these
observations use the same timing boundary.

The saved-work figures are reported comparisons, not measured upper/lower
interval bounds for the ticket-26 rejection gate.

| Scenario | 5k median ms | 50k median ms | 50k range ms | 50k journal B |
|---|---:|---:|---|---:|
| no-change | 480.5 | 4,266.5 | 4,249.1–4,359.1 | 3,289,265 |
| one-note edit | 471.0 | 4,349.1 | 4,251.2–4,382.0 | 3,289,360 |
| hundred-note edit | 442.3 | 4,284.2 | 4,277.2–4,300.4 | 3,298,825 |
| links add/delete/rename, with inbound links | 475.4 | 4,275.5 | 4,267.7–4,296.0 | 3,289,559 |
| policy-change rebuild | 474.4 | 4,333.4 | 4,325.9–4,355.0 | 15,821,983 |

The reported 5k policy-change journal is 1.54 MB. The source gives no 5k
scenario ranges or other 5k scenario journal sizes.

| 50k access case, originally called read-only no-change | Time ms |
|---|---:|
| Without a journal | 6,329.0 |
| With a retained journal | 4,315.3 |

This metric invokes writable refresh/recovery; it is not a pure read-only handle
measurement. A pure read-only operation never opens or applies the journal. The
reported retained-journal advantage therefore belongs to writable no-change
refresh/recovery.

The original report calls 3.2 s roughly 60 times the 53.3 ms “write budget” and
concludes replay exceeds that budget. The ratio preserves the historical
comparison, but 53.3 ms is not a recovery budget. It does not establish a
recovery latency rejection threshold.

## Historical layout observations (superseded for sizing)

These observations predate the real-layout re-target. Each anchor has one serial
run, not medians, A-B-A measurements, or a matched durability comparison.

The incomplete shadow omits timestamps, most range keys, notices, and full query
rows.

| Notes | Tasks | Control store (`None`) | Shadow store (`Immediate`) | Control persist | Shadow persist | Process peak RSS |
|---:|---:|---:|---:|---:|---:|---:|
| 50 | 37 | 1,056,768 B | 1,056,768 B | 44.160 ms | 6.027 ms | 6,373,376 B |
| 5,000 | 3,750 | 8,425,472 B | 16,846,848 B | 199.748 ms | 285.830 ms | 78,446,592 B |
| 50,000 | 37,500 | 67,375,104 B | 134,746,112 B | 1,054.827 ms | 2,865.841 ms | 712,179,712 B |

## Open gates and decisions

Investigation completion does not close these requirements:

| Gate | Missing evidence or owner |
|---|---|
| Complete decided layout | Real eleven-facet implementation, including notices; ticket 16 implementation and ticket 36 decision |
| Notices behavior | Tickets 29 and 34 |
| Cost claims | The 85-percent and 2x claims remain unproved |
| Payload choice | Whole-blob versus streaming payloads; no completed streamed-frame comparison |
| Rebuild value | Incremental rebuild savings; single-transaction rollback is not that evidence |
| 50-note anchor | Outside the measured matrix |
| Crash model | Torn-write ordering needs a block-level injector; not tested |
| Host freshness | Ticket 33 needs an explicit freshness target before a numeric bound |
| Stop windows | Per-phase break-even rates and stop-window rates remain unmeasured |
| Adoption | Ticket 36 must not adopt the journal from this evidence alone |

The corrected protocol closed these gates:

- The full Linux re-targeted suite.
- The 20-run repetition rule with the A-B-A noise control.
- The equal-work placement comparison.
- Native `Immediate` durability evidence with the deferred checkpoint charge.
- The full `ENOSPC` reproduction with row-survival checks.

The measurement tables are present above; they do not close unmeasured rows of
the initial plan.
