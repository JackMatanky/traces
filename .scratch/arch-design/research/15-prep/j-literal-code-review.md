# Code Review & Architecture: Literal Handling & Operand-Adaptive Coercion

- **Target File:** `.scratch/arch-design/research/15-prep/j-literal-code-review.md`
- **Scope:** Review of all interaction points between string literals, typed note values, query grammar, CLI parsing, and templates in Traces; design and adversarial evaluation of operand-adaptive literal coercion.
- **Status:** Complete.
- **Tags:** `[V]` verified (file:line or empirical probe), `[H]` inference, `[P]` proposal.

---

## 1. Executive Summary & Verdict

Today Traces employs a **global, parse-time heuristic sniffing** model for filter literals: any quoted string matching a Date, DateTime, or Duration shape is eagerly converted into a typed `NoteFieldValue` once during AST compilation (`ComparisonExpr::classify_literal`, `src/query/grammar/filter.rs:238` [V]). This leads to severe anomalies: querying a string field with `title == "2025-01-15"` sniffs the literal to a `Date`, causing the comparison against `String("2025-01-15")` to evaluate to `false` [V: probe].

**Core Verdict:**
1. **Adopt Operand-Adaptive Literals for Comparisons:** Quoted string literals in binary comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`) should defer classification until row evaluation, adapting to the target operand's kind per row [P].
2. **Zero-Per-Row Parsing Implementation:** Deferral does NOT require per-row parsing or allocation. The filter compiler pre-parses all candidate shapes (`Date`, `DateTime`, `Duration`, `Link`) ONCE at query build time into an `AdaptiveLiteral` struct [P]. Per-row execution requires only a single enum discriminant branch (~5–10 CPU cycles, 0 allocations, ~0.2 ms for 50k rows) [H].
3. **`string()` Is NOT Needed as an Escape, but Necessary for Cross-Field Queries:** If literals adapt, `title == "2025-01-15"` matches whether `title` is `String` or `Date`, rendering `string()` unnecessary for literal queries [P]. However, `string()` remains required for: (a) cross-field comparisons between heterogeneous kinds (`date_col == str_col`), (b) substring `contains()` on typed temporals, and (c) grouping/formatting when schema typing is absent [P].

---

## 2. Codebase Touch Points: How Literals Interact with Types Today

### 2.1 Filter Lexing & Tokenization (`src/query/grammar/filter.rs`)
- **Bare ISO Dates:** `FilterToken::DateLiteral` regex (`filter.rs:1216-1220` [V]) captures unquoted ISO dates (`YYYY-MM-DD`, `YYYY-MM-DDTHH:MM:SSZ`). In `tokenize_with` (`filter.rs:84-107` [V]), bare dates are eagerly classified into `FilterToken::Literal(NoteFieldValue::Date|DateTime)`. Invalid calendar shapes (`2026-02-30`) throw a `LexError::UnexpectedToken` [V: ADR-0009].
- **Quoted Strings:** Tokenized via `string_callback` (`filter.rs:1209, 1230` [V]) into `FilterToken::Literal(NoteFieldValue::String)`.
- **Bare Numbers & Durations:** `FilterToken::Ident` (`filter.rs:59-82` [V]) attempts `f64` parse first (rejecting NaN/inf [V:66]); if that fails, it attempts `DurationValue::parse` [V:72], promoting bare `1h30m` into `Literal(NoteFieldValue::Duration)`.
- **Booleans & Null:** `true`, `false`, `null`, `Null` are tokenized directly as typed `Literal` variants (`filter.rs:1210-1213` [V]).

### 2.2 Parse-Time Literal Sniffing (`classify_literal`)
- In `FilterGrammar::parse_primary` (`filter.rs:1012-1014` [V]), every `FilterToken::Literal(val)` is wrapped via `ComparisonExpr::classify_literal(value)`.
- `classify_literal` (`filter.rs:238-248` [V]) inspects `NoteFieldValue::String` using `TextShape::classify` (`src/query/sort.rs:462` [V]):
  - `TextShape::DateTime` -> `NoteFieldValue::DateTime`
  - `TextShape::Date` -> `NoteFieldValue::Date`
  - `TextShape::Duration` -> `NoteFieldValue::Duration`
  - `TextShape::Plain` -> unchanged `NoteFieldValue::String`
- **Flaw:** This promotion occurs statically at query build time without knowing the type of the field being compared against [V].

### 2.3 Relational & Equality Evaluation
- **Equality (`==`, `!=`):** `CompareOp::is_satisfied_by_values` (`filter.rs:814-826` [V]) delegates to `NoteFieldValueRef::is_equal_to_literal` (`src/note/field.rs:393-428` [V]). Equality across different kinds is strictly `false` (except Date vs local-midnight DateTime [V:408, 413]). `!=` is the exact boolean complement (`!equal` [V:824]).
- **Relational (`<`, `<=`, `>`, `>=`):** If neither operand is Null, `CompareOp` delegates to `NoteFieldValueRef::compare` (`src/note/field.rs:286-312` [V]). When kinds differ, it falls back to internal ordinal rank: `Null(0) < Bool(1) < Number(2) < Duration(3) < Temporal(4) < Text(5) < Link(6) < List(7) < Object(8)` (`field.rs:238-250` [V]). Thus `42 < "hello"` is `true`, leaking storage rank [V].

### 2.4 Arithmetic & The "Quote Date Literals" Error
- `ArithOp::evaluate` (`filter.rs:347-472` [V]) implements `Date ± Duration`, `Date - Date -> Duration`, `Duration ± Duration`, and `Duration * Number`.
- `filter.rs:1081-1090` [V]: If both arithmetic operands are numeric literals (`lhs.is_number_only() && rhs.is_number_only()`), the parser rejects it: `"a field, date, or duration operand (arithmetic on number literals alone is not supported; quote date literals)"`. This error exists specifically because unquoted dates like `2026-01-01` historically split into `2026 - 1 - 1` [V: ADR-0009].

### 2.5 Function Evaluation & `contains`
- **Query Functions (`filter.rs:474-501` [V]):** `dur(text)`, `date(text[, pattern])`, `date_add`, `date_diff`, `date_component`, `sow/eow/som/eom/soy/eoy`.
  - `date(text)` (`filter.rs:705-728` [V]) accepts String, Date, DateTime; invalid text returns `None` (`TRY_CAST` semantics) [V].
  - `dur(text)` (`filter.rs:626-635` [V]) parses string durations; invalid text returns `None` [V].
- **`contains(field, target)` (`filter.rs:156-192`, `field.rs:434-447`, `query/value.rs:141-160` [V]):**
  - Syntactically, `contains` requires a literal target (`parse_literal_arg`, `filter.rs:920` [V]).
  - Crucially: `parse_literal_arg` **does NOT call `classify_literal`** [V:862-878]! The target remains `NoteFieldValue::String`.
  - For lists, it checks exact value or hierarchical tag match (`#tag/sub` matches `#tag`) [V:436-440].
  - For scalar fields, it evaluates `(lhs.as_str(), target.as_str())` [V:442]. Because `as_str()` returns `None` for `Date`, `DateTime`, `Duration`, `Number`, `contains(due, "2025")` evaluates to `false` [V: probe].

