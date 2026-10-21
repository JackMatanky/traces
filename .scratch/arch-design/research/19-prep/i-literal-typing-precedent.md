# Literal Typing & Cross-Type Comparison Precedent

- **Topic:** PostgreSQL untyped (`unknown`) literal resolution, cross-engine comparative analysis (14 engines), and adaptation to a dynamic, open-world markdown query engine (Traces).
- **Target File:** `.scratch/arch-design/research/19-prep/i-literal-typing-precedent.md`
- **Tags:** `[V]` verified (primary doc URL, live probe, codebase citation), `[H]` inference, `[P]` proposal.
- **Rules:** Max ~350 lines, dense, decision-ready.

---

## 1. Executive Summary & The Core Architectural Dilemma

1. **PostgreSQL's Model is Static, Plan-Time, and Fail-Stop:** Untyped string literals (`'2025-01-15'`) have pseudo-type `unknown` [V: PG 10.1]. The parser infers their concrete type **once per query** from the static schema of the other operand (step 2.a / 3.f) [V: PG 10.2]. If a literal cannot parse as the target type (e.g. `'2025-01'::date`), PostgreSQL aborts with a fatal parse/syntax error (`ERROR: invalid input syntax`) [V]. It **never** falls back to string or returns false [V].
2. **Dynamic Note Vaults Break PostgreSQL's Primary Assumption:** In markdown vaults, note fields are schemaless and heterogeneous (Note A has `due: 2025-01-15` [Date], Note B has `due: "tbd"` [String], Note C omits `due`). A single static type resolution cannot be bound at query plan time [V, H].
3. **The Proposal Q11 Failure Modes:** Adopting per-row operand-kind adaptation with string fallback (V1) causes **silent query poisoning**: typos in date filters (`due < "2025-02-30"`) silently fall back to `String`, fail type-bracketed comparisons, and return 0 results without any error or warning [V, H].
4. **The Clean Solution:** Pre-compile quoted literals into a zero-allocation `PolymorphicLiteral` (V3) or retain strict typed literals + bare ISO syntax (V5), combined with explicit cast functions (`date()`, `string()`, `dur()`) [P].

---

## 2. PostgreSQL Untyped Literal (`unknown`) Resolution Engine

### 2.1 The `unknown` Pseudo-Type (Manual 10.1)

