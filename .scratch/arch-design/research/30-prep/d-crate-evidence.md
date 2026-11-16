# Crate evidence for ticket 30 preparation

Crate-source check for [ticket 30][t30].
The file confirms or rejects the claims in [synthesis.md](./synthesis.md),
questions 9 to 12. This file does not resolve the ticket.

[t30]: ../../issues/30-temporal-boundary-and-dialects.md

Every claim carries a tag. [V] marks a read of named source. [H] marks a
hypothesis that still needs a check. [P] marks a proposal for the grilling.
A later parent-run round measured jiff-english and fuzzydate at runtime.
See `m-gap-closure.md` and `probes/` for the sources, results, and limits.
Compile time and repository-wide deltas stay unmeasured.

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
| New crate in lockfile | no; scanlex absent today | yes; chrono absent today | no; jiff, winnow, num-traits present | yes; chrono absent today |
| Injected anchor | yes; `now: &Zoned` argument (`lib.rs:124-128`) | partial; `parse_relative_to` naive, `aware_parse` optional anchor | partial; `parse_datetime_at_date(Zoned, ...)` (`lib.rs:230-235`) | yes; `now: DateTime<Tz>` argument (`lib.rs:95-99`) |
| Ambient clock or zone reads | none; `grep` over `src/` finds no `now()` call [V] | `parse` reads `Local::now` (`lib.rs:268-270`); `parse_relative_to` resolves through `Local` (`lib.rs:259-262`) | plain `parse_datetime` uses `Zoned::now` (`builder.rs:263`) | none; `Utc::now` appears only in `#[cfg(test)]` code (`lib.rs:146-158`) [V] |
| `next week` | error; `wee` fails weekday and month tables (`types.rs:433-446`) [runtime: confirmed] | point; today's weekday plus one week (`ast.rs:357-370`) [runtime: confirmed] | point; now plus 7 days, time of day kept (`ordinal.rs:32`, `relative.rs:118-122`) | error; same tables as jiff-english (`types.rs:402-460`) |
| Bare weekday | midnight; next occurrence, never today, base time kept without a time part (`types.rs:119-137`) [runtime: confirmed] | today when today matches; else next match; base time kept (`ast.rs:454-463`) [runtime contradicts the today case; see the fuzzydate section] | midnight when today matches; else forward match (`builder.rs:311-349`) | same rule as jiff-english (`types.rs:102-135`) |
| Error spans | none; message string only (`errors.rs:5-24`) | none; token text only (`lib.rs:212-225`) | none; public `InvalidInput` drops detail (`lib.rs:140-159`) | none; message string only (`errors.rs:5-24`) |
| Locale | English only; `Dialect` switches day and month order | English only | English only; GNU date dialect, ASCII lowercasing (`mod.rs:240-246`) | English only; `Dialect` as jiff-english |

All four candidates return one point value, never an interval. [V]
Point types: `jiff::Zoned`, chrono `NaiveDateTime` or `DateTime<Tz>`,
`parse_datetime::ParsedDateTime` and chrono `DateTime<Tz>`.
Measured for jiff-english and fuzzydate: parse runs 215 to 588 ns
(`m-gap-closure.md`). Probe binary cost adds +65.0 KiB and +48.9 KiB
over a 409 KiB baseline. Unmeasured: compile time, repository deltas,
chrono-english, parse_datetime.

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

Check against the owner note. `owner-evidence.md` says the project builds
with default features [H there]. `Cargo.toml:128` sets
`default-features = false` with explicit `tz-system`. The owner conclusion,
a five-minute TTL plus one global lock, stands. The feature spelling above
is the confirmed one. [V]

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
Answer: `parse_bytes_rfc3339` accepts exactly one shape. `from_str` accepts
a strict superset of that shape. No counterexample input is known in either
direction on the strict 10-byte shape. The differential probe has not run.
Probe-level claims stay [H]. The source reading is [V].

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

Consequence for synthesis Q10. Deleting the direct winnow dependency
cannot drop winnow from the lockfile, because the toml family keeps it.
This confirms Deletion lens D2. [V]

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
  Probe cost with the unification: +65.0 KiB over the 409 KiB baseline.