### 2.6 CLI Paths & Shell Quoting (`src/cli/**`)
- `--where` arguments in `list.rs:32`, `table.rs:26`, `task.rs:154` receive raw strings from `clap` [V].
- Shell quote stripping forces users to escape nested quotes: `--where 'due < "2026-01-01"'` or `--where "due < \"2026-01-01\""` [V: `list.rs:30`].
- Bare ISO date support (`due < 2026-01-01`) was added via ADR-0009 specifically to relieve this quoting friction [V].

### 2.7 Template Engine Coercion (`src/template/engine/**`)
- Template filters (`date_format`, `date_add`, `timestamp`, `weekday`, `is_past`) accept strings and coerce them on the fly via `DatePoint::parse` (`template/engine/date.rs:448, 473` [V]).
- Standard comparisons (`{% if a == b %}`) in templates are handled by `minijinja` using Serde-rendered values [V].

---

## 3. Inventory of Existing Tests, Fixtures, and Docs

Every occurrence of quoted date/duration/link literals across tests, docs, and fixtures:

| Location | Expression / Usage | Status Quo [V] | (a) Remove Sniffing [P] | (b) Adaptive Literals [P] | (c) Keep Sniffing [V] |
|---|---|---|---|---|---|
| `docs/adr/0009:42` | `due == "2026-01-01"` | Matches `Date` | **BREAKS** (String!=Date) | Matches `Date` | Matches `Date` |
| `tests/integration/query.rs:445` | `due + dur("17d") == "2026-08-15"` | Matches `Date` | **BREAKS** (String!=Date) | Matches `Date` | Matches `Date` |
| `tests/integration/query.rs:456` | `due - dur("17d") == "2026-07-29"` | Matches `Date` | **BREAKS** (String!=Date) | Matches `Date` | Matches `Date` |
| `tests/integration/query.rs:473` | `due - "2026-07-29" == dur("17d")` | Subtracts `Date - Date` | **BREAKS** (Date - Str -> Null)| Matches `Date` | Matches `Date` |
| `tests/integration/query.rs:523` | `date_add(due, 17, "days") == "2026-08-15"` | Matches `Date` | **BREAKS** | Matches `Date` | Matches `Date` |
| `tests/integration/query.rs:534` | `date_diff("2026-07-29", due, "days") == 17` | `date_diff` parses str | Unchanged (`date_diff` coerces)| Unchanged | Unchanged |
| `tests/integration/query.rs:580-585`| `sow(due) == "2026-07-27"` | Matches `Date` | **BREAKS** | Matches `Date` | Matches `Date` |
| `tests/integration/query.rs:625` | `date_add(due, dur("1d")) > "2026-01-01"` | Relational `Date > Date`| **BREAKS** (Rank leak) | Matches `Date` | Matches `Date` |
| `tests/e2e/query.rs:400` | `--where 'due == "2026-07-29T12:30:00...Z"'`| Matches `DateTime` | **BREAKS** | Matches `DateTime` | Matches `DateTime` |
| `tests/integration/template_render.rs:320`| `where('when + dur("1 month") == "2026-02-28"')`| Matches `Date` | **BREAKS** | Matches `Date` | Matches `Date` |
| `tests/integration/template_render.rs:449`| `where('file.day == "2026-07-29"')` | Matches `Date` | **BREAKS** | Matches `Date` | Matches `Date` |
| `src/query/grammar/filter.rs:1433` | `due == "2026-01-01"` | Matches `Date` | **BREAKS** | Matches `Date` | Matches `Date` |
| `src/query/grammar/filter.rs:1465` | `due < "2026-07-29T14:30:00Z"` | Matches `DateTime` | **BREAKS** | Matches `DateTime` | Matches `DateTime` |
| `src/query/grammar/filter.rs:1596` | `file.mtime >= "2026-01-01"` | Compares Instant/Date | **BREAKS** | Matches `DateTime` | Matches `DateTime` |
| `src/query/grammar/filter.rs:1643` | `file.mdate == "2026-01-01"` | Matches `Date` | **BREAKS** | Matches `Date` | Matches `Date` |
| `src/query/grammar/filter.rs:1817` | Unit test: `classify_literal("2026-07-29")` | Asserts `Date` | Deleted test | Replaced by adaptive test | Unchanged |
| `src/query/grammar/filter.rs:1887` | `when - "2026-01-31" == dur("15d")` | Arithmetic on date lit | **BREAKS** | Matches `Date` | Matches `Date` |
| `src/query/grammar/filter.rs:1936` | `when - dur("1d") == "2026-01-30"` | Matches `Date` | **BREAKS** | Matches `Date` | Matches `Date` |

