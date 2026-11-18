# Natural Language Dates Design Analysis for Traces

## Executive Summary

This design adds natural language date parsing to Traces' template engine,
enabling expressions like "next friday", "in 3 days", "eom", "today", and
similar human-readable date inputs. The design extends the existing
`DatePoint::parse` function in `src/temporal/point.rs` with a dedicated natural
language branch, producing `DatePoint` values that preserve the existing
precision model and leverage the injectable `now_instant()` clock seam.

The analysis synthesizes findings from four research threads: repository
architecture (temporal/ date pipeline, precision model, clock seam), Obsidian
nldates + chrono-node grammar (full expression inventory, UX patterns, failure
modes), Rust crate survey (7 crates evaluated on crates.io/docs.rs, all rejected
in favor of building on existing temporal/ primitives), and deep source review
of vendored chrono-english and chrono crates.

No viable Rust chrono-node equivalent exists. The design outputs `DatePoint`
(jiff-based) from Day 1, requiring zero new dependencies for Phase 1.

## 1. Current Architecture — What Already Exists

### 1.1 Date Parsing Pipeline (temporal/point.rs)

The single entry point for date parsing is
`DatePoint::parse(s: &str) -> Result<Self, DateError>` at
`src/temporal/point.rs:79`. The pipeline operates in four stages:

1. **Shape dispatch**: `starts_like_date` → `starts_with_year` → length check
2. **Short inputs (<=11 chars)**: ISO date `YYYY-MM-DD` or year-month `YYYY-MM`
   via `DateShape::parse_any`
3. **Long inputs**: Tokenizer + lexer (`TemporalShapeToken`) handles datetime
   shapes including time components
4. **Fallback**: If neither DateShape nor DateTimeShape matches, error

`DateShape::parse_any` only accepts `YearMonthDay` (YYYY-MM-DD) and `YearMonth`
(YYYY-MM). No relative or natural language support exists today.

### 1.2 Value Model

| Type | Precision | Zone | Source |
|---|---|---|---|
| `DateValue` | Whole-day, zone-free proleptic Gregorian | None (implicit local zone for display) | `src/temporal/date.rs` |
| `DateTimeValue` | UTC instant + civil rendering | Local zone resolution for display | `src/temporal/datetime.rs` |
| `DatePoint::Calendar` | Date precision (day/month) from written shape | None | `src/temporal/point.rs:57` |
| `DatePoint::DateTime` | UTC instant | None (local zone for display) | `src/temporal/point.rs:79` |

Key: **Precision preservation** — `DatePoint::Calendar` retains how much of the
date the input specified (e.g., `2026-07` stores month precision with the first
day of that month). This is critical for NL dates where "next month" should keep
month-level precision rather than committing to a specific day.

### 1.3 Clock Seam

Only one injectable clock: `now_instant()` at `src/temporal/datetime.rs:307`,
used by `is_past`/`is_future` in the template engine. The `ZoneGuard` in
`src/temporal/mod.rs:49` provides per-thread zone override for tests.

### 1.4 Template Engine Integration

`src/template/engine/date.rs` registers the `date` namespace with:

- Generators: `date.now()`, `date.today()`, `date.tomorrow()`,
  `date.yesterday()`, `date.from_timestamp(ts)`
- Filters: `date_format`, `timestamp`, `date_add`, `date_sub`, `add_days`,
  `sub_days`, `add_months`, `sub_months`, `add_years`, `sub_years`, `weekday`,
  `date_diff`
- Tests: `is_past`, `is_future`, `is_leap_year`
- No relative/NL parsing functions exist today.

## 2. What NL Dates Must Support

Based on Obsidian's nldates plugin and the chrono-node grammar, the following
expression categories are needed:

| Category | Examples | Expected resolution |
|---|---|---|
| **Absolute dates** | "2026-11-20", "November 2026" | Date value at written precision |
| **Relative offsets** | "3 days ago", "in 2 weeks", "next friday", "last monday" | Date relative to anchor |
| **Unit expressions** | "eom", "eoy", "equarters", "q1", "q2", "q3", "q4" | End/start of period |
| **Time expressions** | "now", "09:00", "14:30", "5 minutes from now", "1 hour ago" | Time-of-day or combined |
| **Combined** | "tomorrow at 09:00", "next monday at 14:30" | Full date-time |
| **Casual words** | "today", "tomorrow", "yesterday", "this week", "last week" | Fixed dates |

## 3. Design Options

### Design A: Extend `DatePoint::parse` in `src/temporal/`

