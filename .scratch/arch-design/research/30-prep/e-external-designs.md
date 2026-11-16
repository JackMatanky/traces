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

- `[V]` = I read the linked source in this session and quote or paraphrase its
  text. The link shows the page.
- `[V-repo]` = ticket 30 records a verified code read on main @ `cf6d79bd`.
  I did not re-read the file. I cite the ticket.
- `[H]` = hypothesis or unverified claim. It needs a check before the owner
  treats it as evidence.
- `[P]` = proposal for the grilling. Not a decision.
- No measurement in this file comes from this session. Where a size or cost
  matters and nobody measured it, the text says "not measured".
- Each design option carries a counterexample and a condition that would
  reject it. Ticket 36's evidence-gate rules need this shape.

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
| S22 | crates.io API reads for `jiff-english` 0.0.1 and `fuzzydate` 0.4.1, 2026-11-16 (recorded in ticket 30; I re-checked the cache state of both) |
| S23 | Duckling (facebook/duckling) README, <https://github.com/facebook/duckling> |
| S24 | In-repo probe runs: `probes/parity`, `probes/parity/tests/layout.rs`, `probes/candidates`, `probes/moment`; results recorded in `m-gap-closure.md` (2026-11-17) |

Gaps at the end of this file list what these sources do not cover.

---

## A. Moment compatibility and format dialects (q1 to q5)

### q1. Is Moment compatibility a domain concept?

- [V] Templater delegates all formatting to Obsidian's runtime `moment`
  object and translates nothing (research 07, recorded in ticket 30).
- [V] Moment's week tokens (`w ww gggg` local, `W WW GGGG` ISO) get their week
  start and week-one rule from the locale (`dow`, `doy`). The locale can
  change per call. A value that depends on the locale is not a stable domain
  value (S7).
- [V] Moment's forgiving parser is the default. The string
  `5917238b-33ff-f849-cd63-80f4c9b37d0c` matches `YYYYDDD` and parses as
  `5917-08-26` (S7, strict-mode guide, which also documents the
  `moment.ISO_8601` strict mode).
- [V] `minijinja-contrib` date filters use strftime only. They hold no Moment
  logic [V-repo, research 07].

Counterexample against "Moment is a domain concept": the same template renders
different week numbers after a locale change. A domain value must not change
that way.

Counterexample against dropping Moment tokens now: the owner's vault templates
write `ww`, `YY`, `Q`, `Qo`, and the Fileclass data uses `YYYY`, `DDDD`, `Q`
[V-repo]. A cold drop breaks those templates.

[P] Treat Moment syntax as a template compatibility adapter. Keep the domain
value model dialect-free. Rejecting condition: schema `date.format` checks
must accept Moment tokens. Then the dialect crosses the schema seam. A
second home for the dialect needs a decision (q3, q4).

### q2. Where does the Moment dialect translator live?

Landed state: `pattern_to_strftime`, `MOMENT_TOKENS`, and the ordinal helper
sit in `src/temporal/token.rs`. A `%` outside brackets means strftime;
otherwise the code translates Moment tokens. An unknown token returns
`UnsupportedToken` [V-repo].

- [V] `jiff` supports strftime-like parsing and formatting only, in
  `fmt::strtime`. Its docs say the goal is not to match POSIX exactly
  (S19).
- [V] Moment tokens the translator must map include week-of-year and quarter
  tokens (`ww`, `Q`, `Qo`). Those need a week rule and a quarter rule. Jiff
  gives `%V`/`%G` (ISO week) and `%q` (quarter, format-only). It gives no
  local-week tokens (S19).
- [V] `jiff-english` and `fuzzydate` do not carry format tokens at all
  (S22).

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

- [V] One dialect per surface is the norm in mature systems. PG keeps ISO
  input unambiguous in every `DateStyle` mode, and documents that non-ISO
  input orders (`MDY`, `DMY`, `YMD`) re-read the same literal (S5). Jiff's
  `fmt::strtime` is one grammar for parse and print (S19).
- [V] A two-grammar system needs a dispatch rule and documents it: the
  current `pattern_to_strftime` dispatches on `%`. That rule makes
  `YYYY` a Moment pattern and `%Y` a strftime pattern. It cannot express a
  format that mixes both. Jiff has no quarter directive; its `%Q` is a
  zone-offset alias, not a quarter (S19).
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
- [V] Fileclass skips pattern validation when a field's declared format is
  a plain string, so Moment `dateFormat` tokens go unlinted; its outer
  validator still checks empty values and `required` [V-repo, research 14;
  K8 in `k-citation-verification.md`].
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

[P] Classify each format token at schema load. Formats that name a full date
get the value check. Formats that name a month, quarter, or day-of-year get
a structural check only (string shape). Formats with no date component get
no value check. The ticket 20 fragment is located and read; it does not
gate this proposal (`i-leakage-review.md`, I1).

### q5. `format` under `[frontmatter.date_created]` and `[frontmatter.date_modified]`

- [V] Ticket 22 records that no production code reads these keys. The
  defaults are `%Y-%m-%dT%H:%M:%S` (strftime).
- [V] A configured input format with day-first or month-first order is a
  hazard PG documents: `01/02/03` re-reads per `DateStyle` mode (S5).