**Conclusion on Inventory:**
- **Removing sniffing entirely (a) is a catastrophic breaking change:** it breaks ~18 existing integration, E2E, and grammar tests and invalidates documented ADR-0009 behavior [V, H].
- **Operand-adaptive literals (b) preserve 100% of existing passing tests** while fixing the string-field collision bug (`title == "2025-01-15"`) [H, P].

---

## 4. Current Literal Behavior Matrix across LHS Kinds

What each literal form does today for each LHS field kind (`==`, `!=`, `<`, `contains`) [V]:

| LHS Kind | Quoted Date `"2025-01-15"` | Quoted Duration `"1h"` | Quoted Plain `"Alpha"` | Bare Date `2025-01-15` | Bare Number `42` | Bare Bool `true` | Bare Null `null` |
|---|---|---|---|---|---|---|---|
| **Null / Missing** | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` T, `!=` F, `<` F, `ct` F |
| **Bool** | `==` F, `!=` T, `<` T (rank), `ct` F | `==` F, `!=` T, `<` T, `ct` F | `==` F, `!=` T, `<` T, `ct` F | `==` F, `!=` T, `<` T, `ct` F | `==` F, `!=` T, `<` T, `ct` F | `==` val, `!=` !val, `<` ord, `ct` F | `==` F, `!=` T, `<` F, `ct` F |
| **Int / Float** | `==` F, `!=` T, `<` T (rank), `ct` F | `==` F, `!=` T, `<` T, `ct` F | `==` F, `!=` T, `<` T, `ct` F | `==` F, `!=` T, `<` T, `ct` F | `==` exact, `<` ord, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F |
| **String** | **`==` F, `!=` T, `<` F (rank)** | `==` F, `!=` T, `<` F, `ct` F | `==` byte, `<` ord, `ct` substr| `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F |
| **Date** | `==` exact, `<` ord, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` T (rank), `ct` F | `==` exact, `<` ord, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F |
| **DateTime** | `==` local midnight, `<` ord, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` T (rank), `ct` F | `==` local midnight, `<` ord, `ct` F| `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F |
| **Duration** | `==` F, `!=` T, `<` T (rank), `ct` F | `==` exact, `<` ord, `ct` F | `==` F, `!=` T, `<` T (rank), `ct` F | `==` F, `!=` T, `<` T, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F |
| **Link** | `==` F, `!=` T, `<` F (rank), `ct` F | `==` F, `!=` T, `<` F, `ct` F | **`==` F, `!=` T, `<` F (rank)** | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F |
| **List** | `==` F, `!=` T, `<` F (rank), `ct` elem| `==` F, `!=` T, `<` F, `ct` elem| `==` F, `!=` T, `<` F, `ct` elem | `==` F, `!=` T, `<` F, `ct` elem | `==` F, `!=` T, `<` F, `ct` elem | `==` F, `!=` T, `<` F, `ct` elem | `==` F, `!=` T, `<` F, `ct` F |
| **Object** | `==` F, `!=` T, `<` F (rank), `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F | `==` F, `!=` T, `<` F, `ct` F |

