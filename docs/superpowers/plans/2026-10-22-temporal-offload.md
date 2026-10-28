# Temporal Offload Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move every calendar rule, unit/spelling lookup, constructor, and pattern-translation decision out of the template engine into `src/temporal/`, so the temporal crate is the single owner of date/time semantics and the engine is left with argument coercion, `ErrorKind` selection, and minijinja wiring only.

**Architecture:** The engine (`src/template/engine/date.rs`) currently hand-rolls a leap-year rule, unit-spelling messages, namespace period-edge tables, `now`/`today` construction, `start_of`/`end_of` dispatch, the moment-token translator, and four bespoke error messages. Each task below offloads one concern behind an existing or new `pub(crate)` seam in `src/temporal/`, deletes the duplicated engine copy, and pins behavior with tests before/while moving. The plan also lands the negative-year rendering fix (`temporal/fmt.rs`) and closes the audit's documentation findings.

**Tech Stack:** Rust 2024, jiff 0.2.38 (civil/zoned time), minijinja 3 (engine), serde, rstest + proptest + pretty_assertions (tests), cargo-nextest, `mise` gates (`verify`, `check:visibility`, `doc`), `hk` hooks.

---

## Conventions (apply to every task)

- **Order is load-bearing.** Execute tasks 1→17 exactly. Later tasks cite line numbers produced by earlier ones; `Task 6` deletes ~130 lines from `engine/date.rs` and `Task 10` rewrites `datetime.rs` regions, so **re-verify every line anchor by symbol name (grep) before editing**.
- **Line numbers** below were captured on 2026-10-23 against a clean tree (only `.scratch/arch-design/**` is dirty; do **not** touch those three paths).
- **TDD:** write the failing test first, watch it fail, implement, watch it pass. Where a refactor is behavior-equivalent by construction, the plan says so and marks the pin (a new test or the existing tests that move with the code, green before and after) instead of a fake red (the note under the test says which it is).
- **Gates:** end every task with `mise run verify` (runs `fmt`, then `check` + `lint --no-fix` + `test` in parallel; `check` includes `check:visibility`, which diffs both API baselines and the pub-token snapshot).
- **Baseline regen** (visibility-changing tasks only; the regen form of the snapshot check at `mise.toml:143`, produces a sorted snapshot; expect exactly the listed line delta):

  ```bash
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  ```

  `tests/baselines/public-api-default.txt` regen (`cargo public-api --color=never > tests/baselines/public-api-default.txt`) happens **only in Task 14 (O4)** if its gate selects Branch A. `tests/baselines/public-api-test-utils.txt` regen (`cargo public-api --features test-utils --color=never > tests/baselines/public-api-test-utils.txt`) happens under that same gate. No other task may change the public API.
- **Test commands:** fast loop `mise run test:unit <name-substring>` (all-features nextest, name filter; a filter matching zero tests in every phase is an error). Full suite = `mise run verify`. Single module: `mise run test:unit -m temporal`.
- **Commits:** one per task, conventional style matching `git log` (`refactor(temporal): …`, `refactor(template): …`, `docs(temporal): …`). Stage only the files the task touched, never the three dirty `.scratch/arch-design` paths.
- **Docs touched by tasks** live under `.scratch/temporal-core/` (spec, review, divergence register, issue ACs) and `src/temporal/GLOSSARY.md`.
- **Prose:** a task's `Context:` paragraph is a research summary, not an instruction; it sits outside Simplified Technical English (STE). The `- [ ]` steps are the instructions, written in STE.

## Task order and baseline deltas

| # | ID | What | pub-tokens delta |
|---|----|------|------------------|
| 1 | O1 | Leap-year rule → `DateValue::in_leap_year` (+ AC edit) | +1 |
| 2 | O3 | Namespace fall-through via `resolve_period_edge`, uniform lowercase keys | none |
| 3 | O2 | `UnitError` + `TimeUnit::parse_spelling` + hint↔registry sync test | +2, 1 line rewrite (mod.rs) |
| 4 | O5 | `DateTimeValue::now()` / `DateValue::today()`, lazy-zone collapse | +3 |
| 5 | O6 | `DatePoint::snap` replaces `start_of`/`end_of` (engine + query + tests) | −1 |
| 6 | P1 | Pattern translator extracted to `src/temporal/token.rs` (+ spec/review amendments) | +3 |
| 7 | X4 | Run-based moment-token matching (`match_moment_run`) + exhaustive matrix | none |
| 8 | X5 | Bracket-aware `%` detection, dead-branch delete, docs move here | +1 |
| 9 | H2 | `DateNamespace` → `DateOps`, stateless `register(env)` (Debug kept) | 2 lines rewrite |
| 10 | X1a | `src/temporal/fmt.rs` canonical encoder; Display/Serialize rewrite; proptests | +2 |
| 11 | X1b+X6 | Canonical shortcut in `format_strtime`, zoned-first reorder (Apia test) | none |
| 12 | X8b | `CalendarError::Zone { source: DateError }` forwards the inner Display (kills double-prefix) | none |
| 13 | O7 | Collapse both renderers onto temporal `Display` (origin rule) | none |
| 14 | O4 | **Gated:** `DatePoint::format_in` + `DateError::InvalidFormat` (probe decides) | +1, public-api regen (Branch A only) |
| 15 | H1 | `src/template/engine/duration.rs` + `DurationOps` + `engine.rs` doc truth | +2 |
| 16 | D | Doc-only fixes (X3, X7a-d, D5, GLOSSARY "seven", H5 nits, X1c) | none |
| 17 | F1 | Query unit rejections quote the `UnitError` hint (dynamic `expected`) | none |

**Accepted behavior changes** (each is pinned by a test in its task): case-insensitive `date.*` keys (superset, Task 2) · template zone/range messages adopt temporal `Display` wording (Task 13) · `[%Y]` renders literal `%Y`, unclosed `[…%` errors, the unclosed-bracket error precedes the token walk, and `UnclosedBracket`'s Display drops `moment-dialect` (Task 8) · runs longer than any real token now error instead of silently composing (Task 7) · zone resolution runs before pattern translation, so a range failure wins over an invalid pattern (Task 11) · a zone failure renders its `DateError` once instead of stacking two lookups (Task 12) · negative years render round-trippably from `Display`/`Serialize` (Task 10) · miette gains a `Caused by:` line for range errors (Task 13) · an unrenderable pattern's message gains `: <Jiff's reason>`, Jiff's error leaves the source chain, and the unreachable range edge behind sites 4-5 reports `DateError::OutOfRange` through `into_date_error` instead of `CalendarError::OutOfRange` (Task 14; see amendment 1) · query `date_add`/`date_diff` unit rejections quote the full unit hint instead of the static phrase (Task 17) · datetime parsing accepts sign+4-digit years such as `-0009-01-01T00:00:00Z` (Task 10; `Display`/`Serialize` stay four-digit zero-padded; see amendment 3).

## Post-implementation amendments (2026-10-28)

Recorded after the branch-final plan-conformance audit (two independent auditors over the full branch diff; 24 strays classified, all ratified). Rationale detail lives in `.scratch/temporal-core/divergence-register.md`.

1. **Public API reservation (line 25) amended.** The post-Task-14 quality commit `58cb776a` also changed the public API: `DateError::OutOfRange { input }` joined the two `InvalidFormat` lines in both public-api baselines. A recognized value that leaves the representable range could not be rendered truthfully by any existing variant, and the engine-side reword alternative is forbidden by Task 13's origin rule. The same commit added the `DATETIME_*_PATTERN` consts and the shared `render_pattern` helper, and rewrote Task 10 Step 3's `render_canonical_pattern` re-export line (all `pub(crate)`; `render_canonical_pattern` itself remains). The Task 14 clause in the accepted-behavior list above now reads `OutOfRange` instead of the originally planned `InvalidValue`.
2. **Task 10 Files scope extended.** `expand_signed_year` (about 30 lines in `datetime.rs`) widens datetime parsing to accept sign+4-digit years. Forced: jiff requires six digits after a sign, so the plan's own Step 1 red tests and the Display-reparse proptest cannot arrange without it.
3. **Task 10 fixtures use constructors.** The negative-year test fixtures build values with `civil::Date::new` instead of parsing the plan's example strings, which jiff cannot parse for the same six-digit reason; `render_year` takes `i16` because that is what jiff's `year()` returns.
4. **Task 7 test placement amended.** `every_moment_token_is_a_single_byte_repeat` lives in `src/temporal/token.rs` rather than on the template seam; it pins the single-byte-repeat invariant that `match_moment_run` correctness depends on.
5. **Task 6 error type modernized.** `TemporalTokenError` derives `thiserror::Error` with `Box<str>` fields instead of the hand-written `Display`/`String` sketch; message text is byte-identical and the shape matches Task 3's `UnitError`.

## File map

- Modify: `src/template/engine/date.rs` (tasks 1-9, 11-16), `src/template/engine.rs` (9, 15), `src/temporal/{date,datetime,point,unit,error,mod,duration}.rs`, `src/temporal/GLOSSARY.md` (16), `src/delimiter.rs` (8), `src/lexer.rs` (17), `src/query/grammar/filter.rs` (5, 17), `src/query/error.rs` (17), `src/lib.rs` (3)
- Create: `src/temporal/token.rs` (6, modified in 7, 8), `src/temporal/fmt.rs` (10), `src/template/engine/duration.rs` (15)
- Docs: `.scratch/temporal-core/{spec.md,review.md,divergence-register.md}`, `.scratch/temporal-core/issues/05-query-temporal-functions.md`, `.scratch/temporal-core/issues/06-template-format-dialects.md` (16)
- Baselines: `tests/baselines/pub-tokens.txt` (+1, +2, +3, −1, +3, +1, +2, +1, +2 for tasks 1, 3, 4, 5, 6, 8, 10, 14, 15; tasks 9, 11, 12, 13 regen with zero net change), `tests/baselines/public-api-default.txt` (14 only), `tests/baselines/public-api-test-utils.txt` (14 only)

---

### Task 1 (O1): Offload the leap-year rule to `DateValue::in_leap_year`

**Files:**
- Modify: `src/temporal/date.rs:120-129` (insert new method after `advance_span`, before the `impl` close at :129)
- Test: `src/temporal/date.rs:537-539` (insert new test mod between `mod constructor`'s close and `mod parse {`)
- Modify: `src/template/engine/date.rs:46-49` (import `DateValue`), `:1043-1056` (`is_leap_year`), `:2398-2406` (pin test)
- Modify: `.scratch/temporal-core/issues/05-query-temporal-functions.md:17` (AC no longer names `leap_year()`)
- Baseline: `tests/baselines/pub-tokens.txt` (+1)

**Context:** The engine computes `year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)` itself, a second live copy of a calendar rule, explicitly forbidden by the tracker AC (`issues/05:17` names `leap_year()` as a violation) and by `review.md:1971` (T-new-1). jiff 0.2.38 already provides `civil::Date::in_leap_year(self) -> bool` (`date.rs:701`), and `DateValue` is the declared single owner of date rules. The engine's doc line (:1039-1042) already claims `i32` while the code uses `i16::try_from`; offloading makes the doc true for free.

- [ ] **Step 1: Write the failing temporal tests**

  In `src/temporal/date.rs`, insert immediately before `mod parse {` (line 539; the anchor `    }\n\n    mod parse {` occurs exactly once):

  ```rust
      mod in_leap_year {
          use pretty_assertions::assert_eq;
          use rstest::rstest;

          use super::*;

          #[rstest]
          #[case::divisible_by_4(2024, true)]
          #[case::not_divisible_by_4(2023, false)]
          #[case::divisible_by_100_not_400(1900, false)]
          #[case::divisible_by_400(2000, true)]
          #[case::negative_four(-4, true)]
          #[case::negative_one(-1, false)]
          #[case::year_zero(0, true)]
          #[case::maximum_year(9999, false)]
          #[case::minimum_year(-9999, false)]
          fn computes_the_proleptic_gregorian_rule(
              #[case] year: i32,
              #[case] expected: bool,
          ) {
              // Arrange: `year` is the input.
              // Act
              let result = DateValue::in_leap_year(year);
              // Assert
              assert_eq!(result, Ok(expected));
          }

          #[rstest]
          #[case::above_range(10_000)]
          #[case::below_range(-10_000)]
          #[case::i32_max(i32::MAX)]
          fn rejects_years_outside_the_representable_range(#[case] year: i32) {
              // Arrange: `year` cannot name a representable civil date.
              // Act
              let result = DateValue::in_leap_year(year);
              // Assert
              assert!(matches!(result, Err(DateError::InvalidValue { .. })));
          }
      }

  ```

- [ ] **Step 2: Run the tests to check that they fail**

  Run: `mise run test:unit in_leap_year`
  Expected: FAIL with compile error `no function or associated item named 'in_leap_year' found for struct 'DateValue'`.

- [ ] **Step 3: Implement `DateValue::in_leap_year`**

  In `src/temporal/date.rs`, replace the end of `impl DateValue`:

  ```rust
      pub(super) fn advance_span(
          self,
          span: Span,
      ) -> Result<Self, CalendarError> {
          self.0
              .checked_add(span)
              .map(Self)
              .map_err(|_| CalendarError::OutOfRange)
      }
  }
  ```

  with:

  ```rust
      pub(super) fn advance_span(
          self,
          span: Span,
      ) -> Result<Self, CalendarError> {
          self.0
              .checked_add(span)
              .map(Self)
              .map_err(|_| CalendarError::OutOfRange)
      }

      /// Returns `true` if `year` falls on a proleptic Gregorian leap year.
      ///
      /// # Errors
      ///
      /// - [`DateError::InvalidValue`] if `year` is outside `-9999..=9999`.
      pub(crate) fn in_leap_year(year: i32) -> Result<bool, DateError> {
          Ok(Self::from_ymd(year, 1, 1)?.civil_date().in_leap_year())
      }
  }
  ```

  Notes: `from_ymd` already enforces the `-9999..=9999` gate and produces the `InvalidValue`/`"a field is out of range"` error. `civil_date()` is the existing `pub(crate)` accessor. The leap rule itself is jiff's method. Visibility is `pub(crate)`: the public API must not grow (public-api baseline stays untouched).

- [ ] **Step 4: Run the temporal tests to check that they pass**

  Run: `mise run test:unit in_leap_year`
  Expected: PASS, 12 rstest cases (nine in `computes_the_proleptic_gregorian_rule`, three in `rejects_years_outside_the_representable_range`).

- [ ] **Step 5: Rewrite the engine's `is_leap_year` and add the import**

  In `src/template/engine/date.rs`, add `DateValue` to the temporal import (line 46-49):

  ```rust
      temporal::{
          CalendarError, CalendarPeriod, DateDiff, DateError, DatePoint,
          DateUnit, PeriodEdge, weekday_from_number,
      },
  ```

  becomes:

  ```rust
      temporal::{
          CalendarError, CalendarPeriod, DateDiff, DateError, DatePoint,
          DateUnit, DateValue, PeriodEdge, weekday_from_number,
      },
  ```

  Then replace `is_leap_year` (:1043-1056):

  ```rust
  fn is_leap_year(value: &Value) -> TemplateEngineResult<bool> {
      let year = if let Some(year) = value.as_i64() {
          i16::try_from(year).map_err(|_| leap_year_input_error(value))?
      } else if let Some(s) = value.as_str() {
          parse_local_datetime(s)?.year()
      } else {
          return Err(leap_year_input_error(value));
      };
      if jiff::civil::Date::new(year, 1, 1).is_err() {
          return Err(leap_year_input_error(value));
      }
      let year = i32::from(year);
      Ok(year % 4 == 0 && (year % 100 != 0 || year % 400 == 0))
  }
  ```

  with:

  ```rust
  fn is_leap_year(value: &Value) -> TemplateEngineResult<bool> {
      let year = if let Some(year) = value.as_i64() {
          i32::try_from(year).map_err(|_| leap_year_input_error(value))?
      } else if let Some(s) = value.as_str() {
          i32::from(parse_local_datetime(s)?.year())
      } else {
          return Err(leap_year_input_error(value));
      };
      DateValue::in_leap_year(year).map_err(|_| leap_year_input_error(value))
  }
  ```

  Behavior equivalence (asserted, not assumed):
  - `|year| > 9999`: old `civil::Date::new` error, new `from_ymd` error, both remapped to `leap_year_input_error`.
  - `10_000..=32_767`: old `i16` ok + `Date::new` fails, new `i32` ok + `from_ymd` fails, same remap.
  - `> 32_767`: old `i16::try_from` fails, new `i32::try_from` fails, same remap.
  - Date-string arm unchanged in effect (both end in `from_ymd`/`Date::new`).
  - The two leap formulas agree on the whole `i32` range (proleptic Gregorian).

- [ ] **Step 6: Run the existing engine tests**

  Run: `mise run test:unit leap_year`
  Expected: PASS; every case in `mod is_leap_year` (`2024/2023/1900/2000`, both date-string cases, the non-int/non-string rejection) stays green, plus the two new temporal tests.

- [ ] **Step 7: Add the engine boundary pin**

  In `mod is_leap_year` (insert after `rejects_a_non_integer_non_string_argument`, before the mod's closing `}` at :2406):

  ```rust
          #[test]
          fn rejects_a_year_outside_the_representable_range() {
              let error = env()
                  .render_str("{{ 40000 is is_leap_year }}", minijinja::context!())
                  .expect_err("an out-of-range year fails cleanly");

              assert_eq!(error.kind(), ErrorKind::InvalidOperation);
          }
  ```

  This test is **green before and after** (old `i16::try_from(40_000)` and new `from_ymd(40_000, 1, 1)` both remap to `leap_year_input_error`). It pins the boundary the rewrite must preserve.

- [ ] **Step 8: Update the tracker AC**

  In `.scratch/temporal-core/issues/05-query-temporal-functions.md:17`, remove the `leap_year()` item from the enumerated violations. Old substring (must match exactly):

  ```text
  `weekday()` (`:602`), `leap_year()` (`:691-692`), plus
  ```

  New:

  ```text
  `weekday()` (`:602`), plus
  ```

  GLOSSARY: **no edit**. Neither glossary mentions leap years. The generic "date rules have a single owner" clause already holds once the offload lands. Record this decision. Do not invent a glossary entry.

- [ ] **Step 9: Regenerate the pub-token baseline (+1 line)**

  Run:

  ```bash
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  git diff --stat tests/baselines/pub-tokens.txt
  ```

  Expected: exactly one added line for `src/temporal/date.rs: pub(crate) fn in_leap_year(year: i32) -> Result<bool, DateError> {`; no deletions.

- [ ] **Step 10: Run the full gate**

  Run: `mise run verify`
  Expected: green: `fmt`, `check` (incl. `check:visibility`: public-api diff empty, pub-token diff empty after regen), `lint --no-fix`, `test`.

- [ ] **Step 11: Commit**

  ```bash
  git add src/temporal/date.rs src/template/engine/date.rs .scratch/temporal-core/issues/05-query-temporal-functions.md tests/baselines/pub-tokens.txt
  git commit -m "refactor(temporal): offload leap-year rule to DateValue"
  ```

---

### Task 2 (O3): Route the date namespace through `resolve_period_edge`

**Files:**
- Modify: `src/template/engine/date.rs:46-49` (import `resolve_period_edge`), `:293-356` (`get_value`)
- Test: `src/template/engine/date.rs:1158-1174` (`mod get_value`)
- Baseline: none (no visibility change)

**Context:** Twelve `get_value` arms (`start_of_week`/`sow`/…/`eoy`) hand-map names that `temporal::calendar::resolve_period_edge` (calendar.rs:101-116) already resolves, ASCII-case-insensitively, from one `PERIOD_EDGE_LOOKUP` table. Three-way maintenance (namespace const, match arms, lookup) collapses to one. The key is folded to lowercase once in a 16-byte buffer (mirroring `resolve_period_edge` itself), which makes the whole `date` namespace case-insensitive: a deliberate superset matching the query layer (`filter.rs:514` already resolves case-insensitively). Longest real key is `start_of_month` (15 bytes) so the 16-byte fold never truncates a real name; longer unknown keys fall through raw and resolve to `None` as before.

- [ ] **Step 1: Write the failing tests**

  In `mod get_value` (insert after `get_value_returns_none_for_a_non_string_key`, before the mod's closing `}` at :1174):

  ```rust
          #[test]
          fn get_value_accepts_uppercase_method_names() {
              let ops = Arc::new(DateNamespace);

              for method in NAMESPACE_METHODS {
                  let upper = method.to_ascii_uppercase();
                  assert!(
                      ops.get_value(&Value::from(upper.as_str())).is_some(),
                      "{upper:?} (uppercase of {method:?}) does not resolve"
                  );
              }
          }

          #[test]
          fn get_value_resolves_mixed_case_period_edges() {
              let ops = Arc::new(DateNamespace);

              assert!(ops.get_value(&Value::from("Start_Of_Week")).is_some());
              assert!(ops.get_value(&Value::from("End_Of_Month")).is_some());
          }
  ```

- [ ] **Step 2: Run the tests to check that they fail**

  Run: `mise run test:unit get_value`
  Expected: FAIL: `get_value_accepts_uppercase_method_names` panics with `"NOW" (uppercase of "now") does not resolve` (the other new test fails on `Start_Of_Week`).

- [ ] **Step 3: Import `resolve_period_edge` and rewrite `get_value`**

  In `src/template/engine/date.rs`, extend the temporal import (:46-49):

  ```rust
      temporal::{
          CalendarError, CalendarPeriod, DateDiff, DateError, DatePoint,
          DateUnit, DateValue, PeriodEdge, weekday_from_number,
      },
  ```

  becomes:

  ```rust
      temporal::{
          CalendarError, CalendarPeriod, DateDiff, DateError, DatePoint,
          DateUnit, DateValue, PeriodEdge, resolve_period_edge,
          weekday_from_number,
      },
  ```

  Replace the whole `get_value` body (:293-356):

  ```rust
      fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
          match key.as_str()? {
              "now" => Some(shorthand(Anchor::Now)),
              "today" => Some(shorthand(Anchor::Today)),
              "tomorrow" => Some(shorthand(Anchor::Tomorrow)),
              "yesterday" => Some(shorthand(Anchor::Yesterday)),
              "start_of_week" | "sow" => {
                  Some(shorthand(Anchor::StartOf(CalendarPeriod::Week)))
              }
              "end_of_week" | "eow" => {
                  Some(shorthand(Anchor::EndOf(CalendarPeriod::Week)))
              }
              "start_of_month" | "som" => {
                  Some(shorthand(Anchor::StartOf(CalendarPeriod::Month)))
              }
              "end_of_month" | "eom" => {
                  Some(shorthand(Anchor::EndOf(CalendarPeriod::Month)))
              }
              "start_of_year" | "soy" => {
                  Some(shorthand(Anchor::StartOf(CalendarPeriod::Year)))
              }
              "end_of_year" | "eoy" => {
                  Some(shorthand(Anchor::EndOf(CalendarPeriod::Year)))
              }
              "weekday" => Some(Value::from_function(
                  |n: i64,
                   format: Option<&str>,
                   reference: Option<&str>,
                   reference_format: Option<&str>,
                   kwargs: Kwargs|
                   -> TemplateEngineResult<String> {
                       let target = weekday_or_error(n)?;
                       Anchor::Weekday(target).render(
                           format,
                           None,
                           reference,
                           reference_format,
                           &kwargs,
                       )
                   },
              )),
              "from_timestamp" => Some(Value::from_function(
                  |unix_ts: i64,
                   kwargs: Kwargs|
                   -> TemplateEngineResult<String> {
                       let format = format_kwarg(&kwargs)?;
                       let timestamp =
                           jiff::Timestamp::new(unix_ts, 0).map_err(|_| {
                               Error::new(
                                   ErrorKind::InvalidOperation,
                                   format!("timestamp {unix_ts} is out of range"),
                               )
                           })?;
                       let zone = crate::temporal::local_zone()
                           .map_err(render_date_error)?;
                       let zoned = DateTimeValue::from_timestamp(timestamp)
                           .utc()
                           .to_zoned(zone);
                       format_strtime(&zoned, format, format)
                   },
              )),
              _ => None,
          }
      }
  ```

  with:

  ```rust
      fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
          let key = key.as_str()?;
          let mut buf = [0u8; 16];
          let lower = if key.len() <= buf.len() {
              for (dst, byte) in buf.iter_mut().zip(key.as_bytes()) {
                  *dst = byte.to_ascii_lowercase();
              }
              std::str::from_utf8(&buf[..key.len()]).ok()?
          } else {
              key
          };
          match lower {
              "now" => Some(shorthand(Anchor::Now)),
              "today" => Some(shorthand(Anchor::Today)),
              "tomorrow" => Some(shorthand(Anchor::Tomorrow)),
              "yesterday" => Some(shorthand(Anchor::Yesterday)),
              "weekday" => Some(Value::from_function(
                  |n: i64,
                   format: Option<&str>,
                   reference: Option<&str>,
                   reference_format: Option<&str>,
                   kwargs: Kwargs|
                   -> TemplateEngineResult<String> {
                       let target = weekday_or_error(n)?;
                       Anchor::Weekday(target).render(
                           format,
                           None,
                           reference,
                           reference_format,
                           &kwargs,
                       )
                   },
              )),
              "from_timestamp" => Some(Value::from_function(
                  |unix_ts: i64,
                   kwargs: Kwargs|
                   -> TemplateEngineResult<String> {
                       let format = format_kwarg(&kwargs)?;
                       let timestamp =
                           jiff::Timestamp::new(unix_ts, 0).map_err(|_| {
                               Error::new(
                                   ErrorKind::InvalidOperation,
                                   format!("timestamp {unix_ts} is out of range"),
                               )
                           })?;
                       let zone = crate::temporal::local_zone()
                           .map_err(render_date_error)?;
                       let zoned = DateTimeValue::from_timestamp(timestamp)
                           .utc()
                           .to_zoned(zone);
                       format_strtime(&zoned, format, format)
                   },
              )),
              edge_name => {
                  let (period, edge) = resolve_period_edge(edge_name)?;
                  let anchor = match edge {
                      PeriodEdge::Start => Anchor::StartOf(period),
                      PeriodEdge::End => Anchor::EndOf(period),
                  };
                  Some(shorthand(anchor))
              }
          }
      }
  ```

  Behavior notes: the twelve explicit arms become one fall-through arm. Unknown keys still return `None` (`resolve_period_edge` returns `None`). Keys over 16 bytes skip the fold and still resolve to `None`. `CalendarPeriod` remains used (the `Anchor` enum at :153-154 and `render_period_edge` at :693), so no import is dropped. `NAMESPACE_METHODS` is unchanged (already lowercase, so the existing bidirectional test at :1412-1423 keeps passing).

- [ ] **Step 4: Run the tests to check that they pass**

  Run: `mise run test:unit get_value`
  Expected: PASS: the two original `get_value` tests plus both new tests (5 with the enumerate round-trip that also matches the filter).

- [ ] **Step 5: Run the full gate**

  Run: `mise run verify`
  Expected: green (no baseline regen needed: no `pub` line changed).

- [ ] **Step 6: Commit**

  ```bash
  git add src/template/engine/date.rs
  git commit -m "refactor(template): resolve date namespace keys through resolve_period_edge"
  ```

---

### Task 3 (O2): Type the unknown-unit rejection as `UnitError`

**Files:**
- Modify: `src/temporal/error.rs:168` (insert `UnitError` between `impl CalendarError` and the tests mod), `src/temporal/unit.rs:20` (import) and `:97` (add `parse_spelling` after `from_spelling`), `src/template/engine/date.rs:406-409` (doc link), `:414-415` (`unit_kwarg` body), `:966` (doc link), `:1113-1130` (delete `unknown_unit_error`), `:44-45` (drop the hint import; apply this edit last, it shifts every later line by one), `src/temporal/mod.rs:54`, `src/lib.rs:170-173`
- Test: `src/temporal/unit.rs:499` (new `mod spelling_errors` between `mod lookup` and `mod accessors`), `src/template/engine/date.rs:2244` (message-shape pin inside `date_add_rejects_unknown_unit`)
- Baseline: regen `tests/baselines/pub-tokens.txt`, expected delta `+2` added lines plus the one `mod.rs` line rewrite (the table's "+2, 1 line rewrite"); `public-api-default.txt` and `public-api-test-utils.txt` stay byte-identical

**Context:** Rejecting a bad `unit="..."` kwarg is split over three places: `TimeUnit::from_spelling` (unit.rs:87-97) returns a bare `Option`; `unit_kwarg` (engine/date.rs:411-416) converts `None` through the date.rs-local `unknown_unit_error` (:1116-1129); and the message text comes from `UNIT_SPELLINGS_HINT` (unit.rs:357-363), re-exported through `temporal/mod.rs:54` and `lib.rs:172` solely so that one helper can reach it. O2 gives the rejection a type: `TimeUnit::parse_spelling(unit) -> Result<Self, UnitError>`; the thiserror format string reproduces the old `format!` byte for byte (`Box<str>` debug-prints exactly like the old `&str`); and `unit_kwarg` maps it with `invalid_operation(error.to_string(), error)` (engine/error.rs:48-56), which keeps the message verbatim and chains the cause. minijinja's `Display` prints only `"{kind}: {detail}"` (minijinja error.rs:183), never the source chain, so the user-visible string stays `invalid operation: unknown unit "..." (expected ...)`. The existing kind-only tests (`rejects_an_unknown_unit_instead_of_panicking` :2084, `date_add_rejects_unknown_unit` :2236) stay green; Step 5 adds a shape pin so Step 6 cannot silently rewrap the message. `filter.rs:322/669/685` keeps calling `from_spelling` (query adoption of `parse_spelling` lands in Task 17; the evaluator sites stay on `from_spelling` by design); `DurationError::UnknownUnit` (error.rs:119), the duration parser's hint-less sibling message, is untouched. Visibility: `UnitError` is declared `pub` inside the private `mod error` (mod.rs:36) and is not re-exported, so both public-api baselines stay byte-identical (`unnameable-types` and `unreachable-pub` are allow-by-default; `clippy::redundant_pub_crate` is nursery and not enabled here). GLOSSARY: **no edit**; the `UNIT_SPELLINGS_HINT`/`TEMPORAL_UNITS` sync contract at `src/GLOSSARY.md:49` stays true and is now machine-enforced by `unit_spellings_hint_matches_the_registry`.

- [ ] **Step 1: Write the failing tests**

  In `src/temporal/unit.rs`, insert a new test mod between the end of `mod lookup` (line 499) and `mod accessors` (line 501):

  ```rust
      mod spelling_errors {
          use pretty_assertions::assert_eq;

          use super::*;

          /// The `UnitError` message quotes [`UNIT_SPELLINGS_HINT`] verbatim,
          /// so the hint may only advertise spellings the registry accepts
          /// under its own plural name.
          #[test]
          fn unit_spellings_hint_matches_the_registry() {
              // Arrange: the hint is one `"plural"/"short"` pair per row.
              let row_units = [
                  TimeUnit::Year,
                  TimeUnit::Month,
                  TimeUnit::Week,
                  TimeUnit::Day,
                  TimeUnit::Hour,
                  TimeUnit::Minute,
                  TimeUnit::Second,
              ];
              let entries: Vec<&str> = UNIT_SPELLINGS_HINT
                  .split(", ")
                  .map(|entry| entry.trim_start_matches("or "))
                  .collect();

              // Act & Assert: every pair parses to the row that owns it.
              assert_eq!(
                  entries.len(),
                  row_units.len(),
                  "the hint must name one pair per registry row"
              );
              for (&entry, unit) in entries.iter().zip(row_units) {
                  let (plural, short) = entry
                      .split_once("\"/\"")
                      .expect("hint pairs are `\"plural\"/\"short\"`");
                  let plural = plural.trim_matches('"');
                  let short = short.trim_matches('"');

                  assert_eq!(
                      plural,
                      unit.human_names().1,
                      "the hint's plural must be the registry's own plural"
                  );
                  assert_eq!(
                      TimeUnit::parse_spelling(plural),
                      Ok(unit),
                      "`{plural}` must parse from the hint"
                  );
                  assert_eq!(
                      TimeUnit::parse_spelling(short),
                      Ok(unit),
                      "`{short}` must parse from the hint"
                  );
              }
          }

          #[test]
          fn unknown_spelling_error_names_the_unit_and_the_hint() {
              // Arrange
              let error = TimeUnit::parse_spelling("decades")
                  .expect_err("`decades` is not an accepted unit spelling");

              // Act
              let rendered = error.to_string();

              // Assert
              assert_eq!(
                  rendered,
                  format!("unknown unit \"decades\" (expected {UNIT_SPELLINGS_HINT})")
              );
          }
      }
  ```

- [ ] **Step 2: Run the tests to check that they fail**

  Run: `mise run test:unit spelling_errors`
  Expected: FAIL: compile error, no test runs: `no function or associated item named 'parse_spelling' found for enum 'TimeUnit'`. Neither `parse_spelling` nor `UnitError` exists yet. The engine still compiles because it still calls `from_spelling`.

- [ ] **Step 3: Add `UnitError` and `TimeUnit::parse_spelling`**

  In `src/temporal/error.rs`, replace the tail of `impl CalendarError` plus the test-mod opener:

  ```rust
              },
          }
      }
  }

  #[cfg(test)]
  ```

  with:

  ```rust
              },
          }
      }
  }

  /// Reports a unit spelling outside the registered unit set.
  #[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
  pub enum UnitError {
      /// Reports that a `unit="..."` kwarg matched no registered unit; the
      /// message quotes every accepted spelling.
      #[error("unknown unit {unit:?} (expected {})", super::unit::UNIT_SPELLINGS_HINT)]
      UnknownSpelling {
          /// Stores the rejected spelling.
          unit: Box<str>,
      },
  }

  #[cfg(test)]
  ```

  (`UnitError` is `pub` inside the private `mod error` like `DateError` is. It carries no `#[non_exhaustive]`, matching the internal `CalendarError`. Variant fields carry no `pub`, so the pub-token cost is the enum line alone.)

  In `src/temporal/unit.rs`, change the error import (line 20):

  ```rust
  use super::error::CalendarError;
  ```

  to:

  ```rust
  use super::error::{CalendarError, UnitError};
  ```

  and insert `parse_spelling` directly after `from_spelling` (anchor: the `UNIT_LOOKUP.get(lower).copied()` tail, before the `UnitSpec` doc):

  ```rust
      /// Resolves a unit spelling or reports why it was rejected.
      ///
      /// The [`UnitError`] message quotes [`UNIT_SPELLINGS_HINT`], so a
      /// rejected `unit="..."` kwarg names every accepted spelling without a
      /// second copy of the list beside the caller.
      ///
      /// # Errors
      ///
      /// - [`UnitError::UnknownSpelling`] if `unit` matches no registered
      ///   spelling; matching is ASCII case-insensitive (via
      ///   [`Self::from_spelling`]).
      pub(crate) fn parse_spelling(unit: &str) -> Result<Self, UnitError> {
          Self::from_spelling(unit)
              .ok_or_else(|| UnitError::UnknownSpelling { unit: unit.into() })
      }
  ```

