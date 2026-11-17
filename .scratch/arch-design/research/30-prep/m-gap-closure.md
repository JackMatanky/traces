# Temporal gap-closure record (`m-gap-closure`)

This record lists what the gap-closure round measured, what it proved, and
what it did not measure. The parity probe, the candidate probes, and the
Moment counterexample now live in this repository, so a reader can rebuild and
rerun them. Paths below are relative to this directory.

## Scope and environment

The recorded runs use one Apple Silicon arm64 host (macOS 27.2). Probe runs
use `nightly-2026-10-01` (rustc 1.101.0-nightly, 2026-09-30). Both candidate
features also build with stable rustc 1.92.0 and 1.98.0. The older 1.89.0
record covers `english` only. These builds do not establish a minimum
supported Rust version. The repository declares a 1.96 floor.

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

The probe uses the fixed anchor 2026-03-14T12:00:00Z, a Saturday. The
`english` feature selects `jiff-english 0.0.1`. The `fuzzy` feature selects
`fuzzydate 0.4.1`. Each phrase runs in a separate process.
`TRACES_DIALECT=uk` selects the English parser's UK dialect. The default
dialect is US. Both builds write the same executable path. The expanded
run keeps separate copies to prevent a feature mix-up.

### Grammar, weekdays and safety

Both date-parser entry points return points, not periods. Their result
types supply neither a period unit nor an error span. `jiff-english`
rejects `next week` with `expected week day or month name`. Fuzzydate
returns a +7-day point. Neither accepts `eom` or `in 2 weeks`.

The original Monday case did not test same-day behavior. A mislabeled
fuzzydate run also used the English executable. Neither supports the old
claim that fuzzydate contradicts its source. The expanded run gives:

| Input | `jiff-english`, US | `jiff-english`, UK | `fuzzydate` |
|:------|:-------------------|:-------------------|:------------|
| `saturday` | 2026-03-21 00:00 | 2026-03-21 00:00 | 2026-03-14 12:00 |
| `next saturday` | 2026-03-21 00:00 | 2026-03-28 00:00 | 2026-03-21 12:00 |
| `last saturday` | 2026-03-14 00:00 | 2026-03-14 00:00 | 2026-03-07 12:00 |

These are UTC results with the fixed anchor. The dialect changes weekday
semantics as well as numeric date order. It is not a language locale.
`last friday` still differs: English returns 2026-03-13; fuzzydate returns
2026-03-06. English lenient mode accepts `start of month` as midnight today.
Fuzzydate rejects it. This does not provide a period parser.

The expanded Unicode check finds a real process-abort defect. The Hebrew
input `מחר` reaches `week_day` in English and aborts. The source slices
`&s[0..3]` without a UTF-8 boundary check
(`jiff-english/src/types.rs:433-446`). The error includes
`end byte index 3 is not a char boundary`. The earlier full-width and CJK
cases did not expose this path.

The tracked `unicode-sweep.py` checks 5,514 distinct short inputs. It
selects Latin, Greek, Hebrew, CJK, emoji and format characters. It tests
six shapes for each selected character, then removes duplicates. Each
input gets its own process.

| Build | Inputs | Nonzero exits |
|:------|-------:|--------------:|
| `english` | 5,514 | 1,234 |
| `fuzzy` | 5,514 | 0 |

This release profile uses `panic = "abort"`. An ordinary `Result` wrapper
cannot recover from these aborts. The sweep counts process failures, not
only parser errors. Its inspected English failures show the UTF-8 slice
panic. This is a bounded sample, not exhaustive Unicode safety evidence.

[P] Reject unmodified `jiff-english 0.0.1` for user input. A later fixed
version needs the failing-input regression and grammar checks before
adoption. Fuzzydate passes this sample, not every possible input. It still
fails the direct period-output and error-span requirements.

The expanded phrase run also covers large signed counts, large years,
long strings and non-English words. It is a sample, not an overflow proof.
An embedded NUL cannot pass as a process argument. The subprocess error
is not evidence about either parser.

### Indicative parse timings

The final run makes five sequential repetitions per phrase and candidate.
Each repetition parses 20,000 inputs with `black_box`, in a release build.
Dialect selection occurs before the timed English loop. The successful
phrases return 20,000 successes; the typo returns zero.

These custom timings are not Criterion results or controlled throughput.
They have no confidence intervals. The first English repetition includes
a 587 ns `tomorrow` result; the range below retains it.

