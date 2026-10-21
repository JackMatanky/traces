# Traces Codebase Inventory: Value Coercion, Classification, and Comparison

- **Topic:** Exhaustive inventory and empirical behavior matrix of typing, coercion, and comparison in the Traces codebase.
- **Status:** Complete.
- **Target File:** `.scratch/arch-design/research/19-prep/a-code-inventory.md`.

---

## 1. Inventory Table

| Site (`file:line`) | Owner Module | Operation / Mechanism | Strictness | Inputs | Outputs | Callers |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `src/field.rs:776-797` [V] | Schema / Legacy Field | `From<noyalib::Value> for FieldValueRef` | Lenient / Silent | `noyalib::Value` | `FieldValueRef<'static>` | `RawFrontmatter::parse` (`src/note/metadata.rs:71`) [V] |
| `src/field.rs:802-810` [V] | Schema / Legacy Field | `DateTimeCoercion` (`Classify` vs `Passthrough`) | Mode flag | Config enum | Steering | `FieldStringValue::classify`, `FieldValueRef::deserialize_with` [V] |
| `src/field.rs:821-854` [V] | Schema / Legacy Field | `FieldStringValue::classify` | Silent sniffing | `Cow<'a, str>`, `coercion` | `Date`, `DateTime`, or `String` | `From<noyalib::Value>` [V: 781], `FieldValueRefVisitor` [V: 928] |
| `src/note/metadata.rs:56-74` [V] | Note / Metadata | `RawFrontmatter::parse` | Lenient | Raw YAML string | `Frontmatter` (`IndexMap<FieldKey, NoteFieldValue>`) | `Frontmatter::from(&RawFrontmatter)` [V: 181] |
| `src/note/field.rs:532-572` [V] | Note / Field | `From<FieldValueRef> for NoteFieldValue` | Silent coercion | `FieldValueRef` | `NoteFieldValue` (int->f64, empty str->null, wikilink->Link, duration->Duration) | `RawFrontmatter::parse` [V: 72] |
| `src/note/parser/inline.rs:39-86` [V] | Note / Inline Parser | `parse_inline_value` | Lenient fallback | `&str` (trimmed) | `NoteFieldValue` (atom, list of atoms, or fallback string) | `lexer::body_field_callback` [V: 181], `wrapped_field_callback` [V: 221] |
| `src/note/parser/inline.rs:131-141` [V] | Note / Inline Parser | `parse_atom_at` | Strict grammar | `pos: usize` in `&str` | `Option<ParsedAtom>` (quote-string, link, duration, bool, null, date, number, tag) | `InlineValueParser::parse` [V: 72] |
| `src/note/parser.rs:106-115, 271-278` [V] | Note / Markdown Parser | `parse_markdown` (pulldown-cmark wikilinks) | Destructive rewrite | Markdown source string | `Note` (bare `[[x]]` stripped from paragraph body into `outlinks`; leaves bare text in inline fields) | `IndexerService::refresh_note`, `test_support::parse_note` [V] |
| `src/note/links.rs:27-34` [V] | Note / Links | `Link` struct definition | Derived Eq | `target`, `text`, `kind`, `embedded` | `Link` | `Link::new`, `Link::parse_wikilink_prefix` [V] |
| `src/note/links.rs:74-101` [V] | Note / Links | `Link::parse_wikilink_prefix` | Strict syntax | `&str` | `Option<(Link, usize)>` | `Link::parse_wikilink` [V: 63], `inline.rs:parse_link_at` [V: 173] |
| `src/note/field.rs:42` [V] | Note / Field | `NoteFieldValue::PartialEq` | Strict structural | `&NoteFieldValue`, `&NoteFieldValue` | `bool` (derived `PartialEq`, exact variant match only) | Rust compiler, test assertions [V] |
| `src/note/field.rs:208-251` [V] | Note / Field | `NoteFieldType::rank` | Strict lattice | `NoteFieldType` | `u8` (`Null:0 < Bool:1 < Num:2 < Dur:3 < Temp:4 < Text:5 < Link:6 < List:7 < Obj:8`) | `NoteFieldValueRef::compare` [V: 310], `SortKey::cmp` [V: 437] |
| `src/note/field.rs:286-312` [V] | Note / Field | `NoteFieldValueRef::compare` | Heuristic total | `&self`, `&other` | `Ordering` (ambient-zone Date->DateTime; Rank for cross-kind; Link target+text only) | `NoteFieldValueRef::PartialEq` [V: 467], `CompareOp::is_satisfied_by` [V: 807] |
| `src/note/field.rs:393-428` [V] | Note / Field | `NoteFieldValueRef::is_equal_to_literal` | Strict (cross-kind false except Date/DT) | `&self`, `&NoteFieldValue` | `bool` (Float exact `==`, Date/DT local midnight, Link structural `Link::eq`) | `QueryFieldValueRef::is_equal_to_literal` [V: 108], `CompareOp::is_satisfied_by` [V: 792] |
| `src/note/field.rs:434-447` [V] | Note / Field | `NoteFieldValueRef::is_containing` | Heuristic string/tag | `&self`, `&NoteFieldValue` | `bool` (List item/tag match, substring for text) | `QueryFieldValueRef::is_containing` [V: 143] |
| `src/query/value.rs:106-135` [V] | Query / Value | `QueryFieldValueRef::is_equal_to_literal` | Specialized | `&self`, `&NoteFieldValue` | `bool` (delegates to note_ref; tags/inlinks match lists of strings) | `CompareOp::is_satisfied_by` [V: 792] |
| `src/query/sort.rs:351-401` [V] | Query / Sort | `SortKey::from_value_ref` | Heuristic sniff | `QueryFieldValueRef` | `SortKey` (sniffs Date/DT/Duration from String; List/Obj->Null; Link->Text) | `SortOrder::sort` [V: 147] |
| `src/query/sort.rs:425-439` [V] | Query / Sort | `SortKey::cmp` | Total order | `&self`, `&other` | `Ordering` (`Null < Bool < Number < Duration < Temporal < Text`) | `SortOrder::sort` [V: 147] |
| `src/query/sort.rs:462-476` [V] | Query / Sort | `TextShape::classify` | Silent sniffing | `&str` | `TextShape` (`DateTime`, `Date`, `Duration`, or `Plain`) | `SortKey::from_text` [V: 394], `ComparisonExpr::classify_literal` [V: 242] |
| `src/query/grammar/filter.rs:238-248` [V] | Query / Filter Grammar | `ComparisonExpr::classify_literal` | Silent promotion | `NoteFieldValue` | `NoteFieldValue` (`Date`, `DateTime`, `Duration`, or untouched `String`) | `FilterGrammar::parse_literal_arg` [V: 1013] |
| `src/query/grammar/filter.rs:353-458` [V] | Query / Filter Grammar | `ArithOp::evaluate` | Strict typed | `self`, `left: &NoteFieldValue`, `right` | `Option<NoteFieldValue>` (evaluates Date±Duration, Date-Date, Dur±Dur, Dur*Num; else Null) | `ValueExpr::evaluate` [V: 295] |
| `src/query/grammar/filter.rs:785-838` [V] | Query / Filter Grammar | `CompareOp::is_satisfied_by` | Split: Eq strict, Relational ranked | `self`, `field`, `literal` | `bool` (`==`/`!=` uses `is_equal_to_literal`; Null in `<`/`>` is false; else `compare()`) | `ComparisonExpr::is_matching` [V: 217, 225] |
| `src/query/plan.rs:254-257` [V] | Query / Execution Plan | `QueryTransform::GroupBy` | Delegates to Sort | `rows: Vec<QueryRow>`, `path` | `Vec<QueryRow>` (applies ascending `SortOrder`) | `ExecutionPlan::apply` [V: 55] |
| `src/query/plan.rs:259-269` [V] | Query / Execution Plan | `QueryTransform::Flatten` | Strict list unwrapping | `rows: Vec<QueryRow>`, `path` | `Vec<QueryRow>` (unpacks `NoteFieldValue::List` items) | `ExecutionPlan::apply` [V: 55] |
| `src/temporal/date.rs:195-217` [V] | Temporal / Date | `Serialize` / `Deserialize for DateValue` | Text encoding | `DateValue` / Deserializer | `YYYY-MM-DD` string round-trip | Postcard index persistence (`src/index/store.rs:27`) [V] |
| `src/temporal/datetime.rs:218-220` [V] | Temporal / DateTime | `DateTimeValue::is_midnight_of` | Ambient zone | `self`, `DateValue` | `bool` (promotes Date to local midnight in process zone, checks instant equality) | `NoteFieldValueRef::is_equal_to_literal` [V: 408, 413] |
| `src/temporal/datetime.rs:329-346` [V] | Temporal / DateTime | `TryFrom<DateValue> for DateTimeValue` | Ambient zone | `DateValue` | `Result<DateTimeValue, DateError>` (converts calendar date to local midnight via `local_zone()`) | `NoteFieldValueRef::compare` [V: 297, 299], `SortKey::from_note_ref` [V: 376] |
| `src/temporal/datetime.rs:359-387` [V] | Temporal / DateTime | `Serialize` / `Deserialize for DateTimeValue` | Text encoding (RFC 3339 `...Z`) | `DateTimeValue` / Deserializer | Frozen UTC timestamp string (`...Z`) round-trip | Postcard index persistence (`src/index/store.rs:27`) [V] |
| `src/temporal/duration.rs:83-91, 541-562` [V] | Temporal / Duration | `DurationValue::nominal_seconds`, `PartialEq`, `Ord` | Nominal second equivalence | `DurationValue` | `nominal_seconds: i64`, `bool`, `Ordering` (`1mo == 30d == 2592000s`, `1y == 365d`) | `NoteFieldValueRef::compare` [V: 294], `SortKey::cmp` [V: 434] |
| `src/temporal/duration.rs:621-640` [V] | Temporal / Duration | `Serialize` / `Deserialize for DurationValue` | Text encoding | `DurationValue` / Deserializer | Canonical duration string (`4y 6w`) round-trip | Postcard index persistence (`src/index/store.rs:27`) [V] |
| `src/temporal/point.rs:555-610` [V] | Temporal / DatePoint | `DifferenceEngine` | Multi-regime | `DatePoint`, `DatePoint`, `TimeUnit` | `DateDiff` (calendar discrete, civil clock fractional, physical elapsed) | `DatePoint::diff` [V: 419, 427, 434, 442] |
| `src/temporal/zone.rs:13-20` [V] | Temporal / Zone | `local_zone()` | Global ambient | System discovery / `current_override()` | `Result<TimeZone, DateError>` (`TimeZone::try_system`) | `DateTimeValue::parse_iso` [V: 151], `TryFrom<DateValue>` [V: 342] |
| `src/schema/fields/number.rs:12-41` [V] | Schema / Fields | `SchemaNumberField` | Passive bounds | `min: Option<f64>`, `max`, `step` | Stored bounds (no runtime note validation) | `SchemaFieldBuilder::build` [V] |
| `src/schema/fields/select.rs:28-40, 134-185` [V] | Schema / Fields | `SchemaSelectFieldEntry` | Passive literals | `value: FieldValue`, `label: FieldValue` | Stored entries (strings/objects; no runtime note link checking) | `SchemaFieldBuilder::build` [V], `engine/schema.rs` [V] |
| `src/template/engine/query.rs:594-610` [V] | Template Engine | `field_value(NoteFieldValue) -> minijinja::Value` | Type loss | `NoteFieldValue` | minijinja `Value` (Duration->str, Date->str, DateTime->str, Link->target str) | Template rendering expressions (`src/template/engine.rs`) [V] |
| `src/index/inlinks.rs:319-418` [V] | Index / Inlinks | `LinkResolver::resolve` | Exact case-sensitive | Link `target`, indexed `FileMeta` paths | `Option<Target>` (exact path, `.md` append, nearest basename tie-break) | `InlinkMap::with_resolver` [V: 85] |
| `src/file.rs:303-334` [V] | Index / Core | `BaseNameRef` | Exact case-sensitive | `&Path` | Borrowed file stem `&str` (derived `Eq, Hash, PartialEq`, no Unicode/case fold) | `LinkResolver` [V: 331], `SchemaName` [V: 89] |
| `src/cli/table.rs:27` [V] | CLI / Table | `--filter <EXPR>` args | Passthrough string | CLI strings | Raw strings passed directly to `QueryBuilder::filter` | `cli::dispatch` [V] |

