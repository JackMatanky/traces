# Strategic Architectural Survey: Temporal

> **Survey baseline: `2dc3250d`. Re-verification note (main @ `207cc3fb`).** `src/task.rs` changed only at `TaskStatusMap::iter`; `src/temporal` is untouched. `src/temporal/format.rs` in H5 is a proposed destination, not an existing file. Authoritative corrections: `reverification.md`.

## 1. Responsibilities & seams

- **Domain ownership:** Owns parsing, validation, representation, formatting, and calendar arithmetic for civil dates (`DateValue`), UTC instants rendering in local civil time (`DateTimeValue`), durations with nominal identity (`DurationValue`), and poly-temporal algebraic points (`DatePoint`) (`src/temporal/mod.rs:1-31`). Single source of truth for temporal semantics in Traces (`src/temporal/GLOSSARY.md:3-4`).
- **SLoC & essential vs. accidental breakdown:**
  - Total `src/temporal/*.rs`: 10,308 lines. Production code: 3,046 lines (29.5%). Tests: 7,262 lines (70.5%).
  - Breakdown (prod / test / total): `point.rs`: 502 / 2,791 / 3,293; `datetime.rs`: 615 / 1,256 / 1,871; `duration.rs`: 439 / 1,264 / 1,703; `date.rs`: 390 / 768 / 1,158; `unit.rs`: 364 / 356 / 720; `mod.rs`: 255 / 289 / 544; `calendar.rs`: 126 / 222 / 348; `error.rs`: 169 / 173 / 342; `lexer.rs`: 149 / 72 / 221; `zone.rs`: 37 / 71 / 108.
  - *Essential complexity:* Proleptic Gregorian calendar rules, variable month lengths, leap years, DST fold/gap handling, civil-clock vs elapsed-time dual regime, discrete vs fractional difference engine, nominal duration identity for redb B-tree index total ordering, month vs day precision.
  - *Accidental complexity:* 4 heterogeneous parser dependencies (`logos`, `winnow`, `speedate`, `jiff`), redundant `speedate` dependency for 1 function (`date.rs:296`), ~150 lines of Moment.js formatting logic isolated in `template/engine/date.rs:488-623`, `Cow::Owned` heap allocations on non-canonical datetimes (`datetime.rs:485-534`), and ambient `local_zone()` queries in queries/comparisons requiring 200 lines of `ZoneGuard` thread-local override locks (`mod.rs:58-255`).
- **Public surface (`src/temporal/mod.rs:47-52`):**
  - Public types: `DateValue`, `DateTimeValue`, `DurationValue`, `DateError`, `DurationError`.
  - Crate-internal (`pub(crate)`) types (`src/temporal/mod.rs:42-55`): `DatePoint`, `DateDiff`, `DatePrecision`, `DateUnit`, `TimeUnit`, `CalendarPeriod`, `PeriodEdge`, `resolve_period_edge`, `weekday_from_number`, `DEFAULT_DATE_PATTERN`, `DEFAULT_DATETIME_PATTERN`, `UNIT_SPELLINGS_HINT`, `local_zone`, and `test_support::ZoneGuard`.
- **Callers & consumers:**
  - `note`: `note/field.rs:24-25` (NoteFieldValue variants, cross-kind compare), `note/parser/inline.rs:9-11` (duration prefix parsing), `note/parser/lexer.rs:11, 246` (task date prefix parsing), `note/parser/task.rs:12, 209` (task date ISO parsing).
  - `query`: `query/grammar/filter.rs:35-36` (filter arithmetic/boundary evaluation), `query/sort.rs:22-23, 446-476` (sort key normalization, `TextShape` classification), `query/results.rs:24, 274-292` (metadata date projection).
  - `template`: `template/engine/date.rs:44-50` (MiniJinja `date` namespace, filters, tests, Moment.js dialect translation).
  - `root / storage`: `field.rs:39, 833-850` (YAML frontmatter coercion), `file.rs:52, 91-98` (filename date sniffing for `FileMeta`), `task.rs:32, 507-512` (`TaskDateSet` 6 date slots), `config/model/frontmatter.rs:260` (default datetime format).
  - `schema`: `schema/fields/date.rs:12-14` is decoupled; stores raw unvalidated `Option<String>` format without calling `temporal`.
- **What callers must know:** Callers hold validated, `Copy`, zero-allocation values. Arithmetic errors map to `None` in queries (`query/grammar/filter.rs:386`) or `TemplateEngineResult` in templates (`template/engine/date.rs:121`). Callers must know `DateTimeValue` displays in local clock time but serializes to UTC RFC 3339 (`src/temporal/datetime.rs:359-373`).

