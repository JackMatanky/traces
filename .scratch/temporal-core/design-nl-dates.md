# Natural Language Dates Design Analysis for Traces

## Executive Summary

The task is to add natural language date parsing to Traces' template engine, enabling expressions like "next friday", "in 3 days", "eom", "today", etc. This analysis synthesizes findings from four research threads and provides design options grounded in the repo's actual architecture.

## 1. Current Architecture — What Already Exists

### 1.1 Date Parsing Pipeline (temporal/point.rs)

The single entry point is `DatePoint::parse(s: &str) -> Result<Self, DateError>` at `src/temporal/point.rs:79`. The pipeline:

1. **Shape dispatch**: `starts_like_date` → `starts_with_year` → length check
2. **Short (<=11 chars)**: ISO date `YYYY-MM-DD` or year-month `YYYY-MM` via `DateShape::parse_any`
3. **Long**: Tokenizer + lexer (`TemporalShapeToken`) handles datetime shapes including time components
4. **Fallback**: If neither DateShape nor DateTimeShape matches, error

`DateShape::parse_any` only accepts: `YearMonthDay` (YYYY-MM-DD) and `YearMonth` (YYYY-MM). No relative/natural language support exists.

### 1.2 Value Model

| Type | Precision | Zone | Source |
|---|---|---|---|
| `DateValue` | Whole-day, zone-free proleptic Gregorian | None (implicit local zone for display) | `src/temporal/date.rs` |
| `DateTimeValue` | UTC instant + civil rendering | Local zone resolution for display | `src/temporal/datetime.rs` |
| `DatePoint::Calendar` | Date precision (day/month) from written shape | None | `src/temporal/point.rs:57` |
| `DatePoint::DateTime` | UTC instant | None (local zone for display) | `src/temporal/point.rs:79` |

Key: **Precision preservation** — `DatePoint::Calendar` retains how much of the date the input specified (e.g. `2026-07` → month precision, first day of month). This is critical for NL dates where "next month" should keep month-level precision.

### 1.3 Clock Seam

Only one injectable clock: `now_instant()` at `src/temporal/datetime.rs:307`, used by `is_past`/`is_future` in the template engine. The `ZoneGuard` in `src/temporal/mod.rs:49` provides per-thread zone override for tests.

### 1.4 Template Engine Integration

`src/template/engine/date.rs` registers the `date` namespace with:
- Generators: `date.now()`, `date.today()`, `date.tomorrow()`, `date.yesterday()`, `date.from_timestamp(ts)`
- Filters: `date_format`, `timestamp`, `date_add`, `date_sub`, `add_days`, `sub_days`, `add_months`, `sub_months`, `add_years`, `sub_years`, `weekday`, `date_diff`
- Tests: `is_past`, `is_future`, `is_leap_year`
- No relative/NL parsing functions exist today

## 2. What NL Dates Must Support

Based on Obsidian's nldates plugin and the chrono-node grammar, the following expression categories are needed:

| Category | Examples | Expected resolution |
|---|---|---|
| **Absolute dates** | "2026-11-20", "November 2026" | Date value at written precision |
| **Relative offsets** | "3 days ago", "in 2 weeks", "next friday", "last monday" | Date relative to anchor |
| **Unit expressions** | "eom", "eoy", "equarters", "q1", "q2", "q3", "q4" | End/start of period |
| **Time expressions** | "now", "09:00", "14:30", "5 minutes from now", "1 hour ago" | Time-of-day or combined |
| **Combined** | "tomorrow at 09:00", "next monday at 14:30" | Full date-time |
| **Casual words** | "today", "tomorrow", "yesterday", "noon", "midnight", "this week", "last week" | Fixed dates |

## 3. Design Options

### Design A: Extend `DatePoint::parse` in `src/temporal/`

**Location**: `src/temporal/point.rs:79` — the existing parse entry point

**Pros**:
- Single source of truth for date parsing
- Preserves precision model (DatePoint already tracks precision)
- Reuses existing shape dispatch and zone resolution
- Injectable clock via `now_instant()` already available
- Minimal new code: add NL parsing as a post-parse transform or within the existing lexer path

**Cons**:
- `DatePoint::parse` currently expects ISO dates (`starts_with_year` check). Adding NL would require relaxing or branching the validation.
- The lexer (`TemporalShapeToken`) is tuned to ISO/strftime patterns; extending it is non-trivial.
- Tests for `DatePoint::parse` would need extension without breaking existing ISO behavior.

