# Where do temporal, format dialects and calendar helpers belong?

Type: grilling
Status: resolved
Blocked by: 7, 14, 16, 18, 19, 20, 21, 22, 23, 24, 25, 29, 32

## Question

The survey said temporal is 10k lines of code (3k production). It said
`template/engine/date.rs` is 2.5k lines of code. Both numbers are stale. See
"Current code" below. Scouts judged `DatePoint`, `DateValue`/`DateTimeValue` and
`DurationValue` sound. Nobody has verified that judgment.

Decide the temporal boundary. Answer each numbered question separately. Use
[ticket 36's decision-record rules](./36-reconcile-decisions-and-audit-coverage.md#decision-records-and-evidence-gates).
Keep decision evidence separate from implementation gates.

## Preparation for the grilling

Read in this order, all under
[`research/30-prep/`](../research/30-prep/):

1. [The synthesis](../research/30-prep/synthesis.md) lists, per question,
   the inherited rules, the alternatives and a proposed answer.
2. [The prior-decision ledger](../research/30-prep/b-prior-decisions.md)
   carries the accepted constraints as ids B1 to B83, P1 to P6, C1 to C19.
3. [The grilling guide](../research/30-prep/z-grilling-guide.md) orders the
   questions into clusters and lists the human questions.
4. [The red-team review](../research/30-prep/h-redteam.md) attacks the
   synthesis. The synthesis records the repairs it caused.
5. The review reports are
   [i-leakage-review](../research/30-prep/i-leakage-review.md),
   [j-test-coverage-review](../research/30-prep/j-test-coverage-review.md),
   [k-citation-verification](../research/30-prep/k-citation-verification.md)
   and
   [l-answer-verification](../research/30-prep/l-answer-verification.md).
   The synthesis cites their items as i-file, j-file, k-file and l-file.
6. The evidence files are
   [c-digest-lessons](../research/30-prep/c-digest-lessons.md),
   [d-crate-evidence](../research/30-prep/d-crate-evidence.md),
   [e-external-designs](../research/30-prep/e-external-designs.md) and
   [owner-evidence](../research/30-prep/owner-evidence.md).
7. [m-gap-closure](../research/30-prep/m-gap-closure.md) records the
   four-candidate acceptance matrix, allocations, sizes, historical
   experiments, broad verification pass and Q19 mirror layout.
8. [n-parity-and-scope](../research/30-prep/n-parity-and-scope.md)
   traces the only speedate call and separates its narrow replacement
   from the broader carrier/frame acceptance gates.

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
    weekdays and offsets such as `in 2 weeks`. Adoption, the rule-table
    semantics and the crate choice in question 14 are separate decisions.
14. Decide whether to build the grammar in the repository or use a crate.
    [d-crate-evidence](../research/30-prep/d-crate-evidence.md) holds the
    prep evidence for `jiff-english`, `fuzzydate`, `parse_datetime` and
    `chrono-english`.
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

The candidate review is complete for pre-grilling evidence.
[d-crate-evidence](../research/30-prep/d-crate-evidence.md) combines source
and runtime observations for all four candidates.
The [K report](../research/30-prep/k-citation-verification.md) checks
selected external claims from C and E. Candidate source APIs are outside
its own audit. It does not check every D citation.

The
[M acceptance closure](../research/30-prep/m-gap-closure.md#four-candidate-acceptance-closure)
executes all four candidates at Rust 1.96.0, the actual repository floor. It
records 64 shared definitions, two English lenient cases, four baselines and 36
explicit-context host variations: 298 JSON records. Each contains a native first
call and warmed call with identical outcomes. All seven matching weekday anchors
execute. Selected explicit-anchor/zone results stay invariant under `TZ`,
`LC_ALL` and `TZDIR` changes. First/warmed allocations and whole-harness sizes
are measured, not inferred. The instrumented profile uses opt-z, LTO, strip, one
codegen unit and `panic = "unwind"`; it is not the product abort profile. No new
throughput measurement exists. Earlier abort-profile costs remain historical.
[V]

English, fuzzydate and chrono-english have actual panic evidence.
English's Unicode and unchecked Span constructors panic; fuzzydate
panics on `3000000000 days ago`; chrono-english confirms UTF-8 slicing
panics. Both English parsers wrap `4294967295 years` to 2023 from the
2024 anchor. GNU has no panic in the bounded sample but accepts grammar
outside the proposed closed list. All four date APIs return points,
without native period units or error spans. [P] No unmodified candidate
meets the complete contract; prefer the in-repository grammar if adopted.
Q14 stays open. Fuzzydate's older zero-failure Unicode sample remains true,
not a general safety guarantee. [V: M; D T1 to T10]

The current managed `mise verify` passes 3,975 unit, 41 integration,
104 E2E, 155 feature and 87 doctests; two pre-existing E2E tests stay
ignored. Thirteen semantic help contracts replace brittle snapshot-prose
tests; eleven obsolete fixtures are removed without changing CLI behavior.
The corrected hk effects leave zero unknown plan effects; scoped hk,
clippy, visibility and layer gates pass. Earlier failed/cancelled broad
runs remain history, not the current status. [V: M]

[N](../research/30-prep/n-parity-and-scope.md) confirms that only the
unsigned full-date field branch calls speedate. Datetime, duration and zone
comparisons are not missing speedate-removal experiments. Preserve guarded
4,635,370-input parity without claiming whole-parser replacement parity.
The production redesign and human decisions remain open.

Facts from the crates.io API. The first read was 2026-11-17; the download
count was re-read on 2026-11-18 [V]:

- `fuzzydate` 0.4.1 has the MIT licence, about 2.6k code lines and 14 releases.
  It depends on `chrono` (feature `clock`), `lazy_static` and `thiserror`.
  `Cargo.toml` has no `chrono` entry.
- `jiff-english` 0.0.1 has the MIT licence, about 1.3k code lines and one
  release, dated 2026-08-07. The 2026-11-17 snapshot recorded 29 downloads;
  a 2026-11-18 read records 30. It depends on `jiff` `^0.2`
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

The prep evidence now supplies these items for each candidate. Remaining
production measurements depend on the selected implementation:

- Licence and release history.
- Size, dependencies, historical abort-profile linkage costs and current
  instrumented harness sizes. Neither establishes complete shipped cost.
- Determinism: an injected anchor, no ambient clock and no ambient zone.
- Week rules: the week start and the week-one rule.
- Locale: English only, and the order of month and day.
- Output: a period with a unit, not one point.
- Error reporting: a span that a query error can carry.
- Coverage of the closed grammar that ticket 19 proposes.
- Feature and version coupling with `jiff` `=0.2.38`.
- The size and test cost of a grammar built in the repository.

[P] The prep evidence is in place. Treat a grammar built in the repository as
the default option until the evidence beats it. The build-versus-crate
decision stays open.

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
  the grammar and whether to build on `jiff-english` or `fuzzydate`
  (unreviewed at that handoff; now source- and runtime-reviewed here).
  Fix the meaning of a bare weekday (next vs nearest). The grammar
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
  percent-encoded. External URLs are not note targets. Component separation
  precedes decoding. Take the last raw segment before any percent-decode.
  An encoded separator such as `%2F` never creates a path boundary
  (RFC 3986 section 2.4).

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
date-format check. The lint-only proposal in synthesis Q4 removes the
provisional value check. It needs an explicit amendment to ticket 20;
it is not inherited behavior.

The Answer text of ticket 20 is cut off after "Only date, datetime and". Do not
read a rule into that fragment.

## Inputs from the Fileclass research

See [research 14](../research/14-fileclass-model-binding-and-validation.md).
Fileclass checks dates with ISO regexes only. `validateDatePattern` bypasses
pattern validation when `dateFormat` is a string. It does not itself
enforce a non-empty string. The outer validator handles empty values and
the `required` option (`src/fields/validate.ts`) [V]. `dateFormatCheck.ts`
instead lints the tokens of the declared format string, not values [V].
Fileclass's duration input grammar resembles an RFC 5545 subset (weeks, or
days and time; no years or months). The inspected parser is not an exact
RFC validator. Its regex permits mixed weeks, days and time, and it
ignores a captured sign. Its arithmetic treats a day as 86,400,000
milliseconds in UTC (`src/fields/duration.ts`) [V: k-citation-verification K8].

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
rule. Filename week prefixes stay ISO regardless of a configurable general
week rule. Ticket 21 fixes them to Monday start and the ISO week-one rule
(21:170-179).

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
- Ticket 29 sorts distinct group keys by "temporal class order". Ticket 29
  defines that order as Civil, Instant, Period (29:721-730). The sort key
  decision above decides the value-sort key and how it relates to those
  group classes.
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
- Keep measured four-candidate behavior, allocation, size and floor builds
  separate from production acceptance. The carrier, frame, adopted grammar,
  integrated costs, format epoch and exact-bound proofs need implementation.
- Treat inaccessible English upstream history and arbitrary-input safety
  as evidence limits, not missing runnable pre-grilling checks. The package
  records VCS SHA `a8b87edbeebafebd06a6c4f5a2b502b79b61e409`; its advertised
  repository still returns 404 and public searches find no alternative.
  The 2026-11-17 snapshot recorded 29 downloads; a 2026-11-18 read records 30.
  No bounded sample proves all Unicode, overflow or ambient-state behavior.
  All four build at the repository floor; no candidate MSRV test gap remains.
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
- Ticket 29 defines the group-class order (Civil, Instant, Period) but
  leaves the mixed temporal value-sort key open. Ticket 36 receives the Q22
  outcome. It also uses `eom` as a relative keyword, although the current
  grammar defines a function.
- The temporal survey and `reverification.md` predate the translator move.

These are recorded discrepancies, not changes to the earlier Answers.

## Answer

*Language note: This section uses ASD-STE100 Simplified Technical English (STE)
for rules, contracts, and gates.*

### Summary of the architecture

The temporal architecture splits responsibilities across four boundaries:

- The `temporal` module owns all temporal value types, calendar arithmetic, week
  rules, zone resolution, and date parsing.
- The `config` module owns the flat `[date]` TOML table and parses it into
  `DateConfig`.
- The `temporal` module receives `TemporalPolicy`, which holds validated
  spellings and never imports `config`.
- Each query, render, or request pins one `TemporalFrame` that holds a clock
  sample, a `TemporalPolicy`, and a thread-safe `OnceLock` zone cell.
- All five date-precision levels (`Year`, `Quarter`, `Month`, `Week`, `Day`)
  live on `DatePoint::Calendar` and travel through `NoteFieldValue::Date`.
- `CalendarPeriod` is a span helper that code derives from `DatePoint` when
  membership or boundary math needs a half-open interval.

---

### Question dispositions (Q1 to Q36)

#### Moment compatibility and format dialects (Q1 to Q5)

1. **Q1 — Moment compatibility as a domain concept**: **Decided** (Choice: not a
   domain concept; compatibility adapter only).
   - **Reason**: Ticket 16 makes Templater parity free. Domain values hold no
     dialect tokens.
   - **Evidence**: `research/07-template-bridge-precedent.md`;
     `src/temporal/token.rs`.
   - **Invariant owner**: `template` (compatibility) and `temporal`
     (dialect-free values).

2. **Q2 — Location of the Moment dialect translator**: **Decided** (Choice: move
   the Moment render-only translator to `src/template/`).
   - **Reason**: `pattern_to_strftime` is a render-only helper, and all callers
     sit in `src/template/engine/date.rs`. This move keeps `temporal`
     dialect-free.
   - **Evidence**: `src/temporal/token.rs:15-61,186-189`; CodeGraph caller
     check.
   - **Invariant owner**: `template`.
   - **Downstream**: Ticket 31 receives the translator move and the
     `UnsupportedToken` error shape.
   - **Supersession**: This decision supersedes the landed move in commit
     `6104d0ae`.

3. **Q3 — Dialect per format surface**: **Decided** (Choice: `strftime` is
   native for query and schema; Moment is a render-only adapter in `template`;
   `[date]` periodic formats use `%`-sniff).
   - **Reason**: Query `date(text, pattern)` already uses `strftime`, and schema
     `date.format` shares one native dialect with query.
   - **Rule**: `[date]` periodic formats (`YYYY`, `YYYY-Qq`, `YYYY-MM`,
     `YYYY-Www`, `YYYY-MM-DD`) use `%`-sniff in a dedicated filename-pattern
     compiler in `temporal`. The compiler selects `strftime` when `%` is
     present, and it selects the Moment subset otherwise.
   - **Evidence**: `src/temporal/token.rs:55-61`;
     `docs/vault/05_metadata/cal.md`.
   - **Invariant owner**: `temporal` (`strftime` and filename-pattern compiler)
     and `template` (Moment render adapter).

4. **Q4 — `date.format` check for formats without a full date**: **Decided**
   (Choice: check format syntax at schema load, and check frontmatter values by
   format class).
   - **Reason**: Schema load checks only that `date.format` is a valid format
     string. At note validation, full-date formats accept `Date` or `DateTime`.
     Partial formats (`%Y`, `%j`, quarter) accept `Int`, a matching `String`, or
     `Date` because YAML parses unquoted `2026` as `Int`.
   - **Evidence**: `src/schema/fields/date.rs`; `docs/vault/05_metadata/cal.md`;
     `research/14`.
   - **Invariant owner**: `schema` (load lint and value check).
   - **Downstream**: Ticket 34 receives the schema format lint diagnostic code.
     Ticket 36 records that this rule completes Ticket 20's truncated date-check
     sentence.

5. **Q5 — `format` under `[frontmatter.date_created]` and
   `[frontmatter.date_modified]`**: **Decided** (Choice: keep both frontmatter
   tables reserved; place default date and datetime formats under `[date]`).
   - **Reason**: Both reserved tables hold config slots for future
     `file.date_created` and `file.date_modified` functions. Putting general
     default date and datetime formats in `[date]` gives one home for vault date
     settings.
   - **Evidence**: `src/config/raw.rs`; Ticket 22.
   - **Invariant owner**: `config`.
   - **Rule**: Exclude both reserved frontmatter tables from `IndexPolicy` until
     an index reader consumes them.

#### Calendar helpers and filenames (Q6 to Q8)

6. **Q6 — Calendar helpers in `src/template/engine/date.rs`**: **Decided**
   (Choice: keep minimal filter adapters in `engine/date.rs`; move `Anchor`
   resolution, offset arithmetic, and week-rule logic to `temporal` behind
   `TemporalFrame`).
   - **Reason**: Minimal argument parsing and filter registration in `template`
     removes duplicate calendar logic and ambient clock reads.
   - **Evidence**: `src/template/engine/date.rs:183-294`;
     `src/temporal/calendar.rs:35-64`.
   - **Invariant owner**: `temporal` (arithmetic and week rules) and `template`
     (filter registration).
   - **Downstream**: Ticket 31 receives the thin-adapter boundary.

7. **Q7 — Location of the filename-period parser**: **Decided** (Choice: a
   general string date/period extractor in `src/temporal/` with its own token
   enum, separate from the frozen note-content lexer).
   - **Reason**: `FileMeta::derive_day` in `src/file.rs` already calls
     `temporal`. Placing the new extractor in `src/temporal/` with its own token
     enum keeps one date grammar and protects frozen note parsing (Ticket 16).
   - **Evidence**: `src/temporal/lexer.rs:17-54`; `src/file.rs`; Ticket 16.
   - **Invariant owner**: `temporal`.
   - **Contract**: Use longest-match-first with boundary checks. Return `Null`
     plus one notice for a malformed longer prefix (for example `2026-13-40`).
     Never fall back to a shorter prefix (`2026`).

8. **Q8 — Week-one rule for a custom week start**: **Decided** (Choice: pair
   `[date].week_start` with optional `[date].week_min_days` in `1..=7`; default
   `4` for Monday and `1` for Sunday; no rollover from invalid weeks).
   - **Reason**: `week_start` alone does not define week 1 (`2009-01-01` shifts
     between `2009-W01` and `2008-W53` across `min_days` 4 and 5). Jiff lacks
     custom week-year math, so `temporal` implements this arithmetic. Filename
     `Www` labels stay ISO-fixed (Monday, 4 days) under Ticket 21.
   - **Evidence**: `e-external-designs.md` q8 (`java.time.WeekFields`, CLDR);
     `src/temporal/calendar.rs:41-48`.
   - **Invariant owner**: `temporal`.

#### Parsing stack (Q9 to Q12)

9. **Q9 — `speedate`**: **Deferred / Prototype-gated** (Choice: keep `speedate`
   now; measure removal cost versus gain in a prototype before removal).
   - **Reason**: `speedate` has one call site (`src/temporal/date.rs:369-386`)
     and matches guarded `jiff::civil::Date::new` across 4,635,370 generated
     inputs.
   - **Rule**: Keep `speedate` in the codebase and in Ticket 25's fingerprint
     crate list until a prototype benchmark measures binary size and throughput.
   - **Evidence**: `m-gap-closure.md`; `n-parity-and-scope.md`;
     `src/temporal/date.rs:369-396`.
   - **Downstream**: Ticket 35 receives this prototype gate before it prunes
     `speedate`, `lexical-parse-float`, and `strum`.

10. **Q10 — `winnow`**: **Decided** (Choice: keep `winnow`).
    - **Reason**: `duration_part` in `src/temporal/lexer.rs:139-148` uses
      `winnow` to report the consumed prefix length, which Jiff's friendly
      parser cannot report. `winnow` already stays in `Cargo.lock` through
      `toml`.
    - **Evidence**: `src/temporal/lexer.rs:139-148`; `d-crate-evidence.md`.
    - **Downstream**: Ticket 25 must add `winnow` to the extractor fingerprint
      crate list.

11. **Q11 — `logos`**: **Inherited** (Choice: keep `logos`).
    - **Reason**: `logos` drives four lexers across `temporal`, `note`, and
      `query` with no reported defect.
    - **Evidence**: `src/temporal/lexer.rs`; `src/note/parser/lexer.rs`;
      `src/query/grammar/filter.rs`.

12. **Q12 — `jiff`**: **Ratified** (Choice: keep `jiff = "=0.2.38"` with `std`,
    `tz-system`, `tzdb-bundle-always`).
    - **Reason**: `jiff` is the core temporal engine for civil types, instants,
      `tzdb`, and `strtime`.
    - **Evidence**: `Cargo.toml:128`.

#### Natural-language dates and keywords (Q13 to Q18)

13. **Q13 — Natural-language date grammar**: **Decided** (Choice: closed,
    deterministic English grammar resolved against the frame's `today`).
    - **Reason**: Query plans need deterministic parsing for keywords
      (`today()`, `now()`, `tomorrow`, `yesterday`), weekdays, offsets, and
      period phrases. Offsets use `in <n> <unit>` and `<n> <unit> ago`. Period
      phrases use `this/next/last week/month/quarter/year` and
      `start of/end of <period>`.
    - **Evidence**: Ticket 19; `synthesis.md` Q13 table.
    - **Invariant owner**: `temporal`.

14. **Q14 — Build in-repo or use a crate**: **Decided** (Choice: build the
    closed grammar in the repository).
    - **Reason**: Four-candidate evaluation at Rust 1.96.0 found panics in
      `jiff-english`, `fuzzydate`, and `chrono-english`. The fourth candidate,
      `parse_datetime`, has an overbroad grammar and point-only output with no
      error spans.
    - **Evidence**: `m-gap-closure.md` (298 JSON records across 64 shared cases
      and 36 host variations); `d-crate-evidence.md`.
    - **Invariant owner**: `temporal`.

15. **Q15 — Bare weekday and phrase time policy**: **Decided** (Choice:
    forward-only bare weekdays; midnight `00:00` for phrase datetimes).
    - **Rules**:
      - A bare weekday means the next occurrence strictly after `today`.
      - `this <day>` means the day in the current week.
      - `next <day>` means the next occurrence after `today`.
      - `last <day>` means the previous occurrence before `today`.
      - Phrase datetimes use midnight `00:00`.
    - **Reason**: Forward-only bare weekdays keep `due > friday` and
      `due == friday` predictable when `today` is Friday.
    - **Evidence**: `synthesis.md` Q15; `m-gap-closure.md` same-day weekday
      probe.
    - **Invariant owner**: `temporal`.

16. **Q16 — Interval precision of `next week` and offset phrases**: **Decided**
    (Choice: all natural-language date functions and phrases return `DatePoint`
    with 5-level `DatePrecision` (`Year | Quarter | Month | Week | Day`)).
    - **Rules**:
      - `next week` returns
        `DatePoint::Calendar { date: <week_start>, date_precision: Week }`.
      - Offset phrases (`in 1 month`, `3 months ago`) return
        `DatePoint::Calendar` at `Day` precision, clamped under accepted
        calendar arithmetic.
      - `==` checks exact `(date, precision)` equality.
      - `in`, `not in`, and `member_of` derive the `CalendarPeriod` span
        `[start, end)` on demand to check membership.
      - `<`, `<=`, `>`, and `>=` across different precisions compare span bounds
        (`d < p` means `d < p.start`, and `d <= p` means `d < p.end`).
    - **Evidence**: `src/temporal/point.rs:28-53`.
    - **Invariant owner**: `temporal` (types and span math) and `query`
      (operator semantics).
    - **Supersession**: This decision unifies `DatePoint::DatePrecision` and
      `CalendarPeriod`, revising Ticket 21's separation.

17. **Q17 — Filter spelling of relative keywords**: **Decided** (Choice: nullary
    `today()` and `now()` only; compose period edges as `eom(today())` and
    `sow(today())`).
    - **Reason**: Bare `today` parses as a field path. Adding only `today()` and
      `now()` avoids giving every edge function a second arity.
    - **Evidence**: `src/query/grammar/filter.rs:591-597`.
    - **Invariant owner**: `query`.
    - **Supersession**: Ticket 29 prose using bare `eom` as a keyword yields to
      function `eom(today())`.

18. **Q18 — Frontend spelling of `member_of`**: **Decided** (Choice: support
    both `in` / `not in` infix overload and named `member_of(point, period)`).
    - **Reason**: The infix form matches existing `in` list syntax for
      readability. The named function `member_of(point, period)` gives an
      explicit form for unambiguous queries.
    - **Evidence**: Ticket 29 section 10.
    - **Invariant owner**: `query`.

#### Value shapes (Q19 to Q25)

19. **Q19 — Civil and instant type shape**: **Decided** (Choice: inner
    `Civil | Instant` enum inside `DateTimeValue`, keeping `NoteFieldValue` <=
    80 bytes; remove cross-class `Ord`; bump format epoch).
    - **Reason**: This shape keeps `NoteFieldValue`'s variant count and 80-byte
      slot budget (24-byte temporal payload versus 72-byte `DurationValue` or
      `Object`). Naive datetimes stay civil without a zone lookup at parse or
      cache load.
    - **Evidence**: `m-gap-closure.md` (`probes/parity/tests/layout.rs` 80-byte
      mirror); `src/note/field.rs:579-580`.
    - **Invariant owner**: `temporal` (`DateTimeValue`) and `note`
      (`NoteFieldValue` size assertion).
    - **Downstream**: Ticket 25 bumps the format epoch and adds civil datetime
      to the fingerprint corpus. Ticket 36 records the frozen-parse semantic
      change for naive datetimes (authorized by B16).

20. **Q20 — Placement of `CalendarPeriod` in the value model**: **Decided**
    (Choice: expand `DatePoint::DatePrecision` to
    `Year | Quarter | Month | Week | Day` on `NoteFieldValue::Date`; derive
    `CalendarPeriod` as a span helper).
    - **Reason**: `file.date` and all temporal expressions stay inside
      `NoteFieldValue` with no `QueryValue` wrapper and no
      `NoteFieldValue::Period` variant. `DatePoint::Calendar` normalizes `date`
      to the unit start on construction.
    - **Evidence**: `src/temporal/point.rs:28-53`; `src/note/field.rs:579-580`.
    - **Invariant owner**: `temporal` and `note`.
    - **Supersession**: This decision revises Ticket 18 and Ticket 21 by storing
      5-level `DatePrecision` on `NoteFieldValue::Date` and treating
      `CalendarPeriod` as a derived span helper.

21. **Q21 — Date versus instant at an ambiguous or skipped local midnight**:
    **Decided** (Choice: Option D — project the instant to a civil value in the
    pinned zone, then compare civil values).
    - **Reason**: Instant-to-civil projection is total once the zone is
      available and stays monotone across DST gaps. Civil-to-instant mapping
      across `America/New_York` 2026-03-08 02:30 versus 03:00 breaks
      transitivity. Zone failure stays a typed error (B30).
    - **Evidence**: `d-crate-evidence.md` (`Compatible` row);
      `m-gap-closure.md`; `src/note/field.rs:297-300`.
    - **Invariant owner**: `temporal` (projection) and `query` (error
      propagation).

22. **Q22 — Total sort key for mixed temporal values**: **Decided** (Choice:
    `Civil < Instant < Period`, with `Week < Month < Quarter < Year` tie-break
    inside `Period` for equal start dates).
    - **Reason**: `Civil` covers `Date` (`Day` precision) and civil `DateTime`,
      while `Period` covers `Week`, `Month`, `Quarter`, and `Year` precisions.
      This order reuses Ticket 29's group-class order (B65), aligns with
      PostgreSQL `daterange` `(lower, upper)` ordering, and needs no zone
      lookup.
    - **Evidence**: Ticket 29 (B65); PostgreSQL `daterange` convention.
    - **Invariant owner**: `temporal` (sort key) and `query` (`ORDER BY` /
      `GROUP BY`).

23. **Q23 — Nominal duration equality and order**: **Ratified** (Choice: keep
    nominal rules: 1 month = 30 days, 1 year = 365 days).
    - **Reason**: This rule preserves existing behavior in
      `src/temporal/duration.rs:11-13` and `unit.rs:210` with no observed user
      defect.
    - **Evidence**: `src/temporal/duration.rs:11-13`;
      `src/temporal/unit.rs:210`.

24. **Q24 — Calendar arithmetic documentation**: **Decided** (Choice: update
    `src/temporal/GLOSSARY.md` at cutover with three lists: exact rules, rules
    with a caveat, and undefined cases, tied to Q23).
    - **Reason**: Current-code glossaries stay accurate until implementation
      lands (Ticket 19 / Ticket 36 rule).
    - **Invariant owner**: `temporal`.

25. **Q25 — `date(link)` on Markdown targets**: **Decided** (Choice: take the
    last raw `/` segment, remove `#anchor` and the extension, percent-decode
    once, then parse the prefix or pattern; external URLs yield `Null`).
    - **Reason**: Component separation before decoding (RFC 3986 section 2.4)
      makes sure `%2F` never creates a path boundary and double-decoding never
      occurs.
    - **Evidence**: Ticket 19; `k-citation-verification.md` K22.
    - **Invariant owner**: `query` and `temporal`.

#### Configuration and zone (Q26 to Q32)

26. **Q26 — `[date]` table syntax**: **Decided** (Choice: one flat `[date]`
    table with `deny_unknown_fields`, using global-then-local field merge after
    the trust gate).
    - **Reason**: One flat table avoids nested TOML subtables and obeys Ticket
      22's merge and trust rules.
    - **Invariant owner**: `config`.

27. **Q27 — Fields of `DateConfig`**: **Decided** (Choice: `zone`, `week_start`,
    `week_min_days`, `filename_date`, periodic format strings `year`, `quarter`,
    `month`, `week`, `day`, plus default `date` and `datetime` formats).
    - **Reason**: One flat struct holds all temporal configuration consumed by
      `TemporalPolicy` and `TemporalFrame`.
    - **Invariant owner**: `config`.

28. **Q28 — Temporal-owned policy**: **Decided** (Choice: new
    `temporal::TemporalPolicy` receiving validated spellings and settings from
    `config::DateConfig`).
    - **Reason**: `TemporalPolicy` keeps `temporal` free of `config` imports and
      carries zone spellings rather than a loaded `TimeZone` so acquisition
      stays lazy.
    - **Invariant owner**: `temporal`.

29. **Q29 — Configured zone syntax and validation**: **Decided** (Choice: accept
    an IANA zone name or explicit `±HH:MM` offset; check spelling against
    bundled `tzdb` at config load without acquiring a zone).
    - **Reason**: `±HH:MM` avoids the `Etc/GMT-2` reversed-sign trap. A
      load-time spelling check catches typos early without triggering zone
      acquisition for civil-only commands.
    - **Evidence**: `src/note/field.rs:1041`; `e-external-designs.md` q29.
    - **Invariant owner**: `config` (load check) and `temporal` (syntax
      validator).

30. **Q30 — System zone acquisition**: **Inherited** (Choice: lazy per-operation
    acquisition in `TemporalFrame`, memoized for success or typed failure; UTC
    is never an implicit fallback).
    - **Reason**: Ticket 21 (B28) and Ticket 22 (host override, then config,
      then system) fix this rule.
    - **Evidence**: `owner-evidence.md` (jiff 5-minute cache).

31. **Q31 — Configurable week start**: **Decided** (Choice: configurable
    `[date].week_start` and `[date].week_min_days` with `iso` and `us` presets,
    merging global-then-local).
    - **Reason**: Presets and explicit fields support Sunday-first or custom
      calendar weeks for `sow()`/`eow()` and templates while filename `Www`
      labels stay ISO-fixed.
    - **Invariant owner**: `config` and `temporal`.

32. **Q32 — Opt-in key for filename date extraction**: **Decided** (Choice:
    replace `filename_year_prefix` bool with
    `filename_date = "off" | "basename" | "prefix"` across all `[date]` periodic
    formats).
    - **Reason**: The three-state enum avoids prefix-versus-basename naming
      contradictions and protects non-date stems like `2019 budgets.md` when set
      to `"basename"`.
    - **Invariant owner**: `config` and `temporal`.

#### Boundary after the execution-context decision (Q33 to Q34)

33. **Q33 — Temporal exports to the execution frame**: **Decided** (Choice:
    `TemporalFrame` exports `clock_sample`, `TemporalPolicy`, and
    `OnceLock<Result<TimeZone, ZoneError>>`).
    - **Reason**: `OnceLock` makes the frame `Sync` for LSP requests and lazy
      `QuerySet` execution while keeping zone acquisition lazy.
    - **Invariant owner**: `temporal` (frame type) and `query`/`template`
      (callers).

34. **Q34 — Replacements for ambient reads**: **Inherited** (Choice: pass
    `&TemporalFrame` to all filter arithmetic, template helpers, `Anchor::base`,
    `is_past`/`is_future`, and civil/instant conversions; remove `local_zone()`,
    `now_instant()`, and `ZoneGuard`).
    - **Reason**: Ticket 21 (B38) fixes this rule.

#### Query pushdown and notices (Q35 to Q36)

35. **Q35 — Temporal probe bounds per operator and value kind**: **Decided**
    (Choice: prove `Exact` bounds in first scope for zone-free civil and period
    kinds only; keep zone-dependent and instant mappings `Unsupported` to scan).
    - **Reason**: This boundary matches PostgreSQL's `IMMUTABLE` versus `STABLE`
      sargability rule. Timezone-dependent projections cannot drive index bounds
      without explicit pinned-zone bound proofs.
    - **Evidence**: Ticket 29 R1–R7; PostgreSQL `date` versus `timestamptz`
      sargability convention.
    - **Invariant owner**: `query` and `index`.

36. **Q36 — Notices for `Null` dates**: **Decided** (Choice: emit no notice for
    an absent field; emit one notice for a malformed date-shaped value; return a
    typed error for a zone failure).
    - **Reason**: This rule matches SQL and Dataview conventions so queries do
      not flood users with notices when notes omit optional date fields.
    - **Invariant owner**: `query` and `temporal` (conditions) and Ticket 34
      (notice delivery and deduplication).

---

### Removable or replaceable code

- `pattern_to_strftime` in `src/temporal/token.rs`: move to `src/template/`.
- `local_zone()`, `now_instant()`, and `ZoneGuard`: remove after
  `&TemporalFrame` migration.
- Cross-class `Ord` on `DateTimeValue`: remove in favor of Option D projection
  and mixed sort key.
- `filename_year_prefix` config key: replace with
  `filename_date = "off" | "basename" | "prefix"`.
- `speedate` (plus `lexical-parse-float` and `strum` if unused): keep until the
  prototype benchmark in Gate 4 decides removal.

---

### Downstream contracts

| Receiving ticket | Contract delivered by this decision |
| --- | --- |
| **31 — Template** | Moment render-only adapter moves to `src/template/`; `UnsupportedToken` error shape; thin filter adapters in `engine/date.rs`; `TemporalFrame` and typed `ZoneError`; `today()`/`now()` and closed NL grammar; `DatePoint` (5-level `DatePrecision`) and `DateTimeValue::{Civil, Instant}` relation/sort semantics. |
| **34 — Diagnostics** | Typed `ZoneError` (never coerced to `Null`); notice conditions (absent = none, malformed date-shaped = 1 notice); schema `date.format` load-lint diagnostic code; NL error spans. |
| **35 — Topology** | `temporal` -> `config` edge stays forbidden (`TemporalPolicy` carries spellings); `winnow`, `logos`, `jiff` stay; `speedate` stays pending prototype benchmark. |
| **36 — Reconciliation** | Unification of `DatePrecision` (5 levels) and derived `CalendarPeriod`; naive-datetime civil change (epoch bump); `file.day` -> `file.date` migration; `filename_date` enum; `eom(today())` over bare `eom`; `winnow` added to Ticket 25 crate list; Ticket 20 truncated date-check sentence completed by Q4. |

---

### Implementation gates (unrun; needed at cutover)

1. Implement `DateTimeValue::{Civil, Instant}` and 5-level
   `DatePoint::DatePrecision`.
2. Add a compile-time `size_of::<NoteFieldValue>() <= 80` assertion and a
   compile-fail `Ord` check.
3. Bump the index format epoch and update the fingerprint corpus.
4. Implement `TemporalFrame` with `OnceLock<Result<TimeZone, ZoneError>>`.
5. Replace all `local_zone()`, `now_instant()`, and `ZoneGuard` calls from a
   fresh call-site scan.
6. Implement the closed NL grammar, custom `(week_start, week_min_days)`
   arithmetic, and the filename-pattern compiler.
7. Add tests for malformed longer prefixes (`2026-13-40`) and frozen note-parser
   isolation.
8. Measure `speedate` removal (`cargo bench` and binary size) before pruning
   `speedate`, `lexical-parse-float`, and `strum`.
9. Prove residual equivalence for each `(operator, value_kind)` before marking a
   zone-free temporal pushdown mapping as `Exact`.
10. Update `src/temporal/GLOSSARY.md` at cutover with exact, caveat, and
    undefined arithmetic lists.
