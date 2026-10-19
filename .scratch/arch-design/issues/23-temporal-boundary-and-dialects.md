# What belongs inside temporal, and where do format dialects and calendar helpers live?

Type: grilling
Status: open
Blocked by: 17, 15

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
