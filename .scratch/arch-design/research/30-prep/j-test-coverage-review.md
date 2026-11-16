# Temporal test coverage review: Rust-design lens 10

## Result and evidence limit

[V] The current tests cover many temporal operations. They do not establish the accepted civil-value, frame, period, or pushdown contracts.

This review reads assertions, not only test names. It separates current behavior from target acceptance tests.
I read [brief.md](./brief.md), [synthesis.md](./synthesis.md), and [h-redteam.md](./h-redteam.md).
I also read the accepted frame and period rules in [ticket 21](../../issues/21-ambient-environment-and-execution-context.md), lines 106-179.
I read the pushdown rules in [ticket 29](../../issues/29-query-core-and-plan.md), lines 498-612.

[V] CodeGraph supplied the code and test bodies below. I used LSP for the fuzz-helper reference query.
LSP returned only the declaration. The fuzz target itself contains a call. Thus, that reference result does not establish absence.
CodeGraph briefly flagged `src/template/engine/date.rs` as changed. Later node reads supplied its current test bodies.
All code locations below refer to those source reads. They are not test-run results.

I ran no commands, builds, tests, probes, linters, formatters, or benchmarks.
I changed no production code, test code, accepted Answer, or other research file.

Tags: [V] means source inspection. [H] means an inference. [P] means a proposed acceptance check.
No proposal below resolves a human choice.

## 1. Existing assertions and their limits

### Frozen parsing and surface boundaries

| Current assertion | What the assertion establishes | What it does not establish |
| --- | --- | --- |
| `date.rs:697-725`, `parses_supported_full_date_years`, `rejects_hostile_full_dates_with_existing_reason` | Leap day, year zero, negative years, invalid fields, and exact mapped reasons. | Parity of a replacement for speedate. |
| `point.rs:903-1093`, parse tables | Exact day/month precision; naive shapes resolve in UTC; explicit offsets select instants; invalid inputs have mapped reasons. | Zone-free naive values. These tests assert today's instant conversion. |
| `field.rs:1501-1559`, date-time coercion tests | Explicit ISO strings become Date or DateTime. Passthrough retains date text. | A civil datetime carrier or classification independence from the zone. |
| `note/metadata.rs:337-347`, `iso_date_value_parses_as_a_date` | YAML frontmatter produces a Date field. | A complete frozen classification corpus. |
| `note/parser/inline.rs:447-505`, date-atom tests | Valid dates, month forms, malformed dates, and attached text have specific atom outcomes. | Datetime atom parity with frontmatter. The inline date parser only reads date prefixes. |
| `query/grammar/filter.rs:1487-1538` | Bare date operators select named rows; quoted and bare forms agree in two cases; malformed bare shapes fail. | Target adaptive quoted literals. Agreement between two paths alone cannot prove either result correct. |

[V] The inline parser applies an atom-boundary check after its date prefix.
`T` is not a boundary. Whitespace, comma, and end-of-input are boundaries.
See `src/note/parser/inline.rs:218-229,264-276`.
Do not create a shared expected table that silently makes inline datetime classification match frontmatter.

[V] `NoteFieldValue::from(DatePoint)` drops written precision.
See `src/note/field.rs:162-172`.
The stored Date field has no month-precision marker.
A format hint such as `YYYY` cannot recover written precision from that stored value.

### Serialized civil dates, instants, and the absent civil datetime

| Current assertion | Exact coverage | Limit |
| --- | --- | --- |
| `date.rs:982-1036` | JSON spells `2026-07-29`; postcard bytes equal `b"\x0a2026-07-29"`; both decode to the date. | This is a civil Date, not a civil DateTime. |
| `datetime.rs:1520-1555` | JSON spells UTC with `Z`; postcard bytes equal `b"\x142026-07-29T14:30:05Z"`; decoding retains the instant. | No civil datetime exists in this carrier. |
| `datetime.rs:1562-1609,1628-1638` | Fractions have canonical widths; JSON retains a subsecond instant; negative years retain the sign. | No cross-zone store/read test for a naive civil datetime. |
| `index/codec.rs:235-308`, `note_row_round_trips` | Generated notes survive the actual row codec. | Its temporal generator emits only positive-year dates, with days 1 through 28. It emits no datetime or duration. |
| `note/field.rs:579-580` | `NoteFieldValue` stays at or below 80 bytes. | The current assertion cannot measure a proposed carrier. |