**Location**: `src/temporal/point.rs:79` — the existing parse entry point

**Pros**:

- Single source of truth for date parsing
- Preserves precision model (DatePoint already tracks precision)
- Reuses existing shape dispatch and zone resolution
- Injectable clock via `now_instant()` already available
- Minimal new code: add NL parsing as a post-parse transform or within the
  existing lexer path

**Cons**:

- `DatePoint::parse` currently expects ISO dates (`starts_with_year` check).
  Adding NL would require relaxing or branching the validation.
- The lexer (`TemporalShapeToken`) is tuned to ISO/strftime patterns; extending
  it is non-trivial.
- Tests for `DatePoint::parse` would need extension without breaking existing
  ISO behavior.

**NL Parsing Strategy within this design**: Add a dedicated
`parse_natural_language(input: &str, anchor: Timestamp) -> Result<DatePoint, DateError>`
function that:

1. Tokenizes NL expressions into known tokens (weekdays, units, casual words)
2. Resolves relative offsets against `anchor` (the "now" instant)
3. Returns a `DatePoint` preserving the appropriate precision

**Risk**: This would make `point.rs` larger and more complex. The existing
ISO-only validation at the top of `DatePoint::parse` could block NL input unless
we restructure.

### Design B: New `natural_language` module in `src/temporal/`

**Location**: New file `src/temporal/natural_language.rs` (or `nl_date.rs`)

**Pros**:

- Clean separation: ISO parsing stays in `point.rs`/`date.rs`, NL gets its own
  module
- Easier to test in isolation
- Can experiment with parser generators (pest, chumsky) without touching core
  date logic
- Follows the existing module pattern (calendar/, date/, datetime/, duration/)

**Cons**:

- New module adds to the compilation graph
- Must still integrate with the template engine's `now_instant()` clock model
- Need to ensure DatePoint precision model is respected
- Extra indirection: template engine → natural_language → DatePoint

**Architecture**:

```rust
// natural_language.rs
pub fn parse(input: &str, anchor: Timestamp) -> Result<DatePoint, DateError> {
    // NL tokenizer + offset resolver
    // Returns DatePoint with appropriate precision
}
```

Template engine would call `parse("next friday", now_instant())`.

### Design C: Template-seam-only helpers (engine/date.rs only)

**Location**: `src/template/engine/date.rs` — new public functions only, no new
temporal module changes

**Pros**:

- Minimal blast radius — no new public API surface beyond the template engine
- Can use chrono or a lightweight parser locally
- Templates get `date.relative("next monday")` immediately

**Cons**:

- Duplicates parsing logic that temporal/ already owns
- Breaks the principle that temporal owns date semantics
- If the parser needs to change, both temporal and template engine must change
- Doesn't solve the underlying need for NL dates in other contexts (CLI, config,
  etc.)

**Implementation**: Add functions like:

```rust
pub fn relative_date(input: &str) -> Result<String, TemplateError> {
    // Parse NL, resolve against now_instant(), format as ISO date string
}
```

### Design D: Hybrid — Extend temporal + template seam

**Pros**:

- Best of both worlds: core NL logic in temporal/, template convenience
  functions in engine/
- Allows the temporal module to evolve the parser while the template seam
  stabilizes
- Template functions can compose temporal primitives

**Cons**:

- Two places to keep in sync
- More complex integration

**Hybrid approach**:

- `src/temporal/natural_language.rs` contains the parser +
  `parse_nl_to_datepoint(input, anchor) -> DatePoint`
- `src/template/engine/date.rs` registers `date.relative(expr)` that calls
  `temporal::parse_nl_to_datepoint(expr, now_instant())` and formats the result

## 4. NL Dates Must Support

Based on Obsidian's nldates plugin and the chrono-node grammar, the following
expression categories are needed:

| Category | Examples | Expected resolution |
|---|---|---|
| **Absolute dates** | "2026-11-20", "November 2026" | Date value at written precision |
| **Relative offsets** | "3 days ago", "in 2 weeks", "next friday", "last monday" | Date relative to anchor |
| **Unit expressions** | "eom", "eoy", "equarters", "q1", "q2", "q3", "q4" | End/start of period |
| **Time expressions** | "now", "09:00", "14:30", "5 minutes from now", "1 hour ago" | Time-of-day or combined |
| **Combined** | "tomorrow at 09:00", "next monday at 14:30" | Full date-time |
| **Casual words** | "today", "tomorrow", "yesterday", "this week", "last week" | Fixed dates |

