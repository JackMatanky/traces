# e: External design precedents for ticket 30

Research note for
[issue 30](../../issues/30-temporal-boundary-and-dialects.md) ("Where do
temporal, format dialects and calendar helpers belong?").
Prepared for the owner's grilling. This file gives evidence and
counterexamples from outside the repository. It does not resolve the ticket.

- Researcher: temporal external-designs reviewer.
- Date: 2026-11-16.
- Repair pass: 2026-11-17. K-round citation fixes and the runtime probe
  facts (`m-gap-closure.md`).
- Source rev of this research: brief at
  [`brief.md`](./brief.md) (rev `971bdd5729d9d5f51a1bd6fed46bab6947df936a`).
- Prose: ASD-STE100 style. Sentences stay short. Approved verbs: use, check,
  change, need, get, make sure.

## How to read this file

- `[V]` = a linked source read or a recorded probe result.
  Each citation identifies the source or the historical run.
- `[V-repo]` = ticket 30 records a verified code read on main @ `cf6d79bd`.
  I did not re-read the file. I cite the ticket.
- `[H]` = hypothesis or unverified claim. It needs a check before the owner
  treats it as evidence.
- `[P]` = proposal for the grilling. Not a decision.
- No measurement comes from this writing pass. Later recorded probe
  results appear with their limits. Unmeasured costs remain unmeasured.
- Each proposed option needs a counterexample and a rejection condition.
  A missing check remains an evidence gap, not support for the option.
  These arguments prepare the grilling. They do not decide it.

## Sources

Primary sources, read on 2026-11-16 unless noted:

| Key | Source |
| --- | --- |
| S1 | CLDR/UTS#35 Date/Time Rules, <https://unicode.org/reports/tr35/tr35-dates.html> (CLDR 49) |
| S2 | TC39 Temporal docs: `plaindate`, `plaindatetime`, `zoneddatetime`, `duration`, `plainyearmonth`, `timezone`, <https://tc39.es/proposal-temporal/docs/> |
| S3 | RFC 3339, <https://www.rfc-editor.org/rfc/rfc3339> |
| S4 | RFC 5545 (iCalendar), <https://www.rfc-editor.org/rfc/rfc5545> |
| S5 | PostgreSQL 18 docs: functions-datetime, datatype-datetime, sql-expressions, <https://www.postgresql.org/docs/18/> |
| S6 | IANA tzdb 2026f: theory.html, australasia, northamerica, <https://data.iana.org/time-zones/tzdb-2026f/> |
| S7 | Moment.js docs, <https://momentjs.com/docs/> |
| S8 | Luxon docs (parsing, formatting), <https://moment.github.io/luxon/#/> |
| S9 | Dataview docs: literals, functions, types of metadata, metadata pages, <https://blacksmithgu.github.io/obsidian-dataview/> |
| S10 | Obsidian Tasks docs, Filters, <https://github.com/obsidian-tasks-group/obsidian-tasks/blob/main/docs/Queries/Filters.md> |
| S11 | POSIX.1-2024 strftime, tzset, <https://pubs.opengroup.org/onlinepubs/9799919799.2024edition/functions/strftime.html> |
| S12 | CommonMark 0.31.2 spec, <https://spec.commonmark.org/0.31.2/> |
| S13 | RFC 3986, <https://www.rfc-editor.org/rfc/rfc3986> |
| S14 | java.time SE 25 javadoc: package summary, `WeekFields`, `TemporalAdjusters`, <https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/time/package-summary.html> |
| S15 | Allen, "Maintaining knowledge about temporal intervals", CACM 26(11):832-843, doi 10.1145/182.358434; relation table via CRAN ArchaeoPhases vignette `allen.html` |
| S16 | Apache Arrow Columnar format, <https://arrow.apache.org/docs/format/Columnar.html>; pyarrow `strftime`, <https://arrow.apache.org/docs/python/generated/pyarrow.compute.strftime.html> |
| S17 | NumPy 2.5 datetimes, <https://numpy.org/doc/2.5/reference/arrays.datetime.html> |
| S18 | `crontab(5)`, Linux man-pages, <https://man7.org/linux/man-pages/man5/crontab.5.html> |
| S19 | `jiff` 0.2.38 crate docs (`fmt::temporal`, `fmt::strtime`, `RoundMode`), read from the local `.rust-docs` cache |
| S20 | `chrono-english` 0.2.1 crate docs, read from the local `.rust-docs` cache |
| S21 | W3C XSD 1.1 Part 2: Datatypes, <https://www.w3.org/TR/xmlschema11-2/> |
| S22 | Candidate metadata and API evidence recorded in ticket 30 and `d-crate-evidence.md`; crates.io snapshots on 2026-11-16 and 2026-11-17 |
| S23 | Duckling (facebook/duckling) README, <https://github.com/facebook/duckling> |
| S24 | In-repo probe runs: `probes/parity`, `probes/parity/tests/layout.rs`, `probes/candidates`, `probes/moment`; results recorded in `m-gap-closure.md` (2026-11-17) |

Gaps at the end of this file list what these sources do not cover.

---

## A. Moment compatibility and format dialects (q1 to q5)

### q1. Is Moment compatibility a domain concept?

- [V] Templater delegates all formatting to Obsidian's runtime `moment`
  object and translates nothing (research 07, recorded in ticket 30).
- [V] Lowercase Moment tokens (`w ww gggg`) use locale week rules.
  Uppercase tokens (`W WW GGGG`) use ISO rules. Locale data supplies
  `dow` and `doy` for the lowercase family (S7; K1).
- [V] Moment's strict-mode guide documents a forgiving ISO parse of
  `5917238b-33ff-f849-cd63-80f4c9b37d0c` as `5917-08-26`.
  The example uses `moment.ISO_8601`; `YYYYDDD` describes the accidental
  shape, not the example's API call (S7; K23).
- [V] `minijinja-contrib` date filters use strftime only. They hold no Moment
  logic [V-repo, research 07].

Counterexample against implicit locale policy: the same template can
produce different week numbers after a locale change. A fixed, explicit
week rule removes this device-locale dependence. The example supports an
adapter boundary, not a unique module home.

Counterexample against dropping Moment tokens now: the owner's vault templates
write `ww`, `YY`, `Q`, `Qo`, and the Fileclass data uses `YYYY`, `DDDD`, `Q`
[V-repo]. A cold drop breaks those templates.

[P] Treat Moment syntax as a compatibility adapter. Keep domain values
dialect-free. If schema checks also accept Moment, identify a shared
adapter consumer. That need does not by itself decide the module home.
Reject the adapter boundary if it leaks dialect tokens into domain values
or cannot preserve the adopted compatibility surface [P].

### q2. Where does the Moment dialect translator live?

Landed state: `pattern_to_strftime`, `MOMENT_TOKENS`, and the ordinal helper
sit in `src/temporal/token.rs`. A `%` outside brackets means strftime;
otherwise the code translates Moment tokens. An unknown token returns
`UnsupportedToken` [V-repo].

- [V] `jiff` supports strftime-like parsing and formatting only, in
  `fmt::strtime`. Its docs say the goal is not to match POSIX exactly
  (S19).
- [V] The owner's Moment tokens include `ww`, `Q`, and `Qo`.
  Lowercase `ww` needs a locale-style week rule.
  Jiff gives ISO `%V`/`%G` and format-only `%q` for quarters.
  Its `%U`/`%W` use week zero. They do not replace a general locale
  week-year rule (S19; K20).
- [V] The candidate date-parsing APIs carry no format-token interface
  (`d-crate-evidence.md`, candidate sections).

The options:

1. Keep as is (translator in `src/temporal`). Cost of a change-table row:
   none today. Counterexample against: the translator must translate week
   tokens. Week translation needs the `DateConfig` week rule. The template
   dialect then leaks into temporal's core API.
2. Return it to a template adapter (research 07's proposal). Counterexample
   against: the query surface accepts a pattern argument in `date(text,
   pattern)` today [V-repo]. If only the template adapter translates, the
   query pattern stays strftime-only, and the dialect split needs an explicit
   rule (q3).
3. Drop Moment tokens. Ticket 16 marks this Migratable. Counterexample: the
   owner's templates fail until they change.

[P] Option 2, with option 1 as the recorded keep-as-is. Rejecting condition
for option 2: a query-side need for Moment tokens that the owner confirms.

### q3. The dialect for each of four format surfaces

The surfaces: config `format`, query `date(text, pattern)`, template
`date_format`, schema `date.format`.

Facts per surface:

- Config: `[frontmatter.date_created]` and `[frontmatter.date_modified]`
  default to `%Y-%m-%dT%H:%M:%S`. No production code reads them [V-repo,
  ticket 22].
- Query: `date(text, pattern)` calls `DatePoint::parse_with_pattern`, which
  accepts strftime only [V-repo].
- Template: `date_format` speaks Moment through the translator [V-repo].
- Schema: `SchemaDateField.format` is an unvalidated `Option<String>`. Tests
  use Moment tokens [V-repo]. Fileclass lints the tokens of the declared
  format, not the values [V-repo, research 14].

External evidence:

- [V] The inspected PostgreSQL and Jiff interfaces document their format
  rules. PostgreSQL preserves ISO input across `DateStyle` modes.
  Non-ISO input depends on `MDY`, `DMY`, or `YMD` (S5).
  Jiff's `fmt::strtime` supplies parsing and formatting directives (S19).
  These examples do not prove an ecosystem-wide dialect rule.
- [V-repo] `pattern_to_strftime` selects strftime when it finds `%` outside
  brackets. Otherwise it translates Moment tokens.
  Thus `YYYY` selects Moment and `%Y` selects strftime.
  Jiff's format-only quarter directive is `%q`.
  `%Q` means a zone identifier or offset, not a quarter (S19; K20).
- [V] Dataview accepts two grammars by argument shape: `date(text)` auto-parses
  ISO and `date(text, format)` uses Luxon tokens (S9).

Counterexample against "one dialect, Moment": the config default and the
query parser already speak strftime. A switch means a change-table row for
config and query. `%Y-%m-%d` configs break.

Counterexample against "one dialect, strftime": the schema `date.format`
values in the owner's data (`YYYY`, `DDDD`, `Q`) are Moment. A strftime-only
check must translate or reject them.

[P] One dialect, strftime (jiff `fmt::strtime`), for config, query, and
schema value checks. One Moment translator at the template seam only.
Schema `date.format` keeps Moment tokens as a write hint if and only if the
owner confirms the split. Otherwise it joins strftime. Rejecting condition:
an owner-confirmed need for Moment tokens in queries or config.

### q4. `date.format` values with no full date (`YYYY`, `Q`, `DDDD`)

- [V] The owner's Fileclass data declares exactly these partial formats
  [V-repo].
- [V] Fileclass bypasses pattern validation for a string `dateFormat`.
  That bypass does not enforce a string type or a non-empty value.
  The outer validator handles empty values and `required`
  [V-repo, research 14; K8 in `k-citation-verification.md`].
- [V] Mainstream value types for month- and year-only data: Temporal ships
  `PlainYearMonth` and `PlainMonthDay` (no plain-year type), java.time
  ships `Year` and `YearMonth`, and XSD has `gYear`/`gYearMonth` (S2, S14,
  S21). The `PlainYearMonth` docs warn that a constructor with a
  non-default reference day makes two values of the same month unequal
  (S2). Partial precision needs a normalized representation.
- [V] Moment parse semantics: `Q` sets the month to the first month of the
  quarter; `DDD`/`DDDD` read day-of-year 1 to 366 in leap years (S7). A
  value check can classify each token, but the class of `Q` is "quarter
  selector", not "full date".
- [V] Ticket 20 hands over a truncated sentence ("Only date, datetime
  and") inside its [Date-shaped-text discussion](../../issues/20-schema-note-typing-and-validation.md),
  lines 305-315. Ticket 20 already delegates the value check to the
  date-format check, and B26's accepted rule accepts complete scalars
  provisionally. The fragment does not gate the check in this file.
  Ticket 36 keeps it as a recovery item (`i-leakage-review.md`, I1).

Counterexample against "check every `date.format` against a full date": a
field declared `DDDD` holds `152`. That is a valid value of its declared
format and an invalid full date. A full-date check rejects good notes.

Counterexample against "no check at all": the current state already has no
check, and `SchemaDateField.format` is dead config [V-repo]. An unvalidated
format silently emits wrong dates when a writer uses it.

[P] Classify format tokens at schema load. Separate full-date, partial-date,
and no-date-component formats. Define a value check for each adopted
category. Ticket 20's provisional scalar rule applies only when no date
component exists (L2). Its unrelated truncated fragment does not block
the delegated date-format check (L1).

### q5. `format` under `[frontmatter.date_created]` and `[frontmatter.date_modified]`

- [V] Ticket 22 records that no production code reads these keys. The
  defaults are `%Y-%m-%dT%H:%M:%S` (strftime).
- [V] A configured input format with day-first or month-first order is a
  hazard PG documents: `01/02/03` re-reads per `DateStyle` mode (S5).
- [V] RFC 3339 section 5.1 states conditions for timestamp string order:
  use the same offset, zone spelling, fractional-second digit count,
  and punctuation form (S3). This rule covers complete timestamps.
  It does not establish a format rule for arbitrary stored values.

Options: keep the keys where they are, move them into the new `[date]`
table, or delete them.

Counterexample against "keep as is": two homes for date settings
(`frontmatter.*` and a new `[date]`) with no reader today. The merge rule of
ticket 22 then needs a case for both homes.

[P] Move the two keys into `[date]` as `created_name`, `created_format`,
`modified_name`, `modified_format`, or fold them into the `DateConfig`
field list (q27). Delete nothing silently: ticket 16 makes config format
changes Migratable, which still needs a change-table row.

---

## B. Calendar helpers and filenames (q6 to q8)

### q6. Which calendar helpers stay in `src/template/engine/date.rs`?

`date.rs` holds `DateOps`, `Anchor`, `apply_offset`, `weekday`, and the
`date_*` filters. About 880 of 2,512 lines are production code [V-repo].

- [V] `PERIOD_EDGE_LOOKUP` and `resolve_period_edge` already live in
  `src/temporal/calendar.rs`, and query functions and template filters share
  them [V-repo]. A temporal-owned calendar edge helper is the landed
  pattern.
- [V] java.time externalizes adjustment: `TemporalAdjusters` are static
  policy objects, and the value types carry the arithmetic
  (`plus`, `minus`, `with`). The docs name the pattern: adjusters "exist to
  externalize the process of adjustment" (S14).
- [V] Jiff puts arithmetic on the types (`Span`, `round`, `Zoned::round`)
  and keeps no anchor of its own. Callers supply the zone and the relative
  anchor (S19).
- [V] The template `date.weekday(n)` picks ISO weekday `n` of the current
  week. That is a week-start rule read, not a render-only concern
  [V-repo].

Counterexample against independent anchor selection: a query and template
can select different clocks. Keeping a template wrapper does not cause
that failure if both surfaces use the same pinned frame.

Counterexample against moving the `date_*` filters into temporal: filters
are a template API surface. Ticket 16 freezes template capabilities but the
Rust library surface is free.

[P] Move arithmetic (`apply_offset` and the calendar-series logic it needs)
into temporal, next to `resolve_period_edge`. Keep filter registration in
the template engine. `Anchor` is removable once the frame of ticket 21
supplies `today` (q33). `weekday` moves with the week rule into temporal.

### q7. Where does the filename-period parser live?

- [V-repo] Today `FileMeta::derive_day` reads the base name with
  `DatePoint::parse_prefix` and falls back to the creation time.
  `scan_date_prefix` in `src/temporal/lexer.rs` sees `YYYY-MM-DD` and
  `YYYY-MM` only. The note parser also calls `parse_prefix`. Ticket 16
  freezes that parsing.
- [V] Dataview's `extractDate` searches for dashed or compact full dates
  anywhere in the name. `findDate` checks typed metadata first.
  That helper can also inspect link path, subpath, and display text
  (S9; K15). The inspected extraction helper recognizes no week or quarter
  labels. This limited read does not establish ecosystem-wide absence.
- [V] NumPy's year-granularity `'2005'` equals `'2005-01-01'` (S17).
  That cross-unit equality says nothing about malformed-prefix fallback.
- [V] RFC 3339's ordering rule covers complete same-shape timestamps,
  not arbitrary filename suffixes (S3). Prefix separators and fallback
  rules need their own grammar.

Ticket 21 adds validated ISO weeks, quarters, and an opt-in bare year, and
scopes the new parser to filenames.

Options for the home:

1. In temporal, next to `DatePoint` and the existing prefix scan. The
   parser can share date and week validation. No contrary dependency
   example appears in this read set.
2. In the file scanner. It can call temporal without owning the grammar.
   Duplicating prefix and week logic would create two grammar owners.
3. In the note parser. A shared parser must preserve frozen note meaning,
   except for explicitly accepted changes (L3).
   Filename forms do not authorize new note syntax.

[P] Put the filename-period parser in temporal. Queries derive `file.date`
from its normalized start and unit (ticket 24).
`file.cdate` instead derives from creation time in the pinned zone.
Do not make `file.cdate` consume a filename prefix.

### q8. The week-one rule for a custom week start

- [V] CLDR defines numbered weeks with `firstDay` and `minDays`.
  Territory `001` uses Monday and 1 day. The US uses Sunday and 1 day.
  Many European territories use Monday and 4 days.
  A first week needs at least `minDays` days in that year (S1).
  The rollover note concerns a previously valid week 53 made unavailable
  by changed week settings. It does not define every strict ISO parser
  (K4).
- [V] Moment encodes the same rule as `week: {dow, doy}` with
  `doy = 7 + dow - janX`, where `janX` is the first day of January that
  belongs to week 1. Presets: ISO (dow 1, doy 4, janX 4) and US (dow 0,
  doy 6, janX 1). Other presets: a common Arab one (dow 6, doy 12, janX 1)
  and (dow 1, doy 7, janX 1) (S7).
- [V] java.time `WeekFields` takes `(firstDayOfWeek, minimalDaysInFirstWeek)`
  and separates "week of year" (with a week 0) from "week-based-year" (no
  week 0; week 1 owns its days). Its table shows `2009-01-01` as week 1 of
  2009 with min-days 4, and week 53 of 2008 with min-days 5 (S14).
- [V] POSIX `%U`/`%W` are week-0 schemes keyed to Sunday/Monday starts.
  `%V` is the ISO scheme: week 1 holds January 4 and the first Thursday
  (S11).
- [V] NumPy's `W` unit is an epoch-anchored 7-day span with no week-one
  rule; it is not Monday-keyed (S17). A week unit without a week-one rule
  silently disagrees with ISO `W`.
- [V] Obsidian Tasks pins ISO weeks only, Monday to Sunday, and tracks the
  request for other week starts as an open issue (S10).

Counterexample against "first-day setting alone is enough": with first day
Monday, min-days 4 and min-days 5 put `2009-01-01` in different years.
Week 1 of 2009 versus week 53 of 2008 (S14 week table). A
`week_start = monday` config does not say which.

Counterexample for rollover policy: ISO `2025-W53` is invalid.
CLDR's changed-settings note permits rollover in its stated context.
Ticket 21 instead accepts strict ISO filename validation without rollover.
Do not generalize the CLDR note to every invalid typed week.

[P] Consider a configurable `(first_day, min_days)` pair for general
week operations, with ISO `(monday, 4)` as the proposed default.
The two fields need not change together.
Preserve ISO filename prefixes independently of general settings.
A non-ISO filename spelling needs a separate decision (L10).
Reject a general-week setting that silently changes accepted ISO filename
labels or cannot represent the adopted week-one rule [P].

---

## C. Parsing stack (q9 to q12)

### q9. Does `speedate` stay?

- [V-repo] One call site, in `src/temporal/date.rs`.
- [V] Ticket 25 fingerprints the resolved versions of `jiff` and `speedate`.
  Removing the crate forces one rebuild (ticket 30, freshness input).
- [V] Jiff's `fmt::temporal` parses a hybrid of RFC 3339, RFC 9557, and ISO
  8601, including offsets, zone annotations, and the space separator (S19).
  The one call site's shapes need a parity check.
- [V] Jiff rejects ISO forms it does not support, for example
  `2024-06-16T10.5` (fractional hours) (S19). A second RFC 3339 parser in
  the same binary can accept strings the first rejects.

Counterexample against keeping it: two RFC 3339 grammars can disagree on
edge forms, and the fingerprint carries both crates forever.

Counterexample against removal without parity evidence: a broader Jiff
entry point could change frozen acceptance. The later probe reports
4,635,370 guarded date inputs with zero acceptance, value, or error-category
differences (`probes/parity`, S24). The guard admits only the ten-byte date
shape. Raw Jiff also accepts compact `20260101`, naive datetimes, and
negative years.

[P] Remove speedate only with the existing guard and diagnostic behavior
preserved. Cite the bounded parity evidence.
Do not claim that this probe covers datetimes, spans, zones, or unguarded
Jiff input.

### q10. Does `winnow` stay?

- [V-repo] One call site, in `src/temporal/lexer.rs`. Ticket 30's freshness
  input says `winnow` locates duration prefixes for inline fields, and those
  values reach the store. Ticket 25's crate list omits `winnow`.
- [V] The accepted duration shapes are frozen vault-format parsing (ticket
  16). Jiff's `Span` grammar allows weeks mixed with other units and
  fractional lowest units (S19). RFC 5545 allows weeks only alone and
  forbids years and months (S4). None of these grammars is the frozen one.
- [V] `winnow` is a parser-combinator library. A combinator in one call site
  is a dependency for one grammar.

Counterexample against direct replacement: the frozen duration-prefix
grammar needs a consumed boundary. Jiff's public friendly parser accepts
only complete input and reports no consumed prefix
(`d-crate-evidence.md`, Question 2).

[P] Keep `winnow` for that boundary role until ticket 35 makes the crate cut.
Ticket 36 records ticket 25's omission, or the direct use goes under a
separate grammar-parity argument. A hand-written replacement needs a grammar
specification first. Removing the direct dependency does not remove transitive
`winnow` from this lockfile (d, Question 4).

### q11. Does `logos` stay as part of the parsing base?

- [V-repo] `logos` serves `lexer.rs`, `datetime.rs`, and `point.rs` in
  temporal, plus the shared lexer, the note parser, and the query grammar.
- [V] Logos builds a deterministic automaton from token definitions
  (K20 in `k-citation-verification.md`). That lexical property does not
  prove deterministic callbacks, clock inputs, or relative-date resolution.
- [H] Binary-size effect under `opt-level = "z"`: not measured.

Counterexample against removal: temporal, note, and query lexers already
use Logos. Replacing those consumers needs grammar-parity evidence.
The listed files do not establish exactly four independent lexers.

[P] Keep Logos as the lexing base. Consider reuse if the human adopts a
closed NL grammar. Reuse does not prove parser-wide determinism.

### q12. Does `jiff` stay as part of the parsing base?

- [V-repo] Pinned `=0.2.38` with features `std`, `tz-system`,
  `tzdb-bundle-always`.
- [V] Jiff gives: the Temporal ISO grammar parser (`fmt::temporal`),
  strftime-like `fmt::strtime`, civil/Timestamp/Zoned type split, spans with
  calendar units, a rounding mode, and a bundled or system tz database
  (S19).
- [V] Jiff's strtime gives no localization and no `%+`. `%q` is format-only;
  `E`/`O` POSIX modifiers are unsupported (S19).
- [V] Jiff's parser is type-directed: parsing a `civil::DateTime` from a
  string with `Z` is an error "since interpreting such strings as civil time
  is usually a bug" (S19). This matches the repo rule: `Z` or offset input
  is an instant, naive input is civil.
- [V] `Zoned` parsing defaults: `disambiguation = compatible` (earlier in a
  backward transition, later in a forward one) and `offset_conflict` errors
  by default (S19).
- [V] Jiff's year range tops out at 9999, smaller than Temporal's (S19).
  Note dates in a notes tool sit far inside that range.
- [H] Binary size with `tzdb-bundle-always`: not measured. The feature
  bundles tzdb data in the binary. Reproducibility holds only if the build
  pins the bundled data version and ignores the system database. State the
  selected data source and version before relying on that claim (S19). The
  bundled database updates with crate releases. The extractor fingerprint
  already changes with jiff versions.

Counterexample against removal: temporal needs a zone database with history
and skipped-day correctness (q21 counterexamples come from tzdb data that
jiff bundles). Re-creating that is a project on its own.

Counterexample against keep-as-is without a check: jiff 0.2.38 differs from
POSIX strftime (stated goal, S19). Any place Traces documents "strftime"
semantics needs the strtime table, not POSIX, as the contract.

[P] Keep jiff as the parsing base. Document `fmt::strtime` as the format
dialect of record (q3) and set the parser defaults explicitly where a
`Zoned` parse can occur.

---

## D. Natural-language dates (q13 to q18)

### q13. The grammar

- [V] Ticket 19 proposes a closed English list: keywords, weekdays, offsets
  such as `in 2 weeks`, resolved from injected `today`, shared by filter
  literals, `date()`, `date_add`, and template filters.
- [V] `chrono-english` states the closed-list position in its crate docs:
  it makes no try at full natural language parsing. It supports only a
  limited set of patterns (S20).
- [V] Duckling's example server accepts `reftime`.
  It puts that value in `Context.referenceTime`.
  Without that parameter, it uses the current reference time.
  Its corpus fixes the reference at 2013-02-12 at 04:30.
  Results carry a grain, and rules have positive and negative examples
  (S23; K17).
- [V] Obsidian Tasks uses the flexible parser `chrono` for filter dates and
  warns users against two adjacent word-dates in one range because flexible
  parsing makes the range ambiguous. It recommends `YYYY-MM-DD` (S10).
- [V] Dataview's relative literals are a fixed keyword set
  (`today`, `now`, `tomorrow`, `yesterday`, `sow`, `eow`, `som`, `eom`,
  `soy`, `eoy`) (S9).

Counterexample against unpinned flexible parsing: Tasks documents
ambiguous word-date ranges (S10). A broad grammar can replay consistently
when all relevant inputs are pinned. A closed grammar can still read
an ambient clock. Grammar size does not prove determinism (K17).

[P] Keep ticket 19's hook-only alternative. If the human adopts a grammar,
consider a closed list with pinned inputs. Put positive and negative
examples in the accepted grammar table.

### q14. Build in the repository or use a crate?

Candidates per ticket 30: `jiff-english`, `fuzzydate`. The prep items the
owner needs, with what the evidence gives:

| Item | `jiff-english` 0.0.1 | `fuzzydate` 0.4.1 | in-repo |
| --- | --- | --- | --- |
| Licence, releases | MIT; 1 release (2026-08-07); 29 downloads (crates.io, 2026-11-17) [V, S22, S24] | MIT; 14 releases; last update 2026-04-05 (crates.io, 2026-11-17) [V, S22, S24] | n/a |
| Size | ~1.3k lines [V, S22] | ~2.6k lines [V, S22] | [H] not written yet |
| Dependencies | `jiff ^0.2` default features + `scanlex ^0.1`; probe feature unification adds `tz-fat`, tzdb bundles, `perf-inline` [V, S22, S24] | `chrono 0.4.45` with `clock`, `lazy_static`, `thiserror`; inspected probe tree leaves Jiff unchanged [V, S22, S24] | no new parser crate proposed [P]; actual dependencies unmeasured |
| Determinism and safety | injected-anchor sample; 1,234 Unicode-sweep failures from 5,514 inputs; reject as supplied [P, S24] | injected-anchor sample; zero failures in the same sweep; no complete transitive audit [V, S24] | pinned-input contract proposed [P]; no implementation proof |
| Week rules | US/UK `next saturday` differs; bare same-day Saturday moves +7 days [V, S24] | bare same-day Saturday retains the anchor; `last friday` gives 2026-03-06 [V, S24] | phrase rules need q15/q16; q8 alone does not decide them |
| Output type | point; `next week` rejected [V, S24] | point; `next week` gives +7 days [V, S24] | `CalendarPeriod` proposed for adopted period phrases [P] |
| Coverage | no `next week`, `eom`, or `in 2 weeks`; lenient `start of month` gives midnight today [V, S24] | no `eom` or `in 2 weeks`; `next week` is a point [V, S24] | chosen list only if implemented and checked [P]; no demonstrated coverage |
| Coupling with jiff `=0.2.38` | enables jiff defaults in probe [V, S24] | adds a chrono stack in probe [V, S24] | Jiff-based implementation proposed [P] |
| Final probe binary delta | +81.17 KiB, including feature unification [V, S24] | +98.11 KiB, including chrono [V, S24] | not measured |
| Repository linkage delta | +48.88 KiB; reachable-call experiment, not complete integration [V, S24] | +65.39 KiB; same experiment [V, S24] | not implemented |
| Parse-time medians | 264/582/372 ns for tomorrow/next monday/typo; indicative [V, S24] | 376/597/223 ns for the same inputs; indicative [V, S24] | not measured |
| Compatibility | edition 2021; no `rust-version`; builds with stable 1.92.0 and 1.98.0 [V, S24] | same stable build evidence; no minimum-version claim [V, S24] | repository floor 1.96; proposed parser unbuilt |
| Maintenance evidence | advertised repository API returns 404; search finds no named repository; quality unknown [V, S24] | repository and open-issue snapshot checked; no maintenance-quality guarantee [V, S24] | repository would own changes [P] |

The expanded matrix now tests both candidates' same-day weekday case.
The Unicode failures confirm English's byte-slice defect.
The sample does not prove exhaustive Unicode or overflow safety.
The M report gives timing ranges and all repeated repository build times.
These are indicative observations, not controlled or production benchmarks.
The absence of `rust-version` establishes no minimum supported version.

Evidence from the closest comparable crate, `chrono-english` 0.2.1 (S20):

- It takes a `DateTime<Tz>` anchor. That API supports injection.
  The source read does not prove absence of ambient reads through every
  generic conversion or dependency (S20; K17).
- Its separate `parse_duration` API returns
  `Interval::Seconds | Days | Months`. That is a duration result.
  The inspected candidate date APIs return points, not periods
  (S20; K17; d). Period phrases remain an adoption choice.
- Its weekday semantics are dialect-split: US `next friday` = the coming
  Friday. UK `next friday` = Friday of next week. A repo grammar fixes one
  meaning.
- Its short-year pivot is 1940 to 2040. A repo grammar can choose the same
  or another pivot, but must state it.

Counterexample against `fuzzydate`: it depends on `chrono` with the `clock`
feature. `Cargo.toml` has no `chrono` entry today [V, S22]. Adding it puts a
second calendar implementation next to jiff, with different week and
arithmetic defaults, inside a codebase that just pinned jiff.

Counterexample against adoption based on popularity: jiff-english has one
release on 2026-08-07 and 29 recorded downloads. The 2026-11-17 API
snapshot still shows 29 (S24). Those counts do not prove maintenance
quality or justify calling the crate a prototype.

[P] Prefer an in-repository grammar if the human adopts period phrases
and the candidate APIs cannot meet that contract. Both candidates take
injected anchors. Fuzzydate's naive-anchor API still uses ambient `Local`;
its aware API accepts an explicit zone. Neither candidate returns
`(start, unit)` periods. Jiff-english rejects `next week`
(S24; d, candidate sections). These gaps support the proposal, not a
completed adoption decision.

### q15. The meaning of a bare weekday

The three candidates: next, nearest, or the day in the current week.

Evidence, one library per meaning:

- [V] Nearest: Obsidian Tasks. "if it is Wednesday and you write tuesday,
  Tasks assumes you mean yesterday, as that is the closest Tuesday" (S10).
- [V] Next (with dialect split): `chrono-english`. "friday means this
  coming Friday"; "next Friday" in the US means the coming Friday, else it
  means Friday of next week (S20).
- [V] Explicit four-way, no default: java.time names four adjusters,
  `next`, `nextOrSame`, `previous`, `previousOrSame` (S14). The library
  refuses to bake in one meaning.
- [V] Current-week: Traces' own template filter `date.weekday(n)` selects
  ISO weekday `n` of the current week [V-repo].
- [V] No weekday resolution at all: cron's day-of-week field matches the
  day itself, with no relative meaning (S18).

Counterexample against an unstated nearest rule: on Wednesday, bare
`tuesday` can mean yesterday. A reader who expects next Tuesday interprets
`due > tuesday` differently (S10). This is an expectation mismatch,
not proof that nearest selection is non-monotonic.

Counterexample against "current week" in queries: on Sunday, ISO current
week runs Monday to Sunday, so a bare `monday` on Sunday lands six days
back. The template filter keeps that meaning today. A query meaning that
differs from the template meaning needs a documented split.

Counterexample against "next" as the only meaning: Tasks users document the
nearest expectation. Adopting `next` silently changes answers for users who
come from that habit. Ticket 19 asks to fix exactly this fork.

[P] Define `next <weekday>` and `last <weekday>` explicitly if adopted.
Consider strictly-after `today` for `next`. Choose a bare-weekday meaning
separately. The evidence does not select one safe default.
A `DateConfig` setting is an option, not a requirement.

### q16. Interval precision of phrases such as `next week`

- [V-repo] Tickets 19 and 21 distinguish periods from arithmetic points.
  If the human adopts period phrases, the grammar needs period-valued
  results. Grammar adoption itself remains open (L11).
- [V] Dataview's week keywords are edge points: `sow`/`eow` give the start
  or end of the current week (S9). They are points that imply a span only
  when paired.
- [V] Tasks converts `next week` and friends into an inclusive absolute
  date range internally, and supports only the exact keywords `last`,
  `this`, `next` over `week`, `month`, `quarter`, `year` (S10).
- [V] java.time splits `Period` (y/m/d) from `Duration` (time) and gives
  week-based years their own unit (S14). Jiff's `Span` keeps calendar units
  and requires a relative anchor to resolve them (S19). RFC 5545 splits
  nominal durations (weeks, days) from exact ones (S4).
- [V] Month arithmetic clamps. `chrono-english` states: "adding a month to
  30 Jan will give 28 Feb or 29 Feb depending if a leap year" (S20). PG
  clamps March 31 + 1 month to April 30 (S5). Temporal offers
  `constrain`/`reject` (S2).

Counterexample against a point-only period phrase: `file.date = next week`
does not identify one day. Tasks uses a range (S10).
If Traces adopts that phrase, the proposal needs a period result.

Counterexample against shifting a raw label as a period start:
`2027-01-30` is not a normalized month start. Adding a month can clamp
that point, but it does not define "next month". A normalized month period
starts on the first day. Keep point-shift overflow policy separate from
period construction.

[P] For adopted period phrases, return normalized `CalendarPeriod`
values. Derive the target period from the anchor and unit.
Do not use month-clamp examples as proof of a period-normalization rule.

### q17. Filter spelling of relative keywords such as `today` and `eom`

- [V-repo] The query grammar has `sow`, `eom`, `eoy` as functions that take
  a date argument. It has no relative keyword. A bare word parses as a
  field path, so a bare `today` names a field today [V-repo].
- [V] Dataview spells relatives as function calls in a literal:
  `date(today)`, `date(sow)`, `date(eom)`, `date(soy)` (S9).
- [V] Tasks spells relatives as bare keywords (`due today`) because its
  grammar has no bare-field-path rule. Fields are always `field:`-prefixed
  (S10).
- [V] PG's special literals `today`, `tomorrow`, `yesterday`, `now` are
  read-time shorthands, and its docs warn: in prepared statements, views,
  and function bodies they become stale fixed values; prefer the functions
  `CURRENT_DATE` and friends (S5).

Counterexample against bare keywords in Traces: a bare `today` collides
with the field-path rule that the grammar already has [V-repo]. Tasks can
afford bare keywords because its grammar differs.

Counterexample against argument-less `eom()` only: `eom` is month-relative.
Dataview's `eom` means end of the current month. Traces' `eom(date)`
parameterizes the month, which is strictly more general and matches the
landed function form.

[P] Consider `today()`, `eom()`, and `eom(date)`.
The proposed no-argument forms bind against the frame at plan time.
This is the q17 function alternative, not the existing grammar.
Dataview-style `date(today)` remains a competing spelling.
Reject a spelling that captures existing field paths or binds relative
values outside the plan frame [P]. Explicit edge arguments remain valid.

### q18. The frontend spelling of `member_of`

- [V-repo] Ticket 29 reserves only the IR name. The first pushdown scope
  covers unquoted literals and `And`.
- [V] Tasks' `happens` checks start, scheduled, and due dates against a
  range (S10; K18). It does not check every task date.
- [V] Dataview spells containment as `contains()` on lists (S9).
- [V] PG spells array containment as `@>` and set membership as `IN`
  (S5).
- [V] Ticket 21: membership is separate from equality, grouping, and
  ordering. Period membership tests the civil date of the instant in the
  pinned zone.

Syntax risk: Tasks uses `in` for ranges, but that spelling does not prove
an ambiguity in Traces. A Traces grammar must define its own productions
and relation (K18).

[P] Compare `member_of(...)` or `within` with keeping only the reserved
IR name. Do not infer a required frontend implementation.
If the human adopts syntax, distinguish a matching alias from a new
relation. The evidence does not choose the spelling.

---

## E. Value shapes (q19 to q25)

### q19. Type shape for civil and instant values

- [V] java.time separates `LocalDate`, `LocalTime`, `LocalDateTime`,
  `ZonedDateTime`, `Instant`, and `OffsetDateTime`.
  Its package documentation identifies time-zone complexity.
  It recommends `Offset*` types for network and database protocols (S14).
  That recommendation does not prove how every store represents zones.
- [V] Jiff: separate types `civil::Date`, `civil::DateTime`, `Timestamp`,
  `Zoned`, with explicit conversions (S19).
- [V] Temporal: `PlainDate`/`PlainDateTime` (civil) versus `Instant` versus
  `ZonedDateTime`, with documented conversion traps: `toPlainDateTime()`
  drops the offset, so re-zoning can land on the other side of a fold (S2).
- [V] PG ships two distinct SQL types (`timestamp without time zone`,
  `timestamp with time zone`) whose input parsing is type-directed and
  permissive. The docs record the costs: a `without time zone` value
  silently ignores an offset in its input, and the stored zone is never
  retained (S5).
- [V] NumPy: one naive type. Offset-bearing input parse is deprecated
  (S17).
- [V-repo] `NoteFieldValue` must stay at 80 bytes. `DateValue` is 4 bytes,
  `DateTimeValue` 16, `DurationValue` 72 (research 11).

Counterexample against permissive offset handling: PostgreSQL's
without-zone type ignores an input offset (S5; K13).
PostgreSQL exposes distinct SQL types, not one public mode-flag type.
That parser policy does not prove that a Rust inner enum is unsafe.

Counterexample against assuming variant cost: every variant contributes
to the same enum maximum. Separate variants do not each consume another
80-byte slot. Both proposed Rust shapes need actual layout evidence.

[P] Compare an inner temporal enum with separate runtime variants.
Keep civil datetimes distinct from instants in either shape.
The later default-test-profile mirror measures 80 bytes, not the proposed
product types. Its `LinkType` and duration substitutions limit that result
(S24; `m-gap-closure.md`).

Reject either product shape if it exceeds the 80-byte slot budget [P].
If the inner enum exceeds that limit, compare the separate-variant shape.
Do not assume that the alternative fits without its own product assertion.

### q20. Where `CalendarPeriod` sits in the value model

- [V-repo] Ticket 18 lists no period variant in `NoteFieldValue`. Tickets
  19 and 21 need period values for `file.date` and filter literals. Ticket
  19 puts `CalendarPeriod` in the temporal brackets. Ticket 21 defines it
  as a normalized civil start plus unit.
- [V] Temporal's `PlainYearMonth` supplies a month-value precedent.
  A non-default reference day can affect equality in its constructor.
  This supports canonical representation, not a general week/quarter
  period model (S2; K5).
- [V] Arrow encodes month, day-time, and month-day-nano intervals (S16).
  Those units describe relative quantities, not periods with start dates.
  The encoding analogy does not establish Traces period layout, equality,
  or ordering (K12).
- [V-repo] Ticket 24 derives filename periods during query evaluation.
  Ticket 29 uses `(start, unit)` for period group identity.
  Operator-specific membership bounds remain a separate contract (L5).

Counterexample against raw labels: storing only `2026-W01` leaves later
code to repeat normalization. Ticket 21 already fixes this prefix as ISO.
A general configured week rule must not reinterpret it.

Counterexample against a persisted period column: the store keeps temporal
values as text and derives filename periods at query time. A persisted
period column needs an `IndexPolicy` entry and a fingerprint input
(ticket 25).

[P] Complete ticket 18's single runtime carrier for notes, queries and
templates. Place the normalized period in that carrier without another
period-value type. A separate query carrier needs an explicit amendment
(L4). Runtime representation does not itself authorize period syntax in
stored note fields.

### q21. Date compared with an instant at an ambiguous or absent local midnight

Ticket 21 already fixes period-versus-instant membership through the
instant's civil date in the pinned zone. A skipped next day does not
invalidate the preceding day. This question separately asks about an
ordinary Date field versus an instant. Membership does not decide that
ordinary field comparison.

- [V] java.time makes `LocalDate` and `Instant` incomparable at the type
  level. The package doc's design notes explain the type split (S14).
- [V] Temporal uses specific conversion rules, not a universal cross-kind
  rejection. `PlainDate.compare` accepts `PlainDateTime` through
  `PlainDate.from`, which drops its time (S2; K6).
- [V] PostgreSQL compares `date` to `timestamptz` with midnight in its
  `TimeZone` setting. Its Denver 2005-04-03 example distinguishes
  `+ interval '1 day'` from `+ interval '24 hours'` (S5; K13).
  Traces instead needs an explicit pinned zone. That boundary contrast
  does not itself decide gap/fold resolution.
- [V] tzdb shows the counterexamples that force the rule:
  - Skipped day: Samoa's 2011-12-30 does not exist. The zone switches
    standard offsets from -11 to +13 at 2011-12-29 24:00 local. With the
    DST rules in force on both sides, the observed shift is -10:00 to
    +14:00 (S6, australasia).
  - Ambiguous midnight: Cuba's rules put transitions at standard-time
    midnight (`0:00s`), so the fall-back hour 00:00-00:59 occurs twice and
    the spring-forward night skips local midnight. The 2026f release
    suspends the usual November 2026 return, so any worked example must
    name its year (S6, northamerica).
- [V] Jiff's parser is type-directed about this: a `Z` string into a civil
  type is an error (S19).

Counterexample against compare-by-local-midnight: in Havana on a
spring-forward night, local midnight does not exist as an instant. In a
fall-back night it exists twice. Any rule that needs the midnight instant
is undefined exactly when it matters (S6).

Counterexample against "always project both sides to civil dates": a civil
date and an instant are different classes. Ticket 29 keeps civil values and
instants in separate classes for grouping. Projection erases the class
boundary that grouping needs.

[P] Preserve ticket 21's period-membership rule. Choose the ordinary
Date-versus-instant gap/fold rule separately. Ticket 19 uses the pinned
zone for civil-versus-instant comparison. Group-class separation does not
imply cross-class filter failure. A blanket no-match rule would change
that contract, not apply it (ticket 19, Answer).

### q22. The total sort key for mixed temporal values

- [V] RFC 3339 section 5.1 orders timestamp components from least to most
  precise. Its string-order rule also needs identical offsets, zone
  spelling, fractional digit counts, and punctuation (S3; K10).
- [V] Temporal's `Duration.compare` needs `relativeTo` when any
  years/months/weeks component is nonzero, and throws `RangeError`
  otherwise. It refuses to order mixed-precision nominal values without an
  anchor (S2).
- [V] `PlainYearMonth.compare` sorts by the ISO first day of the month
  (S2). Dataview compares across its types in `compareValue`, with dates by
  epoch, rather than sorting text (S9; c Q22).
- [V-repo] Ticket 29 section 3 fixes the group class order (Civil,
  Instant, Period; B65/C3). This question keeps the mixed-value sort key
  and the unit tie-break.
- [V] XSD 1.1 durations are partially ordered: the spec checks `P1M`
  against `P30D` on four reference datetimes, and the order flips between
  February 1697 and March 1903 (S21; confirmed in the K round,
  `k-citation-verification.md`, K11).

Counterexample against treating periods as instants: a month or day period
does not designate one instant. The value-sort key needs an explicit
convention. XSD's anchor-dependent duration order is a separate example
of why physical comparison and product sorting need distinct contracts.

Limit of written-text sorting: `2026-02-28` sorts before `2026-03`.
That pair is not a reversed-order counterexample.
The RFC theorem covers complete timestamps, not partial periods or a
mixed-class total key (K10).

[P] Consider `(class rank, class-local value, unit tie-break)`.
Preserve the accepted Civil, Instant, Period group order.
State the full value-sort key and its relation to group keys (L6).
One proposal sorts civil components together, instants by epoch, and
periods by `(start, unit)`.
For periods, consider Day < Week < Month < Quarter < Year.
The human must choose this rank. The source analogies do not prove it.

### q23. Nominal duration equality and order

- [V] Ticket 19 kept nominal equality and order provisionally.
- [V] RFC 5545 excludes year and month designators.
  It still calls weeks and days nominal durations.
  It permits weeks alone, or days and time, or time alone (S4; K7).
- [V] java.time distinguishes `Period` from `Duration` (S14).
  `chrono-english` parses `Interval::Seconds | Days | Months` (S20).
  This output distinction does not decide Traces equality or order.
- [V] Dataview uses a fixed Luxon conversion matrix (S9; K9).
  Fileclass's input parser permits mixed weeks and ignores a captured
  sign. Its fixed-day milliseconds are not the RFC's calendar rule
  [V-repo, research 14; K8].

Counterexample against numeric equality across nominal and exact units: one
month is 28 to 31 days depending on the month. XSD's own `P1M` versus
`P30D` checks flip by reference date (S21). Any fixed conversion decides
the flip by convention, not by fact.

Counterexample against physical order without an anchor: Temporal throws
when asked (S2). PG clamps month arithmetic and documents the asymmetry
(S5).

[P] Compare same-unit equality and class-based order with retaining ticket
19's nominal seconds (`1mo == 30d`, `1y == 365d`).
Same-unit-only equality changes that provisional rule. It needs an
explicit human decision and supersession record. Keep comparison
conventions separate from calendar arithmetic.

### q24. How the documents describe calendar arithmetic

- [V] PG documents the one-day-versus-24-hours difference with a worked
  example and a normative sentence (S5). Moment's "The Guide" does the same
  with its own example (S7).
- [V] RFC 5545 states the exact-duration rule: at a DST discontinuity, add
  or subtract the discontinuity. Do not count leap seconds (S4).
- [V] Jiff's `Zoned` arithmetic respects the zone. Civil arithmetic does
  not see zones (S19).

[P] Document civil arithmetic and zoned arithmetic separately.
Civil operations need no zone. Zoned calendar units can cross offset
changes. Exact elapsed units need a different rule.

Use the Denver DST case and the Apia skipped day to explain that split.
State leap-second treatment. Do not impose a frame on pure civil
arithmetic merely because zoned arithmetic needs one.

### q25. How `date(link)` reads a Markdown link target

Ticket 19 accepts last-segment target extraction and a pattern argument.
The later Markdown-link handoff leaves extension normalization and
percent decoding open (ticket 19, Markdown-link handoff).
External URLs are not note targets.

- [V] RFC 3986 §2.4: encode data octets that conflict with delimiters.
  Decoding an unreserved octet is safe and equivalence-preserving. Decoding
  an octet that stands for a reserved character changes the URI. The RFC
  says: "do not percent-encode or decode the same string more than once"
  (S13).
- [V] CommonMark 0.31.2: a bare link destination may hold balanced,
  unescaped parentheses; unbalanced ones need escaping, or the `<...>`
  angle form, which allows spaces and any escaped character. The spec does
  not mandate percent-encoding of non-ASCII by renderers. It notes that
  some contexts must percent-encode (S12).
- [V-repo] `scan_date_prefix` accepts `YYYY-MM-DD` and `YYYY-MM` prefixes;
  the pattern argument of `date()` accepts a strftime format [V-repo].

Counterexample against repeated decoding: decoding `%2541` once yields
the literal `%41`. A second decode changes it to `A`.
RFC 3986 forbids repeated decoding (S13).
The name `2026-01%20notes.md` separately illustrates a single encoded space.

Open extension case: `2026-05-01.review.md` contains multiple dot groups.
The human must specify segment extraction, extension treatment, and
pattern matching. Do not present "keep the extension" as an accepted
Markdown-link rule.

Counterexample for external URLs: `date(https://x/2026-01-01)` must not
match. Ticket 19 says external URLs are not note targets. The value is
Null. The notice policy belongs to q36.

[P] Identify URI components and the target's last segment first.
Then decode that segment once.
Do not let decoded `%2F`, `%23`, or `%3F` create new component boundaries
(S13; K22). Choose the encoded-separator and extension rules explicitly.
Non-note targets yield `Null`. Preserve the malformed-prefix rule.

---

## F. Configuration and zone (q26 to q32)

### q26. `[date]` syntax and validation

- [V-repo] Ticket 22: config owns and parses `[date]` into `DateConfig`;
  `deny_unknown_fields` stays, so the field set is exact. Merge is global
  then local, field by field. No `version` key.
- [V] PG keeps date display/input policy in named settings (`DateStyle`,
  `TimeZone`) with enum validation, and its docs show the failure mode of
  free-form date order settings (`MDY` versus `DMY`) (S5).
- [V-repo] The current `SchemaDateField.format` is unvalidated, and its
  accessor exists only under `#[cfg(test)]`. That is the cautionary state.

Counterexample against delayed format checking: a malformed format can
fail far from its declaration. A config-load token check can report its
key path. This does not prove that runtime zone acquisition must occur
at load.

[P] Use an exact config key set (q27). Specify each field's validation
stage. Keep zone spelling checks separate from required zone acquisition.
Preserve inherited merge and trust rules.

### q27. The fields of `DateConfig`

Candidate fields, each with its evidence:

- Default date and datetime formats: today's config default is
  `%Y-%m-%dT%H:%M:%S` [V-repo]. Keep strftime spelling (q3).
- Week rule: `(first_day, min_days)` (q8).
- Weekday resolution for relative phrases: the q15 output, one enum value.
- NL options, only if the human adopts them (q13).
  A grammar version enters the extractor fingerprint only when it changes
  persisted facts (ticket 25). Filename persistence alone does not prove
  that the NL keyword list produces those facts.
- Zone source, under q29 and ticket 22's precedence.
- Bare-year filename opt-in (q32).
  Ticket 21 already accepts week and quarter prefixes.
  Optional switches that disable them need a separate amendment.

Counterexample against an unspecified keyword list: user-defined words can
collide with field paths or named grammar rules. An extensible grammar
needs explicit conflict and migration rules.
An open list alone does not prove nondeterminism.

[P] Choose only fields with a named consumer. State defaults for an absent
`[date]` table. Those defaults must implement accepted changes, not claim
exact parity with today's retired `file.day` behavior.

### q28. Which temporal-owned policy receives the fields without importing `config`

- [V-repo] `config` already reads `DEFAULT_DATETIME_PATTERN` from temporal.
  The stated edge direction is config to temporal [V-repo].
- [V] Jiff shapes this as caller-side configuration objects
  (`DateTimeParser::disambiguation`, `SpanRound` options): the temporal
  crate exposes configurators, the caller fills them (S19). java.time's
  `WeekFields.of(firstDay, minDays)` is the same shape: policy types live
  with the temporal library, values come from the caller (S14).

Counterexample against temporal importing config: ticket 35 may cut
temporal into its own crate. A config dependency blocks that cut.

[P] Temporal defines a `DatePolicy` (formats, week rule, weekday
resolution, NL options). Config maps `DateConfig` into `DatePolicy` at
load. The edge stays one-way. The frame boundary stays a human choice (q33).

### q29. Zone syntax and validation

- [V] IANA zone names: `AREA/LOCATION`, ASCII letters with `.`, `-`, `_`;
  no digits; no case-only-distinct names; legacy spellings live as links in
  `backward`; `Etc/UTC` and `GMT` exist; `Etc/Unknown` is reserved by CLDR.
  The naming rules are guidelines with legacy exceptions (`Etc/GMT0`,
  `GMT0`, `EST5EDT`) (S6 theory).
- [V] PostgreSQL datetime input accepts IANA names, abbreviations, and
  POSIX-style strings with different meanings.
  Its `TimeZone` setting does not accept abbreviations.
  POSIX `UTC-4` means ISO +04 (S5; K13).
- [V] Jiff parses IANA names and fixed offsets (S19). Temporal
  canonicalizes identifiers and errors on unknown ones (S2).

Counterexample against accepting POSIX strings: `EST5EDT` is a POSIX rule
string. `EST` is an abbreviation with a date-dependent meaning. `UTC-4`
reverses sign relative to ISO. Three spellings, three meanings, one config
key: that is the documented PG trap (S5).

Counterexample against no validation: an identifier absent from the
selected database causes a lookup failure. The design must state whether
that failure occurs at config load or first required zone acquisition.

[P] Consider IANA database names and explicit fixed offsets (`±HH:MM`).
Specify treatment of legacy aliases separately from POSIX rule strings.
Choose spelling validation and actual acquisition timing separately.
State the zone-data source and version before claiming reproducibility.
Preserve lazy acquisition for civil-only work (L9; K21).
Reject a validation path that acquires a zone for civil-only work or
silently interprets a POSIX string as a fixed offset [P].

### q30. How the host gets the system zone and reports failure

- [V-repo] Ticket 22 fixes precedence: host override, then config, then
  system. Ticket 21 fixes memoized per-frame failure.
- [V] POSIX `tzset`: with `TZ` unset, the default zone is
  implementation-defined. Portable programs should call `tzset`
  explicitly before `localtime_r` (S11). The host side owns acquisition;
  the library owns conversion.
- [V] tzdb theory: the database is not sufficient for accurate handling of
  all times before 1970, and the `backzone` file is less reliable (S6).
  System-database and bundled-database versions can differ, which changes
  answers near historical edges.
- [V] Jiff's `TimeZone::try_system` reports discovery or disabled-feature
  errors. `TimeZone::system` can fall back to `Etc/Unknown`, which behaves
  like UTC (d, `TimeZone::try_system`; S19; K20).
  The frame must retain a required acquisition error.
  The convenience fallback does not satisfy ticket 21.

Counterexample against reading the system zone per row: PG's
`transaction_timestamp` note (one consistent notion of current time per
transaction) is the analogous rule for clocks. Per-row zone acquisition is
the zone-side version of the same nondeterminism (S5). Ticket 21 already
bans it.

[P] The host reads the system zone once per frame through jiff, on first
need. Config can override it. A failure is typed and memoized for the
frame. It surfaces as the zone-failure error ticket 34 defines.
Deterministic fixtures inject the zone.

### q31. Is the week start configurable?

Evidence is in q8. The added points:

- [V] CLDR territory `001` defaults to Monday with 1 day, not Sunday. The
  US convention differs from the CLDR default (S1). An English-only tool
  still needs to choose.
- [V] Tasks shipped ISO-only and users file requests for other week starts
  (S10). Configurability is a real request, not a hypothetical.
- [V-repo] The owner's week template displays `day(0)`, Sunday, first.
  Moment `day(0)` always means Sunday (K1).
  This display order does not prove a preferred week-numbering rule.

Counterexample against treating Sunday plus four minimum days as invalid:
CLDR lists that pair for `is` and `pt` (S1; K4).
A configurable first day does not force both fields to change.
Display-week starts and week-numbering rules can be separate policies.

[P] Compare ISO-only operation with a configurable pair.
Name the operations that use any custom rule.
Keep accepted ISO filename prefixes unchanged.
Fingerprint a rule only when that rule changes persisted facts.

### q32. The opt-in key for bare-year filename prefixes

- [V-repo] Ticket 21: opt-in bare year, and no reinterpretation of invalid
  longer prefixes.
- [V] Dataview's filename date rule never takes a bare year (S9). NumPy
  does accept year-only values by default (S17). The opt-in guard is the
  middle path.
- [V] Tasks supports a bare `YYYY` in numbered ranges (S10), so bare years
  are a familiar spelling in this tool family.

Counterexample for opt-out default: filenames like `1998.md` or
`2026-review.md` are common for non-date reasons. A default-on bare year
gives every such note a `file.date` and silently changes sorts and filters.

Counterexample for no validation: a five-digit prefix (`20261-thing`) is
not a year. The parser must reject, not truncate.

[P] Use a boolean key, `[date] filename_year = false`.
Validate the config value as a boolean.
Separately validate a filename year as exactly four digits in range.
Reject malformed longer prefixes without truncation.
Nested `filename_periods.year` is a competing syntax choice.

---

## G. Boundary after the execution-context decision (q33 and q34)

### q33. What temporal exports to the execution frame

- [V-repo] Ticket 21: one frame per query, render, or request pins the
  clock and the week rule. The frame acquires the zone only when needed and
  memoizes a failure. UTC only if explicitly selected.
- [V] java.time models this as an injectable `Clock` (with a fixed variant
  for tests) plus explicit `ZoneId` arguments (S14).
- [V] Duckling's `reftime` parameter supplies `Context.referenceTime`.
  Its corpus fixes a reference time for replay (S23; K17).
- [V] Tasks' relative dates recompute from `today` and the maintainers
  document re-evaluation and staleness pitfalls for long-lived results
  (S10, issue #1289 recorded there).

Counterexample for a frame without the week rule: `sow()` binds at plan
time. A frame with a clock but no week rule cannot compute it. Ticket 21
requires the pin.

Counterexample for a frame without a zone resolver: civil-to-instant
projection needs the zone on demand. A memoized failure is part of the
frame's determinism (ticket 21).

[P] Keep one clock sample, the week rule, policy, and a lazy zone resolver
in the operation frame. Derive the civil `today` only when needed.
Temporal exports value operations and policy inputs.
Query and template retain ownership of their operation frames.
The exact frame constructor boundary remains a human choice.

### q34. Replacements for ambient `local_zone()`, `now_instant()`, `ZoneGuard`

- [V-repo] Ticket 21 decides their removal.
- [V] PG keeps three "current time" functions with different scopes
  (`transaction_timestamp`, `statement_timestamp`, `clock_timestamp`) and
  documents why: one consistent notion of current time per transaction
  (S5). The scope distinction is the evidence that an ambient read has a
  scope. Traces' scope is the frame.
- [V] java.time: ambient clock access exists (`Clock.system...`), and the
  library offers explicit clock and zone overloads where the design needs
  injection. Tests use `Clock.fixed` (S14).

Counterexample for keeping any ambient read: two queries in one render
frame could observe different clocks or zones across the removal boundary.
Ticket 21's memoization rule exists to prevent exactly that.

[P] Give state-dependent functions explicit frame inputs.
Keep pure civil operations free of unnecessary frame parameters.
Build fixed frames in fixtures. Remove the obsolete ambient helpers.

---

## H. Query pushdown and notices (q35 and q36)

### q35. Temporal probe bounds per operator and value kind

- [V-repo] Ticket 24 needs a proof per operator, row kind, temporal frame,
  and field precedence. Unproved mappings scan.
  Ticket 29's first temporal scope uses unquoted literals and `And`.
  R5 permits operator-specific point-period membership bounds.
  The reserved `member_of` node remains unsupported (L5).
- [V] PG's expression evaluator does not define subexpression evaluation
  order, and the planner may reorganize boolean expressions by Boolean
  algebra (S5). A pushdown bound must stay sound if the engine reorders or
  drops predicates.
- [V] Temporal needs anchors for nominal duration comparison (S2).
  That duration rule does not define Traces point-period filter order.
  The q22 total sort key is also separate from filter matching.

Proof case: `mdate < 2026-03` compares an evaluated field with a month
period. Use the field's actual value kind, the operator's matching rule,
and the pinned frame. Do not substitute the total-sort class rank.
The proof must remain sound under predicate reordering.

Membership case: instant-to-period matching uses the pinned zone.
That dependency needs an operator-specific bound proof.
It does not prove that an instant key can never supply sound candidate
bounds. R5 anticipates such proofs without supplying them.

[P] Give the Answer a table of value kinds and existing operators.
Each cell states a proved bound or `Unsupported`.
Keep `member_of` unsupported unless the human separately adopts its
semantics and implementation. Never claim a physical-key contradiction
from group equality alone (L5).

### q36. When a row with a `Null` date gets a notice

- [V-repo] Ticket 34 owns the notice mechanism. Ticket 29 did not adopt the
  first-round notice.
- [V] Dataview's `file.day` is silently absent when the file name has no
  date. No notice, no error (S9). That is the silent-Null baseline.
- [V] Fileclass bypasses pattern validation for a string custom format.
  The outer validator checks empty values and `required`.
  A malformed non-empty date string can bypass the pattern check
  [V-repo, research 14; K8 in `k-citation-verification.md`].

Counterexample against noticing every `Null`: ordinary undated notes
would produce notices despite having no malformed date input.

Counterexample against noticing nothing: `2026-13-40-notes.md` has a
malformed date-shaped prefix. A silent `Null` conceals that defect.
Ticket 19's invalid-quoted-literal advisory is an analogy, not an already
accepted filename-notice rule.

[P] Distinguish these cases:

- Return `Null` without a notice for an expected absent value.
- Propose one notice per malformed declared value or date-shaped prefix.
  Attach its span. Ticket 34 must define delivery and deduplication.
- Return a typed error for required zone-acquisition failure.

Ticket 34 owns notice delivery and deduplication.
A memoized failure does not itself guarantee one emitted notice.

---

## Counterexample index

The grilling checklist in ticket 30 names ten counterexamples. Where the
evidence lives:

| Counterexample | Evidence |
| --- | --- |
| Apia skipped day | S6 australasia, `Pacific/Apia` (2011-12-30 removed; 24:00 end convention) |
| Ambiguous midnight | S6 northamerica, Cuba `0:00s` rules; S2 timezone doc |
| DST fold and gap | S5 functions-datetime (Denver 1 day vs 24 hours); S2 timezone doc (LA fold/gap) |
| Month clamp | S5 (Mar 31 + 1 month = Apr 30); S20 (Jan 30 + 1 month = Feb 28/29); S2 (`constrain`/`reject`) |
| Invalid week 53 | S1 (week 53 rollover note); ISO `2025-W53` invalid, no rollover (ticket 21) |
| Prefix fallback | S9 (Dataview filename forms); S17 (NumPy year-granularity equality — a different convention, not a prefix fallback); S7 (Moment `moment.ISO_8601` forgiving parse of a UUID, K23); ticket 21 no-reinterpretation rule |
| Percent-encoded link | S13 §2.4 (decode once); S12 (destination forms) |
| Period equality vs membership | S2 `PlainYearMonth` (reference-day inequality); ticket 21 `(start, unit)`; S4 (nominal vs exact); S21 `[V]` (partial order) |
| Relative literal parsing | S5 (`today`/`tomorrow` staleness in prepared statements); S10 (Tasks recompute and issue #1289); S23 (Duckling `reftime`, K17) |
| Null-notice observability | S9 (Dataview silent `file.day`); q36 case table |

## Competing designs the evidence supports

These are competing inputs for the grilling, not decisions.

1. Dialect: one native strtime dialect with a template Moment adapter [P].
   Keeping the translator in temporal remains an option.
   Removing Moment needs a migration of the owner's tokens.
2. NL grammar: closed in-repository grammar [P], versus candidate
   integration, versus retaining the hook without adoption.
   Candidate point results and coverage gaps need explicit treatment.
3. Civil/instant: an inner enum versus separate runtime variants.
   Either shape must preserve accepted semantics and the size limit.
   PostgreSQL's input policy does not decide the Rust representation.
4. Zone key: database names plus explicit fixed offsets [P].
   Define legacy aliases, POSIX rules, and validation timing separately.
5. Week config: a configurable pair versus fixed ISO.
   Preserve ISO filename prefixes under general week settings.
   The owner's Sunday-first display does not decide week numbering.
6. Bare weekday: nearest, strictly next, or current-week selection.
   Explicit `next`/`last` forms are another proposal.
   No inspected source chooses the owner's preferred rule.
7. Relative spelling: functions (`today()`, `eom()`) [P], versus bare
   keywords (collides with field paths [V-repo]).
8. Period values: complete the single runtime carrier with a normalized
   period [P]. A separate query carrier needs an amendment.
   Keep runtime representation separate from note-parser acceptance.
9. Parser stack: keep Jiff and Logos [P].
   Remove speedate only with guarded parity and diagnostics preserved.
   Keep winnow's boundary role unless a replacement proves parity.
10. Frame: one clock sample, week rule, and lazy zone result [P].
    Remove obsolete ambient reads after explicit inputs replace them.

## Gaps and unverified claims

List for the integration owner. Each needs a check or a decision to leave
open:

1. Candidate APIs have bounded source and runtime evidence.
   The expanded run confirms fuzzydate's same-day anchor retention.
   Both candidate features build with stable 1.92.0 and 1.98.0.
   Exhaustive safety and either minimum supported version remain unproved.
2. W3C XSD duration partial order: closed. S21 read and confirmed in the
   K round (`k-citation-verification.md`, K11); q22 states it as `[V]`.
3. Arrow formatter evidence: S16 documents `strftime`, not `strptime`.
   Its default is `%Y-%m-%dT%H:%M:%S`, with locale `C`.
   An unknown timestamp zone or unavailable host locale causes an error.
   The function and options pages give no complete directive table.
   They support the formatter and type comparison, not directive parity.
4. Jiff `tz-system` failure behavior: closed. Source-read in the K round
   (K20); q30 states it as `[V]` with the `try_system`/`system` split.
5. Binary sizes: the M report now records final probe deltas of +81.17 KiB
   and +98.11 KiB. Repository dependency-linkage deltas are +48.88 KiB and
   +65.39 KiB. These are not complete integration costs.
   Separate `tzdb-bundle-always`, `logos` and in-repository grammar costs
   remain unmeasured.
6. In-repository grammar size and test cost: no implementation exists.
   Measure the selected grammar before adoption. Do not substitute an
   unsupported estimate.
7. `moment.now` overridability: closed as documented. Moment's plugin
   guide documents the `moment.now` override (K23 in
   `k-citation-verification.md`).
8. Chrono-english's same-day rule remains source-only evidence.
   The expanded fuzzydate run confirms its same-day anchor retention.

## Pointers to sibling files

- Crate-level facts (licences, sizes, dependency trees, cache state) for
  `speedate`, `winnow`, `logos`, `jiff`, `jiff-english`, `fuzzydate`: the
  d-crate researcher's file.
- Repo-side layout facts: the lens scouts' files.
- Prior decisions and their wording: the b-prior file.
- Digest lessons: `c-digest-lessons.md`.
- Integration and cross-checks: the brief owner.
