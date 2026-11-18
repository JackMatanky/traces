# Crate evidence for ticket 30 preparation

Crate-source check for [ticket 30][t30].
The file confirms or rejects the claims in [synthesis.md](./synthesis.md),
questions 9 to 12. This file does not resolve the ticket.

[t30]: ../../issues/30-temporal-boundary-and-dialects.md

Every claim carries a tag. [V] marks a named source read or recorded probe
result. [H] marks a hypothesis. [P] marks a proposal for the grilling.
The 2026-11-18 acceptance round executes all four candidate APIs.
[m-gap-closure.md](./m-gap-closure.md) records the matrix, allocations,
binary sizes, historical timings and dependency-linkage experiments.
Bounded source searches and host variations do not prove absence of all
transitive ambient reads.

## Cache and tools

The `.rust-docs/cache/crates/` tree held these versions at check time.
The first list is: jiff 0.2.38, jiff-english 0.0.1, fuzzydate 0.4.1,
speedate 0.18.0, winnow 1.0.4 and logos 0.16.1.
The second list is: parse_datetime 0.17.0, chrono-english 0.2.1,
chrono 0.4.45, time 0.3.55, toml 1.1.6+spec-1.1.0 and toml_edit 0.25.15.
The two new targets, jiff-english and fuzzydate, were already cached when
this check began. No cache job ran.

The lockfile resolves jiff 0.2.38 against jiff-core 0.1.1
(`Cargo.lock:1204-1206`). jiff 0.2.38 source delegates its ISO week rule to
jiff-core. The checked jiff-core 0.1.1 source came from the cargo registry
copy of this project. Both jiff-core 0.1.1 and 0.1.4 carry the same
`is_long_iso_week_year` guard line. [V]

Tool note. The `get_dependencies` tool failed on both new crates with
`Failed to cache crate: Failed to generate dependency metadata: error:
manifest path ... does not exist`. Dependency facts below come from each
cached `Cargo.toml` and from `Cargo.lock` instead. [V]

## Comparison table: natural-language candidates

| Property | jiff-english 0.0.1 | fuzzydate 0.4.1 | parse_datetime 0.17.0 | chrono-english 0.2.1 |
| --- | --- | --- | --- | --- |
| License | MIT (`Cargo.toml:25`) | MIT (`Cargo.toml:41`) | MIT (`Cargo.toml:25`) | MIT (`Cargo.toml:26`) |
| Direct deps | jiff `0.2`, scanlex `0.1` (`Cargo.toml:42-47`) | chrono 0.4 `clock`, lazy_static `1.5`, thiserror `2.0`; anyhow, clap optional (`Cargo.toml:48-67`) | jiff `0.2.15` with `tz-system` and tzdb features, num-traits `0.2.19`, winnow `1.0.0` (`Cargo.toml:49-63`) | chrono 0.4 `alloc`, scanlex `0.2` (`Cargo.toml:37-44`) |
| Present in repository lockfile | jiff present; jiff-english and scanlex absent | fuzzydate and chrono absent | parse_datetime absent; jiff, winnow, num-traits present | chrono-english, chrono and scanlex absent |
| Injected anchor | yes; `now: &Zoned` argument (`lib.rs:124-128`) | yes; `parse_relative_to` takes a naive datetime; `aware_parse` takes an optional anchor and explicit zone | yes; `parse_datetime_at_date(Zoned, ...)` (`lib.rs:230-235`) | yes; `now: DateTime<Tz>` argument (`lib.rs:95-99`) |
| Ambient clock or zone reads in inspected paths | search of crate `src/` finds no `now()` call [V]; transitive absence unproved | `parse` reads `Local::now` (`lib.rs:268-270`); `parse_relative_to` resolves through `Local` (`lib.rs:259-262`) | plain `parse_datetime` uses `Zoned::now` (`builder.rs:263`) | inspected `Utc::now` calls occur in `#[cfg(test)]` code (`lib.rs:146-158`) [V]; transitive absence unproved |
| `next week` | error; `wee` fails weekday and month tables (`types.rs:433-446`) [runtime confirmed] | point; today's weekday plus one week (`ast.rs:357-370`) [runtime confirmed] | point; now plus 7 days, time kept (`ordinal.rs:32`, `relative.rs:118-122`) [runtime confirmed] | error; same tables as jiff-english (`types.rs:402-460`) [runtime confirmed] |
| Bare weekday | next occurrence; no parsed time gives midnight (`types.rs:119-137,346-363`) [seven same-day anchors confirmed +7 days] | today when today matches; base time kept (`ast.rs:454-463`) [seven same-day anchors confirmed] | midnight when today matches (`builder.rs:311-349`) [seven same-day anchors confirmed] | same source rule as jiff-english (`types.rs:102-135`) [seven same-day anchors confirmed +7 days] |
| Error spans | none; message string only (`errors.rs:5-24`) | none; token text only (`lib.rs:212-225`) | none; public `InvalidInput` drops detail (`lib.rs:140-159`) | none; message string only (`errors.rs:5-24`) |
| Locale and dialect | English only; `Dialect` changes numeric date order and next-weekday semantics [runtime: US/UK next Saturday differs] | English only | English only; GNU date dialect, ASCII lowercasing (`mod.rs:240-246`) | English only; `Dialect` as jiff-english |