- [ ] **Step 4: Run the tests to check that they pass**

  Run: `mise run test:unit spelling_errors`
  Expected: PASS: both new tests. `unit_spellings_hint_matches_the_registry` proves each hint pair parses to its own registry row under the registry's own plural. `unknown_spelling_error_names_the_unit_and_the_hint` pins the byte-exact `UnitError` message.

- [ ] **Step 5: Add the engine message-shape pin**

  In `date_add_rejects_unknown_unit` (engine/date.rs:2236-2245), extend the existing assertion:

  ```rust
              assert_eq!(error.kind(), ErrorKind::InvalidOperation);
          }
  ```

  with:

  ```rust
              assert_eq!(error.kind(), ErrorKind::InvalidOperation);
              assert!(
                  error
                      .to_string()
                      .contains("unknown unit \"fortnight\" (expected "),
                  "the unknown-unit message must keep its shape, got: {error}"
              );
          }
  ```

  This test is **green before and after** the rewrite (minijinja's `Display` renders `"{kind}: {detail}"`, so `contains` sees the same detail on both sides). It pins the text Step 6 must preserve.

  Run: `mise run test:unit date_add_rejects_unknown_unit`
  Expected: PASS against the unmodified engine.

- [ ] **Step 6: Rewire the engine and shrink the hint's re-exports**

  Apply these edits in order. Each old text block is unique. The import drop goes last because it shifts every later line.

  1. engine/date.rs `unit_kwarg` doc (lines 406-409), link to the fallible API:

     ```rust
         /// [`ErrorKind::InvalidOperation`](minijinja::ErrorKind::InvalidOperation) if
         /// `unit` is not one of [`TimeUnit::from_spelling`]'s accepted unit names or
         /// abbreviations, or
         /// [`ErrorKind::TooManyArguments`](minijinja::ErrorKind::TooManyArguments)
     ```

     becomes:

     ```rust
         /// [`ErrorKind::InvalidOperation`](minijinja::ErrorKind::InvalidOperation) if
         /// `unit` is not one of [`TimeUnit::parse_spelling`]'s accepted unit names or
         /// abbreviations, or
         /// [`ErrorKind::TooManyArguments`](minijinja::ErrorKind::TooManyArguments)
     ```

  2. engine/date.rs `unit_kwarg` body (lines 414-415):

     ```rust
         TimeUnit::from_spelling(unit_str)
             .ok_or_else(|| unknown_unit_error(unit_str))
     ```

     becomes:

     ```rust
         TimeUnit::parse_spelling(unit_str)
             .map_err(|error| invalid_operation(error.to_string(), error))
     ```

  3. engine/date.rs `date_diff` doc (line 966):

     ```rust
     /// [`TimeUnit::from_spelling`]'s accepted names (see [`unit_kwarg`]). Returns
     ```

     becomes:

     ```rust
     /// [`TimeUnit::parse_spelling`]'s accepted names (see [`unit_kwarg`]). Returns
     ```

  4. engine/date.rs, delete `unknown_unit_error` entirely (the closing `}` of the previous function through the blank line before `#[cfg(test)]`):

     ```rust
     }

     /// Builds the error for a `unit="..."` kwarg naming anything outside
     /// [`TimeUnit::from_spelling`]'s accepted unit names.
     ///
     /// The expected-spellings hint comes from [`UNIT_SPELLINGS_HINT`], kept beside
     /// the unit registry so the message cannot drift from what [`unit_kwarg`]
     /// actually accepts.
     ///
     /// Shared by [`date_add`], [`date_sub`], and [`date_diff`] via [`unit_kwarg`].
     fn unknown_unit_error(unit: &str) -> Error {
         Error::new(
             ErrorKind::InvalidOperation,
             format!("unknown unit {unit:?} (expected {UNIT_SPELLINGS_HINT})"),
         )
     }

     #[cfg(test)]
     ```

     becomes:

     ```rust
     }

     #[cfg(test)]
     ```

  5. engine/date.rs crate import (lines 44-46), drop the hint:

     ```rust
         DEFAULT_DATE_PATTERN, DateTimeValue, DurationValue, TimeUnit,
         UNIT_SPELLINGS_HINT,
         temporal::{
     ```

     becomes:

     ```rust
         DEFAULT_DATE_PATTERN, DateTimeValue, DurationValue, TimeUnit,
         temporal::{
     ```

  6. `src/temporal/mod.rs:54` (the table's one line rewrite):

     ```rust
     pub(crate) use unit::{DateUnit, TimeUnit, UNIT_SPELLINGS_HINT};
     ```

     becomes:

     ```rust
     pub(crate) use unit::{DateUnit, TimeUnit};
     ```

  7. `src/lib.rs:170-173`, drop the hint line (keep the multi-line braces; `fmt` approved this shape):

     ```rust
     pub(crate) use temporal::{
         DEFAULT_DATE_PATTERN, DEFAULT_DATETIME_PATTERN, TimeUnit,
         UNIT_SPELLINGS_HINT,
     };
     ```

     becomes:

     ```rust
     pub(crate) use temporal::{
         DEFAULT_DATE_PATTERN, DEFAULT_DATETIME_PATTERN, TimeUnit,
     };
     ```

  Behavior notes: the three `UNIT_SPELLINGS_HINT` imports go away together, so no `unused_imports` warning remains.
  `Error`/`ErrorKind` stay used elsewhere in date.rs (lines 136, 282, 341, 559, 598, 648, 1061, 1075).
  `from_spelling` itself stays (unit.rs tests, `filter.rs:322/669/685`, and the new `parse_spelling` body).

  Run: `mise run test:unit unknown_unit` and `mise run test:unit spelling_errors`
  Expected: PASS: `rejects_an_unknown_unit_instead_of_panicking`, `date_add_rejects_unknown_unit` (now with the shape pin), and both `spelling_errors` tests.

- [ ] **Step 7: Regenerate the pub-token baseline (+2 lines, 1 rewrite)**

  Run:

  ```bash
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  git diff tests/baselines/pub-tokens.txt
  ```

  Expected: exactly two added lines, `src/temporal/error.rs: pub enum UnitError {` and `src/temporal/unit.rs: pub(crate) fn parse_spelling(unit: &str) -> Result<Self, UnitError> {`. The one rewritten line `src/temporal/mod.rs: pub(crate) use unit::{DateUnit, TimeUnit};` replaces its three-item form. No other lines change (`src/lib.rs` keeps its `pub(crate) use temporal::{` line, only a non-matching body line was removed).

- [ ] **Step 8: Run the full gate**

  Run: `mise run verify`
  Expected: green (`fmt`, `check` incl. `check:visibility`: both public-api diffs empty because `UnitError` is unreachable from outside the crate, pub-token diff empty after the regen, `lint --no-fix`, `test`).

- [ ] **Step 9: Commit**

  ```bash
  git add src/temporal/error.rs src/temporal/unit.rs src/temporal/mod.rs src/lib.rs src/template/engine/date.rs tests/baselines/pub-tokens.txt
  git commit -m "refactor(temporal): parse unit spellings through UnitError"
  ```

---

### Task 4 (O5): Own the wall clock in temporal with `now()` and `today()`

**Files:**
- Modify: `src/temporal/datetime.rs` (insert `now()` after `from_timestamp` :111-114; tests: insert `mod clock` before `mod constructor` :628)
- Modify: `src/temporal/date.rs` (insert `today()` after Task 1's `in_leap_year`, before the `impl DateValue` close; tests: insert `mod today` before `mod parse {`, the same anchor Task 1 chained onto)
- Modify: `src/temporal/point.rs:779` (`local_zone_for_calendar` gains `pub(super)`)
- Modify: `src/template/engine/date.rs` (`base` :160-173, `transform` :176-205, comment at `is_past` :1007 / `is_future` :1023)
- Baseline: `tests/baselines/pub-tokens.txt` (+3)

**Context:** The engine reads the clock. `Anchor::base` (`engine/date.rs:160-173`) builds `DatePoint::from(DateTimeValue::from_timestamp(jiff::Timestamp::now()))` eagerly, then derives the calendar day through `now.date()`, which resolves the process zone inside `DatePoint::date` (`point.rs:200`, `local_zone_for_calendar()?`). `Anchor::transform` (:176-205) keeps `Now | Today` out of the zone by short-circuiting in an outer match; its inner `Now | Today` arm is unreachable, an exhaustiveness artifact of matching the narrowed `anchor`. O5 gives the clock to temporal behind the same seam `DatePoint::date` already uses: `DateTimeValue::now()` is an infallible instant read, and `DateValue::today()` resolves the local calendar day through `local_zone_for_calendar()`, so the error is value-identical to the old path (`now.date()?` already propagated `CalendarError::Zone` from that constructor, rendered unchanged at `render` :246). `base` collapses to a two-arm match; `transform` collapses to a single match whose `let zone = || ...` closure resolves the zone per arm, preserving the invariant that `Now`/`Today` never touch the zone (fetching the zone at the top of `transform` would make `date.now()` fail when zone lookup fails: forbidden). `is_past`/`is_future` (:1007/:1023) compare instants: no zone and no point construction, so they deliberately keep raw `jiff::Timestamp::now()` and gain a comment saying why. Re-verified by grep: those are the engine's only `Timestamp::now()` sites besides a test (:2326); `render` does not read the clock itself. Engine tests `mod now` (:1176) and `mod today_tomorrow_yesterday` (:1263) are **green before and after**. Visibility: +3 pub tokens, all `pub(crate)`/`pub(super)`, so both public-api baselines stay byte-identical. GLOSSARY: **no edit**; the glossary defines the types against the process-local clock but names no constructor, so `now()`/`today()` add no vocabulary. Note: `transform` still calls `start_of`/`end_of` here; Task 5 converts those two arms to `snap`.

- [ ] **Step 1: Write the failing temporal tests**

  Two insertions.

  1. In `src/temporal/datetime.rs`, insert `mod clock` immediately before `mod constructor {` (line 628; the sequence `    mod constructor {\n        use std::time::Duration;` occurs exactly once):

  ```rust
      mod clock {
          use pretty_assertions::assert_eq;

          use super::*;

          #[test]
          fn now_falls_between_two_bracketing_clock_reads() {
              // Arrange
              let before = Timestamp::now();
              // Act
              let now = DateTimeValue::now();
              let after = Timestamp::now();
              // Assert
              assert!(now.utc() >= before, "{now:?} must not predate {before:?}");
              assert!(now.utc() <= after, "{now:?} must not postdate {after:?}");
          }

          #[test]
          fn now_is_after_a_fixed_timestamp_from_2025() {
              // Arrange: an instant fixed at 2025-10-09.
              let earlier = DateTimeValue::from_unix_seconds(1_760_000_000)
                  .expect("valid timestamp");
              // Act
              let now = DateTimeValue::now();
              // Assert
              assert!(now > earlier, "now must postdate the fixed instant");
          }
      }

  ```

  2. In `src/temporal/date.rs`, insert immediately before `mod parse {`. This is the same anchor Task 1 chained onto: `    }\n\n    mod parse {`, still exactly once after Task 1:

  ```rust
      mod today {
          use pretty_assertions::assert_eq;

          use super::*;

          #[test]
          fn today_is_the_local_calendar_date_of_the_instant() {
              // Arrange: pin a fixed-offset zone so both reads share a date.
              ZoneGuard::with("Etc/GMT+5", || {
                  let zone = jiff::tz::TimeZone::get("Etc/GMT+5")
                      .expect("test zone exists");
                  // Act: the oracle is raw jiff, independent of this crate's
                  // local-zone plumbing.
                  let expected = jiff::Timestamp::now().to_zoned(zone).date();
                  let today = DateValue::today().expect("pinned zone resolves");
                  // Assert: a local-midnight crossing between the two clock
                  // reads is a microseconds-per-day race, accepted here.
                  assert_eq!(today.civil_date(), expected);
              });
          }
      }

  ```

  Notes: `ZoneGuard` is already imported at the `mod tests` root (`date.rs:394`) and reaches child mods through `use super::*`, the same path `ymd` takes. `Etc/GMT+5` is a fixed offset with no DST transitions, so both clock reads see the same wall date except at the accepted midnight race.

- [ ] **Step 2: Run the tests to check that they fail**

  Run: `mise run test:unit now_falls_between`
  Expected: FAIL with compile error `no function or associated item named 'now' found for struct 'DateTimeValue'`. The `'today'` error is the same for `DateValue`. One run covers both files because the crate compiles as one unit.

- [ ] **Step 3: Implement `DateTimeValue::now()`**

  In `src/temporal/datetime.rs`, replace `from_timestamp`:

  ```rust
      /// Wraps a Jiff [`Timestamp`].
      #[inline]
      pub(crate) const fn from_timestamp(timestamp: Timestamp) -> Self {
          Self(timestamp)
      }
  ```

  with:

  ```rust
      /// Wraps a Jiff [`Timestamp`].
      #[inline]
      pub(crate) const fn from_timestamp(timestamp: Timestamp) -> Self {
          Self(timestamp)
      }

      /// Reads the current instant from the system clock.
      #[inline]
      pub(crate) fn now() -> Self {
          Self(Timestamp::now())
      }
  ```

  Note: `Timestamp` is already imported (`datetime.rs:13`); `#[inline]` matches the sibling constructors.

- [ ] **Step 4: Implement `DateValue::today()` and widen `local_zone_for_calendar`**

  Two edits.

  1. In `src/temporal/point.rs`, widen the helper's visibility (line 779; `fn local_zone_for_calendar() -> Result<TimeZone, CalendarError> {` occurs exactly once):

  ```rust
  fn local_zone_for_calendar() -> Result<TimeZone, CalendarError> {
  ```

  becomes:

  ```rust
  pub(super) fn local_zone_for_calendar() -> Result<TimeZone, CalendarError> {
  ```

  2. In `src/temporal/date.rs`, replace Task 1's `in_leap_year` tail (the `impl DateValue` close follows it):

  ```rust
          pub(crate) fn in_leap_year(year: i32) -> Result<bool, DateError> {
              Ok(Self::from_ymd(year, 1, 1)?.civil_date().in_leap_year())
          }
      }
  ```

  with:

  ```rust
          pub(crate) fn in_leap_year(year: i32) -> Result<bool, DateError> {
              Ok(Self::from_ymd(year, 1, 1)?.civil_date().in_leap_year())
          }

          /// Returns the calendar date of the current instant in the
          /// process-local zone.
          ///
          /// # Errors
          ///
          /// - [`CalendarError::Zone`] if process-local time-zone discovery
          ///   fails.
          pub(crate) fn today() -> Result<Self, CalendarError> {
              let zone = super::point::local_zone_for_calendar()?;
              Ok(DateTimeValue::now().local_datetime_in(&zone).date().into())
          }
      }
  ```

  Notes: `DateTimeValue` and `CalendarError` are already imported (`date.rs:14-16`); `super::point::local_zone_for_calendar` resolves because `pub(super)` items are visible throughout `temporal`, including descendant modules such as `temporal::date`. The body converts through `local_datetime_in` + `From<civil::Date>` (`date.rs:156`), the same conversion `DatePoint::date` does today. Visibility is `pub(super)`: the helper gains no wider reach than `temporal` itself.

- [ ] **Step 5: Run the temporal tests to check that they pass**

  Run: `mise run test:unit tests::clock`
  Expected: PASS, 2 tests (`now_falls_between_two_bracketing_clock_reads`, `now_is_after_a_fixed_timestamp_from_2025`).

  Run: `mise run test:unit tests::today`
  Expected: PASS, 1 test (`today_is_the_local_calendar_date_of_the_instant`). The filter may also execute engine `today_tomorrow_yesterday` tests; they still run against the unrewritten engine and stay green.

- [ ] **Step 6: Rewrite the engine's `base`, `transform`, and annotate `is_past`/`is_future`**

  Three edits in `src/template/engine/date.rs`.

  1. Replace `base`:

  ```rust
      fn base(self) -> Result<DatePoint, CalendarError> {
          let now = DatePoint::from(DateTimeValue::from_timestamp(
              jiff::Timestamp::now(),
          ));
          match self {
              Self::Now => Ok(now),
              Self::Today
              | Self::Tomorrow
              | Self::Yesterday
              | Self::StartOf(_)
              | Self::EndOf(_)
              | Self::Weekday(_) => Ok(DatePoint::from(now.date()?)),
          }
      }
  ```

  with:

  ```rust
      fn base(self) -> Result<DatePoint, CalendarError> {
          match self {
              Self::Now => Ok(DatePoint::from(DateTimeValue::now())),
              Self::Today
              | Self::Tomorrow
              | Self::Yesterday
              | Self::StartOf(_)
              | Self::EndOf(_)
              | Self::Weekday(_) => Ok(DatePoint::from(DateValue::today()?)),
          }
      }
  ```

  2. Replace `transform` (the doc comment `/// Moves the date point as this anchor requires.` stays):

  ```rust
      fn transform(
          self,
          date_point: DatePoint,
      ) -> TemplateEngineResult<DatePoint> {
          match self {
              Self::Now | Self::Today => Ok(date_point),
              anchor => {
                  let zone =
                      crate::temporal::local_zone().map_err(render_date_error)?;
                  match anchor {
                      Self::Now | Self::Today => Ok(date_point),
                      Self::Tomorrow => date_point
                          .shift(1, TimeUnit::Day, &zone)
                          .map_err(render_calendar_error),
                      Self::Yesterday => date_point
                          .shift(-1, TimeUnit::Day, &zone)
                          .map_err(render_calendar_error),
                      Self::StartOf(calendar_period) => date_point
                          .start_of(calendar_period, &zone)
                          .map_err(render_calendar_error),
                      Self::EndOf(calendar_period) => date_point
                          .end_of(calendar_period, &zone)
                          .map_err(render_calendar_error),
                      Self::Weekday(target) => date_point
                          .with_weekday(target, &zone)
                          .map_err(render_calendar_error),
                  }
              }
          }
      }
  ```

  with:

  ```rust
      fn transform(
          self,
          date_point: DatePoint,
      ) -> TemplateEngineResult<DatePoint> {
          let zone = || crate::temporal::local_zone().map_err(render_date_error);
          match self {
              Self::Now | Self::Today => Ok(date_point),
              Self::Tomorrow => date_point
                  .shift(1, TimeUnit::Day, &zone()?)
                  .map_err(render_calendar_error),
              Self::Yesterday => date_point
                  .shift(-1, TimeUnit::Day, &zone()?)
                  .map_err(render_calendar_error),
              Self::StartOf(calendar_period) => date_point
                  .start_of(calendar_period, &zone()?)
                  .map_err(render_calendar_error),
              Self::EndOf(calendar_period) => date_point
                  .end_of(calendar_period, &zone()?)
                  .map_err(render_calendar_error),
              Self::Weekday(target) => date_point
                  .with_weekday(target, &zone()?)
                  .map_err(render_calendar_error),
          }
      }
  ```

  3. Annotate the two instant comparisons (their bodies otherwise stay byte-identical):

  ```rust
  fn is_past(value: &str) -> TemplateEngineResult<bool> {
      let instant =
          parse_date_point(value)?.instant().map_err(render_calendar_error)?;
      Ok(instant < jiff::Timestamp::now())
  }
  ```

  becomes:

  ```rust
  fn is_past(value: &str) -> TemplateEngineResult<bool> {
      let instant =
          parse_date_point(value)?.instant().map_err(render_calendar_error)?;
      // Instant comparison, not point construction: no zone or date point.
      Ok(instant < jiff::Timestamp::now())
  }
  ```

  and the same comment line before `Ok(instant > jiff::Timestamp::now())` in `is_future`.

  Notes: `DateValue` is already in the engine's `temporal::{...}` import (Task 1). `zone` stays lazy: the closure runs once per non-`Now`/`Today` arm and never for those two, so `date.now()` still cannot fail on zone lookup. Each arm sees `&zone()?`, `TimeZone` by reference exactly as before. `render_date_error` and `render_calendar_error` remain referenced (the closure plus `render` :246), so no import goes dead.

- [ ] **Step 7: Run the engine tests to check that they pass**

  Run: `mise run test:unit now`
  Expected: PASS (engine `mod now` renders shape only; the new datetime `now` tests pass too).

  Run: `mise run test:unit today_tomorrow_yesterday`
  Expected: PASS. This test is **green before and after**: it drives `base` plus `transform` with real zone resolution. It asserts the single-evaluation day arithmetic, so the rewrite changes no observable behavior.

- [ ] **Step 8: Regenerate the pub-token baseline**

  Run:

  ```
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  ```

  Expected: the diff adds exactly three lines:

  ```
  src/temporal/date.rs: pub(crate) fn today() -> Result<Self, CalendarError> {
  src/temporal/datetime.rs: pub(crate) fn now() -> Self {
  src/temporal/point.rs: pub(super) fn local_zone_for_calendar() -> Result<TimeZone, CalendarError> {
  ```

  and nothing else changes. Both `cargo public-api` baselines stay byte-identical: no `pub` item was added.

- [ ] **Step 9: Run the full gate**

  Run: `mise run verify`
  Expected: PASS (fmt, check including `check:visibility` against both baseline kinds, lint without fixes, full test suite).

- [ ] **Step 10: Commit**

  ```
  git add src/temporal/datetime.rs src/temporal/date.rs src/temporal/point.rs src/template/engine/date.rs tests/baselines/pub-tokens.txt
  git commit -m "refactor(temporal): offload clock reads to now() and today()"
  ```

---

### Task 5 (O6): Collapse the period-edge methods into `DatePoint::snap`

**Files:**
- Modify: `src/temporal/point.rs:340-372` (replace `start_of` + `end_of` with one `snap`), tests at :2324/:2353/:2382/:2440/:2469/:2498 (multiline calls) and :3185/:3188/:3210/:3213 (proptest chain calls)
- Modify: `src/template/engine/date.rs` (transform arms :194/:197, `render_period_edge` match :698-702)
- Modify: `src/query/grammar/filter.rs:640-653` (two closure bodies)
- Baseline: `tests/baselines/pub-tokens.txt` (net −1)

**Context:** `start_of` (:347) and `end_of` (:364) are line-for-line identical except for the `PeriodEdge` constant passed to `calendar_period.snap_date`. Three call sites re-derive that dispatch: the engine's `Anchor::transform` picks the method per anchor variant, `render_period_edge` re-matches the edge in a two-arm `match`, and query's `filter.rs` keeps one closure per filter variant. O6 makes `DatePoint::snap(calendar_period, period_edge, zone)` the single point-level primitive, so callers pass the edge through instead of re-selecting it; the name matches the `snap_date`/`snap_weekday` verb family `calendar.rs` already owns. The `snap` body is exactly `start_of`'s body with `period_edge` as the parameter, so behavior is identical at every site (including fetch order: each caller still resolves the zone before snapping). Query impact is mechanical and coupling-neutral: `filter.rs` already imports `PeriodEdge` (:35), depends on `DatePoint` already, and gains no new import or type across the boundary; `date_boundary`'s `CalendarError::Zone` wildcard stays unchanged. Filter and namespace NAMES (`start_of_week`/`sow`, engine `NAMESPACE_METHODS` :51-70, module doc :14, query test cases :1979-2112) are untouched: they name the surface, not the method. The test mods `mod start_of`/`mod end_of` keep their names deliberately (they name the behavior under test) while every call inside migrates; all point, engine, and filter tests are **green before and after**. Visibility: net −1 pub token (two `pub(crate)` methods deleted, one added). GLOSSARY: **no edit**; no glossary line mentions `start_of`/`end_of`, and `snap` extends the existing `snap_date` vocabulary rather than adding a term.

- [ ] **Step 1: Replace `start_of` and `end_of` with `snap`**

  In `src/temporal/point.rs`, replace both methods (their doc comments included):

  ```rust
      /// Returns the first day of the period, preserving written precision and
      /// time.
      ///
      /// # Errors
      ///
      /// - [`CalendarError::OutOfRange`] if the boundary date is outside the
      ///   supported range.
      pub(crate) fn start_of(
          self,
          calendar_period: CalendarPeriod,
          zone: &TimeZone,
      ) -> Result<Self, CalendarError> {
          let date = self.local_datetime_in(zone).date();
          let target = calendar_period.snap_date(PeriodEdge::Start, date);
          self.with_date(target, zone)
      }

      /// Returns the last day of the period, preserving written precision and
      /// time.
      ///
      /// # Errors
      ///
      /// - [`CalendarError::OutOfRange`] if the boundary date is outside the
      ///   supported range.
      pub(crate) fn end_of(
          self,
          calendar_period: CalendarPeriod,
          zone: &TimeZone,
      ) -> Result<Self, CalendarError> {
          let date = self.local_datetime_in(zone).date();
          let target = calendar_period.snap_date(PeriodEdge::End, date);
          self.with_date(target, zone)
      }
  ```

  with:

  ```rust
      /// Returns the first or last day of the period selected by `period_edge`,
      /// preserving written precision and time.
      ///
      /// # Errors
      ///
      /// - [`CalendarError::OutOfRange`] if the boundary date is outside the
      ///   supported range.
      pub(crate) fn snap(
          self,
          calendar_period: CalendarPeriod,
          period_edge: PeriodEdge,
          zone: &TimeZone,
      ) -> Result<Self, CalendarError> {
          let date = self.local_datetime_in(zone).date();
          let target = calendar_period.snap_date(period_edge, date);
          self.with_date(target, zone)
      }
  ```

  Note: `PeriodEdge` is already imported at the top of `point.rs` (the old bodies referenced it); `snap_date`'s `(edge, date)` argument order is unchanged.

- [ ] **Step 2: Migrate the point.rs test call sites**

  Four replace-all instructions in `src/temporal/point.rs`.

  1. The multiline start form occurs 3 times (test fns `returns_the_first_day_of_the_period`, `preserves_month_precision`, `keeps_the_time_of_day_on_a_date_time_point` in `mod start_of`). Replace all:

  ```rust
              let start = base.start_of(
                  period,
                  &local_zone().expect("system time zone available"),
              );
  ```

  with:

  ```rust
              let start = base.snap(
                  period,
                  PeriodEdge::Start,
                  &local_zone().expect("system time zone available"),
              );
  ```

  2. The multiline end form occurs 3 times (`mod end_of`, same three fn names). Replace all:

  ```rust
              let end = base.end_of(
                  period,
                  &local_zone().expect("system time zone available"),
              );
  ```

  with:

  ```rust
              let end = base.snap(
                  period,
                  PeriodEdge::End,
                  &local_zone().expect("system time zone available"),
              );
  ```

  3. The proptest start chain occurs 3 times (lines ~3185, ~3188, ~3210). Replace all:

  ```rust
                      .start_of(period, &local_zone().expect("system time zone available"))
  ```

  with:

  ```rust
                      .snap(period, PeriodEdge::Start, &local_zone().expect("system time zone available"))
  ```

  4. The proptest end chain occurs once (~3213). Replace it:

  ```rust
                      .end_of(period, &local_zone().expect("system time zone available"))
  ```

  with:

  ```rust
                      .snap(period, PeriodEdge::End, &local_zone().expect("system time zone available"))
  ```

  Notes: the new proptest lines exceed 100 columns; `mise run verify` runs `fmt` first, which reflows those calls vertically, and that reflow is expected. Keep the test mod names `mod start_of` and `mod end_of`: they describe the boundary behavior under test, and nothing outside the file filters on them.

- [ ] **Step 3: Migrate the engine call sites**

  Two edits in `src/template/engine/date.rs`.

  1. In `Anchor::transform` (the text as rewritten by Task 4), replace the two arms:

  ```rust
              Self::StartOf(calendar_period) => date_point
                  .start_of(calendar_period, &zone()?)
                  .map_err(render_calendar_error),
              Self::EndOf(calendar_period) => date_point
                  .end_of(calendar_period, &zone()?)
                  .map_err(render_calendar_error),
  ```

  with:

  ```rust
              Self::StartOf(calendar_period) => date_point
                  .snap(calendar_period, PeriodEdge::Start, &zone()?)
                  .map_err(render_calendar_error),
              Self::EndOf(calendar_period) => date_point
                  .snap(calendar_period, PeriodEdge::End, &zone()?)
                  .map_err(render_calendar_error),
  ```

  2. In `render_period_edge`, replace the edge match:

  ```rust
      let shifted = match period_edge {
          PeriodEdge::Start => date_point.start_of(calendar_period, &zone),
          PeriodEdge::End => date_point.end_of(calendar_period, &zone),
      }
      .map_err(render_calendar_error)?;
  ```

  with:

  ```rust
      let shifted = date_point
          .snap(calendar_period, period_edge, &zone)
          .map_err(render_calendar_error)?;
  ```

  Note: `PeriodEdge` is already in the engine's `temporal::{...}` import, and the function's `period_edge` parameter now flows straight through. The doc comment above `render_period_edge` (:680-686) describes filter names and error kinds only, so it stays as written.

- [ ] **Step 4: Migrate the query call sites**

  In `src/query/grammar/filter.rs`, replace both closure bodies:

  ```rust
              Self::StartOf(calendar_period) => {
                  let zone = crate::temporal::local_zone().ok()?;
                  Self::date_boundary(args, |date_point| {
                      date_point.start_of(calendar_period, &zone)
                  })
              }
              Self::EndOf(calendar_period) => {
                  let zone = crate::temporal::local_zone().ok()?;
                  Self::date_boundary(args, |date_point| {
                      date_point.end_of(calendar_period, &zone)
                  })
              }
  ```

  with:

  ```rust
              Self::StartOf(calendar_period) => {
                  let zone = crate::temporal::local_zone().ok()?;
                  Self::date_boundary(args, |date_point| {
                      date_point.snap(calendar_period, PeriodEdge::Start, &zone)
                  })
              }
              Self::EndOf(calendar_period) => {
                  let zone = crate::temporal::local_zone().ok()?;
                  Self::date_boundary(args, |date_point| {
                      date_point.snap(calendar_period, PeriodEdge::End, &zone)
                  })
              }
  ```

  Note: no import change (`PeriodEdge` at :39-41); `date_boundary` and its `CalendarError::Zone` wildcard are deliberately untouched.

- [ ] **Step 5: Run the migrated tests**

  Run: `mise run test:unit start_of`
  Expected: PASS (point `mod start_of` cases, engine `mod start_and_end_of_month`, and query `start_of_*` cases all route through `snap` now).

  Run: `mise run test:unit end_of`
  Expected: PASS.

  Both filters are **green before and after**: the migration changes which method is called, never what it computes.

- [ ] **Step 6: Regenerate the pub-token baseline**

  Run:

  ```
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  ```

  Expected: net −1 line. Two lines disappear (`src/temporal/point.rs: pub(crate) fn start_of(`, `src/temporal/point.rs: pub(crate) fn end_of(`), one appears (`src/temporal/point.rs: pub(crate) fn snap(`), and nothing else changes. Both `cargo public-api` baselines stay byte-identical.

- [ ] **Step 7: Run the full gate**

  Run: `mise run verify`
  Expected: PASS (fmt applies the proptest reflow, then check including `check:visibility`, lint, full test suite).

- [ ] **Step 8: Commit**

  ```
  git add src/temporal/point.rs src/template/engine/date.rs src/query/grammar/filter.rs tests/baselines/pub-tokens.txt
  git commit -m "refactor(temporal): collapse period edges into DatePoint::snap"
  ```

---

### Task 6 (P1): Move the moment-dialect translator to `temporal::token`

**Files:**
- Create: `src/temporal/token.rs` (~150 lines: module doc, `MOMENT_TOKENS`, `ordinal_suffix`, `moment_to_strftime`, `pattern_to_strftime`)
- Modify: `src/temporal/error.rs` (declare `TemporalTokenError` between `UnitError` and the tests mod, plus `use std::fmt;`)
- Modify: `src/template/engine/date.rs` (delete `MOMENT_TOKENS` :498-520, `ordinal_suffix` :522-534, `moment_to_strftime` :536-623; replace `pattern_to_strftime` :482-496 with a thin wrapper; ~130 lines removed)
- Modify: `src/temporal/mod.rs` (+2 lines: `mod token;`, `pub(crate) use token::pattern_to_strftime;`)
- Modify: `.scratch/temporal-core/spec.md:105`, `.scratch/temporal-core/review.md:113`, `.scratch/temporal-core/review.md:449` (design-doc sync)
- Baseline: `tests/baselines/pub-tokens.txt` (+3)

**Context:** `engine/date.rs:482-623` holds a self-contained moment-dialect tokenizer: `pattern_to_strftime` (the `%`-contains dialect dispatch), `MOMENT_TOKENS` (a 21-entry table), `ordinal_suffix` (the `Do` ordinal, calendar knowledge), and `moment_to_strftime` (the ~90-line tokenizer). It is a pure `(&str, jiff::civil::DateTime) -> String` translation whose only minijinja contact is constructing `minijinja::Error` directly at its two failure sites (`engine/date.rs:558-565` unclosed bracket, `:598-606` unsupported token): a layering violation, since a template-library error is built inside what is otherwise date-domain logic. Task 6 moves the tokenizer into a new `temporal::token` module and declares a temporal-native `TemporalTokenError` in `error.rs` (error types live in error.rs); the engine keeps a same-named thin wrapper that maps `TemporalTokenError` to the existing `minijinja::Error` message, so every engine call site (`:256`, `:476`, `:936`, `:1669`) and every test compiles unchanged and stays **green before and after**: the wrapper preserves the exact interface (`TemplateEngineResult<String>`), error kind (`InvalidOperation`), and message text that `rejects_unsupported_moment_token_with_token_and_dialect_name` (:1635) and `rejects_unclosed_bracket_literal_in_moment_dialect` (:1649) pin through the template seam. The `%`-contains dispatch moves with the tokenizer: it is the translator's own front door, and the engine wrapper delegates it wholesale. `Error`/`ErrorKind` imports stay in the engine (63 uses remain outside :482-623, in `render_date_error`/`render_calendar_error` and the other filters), so no import line changes there. Design-doc sync: the audit's Section C recorded this translator as "template-seam only" in three artifacts; moving it falsifies that, so `spec.md:105` (the formatting-seam decision), `review.md:113` (D-b "template seam only"), and `review.md:449` (D8's translator-error note) are amended in the same change; the remaining stale "template seam"/`format_with` mentions in `issues/06` and `spec.md:104`'s chrono-era `DelayedFormat` wording are left to the final doc-consistency pass (task D) since Task 6 does not touch `format_strtime`. Visibility: +3 pub tokens (`pattern_to_strftime` in `token.rs`, `TemporalTokenError` in `error.rs`, one re-export in `mod.rs`); both `cargo public-api` baselines stay byte-identical: the `token.rs` item and the re-export are `pub(crate)`, and `TemporalTokenError` is `pub` inside the private `mod error`, unreachable from outside just like `UnitError`. GLOSSARY: **no edit**; `TemporalTokenError`/`pattern_to_strftime` are internal implementation names, not domain vocabulary. The full moment-token matrix required by `spec.md:127` (Seam 3, "moment-token rendering (incl. the token-naming error)") and issue-06's AC ("the supported moment token set ... pinned by tests") is deferred to X4 (Task 7), which restructures this tokenizer's matching and is the natural home for it; `spec.md:136` is an out-of-scope line, not a test promise.

- [ ] **Step 1: Create `src/temporal/token.rs`**

  Create the file with a module doc, the token table, and the two moved functions. `MOMENT_TOKENS`, `ordinal_suffix`, and the `moment_to_strftime` body move **verbatim** from `engine/date.rs:498-623`. The only edits inside `moment_to_strftime` are its two `Err(...)` sites (the `minijinja::Error` constructions become `TemporalTokenError` values) and its return type. Full content:

  ```rust
  //! Translates moment-dialect date patterns to strftime patterns.
  //!
  //! The moment dialect is the template-facing token language (`YYYY-MM-DD`,
  //! `Do MMMM YYYY`, `[Daily]`) that the `date_format` filter accepts. This
  //! module owns the tokenizer that converts it to a strftime pattern jiff can
  //! render; patterns it cannot translate fail with [`TemporalTokenError`]
  //! (declared in `error.rs`). This module's `pattern_to_strftime` owns
  //! the `%`-contains dialect dispatch; the engine's same-named wrapper
  //! calls it and maps the error to a template render error.

  use super::error::TemporalTokenError;

  /// The moment-dialect to strftime token table (scope 1).
  const MOMENT_TOKENS: &[(&str, &str)] = &[
      ("YYYY", "%Y"),
      ("YY", "%y"),
      ("MMMM", "%B"),
      ("MMM", "%b"),
      ("MM", "%m"),
      ("M", "%-m"),
      ("dddd", "%A"),
      ("ddd", "%a"),
      ("DD", "%d"),
      ("D", "%-d"),
      ("HH", "%H"),
      ("H", "%-H"),
      ("hh", "%I"),
      ("h", "%-I"),
      ("mm", "%M"),
      ("m", "%-M"),
      ("ss", "%S"),
      ("s", "%-S"),
      ("SSS", "%3f"),
      ("SS", "%2f"),
      ("S", "%1f"),
  ];

  /// Translates `pattern` to a strftime pattern.
  ///
  /// If `pattern` contains `%`, it is treated as a strftime pattern and passed
  /// through. Otherwise, it is translated from common moment-dialect tokens:
  /// bracket literals (`[Daily]`), `YYYY MM DD HH mm ss` family, `Do`/`S`
  /// ordinals, and `dddd`/`ddd`/`MMM`/`MMMM`.
  ///
  /// # Errors
  ///
  /// Returns [`TemporalTokenError::UnclosedBracket`] for an unterminated `[...]`
  /// literal and [`TemporalTokenError::UnsupportedToken`] for a moment token with no
  /// strftime equivalent.
  pub(crate) fn pattern_to_strftime(
      pattern: &str,
      datetime: jiff::civil::DateTime,
  ) -> Result<String, TemporalTokenError> {
      if pattern.contains('%') {
          return Ok(pattern.to_owned());
      }
      moment_to_strftime(pattern, datetime)
  }

  /// Returns the English ordinal suffix (`"st"`, `"nd"`, `"rd"`, `"th"`) for
  /// `day`.
  fn ordinal_suffix(day: i8) -> &'static str {
      match day {
          11..=13 => "th",
          _ => match day % 10 {
              1 => "st",
              2 => "nd",
              3 => "rd",
              _ => "th",
          },
      }
  }

  /// Translates a moment-dialect pattern into a strftime format string.
  fn moment_to_strftime(
      pattern: &str,
      datetime: jiff::civil::DateTime,
  ) -> Result<String, TemporalTokenError> {
      use std::fmt::Write as _;

      let mut result = String::with_capacity(pattern.len().saturating_mul(2));
      let bytes = pattern.as_bytes();
      let len = bytes.len();
      let mut i = 0;

      while i < len {
          let Some(&current_byte) = bytes.get(i) else {
              break;
          };
          if current_byte == b'[' {
              i = i.saturating_add(1);
              let start = i;
              while i < len && bytes.get(i) != Some(&b']') {
                  i = i.saturating_add(1);
              }
              if bytes.get(i) != Some(&b']') {
                  return Err(TemporalTokenError::UnclosedBracket {
                      pattern: pattern.to_owned(),
                  });
              }
              let literal = pattern.get(start..i).unwrap_or_default();
              i = i.saturating_add(1);
              for ch in literal.chars() {
                  if ch == '%' {
                      result.push_str("%%");
                  } else {
                      result.push(ch);
                  }
              }
          } else if pattern.get(i..).is_some_and(|tail| tail.starts_with("Do")) {
              i = i.saturating_add(2);
              let day = datetime.day();
              let _ = write!(result, "{day}{}", ordinal_suffix(day));
          } else if current_byte.is_ascii_alphabetic() {
              let mut matched = false;
              let tail = pattern.get(i..).unwrap_or_default();
              for &(tok, spec) in MOMENT_TOKENS {
                  if tail.starts_with(tok) {
                      result.push_str(spec);
                      i = i.saturating_add(tok.len());
                      matched = true;
                      break;
                  }
              }
              if !matched {
                  let start = i;
                  let ch = current_byte;
                  while i < len && bytes.get(i) == Some(&ch) {
                      i = i.saturating_add(1);
                  }
                  let token = pattern.get(start..i).unwrap_or_default();
                  return Err(TemporalTokenError::UnsupportedToken {
                      token: token.to_owned(),
                      pattern: pattern.to_owned(),
                  });
              }
          } else {
              let ch = pattern
                  .get(i..)
                  .and_then(|tail| tail.chars().next())
                  .unwrap_or(' ');
              i = i.saturating_add(ch.len_utf8());
              if ch == '%' {
                  result.push_str("%%");
              } else {
                  result.push(ch);
              }
          }
      }

      Ok(result)
  }
  ```

  Then declare the error type in `src/temporal/error.rs`. Add `use std::fmt;` after the module doc, and insert the enum between the `UnitError` enum (Task 3's addition) and the tests mod:

  ```rust
  /// An error translating a moment-dialect pattern to strftime.
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum TemporalTokenError {
      /// A bracket literal (`[...]`) was opened but never closed.
      UnclosedBracket {
          /// The pattern containing the unclosed bracket.
          pattern: String,
      },
      /// A token has no moment-dialect to strftime mapping.
      UnsupportedToken {
          /// The offending token.
          token: String,
          /// The pattern containing the token.
          pattern: String,
      },
  }

  impl fmt::Display for TemporalTokenError {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
          match self {
              Self::UnclosedBracket { pattern } => write!(
                  f,
                  "unclosed bracket literal in moment-dialect pattern `{pattern}`"
              ),
              Self::UnsupportedToken { token, pattern } => write!(
                  f,
                  "unsupported moment-dialect token `{token}` in pattern \
                   `{pattern}`; supported tokens are YYYY, YY, MMMM, MMM, MM, M, \
                   dddd, ddd, Do, DD, D, HH, H, hh, h, mm, m, ss, s, SSS, SS, S, \
                   and [bracketed literals]"
              ),
          }
      }
  }

  impl std::error::Error for TemporalTokenError {}
  ```

  Notes: the two `Display` arms reproduce the engine's current messages **byte-for-byte**. The `\` continuations reflow the same joined strings the `format!` calls at engine `:561-564`/`:600-605` produce. The template-seam tests that assert on message substrings keep passing. `TemporalTokenError` carries `pattern: String` (and `token: String`) so `Display` can render the full context the messages include. Both fields are `Send + Sync`, satisfying `invalid_operation`'s source bound. The `use std::fmt::Write as _;` stays function-local inside `moment_to_strftime` as it is today.

- [ ] **Step 2: Register the module in `src/temporal/mod.rs`**

  Two additions. First, add `mod token;` to the module declarations, alphabetically between `point` and `unit` (rustfmt also sorts these):

  ```rust
  mod point;
  mod token;
  mod unit;
  ```

  Second, add the re-export to the `pub(crate) use` block, alphabetically between the `point` and `unit` re-exports:

  ```rust
  pub(crate) use point::{DateDiff, DatePoint};
  pub(crate) use token::pattern_to_strftime;
  pub(crate) use unit::{DateUnit, TimeUnit};
  ```

  Note: `mod token;` is private (the sibling module). `pub(crate) use token::pattern_to_strftime;` exposes the re-exported name at `crate::temporal::pattern_to_strftime` with `pub(crate)` visibility. That is the name the engine wrapper calls. `TemporalTokenError` is deliberately **not** re-exported: the engine maps it with type inference (`|source| invalid_operation(source.to_string(), source)`) and never names the type.

- [ ] **Step 3: Replace the engine's `pattern_to_strftime` with the thin wrapper**

  In `src/template/engine/date.rs`, replace the function and its doc comment (`:482-496`):

  ```rust
  /// Translates `pattern` to a strftime pattern.
  ///
  /// If `pattern` contains `%`, it is treated as a strftime pattern and passed
  /// through. Otherwise, it is translated from common moment-dialect tokens:
  /// bracket literals (`[Daily]`), `YYYY MM DD HH mm ss` family, `Do`/`S`
  /// ordinals, and `dddd`/`ddd`/`MMM`/`MMMM`.
  fn pattern_to_strftime(
      pattern: &str,
      datetime: jiff::civil::DateTime,
  ) -> TemplateEngineResult<String> {
      if pattern.contains('%') {
          return Ok(pattern.to_owned());
      }
      moment_to_strftime(pattern, datetime)
  }
  ```

  with:

  ```rust
  /// Translates `pattern` to a strftime pattern.
  ///
  /// Delegates to the temporal pattern translator and maps its error to a
  /// template render error, preserving the engine's error contract. The
  /// temporal side owns the `%`-contains dialect dispatch and the
  /// moment-dialect tokenizer.
  fn pattern_to_strftime(
      pattern: &str,
      datetime: jiff::civil::DateTime,
  ) -> TemplateEngineResult<String> {
      crate::temporal::pattern_to_strftime(pattern, datetime)
          .map_err(|source| invalid_operation(source.to_string(), source))
  }
  ```

  Then delete the three moved items that followed it: `MOMENT_TOKENS` (`:498-520`), `ordinal_suffix` (`:522-534`), and `moment_to_strftime` (`:536-623`). Delete them as one contiguous block: the const, the fn, and their doc comments. The wrapper calls `crate::temporal::pattern_to_strftime` by full path (no `use` import added, to avoid colliding with the local fn name). `invalid_operation` and `TemplateEngineResult` are already imported at `:42`. The four call sites (`:256`, `:476`, `:936`, `:1669`) are unchanged, since they still call the local `pattern_to_strftime`.

- [ ] **Step 4: Sync the design docs**

  Three surgical amendments so the recorded decisions stay true after the move.

  1. `.scratch/temporal-core/spec.md:105`: replace the "template-seam only" translator line:

  ```
  - Moment→strftime translator is template-seam only, scope 1: bracket literals, `YYYY MM DD HH mm ss` family, `Do`/`S`, `dddd`/`ddd`/`MMM`/`MMMM`; everything else errors naming the token and dialect. The only strftime-grammar surface left is the engine's private `format_with` (template seam).
  ```

  with:

  ```
  - Moment→strftime translator lives in `temporal::token` (`src/temporal/token.rs`), scope 1: bracket literals, `YYYY MM DD HH mm ss` family, `Do`/`S`, `dddd`/`ddd`/`MMM`/`MMMM`; everything else errors naming the token and dialect via `TemporalTokenError`. The engine keeps a thin `pattern_to_strftime` wrapper that delegates the `%`-contains dispatch to the translator and maps `TemporalTokenError` to a template render error. **(Amended by temporal-offload Task 6:** the translator moved from the template seam to `temporal::token`; the "template-seam only" claim is retired.**)**
  ```

  2. `.scratch/temporal-core/review.md:113`: the D-b decision line, located by its unique tail `is now true by deletion.)*`, gets this note appended on the line after it:

  ```
  - *(2026-10-24, temporal-offload Task 6: the translator moved to `temporal::token`; the engine retains only a `pattern_to_strftime` wrapper that delegates dialect dispatch and maps `TemporalTokenError`. The "template seam only" location claim above is retired; the error-message and scope decisions stand.)*
  ```

  3. `.scratch/temporal-core/review.md:449`: the D8 `**Observed:**` paragraph, append a sync note at its end (after "06's edge is 04 only."):

  ```
   (2026-10-24, temporal-offload Task 6: the translator now lives in `temporal::token` and returns `TemporalTokenError`; the engine wrapper still maps it to a template-render error, so the "not a `DateError`" decision holds.)
  ```

  Note: `spec.md:104`'s stale `format_with`/`DelayedFormat` wording and the remaining "template seam" mentions in `issues/06-template-format-dialects.md` are **not** touched here. Task 6 does not change `format_strtime` (the strftime renderer named there). Those references are swept by the final doc-consistency task (D).

- [ ] **Step 5: Run the translator tests**

  Run: `mise run test:unit moment`
  Expected: PASS. The `formats_moment_tokens` rstest table, `rejects_unsupported_moment_token_with_token_and_dialect_name`, `rejects_unclosed_bracket_literal_in_moment_dialect`, and `format_bindings_never_emit_percent_plus` all now route through `crate::temporal::pattern_to_strftime`. Their assertions on error kind and message substrings hold because the wrapper and `Display` reproduce the previous contract exactly.

  Run: `mise run test:unit date_format`
  Expected: PASS (the filters that call `pattern_to_strftime` render unchanged).

  Both are **green before and after**: this is code motion behind a behavior-preserving wrapper, so no test changes.

- [ ] **Step 6: Regenerate the pub-token baseline**

  Run:

  ```
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  ```

  Expected: the diff adds exactly three lines:

  ```
  src/temporal/mod.rs: pub(crate) use token::pattern_to_strftime;
  src/temporal/error.rs: pub enum TemporalTokenError {
  src/temporal/token.rs: pub(crate) fn pattern_to_strftime(
  ```

  and nothing else changes (`mod token;` is not a `pub` line, so it does not appear). Both `cargo public-api` baselines stay byte-identical: the `token.rs` item and the re-export are `pub(crate)`, and the `error.rs` enum is `pub` inside the private `mod error`, unreachable from outside the crate just like `UnitError`.

- [ ] **Step 7: Run the full gate**

  Run: `mise run verify`
  Expected: PASS (fmt, check including `check:visibility`, lint, full test suite).

- [ ] **Step 8: Commit**

  ```
  git add src/temporal/token.rs src/temporal/error.rs src/temporal/mod.rs src/template/engine/date.rs tests/baselines/pub-tokens.txt .scratch/temporal-core/spec.md .scratch/temporal-core/review.md
  git commit -m "refactor(temporal): move moment-dialect translator to temporal::token"
  ```

---

### Task 7 (X4): Match moment tokens by the full letter-run

**Files:**
- Modify: `src/temporal/token.rs` (add `match_moment_run`; rewrite the alphabetic branch of `moment_to_strftime`)
- Tests only: `src/template/engine/date.rs`
- No baseline, doc, or GLOSSARY changes: `match_moment_run` is private, so the pub-token snapshot stays identical.

**Context:** `moment_to_strftime` still matches tokens with a greedy prefix scan: at an alphabetic position it takes the remaining tail of the pattern and consumes the first `MOMENT_TOKENS` entry that prefixes it. A longer run of one letter therefore decomposes into adjacent shorter tokens instead of failing. Walking the current loop confirms ten silent compositions: `DDD` → `"%d%-d"` (renders `"2929"` for July 29), `DDDD` → `"%d%d"`, `MMMMM` → `"%B%-m"`, `HHH` → `"%H%-H"`, `hhh` → `"%I%-I"`, `mmm` → `"%M%-M"`, `sss` → `"%S%-S"`, `SSSS` → `"%3f%1f"`, `Y×6` → `"%Y%y"`, `Y×8` → `"%Y%Y"`; other lengths decompose too (`hhhh`, `mmmm`, `ssss`, `DDDDDD`, `MMMMMM`). None of these reach the unsupported-token error, which violates the recorded "never silently mistranslate" rule (`issues/06-template-format-dialects.md`). The fix keeps the `Do` special-case first (load-bearing: checked against `Dooo`, `DDo`, `MMDo`, `do`), then consumes the maximal run of the repeated byte and looks the whole run up with a new private `match_moment_run(ch, run_len)` helper that does exact equality against the table; an unknown run returns `TemporalTokenError::UnsupportedToken` naming the full run. The rule is total: every table entry except `Do` is a single-character repeat, so the token-bearing letters are exactly `{Y, M, d, D, H, h, m, s, S}` and any run either equals an entry or errors. Error contract: the `Display` token list stays untouched (the `Q` test stays byte-identical), `dd`/`d`/`yyy`/`do` report the same token name before and after, and only partial-consumption leftovers change name (`YYY` now names `YYY` where it named `Y`; `ddddd` now names `ddddd` where it named `d`); both deltas are pinned red-first below. Tests stay on the template seam next to `formats_moment_tokens`, as Seam 3 requires (`spec.md:127`); the matrix satisfies issue-06's AC "the supported moment token set ... pinned by tests". Task 8 (X5) rewrites the adjacent `%`-dispatch code in this same function; the task order keeps them serial.

- [ ] **Step 1: Add the full-table matrix test**

  Add this rstest directly after `formats_moment_tokens` in the `date_format` test module. It is green before and after: every table entry already translates correctly.

  ```rust
        #[rstest]
        #[case::year_full("YYYY", "2026")]
        #[case::year_short("YY", "26")]
        #[case::month_full("MMMM", "July")]
        #[case::month_abbr("MMM", "Jul")]
        #[case::month_padded("MM", "07")]
        #[case::month_short("M", "7")]
        #[case::weekday_full("dddd", "Sunday")]
        #[case::weekday_abbr("ddd", "Sun")]
        #[case::day_padded("DD", "05")]
        #[case::day_short("D", "5")]
        #[case::hour24_padded("HH", "04")]
        #[case::hour24_short("H", "4")]
        #[case::hour12_padded("hh", "04")]
        #[case::hour12_short("h", "4")]
        #[case::minute_padded("mm", "05")]
        #[case::minute_short("m", "5")]
        #[case::second_padded("ss", "03")]
        #[case::second_short("s", "3")]
        #[case::fraction_3("SSS", "123")]
        #[case::fraction_2("SS", "12")]
        #[case::fraction_1("S", "1")]
        #[case::ordinal_day("Do", "5th")]
        #[case::bracket_literal("[Daily]", "Daily")]
        fn covers_the_full_moment_token_table(
            #[case] format: &str,
            #[case] expected: &str,
        ) {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    &format!(r#"{{{{ value | date_format("{format}") }}}}"#),
                    minijinja::context! { value => "2026-07-05T04:05:03.123" },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }
  ```

  The input is Sunday, July 5, 2026 at 04:05:03.123: single-digit fields make every padded and unpadded pair render differently. `SS`/`S` exercise the `%2f`/`%1f` precision directives.

  Run: `mise run test:unit covers_the_full_moment_token_table`
  Expected: PASS (23 cases: all 21 table entries, `Do`, one bracket literal).

- [ ] **Step 2: Add the run-rejection test (the intended red)**

  Add this rstest after the matrix test. Twelve cases fail before Step 3 because those runs compose silently today. Four cases pass before and after.

  ```rust
        #[rstest]
        #[case::triple_d("DDD", "DDD")]
        #[case::quadruple_d("DDDD", "DDDD")]
        #[case::five_m("MMMMM", "MMMMM")]
        #[case::triple_upper_h("HHH", "HHH")]
        #[case::triple_lower_h("hhh", "hhh")]
        #[case::triple_lower_m("mmm", "mmm")]
        #[case::triple_lower_s("sss", "sss")]
        #[case::four_upper_s("SSSS", "SSSS")]
        #[case::six_y("YYYYYY", "YYYYYY")]
        #[case::eight_y("YYYYYYYY", "YYYYYYYY")]
        #[case::triple_y("YYY", "YYY")]
        #[case::five_d("ddddd", "ddddd")]
        #[case::two_lower_d("dd", "dd")]
        #[case::one_lower_d("d", "d")]
        #[case::three_lower_y("yyy", "yyy")]
        #[case::lowercase_do("do", "d")]
        fn rejects_runs_that_are_not_moment_tokens(
            #[case] format: &str,
            #[case] token: &str,
        ) {
            ZoneGuard::set("UTC");

            let error = env()
                .render_str(
                    &format!(r#"{{{{ value | date_format("{format}") }}}}"#),
                    minijinja::context! { value => "2026-07-05" },
                )
                .expect_err("a non-token run fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
            let msg = error.to_string();
            assert!(
                msg.contains(&format!(
                    "unsupported moment-dialect token `{token}`"
                )),
                "expected token `{token}` in: {msg}"
            );
        }
  ```

  `YYY` and `ddddd` are red first because the current scanner reports only the leftover fragment (`Y` and `d`). The `dd`, `d`, `yyy`, `do` cases are pins: they report the same token name before and after.

  Run: `mise run test:unit rejects_runs_that_are_not_moment_tokens`
  Expected: FAIL with 12 red cases (`DDD`, `DDDD`, `MMMMM`, `HHH`, `hhh`, `mmm`, `sss`, `SSSS`, `YYYYYY`, `YYYYYYYY`, `YYY`, `ddddd`). The 4 pins pass. This is the intended red. Do not proceed until the failures match this list.

- [ ] **Step 3: Rewrite the alphabetic branch and add the lookup helper**

  In `src/temporal/token.rs`, leave the bracket branch and the `Do` branch above it unchanged. The `Do` check must stay first: it consumes `Do` before run consumption can see the `D`. Replace the whole `else if current_byte.is_ascii_alphabetic()` branch:

  ```rust
          } else if current_byte.is_ascii_alphabetic() {
              let mut matched = false;
              let tail = pattern.get(i..).unwrap_or_default();
              for &(tok, spec) in MOMENT_TOKENS {
                  if tail.starts_with(tok) {
                      result.push_str(spec);
                      i = i.saturating_add(tok.len());
                      matched = true;
                      break;
                  }
              }
              if !matched {
                  let start = i;
                  let ch = current_byte;
                  while i < len && bytes.get(i) == Some(&ch) {
                      i = i.saturating_add(1);
                  }
                  let token = pattern.get(start..i).unwrap_or_default();
                  return Err(TemporalTokenError::UnsupportedToken {
                      token: token.to_owned(),
                      pattern: pattern.to_owned(),
                  });
              }
          }
  ```

  with:

  ```rust
          } else if current_byte.is_ascii_alphabetic() {
              let start = i;
              let ch = current_byte;
              while i < len && bytes.get(i) == Some(&ch) {
                  i = i.saturating_add(1);
              }
              let run_len = i - start;
              match match_moment_run(ch, run_len) {
                  Some(spec) => result.push_str(spec),
                  None => {
                      let token = pattern.get(start..i).unwrap_or_default();
                      return Err(TemporalTokenError::UnsupportedToken {
                          token: token.to_owned(),
                          pattern: pattern.to_owned(),
                      });
                  }
              }
          }
  ```

  Then add the helper directly after the `MOMENT_TOKENS` const:

  ```rust
  /// Returns the strftime spec for a maximal run of `ch` with length
  /// `run_len`, or `None` when the run is not a moment-dialect token.
  fn match_moment_run(ch: u8, run_len: usize) -> Option<&'static str> {
      MOMENT_TOKENS
          .iter()
          .copied()
          .find(|(tok, _)| {
              tok.len() == run_len && tok.bytes().all(|byte| byte == ch)
          })
          .map(|(_, spec)| spec)
  }
  ```

  Keep `match_moment_run` private. No other function changes. The bracket branch, the `Do` branch, the non-alphabetic fallback, `pattern_to_strftime`'s `%` dispatch, `ordinal_suffix`, `TemporalTokenError`, and its `Display` all stay as Task 6 wrote them.

- [ ] **Step 4: Run the translator tests**

  Run: `mise run test:unit moment`
  Expected: PASS. The new matrix (23 cases) passes. The new rejection table (16 cases, now including `YYY` reporting `YYY` and `ddddd` reporting `ddddd`) passes. The pre-existing pins (`formats_moment_tokens`, the `Q` unsupported-token test, the unclosed-bracket test, `format_bindings_never_emit_percent_plus`) all pass through the unchanged engine wrapper.

  Run: `mise run test:unit date_format`
  Expected: PASS (the rest of the filter suite renders unchanged).

- [ ] **Step 5: Run the full gate**

  Run: `mise run verify`
  Expected: green (no baseline regen needed: no `pub` line changed).

- [ ] **Step 6: Commit**

  ```
  git add src/temporal/token.rs src/template/engine/date.rs
  git commit -m "refactor(temporal): match moment tokens by full letter-run"
  ```

---

### Task 8 (X5): Detect the strftime dialect by % outside brackets

**Files:**
- Modify: `src/temporal/token.rs` (dialect dispatch, two new helpers, one import, two dead spots in `moment_to_strftime`, doc comments)
- Modify: `src/delimiter.rs` (add `find_closing_flat`, the flat sibling of `find_closing`, directly after it)
- Modify: `src/temporal/error.rs` (the `UnclosedBracket` `Display` arm)
- Modify: `src/template/engine/date.rs` (the `date_format` doc paragraphs and the wrapper doc line; tests)
- Baseline: `tests/baselines/pub-tokens.txt` (+1); no GLOSSARY changes.

**Context:** the dialect decision today is `pattern.contains('%')` anywhere in the pattern. Jiff has no bracket-literal syntax of its own, so every bracketed `%` runs as a real directive on the strftime path: `[%Y]` renders `[2026]`, `[Today's %Y]` renders `[Today's 2026]`, `[100%] and %H:%M` dies in jiff on `%]`, and unclosed `[Daily %H` silently renders `[Daily 14`. Meanwhile the moment path's bracket-escaping branches (copy literal content, double `%`) are unreachable for any pattern that contains `%`. The locked fix: the dialect is decided by `%` outside any bracket literal, and the bracket scan moves ahead of the decision so one function owns the unclosed-bracket error for both dialects. `pattern_to_strftime` becomes: `percent_outside_brackets(pattern)?` selects strftime (new `escape_bracket_percents` copies the pattern, drops the delimiters, doubles `%` inside brackets) or moment (tokenizer unchanged). Both scans jump bracket literals through a new `DelimiterType::find_closing_flat` in `src/delimiter.rs`, the flat sibling of `find_closing`: it returns the byte offset of the first closing character, with no quote, escape, or nesting tracking, so a stray `]` stays literal content and an inner `[` stays inside the literal, matching what the moment scanner already produces. Two spots in `moment_to_strftime` die: its unclosed-bracket arm (the scan ran first, so the pattern is closed by then) and the bare-`%` push in the non-alphabetic fallback (the moment path only runs when no `%` sits outside brackets); Task 7 left both for this task, and the tasks run serially. Error-contract changes, all in the plan's accepted list: unclosed-bracket now precedes the token walk, so `Q [x` reports the unclosed bracket instead of the `Q` token (pinned red-first); the `UnclosedBracket` `Display` drops "moment-dialect" because the error is now dialect-neutral (the pin asserts only the `unclosed bracket literal` prefix, so it stays green after the rename below). Four doc spots are owned by this task and must not be re-touched by Task D: the `date_format` syntax paragraph, the `date_format` `# Errors` paragraph, the engine wrapper's `%`-contains wording, and `pattern_to_strftime`'s own doc in `token.rs`.

- [ ] **Step 1: Add the bracket-behavior tests (the intended red)**

  Add this rstest in the `date_format` test module of `src/template/engine/date.rs`:

  ```rust
        #[rstest]
        #[case::bracket_percent_year("[%Y]", "%Y")]
        #[case::bracket_percent_day("[%d] YYYY", "%d 2026")]
        #[case::bracket_apostrophe("[Today's %Y]", "Today's %Y")]
        #[case::mixed_percent_bracket_and_directive(
            "[100%] and %H:%M",
            "100% and 14:30"
        )]
        #[case::stray_close_bracket("1]2", "1]2")]
        #[case::plain_strftime("%H:%M", "14:30")]
        fn renders_bracket_literals_as_text_in_both_dialects(
            #[case] format: &str,
            #[case] expected: &str,
        ) {
            ZoneGuard::set("UTC");

            let rendered = env()
                .render_str(
                    &format!(r#"{{{{ value | date_format("{format}") }}}}"#),
                    minijinja::context! { value => "2026-06-05T14:30:00" },
                )
                .expect("render succeeds");

            assert_eq!(rendered, expected);
        }
  ```

  Then rewrite the existing unclosed-bracket test so it covers both dialects, and add the precedence pin:

  ```rust
        #[test]
        fn rejects_unclosed_bracket_literal_in_any_dialect() {
            let error = env()
                .render_str(
                    r#"{{ "2026-06-05T14:30:00" | date_format("[Daily %H") }}"#,
                    minijinja::context!(),
                )
                .expect_err("unclosed bracket must fail");
            assert!(
                error.to_string().contains("unclosed bracket literal"),
                "expected unclosed bracket error, got: {error}"
            );
        }

        #[test]
        fn unclosed_bracket_takes_precedence_over_unsupported_token() {
            let error = env()
                .render_str(
                    r#"{{ "2026-06-05" | date_format("Q [x") }}"#,
                    minijinja::context!(),
                )
                .expect_err("unclosed bracket must fail first");
            assert!(
                error.to_string().contains("unclosed bracket literal"),
                "expected unclosed bracket error, got: {error}"
            );
        }
  ```

  The first test replaces `rejects_unclosed_bracket_literal_in_moment_dialect`. Rename it. Change its input to `[Daily %H`. The new input contains `%` and takes the strftime path today.

  Run: `mise run test:unit bracket`
  Expected: FAIL. Red:
  - `bracket_percent_year` (today renders `[2026]`)
  - `bracket_percent_day` (today renders `[05] YYYY`)
  - `bracket_apostrophe` (today renders `[Today's 2026]`)
  - `mixed_percent_bracket_and_directive` (today jiff errors on `%]`)
  - `rejects_unclosed_bracket_literal_in_any_dialect` (today renders `[Daily 14`)
  - `unclosed_bracket_takes_precedence_over_unsupported_token` (today reports the `Q` token)

  Green: `stray_close_bracket` and `plain_strftime`. This is the intended red.

- [ ] **Step 2: Add `find_closing_flat`, rewrite the dialect dispatch, and add the two helpers**

  In `src/delimiter.rs`, add `find_closing_flat` directly after `find_closing`:

  ```rust
      /// Finds the byte position of this delimiter's first closing character in
      /// `input`, tracking no nesting, quotes, or escapes.
      ///
      /// `input` begins immediately after this opening delimiter was consumed.
      /// Returns `Some(pos)` of the first closing character, or `None` if it
      /// never appears (always `None` for [`Self::DoubleBracket`]). The flat
      /// scan serves the bracket-literal dialect scans in `temporal::token`,
      /// whose grammar treats every closing character as a close;
      /// [`Self::find_closing`] keeps the nesting, quote, and escape tracking
      /// for note link parsing.
      #[must_use]
      pub(crate) fn find_closing_flat(self, input: &str) -> Option<usize> {
          let close = self.close_char()?;
          input
              .char_indices()
              .find(|&(_, ch)| ch == close)
              .map(|(pos, _)| pos)
      }
  ```

  In `src/temporal/token.rs`, replace the body of `pattern_to_strftime`:

  ```rust
      if pattern.contains('%') {
          return Ok(pattern.to_owned());
      }
      moment_to_strftime(pattern, datetime)
  ```

  with:

  ```rust
      if percent_outside_brackets(pattern)? {
          return Ok(escape_bracket_percents(pattern));
      }
      moment_to_strftime(pattern, datetime)
  ```

  Then add `use crate::DelimiterType;` above `use super::error::TemporalTokenError;` in the imports, and add the two helpers directly after `pattern_to_strftime`:

  ```rust
  /// Returns `true` when `%` occurs outside any bracket literal.
  ///
  /// # Errors
  ///
  /// Returns [`TemporalTokenError::UnclosedBracket`] when a bracket literal is
  /// opened but never closed. This scan runs for both dialects, so the
  /// unclosed error takes precedence over every translation error.
  fn percent_outside_brackets(pattern: &str) -> Result<bool, TemporalTokenError> {
      let mut percent = false;
      let mut rest = pattern;
      while let Some(open) = rest.find('[') {
          percent |= rest[..open].contains('%');
          let after = &rest[open + 1..];
          let Some(close) = DelimiterType::Bracket.find_closing_flat(after) else {
              return Err(TemporalTokenError::UnclosedBracket {
                  pattern: pattern.to_owned(),
              });
          };
          rest = &after[close + 1..];
      }
      Ok(percent | rest.contains('%'))
  }

  /// Copies `pattern` for jiff in the strftime dialect, dropping bracket
  /// delimiters and doubling `%` inside bracket literals so bracketed text
  /// renders literally.
  fn escape_bracket_percents(pattern: &str) -> String {
      let mut out = String::with_capacity(pattern.len());
      let mut rest = pattern;
      while let Some(open) = rest.find('[') {
          out.push_str(&rest[..open]);
          let after = &rest[open + 1..];
          let (close, tail) = match DelimiterType::Bracket.find_closing_flat(after) {
              Some(close) => (close, &after[close + 1..]),
              // Unreachable: percent_outside_brackets already rejected.
              None => (after.len(), ""),
          };
          for ch in after[..close].chars() {
              if ch == '%' {
                  out.push_str("%%");
              } else {
                  out.push(ch);
              }
          }
          rest = tail;
      }
      out.push_str(rest);
      out
  }
  ```

  The scan must walk the whole pattern: an early exit on `%` would miss a later unclosed bracket.

- [ ] **Step 3: Remove the two dead spots in `moment_to_strftime`**

  Delete the unclosed-bracket arm inside the bracket branch. The scan in Step 2 already rejected every unclosed pattern before this function runs:

  ```rust
              if bytes.get(i) != Some(&b']') {
                  return Err(TemporalTokenError::UnclosedBracket {
                      pattern: pattern.to_owned(),
                  });
              }
  ```

  The `while` loop above it and the `let literal = ...` line below it stay as they are. `TemporalTokenError::UnclosedBracket` is now constructed in exactly one place.

  Then simplify the non-alphabetic fallback. The moment path only runs when no `%` sits outside brackets, so `ch == '%'` is unreachable there:

  ```rust
              i = i.saturating_add(ch.len_utf8());
              if ch == '%' {
                  result.push_str("%%");
              } else {
                  result.push(ch);
              }
  ```

  becomes:

  ```rust
              i = i.saturating_add(ch.len_utf8());
              result.push(ch);
  ```

  The `%`-doubling inside bracket literals stays: it lives in the bracket branch and is now reachable, which is the point of this task.

- [ ] **Step 4: Update the message wording and the four doc spots**

  In `src/temporal/error.rs`, the `Display` arm for `UnclosedBracket` loses "moment-dialect" (the error is dialect-neutral now):

  ```rust
              Self::UnclosedBracket { pattern } => write!(
                  f,
                  "unclosed bracket literal in moment-dialect pattern `{pattern}`"
              ),
  ```

  becomes:

  ```rust
              Self::UnclosedBracket { pattern } => write!(
                  f,
                  "unclosed bracket literal in pattern `{pattern}`"
              ),
  ```

  `pattern_to_strftime`'s doc in `src/temporal/token.rs`:

  ```rust
  /// Translates `pattern` to a strftime pattern.
  ///
  /// If `pattern` contains `%`, it is treated as a strftime pattern and passed
  /// through. Otherwise, it is translated from common moment-dialect tokens:
  /// bracket literals (`[Daily]`), `YYYY MM DD HH mm ss` family, `Do`/`S`
  /// ordinals, and `dddd`/`ddd`/`MMM`/`MMMM`.
  ```

  becomes:

  ```rust
  /// Selects the dialect for `pattern` and translates it to a strftime pattern.
  ///
  /// A `%` directive outside any bracket literal selects the strftime dialect:
  /// the pattern is copied with `%` doubled inside bracket literals, so
  /// bracketed text renders literally. Otherwise the moment dialect applies
  /// and tokens (`YYYY MM DD HH mm ss`, `Do`/`S`, `dddd`/`ddd`/`MMM`/`MMMM`,
  /// bracket literals like `[Daily]`) are translated.
  ```

  (its `# Errors` paragraph already names both variants and needs no change). In `src/template/engine/date.rs`, the `date_format` syntax paragraph:

  ```rust
  /// Formats according to either a strftime pattern (containing `%`) or common
  /// moment-dialect tokens (`YYYY MM DD HH mm ss`, `Do`/`S`,
  /// `dddd`/`ddd`/`MMM`/`MMMM`, bracket literals like `[Daily]`).
  ```

  becomes:

  ```rust
  /// Formats according to a strftime pattern (any `%` directive outside a
  /// bracket literal) or moment-dialect tokens (`YYYY MM DD HH mm ss`,
  /// `Do`/`S`, `dddd`/`ddd`/`MMM`/`MMMM`). Bracket literals like `[Daily]`
  /// render as literal text in both dialects.
  ```

  the `date_format` `# Errors` paragraph:

  ```rust
  /// resolved (see [`parse_date_point`]), or if `pattern` is an invalid strftime
  /// pattern or contains an unsupported moment token.
  ```

  becomes:

  ```rust
  /// resolved (see [`parse_date_point`]), or if `pattern` has an unclosed
  /// bracket literal, is an invalid strftime pattern, or contains an
  /// unsupported moment token.
  ```

  and the engine wrapper's doc line:

  ```rust
  /// template render error, preserving the engine's error contract. The
  /// temporal side owns the `%`-contains dialect dispatch and the
  /// moment-dialect tokenizer.
  ```

  becomes:

  ```rust
  /// template render error, preserving the engine's error contract. The
  /// temporal side owns the bracket-aware dialect dispatch and the
  /// moment-dialect tokenizer.
  ```

- [ ] **Step 5: Run the translator tests**

  Run: `mise run test:unit bracket`
  Expected: PASS (all six cases and both error tests; `stray_close_bracket` and `plain_strftime` stayed green throughout).

  Run: `mise run test:unit moment`
  Expected: PASS with the full token matrix, the run-rejection table, the `Q` unsupported-token pin, and `format_bindings_never_emit_percent_plus`. The `%+` test's sample pattern has no bracketed `%`, so its premise is unchanged.

- [ ] **Step 6: Regenerate the pub-token baseline**

  Run:

  ```
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  ```

  Expected: the diff adds exactly one line:

  ```
  src/delimiter.rs: pub(crate) fn find_closing_flat(self, input: &str) -> Option<usize> {
  ```

  and nothing else changes. Both `cargo public-api` baselines stay byte-identical: `find_closing_flat` is `pub(crate)` inside an already-public module, so no new reachable path appears.

- [ ] **Step 7: Run the full gate**

  Run: `mise run verify`
  Expected: green (fmt, `check:visibility` included, lint, full test suite).

- [ ] **Step 8: Commit**

  ```
  git add src/temporal/token.rs src/delimiter.rs src/temporal/error.rs src/template/engine/date.rs tests/baselines/pub-tokens.txt
  git commit -m "fix(temporal): decide the date dialect by % outside brackets"
  ```

---

### Task 9 (H2): Rename `DateNamespace` to `DateOps` with a stateless register

**Files:**
- Modify: `src/template/engine/date.rs` (type name, const name, register signature/doc, call sites)
- Modify: `src/template/engine.rs` (import and one call site)
- Baseline: exactly 2 line rewrites, no count change.

**Context:** `DateNamespace` is a stateless unit struct, yet it registers through a consuming `register(self, env)`, which is the convention only for the stateful ops (`FileOps`, `PathOps`, `QueryOps`, `SchemaOps`, `UiOps`, all constructed with captured data: `SchemaOps::new(service).register(&mut env)` and `PathOps::new(root).register(&mut env)`). Every stateless sibling (`StrOps`, `NumOps`, `YamlOps`) exposes an associated `register(env)`, and `YamlOps` documents the rule verbatim: "This is an associated function because `YamlOps` carries no state." The namespace module breaks two more conventions: its const is `NAMESPACE_METHODS` while every other module says `METHODS` (`file.rs:40`), and its type name ends in `Namespace` while every engine module exports `*Ops`. This task makes the module match its neighbors: `DateOps`, `METHODS`, associated `register(env)` that builds the object at the `add_global` line. `#[derive(Debug)]` **stays**: `minijinja::Object: fmt::Debug + Send + Sync`, so the derive is load-bearing here (`StrOps`/`NumOps` omit it only because they register filters and never implement `Object`). Ordering: this task follows Task 2 (O3), which already rewrote `get_value`, so the rename touches no live design edits. Churn: 11 source references (9 in `date.rs` including the module-doc link and four test sites, 2 in `engine.rs`) plus the baseline. This is behavior-equivalent by construction: the existing `get_value` and `enumerate` tests are the pin (green before and after). GLOSSARY: no edit; `DateOps` is an internal type name, and the `date` namespace itself is unchanged.

- [ ] **Step 1: Rename the type, the const, and the call style**

  In `src/template/engine/date.rs`, replace every `NAMESPACE_METHODS` with `METHODS` (4 occurrences: the const, `enumerate`, and the two enumerate tests).

  In `src/template/engine/date.rs` and `src/template/engine.rs`, replace every `DateNamespace` with `DateOps` at these sites:
  - the module-doc link
  - the struct definition
  - `impl DateOps` and `impl Object for DateOps`
  - the `env()` test helper
  - four `Arc::new(...)` sites
  - the `engine.rs` import
  - the `engine.rs` registration call

  Then fix the two registration call sites that the rename left in method-call form, to match the `StrOps::register(&mut env)` style:

  ```rust
  DateOps.register(&mut env);
  ```

  becomes (once in `engine.rs`, once in the `env()` helper in `date.rs`):

  ```rust
  DateOps::register(&mut env);
  ```

- [ ] **Step 2: Make `register` stateless**

  In `src/template/engine/date.rs`, replace the signature, doc, and `add_global` line:

  ```rust
      /// Registers the `date` global plus every flat `date_*` filter and `is_*`
      /// test this module owns.
      ///
      /// Filters/tests are added first because they are zero-capture free
      /// functions and need no `self`; then `self` is consumed registering the
      /// `date` namespace object last.
      #[inline]
      pub(super) fn register(self, env: &mut Environment<'static>) {
  ```

  with:

  ```rust
      /// Registers the `date` global plus every flat `date_*` filter and `is_*`
      /// test this module owns.
      ///
      /// This is an associated function because [`DateOps`] carries no state:
      /// filters and tests are added first as free functions, then the `date`
      /// namespace object is registered last.
      #[inline]
      pub(super) fn register(env: &mut Environment<'static>) {
  ```

  and the last line of the body:

  ```rust
        env.add_global("date", Value::from_object(self));
  ```

  becomes:

  ```rust
        env.add_global("date", Value::from_object(DateOps));
  ```

  Nothing else in the body changes. Keep `#[derive(Debug)]` above the struct: `Object` needs `Debug`.

- [ ] **Step 3: Regenerate the pub-token baseline**

  Run:

  ```
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  ```

  Expected: exactly two rewritten lines, no additions or removals:

  ```
  -src/template/engine/date.rs: pub(super) fn register(self, env: &mut Environment<'static>) {
  +src/template/engine/date.rs: pub(super) fn register(env: &mut Environment<'static>) {
  -src/template/engine/date.rs: pub(super) struct DateNamespace;
  +src/template/engine/date.rs: pub(super) struct DateOps;
  ```

  Both `cargo public-api` baselines stay byte-identical: everything is `pub(super)`.

- [ ] **Step 4: Run the tests**

  Run: `mise run test:unit date`
  Expected: PASS (the `get_value` and `enumerate` test mods compile against `DateOps` and `METHODS`, and `enumerate_lists_every_method` still matches `Enumerator::Str(METHODS)` structurally).

- [ ] **Step 5: Run the full gate**

  Run: `mise run verify`
  Expected: PASS (fmt, check including `check:visibility`, lint, full test suite).

- [ ] **Step 6: Commit**

  ```
  git add src/template/engine/date.rs src/template/engine.rs tests/baselines/pub-tokens.txt
  git commit -m "refactor(template): rename DateNamespace to DateOps with stateless register"
  ```

---

### Task 10 (X1a): Render canonical patterns with the sign outside the year width

**Files:**
- Create: `src/temporal/fmt.rs`
- Modify: `src/temporal/mod.rs` (register `fmt`, re-export the renderer), `src/temporal/datetime.rs` (imports, `Display`, `Serialize`, tests), `src/temporal/date.rs` (widen the proptest year range; test only)
- Baseline: regen `tests/baselines/pub-tokens.txt` (+2); both `cargo public-api` snapshots stay byte-identical.
- No doc or GLOSSARY changes.

**Context:** The crate holds two year conventions side by side. `DateValue`'s `Display` (date.rs:131-154) writes the sign outside the four-digit width (`-0009-01-01`), and jiff's `%F` does the same (printer.rs:454-468 writes `-` itself, then pads the absolute year to four digits). Jiff's `%Y` counts the sign inside the width (`fmt_year` returns `ItemInteger::new(b'0', 4, year)`, printer.rs:802, so year -9 prints as `-009`). `DateTimeValue`'s `Display` (:301-305) and `Serialize` (:365-369) both render `canonical_pattern()` through `%Y`, so a negative-year value displays `-009-01-01T00:00:00`, which the crate's four-digit-year rule rejects on re-parse (date.rs:524 pins that rejection): `parse(Display(x))` loses the value, and the date-half disagrees with `DateValue` for the same instant. The task adds `src/temporal/fmt.rs`: a seven-entry table (`DEFAULT_DATE_PATTERN`, `"%Y-%m"`, the four `canonical_pattern()` datetime shapes, `"%F"`) maps each canonical pattern to a form, and `render_canonical_pattern(civil_dt, pattern)` writes the year itself (sign outside, mirroring date.rs:136-140) and lets jiff render only yearless tails (`%m-%d`, `%H:%M:%S` plus an optional fraction), so no `%Y` sign convention can leak in. A non-canonical pattern returns `None`; `Display` and `Serialize` keep today's jiff path on `None`, and `canonical_pattern_is_in_the_render_table` proves that arm unreachable for canonical patterns. Positive-year output is byte-identical before and after (sign-outside equals `%Y` for year >= 0), so every existing rendering test stays green; Task 11's engine half routes `format_strtime` through the same renderer. The asymmetry pin records why the renderer must lead: jiff's strtime parser accepts sign+3 digit years (`-009`) while the crate's own parser rejects them, so parse-side leniency can never justify display-side leniency. Visibility: +2 pub tokens, both `pub(crate)`. GLOSSARY: **no edit**; `render_canonical_pattern` and `CanonicalForm` are internal implementation names, not domain vocabulary.

- [ ] **Step 1: Add the red negative-year tests**

  Add two tests to datetime.rs's test module, beside the existing serde round-trip tests. The same scope already imports `ZoneGuard`, `DateTimeValue`, and `serde_json`.

  ```rust
        #[test]
        fn displays_negative_years_with_the_sign_outside_the_width() {
            // Arrange
            ZoneGuard::set("UTC");
            let value = DateTimeValue::parse_iso("-0009-01-01T00:00:00Z")
                .expect("four-digit negative year parses");

            // Act
            let rendered = value.to_string();

            // Assert
            assert_eq!(rendered, "-0009-01-01T00:00:00");
        }

        #[test]
        fn serializes_negative_years_with_the_sign_outside_the_width() {
            // Arrange
            let value = DateTimeValue::parse_iso("-0009-01-01T00:00:00Z")
                .expect("four-digit negative year parses");

            // Act
            let json =
                serde_json::to_string(&value).expect("serialize succeeds");

            // Assert
            assert_eq!(json, "\"-0009-01-01T00:00:00Z\"");
        }
  ```

  Add the full-range property inside `mod proptests` (:1842), beside the existing round-trip proptest. The strategy sweeps seconds across jiff's whole supported range instead of building year/month/day dates: local midnight near the range edge can fall below `Timestamp::MIN`, and that arrange step must never panic.

  ```rust
        proptest! {
            #[test]
            fn display_reparses_for_every_supported_year(
                second in Timestamp::MIN.as_second()..=Timestamp::MAX.as_second(),
            ) {
                // Arrange
                ZoneGuard::set("UTC");
                let value = DateTimeValue::from_unix_seconds(second)
                    .expect("generated second lies inside the timestamp range");

                // Act
                let displayed = value.to_string();
                let parsed = DateTimeValue::parse_iso(&displayed);

                // Assert
                prop_assert_eq!(parsed, Ok(value));
            }
        }
  ```

  In `src/temporal/date.rs`'s `mod proptests`, widen `iso_display_round_trips_supported_dates`'s year strategy. Change `year in 0_i32..=9999` to `year in -9999_i32..=9999` (X1a: "extend proptests to `-9999..=9999`"). This is a **pin (green before and after)**: `DateValue`'s hand-rolled `Display` already writes the sign outside the width.

  Run: `mise run test:unit iso_display_round_trips_supported_dates`
  Expected: PASS green on the unmodified tree; it pins the date-side encoder X1a builds on.

  Run: `mise run test:unit negative_years_with_the_sign`
  Expected: FAIL: jiff's `%Y` pads the sign into the width, so both tests see `-009-01-01T00:00:00` and `"-009-01-01T00:00:00Z"`. This is the intended red.

  Run: `mise run test:unit display_reparses_for_every_supported_year`
  Expected: FAIL: the property stops on a negative-year case where `parse_iso` rejects the three-digit-year spelling.

- [ ] **Step 2: Create `src/temporal/fmt.rs`**

  The table keys use the existing `DEFAULT_*` constants, so the pattern list cannot drift from the constants. `render_year` mirrors `DateValue`'s sign handling at date.rs:136-140.

  ```rust
  //! Renders canonical date-time patterns with one year convention.
  //!
  //! The sign always sits outside the four-digit year width (`-0009-01-01`),
  //! matching `DateValue`'s display and jiff's own `%F`. Jiff's `%Y` counts
  //! the sign inside the width (`-009-01-01`), which the four-digit parse
  //! rule rejects, so canonical output renders through
  //! [`render_canonical_pattern`] instead of `%Y`.

  use jiff::civil;

  use super::{DEFAULT_DATE_PATTERN, DEFAULT_DATETIME_PATTERN};

  /// One canonical output shape and the yearless pattern for each of its
  /// halves.
  enum CanonicalForm {
      /// `YYYY-MM-DD`: the date half only.
      Date,
      /// `YYYY-MM`: reduced year-month precision.
      Month,
      /// A full date-time; the payload renders the time half after `T`.
      /// Time carries no year, so it cannot reintroduce a sign convention.
      DateTime(&'static str),
  }

  /// Every pattern the crate renders canonically, paired with its form.
  ///
  /// Entry one is the date pattern, entry two is month precision, the next
  /// four are `DateTimeValue::canonical_pattern`'s shapes, and `"%F"` is the
  /// user-facing ISO date.
  static CANONICAL_PATTERNS: &[(&str, CanonicalForm)] = &[
      (DEFAULT_DATE_PATTERN, CanonicalForm::Date),
      ("%Y-%m", CanonicalForm::Month),
      (
          DEFAULT_DATETIME_PATTERN,
          CanonicalForm::DateTime("%H:%M:%S"),
      ),
      (
          "%Y-%m-%dT%H:%M:%S%.3f",
          CanonicalForm::DateTime("%H:%M:%S%.3f"),
      ),
      (
          "%Y-%m-%dT%H:%M:%S%.6f",
          CanonicalForm::DateTime("%H:%M:%S%.6f"),
      ),
      (
          "%Y-%m-%dT%H:%M:%S%.9f",
          CanonicalForm::DateTime("%H:%M:%S%.9f"),
      ),
      ("%F", CanonicalForm::Date),
  ];

  /// Renders `pattern` over `datetime` with the sign outside the year width.
  ///
  /// # Arguments
  ///
  /// * `datetime` - civil fields to render
  /// * `pattern` - a pattern from [`CANONICAL_PATTERNS`]
  ///
  /// # Returns
  ///
  /// The rendered spelling, or `None` when `pattern` is not canonical.
  /// Callers keep their jiff-native path for non-canonical patterns.
  pub(crate) fn render_canonical_pattern(
      datetime: civil::DateTime,
      pattern: &str,
  ) -> Option<String> {
      let (_, form) = CANONICAL_PATTERNS
          .iter()
          .find(|(canonical, _)| *canonical == pattern)?;
      let date_tail = match form {
          CanonicalForm::Month => "%m",
          CanonicalForm::Date | CanonicalForm::DateTime(_) => "%m-%d",
      };
      let mut rendered = render_year(datetime.year());
      rendered.push('-');
      rendered.push_str(&jiff::fmt::strtime::format(date_tail, datetime).ok()?);
      if let CanonicalForm::DateTime(time_pattern) = form {
          rendered.push('T');
          rendered.push_str(
              &jiff::fmt::strtime::format(time_pattern, datetime).ok()?,
          );
      }
      Some(rendered)
  }

  /// Writes the year with the sign outside its four-digit width.
  fn render_year(year: i32) -> String {
      if year < 0 {
          format!("-{:04}", year.unsigned_abs())
      } else {
          format!("{year:04}")
      }
  }
  ```

- [ ] **Step 3: Register the module and re-export the renderer**

  In `src/temporal/mod.rs`, insert `mod fmt;` after `mod error;` (line 36):

  ```rust
  mod error;
  mod lexer;
  ```

  to:

  ```rust
  mod error;
  mod fmt;
  mod lexer;
  ```

  and add the re-export after the `error` re-exports (lines 51-52):

  ```rust
  pub(crate) use error::CalendarError;
  pub use error::{DateError, DurationError};
  ```

  to:

  ```rust
  pub(crate) use error::CalendarError;
  pub use error::{DateError, DurationError};
  pub(crate) use fmt::render_canonical_pattern;
  ```

- [ ] **Step 4: Compose `Display` and `Serialize` from the renderer**

  First add the import to datetime.rs (lines 18-24):

  ```rust
  use super::{
      CalendarError, DateValue, TimeUnit,
      date::has_padded_date_prefix,
      error::DateError,
      lexer::{TemporalShapeToken, starts_with_year},
      zone::local_zone,
  };
  ```

  to:

  ```rust
  use super::{
      CalendarError, DateValue, TimeUnit,
      date::has_padded_date_prefix,
      error::DateError,
      lexer::{TemporalShapeToken, starts_with_year},
      render_canonical_pattern,
      zone::local_zone,
  };
  ```

  Then replace the `Display` impl (the doc comment above it stays):

  ```rust
  impl fmt::Display for DateTimeValue {
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
          let local = self.local_datetime().map_err(|_| fmt::Error)?;
          let rendered =
              jiff::fmt::strtime::format(self.canonical_pattern(), local)
                  .map_err(|_| fmt::Error)?;
          f.write_str(&rendered)
      }
  }
  ```

  to:

  ```rust
  impl fmt::Display for DateTimeValue {
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
          let local = self.local_datetime().map_err(|_| fmt::Error)?;
          match render_canonical_pattern(local, self.canonical_pattern()) {
              Some(rendered) => f.write_str(&rendered),
              None => {
                  let rendered = jiff::fmt::strtime::format(
                      self.canonical_pattern(),
                      local,
                  )
                  .map_err(|_| fmt::Error)?;
                  f.write_str(&rendered)
              }
          }
      }
  }
  ```

  And replace the `Serialize` impl (the doc comment above it stays):

  ```rust
  impl Serialize for DateTimeValue {
      #[inline]
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where
          S: Serializer,
      {
          let mut rendered = jiff::fmt::strtime::format(
              self.canonical_pattern(),
              self.utc().to_zoned(TimeZone::UTC).datetime(),
          )
          .map_err(serde::ser::Error::custom)?;
          rendered.push('Z');
          serializer.serialize_str(&rendered)
      }
  }
  ```

  to:

  ```rust
  impl Serialize for DateTimeValue {
      #[inline]
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where
          S: Serializer,
      {
          let utc = self.utc().to_zoned(TimeZone::UTC).datetime();
          let mut rendered =
              match render_canonical_pattern(utc, self.canonical_pattern()) {
                  Some(rendered) => rendered,
                  None => jiff::fmt::strtime::format(
                      self.canonical_pattern(),
                      utc,
                  )
                  .map_err(serde::ser::Error::custom)?,
              };
          rendered.push('Z');
          serializer.serialize_str(&rendered)
      }
  }
  ```

  The `None` arm preserves the old code path byte for byte; Step 5's membership test proves it unreachable for canonical patterns.

  Run: `mise run test:unit negative_years_with_the_sign` and `mise run test:unit display_reparses_for_every_supported_year`
  Expected: PASS: both tests and the property now round-trip, because the table writes `-0009` where jiff's `%Y` wrote `-009`.

- [ ] **Step 5: Add the table, year, and asymmetry tests**

  Append this test module at the end of `src/temporal/fmt.rs`:

  ```rust
  #[cfg(test)]
  mod tests {
      use proptest::prelude::*;
      use rstest::rstest;

      use super::*;
      use super::super::{DatePoint, DateValue};

      #[rstest]
      #[case::year_zero(0, "0000")]
      #[case::two_digits(25, "0025")]
      #[case::negative_two_digits(-25, "-0025")]
      #[case::negative_single(-9, "-0009")]
      #[case::negative_max(-9999, "-9999")]
      fn writes_the_sign_outside_the_year_width(
          #[case] year: i32,
          #[case] expected: &str,
      ) {
          assert_eq!(render_year(year), expected);
      }

      #[test]
      fn renders_every_canonical_pattern_and_rejects_the_rest() {
          // Arrange
          let datetime: civil::DateTime = "2026-07-05T04:05:03.123456789"
              .parse()
              .expect("fixture datetime parses");

          // Act + Assert: the canonical ladder renders.
          for pattern in [
              DEFAULT_DATE_PATTERN,
              "%Y-%m",
              DEFAULT_DATETIME_PATTERN,
              "%Y-%m-%dT%H:%M:%S%.3f",
              "%Y-%m-%dT%H:%M:%S%.6f",
              "%Y-%m-%dT%H:%M:%S%.9f",
              "%F",
          ] {
              assert!(
                  render_canonical_pattern(datetime, pattern).is_some(),
                  "{pattern} must be canonical"
              );
          }

          // Non-canonical patterns fall through to the caller's jiff path.
          assert!(render_canonical_pattern(datetime, "%d %B %Y").is_none());
          assert!(render_canonical_pattern(datetime, "%F%.3f").is_none());
      }

      #[test]
      fn renders_negative_years_in_every_canonical_form() {
          // Arrange: a negative year with time attached.
          let datetime: civil::DateTime = "-0009-01-05T04:05:03"
              .parse()
              .expect("fixture datetime parses");

          // Act + Assert
          assert_eq!(
              render_canonical_pattern(datetime, DEFAULT_DATE_PATTERN),
              Some("-0009-01-05".to_string()),
          );
          assert_eq!(
              render_canonical_pattern(datetime, "%Y-%m"),
              Some("-0009-01".to_string()),
          );
          assert_eq!(
              render_canonical_pattern(datetime, DEFAULT_DATETIME_PATTERN),
              Some("-0009-01-05T04:05:03".to_string()),
          );
          assert_eq!(
              render_canonical_pattern(datetime, "%F"),
              Some("-0009-01-05".to_string()),
          );
      }

      #[test]
      fn date_values_keep_the_same_year_spelling() {
          // Arrange: DateValue's Display predates the table and must stay
          // byte-identical to it.
          let date =
              DateValue::from_ymd(-9, 1, 5).expect("fixture date is valid");

          // Act
          let rendered = render_canonical_pattern(
              date.civil_date().at(4, 5, 3, 0),
              DEFAULT_DATE_PATTERN,
          );

          // Assert
          assert_eq!(rendered.as_deref(), Some(date.to_string().as_str()));
      }

      #[test]
      fn the_parser_accepts_wider_year_forms_than_the_renderer_emits() {
          // Arrange: jiff's strtime parser reads a sign plus up to four
          // digits; the crate's four-digit-year rule is stricter.

          // Act
          let strtime_accepts =
              DatePoint::parse_with_pattern("-009-01-01", "%Y-%m-%d").is_ok();
          let crate_rejects =
              "-009-01-01".parse::<DateValue>().is_err();

          // Assert: the renderer never emits the lenient form.
          assert!(strtime_accepts, "strtime parse reads sign+3 digits");
          assert!(crate_rejects, "the four-digit rule rejects sign+3 digits");
          let datetime: civil::DateTime = "-0009-01-01T00:00:00"
              .parse()
              .expect("fixture datetime parses");
          assert_eq!(
              render_canonical_pattern(datetime, DEFAULT_DATE_PATTERN)
                  .as_deref(),
              Some("-0009-01-01"),
          );
      }

      proptest! {
          #[test]
          fn percent_f_matches_the_canonical_date_spelling(
              year in -9999i32..=9999,
              month in 1u32..=12,
              day in 1u32..=28,
          ) {
              // Arrange: jiff's `%F` writes the sign outside the year width
              // itself (printer.rs:454-468), so both spellings must agree
              // for every representable year.
              let date = DateValue::from_ymd(year, month, day)
                  .expect("generated fields form a valid date");
              let datetime = date.civil_date().at(0, 0, 0, 0);

              // Act
              let ours = render_canonical_pattern(datetime, "%F")
                  .expect("%F is canonical");
              let theirs = jiff::fmt::strtime::format("%F", datetime)
                  .expect("%F renders");

              // Assert
              prop_assert_eq!(ours, theirs);
          }
      }
  }
  ```

  Add the membership test to the datetime.rs test module, beside the two Step 1 tests. It pins all four `canonical_pattern()` precision branches against the table:

  ```rust
        #[test]
        fn canonical_pattern_is_in_the_render_table() {
            // Arrange
            ZoneGuard::set("UTC");
            let fixture: jiff::civil::DateTime = "2026-07-05T04:05:03"
                .parse()
                .expect("fixture datetime parses");
            let values = [
                "2026-07-05T04:05:03Z",
                "2026-07-05T04:05:03.001Z",
                "2026-07-05T04:05:03.000001Z",
                "2026-07-05T04:05:03.000000001Z",
            ];

            // Act + Assert: each precision branch's pattern renders.
            for text in values {
                let value = DateTimeValue::parse_iso(text)
                    .expect("fixture value parses");
                assert!(
                    render_canonical_pattern(fixture, value.canonical_pattern())
                        .is_some(),
                    "{} must be canonical",
                    value.canonical_pattern()
                );
            }
        }
  ```

  Run: `mise run test:unit fmt` and `mise run test:unit canonical_pattern_is_in_the_render_table`
  Expected: PASS: six fmt tests (the rstest contributes five cases) plus the membership test.

- [ ] **Step 6: Regenerate the pub-token baseline (+2 lines)**

  Run:

  ```bash
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  git diff tests/baselines/pub-tokens.txt
  ```

  Expected: exactly two added lines, `src/temporal/fmt.rs: pub(crate) fn render_canonical_pattern(datetime: civil::DateTime, pattern: &str) -> Option<String> {` and `src/temporal/mod.rs: pub(crate) use fmt::render_canonical_pattern;`; no other lines change. A wider diff means a visibility mistake (for example `CanonicalForm` made `pub(crate)`): fix the source instead of accepting the snapshot.

- [ ] **Step 7: Run the full gate**

  Run: `mise run verify`
  Expected: green (`fmt`, `check` incl. `check:visibility`: both public-api diffs empty because every new item is `pub(crate)`, pub-token diff empty after the regen, `lint --no-fix`, `test`).

- [ ] **Step 8: Commit**

  ```bash
  git add src/temporal/fmt.rs src/temporal/mod.rs src/temporal/datetime.rs src/temporal/date.rs tests/baselines/pub-tokens.txt
  git commit -m "fix(temporal): render negative years sign-outside-width"
  ```

---

### Task 11 (X1b + X6): Route strftime rendering through the canonical renderer and align zone-dependent day tokens

**Files:**
- Modify: `src/template/engine/date.rs` (`format_strtime`, the three mixed-dialect sites, tests in `mod now` and `mod date_format`)
- Baseline: regen `tests/baselines/pub-tokens.txt`; expected no diff (every touched item stays private). Both `cargo public-api` snapshots stay byte-identical.
- No doc or GLOSSARY changes.

**Context:** Half one (X1b): `format_strtime` (date.rs:424-437) is the engine's chokepoint, all six rendering sites funnel through it, and today it hands every pattern straight to `jiff::fmt::strtime::format`. Task 10 built `temporal::render_canonical_pattern`, which writes the sign outside the year width for canonical shapes, but templates never reach it: `{{ "-0009-01-01" | date_format("%Y-%m-%d") }}` still renders `-009-01-01`, the spelling `DateValue`'s four-digit rule rejects on reparse, so rendering and the crate parser disagree for the same value. The fix is one guard at the chokepoint: consult `render_canonical_pattern(datetime.datetime(), strftime)` first and keep the jiff path for `None`. Positive years are byte-identical (the table renders them exactly as jiff does), non-canonical patterns still fall through, and a bad pattern still raises `invalid date format` after the miss. Half two (X6): moment-dialect translation bakes datetime-dependent text into the strftime string; `Do` is written as a literal ordinal from the `civil::DateTime` it is handed (date.rs:576-580; Task 6 moves it into `token.rs`), while `%d` renders from the `zoned` value at format time. The three sites that mix both dialects (`Anchor::render` :256-263, `date_format` :471-479, `weekday` :934-947) hand the translator `local_datetime_in(...)`, the raw civil clock, and resolve the zoned instant only afterwards. On a skipped local day the two clocks differ: Samoa's 2011-12-30 never existed (the 2011-12-29 23:59:59 jump skips a whole day), `zoned_datetime_in` lands on 2011-12-31 while `local_datetime_in` stays on Dec 30, so `date_format("Do")` renders `30th` beside `date_format("%d")` rendering `31`. The fix reorders all three sites: resolve the zoned value first, feed `zoned.datetime()` to the translator, then format, so one clock feeds both dialects. A pinned side effect: zone/instant resolution now runs before pattern translation, so `-9999-01-01` under `Etc/GMT-14` (local midnight before jiff's minimum instant) reports the range failure even when the pattern is also invalid; today the pattern error wins. `local_zone()` lookup already precedes the pattern at every site and stays first. Sites that pass only `canonical_pattern()` (`render_period_edge` :695-712, `shift_by_unit` :796-812, `from_timestamp` :357) hand the translator no datetime, so they need no reorder; the chokepoint guard still fixes their negative years. Visibility: none, every touched item stays private. GLOSSARY: no edit, no new domain vocabulary.

- [ ] **Step 1: Add the red negative-year engine tests**

  Add one test to `mod date_format` (date.rs:1426), beside `format_bindings_never_emit_percent_plus` at the end of that module:

  ```rust
        #[test]
        fn renders_a_negative_year_with_the_sign_outside_the_width() {
            // Arrange
            ZoneGuard::set("UTC");

            // Act
            let rendered = env()
                .render_str(
                    r#"{{ value | date_format("%Y-%m-%d") }}"#,
                    minijinja::context! { value => "-0009-01-01" },
                )
                .expect("render succeeds");

            // Assert
            assert_eq!(rendered, "-0009-01-01");
        }
  ```

  Add one test to `mod now` (date.rs:1176), beside `now_rejects_an_unknown_kwarg` at the end of that module:

  ```rust
        #[test]
        fn now_formats_a_negative_year_reference_sign_outside_the_width() {
            // Arrange
            ZoneGuard::set("UTC");

            // Act
            let rendered = env()
                .render_str(
                    r#"{{ date.now(reference="-0009-01-01", reference_format="%Y-%m-%d", format="%Y-%m-%d") }}"#,
                    minijinja::context!(),
                )
                .expect("render succeeds");

            // Assert
            assert_eq!(rendered, "-0009-01-01");
        }
  ```

  Run: `mise run test:unit renders_a_negative_year_with_the_sign`
  Expected: FAIL: jiff's `%Y` pads the sign into the width, so the filter renders `-009-01-01`. This is the intended red.

  Run: `mise run test:unit engine::date::tests::now`
  Expected: FAIL: the `Anchor::render` path shows the same sign padding. Intended red.

- [ ] **Step 2: Consult the canonical renderer inside `format_strtime`**

  In `src/template/engine/date.rs`, change:

  ```rust
  /// Formats a zoned date-time with Jiff's strict strftime implementation.
  ///
  /// # Errors
  ///
  /// Returns a [`minijinja::Error`] with kind
  /// [`ErrorKind::InvalidOperation`](minijinja::ErrorKind::InvalidOperation) if
  /// `strftime` is not a strftime specifier `datetime` can render.
  fn format_strtime(
      datetime: &jiff::Zoned,
      strftime: &str,
      source_pattern: &str,
  ) -> TemplateEngineResult<String> {
      jiff::fmt::strtime::format(strftime, datetime).map_err(|source| {
          invalid_operation(
              format!("invalid date format {source_pattern:?}"),
              source,
          )
      })
  }
  ```

  to:

  ```rust
  /// Formats a zoned date-time, preferring the canonical temporal renderer for
  /// the pattern shapes the crate renders itself.
  ///
  /// # Errors
  ///
  /// Returns a [`minijinja::Error`] with kind
  /// [`ErrorKind::InvalidOperation`](minijinja::ErrorKind::InvalidOperation) if
  /// `strftime` is neither a canonical pattern nor a strftime specifier
  /// `datetime` can render.
  fn format_strtime(
      datetime: &jiff::Zoned,
      strftime: &str,
      source_pattern: &str,
  ) -> TemplateEngineResult<String> {
      if let Some(rendered) = crate::temporal::render_canonical_pattern(
          datetime.datetime(),
          strftime,
      ) {
          return Ok(rendered);
      }
      jiff::fmt::strtime::format(strftime, datetime).map_err(|source| {
          invalid_operation(
              format!("invalid date format {source_pattern:?}"),
              source,
          )
      })
  }
  ```

  The guard reads `datetime.datetime()` (the civil half of the zoned value), so every site's canonical output comes from one renderer. The `source_pattern` in the error message stays the caller's original pattern, not the canonical translation.

  Run: `mise run test:unit renders_a_negative_year_with_the_sign`
  Expected: PASS, `-0009-01-01`.

  Run: `mise run test:unit engine::date::tests::now`
  Expected: PASS, `-0009-01-01`.

  Run: `mise run test:unit engine::date`
  Expected: PASS in full, no regression: positive years render byte-identically through the table.

- [ ] **Step 3: Add the red zone-alignment tests**

  Add two tests to `mod date_format`, beside the test from Step 1:

  ```rust
        #[test]
        fn skipped_local_days_render_one_day_in_both_dialects() {
            // Arrange: 2011-12-30 did not exist in Samoa; midnight shifts to
            // 2011-12-31, and both dialects must render that day.
            ZoneGuard::set("Pacific/Apia");

            // Act
            let rendered = env()
                .render_str(
                    r#"{{ value | date_format("Do") }}|{{ value | date_format("%d") }}"#,
                    minijinja::context! { value => "2011-12-30" },
                )
                .expect("render succeeds");

            // Assert
            assert_eq!(rendered, "31st|31");
        }

        #[test]
        fn zone_resolution_errors_precede_pattern_errors() {
            // Arrange: local midnight sits before jiff's minimum instant, and
            // the pattern is also invalid, so both layers could fail.
            ZoneGuard::set("Etc/GMT-14");

            // Act
            let error = env()
                .render_str(
                    r#"{{ value | date_format("Q") }}"#,
                    minijinja::context! { value => "-9999-01-01" },
                )
                .expect_err("local midnight falls outside the range");

            // Assert
            let msg = error.to_string();
            assert!(
                !msg.contains("moment-dialect"),
                "the pattern error must lose to the range error: {msg}"
            );
            assert!(msg.contains("range"), "expected a range error: {msg}");
        }
  ```

  Run: `mise run test:unit skipped_local_days_render_one_day`
  Expected: FAIL: `Do` bakes the day from `local_datetime_in` (Dec 30 gives `30th`) while `%d` renders the shifted `zoned` day, so the template renders `30th|31`. This is the intended red.

  Run: `mise run test:unit zone_resolution_errors_precede`
  Expected: FAIL: `date_format` translates the pattern before resolving the point's instant, so the message names `unsupported moment-dialect token`. This is the intended red.

- [ ] **Step 4: Resolve the zoned value first at all three mixed-dialect sites**

  In `src/template/engine/date.rs`, change the tail of `Anchor::render`:

  ```rust
          let zone = crate::temporal::local_zone().map_err(render_date_error)?;
          let local_datetime = date_point.local_datetime_in(&zone);
          let pat = pattern_to_strftime(format, local_datetime)?;
          let zoned = date_point
              .zoned_datetime_in(&zone)
              .map_err(render_calendar_error)?;
          format_strtime(&zoned, &pat, format)
  ```

  to:

  ```rust
          let zone = crate::temporal::local_zone().map_err(render_date_error)?;
          let zoned = date_point
              .zoned_datetime_in(&zone)
              .map_err(render_calendar_error)?;
          let local_datetime = zoned.datetime();
          let pat = pattern_to_strftime(format, local_datetime)?;
          format_strtime(&zoned, &pat, format)
  ```

  In the same file, change `date_format`:

  ```rust
  fn date_format(value: &str, pattern: &str) -> TemplateEngineResult<String> {
      let point = parse_date_point(value)?;
      let zone = crate::temporal::local_zone().map_err(render_date_error)?;
      let datetime = point.local_datetime_in(&zone);
      let translated = pattern_to_strftime(pattern, datetime)?;
      let zoned =
          point.zoned_datetime_in(&zone).map_err(render_calendar_error)?;
      format_strtime(&zoned, &translated, pattern)
  }
  ```

  to:

  ```rust
  fn date_format(value: &str, pattern: &str) -> TemplateEngineResult<String> {
      let point = parse_date_point(value)?;
      let zone = crate::temporal::local_zone().map_err(render_date_error)?;
      let zoned =
          point.zoned_datetime_in(&zone).map_err(render_calendar_error)?;
      let datetime = zoned.datetime();
      let translated = pattern_to_strftime(pattern, datetime)?;
      format_strtime(&zoned, &translated, pattern)
  }
  ```

  In the same file, change the target branch of `weekday`:

  ```rust
          let shifted = date_point
              .with_weekday(target_weekday, &zone)
              .map_err(render_calendar_error)?;
          let local_datetime = shifted.local_datetime_in(&zone);
          let pat = pattern_to_strftime(format, local_datetime)?;
          let zoned =
              shifted.zoned_datetime_in(&zone).map_err(render_calendar_error)?;
          Ok(Value::from(format_strtime(&zoned, &pat, format)?))
  ```

  to:

  ```rust
          let shifted = date_point
              .with_weekday(target_weekday, &zone)
              .map_err(render_calendar_error)?;
          let zoned = shifted
              .zoned_datetime_in(&zone)
              .map_err(render_calendar_error)?;
          let local_datetime = zoned.datetime();
          let pat = pattern_to_strftime(format, local_datetime)?;
          Ok(Value::from(format_strtime(&zoned, &pat, format)?))
  ```

  Each site keeps its existing error mapping and local names; only the order changes. `render_period_edge`, `shift_by_unit`, and `from_timestamp` pass `canonical_pattern()` or a caller pattern with no translator, so they stay untouched.

  Run: `mise run test:unit skipped_local_days_render_one_day`
  Expected: PASS, `31st|31`.

  Run: `mise run test:unit zone_resolution_errors_precede`
  Expected: PASS, the message is `date arithmetic overflowed the supported range`.

  Run: `mise run test:unit engine::date`
  Expected: PASS in full.

- [ ] **Step 5: Regenerate the public-token baseline**

  ```bash
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  git diff --stat tests/baselines/pub-tokens.txt
  ```

  Expected: no diff. Every item this task touches (`format_strtime`, `date_format`, `weekday`, `Anchor::render`, the new tests) is private.

- [ ] **Step 6: Run the full gate**

  Run: `mise run verify`
  Expected: green (fmt, check with zero warnings, clippy with zero warnings, full test suite).

- [ ] **Step 7: Commit**

  ```bash
  git add src/template/engine/date.rs tests/baselines/pub-tokens.txt
  git commit -m "fix(template): canonical year rendering and zone-aligned day tokens"
  ```

---

### Task 12 (X8b): Make `CalendarError::Zone` carry the `DateError` instead of its text

**Files:**
- Modify: `src/temporal/error.rs` (`Zone` variant, `into_date_error`, `LocalZoneLookup` docs, tests), `src/temporal/datetime.rs` (`from_local`), `src/temporal/point.rs` (`local_zone_for_calendar`), `src/template/engine/date.rs` (`render_calendar_error`, new test)
- Baseline: regen `tests/baselines/pub-tokens.txt`; expected no diff. The `cargo public-api` snapshots stay untouched because `CalendarError` is `pub(crate)`.
- No GLOSSARY change.

**Context:** One failed time-zone lookup gets diagnosed four times. `local_zone()` fails as `DateError::LocalZoneLookup` whose `input` is the zone-discovery detail (`system zone: {detail}`, zone.rs:31-35). Both `CalendarError::Zone` constructors then stringify that Display into a `Box<str>` (`point.rs:780` inside `local_zone_for_calendar`, `datetime.rs:128` inside `from_local`). The variant's thiserror attribute prepends its own label (`local zone lookup failed for {input}`, error.rs:145), so `CalendarError::Zone`'s Display reads `local zone lookup failed for local zone lookup failed for \`system zone: ...\``. The engine adds a third layer for templates (`local timezone lookup failed for {input:?}`, date.rs:1105-1111), and `into_date_error` (error.rs:155-167) re-wraps the whole stacked text as a fresh `LocalZoneLookup.input` for typed parsers, producing a triple stack through one `?`. The docs lie about the payload while this happens: the `Zone` field doc says it stores "the local clock time that could not be resolved", and the `LocalZoneLookup` docs (error.rs:54-60) say `input` is a naive clock-time input, while production puts zone-discovery detail there. The fix stores the error instead of a copy of its words: `Zone { source: DateError }` behind `#[error("{source}")]` with `#[source]`, so Display renders the DateError's own text (one diagnosis), `source()` reports it (inspectable chain), both constructors pass `local_zone()`'s error through unchanged, `into_date_error` returns the stored source directly, and the engine renders `source.to_string()`. `filter.rs:742` matches `CalendarError::Zone { .. }` and needs no edit; `point.rs:199-201` (`DatePoint::date`) only propagates the helper's error and needs no edit either. The exact top-level template wording is still the engine's, not temporal's; Task 13 collapses both renderers onto temporal `Display`. Every test this task rewrites pins the single-prefix rendering, so the double-prefix bug cannot return. Visibility: none, `pub(crate) enum CalendarError` and its field lines carry no `pub` token.

- [ ] **Step 1: Add the red double-prefix test**

  Add one test to `mod error_translation` (date.rs:2408), beside `preserves_the_local_zone_lookup_diagnosis`. It constructs `Zone` exactly the way the production constructors do today, by storing the DateError's Display text:

  ```rust
        #[test]
        fn renders_a_zone_failure_with_one_lookup_prefix() {
            // Arrange: mirror today's constructor, which stores DateError's
            // Display text; the assertion is the single diagnosis we want.
            let inner = DateError::LocalZoneLookup {
                input: "system zone: clock unavailable".into(),
            };
            let error = render_calendar_error(CalendarError::Zone {
                input: inner.to_string().into(),
            });

            // Act
            let message = error.to_string();

            // Assert
            assert_eq!(
                message,
                "local zone lookup failed for `system zone: clock unavailable`"
            );
        }
  ```

  Run: `mise run test:unit renders_a_zone_failure_with_one_lookup_prefix`
  Expected: FAIL: the rendered message stacks three labels, `local timezone lookup failed for "local zone lookup failed for \`system zone: clock unavailable\`"`. This is the intended red.

- [ ] **Step 2: Store the `DateError` itself**

  In `src/temporal/error.rs`, change the `Zone` variant:

  ```rust
      /// Reports that local-zone resolution failed for a computed local clock
      /// time.
      #[error("local zone lookup failed for {input}")]
      Zone {
          /// Stores the local clock time that could not be resolved.
          input: Box<str>,
      },
  ```

  to:

  ```rust
      /// Reports that system time-zone discovery failed while resolving a
      /// computed local clock time.
      ///
      /// Forwards its text to the inner [`DateError`] and reports it as the
      /// error source, so the lookup failure carries one diagnosis instead
      /// of stacking two.
      #[error("{source}")]
      Zone {
          /// Stores the zone-lookup failure this variant reports.
          #[source]
          source: DateError,
      },
  ```

  In the same file, change the `Zone` arm of `into_date_error`:

  ```rust
              Self::Zone {
                  input: zone_input,
              } => DateError::LocalZoneLookup {
                  input: zone_input,
              },
  ```

  to:

  ```rust
              Self::Zone {
                  source,
              } => source,
  ```

  In `src/temporal/point.rs`, change `local_zone_for_calendar`:

  ```rust
  fn local_zone_for_calendar() -> Result<TimeZone, CalendarError> {
      local_zone().map_err(|error| CalendarError::Zone {
          input: error.to_string().into(),
      })
  }
  ```

  to:

  ```rust
  fn local_zone_for_calendar() -> Result<TimeZone, CalendarError> {
      local_zone().map_err(|source| CalendarError::Zone { source })
  }
  ```

  In `src/temporal/datetime.rs`, change `from_local`:

  ```rust
          let zone = local_zone().map_err(|error| CalendarError::Zone {
              input: error.to_string().into(),
          })?;
  ```

  to:

  ```rust
          let zone = local_zone().map_err(|source| CalendarError::Zone {
              source,
          })?;
  ```

  In `src/template/engine/date.rs`, change the `Zone` arm of `render_calendar_error`:

  ```rust
          CalendarError::Zone {
              input,
          } => invalid_operation(
              format!("local timezone lookup failed for {input:?}"),
              CalendarError::Zone {
                  input,
              },
          ),
  ```

  to:

  ```rust
          CalendarError::Zone {
              source,
          } => invalid_operation(
              source.to_string(),
              CalendarError::Zone {
                  source,
              },
          ),
  ```

  Flip the construction line in the Step 1 test to the new shape (its assertion does not move):

  ```rust
              let error = render_calendar_error(CalendarError::Zone {
                  source: inner,
              });
  ```

  Rewrite the two `error.rs` tests that construct the old shape. In `mod formatting`, change `renders_the_zone_calendar_error`:

  ```rust
        #[test]
        fn renders_the_zone_calendar_error() {
            // Arrange
            let error = CalendarError::Zone {
                input: "2026-03-08 02:30:00".into(),
            };
            let expected = "local zone lookup failed for 2026-03-08 02:30:00";
            // Act
            let rendered = error.to_string();
            // Assert
            assert_eq!(rendered, expected);
        }
  ```

  to:

  ```rust
        #[test]
        fn renders_the_zone_calendar_error() {
            // Arrange
            let error = CalendarError::Zone {
                source: DateError::LocalZoneLookup {
                    input: "system zone: clock unavailable".into(),
                },
            };
            let expected =
                "local zone lookup failed for `system zone: clock unavailable`";
            // Act
            let rendered = error.to_string();
            // Assert
            assert_eq!(rendered, expected);
        }
  ```

  In `mod conversions`, change `maps_zone_calendar_failure_to_local_zone_lookup_error`:

  ```rust
        #[test]
        fn maps_zone_calendar_failure_to_local_zone_lookup_error() {
            // Arrange
            let input = "2026-03-08 02:30:00";
            let error = CalendarError::Zone {
                input: input.into(),
            };
            // Act
            let actual = error.into_date_error(input);
            // Assert
            assert_eq!(actual, DateError::LocalZoneLookup {
                input: input.into(),
            });
        }
  ```

  to:

  ```rust
        #[test]
        fn maps_zone_calendar_failure_to_local_zone_lookup_error() {
            // Arrange
            let input = "2026-03-08 02:30:00";
            let error = CalendarError::Zone {
                source: DateError::LocalZoneLookup {
                    input: input.into(),
                },
            };
            // Act
            let actual = error.into_date_error(input);
            // Assert
            assert_eq!(actual, DateError::LocalZoneLookup {
                input: input.into(),
            });
        }
  ```

  Run: `mise run test:unit renders_a_zone_failure_with_one_lookup_prefix`
  Expected: PASS: the template message is the DateError's own text, `local zone lookup failed for \`system zone: clock unavailable\``.

  Run: `mise run test:unit error::tests`
  Expected: PASS: both rewritten tests hold the single-prefix rendering and the direct pass-through in `into_date_error`.

  Run: `mise run test:unit engine::date`
  Expected: PASS in full; `render_date_error`'s own `LocalZoneLookup` arm is untouched.

- [ ] **Step 3: Pin the exposed source**

  Add one test beside `renders_the_zone_calendar_error` in `mod formatting`:

  ```rust
        #[test]
        fn exposes_the_date_error_as_the_source() {
            // Arrange
            let error = CalendarError::Zone {
                source: DateError::LocalZoneLookup {
                    input: "system zone: clock unavailable".into(),
                },
            };

            // Act
            let source = std::error::Error::source(&error);

            // Assert
            assert!(
                source
                    .is_some_and(|s| s.downcast_ref::<DateError>().is_some()),
                "Zone must expose its DateError as the source"
            );
        }
  ```

  Run: `mise run test:unit exposes_the_date_error_as_the_source`
  Expected: PASS: `source()` returns the stored `DateError`.

- [ ] **Step 4: Correct the `LocalZoneLookup` docs**

  In `src/temporal/error.rs`, the variant and field docs currently claim the payload is a naive clock time, which production never stores. Change:

  ```rust
      /// Reports that the process-local zone could not resolve a naive input.
      ///
      /// DST ambiguities and gaps never produce this error; they resolve
      /// deterministically.
      #[error("local zone lookup failed for `{input}`")]
      LocalZoneLookup {
          /// Stores the naive clock-time input that could not be resolved.
          input: Box<str>,
      },
  ```

  to:

  ```rust
      /// Reports that system time-zone discovery failed.
      ///
      /// `input` carries the zone-discovery failure detail, not a clock time:
      /// the zone layer builds it as `system zone: {detail}`. DST ambiguities
      /// and gaps never produce this error; they resolve deterministically.
      #[error("local zone lookup failed for `{input}`")]
      LocalZoneLookup {
          /// Stores the zone-discovery failure detail.
          input: Box<str>,
      },
  ```

  The Display text does not move; it is part of the crate's contract (error.rs module doc).

- [ ] **Step 5: Regenerate the public-token baseline**

  ```bash
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  git diff --stat tests/baselines/pub-tokens.txt
  ```

  Expected: no diff. The enum declaration line is unchanged and variant fields carry no `pub` token.

- [ ] **Step 6: Run the full gate**

  Run: `mise run verify`
  Expected: green (fmt, check with zero warnings, clippy with zero warnings, full test suite).

- [ ] **Step 7: Commit**

  ```bash
  git add src/temporal/error.rs src/temporal/datetime.rs src/temporal/point.rs src/template/engine/date.rs tests/baselines/pub-tokens.txt
  git commit -m "fix(temporal): carry the zone DateError instead of its text"
  ```

---

### Task 13 (O7): Collapse both error renderers onto temporal `Display` with the origin rule

**Files:**
- Modify: `src/template/engine/date.rs` (`render_date_error`, `render_calendar_error`, `date_out_of_range_error` doc, `mod error_translation` tests)
- Baseline: regen `tests/baselines/pub-tokens.txt`; expected no diff (both renderers stay private). The `cargo public-api` snapshots stay untouched; temporal's Display text is unchanged, only the engine's substitutes are removed.
- Accepted behavior changes, each pinned below: template zone messages adopt `local zone lookup failed for \`...\``, template range messages adopt `date/time value is out of range`, and range errors gain a `Caused by:` line in miette because the `CalendarError` now rides in the source chain.
- No GLOSSARY change.

**Context:** The engine keeps a private vocabulary for temporal failures and it disagrees with the crate's own. `render_date_error` (date.rs:1082-1095) rewords `LocalZoneLookup` as `local timezone lookup failed for {input:?}`, and `render_calendar_error` maps `OutOfRange` to `date_out_of_range_error()` (`date arithmetic overflowed the supported range`, :1074-1080), so templates see messages that appear nowhere in temporal's Display contract (error.rs module doc makes each variant's Display text part of the crate's contract). The origin rule settles who speaks: failures that originate in temporal render temporal's Display through `invalid_operation(error.to_string(), error)`, and failures that originate in engine argument validation keep the engine's own text. The argument-plane sites are exactly `date_out_of_range_error()`'s six callers (:774, :824, :839, :840, :870, :900: `checked_neg` on `i64::MIN` and `u64` to `i64` conversions raised before any calendar arithmetic runs), and they are untouched, so two range messages coexist correctly on two different planes. After Task 12, `CalendarError::Zone`'s Display already equals the inner DateError's text, so collapsing its arm is behavior-neutral there; the changes land on `render_date_error`'s `LocalZoneLookup` arm (wording) and `render_calendar_error`'s `OutOfRange` arm (wording plus a source chain). Breakage inventory: one zone string (the `render_date_error` arm, test :2418), one range string (`:1104`, test :2438); tests :2455 and :2480 assert the range string is absent from parse failures and stay green, and Task 12's `renders_a_zone_failure_with_one_lookup_prefix` stays green because the forwarded Display yields the same bytes. Only `src/template/engine/date.rs` references these renderers; no other crate or golden output quotes either string. Visibility: none.

- [ ] **Step 1: Rewrite the two wording tests (red)**

  In `mod error_translation` (date.rs:2408), change the message assertion inside `preserves_the_local_zone_lookup_diagnosis` (keep the kind and source-chain assertions):

  ```rust
            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
            assert!(
                error.to_string().contains("local timezone lookup failed"),
                "expected the zone diagnosis, got {error}"
            );
  ```

  to:

  ```rust
            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
            let message = error.to_string();
            assert_eq!(
                message.matches("local zone lookup failed").count(),
                1,
                "one zone diagnosis without a doubled prefix, got {message}"
            );
  ```

  Replace `keeps_the_range_message_for_an_overflow` in the same module:

  ```rust
        #[test]
        fn keeps_the_range_message_for_an_overflow() {
            let error = render_calendar_error(CalendarError::OutOfRange);
            assert!(
                error
                    .to_string()
                    .contains("date arithmetic overflowed the supported range"),
                "unexpected detail: {error}"
            );
        }
  ```

  with:

  ```rust
        #[test]
        fn reports_range_failures_with_temporal_display_and_a_source() {
            let error = render_calendar_error(CalendarError::OutOfRange);
            assert_eq!(error.to_string(), "date/time value is out of range");

            // The CalendarError rides in the source chain, so miette prints
            // a `Caused by:` line for range failures now.
            assert!(
                std::error::Error::source(&error).is_some_and(|source| {
                    source.downcast_ref::<CalendarError>().is_some()
                }),
                "CalendarError preserved as source"
            );
        }
  ```

  Run: `mise run test:unit preserves_the_local_zone_lookup_diagnosis`
  Expected: FAIL: the engine still says `local timezone lookup failed`, so the single-prefix count is 0. This is the intended red.

  Run: `mise run test:unit reports_range_failures_with_temporal_display_and_a_source`
  Expected: FAIL: the engine still renders `date arithmetic overflowed the supported range`. This is the intended red.

- [ ] **Step 2: Collapse both renderers**

  In `src/template/engine/date.rs`, change `render_date_error`:

  ```rust
  /// Translates a core parse error into a render error.
  fn render_date_error(error: DateError) -> Error {
      match error {
          DateError::LocalZoneLookup {
              input,
          } => invalid_operation(
              format!("local timezone lookup failed for {input:?}"),
              DateError::LocalZoneLookup {
                  input,
              },
          ),
          DateError::InvalidValue {
              ..
          }
          | DateError::YearNotFourDigits {
              ..
          } => invalid_operation(error.to_string(), error),
      }
  }
  ```

  to:

  ```rust
  /// Translates a core parse error into a render error.
  fn render_date_error(error: DateError) -> Error {
      invalid_operation(error.to_string(), error)
  }
  ```

  In the same file, change `render_calendar_error` (its `Zone` arm is the post-Task-12 shape):

  ```rust
  /// Translates a computed calendar failure into a render error.
  fn render_calendar_error(error: CalendarError) -> Error {
      match error {
          CalendarError::OutOfRange => date_out_of_range_error(),
          CalendarError::Zone {
              source,
          } => invalid_operation(
              source.to_string(),
              CalendarError::Zone {
                  source,
              },
          ),
      }
  }
  ```

  to:

  ```rust
  /// Translates a computed calendar failure into a render error.
  fn render_calendar_error(error: CalendarError) -> Error {
      invalid_operation(error.to_string(), error)
  }
  ```

  `InvalidValue` and `YearNotFourDigits` already rendered `error.to_string()`, so those paths are byte-identical. The `Zone` arm is byte-identical too because `#[error("{source}")]` renders the inner DateError.

  Reword `date_out_of_range_error`'s doc so the origin rule is written down where the argument-plane callers are:

  ```rust
  /// Builds the error for calendar arithmetic that exceeds the supported Jiff
  /// date or time range.
  ///
  /// Checked operations return this instead of producing an invalid value.
  fn date_out_of_range_error() -> Error {
  ```

  to:

  ```rust
  /// Builds the error for an argument-plane overflow: `checked_neg` on
  /// `i64::MIN` and `u64` to `i64` conversions fail here, before any calendar
  /// arithmetic runs.
  ///
  /// Calendar failures discovered during arithmetic report through
  /// [`render_calendar_error`] with temporal's own display text.
  fn date_out_of_range_error() -> Error {
  ```

  The function body and its six callers stay untouched.

  Run: `mise run test:unit preserves_the_local_zone_lookup_diagnosis`
  Expected: PASS, one `local zone lookup failed` occurrence and the DateError still downcastable from the source chain.

  Run: `mise run test:unit reports_range_failures_with_temporal_display_and_a_source`
  Expected: PASS, `date/time value is out of range` with the `CalendarError` in the source chain.

  Run: `mise run test:unit engine::date`
  Expected: PASS in full. Task 12's `renders_a_zone_failure_with_one_lookup_prefix` holds (the forwarded Display gives the same bytes). The argument-plane tests at :2455/:2480 keep asserting the engine overflow string only where it still lives.

- [ ] **Step 3: Regenerate the public-token baseline**

  ```bash
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  git diff --stat tests/baselines/pub-tokens.txt
  ```

  Expected: no diff.

- [ ] **Step 4: Run the full gate**

  Run: `mise run verify`
  Expected: green (fmt, check with zero warnings, clippy with zero warnings, full test suite).

- [ ] **Step 5: Commit**

  ```bash
  git add src/template/engine/date.rs tests/baselines/pub-tokens.txt
  git commit -m "fix(template): temporal Display is the single message vocabulary"
  ```

---

### Task 14 (O4): Move pattern rendering into `DatePoint::format_in`

**Files:**
- Modify: `src/temporal/error.rs` (module doc, enum summary, `InvalidFormat` variant, Display test case), `src/temporal/point.rs` (`format_in` plus a new `mod format_in`), `src/template/engine/date.rs` (six call sites, delete `format_strtime`, strengthen the `%J` test)
- Baseline: regen `tests/baselines/pub-tokens.txt` (+1 line, `format_in`) and both `cargo public-api` snapshots (+3 lines each: `DateError::InvalidFormat` and its two fields).
- Accepted behavior changes, each pinned below: an unrenderable pattern's message gains `: <Jiff's reason>`, Jiff's error leaves the source chain (its text now rides in the message), and the unreachable range edge behind sites 4-5 reports `DateError::InvalidValue` through `into_date_error` instead of `CalendarError::OutOfRange`.
- No GLOSSARY change.

**Context:** After Task 11 the engine still owns pattern execution. `format_strtime` (engine/date.rs) holds the canonical guard and the Jiff fallback, and six sites call it: `Anchor::render` (:260), `date_format` (:479), `weekday` (:939), `render_period_edge` (:704), `shift_by_unit` (:804), `from_timestamp` (:351). O4 gives execution to temporal. `DatePoint::format_in(self, pattern, zone)` resolves the zoned value, tries `render_canonical_pattern(zoned.datetime(), pattern)`, falls back to `jiff::fmt::strtime::format(pattern, &zoned)`, and reports a Jiff failure as a new `DateError::InvalidFormat { pattern, reason }`. The name joins the point.rs `_in(zone)` family (`local_datetime_in`, `zoned_datetime_in`). `DateError` is the error the crate's contract names: the error.rs module doc makes each Display string contractual, and the failing pattern belongs to the failure identity, so the error lives there rather than in `CalendarError`. Jiff's `Error` cannot ride inside `DateError` because `DateError` derives `Clone` + `Eq`, so Jiff's diagnosis folds into Display as a reason-preserving superset: `invalid date format {pattern:?}: {reason}`. Nothing pins the old wording. `invalid date format` appears once in src (engine date.rs:432), in no test and no golden file, and both `%J` tests assert `ErrorKind::InvalidOperation` only, so Step 3 strengthens the filter test into a full pin and red-writes nothing else. Two side effects ride along as accepted: Jiff's error leaves the source chain, and the message gains `: <reason>`. Sites 1-3 keep their post-Task-11 pre-resolution (zoned first), so `zone_resolution_errors_precede_pattern_errors` still sees `render_calendar_error`'s range text; `format_in` resolves a second time from the same point and the same fixed zone, which is deterministic, and its inner resolution can only fail where the pre-resolution already failed, so the `into_date_error` bridge never fires on those sites. Sites 4-6 resolve only inside `format_in`; the range edge behind sites 4-5 (`snap`/`shift` succeeded but local midnight sits outside Jiff's instant range) then reports `DateError::InvalidValue` through `into_date_error`, the bridge typed parsers already use (datetime.rs:344), instead of `CalendarError::OutOfRange`. No test reaches that edge, and `DatePoint::date`'s failure renders unchanged, so only that diagnostic moves. Site 6 rebuilds the value it builds today: `zoned_datetime_in`'s DateTime arm is `datetime.utc().to_zoned(zone.clone())` (point.rs:242), byte-identical to the current `.utc().to_zoned(zone)`. Engine ownership ends at the `ErrorKind`: every site maps `DateError` through `render_date_error`, which since Task 13 is `invalid_operation(error.to_string(), error)`, so no new arm appears. `DatePoint` is `Copy`, so a site can still use its point after calling `format_in`. Visibility: `format_in` carries no `pub` token; `DateError::InvalidFormat` is public API, hence the snapshot regen. Gate: Step 1's probe proves the engine still owns `jiff::fmt::strtime`; X1b already delivered correctness, so the task earns its keep only when execution should move wholesale to temporal.

**Branches.** The steps below execute Branch A. If review rejects the new public contract line (`DateError::InvalidFormat`), execute the same steps under Branch B: `format_in` returns `Result<String, CalendarError>`; resolution propagates with `?` unchanged (no `into_date_error` bridge, so sites 4-5 keep today's range text exactly); the failure is a new `pub(crate) CalendarError::Format { pattern: Box<str>, reason: Box<str> }` with the same Display string; the six sites map it through `render_calendar_error`; Step 3's test downcasts the source to `CalendarError::Format`; Step 2's Display case moves to `CalendarError`'s test module; and Step 7 regenerates only `pub-tokens.txt` (+1 there), because `CalendarError` is `pub(crate)` and both public-api snapshots stay byte-identical.

Branch B's `format_in` signature in full:

```rust
pub(crate) fn format_in(
    self,
    pattern: &str,
    zone: &TimeZone,
) -> Result<String, CalendarError>
```

Everything else, including both red tests' intent, is unchanged.

- [ ] **Step 1: Probe the gate**

  Run: `rg 'jiff::fmt::strtime' src/template/engine/`
  Expected: exactly one match, inside `format_strtime`. The engine still owns execution, so continue with Branch A as written.

  If there is no match, decide before continuing. Run: `rg 'format_in' src/temporal`. A hit means execution already moved. Cancel this task. Start Task 15. No hit means the engine lost execution without an owner, so execute Steps 2-9 under Branch B.

- [ ] **Step 2: Add `DateError::InvalidFormat`**

  In `src/temporal/error.rs`, change the module doc's `DateError` sentence:

  ```rust
  //! Defines public parsing errors and internal temporal arithmetic errors.
  //!
  //! [`DateError`] reports date and date-time parsing or local-zone resolution
  //! failures. Internal calendar arithmetic uses a separate error type.
  ```

  to:

  ```rust
  //! Defines public parsing errors and internal temporal arithmetic errors.
  //!
  //! [`DateError`] reports date and date-time parsing, local-zone resolution,
  //! or rendering failures. Internal calendar arithmetic uses a separate error
  //! type.
  ```

  Change the enum's summary line:

  ```rust
  /// Reports date/date-time parsing and local-zone resolution failures.
  ```

  to:

  ```rust
  /// Reports date/date-time parsing, local-zone resolution, or rendering
  /// failures.
  ```

  Add the variant after `LocalZoneLookup`:

  ```rust
      #[error("local zone lookup failed for `{input}`")]
      LocalZoneLookup {
          /// Stores the naive clock-time input that could not be resolved.
          input: Box<str>,
      },
  }
  ```

  to:

  ```rust
      #[error("local zone lookup failed for `{input}`")]
      LocalZoneLookup {
          /// Stores the naive clock-time input that could not be resolved.
          input: Box<str>,
      },
      /// Reports that a strftime pattern failed to render a resolved value.
      ///
      /// The pattern is the caller's spelling. The reason carries Jiff's
      /// diagnosis of the failure.
      #[error("invalid date format {pattern:?}: {reason}")]
      InvalidFormat {
          /// Stores the pattern that failed to render.
          pattern: Box<str>,
          /// Stores Jiff's diagnosis of the failure.
          reason: Box<str>,
      },
  }
  ```

  In the same file, add one case to `renders_exact_display_text_for_date_errors`, after the `local_zone_lookup` case:

  ```rust
        #[case::invalid_format(
            DateError::InvalidFormat {
                pattern: "%J".into(),
                reason: "unknown conversion `%J`".into(),
            },
            "invalid date format \"%J\": unknown conversion `%J`"
        )]
  ```

  Run: `mise run test:unit renders_exact_display_text_for_date_errors`
  Expected: PASS, the new case pins `invalid date format "%J": unknown conversion \`%J\``. No exhaustive `DateError` match remains elsewhere (Task 13 collapsed `render_date_error`), so the new variant breaks no compile.