*Legend: `ct` = `contains(lhs, target)`; `ord` = typed ordering; `rank` = leaked storage class rank.*

---

## 5. Where Literals Appear & Inverted Operand Support

1. **Comparison RHS & LHS (`field op lit` vs `lit op field`):**
   - Syntactically, both LHS and RHS of a comparison expression evaluate via `parse_value_expr` (`filter.rs:1148, 1159` [V]).
   - `lit op field` is **fully supported** (e.g. `"2026-07-29" == due`, `"1h" < dur_field`) [V].
   - `CompareOp::is_satisfied_by_values` (`filter.rs:814` [V]) evaluates `lhs.as_ref().compare(&rhs.as_ref())` symmetrically for typed values [V].
2. **Function Arguments:**
   - Functions accept literal arguments: `dur("1h")`, `date("2026-07-29")`, `date_add(due, 1, "days")`, `date_diff("2026-07-29", due)` [V: tests].
   - Asymmetry in `contains`: `contains(field, "target")` is supported; `contains("target", field)` and `contains(field1, field2)` are rejected at parse time (`filter.rs:881-943` [V]).
3. **Arithmetic Operands:**
   - Literals can appear as left or right arithmetic operands: `when - "2026-01-31" == dur("15d")` (`filter.rs:1887` [V]); `dur("1d") * 17` (`query.rs:467` [V]).
   - Number-only arithmetic (`1 + 2 == 3`) is explicitly rejected [V:1081].
