# Where do coercion and cross-type comparison live, and what is the policy?

Type: grilling
Status: resolved
Blocked by: 18, 3

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
`Null<Bool<Number<Duration<Temporal<Text<Link<List<Object`
(`src/query/sort.rs:425-439`), strictness-by-seam spread across four owners
(`.scratch/temporal-core/divergence-register.md`).

Alternatives: explicit typed values with query-level cast policy;
schema-informed typing; keep heuristic but centralise it in one lattice owned by
the value model; remove implicit promotion (needs compat decision).

Inputs: `survey/query.md` H6, `survey/temporal.md` H3, `survey/note.md` H7,
`research/03`, value model decision, execution-context decision for the zone
input.

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). Handed to this ticket:
frontmatter literal-string escape (frontmatter classification is
quote-insensitive, inline quotes are the escape); `Int`/`Float` share one
numeric rank, derived `PartialEq` stays structural so semantic numeric equality
lives here; `Link` equality and order, including `embedded`; zone-shifted naive
datetimes (frontmatter `12:00` was observed as `10:00Z`, persisted in the
cache); schema select strings vs note `Link` matching.

## Pre-grilling research

Brief: [research/19-prep/brief.md](../research/19-prep/brief.md) (options,
leans, corrections, proposed grilling order). Scout inputs: [code inventory and
probe matrix](../research/19-prep/a-code-inventory.md), [digest
lessons](../research/19-prep/b-digest-lessons.md), [external
precedent](../research/19-prep/c-external-precedent.md), [prior
decisions](../research/19-prep/d-prior-decisions.md).

## Answer

**Decision:** one value-semantics module owns all comparisons. It provides
three separate relations, not one overloaded `compare`. Classify note values
once at parse time. Do not sniff them again. Only query string literals can
receive late typing, based on the other operand. No comparison reads an
ambient zone or clock.

**Alternatives considered:** keep as is (three equality evaluators, rank-leaking
relational filters, three promotion seams); lexical storage with typing on
demand (rejected: zero-config date queries would need a schema or casts);
quote-sensitive frontmatter (rejected: no public Noyalib route, CST costs +59%
per note and 2.6x for the lazy variant on notes with dates, and Templater,
Metadata Menu and formatters write quoted dates, so notes would silently drop
behaviour); erroring on ordinary data mismatches (hostile to open-world vaults;
environmental failures are different, see ticket 21); a custom `PartialEq`
equating `Int` and `Float` (breaks `Eq`/`Hash` coherence); comparison by
resolved note identity (needs the index, impure).

**Relations (owner: the value-semantics module):**

- `NoteFieldValue: PartialEq` stays structural. It has no `Ord` or `Hash`.
- Loose equality and bracketed order serve filters. The brackets are Numeric
  (`Int`, `Float`), Temporal (`Date`, `DateTime`, `CalendarPeriod`), Duration,
  String, Bool, Link, List, Object and Null.
- Across brackets, `==` is false and `!=` is true. `<`, `<=`, `>` and `>=` do
  not match. `Null` never matches a relational operator. `null == null` is
  true, and a missing field equals `null`.
- `!=` is the plain complement. Thus `not (x < 5)` also matches other kinds
  and nulls, as in MongoDB. Document this behavior; do not rewrite it.
- Total sort order serves `SortKey` and uses an explicit context. Keep the
  implemented rank table: `Null < Bool < Number < Duration < Temporal < Text <
  Link < List < Object`. Nulls sort first; ticket 26 owns their placement.
  Remove `TextShape::classify` sniffing from `SortKey`. Do not map `List` or
  `Object` to `Null`.

**Numbers:** compare `Int` and `Float` exactly on the number line; never use
`i64 as f64`. Treat `-0.0 == 0.0`. Turn `NaN` into `Null` at construction and
warn; only frontmatter `.nan` can produce it. Keep `±inf` as valid `Float`
values at the ends of the sort range. Grouping uses the same equality.

**Temporal:** naive datetimes are civil, zone-free values. Only input with `Z`
or an offset is an instant. Compare an ordinary `Date` field with a civil
`DateTime` field at civil midnight. A period-valued field or date-shaped
literal uses the membership relation below instead.

To compare a civil value with an instant, use the pinned zone from the
execution frame (ticket 21). Do not look up an ambient zone in ordering. The
cache holds no machine zone. Inline fields may now yield `DateTime`. This
revokes ticket 18's exclusion, whose reason was the persisted zone shift.

**Classification:** keep frontmatter classification quote-insensitive;
quoted links remain recognised. Inline quotes remain the escape. This decision
does not add `string()`. Ticket 20 rejects a schema override. An `input`
consumer gets a text view of the typed value. It does not restore the
original frontmatter lexeme.

**Adaptive literals (replaces `classify_literal` sniffing):**

- Adapt quoted string literals only. Bare tokens are already typed. Adapt either
  side of a comparison, the counterpart-typed operand in arithmetic
  (`due - "2026-07-29"`, `due + "1d"`), and the `contains` target. Do not adapt
  field-versus-field or literal-versus-literal comparisons; both literals stay
  strings in the latter case.