The inspected date-parser entry points return points, not periods. [V]
Their point types are `jiff::Zoned`, chrono `NaiveDateTime` or
`DateTime<Tz>`, and `parse_datetime::ParsedDateTime`.
Separate duration-parsing APIs do not change those date result types.
The M report preserves the earlier two-candidate abort-profile experiments:
probe deltas +81.17/+98.11 KiB and repository-linkage deltas +48.88/+65.39
KiB for English/fuzzydate. These are historical bounded observations, not
complete integration costs. The new four-candidate instrumented harness
uses Rust 1.96.0 and `panic = "unwind"`. Its allocation and binary-size
tables are separate; they are not comparable to those abort-profile costs.
No new throughput measurements exist. All four candidates have runtime,
same-day, allocation and repository-floor build evidence. [V]

## jiff 0.2.38

Path prefix: `.rust-docs/cache/crates/jiff/0.2.38/source/`.
This project pins `jiff = { version = "=0.2.38", default-features = false,
features = ["std", "tz-system", "tzdb-bundle-always"] }`
(`Cargo.toml:128`). [V]

### BrokenDownTime getters

Getters live in `src/fmt/strtime/mod.rs`. Read the exact lines there:

- `year()` at line 1962 area, `month()`, `day()`, `hour()` at line 2280,
  `minute()` at line 2299, `second()` at line 2318,
  `subsec_nanosecond()` at line 2353.
- `offset()` at line 2387 returns the `%z` or `%:z` offset, `Option<Offset>`.
- `iana_time_zone()` at line 2417 returns the `%Q` identifier, `Option<&str>`.
  It is always `None` without the `alloc` feature.
- `weekday()` at line 2470 returns the parsed `%A` or `%a` value. It reports
  the parsed value even when it disagrees with the parsed date.
- `meridiem()` at line 2499 returns the `%p` or `%P` value.
- `timestamp()` at line 2553 returns only an explicit `%s` value.
- `hour()` applies the meridiem. `set_meridiem` at line 3139 calls
  `meridiem.adjust_hour`, so `%I%P` reports `13` for `1pm`.

Conversions decide which fields a call needs:

- `to_date()` at line 1679. The rule list is: month and day, an ISO
  week triple, day of year, a Sunday week or a Monday week. A weekday
  that disagrees errors with
  `MismatchWeekday` at lines 1709-1716.
- `to_time()` at line 1921. A missing hour with no lower units returns
  midnight. Lower units without an hour error. The June 31 doctest at lines
  1907-1919 shows that `to_time` alone skips date validation.
- `to_datetime()` at line 1634. Needs `to_date` plus `to_time`.
- `to_timestamp()` at line 1591. Prefers an explicit `%s` timestamp. Else it
  needs a datetime plus an offset, lines 1602-1605.
- `to_zoned_with()` at line 1444. Offset only builds a fixed-offset zone,
  line 1468. Identifier only builds a named zone, lines 1470-1481. Both
  present with a timestamp must agree, lines 1487-1492. A timestamp with no
  offset and no identifier lands in `Etc/Unknown`, lines 1364-1377.

### ISO week dates

`ISOWeekDate::new` rejects week 53 in a short ISO year. The wrapper at
`src/civil/iso_week_date.rs:224-232` documents the rule and defers to
jiff-core. The core guard is
`if week == 53 && !civil::is_long_iso_week_year(year)` at
`jiff-core-0.1.1/src/civil/date.rs:1034`. A long ISO year is one whose last
civil day is a Thursday, lines 1024-1028. The same function also rejects
week 52 Saturday or Sunday in year 9999, lines 1046-1052. [V]