## 2. Invariants

- **[I1] 4-digit ASCII year prefix:** `DateValue` and `DateTimeValue` require exactly 4 ASCII year digits (or `-YYYY`), rejecting 2-digit years. Enforced at parse entry (`src/temporal/date.rs:85-90`, `src/temporal/datetime.rs:164-168`, `src/temporal/point.rs:85-89`). Constructors (`from_ymd`, `from_unix_seconds`) enforce numeric range `[-9999..=9999]`. Safe code cannot violate via parsing.
- **[I2] Zone-free civil date proleptic Gregorian:** `DateValue(civil::Date)` carries no timezone and no time of day (`src/temporal/date.rs:43-45`). `DatePoint::Calendar` preserves month (`YYYY-MM`) vs day (`YYYY-MM-DD`) precision, normalizing month input to day 1 (`src/temporal/point.rs:41-49, 58-63`).
- **[I3] Instant stored as UTC, rendered as local civil time:** `DateTimeValue(Timestamp)` always stores UTC instant (`src/temporal/datetime.rs:74-76`). Naive input resolves in ambient `local_zone()` (`src/temporal/datetime.rs:128-131, 567-571`). `Display` renders local clock without offset (`src/temporal/datetime.rs:191-193, 298-301`); Serde serializes with `…Z` UTC RFC 3339 (`src/temporal/datetime.rs:364-372`).
- **[I4] Deterministic DST fold/gap resolution:** Naive local times in DST fall-back folds choose earlier occurrence; spring-forward gaps shift forward by gap length (`src/temporal/datetime.rs:115-119, 144-146`).
- **[I5] Nominal duration identity for equality, hashing, ordering:** `DurationValue` wraps Jiff `Span` and caches `nominal_seconds: i64` (`src/temporal/duration.rs:157-161`). `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash` compare only `nominal_seconds` (1mo = 30d, 1yr = 365d) (`src/temporal/duration.rs:541-568`), ignoring written spelling or variable calendar duration.
- **[I6] Calendar regime vs elapsed regime arithmetic:** In `DatePoint::shift`, `apply`, and `diff`, calendar units (year/month/week/day) advance/measure civil local clock time; exact units (hour/minute/second) advance/measure physical elapsed time (`src/temporal/point.rs:288-338, 407-445`).
- **[I7] Cross-kind Date vs DateTime comparison promotes Date to local midnight:** Leaked invariant split across owners. `NoteFieldValueRef::compare` invokes `DateTimeValue::try_from(*date)` (`src/note/field.rs:297-300`), which calls `local_zone()` (`src/temporal/datetime.rs:341-344`). Core value comparison relies on process ambient `TZ`.

## 3. Verified observations

- **[V1] Massive test-to-code ratio:** `src/temporal/*.rs` comprises 10,308 lines: 3,046 lines production code (29.5%) and 7,262 lines test code (70.5%). `point.rs` has 502 prod lines vs 2,791 test lines (`src/temporal/point.rs:1-502, 503-3293`).
- **[V2] Redundant speedate dependency:** `speedate = "0.18.0"` is in `Cargo.toml:143` and called in exactly ONE place: `DateShape::parse_year_month_day` (`src/temporal/date.rs:296`). Negative years and `YYYY-MM` bypass speedate and use hand-rolled string slicing (`src/temporal/date.rs:314-326, 337-358`).
- **[V3] Heterogeneous multi-engine lexer stack:** Temporal parsing mixes 4 frameworks: `logos` (`src/temporal/lexer.rs:17-54`), `winnow` (`src/temporal/lexer.rs:118-148`), `speedate` (`src/temporal/date.rs:296`), and `jiff` (`src/temporal/datetime.rs:26, 434`).
- **[V4] Ambient local zone queries in query loops:** `query/grammar/filter.rs:386, 399, 411, 435, 640, 646, 673, 699, 761` invokes `crate::temporal::local_zone().ok()?` on every temporal filter expression evaluation; `query/results.rs:274, 284, 292` invokes `local_zone()` per file row for `file.cday`/`file.mday`.
- **[V5] Semantic discrepancy between NoteFieldValue and NoteFieldValueRef equality:** `NoteFieldValue` derives `PartialEq` (`src/note/field.rs:42`), where `Date` vs `DateTime` variants are always unequal. `NoteFieldValueRef::eq` delegates to `compare()` (`src/note/field.rs:468`), which promotes `Date` to local midnight and returns `Equal` if it matches the datetime. Thus `a == b` is false while `a.as_ref() == b.as_ref()` can be true.
- **[V6] Moment.js format engine trapped in template:** `template/engine/date.rs:488-623` hand-rolls 135 lines of Moment.js-to-strftime translation (`moment_to_strftime`, `MOMENT_TOKENS`, `ordinal_suffix`, bracket literals). `schema/fields/date.rs:12-14, 48-52` accepts a date `format` attribute from TOML schema but never parses or validates it.
- **[V7] Temporary string allocations in datetime parsing:** `DateTimeShape::parse_any` calls `truncate_fractional_seconds` and `normalize_separator` (`src/temporal/datetime.rs:411-412, 485-534`), which allocate `Cow::Owned(String)` when fractional digits > 9 or multi-space separators exist.
- **[V8] Task dates are strictly civil dates:** `src/task.rs:506-512` stores `TaskDateSet` with 6 fixed `Option<DateValue>` fields. Zero time components or zone dependencies exist in tasks.
- **[V9] Temporal is an architectural leaf:** `src/temporal/` has zero dependencies on any other Traces module (`src/temporal/mod.rs:1-56`).

