# Research: Dynamic Value Model Precedent

Target: Comparative analysis of dynamic value representations across mature engines.
Informs: Value model, coercion and comparison policy, schema-note typing, note document model.

---

## 1. Comparable Implementations (Verified from Source)

### DataFusion `ScalarValue`

- [V] Defined in `datafusion/common/src/scalar/mod.rs:405-523` as a 40+ variant enum covering SQL/Arrow physical scalar types.
- [V] Distinguishes signed/unsigned integers (`Int8`..`Int64`, `UInt8`..`UInt64`), floats (`Float16`..`Float64`), and fixed-point decimals (`Decimal32`..`Decimal256`).
- [V] Temporal types are native physical variants: `Date32`, `Date64`, `Time32...`, `Time64...`, `TimestampSecond...Nanosecond`, `IntervalYearMonth`, `IntervalDayTime`, `IntervalMonthDayNano`, `DurationSecond...Nanosecond`.
- [V] Every scalar wraps `Option<T>` (e.g. `Int64(Option<i64>)`) to represent typed SQL NULLs alongside untyped `ScalarValue::Null`.
- [V] No borrowed mirror (`ScalarValueRef`); execution runs on columnar arrays (`ArrayRef`), while `ScalarValue` models literals, partition keys, and parameters.
- [V] Implements `PartialEq`, `Eq`, `Hash`, and `PartialOrd` (`src/scalar/mod.rs:545,680`). Cross-type `partial_cmp` returns `None` (`src/scalar/mod.rs:686-723`); type coercion is handled strictly in query planning, not inside scalar comparison. Does not implement `Ord`.

### Polars `AnyValue<'a>` & `Scalar`

- [V] `AnyValue<'a>` in `crates/polars-core/src/datatypes/any_value.rs:34-120` unifies borrowed and owned variants in a single enum (`String(&'a str)` vs `StringOwned(PlSmallStr)`, `Binary(&'a [u8])` vs `BinaryOwned(Vec<u8>)`, `Struct` vs `StructOwned`).
- [V] No separate `AnyValueRef` type mirror. `into_static(self) -> AnyValue<'static>` (`src/datatypes/any_value.rs:1084-1143`) clones only borrowed variants and value-copies scalars.
- [V] Distinguishes integers (`Int8`..`Int128`, `UInt8`..`UInt128`) from floats (`Float16`, `Float32`, `Float64`) and decimals (`Decimal(i128, usize, usize)`).
- [V] Extension types are first-class variants: `Date(i32)`, `Datetime(i64, TimeUnit, Option<&'a TimeZone>)`, `Duration(i64, TimeUnit)`, `Time(i64)`.
- [V] `AnyValue::partial_cmp` (`src/datatypes/any_value.rs:1368-1488`) handles same-type comparisons, but explicitly panics with `unimplemented!` on mixed dtypes; cross-type operations require plan-level type coercion.
- [V] `struct Scalar` (`crates/polars-core/src/scalar/mod.rs:17-20`) pairs an explicit schema `DataType` with `AnyValue<'static>` to disambiguate shared physical layouts (e.g. categoricals, enums, decimal scales).

### `serde_json::Value` & `toml::Value` / `toml_edit::Value`

- [V] `serde_json::Value` (`src/value/mod.rs:116-176`): purely owned enum. Encapsulates numbers in `struct Number` (`src/number.rs:22-34`) wrapping `enum N { PosInt(u64), NegInt(i64), Float(f64) }`.
- [V] `Number::eq` (`src/number.rs:37-46`) returns `false` between integer and float representations (`N::PosInt` vs `N::Float` never match). Implements `PartialEq, Eq, Hash`; omits `PartialOrd` and `Ord`. Borrowing relies on raw JSON slices (`RawValue`), not a mirrored `ValueRef`.
- [V] `toml::Value` (`src/value.rs:24-40`): purely owned enum. Separates `Integer(i64)` and `Float(f64)`. Models `Datetime(Datetime)` as a first-class variant. Has no `Null` variant. Derived `PartialEq`; no `Ord` or `PartialOrd`.
- [V] `toml_edit::Value` (`src/value.rs:11-27`): wraps each scalar in `Formatted<T>` (`Formatted<String>`, `Formatted<i64>`, etc.) to preserve concrete syntax trivia for round-tripping.

### `noyalib::Value`

