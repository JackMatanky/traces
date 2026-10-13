# Temporal Divergence Register

## Redesign Behavior-Change Register (`R1 to R17`)

This section is the **allow-list for Stage 2 oracle diffs**. Stage 1 of the
temporal redesign is behavior-preserving: the migration oracle
(`src/temporal_oracle.rs`) must show a **zero diff** after every Stage 1
commit. Stage 2 (the engine swap) may change behavior only as listed below,
and **every oracle diff line must carry a register tag**; any diff line
outside `R1` to `R17` is a defect and triggers the Stage 2 drop condition.

Rows `R1` to `R15` come from
`.design/2026-10-12-review-temporal/redesign-plan.md` (section 6); `R16` and
`R17` from `implementation-plan.md` (section 7). Decisions `D1` to `D10` stand
at their defaults, so every row below is `accept` (or `keep`) unless noted.
Case counts
are measured from `tests/snapshots/temporal_oracle.snap`: 2,347 case lines,
713 of which carry a register tag. A line may carry more than one tag, so
per-row counts overlap.

### `R1`: One-Step Calendar Application

- **Today**: Calendar units are applied in written order, so
  `2026-01-31 + "1mo 1mo"` yields `2026-03-28` (two sequential month steps).
- **After**: One step, largest unit first: the same shift yields `2026-03-31`.
- **Recommendation**: accept (`D4`; rejecting `D4` removes this row).
- **Oracle cases**: 116 lines tagged `R1,R3` — the `apply` rows for mixed and
  repeated calendar durations (`1mo 1mo`, `1mo 1d`, `1d 1h`, `1h 1d`,
  `1d 12h`) in the UTC date-point group and in every zone group.

### `R2`: Repeated Units Become a Parse Error

- **Today**: Date-only arithmetic restarts the clock per component, so
  `2026-07-29 + "12h 12h"` does not move at all (each `12h` lands back on
  midnight).
- **After**: A repeated unit is a parse error; `24h` and `36h` still move one
  day as before.
- **Recommendation**: accept.
- **Oracle cases**: 14 lines tagged `R2,R3` — the `apply ... + 12h 12h` rows
  in the UTC date-point group and in every zone group.

### `R3`: Duration Grammar Narrowed to Jiff Friendly and ISO

- **Today**: The friendly grammar is permissive: fractions anywhere
  (`.5h`, `1.5d`, `1.5mo`, `0.5h 15m`), exponents (`1e3s`, `1E-2s`), any unit
  order (`1d 1mo`), an inner sign (`1h +30m`), wrong case (`1H`, `1 H`, `3
  Hours`, `2 MINUTES`), a comma without a space (`1h,30m`), long aliases
  (`4 yrs, 6 wks`), and a separate hand-parsed ISO grammar in the template
  engine.
- **After**: One grammar, jiff friendly plus ISO: fractions only on the lowest
  unit and only hour or smaller, units descending and not repeated, no
  exponents, no inner sign, ISO forms validated by `Span` (`P1.5D` and `P-1M`
  rejected; `P1D` and `PT90M` accepted by `DurationValue` itself).
- **Recommendation**: accept (`D2`, default `G1`).
- **Oracle cases**: 207 lines tagged with `R3` — the duration parse
  accept/reject rows for the forms above (including `P1D`, `PT90M`, `1µs`,
  `1us`, `1ns`), all 18 duration prefix rows, the duration `from json` rows,
  and the `R1`/`R2`/`R4`/`R15` apply and operation rows that share the tag.

### `R4`: Canonical Duration Spelling

- **Today**: The source spelling is kept: `4 yrs, 6 wks` renders as written,
  and `as_str` exposes it.
- **After**: Canonical spelling (`4y 6w`); `as_str` on durations becomes
  `Display`.
- **Recommendation**: accept (`D3`).
- **Oracle cases**: 65 lines tagged with `R4` — the remaining duration parse
  rows, the six `negate` rows, all ten add/subtract pairs (`R4,R16`), and the
  six `to json` rows.

### `R5`: Integer Nanoseconds Instead of F64 Seconds