**NL Parsing Strategy within this design**:
Add a dedicated `parse_natural_language(input: &str, anchor: Timestamp) -> Result<DatePoint, DateError>` function that:
1. Tokenizes NL expressions into known tokens (weekdays, units, casual words)
2. Resolves relative offsets against `anchor` (the "now" instant)
3. Returns a `DatePoint` preserving the appropriate precision

**Risk**: This would make `point.rs` larger and more complex. The existing ISO--only validation at the top of `DatePoint::parse` could block NL input unless we restructure.

### Design B: New `natural_language` module in `src/temporal/`

**Location**: New file `src/temporal/natural_language.rs` (or `nl_date.rs`)

**Pros**:
- Clean separation: ISO parsing stays in `point.rs`/`date.rs`, NL gets its own module
- Easier to test in isolation
- Can experiment with parser generators (pest, chumsky) without touching core date logic
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

**Location**: `src/template/engine/date.rs` — new public functions only, no new temporal module changes

**Pros**:
- Minimal blast radius — no new public API surface beyond the template engine
- Can use chrono or a lightweight parser locally
- Templates get `date.relative("next monday")` immediately

**Cons**:
- Duplicates parsing logic that temporal/ already owns
- Breaks the principle that temporal owns date semantics
- If the parser needs to change, both temporal and template engine must change
- Doesn't solve the underlying need for NL dates in other contexts (CLI, config, etc.)

**Implementation**: Add functions like:
```rust
pub fn relative_date(input: &str) -> Result<String, TemplateError> {
    // Parse NL, resolve against now_instant(), format as ISO date string
}
```

### Design D: Hybrid — Extend temporal + template seam

**Pros**:
- Best of both worlds: core NL logic in temporal/, template convenience functions in engine/
- Allows the temporal module to evolve the parser while the template seam stabilizes
- Template functions can compose temporal primitives

**Cons**:
- Two places to keep in sync
- More complex integration

**Hybrid approach**: 
- `src/temporal/natural_language.rs` contains the parser + `parse_nl_to_datepoint(input, anchor) -> DatePoint`
- `src/template/engine/date.rs` registers `date.relative(expr)` that calls `temporal::parse_nl_to_datepoint(expr, now_instant())` and formats the result

## 4. Candidate Crates Survey (from research thread 3)

### 4.1 dateparser (waltzofpearls) 
- **Ver**: 0.5.4 (2023)
- **Deps**: 10+ (icu, serde, etc.)
- **Grammar**: Casual English dates, ordinals, "next friday", "3 days ago"
- **Relative support**: Yes, with anchor parameter
- **Returns**: `Date<DateTime>` (its own type, not jiff/chrono)
- **License**: MIT
- **Verdict**: Heavy deps (ICU), returns own Date type — would require conversion to jiff/DatePoint. Not ideal.

### 4.2 parse_datetime (momo)
- **Ver**: 0.1.5
- **Deps**: 3 (serde, thiserror, maybe chrono)
- **Grammar**: Basic relative, weekday references
- **Relative support**: Partial
- **Returns**: `Date<chrono::NaiveDate>` — chrono-locked
- **Verdict**: Chrono-locked, limited grammar. Not suitable.

### 4.3 chrono-english
- **Ver**: 0.4.1
- **Deps**: chrono 0.4
- **Grammar**: EN casual expressions ("next tuesday", "3 weeks ago"), weekday/month names
- **Relative support**: Yes, with configurable "now"
- **Returns**: chrono types (NaiveDate, NaiveDateTime) — would need jiff conversion
- **Verdict**: Chrono-locked but grammar is exactly what we need. Could vendor the keyword tables.

### 4.4 humantime
- **Ver**: 2.2.0
- **Deps**: 1 (serde)
- **Grammar**: "X years ago", "X months ago", "X days ago", "X hours ago"
- **Relative support**: Limited to unit expressions, no weekday/month names
- **Returns**: `humantime::Duration` — duration only, no date
- **Verdict**: Too limited. Only handles "N units ago" patterns.

### 4.5 timeago
- **Ver**: 1.1.0
- **Deps**: 2 (chrono, serde)
- **Grammar**: "X time ago", "X time from now"
- **Relative support**: Yes, but only unit expressions
- **Returns**: formatted string, not parseable date
- **Verdict**: Formatting-only, not parsing.

### 4.6 speedate
- **Ver**: 0.3.0
- **Deps**: speedate core (~5kb)
- **Grammar**: RFC 3339, ISO, some relative
- **Relative support**: Limited
- **Returns**: speedate::Date — its own type
- **Verdict**: Interesting but limited grammar. Worth examining the tokenizer structure.

