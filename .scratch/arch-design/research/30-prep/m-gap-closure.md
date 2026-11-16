# Temporal gap-closure record (`m-gap-closure`)

This record lists what the gap-closure round measured, what it proved, and
what it did not measure. The parity probe, the candidate probes, and the
Moment counterexample now live in this repository, so a reader can rebuild and
rerun them. Paths below are relative to this directory.

## Scope and environment

All runs used one Apple Silicon arm64 host (macOS 27.2). Probe runs used the
pinned toolchain `nightly-2026-10-01` (rustc 1.101.0-nightly, 2026-09-30). One
candidate-stack build also ran on the stable 1.89.0 toolchain as a
compatibility cross-check. Probe profiles copy the repository release profile
(`opt-level = "z"`, strip, LTO, one codegen unit, `panic = "abort"`). The
parity and cost probes pin `jiff-core 0.1.1` and `jiff-tzdb 0.1.9` through
`cargo update --precise`, so transitive versions match the repository lockfile
at preparation time.

The probes under `probes/` are throwaway evidence for ticket 30. They are not
part of the product test suite and do not link repository code.

## Parser parity probe (`probes/parity`)

The probe compares the legacy speedate-based field parser against a candidate
Jiff parser. The guarded domain is byte-level: years `0000..9999`, months
`00..13`, and days `00..32`. The probe also mutates one byte at every position
of six seed strings and adds ten malformed or extended shapes. It checks
4,635,370 inputs in total.

Result, reproduced inside the repository during this round with the preserved
sources:

```text
checked=4635370 legacy_valid=3652624 acceptance_differences=0
value_differences=0 error_category_differences=0
raw_jiff "20260101"=Ok(2026-01-01)
raw_jiff "2026-01-01T00:00:00"=Ok(2026-01-01)
raw_jiff "2026-01-01Z"=Err(parsed value '2026-01-01', but unparsed input "Z" remains)
raw_jiff "0000-01-01"=Ok(0000-01-01)
raw_jiff "-0009-01-01"=Err(failed to parse six digit integer as year)
sizes Date=4 DateTime=12 Timestamp=16 Span=64 DateTimePayload=24 PeriodTuple=6
```

Reproduce with:

```sh
cargo +nightly-2026-10-01 run --release --locked
```

Limits. Parity holds only for the guarded ten-byte date shape with strict
ASCII digits and separators. Raw Jiff accepts a superset: compact `20260101`,
naive datetimes, and negative years. The probe does not cover datetimes,
spans, time zones, or any repository parser beyond the field one.

## Complete value-layout proof (`probes/parity/tests/layout.rs`)

Earlier evidence measured the 24-byte temporal payload only. This round adds
a build test that mirrors the complete proposed `NoteFieldValue`. The mirror
replaces the runtime `DateTimeValue` with
`DateTimePayload { Civil, Instant }`. It uses the real Jiff `civil::DateTime`
and `Timestamp` types. It reproduces the link and duration shapes from
`src/note/links.rs:28` and `src/temporal/duration.rs`.

Result, release profile, pinned toolchain:

| Variant | Size (bytes) |
|:--------|-------------:|
| Null, Bool | 0, 1 |
| Number | 8 |
| String | 24 |
| Date | 4 |
| DateTime (with payload) | 24 |
| Duration | 72 |
| Link | 56 |
| List | 16 |
| Object | 72 |
| complete enum | 80 |

The complete enum is exactly 80 bytes and stays inside the cap, because
`Duration` (72 bytes) binds the largest variant. Growing the temporal payload
from 16 to 24 bytes does not move any other variant. A second test parses a
value inside an allocation-counting window (`stats_alloc`) after a warm-up
parse and asserts zero allocations and zero deallocations in the window.

Limits. This proves the proposed layout, not the landed runtime layout: the
real `NoteFieldValue` still stores a transparent `Timestamp` wrapper today.
The link mirror approximates `LinkType` with one boolean byte; any accepted
enum for `LinkType` keeps one-byte storage, so the total stays 80 bytes.

## Candidate probes (`probes/candidates`)

One probe binary tests both natural-language candidates against one fixed
anchor (2026-03-14T12:00:00Z, a Saturday), with `--features english` for
`jiff-english 0.0.1` and `--features fuzzy` for `fuzzydate 0.4.1`. Each phrase
runs as its own process, so a panic cannot hide. A 15-phrase behavior matrix
and a 20,000-iteration indicative benchmark cover both candidates.

Behavior findings, all runtime-verified:

- Both candidates return points only. Neither produces periods or spans, and
  neither reports error spans.
- `jiff-english` rejects `next week` outright (`expected week day or month
  name`). `fuzzydate` accepts it and returns a +7-day point. Neither accepts
  `in 2 weeks`.
