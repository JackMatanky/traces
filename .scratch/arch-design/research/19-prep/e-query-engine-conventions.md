# Query Engine Conventions & Coercion Alignment

- **Topic:** Cross-engine survey of typing, comparison, sorting, and coercion conventions compared to Traces.
- **Engines:** PostgreSQL, SQLite, DuckDB, MySQL, MongoDB, Elasticsearch, jq, CEL, JMESPath, Dataview, Nushell, Polars/pandas.
- **Status:** Complete. Target file: `.scratch/arch-design/research/19-prep/e-query-engine-conventions.md`.
- **Tags:** `[V]` verified (file:line, official doc URL, or live engine probe), `[H]` inference, `[P]` proposal.

---

## 1. Comparative Alignment Matrix

Columns: **PG** (PostgreSQL 16), **LT** (SQLite 3.45), **DK** (DuckDB 1.1), **MY** (MySQL 8.0), **MG** (MongoDB 7.0 find / $expr), **ES** (Elasticsearch 8), **JQ** (jq 1.7), **CL** (CEL v0.18), **JM** (JMESPath), **DV** (Dataview 0.5), **NU** (Nushell 0.108), **PL/PD** (Polars 1.24 / pandas 2.2), **TR-Now** (Traces today), **TR-Lean** (Leans L1–L8).

| # | Behavior / Expression | PG | LT | DK | MY | MG (find / $expr) | ES | JQ | CL | JM | DV | NU | PL/PD | TR-Now [V] | TR-Lean [P] |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `num == 'a'` | Error | False | Error | Coerce (0==0->T) | False / False | Error | False | Error/False | False | False | False | Err/False | False [V:160] | False (L1) |
| 2 | `num < 'a'` | Error | True (num<txt) | Error | Coerce (num<0) | NoMatch / True | Error | True | Error | Null | True | Error | Err/Err | True (rank) [V:162] | NoMatch (L1) |
| 3 | `x != 'a'` (x is NULL) | NULL (falsy) | NULL (falsy) | NULL (falsy) | NULL (falsy) | Match (incl missing) | Match (must_not) | True | True | True | True | True | None/True | True [V:161] | True (L1) |
| 4 | `x == null` (missing vs null) | N/A (no missing) | N/A (no missing) | N/A (no missing) | N/A (no missing) | Matches both | N/A (term missing) | True (missing->null)| True | True | Matches both | True | None/False | Matches both [V:169]| Matches both (L1)|
| 5 | ORDER BY NULL & `x < null` | NULLS LAST (ASC) | NULLS FIRST | NULLS LAST | NULLS FIRST | MinKey/Null lowest | `_last` default | Null lowest | Error | Null lowest | Nulls first | Nulls first | PL:first, PD:last | First / poisoned [V]| First / no-match (L1)|
| 6 | Excluded Middle: `NOT(x<5)` vs `x>=5` | Broken (3VL) | Broken (3VL) | Broken (3VL) | Broken (3VL) | Broken (bracket) | Broken | Preserved | Broken (error)| Broken (null) | Preserved | Broken (null) | Broken | Broken (rank) | Broken (L1) |
| 7 | `int == float` (1 == 1.0) | True | True (exact) | True (exact) | True | True | True | True | True (exact) | True | True | True | True | N/A (f64 only) [V:70]| True (exact) (L3) |
| 8 | NaN / Infinity support | NaN=NaN, sort max | NaN->NULL [V] | NaN=NaN, sort max| Error/NULL | Double supports | Rejects NaN | NaN!=NaN | NaN!=NaN (IEEE)| N/A | NaN!=NaN | NaN!=NaN, sort max| NaN!=NaN | compare() eq [V:146] | NaN->Null (L3) |
| 9 | Date vs Timestamp (zone) | Promotes midnt; tz | Lexical | Promotes midnt; tz | Promotes midnt | UTC millis only | UTC millis | N/A | Sep types | N/A | Luxon local ms | Always tz | Err naive vs tz | Ambient shift [V:82]| Civil/inst context (L4)|
| 10| `1mo == 30d` interval | False (ord True) | N/A | True [V] | N/A | N/A | N/A | N/A | Secs only | N/A | True (norm ms) | Reject mo/yr | Fixed units | True (nominal) [V:153]| True (nominal) (L7) |
| 11| String collation default | Collation/ICU | BINARY (memcmp) | BINARY (memcmp) | utf8mb4_0900_ai_ci | Binary (opt ICU) | Keyword binary | Codepoint | Byte-exact | Codepoint | JS locale/str | Unicode | UTF-8 byte | Byte-exact [V:122] | Unicode sort (L8) |
| 12| Array `tags == 'rust'` | Error (need ANY) | Error (need json) | Error (need list) | Error (MEMBER OF) | Any-match elem [V]| Any-match elem | False (need .[]) | Error (`in`) | False (contains)| Any-match elem | False (`in`) | Error (contains) | False (contains) [V]| Separate fn (L1) |
| 13| `"" == NULL` (empty conflation) | False (distinct) | False (distinct) | False (distinct) | False (distinct) | False (distinct) | False (distinct) | False | False | False | Frontmatter null | False | False | Coerced to Null [V:58]| Null at parse (L1) |
| 14| Date literal string coercion | Untyped `unknown` | Affinity | Coerced to col | Coerced to col | No coercion | Coerced in range | N/A | No coercion | N/A | Strict `date(...)` | Typed literal | Typed parse | Sniffed literal [V:59]| Sniffed or typed (L1)|
| 15| Heterogeneous GROUP BY (1 & 1.0)| Grouped | Grouped | Grouped | Grouped | Grouped | Grouped | Grouped | N/A | Grouped | Grouped | Grouped | Type-bound | N/A (f64 only) | Grouped (L3) |

