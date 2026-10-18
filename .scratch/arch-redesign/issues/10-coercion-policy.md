# Coercion policy: when does untyped text acquire a semantic type?

Type: grilling
Blocked by: 02, 09

## Question

Untyped text becomes a date, date-time, duration, or link at several independent points: `field.rs` `DateTimeCoercion::Classify` (opt-in during generic deserialization), `NoteFieldValue::from` (empty strings, wikilinks, duration strings after frontmatter parsing), `note/parser/inline.rs` (inline-field values), and the query layer's `TextShape::classify` (sniffs free text during sort-key construction and filter-literal classification). A frontmatter string that merely looks like `2026-07-29` is promoted to a date by some paths and compared as text by others depending on where it is read. Decide **where, when, and by what authority** a value acquires a semantic type: at parse time (one place), at schema-directed read time, at query time (implicit sniffing vs explicit conversion functions), or a combination — and what the observable consequences are for sort, filter, display, and round-tripping.

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify core findings](01-research-verify-core-findings.md)/[Verify system findings](02-research-verify-system-findings.md) ledgers land): [query §9.1, §11.6](../survey/query.md), [temporal §9](../survey/temporal.md), [root primitives §5](../survey/root-primitives.md). ADR-0009 (`docs/adr/0009-filter-date-literals.md`) is a claim about filter literals only.

**Verified (ledgers landed 2026-10-18)** — [01 §2](../research/01-verify-core-findings.md): classification into date/date-time happens in exactly one production conversion (`From<noyalib::Value> for FieldValueRef`, `field.rs:772`, frontmatter only); deserializer paths are `Passthrough`. [02 §3](../research/02-verify-system-findings.md): `TextShape::classify` (`query/sort.rs:462`) is called by sort-key construction (`sort.rs:394`) and by filter literal handling (`filter.rs:85`, `filter.rs:242`), i.e. the query layer re-sniffs strings independently of frontmatter coercion. `NoteFieldValue::from` parses wikilinks only; markdown links remain strings. Month/year precision is dropped at the note boundary. Whether the divergence between "typed at parse" and "sniffed at query" is observable for a given string is a question for this ticket's scenarios; it is not yet demonstrated by a failing example.

## Alternatives (not exhaustive)

- Single parse-time classification (one function, one policy) so every consumer sees the same typed value; query never re-sniffs.
- Schema-directed typing: a field's declared type (when a File Class applies) decides interpretation; unknown fields stay text or use a documented default heuristic.
- Keep implicit sniffing but centralize it behind one owner with a stated precedence.
- Explicit conversion in the query language (e.g. a `date(field)`-style function), with implicit coercion removed or limited.

## Done when

The answer states the single owner(s) of coercion, the precedence rules, behavior for untyped fields in notes with no schema, the observable compatibility changes (and whether any are acceptable), how the policy composes with the value model from [Value model](09-value-model.md) and with the typing ticket [Schema and note typing](11-schema-note-typing-and-validation.md), and what code paths disappear.