- [V] RFC 3339 section 5.1 shows why the timestamp format for stored values
  should stay fixed and sortable. Lexicographic order equals time order
  only when all strings share one precision, one uppercase `T`/`Z`
  spelling, and one zone offset (S3).

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

Counterexample against keeping `Anchor` in the template engine: the query
side also needs the pinned `today` (relative keywords bind at plan time,
ticket 29). Two anchors mean two clocks.

Counterexample against moving the `date_*` filters into temporal: filters
are a template API surface. Ticket 16 freezes template capabilities but the
Rust library surface is free.

[P] Move arithmetic (`apply_offset` and the calendar-series logic it needs)
into temporal, next to `resolve_period_edge`. Keep filter registration in
the template engine. `Anchor` dies once the frame from q33 lands. `weekday`
moves with the week rule into temporal.

### q7. Where does the filename-period parser live?

- [V-repo] Today `FileMeta::derive_day` reads the base name with
  `DatePoint::parse_prefix` and falls back to the creation time.
  `scan_date_prefix` in `src/temporal/lexer.rs` sees `YYYY-MM-DD` and
  `YYYY-MM` only. The note parser also calls `parse_prefix`. Ticket 16
  freezes that parsing.
- [V] Dataview's `extractDate` searches the whole file name for full date
  shapes (`yyyy-mm-dd`, `yyyymmdd`, ISO datetimes), and its `findDate`
  checks typed metadata fields first (S9). Two differences from Traces: no
  weeks or quarters, and no strict-prefix scan (frontmatter dates, not
  creation time).
- [V] NumPy's `datetime64` shows the opposite convention: `'2005'` parses as
  a year-granularity value and equals `'2005-01-01'` (S17). Traces must not
  do this: ticket 21 forbids reinterpreting an invalid longer prefix as a
  shorter prefix.
- [V] A `parse_prefix`-style scan is prefix-driven, and year-first ISO is
  the prefix-friendly order. RFC 3339's ordering property covers complete
  same-shape strings, not prefixes with arbitrary tails, so prefix
  unambiguity is a convention, not an RFC theorem (S3).

Ticket 21 adds validated ISO weeks, quarters, and an opt-in bare year, and
scopes the new parser to filenames.

Options for the home:

1. In temporal, next to `DatePoint` and the existing prefix scan. The
   parser shares the date grammar and the week validation. Counterexample
   against: none found. `DatePoint::parse_prefix` already sits in temporal.
2. In the file scanner. It would re-implement prefix and week logic or call
   temporal anyway. Counterexample: two homes for one grammar.
3. In the note parser. Forbidden: vault-format parsing is frozen (ticket
   16), and filename forms must not change note parsing.

[P] Option 1. A `filename::Period` parser unit in temporal, consumed by the
scanner and by `file.date`/`file.cdate` evaluation. The parser returns a
kind plus a start, not one `DateValue` (ticket 21).

### q8. The week-one rule for a custom week start

- [V] CLDR defines a week by two values: `firstDay` (sun..sat) and `minDays`
  (1..7). Root locale: Monday, 1 day. The US uses Sunday, 1 day. Many
  European territories use a 4-day minimum with Monday. The rule: a week
  counts as
  week 1 of a year when it includes at least `minDays` days of that year.
  CLDR also tells implementations how to read week number 53 in a year
  with no week 53. The reader treats it as week 1 of the next year (S1).
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

Counterexample for the rollover trap: ISO `2025-W53` does not exist. CLDR
tells readers to map it to week 1 of 2026. Ticket 21 forbids that
rollover and calls the literal invalid. The config must state which
behavior applies to a typed week literal and to a filename week prefix.
Ticket 21 already decides: invalid, no rollover.

[P] `DateConfig` carries the pair `(first_day, min_days)`, default
`(monday, 4)` = ISO. Week-based numbering always follows the pair.
Rejecting condition for the default: if the owner's usage shows US-style
weeks, the default changes to `(sunday, 1)` and ISO week literals need a
separate spelling.

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

Counterexample against removing it before a check: the call site's accepted
forms had not been diffed against jiff when this file was written. The
parity probe now ran in-repo (`probes/parity`, S24): 4,635,370 guarded
inputs, zero acceptance, value, and error-category differences. The guard
is the ten-byte date shape only; raw jiff accepts a superset (compact
`20260101`, naive datetimes, negative years). Ticket 16 freezes
vault-format parsing. A silent grammar change breaks frozen parsing.

[P] Remove after a parity audit of the call site's accepted forms against
jiff. The audit exists in-repo (`probes/parity`, S24) and holds for the
guarded date shape. The remaining audit scope (the call site's other
shapes) still belongs in the ticket Answer before removal.

### q10. Does `winnow` stay?

- [V-repo] One call site, in `src/temporal/lexer.rs`. Ticket 25's freshness
  input says `winnow` locates duration prefixes for inline fields, and those
  values reach the store.
- [V] The accepted duration shapes are frozen vault-format parsing (ticket
  16). Jiff's `Span` grammar allows weeks mixed with other units and
  fractional lowest units (S19). RFC 5545 allows weeks only alone and
  forbids years and months (S4). None of these grammars is the frozen one.