---

## 2. Current Behavior by Question (Q1..Q11)

### Q1: Where does string -> Date/DateTime/Duration promotion live?

- **Current Behavior:** Promotion is split across THREE uncoordinated seams [V]:
  1. *Frontmatter ingestion:* Two-stage pipeline: `FieldStringValue::classify` (`src/field.rs:843`) [V] sniffs ISO dates/datetimes into `FieldValueRef::Date`/`DateTime`. Then `From<FieldValueRef> for NoteFieldValue` (`src/note/field.rs:548-558`) [V] trims remaining strings and promotes empty strings to `Null`, wikilinks to `Link`, and duration strings (`DurationValue::parse`) to `Duration`.
  2. *Query filter building:* `ComparisonExpr::classify_literal` (`src/query/grammar/filter.rs:238`) [V] runs `TextShape::classify` on quoted string filter literals, promoting them to `Date`, `DateTime`, or `Duration`.
  3. *Sort-key building:* `SortKey::from_value_ref` (`src/query/sort.rs:393`) [V] runs `TextShape::classify` on all `NoteFieldValue::String` values at sort time, promoting any date/datetime/duration-shaped string on the fly.
- **Viability of Lexical Storage with Typing-on-Demand:** Lexical storage is technically viable for query execution, but would require centralizing `TextShape::classify` behind typed view accessors [H]. Currently, frontmatter eagerly caches typed enums into postcard index storage, meaning promotion is already executed and frozen at parse time [V: `src/temporal/datetime.rs:365`].
- **Owner(s):** Frontmatter classifier (`src/field.rs`), Note value conversion (`src/note/field.rs`), Query filter classifier (`src/query/grammar/filter.rs`), Query sort classifier (`src/query/sort.rs`).

