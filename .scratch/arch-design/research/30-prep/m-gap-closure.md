# Temporal gap-closure record (`m-gap-closure`)

This record lists what the gap-closure round measured, what it proved, and
what it did not measure. The parity probe, the candidate probes, and the
Moment counterexample now live in this repository, so a reader can rebuild and
rerun them. Paths below are relative to this directory.

## Scope and environment

The recorded runs use one Apple Silicon arm64 host (macOS 27.2). Probe runs
use `nightly-2026-10-01` (rustc 1.101.0-nightly, 2026-09-30). The recorded
stable build uses rustc 1.89.0 for the `english` feature only.

The release profile sets `opt-level = "z"`, strip, LTO, one codegen unit,
and `panic = "abort"`. The layout tests use the default Cargo test profile,
not that release profile. The lockfiles pin `jiff-core 0.1.1` and
`jiff-tzdb 0.1.9`, as in the repository lockfile at preparation time.
This does not mean that every transitive dependency matches.
The original preparation pins those versions with
`cargo update --precise`. Use the preserved lockfiles when reproducing
the recorded dependency set.

The probes under `probes/` are research artifacts for ticket 30. They are
not part of the product test suite and do not link repository code.

## Parser parity probe (`probes/parity`)

The probe compares the legacy speedate-based field parser against a candidate
Jiff parser. The guarded domain is byte-level: years `0000..9999`, months
`00..13`, and days `00..32`. The probe also mutates one byte at every position
of six seed strings and adds ten malformed or extended shapes. It checks
4,635,370 inputs in total.

The recorded result below summarizes the run. The two error messages are
abbreviated. This is not a verbatim output log.

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

Run this command from `probes/parity`:

```sh
cargo +nightly-2026-10-01 run --release --locked
```

Limits. Parity holds only for the guarded ten-byte date shape with strict
ASCII digits and separators. Raw Jiff accepts a superset: compact `20260101`,
naive datetimes, and negative years. The probe does not cover datetimes,
spans, time zones, or any repository parser beyond the field one.

## Complete-value layout probe (`probes/parity/tests/layout.rs`)

Earlier evidence measures the 24-byte temporal payload only. The layout
test measures a mirror of the complete proposed `NoteFieldValue`. The
mirror replaces `DateTimeValue` with `DateTimePayload { Civil, Instant }`.
It uses Jiff's `civil::DateTime` and `Timestamp` types. It models the link
and duration fields from `src/note/links.rs:28` and
`src/temporal/duration.rs`.

Recorded result, default Cargo test profile, pinned toolchain:

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

The mirror enum measures exactly 80 bytes on this host and compiler.
Its Duration and Object payloads both measure 72 bytes. The temporal
payload increase from 16 to 24 bytes does not increase this mirror's size.

The allocation check parses the fixture before the measured window.
Inside the window, it constructs the temporal enum value from that fixture.
The recorded result is zero allocations and zero deallocations. This does
not measure parsing allocations or the product pipeline.

Limits. The mirror is not the proposed production type. It uses a boolean
in place of `LinkType` and does not import the product's leaf types.
No result guarantees the layout of a different enum, compiler, or target.
The production size assertion remains an implementation gate.

Run the layout tests from `probes/parity`:

```sh
cargo +nightly-2026-10-01 test --locked --test layout -- --nocapture
```

## Candidate probes (`probes/candidates`)

The probe uses the fixed anchor 2026-03-14T12:00:00Z, a Saturday.
The `english` feature selects `jiff-english 0.0.1`. The `fuzzy` feature
selects `fuzzydate 0.4.1`. The recorded matrix contains 15 phrases for
each candidate. Each phrase runs in a separate process. Process isolation
alone does not prove that every exit succeeds or that no input can panic.

The recorded behavior and inspected output types support these findings:

- Both date-parser entry points return points, not periods. Their date
  result types do not provide a period unit or an error span.
- `jiff-english` rejects `next week` outright (`expected week day or month
  name`). `fuzzydate` accepts it and returns a +7-day point. Neither accepts
  `in 2 weeks`.
