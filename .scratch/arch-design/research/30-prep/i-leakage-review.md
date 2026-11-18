# Temporal boundary and error leakage review

## Scope and evidence

This report covers Rust-design lens 6: boundary crossings and vertical
leakage. It prepares ticket 30. It does not settle human choices or change
accepted Answers.

[V] means source inspection in this review. [H] means an inference. [P]
means a candidate for approval. The report separates inspected code from
accepted target rules.

This report records the inspected snapshot, not the repaired synthesis.
References to incorrect owner evidence concern the older note inspected here.
[L5](./l-answer-verification.md#l5-membership-bounds-do-not-prove-a-conflicting-physical-key-rule)
later clarifies L7's claimed wording conflict. Later bounded probes in
[m-gap-closure.md](./m-gap-closure.md) do not replace implementation evidence.

CodeGraph `status` reported 12 modified files. It did not report a fully
synchronized index. CodeGraph `explore` and `node` returned source from disk
for the cited code. One exploration omitted changed `src/template/engine.rs`.
This report makes no claim about that file's body.

LSP supplied references for `pattern_to_strftime`, `find_closing_flat`,
`DEFAULT_DATETIME_PATTERN`, and `DatePoint::parse`. The first
translator-reference request returned no references. A later request
returned seven locations.

These are source reads, not execution evidence. I ran no builds, tests,
linters, formatters, benchmarks, or diagnostic scripts. During that review,
I changed no source files or earlier research files.

## L1. Keep policy direction separate from shared lexer dependencies

**Current source [V].** The inspected crate references show no temporal
imports of `config`, `query`, `template`, `schema`, or `note`. Temporal does
use root primitives. Thus, the claim that temporal has no internal
dependencies is false.

- `src/temporal/token.rs:11-12,86-100,106-131` imports `DelimiterType` and calls
  its flat bracket scan.
- `src/temporal/point.rs:143-150` returns root `Spanned` values through
  `crate::lexer::lex_prefix`.
- `src/temporal/duration.rs:274-275` uses the same prefix adapter.
- `src/lexer.rs:10-25` imports root `Spanned`; `lex_prefix` adds a byte extent
  without temporal policy.
- `src/delimiter.rs:121-133,305-308` defines delimiter vocabulary and the flat
  scan.
- `src/config/model/frontmatter.rs:250-263` reads the temporal default through
  the crate-root re-export.
- `src/lib.rs:170-175` separates crate-local temporal helpers from outward
  temporal values.

LSP found only two external calls to `find_closing_flat`. Both occur in
`temporal/token.rs`. This root dependency does not establish a domain-module
cycle. The prefix adapter also does not make temporal depend on note parsing.

**Inherited rule.** Ticket 22 assigns `[date]` parsing to config and sets zone
precedence. Ticket 21 assigns validated periods, week rules, and explicit-zone
operations to temporal. See
[22:223-233](../../issues/22-config-ownership-and-lifecycle.md) and
[21:106-127,134-179](../../issues/21-ambient-environment-and-execution-context.md).
Ticket 30 Q28 explicitly excludes a temporal import of config.

**Candidate [P].** Let the composition layer convert `DateConfig` into
temporal-owned policy data. Pass that data to the query or render frame.
Do not pass `&Config` into temporal. If Moment translation moves, review
whether its exclusive flat-scan helper should move with it. Do not move
shared prefix recognition merely because temporal calls it.

**Retain option.** Retain the current root lexer, position, and delimiter
dependencies while the crate stays whole. Retain the config-to-temporal constant
dependency if the config format keys remain. Delete that constant dependency
only when the keys lose their last use.

**Disposition for ticket 30.** Q2 and Q28 need separate decisions about dialect
ownership and policy projection. No inspected source requires a new generic
foundation module. Reject a policy design that requires temporal to read config,
trust state, or an index policy.

## L2. Schema format checks must not depend on a render-only adapter

**Current source [V].** `SchemaDateField::parse` checks that `format` is a
string and stores it. Its only accessor has `#[cfg(test)]`. See
`src/schema/fields/date.rs:12-23,41-53`. The tests use `YYYY-MM-DD`. They do
not establish a shared runtime dialect contract.

`pattern_to_strftime` takes a civil datetime, not only a pattern.
The `Do` branch writes the supplied day and its suffix into the result.
See `src/temporal/token.rs:69-77,186-204`.
LSP found its production calls only in the template date adapter.
Query pattern parsing instead calls `DatePoint::parse_with_pattern` directly.
See `src/query/grammar/filter.rs:702-719` and `src/temporal/point.rs:152-185`.

**Inherited rule.** Schema owns the pure value check. Schema definitions
do not change note classification or index freshness. See
[20:53-70,79-89](../../issues/20-schema-note-typing-and-validation.md).
The intact Answer states that `date.format` is a write hint. It also accepts
Date/DateTime and any scalar when the format has no date component. See
[20:119-123](../../issues/20-schema-note-typing-and-validation.md). The
truncated sentence occurs separately at line 315. It does not erase the
intact rule.

**Candidate [P].** Separate pattern inspection from rendering. Let temporal or a
format unit report pattern validity and required components as data. Let schema
attach field provenance and return schema errors or advisory violations. Keep
Moment at the template adapter if no other approved surface needs it. Do not
make schema call the template engine.

**Retain option.** Keep `format` as an unvalidated hint until the dialect
decision. This retains current code, not the full accepted target check. A
shared Moment validator remains possible if schema retains that dialect. It must
not need a real datetime to inspect `Do`.

**Disposition for ticket 30.** Q1-Q4 must distinguish declaration syntax checks
from note-value checks. A load-time lint alone does not settle the accepted
scalar-acceptance rule. Counterexample: a quarter-only format cannot need a full
date value. Reject an adapter that validates formats by sampling the clock or
rendering one arbitrary date. Ticket 34 still owns diagnostic codes and
presentation.

## L3. Query adapters erase environmental failures before the query boundary

**Current source [V].** The temporal layer has domain errors, not exposed Jiff
errors. `CalendarError::Zone` retains a `DateError` source. See
`src/temporal/error.rs:164-201`. `local_zone` maps system discovery failure to
`DateError::LocalZoneLookup`. It stores the upstream detail as text, not as a
typed Jiff source. See `src/temporal/zone.rs:13-35` and
`src/temporal/error.rs:57-76`.

The query adapter removes that distinction:

- Arithmetic uses `local_zone().ok()?` and `apply(...).ok()?` in
  `src/query/grammar/filter.rs:359-407`.
- Calendar boundaries use `local_zone().ok()?` at `filter.rs:636-647`.
- Explicit date parsing uses `.ok()?` at `filter.rs:702-719`.
- `date_boundary` maps both zone and range failures to `None` at
  `filter.rs:727-746`.
- `ComparisonExpr::is_matching` maps a missing evaluated operand to `false` at
  `filter.rs:203-215`.

Other consumers also suppress failures. `NoteFieldValue::as_date` maps
local-date failure to `None` at `src/note/field.rs:109-115`. Its cross-kind
comparator replaces failed midnight conversion with `Less` or `Greater` at
`field.rs:297-300`.

[H] A required-zone failure can therefore change the selected rows without a
typed query error. This does not always mean exclusion: `Not` negates the false
predicate. See `src/query/grammar/expr.rs:116-127`. I did not execute a
failing-zone example.

**Inherited rule.** Data mismatches and failed explicit casts can yield
nonmatches or `Null`. Required zone, calendar-resolution, and presentation
failures must remain typed. See
[19:199-212](../../issues/19-coercion-and-comparison-policy.md) and
[21:123-132](../../issues/21-ambient-environment-and-execution-context.md).

**Candidate [P].** Keep invalid user data distinct from failed required
context through evaluation and lazy materialization. Resolve the frame once.
Do not assume every row-dependent conversion can finish during binding.
Return domain categories to query. Let query attach its operation context.

**Retain option.** Retain temporal error ownership and ordinary mismatch
behavior. Retain `Option` only where absence cannot hide an environmental
failure. The current blanket `.ok()` adapters cannot satisfy the accepted
target.

**Disposition for ticket 30.** Q21 and Q30-Q36 need a failure-category contract,
not only a new frame parameter. Counterexample: a valid instant needs civil
projection, but system-zone discovery fails. Reject any design that converts
this failure to `Null`, a comparison rank, or a boolean result.

## L4. Error preservation in direct helpers and generic text paths

**Current source [V].** Template date helpers generally preserve temporal
causes. `parse_date_point` attaches `DateError` to a render error. `date_format`
attaches translator errors and maps zone and formatting errors. See
`src/template/engine/date.rs:389-392,467-475,865-873`. `invalid_operation` uses
Minijinja's `with_source` at `src/template/engine/error.rs:49-56`. Do not
describe those helpers as silent query-style error suppression.

A separate text path loses the error category.
`DateTimeValue::Display` maps zone and rendering failures to `fmt::Error`.
See `src/temporal/datetime.rs:316-323`.
`NoteFieldValueRef::append_text` ignores the result of writing a DateTime.
See `src/note/field.rs:315-338`.
This path cannot return a typed presentation failure to its caller.
I did not observe blank output or a panic in execution.

**Inherited rule.** Missing presentation context must not produce a blank cell.
Lazy query access and formatting must preserve execution errors.
See [21:123-132](../../issues/21-ambient-environment-and-execution-context.md).
Ticket 19 also requires a typed render error for required presentation failure.

**Candidate [P].** Use explicit, fallible, frame-aware presentation for human
output. Keep context-free canonical serialization separate from local
presentation. Do not try to carry a zone failure through `fmt::Error`.

**Retain option.** Keep the source-preserving Minijinja adapter pattern. Keep
`Display` only with a defined context-free representation, or exclude it from
environmental presentation paths. Do not invent one error wrapper per helper.

**Disposition for ticket 30.** Q33-Q34 must cover generic value-to-text paths as
well as named date filters. Counterexample: a table displays a stored instant
without calling `date_format`. Reject a cutover that fixes date filters but
leaves this path unable to report failure. Hand presentation details to tickets
31 and 34.

## L5. The owner-evidence claim about `DatePoint` is wrong

**Current source [V].** `DatePoint::Calendar` stores `DateValue` and written
date precision. `DatePoint::DateTime` stores `DateTimeValue`. See
`src/temporal/point.rs:25-52`. `DateTimeValue` is a transparent `Timestamp`
wrapper with derived `Ord`. See `src/temporal/datetime.rs:76-78`. Neither
runtime branch stores a civil datetime.

The parser temporarily distinguishes `Absolute(Timestamp)` and
`LocalClock(civil::DateTime)`. Its `resolve` method converts `LocalClock`
through the process-local zone. See `src/temporal/datetime.rs:622-657,133-154`.
`DatePoint::parse` stores the resolved result at
`src/temporal/point.rs:105-114`. This temporary parse distinction is not a
retained value distinction.

Thus, the inspected [owner-evidence.md:13-17](./owner-evidence.md) lists the
variants correctly but draws the wrong conclusion. The outer enum separates
civil dates from instants. It does not separate civil datetimes from
instants. The corrected [synthesis Q19](./synthesis.md) states this
distinction. Do not use the older owner note as evidence for ratification
without redesign.

**Inherited rule.** Naive datetimes must remain civil and zone-free.
Only explicit offsets or `Z` identify instants.
See [19:110-118](../../issues/19-coercion-and-comparison-policy.md).
Mixed temporal sort classes must remain separate.
See [29:716-730](../../issues/29-query-core-and-plan.md).

**Candidate [P].** Compare an inner civil/instant carrier with separate runtime
variants. Carry the distinction into stored values, comparisons, serialization,
and presentation. Changing only `DatePoint` does not change the stored
`NoteFieldValue::DateTime` payload. See `src/note/field.rs:43-64,165-172`.

**Retain option.** Keep the date-precision representation and validated temporal
wrappers. Keep the current instant-only datetime representation only as the
current baseline. It cannot satisfy the accepted civil-datetime target.

**Disposition for ticket 30.** Q19 needs a real carrier decision and a
cache-format handoff. The 80-byte limit remains a requirement, not a measured
property of either candidate. Counterexample: persist an unzoned datetime and
reopen it under another host zone. Reject a carrier that requires zone lookup to
reconstruct that civil value.

## L6. Q20 has a real runtime-carrier conflict

**Current source [V].** In the inspected code, `CalendarPeriod` selects Week,
Month, or Year. It carries no start date. See
`src/temporal/calendar.rs:18-30`. The inspected runtime enum has no period
variant. See `src/note/field.rs:43-64`. Query expressions and function results
use that runtime enum. See `src/query/grammar/filter.rs:238-280,621-649`.
These facts do not establish a target period carrier.

**Inherited rule.** Ticket 18 chooses one owned runtime model for notes,
queries, and templates. It permits a query-local borrowed view, not another
owned semantic model. See [18:456-489,529-536](../../issues/18-value-model.md).
Ticket 20 later places the shared runtime type below schema and note. See
[20:85-89](../../issues/20-schema-note-typing-and-validation.md).

Ticket 21 requires one validated `CalendarPeriod` with a civil start and unit.
It prohibits a second period-value type and heap allocation for that value.
It assigns filename-derived periods to `file.date`.
See [21:136-160](../../issues/21-ambient-environment-and-execution-context.md).

**Candidate [P].** Add the validated temporal period to the common runtime
carrier. Keep stored note classification separate. A runtime variant does
not need the note parser to produce it. Keep `file.date` evaluation-derived,
as the accepted index design requires.

**Retain option.** Retain the current note-origin variants and schema-local
literals. A query-only owned wrapper remains an alternative, but it explicitly
revises ticket 18's shared runtime-carrier decision. It does not necessarily
violate ticket 21 if it reuses the same period payload.

**Disposition for ticket 30.** Q20 must record whether it extends the shared
carrier or supersedes the single-carrier rule. The synthesis condition, “if
periods appear in stored fields,” does not settle runtime ownership.
Counterexample: a query computes `file.date` and passes it to grouping or a
template without storing it. Reject an option that cannot carry that value end
to end. Do not use a list/object requirement as the only rejection test for the
wrapper.

## L7. Q35 must separate canonical keys from predicate-specific bounds

**Current versus target [V].** The cited range-key protocol is an accepted
target, not an executed temporal-key proof here. The current filter code still
uses `NoteFieldValue` and its existing comparison adapter. See
`src/query/grammar/filter.rs:783-808`. This report establishes no temporal index
cost or soundness result.

**Inherited rule.** Ticket 29 defines canonical temporal classes as Civil,
Instant, and Period. Periods use normalized `(start, unit)` identity. Membership
never supplies key equality. See
[29:716-730](../../issues/29-query-core-and-plan.md).

R5 separately requires operator-specific bounds for all eligible temporal kinds.
It expressly uses membership bounds for point-period comparison.
An unproved mapping is Unsupported, and the residual always runs.
See [29:495-542](../../issues/29-query-core-and-plan.md).
R1 requires coverage of the whole bracket, including period-valued rows.
Quoted literals remain Unsupported under R3.

Section 10 reserves a `member_of` node without implementation or frontend.
It says that relation is never a range key or group key.
See [29:1176-1199](../../issues/29-query-core-and-plan.md).

**Candidate interpretation [H].** These clauses can coexist if “key” means a
stored canonical identity/order representation. A planner can derive
predicate bounds without declaring membership to be an equality relation.
However, this review also considers a prohibitive reading of “never a range
key”. L5 later rejects the claim that these reads prove a contradiction.

**Candidate [P].** State the distinction explicitly in Q35 and the
reconciliation handoff. Use a matrix with operator, literal kind, row kind,
frame requirement, and key encoding. Do not derive predicate bounds from
mixed-temporal total sort rank. Do not classify every instant mapping as
zone-dependent. Instant-to-instant epoch comparison differs from
instant-to-civil-period membership. A proof for instant rows alone still fails
R1 if other temporal rows can match.

**Retain option.** Keep temporal mappings Unsupported until their complete
bracket proof exists. Keep canonical group keys and the residual predicate
regardless of pushdown choice.

**Disposition for ticket 30.** Q35 must record the wording conflict, not
silently lift the reserved-node restriction. Counterexample: membership can
match a point and a month without making their group keys equal. Reject a bound
proof that ignores civil rows, period rows, quoted literals, or a required zone.
A later temporal-key execution proof remains an implementation gate.

## L8. Public wrappers hide most Jiff details, but one conversion exposes Jiff

**Current source [V].** The crate exports `DateValue`, `DateTimeValue`, and
`DurationValue` through its root. See `src/lib.rs:173-175`. Their Jiff fields
remain private. See `src/temporal/date.rs:43-45`,
`src/temporal/datetime.rs:76-78`, and `src/temporal/duration.rs:165-169`.
Explicit-zone operations use crate-local or narrower visibility. See
`src/temporal/datetime.rs:148-154,188-209`. Public date errors hold domain
categories and text, not `jiff::Error` fields. See
`src/temporal/error.rs:35-89`.

However, `impl From<jiff::civil::Date> for DateValue` is a public trait
implementation. See `src/temporal/date.rs:177-182`. A private module and a
private tuple field do not hide its argument type from external Rust callers. Do
not report complete public isolation from Jiff.

There is also an ambient public conversion from `DateValue` to `DateTimeValue`.
It resolves local midnight through `local_zone`. See
`src/temporal/datetime.rs:346-360`. This conversion exposes environmental
semantics, even though its signature contains no Jiff type.

**Inherited rule.** Ticket 16 makes the Rust library surface Free, but requires
justified outward visibility. See
[16:92-100](../../issues/16-grilling-compatibility-envelope.md). Ticket 19
removes ambient date-to-datetime promotion from the target relation path. See
[19:223-229](../../issues/19-coercion-and-comparison-policy.md). Ticket 35 owns
the final crate and public-surface cut.

**Candidate [P].** Retain Jiff behind temporal-owned values and frame
capabilities. Remove the outward Jiff conversion if no intended external
consumer needs it. Use the existing crate-local `from_civil_date` internally. Do
not expose `Timestamp`, `TimeZone`, or Jiff errors merely to wire the new frame.
Record any deliberate Jiff exposure as part of the crate contract.

**Retain option.** Keep the Jiff conversion if ticket 35 chooses Jiff
interoperability as an outward capability. Keep domain errors and private
payloads in either option. The current ambient midnight conversion cannot serve
the accepted context-explicit comparison target.

**Disposition for ticket 30.** Q12, Q19, and Q33 must distinguish use of Jiff
from public dependence on Jiff. Counterexample: replacing Jiff would break an
external caller that constructs `DateValue` through its Jiff `From`
implementation. Reject claims that private fields alone isolate the public API.
No external-consumer compatibility test or public-API check ran in this review.

## Integration result

This report covers the leakage lens through direct source inspection. The
findings identify decision and integration work. They do not prove the
proposed implementation. The main repairs concern the older `DatePoint`
claim, schema-check ownership, period-carrier ownership, and membership-key
wording. The parent can integrate these findings without changing completed
Answers.

Checks not run: builds, tests, linters, formatters, benchmarks, or public-API
checks. Remaining risk: candidate carriers and temporal bounds have no execution
or size evidence in this review.
