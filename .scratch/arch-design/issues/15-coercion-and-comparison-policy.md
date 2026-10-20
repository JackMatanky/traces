# Where do coercion and cross-type comparison live, and what is the policy?

Type: grilling
Status: resolved
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

See [ticket 14's Answer](./14-value-model.md). Handed to this ticket:
frontmatter literal-string escape (frontmatter classification is
quote-insensitive, inline quotes are the escape); `Int`/`Float` share one
numeric rank, derived `PartialEq` stays structural so semantic numeric
equality lives here; `Link` equality and order, including `embedded`;
zone-shifted naive datetimes (frontmatter `12:00` was observed as `10:00Z`,
persisted in the cache); schema select strings vs note `Link` matching.

## Pre-grilling research

Brief: [research/15-prep/brief.md](../research/15-prep/brief.md) (options, leans,
corrections, proposed grilling order). Scout inputs:
[code inventory and probe matrix](../research/15-prep/a-code-inventory.md),
[digest lessons](../research/15-prep/b-digest-lessons.md),
[external precedent](../research/15-prep/c-external-precedent.md),
[prior decisions](../research/15-prep/d-prior-decisions.md).

## Answer

**Decision:** one value-semantics module owns every comparison, with three
separate relations instead of one overloaded `compare`. Note values are
classified once at parse and never sniffed again; the only late typing is an
operand-adaptive string literal in queries. No ambient zone or clock reaches
any comparison.

**Alternatives considered:** keep as is (three equality evaluators, rank-leaking
relational filters, three promotion seams); lexical storage with typing on
demand (rejected: zero-config date queries would need a schema or casts);
quote-sensitive frontmatter (rejected: no public Noyalib route, CST costs +59%
per note and 2.6x for the lazy variant on notes with dates, and Templater,
Metadata Menu and formatters write quoted dates, so notes would silently drop
out of date queries); SQLite-style total order in filters (the current
behaviour); erroring filters (hostile to open-world vaults); a custom
`PartialEq` equating `Int` and `Float` (breaks `Eq`/`Hash` coherence);
comparison by resolved note identity (needs the index, impure).

**Relations (owner: the value-semantics module; three relations):**

- `NoteFieldValue: PartialEq` stays structural, with no `Ord` and no `Hash`.
- *Loose equality* and *bracketed order* serve filters. Brackets: Numeric
  (`Int`, `Float`), Temporal (`Date`, `DateTime`), Duration, String, Bool,
  Link, List, Object, Null. Across brackets `==` is false, `!=` is true and
  `<`, `<=`, `>`, `>=` do not match. `Null` never matches a relational
  operator; `null == null` is true and a missing field equals `null`.
  `!=` is the plain complement, so `not (x < 5)` also matches other kinds and
  nulls (same as MongoDB); this is documented, not rewritten.
- *Total sort order* (the third relation) serves `SortKey`, built with an
  explicit context. Rank table kept as implemented: `Null < Bool < Number <
  Duration < Temporal < Text < Link < List < Object`; nulls first (placement is
  ticket 22's). The
  sort-time sniffing (`TextShape::classify` in `SortKey`) is deleted, and
  `List`/`Object` stop being mapped to `Null`.

**Numbers:** an exact `Int`/`Float` comparator (compare on the number line,
never `i64 as f64`), `-0.0 == 0.0`. `NaN` becomes `Null` at construction with a
warning (only frontmatter `.nan` can produce it); `±inf` stays a valid `Float`
and sorts at the ends of the range. Grouping uses the same equality.

**Temporal:** naive datetimes are civil (zone-free) values; only input with `Z`
or an offset is an instant. A `Date` compares as civil midnight against civil
datetimes. A civil value against an instant takes an explicit zone from the
execution context (ticket 17); the zone and `now` enter when sort keys and
filter operands are built, never inside an ordering. The cache holds no machine
state. Inline fields may now yield `DateTime`, which revokes ticket 14's
exclusion (its reason was the persisted zone shift).

**Classification:** frontmatter classification stays quote-insensitive (links
stay recognised when quoted); inline quotes remain the escape. No `string()`
function is part of this decision. A frontmatter value that must stay text uses
the schema override (ticket 16).

**Adaptive literals (replaces `classify_literal` sniffing):**

- Applies to quoted string literals only; bare tokens are already typed.
  Positions: either side of a comparison, the counterpart-typed operand in
  arithmetic (`due - "2026-07-29"`, `due + "1d"`), and the `contains` target.
  Never field vs field and never literal vs literal (both stay strings).
- At query build the literal is pre-parsed once into candidates: Date,
  DateTime, Duration, Link target, Number (decimal grammar of the bare number
  token: `"5"` is `Int`, `"5.0"` is `Float`, integers beyond `i64` become
  `Float`; no hex, separators or non-finite), Bool, and later a
  natural-language date. Each row selects the candidate matching its own kind
  (Date, DateTime, Duration, Link, `Int` or `Float`, Bool). A `DateTime` row
  falls back to the date candidate. `String`, `List`, `Object` and `Null` rows,
  and rows whose kind has no parsed candidate, take the raw text, which then
  fails the bracket. No per-row parsing or allocation.
- A `Link` row adopts a literal as its folded target; a literal such as
  `[[Alpha|A]]` contributes only its target. This settles schema select string
  vs `Link` matching for ticket 16.
- Literal precision decides what it matches: `"2025-01"` is the whole month and
  a day literal against a `DateTime` is the whole day. With `[s, e]` the
  literal's interval: `==` is `s <= x <= e`, `<` is `x < s`, `<=` is `x <= e`,
  `>` is `x > e`, `>=` is `x >= s`. This applies to bare and quoted literals.
  A day literal against an instant takes the context zone (ticket 17). Field vs
  field keeps the midnight convention.
- Natural-language dates are a hook, not decided grammar: a deterministic,
  closed English grammar resolved from the injected `today`, shared by filter
  literals, `date()`, `date_add` and template filters. Grammar and crate
  choice belong to ticket 23.

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
belongs to ticket 23.

**Strings:** `==`/`!=` stay byte-exact, except that link comparison folds case
through `ComparisonKey`, so `project == "alpha"` matches `[[Alpha]]` while
`title == "alpha"` stays case-sensitive. Sorting uses `icu_collator` with the
CLDR root collation, tertiary strength, numeric ordering, punctuation treated
as significant, and a raw-byte tiebreak, so `Chapter 2 < Chapter 10` and
`apfel < Äpfel < apple < Zebra`. Never host-locale collation; sort keys are
transient and never persisted. The scout's ~350 KB binary cost and that ICU4X
is already in the tree through `textwrap` were not verified here; the
specification must confirm them.

**Error mode (principle):** classification failure keeps the string; explicit
casts (`date()`, `dur()`) fail to `Null`; filters never error at evaluation;
arithmetic on mismatched kinds is `Null`. Advisory stderr notices (the
parse-time `NaN` warning plus two query notices), with stdout and exit codes
unchanged: a one-line count of rows excluded by a bracket
mismatch (for example a `Date` row meeting a literal with no date candidate),
and a build-time warning for a quoted literal that is date-shaped but invalid
(`"2025-02-30"`; it still runs as a string). A bare invalid date literal
remains a syntax error. The mechanism is ticket 27's.

**Invariant owners:** parse-time classification (note parser, one precedence
order, ticket 14); numeric identity (the value's constructors); the three
relations (value-semantics module); literal adaptation (query binder);
`ComparisonKey` and link identity (root `Link`); zone and `now` (execution
context, ticket 17).

**Becomes removable:** `NoteFieldValueRef::compare`, `is_equal_to_literal`,
`ComparisonExpr::classify_literal`, `TextShape::classify` and its two callers,
the ambient-zone promotion in `DateTimeValue::try_from(DateValue)` and
`is_midnight_of`, and the `List`/`Object` to `Null` mapping in `SortKey`.

**Deliberate behaviour changes (for the specification's change table):**
relational filters across kinds stop matching (today they match by rank);
quoted filter literals adapt per row (fixes `title == "2025-01-15"` on a String
field); coarse date literals match intervals (`due == 2026-07` no longer means
the first); naive datetimes are civil, so the cache no longer embeds the
indexing machine's zone (cache format change); inline fields can be
`DateTime`; `NaN` becomes `Null`; `Link` equality ignores display and embed;
string sorting is Unicode-collated and natural; `List`/`Object` sort last, not
first; ranges and sorts over date-titled `Link`s go through `date()`.

**Remains unresolved (handed off):**

- [Query core](./22-query-core-and-plan.md): literal syntax for `in (...)` and
  ranges, relative keywords (`today`, `eom`), existence and type predicates,
  string functions (`lower`, `startswith`; `string()` only if a use appears),
  a truncating date function for day matching, null placement, `GroupBy` key
  relation, and how adaptation threads into `ValueExpr` arithmetic.
- [Temporal boundary](./23-temporal-boundary-and-dialects.md): natural-language
  grammar and crate, duration refinement, civil vs instant type shape, and a
  `DateConfig` sub-Config (coordinated with ticket 18).
- [Execution context](./17-ambient-environment-and-execution-context.md): the
  zone and `today`/`now` inputs, and a default zone for civil-vs-instant.
- [Schema note typing](./16-schema-note-typing-and-validation.md): the
  schema string override and select matching via link adoption.
- [Note document model](./19-note-document-model.md): inline `DateTime` and
  multi-value element-wise classification.
- [Index read model](./20-index-read-model-and-query-execution.md) and
  [freshness](./21-index-freshness-and-invalidation.md): civil datetime cache
  encoding and the cache format change.
- [Template boundary](./24-template-engine-boundary.md): templates use this
  relation module, not minijinja's own comparison.
- [Error architecture](./27-error-and-diagnostic-architecture.md): the
  mechanism for the two advisory notices.
- [Crate topology](./28-crate-topology-and-public-surface.md): visibility of
  the relation module and collator dependency.

Research: [brief](../research/15-prep/brief.md) and its inputs `a` to `k`.

## TODOs

Glossary updates deferred until the specification is written (do not edit the
glossaries yet; they describe the current code):

- TODO `src/temporal/GLOSSARY.md`: rewrite *Date-Time* (naive input becomes a
  civil value, only `Z`/offset input is an instant; remove "unzoned input
  resolves in the process's local zone" and "comparisons with Date promote the
  Date to local midnight"); rewrite *Zone* (supplied by the execution context,
  not process-local); keep *Duration* nominal wording once ticket 23 settles it.
- TODO `src/GLOSSARY.md`: *Duration* still says "written source components",
  which conflicts with `src/temporal/GLOSSARY.md` (pre-existing drift);
  *Temporal Strictness & Resolution* says query filters evaluate "strict typed
  expressions", which adaptive literals qualify; *Comparison Key* gains link
  equality and stays excluded from sort order.
- TODO `src/note/GLOSSARY.md`: *Note Field Value* omits `Int`, `Float`,
  `DateTime` and `Duration`.
- TODO `src/query/GLOSSARY.md`: add *Type Bracket*, *Adaptive Literal*,
  *Literal Precision*, and the three relations (loose equality, bracketed order,
  total sort order); adjust *Filter Expression* and *Sort Order*.
- TODO new terms to define with a Not-this list: *Classification* (parse-time
  typing of text), *Cast* (explicit query-time conversion, fails to `Null`),
  *Collation* (sort-only string order).
