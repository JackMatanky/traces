# Where do temporal, format dialects and calendar helpers belong?

Type: grilling
Status: claimed
Blocked by: 7, 14, 16, 18, 19, 20, 21, 22, 23, 24, 25, 29, 32

## Question

The survey said temporal is 10k lines of code (3k production). It said
`template/engine/date.rs` is 2.5k lines of code. Both numbers are stale. See
"Current code" below. Scouts judged `DatePoint`, `DateValue`/`DateTimeValue` and
`DurationValue` sound. Nobody has verified that judgment.

Decide the temporal boundary. Answer each numbered question separately.
Use [ticket 36's decision-record
rules](./36-reconcile-decisions-and-audit-coverage.md#decision-records-and-evidence-gates).
Keep decision evidence separate from implementation gates.

### Moment compatibility and format dialects

1. Decide whether Moment compatibility is a domain concept. Ticket 16 makes
   parity with Templater free. Research 07 says Moment is not a core concept.
2. Decide where the Moment dialect translator lives. It already sits in
   `src/temporal/token.rs`. Ratify that move, return it to a template adapter,
   or drop Moment tokens.
3. Decide the dialect for each of four format surfaces:
   - Config `format`.
   - Query `date(text, pattern)`.
   - Template `date_format`.
   - Schema `date.format`.
   Choose one dialect or justify a split.
4. Decide how the `date.format` value check treats a format with no full date.
   Examples are `YYYY`, `Q` and `DDDD`.
5. Decide whether `format` under `[frontmatter.date_created]` and
   `[frontmatter.date_modified]` stays, joins the new `[date]` table, or goes.

### Calendar helpers and filenames

6. Decide which calendar helpers stay in `src/template/engine/date.rs`.
   The candidates are `Anchor`, `apply_offset` and `weekday`.
7. Decide where the filename-period parser lives. It replaces the `Dataview`
   filename-day derivation in `src/file.rs`. Ticket 21 retires `file.day`.
8. Decide the week-one rule for a custom week start. Ticket 21 fixes the ISO
   rule for the default week only.

### Parsing stack

9. Decide whether `speedate` stays. It has one call site.
10. Decide whether `winnow` stays. It has one call site.
11. Decide whether `logos` stays as part of the parsing base.
12. Decide whether `jiff` stays as part of the parsing base.

The old question said "four call sites". The code has four parser crates.
See "Current code".

### Natural-language dates

13. Decide the grammar. Ticket 19 proposes a closed English list of keywords,
    weekdays and offsets such as `in 2 weeks`.
14. Decide whether to build the grammar in the repository or use a crate.
    The candidates are `jiff-english` and `fuzzydate`. Run the prep step first.
15. Decide the meaning of a bare weekday: next, nearest, or the day in the
    current week.
16. Decide how the grammar represents the interval precision of phrases such as
    `next week`. Tickets 19 and 21 separate periods from arithmetic points.
17. Decide the filter spelling of relative keywords such as `today` and `eom`.
18. Decide the frontend spelling of `member_of`.

### Value shapes

19. Decide the type shape for civil and instant values: a `DatePoint` variant or
    an inner enum. Keep `NoteFieldValue` at 80 bytes.
20. Decide where `CalendarPeriod` sits in the value model. Ticket 18 lists no
    period variant. Tickets 19 and 21 need period values for `file.date`.
21. Decide how an ordinary Date field compares with an instant when local
    midnight is ambiguous or absent. Keep this rule separate from period
    membership.
22. Decide the total sort key for mixed temporal values. Include the unit
    tie-break for periods.
23. Decide whether to refine nominal duration equality and order.
24. Decide how the documents describe calendar arithmetic.
25. Decide how `date(link)` reads a Markdown link target.

### Configuration and zone

26. Decide the `[date]` table syntax and its validation.
27. Decide the fields of the config-owned `DateConfig`.
28. Decide which temporal-owned policy receives those fields without importing
    `config`.
29. Decide the syntax and validation of a configured zone.
30. Decide how the host gets the system zone and reports a failure.
    Ticket 22 fixes the precedence.
31. Decide whether the week start is configurable.
32. Decide the opt-in key for bare-year filename prefixes.

### Boundary after the execution-context decision

33. Decide what temporal exports to the execution frame.
34. Identify the replacements for ambient `local_zone()`, `now_instant()` and
    `ZoneGuard` reads. Ticket 21 already decides their removal.

### Query pushdown and notices

35. Decide the temporal probe bounds for each operator and each eligible value
    kind. An unproved mapping scans.
36. Decide when a row with a `Null` date gets a notice.
    Ticket 34 owns the notice mechanism.

Facts: `survey/temporal.md`, `survey/template.md` H2,
`survey/root-primitives.md` §3, `src/schema/fields/date.rs` (unvalidated
format). Ticket 16 rules that the divergence register is not an input. Use
`.scratch/temporal-core/divergence-register.md` only as a pointer. Check each
claim in code. Ticket 16 gives the reason: the register belongs to
temporal-core.

Alternatives:

- Keep as is. That is the landed state: translator in `src/temporal/token.rs`,
  calendar edges in `src/temporal/calendar.rs`, anchors in the engine, an
  unvalidated schema format, and four parser crates. Say what withstood
  scrutiny.
- Move the remaining dialect code into temporal.
- Keep the dialect code in a sibling `format` unit outside the template. Schema
  `format` validation and the CLI would share that unit.
- Return Moment translation to a template compatibility adapter.
- Drop Moment tokens in favour of strftime. Ticket 16 already makes that option
  Migratable, so it needs no further compatibility decision.
- Build the natural-language grammar in the repository.
- Use a crate for the natural-language grammar.
- Defer the natural-language grammar to fog only with an explicit scope
  decision. Preserve the hook from ticket 19.

Evidence the owner needs for the dialect decision: the owner's tokens (see
"Current code"), the unsupported-token behavior, and the cost of the change
table row.