- `last friday` diverges: `jiff-english` returns the previous day
  (2026-03-13), `fuzzydate` returns the Friday one week back (2026-03-06).
- With this Saturday anchor, bare `monday` returns the next Monday.
  `jiff-english` returns midnight; `fuzzydate` retains the anchor time.
  This case does not establish either candidate's same-day weekday rule.
- Neither accepts `eom`. `jiff-english` lenient mode misparses
  `start of month` as midnight today; `fuzzydate` rejects it.
- The sampled typo, full-width digits, and CJK input return errors.
  No panic appears in those recorded cases. The `week_day` byte-slice
  hazard remains a source observation, not a general runtime guarantee.
- Both reject `3 days ago tomorrow` and `tomorrow xyz`.

Recorded indicative timings cover 20,000 iterations with `black_box`.
The run uses the release profile on one host. It has no confidence
intervals and is not a controlled benchmark. These numbers describe the
executed probe revision, not a guaranteed result from the final source.

| Phrase | `jiff-english` | `fuzzydate` |
|:-------|---------------:|------------:|
| tomorrow | 263 ns/parse | 366 ns/parse |
| next monday | 588 ns/parse | 585 ns/parse |
| `next weke` (fail) | 383 ns/parse | 215 ns/parse |

Earlier probe revision, release profile, recorded binary sizes:

| Build | Binary size | Delta |
|:------|------------:|------:|
| baseline (Jiff set only) | 418,944 B | — |
| `--features english` | 485,488 B | +65.0 KiB |
| `--features fuzzy` | 469,008 B | +48.9 KiB |

These sizes predate the final timing-mode edits to the probe source.
The current source and lockfiles permit new measurements, not an exact
reproduction of those earlier binary sizes.

The recorded `cargo tree -e features -i jiff` output shows:

- `english` pulls `jiff` default features on top of the repository's
  deliberate `default-features = false` set: `tz-fat`, `tzdb-zoneinfo`,
  `tzdb-concatenated`, `tzdb-bundle-platform`, and `perf-inline` join the
  unified build. The delta above includes this unification plus the parser.
- `fuzzy` leaves the Jiff feature set unchanged and adds `chrono 0.4.45`.
  Its recorded size delta includes the `chrono` stack and parser.

The source manifests and build record support these compatibility facts:

- Both candidates declare edition 2021 and no `rust-version`.
- The recorded stable rustc 1.89.0 build selects `english` only.
  It does not establish stable compatibility for `fuzzy`.
- Neither observation establishes a minimum supported Rust version.
  The repository declares a 1.96 floor.

The crates.io API gives these metadata facts on 2026-11-17:

- `fuzzydate 0.4.1`: created 2022-02-17, last update 2026-04-05, 164,489
  downloads, repository `DevinVS/fuzzydate`.
- `jiff-english 0.0.1`: single release on 2026-08-07, 29 downloads,
  repository `Squirreljetpack/feeling`, a port of `chrono-english`.

Run these commands from `probes/candidates`. Select one candidate feature
per build. Both features share the same binary path, so check which build
produces each result.

```sh
cargo +nightly-2026-10-01 build --release --locked --features english
target/release/temporal-cost-probe "next monday"
target/release/temporal-cost-probe "next monday" --bench
cargo +nightly-2026-10-01 build --release --locked --features fuzzy
target/release/temporal-cost-probe "next monday"
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
  Probe sizes and warm-build completion times are not repository
  measurements. No compile-time benchmark ran. The 6.3/6.7/6.0 second
  figures are Cargo completion times, not isolated linker measurements.
- Temporal-key index timing and memory on representative corpora. That needs
  the implemented temporal keys and stays an acceptance gate.
- Parsing allocations or the product allocation profile. The layout test
  counts only enum construction after the fixture parse.
- Exhaustive Unicode, overflow, and ambient-state behavior. The matrix
  contains a bounded phrase list.
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

Each item maps to a numbered question in `z-grilling-guide.md`.
The guide records the current status of each question.