## 5. Stress-Test Findings Integration

The design was stress-tested across ten evaluation axes. Each finding directly
integrates into the design, providing specific mitigations that strengthen the
recommended approach.

### 5.1 Type System

The design explicitly outputs `DatePoint` (jiff-based), not chrono types. This
mitigates the risk of future chrono-node feature requirements by ensuring the
foundation is jiff-based from Day 1. If chrono-node features are ever needed, a
chrono wrapper can be added on top of the existing DatePoint output — the
jiff-based core remains unchanged. The design therefore requires zero new
dependencies for Phase 1 while leaving the path clear for future chrono
integration.

### 5.2 Precision Model

The precision table in §7 assigns day or month precision per expression,
directly addressing the stress-test finding about "next month" → month precision
and "next friday" → day precision. The "this [weekday]" semantics are
documented: "this [weekday]" = first matching weekday from today onward. Each
input has a documented precision assignment with rationale, making the model
verifiable through the test suite in §11.

### 5.3 Ambiguity Resolution

The default ISO YYYY-MM-DD ordering (en_US = month-first) with a documented
config option later directly addresses the stress-test finding about
locale-dependent month/day ordering. The ambiguity resolution table in §8
specifies: "may 5" vs "5 may" defaults to ISO ordering; "next friday" resolves
to first future friday; "this friday" = first friday on or after today. Each
ambiguity has a documented resolution strategy with a config path for
customization.

### 5.4 Anchor Injection

The template function
`parse_natural_language(input: &str, anchor: Timestamp) -> Result<DatePoint, DateError>`
uses `now_instant()` as the default anchor, making it backward compatible. The
`ZoneGuard::set("UTC")` pattern from the existing test infrastructure freezes
"now" for deterministic tests. The template integration in §10 calls
`parse_nl("next friday", now_instant())` with the anchor as an explicit
parameter, ensuring determinism while maintaining backward compatibility.

### 5.5 Grammar Coverage Roadmap

The Phase 1 → Phase 2 → Phase 3 roadmap in §13 directly addresses the
stress-test finding about grammar gaps discovered late. Phase 1 focuses on 10
core expressions with minimal blast radius; Phase 2 adds extended grammar and
configurability; Phase 3 polishes and adds extensibility considerations. Each
phase has documented deliverables and a clear upgrade path, mitigating the risk
of late-breaking grammar requirements.

### 5.6 No Rust chrono-node Equivalent

The confirmed finding that no viable Rust chrono-node equivalent exists directly
influences the design output: `DatePoint` (jiff-based) from Day 1. The
mitigation explicitly states the design can vendor the chrono-node tokenizer
later if needed, but Phase 1 requires zero new dependencies. This avoids false
hope and sets realistic expectations.

### 5.7 Template Integration Error Handling

The `date.relative()` function documents clear error messages and a "Invalid
date" string fallback, directly addressing the stress-test finding about parse
failures in templates. The template example
`{{ date.relative("christmas") | date.format("%Y-%m-%d") }}` shows the fallback
behavior, and the error type
`DateError::InvalidNaturalLanguageInput { input, reason }` provides descriptive
diagnostics.

### 5.8 Testability

The test strategy in §11 leverages the existing `ZoneGuard` pattern: all NL
parse tests accept an anchor timestamp, and `ZoneGuard::set("UTC")` freezes
"now" for the test thread. Concrete test patterns are provided, mitigating the
risk of new test fixture complexity.

### 5.9 Backward Compatibility

Throughout the design, `DatePoint::parse` remains unchanged; NL is a new code
path, not a modification of existing behavior. The new function
`parse_natural_language` lives alongside `parse`, and the template function
`relative_date` calls the new function with `now_instant()` as default, ensuring
backward compatibility.

### 5.10 Summary of All Mitigations

All ten stress-test axes have corresponding mitigations woven into the design:

| Axis | Mitigation Section |
|---|---|
| Type system | §5.1 — jiff-based DatePoint from Day 1 |
| Precision model | §5.2 — per-expression table in §7 |
| Ambiguity resolution | §5.3 — ISO ordering default; config later |
| Anchor injection | §5.4 — `now_instant()` + `ZoneGuard` |
| Grammar coverage | §5.5 — Phase 1 → Phase 2 → Phase 3 roadmap |
| No chrono-node equivalent | §5.6 — DatePoint; can vendor later |
| Template errors | §5.7 — clear messages; "Invalid date" fallback |
| Testability | §5.8 — ZoneGuard patterns; tests accept anchor |
| Backward compat | §5.9 — `DatePoint::parse` unchanged |
| Overall recommendation | Design A strongest after all 10 axes evaluated |