Dependencies on open tickets: this ticket needs no open answer. Ticket 33 later
wires the frame source and `DateConfig` into the host. Ticket 34 later owns the
notice mechanism. Ticket 35 later fixes the crate cut and the pruning. Ticket 31
waits on this ticket.

Inputs: `research/07`, execution-context decision, coercion policy, compat
envelope.

## Current code

[V] means that someone read or measured the code on main @ `cf6d79bd`.

- `src/temporal` has 12 files and about 11.8k lines. About 4.0k lines are
  production code [V].
- Two files are new since the survey: `src/temporal/token.rs` and
  `src/temporal/fmt.rs`. The plan
  `docs/superpowers/plans/2026-10-22-temporal-offload.md` created them [V].
- Commit `6104d0ae` moved the Moment translator to `src/temporal/token.rs`. The
  file holds `pattern_to_strftime`, `MOMENT_TOKENS` and the ordinal suffix [V].
  Research 07 challenged such a move [H].
- `pattern_to_strftime` treats a `%` outside brackets as strftime. Otherwise it
  translates Moment tokens. An unknown token returns
  `TemporalTokenError::UnsupportedToken` [V].
- `MOMENT_TOKENS` has no `ww`, `Q`, `Qo` or `DDDD`. By this code those tokens
  fail [V by reading, not run].
- `src/template/engine/date.rs` has 2,512 lines. About 880 lines are production
  code. It keeps `DateOps`, `Anchor`, `apply_offset`, `weekday` and the `date_*`
  filters [V].
- `PERIOD_EDGE_LOOKUP` and `resolve_period_edge` live in
  `src/temporal/calendar.rs`. Query functions and template filters share them
  [V].
- The query grammar has functions such as `sow`, `eom` and `eoy` that take a
  date argument. It has no relative keyword `today` or `eom` [V].
- A bare word in a filter parses as a field path. A word followed by `(` is a
  function call. A bare `today` would name a field [V].
- Today `CalendarPeriod` has the variants `Week`, `Month` and `Year`. `Week` is
  the ISO week, Monday to Sunday [V].