- At query build, parse each quoted literal once into candidates: DatePoint and
  its validated CalendarPeriod, DateTime, Duration, Link target, Number, Bool,
  and later a natural-language date. Use the bare number token's decimal
  grammar: `"5"` is `Int`, `"5.0"` is `Float`, and integers beyond `i64` become
  `Float`. Do not accept hex, separators or non-finite numbers.
- Against a Date, DateTime or CalendarPeriod row, select the period candidate
  of a date-shaped literal *for comparison*. Keep DatePoint for date
  arithmetic. Other rows select the candidate matching their kind (Duration,
  Link, `Int` or `Float`, Bool). `String`, `List`, `Object` and `Null` rows use
  raw text. So do rows whose kind has no parsed candidate; that text fails
  the bracket. Do not parse or allocate per row. Use the pinned frame for any
  row-dependent zone conversion.
- A `Link` row adopts a literal as its folded target. In `[[Alpha|A]]`, only
  the target contributes. This resolves schema select string versus `Link`
  matching for ticket 20.
- Literal precision controls matching. `"2025-01"` covers the whole month; a
  day literal against a `DateTime` covers the whole day. A civil period has a
  first and last *civil date*. For a point compared with a period:
  - `==` matches when the point's civil date is inside the period.
  - `<` matches a date before the period; `<=` matches one before or inside it.
  - `>` matches a date after the period; `>=` matches one inside or after it.
  Swap operands to reverse the relation.
- For instants, get their civil dates in the pinned zone. When local date
  mapping is monotone, the equivalent instant bounds are `[s, e)`, with an
  **exclusive** upper bound. Do not put the next period's starting instant in
  both periods. Apply this rule to bare and quoted literals.
- Keep civil-midnight comparison for an ordinary Date field versus DateTime
  field. A `CalendarPeriod` field uses membership against a point. Two periods
  filter-equal only if their normalized starts and units match. Relational
  filters compare their `(start, unit)` keys with a fixed unit tie-break, not
  interval overlap. Keep structural `PartialEq` and deterministic sort
  separate from these filter relations.
- Natural-language dates remain a hook, not a decided grammar. The proposed
  grammar is deterministic, closed and English. It resolves against injected
  `today` and is shared by filter literals, `date()`, `date_add` and template
  filters. Ticket 27 owns grammar and crate choice.

**Links:** equality and order use the target-plus-subpath `ComparisonKey`.
`display` and `embedded` remain queryable attributes. Sort Links with the
string collator on the target, not with `ComparisonKey`.

`date(link)` reads only the target's last path segment. Strip folders,
`#anchor` and `.md`, but never read display text. It accepts an optional
pattern argument for non-ISO titles. A date-titled link stays in the Link
bracket. Link adoption lets `due == "2025-01-15"` match `[[2025-01-15]]`.
For range filters and sorts across mixed Date and Link rows, use `date(field)`.

**Durations:** nominal seconds (`1mo == 30d`, `1y == 365d`) stay the equality
and order, provisionally; calendar arithmetic stays Jiff-based. Refinement
belongs to ticket 27.

**Strings:** `==` and `!=` stay byte-exact for strings. Link comparison instead
folds case through `ComparisonKey`: `project == "alpha"` matches `[[Alpha]]`,
but `title == "alpha"` stays case-sensitive.

Sort strings with `icu_collator`: CLDR root collation, tertiary strength,
numeric ordering, significant punctuation and a raw-byte tiebreak. Thus
`Chapter 2 < Chapter 10` and `apfel < Äpfel < apple < Zebra`. Do not use
host-locale collation. Sort keys are transient; do not persist them.
The scout's ~350 KB binary cost and that ICU4X is already in
the tree through `textwrap` were not verified here; the specification must
confirm them.

**Error mode (principle, refined by ticket 21):**

- Classification failure keeps the string. Explicit casts (`date()`, `dur()`)
  fail to `Null`. Ordinary data mismatches do not cause filter evaluation
  errors. Arithmetic on mismatched kinds returns `Null`.
- A failure of a required zone, calendar resolution or presentation produces
  a typed query/render error. This also applies during lazy row
  materialization. Do not turn such failures into nonmatches or blank cells.
- Advisory stderr notices do not change successful stdout or exit codes.
  These notices are the parse-time `NaN` warning, a count of rows excluded by
  bracket mismatch, and a build-time warning for an invalid date-shaped
  quoted literal. For example, `"2025-02-30"` still runs as a string.
  A bare invalid date literal remains a syntax error.
- Ticket 31 owns the diagnostic and error mechanism.