### 4.7 jiff (already in use)
- **Ver**: 0.2.38
- **Deps**: ~5 (already in Cargo.toml)
- **Grammar**: None natively for NL
- **Relative support**: Has `civil::Date` arithmetic but no "parse "next friday""
- **Returns**: jiff types (civil::Date, ZonedDateTime)
- **Verdict**: The project's date foundation. No NL parser, but the type system is what we need.

## 5. Recommended Design

### Option 1: Extend `DatePoint::parse` with NL Branch (Recommended)

**Rationale**:
- The repo's date primitives already have the right value model (DatePoint + precision)
- The `now_instant()` clock seam is already in place
- No new dependencies needed
- Leverages existing module structure
- Single entry point for all date parsing

**Implementation Plan**:

1. **Add NL tokenizer** to `src/temporal/point.rs` or a new `src/temporal/nl_parser.rs`:
   - Tokenize: weekday names, month names, unit expressions ("3 days", "2 weeks"), casual words ("today", "tomorrow"), "eom", "eoy", "next", "last"
   - Handle "next friday", "last monday", "3 days ago", "in 2 weeks"

2. **Add `parse_natural_language(input: &str, anchor: Timestamp) -> Result<DatePoint, DateError>`**:
   - Parse tokens into a normalized form
   - Apply offsets against `anchor` (the "now" instant)
   - Preserve precision based on input specificity (e.g. "next month" → month precision)

