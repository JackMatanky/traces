# Digest lessons for ticket 30: temporal boundary and dialects

Artifact: `research/30-prep/c-digest-lessons.md`.
Ticket: `issues/30-temporal-boundary-and-dialects.md` (36 questions).
This file gives digest evidence and per-question Traces proposals.
Product behavior and proposals remain separate.

Method and limits:

- Sources include Templater, Dataview, Natural Language Dates (`nldates`),
  Fileclass, Metadata Menu, Obsidian Tasks, IWE, and zk.
  Rust PKM sources include basalt, obsidian-export, and obsidian-rs.
  This review uses `docs/digests/` excerpts, not new upstream clones.
- Tags: `[V]` records a digest or ticket 30 read. `[H]` marks a hypothesis.
  `[P]` marks a proposal. `[A]` marks absence from the inspected excerpts,
  not proof of absence from complete upstream projects.
- Upstream file names and line numbers come from digest excerpts.
  Treat those line numbers as approximate.
- Prose uses short, direct sentences. Technical terms retain their domain
  meanings. The owner material and this report use English.

## Q1 to Q5: Moment compatibility and format dialects

### Q1: Is Moment compatibility a domain concept?

Evidence:

- Templater calls Obsidian's runtime `moment`.
  It exposes that object to user scripts.
  Docs: `obsidian_silentvoid13-templater-docs-digest.txt:1587-1597`.
  Source: `InternalModuleDate.ts`, with default `"YYYY-MM-DD"`,
  `.weekday(n)`, and a `moment(reference, reference_format)` guard.
  Upstream: github.com/SilentVoid13/Templater. `[V]`
- Dataview uses Luxon `DateTime`, not Moment.
  Source: `obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:7643-7666`,
  including `export type { DateTime, Duration } from "luxon"`.
  Its default format is `"MMMM dd, yyyy"` (settings defaults).
  Upstream: github.com/blacksmithgu/obsidian-dataview. `[V]`
- Fileclass and Metadata Menu store Moment format strings (`YYYY-MM-DD`,
  `YYYY`, `DDDD`, `Q`). Fileclass has a separate token scanner.
  Its value-validation path bypasses that scanner for a string
  `dateFormat` (K8 in `k-citation-verification.md`).
  Sources: `obsidian_mdelobelle-fileclass-src-digest.txt:3082-3318`,
  `obsidian_mdelobelle-metadatamenu-src-digest.txt:11862-11931`. `[V]`
- Traces' `MOMENT_TOKENS` lacks `ww`, `Q`, `Qo`, and `DDDD`
  (`src/temporal/token.rs`). The owner uses `ww`, `YY`, `Q`, and `Qo`
  in `docs/vault/07_templates/12_01_week_periodic.md` and
  `14_01_quarter_periodic.md`.
  Metadata uses `YYYY`, `DDDD`, and `Q` in
  `docs/vault/05_metadata/cal.md`. `[V ticket]`

Product behavior: the inspected Obsidian tools use Moment for templates
and metadata formats. Dataview uses Luxon. These examples show format
syntax at tool boundaries. They do not prove a unique domain design.

Lesson: `[P]` Keep Moment compatibility at a syntax-adapter boundary.
Research 07 proposes that placement. Ticket 16 makes parity Free.
The owner's templates use `Q`, `Qo`, `ww`, and `YY`. The metadata uses
`DDDD`. Retention needs support for those tokens. Removal needs an
explicit migration, not an unsupported claim that no user needs them.

### Q2: Where does the Moment dialect translator live?

Evidence:

- Fileclass scans tokens without a Moment runtime dependency.
  `TOKENS` uses longest-first order. It includes week-year, quarter,
  and ordinal tokens (`dateFormatCheck.ts:3082-3318`).
  Its comment explains the failure: unknown format text prints unchanged.
  Thus `YYYY-KK-007` can write `2026-KK-007` without a format error. `[V]`
- Metadata Menu calls moment directly.
  Its settings show a live format example
  (`AbstractDate.ts:11884-11931`). `[V]`
- Minijinja contrib filters use strftime without Moment logic
  (research 07, recorded in ticket 30). `[V research]`
- Commit `6104d0ae` puts the translator in `src/temporal/token.rs`.
  It includes `pattern_to_strftime`, `MOMENT_TOKENS`, and the ordinal suffix.
  Config imports `DEFAULT_DATETIME_PATTERN` from temporal.
  Thus the `config -> temporal` edge already exists. `[V ticket]`

Product behavior: Templater and Metadata Menu use the runtime formatter.
Fileclass instead scans tokens without that runtime dependency.
Minijinja supplies a separate strftime interface.
These examples do not all include translation.

Lesson: `[P]` Keep the translator pure. Return an explicit
unsupported-token error. The template compatibility seam and temporal
remain placement options. The examples do not decide placement.
`[H]` Shared schema support for Moment would support a shared adapter.
It would not by itself prove that temporal must own the adapter.

### Q3: One dialect or a split across four format surfaces?

Evidence:

- IWE uses chrono format specifiers for keys and content.
  `[workspace]` stores key `date_format` and `time_format`.
  `[markdown]` stores the corresponding content fields.
  Defaults differ: `%Y-%m-%d` for keys and `%b %d, %Y` for content.
  Other examples use `%Y`.
  Sources: `iwe-org-iwe-src-digest.txt:844-896`, `:13744-13873`,
  `:9607-9733`, `:9888-9890`.
  Upstream: github.com/iwe-org/iwe. `[V]`
- Dataview uses Luxon across the inspected format surfaces.
  Settings include `defaultDateFormat` and `taskCompletionDateFormat`.
  Query functions include `dateformat(date, string)` and
  `date(text, "format")`.
  Sources: src `:6924-6928`; changelog `:366`, `:373-374`. `[V]`
- zk maps named styles to strftime.
  `{{format-date now}}` accepts
  `short|medium|long|full|year|time|timestamp|timestamp-unix`.
  Other strings pass through as strftime
  (`zk-src-digest.txt:2213-2316`).
  Upstream: github.com/zk-org/zk. `[V]`