- Template `date.weekday(n)` selects ISO weekday `n` of the current week [V].
- `speedate` has one call site in `src/temporal/date.rs`. `winnow` has one call
  site in `src/temporal/lexer.rs` [V].
- `logos` serves `lexer.rs`, `datetime.rs` and `point.rs` in temporal. It also
  serves the shared lexer, the note parser and the query grammar [V].
- `jiff` is pinned as `=0.2.38` with the features `std`, `tz-system` and
  `tzdb-bundle-always` [V].
- `FileMeta::derive_day` in `src/file.rs` reads the base name with
  `DatePoint::parse_prefix`. It falls back to the creation time. `FileMeta`
  stores the result in `day` [V].
- `scan_date_prefix` in `src/temporal/lexer.rs` recognizes `YYYY-MM-DD` and
  `YYYY-MM` only. It never yields an instant [V].
- The note parser also calls `DatePoint::parse_prefix` for task dates and inline
  fields. Ticket 16 freezes that parsing [V].
- `SchemaDateField` in `src/schema/fields/date.rs` stores `format` as an
  unvalidated `Option<String>`. Its accessor exists only under `#[cfg(test)]`
  [V]. The schema unit tests use `YYYY-MM-DD`.
- Config has `[frontmatter.date_created]` and `[frontmatter.date_modified]`.
  Each has `name` and `format`. The default format is `%Y-%m-%dT%H:%M:%S`.
  Ticket 22 records that no production code reads them [V]. Config has no
  `[date]` table [V].
- Query `date(text, pattern)` calls `DatePoint::parse_with_pattern`. That
  function accepts strftime only [V].
- ADR-0010 has the status `proposed`. It lexes a bare ISO date as one literal
  [V].
- `tests/features/temporal/temporal-dates.feature` pins nine scenarios. It is an
  oracle for current behavior [V].
- `src/temporal/GLOSSARY.md` still says that unzoned input uses the process zone
  and that a Date promotes to local midnight [V].
- The owner's vault templates write Moment tokens that the translator rejects.
  `docs/vault/07_templates/12_01_week_periodic.md` uses `ww`, `YY` and `Q`.
  `docs/vault/07_templates/14_01_quarter_periodic.md` uses `Q` and `Qo`. The
  week template lists days from `day(0)`, which is Sunday [V]. Traces does not
  run these Templater scripts. They show which tokens the owner writes.
- The owner's Fileclass data in `docs/vault/05_metadata/cal.md` has Date fields
  with `dateFormat` values `YYYY-MM-DDTHH:mm`, `YYYY`, `DDDD` and `Q` [V].

## Prep before grilling

The natural-language grammar needs a prep step before the grilling. Ticket 19
says nobody reviewed `jiff-english` or `fuzzydate`. A search of `.scratch` finds
no review of either crate [V].

Facts from the crates.io API, read on 2026-11-16 [V]:

- `fuzzydate` 0.4.1 has the MIT licence, about 2.6k code lines and 14 releases.
  It depends on `chrono` (feature `clock`), `lazy_static` and `thiserror`.
  `Cargo.toml` has no `chrono` entry.
- `jiff-english` 0.0.1 has the MIT licence, about 1.3k code lines and one
  release, dated 2026-08-07. It has 29 downloads. It depends on `jiff` `^0.2`
  with default features and on `scanlex` `^0.1`. Traces sets `default-features =
  false` for `jiff`.

The file `.scratch/temporal-core/design-nl-dates.md` (2026-11-12) proposes an
in-house parser with no new dependency. It says seven crates were evaluated and
rejected. It names none of them. An agent wrote it and nobody reviewed it [H].
It conflicts with the decisions above in four places:

- It returns a `DatePoint` with month precision for `next month`. Ticket 19
  needs an interval.
- It adds a template function `date.relative`. Ticket 19 needs one grammar for
  filter literals, `date()`, `date_add` and template filters.
- It reads `now_instant()` and `ZoneGuard`. Ticket 21 replaces both with an
  injected frame.
- It fixes weekday meanings that this ticket must still decide.

The prep step must give the owner this evidence for each candidate:

- Licence and release history.
- Size, dependencies and effect on binary size under the shipped `opt-level =
  "z"` profile.
- Determinism: an injected anchor, no ambient clock and no ambient zone.
- Week rules: the week start and the week-one rule.
- Locale: English only, and the order of month and day.
- Output: a period with a unit, not one point.
- Error reporting: a span that a query error can carry.
- Coverage of the closed grammar that ticket 19 proposes.
- Feature and version coupling with `jiff` `=0.2.38`.
- The size and test cost of a grammar built in the repository.

[P] Run the prep first. Treat a grammar built in the repository as the default
option until the evidence beats it.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Handed to this
ticket:

- **`DateConfig` sub-Config type.** Consider defining a `DateConfig` type for
  temporal settings. Candidate fields include default date and datetime formats,
  week-start rules and weekday resolution for relative phrases. Further fields
  include natural-language grammar options and zone source if placed in
  configuration. Where it lives and how it validates depends on [config
  ownership and lifecycle](./22-config-ownership-and-lifecycle.md). That ticket
  blocks this one.
- **Natural-language dates.** A deterministic, closed English grammar (keywords,
  weekdays, offsets such as `in 2 weeks`), resolved from the injected `today`,
  shared by filter literals, `date()`, `date_add` and template filters. Choose
  the grammar and whether to build on `jiff-english` or `fuzzydate` (neither
  reviewed). Fix the meaning of a bare weekday (next vs nearest). The grammar
  must yield an interval precision for phrases such as `next week`.
- **Civil vs instant.** Naive datetimes are civil values and only input with `Z`
  or an offset is an instant. Decide the type shape (a variant or an inner enum)
  and how `DatePoint` precision survives.
- **Durations.** Nominal equality and order are kept provisionally. Decide
  whether to refine them and how calendar arithmetic is documented.
- **`date(link)`** reads only the link target's last segment and accepts a
  pattern argument for non-ISO titles. Ticket 19 decides this. This ticket fixes
  the dialect of the pattern argument.
- The temporal glossary line saying a Date promotes to local midnight is
  superseded.
- A Markdown link is open. Ticket 19 asks for a rule for how `date(link)` reads
  a Markdown target. A Markdown target keeps its extension and can be
  percent-encoded. External URLs are not note targets.

Other decided rules that bind this ticket:

- Temporal brackets hold `Date`, `DateTime` and `CalendarPeriod`. Across
  brackets `==` is false and relational operators do not match.
- A quoted date-shaped literal parses once into candidates. The candidates are a
  `DatePoint`, a validated `CalendarPeriod` and a `DateTime`.
- A period literal matches a whole span. The upper bound is exclusive.
- A build-time advisory notice covers an invalid date-shaped quoted literal. A
  bare invalid date literal stays a syntax error.
- Inline fields can now yield `DateTime`.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). Schema
`date.format` is a write-format hint. The note model has no time-of-day or
year-only variant. The `date` check accepts any scalar when the format has no
date component. Tests in `schema` use Moment tokens. Temporal and the template
filter seam translate them. Pick the dialect. After choosing a dialect, add the
date-format check.

The Answer text of ticket 20 is cut off after "Only date, datetime and". Do not
read a rule into that fragment.

## Inputs from the Fileclass research

See [research 14](../research/14-fileclass-model-binding-and-validation.md).
Fileclass checks dates with ISO regexes only. A field with a custom Moment
`dateFormat` accepts any non-empty string. It also accepts a wikilink as a date
(`src/fields/validate.ts`) [V]. `dateFormatCheck.ts` instead lints the tokens of
the declared format string, not values [V]. Fileclass validates durations
against an RFC 5545 subset (weeks, or days and time; no years or months)
(`src/fields/duration.ts`) [V].

[H] Two cheap options exist for the dialect choice. Check a declared format at
schema load as a schema error, not a note violation. Keep the value check
separate until a dialect is fixed. IWE gives no input: it has no date type and
hands YAML dates to its validator as ISO strings.

[H] The research says Traces can validate structured formats with a dedicated
parser crate. Nobody verified that claim. `jiff` already provides strftime
parsing. Research 14 also says `date.format` stays a write hint unless a native
temporal engine supports a check.

