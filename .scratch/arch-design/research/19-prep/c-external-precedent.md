# External Precedent: Coercion, Equality, Ordering, and Typing Policies

- **Topic:** External precedent across language ecosystems, databases, query languages, and PKM tools for Wayfinder Ticket 15.
- **Status:** Complete.
- **Deliverable:** `.scratch/arch-design/research/19-prep/c-external-precedent.md`.

---

## 1. Numeric Equality and Ordering Across i64 and f64

- **PostgreSQL:** Direct operators exist for mixed numeric comparisons (`int8` vs `float8`, `int8` vs `numeric`) [V: `functions-comparison.html:9.2`]. When comparing integer and floating-point types, PostgreSQL evaluates exact numerical values rather than truncating integers to IEEE float [V: `pg_num2int_direct_comp`].
- **SQLite (`sqlite3IntFloatCompare`):** Implemented in SQLite core (`src/util.c` / `vdbeaux.c`) [V: Turso issue #6056]. When comparing `i64` and `double`, SQLite avoids converting `i64` to `f64` (which loses precision above $2^{53}$). Instead, it truncates the float to `i64` for integer-range values, compares integer-to-integer, and uses the float's fractional part to break ties [V: Turso #6056; SQLite `datatype3.html#sort_order`]. Floats outside `[i64::MIN, i64::MAX]` compare by magnitude/sign [V].
- **DuckDB:** Implicit cast rules promote `BIGINT` to `DOUBLE` in generic binary comparisons, causing precision loss beyond $2^{53}$ unless explicit `DECIMAL` / `HUGEINT` casts are used [V: `cast_rules.cpp:54-311`].
- **Python:** `int == float` performs exact mathematical comparison (`Objects/floatobject.c:float_richcompare`) [V]. `(1 << 53) + 1 == float((1 << 53) + 1)` evaluates to `False`; `(1 << 53) + 1 > float((1 << 53) + 1)` is `True` [V]. No precision loss occurs for large integers.
- **JavaScript (ES2020+):** `Number` vs `BigInt`: `9007199254740993n == 9007199254740992.0` is `false`; `9007199254740993n > 9007199254740992.0` is `true` [V]. Loose equality `==` and relational operators `<`/`>` perform mathematical comparison; strict `===` returns `false` due to differing types [V].
- **CEL (Common Expression Language):** Proposal 210 establishes that numbers exist on a continuous number line (ZetaSQL model) [V: `cel-spec/proposal-210.md:37-45`]. Relational overloads (`<`, `<=`, `>`, `>=`) and runtime heterogeneous equality (`==`, `!=`) operate across all combinations of `int`, `uint`, and `double` [V: `proposal-210.md:46-52`]. `1 == 1.0` is `true` [V]. `-0.0 == +0.0` is `true`, `-0.0 < 0` is `false` [V: `proposal-210:175-185`].
- **jq & JSON Schema:** In jq, all numbers are IEEE doubles; sort rank places numbers between booleans and strings: `null < false < true < numbers < strings < arrays < objects` [V: jq manual]. JSON Schema (2020-12 §4.2.1) mandates mathematical equality: `1 == 1.0` is `true` ("two numbers are equal if their difference is zero") [V].
- **Rust `f64::total_cmp` vs `ordered_float::OrderedFloat`:**
  - `f64::total_cmp` implements IEEE 754-2008 `totalOrder`: `-NaN < -Inf < -positive < -0.0 < +0.0 < +positive < +Inf < +NaN` [V: `std::cmp::Ord`]. Cost: `-0.0 < +0.0` contradicts `==`; distinguishes quiet from signaling NaNs [V].
  - `ordered_float::OrderedFloat`: treats `-0.0 == +0.0`, equates all `NaN == NaN`, and orders `NaN` greater than all finite numbers [V: `ordered-float/latest/struct.OrderedFloat.html`].
- **Rust Exact `i64` vs `f64` Algorithm:** If $f$ is NaN, return `None` (or sort as greatest). If $f$ is infinite or outside $[ -2^{63}, 2^{63}-1 ]$, compare by sign. If $f$ has a fractional part (`f.fract() != 0.0`), compare `i` against `f.floor() as i64` / `f.ceil() as i64`. If $f$ is an exact whole number, `(f as i64).cmp(&i)` is exact [H].
- **Eq/Hash/Ord Coherence:** If `Int(1) == Float(1.0)` is true in Rust's derived `PartialEq`, `Hash` MUST produce the identical hash code, which is expensive and incompatible with fast integer hashing [V: Rust `std::hash::Hash` contract]. **Verdict:** Keep `NoteFieldValue::PartialEq` structural (variant-exact); evaluate semantic numeric equality (`1 == 1.0`) strictly in query expressions and comparator adapters [P].

---

## 2. Date vs DateTime vs Zoned Timestamps

- **Jiff Model (v0.2.38):**
  - Concrete separated types: `civil::Date` (`src/civil/date.rs:172`), `civil::DateTime` (`src/civil/datetime.rs:218`), `Timestamp` (UTC instant), and `Zoned` (`src/zoned.rs:314`) [V].
  - **Mutual Incomparability:** Jiff provides ZERO cross-type `PartialEq` or `PartialOrd` between `Date`, `DateTime`, `Timestamp`, and `Zoned` [V: `jiff` docs]. Comparing them requires explicit projection (`date.to_datetime(...)` or `zdt.datetime()`) [V].
  - `Zoned::cmp` compares instant in time (epoch nanoseconds) only, completely ignoring civil datetime and timezone [V: `src/zoned.rs:314`]. `Zoned` equality `==` also compares only the instant [V].
  - Ambiguity resolution: Jiff's "compatible" strategy resolves DST forward gaps by shifting forward and backward folds by selecting the earlier time [V: `src/zoned.rs:314`].
- **TC39 Temporal:**
  - `PlainDate.compare(d1, d2)` converts operands via `PlainDate.from` (truncating any time component) [V: MDN `Temporal.PlainDate.compare`].
  - `PlainDateTime.compare(dt1, dt2)` compares wall-clock date, then wall-clock time [V: MDN `Temporal.PlainDateTime.compare`].
  - `ZonedDateTime.compare(z1, z2)` compares exact epoch nanoseconds; `z1.equals(z2)` requires exact instant, timezone, and calendar match [V: MDN `Temporal.ZonedDateTime.compare`].
- **SQL Standards (PostgreSQL & DuckDB):**
  - PostgreSQL promotes `date` to `timestamp without time zone` at `00:00:00` (midnight) [V: Postgres docs §9.9]. When comparing `timestamp without time zone` to `timestamp with time zone`, PostgreSQL rotates the naive timestamp to UTC using the session `TimeZone` [V: Postgres docs §9.9]. This creates an ambient timezone leak [H].
  - DuckDB casts `DATE` to `TIMESTAMP` at `00:00:00` without timezone shifts [V: `duckdb.org/docs/sql/expressions/comparison_operators`].
- **Pandas & Polars (Strict Naive vs Aware Separation):**
  - Polars strictly REFUSES to compare naive `datetime` and tz-aware `datetime`, raising `SchemaError: could not evaluate '>' comparison between series ... of dtype: datetime[μs, Europe/Budapest] and series ... of dtype: datetime[μs]` [V: Polars issue #21362].
  - Python standard library `datetime` returns `False` for `naive == aware` and raises `TypeError: can't compare offset-naive and offset-aware datetimes` for relational `<`/`>` [V: Python `datetime.py`].
- **Obsidian / Dataview (Luxon):**
  - Dataview parses all frontmatter dates into Luxon `DateTime` objects with an ambient local zone, collapsing dates and datetimes into one type [V: `survey/temporal.md:12`].
- **Deterministic Cross-Granularity Options for Traces:**
  1. *Incomparable / Separate Rank:* `Date` and `DateTime` remain separate variants. Derived `PartialEq` is false; cross-type filter returns false (or error) [P].
  2. *Civil Midnight Promotion:* Promote `Date` to civil `DateTime` at `00:00:00` without any timezone conversion [P].
  3. *Date-as-Interval:* `Date` represents `[YYYY-MM-DD 00:00:00, YYYY-MM-DD 23:59:59.999]`; equality is containment, ordering is interval comparison [H].

---

## 3. Calendar Durations

- **Jiff `Span` Comparison Invariants:**
  - `Span` intentionally does NOT implement `PartialEq` or `Eq` [V: `jiff::Span` docs, `src/span.rs:708`]. BurntSushi: *"It is fundamentally impossible to do such comparisons without a reference date in all cases."* `120.minutes()` and `2.hours()` have different in-memory representations; `1.month()` vs `30.days()` depends on the reference month [V: `src/span.rs:708`].
  - `Span::fieldwise()` creates a `SpanFieldwise` wrapper that compares components field-by-field [V: `src/span.rs:708`].
  - `Span::compare()` requires a `SpanRelativeTo` reference date whenever calendar units (years, months, weeks) are present; calling `1.month().compare(30.days())` without a reference date returns an error [V: `src/span.rs:708`].
- **TC39 `Temporal.Duration`:**
  - `Temporal.Duration.compare(d1, d2, { relativeTo: ... })` requires `relativeTo` whenever either duration contains calendar units (years, months, weeks) [V: MDN; `temporal_rs::Duration::compare` in `src/builtins/compiled/duration.rs:30`].
- **PostgreSQL `interval` Pitfall:**
  - PostgreSQL converts intervals to microseconds using fixed nominal ratios: 1 month = 30 days (720 hours), 1 day = 24 hours [V: `functions-datetime.html`].
  - Anomaly: `INTERVAL '1 month' = INTERVAL '30 days'` is `TRUE`; `INTERVAL '1 year' = INTERVAL '360 days'` is `TRUE`; but `INTERVAL '1 year' = INTERVAL '365 days'` is `FALSE` [V: pgsql-general thread `CA+CZih6CdRpRES8_yahkAYfypzzxe2mYCcTZJvMVd-+zOMVpRQ@mail.gmail.com`].
- **Java `java.time` (Strict Separation):**
  - `Period` (date-based: years, months, days) vs `Duration` (time-based: seconds, nanoseconds) are separate types. `Period` does NOT implement `Comparable` at all because calendar periods lack a total order without a timeline anchor [V: `java.time.Period` javadoc].
- **Rust Crates:**
  - `humantime`: Parses `month` as `30.44 days` and `year` as `365.25 days` into `std::time::Duration` (lossy scalar approximation) [V: `humantime::duration::parse_duration` docs].
  - `time::Duration` / `time::SignedDuration`: Represents whole seconds and nanoseconds only; completely lacks calendar units (years/months) [V: `time::signed_duration::SignedDuration` in `src/signed_duration.rs:50`].
- **Transfer to Traces:** Traces currently caches `nominal_seconds: i64` (1mo = 30d, 1yr = 365d) in `DurationValue` (`src/temporal/duration.rs:157-161`) to provide a total `Ord` for redb B-trees [V: `survey/temporal.md:30`]. This pays the PostgreSQL nominal-equivalence penalty [H].

---

## 4. Heterogeneous Ordering vs Comparison Semantics

- **Total Order for Sorting vs Filter Comparison:**
  - **SQLite:** Defines a single global type rank for storage classes: `NULL < NUMERIC < TEXT < BLOB` (`sqlite3MemCompare`, `src/vdbeaux.c:4555-4623`) [V]. Pitfall: This rank leaks directly into filter comparisons: `SELECT 1 < 'a'` returns `1` (true) [V]. A filter `WHERE age > 18` unexpectedly matches text values like `'unknown'` [H].
  - **MongoDB (The Dual Model):**
    - *Sorting:* Uses global BSON comparison order: `MinKey < Null < Numbers < Symbol/String < Object < Array < BinData < ObjectId < Bool < Date < Timestamp < Regex < MaxKey` [V: `bson-type-comparison-order`].
    - *Query Filtering (Type Bracketing):* Predicate operators (`$gt`, `$gte`, `$lt`, `$lte`) enforce **Type Bracketing** [V: `mongodb.com/docs/manual/reference/mql/query-predicates/type-bracketing/`]: a comparison on a given type matches ONLY values of that type (with all numeric types grouped into one bracket) [V]. `{ qty: { $gt: 4 } }` evaluates to `false` for string `"fourteen"` or object `{ in_stock: 8 }` [V]. Negation `$ne: 4` evaluates to `true` for non-numbers [V].
  - **CEL:** No cross-type ordering exists; relational comparisons between mismatched non-numeric types (e.g. `string < int`) fail type-checking or raise runtime evaluation errors [V: `cel-spec/proposal-210.md`]. Runtime equality across disparate types (e.g. `"" == null` or `0 == null`) evaluates to `false` [V: `proposal-210.md:520-535`].
  - **SQL Standard:** Relational comparisons across incompatible types result in type errors at prepare/compile time.
  - **Dataview:** Sort order compares internal type names alphabetically: `"array" < "boolean" < "date" < "duration" < "link" < "number" < "object" < "string"` (`Values.compareValue`, `src/data-model/value.ts:5487`) [V].
- **Verdict for Traces:** Total order is required for `SORT BY` (must not error on mixed notes); but relational filter predicates (`WHERE x > 10`) should adopt MongoDB's **Type Bracketing**: mismatched types evaluate to `false`, not rank-order truth [P].

---

## 5. Frontmatter/YAML Typing Pitfalls and Escapes

- **YAML 1.1 vs YAML 1.2 Core Schema:**
  - *The Norway Problem:* In YAML 1.1, unquoted `no`, `yes`, `on`, `off`, `y`, `n` resolve to booleans (`country: NO` becomes `country: false`) [V: YAML 1.1 spec §10.2.1.4]. YAML 1.2 Core Schema restricts booleans to `true`/`false` [V: YAML 1.2 spec §10.2.1.4].
  - *Sexagesimal:* YAML 1.1 parses `12:34:56` as base-60 integer `45296` [V: YAML 1.1 spec §10.2.1.4]. Removed in YAML 1.2.
  - *Octal:* YAML 1.1 treats `012` as octal 10; YAML 1.2 requires `0o12` [V: YAML 1.2 spec].
  - *Timestamps:* YAML 1.1 includes an implicit timestamp regex converting `YYYY-MM-DD` and `YYYY-MM-DDTHH:MM:SS` to timestamps [V: YAML 1.1 spec]. YAML 1.2 Core Schema treats unquoted dates as plain strings unless explicitly tagged `!!timestamp` [V: YAML 1.2 spec §10.2.1.4].
- **Obsidian Properties:**
  - Property types are defined vault-wide in `.obsidian/types.json` (`{ "due": "date", "tags": "multitext" }`) [V: `obsidian-help/.../Properties.md`].
  - Text fields containing wikilinks `[[Link]]` must be enclosed in quotes in YAML because `[` begins a YAML sequence [V: `Properties.md:125-132`].
  - Obsidian does NOT heuristically sniff arbitrary strings as dates; type enforcement is schema-informed by `types.json` [V: `Properties.md`].
- **Static Site Generators (SSGs):**
  - *Jekyll:* Uses Psych (Ruby YAML parser). Unquoted dates become Ruby `Time`/`Date`. Strings must be quoted (`title: "2024-01-15"`).
  - *Hugo:* Parses dates via Go's `time.Parse` only for known frontmatter date keys (`date`, `publishDate`, `lastmod`); arbitrary custom keys remain strings unless YAML types them.
  - *Zola:* Uses TOML frontmatter by default (`+++`). TOML has native date syntax (`date = 2024-01-15`); quoted values (`date = "2024-01-15"`) remain strictly strings [V: TOML v1.0.0 spec].
  - *Pandoc:* Parses YAML frontmatter into AST `Meta` blocks; dates without quotes become `MetaInlines` / `MetaString`, preserving textuality [V: Pandoc manual].
  - *Universal YAML Escape:* Explicit tag `!!str 2025-01-15` forces string interpretation across all YAML 1.1 and 1.2 parsers [V: YAML 1.2 spec].
- **Frictionless Table Schema & JSON Schema (`format: date`):**
  - Table Schema `castFormat` and JSON Schema `format: date`: strings validated against ISO 8601 remain `type: "string"` in storage and JSON representation [V: JSON Schema 2020-12]. Typing is applied on demand during analysis.

---

## 6. Link Identity Across PKM Systems

- **Obsidian, Logseq, Foam, Dendron, Marksman, Markdown-Oxide:**
  - *Case-Insensitive Target:* Link targets are matched case-insensitively on Windows and macOS (matching filesystem semantics) [V: Marksman `features.md`; `oxide.md`].
  - *Extension Elision:* Wikilinks elide `.md` (`[[note]]` resolves to `note.md`) [V: Marksman `features.md`].
  - *Alias / Display Independence:* `[[target|Display Text]]`: the display text is strictly a rendering/transclusion label; it is completely ignored for link target identity, backlink indexing, and equality [V: Marksman `features.md`; Obsidian Help `Internal links`].
  - *Subpaths (Anchors):* `[[target#Heading]]` or `[[target^block-id]]`: link identity consists of `(target, Option<anchor>)`. When resolving note-level graph edges, the anchor is stripped [V: Marksman `features.md`].
  - *Embed Flag:* `![[target]]` is a transclusion directive in the markdown document parser; it does NOT alter the target note's identity in the link graph [V: Obsidian Help `Embed files`].

---

## 7. String Collation and Sorting

- **Database Precedents:**
  - **SQLite:** Default collation is `BINARY` (byte-by-byte `memcmp`) [V: SQLite `datatype3.html`]. Built-in `NOCASE` folds only the 26 ASCII letters A–Z to a–z; non-ASCII Unicode characters (e.g. `ä` vs `Ä`) compare unequal [V: SQLite docs].
  - **PostgreSQL:** Supports `COLLATE "C"` (byte compare), libc locales (`COLLATE "en_US.UTF-8"`), and ICU (`COLLATE "und-x-icu"`). **Warning:** Libc collations differ across operating systems (glibc on Linux sorts differently from macOS libc for the same locale), breaking cross-machine test determinism [V: Postgres docs §23.2].
  - **DuckDB:** Default collation is `BINARY`; ICU extension provides locale collations [V: `duckdb.org/docs/sql/expressions/collations`].
- **Rust Collation Crates:**
  - `unicase`: Provides `UniCase<T>` wrapper supporting ASCII or full Unicode case folding [V: `unicase` docs].
  - `caseless`: Provides canonical Caseless matching conforming to Unicode Standard Annex #44 [V: `caseless` docs].
- **PKM User Expectations & Determinism:**
  - Search and link matching: case-insensitive (Unicode-aware folding) [H].
  - Note sorting: natural / alphanumeric sort (`Chapter 2` before `Chapter 10`), case-insensitive [H].
  - Machine determinism: Traces runs as a local CLI/LSP on Linux and macOS; sorting must be 100% reproducible across OSes. Host libc collation MUST NOT be used [P].

---

## 8. Rust-Specific Design Patterns for Dynamic Values

- **DataFusion `ScalarValue`:**
  - `ScalarValue` (`datafusion-common`, 54 variants) implements `PartialEq` and `PartialOrd` [V: `docs.rs/datafusion-common/54.1.0`].
  - `partial_cmp` between mismatched variants returns `None` [V: `ScalarValue` docs]. Cross-type comparisons are never guessed at runtime inside `ScalarValue`; DataFusion's query planner inserts explicit `Cast` expressions using `type_coercion::binary::comparison_coercion` before execution [V].
  - `try_cmp` provides explicit comparison returning `Result<Ordering>` [V].
- **Polars `AnyValue`:**
  - `AnyValue` implements `PartialEq` variant-by-variant; cross-variant comparisons evaluate to `false` [V: `polars-core/src/datatypes/any_value.rs`].
- **`serde_json::Value` & `Number`:**
  - `serde_json::Number` derives `PartialEq` and `Eq` [V: `serde_json::number::Number` in `src/number.rs:22`]. Integer `1` and float `1.0` are structurally distinct enum variants (`PosInt(1)` vs `Float(1.0)`); `json!(1) == json!(1.0)` is `false` [V].
- **`toml::Value::Datetime`:**
  - Re-exports `toml_datetime::Datetime` [V: `toml::value::Datetime` in `src/value.rs:18`]. Uses structural `Ord`: compares `date`, then `time`, then `offset` [V]. Date without time (`time: None`) sorts before date with midnight (`time: Some(00:00:00)`), keeping them strictly distinct [V].

---

## 9. Transferability Matrix to Traces

| Precedent | What It Does | Why It Does It | Cost Paid | Transfers to Traces? |
|---|---|---|---|---|
| **SQLite `sqlite3IntFloatCompare`** | Exact `i64` vs `f64` comparison without float truncation | Avoid precision loss above $2^{53}$ | Branching overhead on mixed comparison | **YES [P]:** Solves numeric equality in queries without multi-crate dependencies. |
| **CEL Proposal 210** | Numbers on continuous number line; heterogeneous equality | Align user expectations (`1 == 1.0`, `0 != null`) | Special runtime dispatch for numeric pairs | **YES [P]:** Ideal semantic model for query filters (`--where "num == 1"`). |
| **MongoDB Type Bracketing** | Sort uses total rank; filters match only same type bracket | Prevent rank-order leakage (`1 < 'a'`) into queries | Query evaluator must check type before comparing | **YES [P]:** Essential for predictable note query filtering. |
| **Jiff Separate Temporal Types** | Mutual incomparability between `Date`, `DateTime`, `Zoned` | Prevent invalid assumptions and DST bugs | Callers must explicitly convert | **YES [P]:** Prevents ambient timezone leaks in core `NoteFieldValue`. |
| **Jiff `SpanRelativeTo`** | Requires reference date to compare calendar spans | 1mo vs 30d is fundamentally ambiguous | Fallible / context-dependent comparison | **YES [P]:** Query engine supplies context `now` for duration comparisons. |
| **Obsidian Properties (`types.json`)** | Schema-informed typing; no eager string sniffing | Prevent string mutation (`title: "2025-01-15"`) | Requires schema / property registry | **YES [P]:** Supports transition from sniffing to schema typing. |
| **DataFusion `ScalarValue` Coercion** | Planner inserts `Cast`; `ScalarValue` is strictly typed | Separation of planning from evaluation | Planning phase required | **PARTIAL [P]:** Adopt type-coercion lattice in query binder; keep runtime value simple. |
| **PostgreSQL `interval` 30-day conversion** | Converts 1 month to 30 days in all comparisons | Force a total order without reference date | Serious anomalies (`1 mon == 30 days`, `1 yr == 360 days`) | **NO [H]:** Keep nominal seconds only for storage sort keys, not semantic equality. |
| **Dataview Type-Name Alphabetical Sort** | Sorts mismatched types by name (`"date" < "number"`) | Quick JavaScript hack | Bizarre ordering (`false < 1970 < 100 < "a"`) | **NO [H]:** Traces needs a semantic rank hierarchy. |
| **SQLite Collation with Host libc** | Delegates locale collation to OS libc | Avoid bundling full Unicode tables | Inconsistent cross-platform results (Linux vs macOS) | **NO [H]:** Breaks reproducible CLI/test execution across dev machines. |

---

## 10. Implications for Ticket 15

### Q1: Where Does String-to-Date/Duration Promotion Live?
- **Option A (Keep as is / Heuristic Classifier):** Strings matching temporal patterns are promoted at ingestion time during frontmatter/inline parsing (`From<noyalib::Value>` and inline lexer). *Cost:* Strings like `title: 2025-01-15` become dates unless escaped.
- **Option B (Lexical Storage + Query/Schema Cast on Demand):** All unquoted strings store as `String`. Promotion happens only via schema type declarations (`type = "date"`) or explicit query functions (`date(due)` / `TRY_CAST(due AS date)`). Follows Frictionless, Logseq, and DuckDB.
- **Option C (Dual Ingestion: Safe Classifier + Query Cast):** Ingestion classifies unambiguous ISO dates (`YYYY-MM-DD`) and durations (`P1D`, `2h30m`) into `Date`/`Duration`, but leaves ambiguous text as `String`. Query binder provides `date(...)` and `duration(...)` for explicit coercion.

### Q2: Frontmatter Literal-String Escape
- **Option A (Keep as is / Quoting as Escape):** Ticket 14 decided frontmatter classification is quote-insensitive. Users must use inline quotes (`title: '"2025-01-15"'`) or YAML tags (`title: !!str 2025-01-15`).
- **Option B (YAML Quote Sensitivity via CST/Event Parser):** Use YAML quote presence to distinguish `title: "2025-01-15"` (string) from `title: 2025-01-15` (date). *Cost:* Ticket 14 noted Noyalib CST is fragile on block scalars and dotted keys.
- **Option C (Standard YAML `!!str` Tag + Schema Override):** Support `!!str 2025-01-15` as standard YAML escape; schema field definitions (`type = "string"`) override any heuristic classification.

### Q3: Int vs Float Semantic Equality, NaN, -0.0, and $2^{53}$ Exactness
- **Option A (Derived Structural Equality Only):** `Int(1) == Float(1.0)` is `false`. Query filters require exact matching variant.
- **Option B (CEL / SQLite Number Line Model):** `NoteFieldValue::PartialEq` remains structural (variant-exact for Rust maps/sets). Query evaluation and filter comparison evaluate mathematical equality across `Int` and `Float` using SQLite's `sqlite3IntFloatCompare` algorithm. `-0.0 == +0.0` is true; `NaN` is not equal to anything (or ordered as null).
- **Option C (OrderedFloat Wrapper in Enum):** Store `Float(OrderedFloat<f64>)` in `NoteFieldValue`. Equates `-0.0` and `+0.0`; orders `NaN` greater than all numbers. Cross-type `Int` vs `Float` still requires query-layer comparison.

### Q4: Date vs DateTime Equality and Order (Ambient Zone Elimination)
- **Option A (Keep as is / Ambient Local Zone Promotion):** Promote `Date` to `DateTime` at local midnight using `local_zone()`. Violates determinism; rejected by prior audits.
- **Option B (Separate Incomparable Ranks):** `Date` and `DateTime` have distinct ranks in `Ord` (`Date < DateTime` or vice versa). Derived `PartialEq` is false. Query equality `date == datetime` is false unless explicit `date(dt)` or `datetime(d)` is called. Zero timezone dependency.
- **Option C (Pure Civil Midnight Promotion):** In query comparison, promote `Date` to civil `DateTime` at `00:00:00` without consulting any timezone. `2025-01-15 == 2025-01-15T00:00:00` is true. Store naive datetimes as civil `DateTime` (not UTC instant shifted by ambient zone).

### Q5: Cross-Kind Rank Order and Equality
- **Option A (Keep as is / Global Rank for Sort and Filter):** `Null < Bool < Number < Duration < Temporal < Text < Link < List < Object`. Relational filter operators (`<`, `>`) use this rank directly (SQLite style: `1 < "text"` is true).
- **Option B (MongoDB Dual Model: Total Sort Order + Type-Bracketed Filtering):** Total rank order is used strictly for `ORDER BY` / sorting. Query filter predicates (`=`, `!=`, `<`, `<=`, `>`, `>=`) use **Type Bracketing**: comparisons across incompatible kinds evaluate to `false` (or `true` for `!=`), never leaking type rank into search results.
- **Option C (Strict Erroring Filters):** Mixed-kind comparisons in filter expressions raise a query diagnostic/error (CEL static model / SQL standard).

### Q6: Link Equality and Order
- **Option A (Keep as is / Full Structural Comparison):** `Link` derives `PartialEq`/`Ord` comparing `target`, then `display`, then `embedded`. Two links with different aliases are unequal.
- **Option B (Target-Only Identity with Normalized Display):** `Link` equality `==` compares `target` only (case-insensitively). `embedded` and `display` are presentation metadata, not target identity. Sorting orders by `target`, then subpath.
- **Option C (Normalized Path Identity):** Links normalize relative paths and strip `.md`. Equality checks resolved file target identity.

### Q7: Schema Select String vs Note Link Matching
- **Option A (Literal Textual Match):** Select option `"[[Note]]"` matches note link only if target string exactly matches `"[[Note]]"`.
- **Option B (Consumer-Boundary Parsing via Root Link):** Schema select strings naming links are parsed through `Link::parse_wikilink` at the validation boundary (Ticket 16). Matching compares parsed `target` case-insensitively.

### Q8: Query Literal Promotion and Sniffing at Sort Time
- **Option A (Keep as is / `TextShape::classify`):** Sort keys classify unparsed strings at sort time (`src/query/sort.rs:462`). Filter literals classify via `classify_literal` (`src/query/grammar/filter.rs:238`).
- **Option B (Eliminate Sniffing; Bound Query Types):** Eliminate `TextShape::classify` in `query/sort.rs`. Values sort by their actual stored `NoteFieldValue` variant. Query literals are parsed into typed AST literals during query parsing.

### Q9: Calendar Duration Comparison Policy
- **Option A (Keep as is / Nominal Seconds):** `DurationValue` caches `nominal_seconds: i64` (1mo = 30d, 1yr = 365d) for total order. Accepts PostgreSQL-style nominal anomalies.
- **Option B (Jiff Context-Relative Comparison):** Query execution passes `ExecutionContext { now: Timestamp }`. Durations with calendar units evaluate `Span::compare((other, ctx.now))` for accurate elapsed comparison.
- **Option C (Fieldwise Comparison for Equality, Nominal for Sorting):** `DurationValue` equality requires exact fieldwise equivalence (`1mo != 30d`). Sorting uses nominal seconds as a stable tie-breaker.

### Q10: String Collation (Case Sensitivity & Normalization)
- **Option A (Keep as is / Standard Rust Byte Comparison):** `String` compares byte-by-byte (`Ord for String`). Sorting is ASCII-first (`Z < a`).
- **Option B (Deterministic Natural + ASCII Case-Insensitive Collation):** Queries and note sorting use a custom comparator: ASCII case-folded, with natural integer sorting (`Chapter 2` < `Chapter 10`). 100% deterministic across Linux and macOS.
- **Option C (Unicode `unicase` Collation):** Integrate `unicase` for full Unicode case folding and natural sorting.

### Q11: Strictness, Seams, and Error Modes
- **Option A (DuckDB Dual Cast: Implicit Null / Explicit Error):** Implicit query coercions evaluate unparseable data to `Null` (filters treat as false). Explicit casts (`CAST(field AS date)`) emit structured diagnostics with `ByteSpan` for LSP.
- **Option B (Strict Type Checking with Advisory Diagnostics):** Type mismatches emit non-blocking advisory diagnostics (the Taplo pattern); query execution treats mismatched filters as `false`.
- **Option C (Silent Fallback):** Failed coercions silently return the original string (SQLite affinity model).