- **Today**: A duration is f64 seconds, so non-finite states exist and
  scientific spellings parse and display (`1e400s`, `9e15s`, `0.0005s`).
- **After**: Integer nanoseconds; range about 20 thousand years; no
  non-finite state. `7304484d` and `7304485d` become the accepted and
  rejected range edges.
- **Recommendation**: accept.
- **Oracle cases**: 6 lines tagged `R5` — `9e15s`, `1e400s`, `100000000y`,
  `0.0005s`, `7304484d`, `7304485d` in the duration parse group.

### `R6`: Rejecting Dates Without Padding

- **Today**: `FromStr` accepts dates without padding, such as `2026-1-1` and
  `2026-7-29`.
- **After**: Rejected. Pattern matching with `strptime` stays lenient
  (`%Y-%m-%d` still matches `2026-7-9`), and `YYYY-MM` stays a custom shape.
- **Recommendation**: accept (`D5`; rejecting `D5` removes this row).
- **Oracle cases**: 19 lines tagged `R6` — `2026-1-1` and `2026-7-29` in
  `date value parse`, `date from json`, `point`, `point prefix`, and the
  per-zone `date-time` parse rows.

### `R7`: Calendar-Aware Day Difference Across DST

- **Today**: A day difference across a DST transition counts exact elapsed
  time, so the 23-hour day returns `0.958`.
- **After**: Calendar-aware: the same difference returns `1`.
- **Recommendation**: accept (`D6`; rejecting `D6` removes this row).
- **Oracle cases**: 15 lines tagged `R7` — `diff ... in d` rows where both
  ends are date-times, plus the `2026-03-07` day diff in the zone-free group.

### `R8`: Fold and Gap Doctrine Kept and Pinned

- **Today**: A repeated local time (fold) resolves to the earlier instant and
  a skipped local time (gap) shifts forward by the gap length (compatible
  disambiguation), verified on New York.
- **After**: Unchanged; pins are added for at least 7 zones.
- **Recommendation**: keep; add 7-zone pins.
- **Oracle cases**: 301 lines tagged `R8` — `date-time` parse, `date-time from
  json`, and `promote` rows in every zone group.

### `R9`: Strftime %Z Prints the Abbreviation

- **Today**: `strftime %Z` prints a numeric offset and `%+` is supported.
- **After**: `%Z` prints the zone abbreviation; `%+` is unsupported (jiff
  `strtime` returns an error for unknown specifiers).
- **Recommendation**: accept.
- **Oracle cases**: 14 lines tagged `R9` — the `strtime`-dependent pattern
  rows (`%s` and `%Q`) in each zone group. `%Z` and `%+` are not in the
  pattern corpus yet; their diffs belong to the template rows in `S2.4`.

### `R10`: Malformed `ISO-8601` Durations Rejected

- **Today**: The template engine's hand-written ISO translator accepts
  malformed forms such as `P-1M` and `P1.5D`.
- **After**: Rejected; ISO text is parsed by `Span`, which refuses them.
- **Recommendation**: accept.
- **Oracle cases**: none tagged yet — the translator has no oracle probe, so
  the change surfaces in the template engine rows (`S2.4`); tag any resulting
  diff `R10`.

### `R11`: Public Error Variant Sets Change

- **Today**: `DurationError` has six variants and `DateError` five.
- **After**: New variant sets, and the messages change (the oracle prints
  variant names, never message text).
- **Recommendation**: accept.
- **Oracle cases**: none tagged yet — variant names print on the rejection
  and error rows of the duration, date, point, and date-time groups; those rows
  must carry `R11` when Stage 2 changes the variant sets.

### `R12`: `ZoneGuard` Replaces `TzGuard`

- **Today**: `TzGuard` mutates the `TZ` environment variable under a global
  mutex using `unsafe`.
- **After**: `ZoneGuard` overrides the Jiff time zone for the current test
  thread and serializes guard lifetimes. It does not mutate `TZ` or use
  `unsafe`.
- **Recommendation**: accept.
- **Oracle cases**: the adapter uses `ZoneGuard` (`S2.1`). The snapshot now
  records each declared zone explicitly; the previous `TZ`-based run left
  cross-zone rows with stale offsets.

