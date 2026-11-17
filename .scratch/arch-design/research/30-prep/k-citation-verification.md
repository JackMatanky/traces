# Citation checks for temporal preparation

## Scope and method

This report checks material claims in `c-digest-lessons.md` and
`e-external-designs.md`. It also identifies affected claims in
`synthesis.md`. It does not change a ticket Answer or select a design.

The corrections address the versions inspected for this report. Later
repairs can remove the cited errors. The report does not claim that those
errors remain in the present preparation. “Precise repair” introduces a
recommended text correction, not an accepted Traces design rule.

- **Confirmed** means that the inspected source supports the stated,
  limited claim.
- **Corrected** means that the source contradicts the claim or supports only
  a narrower statement.
- **Unresolved** means that this inspection does not establish the claim.
- `[V]` identifies a source read, not a program execution.
- `[INFERENCE]` identifies a conclusion from the inspected source.

I read the brief, the synthesis, and both research files. I opened the
primary URLs and the digest blocks cited below. I used CodeGraph for the
initial production-code inspection. I used project-local rust-docs records
for the Rust documentation reads. During that inspection, I changed only
this report. I ran no builds, tests, linters, formatters, benchmarks, or
runtime probes.

The owner separately reports a Moment runtime probe. Section K1 records
that distinction. Live upstream pages describe their current content, not
the owner's installed plugin versions. Pinned source links identify the
version where available. The digest extracts do not prove that their code
matches a released plugin build.

## Repairs that affect the grilling

1. Dataview's inspected `sow` and `eow` calls use ISO weeks, not locale weeks.
2. CLDR 49 gives Portugal and Iceland the pair `(sunday, 4)`.
3. Temporal has no `PlainYear`. Java has `Year` and `YearMonth`.
4. RFC 3339 puts its string-order rule in section 5.1, not section 5.6.
5. RFC 3339's direction is **least precise to most precise**: year, month, day, then time.
6. RFC 5545 calls weeks and days nominal durations. It does not make every accepted duration a fixed millisecond count.
7. PostgreSQL exposes distinct timestamp types. It is not evidence for one public type with a mode flag.
8. Dataview compares date objects numerically. It does not sort those objects by their ISO text.
9. Dataview has a cross-type comparison function. The digest report overlooks that function.
10. An explicit anchor supports reproducibility. It does not prove a whole parser deterministic by construction.
11. `Europe/Prague` exists. It is not an unknown-zone example.
12. Apia's 2011 transition uses actual offsets `-10:00` and `+14:00`, including daylight saving time.

## K1. Moment tokens, week years, and locale policy

**Status: Confirmed, with corrections.**

Claims affected: c Q1, Q2, Q8, Q15; e q1, q2, q4, q8; synthesis Q1–Q3 and Q7.

Sources read:

- [Moment token tables](https://momentjs.com/docs/#/parsing/string-format/).
- [Moment formatting](https://momentjs.com/docs/#/displaying/format/).
- [Moment week customization](https://momentjs.com/docs/#/customization/dow-doy/).
- Moment 2.30.1 [`week.js`](https://raw.githubusercontent.com/moment/moment/2.30.1/src/lib/units/week.js).
- Moment 2.30.1 [`week-calendar-utils.js`](https://raw.githubusercontent.com/moment/moment/2.30.1/src/lib/units/week-calendar-utils.js).
- Moment 2.30.1 [`day-of-year.js`](https://raw.githubusercontent.com/moment/moment/2.30.1/src/lib/units/day-of-year.js).
- Moment 2.30.1 [`from-array.js`](https://raw.githubusercontent.com/moment/moment/2.30.1/src/lib/create/from-array.js).

[V] Lowercase `w`, `ww`, and `gggg` use locale week rules.
Uppercase `W`, `WW`, and `GGGG` use ISO week rules.
`YYYY` names the calendar year. It does not name either week year.
A literal `[W]` does not make a lowercase `ww` an ISO week token.

[V] The customization formula is `doy = 7 + dow - janX`.
`janX` identifies the January date that must occur in week one.
The e q8 preset list confuses `janX` with `doy`.

Precise repair:

| Convention in the Moment documentation | `dow` | `doy` | `janX` |
| --- | --- | --- | --- |
| ISO | 1 | 4 | 4 |
| US/Canada example | 0 | 6 | 1 |
| Many Arab countries example | 6 | 12 | 1 |
| Other Monday-first example | 1 | 7 | 1 |

[V] `Q` formats the quarter. `Qo` formats its ordinal. Parsing `Q` sets the
first month of that quarter. `DDD` and `DDDD` identify the day of year.
The source checks the selected year's day count. The valid range includes
day 366 in leap years. Replace the blanket range `1 to 365` in e q4 with
`1 to 365, or 366 in a leap year`.

[V] `day(0)` always selects Sunday. `weekday(0)` selects the locale's first
weekday. Do not treat those methods as the same API. The inspected
Templater helper uses `weekday`, not `day`. See K2.

[V] The owner reports this separate runtime result for Moment 2.30.1, English locale, at UTC `2025-12-28`:

- `YYYY-[W]ww` gives `2025-W01`.
- `gggg-[w]ww` gives `2026-w01`.
- `GGGG-[W]WW` gives `2025-W52`.

The owner's evidence file is `m-gap-closure.md`.
I did not repeat that runtime probe.
The source independently confirms the distinct token rules that explain the result.

Precise repair: retain the filename collision example with its explicit
locale and token spelling. Do not claim that all local-week formats are
inherently unstable. A fixed, explicit week policy removes device-locale
dependence. The human still chooses whether to support those tokens.

## K2. Templater delegates formatting and weekday policy

**Status: Confirmed.**

Claims affected: c Q1, Q6, Q15; e q1 and q6.

Digest blocks read:

- `docs/digests/obsidian_silentvoid13-templater-src-digest.txt:2456-2541`, `InternalModuleDate.ts`.
- `docs/digests/obsidian_silentvoid13-templater-docs-digest.txt:168-207,255-290,1587-1602`.

[V] `InternalModuleDate.ts` imports `moment` from `obsidian`.
`generate_now` calls `moment(reference, reference_format).add(duration).format(format)`.
A numeric offset means days. A string offset goes to `moment.duration`.
`generate_weekday` calls `.weekday(weekday)`.
The documentation exposes the complete `moment` object.

Precise repair: keep the delegation and locale-weekday precedent.
Do not infer that Traces must execute these scripts or preserve every Moment token.
The source shows a plugin boundary, not the required home of Traces arithmetic.

## K3. Dataview and Luxon default weeks

**Status: Corrected.**

Claims affected: c Q8, Q31 and its product summaries; synthesis Q31.

Sources read:

- `docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:7643-7666`.
- Luxon 3.7.2 [`datetime.js:1693-1774`](https://raw.githubusercontent.com/moment/luxon/3.7.2/src/datetime.js).
- Luxon 3.7.2 [locale-based weeks](https://raw.githubusercontent.com/moment/luxon/3.7.2/docs/intl.md).
- [Luxon API, `startOf` and `endOf`](https://moment.github.io/luxon/api-docs/index.html#datetimestartof).
- Current Dataview [`package.json`](https://raw.githubusercontent.com/blacksmithgu/obsidian-dataview/master/package.json).

[V] Dataview calls `DateTime.local().startOf("week")` and `.endOf("week")` without options.
Luxon 3.7.2 sets `useLocaleWeeks = false` by default.
Its default branch sets `weekday = 1`, Monday.
Locale-based week arithmetic needs `{ useLocaleWeeks: true }`.
Dataview's current manifest requests `luxon: ^3.2.0`.
That manifest does not identify the exact version installed with the owner's plugin.

Precise repair: replace “Dataview follows the runtime locale” with “the inspected Dataview calls use Luxon's default ISO week”.
Keep nldates as the inspected locale-following example.
Do not use Dataview-versus-Tasks as evidence for different week starts.

## K4. CLDR first day, minimum days, and week rollover

**Status: Confirmed, with narrower wording and a false counterexample removed.**

Claims affected: e q8 and q31; synthesis Q8 and Q31.

Sources read:

- [UTS #35, CLDR 49, Week Data](https://unicode.org/reports/tr35/tr35-dates.html#Week_Data).
- [CLDR release-49 `supplementalData.xml`, `weekData`](https://raw.githubusercontent.com/unicode-org/cldr/release-49/common/supplemental/supplementalData.xml), lines 4517–4589.
- [Java 25 `WeekFields`](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/time/temporal/WeekFields.html).
- [POSIX.1-2024 `strftime`](https://pubs.opengroup.org/onlinepubs/9799919799.2024edition/functions/strftime.html).

[V] CLDR uses `firstDay` and `minDays` for week numbering.
The territory fallback `001` uses Monday and one day.
The US uses Sunday and one day.
“Root locale” is less precise here than “territory fallback `001`”.

[V] CLDR's four-day list includes `IS` and `PT`.
Its Sunday-first list also includes `IS` and `PT`.
The statement “no major locale uses `(sunday, 4)`” is false under this cited data.
CLDR also permits a Unicode `fw` override for the first day.

Precise repair:

- Delete the “no major locale” assertion.
- Remove its implied restriction.

A pair can describe a week rule. A change to one field does not need a
change to the other. The policy still needs an explicit rule for week
numbering. Display-week start and numbered-week semantics can be separate
concepts. The external evidence does not select either design.

[V] CLDR recommends rollover when changed week settings make a previously valid week 53 unavailable.
Its note says to treat that date as part of the next year's first week.
This is compatibility guidance within CLDR's parsing discussion.
It is not a requirement for every strict ISO week parser.

Precise repair: retain the contrast with the accepted no-rollover rule.
Do not present a general CLDR-conformance duty for Traces filename parsing.

[V] Java's `2009-01-01` table contrasts minimum days **4 and 5**, not **1 and 4**.
Replace e q8's incorrect table attribution with the actual comparison.
Java distinguishes `weekOfYear`, which can use week zero, from `weekOfWeekBasedYear`.
POSIX `%U` and `%W` use week zero. `%V` uses ISO week numbering.
Neither `%U` nor `%W` is a locale week-year substitute.

## K5. Temporal partial types and normalization

**Status: Corrected.**

Claims affected: e q4, q20 and q22.

Sources read:

- [Temporal API inventory](https://tc39.es/proposal-temporal/docs/).
- [Temporal `PlainYearMonth`](https://tc39.es/proposal-temporal/docs/plainyearmonth.html), constructor, `compare`, and `from`.
- [Java 25 package inventory](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/time/package-summary.html).
- [XSD 1.1 `gYear` and `gYearMonth`](https://www.w3.org/TR/xmlschema11-2/#gYear).

[V] Temporal has `PlainYearMonth` and `PlainMonthDay`. It has no `PlainYear`.
Java exposes `Year` and `YearMonth`.
XSD defines `gYear` and `gYearMonth`.
The claim that Temporal supplies the only mainstream partial-date types is false.

[V] The `PlainYearMonth` constructor defect is real.
A non-default ISO reference day can make values unequal despite identical displayed year and month.
The normal `from` path supplies the compatible reference day.

Precise repair: retain the constructor example as evidence for canonical representation.
Do not claim that all partial types have that defect.
Do not infer Traces period equality from Temporal constructor internals.
`PlainYearMonth` supplies one month-value precedent. It does not establish a general week/quarter period model.

## K6. Civil values, instants, and cross-kind comparison

**Status: Confirmed distinctions; corrected blanket claims.**

Claims affected: e q19 and q21; c Q19 and Q21.

Sources read:

- [Temporal type inventory](https://tc39.es/proposal-temporal/docs/).
- [Temporal time zones](https://tc39.es/proposal-temporal/docs/timezone.html).
- [Temporal `PlainDate.from` and `compare`](https://tc39.es/proposal-temporal/docs/plaindate.html).
- [Java 25 package design notes](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/time/package-summary.html).
- `docs/digests/obsidian_obsidian-tasks-src-digest.txt:4934-4944,5043-5072`.

[V] Temporal and Java separate civil types from exact-time types.
Temporal documents a fold example where conversion to `PlainDateTime` loses the selected occurrence.
Tasks normalizes parsed dates and range endpoints to local midnight.
Its comment explicitly names equality as the reason.

[V] “Temporal cross-kind `compare` always throws” is too broad.
`PlainDate.compare` converts inputs through `PlainDate.from`. The documented
`PlainDate.from(PlainDateTime)` conversion succeeds and drops the time.

Precise repair: cite the specific conversion or comparison API. Do not
generalize one rejected civil/instant conversion to every cross-kind input.
Treat Traces cross-class equality and ordering as its own contract.

[INFERENCE] Midnight normalization and ambient zones identify a portability risk.
The inspected Tasks comment does not measure a cross-machine equality failure.
Replace “documented failure” with “source pattern that exposes this risk” where no failure record exists.

## K7. RFC 5545 durations are not all exact

**Status: Corrected.**

Claims affected: c Q23 and analogies; e q10, q16, q23 and q24.

Source read: [RFC 5545 section 3.3.6](https://www.rfc-editor.org/rfc/rfc5545#section-3.3.6).

[V] The grammar permits a week form, or days and time, or time alone.
It does not combine weeks with the other units.
It excludes year and month designators.
It calls **weeks and days nominal durations**.
It calls hours, minutes, and seconds accurate durations.
Exact elapsed duration across a clock discontinuity needs the discontinuity adjustment.
The RFC excludes leap seconds from that computation.
It requires larger components before smaller components.

Precise repair: replace “RFC 5545's non-nominal units” with “RFC 5545's supported units”.
Delete “forbidding nominal units makes equality trivial” as a summary of this RFC.
Removing years and months does not remove calendar-day or week dependence.
A fixed-millisecond rule is a separate product rule, not an RFC consequence.

## K8. Fileclass duration and validation evidence

**Status: Corrected and narrowed.**

Digest blocks read:

- `docs/digests/obsidian_mdelobelle-fileclass-src-digest.txt:3082-3188`, `dateFormatCheck.ts`.
- The same digest, `:3274-3318`, `dateLink.ts`.
- The same digest, `:3501-3688`, `duration.ts`.
- The same digest, `:6079-6087,6107-6112,6153-6164,6191-6196`, `validate.ts`.

[V] Fileclass has a pure token scanner and an injected Moment formatter for link-path expansion.
The latter expands only `{{…}}` sections.
The source explains why formatting a complete path damages literal text.
These are good boundary examples. They do not prove an adapter's required repository placement.

[V] `durationMs` treats each day as `86_400_000` milliseconds.
`addDuration` uses UTC arithmetic.
That is Fileclass behavior, not the RFC's nominal-day rule.

[V] The input regex independently permits weeks, days, and time in one string.
It captures an optional sign, but `parseDuration` does not apply that sign to the returned parts.
`buildDuration` separately folds mixed weeks into days.

Precise repair: do not describe the inspected parser as an exact RFC-validating oracle.
Separate its intended output grammar from its more permissive input parser.
Do not adopt its sign handling or fixed-day arithmetic without an independent contract.
These are source findings. I did not execute example inputs.

[V] `validateDatePattern` bypasses pattern validation when `dateFormat` is a string.
It does not itself check for a non-empty string or enforce a string type.
The outer validator handles empty values and the `required` option.
Replace “accepts any non-empty string” with that exact split.

[V] The silent-drop comment concerns scalar **links** and a link-membership test.
The same source says scalar plain values work like lists for several tested comparisons.
Do not generalize the reported link case to every scalar-versus-list mismatch.
The “measured” label describes the upstream comment, not a measurement in this report.

## K9. Dataview duration comparison and total order

**Status: Corrected.**

Claims affected: c Q22 and Q23; e q22 and q23.

Sources read:

- `docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:5470-5533,6164-6168,7602-7640,13849-13854`.
- Luxon 3.7.2 [`duration.js`](https://raw.githubusercontent.com/moment/luxon/3.7.2/src/duration.js), `casualMatrix`, constructor, `valueOf`, and `equals`.
- Luxon 3.7.2 [math documentation](https://raw.githubusercontent.com/moment/luxon/3.7.2/docs/math.md).

[V] Dataview's `compareValue` documents a total order across its value types.
Different types compare by their type-name strings through `localeCompare`.
`createBinaryOps` registers that function as its universal comparison function.
There is no inspected period type. That limit does not erase the existing cross-type comparator.

[V] Its date branch uses numeric `<`, then Luxon `.equals`.
Its duration branch uses the same structure.
Luxon duration numeric conversion uses a conversion matrix, not a calendar anchor.
The default casual matrix uses 30-day months and 365-day years.
Luxon `.equals` checks unit fields and locale information, not only elapsed milliseconds.
Dataview also normalizes duration values with `shiftToAll().normalize()`.

Precise repair: remove “Dataview sorts its ISO date strings lexically”.
Replace “calendar-dependent duration comparison” with the fixed conversion-matrix behavior for this inspected path.
Keep actual calendar arithmetic separate from duration-number conversion.
Do not cite `.equals` as plain millisecond equality.
Do not copy this comparator as proof of Traces total-order laws.
I did not execute cases with equal numeric values and different metadata.

## K10. RFC 3339 string ordering

**Status: Confirmed rule; corrected section and invalid deductions.**

Claims affected: e q5, q7 and q22.

Source read: [RFC 3339 section 5.1, Ordering](https://www.rfc-editor.org/rfc/rfc3339#section-5.1).

[V] The RFC says **least precise to most precise**.
That means year before month, then day, then time components and fractional seconds.
Do not reverse that direction.

[V] String order gives time order only under the stated conditions:

- The timestamps use the same time zone or offset.
- They use the same zone spelling, such as all `Z` or all `+00:00`.
- They use the same number of fractional-second digits.
- They do not mix optional punctuation forms.

The inspected preparation omits the precision condition in q5 and cites
section 5.6 instead of 5.1.

Precise repair: cite section 5.1. State all conditions. Do not use the ordering
property as proof that filename prefix parsing is unambiguous. That
property does not define suffix separators or malformed-prefix fallback.

[V] e q22's example is also wrong: lexical order puts `2026-02-28` before `2026-03`.
Delete that example. Neither partial periods nor cross-class ordering fall under the RFC's timestamp theorem.

## K11. Temporal duration comparison and XSD partial order

**Status: Confirmed, with scope retained.**

Sources read:

- [Temporal `Duration.compare`](https://tc39.es/proposal-temporal/docs/duration.html#compare).
- [XSD 1.1 duration value space](https://www.w3.org/TR/xmlschema11-2/#duration-value-space), through the raw HTML source.

[V] Temporal documents `relativeTo` as required when years, months, or weeks are nonzero.
A zoned anchor accounts for offset changes.
A plain-date anchor treats days as 24 hours.
Do not infer that every duration comparison always needs a zone.

[V] XSD 1.1 defines a partial order for `duration`.
It compares results at four reference datetimes.
Its `P1M` versus `P30D` example changes order between February 1697 and March 1903.
This closes e q22's metadata-only fetch gap.
The raw URL form returned the normative text.

Precise repair: mark the XSD statement as confirmed source evidence.
Keep physical comparison, field equality, and a product's class-ranked sort as separate contracts.
Delete e q23's erroneous “twice a year” and “wrong four times” counts.
Use the inspected XSD counterexample instead.

## K12. Arrow dates, timestamps, and intervals

**Status: Confirmed definitions; narrower design analogy.**

Sources read:

- [Arrow Columnar Format 1.5](https://arrow.apache.org/docs/format/Columnar.html).
- The authoritative [`format/Schema.fbs`](https://raw.githubusercontent.com/apache/arrow/main/format/Schema.fbs), `Date`, `Timestamp`, `IntervalUnit`, and `Duration` definitions.
- [`pyarrow.compute.strftime`](https://arrow.apache.org/docs/python/generated/pyarrow.compute.strftime.html).

[V] `Date` uses days in 32 bits or milliseconds in 64 bits since the Unix epoch.
The millisecond date representation needs values divisible by `86400000`.
A timestamp with a non-empty zone uses the UTC epoch.
An empty zone denotes unspecified wall-clock interpretation. It does not mean UTC.
The schema permits a date/time struct or an empty-zone timestamp for naive datetime values.

[V] Interval units include `YEAR_MONTH`, `DAY_TIME`, and `MONTH_DAY_NANO`.
The first stores an integer count of whole months.
These intervals describe amounts. They do not identify an anchored calendar month or week.

Precise repair: retain Arrow as evidence for representation choices and civil/instant distinctions.
Do not use its interval type as direct evidence for `(start, unit)` period identity.

[V] Arrow's formatter defaults to `%Y-%m-%dT%H:%M:%S` and locale `C`.
It reports an error when a defined time zone is unavailable or the requested
locale does not exist. Its `%S` precision matches the input precision.
The page does not provide the missing directive-by-directive comparison.
Keep that comparison unresolved rather than infer POSIX parity.

## K13. PostgreSQL date and timestamp semantics

**Status: Confirmed behavior; corrected type analogy.**

Sources read:

- [PostgreSQL 18, Date/Time Types](https://www.postgresql.org/docs/18/datatype-datetime.html), sections 8.5.1–8.5.3.
- [PostgreSQL 18, Date/Time Functions](https://www.postgresql.org/docs/18/functions-datetime.html), introduction and arithmetic discussion.
- [PostgreSQL 18, Expression Evaluation](https://www.postgresql.org/docs/18/sql-expressions.html#SYNTAX-EXPRESS-EVAL).

[V] PostgreSQL exposes `timestamp without time zone` and `timestamp with time zone` as distinct SQL types.
Its type decision precedes inspection of the literal's contents.
The without-zone type ignores a supplied offset.
The with-zone type stores the instant in UTC and does not retain the original zone.

Precise repair: replace “one type with a mode flag” with “distinct SQL types with permissive typed input”.
Keep the silent-offset-drop example as evidence about parser policy.
It does not prove that a Rust inner enum is unsafe or that separate Rust variants are necessary.

[V] PostgreSQL `date` has day resolution and no time of day.
When comparing dates with timestamps, PostgreSQL assumes midnight in the `TimeZone` setting.
This supports the stated contrast with an explicit Traces projection policy.
It does not decide that policy.

[V] PostgreSQL documents the March 31 month-clamp example and the Denver one-day-versus-24-hours example.
It also warns about stale `today`/`now` text literals in saved statements.
Its clock functions distinguish transaction, statement, and changing wall-clock time.
Its evaluator can reorder Boolean expressions and omit unnecessary subexpressions.
Those citations support the limited arithmetic, frame-scope, and evaluation-order analogies.

[V] PostgreSQL documents IANA names, abbreviations, POSIX strings, and their different meanings.
It also says `TimeZone` cannot be set to an abbreviation.
Do not present every input spelling as an interchangeable configuration spelling.

## K14. Unicode, zone names, and absent-zone examples

**Status: Corrected scope.**

Claims affected: e q29; c Q17 and Q29; Unicode safety analogies in c Q9–Q12.

Sources read:

- [tzdb 2026f theory, Timezone identifiers](https://data.iana.org/time-zones/tzdb-2026f/theory.html#naming).
- `docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:7852-7858,7902-7947`.
- `docs/digests/iwe-org-iwe-src-digest.txt:76423-76482,76925-76939`.

[V] tzdb's ASCII/no-digit restrictions are naming guidelines with stated legacy exceptions.
The text lists names such as `Etc/GMT0`, `GMT0`, and `EST5EDT` among those exceptions.
Do not turn those guidelines into a complete accepted-zone regex.

[V] The same primary source explicitly names `Europe/Prague` as a valid selection example.
Delete e q29's false unknown-zone example.
Use a value that fails the selected database lookup, not a guessed invalid city.

[V] Dataview identifiers accept Unicode letters and emoji.
Its date digit patterns use JavaScript `\d`, which means ASCII decimal digits here.
Its bracket-zone regex is `[0-9A-Za-z+-\/]+`.
That regex excludes underscores, including the underscore in `America/New_York`.
Its numeric-offset branch accepts one or two hour digits and an optional colon-minute pair.
It does not establish c Q29's `+0200` claim for this grammar.

Precise repair: separate Unicode identifier support from date and zone grammar support.
Do not describe this digest as accepting all IANA names or all common offset spellings.

[V] IWE's scanner checks ASCII digits and slices date/time components by byte
positions. Its tests contain three multibyte timezone-tail rejection cases.
Those are test-source reads, not executed tests. The date scanner checks
shape, not calendar ranges. `is_iso_date` does not validate month lengths.

Precise repair: retain the narrow adversarial-test precedent.
Do not generalize it into complete Unicode safety or semantic date validation.

## K15. Filename periods and Dataview reduced precision

**Status: Corrected blanket claims.**

Claims affected: c Q4, Q7, Q32; e q7 and q32.

Sources read:

- [Dataview implicit fields](https://blacksmithgu.github.io/obsidian-dataview/annotation/metadata-pages/).
- `docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3631-3648,7902-7947,13867-13879`.
- [NumPy 2.5 datetime documentation](https://numpy.org/doc/2.5/reference/arrays.datetime.html).

[V] Dataview's `rootDate` requires a year and a month.
The subsequent ladder adds optional day, time, fraction, and zone components.
The inspected literal grammar does **not** accept a bare year.
Replace c Q4 and Q32's year-only claims with year-month support.
Do not infer literal support from Luxon's separate `fromObject` API.

[V] Dataview's `extractDate` searches for a dashed or compact full date anywhere in a string.
It is not a strict leading-prefix parser.
Its `findDate` checks typed `date` or `day` metadata before the filename.
That helper can also inspect link path, subpath, and display text.
This is more precise than the public documentation's summary.

[V] NumPy documents equality between year-granularity `2005` and day-granularity `2005-01-01`.
That is a cross-unit equality rule. It is not malformed-prefix fallback behavior.

Precise repair: replace “no ecosystem tool parses filename periods” with
this limited observation:

“The inspected Dataview extraction helper recognizes full dates, not week or quarter labels.”

The inspected Tasks query parser recognizes numbered weeks, months,
quarters, and years. It does not establish a filename parser. No finite
sample supports the blanket absence claim. The universal claim remains
unresolved and must not decide the parser boundary.

## K16. NumPy week units

**Status: Corrected.**

Claim affected: e q8's “keyed to Monday”.

Sources read:

- [NumPy datetime units](https://numpy.org/doc/2.5/reference/arrays.datetime.html#datetime-units).
- Current NumPy [`datetime.c`](https://raw.githubusercontent.com/numpy/numpy/main/numpy/_core/src/multiarray/datetime.c), lines 331–352 and 490–506.

[V] NumPy stores units relative to the `1970-01-01` epoch.
Its week conversion divides epoch-relative days by seven.
The reverse conversion multiplies by seven.
There is no Monday adjustment in those branches.

Precise repair: replace “Monday-based span” with “seven-day epoch-based unit, not an ISO numbered-week rule”.
Do not confuse this unit with NumPy business-day weekmasks.
The inspected source is current upstream. I did not execute NumPy.

## K17. Natural-language grammar and determinism

**Status: Confirmed examples; unresolved blanket guarantee.**

Claims affected: c Q13–Q17 and Q33–Q34; e q13–q15 and q33.

Sources read:

- `docs/digests/obsidian_community-archive-nldates-digest.txt:414-426,456-590,852-860`.
- `docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:7643-7672,7907-7912,7950-7954`.
- [Duckling README](https://raw.githubusercontent.com/facebook/duckling/main/README.md).
- [Duckling example server](https://raw.githubusercontent.com/facebook/duckling/main/exe/ExampleMain.hs), `parseHandler`.
- Project-local `chrono-english` 0.2.1 documentation, item 178, all-features cache variant.
- The same crate's `parse_date_string`, item 174, `src/lib.rs:95-123`.
- [Java 25 `Clock`](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/time/Clock.html).

[V] Dataview has a closed shorthand table, but its entries call `DateTime.local()`.
A closed keyword set does not remove ambient-clock dependence.

[V] nldates reads ambient dates and Moment locale data.
It gets the first weekday from `localeData()._week.dow`.
Its `next week` branch parses the configured first weekday, not an unchanged weekday seven days later.
Replace c Q16's “same weekday next week” claim with that inspected branch.
Its `next month` and `next year` branches use two-stage parsing.

[V] Duckling provides grain-bearing results and positive/negative corpus
examples. Its example HTTP server accepts **`reftime`**, not
`reference_time`. It uses that value to fill `Context.referenceTime`.
Otherwise, it reads the current reference time. The README pins the corpus
reference to 2013-02-12 at 04:30.

This source supports an injected-context precedent. It does not prove that
an open grammar cannot replay consistently.

[V] `chrono-english::parse_date_string` takes text, a `DateTime<Tz>` anchor, and a dialect.
Its documentation limits the grammar and explains the UK/US weekday difference.
Its `parse_duration` returns `Interval::Seconds`, `Days`, or `Months`.
Those facts support the documented API comparison.
The generic `TimeZone` and conversion paths still matter to a full determinism claim.
I did not inspect every transitive conversion or execute same-day weekday cases.

Precise repair: replace “deterministic by construction” with a conditional
statement.

“The parser must depend only on its text and pinned inputs, including zone data, week rules, and dialect.”

A source audit and fixed-input behavior evidence must establish that
proposed contract. This report does not demonstrate the coverage or
determinism of the unwritten in-repository grammar. A closed grammar can
simplify the contract. It does not prove the contract. An open but
deterministic grammar can also replay with pinned inputs.

[V] Java recommends injected clocks and provides `Clock.fixed`.
It also documents ambient clock APIs.
Replace “every Java API takes the clock or zone” with “Java offers explicit clock and zone overloads”.

## K18. Tasks ranges, weekday meaning, and membership

**Status: Confirmed, with limited scope.**

Sources read:

- [Tasks Filters](https://raw.githubusercontent.com/obsidian-tasks-group/obsidian-tasks/main/docs/Queries/Filters.md), date-search sections.
- `docs/digests/obsidian_obsidian-tasks-src-digest.txt:4934-5141`.

[V] Tasks documents the nearest-Tuesday example.
It documents `last|this|next week|month|quarter|year` as inclusive ranges.
Its code changes `week` to `isoWeek`.
Its numbered-range table includes year, quarter, month, and week forms.
It uses mutable endpoint pairs and adjusts month/quarter range ends after a shift.
Its comments identify an untested cross-unit shift.
These citations support the stated range-representation comparison.

[V] Its `HappensDate` type lists start, scheduled, and due dates.
Do not describe `happens` as checking every task date without that qualification.

Precise repair: retain Tasks as one documented weekday/range policy.
The words `in`, `within`, or `member_of` remain Traces syntax choices.
Tasks' use of `in` does not prove that one Traces grammar must become ambiguous.

## K19. Formatting surfaces, defaults, and error precedents

**Status: Confirmed examples; corrected breadth.**

Digest blocks read:

- `docs/digests/iwe-org-iwe-src-digest.txt:9599-9609,9629-9633,9725-9733,13792-13833,54931-54951`.
- `docs/digests/zk-src-digest.txt:2213-2316`, `date.go`.
- `docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:6865-6909,6924-6928`.

[V] IWE separates key and content formatting values. It uses the same chrono
formatting family for both. The inspected current configuration places
content fields under `[markdown]`, not `[formatting]`. Its
`format_options()` selects content options. Its locale helper uses the system
locale only when no configured value exists. An invalid configured locale
falls directly to `POSIX`. It does not retry the system locale. Correct
those table names and fallback descriptions in c Q3, Q5, Q27, and Q29.

[V] zk has named styles with custom strftime fallback.
Its helper logs a parse error and returns the utility's returned time value.
The helper's default branch returns an initialized zero `time.Time`.
I did not inspect the utility's error return contract.
Do not claim that every parse error demonstrably returns zero from this helper read alone.
Its old time-valued helper path calls `os.Exit(1)`.

[V] Dataview returns `null` for failed single-string parsing.
Its two-string custom-format branch can throw.
It formats through Luxon's `toFormat`.

Precise repair: keep these specific error shapes. Delete “every integration
has a silent-or-drastic failure path” as an unproved universal statement.
One-dialect-per-tool and one-dialect-per-surface describe the inspected
sample, not ecosystem laws. Neither sample proves a unique Traces placement.

## K20. Jiff formatting and acquisition citations

**Status: Confirmed, with one material token correction.**

Project-local sources read:

- `jiff` 0.2.38, `jiff::fmt::strtime`, documentation item 74.
- `jiff` 0.2.38, `jiff::fmt::temporal`, documentation item 5.
- `jiff` 0.2.38, `TimeZone::try_system`, documentation item 4026.
- `logos` 0.16.1 crate documentation, item 272.

These reads use the pre-existing all-features documentation cache.
That cache is not the production feature configuration.
The reads do not measure feature cost or prove parser-replacement parity.

[V] Jiff describes a hybrid RFC 3339/RFC 9557/ISO grammar.
Its documentation rejects fractional hours and accepts mixed week/unit spans.
It documents the `Z`-to-civil rejection, compatible fold/gap behavior, and default offset-conflict errors.
Its strtime documentation lists `%G`, `%V`, `%U`, `%W`, and format-only `%q`.
It supplies no general locale-week-year directive in that complete table.

[V] `%Q` **exists**. It means a zone identifier or offset, not a quarter.
Repair e q3's “`%Q` does not exist” statement.
The correct statement is that `%Q` is not a strftime quarter token.
Jiff does not claim exact POSIX parity or built-in locale support.
Its `Custom` extension permits caller-provided formatting behavior.

[V] `try_system` returns an error when discovery fails or `tz-system` is disabled.
`system` can fall back to UTC.
Discovery uses platform heuristics and honors `TZ`.
Do not cite Jiff's convenience fallback as the accepted Traces policy.
The read closes e q30's basic failure-mode documentation gap.
It does not establish the complete failure and cache behavior of a new frame.

[V] Logos documents a deterministic state machine for token definitions.
That lexical property does not prove deterministic callbacks, relative-date binding, or parser-wide semantics.
Do not extend the Logos citation to the whole natural-language parser.

## K21. Time-zone counterexamples and bundle claims

**Status: Confirmed examples; corrected offsets and reproducibility claim.**

Sources read:

- [tzdb 2026f `australasia`](https://data.iana.org/time-zones/tzdb-2026f/australasia), `WS` rules and `Pacific/Apia`, lines 791–802.
- [tzdb 2026f `northamerica`](https://data.iana.org/time-zones/tzdb-2026f/northamerica), `Cuba` and `America/Havana`, lines 3559–3605.
- [tzdb 2026f theory](https://data.iana.org/time-zones/tzdb-2026f/theory.html), scope and naming sections.

[V] Apia changes its standard offset from `-11:00` to `+13:00` at `2011 Dec 29 24:00`.
The `WS` rules add one hour of daylight saving time on both sides.
The actual transition skips from December 29 at `-10:00` to December 31 at `+14:00`.
Repair e q21's stated offsets. The skipped-day counterexample remains valid.

[V] Cuba has midnight standard-time transitions that support the fold/gap example.
The 2026f rules suspend the usual November 2026 return.
Do not assume an annual transition without choosing a year supported by the selected data.

[V] tzdb limits its historical accuracy claims, especially before 1970.
A bundle supplies data. The feature name alone does not prove that every lookup ignores system data.
Repair e q12's unconditional reproducibility claim and e q29's feature-only validation guarantee.
State the selected data source and version before claiming repeatable zone results.

## K22. Markdown, percent decoding, and Unicode text

**Status: Confirmed rules; corrected operation order.**

Sources read:

- [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/), introduction and link-destination definition.
- [RFC 3986 section 2.4](https://www.rfc-editor.org/rfc/rfc3986#section-2.4) and section 7.3.
- `docs/digests/obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:6872-6890`.

[V] CommonMark permits balanced unescaped parentheses in the bare destination form.
It also permits escaped characters and the angle-delimited form.
It lets renderers leave non-ASCII destination characters unencoded.
Do not reduce its destination grammar to “all parentheses need escaping”.

[V] RFC 3986 requires component separation before decoding reserved octets.
It prohibits repeated percent decoding. Decoding a reserved delimiter can
change interpretation.

Precise repair:

- Identify URI components before percent decoding.
- Identify the target segment before decoding it once.
- Do not decode a complete target before identifying its `/`, `#`, or `?`
  boundaries.
- Do not let `%2F` create a new final-segment boundary.

The accepted ticket rules still control Markdown parsing and note-target
classification. This RFC does not replace those rules. Traces
percent-decoding policy remains a human choice.

[V] Dataview's `date(link)` tries display text, then path, then resolved `file.day`.
That source supports the alias-risk example.
It does not prove percent-encoded-target behavior.
No runtime link example was executed here.

## K23. Other inspected external claims

**Status: Confirmed within these limits.**

- [Java `TemporalAdjusters`](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/time/temporal/TemporalAdjusters.html) documents externalized adjustment and four explicit next/previous variants. It does not choose Traces bare-weekday semantics.
- [POSIX `tzset`](https://pubs.opengroup.org/onlinepubs/9799919799.2024edition/functions/tzset.html) documents an implementation-defined zone when `TZ` is absent. It recommends explicit setup before portable `localtime_r` use.
- [Cron's manual](https://man7.org/linux/man-pages/man5/crontab.5.html) defines weekday matching, not relative-weekday resolution. It also documents missing-time and repeated-time scheduling effects.
- [Moment's guide](https://momentjs.com/guides/#/parsing/strict-mode/) gives the UUID parsing example.
  Its exact UUID is `5917238b-33ff-f849-cd63-80f4c9b37d0c`, parsed with
  `moment.ISO_8601`. Correct e q1's altered input/API attribution. If you
  paraphrase the example, do not present it as an executed example.
- [Moment time-source customization](https://momentjs.com/docs/#/customization/now/) documents overridable `moment.now`. This closes e's stated documentation gap. A mutable global clock hook is not per-request clock injection.

## Exact limits of this inspection

This is a source audit of selected material claims, not every line of every
cited project. Do not mark every statement in c or e as checked from this
report. The claims below remain unexamined **in this report**. Later
source checks and bounded probes in [m-gap-closure.md](./m-gap-closure.md)
have their own scope. They do not resolve every item in this list.

- The accepted Answers, ledger identifiers, supersession rules, and compatibility classifications in earlier tickets.
- The owner's complete template inventory and schema-format inventory.
- Production token counts, caller counts, byte sizes, dependency trees, and extractor-fingerprint membership.
- Jiff-versus-speedate parity for frozen inputs and diagnostics.
- Winnow prefix-consumption behavior and the complete frozen duration grammar.
- Jiff's full system-zone cache, database precedence, and bundled-data selection implementation.
- `fuzzydate` and `jiff-english` source APIs, full grammars, release counts, licence files, download counts, and maintenance claims.
- Candidate parser same-day weekday behavior, full Unicode behavior, all overflow paths, and transitive absence of ambient reads.
- Metadata Menu's complete date implementation and the claimed `90d` duration path.
- The obsidian-export mtime flag and the obsidian-rs/basalt platform-failure examples.
- IWE's claimed unknown-field diagnostic, all configuration consumers, and exact chrono dependency features.
- zk's entire repository and the utility's parse-error return contract.
- An exhaustive inventory of tools that parse filename periods.
- Allen's original interval-relation paper and the secondary relation table. Neither supplies a checked rule in this report.
- PostgreSQL array-containment and `IN` operator details. They do not establish a Traces grammar ambiguity.
- A directive-by-directive Arrow parsing/formatting comparison.
- Binary size, allocation cost, throughput, and the size of a proposed runtime value.

These limits do not undo the confirmed corrections above. Use separate
evidence for claims unexamined here before those claims decide an
architecture choice. No source sample proves a globally optimal design.