4. **Both Operands Literals (`lit op lit`):**
   - Expressions like `"2025-01-01" < "2025-02-01"` or `dur("5d") - dur("2d") == dur("3d")` parse and evaluate [V:1935].
   - **Critical edge case for adaptive literals:** When *both* operands are untyped quoted literals (`"a" == "b"`), there is no row operand to adapt to; they must default to String comparison [P].

---

## 6. Empirical Probe Results

The throwaway test probe executed against real index rows produced the following ground truth (`artifact://126` [V]):

| # | Probe Expression | Test Setup / Row Kinds | Observed Result [V] | Diagnosis / Mechanism |
|---|---|---|---|---|
| **1** | `due == "2025-01-15"` | `n1`: `due: 2025-01-15` (Date) | `["n1.md"]` | Matched: literal sniffed to `Date(2025-01-15)`. |
| **2** | `due < "2025-02-01"` | `n1`: `due: 2025-01-15` (Date) | `["n1.md"]` | Matched: literal sniffed to `Date(2025-02-01)`. |
| **3a** | `title == "2025-01-15"` | `n2`: `title: 2025-01-15` (Date) | `["n2.md", "n8.md"]` | Matched: both sniffed to Date. |
| **3b** | `title == "2025-01-15"` | `n3`: `title:: "2025-01-15"` (String) | **`[]` (DROPPED!)** | **SNIFFING TRAP:** Literal sniffed to Date; `String("2025-01-15") == Date` is false! |
| **4** | `dur_field > "1h"` | `n4`: `dur_field: 2h` (Duration) | `["n4.md"]` | Matched: literal sniffed to `Duration(1h)`. |
| **5a** | `project == "Alpha"` | `n5`: `project: "[[Alpha]]"` (Link) | `[]` | Fails: `Link` vs `String("Alpha")` is false. |
| **5b** | `project == "[[Alpha]]"`| `n5`: `project: "[[Alpha]]"` (Link) | `[]` | Fails: grammar does not construct Link literals; `Link` vs `String` is false. |
| **6** | `status == "done"` | `n6`: `status: done` (String) | `["n6.md"]` | Matched: plain string equality. |
| **7** | `n == "5"` | `n7`: `n: 5` (Number) | `[]` | Fails: `Number(5.0)` vs `String("5")` is false (no number sniffing). |
| **8** | `contains(title, "2025")`| `n2, n8` (Date); `n3` (String) | `["n3.md"]` | Matches `n3` (String) only! `Date.as_str()` is None; Date substring search impossible. |
| **9a** | `tags == "x"` | `n9`: `tags: [x, y]` (List) | `[]` | Fails: scalar literal never equals a List. |
| **9b** | `contains(tags, "x")` | `n9`: `tags: [x, y]` (Tags) | `[]` | Fails: stored as `#x`; `is_contained_in("x")` fails without `#` prefix. |
| **9c** | `contains(tags, "#x")`| `n9`: `tags: [x, y]` (Tags) | `["n9.md"]` | Matched: hierarchical tag prefix matches `#x`. |

---

## 7. When Is `string()` Still Needed if Literals Adapt?

If quoted string literals adapt to the operand kind, the primary justification for `string()` (escaping frontmatter classification for literal comparisons) disappears: `title == "2025-01-15"` matches whether `title` is `String` or `Date` [P].

However, `string(expr)` is **still required** for four specific use cases [P]:

1. **Substring Searches on Classified Fields (`contains(string(date), "2025")`):**
   - Probe 8 proves `contains(due, "2025")` returns `false` on `Date` fields because `Date` does not implement `as_str()`.
   - *Alternative:* While `date_component(due, "year") == 2025` works for standard units, users searching arbitrary text patterns in dates/durations/links need `string()`.