### `R13`: Explicit `LocalZoneLookup` Error Kept

- **Today**: `DateError::LocalZoneLookup` is produced when the chrono local
  zone lookup fails.
- **After**: A failing system-zone lookup maps to the same explicit
  `LocalZoneLookup` error (no UTC fallback).
- **Recommendation**: accept (`D8`; rejecting `D8` removes this row and the
  variant).
- **Oracle cases**: none tagged — no oracle probe forces a zone-lookup
  failure.

### `R14`: Date Range Limited to Four-Digit Years

- **Today**: Dates span up to year 262k, far beyond the four-digit-year rule.
- **After**: `-9999..=9999`, matching the four-digit rule; shifts past either
  edge report out of range.
- **Recommendation**: accept.
- **Oracle cases**: 62 lines tagged `R14` — `0000-01-01` and `9999-12-31`
  parses, the range-edge shifts (`9999-12-31 + 1d`, `i64::MAX`, `300000y`,
  `5000000mo`, and the per-zone year-edge shifts), and the `100y` / `-8000y`
  apply rows.

### `R15`: Fractional Multiplier Yields Nominal Decomposition

- **Today**: A float multiplier keeps the written components.
- **After**: An integral factor keeps units; a fractional factor yields the
  nominal decomposition (`1d * 0.5` becomes `12h`).
- **Recommendation**: accept (`D9`).
- **Oracle cases**: 70 lines tagged `R15` — all `duration * factor` rows
  (6 durations by 7 factors = 42 lines) plus the `1.5d` and `0.5mo` apply rows
  (28 lines).

### `R16`: `Fieldwise` Arithmetic Spelling

- **Today**: Arithmetic on fixed-regime durations re-synthesizes a canonical
  spelling: `23h + 1h` shows `1d`, a 24-hour day.
- **After**: `Fieldwise` through the unit table: `23h + 1h` is `24h`. Behavior
  is identical (exact 24 hours); only the spelling differs.
- **Recommendation**: accept (`D14`).
- **Oracle cases**: 20 lines tagged `R4,R16` — the ten duration add/subtract
  pairs (`23h 1h`, `1d 1h`, `1h 30m 30m`, `1mo 1d`, `2d 1d`, `1d 2d`, `1h
  1h`, `1w 1d`, `90m 1h 30m`, `1y 1mo`), each with `+` and `-`.

### `R17`: Humanize Rounds the Whole Magnitude Before Decomposition

- **Today** (from commit `S1.4`): `humanize` rounds the total magnitude to whole
  milliseconds before decomposing, so a fraction in `[0.9995, 1)` of the last
  human unit rolls into it: `1.9999s` renders `"2 seconds"` and `59.9999s`
  renders `"1 minute"`.
- **Before commit `S1.4`**: The whole units were floored and only the tail rounded, so
  the same magnitudes rendered `"1 second, 1000 milliseconds"` and
  `"59 seconds, 1000 milliseconds"`.
- **Scope**: display only; value identity and canonical spelling are
  unchanged. Every magnitude with a sub-second fraction below `0.9995`
  renders exactly as before.
- **Recommendation**: accept.
- **Oracle cases**: none — no corpus row has a fraction in the roll-up range,
  which is why the `S1.4` oracle diff stayed zero. Pinned by
  `rolls_a_near_whole_magnitude_into_the_next_unit` in
  `src/temporal/duration.rs`.

---

## Divergences from External Systems

The entries below predate the redesign: they document intentional
architectural and behavioral divergences between Traces and external systems
(Dataview, `Templater`, SQL engines, `Luxon`, and Temporal), citing design
decisions and the underlying research corpus:

- [SQL Temporal Conventions](.scratch/temporal-core/research/sql-temporal-conventions.md)
- [General Temporal Libraries](.scratch/temporal-core/research/general-temporal-libraries.md)
- [Rust Temporal Ecosystem](.scratch/temporal-core/research/rust-temporal-ecosystem.md)

### Entry 1: UTC Storage of Instants