[V] Today's `DateTimeInput::LocalClock` resolves through `DateTimeValue::from_local`.
See `src/temporal/datetime.rs:622-658`.
A successful current postcard round trip does not prove the accepted civil-datetime persistence rule.

[P] Reuse the codec property test. Extend its generated temporal cases after the selected carrier exists.
Retain exact byte fixtures for unchanged Date and instant encodings.
An authorized format-epoch change does not need unchanged bytes.
Test cache rejection or rebuild at that epoch boundary. Do not reinterpret old naive-derived instants as recovered civil values.

### DatePoint, precision, and DurationValue

[V] `DatePoint` has Calendar and DateTime variants. Calendar stores Month or Day precision.
It derives `Eq` and `PartialEq`, not `Ord`.
See `src/temporal/point.rs:25-51`.

[V] Current DatePoint assertions check more than construction:

- `point.rs:1157-1200` checks prefix values, precision, and full byte spans.
- `point.rs:1231-1389` checks patterned parsing, ISO week dates, fractions, invalid weekdays, unsupported flags, and invalid directives.
- `point.rs:1726-1771` checks one-step month clamping and preservation of Month precision after a day shift.
- `point.rs:2182-2223` checks equal `1d` and `24h` values that apply differently across a New York gap.
- `point.rs:2227-2270` checks `1w`, `7d`, and `168h` across the same gap.
- `point.rs:2311-2342` checks equal `1 month` and `30 days` from 2026-01-15. Results are February 15 and February 14.
- `point.rs:3112-3192` checks 23-hour and 25-hour days, reverse differences, and zone-free date differences.

[V] `DurationValue::eq`, `cmp`, and `hash` use rounded nominal seconds.
See `src/temporal/duration.rs:562-589`.
The test name `compares_by_nominal_nanoseconds` does not describe that implementation.
Its assertions check `90m == 1h 30m`, `1d == 24h`, `1mo == 30d`, and signed zero.
See `duration.rs:1641-1658`.
Ordering and hashing check the same identities at `duration.rs:1667-1698`.

[V] Duration arithmetic tests assert unit-preserving text, nominal decomposition, rounding, and nonfinite errors.
See `duration.rs:1492-1631`.
These tests must not become only nominal-equality assertions during refactoring.
For example, equality cannot detect a change from `1mo` to `30d`, although calendar application can detect it.

[V] The duration serde regression asserts nominal equality and rejects unsupported subsecond unit spellings.
See `src/temporal/duration.rs:1761-1783`.
It does not compare the restored fractional fields. Rounding can hide fractional loss.
The duration fuzz target also uses nominal equality after display and parse.
See `fuzz/fuzz_targets/duration_value.rs:8-17`.

[V] The date fuzz target calls `date_point_display_law_holds`.
See `fuzz/fuzz_targets/date_point.rs:7-11` and `src/lib.rs:251-270`.
That helper checks only Day-precision Calendar values. It excludes Month points and all datetimes, and accepts documented range failures.
It does not establish general temporal round-trip coverage.

[V] The written-order test at `point.rs:1998-2014` uses `1w 2d`.
Both application orders produce August 7. The fixture cannot distinguish order.
The query fixture at `filter.rs:1953-1966` uses January 31 plus `1mo 1d`.
Month-first and day-first application both produce March 1 for that fixture.

[P] Use a distinguishing anchor when testing the approved arithmetic rule. Do not infer rule coverage from these test names.

### Moment tokens and render behavior

[V] The render tests already cover the full current token table and `Do`.
They compare rendered output with independent expected strings.
See `src/template/engine/date.rs:1393-1471`.
More assertions cover these cases:

- Unsupported maximal runs and exact token names: `date.rs:1474-1511`.
- Ordinal edges, including 11, 12, and 13: `date.rs:1514-1535`.
- Unsupported `Q` and a supported-token diagnosis: `date.rs:1539-1550`.
- Percent signs inside bracket literals and strftime selection outside brackets: `date.rs:1552-1580`.
- Unclosed brackets and error precedence: `date.rs:1584-1607`.
- Apia's skipped 2011-12-30 renders as `31st|31`: `date.rs:1640-1654`.

