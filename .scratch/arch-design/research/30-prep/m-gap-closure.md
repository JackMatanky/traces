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

The original parity and cost probes use a release profile with
`opt-level = "z"`, strip, LTO, one codegen unit and `panic = "abort"`.
The later acceptance instrument instead uses unwind to classify panics.

The layout tests use the default Cargo test profile. The lockfiles pin
`jiff-core 0.1.1` and
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

The
[open-issues API](https://api.github.com/repos/DevinVS/fuzzydate/issues?state=open&per_page=10)
returns three entries, including a pull request. Issue 32 requests
multi-language support; pull request 31 remains open and unmerged. Reopened
issue 17 reports that `next monday` selects January 19 from Saturday January 10,
not January 12. That report is not a new reproduction against the cached
release. The fixed-anchor weekday results above are the measured cases.

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

## Four-candidate acceptance closure

The 2026-11-18 round adds the standalone `probes/acceptance` instrument.
All four candidate builds pass with Rust 1.96.0, the repository floor.
Its dependencies pin Jiff 0.2.38, jiff-core 0.1.1 and jiff-tzdb 0.1.9.
The Chrono routes use `chrono 0.4.45` and harness-only
`chrono-tz 0.10.4`, whose recorded IANA data version is `2025b`.
These rules are explicit inputs, not the process-local zone.

The release instrument uses size optimization, LTO and one codegen unit.
It uses unwind, not the product's abort strategy. A caught panic is a
`PANIC` result, not an ordinary error or evidence of product recovery.
The no-op panic hook prevents panic diagnostics from entering the counts.
Native library stderr remains visible in the JSON records.

The tracked `run-matrix.py` defines 64 shared cases. It adds two English
lenient cases, four no-parser baseline records and 36 host-context
variations. The completed run emits 298 JSON records. Each records a first
call and a warmed call with identical explicit inputs. The driver checks
their result equality. It also checks that baseline calls allocate
nothing. The baseline flag skips calls, not parser linkage.

The rows below count ordinary cases, not the baseline or host variations.
These counts describe the sampled grammar and inputs. They are not
coverage percentages or safety rankings.

| Candidate | Cases | OK | ERROR | PANIC |
|:----------|------:|---:|------:|------:|
| `jiff-english 0.0.1` | 66 | 32 | 27 | 7 |
| `fuzzydate 0.4.1` | 64 | 29 | 34 | 1 |
| `parse_datetime 0.17.0` | 64 | 38 | 26 | 0 |
| `chrono-english 0.2.1` | 64 | 32 | 30 | 2 |

### Safety and consumption

The English build panics on the Unicode identifiers and several large
counts. Its Jiff span constructors reject those counts by panicking before
`checked_add` can return an error. Fuzzydate panics on
`3000000000 days ago` with `` `NaiveDate - TimeDelta` overflowed ``.
English and Chrono-English also panic on the two Unicode identifiers.
English panics there because slicing cuts inside a multi-byte character.

English and Chrono-English accept `4294967295 years` from the 2024 anchor
but return 2023. The accepted numeric count wraps to a negative year
offset. The other two candidates reject the same input. These are
input-handling defects, not desired calendar policies.

Chrono-English writes `fkd date` to stderr on each `1000000 years` call.
English panics on that input because the year count exceeds the span
range. Fuzzydate and GNU return ordinary errors. Native library stderr
does not pollute the JSON result channel, but it is still unsolicited
process output.

JSON input permits an embedded NUL, which an OS argument alone cannot
carry. Strict English, fuzzydate and GNU reject `today\u0000junk`.
Chrono-English returns the anchor, ignoring the suffix.
[P] Reject the three panic-producing versions as supplied for user input.
The GNU sample has no panic, but its date API still returns points without
period units or error spans. It also accepts grammar forms outside the
proposed closed grammar. No supplied API meets that complete contract.

### T1 to T10 observations

The instrument executes concrete cases for every rejection-test family.
Observed results do not choose Traces' weekday, overflow or zone policy.

| Test | Observed evidence |
|:-----|:------------------|
| T1, period | English and Chrono-English reject `next week`; fuzzydate and GNU return March 21 noon from March 14 noon. No period unit survives. |
| T2, week rule | English APIs select March 15 in US and March 22 in UK for `next friday`. Fuzzydate selects March 22; GNU selects March 15. |
| T3, same day | All seven matching weekdays run. English APIs move forward; fuzzydate retains the day and time; GNU selects midnight today. |
| T4, gap/fold | Explicit `00:00` runs in São Paulo and Havana. The table below records the different policies. |
| T5, explicit context | All 36 host variations preserve the selected explicit-anchor/zone result. |
| T6, Unicode | Both English APIs panic on `éé` and `מחר`. The earlier wider English Unicode sweep remains separate evidence. |
| T7, suffix | Strict English rejects the trailing-time case; its lenient API accepts it. Chrono-English rejects that case but ignores a NUL suffix. GNU accepts `2024-03-14 x` because `x` names military zone −11:00. |
| T8, grammar | GNU accepts `@1758724019` and `TZ="UTC" 2 days`. These need a closed-grammar gate if adopted. |
| T9, month end | From March 31, `1 month ago` gives February 29 in the English APIs and fuzzydate. GNU gives March 2. |
| T10, errors | All four return native error values for invalid input. None supplies the required span contract. |

The weekday examples use the explicit 2024-03-14T12:00:00Z anchor.
The month-end case uses 2024-03-31T12:00:00Z. English APIs return midnight
in that month-end case; fuzzydate and GNU retain noon.

### Midnight and skipped-day evidence

The literal `today at 00:00` is not interchangeable with `00:00`.
Some parsers reject its syntax; Chrono-English can ignore its suffix.
The explicit-midnight rows below use `00:00`, not the rejected spelling.
The date-only `today` case retains a base time and proves no midnight rule.

| Zone and local date | English | Fuzzydate | GNU | Chrono-English |
|:--------------------|:--------|:----------|:----|:---------------|
| São Paulo, 2015-10-18 gap | 01:00 −02:00 | ERROR | 01:00 −02:00 | ERROR |
| Havana, 2020-03-08 gap | 01:00 −04:00 | ERROR | 01:00 −04:00 | ERROR |
| Havana, 2020-11-01 fold | 00:00 −04:00 | 00:00 −04:00 | 00:00 −05:00 | ERROR |

The anchors are 2015-10-18T15:00:00Z, 2020-03-08T16:00:00Z and
2020-11-01T17:00:00Z respectively. GNU's fold result retains the base's
offset in this case. A common Jiff dependency does not imply identical
fold selection across APIs.

For `tomorrow` from 2011-12-29T22:00:00Z in Pacific/Apia, English, GNU and
Chrono-English return December 31 noon. Fuzzydate returns an error.
This is point arithmetic. It does not replace ticket 21's civil-date
membership rule for a skipped day.

### Parse allocations and result lifetimes

Input decoding, timestamp parsing and zone lookup occur before counting.
The construction region measures typed anchor projection. It records zero
allocations in every emitted record. Each parser region includes native
input copies, errors and unwind work. Formatting and result destruction
occur outside those regions. Drop counts appear separately in the JSON.
The first result stays alive during the warmed call.

The table uses `thursday` with the March 14 noon UTC anchor. The parsers
return different points, so these are API observations, not equal-work
performance rankings.

| Candidate | First allocations | First allocated bytes | Warmed allocations | Warmed allocated bytes |
|:----------|------------------:|----------------------:|-------------------:|-----------------------:|
| English | 8 | 73 | 8 | 73 |
| Fuzzydate | 12 | 12,834 | 5 | 78 |
| GNU | 1 | 128 | 1 | 128 |
| Chrono-English | 4 | 41 | 4 | 41 |

Warmed means one earlier call in the same process. The fuzzydate difference
includes first-call initialization; it is not evidence of a per-row leak.
The input string is borrowed where the native API permits it. Required
native copies remain part of the measured parser cost.

### Explicit context and compatibility

The host variants change `TZ`, `LC_ALL` and `TZDIR` in fresh processes.
They include an invalid host zone and an absent zone-data directory.
The supplied zones are UTC, America/Havana and Europe/Prague.
All selected results stay equal under these 36 variations.
The driver also checks negative fractional epoch conversion with
1969-12-31T23:59:59.5Z.

Source inspection explains the selected paths. Fuzzydate's
`aware_parse(Some(anchor), zone)` skips the lazy `Local::now` fallback.
Its AST uses the supplied zone. Both English APIs receive their base
explicitly. GNU's in-range and extended builders use `Some(base)` rather
than their clock fallback. Parsed GNU zone prefixes can still override
the supplied zone. Such input needs its own grammar restriction.

These are scoped source checks plus host perturbations, not a proof about
every generic timezone implementation or every input. A selected
integration must bind an actual zone-data source and version.

All four configurations compile with Rust 1.96.0. GNU's package declares
`rust-version = "1.71.1"`; that is not a test of its entire current
dependency graph on that older compiler. Chrono-English declares edition
2024 without a Rust-version promise. The other two declare edition 2021
without a Rust-version promise. Missing upstream support promises are
unknown facts, not values that a successful build can establish.

### Instrument size and reproduction

The binaries below include JSON reporting, allocation instrumentation and
unwind handling. The Chrono variants also include the harness-only IANA
adapter. They are not comparable with the older abort-profile cost probe.

| Instrument build | Stripped bytes |
|:-----------------|---------------:|
| Jiff-only baseline | 766,544 |
| Chrono anchor baseline | 1,955,888 |
| English | 866,560 |
| Fuzzydate | 2,039,792 |
| GNU | 866,736 |
| Chrono-English | 2,022,288 |

Run from the repository root. Build exactly one parser feature at a time.
Copy each executable before the next build overwrites its path.

```sh
P=.scratch/arch-design/research/30-prep/probes/acceptance
mkdir -p "$P/target/bin"
for feature in baseline chronoanchor english fuzzy gnu chronoenglish; do
  if [ "$feature" = baseline ]; then
    mise run --force --tool rust@1.96.0 release \
      --manifest-path "$P/Cargo.toml" --locked
  else
    mise run --force --tool rust@1.96.0 release \
      --manifest-path "$P/Cargo.toml" --locked --features "$feature"
  fi
  cp "$P/target/release/temporal-acceptance-probe" "$P/target/bin/$feature"
done
python3 "$P/run-matrix.py" "$P/target/bin"
```

The existing production test task hardcodes package `traces`.
It cannot run a standalone probe package. The probe's own unit test command
is `cargo +1.96.0 test --manifest-path "$P/Cargo.toml" --locked`.
Its three tests distinguish a valid date, an ordinary error and a panic.
They do not claim that any candidate is panic-free.

### Upstream access and parity scope

The advertised English repository still returns HTTP 404 on 2026-11-18.
The packaged `.cargo_vcs_info.json` names commit
`a8b87edbeebafebd06a6c4f5a2b502b79b61e409`. A public commit search finds
no result. Public repository search and the publisher's public repository
list supply no alternative. The cached package remains inspectable.
Private or unavailable maintenance history cannot become verified
maintenance evidence through more local tests.

The [scope report](./n-parity-and-scope.md) traces every speedate call and
its shared consumers. Only the unsigned full-date field branch calls it.
Datetime, duration and zone parser comparisons are not missing comparisons
for that narrow removal. Their redesign still needs separate acceptance
evidence. The preserved guarded parity result remains the relevant
replacement evidence; it does not justify a whole-parser replacement.

## Production regression check

The parent ran the canonical `test` task with the `temporal` filter. It
passed 831 unit tests, five integration tests and nine feature tests. No
E2E test matched that filter. The task also passed 87 doctests. This proves
the exercised current behavior, not the proposed redesign.

The old MCP `verify` request timed out. Its original assertion output
remains unavailable. A later diagnostic capture identifies five E2E help
snapshots whose prose differs from the current CLI documentation.
The repair retains all 13 help surfaces as semantic usage, option and alias
contracts. It removes the obsolete wording snapshots and their 11 fixtures.
It changes no CLI command or flag.

The final managed `mise run verify` passes. It runs formatting, checks,
strict Clippy and the canonical test task. That task passes 3,975 unit
tests, 41 integration tests, 104 E2E tests, 155 feature tests and 87
doctests. Two pre-existing E2E tests remain skipped.

The hk refusal was missing effect metadata, not a failed static analysis.
Ten check-side wrappers now state the effects of the eight custom steps.
The commands remain unchanged.

Write effects include Cargo artifacts,
advisory-cache updates and task-launched environment preparation.
The full plan has no unknown included check effects. The `check` task uses
`hk check --safe`, and its changed-file checks pass within verification.
Effect metadata states intended writes; it is not a sandbox.

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
- Parsing allocations now have first-call and warmed observations for all
  four candidates. They remain instrument observations, not product
  allocation guarantees.
- All four candidates now have runtime cases and instrument-size
  observations. No new controlled throughput ranking is claimed.
- The in-repository grammar has no measured size or test cost.
  Its grammar choices remain proposals.
- Startup, representative temporal-index costs and full integration costs
  remain implementation gates, not dependency-linkage measurements.
- Selected explicit-anchor/zone paths now have source inspection and 36
  host perturbations. This does not prove every possible transitive path.
- Private or unavailable upstream maintenance history remains inaccessible.
  A package with no declared Rust-version promise has no verified promise.

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