The owner's own Fileclass data uses formats that are not full dates: `YYYY`,
`DDDD` and `Q` (see "Current code") [V].

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md).
Rename the existing kind-only `CalendarPeriod` to `CalendarUnit` (Day, Week,
Month, Quarter, Year). Use `CalendarPeriod` for a normalized civil start plus
unit. Keep `DatePoint` written precision and arithmetic separate.

A period compared to an instant tests the civil date of the instant in the
pinned zone. It does not test the existence of next day midnight. A skipped
local day has no matching instants. Civil Date and civil DateTime points use
their civil dates without a zone.

Period-vs-period filter equality is exact `(start, unit)` identity. Relational
order uses that key with an explicit unit tie-break. Mixed-temporal sorting has
a distinct total key.

Day/month filename prefixes continue. Add validated ISO weeks (Monday, four-day
week one) and quarters, with an opt-in bare year. Do not reinterpret invalid
longer prefixes as shorter prefixes. Custom week starts also need a week-one
rule.

Scope the new prefix parser to filenames: shared vault-format parsing remains
frozen under ticket 16. `file.date` is filename-only. `file.cdate` is
creation-only and `Null` if absent, never mtime. Civil `cdate` and `mdate`
project stored timestamps in the pinned zone during evaluation, with a typed
failure if that zone is unavailable. `file.day` is retired. Set the `DateConfig`
syntax and the system-zone acquisition/override policy with ticket 22.

Further decided rules that bind this ticket:

- One frame for each query, render or request pins the clock and the week rule.
  The frame acquires the zone only when needed. A failure is memoized.
- A host can select UTC explicitly. UTC is never an implicit fallback.
- Ambient `local_zone()` leaf reads, per-row zone lookups and the test-only zone
  guard leave after injection gives deterministic fixtures.
- Temporal owns validated periods, week rules and explicit-zone civil
  conversion. Query owns the pinned frames. Template owns the render frame.
- Ticket 21 leaves these items to later tickets: the unit tie-break for period
  sort, compact encoding, week formats beyond ISO and the configured zone
  source.

Evidence from the prep files for ticket 21
([e-design-temporal](../research/21-prep/e-design-temporal.md) and
[r5-temporal-adversarial](../research/21-prep/r5-temporal-adversarial.md)):

- [H] Fixed Monday is not automatically the existing Traces rule. Check it here.
  The check is done: `CalendarPeriod::Week` is the ISO week today [V]. The
  owner's week template uses Sunday-first listing (see "Current code").
- [H] A week-start setting alone does not define week-year numbering. The
  minimum days in week one also matter. ISO `2026-W01` starts on `2025-12-29`.
  ISO `2025-W53` is invalid. Accept no rollover from an invalid week.
- [H] A skipped next day, such as `2011-12-30` in `Pacific/Apia`, must not
  invalidate the real day before it.
- [H] A filename period needs a kind and a start, not one `DateValue`. Changing
  `DateConfig` can change how a stored week label reads.

## Inputs from the config decision

