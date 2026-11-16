# Digest lessons for ticket 30: temporal boundary and dialects

Artifact: `research/30-prep/c-digest-lessons.md`. Ticket: `issues/30-temporal-boundary-and-dialects.md`
(36 questions). This file gives per-question evidence from `docs/digests/`, then the lesson for
Traces. Product behavior and Traces proposals stay separate.

Method and limits:

- Sources: digest excerpts of Templater (docs, src), Dataview (docs, src), Natural Language Dates
  (`nldates`), Fileclass, Metadata Menu, Obsidian Tasks, IWE, zk, plus three Rust PKM tools
  (basalt, obsidian-export, obsidian-rs). I cloned no upstream repository. Digest excerpts are the
  inspection surface.
- Tags: `[V]` read in a digest or in ticket 30. `[H]` hypothesis from that reading. `[P]` proposal
  for the grilling. `[A]` the topic is absent from every digest read for this file.
- Upstream file names and line numbers come from the digest excerpts. Treat them as approximate.
- Prose: ASD-STE100. Sentences stay short. Where a caveat needs nuance, the text says so and steps
  out of STE. Owner material is English; this report is English by the same choice.

## Q1 to Q5: Moment compatibility and format dialects

### Q1: Is Moment compatibility a domain concept?

Evidence:

- Templater translates nothing. It calls Obsidian's runtime `moment` and exposes the whole object
  to user scripts. Docs digest `obsidian_silentvoid13-templater-docs-digest.txt:1587-1597`; src
  digest `InternalModuleDate.ts` (default `"YYYY-MM-DD"`, `.weekday(n)`, `moment(reference,
  reference_format)` guard). Upstream: github.com/SilentVoid13/Templater. `[V]`