- [V] `winnow` is a parser-combinator library. A combinator in one call site
  is a dependency for one grammar.

Counterexample against swapping winnow for jiff: the frozen inline-field
duration grammar does not match jiff's `Span` grammar. A swap changes what
parses, which ticket 16 forbids without a change-table row.

[P] Keep `winnow` until ticket 35 does the crate cut. Keep the crate in the
ticket 25 fingerprint list (ticket 36 records the omission). A hand-written
replacement needs a grammar spec first. That is a new artifact, not a
shortcut.

### q11. Does `logos` stay as part of the parsing base?

- [V-repo] `logos` serves `lexer.rs`, `datetime.rs`, and `point.rs` in
  temporal, plus the shared lexer, the note parser, and the query grammar.
- [V] `logos` builds a deterministic automaton from regexes at compile
  time. Errors carry spans. The closed-keyword NL grammar of ticket 19 fits
  a token-based lexer well.
- [H] Binary-size effect under `opt-level = "z"`: not measured.

Counterexample against removal: three temporal lexers plus the query grammar
already share it. A removal is a rewrite of four lexers with no reported
defect.

[P] Keep `logos` as the lexing base. The NL grammar (q13) should reuse it
for the closed keyword set.

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
- [V] Duckling (Haskell, rule-based) resolves relative time against a
  `reference_time` parameter and reports a grain (day, hour, second). Its
  corpus tests each rule with positive and negative examples (S23).
- [V] Obsidian Tasks uses the flexible parser `chrono` for filter dates and
  warns users against two adjacent word-dates in one range because flexible
  parsing makes the range ambiguous. It recommends `YYYY-MM-DD` (S10).
- [V] Dataview's relative literals are a fixed keyword set
  (`today`, `now`, `tomorrow`, `yesterday`, `sow`, `eow`, `som`, `eom`,
  `soy`, `eoy`) (S9).

Counterexample against an open NL parser: Tasks documents the failure mode
(ambiguous ranges) of flexible parsing in a user-facing note (S10). A query
engine needs deterministic plans. Ambiguity breaks caching and replays.

[P] Closed grammar, as ticket 19 proposes. The grammar table belongs in the
ticket Answer with one example and one negative example per rule (the
Duckling corpus practice, S23).

### q14. Build in the repository or use a crate?

Candidates per ticket 30: `jiff-english`, `fuzzydate`. The prep items the
owner needs, with what the evidence gives:

| Item | `jiff-english` 0.0.1 | `fuzzydate` 0.4.1 | in-repo |
| --- | --- | --- | --- |
| Licence, releases | MIT; 1 release (2026-08-07); 29 downloads (crates.io, 2026-11-17) [V, S22, S24] | MIT; 14 releases; last update 2026-04-05 (crates.io, 2026-11-17) [V, S22, S24] | n/a |
| Size | ~1.3k lines [V, S22] | ~2.6k lines [V, S22] | [H] not written yet |
| Dependencies | `jiff ^0.2` default features + `scanlex ^0.1`; the probe shows the jiff default features unify into the build (`tz-fat`, tzdb bundles, `perf-inline`) [V, S22, S24] | `chrono 0.4.45` (feature `clock`), `lazy_static`, `thiserror`; the jiff tree stays untouched [V, S22, S24] | none |
| Determinism | bounded probe matrix, one anchor, own process per phrase [V, S24]; a source-level `week_day` slicing panic risk remains | same probe shape; no panic observed on the matrix [V, S24] | yes, by construction (frame in q33) |
| Week rules | `last friday` returns the previous day (2026-03-13 from the 2026-03-14 anchor) [V, S24] | `last friday` returns one week back (2026-03-06); bare weekdays keep the anchor time [V, S24] | decided by q8 |
| Output type | a point; `next week` is rejected outright [V, S24] | a point; `next week` returns a +7-day point [V, S24] | `CalendarPeriod` (start, unit) per ticket 21 |
| Coverage of the closed grammar | no `next week`, no `eom`, no `in 2 weeks`; lenient mode misparses `start of month` as midnight today [V, S24] | no `eom`, no `in 2 weeks`; `next week` covered as a point [V, S24] | exactly the decided list |
| Coupling with jiff `=0.2.38` | unifies jiff default features into the build [V, S24] | adds `chrono`; two datetime stacks in one binary [V, S24] | none |
| Binary cost, probe | +65.0 KiB over the 409 KiB probe baseline, including the feature unification [V, S24] | +48.9 KiB, including the `chrono` stack [V, S24] | [H] not measured |
| Speed, probe | 263 to 588 ns/parse (indicative) [V, S24] | 215 to 585 ns/parse (indicative) [V, S24] | [H] not measured |
| Version risk | 0.0.1, 29 downloads, one release [V, S22] | active but pulls a second stack | repo owns changes |

Limits of the probe rows: the matrix is a bounded phrase list. Exhaustive
Unicode, overflow, and ambient-state behavior is not covered, and the
benchmark is indicative, not controlled (S24).

Evidence from the closest comparable crate, `chrono-english` 0.2.1 (S20):

- It takes the anchor as an explicit `DateTime` parameter. Determinism is
  conditional: the result depends only on the text and pinned inputs, but
  zone data and week rules are among those inputs (S20).