### Skipped and doubled local midnight

`src/tz/ambiguous.rs:77-102` defines `Disambiguation`: `Compatible` is the
default, then `Earlier`, `Later`, `Reject`. `AmbiguousZoned::disambiguate`
at lines 1274-1280 dispatches the four strategies. Per-strategy behavior:

| Strategy | Gap midnight | Fold midnight | Cites |
| --- | --- | --- | --- |
| `Compatible` | forward; Sao_Paulo 2015-10-18 start of day is 01:00 (`zoned.rs:1104-1118`) | earlier offset; New_York 2024-11-03 01:30 is `-04` (`ambiguous.rs:1246-1252`) | `ambiguous.rs:990-992`, RFC 5545 |
| `Earlier` | pre-gap offset; New_York 02:30 maps to 01:30 `-05` (`ambiguous.rs:1078-1084`) | earlier offset (`ambiguous.rs:1086-1092`) | `ambiguous.rs:1051-1053` |
| `Later` | post-gap offset; same instant as `Compatible` in a gap (`ambiguous.rs:1137-1143`) | later offset; `-05` (`ambiguous.rs:1261-1266`) | `ambiguous.rs:1110-1112` |
| `Reject` | error via `unambiguous()` (`ambiguous.rs:1200`) | error (`ambiguous.rs:1204-1205`) | `ambiguous.rs:1230-1232` |

`civil::DateTime::to_zoned` uses the `Compatible` path, `datetime.rs:1514`.
`civil::Date::to_zoned` sets midnight first, `date.rs:1154-1155`.
`Zoned::start_of_day` maps the civil midnight again through the zone,
`zoned.rs:1147-1148`. [V]

### TimeZone::try_system

`TimeZone::try_system` needs the `tz-system` feature, which itself enables
`std` and `windows-link` (`Cargo.toml:119-122`). Without the feature it
returns an error at `src/tz/timezone.rs:392-396`. With the feature it calls
`crate::tz::system::get(db)`, line 399. An unparsable `TZ` value errors at
`system/mod.rs:148-159`. Missing `TZ` with no readable `/etc/localtime`
errors at lines 160-163. The cache keeps one zone for
`TTL = 5 * 60` seconds behind one `RwLock`, lines 69 and 87, and refills
after expiry, lines 132-142. `TimeZone::system()` never errors: it warns and
falls back to `Etc/Unknown`, which behaves like UTC, `timezone.rs:325-337`.
[V]

The owner note originally claimed default features [H].
`Cargo.toml:128` instead sets `default-features = false` with explicit
`tz-system`. The source supports a five-minute TTL and one global lock.
The corrected owner note must retain this feature distinction. [V]

## Question 2: SpanParser prefix length

Question: can the jiff friendly parser report a consumed-prefix length?
Answer: no, not through its public API. [V]

Evidence. `SpanParser` exposes `parse_span` at
`.rust-docs/cache/crates/jiff/0.2.38/source/src/fmt/friendly/parser.rs:183`,
`parse_duration` at line 232 and `parse_unsigned_duration` at line 289.
Each ends with `parsed.into_full()`, lines 189, 244 and 303. `into_full`
errors when any input remains, `src/fmt/mod.rs:216-221`. The internal
`Parsed` struct records `input`, the unconsumed tail, but it is `pub(crate)`
and its fields are private, `fmt/mod.rs:191-207`. The internal `parse`
returns that `Parsed`, `friendly/parser.rs:311-348`. No public routine
returns the tail or its length.

This confirms synthesis Q10: winnow keeps its boundary job because jiff
cannot report the prefix. The synthesis cites `parser.rs:134-194`.
The checked line numbers are 183 to 194 and 311 to 348. [V]

## Question 3: speedate versus jiff on plain dates