- [ ] **Step 3: Strengthen the `%J` filter test (red)**

  In `mod date_format` (engine/date.rs), change the test body:

  ```rust
        #[test]
        fn rejects_an_invalid_format_specifier_instead_of_panicking() {
            let error = env()
                .render_str(
                    r#"{{ "2026-07-23" | date_format("%J") }}"#,
                    minijinja::context!(),
                )
                .expect_err("invalid format specifier fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
        }
  ```

  to:

  ```rust
        #[test]
        fn rejects_an_invalid_format_specifier_instead_of_panicking() {
            let error = env()
                .render_str(
                    r#"{{ "2026-07-23" | date_format("%J") }}"#,
                    minijinja::context!(),
                )
                .expect_err("invalid format specifier fails cleanly");

            assert_eq!(error.kind(), ErrorKind::InvalidOperation);
            let message = error.to_string();
            assert!(
                message.contains("invalid date format \"%J\":"),
                "the message names the pattern and Jiff's reason: {message}"
            );
            let source = std::error::Error::source(&error)
                .expect("a temporal source rides along");
            assert!(
                matches!(
                    source.downcast_ref::<DateError>(),
                    Some(DateError::InvalidFormat { pattern, .. })
                        if pattern.as_ref() == "%J"
                ),
                "the source is DateError::InvalidFormat, got {source:?}"
            );
        }
  ```

  Run: `mise run test:unit rejects_an_invalid_format_specifier`
  Expected: FAIL: the detail is still `invalid date format "%J"` without a reason, so the `contains` assertion trips. This is the intended red. The kind assertion still passes.