| Phrase | English median (range), ns/parse | Fuzzy median (range), ns/parse |
|:-------|--------------------------------:|------------------------------:|
| `tomorrow` | 264 (254–587) | 376 (373–380) |
| `next monday` | 582 (573–696) | 597 (589–610) |
| `next weke` (error) | 372 (357–413) | 223 (219–249) |

An intermediate timing loop read the dialect environment each iteration.
Its 811/854/547 ns figures are not parser-only timing evidence.
The earlier 263/588/383 ns and 366/585/215 ns single-run figures are
historical observations, not the current summary.

### Final probe size and compatibility

These final release builds use the same source and lockfile.
The baseline has the probe's Jiff features but no candidate feature.

| Build | Binary size | Delta from baseline |
|:------|------------:|--------------------:|
| baseline | 418,960 B | 0 B |
| `--features english` | 502,080 B | 83,120 B (81.17 KiB) |
| `--features fuzzy` | 519,424 B | 100,464 B (98.11 KiB) |

The older sizes, 418,944/485,488/469,008 B, precede the final probe edits.
They are not current deltas. Probe sizes are not repository binary sizes.

The feature tree confirms that English enables Jiff defaults.
`tz-fat`, `tzdb-zoneinfo`, `tzdb-concatenated`, `tzdb-bundle-platform` and
`perf-inline` join the explicit project features. Fuzzydate adds
`chrono 0.4.45` without changing Jiff's feature set.
Both candidates declare edition 2021 and no `rust-version`.
Both build with stable 1.92.0 and 1.98.0 using separate target directories.
No result establishes their minimum supported versions.

The crates.io API gives these metadata facts on 2026-11-17:

- `fuzzydate 0.4.1`: created 2022-02-17, last update 2026-04-05, 164,489
  downloads, repository `DevinVS/fuzzydate`.
- `jiff-english 0.0.1`: single release on 2026-08-07, 29 downloads,
  repository `Squirreljetpack/feeling`, a port of `chrono-english`.

### Upstream repository snapshot