- **Divergence**: Traces stores all date-time instants as UTC in index and memory, while rendering human displays as the local clock time.
- **External Behavior**: Dataview and `Templater` often preserve local timezone offsets or arbitrary strings in memory, leading to offset mismatches across machines or synchronization boundaries.
- **Rationale**: Eliminates cross-platform timezone comparison defects while maintaining human-intuitive local displays.
- **Specification**: Spec Story 12, Decision `D9`.

### Entry 2: Space-Separated `DateTime` Acceptance

- **Divergence**: Traces accepts space-separated date-time strings (`2026-07-29 14:30:00`) in frontmatter and query literals alongside standard ISO-8601 `T`-separated strings.
- **External Behavior**: Strict ISO-8601 parsers reject space-separated dates.
- **Rationale**: User frontmatter and Markdown vaults frequently use space-separated dates for readability; Traces accommodates common authoring conventions without compromising parse safety.
- **Specification**: Spec Story 14, `TextShape` recognition.

### Entry 3: Dual Regime for Duration Identity Vs. Application (`Luxon` Parity)

- **Divergence**: Duration equality and hashing compare canonical total seconds (`1d == 24h`, `1mo == 30d`), but duration application treats calendar units (`d`, `w`, `mo`, `y`) as calendar units on the calendar clock.
- **External Behavior**: Strict physical duration models treat all units as fixed seconds; pure calendar models reject cross-unit equality.
- **Rationale**: Follows the dual-regime convention of `Luxon`: users expect `1d == 24h` in numeric comparisons, but expect adding `1d` across a Daylight Saving Time shift to preserve clock time rather than advancing 24 physical hours.
- **Specification**: Spec Decision `A2′`, Decision `D13`; backed by Seam 1 test pins.

### Entry 4: Explicit `NoteFieldType` Rank

- **Divergence**: Field comparisons between mismatched types follow an explicit, type-safe total ordering hierarchy (`Null < Bool < Number < String < Date < DateTime < Duration < Link < List < Object`) rather than JavaScript's dynamic coercion or alphabetical type-tag sorting.
- **External Behavior**: Dataview sorts types by string representation or dynamic JavaScript ordering rules.
- **Rationale**: Guarantees deterministic, reproducible sort keys and query results across platforms.
- **Specification**: Spec Story 42, `NoteFieldValueRef::compare`.

### Entry 5: Canonical Duration Equality

- **Divergence**: Durations compare equal based on total normalized magnitude in seconds, independent of input spelling (`1h 30m == 90m`).
- **External Behavior**: Text-based or AST-based systems distinguish spelling variations as unequal.
- **Rationale**: Users expect semantic equality between equivalent duration expressions.
- **Specification**: Spec Story 42, `DurationValue::eq`.

### Entry 6: Null Ordering in Sort and Filter (`ADR` 0009)

- **Divergence**: `Null` ranks below all non-null values in sorting (nulls place first in `ASC`, last in `DESC`), and `Null` never satisfies any ordered comparison (`<`, `<=`, `>`, `>=`) in filter expressions.
- **External Behavior**: SQL engines diverge widely: PostgreSQL defaults to `NULLS LAST` in `ASC`; SQLite puts nulls first; `Trino` and `DuckDB` put nulls last. Dataview implicitly coerces nulls to falsy values.
- **Rationale**: Prevents missing or optional fields from inadvertently matching inequality filters while clustering missing records predictably at list boundaries.
- **Specification**: `ADR` 0009, Spec Decision `D15`.

### Entry 7: ISO Weeks Pinned to Monday

- **Divergence**: Week starts and ISO week numbering are pinned strictly to ISO Monday across all platforms, templates, and queries, with no `CLDR` locale dependency.
- **External Behavior**: `Moment.js`, `Templater`, and standard locale libraries vary the start of the week (Sunday vs. Monday) based on the host OS locale.
- **Rationale**: Eliminates environment-dependent query divergence where the same vault produces different weekly buckets on machines with different system locales.
- **Specification**: Spec Story 4, Week mandate `L109`.

### Entry 8: Local-Naive Parse with Deterministic DST Policy