**Invariant owners:** parse-time classification (note parser, one precedence
order, ticket 18); numeric identity (the value's constructors); the three
relations (value-semantics module); literal adaptation (query binder);
`ComparisonKey` and link identity (root `Link`); zone and `now` (execution
context, ticket 21).

**Becomes removable:** `NoteFieldValueRef::compare`, `is_equal_to_literal`,
`ComparisonExpr::classify_literal`, `TextShape::classify` and its two callers,
the ambient-zone promotion in `DateTimeValue::try_from(DateValue)` and
`is_midnight_of`, and the `List`/`Object` to `Null` mapping in `SortKey`.

**Deliberate behaviour changes (for the specification's change table):**
relational filters across kinds stop matching (today they match by rank); quoted
filter literals adapt per row (fixes `title == "2025-01-15"` on a String field);
coarse date literals match intervals (`due == 2026-07` no longer means the
first); naive datetimes are civil, so the cache no longer embeds the indexing
machine's zone (cache format change); inline fields can be `DateTime`; `NaN`
becomes `Null`; `Link` equality ignores display and embed; string sorting is
Unicode-collated and natural; `List`/`Object` sort last, not first; ranges and
sorts over date-titled `Link`s go through `date()`.

**Remains unresolved (handed off):**

- [Query core](./26-query-core-and-plan.md): literal syntax for `in (...)` and
  ranges, relative keywords (`today`, `eom`), existence and type predicates,
  string functions (`lower`, `startswith`; `string()` only if a use appears), a
  truncating date function for day matching, null placement, `GroupBy` key
  relation, and how adaptation threads into `ValueExpr` arithmetic.
- [Temporal boundary](./27-temporal-boundary-and-dialects.md): natural-language
  grammar and crate, duration refinement, civil vs instant type shape, and a
  `DateConfig` sub-Config (coordinated with ticket 22).
- [Note document model](./23-note-document-model.md): inline `DateTime` and
  multi-value element-wise classification.
- [Index read model](./24-index-read-model-and-query-execution.md) and
  [freshness](./25-index-freshness-and-invalidation.md): civil datetime cache
  encoding and the cache format change.
- [Template boundary](./28-template-engine-boundary.md): templates use this
  relation module, not minijinja's own comparison.
- [Error architecture](./31-error-and-diagnostic-architecture.md): the mechanism
  for the two advisory notices.
- [Crate topology](./32-crate-topology-and-public-surface.md): visibility of the
  relation module and collator dependency.

Research: [brief](../research/19-prep/brief.md) and its inputs `a` to `k`.

## Superseded in part by ticket 20

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The schema
override for date-shaped text referred to above is not built: `input` accepts
any scalar, consumers get a text view rendered from the typed value, and no
`string()` cast is added until a real use appears. Select strings naming links
are parsed into `Link` values in the schema checker and use the shared Link
equality relation, not the query binder's literal adaptation.

## Refined by ticket 21

The [execution-context
Answer](./21-ambient-environment-and-execution-context.md) sets a lazily
acquired system-local default zone (configurable override source still belongs
to tickets 22/27), civil `CalendarPeriod` identity and an exclusive literal
upper boundary. Period membership is a filter predicate, not `PartialEq` or
`Ord`: period-versus-period equality is exact normalized identity; overlap and
containment require separate operators if needed. Its request frame also makes
environmental failures typed while leaving ordinary bracket mismatches as
nonmatches. These refinements override the earlier inclusive `[s, e]` wording
and blanket "filters never error" principle.

## TODOs

Glossary updates deferred until the specification is written (do not edit the
glossaries yet; they describe the current code):

- TODO `src/temporal/GLOSSARY.md`: rewrite *Date-Time* (naive input becomes a
  civil value, only `Z`/offset input is an instant; remove "unzoned input
  resolves in the process's local zone" and "comparisons with Date promote the
  Date to local midnight"); rewrite *Zone* (supplied by the execution context,
  not process-local); keep *Duration* nominal wording once ticket 27 settles it.
- TODO `src/GLOSSARY.md`: *Duration* still says "written source components",
  which conflicts with `src/temporal/GLOSSARY.md` (pre-existing drift);
  *Temporal Strictness & Resolution* says query filters evaluate "strict typed
  expressions", which adaptive literals qualify; *Comparison Key* gains link
  equality and stays excluded from sort order.
- TODO `src/note/GLOSSARY.md`: *Note Field Value* omits `Int`, `Float`,
  `DateTime` and `Duration`.
- TODO `src/query/GLOSSARY.md`: add *Type Bracket*, *Adaptive Literal*, *Literal
  Precision*, and the three relations (loose equality, bracketed order, total
  sort order); adjust *Filter Expression* and *Sort Order*.
- TODO new terms to define with a Not-this list: *Classification* (parse-time
  typing of text), *Cast* (explicit query-time conversion, fails to `Null`),
  *Collation* (sort-only string order).

## Comments

2026-10-21, after [research
13](../research/13-iwe-query-schema-and-agent-write-gating.md) and
[14](../research/14-fileclass-model-binding-and-validation.md): no decision
changes. IWE independently uses the same value semantics for its query engine:
strict scalar types, cross-type `==` false, cross-type ordering undefined for
filters, ICU natural collation, nulls first ascending, a rank-based total order
for sort (`crates/liwe/src/query/filter.rs`, `sort.rs`, `collation.rs`) [V]. It
has no date, duration or link kinds, so it does not test adaptive literals or
interval semantics. Fileclass's validator goes the other way: a number field
accepts any string `Number()` can parse and a boolean field accepts `"true"` and
`"false"` (`src/fields/validate.ts:66-81`) [V]. That is the lenient behaviour
this ticket's classification rule avoids.