### Q2: Frontmatter literal-string escape

- **Current Behavior:** Frontmatter has NO literal-string escape mechanism [V]. Noyalib parses YAML `title: "2025-01-15"` and `title: 2025-01-15` into identical `noyalib::Value::String("2025-01-15")` [V]. `FieldStringValue::classify` (`src/field.rs:843`) [V] treats all strings identically regardless of source quoting, eagerly coercing them into `DateValue`. Inline fields, by contrast, treat quotes (`"..."`) as an explicit escape via `parse_quoted_string_at` (`src/note/parser/inline.rs:148`) [V].
- **Owner(s):** Note frontmatter parser (`src/note/metadata.rs`), `From<noyalib::Value>` (`src/field.rs`).

### Q3: Int vs Float semantic equality, ordering, NaN, -0.0, i64 > 2^53

- **Current Behavior:**
  - *Current code state:* `NoteFieldValue` has only `Number(f64)` [V: `src/note/field.rs:49`]. Frontmatter `FieldValueRef::Int(i)` eagerly converts via `i as f64` [V: `src/note/field.rs:545`].
  - *Large integers (> 2^53):* Precision loss occurs immediately at ingestion: `9_007_199_254_740_993` (`2^53 + 1`) is cast to `9007199254740992.0` [V: empirical probe].
  - *Signed zero:* `-0.0 == +0.0` is `true` in `PartialEq` and `is_equal_to_literal` [V]. In `compare()`, `normalize_zero(n)` (`src/note/field.rs:495`) adds `+0.0`, ensuring `-0.0` and `+0.0` produce `Ordering::Equal` [V].
  - *NaN:* `NaN == NaN` is `false` in `PartialEq` and `is_equal_to_literal` (IEEE 754 float comparison) [V]. But in `compare()`, `normalize_zero(NaN).total_cmp(&normalize_zero(NaN))` evaluates to `Ordering::Equal` [V].