[V] The translator's structural test checks that table entries repeat one byte.
See `src/temporal/token.rs:224-235`.
That assertion protects `match_moment_run`'s precondition. It is not a rendering oracle, but it is not a tautology.

[V] `format_bindings_never_emit_percent_plus` checks one sample translation.
See `src/template/engine/date.rs:1612-1619`.
It does not prove the full dialect contract. Retain the rendered-output tables as the primary contract.

[P] Move or reuse these tables if the translator moves. Do not add a second table-derived oracle.
For newly selected tokens, add independent rendered examples. For rejected tokens, assert the typed error and token.
Do not add `Q`, `Qo`, `ww`, or `DDDD` success expectations before the human selects their scope.

### Gaps, folds, and unavailable zones

[V] Current resolution tests cover New York, London, Lord Howe, Sydney, a synthetic midnight gap, and UTC.
They assert the earlier fold occurrence and forward gap adjustment.
See `src/temporal/datetime.rs:873-1063`.
The Lord Howe tests distinguish a 30-minute transition from a one-hour transition.

[V] The shared fixtures use explicit POSIX rules for New York, London, Lord Howe, and the midnight gap.
They do not consult the host zone database.
See `src/temporal/mod.rs:224-260`.
Reuse these fixtures for arithmetic checks. Use named historical zones for historical skipped-day checks.

[V] `zone.rs:93-106` passes a failing closure to the private resolver. It asserts `DateError::LocalZoneLookup` exactly.
This checks error mapping. It does not exercise a failed zone lookup through a query or render.

[V] Template translation tests preserve a constructed zone error in the source chain and avoid duplicate lookup prefixes.
See `src/template/engine/date.rs:2400-2445`.
They do not inject zone unavailability into the public render path.

[V] `zone_resolution_errors_precede_pattern_errors` causes timestamp range failure with `Etc/GMT-14`.
It does not cause system-zone lookup failure.
See `src/template/engine/date.rs:1658-1677`.
Do not cite this test as unavailable-zone coverage.

[V] Query calendar operations call `local_zone().ok()?` and return `Option` today.
See `src/query/grammar/filter.rs:636-646,652-699,727-760`.
An error-mapper unit test cannot establish the accepted typed operation-failure contract through that path.

### Prefix byte extents

[V] `lexer.rs:157-179` checks day/month prefixes, signed years, leading spaces, and invalid starts.
It asserts the consumed count and Calendar variant, but not the parsed date.
`point.rs:1119-1200` supplies the stronger value-plus-precision-plus-span assertions.

[V] Duration-prefix tests assert values and extents at `duration.rs:1434-1483`.
They cover packed components, comma stops, leading signs, multibyte text, and attached symbols.
`lexer.rs:196-219` adds character-boundary checks and rejects incomplete components, internal signs, and Unicode separators.

[V] Current `DatePoint::parse_prefix` accepts `2026-0729` as a month prefix with extent 7.
See `point.rs:1151-1178`.
Do not impose a new filename grammar on that frozen shared parser.
The new filename parser needs its own boundary tests.

### Frames and pushdown

[V] The template clock tests check output shape and day relationships.
See `src/template/engine/date.rs:959-994,1061-1092`.
The comment claims one clock read per template. The assertion does not count reads or force a midnight crossing.

`Anchor::base` calls `DateTimeValue::now()` or `DateValue::today()` per anchor at `date.rs:245-254`.
`Anchor::render` calls `self.base()` when no reference exists at `date.rs:327-335`.
Do not treat this test as proof of a pinned render frame.

[V] `current_timestamp_is_neither_past_nor_future` asserts only `!past || !future`.
See `date.rs:2301-2312`.
That means “not both true.” It does not assert that both are false.
It also formats a live timestamp without its offset before calling the helpers.

[V] Today's `ExecutionPlan` fuses filters, sorts, and sort-limit pairs.
See `src/query/plan.rs:1-37,133-152`.
The inspected plan has no temporal candidate-bound protocol.
The query/temporal search found no frame or residual-pushdown implementation under the searched identifiers.
This is a scoped source observation, not a whole-repository absence proof.