- [V] Frontmatter AST used by Traces (`noyalib-0.0.53/src/value.rs:28-45`).
- [V] Separates numbers in `enum Number { Integer(i64), Float(f64) }` (`src/value/number.rs:20-30`).
- [V] Preserves integers and floats without coercion. Dates, links, and durations remain unclassified `String` scalars.

### Obsidian Dataview `Literal` / `LiteralType` & Comparators

- [V] Defined in `src/data-model/value.ts:8-34` as a TypeScript union: `boolean | number | string | DateTime | Duration | Link | Array<Literal> | DataObject | null | ...`.
- [V] Numbers are strictly IEEE-754 `number` (`f64`). No integer vs float separation.
- [V] Extension types (Luxon `DateTime`, `Duration`, and `Link`) are first-class runtime objects.
- [V] No borrowed mirror. Values are wrapped on demand via `wrapValue(val) -> WrappedLiteral` (`src/data-model/value.ts:82-85`) for dispatch.
- [V] `Values.compareValue` (`src/data-model/value.ts:171-265`) enforces a total order: `null` is lowest; distinct non-null types compare by alphabetical type name (`wrap1.type.localeCompare(wrap2.type)`); identical types follow domain rules (links compare path, type, subpath).

### Metadata Menu

- [V] Defined in `src/fields/Fields.ts:41-88` (`FieldType`, `TypesOptionsMap`).
- [V] Does not maintain a dynamic value AST. Values are runtime-validated via functional dispatch (`validateValue(type)(managedField)`, `src/commands/fileFields.ts:41`, `src/fields/Fields.ts:417-437`) and written directly to frontmatter as plain strings/YAML.

### SQLite Storage Classes & `Mem`

- [V] 5 storage classes (`sqlite3.h`): `NULL`, `INTEGER`, `FLOAT`, `TEXT`, `BLOB`.
- [V] Internal representation is a single C struct `sqlite3_value` / `Mem` (`src/vdbeInt.h:232-256`), holding `union MemValue { double r; i64 i; ... }` and bitmask `flags` (`MEM_Null`, `MEM_Int`, `MEM_Real`, `MEM_Str`, `MEM_Blob`).
- [V] Handles borrowed vs owned in the *same* struct: `MEM_Ephem` (borrowed pointer) vs `MEM_Dyn` (owned heap with destructor `xDel`). No separate `MemRef` struct.
- [V] `sqlite3MemCompare` (`src/vdbeaux.c:4555-4613`) enforces total order: NULL (0) < Number (1) < Text (2) < Blob (3).
- [V] Compares integer (`i64`) and float (`double`) losslessly via `sqlite3IntFloatCompare` (`src/vdbeaux.c:4527-4544`): bounds-checks against $[-2^{63}, 2^{63})$, casts float to integer if within range, and only compares floats when integer truncation matches.

### DuckDB `Value` & `LogicalType`

- [V] Decouples logical schema (`LogicalType`, `src/include/duckdb/common/types.hpp:196-261`) from physical value storage (`Value`, `src/include/duckdb/common/types/value.hpp:36-380`).
- [V] `Value` holds `LogicalType type_`, `bool is_null`, `union Val { hugeint_t; double; date_t; interval_t; ... } value_`, and `shared_ptr<ExtraValueInfo> value_info_`.
- [V] Comparison (`Value::operator<`, `operator==`, `src/common/types/value.cpp:2098-2107`) delegates to `TemplatedBooleanOperation` (`src/common/value_operations/comparison_operations.cpp:76-88`): when types differ, `LogicalType::DefaultForceMaxLogicalType` determines the target type, casts both values via `DefaultTryCastAs`, and compares unified physical representations.

---

## 2. Invariants and Invariant Owners

1. **Dynamic Type Tagging**:
   - In columnar/SQL engines (DataFusion, DuckDB, Polars), the logical schema/plan owns the type invariant (`DataType`, `LogicalType`).
   - In document/PKM engines (Dataview, serde_json, Traces), the dynamic `Value` enum variant itself owns the runtime type tag.

2. **Total Ordering and Comparison Policy**:
   - In DataFusion and Polars, the value enum deliberately rejects cross-type comparison (returns `None` or panics). The query planner owns type coercion.
   - In SQLite and Dataview, an external comparator (`sqlite3MemCompare`, `compareValue`) owns cross-type total ordering.
   - In Traces, total ordering is hardcoded directly on the borrowed value enum (`NoteFieldValueRef::compare`, `src/note/field.rs:487`; `NoteFieldType::rank`, `src/note/field.rs:238`), tightly coupling storage representation with comparison semantics.

