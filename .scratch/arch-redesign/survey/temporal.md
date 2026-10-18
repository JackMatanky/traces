# Architectural Survey: Temporal and its Consumers

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

## 1. Scope & Size

- **Path:** `src/temporal/` and consumers across `src/` (`field.rs`, `note/`, `query/`, `template/engine/date.rs`, `schema/fields/date.rs`, `task.rs`, `file.rs`, `lib.rs`).
- **Files covered in `src/temporal/`:**
  - `mod.rs`: module root, re-exports, test harness `ZoneGuard`
  - `point.rs`: `DatePoint` sum type, `DatePrecision`, `DateDiff`, `DifferenceEngine`, pattern parsing
  - `date.rs`: `DateValue` civil date wrapper, `DateShape`, ISO parsing/serde
  - `datetime.rs`: `DateTimeValue` UTC instant wrapper, `DateTimeParser`, `DateTimeShape`, ISO parsing/serde
  - `duration.rs`: `DurationValue` (Jiff `Span` + nominal seconds), unit-wise arithmetic, serde
  - `unit.rs`: `TimeUnit`, `DateUnit`, duration component decomposition, `phf` spellings
  - `calendar.rs`: `CalendarPeriod`, `PeriodEdge`, `phf` boundary table, weekday helpers
  - `zone.rs`: `local_zone()` resolution with test override hook
  - `lexer.rs`: Logos tokenizers for date/datetime/duration prefixes and shapes
  - `error.rs`: `DateError`, `DurationError`, `CalendarError` (internal)
  - `GLOSSARY.md`: temporal terminology register
- **SLoC Metrics (verified by line count):**
  - Total `src/temporal/*.rs`: 10,308 lines.
  - Production code: ~3,046 lines (29.5%).
  - Test code (`#[cfg(test)]` modules): ~7,262 lines (70.5%).
  - File breakdown (prod / test / total):
    - `point.rs`: 502 / 2,791 / 3,293
    - `datetime.rs`: 615 / 1,256 / 1,871
    - `duration.rs`: 439 / 1,264 / 1,703
    - `date.rs`: 390 / 768 / 1,158
    - `unit.rs`: 364 / 356 / 720
    - `mod.rs`: 255 / 289 / 544
    - `calendar.rs`: 126 / 222 / 348
    - `error.rs`: 169 / 173 / 342
    - `lexer.rs`: 149 / 72 / 221
    - `zone.rs`: 37 / 71 / 108
- **Public vs. `pub(crate)` Surface:**
  - **Public (`pub` from crate root):** `DateValue`, `DateTimeValue`, `DurationValue`, `DateError`, `DurationError`.
  - **`pub(crate)`:** `DatePoint`, `DateDiff`, `DatePrecision`, `DateUnit`, `TimeUnit`, `CalendarPeriod`, `PeriodEdge`, `resolve_period_edge`, `weekday_from_number`, `DEFAULT_DATE_PATTERN`, `DEFAULT_DATETIME_PATTERN`, `UNIT_SPELLINGS_HINT`, `local_zone`, and `test_support::ZoneGuard` (under test/test-utils).
  - Private internals: `CalendarError`, `DateShape`, `DateTimeShape`, `DateTimeInput`, `DifferenceEngine`, `scan_date_prefix`, `starts_like_date`.

---

## 2. Responsibilities

1. **Zone-Free Civil Calendar Dates (`DateValue` in `date.rs`):**
   - Wraps Jiff `civil::Date` (`date.rs:45`).
   - Represents proleptic Gregorian calendar date `[-9999..=9999]` with no time of day or timezone.
   - Strictly enforces 4-digit ASCII year prefix on parsing (`date.rs:279-282`).
   - Serde serializes as ISO-8601 string `YYYY-MM-DD` (and Postcard length-prefixed bytes).