- **Divergence**: Naive datetime inputs are interpreted in the local system timezone and resolved to UTC via an explicit DST resolver. Ambiguous fall-back times resolve to the earliest occurrence, spring-forward gaps advance across the gap, and missing system zone information produces an explicit error.
- **External Behavior**: Chrono and standard libraries often reject ambiguous or nonexistent times outright or panic.
- **Rationale**: Replicates Temporal `'compatible'` / RFC 5545 behavior so that user notes written across DST transitions never fail to load.
- **Specification**: Spec Decision `D14`; `local_minus_utc` resolution.

### Entry 9: Strictness-by-Seam

- **Divergence**: Input parsing strictness depends explicitly on the operational seam: note and template inputs are lenient with actionable error diagnostics; `serde` deserialization is strict; query filters enforce typed evaluations without silent coercion.
- **External Behavior**: Many engines enforce either blanket leniency (silent null fallbacks) or blanket strictness.
- **Rationale**: Balances forgiving human authoring with invariant-preserving persistence and query correctness.
- **Specification**: Spec Story 42.

### Entry 10: Strftime Dialect with Scoped Moment-Token Translation

- **Divergence**: Core formatting uses `chrono::format::strftime` syntax, while template date filters accept `moment.js` tokens via a scoped translation layer (bracket literals, `YYYY MM DD HH mm ss`, `Do`/`S` ordinals, `dddd`/`ddd`/`MMM`/`MMMM`). Unsupported moment tokens fail with an explicit diagnostic naming the token.
- **External Behavior**: `Templater` uses `moment.js` strings with no validation; Dataview uses `Luxon` tokens.
- **Rationale**: Maintains a single robust formatting engine while allowing templates copied from `Templater` recipes to render accurately.
- **Specification**: Spec Story 8–9, Decision `D-b`.

### Entry 11: DST Overlap Resolves Earlier

- **Divergence**: During an ambiguous fall-back DST transition (e.g., 01:30 repeating twice), Traces resolves to the earlier UTC instant.
- **External Behavior**: PostgreSQL resolves to the later occurrence; Temporal, Jiff, and BigQuery resolve to the earlier occurrence.
- **Rationale**: Adheres to modern ecosystem consensus (Temporal/RFC 5545) for deterministic instant assignment.
- **Specification**: Spec Story 13, Decision `D14`.

### Entry 12: Sub-Hour Exact Application Versus Calendar Units

- **Divergence**: Applying days, weeks, months, or years shifts the calendar clock date, whereas sub-day units (hours, minutes, seconds, milliseconds) apply as exact physical durations on the instant.
- **External Behavior**: Naive timestamp libraries treat `+ 1 day` as `+ 86400 seconds`, which drifts the clock time across DST boundaries.
- **Rationale**: Replicates PostgreSQL `interval '1 day'` vs `interval '24 hours'` semantics: human calendar intent differs between "tomorrow at this time" and "in exactly 86,400 seconds."
- **Specification**: Spec Decision `D13`.

### Entry 13: WebAssembly Local Time Limitation Documented