## 4. Hypotheses

- **[H1] (Rank 1) Eliminate speedate dependency in favor of Jiff / hand-rolled byte parsing.**
  - *Thesis:* `speedate = "0.18.0"` is an unnecessary third-party dependency. Positive `YYYY-MM-DD` can be parsed via Jiff's `civil::Date::strptime` or direct byte parsing (like `date.rs:317-324`).
  - *Current vs Alt:* Current calls `speedate::Date::parse_bytes_rfc3339` then copies to `civil::Date`. Alternative: parse directly into `civil::Date` via ASCII slice math. Keep as-is retains redundant dependency.
  - *Removable:* `speedate` dependency in `Cargo.toml:143`, intermediate speedate type mapping in `src/temporal/date.rs:296-312`.
  - *Disproof:* If Criterion micro-benchmarks prove `speedate` is >5x faster than direct slice math and date parsing is a demonstrated macro-bottleneck in `traces index`.
  - *Blast radius:* Local to `src/temporal/date.rs:296-312` and `Cargo.toml`.
- **[H2] (Rank 2) Pass explicit ExecutionContext / ZoneContext to query and template evaluation instead of ambient `local_zone()`.**
  - *Thesis:* Calling `local_zone()` repeatedly in AST nodes (`query/grammar/filter.rs:386-761`, `template/engine/date.rs:121`) causes redundant OS `TimeZone::try_system` lookups, prevents query timezone parameterization, and necessitates complex global `ZoneGuard` locking for test determinism (`src/temporal/mod.rs:58-202`).
  - *Current vs Alt:* Current queries ambient OS zone on every AST leaf. Alternative: `QueryService::run` and `TemplateEngine::render` accept `&ExecutionContext { zone: TimeZone, now: Timestamp }`.
  - *Removable:* Hundreds of repeated `local_zone()` calls in loops; global `ZONE_LOCK` and `ZONE_OVERRIDE` in `src/temporal/mod.rs:246-255`.
  - *Disproof:* If passing `&ExecutionContext` requires pervasive lifetime annotations or ergonomics degradation in MiniJinja callbacks that outweigh the determinism benefits.
  - *Blast radius:* `query/service.rs`, `query/grammar/filter.rs`, `template/engine/date.rs`, `temporal/zone.rs`.
- **[H3] (Rank 3) Separate cross-kind Date vs DateTime comparison from generic `Ord` into query-specific evaluation policy.**
  - *Thesis:* Having `NoteFieldValueRef::compare` promote `Date` to `DateTime` via ambient `local_zone()` (`src/note/field.rs:297-300`) violates pure value semantics, breaks consistency with `NoteFieldValue` (`src/note/field.rs:42`), and leaks timezone into generic sorting.
  - *Current vs Alt:* Current promotes in generic `compare()`. Alternative: In generic `Ord`, `Date` and `DateTime` are distinct ranks (total order). Midnight promotion is an explicit query-level coercion policy applied where user intent requires it (e.g. `SortKey::from_note_ref` in `query/sort.rs:376-377`).
  - *Removable:* Ambient zone dependence in `NoteFieldValueRef::compare`, discrepancies between `NoteFieldValue` and `NoteFieldValueRef` equality.
  - *Disproof:* If existing test suites or user workflows rely on generic `BTreeSet<NoteFieldValueRef>` ordering mixed dates and datetimes together.
  - *Blast radius:* `note/field.rs:297-300`, `query/sort.rs:376-377`, `query/grammar/filter.rs`.