- It outputs `Interval = Seconds | Days | Months` for bare offsets. That is
  a duration, not a period. `next week` needs a period (q16). No candidate
  crate's documented output is a period.
- Its weekday semantics are dialect-split: US `next friday` = the coming
  Friday. UK `next friday` = Friday of next week. A repo grammar fixes one
  meaning.
- Its short-year pivot is 1940 to 2040. A repo grammar can choose the same
  or another pivot, but must state it.

Counterexample against `fuzzydate`: it depends on `chrono` with the `clock`
feature. `Cargo.toml` has no `chrono` entry today [V, S22]. Adding it puts a
second calendar implementation next to jiff, with different week and
arithmetic defaults, inside a codebase that just pinned jiff.

Counterexample against `jiff-english`: one release, one month old by its
date, 29 downloads. That is not a maintenance signal. It is a prototype
signal. A crates.io API re-check on 2026-11-17 still shows 29 downloads
(S24).

[P] Build in the repository, as ticket 30 proposes. The runtime probe
settles the rejecting condition in the negative. Neither candidate yields
`(start, unit)` periods or takes an injected anchor, and `jiff-english`
rejects `next week` outright (S24, `m-gap-closure.md`). Probe costs:
+65.0 KiB (english, with jiff default-feature unification) and +48.9 KiB
(fuzzy, with a `chrono` stack); both parse in under a microsecond.

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

Counterexample against "nearest" in a query engine: `nearest` is
non-monotonic in `today`. On Wednesday a bare `tuesday` resolves to
yesterday, so `due > tuesday` includes today. A user who reads the filter
as "next Tuesday" gets extra rows and gets no error (S10).

Counterexample against "current week" in queries: on Sunday, ISO current
week runs Monday to Sunday, so a bare `monday` on Sunday lands six days
back. The template filter keeps that meaning today. A query meaning that
differs from the template meaning needs a documented split.

Counterexample against "next" as the only meaning: Tasks users document the
nearest expectation. Adopting `next` silently changes answers for users who
come from that habit. Ticket 19 asks to fix exactly this fork.

[P] Spell the meanings, do not imply them. Expose `next <weekday>` and
`last <weekday>`. java.time's `nextOrSame` decision can wait. `next` =
strictly after `today` is the monotonic choice. Give a bare weekday one
fixed meaning chosen by the owner, recorded in `DateConfig` (q27).
Rejecting condition for any bare-weekday default: none is safe by evidence
alone. This input does not decide it.

### q16. Interval precision of phrases such as `next week`

- [V-repo] Tickets 19 and 21 separate periods from arithmetic points. The
  grammar must yield a period with a unit.
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

Counterexample against a point-only grammar: `file.date = next week` as a
point is underdetermined (which day?). Tasks resolves it as a range (S10);
Traces resolves it as a period (ticket 19).

Counterexample for the clamp rule in periods: the period "next month"
starts on a computed date. The start can clamp if `today` is `2027-01-30`
and the unit arithmetic walks month edges. The ticket must state whether
the start uses clamping or reject-on-invalid.

[P] Grammar yields `CalendarPeriod` (normalized civil start, unit),
equality by `(start, unit)`, per tickets 19 and 21. Start computation uses
clamping, matching PG and `chrono-english`, with `reject` recorded as the
alternative.

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

[P] Keep the function spelling: `today()`, `eom()`, `eom(date)`, with the
no-argument forms resolving against the frame's `today` at plan time
(ticket 29). Rejecting condition: none known. The collision with field
paths is decisive.

### q18. The frontend spelling of `member_of`

- [V-repo] Ticket 29 reserves only the IR name. The first pushdown scope
  covers unquoted literals and `And`.
- [V] Tasks has a semantic membership filter, `happens`: true when any of
  the task's dates falls in the given range (S10). The name reads as
  membership, not containment.
- [V] Dataview spells containment as `contains()` on lists (S9).
- [V] PG spells array containment as `@>` and set membership as `IN`
  (S5).
- [V] Ticket 21: membership is separate from equality, grouping, and
  ordering. Period membership tests the civil date of the instant in the
  pinned zone.

Counterexample against overloading `in`: range filters already use `in`
spelling in this family of tools (Tasks `in 2026-06`, S10). Overloading
makes `date in 2026-06` ambiguous between range and membership.

Counterexample against an operator soup (`@>`): the DSL is user-facing
text. Symbol operators read as implementation, not query.

[P] One word, one meaning: a named operator or function spelled
`member_of(...)`, or a distinct word such as `within` if the owner prefers
prose. Keep `in` for ranges. Rejecting condition: if `member_of` reads as
implementation to the owner's users, the word changes. The IR name stays.

---

## E. Value shapes (q19 to q25)

### q19. Type shape for civil and instant values

- [V] java.time: separate types `LocalDate`, `LocalTime`, `LocalDateTime`,
  `ZonedDateTime`, `Instant`, `OffsetDateTime`. The package doc states the
  design intent: "the widespread use of time-zones tends to add considerable
  complexity" (S14). The doc also recommends `Offset*` types for network and
  database protocols. Stores hold offsets, not zone identities.
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