- *Ticket 18 handoff (`Int(i64)` and `Float(f64)`):* In `FieldValueRef`, derived `PartialEq` evaluates `Int(1) == Float(1.0)` as `false` [V]. No cross-variant integer/float comparator currently exists in `NoteFieldValue` [V].
- **Owner(s):** Note field model (`src/note/field.rs`), legacy field model (`src/field.rs`).

### Q4: Date vs DateTime equality/order, naive zone-shifting, ambient zone, cache determinism

- **Current Behavior:**
  - *Naive zone-shifting:* Naive datetimes (e.g., `2026-07-29T12:00:00`) resolve against the ambient `local_zone()` at parse time (`src/temporal/datetime.rs:151`) [V] and convert to UTC.
  - *Cache freezing:* Postcard cache serializes `DateTimeValue` as an explicit RFC 3339 `...Z` string (`src/temporal/datetime.rs:370`) [V]. Once written, the UTC instant is permanently frozen in cache.
  - *Date to DateTime promotion:* `NoteFieldValueRef::compare` (`src/note/field.rs:297-300`) [V] and `SortKey::from_note_ref` (`src/query/sort.rs:376`) [V] promote bare `Date` to `DateTime` using `DateTimeValue::try_from(date)`, which looks up the CURRENT runtime `local_zone()` (`src/temporal/datetime.rs:342`) [V].
  - *Non-determinism:* If a note is indexed in UTC, its midnight datetime is `00:00:00Z`. If queried on a machine in `America/Los_Angeles` (UTC-7), `Date(2026-07-29)` promotes to `2026-07-29T07:00:00Z` (local midnight). Consequently, `Date` compares `Greater` than `DateTime(2026-07-29T00:00:00)` for the SAME calendar day [V: empirical probe line 25].
  - *Equality:* Derived `NoteFieldValue::PartialEq` returns `false` between `Date` and `DateTime` (different variants) [V]. But `is_equal_to_literal` calls `is_midnight_of` (`src/temporal/datetime.rs:218`) [V], which also promotes `Date` to local midnight and returns `true` if the instant matches [V].
- **Owner(s):** Temporal module (`src/temporal/datetime.rs`, `src/temporal/zone.rs`), Note field comparator (`src/note/field.rs`).

### Q5: Cross-kind rank order and equality; sort vs filters

- **Current Behavior:** Two completely divergent relations govern sorting and filtering [V]:
  - *Rank Lattice:* `Null(0) < Bool(1) < Number(2) < Duration(3) < Temporal(4) < Text(5) < Link(6) < List(7) < Object(8)` (`src/note/field.rs:238`) [V].
  - *Filter Equality (`==`, `!=`):* Strictly typed via `is_equal_to_literal` (`src/note/field.rs:393`) [V]. Mismatched kinds ALWAYS return `false` (e.g. `42 == "42"` is `false`), with the sole exception of `Date` vs `DateTime` at local midnight [V].
  - *Filter Ordering (`<`, `<=`, `>`, `>=`):* Poisoned on `Null` (`if lhs.is_null() || rhs.is_null() { return false; }`, `src/query/grammar/filter.rs:802, 827`) [V]. For non-null mismatched kinds, filters fall back to `compare()` and evaluate relations according to the rank lattice! For example, `42 < "hello"` evaluates to `true` (rank 2 < rank 5); `true < 42` evaluates to `true` (rank 1 < rank 2) [V: empirical probe lines 40, 48].
  - *Sort Order (`SortKey::cmp`):* `Null` sorts smallest [V: `src/query/sort.rs:428`]. `SortKey` maps `List` and `Object` to `SortKey::Null` (rank 0), and maps `Link` to `SortKey::Text(target)` (rank 5) [V: `src/query/sort.rs:380-385`].
- **Owner(s):** Note field comparator (`src/note/field.rs`), Query filter operator evaluation (`src/query/grammar/filter.rs`), Query sort key (`src/query/sort.rs`).

### Q6: Link equality/order, embedded, display, textual vs resolved