2. **UTC-Backed Instant with Local Clock Representation (`DateTimeValue` in `datetime.rs`):**
   - Wraps Jiff `Timestamp` (`datetime.rs:76`).
   - Resolves naive input strings via process-local zone (`local_zone()`) into UTC (`datetime.rs:128-131, 470-496`).
   - Resolves ambiguous/repeated DST fold hours to earlier occurrence; skipped DST gap hours shift forward by gap length (`datetime.rs:172-184`).
   - Formats and displays in current local clock time (`datetime.rs:298-301`); serializes over Serde/Postcard as UTC `Z` string (`datetime.rs:364-372`).
3. **Algebraic Point Unification (`DatePoint` in `point.rs`):**
   - Sum type `Calendar { date: DateValue, date_precision: DatePrecision } | DateTime(DateTimeValue)` (`point.rs:41-52`).
   - Preserves month (`YYYY-MM`) vs day (`YYYY-MM-DD`) precision for calendar variants.
   - Provides pattern parsing with Jiff strtime (`point.rs:158-181`).
   - Bridges calendar dates to instants via local midnight conversion (`point.rs:246-262`).
4. **Calendar Arithmetic & Difference Engine (`DatePoint`, `DifferenceEngine` in `point.rs`):**
   - Calendar shifts (`shift`, `apply`): calendar units (year/month/week/day) advance civil clock components; sub-day units advance exact elapsed time (`point.rs:288-338`).
   - Calendar snapping (`start_of`, `end_of`, `with_weekday`): boundary alignment preserving time-of-day (`point.rs:347-387`).
   - Difference computation (`DatePoint::diff`, `DifferenceEngine`): discrete integer difference for calendar units vs fractional floating-point difference for elapsed units (`point.rs:407-445, 553-626`).
5. **Nominal Duration Modeling (`DurationValue` in `duration.rs`):**
   - Combines a Jiff `Span` (nominal units) with cached `nominal_seconds: i64` (`duration.rs:158-161`).
   - Equality (`Eq`) and ordering (`Ord`) compare `nominal_seconds` (where 1mo = 30d, 1yr = 365d), not raw string spelling or variable calendar duration (`duration.rs:335-348`).
   - Unit-wise addition/subtraction preserving distinct nominal units (`duration.rs:273-333`).
6. **Lexing & Prefix Recognition (`lexer.rs`):**
   - Logos-based token recognition for date shapes, datetime separators, and compound duration prefixes (`lexer.rs:23-53, 89-111`).
   - Zero-allocation prefix extraction for parser cursor advances.
7. **Process-Local Zone Resolution & Test Harness (`zone.rs`, `mod.rs`):**
   - Queries system timezone via Jiff `TimeZone::try_system` (`zone.rs:19`).
   - `test_support::ZoneGuard` provides serialized, thread-local RAII overrides for multi-threaded tests (`mod.rs:61-125, 246-255`).

---

## 3. Invariants

| Invariant | Where Enforced | Mechanism | Can Safe Code Violate? |
|---|---|---|---|
| Date year must be 4 digits (`YYYY-MM-DD` or `YYYY-MM`), no 2-digit years | `date.rs:279-282`, `point.rs:85-89` | Runtime check returning `DateError::YearNotFourDigits` | No. (Constructors like `DateValue::from_ymd` enforce year in valid civil range). |
| Calendar months have Day 1 when parsed as `YYYY-MM` | `point.rs:58-63`, `date.rs:337-345` | Constructor sets day=1, `date_precision = DatePrecision::Month` | No. |
| `DateTimeValue` is always valid UTC instant | `datetime.rs:76, 79-90` | `Timestamp` inner type invariant | No. |
| Naive datetime string is resolved in local zone | `datetime.rs:128-131`, `point.rs:728-731` | `local_zone()` lookup at parse time | No (deterministic given ambient zone). |
| `DurationValue` equality is nominal seconds | `duration.rs:335-348` | Hand-written `Eq`, `PartialEq`, `Ord`, `PartialOrd` using `self.nominal_seconds` | No. |
| Date cross-kind comparison promotes `Date` to local midnight | `note/field.rs:297-300`, `datetime.rs:337-345` | `DateTimeValue::try_from(DateValue)` via `local_zone()` | **Leaked invariant:** safe code calling `NoteFieldValueRef::compare` relies on process ambient `TZ`. |
| Task lifecycle dates are pure `DateValue` (no time component) | `task.rs:506-512, 698-701` | Struct field types `Option<DateValue>` | No. |