- [ ] **Step 4: Add the red `format_in` tests**

  In `src/temporal/point.rs`, add `mod format_in` to the tests mod, between `point_at` and `mod parse`:

  ```rust
    fn point_at(text: &str) -> DatePoint {
        DatePoint::parse(text).expect("valid date point")
    }

    mod parse {
        use pretty_assertions::assert_eq;
  ```

  becomes:

  ```rust
    fn point_at(text: &str) -> DatePoint {
        DatePoint::parse(text).expect("valid date point")
    }

    mod format_in {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn renders_a_canonical_pattern_with_the_sign_outside_the_width() {
            let point = point_at("-0009-01-01");

            let rendered = point
                .format_in("%Y-%m-%d", &TimeZone::UTC)
                .expect("canonical patterns render");

            assert_eq!(rendered, "-0009-01-01");
        }

        #[test]
        fn reports_an_invalid_pattern_with_the_pattern_and_the_reason() {
            let point = point_at("2026-07-23");

            let error = point
                .format_in("%J", &TimeZone::UTC)
                .expect_err("%J is not a Jiff specifier");

            match error {
                DateError::InvalidFormat { pattern, reason } => {
                    assert_eq!(pattern.as_ref(), "%J");
                    assert!(!reason.is_empty(), "Jiff supplies a reason");
                }
                other => panic!("expected InvalidFormat, got {other:?}"),
            }
        }
    }

    mod parse {
        use pretty_assertions::assert_eq;
  ```

  Run: `mise run test:unit format_in`
  Expected: FAIL: the test binary does not compile because `DatePoint::format_in` does not exist. This is the intended red.