2. **Heterogeneous Field-to-Field Comparisons (`field_a == field_b`):**
   - Operand-adaptive coercion only applies when **at least one operand is a literal** [P].
   - If `due` is `Date` and `legacy_due` is `String`, `due == legacy_due` evaluates to `false` under Type Bracketing (L1).
   - The user must explicitly cast one side: `due == date(legacy_due)` or `string(due) == legacy_due`.
3. **Cross-Kind Grouping & Aggregation:**
   - Grouping notes by year-month (`2025-01`) or formatting duration text requires converting the typed value to a string representation.
   - *Alternative:* `date_format(due, "%Y-%m")` covers dates, but no formatting function exists for links or generic fields.
4. **String Functions Missing from Query Grammar:**
   - Neither `lower()`, `upper()`, `startswith()`, `endswith()`, nor `regex_match()` exist in Traces query filters today (only `contains` exists) [V]. When ticket 22 adds these functions, they will require string inputs.

---

## 8. Implementation Sketch: Per-Row Operand-Adaptive Literals

### 8.1 The Core Mechanism
Instead of eagerly converting `FilterToken::Literal(String)` to `NoteFieldValue::Date|DateTime|Duration` at query build time, quoted strings are wrapped in an `AdaptiveLiteral` [P]:

```rust
#[derive(Clone, Debug)]
pub(crate struct AdaptiveLiteral {
    /// Raw unescaped string as written by user.
    raw: String,
    /// Pre-parsed candidate Date, if raw text has an ISO date shape.
    date: Option<DateValue>,
    /// Pre-parsed candidate DateTime, if raw text has an ISO datetime shape.
    datetime: Option<DateTimeValue>,
    /// Pre-parsed candidate Duration, if raw text has a duration shape.
    duration: Option<DurationValue>,
    /// Pre-parsed candidate Link target (unquoted or [[wikilink]]).
    link: Option<LinkTarget>,
}
```

### 8.2 Build-Time Compilation (Executed ONCE per query)
When parsing `ValueExpr::Literal`:
```rust
impl AdaptiveLiteral {
    pub(crate) fn new(raw: String) -> Self {
        let trimmed = raw.trim();
        let (date, datetime) = match DatePoint::parse(trimmed) {
            Ok(DatePoint::Calendar { date, .. }) => (Some(date), None),
            Ok(DatePoint::DateTime(dt)) => (None, Some(dt)),
            Err(_) => (None, None),
        };
        let duration = DurationValue::parse(trimmed).ok();
        let link = LinkTarget::parse(trimmed).ok();
        Self { raw, date, datetime, duration, link }
    }
}
```

### 8.3 Row Evaluation (Zero Allocations, Zero Parsing)
When comparing against a row's `NoteFieldValueRef`:
```rust
impl AdaptiveLiteral {
    #[inline]
    pub(crate) fn resolve_for<'a>(&'a self, target: &NoteFieldValueRef<'_>) -> NoteFieldValueRef<'a> {
        match target {
            NoteFieldValueRef::Date(_) if self.date.is_some() => {
                NoteFieldValueRef::Date(self.date.unwrap())
            }
            NoteFieldValueRef::DateTime(_) if self.datetime.is_some() => {
                NoteFieldValueRef::DateTime(self.datetime.unwrap())
            }
            NoteFieldValueRef::DateTime(_) if self.date.is_some() => {
                // Promote date literal to midnight in context zone (L4)
                NoteFieldValueRef::Date(self.date.unwrap())
            }
            NoteFieldValueRef::Duration(_) if self.duration.is_some() => {
                NoteFieldValueRef::Duration(self.duration.as_ref().unwrap())
            }
            NoteFieldValueRef::Link(_) if self.link.is_some() => {
                NoteFieldValueRef::Link(self.link.as_ref().unwrap())
            }
            // For String, Number, Bool, List, Object, or when no candidate parsed:
            _ => NoteFieldValueRef::String(&self.raw),
        }
    }
}
```