Question: what does `Date::parse_bytes_rfc3339` accept that
`jiff::civil::Date::from_str` does not, and the reverse?
Answer from source: `parse_bytes_rfc3339` accepts a 10-byte date shape.
Jiff's parser accepts more shapes. The initial source review found no
counterexample inside the guarded shape. The later parity probe reports
zero differences over 4,635,370 guarded date inputs
(`m-gap-closure.md`). That result does not cover raw Jiff grammar,
datetimes, spans, or zones. [V]
The source trace in [n-parity-and-scope.md](./n-parity-and-scope.md)
confirms the exact replacement boundary: only the unsigned full-date
field branch calls speedate. Datetimes, durations and zone conversion
are not missing speedate-removal comparisons. The broader carrier and
frame redesign still needs its own acceptance evidence. [V]

Path prefix: `.rust-docs/cache/crates/speedate/0.18.0/source/`.

- `parse_bytes_rfc3339` at `src/date.rs:150-158` calls
  `parse_bytes_partial`. It rejects any input longer than 10 bytes.
- `parse_bytes_partial` at `src/date.rs:333-386` demands 10 bytes, four
  ASCII year digits, hyphens at fixed offsets and two-digit fields.
  Month 0 or above 12 errors at line 378. Day outside 1 to month length
  errors at lines 381-383, with leap-year February at lines 371-377.
- The broader `Date::parse_bytes` at `src/date.rs:181-195` additionally
  accepts a bare integer as a Unix timestamp, for example `1577836800`.

Path prefix: `.rust-docs/cache/crates/jiff/0.2.38/source/`.

- `Date::from_str` at `src/civil/date.rs:2045-2051` calls
  `DEFAULT_DATETIME_PARSER.parse_date`, which is
  `fmt::temporal::DateTimeParser::parse_date` at
  `src/fmt/temporal/mod.rs:822-831`.
- `parse_date_spec` at `src/fmt/temporal/parser.rs:524-551` accepts the
  extended `YYYY-MM-DD` form and the basic `YYYYMMDD` form, because the
  separator is optional, lines 530-546.
- `parse_year` at lines 693-707 accepts four digits, and `parse_signed_year`
  at lines 711-724 accepts a sign plus six digits.
- `parse_temporal_datetime` at lines 286-315 accepts a trailing time after
  a space or `T`, then an optional offset, then annotations.
- A trailing `Z` errors on the civil path, `fmt/temporal/mod.rs:802-821`.
  `into_full` rejects any other trailing bytes, `fmt/mod.rs:216-221`.

So jiff accepts every strict `YYYY-MM-DD` string speedate accepts, plus
basic format, signed years and date-time inputs. Speedate accepts no shape
in this pair that jiff rejects. The one broader speedate entry point,
`Date::parse_bytes`, does accept bare timestamps that jiff rejects, but the
project never calls it (`src/temporal/date.rs:366-374`).

Call-site fact. `src/temporal/date.rs:350-374` guards the length first and
then calls `speedate::Date::parse_bytes_rfc3339` only for four-digit years.
It maps `OutOfRangeMonth` and `OutOfRangeDay` to
`"a field is out of range"` and every other error to `"invalid input"`.
Negative years bypass speedate and build through `civil::Date::new`,
lines 385-396. Removing speedate keeps a jiff-only path for the same
accepted set. [V]

## Question 4: winnow in the lockfile

Question: is winnow 1.0.4 a hard dependency of `toml` or `toml_edit` in
this lockfile? Answer: yes for `toml`, yes for `toml_edit` and
`toml_parser`. [V]

- `Cargo.lock:2540-2553`: `toml 1.1.6+spec-1.1.0` lists `winnow`. The
  project depends on `toml = "1.1.6"` with default features
  (`Cargo.toml:147`), and the default feature set includes `parse`
  (`.rust-docs/cache/crates/toml/1.1.6+spec-1.1.0/source/Cargo.toml:98-112`).
  `parse` activates `dep:winnow`.
- `Cargo.lock:2565-2574`: `toml_edit 0.25.15+spec-1.1.0` lists `winnow`.
  Its lockfile dependent is `proc-macro-crate` (`Cargo.lock:1707-1713`), a
  build-path crate, not a direct project dependency.
- `Cargo.lock:2577-2583`: `toml_parser` lists `winnow`.
- Direct project use: `Cargo.toml:152` lists `winnow = "1.0.4"`, and the
  `traces` package entry lists `winnow` (`Cargo.lock:2592-2641`).

Consequence for synthesis Q10. Removing the direct winnow dependency
does not remove winnow from this lockfile. The toml family still needs it.
This supports Deletion lens D2, not a binary-size or runtime-cost claim. [V]