- [ ] **Step 5: Implement `format_in`**

  In `src/temporal/point.rs`, insert the method after `zoned_datetime_in`, before the `UTC-stored Date-Time` helper:

  ```rust
            Self::DateTime(datetime) => {
                Ok(datetime.utc().to_zoned(zone.clone()))
            }
        }
    }

    /// Returns the UTC-stored Date-Time equivalent of this date point.
  ```

  becomes:

  ```rust
            Self::DateTime(datetime) => {
                Ok(datetime.utc().to_zoned(zone.clone()))
            }
        }
    }

    /// Renders this date point in `zone` with the strftime `pattern`.
    ///
    /// Canonical patterns render through the crate's encoder, so the sign
    /// stays outside the year width. Other patterns render through Jiff.
    ///
    /// # Errors
    ///
    /// - [`DateError::InvalidValue`] if `zone` resolves this point outside
    ///   the supported range.
    /// - [`DateError::InvalidFormat`] if Jiff cannot render `pattern`.
    pub(crate) fn format_in(
        self,
        pattern: &str,
        zone: &TimeZone,
    ) -> Result<String, DateError> {
        let zoned = self.zoned_datetime_in(zone).map_err(|error| {
            error.into_date_error(&self.local_datetime_in(zone).to_string())
        })?;
        if let Some(rendered) = crate::temporal::render_canonical_pattern(
            zoned.datetime(),
            pattern,
        ) {
            return Ok(rendered);
        }
        jiff::fmt::strtime::format(pattern, &zoned).map_err(|error| {
            DateError::InvalidFormat {
                pattern: pattern.into(),
                reason: error.to_string().into(),
            }
        })
    }

    /// Returns the UTC-stored Date-Time equivalent of this date point.
  ```

  The resolution runs first, so a range failure reports before any pattern renders, exactly as the sites do today. The canonical shortcut consumes the resolved civil datetime, the same value Task 11's guard reads.

  Run: `mise run test:unit format_in`
  Expected: PASS. `-0009-01-01` comes from the crate's encoder; `%J` reports `InvalidFormat` with pattern `%J` and a non-empty reason.