[V] Existing query tests assert exact matching rows for date operators, duration comparisons, and month arithmetic.
See `src/query/grammar/filter.rs:1487-1499,1629-1647,1953-2019`.
They are useful residual-evaluator fixtures. They do not test candidate-set soundness.

[V] CLI scenarios also assert exact output for nanosecond instant equality and calendar-versus-elapsed arithmetic.
See `tests/features/temporal/temporal-dates.feature:18-25,45-82`.
The first scenario, lines 4-16, asserts the current naive-to-instant behavior.
Migrate that expected behavior explicitly when the accepted civil-value rule lands.

## 2. Ranked meaningful gaps

These ranks express correctness risk, not measured cost or implementation order.

1. **Failed or repeated environment reads can escape operation ownership.**
   Current error tests stop at the mapper. Current clock tests do not force different samples.
   Target: the public query, render, and lazy-result seams must retain one frame and a typed failure.
2. **Serialization checks can miss temporal information loss.**
   No current civil-datetime carrier exists. Nominal duration equality hides component and fractional changes.
   Target: decoded kind, exact civil fields or instant, and duration components must survive the real row codec.
3. **A precision marker can become a false period invariant.**
   A Month-precision DatePoint can shift to July 2. Stored fields drop that precision.
   Target: validate period starts separately. Never infer canonical periods from a DatePoint precision marker.
4. **Filename periods have no current acceptance corpus.**
   Current `FileMeta::day` tests assert day-prefix precedence and creation-time fallback.
   See `src/file.rs:482-538`. The target removes that fallback and adds week, quarter, and optional year forms.
5. **Mixed temporal comparison and candidate pruning lack target checks.**
   Current sort tests promote a Date to midnight; current comparison tests also encode that model.
   See `src/query/sort.rs:842-875` and `src/note/field.rs:996-1025,1040-1054`.
   Target filter relations, grouping keys, total sort keys, and candidate bounds need separate oracles.

[P] These gaps need behavioral tests, not new production abstractions for testing alone.
No production-code deletion follows from this review. Reuse the existing fixtures and test seams where they fit.

## 3. Precise acceptance matrices

All rows below are [P]. “Target” means absent target coverage, not an assertion that current code passes.

### Matrix A: frozen parsing and storage

Run each applicable case through the existing parser seam and the real note-row codec.
Record the expected variant, value, fallback text, error kind, reason, and byte extent where the API supplies one.
Keep frontmatter and inline expectations separate.

| Case family | Inputs or construction | Acceptance oracle |
| --- | --- | --- |
| Full dates | `2024-02-29`, `0000-02-29`, `-2024-02-29`, minimum and maximum years | Same accepted date fields before and after parser replacement. |
| Invalid dates | `2026-02-29`, month `00`/`13`, day `00`, `2026-07-2x` | Same rejection kind, reason, and fallback behavior at each seam. |
| Shape boundaries | `2026-1-1`, `20260729`, `+2026-07-29`, short/long years, trailing text | No acceptance expansion from a broader upstream parser. |
| Existing month precision | `2026-07` and `2026-07-01` | Current DatePoint distinction stays explicit; stored Date conversion still has its documented behavior. |
| Numeric and text boundaries | Unquoted `2026`, `2`, `152`; quoted forms; `next week`; malformed date text | Preserve each note surface's current classification. Do not introduce natural-language note typing. |
| Authorized naive change | `2026-07-29T14:30`, seconds, fractions, and space separator | Target civil kind and exact fields survive storage without any zone lookup. Record the deliberate migration. |
| Explicit instants | Same instant with `Z`, `+02:00`, and a negative offset; instants one nanosecond apart | Same exact instant and nanoseconds after storage. Distinct instants remain distinct. |
| Cross-zone read | Serialize civil and instant values under zone A; decode under zone B or unavailable zone | Civil fields and stored instants stay unchanged. Only an explicit presentation projection may differ. |
| Format epoch | Old cache bytes, current bytes, corrupted payload | Apply the selected epoch/rebuild rule. Never silently accept a wrong temporal kind. |
| Value budget | Selected carrier inside `NoteFieldValue`, including List/Object cases | Keep the existing `<= 80` byte assertion. Measure candidate layouts separately. |