- String literals without explicit type syntax (e.g. `'abc'`, `'2025-01-15'`, but not `DATE '2025-01-15'`) are initially assigned the pseudo-type `unknown` [V: https://www.postgresql.org/docs/current/typeconv-overview.html].
- Constants of non-string non-numeric types (dates, timestamps, intervals, json) begin lexical life as `unknown` strings [V: 10.1].
- Data types are partitioned into type categories (`string`, `numeric`, `datetime`, `timespan`, `boolean`, etc.) with designated *preferred types* (e.g., `text` for string, `float8` for numeric) [V: 10.1].

### 2.2 Operator & Function Type Resolution (Manual 10.2 & 10.3)

Given binary expression `LHS <op> RHS` [V: https://www.postgresql.org/docs/current/typeconv-oper.html]:

- **Step 1:** Lookup candidate operators by name and argument count visible in search path.
- **Step 2 (Exact Match & Unknown Pair Rule):**
  - **Step 2.a:** If exactly one argument is `unknown`, **assume it is the same type as the other argument** for this check. If an exact operator matches `(T, T)`, use it! (Invocations with two `unknown` inputs never match at Step 2) [V: 10.2.2.a].
  - **Step 2.b:** Domain types collapse to their base type for subsequent matching.
- **Step 3 (Best Match & Fallbacks):**
  - **Step 3.a:** Discard candidates where input types cannot be implicitly cast; `unknown` is assumed convertible to anything.
  - **Step 3.c:** Keep candidates with most exact matches.
  - **Step 3.d:** Keep candidates accepting preferred types in the argument's type category.
  - **Step 3.e (Bias Towards String):** If an argument is `unknown`, check accepted categories. If any candidate accepts `string`, **select `string` category**, and pick preferred type `text` [V: 10.2.3.e].
  - **Step 3.f:** If both `unknown` and known arguments exist and all known have type `T`, assume `unknown` is `T` [V: 10.2.3.f].

### 2.3 UNION, CASE, and ARRAY Constructs (Manual 10.5)

- All inputs are matched to find a single common supertype [V: https://www.postgresql.org/docs/current/typeconv-union-case.html]:
  - If all inputs are `unknown`, resolve as `text` (preferred string type) [V: 10.5.3].
  - Otherwise, `unknown` inputs are ignored while finding the candidate type among non-unknown inputs [V: 10.5.3].
  - If non-unknown inputs span multiple incompatible categories, resolution fails with `ERROR: UNION types <t1> and <t2> cannot be matched` [V: 10.5.4, Ex 10.13].

### 2.4 Concrete Operator Evaluation in PostgreSQL

| Expression | LHS Type | RHS Type | PG Resolution Path | Outcome / SQL Result |
| :--- | :--- | :--- | :--- | :--- |
| `date_col = '2025-01-15'` | `date` | `unknown` | Step 2.a: RHS assumed `date`. Operator `date_eq` found. `date_in('2025-01-15')` called. | `true`/`false` boolean comparison [V] |
| `date_col < '2025-01'` | `date` | `unknown` | Step 2.a: RHS assumed `date`. `date_in('2025-01')` fails input syntax. | **ERROR:** `invalid input syntax for type date: "2025-01"` [V] |
| `text_col = '2025-01-15'` | `text` | `unknown` | Step 2.a: RHS assumed `text`. Operator `texteq` found. | Byte/Collation text comparison [V] |
| `'2025-01-15' = '2025-01-15'` | `unknown` | `unknown` | Step 2 fails. Step 3.e selects `string` category -> `text = text`. | `true` (`text` comparison) [V] |
| `date_col = '2025-01-15'::text`| `date` | `text` | No operator `date = text` exists in `pg_operator`. Implicit cast disabled since PG 8.3. | **ERROR:** `operator does not exist: date = text` [V] |
| `interval_col = '1 hour'` | `interval`| `unknown` | Step 2.a: RHS assumed `interval`. `interval_in('1 hour')` called. | Evaluates exact interval equality [V] |
| `ts_col > 'today'` / `'now'` | `timestamp`| `unknown` | `timestamp_in` recognizes special strings: `today`, `yesterday`, `now`, `epoch`, `infinity`. | Replaced at parse time with current date/time/infinity [V] |

### 2.5 Error vs False, Static Plan-Time Binding, and Prepared Statements

- **Fatal Error vs False:** PostgreSQL **never** evaluates an invalid literal conversion to `false`. Literal parsing errors (`date_in`, `int4in`) abort transaction execution with a fatal error [V].
- **One Resolution Per Query:** Resolution occurs strictly in the parser/planner (`parse_analyze.c`) using static catalog metadata (`pg_attribute`, `pg_type`, `pg_operator`) [V]. The executor runs compiled, monomorphic plan nodes (`OpExpr`, `Const`). It is **structurally impossible** for an operator in PostgreSQL to alter its types per row [V].
- **Prepared Statement Parameter Inference (`$1`):** In `PREPARE p AS SELECT * FROM t WHERE date_col = $1`, `$1` is typed as `unknown` [V: https://www.postgresql.org/docs/current/sql-prepare.html]. The parser infers `$1`'s type as `date` via Step 2.a. If `$1` appears in ambiguous context (`SELECT $1`), prepare fails: `ERROR: could not determine data type of parameter $1` [V].

---

## 3. Cross-Engine Precedent Matrix (14 Engines)

### 3.1 Comparative Matrix

| Engine | Typing Model | Heterogeneous Column | Literal Parse Failure | Ambiguity Handling | Partial-Date Granularity | Relative Keywords | Text Escape |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **PostgreSQL** [V] | Static, catalog | N/A (rigid schema) | Fatal `ERROR` (abort) | Category/Preferred | Rejects partial (syntax error) | `'today'`, `'now'`, `'infinity'` | `'val'::text` |
| **SQLite** [V] | Dynamic + affinity | Storage classes mix | Falls back to Text | No affinity applied | Plain strings (lexical ISO) | Built-in `date('now')` | `+col` or explicit `TEXT` |
| **DuckDB** [V] | Static, vectorized | N/A (rigid schema) | `Conversion Error` | `STRING_LITERAL` to any | Rejects partial | `current_date`, `now()` | `'val'::VARCHAR` |
| **MySQL** [V] | Static, coercing | N/A (rigid schema) | Coerces to 0 / NULL | Numeric over string | Warns/Zeroes partial | `CURDATE()`, `NOW()` | `BINARY 'val'` |
| **SQL Server** [V] | Static precedence | `sql_variant` (rank) | Fatal `Conversion failed` | Precedence list (#1..#32) | Rejects partial date | `GETDATE()` | `CAST('v' AS varchar)` |
| **BigQuery** [V] | Static GoogleSQL | N/A (rigid schema) | Analysis-time Error | `STRING literal` coercion | Rejects partial | `CURRENT_DATE()` | Explicit `STRING` cast |
| **Snowflake** [V] | Static SQL | `VARIANT` (needs cast)| Runtime execution err | Session format params | Rejects partial | `CURRENT_DATE()` | `::string` |
| **Spark SQL** [V] | Static (ANSI/leg) | N/A (rigid schema) | ANSI: Error; Leg: NULL | ANSI disallows cast | Rejects partial | `current_date()` | Explicit `string()` |
| **Kusto (KQL)** [V] | Static + dynamic | `dynamic` (JSON) | Semantic query error | Strict no-coercion | Rejects partial | `now()`, `ago(7d)` | Explicit `tostring()` |
| **Polars** [V] | Static Series | Mismatched Series Err| `ComputeError` | Coerces literal only | Rejects partial | N/A (expressions) | `pl.lit('v')` |
| **Pandas** [V] | Dynamic Series | `object` dtype (slow) | Coerces or throws | Dtype dependent | Slices month/year! | `pd.Timestamp.now()` | Explicit `.astype(str)` |
| **Elasticsearch** [V]| Schematized Lucene| Multi-fields allowed | `parse_exception` | Mapping `format` | Rounds: gte=down, lte=up | `now-7d/d`, `now` date math | `keyword` mapping |
| **Jira JQL** [V] | Static domain | Custom fields typed | JQL syntax error | Date vs text field | Year/month partials | `-7d`, `startOfWeek()` | Quotes `"string"` |
| **Obsidian Bases** [V]| Dynamic JS/YAML | Untyped properties | Evaluation error/null | Strict JS methods | Explicit `date.date()` | `today()`, `now()`, `+"1d"` | String quotes `"text"` |
| **Dataview** [V] | Dynamic Luxon | Mixed frontmatter | Type rank trap (leak) | Order: Date < String | Luxon string parsing | `date(today)`, `dur(1d)` | `string(val)` |

### 3.2 Deep Dives into Key Precedents

1. **DuckDB 0.10.0+ `STRING_LITERAL` Type:** DuckDB introduced a dedicated `STRING_LITERAL` type (`duckdb.org/docs/current/sql/data_types/literal_types`) [V]. While a column of type `VARCHAR` cannot be compared to `DATE` without explicit casting (`Binder Error: Cannot compare DATE and VARCHAR`), a `STRING_LITERAL` implicitly converts to *any* type [V]. If conversion fails, DuckDB raises `Conversion Error: Could not convert string 'invalid' to DATE` [V].
2. **SQLite Storage Class Affinity:** SQLite assigns affinities (`INTEGER`, `REAL`, `TEXT`, `NUMERIC`, `BLOB`) to table columns, but **expressions have no affinity** (`sqlite.org/datatype3.html:318`) [V]. In `col = '123'`, numeric affinity coerces `'123'` to integer `123`. But in `+col = '123'`, unary `+` strips affinity; SQLite compares integer `123` to text `'123'` and evaluates to `0` (false) under storage class ordering (`INTEGER < TEXT`) [V].
3. **SQL Server Data Type Precedence:** SQL Server enforces a fixed 32-level hierarchy: `datetime` (#7) > `date` (#9) > `int` (#17) > `varchar` (#28) [V: `learn.microsoft.com`]. When `date` and `varchar` meet, `varchar` is always converted to `date`. If the string is malformed, the query aborts with `Conversion failed when converting date and/or time from character string` [V].
4. **Elasticsearch Partial-Date Range Rounding:** Elasticsearch range queries round partial date strings based on operator direction [V: `elastic.co/docs/reference/query-languages/query-dsl/query-dsl-range-query`]. For field format `yyyy-MM`: `gte: "2025-01"` rounds down to `2025-01-01T00:00:00.000Z`, while `lte: "2025-01"` rounds up to `2025-01-31T23:59:59.999Z` [V].
5. **Obsidian Bases (Core 2025+):** Bases requires explicit constructor functions: `date("2025-01-15")`, `duration("1d")`, `link("target")` [V: `raw.githubusercontent.com/obsidianmd/obsidian-help/master/en/Bases/Functions.md`]. Date arithmetic allows string duration offsets: `now() + "1 day"`, `date("2024-12-01") + "1M"` [V]. Bare quoted strings in filters are strictly strings.
6. **Dataview's Type-Ranking Trap:** In Dataview, `WHERE due <= "2025-01-01"` performs cross-type ranking via `Values.compareValue`, ranking `DateTime < String` [V: `e-query-engine-conventions.md:42`]. The query matches every row containing a date because any date is ranked lower than any string! Dataview's documentation explicitly warns users that they must write `due <= date(2025-01-01)` [V].

---

## 4. Adapting PostgreSQL's Rule to Dynamic, Open-World PKM (Traces)

### 4.1 The Fundamental Impedance Mismatch

In PostgreSQL, type resolution succeeds because the schema guarantees that `date_col` is monomorphic across all rows. In Traces, notes are open-world:

- In Note 1: `due: 2025-01-15` (`Date`)
- In Note 2: `due: "tbd"` (`String`)
- In Note 3: `due: null` (`Null`)
- In Note 4: `due` is absent.

If Traces adopts a "per-row literal adaptation" rule, it violates PostgreSQL's foundational contract: **PostgreSQL never re-binds literal types per row, and PostgreSQL never uses string fallback to swallow parse errors.**

### 4.2 Five Design Variants & Their Failure Modes

#### Variant 1 (V1): Per-Row Adoption with Fallback to String (Proposal Q11)

- *Mechanism:* For each row, inspect LHS kind `K`. If literal string parses as `K`, compare as `K`. If it fails to parse as `K`, it stays `String`.
- *Failure Modes:*
  1. **Silent Typo Poisoning:** User queries `due < "2025-02-30"`. Feb 30 fails date parsing. It falls back to `String`. Under Type Bracketing (Lean L1), `Date < String` evaluates to `false` (relational no-match). The query **silently returns 0 results** with no error [V, H].
  2. **Per-Row Parsing Overhead:** A 10,000-note vault parses `"2025-01-15"` 10,000 times during table scan [H].
  3. **`!=` Inversion Trap:** In Note 1 (`due` is Date), user queries `due != "tbd"`. `"tbd"` cannot parse as Date -> falls back to String -> `Date == String` is false -> `Date != String` evaluates to `true`. While seemingly benign, `due != "2025-01-15"` on Note 2 (`due: "tbd"`) evaluates `String != String` -> `true`.

#### Variant 2 (V2): Adoption Only When Other Operand is a Bare Field Path

- *Mechanism:* Restricts V1 so adaptation only occurs if the operand is `FieldPath == Literal`.
- *Failure Modes:* Same silent typo poisoning as V1. Fails unpredictably when user wraps field in an expression: `due == "2025-01-15"` adapts, but `date_add(due, "0d") == "2025-01-15"` does not!

#### Variant 3 (V3): Build-Time Polymorphic Literal (`PolymorphicLiteral`) [RECOMMENDED FOR ADAPTATION]

- *Mechanism:* When compiling the query filter AST, the literal string is parsed **once** into all candidate representations:

  ```rust
  struct PolymorphicLiteral {
      raw: String,
      date: Option<CivilDate>,
      datetime: Option<CivilDateTime>,
      duration: Option<DurationValue>,
      number: Option<NumberValue>,
      link: Option<LinkTarget>,
  }
  ```

  During per-row evaluation:
  - If LHS is `Date`: check `literal.date`. If `Some(d)`, compare `lhs == d`. If `None`, evaluate cross-bracket (false).
  - If LHS is `Duration`: check `literal.duration`.
  - If LHS is `String`: compare `lhs == literal.raw`.
- *Advantages:* Zero per-row allocation; zero re-parsing; build-time diagnostics (can emit warning if a literal matches regex `^\d{4}-\d{2}-\d{2}$` but is an invalid calendar day).

#### Variant 4 (V4): Bind-Time Inference from Schema Definition

- *Mechanism:* Relies on `traces.toml` schema declarations. If `due` is declared `date`, bind literal to `Date` at plan time.
- *Failure Modes:* Markdown vaults are predominantly schema-free (zero-config). V4 is unusable for un-schematized vaults.

#### Variant 5 (V5): Strict Typed Literals + Bare ISO Only (No String Adaptation)

- *Mechanism:* Bare unquoted ISO text (`2025-01-15`, `src/query/grammar/filter.rs:1216`) tokenizes as typed `Date`. Quoted strings (`"2025-01-15"`) are **strictly strings**. Explicit functions `date("2025-01-15")`, `dur("1h")`, `link("Alpha")` construct typed values.
- *Failure Modes:* Breaks existing user habits and Traces integration tests where users wrote `file.mtime >= "2026-01-01"` (`filter.rs:1596`) [V].

---

### 4.3 Semantic Precision across Edge Cases

1. **Operand Positions that Adapt:**
   - *Comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`):* Literal adapts to LHS kind.
   - *`contains(container, elem)`:* If LHS is `List<Link>`, string `"Alpha"` adapts to Link target.
   - *Function Arguments (`date_add(d, "1d")`):* Target signature expects duration; literal parsed as duration at query compile time.
   - *Arithmetic (`d + "1d"`):* `Date + Duration` expects duration; compile-time parse.
2. **Field-vs-Field Comparisons (`due == original_due`):**
   - **NEVER adapts.** In PostgreSQL, column-to-column comparison requires an exact operator or explicit cast. In Traces, both sides are runtime values. Type Bracketing applies strictly (cross-bracket `==` is false).
3. **Null / Missing LHS:**
   - Evaluates to `Null` / false for `==` and relational ops; `true` for `!=`. Literal does not adapt.
4. **List LHS (`tags == "rust"`):**
   - In Traces, `tags == "rust"` is Whole-List equality (`List == String` -> false) [V: `brief.md:90`]. Membership requires `contains(tags, "rust")`.
5. **Ambiguous Literals:**
   - `"2025-01"`: Fails day-level date parsing. Stays `String`. Does NOT match `Date(2025-01-15)`.
   - `"1h"`: Parses as both `String` and `Duration(1 hour)`. Matches `Duration(1h)` on duration fields, and matches `"1h"` on string fields.
   - `"5"`: Does NOT adapt to Number. Quoted `"5"` is strictly text. Bare `5` is a number literal.
6. **Wikilink Literals:**
   - Quoted `"[[Alpha]]"` parsed as `LinkTarget("Alpha")`.
   - Bare quoted string `"Alpha"` against `Link` LHS matches folded target if Link-vs-String reading is enabled [P].
7. **Reporting Non-Parsing Literals:**
   - Under V3, emit a query compiler **advisory warning** (stderr) if a string literal matches date/duration pattern syntax but fails semantic validation (e.g., `"2025-02-30"` or `"99h99m"`). Evaluation returns false/no-match.

---

### 4.4 Decision Table: Expressions x LHS Kinds x Variant Outcomes

| Expression | LHS Note Value (`v`) | V1 (Per-Row + Fallback) | V3 (Polymorphic Build-Time) | V5 (Strict / No Adapt) | Status Quo (`classify_literal`) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `v == "2025-01-15"` | `Date(2025-01-15)` | **True** (adapts to Date) | **True** (matches `date`) | **False** (Date vs String) | **True** (sniffed to Date) [V] |
| `v == "2025-01-15"` | `String("2025-01-15")` | **True** (stays String) | **True** (matches `raw`) | **True** (String vs String)| **False** (sniffed to Date!) [V] |
| `v == "2025-01-15"` | `String("Meeting")` | **False** (stays String) | **False** (String mismatch)| **False** (String mismatch)| **False** (Date vs String) [V] |
| `v < "2025-02-30"` (typo) | `Date(2025-01-15)` | **False** (silent no-match)| **False** (+ compile warn!) | **False** (Date vs String) | **Parse Error** at compile [V] |
| `v == "2025-01"` (month) | `Date(2025-01-15)` | **False** (falls back str) | **False** (no date parsed) | **False** (Date vs String) | **False** (stays String) [V] |
| `v == "1h"` | `Duration(1h)` | **True** (adapts to Dur) | **True** (matches `dur`) | **False** (Dur vs String) | **True** (sniffed to Dur) [V] |
| `v == "1h"` | `String("1h")` | **True** (stays String) | **True** (matches `raw`) | **True** (String vs String)| **False** (sniffed to Dur!) [V] |
| `v == "5"` | `Int(5)` | **False** (str, no num adapt)| **False** (no num adapt) | **False** (Num vs String) | **False** (stays String) [V] |
| `v == "[[Alpha]]"` | `Link([[Alpha]])` | **True** (parses as Link) | **True** (matches `link`) | **False** (Link vs String) | **False** (stays String) [V] |
| `v == "Alpha"` | `Link([[Alpha]])` | **True** (if target-fold) | **True** (if target-fold) | **False** (Link vs String) | **False** (Link vs String) [V] |
| `v != "tbd"` | `Date(2025-01-15)` | **True** (complement) | **True** (cross-bracket !=) | **True** (cross-bracket !=)| **True** (cross-bracket !=) [V] |
| `v < "tbd"` | `Date(2025-01-15)` | **False** (cross-bracket) | **False** (cross-bracket) | **False** (cross-bracket) | **True** (leaks rank table!) [V] |

---

## 5. Implications for Ticket 19

### 5.1 The Alignment of Q10 (`string()`) and Q11 (Literal Adaptation)

The user's critique asked: *If literals adapt, why is `string()` needed?*

1. **Frontmatter vs Query Seam:** Frontmatter classification is quote-insensitive (Ticket 18: lines 501-502) [V]. A note with `title: "2025-01-15"` has `NoteFieldValue::Date` in the index [V]. If a user wants to run string operations on that title (`string(title).contains("2025")` or regex match), `string(title)` is required to convert the **stored note field**, not the query literal! [P]
2. **Literal Disambiguation Escape:** If a user queries a heterogeneous field and explicitly wants to match ONLY string values (preventing literal adaptation to Date), writing `v == string("2025-01-15")` forces the literal into a strict, non-adapting `String` [P]. This precisely mirrors PostgreSQL's `'2025-01-15'::text` [V].

### 5.2 Decision-Ready Options for Ticket 19

- **Option A (Status Quo Sniffing - Reject):** Keep `ComparisonExpr::classify_literal` (`filter.rs:238`). Unconditionally converts quoted `"2025-01-15"` into `Date`.
  *Why reject:* Causes severe bugs where `title == "2025-01-15"` fails on notes where `title` is a string (e.g. from inline fields `title:: "2025-01-15"` or schema `type = string`).
- **Option B (Build-Time Polymorphic Literal V3 - Lean):** Quoted literals in filters compile into a `PolymorphicLiteral` containing all valid parsed representations (`raw`, `date`, `datetime`, `duration`, `link`). In row evaluations, the literal matches if the row's LHS kind matches any valid parsed representation.
  *Why lean:* Delivers 100% ergonomic zero-config querying (`due >= "2025-01-01"` and `title == "2025-01-15"` both work without syntax clutter), completely eliminates per-row re-parsing overhead, and enables compile-time diagnostic warnings for invalid date/duration strings.
- **Option C (Strict Bases/Dataview Model V5):** Bare ISO `2025-01-15` is Date; quoted `"2025-01-15"` is strictly String. Explicit functions `date(...)` and `dur(...)` required for quoted temporals.
  *Why consider:* Purest theoretical model, zero ambiguity. Costs breaking existing test idioms and queries with quoted dates.

### 5.3 Concrete Recommendation for Ticket 19

1. **Adopt Option B (Variant 3: Build-Time Polymorphic Literal):** Replace `ComparisonExpr::classify_literal` with build-time multi-representation binding. When a string literal parses as Date/DateTime/Duration/Link, store those parsed values in the literal AST node. In filter evaluation, compare against the representation corresponding to the LHS operand's kind.
2. **Retain Bare ISO Date Syntax:** Bare ISO dates (`due >= 2025-01-01`) remain first-class typed Date literals (`FilterToken::DateLiteral`).
3. **Retain Explicit Cast Functions:** Provide `date(x)`, `dur(x)`, and `string(x)`. Explicit `string(lit)` suppresses polymorphic adaptation.
4. **Compile-Time Typo Diagnostics:** When a quoted string matches ISO date regex but fails calendar validation (`"2025-02-30"`), emit a compiler advisory warning instead of failing silently.
5. **Strict Field-vs-Field Bracketing:** Column-to-column comparisons never adapt; cross-kind comparisons evaluate strictly via Type Bracketing (Lean L1).