Counterexample against a single `DatePoint` with a mode flag: PG's mode
flag is the documented hazard (silent offset drop, no zone retention). The
value also cannot state which conversions are legal at the type level.

Counterexample against one `NoteFieldValue` variant per kind (`Date`,
`CivilDateTime`, `Instant`, `Period`): each variant spends the 80-byte
envelope and each needs coercion rules. Ticket 19 already lists the three
temporal brackets (`Date`, `DateTime`, `CalendarPeriod`).

[P] One `DateValue` slot holding an inner enum `civil | instant` (plus a
period representation at the temporal level, q20). Precision of written
input stays on the civil side. Rejecting condition: if an inner enum pushes
`DateValue` past the slot budget, the variant split returns. Sizes are not
measured here.

### q20. Where `CalendarPeriod` sits in the value model

- [V-repo] Ticket 18 lists no period variant in `NoteFieldValue`. Tickets
  19 and 21 need period values for `file.date` and filter literals. Ticket
  19 puts `CalendarPeriod` in the temporal brackets. Ticket 21 defines it
  as a normalized civil start plus unit.
- [V] Temporal's `PlainYearMonth` is a period-as-value precedent. Its
  constructor pitfall (reference day makes equal months unequal) is
  evidence for storing the normalized start, never the raw label (S2).
- [V] Arrow carries an interval type in its columnar format with units
  months, day-time, and month-day-nano (S16). The minimal unit (month
  interval) is a plain integer of months. It is a storage encoding without
  calendar semantics; it supports a `(start, unit)`-style layout but does
  not decide period equality or ordering.
- [V-repo] The store does not persist filename periods. Queries derive them
  (ticket 24). Group keys use `(start, unit)`. Membership never defines a
  group key or range key (ticket 29).

Counterexample against putting the raw filename label in the value:
`2026-W01` reads differently under different week rules (ticket 21). The
value must be the normalized start.

Counterexample against a persisted period column: the store keeps temporal
values as text and derives filename periods at query time. A persisted
period column needs an `IndexPolicy` entry and a fingerprint input
(ticket 25).

[P] `CalendarPeriod` exists as a temporal-level value (filter literals,
`file.date` evaluation, grouping). Whether it needs a `NoteFieldValue`
variant depends on whether note fields themselves can hold periods. No
evidence in this file says they can. Default: no new `NoteFieldValue`
variant.

### q21. Date compared with an instant at an ambiguous or absent local midnight

Ticket 21's handed-over rule: a period against an instant tests the civil
date of the instant in the pinned zone. It does not test that the next
day's midnight exists. Civil values use their civil dates without a zone.
This question keeps that rule separate from membership.

- [V] java.time makes `LocalDate` and `Instant` incomparable at the type
  level. The package doc's design notes explain the type split (S14).
- [V] Temporal makes cross-kind `compare` a TypeError. Only like types
  compare (S2).
- [V] PG compares `date` to `timestamptz` by assuming midnight in the
  `TimeZone` setting, and documents `+ interval '1 day'` versus
  `+ interval '24 hours'` across a DST boundary (Denver 2005-04-03 example)
  (S5). PG's comparison acquires a zone implicitly, which the repo forbids
  (ticket 21: UTC is never an implicit fallback).
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

[P] Keep ticket 21's rule. Equality stays cross-class false. Relational
filters against periods use the pinned-zone civil date of the instant.
Relational filters between civil and instant values do not match (same as
ticket 19 across brackets). Document the Havana and Apia cases in the
Answer.

### q22. The total sort key for mixed temporal values

- [V] RFC 3339 section 5.1 defines when plain string sort equals time sort:
  components from least to most precise, one zone spelling, same fractional
  digits. Optional punctuation breaks it (S3).
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

Counterexample against "sort everything by physical time": a month period
and a day period have no physical time without an anchor. Nominal
durations flip order by anchor month (XSD example, S21). The key must be
label-based, not time-based, across classes.

Counterexample against "sort by written text only": ISO strings sort
correctly only inside one class, one shape, and one zone spelling (S3).
Mixed shapes carry no time order. `2026-02-28` (date) and `2026-03`
(month) differ in shape, so their text order is a byte order, not a time
order.

[P] Key = (class rank, class-local value, unit tie-break). Class rank must
respect the accepted group class order: Civil, Instant, Period (B65/C3).
Within Civil, `Date` before `DateTime` is a free choice. The constraint:
one rank per class, periods after points, unit tie-break ascending Day <
Week < Month < Quarter < Year. Civil values sort by their civil components
(year, month, day, time). Instants sort by instant. The Answer must state
the rank explicitly. This file only supplies the shape and the traps.

### q23. Nominal duration equality and order

- [V] Ticket 19 kept nominal equality and order provisionally.
- [V] RFC 5545 forbids years and months in durations, and its editors say
  why in effect: nominal units have no fixed length (S4).
- [V] java.time splits `Period` from `Duration` so nominal and exact
  quantities never compare (S14). `chrono-english` returns
  `Interval::Seconds | Days | Months` and never compares across the three
  (S20).
- [V] Dataview durations (`dur`) do include months, through Luxon (S9).
  Fileclass validates durations against an RFC 5545 subset: weeks, or days
  and time. No years or months [V-repo, research 14].