3. **Register template functions** in `src/template/engine/date.rs`:
   ```rust
   /// Parse a natural language date expression relative to now.
   ///
   /// # Examples
   ///
   /// ```
   /// {{ date.relative("next friday") | date.format("%Y-%m-%d") }}
   /// {{ date.relative("3 days ago") | date.format("%Y-%m-%d") }}
   /// {{ date.relative("eom") | date.format("%Y-%m-%d") }}
   /// ```
   pub fn relative_date(input: &str) -> Result<String, TemplateError> {
       let dp = crate::temporal::parse_natural_language(input, crate::temporal::now_instant())?;
       Ok(dp.format("%Y-%m-%d").map_err(|e| ...)?)
   }
   ```

4. **Add test coverage** in `tests/` for NL expressions with fixed anchors using `ZoneGuard`.

### Option 2: New `temporal::nl` module (Alternative)

If the NL parsing logic becomes complex enough to warrant its own module (e.g. a PEG parser for the grammar), extract it to `src/temporal/nl/` with a clean API:

```rust
// src/temporal/nl/mod.rs
pub fn parse(input: &str, anchor: Timestamp) -> Result<DatePoint, DateError>;
pub fn parse_with_format(input: &str, format: &str, anchor: Timestamp) -> Result<String, DateError>;
```

Then have `engine/date.rs` call `crate::temporal::nl::parse(...)`.

### Option 3: Template-seam-only (Not Recommended)

Only add functions in `engine/date.rs` without touching `temporal/`. This duplicates logic and doesn't solve the architectural problem. Only suitable as a very temporary POC.

## 6. Precision Model Considerations

The existing `DatePoint` precision model is a key asset:

| Input | Expected precision output |
|---|---|
| "2026-11-20" (ISO) | Day precision |
| "2026-11" (year-month) | Month precision (day=1) |
| "next friday" | Day precision (relative to now) |
| "3 days ago" | Day precision |
| "eom" | Day precision (last day of current month) |
| "next month" | Month precision |
| "this week" | Week precision? (need to decide) |

**Key design decision**: What precision does "next month" produce?

- **Option A**: Month precision → the `DatePoint` would have `DatePrecision::Month`, and formatting would show the first day of next month. This is most useful for template use cases where you want to display "next month" without committing to a specific day.

- **Option B**: Day precision → compute the actual date (e.g. if today is Nov 12, "next month" → Dec 12). Less useful because it depends on month length.

**Recommendation**: Month precision for "next month", "last month", "next year", etc. Day precision for "next friday", "3 days ago", "eom" (which resolves to a specific day). This matches how humans think about these expressions.

## 7. Anchor/Now Resolution

The anchor (reference time) must be injectable for determinism:

- **Primary**: `now_instant()` from `src/temporal/datetime.rs:307` — the process-local clock
- **Test override**: `ZoneGuard::set("UTC")` from `src/temporal/mod.rs:49` — overrides the zone for `now_instant()`
- **Template context**: Could accept an optional second argument or use a thread-local

**Design choice**: Make `now` an explicit parameter to `parse_natural_language`, with a default of `now_instant()` for backward compatibility:

```rust
pub fn parse_natural_language(input: &str, anchor: Timestamp) -> Result<DatePoint, DateError>;
```

Template engine calls: `parse_nl("next friday", now_instant())`

Tests can: `parse_nl("next friday", Timestamp::from_unix_seconds(0))`

## 8. Ambiguity Resolution

Natural language dates have inherent ambiguities. The design must decide how to resolve them:

| Ambiguity | Resolution strategy |
|---|---|
| "may 5" vs "5 may" (month-first vs day-first) | Configurable `little_endian` flag (default: en_US = month-first), or always ISO |
| "next friday" — which friday? | First future friday (standard convention) |
| "in 3 days" — inclusive/exclusive | Exclusive: today + 3 |
| "may 5 2026" — year ambiguous | Require 4-digit year, or default to current year |
| "tuesday" without "next/last" | Next upcoming tuesday (or previous if already past tuesday this week) |
| "eom" in which month? | Current month |
| "this week" start day | Configurable week start (default: Sunday or Monday per locale) |

**Traces convention**: Always use ISO year-month-day ordering (little_endian=false), matching the existing `DateShape::YearMonthDay` which uses `YYYY-MM-DD`. For "next friday", resolve to the first future friday. For ambiguous month/day, default to month-first (en_US) but document the behavior.

## 9. Test Points to Preserve

Existing tests that must continue passing:

1. `DatePoint::parse` with ISO dates: `2026-11-20`, `2026-11`, `2026-11-20T14:30`
2. `is_past`/`is_future` with `now_instant()` 
3. `ZoneGuard` zone overrides
4. `date_format` filter with various patterns
5. `date_add`/`date_sub` with unit expressions

New tests needed for NL:

1. "today" → today's date at day precision
2. "tomorrow" → tomorrow's date
3. "yesterday" → yesterday's date
4. "next monday" → first future monday
5. "last friday" → most recent past friday (or first future if none this month?)
6. "3 days ago" → 3 days before now
7. "in 2 weeks" → 2 weeks from now
8. "eom" → last day of current month
9. "eoy" → December 31 of current year
10. "now" → current datetime

## 10. Risks and Constraints

| Risk | Mitigation |
|---|---|
| **Chrono vs jiff type mismatch** | Design NL parser to output `DatePoint` (already jiff-based), not chrono types. If vendor a parser, ensure it can output jiff `civil::Date`. |
| **DST transitions** | "next monday" resolves to the next monday at midnight local time, which DST-aware zone resolution handles via `now_instant()` → local zone. |
| **Locale dependence** | Traces defaults to en_US ordering (month-first) but documents it. Could add a config option later. |
| **Grammar bloat** | Start with a focused subset (today, tomorrow, next/last [weekday], N units ago/from now, eom, eoy). Extend incrementally. |
| **Test fixture complexity** | Use `ZoneGuard` to freeze "now" in tests. All NL parse tests should accept an anchor timestamp. |
| **Backward compatibility** | `DatePoint::parse` must continue accepting ISO dates unchanged. NL is a new code path, not a modification of existing behavior. |

## 11. Implementation Roadmap

### Phase 1: Minimal NL parsing (2-3 days)
- Add `parse_natural_language` function in `src/temporal/point.rs` or new module
- Support: "today", "tomorrow", "yesterday", "next monday", "last friday", "3 days ago", "in 2 weeks", "eom"
- Template function: `date.relative(expr)`
- Tests with `ZoneGuard`

### Phase 2: Extended grammar (1 week)
- Add: "this week", "last week", "next month", "last month", "eoy", "now at HH:MM"
- Refine precision model
- Add more template functions: `date.time(expr)`, `date.combined(expr)`

### Phase 3: Polish and extension points (1 week)
- Add configurability (locale, week start day)
- Add error messages for unrecognized expressions
- Document the API
- Consider vendoring a parser generator if grammar grows significantly

## Conclusion

The most robust design **extends `DatePoint::parse` with a dedicated NL branch** (Design A), producing `DatePoint` values that respect the existing precision model and leverage the injectable `now_instant()` clock seam. This approach:

- ✅ Zero new dependencies
- ✅ Reuses existing temporal infrastructure
- ✅ Preserves precision semantics
- ✅ Single, well-defined API surface
- ✅ Testable with existing `ZoneGuard` patterns
- ✅ Template integration via `date.relative()` helper

The implementation should start with a focused subset of expressions (today, tomorrow, next/last weekday, N units, eom, eoy) and expand incrementally. The key design leverages what already works: the `DatePoint` precision model, the `now_instant()` clock, and the `ZoneGuard` test override pattern.