[P] Reuse `date.rs` strict cases, `point.rs` parsing tables, `field.rs` coercion tests, and `index/codec.rs` round-trip strategies.
Do not compare only two parsers that both call the same replacement validator.
For parser parity, use the current implementation as the baseline and independent expected calendar fields for representative cases.

### Matrix B: identity, arithmetic, periods, and order

| Subject | Fixture | Acceptance oracle |
| --- | --- | --- |
| Application order | 2026-01-30 with a month then a day, versus a day then a month | Month-first gives March 1; day-first gives February 28. Assert the approved rule, not only a compound result. |
| Nominal duration identity | `1mo`/`30d`, `1d`/`24h`, `1h 30m`/`90m` | If Q23 retains the rule, Eq, order, and hash retain those identities. This does not authorize arithmetic substitution. |
| Calendar application | 2026-01-15 plus `1mo` versus `30d`; New York gap plus `1d` versus `24h` | Preserve the distinct dates and clock readings already asserted by DatePoint tests. |
| Duration round trip | `1mo`, `30d`, `1d`, `24h`, `0.000000001s`, `0.001s`, `1.5s`, signed compounds | Compare stored components and exact fractional fields, not only DurationValue Eq. Apply restored calendar durations at a distinguishing anchor. |
| Point precision | Month-precision July 1 shifted by one day | Result remains a point on July 2. Do not accept it as a canonical Month period start. |
| Target period construction | Day, Week, Month, Quarter, Year; valid starts and one-day misaligned starts | Constructor rejects or normalizes according to the approved rule. The resulting period always has a canonical start. |
| Target period equality | Same start and unit; same start with different units; overlapping periods | Only matching normalized `(start, unit)` pairs are equal. Overlap and containment never define equality. |
| Target mixed sorting | Civil values, instants, and periods in every input permutation | Use the accepted Civil, Instant, Period class order. Check antisymmetry, transitivity, and deterministic group order. |
| Target period unit tie-break | Equal starts with all five units | Use the human-approved fixed rank. Do not rank by nominal duration or written precision. |

[P] Equal nominal durations need not stay substitutive under calendar addition. Current tests deliberately show the opposite.
Do not use period membership as a Rust Eq, hash, grouping, or total-order test.

**Repair the “no-cross-class-Ord test” recommendation.**
`Ord` compares a type, not selected enum variants.
A runtime test cannot prove that a trait implementation is absent.
Do not inspect source text for `derive(Ord)` or replace the check with two unequal sample values.

If the selected API prohibits `Ord`, use the project's compile-time contract test mechanism, when available.
A compile-fail check must fail specifically because `Ord` is unavailable, not because an import or constructor is wrong.
Separately test the actual filter relation and total sort key through their intended seams.
The present `DatePoint` already lacks `Ord`; adding an absence test there would not prove the proposed inner carrier's contract.

### Matrix C: zone-sensitive boundaries

The period rows use civil-date membership. They must not construct the next day's midnight.
Ordinary Date-versus-instant equality remains a separate rule.

| Fixture | Target acceptance |
| --- | --- |
| New York 2026-03-08, civil 02:30 and 03:00; instants 07:00Z and 07:30Z | Civil values remain civil. If Q21 selects instant projection, compare the projected civil fields, not a shifted gap value. |
| New York 2026-11-01, 05:30Z and 06:30Z | Both project to 01:30. They remain distinct instants and retain instant order. Both belong to that civil day. |
| Lord Howe half-hour gap and fold | Preserve the existing exact transition checks. Do not assume every change is one hour. |
| Sao Paulo 2015-10-18, first existing time 01:00 | Under Q21's proposed projection, no instant equals that Date's civil midnight. Day membership can still match later times. |
| Apia 2011-12-30 and the preceding valid day | The skipped Day period matches no instant. The preceding valid day works without resolving the skipped next midnight. |
| Any period boundary with representable adjacent instants | Start belongs; instant immediately before start does not. Last included instant belongs; next start does not. |
| Required zone unavailable | Query materialization and rendering return the typed failure. Do not return Null, omit rows, use UTC, or emit successful partial output. |
| Stored civil-only work with unavailable zone | Parsing, storage, civil comparison, and civil membership succeed without resolver calls. |

