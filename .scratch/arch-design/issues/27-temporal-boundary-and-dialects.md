# What belongs inside temporal, and where do format dialects and calendar helpers live?

Type: grilling
Status: open
Blocked by: 21, 19, 22, 14

## Question

Temporal is 10k LoC (3k production); scouts judged `DatePoint`,
`DateValue`/`DateTimeValue`, `DurationValue` sound. Decide the boundary: whether
the Moment.js dialect translator, ordinal formatting and calendar anchors in
`template/engine/date.rs` (2.5k LoC) move into temporal (or a sibling `format`
unit shared with schema `format` validation and CLI); where `Dataview`
filename-day derivation lives (`src/file.rs:88-98`); the parsing stack
(`speedate`/`winnow`/`logos`/`jiff` for four call sites) and what remains after
the execution-context decision; whether Moment compatibility is a domain
concept.

Facts: `survey/temporal.md`, `survey/template.md` H2,
`survey/root-primitives.md` §3, `src/schema/fields/date.rs:12-14` (unvalidated
format), `.scratch/temporal-core/divergence-register.md`.

Alternatives: move dialects into temporal; keep in template as a compatibility
adapter; drop Moment tokens in favour of strftime (needs compat decision).
Include keep-as-is and say what withstood scrutiny.

Inputs: `research/07`, execution-context decision, coercion policy, compat
envelope.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Handed to this
ticket:

- **`DateConfig` sub-Config type.** Consider defining a `DateConfig` as a
  sub-Config type for temporal settings (candidates: default date and datetime
  formats, week-start and weekday resolution for relative phrases, the
  natural-language grammar options, and the source of the zone if the
  execution-context decision places it in configuration). Where it lives and how
  it is validated depends on [config ownership and
  lifecycle](./22-config-ownership-and-lifecycle.md), which is why that ticket
  now blocks this one.
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
`date.format` is a write-format hint; the note model has no time-of-day or
year-only variant, so the `date` check accepts any scalar when the format has no
date component. The schema's own tests use Moment tokens while temporal and the
template filter seam translate them. Pick the dialect; if one is chosen the
date-format check can be added.

## Inputs from the Fileclass research

See [research 14](../research/14-fileclass-model-binding-and-validation.md).
Fileclass checks dates with ISO regexes only; a field with a custom Moment
`dateFormat` accepts any non-empty string, and a wikilink is accepted as a date
(`src/fields/validate.ts:62-64,90-100`) [V]. `dateFormatCheck.ts` instead lints
the declared format string's tokens, not the values [V]. Durations are validated
against an RFC 5545 subset (weeks, or days and time; no years or months)
(`src/fields/duration.ts`) [V]. [H] Two cheap options for the dialect choice
here: check a declared format at schema load (a schema error, not a note
violation), and keep the value check separate until a dialect is fixed. IWE
gives no input: it has no date type and hands YAML dates to its validator as ISO
strings.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md).
Rename the existing kind-only `CalendarPeriod` to `CalendarUnit` (Day, Week,
Month, Quarter, Year); use `CalendarPeriod` for a normalized civil start plus
unit. Keep `DatePoint`'s written precision and arithmetic separate. A period
compared to an instant tests the instant's civil date in the pinned zone, not
the existence of the next day's midnight; a skipped local day simply has no
matching instants. Civil Date and civil DateTime points use their civil dates
without a zone. Period-vs-period filter equality is exact `(start, unit)`
identity; relational order uses that key with an explicit unit tie-break, while
mixed-temporal sorting has a distinct total key. Day/month filename prefixes
continue; add validated ISO weeks (Monday, four-day week one) and quarters, with
an opt-in bare year. Do not reinterpret invalid longer prefixes as shorter ones;
custom week starts also require a week-one rule. Scope the new prefix parser to
filenames: shared vault-format parsing remains frozen under ticket 16.
`file.date` is filename-only; `file.cdate` is creation-only and `Null` if
absent, never mtime. Civil `cdate` and `mdate` project stored timestamps in the
pinned zone during evaluation, with a typed failure if that zone is unavailable.
`file.day` is retired. Set the `DateConfig` syntax and the system-zone
acquisition/override policy with ticket 22.