---

## 2. Row-by-Row Deep Dive & Precedent Analysis

### Row 1 & 2: Cross-Type Equality & Relational Ordering (`num == 'a'`, `num < 'a'`)

- **SQL (Postgres, DuckDB, Polars):** Strongly typed engines reject cross-type comparisons with an error (`Conversion Error: Could not convert string 'a' to INT32` [V: DuckDB probe; PG: `42.8.2`]).
- **SQLite:** Dynamic typing with storage class ordering (`NULL < INTEGER/REAL < TEXT < BLOB`). `42 == 'a'` is 0; `42 < 'a'` is 1 (`sqlite3.c:110243` [V: https://www.sqlite.org/datatype3.html]).
- **MongoDB:** Dual regime:
  1. *Query Predicates (find):* Applies **Type Bracketing** (`mql/query-predicates/type-bracketing/` [V]). `{ qty: { $gt: 4 } }` matches only numeric fields; documents where `qty` is string or object do NOT match. `{ a: { $eq: "42" } }` on `42` is false.
  2. *Aggregation / $expr / Sort:* Applies total BSON comparison order (`MinKey < Null < Numbers < Strings < Object < Array < ... < MaxKey` [V: `bson-type-comparison-order/`]).
- **jq & Dataview:** Implement total ordering across types. In jq, `numbers < strings`, so `42 < "a"` is `true` [V: `jq manual`]. In Dataview, `Values.compareValue` ranks types so `42 < "a"` is `true` [V: `reference/expressions/`].
- **Traces Assessment:** Today Traces is in an incoherent hybrid state: `==` is strictly typed (`false`), but `<` leaks the internal rank table (`42 < "a"` is `true` [V: `src/query/grammar/filter.rs:807`, `a-code-inventory:162`]). Lean L1 eliminates the leak by adopting MongoDB-style **Type Bracketing** for filters (`<` across brackets evaluates to NoMatch/false), while reserving the rank table strictly for `SortKey`.

### Row 3 & 4: Null, Missing Fields, and Three-Valued Logic (3VL)

- **SQL (Postgres, DuckDB, SQLite, MySQL):** Strict 3-valued logic. `NULL != 'a'`, `NULL = NULL`, and `NULL < 5` all evaluate to `NULL`, which is treated as falsy in `WHERE`. SQL requires explicit null-safe constructs (`IS NULL`, `IS DISTINCT FROM`).
- **Document / PKM (MongoDB, Dataview, jq, Nushell):** 2-valued complement logic.
  - In MongoDB find, `{ x: { $ne: 'a' } }` matches documents where `x` is null, missing, or any non-'a' type [V: `type-bracketing.md`].
  - In jq, `null != 'a'` is `true`, `null == null` is `true` [V: live probe].
  - In Dataview, missing fields yield `null`, and `null != "a"` is `true`.
- **Traces Assessment:** Traces has always used 2-valued complement equality (`!=` is the exact boolean inverse of `==` [V: `src/query/grammar/filter.rs:792`]). In open-world markdown vaults, 3VL would cause severe user friction: `--filter 'status != "done"'` would drop every note that omitted the `status` field! Keeping 2-valued complement logic aligns Traces firmly with MongoDB, Dataview, jq, and Nushell.

### Row 5: Null Ordering in ORDER BY

- **SQL Engines:** Split defaults:
  - PostgreSQL & DuckDB: `NULLS LAST` for ASC by default [V: `duckdb.org/docs/sql/query_syntax/orderby`].
  - SQLite & MySQL: `NULLS FIRST` for ASC (null is treated as the lowest value) [V: `sqlite.org/datatype3.html`].
- **Document / Shell:** MongoDB sorts `null` and missing fields lowest [V: `bson-type-comparison-order`]. Dataview sorts `null` lowest [V: `reference/expressions`].
- **Traces Assessment:** Traces sorts `Null` lowest (`Null < Bool < Number...` [V: `src/query/sort.rs:428`]). Relational operators in filters poison Null (always false [V: `src/query/grammar/filter.rs:802`]). Lean L1 keeps this cleanly: Null sorts lowest, but never matches relational filter predicates.

### Row 6: Law of Excluded Middle (`NOT (x < 5)` vs `x >= 5`)

- In SQL 3VL, the law is broken because `NULL` yields `NULL` for both.
- In MongoDB find queries, **Type Bracketing breaks the law across types**: if `x` is `"hello"`, `{ x: { $lt: 5 } }` is false, `{ x: { $gte: 5 } }` is false, but `{ x: { $not: { $lt: 5 } } }` is TRUE [V: `type-bracketing.md`].
- Under Traces Lean L1, Traces **matches MongoDB**: if `due` is string `"tomorrow"`, `due < 2025-01-01` is false, and `due >= 2025-01-01` is false (cross-bracket relational no-match). Therefore `NOT (due < 2025-01-01)` evaluates to `NOT false` = `true`. This asymmetry is an inevitable consequence of type bracketing in a 2-valued filter engine.

### Row 7 & 8: Numbers, Floats, and NaN / Infinity

- **Int vs Float:** PostgreSQL, SQLite, DuckDB, CEL, MongoDB, and Polars all equate exact integer and float values (`1 == 1.0` is true [V: live DuckDB/SQLite/Polars probes]). SQLite uses `sqlite3IntFloatCompare` to compare 64-bit ints and doubles without loss of precision [V: `sqlite3.c:110052`]. CEL specifies heterogeneous numeric equality across `int`, `uint`, and `double` [V: `cel-spec/doc/langdef.md`].
- **NaN Handling:**
  - PostgreSQL & DuckDB treat `NaN = NaN` as `true` and sort NaN higher than all other numbers to allow B-tree indexing [V: PG doc `8.1.3`, live DuckDB probe].
  - SQLite **converts NaN to NULL upon storage** [V: `https://www.sqlite.org/floatingpoint.html`].
  - IEEE 754 engines (CEL, JS/Dataview, pandas) treat `NaN == NaN` as false.
- **Traces Assessment:** Lean L3 proposes exact integer/float comparison outside derived `PartialEq`, adopting SQLite's exact numeric comparison. Rejecting NaN to `Null` at ingestion mirrors SQLite's battle-tested policy and removes IEEE non-reflexivity from queries.

### Row 9 & 10: Temporals and Nominal Duration Equivalence

- **Date vs DateTime / Zones:**
  - PostgreSQL and DuckDB promote `date` to `timestamp` at `00:00:00`. When comparing naive `timestamp` with `timestamptz`, both engines interpret the naive timestamp in the session's local `TimeZone` [V: PG doc `9.9.2`].
  - Polars strictly forbids comparing naive and timezone-aware datetimes (`ComputeError`) [V: Polars doc].
  - Traces today bakes the indexing machine's ambient zone into the postcard cache (`...Z`) [V: `src/temporal/datetime.rs:370`]. Lean L4's civil model stores naive datetimes without a zone and resolves cross civil-vs-instant comparisons using the execution context's zone, matching PostgreSQL/DuckDB session semantics.
- **Interval Equivalence (`1mo == 30d`):**
  - PostgreSQL stores months, days, and microseconds separately: `interval '1 month' = interval '30 days'` is `false` for equality, though `ORDER BY` compares them nominally [V: PG doc `9.9.5`].
  - DuckDB normalizes intervals to nominal microseconds: `INTERVAL 1 MONTH = INTERVAL 30 DAYS` is `true` for both equality and ordering [V: live DuckDB probe].
  - Dataview normalizes durations to nominal milliseconds: `dur(1 mo) == dur(30 d)` is `true` [V: `reference/expressions/`].
  - Nushell and CEL reject variable calendar units (month, year) from duration types entirely [V: Nushell doc `duration.html`].
  - Traces Lean L7 aligns with DuckDB and Dataview: nominal seconds (`1mo == 30d`, `1y == 365d`) for comparison and sorting, while calendar arithmetic uses Jiff spans.

### Row 11: String Collation

- PostgreSQL defaults to database collation (libc or ICU). SQLite and DuckDB default to byte-for-byte `memcmp` (binary) [V: `sqlite.org/datatype3.html`]. MySQL defaults to `utf8mb4_0900_ai_ci` (Unicode UCA, case/accent-insensitive).
- Lean L8 adopts Unicode-based sorting (NFC normalization, case-folded primary sort, byte tie-break) over raw byte order, matching desktop expectations (Nushell, Dataview) while avoiding host libc dependencies.

### Row 12 & 13: Array Semantics and Empty String Conflation

- **Multi-Valued Arrays:** MongoDB and Elasticsearch automatically treat `field = scalar` as an any-element match (`{ tags: "rust" }` matches `["rust", "db"]` [V: Mongo doc `type-bracketing.md`]). SQL engines (Postgres, DuckDB) reject scalar equality and require `scalar = ANY(tags)` or `contains` [V]. Dataview supports both auto-unwrapping and `contains()`. Traces uses a dedicated `contains` operator/function [V: `src/note/field.rs:434`], avoiding ambiguity between whole-array equality and element membership.
- **Empty String vs NULL:** Oracle SQL conflates empty string with NULL. PostgreSQL, SQLite, DuckDB, MySQL, MongoDB, and jq strictly distinguish them. Traces currently coerces empty frontmatter strings to `NoteFieldValue::Null` at parse [V: `src/note/field.rs:551`]. This Oracle-style conflation is justified in markdown frontmatter where `key:` and `key: ""` both signify "no value".

### Row 14 & 15: Literal Sniffing and GROUP BY

- **Literal Sniffing:** Relational databases resolve untyped string literals based on the target column schema (e.g., PostgreSQL `unknown` literal resolution [V: PG doc `10.1`]). Traces lacks a mandatory schema, so it sniffs quoted string literals in filters (`filter.rs:238` [V]).
- **GROUP BY:** In PostgreSQL, DuckDB, SQLite, and MongoDB, `1` and `1.0` group into the same bucket [V: live DuckDB probe]. Traces Lean L3 ensures `Int(1)` and `Float(1.0)` group together by evaluating equality via the semantic comparator.

---

## 3. Where Traces Aligns, Diverges, and Is Unusual

### Ranking Divergences by Surprise to a SQL-Literate User

1. **Rank 1 (High Surprise): 2-Valued Complement Logic on `!=` (`NULL != 'a'` is True)**
   - *SQL Expectation:* Any comparison involving NULL evaluates to NULL (3VL falsy in WHERE). `col != 'a'` excludes NULL rows unless `OR col IS NULL` is written.
   - *Traces Behavior:* `!=` is the exact boolean complement of `==`. Missing fields and NULL values return `true` for `!= 'a'` [V: `a-code-inventory:161`].
   - *Precedent & Defensibility:* High precedent in document and CLI engines. MongoDB `{ x: { $ne: 'a' } }`, jq, Nushell, and Dataview all treat `!=` as 2-valued complement. Indispensable for open-world PKM vaults.
2. **Rank 2 (High Surprise): Type-Bracketed Relational Operators (`x < 5` is NoMatch, but `NOT(x < 5)` is True)**
   - *SQL Expectation:* Cross-type comparison either throws a type error (Postgres, DuckDB) or coerces (MySQL).
   - *Traces Behavior (L1):* Relational operators across different brackets never match (evaluate to false). Negation inverts this to true, breaking the Law of Excluded Middle across types.
   - *Precedent & Defensibility:* Directly mirrors MongoDB's query predicate Type Bracketing [V: `mongodb.com/docs/manual/reference/mql/query-predicates/type-bracketing/`]. Prevents a single note with `due: "tbd"` from aborting an entire vault query.
3. **Rank 3 (Medium Surprise): Query Literal Sniffing (Quoted `"2025-01-15"` promoted to Date)**
   - *SQL Expectation:* Literals are typed by schema column affinity; in Dataview/CEL, explicit `date(...)` syntax is required.
   - *Traces Behavior:* Filter parser classifies date/duration-shaped quoted strings into typed values [V: `src/query/grammar/filter.rs:238`].
   - *Precedent & Defensibility:* Common in search systems (Elasticsearch range queries parse ISO date strings automatically). Defensible for zero-config CLI ergonomics, but causes edge-case collisions (e.g. title `"2025-01-15"`).
4. **Rank 4 (Medium Surprise): Empty String Coerced to NULL at Ingestion**
   - *SQL Expectation:* Empty string `""` is a distinct 0-length string value (except in Oracle).
   - *Traces Behavior:* Blank/whitespace-only frontmatter strings silently coerce to `NoteFieldValue::Null` [V: `src/note/field.rs:551`].
   - *Precedent & Defensibility:* Precedent in Oracle and YAML parser conventions (bare `key:` in YAML is null). Highly practical in markdown vaults where users write `due: ""` to clear a template field.
5. **Rank 5 (Low Surprise): Nominal Duration Equivalence (`1mo == 30d`)**
   - *SQL Expectation:* Mixed; PostgreSQL rejects interval equality across units, but DuckDB accepts `INTERVAL 1 MONTH = INTERVAL 30 DAYS` [V: live DuckDB probe].
   - *Precedent & Defensibility:* Strong precedent in DuckDB and Dataview. Essential because PKM queries without an anchor date cannot perform calendar duration comparisons.

---

## 4. Adversarial Stress-Tests: Where the Leans Fail

### Stress-Test 1: Filter Type Bracketing (L1) Breaks Law of Excluded Middle

- *Scenario:* User queries notes with `--filter 'due >= 2025-01-01'` vs `--filter 'NOT (due < 2025-01-01)'`.
- *Failure Mode:* Note A has `due: "someday"` (String). Note B has `due: null` (Null).
  - Under `due >= 2025-01-01`: Note A is NoMatch (false). Note B is NoMatch (false).
  - Under `NOT (due < 2025-01-01)`: Note A evaluates `NOT (false)` -> **MATCHES**! Note B evaluates `NOT (false)` -> **MATCHES**!
- *Verdict:* The complement of a bracketed comparison includes ALL out-of-bracket values and NULLs. SQL users expect `NOT (a < b)` to equal `a >= b`. In Traces, this identity fails [H].

### Stress-Test 2: Frontmatter Quoted String Escape (L5) vs Noyalib 0.0.53 Constraints

- *Scenario:* User writes `title: "2025-01-15"` in frontmatter expecting it to remain a string.
- *Failure Mode:* In Noyalib 0.0.53, `ScalarStyle` is private (`pub(crate)` in `scanner.rs:21` [V: `brief.md:41`]). Both `title: "2025-01-15"` and `title: 2025-01-15` produce identical `noyalib::Value::String`. Ticket 18 demonstrated that Noyalib drops YAML tags (`!!str`) to plain values and `span_at` is unsound for complex keys [V: `18:468-470`].
- *Verdict:* Lean L5 is logically sound but **mechanically blocked** without either a custom frontmatter parser pass, a lazy CST consult, or schema-driven field typing.

### Stress-Test 3: Date vs DateTime Midnight Instant in Civil Model (L4)

- *Scenario:* Note has `time: 2026-07-29T00:00:00` (naive midnight). Query filters by `date == 2026-07-29`.
- *Failure Mode:* If `time` is civil and `date` is civil midnight, they match. But if the note specifies `time: 2026-07-29T00:00:00Z` (instant) and query runs in a timezone at UTC+2, local midnight is `2026-07-28T22:00:00Z`. The calendar date no longer contains the instant!
- *Verdict:* Cross-kind equality between `Date` and `DateTime` depends on whether the datetime is civil or instant, and requires defining whether `date == instant` means "instant falls within the 24 hours of that calendar day in context zone" or "exact instant equals context local midnight" [P].

### Stress-Test 4: Link Equality Ignoring Display Text (L6)

- *Scenario:* User has notes linking `[[Alice|Dr. Smith]]` and `[[Alice|CEO]]`.
- *Failure Mode:* If equality checks only canonical target, `--filter 'link == "[[Alice|Dr. Smith]]"'` matches `[[Alice|CEO]]`.
- *Verdict:* Acceptable if display is exposed as a queryable sub-attribute (`link.display`), but surprising if the query literal explicitly included an alias.

---

## 5. Implications for Ticket 19

Mapping findings to leans L1–L8:

- **L1 (Type Bracketing in Filters, Rank Table in Sort): KEEP, with formal specification of Excluded Middle.**
  - *Refinement:* Adopt MongoDB find query type bracketing (`Numeric`, `Temporal`, `Duration`, `String`, `Bool`, `Link`, `List`, `Object`, `Null`). Relational ops (`<, <=, >, >=`) across brackets evaluate to `false`. Explicitly document that `NOT (x < 5)` matches out-of-bracket kinds and nulls, mirroring MongoDB `{ $not: { $lt: 5 } }`.
- **L2 (Delete Sort-Time Sniffing): KEEP unchanged.**
  - *Refinement:* Delete `TextShape::classify` in `SortKey::from_value_ref`. Every surveyed production engine sorts by stored type; sniffing during sort causes non-transitive sorting bugs and ruins string index predictability.
- **L3 (Exact Int/Float Comparator Outside `PartialEq`, NaN -> Null): KEEP unchanged.**
  - *Refinement:* Adopt SQLite's `sqlite3IntFloatCompare` algorithm. Derived `PartialEq` remains structural. Ingest-time conversion of `NaN` to `NoteFieldValue::Null` mirrors SQLite floating-point storage, eliminating IEEE 754 non-reflexive equality hazards in sorting and grouping.
- **L4 (Civil Model for Naive Datetimes, Context Zone for Instants): KEEP, with calendar containment rule.**
  - *Refinement:* Naive datetimes are stored as civil (no zone shift in postcard cache). When comparing `Date` to an instant `DateTime`, the comparison must evaluate via the query execution context's explicit timezone. Define `Date == DateTime` strictly as: instant falls within the `[start_of_day, end_of_day)` interval of the Date in the context timezone.
- **L5 (Quoted Frontmatter Scalars Stay Strings): REFINE mechanism requirement.**
  - *Refinement:* Lean L5 cannot be achieved via Noyalib 0.0.53 AST alone. Mandate one of two concrete implementation paths: (a) evaluate the lazy CST consult for date/duration-shaped plain strings, or (b) defer quote-preservation to Ticket 20 schema typing (`type = "string"`), leaving frontmatter sniffing quote-insensitive in the zero-config fallback.
- **L6 (Link Equality on Folded Target + Subpath Only): KEEP unchanged.**
  - *Refinement:* Canonical equality and ordering compare folded `target` and `subpath` only (reusing `ComparisonKey`). `display` and `embedded` remain queryable record attributes (`link.display`, `link.embedded`).
- **L7 (Nominal Duration Equivalence: 1mo == 30d, 1y == 365d): KEEP unchanged.**
  - *Refinement:* Aligns with DuckDB and Dataview. Retain nominal second equality for filter comparisons and sort order. Preserve calendar arithmetic in Jiff when adding durations to anchored dates.
- **L8 (Unicode-Based String Sorting): KEEP, specify locale-independent UCA.**
  - *Refinement:* Collation must use Unicode NFC normalization and simple case folding with byte-exact tiebreak. Avoid host OS libc collation (Postgres corruption warning); ensure sort behavior is identical across Linux, macOS, and Windows.