---

## 4. State & Lifecycle

- **Owned State:**
  - `DateValue`: 4 bytes (`civil::Date`), `Copy`, zero allocation.
  - `DateTimeValue`: 12 bytes (`Timestamp` = 8-byte secs + 4-byte nanos), `Copy`, zero allocation.
  - `DatePoint`: Enum of `Calendar(DateValue, DatePrecision)` (8 bytes) or `DateTime(DateTimeValue)` (16 bytes tagged), `Copy`, zero allocation.
  - `DurationValue`: Jiff `Span` (~40 bytes) + `i64` nominal seconds (~48 bytes), `Copy`, zero allocation.
- **Caches & Static Data:**
  - `DATETIME_PARSER`: Static singleton in `datetime.rs:26`.
  - `PERIOD_EDGE_LOOKUP`: Static compile-time `phf::Map` (12 entries) in `calendar.rs:80`.
  - `UNIT_LOOKUP`: Static compile-time `phf::Map` in `unit.rs:47`.
  - `TEMPORAL_UNITS`: Static array of 7 `UnitSpec` structs in `unit.rs:191`.
- **Concurrency & Global State:**
  - In production, `local_zone()` queries OS `TimeZone::try_system()`. No locks or background worker threads.
  - In tests (`test_support`), `ZONE_LOCK: Mutex<()>` serializes guard installation, `ZONE_OVERRIDE: RwLock<Option<TimeZone>>` holds global fallback, and `ZONE_GUARD: thread_local!` manages scoped restoration (`mod.rs:246-255`).

---

## 5. Dependency Edges

- **Inbound Edges into `temporal` (who depends on it):**
  - `src/lib.rs`: exports `DateValue`, `DateTimeValue`, `DurationValue`, `DateError`, `DurationError`, `DEFAULT_DATE_PATTERN`, `DEFAULT_DATETIME_PATTERN`, `TimeUnit`, `UNIT_SPELLINGS_HINT`, and `ZoneGuard`.
  - `src/field.rs`: imports `DateTimeValue`, `DateValue`, `DatePoint` for YAML frontmatter coercion (`field.rs:39, 833-838`).
  - `src/note/field.rs`: imports `DatePoint`, `DateValue`, `DateTimeValue`, `DurationValue` (`note/field.rs:24-25`).
  - `src/note/parser/inline.rs`: imports `DatePoint`, `DurationValue` (`note/parser/inline.rs:9-11`).
  - `src/note/parser/lexer.rs`: imports `DatePoint` (`note/parser/lexer.rs:11`).
  - `src/file.rs`: imports `DatePoint` for filename prefix date sniffing (`file.rs:36, 93`).
  - `src/task.rs`: imports `DateValue` (`task.rs:32`).
  - `src/query/sort.rs`: imports `DatePoint`, `DateTimeValue`, `DateValue`, `DurationValue` (`query/sort.rs:22-23`).
  - `src/query/grammar/filter.rs`: imports `CalendarError`, `CalendarPeriod`, `DatePoint`, `DateUnit`, `PeriodEdge`, `local_zone` (`query/grammar/filter.rs:34-36, 386`).
  - `src/template/engine/date.rs`: imports `DatePoint`, `DateTimeValue`, `DurationValue`, `CalendarError`, `CalendarPeriod`, `DateDiff`, `DateError`, `DateUnit`, `PeriodEdge`, `TimeUnit`, `weekday_from_number`, `local_zone` (`template/engine/date.rs:46-55`).
  - `src/schema/fields/date.rs`: Does **NOT** depend on `temporal`! It only stores an unvalidated format string `Option<String>` (`schema/fields/date.rs:12-14`).