## Question 5: logos in the temporal lexer

Keep logos. One paragraph, with citations. [P]

`src/temporal/lexer.rs:17-54` derives two logos token enums.
`DatePrefixToken` matches `FullDate` and `YearMonth` prefixes. A query or
template value may carry trailing text. The match must therefore stay a
prefix (`src/temporal/lexer.rs:93-111`, `src/temporal/datetime.rs:517-542`).
`TemporalShapeToken` matches six whole-value shapes. Their regexes overlap.
Logos resolves the overlap by longest match. `recognize` then demands one
token that spans all of the input (`src/temporal/datetime.rs:517-525`).
`src/temporal/point.rs:101-122` uses the same lexer to decide date versus
date-time and to classify trailing `T` or space bytes. The same crate
mechanism also drives the note grammar lexer
(`src/note/parser/lexer.rs:116-298`) and the query grammars
(`src/query/grammar/filter.rs:1212`, `src/query/grammar/source.rs:617`).
A hand scanner would have to re-implement longest-match resolution across
six regexes plus exact span bookkeeping. That duplicates frozen grammar
behavior (ticket 16) with no measured gain, and no defect against logos
appears in the b-file ledger.

## Question 6: candidate detail sections

### jiff-english 0.0.1

Path prefix: `.rust-docs/cache/crates/jiff-english/0.0.1/source/`.

- License and deps. MIT. Deps: `jiff = "0.2"` and `scanlex = "0.1"`.
  The `serde` dependency is optional (`Cargo.toml:42-50`). Feature-additive
  note [V]: a probe `cargo tree -e features -i jiff` run confirms that
  jiff-english enables jiff default features. `tz-fat`, `tzdb-zoneinfo`,
  `tzdb-concatenated`, `tzdb-bundle-platform`, and `perf-inline` join the
  unified build. The project disables jiff defaults (`Cargo.toml:128`).
  Final probe linkage adds +81.17 KiB. A separate repository-copy experiment
  adds +48.88 KiB. Neither measures a complete adopted integration.
- Anchor. `parse_date_string(s, now: &Zoned, dialect)` and
  `parse_and_remainder` take the base zone and instant as arguments
  (`src/lib.rs:124-155`). A search over this crate's `src/` finds no
  `now()` call. This bounded search does not prove absence of ambient reads
  through dependencies or conversions. [V]
- `next week`. Rejected. `wee` matches no weekday in `week_day`
  (`src/types.rs:433-447`) and no month in `month_name`
  (`src/types.rs:449-468`), so `parse_date` errors with
  `expected week day or month name` (`src/parser.rs:124`). A bare `N weeks`
  is an interval of days, not a week-floor value (`src/types.rs:470-495`).
  A runtime check confirms the rejection (`probes/candidates`).
- Bare weekday. `friday` resolves to the next Friday. Same-day inputs move
  forward seven days unless a parsed time sorts later than the base time
  (`src/types.rs:119-136`). Result time is midnight when no time part
  parses (`src/types.rs:346-363`). The result is one `Zoned` point.
  A runtime check confirms midnight results and the +7-day same-day move.
- Errors. One message string field lives at `errors.rs:5-34` in
  `DateError`. The type exposes no spans. `parse_and_remainder` returns the
  unparsed remainder (`src/lib.rs:147`), which is text, not a span.
- Locale. English only. `Dialect::Uk` and `Dialect::Us` change numeric
  date order and weekday semantics. With the fixed Saturday anchor,
  `next saturday` gives March 28 under UK and March 21 under US.
- Safety defect [V]. `week_day` slices `&s[0..3]` on scanner identifiers
  (`src/types.rs:433-446`). The scanner accepts non-ASCII letters.
  The expanded run aborts on Hebrew `מחר`. The tracked Unicode sweep
  records 1,234 nonzero exits from 5,514 inputs for this English build.
  The fuzzy build records zero. The earlier CJK and full-width cases
  missed the failing identifier path. The release profile aborts on panic.
  A `Result` wrapper cannot recover from it. Reject this version as
  supplied for user input [P]; see `m-gap-closure.md`.

### fuzzydate 0.4.1

Path prefix: `.rust-docs/cache/crates/fuzzydate/0.4.1/source/`.