- **Current Behavior:**
  - *Link struct:* `Link { target: String, text: Option<String>, kind: LinkType, embedded: bool }` (`src/note/links.rs:28`) [V]. Derived `PartialEq` compares ALL four fields [V].
  - *`compare()` ordering:* Compares `target` first, then `text` (`src/note/field.rs:303`) [V]. It COMPLETELY IGNORES `kind` and `embedded`!
  - *`is_equal_to_literal`:* Uses derived `Link::eq` [V: `src/note/field.rs:404`].
  - *Embed divergence:* `[[target]]` vs `![[target]]` evaluates to `compare() == Ordering::Equal`, but `PartialEq` returns `false` and `is_equal_to_literal` returns `false` [V: empirical probe lines 30, 35].
  - *Link vs String:* `compare()` places `Link` (rank 6) strictly above `Text` (rank 5) [V]. `is_equal_to_literal` returns `false` [V].
  - *Case and Extension:* Link comparison is byte-exact case-sensitive UTF-8; `[[target]]` != `[[TARGET]]` and `[[target]]` != `[[target.md]]` [V: empirical probe lines 31, 32].
- **Owner(s):** Link model (`src/note/links.rs`), Note field comparator (`src/note/field.rs`).

### Q7: Schema select string vs note Link matching

- **Current Behavior:** Schema select entries store values as `FieldValue::String(s)` (`src/schema/fields/select.rs:158`) [V]. Traces currently has NO runtime note validation against schemas [V: codebase grep confirmed zero calls]. When schema field options are exposed to templates (`src/template/engine/schema.rs:194`) [V], entries are exposed as strings or maps. Note fields containing `NoteFieldValue::Link` cannot match schema `FieldValue::String` under standard equality without an explicit conversion [H].
- **Owner(s):** Schema select field (`src/schema/fields/select.rs`), Template schema bridge (`src/template/engine/schema.rs`).

### Q8: Query literal promotion and sort-time sniffing

- **Current Behavior:**
  - *Query literal promotion:* `ComparisonExpr::classify_literal` (`src/query/grammar/filter.rs:238`) [V] promotes quoted string filter literals matching ISO date, datetime, or duration shapes into typed values once at query parse time.
  - *Sort-time sniffing:* `SortKey::from_value_ref` (`src/query/sort.rs:393`) [V] calls `TextShape::classify(&s)` on ANY string field. If a note stored `v_str: "2026-07-29"` as text, sorting by `v_str` opportunistically promotes it to `SortKey::DateTime` [V].
- **Owner(s):** Query filter parser (`src/query/grammar/filter.rs`), Query sort key (`src/query/sort.rs`).

### Q9: Duration comparison (Span + i64, calendar units)

- **Current Behavior:** `DurationValue` implements `PartialEq` and `Ord` strictly by `self.nominal_seconds() == other.nominal_seconds()` (`src/temporal/duration.rs:544, 560`) [V].
  - `TimeUnit` defines fixed nominal conversion ratios (`src/temporal/unit.rs:194-224`) [V]: `1y = 31_536_000s (365d)`, `1mo = 2_592_000s (30d)`, `1w = 604_800s (7d)`, `1d = 86_400s`, `1h = 3_600s`, `1m = 60s`.
  - Nominal equality holds universally: `30d == 1mo`, `1h == 60m`, `365d == 1y` [V: empirical probe lines 26-28].
  - Application vs comparison dual regime: Although `1mo == 30d` in comparison, applying `1mo` to a `DatePoint` uses Jiff's calendar arithmetic (`src/temporal/mod.rs:18`) [V], which respects variable month lengths.
- **Owner(s):** Temporal duration (`src/temporal/duration.rs`, `src/temporal/unit.rs`).

### Q10: String collation (case sensitivity, Unicode normalization)

- **Current Behavior:** String comparison in `NoteFieldValueRef::compare` [V: `src/note/field.rs:301`], `SortKey::cmp` [V: `src/query/sort.rs:436`], and `is_equal_to_literal` [V: `src/note/field.rs:402`] delegates entirely to standard Rust `String::cmp` and `==`.
  - Collation is byte-for-byte UTF-8 code point comparison [V].
  - It is strictly case-sensitive (`"a" != "A"`) [V].
  - Zero Unicode normalization is performed (NFC and NFD strings compare unequal) [V].
- **Owner(s):** Note field (`src/note/field.rs`), Query sort (`src/query/sort.rs`).

### Q11: Strictness-by-seam spread and error mode on failed coercion

- **Current Behavior across Four Seams:**
  1. *Note Parsing Boundary (Seam 1):* Silent, lossy coercion. Empty strings silently coerce to `Null` [V: `src/note/field.rs:551`]; invalid numbers fallback to strings; integer values lose precision to `f64` [V: `src/note/field.rs:545`].
  2. *Core Value Model (Seam 2):* Derived structural `PartialEq` is strict; `NoteFieldValueRef::compare` is heuristic and total [V].
  3. *Query Evaluation (Seam 3):* `TRY_CAST` / null-propagation semantics. Mismatched operands in arithmetic return `NoteFieldValue::Null` [V: `src/query/grammar/filter.rs:371`]. `==` on mismatched types returns `false` [V]. Missing fields evaluate to `Null` [V: `src/query/results.rs:210`].
  4. *Template Engine Boundary (Seam 4):* Degrades complex values to strings (`field_value`, `src/template/engine/query.rs:594-610`) [V]. `Null` becomes minijinja `none` [V: 596].