- Dataview does not speak Moment at all. Its value type is Luxon `DateTime`
  (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:7643-7666` area; `export type {
  DateTime, Duration } from "luxon"`). Its settings and docs use Luxon tokens in lowercase form:
  default `"MMMM dd, yyyy"` (src, settings defaults). Upstream:
  github.com/blacksmithgu/obsidian-dataview. `[V]`
- Fileclass and Metadata Menu store and lint Moment tokens (`dateFormat: "YYYY-MM-DD"`, `YYYY`,
  `DDDD`, `Q`). `obsidian_mdelobelle-fileclass-src-digest.txt:3082-3318`,
  `obsidian_mdelobelle-metadatamenu-src-digest.txt:11862-11931`. `[V]`
- Traces facts: `MOMENT_TOKENS` in `src/temporal/token.rs` lacks `ww`, `Q`, `Qo`, `DDDD`. The
  owner's Templater-shaped templates use `ww`, `YY`, `Q`, `Qo`
  (`docs/vault/07_templates/12_01_week_periodic.md`, `14_01_quarter_periodic.md`). The owner's
  Fileclass data uses `YYYY`, `DDDD`, `Q` (`docs/vault/05_metadata/cal.md`). `[V ticket]`

Product behavior: Moment is the authoring dialect of the Obsidian template and metadata ecosystem.
Dataview alone uses a Luxon dialect. No plugin makes Moment a domain concept; each treats it as a
string syntax at its own edge.

Lesson: `[P]` Moment compatibility is a compat surface, not a domain concept. Research 07 and
ticket 16 support this. The owner evidence says the surface must accept `Q`, `Qo`, `ww`, `YY`,
`DDDD`, because the owner writes them. A translator that drops them fails on real files.

### Q2: Where does the Moment dialect translator live?

Evidence:

- Fileclass lints Moment tokens with a pure scanner and no moment dependency. `TOKENS` is sorted
  longest-first and includes week-year, quarter, and ordinal tokens
  (`dateFormatCheck.ts:3082-3318`). Its code comment records the failure that motivates the lint.
  Moment never fails on a bad format. It prints unknown text verbatim. So `YYYY-KK-007` silently
  stores `2026-KK-007`. `[V]`
- Metadata Menu formats through moment directly and shows a live example of the user's format in
  settings (`AbstractDate.ts:11884-11931`). No translation layer exists. `[V]`
- Minijinja contrib date filters use strftime and hold no Moment logic (research 07, restated in
  ticket 30). `[V research]`
- Traces facts: the translator sits in `src/temporal/token.rs` since commit `6104d0ae`. It holds
  `pattern_to_strftime`, `MOMENT_TOKENS`, and the ordinal suffix. `config` already imports
  `DEFAULT_DATETIME_PATTERN` from temporal, so the `config -> temporal` edge exists. `[V ticket]`

Product behavior: two shapes exist in the ecosystem. Shape A: a per-plugin runtime dependency on
one dialect (Templater, Metadata Menu). Shape B: a pure token scanner or translator at the compat
edge with no runtime dependency (Fileclass; minijinja strftime).

Lesson: `[P]` The translator is a syntax adapter. It can live at the template compat seam or in
temporal, but it must stay a pure function with an explicit unsupported-token error. The ticket's
alternatives list both placements. The evidence does not force one placement; it does force
purity (no ambient state) and an explicit error. `[H]` Keeping it in temporal is defensible only
if schema formats need Moment syntax too (ticket 20 leaves that open).

### Q3: One dialect or a split across four format surfaces?

Evidence:

- IWE splits values, not dialects. `[workspace]` has `date_format` and `time_format` for keys.
  `[formatting]` has `date_format` and `time_format` for content. Both use chrono strftime
  specifiers (`%Y`, `%b %d, %Y`). Defaults differ: key default `%Y-%m-%d`, content default
  `%b %d, %Y`. Docs and code agree (`iwe-org-iwe-src-digest.txt:844-896`, `:13744-13873`,
  `:9607-9733`, `:9888-9890`). Upstream: github.com/iwe-org/iwe. `[V]`
- Dataview uses one Luxon dialect across settings (`defaultDateFormat`,
  `taskCompletionDateFormat`), the per-query `dateformat(date, string)` function, and the
  arbitrary-format parse `date(text, "format")` (src `:6924-6928`; docs changelog `:366`,
  `:373-374`). One dialect, four surfaces. `[V]`
- zk names styles plus one dialect: `{{format-date now}}` maps `short|medium|long|full|year|time|
  timestamp|timestamp-unix` to strftime and passes any other string through as strftime
  (`zk-src-digest.txt:2213-2316`). Upstream: github.com/zk-org/zk. `[V]`
- Traces facts: query `date(text, pattern)` accepts strftime only through
  `DatePoint::parse_with_pattern` `[V ticket]`. Schema tests use Moment tokens `[V ticket 20]`.
  Config date keys use strftime default `%Y-%m-%dT%H:%M:%S` `[V ticket]`.

Product behavior: the ecosystem norm is one dialect per tool with per-surface values and defaults.
IWE's key-versus-content split is the only two-value split; its dialect stays strftime. A
dialect-per-surface split has no precedent in the read set.

Lesson: `[P]` Pick one dialect for the four surfaces, with separate default values per surface.
The ticket asks "one dialect or justify a split". The evidence justifies one dialect plus
per-surface defaults, and it makes Moment a fifth, compat-only dialect at the template seam.
Caveat, outside STE strictness: the owner's schema tests already use Moment tokens. The schema
check then needs a Moment lint (Fileclass precedent) or a migration of those tests. This is a
compat cost, not a reason for a second native dialect.

### Q4: How does `date.format` treat a format with no full date?

Evidence:

- Fileclass separates two checks. Check one lints the format string's tokens (`dateFormatCheck`).
  Check two validates values. For a custom `dateFormat` it accepts any non-empty string. The
  reason: a check needs a date library (`validate.ts:6079-6087`). Wikilink-stored dates pass
  as-is. `[V]`
- Dataview's literal parser accepts a precision ladder: `YYYY-MM[-DDTHH:mm:ss.nnn+ZZ]`, where
  everything after the month is optional (docs `:1788`; src `EXPRESSION.date` `:7913-7947`).
  Year-only and year-month values are valid dates in its model. `[V]`
- Dataview's date render path writes `null` for unparseable text and compares with typed
  operators; it has no schema check. `[V]`
- Traces facts: `SchemaDateField.format` is an unvalidated `Option<String>` with a test-only
  accessor; tests use `YYYY-MM-DD` `[V ticket]`. The owner's Fileclass data declares `YYYY`,
  `DDDD`, `Q` `[V ticket]`, so formats without a full date exist in the owner's real data.

Product behavior: two ecosystem answers exist. Metadata Menu ignores the problem (moment prints
garbage verbatim). Fileclass lints the format at declaration time and leaves values alone.
Dataview has no declared formats at all.

Lesson: `[P]` Check the declared format at schema load as a schema error (the ticket's own
`[H]` option). Use a precision-aware value check afterwards. The check must accept reduced
precision (`YYYY`, `YYYY-MM`) because the ISO ladder precedent and the owner's data both use it.
`Q` and `DDDD` need a rule; they are calendar metadata, not precision reductions of the ISO
ladder. `[H]` A dialect with a token table can classify `Q` as quarter-of-year and `DDDD` as
day-of-year. The value check can then accept the matching value shapes or reject them with a
named reason.

### Q5: Fate of `format` under `[frontmatter.date_created]` and `[frontmatter.date_modified]`

Evidence:

- Traces facts: each table has `name` and `format`; default format `%Y-%m-%dT%H:%M:%S`; ticket 22
  records that no production code reads them; config has no `[date]` table `[V ticket]`.
- IWE shows the target shape: one `[date]`-like policy with per-surface format, time format, and
  locale fields, consumed through accessors (`format_options()`), not raw config
  (`:13744-13873`, `:65575`). `[V]`
- Rust precedent for file stamps: `obsidian-export` copies mtimes with `filetime::set_file_mtime`
  behind an opt-in `--preserve-mtime` flag (`rust_zoni-obsidian-export-digest.txt:452-454`,
  `:2818-2839`). Upstream: github.com/zoni/obsidian-export. `[V]`

Product behavior: frontmatter stamp configs exist in the Obsidian world, but Traces never reads
them. File mtimes are handled by copy tools, not by note formats.

Lesson: `[P]` The ticket's "joins the new `[date]` table" option matches the IWE shape: one
policy, several fields. The "goes" option matches the unread-config evidence and ticket 22's
fingerprint cleanup. `[H]` Keep the fields only if a migration writes them; otherwise remove and
record a change-table row (ticket 16 makes config formats Migratable).

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

Lesson: `[P]` The shared arithmetic (edges, offsets) belongs in temporal. The anchor choice stays
per surface. The engine keeps thin wrappers that pass the render frame's `today` and week rule.
Ticket 31 will need the wrapper signatures; this ticket fixes the split.

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
- Dataview derives `file.day` from the file and exposes it to `date(link)` (src `:6872-6890`
  falls back to `resolved.file.day`). `[V]`

Product behavior: no ecosystem tool parses period prefixes (week, quarter) from filenames. Dataview
derives a day only. Pattern-driven filename writing exists (Metadata Menu, zk), and both keep
literal text literal.

Lesson: `[P]` The parser is a temporal-owned, filename-scoped reader that yields a period (kind
plus start), per ticket 21. It must not read the frozen vault-format paths. The Metadata Menu
failure gives the contract rule. Only explicit prefix forms match. Longer invalid prefixes never
downgrade to shorter ones. Ticket 21 already decides this rule. The checks list must exercise it.

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
- Dataview's `sow`/`eow` call Luxon `startOf("week")`/`endOf("week")`, which follow the runtime
  locale, not ISO (src `:7643-7666`). `[V]`
- Traces facts: `CalendarPeriod::Week` is the ISO week, Monday to Sunday; ticket 21 records that
  ISO `2026-W01` starts `2025-12-29` and that ISO `2025-W53` is invalid `[V ticket]`.

Product behavior: the ecosystem is inconsistent on the same word. Tasks means ISO week. Dataview
`date(sow)` means locale week. nldates follows the Obsidian locale through a private field.
Moment exposes both rules and pairs them (`dow` + `doy`).

Lesson: `[P]` A configurable week start needs a paired week-one parameter, exactly as Moment
pairs `dow` with `doy`. Ticket 21's ISO facts become the defaults. `[H]` The Tasks precedent
(forcing ISO) is the low-surface alternative: no config, no week-year drift across devices.
The nldates precedent is the failure mode to avoid: a locale change silently shifts every week
label.

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
  `.UTF-8` and `@euro`, converts `de-DE` to `de_DE`, falls back to system locale, and falls back
  to `Locale::POSIX` on invalid input (`locale.rs:54931-54990`). `[V]`
- zk (Go) layers three date capabilities over stdlib time: a strftime formatter
  (`lestrrat-go/strftime`), a natural-language parser (`naturaldate`), and named styles. `[V]`
- Traces facts: `speedate` has one call site in `src/temporal/date.rs`. `winnow` has one call
  site in `src/temporal/lexer.rs`. It also locates duration prefixes for inline fields that reach
  the store. `logos` serves three temporal files plus the shared lexer, note parser, and query
  grammar. `jiff` is pinned `=0.2.38` with `std`, `tz-system`, `tzdb-bundle-always`. The
  extractor fingerprint hashes resolved versions of `jiff` and `speedate`; ticket 25's list omits
  `winnow`. `[V ticket]`

Product behavior: single-parser-crate designs dominate the read set. IWE shows that hand-rolled
scanners need their own adversarial tests. zk shows that layered capabilities (format, NL, named
styles) can come from separate small crates.

Lesson: `[P]` Crate decisions are coupling decisions, not style decisions. Removing `speedate`
changes the fingerprint epoch behavior of ticket 25. Removing `winnow` moves duration-prefix
locating into other code and requires the ticket 25 list fix. Keeping `logos` serves five
call-site clusters. `[A]` No digest measures any of these crates. The prep must run the
ticket 25 cross-check before any removal, and the grilling needs a measurement or an explicit
acceptance of the rebuild cost.

Critic note: the digest set has no speedate or winnow user. Absence of precedent is weak evidence
for removal. Say so in the grilling.

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
- Ticket 19 proposes a closed English grammar: keywords, weekdays, offsets such as `in 2 weeks`.
  `[V ticket]`

Product behavior: both shapes exist in production. The closed list gives determinism and
testability. The open grammar gives coverage and pays with ambiguity, locale switches, and
heuristic weekday inference.

Lesson: `[P]` The closed list is the Dataview precedent and matches ticket 19. The nldates
inventory is the requirements list for the closed grammar: relative keywords, weekday names,
offset phrases, period phrases (`this|next|last X`), and edge words (`end of`, `mid`) only if
the owner uses them. `[H]` The owner's templates and suggesters show no `Christmas`-style words;
keep the grammar closed.

### Q14: Build in the repository or use a crate?

Evidence:

- zk embeds `naturaldate`. Its failure path logs an error and returns a zero `time.Time`
  (`date.go:2233-2251`). A silent zero date is the worst error shape in the read set. `[V]`
- zk hard-exits the process when a renamed helper is used with the old spelling
  (`date.go:2243-2246`, `os.Exit(1)`). `[V]`
- nldates logs `"Invalid date"` at debug level and continues (`main.ts:414-426`). `[V]`
- Dataview's `date()` returns `null` on a bad string and throws on a bad format argument
  (`:6865-6909`). `[V]`
- Ticket 30 prep facts: `fuzzydate` 0.4.1 (MIT, ~2.6k lines, 14 releases, chrono dep with
  `clock`), `jiff-english` 0.0.1 (MIT, ~1.3k lines, one release, 29 downloads, `jiff ^0.2`
  default features + `scanlex`). Traces sets `default-features = false` for jiff. `[V ticket]`
- `.scratch/temporal-core/design-nl-dates.md` proposes an in-house parser; ticket 30 records four
  conflicts with decided rules. `[V ticket]`

Product behavior: error reporting, not parsing power, separates the integrations. Every crate
integration in the read set has a silent-or-drastic failure path. Dataview's typed `null` plus a
thrown error for a bad format is the closest to a usable contract.

Lesson: `[P]` The grilling needs the ticket's prep first. The digest evidence adds the error
contract requirement. That contract is: a span-carrying error, no zero-date fallback, no process
exit, no debug-log silence. `[H]` The in-house default stands unless a crate meets that contract and the coverage
list. The crate evidence in the ticket is thin (one release, 29 downloads) and cannot carry the
decision alone.

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

Product behavior: "which Monday" has three ecosystem answers: locale-indexed (Templater),
parser-flagged forward or backward (nldates/chrono), or index-into-current-week (Traces today,
ISO). None of the read tools answers "next vs nearest" as a stated rule; chrono hides the choice
in a flag.

Lesson: `[P]` A closed grammar cannot carry a hidden flag. State the rule: the ticket asks next,
nearest, or current-week day. `[H]` Pin the rule to the frame's week rule (current-week day).
That is deterministic and matches the existing template filter. `next` and `last` prefixes give
the other two meanings. Caveat: owner preference is unknown; ask in the grilling.

### Q16: Interval precision of `next week`

Evidence:

- Tasks models period phrases as ranges. `this week` builds `startOf(week)`..`endOf(week)`
  inclusive at day precision. `YYYY-W10` builds an isoWeek range. Moving a range by a month
  re-derives the end with `endOf(month).startOf('day')` because month lengths differ. The
  honest comment: "subtracting a month from a week long range was not tested"
  (`DateRange.ts:5043-5141`, `DateParser.ts:4953-5037`). `[V]`
- Dataview splits the two bounds into two spellings: `date(sow)` and `date(eow)` are separate
  point functions; there is no `next week` value (src `:7643-7666`). `[V]`
- nldates parses `next week` through chrono with `forwardDate: true`, which yields a point (same
  weekday next week), not a span; `next month` uses a two-stage anchor. `[V]`
- Ticket 21: `CalendarPeriod` becomes a normalized civil start plus a `CalendarUnit`; tickets 19
  and 21 need period values for `file.date`; periods match a span with an exclusive upper bound.
  `[V ticket]`

Product behavior: three shapes exist: span value (Tasks), two point spellings (Dataview), and
flag-driven point (chrono). Tasks' start-plus-end shape pays an end-adjustment tax on arithmetic.

Lesson: `[P]` Start-plus-unit (ticket 21) derives bounds on demand and avoids Tasks'
end-adjustment complexity. The exclusive upper bound needs one documented derivation rule per
unit; the Fileclass display precedent (store the canonical value, derive the reading) supports
that split.

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

Lesson: `[P]` Keep one spelling: keywords as function arguments, exactly as today. The
supersession note in ticket 29 then resolves by deletion, not by a second rule. `[H]` A bare
keyword would need a reserved-word list. It would also need a migration of any field named
`today` or `eom`. Nothing in the evidence needs that cost.

### Q18: Frontend spelling of `member_of`

Evidence:

- `[A]` No read digest has a membership operator or spelling. Dataview's emoji shorthands on
  tasks (`📅`) mark a date field membership on a task (docs `:1599-1640`), which is storage
  syntax, not a query operator. `[V]`
- Fileclass's measured failure is adjacent: a Bases `contains` filter on a scalar silently drops
  the note from the view, so membership semantics decide visibility without notice (validation
  messages in Fileclass teach the storage shape to prevent this). `[V]`
- Ticket 29 reserves only the IR name. `[V ticket]`

Product behavior: no precedent. The nearest lesson is negative: silent membership mismatches hide
rows.

Lesson: `[P]` The grilling can pick the spelling freely. It needs a paired notice rule, so a
scalar-versus-list mismatch is visible. This answers the Fileclass failure and ticket 34.

Grammar ambiguities worth recording, all `[V]`:

- `date(` parsing: Dataview requires the call form; a bare `today` stays a field. Traces shares
  this ambiguity and today resolves it the same way.
- Longest-match ordering: Dataview sorts shorthand keys and duration units longest-first, because
  `sow` must beat `so`-prefixes and `minutes` must beat `m`. Any closed Traces grammar needs the
  same deterministic order rule.
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

Product behavior: no read tool separates civil from instant values. The day-normalization hack
is the documented failure class that a civil variant removes: equality silently breaks when two
machines disagree on the zone.

Lesson: `[P]` The civil/instant split is Traces-native; the digests supply the failure story, not
a design. `[H]` An inner-enum `DatePoint` keeps `NoteFieldValue` inside 80 bytes. The reason: the
largest payload today is 72 bytes. An outer variant adds a discriminant plus the same payload.
Both shapes fit. The grilling decides on the 80-byte proof.

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

Lesson: `[P]` A `CalendarPeriod` variant with start plus unit follows the canonical-store
pattern and ticket 21. `[H]` A start-plus-end variant would import Tasks' end-adjustment
complexity into every shift. The 80-byte cap accepts either; unit is smaller than a second
endpoint.

### Q21: Date versus instant at an ambiguous or absent local midnight

Evidence:

- `[A]` No read tool handles an ambiguous or skipped local midnight. Dataview asserts validity
  of parsed literals only (`:7946`). `[V]`
- Fileclass avoids the question in duration math with floating UTC: "UTC math sidesteps DST";
  unparseable inputs return `null` (`duration.ts:3620-3688`). `[V]`
- Ticket 21 evidence: a skipped next day such as `2011-12-30` in `Pacific/Apia` must not
  invalidate the real day before it; period-versus-instant tests the civil date of the instant
  in the pinned zone and never tests midnight existence. `[V ticket]`

Product behavior: the ecosystem sidesteps; none solves.

Lesson: `[P]` Keep the ticket 21 rule separate from membership, as the ticket orders. The
Fileclass floating-UTC pattern is the rejected analogy: it hides the zone question and mislabels
civil values as UTC. State that rejection with the failure scenario: a floating-UTC date
arithmetic across a DST boundary disagrees with every civil calendar view.

### Q22: Total sort key for mixed temporal values

Evidence:

- Dataview compares by typed operator pairs: date-date, duration-duration, each with `<` and
  `equals` implementations; mixed pairs fall through its binary-op dispatch (`src`, comparison
  and binaryop modules). `[V]`
- `[A]` No read tool defines a total order across date, datetime, duration, and period values.
- Ticket 29 needs a "temporal class order" for group keys and leaves it undefined; this ticket
  defines it. `[V ticket]`

Product behavior: absent.

Lesson: `[P]` The class order is new ground. The Dataview typed-pair structure supports the
shape: compare within a class, tie-break across classes by a fixed class rank, and inside
periods tie-break by unit. The grilling picks the rank; the digests only prove that leaving it
implicit makes cross-class comparisons tool-dependent.

### Q23: Nominal duration equality and order

Evidence:

- Fileclass restricts durations to RFC 5545's non-nominal units (weeks, or days plus time; the
  code states RFC 5545 forbids months and years) and computes total milliseconds, so equality is
  millisecond equality (`duration.ts:3499-3688`). `[V]`
- Dataview's duration units include years, months, and weeks (`DURATION_TYPES`, src `:7602-7640`;
  changelog `dur(2yr8mo12d)`), and comparisons go through Luxon's millisecond value. Nominal
  units make that comparison calendar-dependent. `[V]`
- Ticket 19 keeps nominal equality and order provisionally and asks whether to refine. `[V
  ticket]`

Product behavior: forbidding nominal units makes equality trivial (Fileclass). Allowing them
buys a comparison ambiguity (Dataview).

Lesson: `[P]` The provisional rule stands unless the owner needs month-based durations.
`[H]` The Fileclass evidence gives a cheap refinement. Define nominal equality exactly at the
unit level. Treat 30-day months as the rule only if the owner states it. Or forbid nominal units
in stored durations and keep them in arithmetic. The grilling decides with the owner's real data.

### Q24: How documents describe calendar arithmetic

Evidence:

- Tasks documents its own limit in code: the month-shift end adjustment and the untested
  week-range case (`DateRange.ts:5108-5140`). `[V]`
- Dataview docs teach arithmetic by example: `date + duration`,
  `this.release-date - date(now)`, and duration render rules (docs `:1842-1858`). `[V]`
- IWE documents its specifier set in a "Date format patterns" table (digest `:9888-9890`). `[V]`

Product behavior: honest limits in code comments; example-led docs; explicit specifier tables.

Lesson: `[P]` Document the arithmetic as three lists: exact rules (civil add), rule-with-caveat
(nominal add, DST), and undefined cases (skipped days). Ticket 34 carries the caveat notices.
The Tasks comment is the tone model for the undefined cases.

### Q25: How `date(link)` reads a Markdown link target

Evidence:

- Dataview's cascade order is display text first, then path, then the file's derived day (src
  `:6872-6890`). A user-chosen alias can therefore produce the date. `[V]`
- Fileclass expands patterns inside link paths and keeps literal text literal
  (`expandLinkPath`, `dateFormatCheck.ts` area `:3180-3318`). `[V]`
- Ticket 19 decides: read the last segment of the target only, accept a pattern argument for
  non-ISO titles, keep the extension, allow percent-encoding, exclude external URLs. This ticket
  fixes that pattern's dialect. `[V ticket]`

Product behavior: display-first is the documented hazard. Path-token expansion is the precedent
for pattern-in-target.

Lesson: `[P]` The last-segment rule removes the display hazard by construction. `[H]` The
pattern argument should be the same dialect as `date(text, pattern)` (Q3) so one dialect serves
both. `[A]` No digest covers percent-encoded targets; the prep needs one test with an encoded
target before the grilling.

## Q26 to Q32: Configuration and zone

### Q26: `[date]` table syntax and validation

Evidence:

- IWE's config uses TOML tables with strict field sets; an unknown field error lists the exact
  expected names (`unknown field refs_extention, expected one of ...`, digest `:1649`). `[V]`
- Ticket 22 decides: `config` owns and parses `[date]` into `DateConfig`; `deny_unknown_fields`
  stays; merge is field-by-field global then local; no `version` key. `[V ticket]`

Product behavior: strict tables with name-listing errors are the IWE precedent.

Lesson: `[P]` Fix the field set exactly (Q27), validate at parse, and let the unknown-field error
carry the expected names. The IWE typo in its own error message (`refs_extention`) shows that
these errors get read and that their text matters.

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

Lesson: `[P]` The field list follows the hand-off. Each field needs a default and a validation
rule at parse time (Q26). `[H]` Keep format fields as one dialect (Q3) with per-surface
defaults, mirroring IWE's key-versus-content split only if Traces has two real surfaces.

### Q28: Temporal-owned policy without importing config

Evidence:

- IWE parses config into a `Configuration` and engines consume accessors (`format_options()`),
  so the config crate keeps the TOML knowledge (`:13744-13873`, `:65575`). `[V]`
- Traces fact: the edge `config -> temporal` already exists; ticket 30 keeps it. `[V ticket]`

Product behavior: parse in config, consume as plain values.

Lesson: `[P]` Temporal receives a plain `DateConfig`-shaped policy value. No new edge, no
temporal-to-config import. This ratifies the ticket 22 direction with IWE as the precedent.

### Q29: Configured zone syntax and validation

Evidence:

- `[A]` No read tool configures a zone in config. Dataview accepts zones inside value literals:
  `Z`, offsets `+02`, `+0200`, `+02:00`, and `[Area/City]` names in its literal grammar (src
  `:7928-7945`). `[V]`
- IWE's locale-string normalization (strip `.UTF-8`, `@euro`, map `de-DE` to `de_DE`, invalid to
  `POSIX` with a fallback) is the nearest validation pattern (`locale.rs:54931-54990`). `[V]`

Product behavior: zone syntax lives in values; config-level zone is new ground. Locale-string
normalization is the proven validation pattern.

Lesson: `[P]` Accept IANA names plus fixed offsets. Validate at parse. The parse error names the
accepted forms. Reuse the normalization discipline from IWE. `[A]` No digest evidence constrains
the exact accepted set; the grilling decides it.

### Q30: Host system zone and failure reporting

Evidence:

- `[A]` No read tool reports a zone-acquisition failure. Every plugin runs inside the host app
  with full ICU data.
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
locale-following, Moment paired rules. `[V]`

Lesson: `[P]` If configurable, the config needs the week-one companion (Q8). The functions
`sow`/`eom`/`eoy` and template `date.weekday` must read the same rule from the frame. Otherwise
the Dataview-versus-Tasks inconsistency reappears inside one product.

### Q32: Opt-in key for bare-year filename prefixes

Evidence:

- `[A]` No read tool parses year-only filename prefixes.
- Nearest analogs: Dataview's value ladder accepts `YYYY` and `YYYY-MM` as values (docs
  `:1788`); Traces' `scan_date_prefix` accepts `YYYY-MM-DD` and `YYYY-MM` and never an instant;
  ticket 21 adds quarters and an opt-in bare year. `[V]`

Product behavior: absent.

Lesson: `[P]` The opt-in key obeys the ticket 22 config rules (exact field set, deny
unknown). `[H]` A boolean key is enough. The parser change is the real content. Ticket 25's
`IndexPolicy` rule applies only if the index persists parsed periods.

## Q33 to Q36: Frame and pushdown

### Q33: What temporal exports to the execution frame

### Q34: Replacements for ambient reads

Evidence, shared:

- Every read tool anchors relative words to parse-time ambient state: Dataview's shorthand table
  calls `DateTime.local()` during parse (src `:7950-7954`); nldates uses `window.moment()` and
  the runtime locale; Tasks uses `window.moment()` in `DateParser`. `[V]`
- No read tool has an injected clock or zone. `[A]`
- Ticket 21 decides: one frame per query, render, or request pins clock and week rule; the frame
  acquires the zone lazily and memoizes failures; ambient `local_zone()`, `now_instant()`, and
  `ZoneGuard` leaves. `[V ticket]`

Product behavior: ambient parse-time anchoring everywhere; the bug class is stale or
device-dependent results on replay.

Lesson: `[P]` The frame exports exactly what the closed grammar and helpers read. That is:
`today` (civil, in the pinned zone), the week rule, and the zone handle. `Anchor` wrappers (Q6)
take the frame. This is Traces-native; the digests supply the failure story only.

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

- Fileclass measured the silent-drop failure: a `contains` filter skips scalar-stored values, so
  notes vanish from a view with no message; its validation messages then teach the storage shape
  (`dateFormatCheck` message design, `[V]`).
- nldates logs `"Invalid date"` at debug level and continues (`main.ts:414-426`). `[V]`
- Dataview yields `null` for unparseable `date()` input with no notice (`:6865-6909`). `[V]`
- Ticket 34 owns the mechanism; ticket 29 did not adopt the first-round notice. `[V ticket]`

Product behavior: the norm is silence.

Lesson: `[P]` Silence is the rejected analogy with a measured cost (invisible rows). `Notice`
on: an unparseable date-shaped literal that filters a row out, and a scalar-versus-list
membership mismatch. `[H]` A plain absent field is not a notice case, or every vault gets noisy.
The grilling draws that line with ticket 34.

## Analogies that do not transfer

Each entry states the failure that disqualifies it as a Traces pattern.

- zk's zero-time fallback for NL parse failure (`date.go:2238-2242`): a zero date silently joins
  comparisons and sorts as an epoch value. `[V]`
- nldates' private-API locale read (`moment.localeData()._week.dow`): week labels shift when the
  host locale changes, and the API can vanish. `[V]`
- Dataview's display-first `date(link)` cascade: an alias decides the value. `[V]`
- Tasks' mutable start-plus-end range with end re-derivation on shift: arithmetic must patch two
  endpoints per unit, and its own comment admits untested combinations. `[V]`
- Fileclass's RFC 5545 duration subset: forbidding months and years is clean for stored values
  but does not cover nominal arithmetic that ticket 19 keeps provisionally. `[V]`
- Metadata Menu's raw moment format over whole link paths: literal text mangles (`Daily` ->
  day-of-month). Token expansion must be explicit. `[V]`
- `obsidian-rs` unwrap chains and basalt's `0` sentinel: platform reads need typed failures,
  not crashes or silent zeros. `[V]`
- Locale-following week tokens as a single rule: Moment must expose two rule sets because one
  set cannot serve both conventions; a deterministic store cannot follow a device locale. `[V
  web]`

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
| 8 | Moment dow/doy; Tasks isoWeek; nldates locale read | Week start pairs with week-one rule `[P]` |
| 9-12 | IWE chrono; zk layered crates; ticket fingerprint facts | Decide by coupling and fingerprint cost, not style `[P]` |
| 13 | nldates chrono patches; Dataview closed shorthands | Closed list matches ticket 19 `[P]` |
| 14 | zk naturaldate failures; crate facts; design-nl-dates conflicts | In-house default; crate needs error-contract proof `[P]` |
| 15 | Templater locale rule; chrono forwardDate; Traces ISO weekday | Pin to frame week rule; state the rule `[P]` |
| 16 | Tasks DateRange; Dataview sow/eow; nldates chrono points | Start-plus-unit period; exclusive upper bound `[P]` |
| 17 | Traces grammar facts; Dataview identifiers and keywords | Keywords as call arguments only `[P]` |
| 18 | Absence; Fileclass silent-drop failure | Spelling free; pair with notice rule `[P]` |
| 19 | Tasks startOf-day comment; Dataview single type; byte facts | Civil variant fits 80 bytes; prove shape `[P]` |
| 20 | Tasks DateRange; Fileclass canonical-store pattern | Start-plus-unit variant `[P]` |
| 21 | Absence; Fileclass floating-UTC; ticket 21 Apia facts | Keep separate from membership; reject floating `[P]` |
| 22 | Dataview typed pairs; ticket 29 gap | New class-order rule; typed-pair shape `[P]` |
| 23 | Fileclass non-nominal subset; Dataview nominal units | Provisional rule stands; refine or forbid `[P]` |
| 24 | Tasks code comments; Dataview docs; IWE specifier table | Three-list documentation model `[P]` |
| 25 | Dataview display-first cascade; Fileclass path tokens | Last segment; one dialect for pattern arg; test encoding `[P]` |
| 26 | IWE strict tables and error text; ticket 22 rules | Exact field set; name-listing errors `[P]` |
| 27 | IWE fields; nldates settings; ticket 19 hand-off | Field list per hand-off; defaults and validation `[P]` |
| 28 | IWE accessors; existing config-to-temporal edge | Plain policy value; no new edge `[P]` |
| 29 | Absence in config; Dataview value-literal zones; IWE locale normalization | IANA plus offsets; parse-time validation `[P]` |
| 30 | Anti-precedents (unwrap, zero); ticket 21 rules | Typed memoized failure; feed ticket 34 `[P]` |
| 31 | Q8 evidence | Configurable only with paired week-one rule `[P]` |
| 32 | Absence; Dataview ladder; ticket 21 opt-in | Boolean opt-in key under ticket 22 rules `[P]` |
| 33-34 | Ambient parse-time anchoring everywhere; ticket 21 frame rules | Frame exports today, week rule, zone handle `[P]` |
| 35 | Absence; Dataview indexed-data note; ticket 29 scope | Prove or leave Unsupported `[P]` |
| 36 | Fileclass silent-drop cost; nldates debug log; Dataview null | Notice on exclusion mismatches; line with ticket 34 `[P]` |

Open items this file does not settle, by design:

- Crate decisions Q9 to Q12 need the ticket 25 cross-check and a measurement; the digests have
  none. `[A]`
- Crate review for Q14 belongs to the prep step and the external-designs peer, not to digests.
- Percent-encoded `date(link)` targets (Q25) have no digest evidence; one test before grilling.
- The owner's bare-weekday preference (Q15) is a question for the grilling, not for digests.