- [ ] **Step 6: Rewire the six sites and remove `format_strtime`**

  In `src/template/engine/date.rs`, change the tail of `Anchor::render`:

  ```rust
          let zone = crate::temporal::local_zone().map_err(render_date_error)?;
          let zoned = date_point
              .zoned_datetime_in(&zone)
              .map_err(render_calendar_error)?;
          let local_datetime = zoned.datetime();
          let pat = pattern_to_strftime(format, local_datetime)?;
          format_strtime(&zoned, &pat, format)
  ```

  to:

  ```rust
          let zone = crate::temporal::local_zone().map_err(render_date_error)?;
          let zoned = date_point
              .zoned_datetime_in(&zone)
              .map_err(render_calendar_error)?;
          let local_datetime = zoned.datetime();
          let pat = pattern_to_strftime(format, local_datetime)?;
          date_point.format_in(&pat, &zone).map_err(render_date_error)
  ```

  Change the last line of `date_format`:

  ```rust
  fn date_format(value: &str, pattern: &str) -> TemplateEngineResult<String> {
      let point = parse_date_point(value)?;
      let zone = crate::temporal::local_zone().map_err(render_date_error)?;
      let zoned =
          point.zoned_datetime_in(&zone).map_err(render_calendar_error)?;
      let datetime = zoned.datetime();
      let translated = pattern_to_strftime(pattern, datetime)?;
      format_strtime(&zoned, &translated, pattern)
  }
  ```

  to:

  ```rust
  fn date_format(value: &str, pattern: &str) -> TemplateEngineResult<String> {
      let point = parse_date_point(value)?;
      let zone = crate::temporal::local_zone().map_err(render_date_error)?;
      let zoned =
          point.zoned_datetime_in(&zone).map_err(render_calendar_error)?;
      let datetime = zoned.datetime();
      let translated = pattern_to_strftime(pattern, datetime)?;
      point.format_in(&translated, &zone).map_err(render_date_error)
  }
  ```

  Change the last line of `weekday`'s target branch:

  ```rust
        let shifted = date_point
            .with_weekday(target_weekday, &zone)
            .map_err(render_calendar_error)?;
        let zoned = shifted
            .zoned_datetime_in(&zone)
            .map_err(render_calendar_error)?;
        let local_datetime = zoned.datetime();
        let pat = pattern_to_strftime(format, local_datetime)?;
        Ok(Value::from(format_strtime(&zoned, &pat, format)?))
  ```

  to:

  ```rust
        let shifted = date_point
            .with_weekday(target_weekday, &zone)
            .map_err(render_calendar_error)?;
        let zoned = shifted
            .zoned_datetime_in(&zone)
            .map_err(render_calendar_error)?;
        let local_datetime = zoned.datetime();
        let pat = pattern_to_strftime(format, local_datetime)?;
        Ok(Value::from(
            shifted.format_in(&pat, &zone).map_err(render_date_error)?,
        ))
  ```

  Change the tail of `render_period_edge` (its head is the post-Task-5 `snap` shape):

  ```rust
    let shifted = date_point
        .snap(calendar_period, period_edge, &zone)
        .map_err(render_calendar_error)?;
    let pat = shifted.canonical_pattern();
    format_strtime(
        &shifted.zoned_datetime_in(&zone).map_err(render_calendar_error)?,
        pat,
        pat,
    )
}
  ```

  to:

  ```rust
    let shifted = date_point
        .snap(calendar_period, period_edge, &zone)
        .map_err(render_calendar_error)?;
    let pat = shifted.canonical_pattern();
    shifted.format_in(pat, &zone).map_err(render_date_error)
}
  ```

  Change the tail of `shift_by_unit`:

  ```rust
    let pat = shifted_date_point.canonical_pattern();
    format_strtime(
        &shifted_date_point
            .zoned_datetime_in(&zone)
            .map_err(render_calendar_error)?,
        pat,
        pat,
    )
}
  ```

  to:

  ```rust
    let pat = shifted_date_point.canonical_pattern();
    shifted_date_point
        .format_in(pat, &zone)
        .map_err(render_date_error)
}
  ```

  Change the tail of `from_timestamp`'s closure:

  ```rust
                    let zone = crate::temporal::local_zone()
                        .map_err(render_date_error)?;
                    let zoned = DateTimeValue::from_timestamp(timestamp)
                        .utc()
                        .to_zoned(zone);
                    format_strtime(&zoned, format, format)
  ```

  to:

  ```rust
                    let zone = crate::temporal::local_zone()
                        .map_err(render_date_error)?;
                    DatePoint::from(DateTimeValue::from_timestamp(timestamp))
                        .format_in(format, &zone)
                        .map_err(render_date_error)
  ```

  Sites 1-3 keep their pre-resolution and its `render_calendar_error` mapping; only the final line changes. Sites 4-6 keep their `canonical_pattern()` locals; `format_in` now owns resolution.

  Remove `format_strtime` (the post-Task-11 shape, doc comment included):

  ```rust
  /// Formats a zoned date-time, preferring the canonical temporal renderer for
  /// the pattern shapes the crate renders itself.
  ///
  /// # Errors
  ///
  /// Returns a [`minijinja::Error`] with kind
  /// [`ErrorKind::InvalidOperation`](minijinja::ErrorKind::InvalidOperation) if
  /// `strftime` is neither a canonical pattern nor a strftime specifier
  /// `datetime` can render.
  fn format_strtime(
      datetime: &jiff::Zoned,
      strftime: &str,
      source_pattern: &str,
  ) -> TemplateEngineResult<String> {
      if let Some(rendered) = crate::temporal::render_canonical_pattern(
          datetime.datetime(),
          strftime,
      ) {
          return Ok(rendered);
      }
      jiff::fmt::strtime::format(strftime, datetime).map_err(|source| {
          invalid_operation(
              format!("invalid date format {source_pattern:?}"),
              source,
          )
      })
  }
  ```

  The function disappears entirely. The `invalid_operation` import stays for both renderers.

  Run: `mise run test:unit rejects_an_invalid_format_specifier`
  Expected: PASS, the message contains `invalid date format "%J":` and the source downcasts to `DateError::InvalidFormat` with pattern `%J`.

  Run: `mise run test:unit engine::date`
  Expected: PASS in full: the Apia `Do|%d` pin, the precedence pin, both negative-year pins, and `now_returns_an_error_instead_of_panicking_on_an_invalid_format` (kind-only) all hold.

  Run: `rg 'jiff::fmt::strtime' src/template/engine/`
  Expected: no output. The engine holds zero format-execution references.