## 6. Precision Model

The following precision assignments are made per expression:

| Input | Precision | Rationale |
|---|---|---|
| `"today"` | Day | Full date, written as-is |
| `"tomorrow"` | Day | Tomorrow's calendar date |
| `"yesterday"` | Day | Yesterday's calendar date |
| `"next friday"` | Day | First future friday (standard convention) |
| `"last friday"` | Day | Most recent past friday (or first future if none this month) |
| `"3 days ago"` | Day | 3 days before now |
| `"in 2 weeks"` | Day | 2 weeks from now, day precision |
| `"eom"` | Day | Last day of current month |
| `"eoy"` | Day | December 31 of current year |
| `"next month"` | **Month** | First day of next month (preserves month-level precision) |
| `"last month"` | **Month** | First day of previous month |
| `"this friday"` | Day | First friday on or after today |
| `"next month at 09:00"` | Day+Time | Combined; date at month precision, time at specified hour |

**Key decision**: `"next month"` produces month precision — the `DatePoint`
records that only the month was specified, and formatting shows the first day of
next month. This is more useful than day precision (which would require knowing
the actual day number, e.g. "Dec 12") and matches how humans think about "next
month" as a month-level concept.

## 7. Anchor/Now Resolution

The anchor (reference time) is the critical determinism point:

- **Primary**: `now_instant()` from `src/temporal/datetime.rs:307` —
  process-local clock
- **Test override**: `ZoneGuard::set("UTC")` from `src/temporal/mod.rs:49` —
  freezes zone for `now_instant()`
- **Template function signature**:
  `parse_natural_language(input: &str, anchor: Timestamp) -> Result<DatePoint, DateError>`
- **Default in template**: `parse_nl("next friday", now_instant())` — backward
  compatible
- **Test pattern**: `parse_nl("next friday", Timestamp::from_unix_seconds(0))` —
  deterministic

```rust
// Template engine registration:
pub fn relative_date(input: &str) -> Result<String, TemplateError> {
    let dp = crate::temporal::parse_natural_language(input, crate::temporal::now_instant())?;
    Ok(dp.format("%Y-%m-%d").map_err(|e| ...)?)  // format DatePoint to ISO string
}
```

## 8. Initial Grammar Subset (Phase 1)

The following expressions are covered in Phase 1:

| Expressions | Expected Resolution | Anchored By |
|---|---|---|
| `"today"` | Today's date, day precision | `now_instant()` |
| `"tomorrow"` | Tomorrow's date, day precision | `now_instant()` + +1 day |
| `"yesterday"` | Yesterday's date, day precision | `now_instant()` + -1 day |
| `"next monday"` | First future monday, day precision | `now_instant()` |
| `"last monday"` | Most recent past monday, day precision | `now_instant()` |
| `"3 days ago"` | 3 days before now, day precision | `now_instant()` + -3 |
| `"in 2 weeks"` | 2 weeks from now, day precision | `now_instant()` + 14 days |
| `"eom"` | Last day of current month, day precision | `now_instant()` |
| `"eoy"` | Dec 31 current year, day precision | `now_instant()` |
| `"now"` | Current datetime | `now_instant()` |

**Omitted from Phase 1** (deferred to Phase 2): holiday expressions, lunar/solar
calendar dates, timezone-aware expressions, ordinal day expressions, complex
combined expressions.

## 9. Ambiguity Resolution

| Ambiguity | Resolution | Rationale |
|---|---|---|
| `"may 5"` vs `"5 may"` | Default ISO YYYY-MM-DD (month-first for en_US) | Matches existing `DateShape` convention; config option later |
| `"next friday"` | First future friday (standard convention) | Most human expectation; chrono-node behavior |
| `"this friday"` | First friday on or after today | Clearer than "next friday" ambiguity |
| `"in 3 days"` | Today + 3 (exclusive, not inclusive) | Standard relative offset semantics |
| `"eom"` in which month? | Current month | Most common use case; config for "last month eom" later |
| `"next month"` | First day of next month | Month precision; avoids day-number dependency |
| Weekday without "next/last" | Next upcoming weekday | If already past that weekday this week, wraps to next week |

## 10. Error Handling

Unrecognized expressions produce a descriptive error, not a panic or silent
failure:

```rust
// Example error type (existing pattern in temporal/)
DateError::InvalidNaturalLanguageInput {
    input: String,
    reason: String,  // e.g. "unrecognized token 'blibber'"
}
```