- **Owner(s):** Divergence register (`.scratch/temporal-core/divergence-register.md`), Query filter (`src/query/grammar/filter.rs`), Template engine (`src/template/engine/query.rs`).

---

## 3. Empirical Probe Matrix

Observed directly via in-crate test execution (`cargo test --lib note::field::tests::empirical_probe -- --nocapture`) [V]:

| Probe Category | Expression / Operands | `PartialEq` | `compare()` | `is_equal_to_literal` | Filter Match Result (`QueryBuilder`) | Notes / Root Cause |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Int vs Float** | `FieldValueRef::Int(1)` vs `Float(1.0)` | `false` | N/A | N/A | N/A | Derived structural enum `PartialEq` fails across variants [V]. |
| **Int vs Float** | `FieldValue::Int(1)` vs `Float(1.0)` | `false` | N/A | N/A | N/A | Schema literal enum fails across variants [V]. |
| **Float NaN** | `NaN` vs `NaN` | `false` | `Equal` | `false` | N/A | `normalize_zero` + `total_cmp` returns `Equal`, violating `PartialEq` [V]. |
| **Float Zero** | `-0.0` vs `+0.0` | `true` | `Equal` | `true` | N/A | `normalize_zero` (`n + 0.0`) canonicalizes `-0.0` to `+0.0` [V]. |
| **Int Precision** | `9007199254740993` (`2^53+1`) as `f64` | `false` | N/A | N/A | N/A | Evaluates to `9007199254740992.0` (precision truncated in f64) [V]. |
| **Date vs DateTime** | `Date(2026-07-29)` vs `DT(midnight)` (UTC) | `false` | `Equal` | `true` | N/A | `Date` promoted to local midnight; matches UTC instant [V]. |
| **Date vs DateTime** | `Date(2026-07-29)` vs `DT(noon)` (UTC) | `false` | `Less` | `false` | N/A | Midnight orders before noon on same calendar day [V]. |
| **Date vs DateTime** | `Date(2026-07-29)` vs `DT(midnight)` (LA) | `false` | `Equal` | `true` | N/A | When both created and compared in LA, matches [V]. |
| **Date vs DateTime** | `Date` (UTC) vs `DT(midnight)` (LA zone shift) | `false` | `Greater` | `false` | N/A | **Non-deterministic:** LA midnight is 7h later than UTC midnight [V]! |
| **Duration Equiv** | `30d` vs `1mo` | `true` | `Equal` | N/A | N/A | Both evaluate to `2_592_000` nominal seconds [V]. |
| **Duration Equiv** | `1h` vs `60m` | `true` | `Equal` | N/A | N/A | Both evaluate to `3_600` nominal seconds [V]. |
| **Duration Equiv** | `1y` vs `365d` | `true` | `Equal` | N/A | N/A | Both evaluate to `31_536_000` nominal seconds [V]. |
| **Link Display** | `[[target]]` vs `[[target\|alias]]` | `false` | `Greater` | `false` | N/A | `text` field (`None` vs `Some("alias")`) breaks equality [V]. |
| **Link Embed** | `[[target]]` vs `![[target]]` | `false` | `Equal` | `false` | N/A | **Disagreement:** `compare()` ignores `embedded`; `PartialEq` checks it [V]! |
| **Link Case** | `[[target]]` vs `[[TARGET]]` | `false` | `Greater` | `false` | N/A | Case-sensitive UTF-8 byte comparison ("t" > "T") [V]. |
| **Link Extension** | `[[target]]` vs `[[target.md]]` | `false` | `Less` | `false` | N/A | Lexical string comparison; does not strip `.md` in Link struct [V]. |
| **Filter Num vs Str** | `v == "42"` (where `v: 42`) | N/A | N/A | `false` | `[]` (no match) | Filter equality is strictly typed; number never equals string [V]. |
| **Filter Num vs Str** | `v != "42"` | N/A | N/A | `true` | All rows match | Inverse of strictly typed equality [V]. |
| **Filter Num vs Str** | `v < "42"` | N/A | `Less` | N/A | Matches `bool, date, dt, dur, num` | Rank fallback: Number(2), Bool(1), Temporal(4) < Text(5) [V]! |
| **Filter Num vs Str** | `v > "42"` | N/A | `Greater` | N/A | Matches `link, list, obj, str` | Rank fallback: Link(6), List(7), Object(8) > Text(5) [V]! |
| **Filter Num vs Num** | `v == 42`, `v < 100`, `v > 10` | N/A | Value | Value | Matches `n_num` | Homogeneous numeric comparisons succeed as expected [V]. |
| **Filter Dur vs Num** | `v == 3600` (where `v: 1h`) | N/A | N/A | `false` | `[]` (no match) | Duration never equals number under filter equality [V]. |
| **Filter Dur vs Num** | `v > 3600` (where `v: 1h`) | N/A | `Greater` | N/A | Matches `dur, date, dt, str, link, ...` | Rank fallback: Duration(3) > Number(2) [V]! |
| **Filter Bool vs Num** | `v == 1` (where `v: true`) | N/A | N/A | `false` | `[]` (no match) | Bool never equals number under filter equality [V]. |
| **Filter Bool vs Num** | `v < 1` (where `v: true`) | N/A | `Less` | N/A | Matches `n_bool` | Rank fallback: Bool(1) < Number(2) [V]! |
| **Filter Null Poison** | `v == null` | N/A | N/A | `true` | Matches `n_null` and missing field | Null matches null and missing fields [V]. |
| **Filter Null Poison** | `v < 5`, `v <= 5`, `v > 5` (null row) | N/A | N/A | N/A | `n_null` NEVER matches | **Null poison:** relational operators on Null always return `false` [V]. |
| **Mixed-Kind Sort** | `SortOrder::single("v", Ascending)` | N/A | Rank/Value | N/A | Order: `[List, Null, Obj] < Bool < Num < Dur < Temp < Text < Link]` | `SortKey` coerces `List` and `Object` to `Null`! Coerces `Link` to `Text` [V]. |