The repository checks below occurred on 2026-11-17. The advertised
[English repository API](https://api.github.com/repos/Squirreljetpack/feeling)
and its commit endpoint return HTTP 404. A separate
[repository search](https://api.github.com/search/repositories?q=jiff-english&per_page=5)
returns zero results. These failures do not establish whether the
repository is private, removed or renamed. Its maintenance quality is
unknown from these checks.

The [fuzzydate repository API](https://api.github.com/repos/DevinVS/fuzzydate)
reports a public, non-archived repository. The
[latest commit API](https://api.github.com/repos/DevinVS/fuzzydate/commits?per_page=1)
returns `8dea411bc9325027ce7de20650d6392d6893f268`, dated 2026-04-05.
Its message is `bump version to 0.4.1 (#30)`. These facts establish a
repository snapshot, not a maintenance-quality guarantee.

The [open-issues API](https://api.github.com/repos/DevinVS/fuzzydate/issues?state=open&per_page=10)
returns three entries, including a pull request. Issue 32 requests
multi-language support; pull request 31 remains open and unmerged.
Reopened issue 17 reports that `next monday` selects January 19 from
Saturday January 10, not January 12. That report is not a new reproduction
against the cached release. The fixed-anchor weekday results above are
the measured cases.

### Reproduce the candidate checks

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

## Repository dependency-cost experiment

The experiment uses three throwaway copies of commit `e4bd68e679`.
It leaves the production worktree unchanged. The baseline is that commit.
The English copy adds `jiff-english 0.0.1`. The fuzzy copy adds
`fuzzydate 0.4.1` and `chrono 0.4.45`.

Each modified copy adds a reachable parser call inside
`DatePoint::parse_with_pattern`. `black_box` consumes its success flag. The
original parser still supplies the function's result. This prevents unused
dependencies from disappearing at link time. It is a dependency-linkage
experiment, not an adopted parser integration.

The temporary English call uses `Zoned::now()`. The fuzzy call supplies
no anchor and uses UTC. Those ambient anchors deliberately do not model
the proposed deterministic frame. The experiment has no period adapter,
span adapter, new value type, index change or production grammar.

All builds use the shipped release profile and pinned nightly. Builds run
sequentially, with separate clean target directories. The first builds
include uncontrolled cache and download effects. Two later rounds clean
each target before its build. The executable sizes are identical in all
three rounds.

| Build | Stripped binary | Delta | First wall time | Later clean-build times |
|:------|----------------:|------:|----------------:|:------------------------|
| baseline | 3,916,240 B | 0 B | 58 s | 58 s, 56 s |
| English | 3,966,288 B | 50,048 B (48.88 KiB) | 98 s | 59 s, 60 s |
| fuzzy | 3,983,200 B | 66,960 B (65.39 KiB) | 98 s | 56 s, 59 s |

The repeat times do not prove a compile-time regression or improvement.
They have one-second resolution, fixed ordering and no statistical control.
The first 98-second observations remain part of the evidence. These sizes
include a reachable candidate call, not the complete feature. They do not
replace a post-integration measurement.

The baseline lockfile has 345 packages. English adds three entries:
`jiff-english 0.0.1`, `jiff-tzdb-platform 0.1.3` and `scanlex 0.1.4`.
Fuzzy adds eleven entries: `fuzzydate 0.4.1`, `chrono 0.4.45`,
`iana-time-zone 0.1.65`, `iana-time-zone-haiku 0.1.2`,
`android_system_properties 0.1.6`, `core-foundation-sys 0.8.7`,
`windows-core 0.62.2`, `windows-implement 0.60.2`,
`windows-interface 0.59.3`, `windows-result 0.4.1` and
`windows-strings 0.5.1`. Neither copy removes a package.
Target-specific lock entries do not mean that every package compiles on
macOS. The English repository feature tree also confirms Jiff defaults.

## Semantic audit of the STE changes

The audit compares the four evidence reports between `e0b69bc365` and
`e4bd68e679`. It covers all 214 diff hunks: 47 in B, 72 in C, 20 in D and
75 in E. No hunk remains unjudged. Later corrections distinguish repaired
historical discrepancies from current facts and restore proposal rejection
conditions. The current synthesis, guide and ticket use the corrected
runtime evidence. This audit does not make proposals accepted decisions.

## Production regression check

The parent ran the canonical `test` task with the `temporal` filter. It
passed 831 unit tests, five integration tests and nine feature tests. No
E2E test matched that filter. The task also passed 87 doctests. This proves
the exercised current behavior, not the proposed redesign.

The earlier `verify` request timed out at the MCP transport. Task history
records that verification as cancelled and its unfiltered test task as
failed with exit code 100. Task logs were disabled. The original
failing-test output is unavailable. No broad verification pass is claimed.

The safe hk check refused to run because eight check commands have unknown
effects. No unsafe check or configuration change was used to bypass it.

## Evidence gates before adoption

The table below lists explicit gates, not completed experiments.
They distinguish adoption requirements from pre-grilling research.
The product gates need the selected design and its implementation.

| Gate | Current evidence or disposition | Required evidence |
|:-----|:--------------------------------|:------------------|
| User-input safety | English fails the Unicode sample; reject this version as supplied [P] | Fixed-code regression plus bounded malformed-input tests |
| Runtime layout | The proposed mirror is 80 B on one target | Actual product type at most 80 B; no added construction allocation |
| Civil ordering | The proposed type must not expose context-free `Ord` | Compile-fail consumer test against the selected product API |
| Grammar | All inspected candidate date APIs return points without spans | Adopted closed grammar, period units and span-carrying errors |
| Temporal keys | No selected key implementation exists | Representative index timing, memory and residual equivalence |
| Exact pushdown | Ticket 29 requires exactness | Per-operator residual-equivalence proof before each `Exact` claim |
| Persisted format | The redesign is not implemented | Format epoch change and rebuild tests with the landed representation |
| Runtime notices | Frame failure memoization does not deduplicate notices | Ticket 34 delivery and deduplication tests |
| Integrated cost | Dependency-linkage sizes and repeated build observations only | Size, startup and throughput of the adopted integration |

No prototype can prove the layout, index cost or completeness of product
code that does not yet exist. A speculative implementation would select
human-owned choices. These gates must not be relabeled as passed research.

## Remaining limits

- Unicode, overflow, locale and ambient-state checks are bounded.
  The English US/UK dialect checks do not test every host locale.
- Parsing allocations remain unmeasured. The layout allocation check
  starts after fixture parsing.
- The two source-only alternatives have no runtime or size measurements.
  Their point-only results and missing spans already fail the direct API
  requirements. No competitive speed ranking is claimed.
- The in-repository grammar has no measured size or test cost.
  Its grammar choices remain proposals.
- Startup, representative temporal-index costs and full integration costs
  remain implementation gates, not dependency-linkage measurements.
- No exhaustive transitive ambient-state proof exists.
  Adoption must select an explicit-anchor, explicit-zone path.
- Crates.io metadata does not prove repository maintenance quality.

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