- **[H4] (Rank 4) Extract Moment.js dialect parser to `temporal::format` for reuse by schema and LSP.**
  - *Thesis:* `template/engine/date.rs:488-623` hand-rolls Moment token translation. When `schema/fields/date.rs:13` validates user date formats or the PKM LSP checks daily note filenames, they will either duplicate this logic or need access to it.
  - *Current vs Alt:* Current confines Moment translation to `template/engine/date.rs`. Alternative: Move translation into `src/temporal/format.rs` with public pattern validation. Keep as-is leaves `schema/fields/date.rs` format unvalidated.
  - *Removable:* Future duplication in schema and LSP crates; unvalidated format risk.
  - *Disproof:* If Traces deprecates Moment tokens in templates in favor of standard strftime `%Y-%m-%d`, making the translator dead code.
  - *Blast radius:* `template/engine/date.rs:488-623`, `temporal/`, `schema/fields/date.rs`.
- **[H5] (Rank 5) Replace winnow duration prefix parser with unified scanner.**
  - *Thesis:* Using `winnow` solely for `parse_duration_prefix` (`src/temporal/lexer.rs:118-148`) adds a second parser framework alongside `logos`. A simple byte-scanning loop or Logos rule can find duration prefix boundaries before delegating to `DurationValue::parse`.
  - *Current vs Alt:* Current uses winnow combinators (`alt`, `take_while`, `digit1`). Alternative: Hand-rolled byte boundary scanner matching prefix `[0-9a-zA-Z.]+`.
  - *Removable:* `winnow` dependency from `src/temporal/lexer.rs:7-12`.
  - *Disproof:* If edge cases in compound duration prefix slicing (e.g. `1h 30m after`) are significantly more error-prone without parser combinators.
  - *Blast radius:* Local to `src/temporal/lexer.rs:118-148`.
- **[H6] (Rank 6) Eliminate Cow allocations in `DateTimeShape::parse_any`.**
  - *Thesis:* `truncate_fractional_seconds` and `normalize_separator` (`src/temporal/datetime.rs:485-534`) allocate `Cow::Owned(String)` when inputs have >9 fractional digits or multiple spaces. Jiff's `DATETIME_PARSER.parse_pieces` can parse directly from byte slices.
  - *Current vs Alt:* Current allocates formatted String. Alternative: Slicing bytes directly or in-place stack buffer (max 32 bytes for ISO datetime string).
  - *Removable:* Heap allocations on non-canonical naive datetime inputs.
  - *Disproof:* If non-canonical datetime strings are sufficiently rare in Markdown files that micro-allocations have no measurable impact.
  - *Blast radius:* `src/temporal/datetime.rs:485-534`.
- **[H7] (Rank 7) Retain `DatePoint` algebraic sum type and `DifferenceEngine` (Keep As-Is).**
  - *Thesis:* `DatePoint` (`src/temporal/point.rs:40-52`) is cohesive and only 502 production lines. It cleanly encapsulates the three temporal regimes (calendar discrete, civil-clock wall time, physical elapsed) in `DifferenceEngine` (`src/temporal/point.rs:553-626`).
  - *Current vs Alt:* Current unified sum-type vs splitting into separate date and datetime arithmetic modules.
  - *Removable:* Splitting would fragment cohesive calendar arithmetic across multiple files without line savings.
  - *Disproof:* If consumers frequently downcast or if `DatePoint` leaks time-of-day complexity into pure date contexts.
  - *Blast radius:* None (keep as-is).
- **[H8] (Rank 8) Retain `DurationValue` nominal identity cache (Keep As-Is).**
  - *Thesis:* Caching `nominal_seconds: i64` in `DurationValue` (`src/temporal/duration.rs:157-161`) provides $O(1)$ total ordering, equality, and hashing (`src/temporal/duration.rs:541-568`) for database indexing and query sorting, while preserving the underlying `Span` for calendar arithmetic.
  - *Current vs Alt:* Current cached nominal seconds vs dynamically anchoring to an instant or disallowing `Ord`.
  - *Removable:* Anchoring would require passing reference timestamps to all sorting/indexing calls.
  - *Disproof:* If user PKM data suffers semantic bugs due to `1mo` (30d) sorting before `31d`.
  - *Blast radius:* None (keep as-is).

## 5. Cross-slice couplings