- [ ] **Step 7: Regenerate the three baselines**

  ```bash
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  cargo public-api --color=never > tests/baselines/public-api-default.txt
  cargo public-api --features test-utils --color=never > tests/baselines/public-api-test-utils.txt
  git diff --stat tests/baselines/
  ```

  Expected: `pub-tokens.txt` gains one line, `src/temporal/point.rs: pub(crate) fn format_in(self, pattern: &str, zone: &TimeZone) -> Result<String, DateError> {`. Each `public-api` snapshot gains three lines: the `InvalidFormat` variant plus its `pattern` and `reason` field lines, sorted before `InvalidValue`. Under Branch B, run only the first command and expect one `CalendarError::Format` line with no snapshot diff.

- [ ] **Step 8: Run the full gate**

  Run: `mise run verify`
  Expected: green (fmt, check with zero warnings, clippy with zero warnings, full test suite).

- [ ] **Step 9: Commit**

  ```bash
  git add src/temporal/error.rs src/temporal/point.rs src/template/engine/date.rs tests/baselines/pub-tokens.txt tests/baselines/public-api-default.txt tests/baselines/public-api-test-utils.txt
  git commit -m "feat(temporal): format_in owns pattern execution"
  ```

---

### Task 15 (H1): Move duration rendering out of `date.rs` into its own module

**Files:**
- Create: `src/template/engine/duration.rs`
- Modify: `src/template/engine/date.rs` (drop the two filter registrations, the function, the test module), `src/template/engine.rs` (module list, import, registration call, two doc lists)
- Baseline: regen `tests/baselines/pub-tokens.txt` (+2 lines, `DurationOps` and its `register`); both `cargo public-api` snapshots stay byte-identical.
- No GLOSSARY change.

**Context:** `date.rs` opens with "Registers date and time helpers for templates" and its filter bullet lists `date_format`, `timestamp`, `date_add`, ... , `date_diff` with no duration entry, yet the file registers `duration_format` and `durationformat` (date.rs:85-86), defines `duration_format` (:627-654), and owns the `durationformat` test module (:1674-1738). The first audit ranked this the top doc-truth mismatch: the header claims look complete while the file silently owns a non-date filter, and `engine.rs`'s helper-module list (:9-20) has no duration bullet either. H1 gives durations a module modeled exactly on the stateless siblings (`num.rs`, `string.rs`, `yaml.rs`): a unit struct, an associated `register(env)` whose doc says "This is an associated function because [...] carries no state", plain functions added through `Environment::add_filter`, and a tests mod with its own `env()` helper. `duration.rs` moves the function, its doc, the registration pair, and all four tests byte for byte; `date.rs` keeps its `DurationValue` import because `apply_offset` (:112) still parses duration strings, and it keeps `invalid_operation`, `Error`, and `ErrorKind` for its own filters. After the move `date.rs`'s module doc needs no edit: "the flat `date_*` filters and `is_*` tests" becomes exhaustive. `engine.rs` gains `mod duration;`, the `duration::DurationOps` import, one registration call after `DateOps::register`, a module-doc bullet between `date` and `file` (declaration order), and a truthful `new` doc: the sentence at :82-83 gains `[duration]` and closes the pre-existing `yaml` gap in the same breath. Behavior is unchanged: `TemplateEngine::new` registers the same filter names through a different module, and the moved tests are the pin (green before and after). Visibility: +2 `pub(super)` lines, no public API change.

- [ ] **Step 1: Create the duration module**

  Create `src/template/engine/duration.rs` with the whole file:

  ```rust
  //! Registers duration rendering filters for templates.
  //!
  //! [`DurationOps`] adds the two compound-format aliases for [`DurationValue`]:
  //! `duration_format` and `durationformat` (e.g. `"3 days, 4 hours"`).
  //!
  //! Each filter is a plain function registered through
  //! [`Environment::add_filter`], not an [`Object`], because no filter carries
  //! shared state.
  //!
  //! [`DurationValue`]: crate::DurationValue
  //! [`Object`]: minijinja::value::Object

  use minijinja::{Environment, Error, ErrorKind, value::Value};

  use crate::DurationValue;

  use super::error::{TemplateEngineResult, invalid_operation};

  /// Registration namespace for the stateless duration filters.
  pub(super) struct DurationOps;

  impl DurationOps {
      /// Registers the duration filters with `env`.
      ///
      /// This is an associated function because [`DurationOps`] carries no state.
      #[inline]
      pub(super) fn register(env: &mut Environment<'static>) {
          env.add_filter("duration_format", duration_format);
          env.add_filter("durationformat", duration_format);
      }
  }

  /// Renders a duration value or string in human-readable compound format.
  ///
  /// Syntax: `{{ value | duration_format }}` or `{{ value | durationformat }}`
  /// (e.g. `"3 days, 4 hours"`).
  ///
  /// # Errors
  ///
  /// Returns a [`minijinja::Error`] with kind
  /// [`ErrorKind::InvalidOperation`](minijinja::ErrorKind::InvalidOperation) if
  /// `value` cannot be parsed or converted into a [`DurationValue`].
  fn duration_format(value: Value) -> TemplateEngineResult<String> {
      let dv = if let Some(s) = value.as_str() {
          DurationValue::parse(s).map_err(|source| {
              invalid_operation(format!("invalid duration {s:?}"), source)
          })?
      } else if let Ok(f) = f64::try_from(value) {
          DurationValue::from_nominal_seconds(f).map_err(|source| {
              invalid_operation(
                  format!("out-of-range duration seconds {f}"),
                  source,
              )
          })?
      } else {
          return Err(Error::new(
              ErrorKind::InvalidOperation,
              "durationformat expects a duration string or number",
          ));
      };
      Ok(dv.to_compound_string())
  }

  #[cfg(test)]
  mod tests {
      use super::*;

      fn env() -> Environment<'static> {
          let mut env = Environment::new();
          DurationOps::register(&mut env);
          env
      }

      mod durationformat {
          use pretty_assertions::assert_eq;
          use rstest::rstest;

          use super::*;

          #[rstest]
          #[case::compound_days_hours("3d 4h", "3 days, 4 hours")]
          #[case::single_day("1d", "1 day")]
          #[case::single_hour("1h", "1 hour")]
          #[case::minutes_seconds("1m 30s", "1 minute, 30 seconds")]
          #[case::zero_duration("0s", "0 seconds")]
          #[case::negative_duration("-3d 4h", "-3 days, 4 hours")]
          #[case::fractional_seconds("1.5s", "2 seconds")]
          #[case::negative_fractional_seconds("-1.5s", "-2 seconds")]
          fn formats_compound_durations(
              #[case] input: &str,
              #[case] expected: &str,
          ) {
              let rendered = env()
                  .render_str(
                      &format!(r#"{{{{ "{input}" | durationformat }}}}"#),
                      minijinja::context!(),
                  )
                  .expect("render succeeds");

              assert_eq!(rendered, expected);
          }

          #[test]
          fn formats_duration_from_numeric_seconds() {
              let rendered = env()
                  .render_str(
                      r"{{ 86400 | durationformat }}",
                      minijinja::context!(),
                  )
                  .expect("render succeeds");

              assert_eq!(rendered, "1 day");
          }

          #[test]
          fn duration_format_is_an_alias_of_durationformat() {
              let rendered = env()
                  .render_str(
                      r"{{ 90 | duration_format }}|{{ 90 | durationformat }}",
                      minijinja::context!(),
                  )
                  .expect("render succeeds");

              assert_eq!(rendered, "1 minute, 30 seconds|1 minute, 30 seconds");
          }

          #[test]
          fn rejects_invalid_duration_value() {
              let error = env()
                  .render_str(
                      r#"{{ "not a duration" | durationformat }}"#,
                      minijinja::context!(),
                  )
                  .expect_err("invalid duration fails");

              assert_eq!(error.kind(), ErrorKind::InvalidOperation);
          }
      }
  }
  ```

  The function, its doc, and the four tests are verbatim moves from `date.rs`. Only the module doc, the struct, `register`, and the `env()` helper are new.

- [ ] **Step 2: Strip the duration code from `date.rs`**

  In `src/template/engine/date.rs`, remove the two registration lines:

  ```rust
        env.add_filter("date_format", date_format);
        env.add_filter("duration_format", duration_format);
        env.add_filter("durationformat", duration_format);
        env.add_filter("timestamp", timestamp);
  ```

  becomes:

  ```rust
        env.add_filter("date_format", date_format);
        env.add_filter("timestamp", timestamp);
  ```

  Remove `duration_format` and its doc comment (keep the `timestamp` doc that follows):

  ```rust
  /// Renders a duration value or string in human-readable compound format.
  ///
  /// Syntax: `{{ value | duration_format }}` or `{{ value | durationformat }}`
  /// (e.g. `"3 days, 4 hours"`).
  ///
  /// # Errors
  ///
  /// Returns a [`minijinja::Error`] with kind
  /// [`ErrorKind::InvalidOperation`](minijinja::ErrorKind::InvalidOperation) if
  /// `value` cannot be parsed or converted into a [`DurationValue`].
  fn duration_format(value: Value) -> TemplateEngineResult<String> {
      let dv = if let Some(s) = value.as_str() {
          DurationValue::parse(s).map_err(|source| {
              invalid_operation(format!("invalid duration {s:?}"), source)
          })?
      } else if let Ok(f) = f64::try_from(value) {
          DurationValue::from_nominal_seconds(f).map_err(|source| {
              invalid_operation(
                  format!("out-of-range duration seconds {f}"),
                  source,
              )
          })?
      } else {
          return Err(Error::new(
              ErrorKind::InvalidOperation,
              "durationformat expects a duration string or number",
          ));
      };
      Ok(dv.to_compound_string())
  }

  /// Converts a piped date/time string to Unix seconds.
  ```

  becomes:

  ```rust
  /// Converts a piped date/time string to Unix seconds.
  ```

  Remove the `durationformat` test module (keep the `timestamp` module that follows):

  ```rust
      mod durationformat {
          use pretty_assertions::assert_eq;
          use rstest::rstest;

          use super::*;

          #[rstest]
          #[case::compound_days_hours("3d 4h", "3 days, 4 hours")]
          #[case::single_day("1d", "1 day")]
          #[case::single_hour("1h", "1 hour")]
          #[case::minutes_seconds("1m 30s", "1 minute, 30 seconds")]
          #[case::zero_duration("0s", "0 seconds")]
          #[case::negative_duration("-3d 4h", "-3 days, 4 hours")]
          #[case::fractional_seconds("1.5s", "2 seconds")]
          #[case::negative_fractional_seconds("-1.5s", "-2 seconds")]
          fn formats_compound_durations(
              #[case] input: &str,
              #[case] expected: &str,
          ) {
              let rendered = env()
                  .render_str(
                      &format!(r#"{{{{ "{input}" | durationformat }}}}"#),
                      minijinja::context!(),
                  )
                  .expect("render succeeds");

              assert_eq!(rendered, expected);
          }

          #[test]
          fn formats_duration_from_numeric_seconds() {
              let rendered = env()
                  .render_str(
                      r"{{ 86400 | durationformat }}",
                      minijinja::context!(),
                  )
                  .expect("render succeeds");

              assert_eq!(rendered, "1 day");
          }

          #[test]
          fn duration_format_is_an_alias_of_durationformat() {
              let rendered = env()
                  .render_str(
                      r"{{ 90 | duration_format }}|{{ 90 | durationformat }}",
                      minijinja::context!(),
                  )
                  .expect("render succeeds");

              assert_eq!(rendered, "1 minute, 30 seconds|1 minute, 30 seconds");
          }

          #[test]
          fn rejects_invalid_duration_value() {
              let error = env()
                  .render_str(
                      r#"{{ "not a duration" | durationformat }}"#,
                      minijinja::context!(),
                  )
                  .expect_err("invalid duration fails");

              assert_eq!(error.kind(), ErrorKind::InvalidOperation);
          }
      }

      mod timestamp {
  ```

  becomes:

  ```rust
      mod timestamp {
  ```

  The `DurationValue` import at date.rs:44 stays; `apply_offset` (:112) still uses it. The module doc needs no edit: its `date_*` filter claim becomes exhaustive.

- [ ] **Step 3: Wire the module into `engine.rs`**

  In `src/template/engine.rs`, register the module after `date`:

  ```rust
  mod cache;
  mod date;
  mod error;
  ```

  becomes:

  ```rust
  mod cache;
  mod date;
  mod duration;
  mod error;
  ```

  Add the import to the `use self::{...}` block:

  ```rust
  use self::{
      date::DateOps,
      file::{FileOps, WRITE_TO_KEY},
  ```

  becomes:

  ```rust
  use self::{
      date::DateOps,
      duration::DurationOps,
      file::{FileOps, WRITE_TO_KEY},
  ```

  Register it after the date namespace:

  ```rust
        DateOps::register(&mut env);
        StrOps::register(&mut env);
  ```

  becomes:

  ```rust
        DateOps::register(&mut env);
        DurationOps::register(&mut env);
        StrOps::register(&mut env);
  ```

  Add the helper-module bullet between `date` and `file`:

  ```rust
  //! - [`date`] registers date generators, date filters, and date tests.
  //! - [`mod@file`] registers `file.write_to()` and `file.include()`.
  ```

  becomes:

  ```rust
  //! - [`date`] registers date generators, date filters, and date tests.
  //! - [`duration`] registers duration rendering filters.
  //! - [`mod@file`] registers `file.write_to()` and `file.include()`.
  ```

  Fix the submodule list in `TemplateEngine::new`'s doc (it omits `yaml` as well as duration):

  ```rust
      /// Registers functions from the [`date`], [`mod@file`], [`path`], [`num`],
      /// [`query`], [`mod@schema`], [`string`], and [`ui`] submodules. Enables
      /// debug mode on the underlying minijinja environment to support line and
  ```

  becomes:

  ```rust
      /// Registers functions from the [`date`], [`duration`], [`mod@file`],
      /// [`path`], [`num`], [`query`], [`mod@schema`], [`string`], [`ui`], and
      /// [`mod@yaml`] submodules. Enables
      /// debug mode on the underlying minijinja environment to support line and
  ```

- [ ] **Step 4: Run the moved tests and the date suite**

  Run: `mise run test:unit durationformat`
  Expected: PASS, all four tests run from `duration.rs` with the same registrations.

  Run: `mise run test:unit engine::date`
  Expected: PASS in full. No date test renders a duration filter, so dropping the registration from `DateOps::register` changes nothing.

- [ ] **Step 5: Regenerate the pub-token baseline**

  ```bash
  git grep -I -E '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ | sed -E 's/^([^:]+):[[:space:]]*(.*)/\1: \2/; s/[[:space:]]+/ /g' | LC_ALL=C sort > tests/baselines/pub-tokens.txt
  git diff --stat tests/baselines/pub-tokens.txt
  ```

  Expected: +2 lines, zero removals. Both lines are `src/template/engine/duration.rs:` entries (`pub(super) fn register(env: &mut Environment<'static>) {` and `pub(super) struct DurationOps;`), sorted between the `date.rs` and `error.rs` entries. The `cargo public-api` snapshots stay byte-identical.

- [ ] **Step 6: Run the full gate**

  Run: `mise run verify`
  Expected: green (fmt, check with zero warnings, clippy with zero warnings, full test suite).

- [ ] **Step 7: Commit**

  ```bash
  git add src/template/engine.rs src/template/engine/date.rs src/template/engine/duration.rs tests/baselines/pub-tokens.txt
  git commit -m "refactor(template): move duration rendering into duration.rs"
  ```

---

### Task 16 (D): Documentation truth pass and hygiene nits

**Files:**
- Modify: `src/template/engine/date.rs` (X3, X7a, X7b, X1c, three H5 nits), `src/temporal/GLOSSARY.md` (X7c), `src/temporal/duration.rs`, `src/temporal/error.rs` (X7d), `.scratch/temporal-core/spec.md`, `.scratch/temporal-core/divergence-register.md` (D5), `.scratch/temporal-core/issues/06-template-format-dialects.md` (Task 6 hand-off sweep)
- Baseline: none. Doc text moves no `pub` line, so both `cargo public-api` snapshots and `tests/baselines/pub-tokens.txt` stay byte-identical.
- `src/GLOSSARY.md`: no edit. Task 3's invariant test made its sync-contract claim machine-enforced.