[P] Keep current gap/fold tests as explicit conversion tests where that conversion remains supported.
Do not retain naive parse-to-instant assertions under a renamed civil API.
Do not use the Apia render expectation as the target civil-Date rendering rule without a human decision about that surface.

### Matrix D: new filename prefixes and old byte extents

| Subject | Cases | Acceptance oracle |
| --- | --- | --- |
| Existing prefix API | Full/month dates; signed years; leading Unicode whitespace; multibyte suffix | Assert the actual date, precision, exact byte span, and UTF-8 boundary. Preserve existing accepted shapes. |
| Existing duration prefix | Packed and spaced parts, comma, `é`, `°C`, incomplete next part, internal sign | Reuse existing expected values and extents. A scanner replacement must match both. |
| Target filename day/month | Existing valid stems and malformed longer dates such as `2026-07-2` or `2026-02-30` | Valid forms yield the intended period. Malformed longer forms never fall back to Month or Year. |
| Target ISO weeks | `2020-W53`, `2021-W53`, week `00`, week `54`, year crossings | 2020-W53 starts 2020-12-28. Reject nonexistent 2021-W53 and invalid week numbers without rollover. |
| Target quarters | Approved spellings for Q1 through Q4, Q0, Q5, and malformed suffixes | Exact quarter starts; no rollover; exact consumed-byte and suffix rules. |
| Target bare year | Opt-in off/on; `2026.md`; `2019 budgets.md` | Assert the selected whole-stem or prefix policy. Do not select that policy in the test design. |
| Target grammar boundaries | `W1`/`W01`, `w`/`W`, `q`/`Q`, separators, attached letters/digits, non-ASCII tails | A decision table gives every form an explicit accept/reject result and extent. |
| Target extraction seam | No date prefix with a valid creation timestamp | `file.date` is Null. Never use the current `file.day` creation fallback. |
| Custom week choice | Year-edge dates under each approved preset or pair | Add independently specified week-year and start-date examples only after the week policy is approved. |

[V] ISO week prefix meaning already appears in ticket 21:170-177.
Any proposal to reinterpret those prefixes under custom weeks needs explicit reconciliation.
A passing test for a proposed preset cannot settle that decision.

### Matrix E: operation frame ownership

Use deterministic clock and resolver inputs. Count reads at the adapter boundary, not by scanning source text.

| Operation | Adversarial input | Target acceptance |
| --- | --- | --- |
| One render with nested queries | Clock source offers samples on opposite sides of local midnight | All helpers and nested queries use the first operation sample and the same week rule. |
| Reusable expression, next operation | Second frame has a later date and a different policy | The new operation resolves again. It does not reuse the earlier operation's relative values. |
| Lazy QuerySet after render exit | Advance the clock and change the ambient test override before terminal access | The set retains its creator's frame and resolved plan. |
| First required zone lookup | Resolver offers zone A, then zone B | One operation retains zone A. Repeated row and helper evaluations make no second resolver call. |
| Failed first lookup | Resolver first fails, then offers UTC | The operation retains the original typed failure. Repeated terminal access does not retry or silently succeed. |
| Zone-free operation | Resolver would fail if called | Zero calls. Civil-only work succeeds. |
| Independent roots and requests | Interleave frames with distinct zones and week rules | No cross-frame state transfer. Add concurrent access if the selected API supports cross-thread sharing. |
| Past/future equality boundary | Fixed instant exactly equal to the pinned sample, then one nanosecond before/after | Assert the approved exact booleans. Do not use `!past || !future` as an equality oracle. |

[P] Reuse the existing explicit-zone fixtures and render helper.
Replace ambient-clock shape checks with exact fixed-sample checks after injection exists.
Do not simulate pinning by returning the same value from every source read. That would hide repeated reads.

### Matrix F: temporal pushdown

Ticket 29:506-550 defines set equality for Exact and containment for Inexact.
Ticket 29:571-597 limits the first implementation and leaves unproved mappings Unsupported.
The matrix must preserve those distinctions.