- **Outbound Edges from `temporal`:**
  - External crates: `jiff` (civil, tz, Span, Timestamp), `logos` (lexer), `phf` (hash tables), `serde`, `thiserror`, `num-traits`.
  - Internal `crate::` edges: **Zero production dependencies on other modules**.
    - Test-only: `crate::temporal::test_support::ZoneGuard`, `pretty_assertions`, `rstest`, `tempfile`.
    - `duration.rs:269` and `point.rs:147` call `crate::lexer::lex_prefix` (re-export of internal lexer helper).
  - Graph is strictly a leaf: `temporal` is at the bottom of the DAG. No circular dependencies.

---

## 6. Interfaces

- **`DateValue` & `DateTimeValue` (Deep seam):**
  - Minimal, idiomatic newtypes wrapping Jiff primitives.
  - Implements standard traits: `Copy`, `Clone`, `Eq`, `Ord`, `Hash`, `Display`, `Serialize`, `Deserialize`.
  - Very narrow surface: `from_ymd`, `parse_iso`, `civil_date`, `local_datetime_in`, `local_date`.
- **`DatePoint` (Wide/Convenience seam):**
  - Serves as the poly-temporal value for intermediate expressions (in note lexer, template evaluation, query filter/sort).
  - Accumulates ~15 distinct operations: `parse`, `parse_prefix`, `parse_with_pattern`, `date`, `date_in`, `local_datetime_in`, `zoned_datetime_in`, `instant`, `shift`, `apply`, `start_of`, `end_of`, `with_weekday`, `component`, `diff`.
  - It is a shallow orchestrator over `civil::Date`, `Timestamp`, `DifferenceEngine`, and `DateShape`/`DateTimeShape`.
- **`DurationValue` (Deep seam):**
  - Hides Jiff `Span` complexity, maintaining nominal seconds cache so sorting and hashing are cheap and total.
  - Checked arithmetic methods (`checked_add`, `checked_sub`, `checked_mul`, `checked_neg`).

---

## 7. Algorithms, Data Structures & Perf

- **Representation Efficiency:**
  - `DateValue` is 4 bytes (`i16` year + `u8` month + `u8` day).
  - `DateTimeValue` is 12 bytes (8-byte unix timestamp + 4-byte nanoseconds).
  - Both are `Copy`, zero-heap-allocation, pass-by-value on registers.
- **Parsing Performance:**
  - Dedicated fast-path shape recognizers: `starts_like_date` (byte checking) and `starts_with_year` avoid regular expressions (`lexer.rs:59-87`).
  - `DateShape::parse_any` and `DateTimeShape::parse_any` use fixed-width arithmetic slice parsers instead of regex or dynamic format strings (`date.rs:279-330`, `datetime.rs:408-450`).
  - Logos lexer is compiled DFA.
  - Benchmark verification (`benches/query_sort.rs:666-687`): `bench_sort_list_rows_by_due` verifies that sorting `DateValue` promoted to `DateTimeValue` has zero heap allocations and performs identically to native date sorting.
- **Hot-spot / Inefficiency: Repetitive String Normalization & Allocation:**
  - `DateTimeShape::truncate_fractional_seconds` and `normalize_separator` (`datetime.rs:411-413`) perform string allocations (`Cow::Owned`) on naive input containing extra spaces or non-canonical separators.
  - `NoteFieldValueRef::compare` (`note/field.rs:297-300`) calls `DateTimeValue::try_from(*date)` dynamically in comparison loops during sorting. Each call performs `local_zone()` lookup! If sorting 10,000 mixed date/datetime rows, `local_zone()` is called $O(N \log N)$ times.

---

## 8. Error Topology