Counterexample against numeric equality across nominal and exact units: one
month is 28 to 31 days depending on the month. XSD's own `P1M` versus
`P30D` checks flip by reference date (S21). Any fixed conversion decides
the flip by convention, not by fact.

Counterexample against physical order without an anchor: Temporal throws
when asked (S2). PG clamps month arithmetic and documents the asymmetry
(S5).

[P] Equality: same unit classes, equal counts. Order: by class, then count;
no cross-class comparison. Arithmetic resolves calendar units through the
pinned frame (q33). Document that `dur` values with months keep nominal
semantics and never equal a day count.

### q24. How the documents describe calendar arithmetic

- [V] PG documents the one-day-versus-24-hours difference with a worked
  example and a normative sentence (S5). Moment's "The Guide" does the same
  with its own example (S7).
- [V] RFC 5545 states the exact-duration rule: at a DST discontinuity, add
  or subtract the discontinuity. Do not count leap seconds (S4).
- [V] Jiff's `Zoned` arithmetic respects the zone. Civil arithmetic does
  not see zones (S19).

[P] One normative paragraph in the temporal docs. Day, week, month, year
arithmetic is a calendar series in the frame's zone and pinned week rule.
Hour, minute, second arithmetic is exact. The series examples use the
Denver DST case and the Apia skipped day. No leap second handling. Keep the
examples in the docs, not in this file.

### q25. How `date(link)` reads a Markdown link target

Ticket 19 hands over these rules: read only the last segment. Keep the
extension. Accept a pattern argument. Targets can be percent-encoded.
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

Counterexample for decode-once: a note named `2026-01%20notes.md` decodes
once to `2026-01 notes.md` and the prefix `2026-01` still parses. Decode
twice and a literal `%2541` becomes `A` wrongly. RFC 3986 forbids the
double decode (S13).

Counterexample for stripping the extension: `2026-05-01.review.md` keeps
its extension per ticket 19. The prefix parse must look at the segment
before the first dot-group? The ticket says the target keeps the
extension, so the parser must parse the prefix of the full segment, not of
a stem. State that in the Answer.

Counterexample for external URLs: `date(https://x/2026-01-01)` must not
match. Ticket 19 says external URLs are not note targets. The value is
Null. The notice policy belongs to q36.

[P] Decode percent-escapes once, then parse the leading prefix (or the
given pattern) of the last segment with its extension intact. Non-note
targets yield Null. No silent reinterpretation of a malformed prefix
(ticket 21 rule).

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

Counterexample against free-form keys: an unvalidated `format` string
parses at first use, deep in a query. The error lands far from the config
file. Validation must run at load.

[P] One table with an exact key set (q27), each key scalar or enum,
validated at config load, errors carry the key path. No nested free-form
strings.

### q27. The fields of `DateConfig`

Candidate fields, each with its evidence:

- Default date and datetime formats: today's config default is
  `%Y-%m-%dT%H:%M:%S` [V-repo]. Keep strftime spelling (q3).
- Week rule: `(first_day, min_days)` (q8).
- Weekday resolution for relative phrases: the q15 output, one enum value.
- NL grammar options: on/off plus the keyword list version (q13). A
  version key on the grammar lets the index fingerprint change with it if
  filename periods ever persist (ticket 25).
- Zone: the configured zone key per q29 and ticket 22's precedence.
- Filename period policy: week/quarter prefixes on or off, bare-year opt-in
  key (q32).

Counterexample against putting the NL keyword list itself in user config:
the grammar is a closed, versioned list (q13). Config turns it on or off;
it does not extend it. An open keyword list in config breaks determinism
and the change-table discipline.

[P] The field list above, no more. Every field has a default so an absent
`[date]` table equals today's behavior.

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
resolution, NL options) and a frame type (q33). Config maps `DateConfig`
into `DatePolicy` at load. The edge stays one-way.

### q29. Zone syntax and validation

- [V] IANA zone names: `AREA/LOCATION`, ASCII letters with `.`, `-`, `_`;
  no digits; no case-only-distinct names; legacy spellings live as links in
  `backward`; `Etc/UTC` and `GMT` exist; `Etc/Unknown` is reserved by CLDR.
  The naming rules are guidelines with legacy exceptions (`Etc/GMT0`,
  `GMT0`, `EST5EDT`) (S6 theory).
- [V] PG accepts three zone spellings with different meanings: full IANA
  names, abbreviations (date-dependent offset, e.g. `MSK` moved +3 to +4),
  and POSIX-style strings whose sign is opposite to ISO (`UTC-4` = ISO
  +04). Its docs record the sign confusion as a fact (S5).
- [V] Jiff parses IANA names and fixed offsets (S19). Temporal
  canonicalizes identifiers and errors on unknown ones (S2).

Counterexample against accepting POSIX strings: `EST5EDT` is a POSIX rule
string. `EST` is an abbreviation with a date-dependent meaning. `UTC-4`
reverses sign relative to ISO. Three spellings, three meanings, one config
key: that is the documented PG trap (S5).

Counterexample against no validation: a typo (`Europe/Prag` is not a zone
name) surfaces as a per-row zone lookup failure instead of a config
error.