| Axis | Cases and required assertions |
| --- | --- |
| Kinds | Date, civil DateTime, instant, Period, Duration; Null, missing, text, and list controls. |
| Operators | `==`, `<`, `<=`, `>`, `>=`, separately. Include equal values and immediate neighbors around each bound. |
| Oracle | Write expected row ids independently. The scan must match them. Then compare candidates and final pushed results with that same truth set. |
| Exact | Candidate row ids equal the independent match set for the atom, frame, generation, and row mode. |
| Inexact | Candidates contain every independent match. Residual evaluation removes false positives. First-scope execution still scans unproved or merely Inexact cases. |
| Unsupported | Quoted/adaptive literals, `!=`, functions, arithmetic, field-versus-field forms, and unproved mappings use the scan path. |
| Effective values | Frontmatter beats inline, including frontmatter Null. Repeated inline values become a list, not several scalar keys. |
| Modes | Metadata pruning applies only to Pages. Task/List item overrides cannot disappear behind note-level metadata keys. |
| Period and zone edges | Include Matrix C, equal-start different-unit periods, and point-versus-period matching. Keep membership bounds separate from group/range key identity. |
| Residual execution | Use an observable predicate-evaluation counter or existing executor test seam. Assert residual execution even for an Exact candidate source. |
| Diagnostics | Scan and pushed paths emit equal notices under source restrictions, And, Limit, row modes, and buffer edits. |
| Initial Boolean scope | Test supported And intersection. Or, Not, and quoted forms still scan in the first implementation. |

[P] Do not infer temporal-key correctness from numeric-key timings.
Do not prove Exact by comparing a key against its own encoder.
Do not compare only pushed output with a scan that shares the same incorrect temporal conversion.
Include independent boundary answers and distinct expected notices.
Resolve the membership-bound versus range-key wording before claiming an Exact period mapping.

## 4. Probes now versus implementation gates

No probe in this table ran during this review.

| Work | Can close a decision gap now? | What it can establish |
| --- | --- | --- |
| Current parser versus candidate speedate replacement | Yes, in an isolated parent-run probe. | Compare accepted values and mapped failures under the frozen shape guard. Include Matrix A; raw upstream acceptance alone is insufficient. |
| Current prefix scanner versus a candidate scanner | Yes, in an isolated parent-run probe. | Compare values, rejection, and exact byte extents from Matrix D. This informs Q10 without production edits. |
| Jiff gap/fold and projection examples | Yes, in an isolated parent-run probe. | Confirm concrete counterexamples and projection results. This cannot select ordinary equality semantics for the human. |
| Candidate civil/instant layouts | Yes, in an isolated parent-run layout probe. | Measure candidate carrier sizes inside NoteFieldValue. The present 80-byte assertion cannot choose a representation. |
| Selected crate grammar cases | Yes, for rejection evidence. | Run the existing d-file T1-T10 cases. Distinguish span, anchor, period, trailing-input, and non-ASCII behavior. |
| New filename grammar | Partly. | A prototype can expose ambiguous boundaries. Complete acceptance needs the approved spelling, week rule, and bare-year policy. |
| Civil persistence, epoch invalidation, and public errors | Only after implementation. | Exercise the selected carrier, real codec, cache admission, and fallible public seams. |
| Pinned frames and escaping lazy sets | Only after implementation. | Exercise operation ownership, resolver read counts, retained failures, and new-operation refresh. |
| Exact temporal bounds and whole-process benefit | Only after key implementation. | Prove candidate sets, residual execution, notices, and measured benefit with the optimization enabled and disabled. |

[V] The crate report lists its unrun differential probe at `d-crate-evidence.md:161-209,428`.
The current strict-date tests provide reusable fixtures. They do not supply a differential result.

## 5. Lens-10 conclusion

[P] Reuse strong value, extent, rendered-output, codec, and CLI fixtures.
Add independent expected outcomes where current tests compare two paths with shared logic.
Do not replace duration components with nominal equality, point precision with period normalization, or filter equality with sort order.

The current source supports these coverage findings. It does not prove that the target design passes.
Unexecuted probes, unselected grammar rules, and absent target seams remain explicit limits.