---

## 4. Disagreement Cases between `PartialEq` and `compare() == Equal`

The codebase exhibits THREE distinct semantic equality evaluators:

1. `NoteFieldValue::PartialEq` (derived structural equality) [V: `src/note/field.rs:42`].
2. `NoteFieldValueRef::compare(&self, &other) == Ordering::Equal` (total ordering comparator) [V: `src/note/field.rs:286`].
3. `NoteFieldValueRef::is_equal_to_literal(&self, literal)` (query filter `==` evaluator) [V: `src/note/field.rs:393`].

Empirically verified cases where these disagree [V]:

1. **`f64::NAN` vs `f64::NAN`:**
   - `NoteFieldValue::PartialEq`: `false` (IEEE 754: `NaN != NaN`).
   - `is_equal_to_literal`: `false`.
   - `compare() == Ordering::Equal`: `true` (`normalize_zero` + `f64::total_cmp`).
2. **`Date` vs `DateTime` (at exact local midnight):**
   - `NoteFieldValue::PartialEq`: `false` (distinct enum variants `NoteFieldValue::Date` vs `NoteFieldValue::DateTime`).
   - `is_equal_to_literal`: `true` (checks `DateTimeValue::is_midnight_of`).
   - `compare() == Ordering::Equal`: `true` (`DateTimeValue::try_from(date)` local promotion).
3. **`Link` (regular `[[target]]`) vs `Link` (embed `![[target]]`):**
   - `NoteFieldValue::PartialEq`: `false` (`Link::eq` checks `self.embedded == other.embedded`).
   - `is_equal_to_literal`: `false` (delegates to `Link::eq`).
   - `compare() == Ordering::Equal`: `true` (`NoteFieldValueRef::compare` compares `target()` and `text()` only; `embedded` is completely ignored).
4. **`SortKey` Sort Order vs `NoteFieldValueRef::compare` Order:**
   - In `NoteFieldValueRef::compare`, `List` has rank 7 and `Object` has rank 8 (sorting after text and links) [V: `src/note/field.rs:248-249`].
   - In `SortKey::from_note_ref` (`src/query/sort.rs:383-385`), `List` and `Object` are mapped to `SortKey::Null`! Consequently, in real query sorting, notes with list or object values sort at the VERY TOP alongside nulls [V: empirical probe line 36].

---

## 5. Implications for Ticket 19

### Q1: Promotion Seam & Lexical Storage

- **Option 1A (Keep as is):** Two-stage ingestion (Noyalib -> `FieldValueRef` -> `NoteFieldValue`), query literal sniffing, and sort-time sniffing. *Risk:* Triplicated classifiers (`src/field.rs`, `src/query/sort.rs`, `src/query/grammar/filter.rs`), high conversion overhead (41% parse time) [P].
- **Option 1B (Single Parser Boundary):** Parse directly from Noyalib into `NoteFieldValue` using a unified parser order. Eliminate `TextShape::classify` sniffing at sort time. Strings remain strings unless explicitly cast in queries [P].
- **Option 1C (Lexical Storage with Typing-on-Demand):** Keep all note values as raw strings/spans and classify lazily when evaluated by queries. *Constraint:* Rejected by Ticket 18 Answer (Ticket 18 established owned enum `NoteFieldValue` with postcard cache) [V: `.scratch/arch-design/issues/18-value-model.md:447`].

### Q2: Frontmatter Literal Escape