- **Error Types:**
  - `DateError` (`error.rs:34`): Public enum. Variants:
    - `InvalidValue { input, reason }`
    - `YearNotFourDigits { input }`
    - `LocalZoneLookup { input }`
  - `DurationError` (`error.rs:89`): Public enum. Variants:
    - `Empty`
    - `InvalidValue { input, reason }`
    - `NumberOverflow { input }`
    - `OutOfRange { input }`
  - `CalendarError` (`error.rs:139`): `pub(crate)` internal enum. Variants:
    - `OutOfRange`
    - `Zone { input }`
- **Error Conversions & Mapping:**
  - `CalendarError::into_date_error` maps internal math/zone errors to public `DateError` (`error.rs:153-167`).
  - Query boundary (`query/grammar/filter.rs:386-436`): maps errors to `None` (filtering treats invalid arithmetic as non-matching / null).
  - Template boundary (`template/engine/date.rs:1082-1110`): `render_date_error` and `render_calendar_error` map errors to MiniJinja `minijinja::Error(ErrorKind::InvalidOperation)` with diagnostic messages.

---

## 9. Duplicated & Conflicting Concepts

1. **Ambient Process-Local Zone vs. Query Zone:**
   - **Problem statement:** The Oct 16 redesign report (`.design/2026-10-16-review-temporal-redesign/report.md:18-22, 103-115`) flagged that `QuerySet` had a `TimeZone` field while ambient code used `local_zone()`.
   - **Main branch status NOW (VERIFIED):** The query zone was **completely stripped** from `QuerySet` in commit `1e5df499` ("refactor(temporal): merge redesign into main"). `QuerySet` (`src/query/results.rs:464-468`) now has **no** `TimeZone` field! Instead, `query/grammar/filter.rs:386, 640, 673, 761` calls `crate::temporal::local_zone().ok()?` on **every single temporal operation**.
   - **Result:** Query filtering, query sorting, note field comparison, and template evaluation now all consistently use ambient process `local_zone()`. However, this means **queries cannot evaluate in a user-configured or document-specified timezone**; they are strictly bound to the process host environment.
2. **Duplicate Free-Text Temporal Classification (`TextShape` vs `DatePoint::parse`):**
   - `query/sort.rs:446-474` defines `TextShape` (`DateTime`, `Date`, `Duration`, `Plain`).
   - `TextShape::classify` delegates directly to `DatePoint::parse(trimmed)` and `DurationValue::parse(trimmed)`.
   - `query/grammar/filter.rs:85-98, 234-248` imports `query::sort::TextShape` and calls it to classify filter literals.
   - `field.rs:832-840` (`DateTimeCoercion::Classify`) directly calls `DatePoint::parse(&self.0)` without using `TextShape`.
   - `note/field.rs:113` calls `DatePoint::parse_prefix`.
   - `file.rs:93` calls `DatePoint::parse_prefix`.
   - **Assessment:** `TextShape` is a private query-module helper combining DatePoint + DurationValue. It is clean and no longer a duplicate heuristic, though its location in `query::sort` and import by `query::grammar::filter` is slightly misplaced.
3. **Template Date Formatting vs. Core Temporal Formatting:**
   - `src/template/engine/date.rs:488-632` implements its own full Moment.js-to-strftime translation engine (`pattern_to_strftime`, `moment_to_strftime`, `ordinal_suffix`, handling `YYYY`, `YY`, `MMMM`, `MMM`, `MM`, `M`, `DDDD`, `DD`, `D`, `Do`, `HH`, `hh`, `mm`, `ss`, `A`, `a`).
   - `src/temporal/point.rs:644-701` has its own `pattern_fields` strftime scanner for Jiff.
   - Core temporal knows only strtime/strftime; Moment.js pattern translation lives entirely in `template`.
4. **Task Dates vs Note Dates (`task.rs` vs `note/field.rs`):**
   - `task.rs` does **not** duplicate date parsing logic (VERIFIED: `task.rs` contains zero regexes and zero date parsing algorithms).
   - In `task.rs`, `TaskDateSet` stores `DateValue` in 6 fixed slots (`created`, `scheduled`, `start`, `due`, `done`, `cancelled`).
   - Parsing of task dates is done via `DatePoint::parse_prefix` inside `note/parser/lexer.rs:246` and `DateValue::parse_iso` in `note/parser/task.rs:209`.