[P] Accept IANA names plus an explicit fixed-offset form (`±HH:MM`).
Validate at config load against a stated data source and version (a bundled
database with a pinned tzdb version makes load-time validation
deterministic). Reject abbreviations and POSIX strings.

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
- [V] Jiff's failure mode is source-read and recorded: `TimeZone::try_system`
  returns an error when zone discovery fails or the `tz-system` feature is
  off, and `TimeZone::system` can fall back to `UTC` under the same
  conditions. The frame must use `try_system` and memoize the error
  (S19; owner-evidence.md, jiff section; K20 in
  `k-citation-verification.md`).

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
- [V-repo] The owner's week template lists days from `day(0)`, Sunday
  first [V-repo]. The owner's own templates disagree with an ISO-only
  default.

Counterexample against "week start configurable, week-one rule fixed at
ISO": `first_day = sunday` with `min_days = 4` is rare. CLDR lists it for
`is` and `pt`, but it is not unused (S1). It re-numbers weeks in ways the q8
tables show (S14). The pair must move together.

[P] Configurable as the `(first_day, min_days)` pair, default ISO. The
week rule enters the frame and the fingerprint-relevant policy set if
filename weeks ever persist (ticket 25 gate).

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

[P] Key `[date] filename_year = false` (default), validated as exactly
four digits, range check on the value, no rollover. Alternative spellings
(`filename_periods.year`) go to the change-table row if the owner prefers
nesting.

---

## G. Boundary after the execution-context decision (q33 and q34)

### q33. What temporal exports to the execution frame

- [V-repo] Ticket 21: one frame per query, render, or request pins the
  clock and the week rule. The frame acquires the zone only when needed and
  memoizes a failure. UTC only if explicitly selected.
- [V] java.time models this as an injectable `Clock` (with a fixed variant
  for tests) plus explicit `ZoneId` arguments (S14).
- [V] Duckling takes `reference_time` as a parameter. Its corpus pins one
  reference time to make tests deterministic (S23).