- **Option 2A (Keep as is):** Frontmatter is quote-insensitive. `title: "2025-01-15"` is always parsed as `Date`. Escape requires query-side `string(title)` cast function [V].
- **Option 2B (Schema-Bound Pass):** If a schema declares `type = "string"`, suppress date coercion for that field [P].
- **Option 2C (Custom Frontmatter String Marker / Tag):** Adopt a convention (e.g. `!str 2025-01-15` or Serde visitor tracking quotes). *Constraint:* Ticket 18 Answer already rejected CST / quote-tracking Serde visitors due to Noyalib AST constraints [V: `.scratch/arch-design/issues/18-value-model.md:467`].

### Q3: Int vs Float Equality and Ordering

- **Option 3A (Structural Separation):** Derived `PartialEq` treats `Int(1) != Float(1.0)`. *Risk:* Highly unintuitive for users (`rating == 1.0` fails if frontmatter has integer `1`) [P].
- **Option 3B (Semantic Numeric Equality):** Custom `PartialEq` or explicit numeric comparator: if both are numbers, promote `Int` to `Float` if within `[-2^53, 2^53]`, or compare with exact mixed integer/float math. Preserve exact integer equality for `i64 > 2^53` [P].
- **Option 3C (Total IEEE-754 Normalization):** `NaN == NaN` is false in filters, but ordered deterministically at the top or bottom of numeric sort keys [P].

### Q4: Date vs DateTime Determinism & Ambient Zone

- **Option 4A (Keep as is):** Date promotes to DateTime at local midnight using ambient OS zone. *Risk:* Verified non-deterministic across machines and zones (empirical probe line 25: `Date` compares `Greater` to same-day `DateTime` when zone shifts) [V].
- **Option 4B (Execution Context Injection):** As decided in Ticket 18, ambient zone must be passed explicitly via an execution context rather than querying global `local_zone()` [V: `.scratch/arch-design/issues/18-value-model.md:546`].
- **Option 4C (Calendar Day Timeline Separation):** Treat `Date` and `DateTime` as strictly separate domains, or promote `Date` to UTC midnight `00:00:00Z` unconditionally for comparison purposes, guaranteeing machine-independent total order [P].

### Q5: Sort Order vs Filter Relation

- **Option 5A (Keep as is):** `==` is strictly typed (false for cross-kind), while `<`/`>` falls back to rank ordering (`42 < "hello"` is true). *Risk:* Extremely confusing SQL/filter semantics; ordering behaves differently from equality [P].
- **Option 5B (SQL / DuckDB Strict Filter Relations):** Relational operators (`<`, `<=`, `>`, `>=`) on mismatched types evaluate to `false` (or 3-valued `Null`). Cross-kind rank order is strictly confined to `ORDER BY` / `SortKey` [P].
- **Option 5C (Fix SortKey List/Object Bug):** Align `SortKey` with `NoteFieldType::rank` so that `List` and `Object` do not silently sort as `Null` at the top of query outputs [P].

### Q6: Link Equality and Ordering

- **Option 6A (Canonical Target Equality):** Align `Link::eq` and `compare()`: two links are equal if their canonical normalized targets match. `embedded` and `display` are metadata attributes, not identity [P].
- **Option 6B (Strict Identity):** Two links are equal only if `target`, `display`, and `embedded` match. Update `compare()` so it does not falsely return `Equal` for embed vs non-embed [P].

### Q7: Schema Select String vs Note Link Matching

- **Option 7A (String Coercion at Boundary):** When validating a note `Link` against a schema select string, compare `link.target()` to the select string [P].
- **Option 7B (Ticket 20 Wikilink Parse):** As handed off by Ticket 18, schema select strings that look like links are parsed via the shared wikilink parser at the consumer boundary [V: `.scratch/arch-design/issues/18-value-model.md:481`].

### Q8: Elimination of Sort Sniffing

- **Option 8A (Keep as is):** `SortKey::from_value_ref` continues to sniff date/duration text shape on all strings [V].
- **Option 8B (No Implicit Sort Promotion):** Strings sort strictly as strings. Dates sort as dates. Users desiring date sorting on text fields must use typed fields or explicit query functions [P].

### Q9: Duration Comparison Anchor

- **Option 9A (Keep Nominal Conversion):** Keep `1mo == 30d` and `1y == 365d` for nominal comparisons and sorting. Retain dual regime where calendar arithmetic uses Jiff spans [V].
- **Option 9B (Pure Elapsed Ordering):** Compare durations strictly by elapsed seconds; reject calendar unit equivalence without an explicit reference anchor date [P].

### Q10: String Collation Policy

- **Option 10A (Keep Byte-Exact):** Keep standard Rust `&str` code point ordering. Fast, zero-allocation, deterministic [V].
- **Option 10B (Unicode NFC Normalization):** Normalize strings to NFC on ingestion or comparison to prevent cross-platform file/text mismatch [P].

### Q11: Unified Error Mode on Coercion Failure

- **Option 11A (Silent Null / False - DuckDB TRY_CAST):** Failed conversions or invalid arithmetic silently evaluate to `Null` or `false` without terminating queries [P].
- **Option 11B (Diagnostic Reporting):** Return `Null` for evaluation, but collect non-fatal query warnings for type mismatches, matching the schema warning doctrine [P].