**Context:** The final pass closes the audit's documentation group (X3, X7a-d, X1c, D5, and the `is_leap_year` doc) plus the H5 hygiene nits, all against the post-Task-15 tree. X3: the `date_add`, `date_sub`, and `date_diff` docs promise `unit="ms"`, but the unit registry rejects `ms` (its own reject case is named `millisecond_compact`) and `DurationValue::parse` has no millisecond unit, so the documented spelling errors at runtime. X7a: `date_diff` cites `DateValue::whole_months_until`, which does not exist; the method that does the work is `DatePoint::diff` (point.rs:407), and the citation is plain code text, so the fix stays a backticked name with no new link. X7b: the module doc describes a retry cascade ("A full datetime is tried first, falling back...") while `DatePoint::parse` dispatches once on the input shape (`from_date_shape`, failure reason "not a date or date-time shape"). X7c: the temporal glossary calls `TimeUnit` one of the "eight accepted duration quantities" while `TEMPORAL_UNITS` is `[UnitSpec; 7]`; the catalog's `unit.rs:52` cite lands in `UNIT_LOOKUP`, which carries no count, and the "eight registered names" sentence at unit.rs:309 counts `DateUnit`'s eight field names and is correct, so only the glossary line changes. X7d: `DurationValue::parse`'s error list promises `MalformedNumber`, but a grep of `src/` shows the variant constructed only by the Display test at error.rs:252; the bullet comes out, the variant doc records its reachability, and the variant itself stays because it is public contract. The `is_leap_year` doc claims `i32` while the code used `i16::try_from`; Task 1 recorded that offloading makes the doc true, and Step 12 verifies it. X1c: `date_format` gains one paragraph on explicit `%Y` semantics (jiff counts the sign inside the width; canonical forms round-trip outside it), inserted after the `fmt::strtime` paragraph so the syntax paragraph, the `# Errors` paragraph, the engine wrapper line, and `pattern_to_strftime`'s doc, all owned by Task 8, stay byte-identical. D5: the spec's format-bindings bullet and divergence-register Entry 10 still describe chrono; both get jiff-era wording (`%+` fails hard under jiff, `%S` never renders `60`), and the spec edit matches on the line's prefix because Task 6 rewrote an earlier line in the same file. Task 6's hand-off: its Step 4 deferred `spec.md:104`'s chrono-era `format_with`/`DelayedFormat` bullet and issues/06's `format_with`/template-seam mentions to this pass; Step 8 sweeps the four mentions in the issue's living sections (What to build, Rules, and the two AC lines), while the Agent Brief, Triage Notes, and Comments sections stay untouched as historical records. H5 (catalog disposition D/P4): `mod date_diff` re-imports `ZoneGuard` though the tests root already imports it and `use super::*` re-exports it (the same path Task 3's notes rely on); `mod is_past_and_is_future` lacks the blank line its siblings have; and `impl Object for DateOps` sits between `weekday_or_error` and `parse_date_point` while `file.rs` and `query.rs` keep each `impl Object` beside its type, so the block moves next to `impl DateOps` as a byte-identical cut and paste. The block quoted in Step 11 is the post-Task-2 `get_value` body with Task 9's rename and Task 14's `from_timestamp` rewire applied; nothing references item order, so the move is behavior-neutral. GLOSSARY: `src/temporal/GLOSSARY.md` changes in Step 5; `src/GLOSSARY.md` stays untouched.

- [ ] **Step 1: Drop the `"ms"` promise from `date_add` and `date_sub`**

  In `src/template/engine/date.rs`, change the `date_add` doc:

  ```rust
  /// Adds `n` units to a piped date/time string.
  ///
  /// Syntax: `{{ value | date_add(n, unit="days") }}`.
  /// `unit` defaults to `"days"` and accepts any [`TimeUnit`] spelling
  /// (e.g. `"years"`/`"y"`, `"months"`/`"mo"`, `"weeks"`/`"w"`, `"days"`/`"d"`,
  /// `"hours"`/`"h"`, `"minutes"`/`"m"`, `"seconds"`/`"s"`, or `"ms"`).
  ```

  to:

  ```rust
  /// Adds `n` units to a piped date/time string.
  ///
  /// Syntax: `{{ value | date_add(n, unit="days") }}`.
  /// `unit` defaults to `"days"` and accepts any [`TimeUnit`] spelling
  /// (e.g. `"years"`/`"y"`, `"months"`/`"mo"`, `"weeks"`/`"w"`, `"days"`/`"d"`,
  /// `"hours"`/`"h"`, `"minutes"`/`"m"`, `"seconds"`/`"s"`).
  ```

  Then change the `date_sub` doc the same way:

  ```rust
  /// Subtracts `n` units from a piped date/time string.
  ///
  /// Syntax: `{{ value | date_sub(n, unit="days") }}`.
  /// `unit` defaults to `"days"` and accepts any [`TimeUnit`] spelling
  /// (e.g. `"years"`/`"y"`, `"months"`/`"mo"`, `"weeks"`/`"w"`, `"days"`/`"d"`,
  /// `"hours"`/`"h"`, `"minutes"`/`"m"`, `"seconds"`/`"s"`, or `"ms"`).
  ```

  to:

  ```rust
  /// Subtracts `n` units from a piped date/time string.
  ///
  /// Syntax: `{{ value | date_sub(n, unit="days") }}`.
  /// `unit` defaults to `"days"` and accepts any [`TimeUnit`] spelling
  /// (e.g. `"years"`/`"y"`, `"months"`/`"mo"`, `"weeks"`/`"w"`, `"days"`/`"d"`,
  /// `"hours"`/`"h"`, `"minutes"`/`"m"`, `"seconds"`/`"s"`).
  ```

- [ ] **Step 2: Fix the `date_diff` unit docs and its method cite (X3, X7a)**

  In `src/template/engine/date.rs`, change the `date_diff` doc:

  ```rust
  /// The `unit` kwarg defaults to `"days"` and accepts `"years"`, `"months"`,
  /// `"weeks"`, `"hours"`, `"minutes"`, `"seconds"`, or `"ms"`.
  /// `"years"`/`"months"` are calendar counts: whole units elapsed, day-of-month
  /// aware (see `DateValue::whole_months_until`),
  /// always an `i64` regardless of input precision. The remaining units,
  /// including `"weeks"`, `"ms"`, and the default `"days"`, are fixed-duration:
  ```

  to:

  ```rust
  /// The `unit` kwarg defaults to `"days"` and accepts `"years"`, `"months"`,
  /// `"weeks"`, `"hours"`, `"minutes"`, or `"seconds"`.
  /// `"years"`/`"months"` are calendar counts: whole units elapsed, day-of-month
  /// aware (see `DatePoint::diff`),
  /// always an `i64` regardless of input precision. The remaining units,
  /// including `"weeks"` and the default `"days"`, are fixed-duration:
  ```

- [ ] **Step 3: Rewrite the parse paragraph in the module doc (X7b)**

  In `src/template/engine/date.rs`, change:

  ```rust
  //! Date/time string parsing funnels through [`DatePoint::parse`] via
  //! [`parse_local_datetime`]. A full datetime is tried first, falling back to a
  //! bare ISO date (`YYYY-MM-DD` or reduced-precision `YYYY-MM`) at midnight.
  //! Arithmetic filters re-serialize at the input's original precision via the
  //! calendar owner.
  ```

  to:

  ```rust
  //! Date/time string parsing funnels through [`DatePoint::parse`] via
  //! [`parse_local_datetime`], which dispatches once on the input's shape: a
  //! full date-time shape parses as a date-time, and a bare ISO date shape
  //! (`YYYY-MM-DD` or reduced-precision `YYYY-MM`) parses as a calendar date
  //! at midnight. Arithmetic filters re-serialize at the input's original
  //! precision via the calendar owner.
  ```

- [ ] **Step 4: Add the explicit `%Y` semantics paragraph to `date_format` (X1c)**

  In `src/template/engine/date.rs`, change:

  ```rust
  /// Jiff [`fmt::strtime`] formats a zoned date-time, so `%Z` reflects the
  /// resolved local zone even on either side of a DST fold. Leap seconds are
  /// not representable. Week numbers use ISO `%V`/`%G`, never `%U` or `%W`.
  ///
  /// # Errors
  ```

  to:

  ```rust
  /// Jiff [`fmt::strtime`] formats a zoned date-time, so `%Z` reflects the
  /// resolved local zone even on either side of a DST fold. Leap seconds are
  /// not representable. Week numbers use ISO `%V`/`%G`, never `%U` or `%W`.
  ///
  /// Explicit `%Y` is display-only for negative years: jiff counts the sign
  /// inside the four-digit width, so year `-9` renders as `-009`, which the
  /// crate's four-digit-year rule rejects on reparse. The canonical
  /// round-trip forms (`%Y-%m-%d`, `%Y-%m`, `%F`, and the canonical datetime
  /// patterns) keep the sign outside the year digits and reparse exactly.
  ///
  /// # Errors
  ```

  The syntax paragraph and the `# Errors` paragraph belong to Task 8. Do not touch either.

- [ ] **Step 5: Correct the quantity count in the temporal glossary (X7c)**

  In `src/temporal/GLOSSARY.md`, change:

  ```markdown
  `TimeUnit` is one of the eight accepted duration quantities. Its shared
  ```

  to:

  ```markdown
  `TimeUnit` is one of the seven accepted duration quantities. Its shared
  ```

- [ ] **Step 6: Correct the `MalformedNumber` docs (X7d)**

  In `src/temporal/error.rs`, change the variant doc:

  ```rust
      /// Reports that the number portion is not valid float syntax (e.g., a bare
      /// `"."`).
  ```

  to:

  ```rust
      /// Reports that the number portion is not valid float syntax (e.g., a bare
      /// `"."`). No production path constructs this variant; it stays as public
      /// contract.
  ```

  In `src/temporal/duration.rs`, remove the unreachable bullet from `parse`'s error list:

  ```rust
      /// - [`DurationError::MalformedNumber`] if a component has invalid number
      ///   syntax.
      /// - [`DurationError::MissingUnit`] if a number appears without a following
  ```

  to:

  ```rust
      /// - [`DurationError::MissingUnit`] if a number appears without a following
  ```

- [ ] **Step 7: Refresh the chrono-era format notes (D5)**

  In `.scratch/temporal-core/divergence-register.md`, change Entry 10's divergence line:

  ```markdown
  - **Divergence**: Core formatting uses `chrono::format::strftime` syntax, while template date filters accept `moment.js` tokens via a scoped translation layer (bracket literals, `YYYY MM DD HH mm ss`, `Do`/`S` ordinals, `dddd`/`ddd`/`MMM`/`MMMM`). Unsupported moment tokens fail with an explicit diagnostic naming the token.
  ```

  to:

  ```markdown
  - **Divergence**: Core formatting uses jiff `strtime` syntax, while template date filters accept `moment.js` tokens via a scoped translation layer (bracket literals, `YYYY MM DD HH mm ss`, `Do`/`S` ordinals, `dddd`/`ddd`/`MMM`/`MMMM`). Unsupported moment tokens fail with an explicit diagnostic naming the token.
  ```

  In `.scratch/temporal-core/spec.md`, replace the format-bindings line. The script matches on the line's prefix, so Task 6's earlier edit to the same file cannot break it:

  ```bash
  python3 - <<'PY'
  from pathlib import Path

  path = Path(".scratch/temporal-core/spec.md")
  text = path.read_text(encoding="utf-8")
  prefix = "- **Format bindings:** core formatting stays a thin chrono wrapper"
  assert text.count(prefix) == 1, "format-bindings line not found exactly once"
  start = text.index(prefix)
  end = text.index("\n", start)
  new = (
      "- **Format bindings:** core formatting is a thin jiff wrapper; jiff's "
      "`strtime` syntax is the documented grammar (intra-doc link in touched "
      "rustdoc); invalid-pattern detection is jiff's own pattern error surfaced "
      "with the pattern; no parallel specifier validator; `%+` is not a jiff "
      "directive and fails hard; RFC3339 interop emits the `…Z` channel "
      "(RPT-F3 closed); format docs note `%Z` prints only an offset, `%S` never "
      "renders `60`, and week numbers use `%V`/`%G` (ISO), not `%U`/`%W`."
  )
  path.write_text(text[:start] + new + text[end:], encoding="utf-8")
  print("rewrote format-bindings line")
  PY
  ```

  Expected: the script prints `rewrote format-bindings line`.

- [ ] **Step 8: Sweep the retired `format_with`/`DelayedFormat` references (Task 6 hand-off)**

  In `.scratch/temporal-core/spec.md`, replace the `Invalid-pattern` bullet. The script matches on the line's prefix, so Task 6's earlier edit to the same file cannot break it:

  ```bash
  python3 - <<'PY'
  from pathlib import Path

  path = Path(".scratch/temporal-core/spec.md")
  text = path.read_text(encoding="utf-8")
  prefix = '- Invalid-pattern (`DelayedFormat`) rendering lives at exactly one site'
  assert text.count(prefix) == 1, "invalid-pattern line not found exactly once"
  start = text.index(prefix)
  end = text.index("\n", start)
  new = (
      '- Invalid-pattern rendering lives at exactly one site: `DatePoint::format_in`'
      ' (`src/temporal/point.rs`); we own no validator. **(Amended 2026-10-25,'
      ' temporal-offload Task 16:** the chrono-era `format_with`/`DelayedFormat`'
      " wording predates the jiff offload; Task 14 deleted the engine's"
      ' `format_strtime`, the guard now sits in `DatePoint::format_in`, and'
      ' failures surface as `DateError::InvalidFormat`. `grep -rn "format_with" src/`'
      ' finds no match.**)**'
  )
  path.write_text(text[:start] + new + text[end:], encoding="utf-8")
  print("rewrote invalid-pattern line")
  PY
  ```

  Expected: the script prints `rewrote invalid-pattern line`.

  In `.scratch/temporal-core/issues/06-template-format-dialects.md`, sweep the four `format_with`/template-seam mentions in its living sections. The Agent Brief, Triage Notes, and Comments sections are historical and stay untouched:

  ```bash
  python3 - <<'PY'
  from pathlib import Path
  import re

  path = Path(".scratch/temporal-core/issues/06-template-format-dialects.md")
  text = path.read_text(encoding="utf-8")

  text, n = re.subn(
      r"Invalid-pattern detection exists once.*?→ no match\)\.",
      "Invalid-pattern detection exists once: `DatePoint::format_in` "
      "(`src/temporal/point.rs`), the sole renderer (ticket 03's remediation "
      "deleted the core copies; temporal-offload Task 14 deleted the engine's "
      "`format_strtime` and jiff failures surface as `DateError::InvalidFormat`).",
      text,
      count=1,
  )
  assert n == 1, "what-to-build sentence not found"

  old = "cargo test --doc`); translator is a pure function at the template seam"
  assert text.count(old) == 1, "rules seam clause not found exactly once"
  text = text.replace(
      old,
      "cargo test --doc`); translator is a pure function in `temporal::token`, "
      "reached through the template seam",
  )

  old = "regression-pinned so a future copy doesn't reappear"
  assert text.count(old) == 1, "AC invalid-pattern tail not found exactly once"
  text = text.replace(
      old,
      old + " **(Amended 2026-10-25, temporal-offload Task 16:** the one site "
      "is now `DatePoint::format_in` (`src/temporal/point.rs`), which maps "
      "jiff's format failure as `DateError::InvalidFormat`; `format_with` no "
      "longer exists.**)**",
  )

  old = "it no longer does)"
  assert text.count(old) == 1, "AC translator tail not found exactly once"
  text = text.replace(
      old,
      old + " **(Amended 2026-10-25, temporal-offload Task 16:** the translator "
      "and its dialect dispatch live in `temporal::token` (Task 6) and "
      "strftime execution in `DatePoint::format_in` (Task 14); the engine "
      "keeps only a thin `pattern_to_strftime` wrapper.**)**",
  )

  path.write_text(text, encoding="utf-8")
  print("swept issues/06 format_with and template-seam mentions")
  PY
  ```

  Expected: the script prints `swept issues/06 format_with and template-seam mentions`.

- [ ] **Step 9: Remove the duplicate `ZoneGuard` import (H5)**

  In `src/template/engine/date.rs`, in `mod date_diff`, change:

  ```rust
          use super::*;
          use crate::temporal::test_support::ZoneGuard;
  ```

  to:

  ```rust
          use super::*;
  ```

  The tests root already imports `ZoneGuard`, and `use super::*` re-exports it.

- [ ] **Step 10: Add the missing blank line before `mod is_past_and_is_future` (H5)**

  In `src/template/engine/date.rs`, change:

  ```rust
          }
      }
      mod is_past_and_is_future {
  ```

  to:

  ```rust
          }
      }

      mod is_past_and_is_future {
  ```

- [ ] **Step 11: Move `impl Object for DateOps` next to `impl DateOps` (H5)**

  First remove the block from its current position. In `src/template/engine/date.rs`, change:

  ```rust
  impl Object for DateOps {
      fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
          let key = key.as_str()?;
          let mut buf = [0u8; 16];
          let lower = if key.len() <= buf.len() {
              for (dst, byte) in buf.iter_mut().zip(key.as_bytes()) {
                  *dst = byte.to_ascii_lowercase();
              }
              std::str::from_utf8(&buf[..key.len()]).ok()?
          } else {
              key
          };
          match lower {
              "now" => Some(shorthand(Anchor::Now)),
              "today" => Some(shorthand(Anchor::Today)),
              "tomorrow" => Some(shorthand(Anchor::Tomorrow)),
              "yesterday" => Some(shorthand(Anchor::Yesterday)),
              "weekday" => Some(Value::from_function(
                  |n: i64,
                   format: Option<&str>,
                   reference: Option<&str>,
                   reference_format: Option<&str>,
                   kwargs: Kwargs|
                   -> TemplateEngineResult<String> {
                      let target = weekday_or_error(n)?;
                      Anchor::Weekday(target).render(
                          format,
                          None,
                          reference,
                          reference_format,
                          &kwargs,
                      )
                  },
              )),
              "from_timestamp" => Some(Value::from_function(
                  |unix_ts: i64,
                   kwargs: Kwargs|
                   -> TemplateEngineResult<String> {
                      let format = format_kwarg(&kwargs)?;
                      let timestamp =
                          jiff::Timestamp::new(unix_ts, 0).map_err(|_| {
                              Error::new(
                                  ErrorKind::InvalidOperation,
                                  format!("timestamp {unix_ts} is out of range"),
                              )
                          })?;
                      let zone = crate::temporal::local_zone()
                          .map_err(render_date_error)?;
                      DatePoint::from(DateTimeValue::from_timestamp(timestamp))
                          .format_in(format, &zone)
                          .map_err(render_date_error)
                  },
              )),
              edge_name => {
                  let (period, edge) = resolve_period_edge(edge_name)?;
                  let anchor = match edge {
                      PeriodEdge::Start => Anchor::StartOf(period),
                      PeriodEdge::End => Anchor::EndOf(period),
                  };
                  Some(shorthand(anchor))
              }
          }
      }

      fn enumerate(self: &Arc<Self>) -> Enumerator {
          Enumerator::Str(METHODS)
      }
  }

  /// Parses `s` as a date or date-time expression via [`DatePoint::parse`].
  ```

  to:

  ```rust
  /// Parses `s` as a date or date-time expression via [`DatePoint::parse`].
  ```

  Then insert the same block directly after `impl DateOps`. Change:

  ```rust
          env.add_global("date", Value::from_object(DateOps));
      }
  }

  fn apply_offset(
  ```

  to:

  ```rust
          env.add_global("date", Value::from_object(DateOps));
      }
  }

  impl Object for DateOps {
      fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
          let key = key.as_str()?;
          let mut buf = [0u8; 16];
          let lower = if key.len() <= buf.len() {
              for (dst, byte) in buf.iter_mut().zip(key.as_bytes()) {
                  *dst = byte.to_ascii_lowercase();
              }
              std::str::from_utf8(&buf[..key.len()]).ok()?
          } else {
              key
          };
          match lower {
              "now" => Some(shorthand(Anchor::Now)),
              "today" => Some(shorthand(Anchor::Today)),
              "tomorrow" => Some(shorthand(Anchor::Tomorrow)),
              "yesterday" => Some(shorthand(Anchor::Yesterday)),
              "weekday" => Some(Value::from_function(
                  |n: i64,
                   format: Option<&str>,
                   reference: Option<&str>,
                   reference_format: Option<&str>,
                   kwargs: Kwargs|
                   -> TemplateEngineResult<String> {
                      let target = weekday_or_error(n)?;
                      Anchor::Weekday(target).render(
                          format,
                          None,
                          reference,
                          reference_format,
                          &kwargs,
                      )
                  },
              )),
              "from_timestamp" => Some(Value::from_function(
                  |unix_ts: i64,
                   kwargs: Kwargs|
                   -> TemplateEngineResult<String> {
                      let format = format_kwarg(&kwargs)?;
                      let timestamp =
                          jiff::Timestamp::new(unix_ts, 0).map_err(|_| {
                              Error::new(
                                  ErrorKind::InvalidOperation,
                                  format!("timestamp {unix_ts} is out of range"),
                              )
                          })?;
                      let zone = crate::temporal::local_zone()
                          .map_err(render_date_error)?;
                      DatePoint::from(DateTimeValue::from_timestamp(timestamp))
                          .format_in(format, &zone)
                          .map_err(render_date_error)
                  },
              )),
              edge_name => {
                  let (period, edge) = resolve_period_edge(edge_name)?;
                  let anchor = match edge {
                      PeriodEdge::Start => Anchor::StartOf(period),
                      PeriodEdge::End => Anchor::EndOf(period),
                  };
                  Some(shorthand(anchor))
              }
          }
      }

      fn enumerate(self: &Arc<Self>) -> Enumerator {
          Enumerator::Str(METHODS)
      }
  }

  fn apply_offset(
  ```

- [ ] **Step 12: Check the `is_leap_year` doc claim**

  Run: `grep -n 'i16' src/template/engine/date.rs`
  Expected: no matches. Task 1 rewrote `is_leap_year` onto `i32`, so the doc line that promises an `i32` year is now true.

- [ ] **Step 13: Run the gates**

  Run: `mise run doc --all-features`
  Expected: green (rustdoc builds with warnings denied; every doc link in the moved block and the edited comments resolves).

  Run: `mise run verify`
  Expected: green (fmt, check, lint, full test suite; no baseline regen: doc text only).

- [ ] **Step 14: Commit**

  ```bash
  git add src/template/engine/date.rs src/temporal/GLOSSARY.md src/temporal/duration.rs src/temporal/error.rs .scratch/temporal-core/spec.md .scratch/temporal-core/divergence-register.md .scratch/temporal-core/issues/06-template-format-dialects.md
  git commit -m "docs(temporal): align stale claims with jiff-era behavior"
  ```

---

### Task 17 (F1): Quote the unit hint in filter unit rejections

**Files:**
- Modify: `src/lexer.rs` (`LexError::UnexpectedEndOfInput.expected` becomes `String`; two construction sites, two test literals)
- Modify: `src/query/error.rs` (`unexpected_end` takes `impl Into<String>`; one test literal)
- Modify: `src/query/grammar/filter.rs` (the `date_add`/`date_diff` unit branches quote the hint; delete `is_unrecognized_unit`; one new test)
- Baseline: none (`pub` first lines change nowhere, so both `cargo public-api` snapshots stay byte-identical). No GLOSSARY changes.

**Context:** after Task 3, `UnitError` carries the full spelling hint (`unknown unit "mo3" (expected y, mo, ...)`), but filter's validation still rejects a bad unit with the static expectation `"recognized duration unit"`, so the query diagnostic never lists the valid spellings. The adoption is deliberately narrow: the two validation branches call `TimeUnit::parse_spelling` and feed `error.to_string()` into `QuerySyntaxError::unexpected_end`; the two `evaluate` sites keep `from_spelling` by design, because evaluation stays `Option`-shaped and never fails on a spellable unit. The dynamic text needs one field type change: `LexError::UnexpectedEndOfInput.expected` becomes `String`. `LexError` is `pub(crate)`, so no public surface moves, and `unexpected_end` takes `impl Into<String>`, so its literal call sites compile unchanged; `LexError::UnexpectedToken.expected` stays `&'static str`, since per-variant field types mean nothing forces that widening. No test pins the old static phrase, the field type, or the widened signature, so the only new assertion is this task's own. The carrier keeps its phrasing: the user sees `unexpected end of input, expected unknown unit "mo3" (expected ...)`, and the pre-existing double `expected` is accepted as the carrier's own wording. GLOSSARY: **no edit**; no new domain vocabulary.

- [ ] **Step 1: Write the failing test**

  Add this rstest beside `rejects_unquoted_date_shapes_that_are_not_dates` in the `filter.rs` test module:

  ```rust
        #[rstest]
        #[case::date_add_unit("date_add(due, 17, \"mo3\") == \"2026-08-15\"")]
        #[case::date_diff_unit("date_diff(due, \"2026-07-29\", \"mo3\") == 15")]
        fn unit_rejections_quote_the_unit_hint(#[case] expr: &str) {
            let QueryBuilderError::Syntax(syntax) = FilterExpr::parse(expr)
                .expect_err("unknown unit rejects")
            else {
                panic!("expected a syntax error");
            };

            match &*syntax.lex_error {
                LexError::UnexpectedEndOfInput { expected, .. } => assert!(
                    expected.contains("unknown unit \"mo3\" (expected "),
                    "expected the unit hint in {expected:?}"
                ),
                other => panic!("expected end of input, got {other:?}"),
            }
        }
  ```

  Run: `mise run test:unit unit_rejections_quote_the_unit_hint`
  Expected: FAIL: both cases parse to the static expectation, so the assertion reports `expected="recognized duration unit"`. The test compiles on the untouched tree: `expected` is `&str` there and `&String` after Step 2, and `contains` works for both.

- [ ] **Step 2: Make `LexError::UnexpectedEndOfInput.expected` a `String`**

  In `src/lexer.rs`, change the field declaration:

  ```rust
      /// The input stream ended unexpectedly.
      #[error("unexpected end of input, expected {expected}")]
      UnexpectedEndOfInput {
          /// The byte span at the end of input.
          span: Range<usize>,
          /// Description of the expected token.
          expected: &'static str,
      },
  ```

  becomes:

  ```rust
      /// The input stream ended unexpectedly.
      #[error("unexpected end of input, expected {expected}")]
      UnexpectedEndOfInput {
          /// The byte span at the end of input.
          span: Range<usize>,
          /// Description of the expected token.
          expected: String,
      },
  ```

  Append `.into()` to `expected` in the `None =>` arms of `expect` (value `expected.desc`) and `expect_map` (value `expected_desc`). Both `UnexpectedToken` arms keep `expected: &'static str` unchanged. Append `.into()` to the `expected: "a literal value"` literal in `display_unexpected_end_of_input_includes_expected`, and to the `UnexpectedEndOfInput { span: span.clone(), expected: "y" }` literal in `span_returns_the_stored_source_span`. That test's `UnexpectedToken` literal keeps the plain `&'static str` form.

- [ ] **Step 3: Widen `unexpected_end`**

  In `src/query/error.rs`, change the parameter type from `expected: &'static str` to `expected: impl Into<String>` and the body field from `expected,` to `expected: expected.into(),`.

  In the `syntax_error_preserves_dialect_input_span_and_label` test, change the assertion's `expected: "a literal value",` to `expected: "a literal value".into(),`. The `"unexpected end of input, expected a literal value"` display pin stays unchanged: literals render byte-identically.

- [ ] **Step 4: Quote the hint in the two validation branches**

  In `src/query/grammar/filter.rs`, replace the `Self::DateAdd` unit branch:

  ```rust
                } else if args.len() == 3
                    && args.get(2).is_some_and(ValueExpr::is_unrecognized_unit)
                {
                    Some("recognized duration unit")
                } else {
  ```

  with:

  ```rust
                } else if let Some(ValueExpr::Literal(NoteFieldValue::String(unit))) =
                    args.get(2)
                {
                    if let Err(error) = TimeUnit::parse_spelling(unit) {
                        return Err(QuerySyntaxError::unexpected_end(
                            QueryDialect::Filter,
                            input,
                            span,
                            error.to_string(),
                        )
                        .into());
                    }
                    None
                } else {
  ```

  Replace the `Self::DateDiff` unit branch, which has the same shape after `} else if args`, with the same block. The quoted condition appears in both arms. When editing, include the arm's `Self::` pattern line above the snippet.

  The `args.len() == 3` term goes: the first branch already rejects every length outside 2 and 3, so `get(2)` is `Some` only when the length is 3. A valid unit keeps the old path: the `if let` does not fire and the arm yields `None`.

  Delete `is_unrecognized_unit` with its doc comment (`Whether this is a string literal that names no duration unit.`): the two branches were its only callers. `TimeUnit::from_spelling` stays: the evaluators and `parse_spelling` itself still call it.

- [ ] **Step 5: Run the affected suites**

  Run: `mise run test:unit unit_rejections_quote_the_unit_hint`
  Expected: PASS: both cases report `unknown unit "mo3" (expected ...`.

  Run: `mise run test:unit unexpected_end`
  Expected: PASS: the lexer display pins stay byte-identical.

  Run: `mise run test:unit syntax_error_preserves`
  Expected: PASS: the query pin `syntax_error_preserves_dialect_input_span_and_label` stays byte-identical.

  Run: `mise run test:unit date_add`
  Expected: PASS: the engine's `date_add_rejects_unknown_unit` and the filter acceptance tests are untouched.

- [ ] **Step 6: Run the full gate**

  Run: `mise run verify`
  Expected: green (fmt, `check:visibility` included, lint, full test suite).

- [ ] **Step 7: Commit**

  ```
  git add src/lexer.rs src/query/error.rs src/query/grammar/filter.rs
  git commit -m "feat(query): quote the full unit hint in filter unit rejections"
  ```