See [ticket 22's Answer](./22-config-ownership-and-lifecycle.md). Ticket 22
hands the `[date]` syntax, its validation, and the zone-source and override
policy to this ticket.

Decided by ticket 22:

- `config` owns and parses the `[date]` table into `DateConfig`. This ticket
  decides the syntax.
- The zone precedence is: host override, then config, then system.
- The time zone never enters the fingerprint.
- Ticket 22 removed `title`, `aliases` and the date keys from the fingerprint.
  No production code reads them. An index-time read must add its key to
  `IndexPolicy`.
- `IndexPolicy` holds the filename-period policy only if the index stores parsed
  periods.
- `deny_unknown_fields` stays. The new table needs an exact set of fields.
- The merge applies the global config first, then the local config, field by
  field. State whether `[date]` follows that rule.
- Ticket 22 added no `version` key to the config format.
- Config values from a local file apply only after the trust gate passes.

Open for this ticket:

- The syntax and validation of `[date]`.
- The fields of `DateConfig`.
- The temporal policy that receives the config fields. Ticket 22 already gives
  `DateConfig` to `config`.

Temporal now imports `DelimiterType` from the crate root in
`src/temporal/token.rs`. The survey claim of zero dependencies is stale.
`config` reads `DEFAULT_DATETIME_PATTERN` from temporal. [P] Keep the edge
direction from `config` to temporal.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- The extractor fingerprint hashes the resolved versions of `jiff` and
  `speedate`. A change to the parsing stack changes the fingerprint and forces
  one rebuild. Update the crate list in ticket 25 if this ticket removes a
  crate.
- The crate list in ticket 25 omits `winnow`. `winnow` locates duration prefixes
  for inline fields, and those values reach the store. Keep `winnow` in the
  list, or remove `winnow` from the code.
- A civil datetime must deserialize from the cache without a zone lookup.
- Until the civil variant exists in code, the fingerprint corpus uses only
  datetimes with an explicit offset. Add the civil variant when it lands. That
  change bumps the format epoch.
- Ticket 25 does not fingerprint filename-period rules. The store does not
  persist filename periods. If a persisted facet starts to use those rules, the
  rules enter `IndexPolicy` and the corpus.

## Inputs from the read model decision

See [ticket 24's Answer](./24-index-read-model-and-query-execution.md).

- The store persists source timestamps and size. It does not persist `day`. A
  query derives the filename period. This ticket owns that parser.
- The store keeps temporal values as text until a compact encoding shows a
  measured gain. Postcard re-parses them on each load.
- A range key needs proof for each operator, row kind, temporal frame and field
  precedence. Unsupported mappings scan.
- The store persists naive datetimes as civil values. It holds no zone.
- Research 25 (`research/25-index-freshness-prep.md`) says a persisted key for
  `file.mdate` keeps UTC instants, not civil dates.
- Ticket 24 lists tickets 25, 29, 31, 32 and others as downstream. It does not
  list this ticket.

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md).

- `NoteFieldValue` has the variants `Date`, `DateTime` and `Duration`. It has no
  period variant. Ticket 18 keeps an 80 byte size test.
- `DurationValue` is 72 bytes. `DateTimeValue` is 16 bytes. `DateValue` is 4
  bytes [V in research 11].
- A new civil variant must keep `NoteFieldValue` at 80 bytes.
- Native TOML dates in schema data read as strings.
- Ticket 19 revoked the exclusion of inline `DateTime`.

## Inputs from the note document decision

See [ticket 23's Answer](./23-note-document-model.md).

- Inline fields can yield `DateTime`. A naive one is civil.
- The parser classifies a value once (ticket 19). Frontmatter and inline fields
  share one parser set (ticket 18).
- Several inline values form a list.
- Task dates stay civil `DateValue` values. `TaskDateSet` holds six of them.
- Ticket 23 has a stale line. Its Question says to add the date keys to
  `IndexPolicy`. Its later input says ticket 22 removed them.

## Inputs from the compat envelope

See [ticket 16's Answer](./16-grilling-compatibility-envelope.md).

- Dataview and Templater semantic parity is free.
- Template function API and query DSL spelling are Migratable. That includes
  heuristic string-to-date promotion.
- Template and query capabilities are frozen. They include date arithmetic.
- `config.toml` and schema TOML formats are Migratable. A `[date]` table and a
  `date.format` check need a change-table row.
- Vault-format parsing is frozen. That includes date and duration shapes in
  frontmatter, inline fields and tasks.
- The divergence register is not an input.
- The Rust library surface is free. The visibility rule still applies.

## Inputs from the template bridge research

See [research 07](../research/07-template-bridge-precedent.md).

- Minijinja contrib date filters use strftime. They hold no Moment logic [V].
- Templater delegates Moment to Obsidian's runtime `moment`. It translates
  nothing [V].
- Jiff and chrono support strftime only [V].
- [H] Moment compatibility is a Templater compat feature, not a temporal
  concept. The research proposes a template compat module. It allows a
  `temporal::format` parser only if schema formats need Moment syntax.
- The code now disagrees with that research: the translator sits in temporal.

## Inputs from the path decision

See [ticket 32's Answer](./32-path-and-capability-model.md). `RelativePath` is a
pure UTF-8 string with `/` separators. The scan skips non-UTF-8 names with a
notice. The filename-period parser reads the last segment of that string. Today
`derive_day` reads a `Path` through `BaseNameRef::from_path`.

## Inputs from the query-core decision

See [ticket 29's Answer](./29-query-core-and-plan.md).

- Bind relative keywords and range-key bounds to the pinned frame at plan time.
- Choose the `member_of` frontend spelling. Ticket 29 reserves only the IR name.
- Define the filename-period parser and the `date.format` dialect.
- Keep filename-derived `file.date` keys Unsupported until the parser contract
  is complete. `file.day` is retired.
- Specify temporal probe bounds for each operator and eligible value kind. An
  unproved mapping scans instead of excluding candidates.
- Decide with ticket 34 when a row with a `Null` date gets a notice. Ticket 29
  did not adopt the first-round notice.
- Group keys: a `Date` and a civil-midnight `DateTime` share a key when their
  civil values agree. Civil values and instants stay separate classes. A period
  groups by `(start, unit)`. Membership never defines a group key or a range
  key.
- Ticket 29 sorts distinct group keys by "temporal class order". Nobody has
  defined that order. The sort key decision above defines it.
- The first pushdown scope covers unquoted temporal literals and `And` only.

## Downstream consumers

- [Ticket 31](./31-template-engine-boundary.md) is blocked by this ticket. It
  needs these decisions:
  - The owner and behavior of `date_format` and the Moment translator.
  - Which calendar helpers stay in `src/template/engine/date.rs`.
  - The names of template functions and filters that accept natural language.
  - How the engine renders civil, instant and period values.
  - The frame that the engine receives, and the error type for a zone failure.
  - The error shape for an unsupported Moment token.
- [Ticket 34](./34-error-and-diagnostic-architecture.md) does not list this
  ticket as a blocker. It needs the temporal error set, the typed zone failure,
  the `Null`-date notice condition and any new schema date-format code.
- [Ticket 35](./35-crate-topology-and-public-surface.md) prunes `speedate` and
  `winnow` after this ticket. It does not list this ticket as a blocker.
- [Ticket 36](./36-reconcile-decisions-and-audit-coverage.md) records the
  `file.day` to `file.date` migration, and checks that the new filename prefix
  parser stays apart from the frozen vault-format parser.

## Checks before resolution

Keep resolved rules separate from new decisions. Do not replace an earlier
Answer through a new Question.

- Record the invariant owner, the alternatives, the removable code and the open
  items. Include keep-as-is.
- Exercise invalid week numbers, quarter boundaries and malformed longer
  prefixes against the filename parser contract.
- Exercise a skipped day, an ambiguous midnight and an unavailable zone against
  both ordinary field comparisons and period membership.
- Prove each range bound against the residual predicate. Leave an unproved bound
  Unsupported.
- Check the shared note parser separately. New filename forms must not change
  frozen vault-format parsing.
- Separate current measurements from proposed evidence. The natural-language
  crate comparison and parser replacement benchmarks remain incomplete.
- Give tickets 31, 34 and 35 the temporal types, error contracts and crate-list
  changes. Give ticket 36 the supersession list below.

Supersession list for ticket 36:

- Ticket 20 ends a date-check sentence at "Only date, datetime and". Get the
  intended rule before adopting a date-format check.
- Ticket 18 lists no period value. Reconcile that list with ticket 21's
  filename-derived `CalendarPeriod`.
- Ticket 25 omits `winnow` from the extractor crate list.
- Ticket 23's Question still adds date keys to `IndexPolicy`. Its later input
  removes them unless the index consumes them.
- Ticket 29 leaves the mixed temporal class order undefined. It also uses `eom`
  as a relative keyword, although the current grammar defines a function.
- The temporal survey and `reverification.md` predate the translator move.

These are recorded discrepancies, not changes to the earlier Answers.