- Anchor. `parse_date_string(s, now: &Zoned, dialect)` and
  `parse_and_remainder` take the base zone and instant as an argument
  (`src/lib.rs:124-155`). No routine reads an ambient clock or zone.
  A grep over `src/` finds no `now()` call. [V]
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
- Locale. English only. `Dialect::Uk` or `Dialect::Us` switches day and
  month order (`src/lib.rs:109-113`, `src/parser.rs:71`).
- Risk note [V]. `week_day` slices `&s[0..3]` on scanner identifiers.
  scanlex accepts non-ASCII alphabetic identifier characters
  (scanlex 0.1.4 `src/lib.rs:291-292`). A three-byte-or-longer identifier
  whose third byte is not a char boundary can panic there. The runtime
  matrix fed `週` and full-width digits; both failed cleanly with no
  panic. The identifier path itself (`ééé`) stays unexecuted.

### fuzzydate 0.4.1

Path prefix: `.rust-docs/cache/crates/fuzzydate/0.4.1/source/`.

- License and deps. MIT. Deps: `chrono 0.4` with `clock`, `lazy_static`
  and `thiserror 2.0`. The `anyhow` and `clap` dependencies are optional
  (`Cargo.toml:48-67`).
  chrono is absent from this lockfile today, so adoption adds a second
  datetime base. [V]
- Anchor. Partial. `parse_relative_to(input, relative_to: NaiveDateTime)`
  accepts an injected naive instant but resolves it through `Local`
  (`src/lib.rs:251-265`). `aware_parse(input, relative_to, tz)` takes an
  optional anchor and a zone. A `None` anchor falls back to `Local::now`
  (`src/lib.rs:273-290`). `parse` always reads the ambient local clock
  (`src/lib.rs:268-270`).
- `next week`. Accepted as a point. The `UnitRelative(Next, Week)` branch
  adds one week of days to today. The base time of day stays
  (`src/ast.rs:357-370`, `src/ast.rs:768-781`). A runtime check confirms
  the +7-day point with the base time kept (`probes/candidates`).
- Bare weekday. `Weekday(day)` scans forward from today. Today matches
  today. So `thursday` on a Thursday returns today
  (`src/ast.rs:454-463`). Week arithmetic uses `iso_week`, so the week
  rule is Monday-start ISO and ignores a configured week start
  (`src/ast.rs:374-409`). A runtime check contradicts the today case:
  `saturday` on Saturday 2026-03-14 returned 2026-03-21T00:00, not
  today at the base time. Re-verify the same-day rule at runtime
  before any adoption.
- Errors. `Error` carries `InvalidDate(String)`, `UnrecognizedToken(String)`
  or a bare `ParseError`. It carries no offsets or spans
  (`src/lib.rs:212-225`). `debug_parse` returns lexemes and the consumed
  token count (`src/lib.rs:296-319`), which is a debug aid, not error spans.
- Locale. English only. The lexer lowercases input and matches a fixed
  keyword table (`src/lexer.rs:250-335`).

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

### chrono-english 0.2.1

Path prefix: `.rust-docs/cache/crates/chrono-english/0.2.1/source/`.

- License and deps. MIT. Deps: `chrono 0.4` with `alloc`,
  `default-features = false`, and `scanlex 0.2` (`Cargo.toml:37-44`).
  chrono and scanlex are absent from this lockfile. [V]
- Anchor. `parse_date_string(s, now: DateTime<Tz>, dialect)` takes the
  base as an argument (`src/lib.rs:95-123`). `Utc::now` appears only in
  `#[cfg(test)]` code (`src/lib.rs:146-158`). The library reads no
  ambient clock. [V]
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
  `hence` and a 12-hour clock fix (`jiff-english README.md:26-37`). Same
  grammar means the same rejection tests apply to both. [V]

## Question 7: rejection tests

Run these later by hand or as unit tests. A candidate fails a test when
the shown behavior differs from the stated expectation. Each test names
one accept-or-reject decision. Anchors are explicit values, never ambient.
T1, T3, T6, and T10 later gained partial runtime evidence; their notes
carry it.

- T1, interval grammar, all candidates. Feed `next week` with base
  Thursday 2024-03-14 12:00 UTC. jiff-english and chrono-english error.
  fuzzydate returns 2024-03-21 12:00 local. parse_datetime returns now
  plus 7 days at the base time of day. Reject a candidate for the value
  layer if `next week` must denote a week period. (Runtime: jiff-english
  error and fuzzydate point confirmed on a 2026-03-14 base.)