- License and deps. MIT. Deps: `chrono 0.4` with `clock`, `lazy_static`
  and `thiserror 2.0`. The `anyhow` and `clap` dependencies are optional
  (`Cargo.toml:48-67`).
  chrono is absent from this lockfile today, so adoption adds a second
  datetime base. [V]
- Anchor. `parse_relative_to(input, relative_to: NaiveDateTime)` accepts
  an injected civil datetime. A naive datetime is not an instant.
  This API resolves through ambient `Local` (`src/lib.rs:251-265`).
  `aware_parse(input, relative_to, tz)` takes an optional anchor and an
  explicit zone. A `None` anchor uses `Local::now`
  (`src/lib.rs:273-290`). `parse` always reads the local clock
  (`src/lib.rs:268-270`). An injected anchor alone does not prove
  a complete absence of ambient inputs.
- `next week`. Accepted as a point. The `UnitRelative(Next, Week)` branch
  adds one week of days to today. The base time of day stays
  (`src/ast.rs:357-370`, `src/ast.rs:768-781`). A runtime check confirms
  the +7-day point with the base time kept (`probes/candidates`).
- Bare weekday. The `Weekday(day)` branch scans forward from today.
  A matching weekday returns today (`src/ast.rs:454-463`).
  An older supposed fuzzydate run actually used the English binary.
  The expanded run uses a separate fuzzy executable. Bare `saturday`
  returns 2026-03-14T12:00Z with the Saturday anchor.
  That result agrees with the inspected source.

  Week arithmetic uses `iso_week` and Monday-start ISO rules
  (`src/ast.rs:374-409`). The inspected path takes no configured week-start
  parameter. Do not generalize that observation to every transitive path.
- Errors. `Error` carries `InvalidDate(String)`, `UnrecognizedToken(String)`
  or a bare `ParseError`. It carries no offsets or spans
  (`src/lib.rs:212-225`). `debug_parse` returns lexemes and the consumed
  token count (`src/lib.rs:296-319`), which is a debug aid, not error spans.
- Locale. English only. The lexer lowercases input and matches a fixed
  keyword table (`src/lexer.rs:250-335`).
- Safety correction [V]. The acceptance run catches a panic on
  `3000000000 days ago`: `NaiveDateTime` subtraction overflows through
  `TimeDelta`. The earlier zero-failure Unicode sweep remains true for
  that sample; it does not establish general input safety (M).

### parse_datetime 0.17.0

Path prefix: `.rust-docs/cache/crates/parse_datetime/0.17.0/source/`.

- License and deps. MIT. Deps: `jiff 0.2.15` with `tz-system`,
  `tzdb-bundle-platform` and `tzdb-zoneinfo`, `num-traits 0.2.19`,
  `winnow 1.0.0` (`Cargo.toml:49-63`). All three names already live in
  this lockfile. [V]
- Anchor. Partial. `parse_datetime_at_date(date: Zoned, input)` injects
  the base (`src/lib.rs:230-235`). Plain `parse_datetime` builds on
  `Zoned::now()` (`src/items/builder.rs:259-264`).
- Ambient reads. The `TZ="..."` input prefix can switch the zone. A
  timestamp item `@N` overrides the result
  (`src/items/mod.rs:200-260`, `src/items/builder.rs:266-270`). These are
  input-driven, not clock reads.
- `next week`. A point. `next` is ordinal 1 (`src/items/ordinal.rs:32`),
  and `week` is 7 days (`src/items/relative.rs:118-122`). A relative item
  alone keeps the current time of day (`src/items/builder.rs:272-286`).
  The result lands 7 days ahead at the base time of day.
- Bare weekday. Forward-only. Today matches today. So `thursday` on a
  Thursday returns midnight today. `next thursday` on a Thursday moves
  one week (`src/items/builder.rs:310-350`). A weekday with no time
  resets the clock to midnight (`builder.rs:312-314`).
- Errors. `ParseDateTimeError::InvalidInput` is the only public variant.
  It drops the internal reason string (`src/lib.rs:140-159`,
  `src/items/error.rs:1-38`). It exposes no spans.
- Locale. English GNU `date` dialect. Items are lowercased with
  `to_ascii_lowercase` before matching (`src/items/mod.rs:240-246`).
  Non-ASCII input keeps its case. It then fails to match.
- Runtime correction [V]. The acceptance run confirms `next week`,
  same-day weekdays and GNU zone/timestamp items. `2024-03-14 x` succeeds
  because `x` is a military zone, not rejected trailing text. No panic
  occurs in its 64 shared cases. That bounded result is not a safety
  proof (M).