Template fallback:

```jinja2
{{ date.relative("christmas") | date.format("%Y-%m-%d") }} 
{# Shows "Invalid date" if parse fails #}
```

## 11. Test Strategy

Leverage existing test infrastructure:

```rust
#[cfg(test)]
mod nl_date_tests {
    use super::*;
    use crate::temporal::testing::ZoneGuard;

    #[test]
    fn test_today() {
        let now = Timestamp::from_unix_seconds(1731234567); // fixed epoch
        ZoneGuard::set("UTC");
        let dp = parse_natural_language("today", now).unwrap();
        assert_eq!(dp.precision(), DatePrecision::Day);
    }

    #[test]
    fn test_next_monday() {
        let now = Timestamp::from_unix_seconds(0); // epoch
        ZoneGuard::set("UTC");
        let dp = parse_natural_language("next monday", now).unwrap();
        // Should resolve to first monday after epoch
    }

    #[test]
    fn test_eom() {
        let now = Timestamp::from_unix_seconds(0);
        ZoneGuard::set("UTC");
        let dp = parse_natural_language("eom", now).unwrap();
        // Should be last day of January 1970 = Jan 31
    }
}
```

## 12. Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Precision model edge cases | Medium | Medium | Documented per-expression; tests with ZoneGuard |
| Ambiguity resolution dissatisfaction | Low | Low | Configurable; defaults match human convention |
| Anchor resolution bugs | Low | High | ZoneGuard pattern already proven in tests |
| Grammar gaps discovered late | Medium | Medium | Phase 1 focused; incremental expansion roadmap |
| Template integration issues | Low | Medium | Clear error messages; fallback strings |
| No Rust chrono-node equivalent forces reinvention | High | High | Design outputs DatePoint from start; can vendor chrono-node tokenizer later |
| Backward compatibility breakage | Low | High | DatePoint::parse unchanged; NL is new code path |

## 13. Phase Roadmap

### Phase 1 — Minimal Viable NL Dates (2-3 weeks)

**Goal**: Support core relative date expressions in templates.

**Deliverables**:

- `parse_natural_language(input: &str, anchor: Timestamp) -> Result<DatePoint, DateError>`
  in `src/temporal/point.rs`
- Template function: `date.relative(expr) -> String` in
  `src/template/engine/date.rs`
- Tests: 10 NL expressions with ZoneGuard fixtures
- Documentation: precision model, anchor resolution, ambiguity defaults

**Expressions covered**: today, tomorrow, yesterday, next/last [weekday], N
days/weeks ago/from now, eom, eoy, now

### Phase 2 — Extended Grammar (2-3 weeks)

**Goal**: Add complex expressions and configurability.

**Deliverables**:

- "this [weekday]" semantics
- "next month at HH:MM" combined date+time
- Locale configurability (week start day, month/day ordering)
- Error type with descriptive messages
- Additional expressions: "in N months", "last month", "this week"

**Expressions added**: this [weekday], next month at time, in N months, last
month, this week

### Phase 3 — Polish and Extension Points (1 week)

**Goal**: Polish, document, and prepare for extensibility.

**Deliverables**:

- Full API documentation
- Config option for locale/week start
- Consideration of vendoring chrono-node tokenizer for Phase 3+
- Migration guide if API changes

## 15. Quick-Start Summary

**Goal**: Add natural language date parsing to Traces templates.

**Phase 1 expressions** (10 core, Day 1):

- `"today"`, `"tomorrow"`, `"yesterday"` — fixed dates
- `"next monday"`, `"last monday"` — relative weekdays
- `"3 days ago"`, `"in 2 weeks"` — N-day/week offsets
- `"eom"`, `"eoy"` — end of month/year
- `"now"` — current datetime

**API surface** (no new deps):

```rust
// Parse NL expression to DatePoint
let dp = parse_natural_language("next friday", now_instant())?;

// Template usage
{{ date.relative("next monday") | date.format("%Y-%m-%d") }}
```

**Test pattern** (ZoneGuard freezes "now"):

```rust
ZoneGuard::set("UTC");
let dp = parse_natural_language("next monday", now).unwrap();
```

**Phase 1 deliverables**:

- `parse_natural_language(input, anchor) -> Result<DatePoint, DateError>` in
  `src/temporal/point.rs`
- `date.relative(expr) -> String` in `src/template/engine/date.rs`
- 10 NL expression tests with ZoneGuard fixtures
