# What belongs inside temporal, and where do format dialects and calendar helpers live?

Type: grilling
Status: open
Blocked by: 14, 19, 21, 22

## Question

Temporal is 10k LoC (3k production). Scouts judged `DatePoint`,
`DateValue`/`DateTimeValue`, `DurationValue` sound. Decide the boundary:

- Decide whether the Moment.js dialect translator, ordinal formatting and
  calendar anchors in `template/engine/date.rs` (2.5k LoC) move into temporal.
  Keep them in a sibling `format` unit instead. The unit is shared with schema
  `format` validation and CLI.
- Decide where `Dataview` filename-day derivation lives (`src/file.rs:88-98`).
- Decide the parsing stack: `speedate`/`winnow`/`logos`/`jiff` for four call
  sites.
- Decide what remains after the execution-context decision.
- Decide whether Moment compatibility is a domain concept.

Facts: `survey/temporal.md`, `survey/template.md` H2,
`survey/root-primitives.md` §3, `src/schema/fields/date.rs:12-14` (unvalidated
format), `.scratch/temporal-core/divergence-register.md`.

Alternatives: move dialects into temporal. Keep them in the template as a
compatibility adapter. Drop Moment tokens in favour of strftime (needs compat
decision). Include keep-as-is and say what withstood scrutiny.

Inputs: `research/07`, execution-context decision, coercion policy, compat
envelope.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Handed to this
ticket:

- **`DateConfig` sub-Config type.** Consider defining a `DateConfig` type for
  temporal settings. Candidate fields include default date and datetime
  formats, week-start rules and weekday resolution for relative phrases.
  Further fields include natural-language grammar options and zone source if
  placed in configuration. Where it lives and how it validates depends on
  [config ownership and lifecycle](./22-config-ownership-and-lifecycle.md). That
  ticket blocks this one.
- **Natural-language dates.** A deterministic, closed English grammar (keywords,
  weekdays, offsets such as `in 2 weeks`), resolved from the injected `today`,
  shared by filter literals, `date()`, `date_add` and template filters. Choose
  the grammar and whether to build on `jiff-english` or `fuzzydate` (neither
  reviewed). Fix the meaning of a bare weekday (next vs nearest). The grammar
  must yield an interval precision for phrases such as `next week`.
- **Civil vs instant.** Naive datetimes are civil values and only input with `Z`
  or an offset is an instant; decide the type shape (a variant or an inner enum)
  and how `DatePoint` precision survives.
- **Durations.** Nominal equality and order are kept provisionally; decide
  whether to refine and how calendar arithmetic is documented.
- **`date(link)`** reads only the link target's last segment and accepts a
  pattern argument for non-ISO titles.
- The temporal glossary line saying a Date promotes to local midnight is
  superseded.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). Schema
`date.format` is a write-format hint. The note model has no time-of-day or
year-only variant. The `date` check accepts any scalar when the format has no
date component. Tests in `schema` use Moment tokens. Temporal and the template
filter seam translate them. Pick the dialect. After choosing a dialect, add the
date-format check.

## Inputs from the Fileclass research

See [research 14](../research/14-fileclass-model-binding-and-validation.md).
Fileclass checks dates with ISO regexes only. A field with a custom Moment
`dateFormat` accepts any non-empty string. It also accepts a wikilink as a date
(`src/fields/validate.ts:62-64,90-100`) [V]. `dateFormatCheck.ts` instead lints
the tokens of the declared format string, not values [V]. Fileclass validates
durations against an RFC 5545 subset (weeks, or days and time; no years or
months) (`src/fields/duration.ts`) [V].

[H] Two cheap options exist for the dialect choice. Check a declared format at
schema load as a schema error, not a note violation. Keep the value check
separate until a dialect is fixed. IWE gives no input: it has no date type and
hands YAML dates to its validator as ISO strings.

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

Day/month filename prefixes continue. Add validated ISO weeks (Monday,
four-day week one) and quarters, with an opt-in bare year. Do not reinterpret
invalid longer prefixes as shorter prefixes. Custom week starts also need a
week-one rule.

Scope the new prefix parser to filenames: shared vault-format parsing remains
frozen under ticket 16. `file.date` is filename-only. `file.cdate` is
creation-only and `Null` if absent, never mtime. Civil `cdate` and `mdate`
project stored timestamps in the pinned zone during evaluation, with a typed
failure if that zone is unavailable. `file.day` is retired. Set the `DateConfig`
syntax and the system-zone acquisition/override policy with ticket 22.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- The extractor fingerprint hashes the resolved versions of `jiff` and `speedate`. A change to the parsing stack changes the fingerprint and forces one rebuild. Update the crate list in ticket 25 if this ticket removes a crate.
- A civil datetime must deserialize from the cache without a zone lookup.
- Until the civil variant exists in code, the fingerprint corpus uses only datetimes with an explicit offset. Add the civil variant to the corpus when it lands. That change bumps the format epoch.
