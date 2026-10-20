# What belongs inside temporal, and where do format dialects and calendar helpers live?

Type: grilling
Status: open
Blocked by: 17, 15, 18

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

See [ticket 15's Answer](./15-coercion-and-comparison-policy.md). Handed to
this ticket:

- **`DateConfig` sub-Config type.** Consider defining a `DateConfig` as a
  sub-Config type for temporal settings (candidates: default date and datetime
  formats, week-start and weekday resolution for relative phrases, the
  natural-language grammar options, and the source of the zone if the
  execution-context decision places it in configuration). Where it lives and
  how it is validated depends on [config ownership and
  lifecycle](./18-config-ownership-and-lifecycle.md), which is why that ticket
  now blocks this one.
- **Natural-language dates.** A deterministic, closed English grammar
  (keywords, weekdays, offsets such as `in 2 weeks`), resolved from the injected
  `today`, shared by filter literals, `date()`, `date_add` and template filters.
  Choose the grammar and whether to build on `jiff-english` or `fuzzydate`
  (neither reviewed). Fix the meaning of a bare weekday (next vs nearest). The
  grammar must yield an interval precision for phrases such as `next week`.
- **Civil vs instant.** Naive datetimes are civil values and only input with
  `Z` or an offset is an instant; decide the type shape (a variant or an inner
  enum) and how `DatePoint` precision survives.
- **Durations.** Nominal equality and order are kept provisionally; decide
  whether to refine and how calendar arithmetic is documented.
- **`date(link)`** reads only the link target's last segment and accepts a
  pattern argument for non-ISO titles.
- The temporal glossary line saying a Date promotes to local midnight is
  superseded.