### chrono-english 0.2.1

Path prefix: `.rust-docs/cache/crates/chrono-english/0.2.1/source/`.

- License and deps. MIT. Deps: `chrono 0.4` with `alloc`,
  `default-features = false`, and `scanlex 0.2` (`Cargo.toml:37-44`).
  chrono and scanlex are absent from this lockfile. [V]
- Anchor. `parse_date_string(s, now: DateTime<Tz>, dialect)` takes the
  base as an argument (`src/lib.rs:95-123`). The inspected `Utc::now`
  occurrences are in `#[cfg(test)]` code (`src/lib.rs:146-158`).
  This scoped read does not prove absence of ambient inputs in every
  dependency or generic conversion. [V]
- `next week`. Rejected. `wee` fails both name tables, so `parse_date`
  errors with `expected week day or month name`
  (`src/parser.rs:108-142`, `src/types.rs:402-460`).
- Bare weekday. Same rule as jiff-english: next occurrence, same-day
  move decided by the time part (`src/types.rs:102-135`).
- Errors. One message string field lives at `errors.rs:5-34` in
  `DateError`. No spans, no remainder API.
- Locale. English only. `Dialect::Uk` or `Dialect::Us`
  (`src/lib.rs:89-93`).
- Port relation. jiff-english ports this grammar to jiff and adds `eod`,
  `hence` and a 12-hour clock repair (`jiff-english README.md:26-37`).
  The acceptance run independently confirms chrono-english's UTF-8
  slice panic and same-day behavior. Both English parsers turn
  `4294967295 years` into a 2023 result from the 2024 anchor.
  Chrono-english emits unsolicited `fkd date` stderr twice for
  `1000000 years` (M). [V]

## Question 7: rejection tests

These checks now have observed results from
[probes/acceptance](./probes/acceptance), recorded in the M report.
They do not select human semantics. `ERROR`, `OK` and `PANIC` describe
the native API outcomes, not acceptance of the proposed product contract.
The source citations above explain expectations; the observations below
supersede contradictory expectations. [V]

- T1, interval grammar: executed with Thursday 2024-03-14 12:00 UTC.
  English and chrono-english return `ERROR` for `next week`.
  Fuzzydate and GNU return 2024-03-21 12:00 UTC, one point.
  All four lack native period units. A chosen week-period contract
  rejects all four unmodified APIs [P].
- T2, week rule: executed `next friday` from the same anchor.
  English and chrono-english return March 15 under US and March 22
  under UK, at midnight. Fuzzydate returns March 22 noon in both runs;
  GNU returns March 15 midnight in both. Their APIs ignore the harness
  dialect where no dialect parameter exists. None supplies the proposed
  arbitrary frame week-rule interface. The owner still chooses phrase
  semantics; these outcomes do not choose them.
- T3, same-day weekday: executed all seven matching weekday anchors.
  Bare `thursday` from March 14 noon gives March 21 midnight in both
  English parsers, March 14 noon in fuzzydate, and March 14 midnight in
  GNU. This expands, not invalidates, the older Saturday observations.
  The selected product still needs the owner's same-day rule [P].
- T4, midnight gaps and folds: executed with explicit `00:00`.
  Sao_Paulo 2015-10-18 gives 01:00-02 in English/GNU and `ERROR` in
  fuzzydate/chrono-english. Havana 2020-03-08 gives 01:00-04 in
  English/GNU and `ERROR` in the other two. Havana 2020-11-01 gives
  00:00-04 in English/fuzzydate, 00:00-05 in GNU, and `ERROR` in
  chrono-english. GNU retains the base offset in this fold.
  Bare `today` retains the base time; it is not a midnight test.
  `today at 00:00` can error or ignore text; it does not replace
  the successful explicit-time syntax. Compare these observations with
  the adopted conversion rule only after the human selects that rule.
- T5, injected context: executed 36 explicit-anchor/zone host variations
  over `TZ`, `LC_ALL` and `TZDIR`. Every selected result stays unchanged.
  This supports the tested explicit-context paths, not every entry point
  or transitive path. Resolved features still matter: English enables
  Jiff defaults; fuzzydate and chrono-english add Chrono.
  The proposed restricted-feature/no-new-stack gate remains [P].