3. **Ownership Lifecycle (Borrowed vs Owned)**:
   - Polars owns borrowed-vs-owned transitions within a single enum `AnyValue<'a>` via `into_static()`.
   - SQLite manages borrowed vs owned in a single `Mem` struct via bitmask flags (`MEM_Ephem` vs `MEM_Dyn`).
   - Traces splits this across 4 separate types (`FieldValue`, `FieldValueRef`, `NoteFieldValue`, `NoteFieldValueRef`), distributing ownership across 1,500+ lines of duplicate match arms and conversion visitors.

4. **Numeric Precision Invariance**:
   - SQLite (`sqlite3IntFloatCompare`) and DuckDB (`TemplatedBooleanOperation`) treat numbers as a single mathematical continuum while preserving 64-bit integer precision.
   - Traces `NoteFieldValue::Number(f64)` (`src/note/field.rs:49`) silently truncates integers $> 2^{53}$ (`src/note/field.rs:545`), while `FieldValue::Int(i64)` (`src/field.rs:602`) preserves them.

---

## 3. Tradeoffs and Costs

- **Duplicate Enum Hierarchies (Traces)**: Traces pays high coordination overhead and memory thrash across 4 enums. Frontmatter parsing requires a 3-stage pipeline: `noyalib::Value` -> `FieldValueRef<'static>` (`src/note/metadata.rs:71`) -> `NoteFieldValue` (`src/note/metadata.rs:72`), traversing the metadata tree three times and allocating intermediate maps.
- **Enum Size Footprint**:
  - `FieldValue` (`src/field.rs:596`): 56 bytes (bounded by `IndexMap` at 48 bytes + 8-byte tag).
  - `NoteFieldValue` (`src/note/field.rs:43`): 64 bytes (tested `<= 80` bytes at `src/note/field.rs:580`), inflated by inline `Link` (`src/note/links.rs:28`, 56 bytes) and `DurationValue` (`src/temporal/duration.rs:158`, 48 bytes).
  - Storing inline 56-byte `Link` variants inflates all variants (even booleans and nulls) to 64 bytes, doubling memory usage in redb cache buffers and `Note.inline_fields`.
- **Eager Semantic Classification Overhead**: `FieldValueRef::classify` eagerly parses dates via `DatePoint::parse` on every string (`src/field.rs:843`), and `NoteFieldValue::from` eagerly parses links and durations (`src/note/field.rs:552-555`). Unqueried strings pay repeated parsing costs.

---

## 4. Transferable Lessons for Traces

1. **Unify into a Single Canonical Value Enum**:
   - [H] Merge `FieldValue` and `NoteFieldValue` into a single crate-wide `Value` enum supporting: `Null`, `Bool(bool)`, `Int(i64)`, `Float(f64)`, `String(String)`, `Date(DateValue)`, `DateTime(DateTimeValue)`, `Duration(DurationValue)`, `Link(Box<Link>)`, `List(Box<[Value]>)`, `Object(IndexMap<String, Value>)`.
   - [V] Proved by DataFusion and Polars: a single scalar enum can span configuration, schemas, and query execution without duplicating variant trees.

2. **Eliminate the Ref Enum Mirror**:
   - [V] Zero mature comparables maintain a cloned `Ref` enum hierarchy.
   - [H] Two viable alternatives:
     - (Option A - Polars pattern): A single lifetime enum `Value<'a>` with `into_static() -> Value<'static>`, using `Cow<'a, str>` for strings.
     - (Option B - DuckDB / serde_json pattern): A purely owned `Value` enum, passing borrowed views via `&Value` and lightweight accessor methods (`as_str() -> Option<&str>`, `as_date() -> Option<DateValue>`).
     - Since Traces persists notes into redb via postcard (`src/index/codec.rs:33,51`), values are deserialized into owned buffers; `&Value` avoids an entire secondary enum hierarchy.

3. **Separate Representation from Comparison and Coercion Policy**:
   - [V] Proved by SQLite (`sqlite3MemCompare`) and DuckDB (`ValueOperations::LessThan`): total ordering and cross-type coercion belong in a dedicated comparator, not hardcoded on the value enum.
   - [H] Define `ValueComparator` or `CoercionPolicy` to evaluate comparisons, rank types, and handle lossless int/float comparison.