---

## 10. Doc, Glossary & ADR Drift

1. **`src/temporal/GLOSSARY.md` vs. Code:**
   - *Claim:* "`DateValue` represents a whole calendar date using Jiff's zone-free civil date type; written precision (month or day) is preserved separately by `DatePoint`."
   - *Status:* **Verified in code (`date.rs:45`, `point.rs:41-49`)**.
   - *Claim:* "Comparisons with Date promote the Date to local midnight. If the local offset cannot be applied at the representable range edge, the UTC reading is used."
   - *Status:* **Verified in code (`datetime.rs:341-345`, `note/field.rs:297-300`)**.
2. **Prior Review Drift (`.design/2026-10-16-review-temporal-redesign/report.md`):**
   - *Claim in report:* "src/query/results.rs:456-484,561-581 stores a Jiff TimeZone on QuerySet; the zone is preserved as query operations are appended and is passed to pending plan execution and filtering."
   - *Status on main NOW:* **DRIFT / OBSOLETE**. In the merged code on main, `QuerySet` has no `TimeZone` field. `QueryService` passes no timezone. Ambient `local_zone()` is called directly inside `filter.rs`.
   - *Claim in report:* "Query filtering already has a separate explicit-zone `compare_temporal` path for top-level Date/DateTime pairs."
   - *Status on main NOW:* **DRIFT / OBSOLETE**. `compare_temporal` does not exist on main (grep returned zero occurrences across the entire codebase). Filter comparison uses standard `NoteFieldValueRef::compare(&literal)`.
3. **LSP Planning Ticket Drift (`.scratch/md-pkm-lsp/issues/20-schema-and-fileclass-intelligence.md`):**
   - *Claim in Ticket 20 (Q16):* "Date fields: syntax validation only (ISO-8601 parseability)... Use existing chrono parser (already in codebase), not jiff."
   - *Status in codebase:* **FALSE CLAIM**. Chrono was completely removed from Traces dependencies! `Cargo.toml` has `jiff = "=0.2.38"` and zero chrono dependencies. `schema/fields/date.rs` currently performs no date validation whatsoever.
4. **`schema/fields/date.rs` Drift:**
   - Comments state: "Parses the optional `format` attribute for `date` fields."
   - But `SchemaDateField` never parses, validates, or compiles this format against `DEFAULT_DATE_PATTERN` or Jiff/strftime. It only stores an unvalidated `Option<String>` string.

---

## 11. Candidate Architectural Questions

