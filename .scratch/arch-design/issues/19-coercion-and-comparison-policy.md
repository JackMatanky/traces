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

**Decision:** one value-semantics module owns every comparison, with three
separate relations instead of one overloaded `compare`. Note values are
classified once at parse and never sniffed again; the only late typing is an
operand-adaptive string literal in queries. No ambient zone or clock reaches any
comparison.

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

**Relations (owner: the value-semantics module; three relations):**

- `NoteFieldValue: PartialEq` stays structural, with no `Ord` and no `Hash`.
- *Loose equality* and *bracketed order* serve filters. Brackets: Numeric
  (`Int`, `Float`), Temporal (`Date`, `DateTime`, `CalendarPeriod`), Duration,
  String, Bool, Link, List, Object, Null. Across brackets `==` is false, `!=` is
  true and `<`, `<=`, `>`, `>=` do not match. `Null` never matches a relational
  operator; `null == null` is true and a missing field equals `null`. `!=` is
  the plain complement, so `not (x < 5)` also matches other kinds and nulls
  (same as MongoDB); this is documented, not rewritten.
- *Total sort order* (the third relation) serves `SortKey`, built with an
  explicit context. Rank table kept as implemented: `Null < Bool < Number <
  Duration < Temporal < Text < Link < List < Object`; nulls first (placement is
  ticket 26's). The sort-time sniffing (`TextShape::classify` in `SortKey`) is
  deleted, and `List`/`Object` stop being mapped to `Null`.

**Numbers:** an exact `Int`/`Float` comparator (compare on the number line,
never `i64 as f64`), `-0.0 == 0.0`. `NaN` becomes `Null` at construction with a
warning (only frontmatter `.nan` can produce it); `±inf` stays a valid `Float`
and sorts at the ends of the range. Grouping uses the same equality.

**Temporal:** naive datetimes are civil (zone-free) values; only input with `Z`
or an offset is an instant. An ordinary `Date` **field versus** a civil
`DateTime` field compares at civil midnight; a period-valued field or
date-shaped literal instead uses the membership relation below. Comparing a
civil value with an instant takes the pinned zone from the execution frame
(ticket 21), never an ambient lookup inside ordering. The cache holds no machine
zone. Inline fields may now yield `DateTime`, which revokes ticket 18's
exclusion (its reason was the persisted zone shift).

**Classification:** frontmatter classification stays quote-insensitive (links
stay recognised when quoted); inline quotes remain the escape. No `string()`
function is part of this decision. Ticket 20 rejects a schema override: an
`input` consumer gets a text view of the typed value, not an escape that
restores the original frontmatter lexeme.

**Adaptive literals (replaces `classify_literal` sniffing):**

- Applies to quoted string literals only; bare tokens are already typed.
  Positions: either side of a comparison, the counterpart-typed operand in
  arithmetic (`due - "2026-07-29"`, `due + "1d"`), and the `contains` target.
  Never field vs field and never literal vs literal (both stay strings).
- At query build, pre-parse a quoted literal once into candidates: DatePoint and
  its validated CalendarPeriod, DateTime, Duration, Link target, Number (decimal
  grammar of the bare number token: `"5"` is `Int`, `"5.0"` is `Float`, integers
  beyond `i64` become `Float`; no hex, separators or non-finite), Bool, and
  later a natural-language date. A date-shaped literal against a Date, DateTime
  or CalendarPeriod row selects the period candidate *for comparison*; DatePoint
  remains available to date arithmetic. Other rows select the candidate matching
  their kind (Duration, Link, `Int` or `Float`, Bool). `String`, `List`,
  `Object` and `Null` rows, and rows whose kind has no parsed candidate, take
  the raw text, which then fails the bracket. No per-row parsing or allocation;
  a row-dependent zone conversion still uses the pinned frame.
- A `Link` row adopts a literal as its folded target; a literal such as
  `[[Alpha|A]]` contributes only its target. This settles schema select string
  vs `Link` matching for ticket 20.
- Literal precision decides what it matches: `"2025-01"` is the whole month and
  a day literal against a `DateTime` is the whole day. A civil period has a
  first and last *civil date*: `==` matches a point whose civil date lies inside
  it, `<` a date before it, `<=` a date before or inside it, `>` a date after
  it, `>=` a date inside or after it; swap operands for the reversed relation.
  For instants, obtain their civil dates in the pinned zone; equivalently, when
  local date mapping is monotone, instant bounds are `[s, e)` with an
  **exclusive** upper bound. Never include the next period's starting instant in
  both periods. This applies to bare and quoted literals. Ordinary Date field
  versus DateTime field retains the civil-midnight convention; a
  `CalendarPeriod` field uses membership against a point. Two periods
  filter-equal only on matching normalized start and unit; relational filters
  compare their `(start, unit)` keys with a fixed unit tie-break, not interval
  overlap. Their structural `PartialEq` and deterministic sort relation remain
  separate.
- Natural-language dates are a hook, not decided grammar: a deterministic,
  closed English grammar resolved from the injected `today`, shared by filter
  literals, `date()`, `date_add` and template filters. Grammar and crate choice
  belong to ticket 27.

**Links:** equality and order use the `ComparisonKey` of the target plus
subpath; `display` and `embedded` stay queryable attributes. Link sorting uses
the string collator on the target, not `ComparisonKey`. `date(link)` reads only
the target's last path segment (folders, `#anchor` and `.md` stripped), never
display text, with an optional pattern argument for non-ISO titles. A
date-titled link stays in the Link bracket: `due == "2025-01-15"` still matches
`[[2025-01-15]]` through link adoption, but range filters and sorts across mixed
Date and Link rows use `date(field)`.

**Durations:** nominal seconds (`1mo == 30d`, `1y == 365d`) stay the equality
and order, provisionally; calendar arithmetic stays Jiff-based. Refinement
belongs to ticket 27.

**Strings:** `==`/`!=` stay byte-exact, except that link comparison folds case
through `ComparisonKey`, so `project == "alpha"` matches `[[Alpha]]` while
`title == "alpha"` stays case-sensitive. Sorting uses `icu_collator` with the
CLDR root collation, tertiary strength, numeric ordering, punctuation treated as
significant, and a raw-byte tiebreak, so `Chapter 2 < Chapter 10` and `apfel <
Äpfel < apple < Zebra`. Never host-locale collation; sort keys are transient and
never persisted. The scout's ~350 KB binary cost and that ICU4X is already in
the tree through `textwrap` were not verified here; the specification must
confirm them.

**Error mode (principle, refined by ticket 21):** classification failure keeps
the string; explicit casts (`date()`, `dur()`) fail to `Null`; *ordinary data
mismatches* in filters never error at evaluation, and arithmetic on mismatched
kinds is `Null`. A required zone, calendar resolution or presentation failure is
instead a typed query/render error, including when it arises during lazy row
materialization; never turn it into a nonmatch or a blank cell. Advisory stderr
notices (the parse-time `NaN` warning plus two query notices) leave successful
stdout and exit codes unchanged: a count of rows excluded by a bracket mismatch
and a build-time warning for a quoted literal that is date-shaped but invalid
(`"2025-02-30"`; it still runs as a string). A bare invalid date literal remains
a syntax error. The diagnostic/error mechanism is ticket 31's.

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