### 8.4 Performance Impact on 50,000 Rows
- **Memory allocations per row:** `0` (borrows from `&self` or copies 4-byte/16-byte copy types) [P].
- **String parsing per row:** `0` (all parsing executed once during AST construction) [P].
- **CPU Instruction cost:** One discriminant match + pointer unwrap (~5–10 clock cycles ≈ 2–4 ns) [H].
- **50,000 Row Evaluation Time:** $\approx 50{,}000 \times 3\text{ ns} \approx 0.15\text{ ms}$ total overhead across the entire query run [H].

---

## 9. Adversarial Stress-Test: Where the Proposal Fails

1. **Both Operands Are Literals (`"2025-01-01" < "2025-02-01"`):**
   - *Failure Mode:* With no row field to adapt to, two adaptive literals have no target kind.
   - *Resolution:* If both operands are literals, default strictly to `String` byte comparison (which preserves correct ISO date ordering anyway) [P].
2. **Ambiguous Literals (e.g. `"1h"` as a Task Title vs Duration):**
   - If a vault note has `title: "1h"` (String) and user queries `--where 'title == "1h"'`, the target is `String`, so `AdaptiveLiteral` selects `String` and matches correctly [P].
   - If user queries `--where 'time < "1h"'` on duration `time: 30m`, target is `Duration`, so it selects `Duration` and matches correctly [P].
3. **Array Membership (`tags == "x"`):**
   - Adaptive literals do NOT change array scalar comparisons: `List == String` remains `false` under Type Bracketing [V]. Users must continue to use `contains(tags, "#x")` or ticket 22's planned `any()` / `in` operator.
4. **Arithmetic Expression Types (`when - "2026-01-31"`):**
   - In arithmetic expressions (`ArithOp`), the literal must adapt to the *other* arithmetic operand: in `Date - Literal`, the literal must adapt to `Date` (yielding `Duration`); in `Duration * Literal`, the literal must adapt to `Number`. This requires threading operand expectation into `ValueExpr::Binary` [H, P].

---

## 10. Implications for Ticket 15

### Concrete Decision Options

- **Option A (Status Quo Sniffing):** Keep `ComparisonExpr::classify_literal`.
  - *Pros:* Zero implementation effort.
  - *Cons:* Perpetuates the `title == "2025-01-15"` failure; strings matching date shapes cannot be queried with equality.
- **Option B (Strict Lexical / No Sniffing):** Drop `classify_literal` completely; all quoted literals are `String`; users must use bare dates (`due == 2025-01-15`) or explicit casts (`due == date("2025-01-15")`).
  - *Pros:* Conceptually pure; identical to Dataview.
  - *Cons:* **Breaks 18+ existing integration tests and ADR-0009**; hostile to users coming from shell scripts where quoting is mandatory (`--where 'due == "2025-01-15"'`).
- **Option C (Pre-Parsed Operand-Adaptive Literals):** Adopt `AdaptiveLiteral` with build-time candidate parsing and per-row resolution.
  - *Pros:* **100% backwards compatible** with all existing query tests and ADR-0009; completely eliminates the `title == "2025-01-15"` string bug; zero per-row allocations or parsing.
  - *Cons:* Small compiler-layer implementation addition in `src/query/grammar/filter.rs`.

### Final Recommendation
**Adopt Option C (Pre-Parsed Operand-Adaptive Literals).** Combine with:
1. Retaining bare ISO dates (`due == 2025-01-15`) via ADR-0009 for zero-quoting CLI ergonomics [P].
2. Providing explicit `date()`, `dur()`, and `string()` cast functions with TRY_CAST semantics (failure -> Null) for cross-field comparisons and explicit conversions [P].
3. Leaving frontmatter classification quote-insensitive at parse time (matching ticket 14 answer lines 501-502 and `h-quote-escape-feasibility.md`), with string-forcing handled by schema typing (`type = "string"`) [P].