- T2, week rule. Feed `next friday` with base Thursday 2024-03-14.
  jiff-english returns 2024-03-22 under `Dialect::Uk`. It returns
  2024-03-15 under `Dialect::Us`. parse_datetime returns 2024-03-15.
  fuzzydate returns 2024-03-22, Monday-start ISO week. Reject a candidate
  whose rule cannot obey the frame `WeekRule` and `WeekStart` inputs.
- T3, same-day weekday. Feed the bare weekday that matches the base day,
  for example `thursday` on Thursday 2024-03-14 12:00. jiff-english and
  chrono-english return 2024-03-21 00:00. parse_datetime returns
  2024-03-14 00:00. fuzzydate returns 2024-03-14 12:00. Reject a
  candidate whose same-day rule differs from the note-model rule.
  (Runtime: jiff-english confirmed. Fuzzydate moved forward and reset
  to midnight for the tested day, so its stated expectation needs
  re-verification.)
- T4, midnight gap. Pick a zone and a date whose local midnight falls in
  a gap, for example America/Sao_Paulo 2015-10-18. Feed `today`.
  jiff-english maps through jiff `Compatible` and lands at 01:00
  (`jiff zoned.rs:1104-1118`). Reject any candidate that reports a
  different instant for the same civil day or cannot state its mapping.
- T5, injected anchor only. For each candidate, build with the project
  feature line: jiff with `default-features = false`, and no chrono.
  For fuzzydate and plain parse_datetime, call only the anchor-taking
  entry points. Then read the dependency list. Reject any candidate that
  still needs an ambient clock on that path, or that re-enables jiff
  default features (`Cargo.toml:128` conflict).
- T6, panics on non-ASCII names. Feed `ééé` or `next ééé` to jiff-english
  and chrono-english. A byte-slice panic at the `&s[0..3]` tables rejects
  the candidate for user-visible values unless it fixes the slice.
  (Runtime: `週` and full-width digits failed cleanly with no panic;
  the `ééé` identifier path stays unexecuted.)
- T7, trailing input. Feed `10pm meeting` and `2024-03-14 x`. jiff-english
  accepts the first in lenient mode and offers `parse_strict`
  (`src/lib.rs:157-166`). fuzzydate and parse_datetime reject both.
  Reject a candidate that cannot run in strict mode for frozen values.
- T8, closed grammar. Feed `@1758724019` and `TZ="UTC" 2 days` to
  parse_datetime. Both parse. Reject parse_datetime for the closed note
  grammar if any input-driven zone or timestamp item must stay rejected.
- T9, month-end overflow. Feed `1 month ago` with base 2024-03-31.
  parse_datetime reproduces GNU overflow behavior
  (`src/items/builder.rs:362-381`). jiff-english clamps through jiff
  span arithmetic (`src/types.rs:234-243`). Reject a candidate whose
  result contradicts the accepted calendar-arithmetic rule.
- T10, error spans. For each candidate, feed a bad input and inspect the
  error type. No candidate exposes a span. Reject the error-design part
  of any candidate that must report reason spans. Such a candidate needs
  a wrapper that recovers spans. (Runtime: both leading candidates
  expose no spans, as stated.)

## Not checked

- Compile time, repository-wide size deltas, and startup cost for any
  adoption. Probe sizes and warm-cache link times are not repository
  measurements (`m-gap-closure.md`).
- Parse run time for `chrono-english` and `parse_datetime`. The two
  leading candidates are measured; these two stay source-only.
- The datetime scenarios of the old differential matrix. The in-repo
  parity probe covers 4,635,370 guarded date inputs with zero
  differences, but datetimes, spans, and zones stay outside it.
- `chrono` feature resolution beyond the probe tree. The probe shows
  fuzzydate adds `chrono 0.4.45` with `clock` and no `chrono-tz`; the
  jiff tree stays untouched.
- Repository `Cargo.lock` claims. They rest on read lockfile entries;
  the probe tree ran against the probe lockfile only.
- Issue-tracker state and commit activity for both upstream projects.
  The crates.io API check on 2026-11-17 gave release and download
  facts (`m-gap-closure.md`); repository reads stayed open.
- `time 0.3.55` sources in the cache. Not read. No question in this
  file requires it. `chrono 0.4.45` was built in the probe, not read.
- Behavior of the candidates under locales other than English. Not
  tested. All four are English-only by source reading, and the runtime
  matrix used English inputs.