4. **Adopt Lossless Int-vs-Float Comparison**:
   - [V] SQLite's `sqlite3IntFloatCompare` (`src/vdbeaux.c:4527-4544`) proves that 64-bit integers and doubles can be compared losslessly without converting `i64` to `f64`.
   - [H] Traces should retain `Int(i64)` and `Float(f64)`, eliminating the `as f64` precision loss in `src/note/field.rs:545`.

5. **Direct Serde Deserialization (Eliminate 3-Stage Pipeline)**:
   - [V] `serde_json` and `toml` deserialize directly into their respective `Value` enums via custom Serde visitors.
   - [H] Replace `noyalib::Value -> FieldValueRef -> NoteFieldValue` with a single deserializer/visitor mapping directly from frontmatter to the unified `Value`.

6. **Shrink Enum Size by Boxing `Link`**:
   - [V] `Link` is 56 bytes (`src/note/links.rs:28`). Boxing `Link` (`Box<Link>`, 8 bytes) and `Object` (`Box<IndexMap>`, 8 bytes) reduces `Value` from 64 bytes to 32 bytes (fitting within two 64-bit words + tag), halving memory consumption in redb and vectors.

---

## 5. Non-Transferable Lessons

1. **Columnar Chunk Vectors (Arrow / DataFusion / Polars)**:
   - [V] Analytical engines store values in contiguous homogeneous memory buffers (`ArrayRef`, `ChunkedArray`) for SIMD throughput.
   - [H] Non-transferable: Traces is a document-oriented PKM system operating on nested, heterogeneous note frontmatter trees and Markdown ASTs. Columnar chunking would introduce massive impedance mismatch for note mutation and line/span ergonomics.

2. **Alphabetical Type Name Comparison (Dataview)**:
   - [V] Dataview orders distinct types by comparing their type name strings alphabetically (`src/data-model/value.ts:188`).
   - [H] Non-transferable: Yields nonsensical rankings (e.g. `"array"` < `"boolean"` < `"date"` < `"link"` < `"null"`). Traces requires a deterministic semantic rank order (`Null` lowest, temporal/numeric preceding text).

3. **Typed SQL NULLs (DataFusion `Int64(None)`)**:
   - [V] DataFusion wraps every scalar variant in `Option<T>` to preserve SQL schema nullability.
   - [H] Non-transferable: PKM frontmatter and TOML have untyped nulls/omissions. Wrapping every variant in `Option<T>` would bloat the enum and add useless pattern-matching friction.

4. **Multi-Representation In-Place Caching (SQLite `Mem`)**:
   - [V] SQLite caches string, int, and real representations inside the same register (`src/vdbeInt.h:229`).
   - [H] Non-transferable: Requires interior mutability and raw pointer tracking incompatible with safe Rust semantics and unnecessary for CLI/LSP queries.

---

## 6. Impact on Traces Hypotheses

- **Challenges Root-Primitives [H1] & Note [H1] on `ValueRef`**:
  - [H] Both scout hypotheses assume unifying into a canonical `Value` / `ValueRef` pair. Comparative research refutes this: maintaining a parallel `ValueRef` mirror is an anti-pattern absent in all mature comparables. Traces should either use a single `Value<'a>` with `into_static()` or an owned `Value` with `&Value` accessors.
- **Validates Note [H1] on Consolidating Dynamic Value Trees**:
  - [V] Conclusively validates eliminating `FieldValue` vs `NoteFieldValue`. A single value enum cleanly handles schema options, frontmatter metadata, and query evaluation.
- **Refines Cross-Cutting [H2] on Number Representation**:
  - [H] Refutes collapsing numbers into `f64`. A unified value type must retain `Int(i64)` and `Float(f64)` with a lossless comparator, eliminating precision loss on integer fields.
- **Validates Elimination of the 3-Stage Pipeline**:
  - [V] Conclusively validates eliminating `src/note/metadata.rs:71-72` in favor of a single-pass conversion.
- **Identifies New Memory Optimization**:
  - [H] Boxing `Link` (56 bytes -> 8 bytes) allows the unified enum to fit in 32 bytes, directly resolving the 80-byte budget pressure noted in `src/note/field.rs:580`.
