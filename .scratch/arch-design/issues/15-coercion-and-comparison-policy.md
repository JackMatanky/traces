# Where do coercion and cross-type comparison live, and what is the policy?

Type: grilling
Status: open
Blocked by: 14, 3

## Question

Given the value model, where does string-to-date/duration promotion happen
(value construction, schema-bound pass, query evaluation), what is the
cross-kind ordering and equality policy (Date vs DateTime, Number vs String,
Duration), and how is determinism preserved (no ambient timezone in `Ord`)?

Facts to start from: coercion at `src/field.rs:843` — NOTE `research/01`
corrected the scope: default `Deserialize for FieldValueRef` is `Passthrough`;
classification runs only via `From<noyalib::Value>` (`src/field.rs:781`, the
frontmatter path) and explicit `deserialize_with(..Classify)` —
`TextShape::classify` (`src/query/sort.rs:462`), literal promotion
(`src/query/grammar/filter.rs:238`), ambient-zone Date→DateTime promotion inside
`NoteFieldValueRef::compare` (`src/note/field.rs:297-300`) which diverges from
`NoteFieldValue: PartialEq`, rank order
Null<Bool<Number<Duration<Temporal<Text<Link<List<Object
(`src/query/sort.rs:425-439`), strictness-by-seam spread across four owners
(`.scratch/temporal-core/divergence-register.md`).

Alternatives: explicit typed values with query-level cast policy;
schema-informed typing; keep heuristic but centralise it in one lattice owned by
the value model; remove implicit promotion (needs compat decision).

Inputs: `survey/query.md` H6, `survey/temporal.md` H3, `survey/note.md` H7,
`research/03`, value model decision, execution-context decision for the zone
input.

## Inputs from the value model decision

See [ticket 14's Answer](./14-value-model.md). Handed to this ticket:
frontmatter literal-string escape (frontmatter classification is
quote-insensitive, inline quotes are the escape); `Int`/`Float` share one
numeric rank, derived `PartialEq` stays structural so semantic numeric
equality lives here; `Link` equality and order, including `embedded`;
zone-shifted naive datetimes (frontmatter `12:00` was observed as `10:00Z`,
persisted in the cache); schema select strings vs note `Link` matching.