- **`temporal` <-> `note` (`src/note/field.rs:24-25, 297-300`, `src/note/parser/inline.rs:9-11`):** `NoteFieldValueRef::compare` invokes `DateTimeValue::try_from(DateValue)` which queries `local_zone()`. Note field parser imports `DatePoint::parse_prefix` and `DurationValue::parse_prefix`.
- **`temporal` <-> `query` (`src/query/grammar/filter.rs:35-36`, `src/query/sort.rs:22-23, 446-476`):** Filter grammar directly imports `CalendarError`, `CalendarPeriod`, `DatePoint`, `DateUnit`, `PeriodEdge`, `TimeUnit`, and `local_zone`. Query AST evaluates temporal operations with inline ambient `local_zone()` lookups. `TextShape` in `query/sort.rs:446-476` wraps `DatePoint::parse` and `DurationValue::parse`.
- **`temporal` <-> `template` (`src/template/engine/date.rs:44-50`):** Heavy consumer importing 11 temporal types. Registers `date` namespace and 15+ filters/tests. Hand-rolls Moment.js translation.
- **`temporal` <-> `file` / `task` (`src/file.rs:52, 91-98`, `src/task.rs:32, 507-512`):** `FileMeta` caches `Option<DateValue>` from filename date prefix. `TaskDateSet` stores 6 `Option<DateValue>` fields.
- **`temporal` <-> `schema` (`src/schema/fields/date.rs:12-14`):** Zero coupling today; schema field stores unvalidated format string, representing a missing validation seam.

## 6. Comparables worth studying

- **`jiff` (`jiff::civil::Date`, `jiff::Timestamp`, `jiff::Span`, `jiff::fmt::strtime`):** Upstream crate. Informs [H1], [H5], [H6]. Study Jiff's native `civil::Date::strptime` and `DateTimeParser` zero-allocation slice parsing to eliminate `speedate` and `winnow`.
- **`DuckDB` (`src/include/duckdb/common/types/date.hpp`, `timestamp.hpp`):** Informs [H3]. Study how DuckDB handles `DATE` vs `TIMESTAMP` type comparison and casting rules without ambient timezone contamination in generic row comparators.
- **`Obsidian Dataview` (`src/data-model/value.ts`, `src/util/normalize.ts`):** Informs [H3], [H8]. Study how Dataview unifies Luxon `DateTime` and `Duration` objects, handles nominal sorting, and promotes dates to midnight in user query expressions.
- **`DataFusion` (`datafusion/expr/src/timezone.rs`):** Informs [H2]. Study how analytical query engines pass `SessionConfig` / `ExecutionProps` carrying execution timezone and timestamp instead of reading process ambient environment.

## 7. Open decisions

- **[OD-1] Explicit ExecutionContext in Query and Template:** Candidate ticket: `refactor(query,template): introduce ExecutionContext to eliminate ambient local_zone lookups`. Sharply statable.
- **[OD-2] De-ambientize NoteFieldValueRef cross-kind comparison:** Candidate ticket: `refactor(note): make NoteFieldValueRef::Ord pure by decoupling cross-kind Date/DateTime promotion`. Sharply statable.
- **[OD-3] Drop speedate dependency:** Candidate ticket: `chore(temporal): replace speedate with Jiff/slice parser in DateShape`. Sharply statable.
- **[OD-4] Unify Date Format Dialects:** Candidate ticket: `refactor(temporal): extract Moment/strftime format translator for template and schema validation`. Sharply statable.
- **[Fog-1] Zoned DateTimes in PKM metadata:** Unclear whether users will ever require explicit per-note or per-field timezone preservation (e.g. `2026-07-29T14:30:00+02:00[Europe/Paris]`) rather than normalizing everything to UTC + local clock.

## 8. Withstands scrutiny

- **`DateValue` and `DateTimeValue` as transparent `Copy` newtypes (`src/temporal/date.rs:43-45`, `src/temporal/datetime.rs:74-76`):** Wrapping Jiff's `civil::Date` (4 bytes) and `Timestamp` (12 bytes) with `#[repr(transparent)]` gives zero-allocation value semantics, register passing, and strict validation.
- **`DurationValue` nominal seconds cache (`src/temporal/duration.rs:157-161`):** Caching `nominal_seconds: i64` alongside Jiff `Span` provides $O(1)$ total order and hashing for Redb index keys and sort comparators, solving the undecidable calendar duration ordering problem cleanly.
- **Calendar vs Elapsed regime arithmetic in `DifferenceEngine` (`src/temporal/point.rs:553-626`):** Separating whole calendar differences (accounting for variable month lengths and DST) from exact elapsed differences matches real-world calendar intuition without numerical drift.
- **Strict 4-digit year rule (`src/temporal/date.rs:85-90`):** Enforcing 4 ASCII digits prevents catastrophic ambiguity with 2-digit years (`26-08-22` as 1926 vs 2026 vs August 22, 2026).