- **Divergence**: On the `wasm32-unknown-unknown` target, local timezone resolution falls back to UTC because `chrono::Local` returns only `Single` instances in browser runtimes (chrono #1701).
- **External Behavior**: Undocumented failure or unexpected UTC display.
- **Rationale**: Formally documents the target limitation; native targets remain fully zone-aware.
- **Specification**: Spec Story 42.

### Entry 14: Non-Gregorian Calendar Systems out of Scope

- **Divergence**: Traces supports only the Gregorian calendar; eras, lunar calendars, and non-Gregorian systems supported by `Luxon` are explicitly out of scope.
- **External Behavior**: `Luxon` supports diverse international calendar systems.
- **Rationale**: Focuses scope strictly on personal knowledge management requirements.
- **Specification**: Spec Story 42.

### Entry 15: RFC 3339 `…Z` Serialization

- **Divergence**: `Serialize for DateTimeValue` emits explicit RFC 3339 `…Z` UTC representations (`to_rfc3339_opts(SecondsFormat::AutoSi, true)`), whereas `Display` renders the everyday local clock time.
- **External Behavior**: Some libraries serialize using the same format as human display, losing timezone context across serialization boundaries.
- **Rationale**: Interoperable data interchange requires explicit UTC offsets, while human display requires local clock times.
- **Specification**: Spec Story 41, Decision `RPT-F3`.

### Entry 16: Computed Duration Display Synthesis

- **Divergence**: A computed duration's `Display` output is synthesized from normalized total seconds (`canonical_spelling`) and is display-only; internal equality and hashing remain strictly seconds-based. Equal-seconds durations may display a representation differing from their written components (e.g., `parse("1mo") + parse("0s")` displays as `"4w 2d"` while retaining calendar month application).
- **External Behavior**: Systems either discard written components entirely or serialize the written form unchanged.
- **Rationale**: Raw string representations serve display purposes only; witness parts are preserved for calendar math without corrupting value equality.
- **Specification**: Spec Decision NU-1, Spec `L84/L90`.

### Entry 17: `From<DateValue> for DateTimeValue` Fallback to Neutral UTC Frame

- **Divergence**: In the infallible conversion `From<DateValue> for DateTimeValue`, if the host system timezone lookup fails, the conversion falls back to UTC midnight rather than surfacing `DateError::LocalZoneLookup`.
- **External Behavior**: Fallible operations surface `LocalZoneLookup` directly.
- **Rationale**: Infallible promotions are required by `Ordering::cmp`, sort key construction, and filter equality checks that cannot propagate errors. A `TryFrom` refactor would duplicate local-zone fallback logic across every sort and comparison site. Fallible query and template operations continue to surface `LocalZoneLookup`.
- **Specification**: Spec Decision `X4(a)`, Decision `D14` declared exception.

### Entry 18: List-First Comma Precedence at Inline Value Seam

- **Divergence**: In Markdown body and inline field text, when a comma follows a complete atomic value with remaining content (e.g., `1h, 30m`), list parsing takes precedence, yielding a two-element list of durations. Commas retain duration-part separator semantics only in explicit duration contexts (`dur("1h, 30m")`, YAML scalars).
- **External Behavior**: A duration parser with comma lookahead would consume the comma and parse `1h, 30m` as a single compound duration, breaking Markdown list syntax.
- **Rationale**: Preserves standard Markdown comma-separated list parsing while keeping compound duration parsing functional in explicit expressions.
- **Specification**: Spec Decision `U1/F1`, Ticket 04.

### Entry 19: No Epoch-Day Division for Week Calculations

- **Divergence**: Week calculations use the ISO week primitives of the `chrono` crate (`NaiveDate::iso_week()`, `from_isoywd_opt`), strictly avoiding epoch-day integer division (`floor(day / 7) + 1`).
- **External Behavior**: Dataview historically calculated week numbers via `floor(day / 7) + 1`, causing incorrect week assignments near year boundaries.
- **Rationale**: Prevents the Dataview `.week` trap; ISO-8601 week calculations are mathematically correct and standard.
- **Specification**: Spec Story 4, Week mandate `L109`.

### Entry 20: No Lenient Format Guessing In `DatePoint::parse_with_pattern`

- **Divergence**: `DatePoint::parse_with_pattern(text, pattern)` enforces a strict contract: matching formats succeed; non-matching text or invalid format patterns return explicit errors, rejecting heuristic guessing.
- **External Behavior**: `Templater` and `moment.js` attempt heuristic format recovery when a format pattern fails to match.
- **Rationale**: Silent or heuristic date parsing causes subtle data corruption in notes and queries; strict errors ensure predictable behavior.

### Entry 21: ISO Weekday Numbering in the Template `weekday` Filter

- **Divergence**: The template `weekday` filter without an argument returns the ISO weekday number, Monday = 1 through Sunday = 7, matching `date_component(..., "weekday")` in queries. `weekday(n)` accepts only `1..=7`; `0` is rejected because Sunday-based conventions disagree on it (Sunday is `0` in `Moment.js` and `1` in US-style 1–7 numbering).
- **External Behavior**: `Moment.js` and `Templater` return `0`–`6`, Sunday-based.
- **Rationale**: One weekday numbering across templates and queries, consistent with the pinned ISO Monday weeks of Entry 7.
- **Specification**: Spec Story 18, Ticket 07.