- Traces query `date(text, pattern)` uses strftime through
  `DatePoint::parse_with_pattern`. Schema tests use Moment tokens.
  Config defaults to `%Y-%m-%dT%H:%M:%S`. `[V ticket]`

Product behavior: IWE and Dataview use one format family across their
inspected surfaces. IWE separates key and content format values.
This sample does not establish an ecosystem-wide dialect rule.

Lesson: `[P]` Use one native dialect across the four format surfaces.
Keep separate defaults where the surfaces need them. Treat template
Moment support as a compatibility adapter, not a fifth native dialect.
The schema tests already use Moment tokens. A change needs either a
Moment scanner or a documented migration of those tests.

### Q4: How does `date.format` treat a format with no full date?

Evidence:

- Fileclass separates token scanning from value validation.
  `validateDatePattern` bypasses pattern validation for a string
  `dateFormat`. The outer validator handles empty values and `required`.
  This bypass does not itself enforce a string type.
  Sources: `validate.ts:6079-6087,6107-6112,6153-6164,6191-6196`;
  K8 in `k-citation-verification.md`. `[V]`
- Dataview's literal parser needs a year and month.
  Its ladder is `YYYY-MM[-DDTHH:mm[:ss[.nnn]]][Zone]`.
  Bare `YYYY` does not match.
  Sources: docs `:1788`; `EXPRESSION.date` at src `:7913-7947`. `[V]`
- Dataview renders unparseable date text as `null`.
  Its typed comparisons do not supply a schema check. `[V]`
- Traces' `SchemaDateField.format` is an unvalidated `Option<String>`.
  Its accessor is test-only. Tests use `YYYY-MM-DD`.
  The owner's metadata declares `YYYY`, `DDDD`, and `Q`.
  Thus partial formats occur in real owner data. `[V ticket]`

Product behavior: Fileclass has a token scanner but bypasses pattern
validation on the string-format path. Dataview has no schema format
declarations. Neither example decides Traces partial-date validation.

Lesson: `[P]` Check declared tokens at schema load. Separate full-date,
partial-date, and no-date-component formats. Ticket 20's provisional
scalar acceptance applies only to the last category.

`YYYY-MM` has a Dataview literal precedent. Bare `YYYY`, `Q`, and `DDDD`
do not belong to its inspected literal ladder. `[H]` A token table can
classify quarter-of-year and day-of-year formats. The human must choose
which values those formats accept. The unrelated truncated ticket 20
fragment does not block this choice (L1/L2 in `l-answer-verification.md`).

### Q5: Fate of `format` under `[frontmatter.date_created]` and `[frontmatter.date_modified]`

Evidence:

- Each Traces table has `name` and `format`.
  The default format is `%Y-%m-%dT%H:%M:%S`.
  Ticket 22 records no production reader.
  Config has no `[date]` table at that inspection date. `[V ticket]`
- IWE uses per-surface date, time, and locale fields.
  Consumers call accessors such as `format_options()`
  (`:13744-13873`, `:65575`). `[V]`
- `obsidian-export` copies mtimes with `filetime::set_file_mtime`.
  Its `--preserve-mtime` flag is opt-in.
  Sources: `rust_zoni-obsidian-export-digest.txt:452-454`, `:2818-2839`.
  Upstream: github.com/zoni/obsidian-export. `[V]`

Product behavior: the inspected Traces stamp formats have no reader.
The export tool preserves filesystem mtime separately from note formats.

Lesson: `[P]` Compare moving the fields into `[date]` with removing the
unread fields. IWE supports the first shape. Ticket 22 records the
unread-config evidence. If the fields stay, identify a consumer.
If the fields go, record a change-table row. Ticket 16 makes config
formats Migratable.

## Q6 to Q8: Calendar helpers and filenames

### Q6: Which calendar helpers stay in `src/template/engine/date.rs`?

Evidence:

- Templater's helpers are template-layer and anchor-driven. `tp.date.now(format, offset,
  reference, reference_format)` takes some days or an ISO 8601 duration string (`"P-1M"`).
  `tp.date.weekday(format, weekday, reference, reference_format)` resolves a weekday against a
  locale-first week (`templater docs digest :171-207`, `:255-290`). The reference date is
  usually the note title, so the anchor is a template concept. `[V]`
- Metadata Menu shifts dates with `"90d"`-style strings parsed to `moment.duration`
  (`ValueModal.ts` area `:12032-12295`). `[V]`
- Dataview puts edge helpers in the query language, not the template: `date(sow)`, `date(eow)`,
  `date(som)`, `date(eom)` as function spellings of period edges (src `:7643-7666` `DATE_SHORTHANDS`;
  docs `:991`). `[V]`
- Traces facts: `PERIOD_EDGE_LOOKUP` and `resolve_period_edge` already live in
  `src/temporal/calendar.rs` and serve both query and template `[V ticket]`. `Anchor`,
  `apply_offset`, `weekday` remain in the engine `[V ticket]`.

Product behavior: anchors live where the request lives. Templates anchor to a file title or render
time. Queries anchor to the frame. Edge resolution is shared arithmetic.

Lesson: `[P]` Put shared arithmetic in temporal. Keep anchor selection at
each surface. The engine keeps thin wrappers for the render frame.
Only operations that need a date acquire the frame's zone-derived `today`.
Ticket 31 needs wrapper signatures after the human decides this boundary.

### Q7: Where does the filename-period parser live?

Evidence:

- Traces facts: `FileMeta::derive_day` in `src/file.rs` reads the base name with
  `DatePoint::parse_prefix`, falls back to creation time, and stores `day`; ticket 21 retires
  `file.day`. `scan_date_prefix` in `src/temporal/lexer.rs` knows `YYYY-MM-DD` and `YYYY-MM`
  only and never yields an instant. Ticket 21 adds ISO weeks, quarters, and an opt-in bare year,
  scoped to filenames; vault-format parsing stays frozen. `[V ticket]`
- Metadata Menu builds link paths from patterns: `{{moment}}` tokens expand inside the path and
  literal text stays literal (`ValueModal.ts` `buildPath`). Its code records the failure when a
  raw moment format hits a whole path: `Daily` becomes `D` = day-of-month, `a` becomes am/pm.
  `[V]`
- zk writes filename stamps from templates (`{{format-date now 'timestamp'}}` -> `200911172034`,
  `zk-src-digest.txt:2019-2020`). It has no filename parser. `[V]`
- Dataview's `extractDate` searches the whole filename for its date patterns, and `findDate`
  checks typed metadata fields before that; `date(link)` falls back to `resolved.file.day`
  (src `:6872-6890`). `[V]`

Product behavior: the inspected Dataview helper finds full dates anywhere
in a filename. It does not recognize week or quarter labels.
Metadata Menu and zk show pattern-based filename writing.
These excerpts do not prove that no other tool parses filename periods.

Lesson: `[P]` Put the filename-scoped period parser in temporal.
Keep the new filename forms out of frozen note parsing. Preserve the
explicitly accepted civil and inline-datetime changes (L3).
Recognize only explicit prefix forms. Never reinterpret a malformed
longer prefix as a shorter one. Ticket 21 already accepts that rule.

### Q8: Week-one rule for a custom week start

Evidence:

- Moment separates week start from week-one rule in locale data: `dow` (first day) and `doy`
  (day of year that decides week one). Its docs state that lowercase week tokens
  (`gggg`, `w`, `e`) obey locale week rules. Uppercase tokens (`GGGG`, `W`) obey ISO week rules.
  Tokens are case-sensitive (momentjs.com/docs, week-year section; checked on the web).
  `[V web]`
- Obsidian Tasks forces ISO weeks for ranges: `buildRelative('week')` maps `week` to `isoWeek`,
  with the comment "Treat all weeks as ISO 8601 weeks" (`DateRange.ts:5043-5141`). Its numbered
  ranges parse `YYYY-W10` with `startOf('isoWeek')`/`endOf('isoWeek')`
  (`DateParser.ts:5016-5037`). `[V]`
- nldates reads the week start from `window.moment.localeData()._week.dow`, a private API, with a
  `locale-default` setting (`utils.ts:852-860`; `parser.ts:456-589`). `[V]`
- Dataview's `sow`/`eow` call Luxon `startOf("week")`/`endOf("week")` with no options. Luxon's
  default week is ISO (Monday start; `useLocaleWeeks` defaults to false), so the runtime
  locale does not decide it (src `:7643-7666`). `[V]`
- Traces facts: `CalendarPeriod::Week` is the ISO week, Monday to Sunday; ticket 21 records that
  ISO `2026-W01` starts `2025-12-29` and that ISO `2025-W53` is invalid `[V ticket]`.

Product behavior: Tasks and the inspected Dataview calls use ISO weeks.
nldates reads Obsidian locale data through a private field.
Moment exposes both ISO and locale rules.

Lesson: `[P]` Define the week-one rule if configurable starts affect week
numbering. A pair such as `dow` and `doy` can express that policy.
The fields do not have to change together (K4).
Ticket 21 keeps ISO filename weeks stable under general week settings.
`[H]` Fixed ISO is the smaller alternative. Avoid an implicit device-locale
read, which can change labels without a config change.

## Q9 to Q12: Parsing stack

### Q9: Does `speedate` stay?

### Q10: Does `winnow` stay?

### Q11: Does `logos` stay?

### Q12: Does `jiff` stay?

Evidence, shared:

- IWE parses dates with one crate, chrono, and hand-rolls the rest: `NaiveDate::parse_from_str`
  for format candidate checks (`:61875`), a hand-written `is_iso_datetime` scanner for query
  typing (`:76396-76482`), and adversarial tests for multibyte timezone tails (`:76926-76939`).
  `[V]`
- IWE formats with chrono locale support and normalizes locale strings itself: strips
  `.UTF-8` and `@euro`, converts `de-DE` to `de_DE`. An unconfigured locale takes the system
  locale; an invalid configured locale falls directly to `Locale::POSIX`
  (`locale.rs:54931-54990`). `[V]`
- zk (Go) layers three date capabilities over stdlib time: a strftime formatter
  (`lestrrat-go/strftime`), a natural-language parser (`naturaldate`), and named styles. `[V]`
- Traces facts: `speedate` has one call site in `src/temporal/date.rs`. `winnow` has one call
  site in `src/temporal/lexer.rs`. It also locates duration prefixes for inline fields that reach
  the store. `logos` serves three temporal files plus the shared lexer, note parser, and query
  grammar. `jiff` is pinned `=0.2.38` with `std`, `tz-system`, `tzdb-bundle-always`. The
  extractor fingerprint hashes resolved versions of `jiff` and `speedate`; ticket 25's list omits
  `winnow`. `[V ticket]`

Product behavior: IWE combines chrono with hand-written scanners.
Its scanner tests supply an adversarial-test precedent, not complete
Unicode safety. zk uses separate formatting and natural-date libraries.

Lesson: `[P]` Evaluate parser coupling and persisted producers.
Removing `speedate` changes ticket 25's extractor fingerprint.
Removing direct `winnow` use needs a producer-list update.
Keeping that use needs correction of the current omission.
Keeping `logos` serves the listed lexer consumers.

`[A]` The digest excerpts contain no crate measurements. Use the separate
crate evidence for probe results. Do not treat a probe binary delta as
a repository measurement or acceptance of rebuild cost.

Critic note: the inspected excerpts contain no speedate or winnow user.
That absence is weak evidence for removal. State this limit in the grilling.

## Q13 to Q18: Natural-language dates

### Q13: The grammar

Evidence:

- nldates delegates to `chrono-node`, an open grammar, then patches it. Custom parsers map
  `Christmas` to Dec 25 and ordinal days to the current month. `getLocalizedChrono` switches
  casual-config by `window.moment.locale()`. `getParsedDate` reads chrono's `impliedValues` to
  decide `weekdayIsCertain` and picks a reference date from it. Two-stage anchors parse `next
  month` by parsing the anchor period, then the phrase against it. `mid X` becomes `X 15th`.
  `last day of X` and `end of X` become month ends (`parser.ts:456-589`). `[V]`
- Dataview's `DATE_SHORTHANDS` is a closed list: `now`, `today`, `yesterday`, `tomorrow`,
  `sow|start-of-week`, `eow|end-of-week`, `som`, `eom`, `soy`, `eoy` (src `:7643-7666`). The
  parser picks `dateShorthand` or the ISO ladder, longest key first (`:7907-7912`,
  `:7950-7954`). `[V]`
- Ticket 19 leaves natural-language dates as a hook. Its closed English
  grammar is a proposal, not an adopted grammar (L11). `[V ticket]`

Product behavior: Dataview uses a closed shorthand list.
That list still reads an ambient clock. nldates uses a broader grammar
with locale settings and weekday inference. A closed list alone does
not prove determinism (K17 in `k-citation-verification.md`).

Lesson: `[P]` Keep the hook-only alternative. If the human adopts a grammar,
consider a closed list with pinned inputs. Decide coverage separately
from implementation ownership.
The nldates examples supply candidate phrases, not mandatory scope.
`[H]` The inspected owner templates show no `Christmas`-style words.
That absence supports a smaller proposal, not a universal user requirement.

### Q14: Build in the repository or use a crate?

Evidence:

- zk's helper logs parsing errors and returns the utility's returned time.
  Its default branch returns an initialized zero `time.Time`.
  The inspected helper does not establish every utility error return
  (`date.go:2233-2251`; K19). `[V]`
- zk hard-exits the process when a renamed helper is used with the old spelling
  (`date.go:2243-2246`, `os.Exit(1)`). `[V]`
- nldates logs `"Invalid date"` at debug level and continues (`main.ts:414-426`). `[V]`
- Dataview's `date()` returns `null` on a bad string and throws on a bad format argument
  (`:6865-6909`). `[V]`
- Ticket 30 records `fuzzydate` 0.4.1 and `jiff-english` 0.0.1 as candidates.
  The historical fuzzydate snapshot reports MIT, ~2.6k lines, 14 releases,
  and a chrono `clock` dependency.
  The English snapshot reports MIT, ~1.3k lines, one release, 29 downloads,
  `jiff ^0.2` defaults, and `scanlex`.
  These counts do not establish maintenance quality.
  Traces disables Jiff defaults. The probe enables them through English
  dependency unification (`d-crate-evidence.md`). `[V ticket, probe]`
- The recorded candidate matrix returns points, not periods.
  `jiff-english` rejects `next week`. `fuzzydate` returns a +7-day point.
  `last friday` differs between the candidates.
  Neither accepts `eom` or `in 2 weeks`.
  Indicative timings are below a microsecond for the measured phrases.
  These results are not controlled or production benchmarks
  (`probes/candidates`; `m-gap-closure.md`). `[V]`
- `.scratch/temporal-core/design-nl-dates.md` proposes an in-house parser; ticket 30 records four
  conflicts with decided rules. `[V ticket]`

Product behavior: zk's inspected default branch returns a zero time.
nldates logs at debug level and continues. Dataview returns typed `null`
for failed string parsing and can throw for custom-format parsing.
These are specific error paths, not a complete integration inventory.

Lesson: `[P]` Compare crate behavior with the chosen grammar and error
contract. The proposed contract needs span-carrying errors.
It permits no zero-date fallback, process exit, or debug-only silence.
`[H]` Prefer an in-repository grammar unless a candidate meets that
contract. Keep deferral valid because the grammar itself is not adopted.

The bounded matrix establishes specific coverage gaps.
Neither candidate returns periods, and `jiff-english` rejects `next week`.
The matrix does not settle complete coverage, all runtime behavior,
or maintenance quality. Release and download counts alone cannot decide
adoption.

### Q15: Meaning of a bare weekday

Evidence:

- Templater documents the locale rule: "If the locale assigns Monday as the first day of the
  week, `0` will be Monday, `-7` will be last week's day" (`tp.date.weekday`, docs `:255-290`).
  `[V]`
- nldates resolves bare weekday names through chrono and controls past-vs-future with chrono's
  `forwardDate` flag; `weekdayIsCertain` decides the reference date (`parser.ts:456-589`).
  `[V]`
- Dataview has no bare weekday spelling; `date(sow)` and friends are the only week words. `[V]`
- Traces fact: `date.weekday(n)` selects ISO weekday `n` of the current week `[V ticket]`.

Product behavior: the examples implement different operations.
Templater indexes a locale week. nldates controls forward selection with
a parser flag. Traces indexes the current ISO week.
These APIs do not decide one bare-weekday rule for Traces.

Lesson: `[P]` State the bare-weekday rule explicitly if the human adopts
weekday phrases. Next, nearest, and current-week remain choices.
`[H]` Current-week selection can use the pinned week rule and match the
existing template filter. Define `next` and `last` separately.
The owner's preference remains unknown. Ask during the grilling.

### Q16: Interval precision of `next week`

Evidence:

- Tasks models period phrases as ranges. `this week` builds `startOf(week)`..`endOf(week)`
  inclusive at day precision. `YYYY-W10` builds an isoWeek range. Moving a range by a month
  re-derives the end with `endOf(month).startOf('day')` because month lengths differ. The
  honest comment: "subtracting a month from a week long range was not tested"
  (`DateRange.ts:5043-5141`, `DateParser.ts:4953-5037`). `[V]`
- Dataview splits the two bounds into two spellings: `date(sow)` and `date(eow)` are separate
  point functions; there is no `next week` value (src `:7643-7666`). `[V]`
- nldates parses `next week` through chrono with `forwardDate: true`; chrono resolves it to
  the configured first weekday of the next week, a point, not a span. `next month` uses a
  two-stage anchor. `[V]`
- Ticket 21: `CalendarPeriod` becomes a normalized civil start plus a `CalendarUnit`; tickets 19
  and 21 need period values for `file.date`; periods match a span with an exclusive upper bound.
  `[V ticket]`

Product behavior: three shapes exist: span value (Tasks), two point spellings (Dataview), and
flag-driven point (chrono). Tasks' start-plus-end shape pays an end-adjustment tax on arithmetic.

Lesson: `[P]` Derive bounds from normalized start plus unit, as ticket 21
accepts. Document the exclusive upper-bound derivation for each unit.
Fileclass's separate stored-value and display paths are an analogy,
not a proof of period arithmetic.

### Q17: Filter spelling of relative keywords

Evidence:

- Traces facts: the grammar has `sow`, `eom`, `eoy` as functions with a date argument and no bare
  relative keyword; a bare word parses as a field path, so bare `today` names a field. `[V
  ticket]`
- Dataview never spells relative words bare. They are arguments of `date(...)`. Its identifier
  rule is permissive. A name can start with any letter or emoji, then letters, digits, `_`, `-`,
  and emoji. A bare keyword would collide with field names (`EXPRESSION.identifier`, src
  `:7853-7858`). `[V]`
- Dataview reserves `FROM`, `WHERE`, `LIMIT`, `GROUP`, `FLATTEN` as keywords and nothing else
  (src `:7668-7672`). `[V]`

Product behavior: call syntax (`date(today)`) is the established spelling that avoids the
field-name collision. Ticket 29 already uses `eom` as a relative keyword in one place while the
grammar defines it as a function; ticket 30 lists that discrepancy.

Lesson: `[P]` Consider relative words as `date(...)` arguments.
This Dataview precedent is not the existing Traces relative-date grammar.
Traces currently has no such relative keyword.
Bare keywords need an explicit field-name collision rule and migration.
Keep this spelling proposal separate from adopting a grammar.

### Q18: Frontend spelling of `member_of`

Evidence:

- `[A]` The excerpts show no directly equivalent membership syntax.
  Dataview's task shorthand `📅` marks a date field
  (docs `:1599-1640`). It is storage syntax, not a query operator. `[V]`
- Fileclass's recorded failure is adjacent: a Bases `contains` filter on a link-stored scalar
  silently drops the note from the view (an upstream code comment records it; this task did not
  measure it). Plain scalar values still work like lists for several comparison operators. The
  validation messages in Fileclass teach the storage shape to prevent this. `[V]`
- Ticket 29 reserves only the IR name. `[V ticket]`

Product behavior: no precedent. The nearest lesson is negative: silent membership mismatches hide
rows.

Lesson: `[P]` Keep the hook-only and IR-reservation alternatives.
If the human adopts a frontend spelling, define its relation first.
Consider a notice for the specific link-scalar mismatch.
Do not infer a general scalar-versus-list rule from that upstream comment.

Source patterns and proposed grammar implications:

- Dataview treats bare `today` as a field outside its `date(...)` syntax.
  Traces also parses a bare word as a field path. `[V]`
- Dataview sorts shorthand keys and duration units longest-first.
  An adopted Traces grammar needs an explicit overlap rule.
  These examples do not prove that every grammar must use the same
  implementation. `[V, P]`
- Dataview's `date(link)` display-first branch is an ambiguity by design; see Q25.

## Q19 to Q25: Value shapes

### Q19: Civil and instant type shape

Evidence:

- Tasks normalizes every parsed date to local midnight with the stated reason: "Using start of
  day to correctly match on comparison with other dates (like equality)" (`DateParser.ts:
  4934-4944`). `[V]`
- Dataview has one Luxon `DateTime` type; naive ISO input gets the runtime zone silently. Its
  parser asserts `isValid` and rejects invalid ladders (`EXPRESSION.date` `:7913-7947`). `[V]`
- The superseded Traces glossary line "a Date promotes to local midnight" matches the Tasks
  hack. `[V ticket]`
- Traces facts: `DateValue` 4 bytes, `DateTimeValue` 16 bytes, `DurationValue` 72 bytes,
  `NoteFieldValue` capped at 80 bytes; a new civil variant must keep the cap. `[V ticket]`

Product behavior: the inspected Tasks and Dataview paths do not separate
civil datetimes from instants. Tasks' midnight-normalization comment
exposes a portability risk. It does not document a measured cross-machine
equality failure (K6).

Lesson: `[P]` Preserve the accepted Traces civil/instant distinction.
The digest examples show risks, not a representation proof.
The later default-test-profile mirror measures 80 bytes.
It substitutes a boolean for `LinkType` and a tuple for `DurationValue`.
It does not measure the actual proposed product types
(`m-gap-closure.md`). The human still chooses the representation.

### Q20: Where does `CalendarPeriod` sit in the value model?

Evidence:

- Tasks' `DateRange` is a first-class filter value with two mutable Moment endpoints
  (`DateRange.ts:5043-5141`). `[V]`
- Dataview has no period value in the read set; period edges are functions that return points.
  `[V]`
- Fileclass stores canonical duration strings and derives readings beside them
  (`humanDurationsFor`, `duration.ts:3611-3649`). `[V]`
- Ticket 21 renames the kind-only period to `CalendarUnit` and makes `CalendarPeriod` a
  normalized civil start plus unit; ticket 18 has no period variant and an 80-byte cap. `[V
  ticket]`

Product behavior: span values exist in one tool, as endpoint pairs. The canonical-store,
derived-display pattern exists for durations.

Lesson: `[P]` Represent each period with normalized start plus unit,
as ticket 21 accepts. Complete the one-runtime-value model from ticket 18.
A separate query runtime carrier needs an explicit amendment (L4).
The absence of period syntax in note fields does not justify another
runtime carrier.

`[H]` Endpoint pairs would add Tasks' end-adjustment work.
The start-plus-unit mirror fits the 80-byte limit.
That mirror does not prove the actual product layout.

### Q21: Date versus instant at an ambiguous or absent local midnight

Evidence:

- `[A]` No read tool handles an ambiguous or skipped local midnight. Dataview asserts validity
  of parsed literals only (`:7946`). `[V]`
- Fileclass avoids the question in duration math with floating UTC: "UTC math sidesteps DST";
  unparseable inputs return `null` (`duration.ts:3620-3688`). `[V]`
- Ticket 21 evidence: a skipped next day such as `2011-12-30` in `Pacific/Apia` must not
  invalidate the real day before it; period-versus-instant tests the civil date of the instant
  in the pinned zone and never tests midnight existence. `[V ticket]`

Product behavior: these excerpts do not establish an ordinary
Date-versus-instant gap/fold comparison rule.

Lesson: `[P]` Keep ordinary field comparison separate from period
membership. Ticket 21 already fixes membership through the instant's
civil date in the pinned zone.
Fileclass's UTC arithmetic does not answer the ordinary comparison
question. A calendar-day shift across DST can differ from a fixed
24-hour shift. Do not claim that every civil operation differs.

### Q22: Total sort key for mixed temporal values

Evidence:

- Dataview's `compareValue` orders types by `localeCompare`.
  Dates compare by epoch. Durations use a fixed conversion matrix after
  `shiftToAll().normalize()`. The matrix assigns 30 days per month and
  365 days per year. Luxon equality also considers fields and metadata.
  These reads do not prove comparator laws for every equal numeric value
  (`src`, comparison and binaryop modules; K9). `[V]`
- `[A]` The excerpts supply no normalized start-plus-unit period sort key.
  Tasks' endpoint ranges are not that representation.
- Ticket 29 section 3 fixes the group class order as Civil, Instant, Period; the owner
  confirmed it (B65/C3). This ticket keeps the mixed temporal value sort key and the period
  unit tie-break. `[V ticket]`

Product behavior: Dataview compares its own types.
Its comparator supplies no Traces period-order proof.

Lesson: `[P]` Preserve the accepted Civil, Instant, Period group order
(B65/C3). Define the complete value-sort key and period unit tie-break.
State how that key relates to group keys.
Do not infer its laws from Dataview's comparator (K9; L6).

### Q23: Nominal duration equality and order

Evidence:

- Fileclass excludes year and month designators.
  Its input regex permits weeks mixed with days and time.
  It captures a sign that `parseDuration` does not apply.
  `buildDuration` folds mixed weeks into days.
  `durationMs` uses `86_400_000` milliseconds per day.
  These are source findings, not executed examples
  (`duration.ts:3499-3688`; K8). `[V]`
- RFC 5545 instead permits weeks alone, or days and time, or time alone.
  It calls weeks and days nominal durations.
  It excludes years and months (K7). `[V]`
- Dataview includes years, months, and weeks in `DURATION_TYPES`
  (src `:7602-7640`; changelog `dur(2yr8mo12d)`).
  Its comparator uses a fixed matrix after normalization.
  It assigns 30 days per month and 365 days per year.
  Equality also considers shifted unit fields and locale.
  This is a conversion convention, not proof of calendar equivalence
  (K9). `[V]`
- Ticket 19 keeps nominal equality and order provisionally and asks whether to refine. `[V
  ticket]`

Product behavior: Fileclass uses fixed-day milliseconds.
Dataview uses a fixed conversion matrix for more units.
Neither conversion follows from excluding years and months.
The RFC still treats weeks and days as nominal units.

Lesson: `[P]` Compare retaining ticket 19's nominal-seconds rule with
an explicit refinement. Excluding years and months is a separate scope
choice. It does not make calendar-day equality trivial.
Define unit conversion, equality, order, and arithmetic separately.
Do not adopt Fileclass's permissive input grammar or ignored sign as
an RFC requirement.

### Q24: How documents describe calendar arithmetic

Evidence:

- Tasks records month-shift end adjustment and an untested week-range case
  (`DateRange.ts:5108-5140`). `[V]`
- Dataview teaches arithmetic with `date + duration`,
  `this.release-date - date(now)`, and rendering examples
  (docs `:1842-1858`). `[V]`
- IWE gives a "Date format patterns" table (digest `:9888-9890`). `[V]`

Product behavior: these sources use examples, tables, and explicit limits.

Lesson: `[P]` Separate accepted rules, operation-specific caveats, and
unresolved cases. Skipped-day period membership already has an accepted
rule. Do not label every skipped-day operation undefined.
Ticket 34 owns any adopted caveat-notice mechanism.

### Q25: How `date(link)` reads a Markdown link target

Evidence:

- Dataview's cascade order is display text first, then path, then the file's derived day (src
  `:6872-6890`). A user-chosen alias can therefore produce the date. `[V]`
- Fileclass expands patterns inside link paths and keeps literal text literal
  (`expandLinkPath`, `dateFormatCheck.ts` area `:3180-3318`). `[V]`
- Ticket 19 accepts last-segment target extraction and a pattern argument.
  Its later Markdown-link handoff leaves percent decoding and extension
  normalization open (`19:299-313`; L, remaining human choices).
  External URLs are not note targets. This ticket chooses the pattern
  dialect. `[V ticket]`

Product behavior: display-first is the documented hazard. Path-token expansion is the precedent
for pattern-in-target.

Lesson: `[P]` Keep target extraction independent of display text.
Consider the same pattern dialect as `date(text, pattern)` (Q3).
The digests supply no percent-decoding runtime evidence.
RFC component separation and the later handoff keep encoded separators
as a separate decision (K22; L).

## Q26 to Q32: Configuration and zone

### Q26: `[date]` table syntax and validation

Evidence:

- IWE's config uses TOML tables with strict field sets; an unknown field error lists the exact
  expected names (`unknown field refs_extention, expected one of ...`, digest `:1649`). `[V]`
- Ticket 22 decides: `config` owns and parses `[date]` into `DateConfig`; `deny_unknown_fields`
  stays; merge is field-by-field global then local; no `version` key. `[V ticket]`

Product behavior: strict tables with name-listing errors are the IWE precedent.

Lesson: `[P]` Define the exact field set (Q27).
Check each field at the chosen validation stage.
Let unknown-field errors list the expected names.
The IWE typo `refs_extention` shows why diagnostic text needs care.

### Q27: Fields of config-owned `DateConfig`

Evidence:

- IWE's per-surface fields: `date_format`, `time_format`, `locale` for keys and for content,
  each with defaults and fallbacks (`:9607-9733`, `:13744-13873`). `[V]`
- nldates settings: `weekStart` (default `locale-default`) plus parser toggles. `[V]`
- Fileclass keeps formats per field, not global. `[V]`
- Ticket 19's hand-off lists candidate fields: default date and datetime formats, week start,
  weekday resolution, NL grammar options, zone source. `[V ticket]`

Product behavior: format pairs (date and datetime), week rule, locale, and NL toggles are the
field vocabulary across tools.

Lesson: `[P]` Give each adopted field a default and a validation rule.
Choose spelling checks separately from runtime acquisition.
`[H]` Use one format dialect with per-surface defaults only where a named
consumer needs a different format.

### Q28: Temporal-owned policy without importing config

Evidence:

- IWE parses config into a `Configuration` and engines consume accessors (`format_options()`),
  so the config crate keeps the TOML knowledge (`:13744-13873`, `:65575`). `[V]`
- Traces already has a `config -> temporal` edge.
  Retaining that direction is a ticket 30 proposal, not a new accepted
  decision. `[V ticket]`

Product behavior: parse in config, consume as plain values.

Lesson: `[P]` Pass plain policy values from config into temporal.
Keep TOML parsing in config. Do not make temporal import config.
The existing edge and IWE accessors support that direction.

### Q29: Configured zone syntax and validation

Evidence:

- `[A]` The inspected excerpts show value-literal zones, not a zone config
  policy. Dataview's bracket-zone regex is `[0-9A-Za-z+-\/]+`.
  It excludes underscores, including `America/New_York`.
  The offset branch accepts one or two hour digits and optional
  colon-minute digits. It does not establish `+0200` support.
  Sources: src `:7928-7945`; K14. `[V]`
- IWE normalizes locale strings. It removes `.UTF-8` and `@euro`.
  It maps `de-DE` to `de_DE`. An invalid configured locale selects
  `POSIX`. Only an absent configured locale permits a system-locale read
  (`locale.rs:54931-54990`). `[V]`

Product behavior: Dataview supplies limited value-zone syntax.
IWE supplies a locale-normalization example, not a zone-validation proof.

Lesson: `[P]` Consider IANA names and fixed offsets.
Choose accepted spellings and validation timing separately.
Keep spelling checks distinct from runtime zone acquisition.
Preserve lazy resolution for civil-only work (L9).
No digest excerpt decides the exact accepted zone set.

### Q30: Host system zone and failure reporting

Evidence:

- `[A]` The inspected excerpts show no zone-acquisition error contract.
  They do not prove that every host has complete ICU data.
- Anti-precedents for platform reads: `obsidian-rs` chains `unwrap()` on file
  `metadata().created()` (`rust_adamisrael-obsidian-rs-digest.txt:509-523`); basalt stores an
  optional timestamp with a silent `0` fallback (commit note in
  `rust_erikjuhani-basalt-digest.txt:9547`, field at `:11007-11008`). `[V]`
- Ticket 21 decides: memoized failure, typed, host can select UTC explicitly, UTC never implicit.
  `[V ticket]`

Product behavior: unwrap chains and zero sentinels are the observed failure styles.

Lesson: `[P]` The typed, memoized failure is new ground with two named anti-precedents to avoid.
Ticket 34 needs the error variant from this ticket.

### Q31: Is the week start configurable?

Evidence: see Q8. The same evidence splits: nldates configurable, Tasks fixed ISO, Dataview
Luxon-default (ISO unless `useLocaleWeeks` is set), Moment paired rules. `[V]`

Lesson: `[P]` State which operations use a configurable week rule.
Week numbering needs a week-one rule. `eom` and `eoy` are month/year
operations, not week-start consumers.
Preserve accepted ISO filename prefixes independently of general settings
(L10). Do not infer a numbering policy from a display-week start alone.

### Q32: Opt-in key for bare-year filename prefixes

Evidence:

- `[A]` No read tool parses year-only filename prefixes.
- The inspected Dataview literal ladder starts at `YYYY-MM`, not `YYYY`
  (docs `:1788`; K15). Traces' `scan_date_prefix` accepts `YYYY-MM-DD`
  and `YYYY-MM`, never an instant.
  Ticket 21 adds filename quarters and an opt-in bare year. `[V]`

Product behavior: absent.

Lesson: `[P]` The opt-in key obeys the ticket 22 config rules (exact field set, deny
unknown). `[H]` A boolean key is enough. The parser change is the real content. Ticket 25's
`IndexPolicy` rule applies only if the index persists parsed periods.

## Q33 to Q36: Frame and pushdown

### Q33: What temporal exports to the execution frame

### Q34: Replacements for ambient reads

Evidence, shared:

- The inspected Dataview shorthand table calls `DateTime.local()` during
  parsing (src `:7950-7954`). nldates uses `window.moment()` and runtime
  locale data. Tasks uses `window.moment()` in `DateParser`. `[V]`
- These paths show ambient inputs. They do not prove absence of injection
  APIs throughout the complete upstream projects.
- Ticket 21 accepts one frame per query, render, or request.
  The frame pins the clock and week rule. It acquires a required zone
  lazily and retains success or failure.
  Remove `local_zone()`, `now_instant()`, and `ZoneGuard` after injection
  replaces their uses. `[V ticket]`

Product behavior: the named relative-date paths read ambient inputs.
Those inputs create replay and cross-device risks.
The excerpts do not measure failures in every tool.

Lesson: `[P]` Export the inputs that operations need.
Keep one clock sample, the week rule, and a lazy zone resolver.
Derive `today` only when an operation needs the selected zone.
Pass the frame through anchor wrappers (Q6).
Do not force zone acquisition for civil-only operations.

### Q35: Temporal probe bounds per operator and value kind

Evidence:

- `[A]` No read tool has a storage layer with pushdown; plugin queries scan memory. Dataview's
  boundary note "only indexed data is available in a Dataview query" is the nearest concept. It
  is an availability rule, not a bound proof (docs `:1250`). `[V]`
- Ticket 29: unproved mappings scan; the first pushdown scope is unquoted temporal literals plus
  `And`. `[V ticket]`

Product behavior: absent.

Lesson: `[P]` Provide the proof per operator and kind, or leave `Unsupported` and scan. The
digests add no evidence here; the grilling decides from ticket 24's key rules and the type
shape answers (Q19, Q20, Q22).

### Q36: When a Null date gets a notice

Evidence:

- Fileclass records the silent-drop failure (upstream comment, not measured here): a
  `contains` filter on link-stored scalar values skips them, so notes vanish from a view with
  no message. Its validation messages then teach the storage shape (`dateFormatCheck` message
  design, `[V]`).
- nldates logs `"Invalid date"` at debug level and continues (`main.ts:414-426`). `[V]`
- Dataview yields `null` for unparseable `date()` input with no notice (`:6865-6909`). `[V]`
- Ticket 34 owns the mechanism; ticket 29 did not adopt the first-round notice. `[V ticket]`

Product behavior: the named date-error paths often provide no user notice.
That sample does not establish a universal silence rule.

Lesson: `[P]` Define the notice condition with ticket 34.
Consider malformed date-shaped input and the specific link-scalar
membership failure. Do not infer notices for every absent date.
Memoized resolution failure does not guarantee one emitted notice.
Delivery and deduplication remain ticket 34 concerns.

## Analogies that do not transfer

Each entry states the failure that disqualifies it as a Traces pattern.

- zk's zero-time fallback in the `naturaldate` helper's default branch (`date.go:2238-2242`):
  a zero date silently joins comparisons and sorts as an epoch value. `[V]`
- nldates' private-API locale read (`moment.localeData()._week.dow`): week labels shift when the
  host locale changes, and the API can vanish. `[V]`
- Dataview's display-first `date(link)` cascade: an alias decides the value. `[V]`
- Tasks' mutable start-plus-end range with end re-derivation on shift: arithmetic must patch two
  endpoints per unit, and its own comment admits untested combinations. `[V]`
- Fileclass's duration parser: ignored signs and mixed-week input are not
  a safe RFC 5545 validation model (K8). `[V]`
- Metadata Menu's raw moment format over whole link paths: literal text mangles (`Daily` ->
  day-of-month). Token expansion must be explicit. `[V]`
- `obsidian-rs` unwrap chains and basalt's `0` sentinel: platform reads need typed failures,
  not crashes or silent zeros. `[V]`
- Implicit locale week policy: device locale can change week labels.
  A fixed explicit locale or week rule removes that ambient dependence.
  The inspected Moment APIs support both week families. `[V web]`

## Question-to-evidence map

| Q | Evidence sources | Verdict for grilling |
| --- | --- | --- |
| 1 | Templater docs/src; Dataview src; Fileclass src; owner files | Compat surface, not domain concept `[P]` |
| 2 | Fileclass dateFormatCheck; research 07; ticket code | Pure adapter; placement still open `[P]` |
| 3 | IWE config; Dataview surfaces; zk styles; ticket facts | One dialect, per-surface values `[P]` |
| 4 | Fileclass validate + lint; Dataview ladder; owner `Q`,`DDDD` data | Schema-load lint; precision-aware value check `[P]` |
| 5 | Ticket 22 unread-config facts; IWE shape; obsidian-export | Join `[date]` or remove; needs owner use case `[P]` |
| 6 | Templater weekday/offset; Dataview edges; Traces calendar.rs | Shared arithmetic in temporal; anchors per surface `[P]` |
| 7 | Traces derive_day; Metadata Menu path patterns; zk stamps | Temporal-owned filename parser, period yield `[P]` |
| 8 | Moment dow/doy; Tasks ISO; nldates locale read | Specify numbering policy; general settings do not reinterpret ISO prefixes `[P]` |
| 9-12 | IWE chrono; zk layered crates; ticket fingerprint facts | Decide by coupling and fingerprint cost, not style `[P]` |
| 13 | nldates patches; Dataview ambient closed shorthands; ticket 19 hook | Closed list is a proposal; hook-only remains valid `[P]` |
| 14 | Candidate matrix; source APIs; recorded error shapes | Compare contract and coverage; bounded probes do not settle adoption `[P]` |
| 15 | Locale indexing; forward flag; current-week filter | Choose bare weekday semantics explicitly `[P]` |
| 16 | Tasks ranges; Dataview points; ticket 21 | Normalize periods if phrases are adopted `[P]` |
| 17 | Traces field paths; Dataview date arguments | Consider date arguments; grammar adoption remains open `[P]` |
| 18 | IR reservation; specific link-scalar failure | Define relation before syntax; reservation-only remains valid `[P]` |
| 19 | Tasks midnight comment; Dataview type; limited layout mirror | Preserve civil behavior; choose and measure actual representation `[P]` |
| 20 | Ticket 18 runtime owner; ticket 21 normalized period | Complete one runtime carrier; keep parser acceptance separate `[P]` |
| 21 | Ticket 21 membership; ordinary field comparison gap | Decide ordinary gap/fold rule separately `[P]` |
| 22 | Dataview comparator limits; ticket 29 group order | Define full value-sort key and unit tie-break `[P]` |
| 23 | Fileclass permissive parser and ignored sign; Dataview fixed matrix | Retain or refine nominal seconds explicitly `[P]` |
| 24 | Tasks code comments; Dataview docs; IWE specifier table | Three-list documentation model `[P]` |
| 25 | Display-first risk; path-token example; K22/L | Decide decoding, encoded separators, and extension treatment `[P]` |
| 26 | IWE strict tables and error text; ticket 22 rules | Exact field set; name-listing errors `[P]` |
| 27 | IWE fields; nldates settings; ticket 19 hand-off | Field list per hand-off; defaults and validation `[P]` |
| 28 | IWE accessors; existing config-to-temporal edge | Plain policy value; no new edge `[P]` |
| 29 | Limited Dataview zone grammar; IWE locale example; L9 | Choose spellings and timing; preserve lazy acquisition `[P]` |
| 30 | Anti-precedents (unwrap, zero); ticket 21 rules | Typed memoized failure; feed ticket 34 `[P]` |
| 31 | Q8 evidence; L10 | Define configurable operations; keep ISO filename weeks stable `[P]` |
| 32 | Year-month literal precedent; ticket 21 opt-in | Boolean opt-in proposal under ticket 22 rules `[P]` |
| 33-34 | Named ambient paths; ticket 21 frame rules | Clock, week rule, lazy zone; derive today on demand `[P]` |
| 35 | Absence; Dataview indexed-data note; ticket 29 scope | Prove or leave Unsupported `[P]` |
| 36 | Specific silent failures; ticket 34 ownership | Choose notice conditions; keep delivery and deduplication separate `[P]` |

Open items this file does not settle, by design:

- Crate decisions need producer-list and cost evidence beyond these digests.
  The probe results are bounded snapshots, not production measurements.
- Candidate source and runtime limits appear in `d-crate-evidence.md`.
- Markdown-link decoding, encoded separators, and extensions remain choices.
  Digest excerpts supply no runtime proof for those rules.
- The owner's bare-weekday preference remains a grilling question.