- `last friday` diverges: `jiff-english` returns the previous day
  (2026-03-13), `fuzzydate` returns the Friday one week back (2026-03-06).
- Bare weekdays land on the next strictly-after occurrence in both, with
  midnight in `jiff-english` and preserved anchor time in `fuzzydate`.
- Neither accepts `eom`. `jiff-english` lenient mode misparses
  `start of month` as midnight today; `fuzzydate` rejects it.
- Typos, full-width digits, and CJK input all fail cleanly in both; no panic
  was observed. The `week_day` slicing panic risk in `jiff-english` stays a
  source-level observation, not an executed proof.
- Both reject `3 days ago tomorrow` and `tomorrow xyz`.

Indicative cost, 20,000 in-process iterations with `black_box`, release
profile (not a controlled benchmark; single host, warm cache):

| Phrase | `jiff-english` | `fuzzydate` |
|:-------|---------------:|------------:|
| tomorrow | 263 ns/parse | 366 ns/parse |
| next monday | 588 ns/parse | 585 ns/parse |
| `next weke` (fail) | 383 ns/parse | 215 ns/parse |

Integration cost, release profile, same codegen flags as the baseline:

| Build | Binary size | Delta |
|:------|------------:|------:|
| baseline (Jiff set only) | 418,944 B | — |
| `--features english` | 485,488 B | +65.0 KiB |
| `--features fuzzy` | 469,008 B | +48.9 KiB |

Feature unification, measured with `cargo tree -e features -i jiff`:

- `english` pulls `jiff` default features on top of the repository's
  deliberate `default-features = false` set: `tz-fat`, `tzdb-zoneinfo`,
  `tzdb-concatenated`, `tzdb-bundle-platform`, and `perf-inline` join the
  unified build. The delta above includes this unification plus the parser.
- `fuzzy` leaves the Jiff tree untouched, because `fuzzydate` sits on
  `chrono 0.4.45`; the delta is the `chrono` stack plus the parser.

Compatibility and maintenance, checked 2026-11-17 through the crates.io API:

- Both candidates declare edition 2021 and no `rust-version`. Both build
  clean under stable rustc 1.89.0, well below the repository's 1.96 floor.
- `fuzzydate 0.4.1`: created 2022-02-17, last update 2026-04-05, 164,489
  downloads, repository `DevinVS/fuzzydate`.
- `jiff-english 0.0.1`: single release on 2026-08-07, 29 downloads,
  repository `Squirreljetpack/feeling`, a port of `chrono-english`.

Reproduce with:

```sh
cargo +nightly-2026-10-01 build --release --locked --features english
target/release/temporal-cost-probe "next monday"
target/release/temporal-cost-probe "next monday" --bench
```

## Moment counterexample (`probes/moment`)

`moment-proof.cjs` asserts results for Moment 2.30.1 with the `en` locale
(`dow=0`, `doy=6`). For 2025-12-28 it checks three token styles:
`YYYY-[W]ww` gives `2025-W01`, `gggg-[w]ww` gives `2026-w01`, and the ISO
token `GGGG-[W]WW` gives `2025-W52`. The script, the vendored `moment.cjs`,
and its SHA-256
(`5b3eef58af8f051a73350fb530ccd1826e6a21ea840a60d5a9b81acb6d3823a0`) are
tracked here, and the assertion set ran during this round.

## What this round did not measure

- Repository binary-size, compile-time, and startup deltas for any adoption.
  Probe sizes and warm-cache link times are not repository measurements, and
  no compile-time benchmark ran. The 6.3/6.7/6.0 second figures in the build
  log are final-crate link times with a warm target directory.
- Temporal-key index timing and memory on representative corpora. That needs
  the implemented temporal keys and stays an acceptance gate.
- Allocation profile of the real parse pipeline beyond the single layout
  window test.
- Exhaustive Unicode, overflow, and ambient-state behavior of every
  candidate; the matrix is a bounded phrase list.
- Throughput of the repository grammar, winnow duration-prefix handling, and
  `jiff` friendly parsing in repository shapes.
- Live repository health of the two candidate projects beyond the crates.io
  metadata quoted above.

## Questions still owned by the human grilling

- Which dialect surfaces stay, and which go.
- Whether the grammar table in the synthesis proposal is adopted, and which
  weekday and same-day rules apply.
- Period placement: query-only wrapper versus runtime carrier, and the
  `CalendarPeriod` shape.
- Zone-frame timing: lazy acquisition policy details.
- Filename period forms: which labels stay frozen and which add.

Each item maps to a numbered question in `z-grilling-guide.md`; the guide
carries the dispositions.