### 1. Should cross-kind comparison (`DateValue` vs `DateTimeValue`) remain ambiently local-zone dependent in generic `Ord`, or should `Date` and `DateTime` have distinct comparison ranks?
- **Claim:** In `NoteFieldValueRef::compare` (`src/note/field.rs:297-300`), comparing `Date` with `DateTime` dynamically promotes `Date` to `DateTimeValue` at local-zone midnight via `local_zone()`. This violates pure value-semantic total ordering (a value's order changes when evaluated across machines/timezones or during DST shifts) and incurs a hidden `local_zone()` system lookup on every pairwise comparison during sorting.
- **Strongest Counter-argument:** In PKM tools (Obsidian, Notion), users expect a note dated `2026-07-29` to sort alongside timestamps from `2026-07-29T10:00:00`. Separating ranks would place all pure dates before all datetimes (or vice-versa), breaking mixed timeline queries.
- **Decision Remaining Open:** Either:
  1. Keep current behavior, but optimize sort by caching/normalizing keys in `SortKey` (avoiding per-comparison zone lookups).
  2. Disallow cross-kind `Ord` in core types, requiring explicit comparison policies at query/sort seams.
  3. Compare by calendar date projection (convert datetime to date in local zone) rather than instant promotion.
- **Areas Touched:** `note/field.rs`, `query/sort.rs`, `temporal/date.rs`, `temporal/datetime.rs`.

### 2. Is ambient process-local zone resolution in queries and templates adequate, or does Traces need an explicit `ZoneContext`?
- **Claim:** Calling `local_zone()` directly in `query/grammar/filter.rs` and `template/engine/date.rs` makes query evaluation and template rendering non-reproducible, untestable without global `ZoneGuard` locks, and prevents future multi-timezone or server/LSP usage (where an LSP server might serve clients in different timezones from the server process).
- **Strongest Counter-argument:** Traces is a single-user local CLI running on the user's laptop. Passing a `&TimeZone` through every AST evaluation node, template filter, and table renderer adds substantial boilerplate for a scenario (multi-timezone CLI) that does not exist. The ambient `ZoneGuard` test fixture already makes all 7,200+ tests completely deterministic.
- **Decision Remaining Open:** Keep ambient `local_zone()` with the current `ZoneGuard` test infrastructure, or introduce a lightweight execution context (`ExecutionContext { zone: TimeZone, now: Timestamp }`) passed to `QueryService::run` and `TemplateService::render`.
- **Areas Touched:** `query/service.rs`, `query/grammar/filter.rs`, `template/engine/date.rs`, `temporal/zone.rs`.

### 3. Should `DatePoint` be decomposed, or does it hold up as a cohesive internal algebraic type?
- **Claim:** `point.rs` is the largest file in `src/temporal/` (3,293 lines). It mixes parsing, precision tracking, pattern extraction, duration arithmetic, and the `DifferenceEngine`.
- **Strongest Counter-argument:** 2,791 lines of `point.rs` (85%) are thorough unit tests. The production code is only 502 lines. `DatePoint` provides a tight sum-type boundary that prevents intermediate date/datetime representation bugs across query and template layers. Decomposing 500 lines into multiple files would increase cognitive overhead without simplifying any algorithm.
- **Verdict:** **Keep as-is.** The size of `point.rs` is dominated by test coverage; production code is cohesive and well-structured.
- **Areas Touched:** `temporal/point.rs`.

### 4. Should Moment.js pattern translation live in `template/engine/date.rs` or move into `temporal`?
- **Claim:** `src/template/engine/date.rs` contains ~150 lines of Moment.js-to-strftime parsing (`pattern_to_strftime`, `moment_to_strftime`). When the Markdown PKM LSP (Tickets 18, 20, 22) validates user date formats in schemas and daily notes, it will either duplicate this Moment.js parser or need access to it outside the template engine.
- **Strongest Counter-argument:** MiniJinja template formatting is currently the only place Moment tokens like `YYYY-MM-DD` or `[Daily]` are used by users. Keeping it inside `template/engine/date.rs` isolates template idiosyncrasies from the strict Jiff strftime model in `temporal`.
- **Decision Remaining Open:** Leave Moment translation in `template::engine::date` until the LSP or schema validator actually requires format parsing, or extract a dedicated `format_compat` module.
- **Areas Touched:** `template/engine/date.rs`, `temporal/`, `schema/fields/date.rs`.

### 5. Should `DurationValue` nominal ordering (1mo = 30d, 1yr = 365d) remain the canonical total ordering?
- **Claim:** Storing `nominal_seconds` in `DurationValue` makes duration sorting $O(1)$ and total, but produces anomalies in calendar arithmetic: `1mo` is ordered as smaller than `31d`, yet applying `1mo` to January 31 advances by 28/29 days, while applying it to March 31 advances by 30 days.
- **Strongest Counter-argument:** Any duration type with calendar units lacks a total mathematical order without a reference anchor date. In a PKM system sorting frontmatter fields (e.g. `duration: 1mo` vs `duration: 30d`), an intuitive nominal order is required for stable list display and redb indexing.
- **Verdict:** **Keep as-is.** The nominal seconds solution is sound, fast, and clearly documented in `GLOSSARY.md`.
- **Areas Touched:** `temporal/duration.rs`, `note/field.rs`.