- T6, malformed input and overflow: all four execute Unicode and huge-count
  cases. English has seven `PANIC` cases, including UTF-8 slicing and
  unchecked Span constructors. Fuzzydate has one, on `3000000000 days ago`
  (`NaiveDateTime`/`TimeDelta` overflow). Chrono-english has two UTF-8
  slice panics. GNU has zero in this sample. Both English parsers wrap
  `4294967295 years` to a 2023 result. The earlier English Unicode-sweep
  count, 1,234 failures from 5,514 inputs, remains bounded history. The
  new panics come from the unwind harness, not the abort-profile builds.
  An abort-profile build terminates; no `Result` wrapper recovers it.
  Reject the three panic-bearing versions as supplied [P].
  GNU's zero is bounded, not proof of universal safety.
- T7, trailing input: executed `10pm meeting` and `2024-03-14 x`.
  Strict English, fuzzydate and chrono-english reject both.
  Lenient English accepts the first and rejects the second.
  GNU rejects the first but accepts the second as military zone `x`,
  producing 2024-03-14 00:00-11:00. The earlier source expectation that
  GNU rejected both was wrong. Embedded `today\u0000junk` errors in
  strict English, fuzzydate and GNU; chrono-english accepts the anchor,
  ignoring the suffix. Full consumption remains a product requirement [P].
- T8, closed grammar: executed `@1758724019` and `TZ="UTC" 2 days`.
  GNU accepts both; the other three reject both. GNU's native grammar
  exceeds the proposed closed grammar. A wrapper must prove restriction
  and diagnostics independently; the unmodified API does not pass [P].
- T9, month-end arithmetic: executed `1 month ago` from
  2024-03-31 12:00 UTC. English/chrono-english return February 29 midnight;
  fuzzydate returns February 29 noon; GNU returns March 2 noon.
  The first three clamp the date in this case; GNU overflows it.
  The accepted clamp rule and the human-selected phrase time policy
  remain separate constraints.
- T10, errors: executed invalid inputs against all four APIs and inspected
  native error types. None provides an error byte span. Message text,
  token text or a line number does not establish a correct query span.
  A selected implementation needs independent span evidence [P].

## Current evidence and remaining limits

The 64 shared definitions, two English lenient cases, four baseline records
and 36 host variations produce 298 JSON records. Every record contains a
native first call and a warmed call with identical outcomes. Candidate
`OK/ERROR/PANIC` counts are English 32/27/7, fuzzydate 29/34/1,
GNU 38/26/0 and chrono-english 32/30/2. The English total includes its two
lenient cases; the others each contain 64 cases. [V: M]

All four build at the repository floor, Rust 1.96.0. GNU declares
`rust-version = "1.71.1"`. Chrono-english declares edition 2024 without
a floor; English and fuzzydate declare edition 2021 without a floor.
These successful builds close the compatibility question for this ticket.
Support for each crate's undocumented minimum is an untestable upstream
promise, not an open test gap. [V: M]

The M tables separate first-call allocations, warmed allocations,
construction, formatting and result drop. Fixtures and zone lookup stay
outside the parse window; native input copies and unwind handling stay
inside. Construction and baseline parse each allocate zero in all records.
These are instrumented research costs, not production throughput. Jiff
0.2.38, jiff-core 0.1.1 and jiff-tzdb 0.1.9 match the pinned set.
Chrono 0.4.45 and harness-only chrono-tz 0.10.4 use its 2025b data.
The latter is not an adopted second product zone database. [V: M]

Remaining work depends on the human-selected implementation. Its gates are
grammar completeness, period units, spans, the actual carrier size/traits,
frame acceptance, integrated costs, format epoch and residual-equivalence
proofs. The narrow speedate replacement has no more datetime, duration or
zone comparison gaps (N). This research passes no implementation gate.

Inaccessible facts and universal limits remain separate. The advertised
English upstream still returns 404; public repository and commit searches
find no alternative. The package records VCS SHA
`a8b87edbeebafebd06a6c4f5a2b502b79b61e409`, not accessible maintenance
history. A 2026-11-17 snapshot recorded 29 downloads; a 2026-11-18 read
records 30. No count proves maintenance quality. No bounded run proves
arbitrary Unicode, overflow, host-state or transitive safety. Earlier
timing and linkage observations remain historical evidence with their
recorded limits. [V: M]