- [V] Tasks' relative dates recompute from `today` and the maintainers
  document re-evaluation and staleness pitfalls for long-lived results
  (S10, issue #1289 recorded there).

Counterexample for a frame without the week rule: `sow()` binds at plan
time. A frame with a clock but no week rule cannot compute it. Ticket 21
requires the pin.

Counterexample for a frame without a zone resolver: civil-to-instant
projection needs the zone on demand. A memoized failure is part of the
frame's determinism (ticket 21).

[P] Frame contents: `today` (civil date), week rule, zone resolver (lazy,
memoized failure), and the `DatePolicy` (q28). Temporal exports the frame
type and constructors. Query and template hold and pass frames.

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

[P] Every temporal function takes the frame parameter. Test fixtures build
fixed frames. No global state remains in temporal.

---

## H. Query pushdown and notices (q35 and q36)

### q35. Temporal probe bounds per operator and value kind

- [V-repo] Ticket 24: a range key needs proof for each operator, row kind,
  temporal frame, and field precedence. Unproved mappings scan. The first
  pushdown scope covers unquoted temporal literals and `And`. Membership
  never defines a range key (ticket 29).
- [V] PG's expression evaluator does not define subexpression evaluation
  order, and the planner may reorganize boolean expressions by Boolean
  algebra (S5). A pushdown bound must stay sound if the engine reorders or
  drops predicates.
- [V] Cross-kind ordering is where proofs break: Temporal refuses mixed
  nominal ordering without an anchor (S2), and the q22 class ranks decide
  whether `<` between a date and a period even has a meaning.

Counterexample for a proved bound across classes: `mdate < 2026-03` is a
month period tested against a mix of civil dates and instants. The proof
needs the class rank of the period and the exclusive upper bound. If the
plan reorders `A And B` to `B And A`, the bound must still hold. Bounds
that depend on evaluation order are unsound (S5).

Counterexample for membership pushdown: membership tests the civil date of
the instant in the pinned zone. A range key over instants cannot answer it
without the zone. Ticket 29 says membership never defines a range key
[V-repo]. That rule is consistent with the zone-dependency.

[P] A table in the Answer. Rows = value kinds (Date, DateTime civil,
Instant, Period). Columns = operators (`==`, `<`, `<=`, `>`, `>=`,
`member_of`). Each cell = the proved bound formula or `Unsupported`. No
cell depends on evaluation order.

### q36. When a row with a `Null` date gets a notice

- [V-repo] Ticket 34 owns the notice mechanism. Ticket 29 did not adopt the
  first-round notice.
- [V] Dataview's `file.day` is silently absent when the file name has no
  date. No notice, no error (S9). That is the silent-Null baseline.
- [V] Fileclass skips pattern validation for a plain-string custom format
  today; the outer validator still checks empty values and `required`
  [V-repo, research 14; K8 in `k-citation-verification.md`]. A malformed
  date value in such a field is invisible today.

Counterexample for noticing every Null: `file.date` is Null for most files.
A notice per row floods output and trains users to ignore notices.

Counterexample for noticing nothing: `2026-13-40-notes.md` has a
date-shaped prefix that fails validation. Silent Null hides the malformed
file name from the user. Ticket 19 already chose an advisory notice for an
invalid quoted date literal. Silence would be inconsistent.

[P] Three cases. (a) Absent value with expected shape = Null: no notice.
(b) Malformed date-shaped value where a format was declared or a prefix
failed validation = one notice per file or literal, with the span.
(c) Zone failure during projection = the typed zone error, not a notice.
Ticket 34 owns the plumbing. This ticket owns the condition.

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
| Prefix fallback | S9 (Dataview filename forms); S17 (NumPy year-granularity equality — a different convention, not a prefix fallback); S7 (Moment forgiving parse of a UUID as `YYYYDDD`); ticket 21 no-reinterpretation rule |
| Percent-encoded link | S13 §2.4 (decode once); S12 (destination forms) |
| Period equality vs membership | S2 `PlainYearMonth` (reference-day inequality); ticket 21 `(start, unit)`; S4 (nominal vs exact); S21 `[V]` (partial order) |
| Relative literal parsing | S5 (`today`/`tomorrow` staleness in prepared statements); S10 (Tasks recompute and issue #1289); S23 (Duckling `reference_time`) |
| Null-notice observability | S9 (Dataview silent `file.day`); q36 case table |

## Competing designs the evidence supports

One line per axis, options in evidence order. These are inputs for the
grilling, not decisions.

1. Dialect: one strftime dialect everywhere plus a template-only Moment
   adapter [P]. Alternatives: keep-as-is translator in temporal
   [V-repo state]. Moment everywhere is rejected (q1 counterexamples).
   Strftime everywhere with no adapter is rejected by the owner's template
   tokens [V-repo].
2. NL grammar: closed in-repo grammar [P], versus `fuzzydate` (second
   calendar stack), versus `jiff-english` (0.0.1 prototype), versus defer
   to fog with a scope decision (ticket 19 preserves the hook).
3. Civil/instant: inner enum in one slot [P], versus separate value
   variants, versus mode-flag type (PG's documented hazard).
4. Zone key: IANA names plus explicit fixed offsets [P], versus full PG
   spelling zoo (documented sign traps), versus free-form string (late
   errors).
5. Week config: `(first_day, min_days)` pair [P], versus first-day-only
   (underdetermined numbering), versus ISO-only (Tasks' shipped choice,
   owner's templates disagree).
6. Bare weekday: spelled `next`/`last` with one bare meaning in
   `DateConfig` [P], versus nearest-only (Tasks), versus next-only
   (monotonic), versus current-week (template parity), versus four adjuster
   spellings (java.time). No evidence-based single winner. Owner decides.
7. Relative spelling: functions (`today()`, `eom()`) [P], versus bare
   keywords (collides with field paths [V-repo]).
8. Period values: temporal-level value, no new `NoteFieldValue` variant
   [P], versus new variant (spends slots, no evidence note fields hold
   periods).
9. Parser stack: keep jiff and logos, remove speedate after a parity audit,
   keep winnow until ticket 35 [P], versus big-bang removal (frozen
   grammar risk).
10. Frame: frame carries clock, week rule, lazy memoized zone, policy
    [P]. Zero ambient reads.

## Gaps and unverified claims

List for the integration owner. Each needs a check or a decision to leave
open:

1. `fuzzydate` and `jiff-english` APIs: closed. The runtime probe
   (`probes/candidates`, S24) measured the behavior matrix; the q14 table
   rows now cite it.
2. W3C XSD duration partial order: closed. S21 read and confirmed in the
   K round (`k-citation-verification.md`, K11); q22 states it as `[V]`.
3. Arrow strptime directive table: not retrieved. S16 gives the columnar
   types and the pyarrow signature (default `%Y-%m-%dT%H:%M:%S`, locale `C`,
   error on unknown tz). Enough for the interval-type evidence, not for a
   directive-by-directive comparison.
4. Jiff `tz-system` failure behavior: closed. Source-read in the K round
   (K20); q30 states it as `[V]` with the `try_system`/`system` split.
5. Binary size effects: the two NL candidates are now measured in the
   probe (+65.0 KiB and +48.9 KiB, S24). `tzdb-bundle-always`, `logos`,
   and an in-repo NL grammar in repository builds: still not measured. The
   brief forbids unmeasured cost claims; change-table rows must not cite
   sizes except the probe's own.
6. In-repo grammar size and test cost (q14 Size row): not written yet.
   That estimate belongs to the owner with the d-crate researcher's input.
7. `moment.now` overridability: closed as documented. Moment's plugin
   guide documents the `moment.now` override (K23 in
   `k-citation-verification.md`).
8. The `chrono-english` doc phrase "this coming Friday" leaves
   same-day behavior unstated (is `friday` on a Friday today or +7?). A
   source read answers it if the owner wants the parallel exactly.

## Pointers to sibling files

- Crate-level facts (licences, sizes, dependency trees, cache state) for
  `speedate`, `winnow`, `logos`, `jiff`, `jiff-english`, `fuzzydate`: the
  d-crate researcher's file.
- Repo-side layout facts: the lens scouts' files.
- Prior decisions and their wording: the b-prior file.
- Digest of plans and designs: the b-digest file.
- Integration and cross-checks: the brief owner.
