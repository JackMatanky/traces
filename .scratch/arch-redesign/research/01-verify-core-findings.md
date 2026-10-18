# Research Verification: Value, Note, Schema, Config, and Temporal Findings

**Baseline commit**: `0c43149da740fd5d7102a067b8d08ff759ba830b` on `main` (working tree clean for `src/`).  
**Active in-flight worktree**: `.worktrees/config-digest-sections` (`49c5ac39e3c759d68bde58f0751bb8b0ca0b6cad`, branch `config-digest-sections`).  
**Scope**: Factual verification ledger of area survey claims across note parsing, value representations, schema, configuration, temporal handling, ambient statics, and root primitives. Planning-only; zero production code or test changes.

---

## 1. Verification Ledger Summary

| # | Topic | Verdict | Key Finding / Correction |
|---|---|---|---|
| 1 | **note/index → config edges & ParserInput** | `VERIFIED` | `src/note/parser/input.rs:9` is the sole non-test `note -> config` edge. `src/index/service.rs:22-23, 42-51` is the sole non-test `index -> config` edge (`IndexerService::from(&Config)`). `IndexerService::parse_note` (`src/index/service.rs:270`) is the only production caller constructing `MarkdownParserInput`. |
| 2 | **Value trees & conversion chains** | `VERIFIED` | Consumers mapped across all 8 types; `FieldStringValue` is strictly private to `src/field.rs` (zero external consumers); frontmatter chain `noyalib::Value -> FieldValueRef -> NoteFieldValue` confirmed (`src/note/metadata.rs:71-72`); `DateTimeCoercion::Classify` is enabled **only** in `FieldValueRef::from(noyalib::Value)` (`src/field.rs:772`), while serde deserializers always use `Passthrough`; conversions are lossy for `Int -> f64` (clippy suppression), month/year precision (truncated to day 01), durations (normalized to nominal seconds / stringified in templates), and links (markdown links remain strings in frontmatter, aliases dropped in sorting and templates). |
| 3 | **Temporal coupling & ambient zone** | `VERIFIED` | `QuerySet` holds **no** `TimeZone` (`src/query/results.rs:464-468`); `compare_temporal` does **not** exist; `local_zone()` is called at 24 non-test sites across `temporal`, `query::grammar::filter`, and `template::engine::date`; sort reaches `local_zone()` once per row during projection, and once per comparison in cross-kind `Date` vs `DateTime` tie-breaking; filter expressions reach `local_zone()` once per candidate row; `ZoneGuard` overrides required in 5 test suites; `field.rs` and `file.rs` import `temporal` for ISO date classification and filename/metadata calendar day extraction. |
| 4 | **Schema ↔ note decoupling & types** | `VERIFIED` | `src/schema/` has zero references to `note` types; runtime validator of notes against schemas does **not** exist anywhere in the repo; `SchemaDateField.format` does **nothing** in production (getter is `#[cfg(test)]`); 4 taxonomies confirmed (`RawSchemaFieldType` [6], `SchemaFieldTypeTag` [6], `SchemaFieldType` [6], `NoteFieldType` [9]); schema types map 1:1 among themselves, but have **zero** mapping to `NoteFieldType`. |
| 5 | **Config sections, hashing & drift** | `VERIFIED` / `MOVING (07/17)` | Production consumers mapped per section; in `frontmatter`, only `tags_name()` is read in production (`title`, `aliases`, `date_created`, `date_modified` are completely unread); `Config::hash()` is unused outside tests (`model.rs:246-253`); on `main` (`0c43149d`), no per-section digest or redb index fingerprint exists; in active worktree `.worktrees/config-digest-sections` (`49c5ac39`), Issue 07 has implemented `ConfigDigests` with per-setting digests, to be followed by Issue 17's 8th `meta` table and `IndexFingerprint`; adding a new `tasks` setting touches 4-5 files. |
| 6 | **Note parse pipeline & GLOSSARY claims** | `PARTIAL` | `parse_markdown` runs pulldown-cmark twice (`src/note/parser.rs:109, 122` with different options); list-item text is lexed at least 3 times; all spans (`ByteSpan`) are discarded (only `SourceLine` numbers retained); `Note` is serialized via `postcard` into redb `NOTES` table; GLOSSARY claims of heading slug dedup at parse time, footnote nodes, and callout nodes are **REFUTED** (none exist in code). |
| 7 | **Ambient statics & env reads** | `VERIFIED` | Process-global statics enumerated across `dirs.rs`, `env_vars.rs`, `yaml.rs`, `temporal`, `position`, and `cli/cwd.rs`; `std::env::*` reads outside `cli` exist **only** in `dirs.rs:193` and `env_vars.rs:9, 28`; test override mechanisms verified (`dirs::HOME` hardcoded compile-time via `#[cfg(test)]`, `ZoneGuard` via static RwLock, others fixed on first access). |
| 8 | **Root-primitive homes & single consumers** | `PARTIAL` (see Errata) | Consumers and module counts mapped across all 9 primitives; strictly single-consumer primitives: `delimiter` (only `src/note/`), `lexer` (only `src/query/grammar/`), and `dirs` (only `src/config/` + companion `file_tracker.rs`); survey claim that `strsim`, `yaml`, `env_vars`, and `hash` are single-consumer is **REFUTED** (all have 2+ consuming modules). |

---

## 2. Detailed Verification Ledger

### Claim 1: note → config edge, index → config edge, and MarkdownParserInput callers

- **Status**: `VERIFIED`
- **Evidence**:
  - `src/note/parser/input.rs:9`: `use crate::config::{FrontmatterConfig, TaskConfig};` is the **only** non-test `note -> config` edge in the entire `src/note/` tree. `MarkdownParserInput<'a>` (`input.rs:25-34`) borrows `tasks: &'a TaskConfig` and `frontmatter: &'a FrontmatterConfig`. Static defaults `DEFAULT_TASK_CONFIG` and `DEFAULT_FRONTMATTER_CONFIG` (`input.rs:11-17`) are strictly scoped to `#[cfg(any(test, feature = "test-utils"))]`.
  - All other references to `config` in `src/note/` are confined to unit tests: `src/note/parser.rs:620, 632, 1769, 1794` (inside `#[cfg(test)] mod tests`) and `src/note/parser/task.rs:869, 942` (inside `#[cfg(test)] mod tests`).
  - `src/index/service.rs:22-23, 42-51`: `use crate::{Config, ..., TaskConfig, config::FrontmatterConfig};` is the **only** non-test `index -> config` edge in the entire `src/index/` tree. `IndexerService::from(&Config)` extracts:
    1. `root: config.root().to_path_buf()` (`service.rs:46`)
    2. `tasks: config.tasks().clone()` (`service.rs:47`)
    3. `frontmatter: config.frontmatter().clone()` (`service.rs:48`)
    4. `class_field: config.schemas().class_field_name().to_owned()` (`service.rs:49`)
    No other file in `src/index/` (`store.rs`, `tables.rs`, `refresh.rs`, `inlinks.rs`, `sort.rs`, `trie.rs`, `delta.rs`, `codec.rs`, `entry.rs`, `error.rs`) imports or touches `config`.
  - `src/index/service.rs:270`: `IndexerService::parse_note` is the **only production caller** constructing `MarkdownParserInput`:

    ```rust
    let input = MarkdownParserInput::new(
        file.path(),
        &content,
        &self.tasks,
        &self.frontmatter,
    );
    ```

    All other constructors use `MarkdownParserInput::for_test` (`src/note/parser/input.rs:59`), which is guarded by `#[cfg(any(test, feature = "test-utils"))]` and called exclusively by test helpers (`src/lib.rs:558` `test_support::parse_note`), benchmarks (`benches/memory_footprint.rs:82, 103`), doctests (`src/note/parser.rs:39, 99`), and parser unit tests.

---

### Claim 2: Value Trees, Conversion Chains, DateTimeCoercion, and Lossy Paths

- **Status**: `VERIFIED`
- **Evidence**:
  - **Production Consumers by Type**:
    1. `FieldValue` (`src/field.rs:586`): `src/field.rs` (definition, serde Serialize/Deserialize, `From<FieldValueRef>`); `src/schema/` (`raw.rs:59, 149, 245`, `fields/builder.rs:216`, `fields/parser.rs:58, 150, 190, 239`, `fields/date.rs:43`, `fields/file.rs:87`, `fields/number.rs:67`, `fields/select.rs:24, 61, 136-138`) for raw schema field options and `SchemaSelectFieldEntry`; `src/template/engine/schema.rs:67, 258` for converting select entries to minijinja `Value` via `Value::from_serialize(&object)`.
    2. `FieldValueRef` (`src/field.rs:707`): `src/field.rs` (definition, serde visitor/seed, `From<noyalib::Value>`); `src/note/field.rs:24, 532` (`From<FieldValueRef<'_>> for NoteFieldValue`); `src/note/metadata.rs:12, 71` (`RawFrontmatter::parse`).
    3. `DateTimeCoercion` (`src/field.rs:793`): `src/field.rs` only (lines 772, 793, 830, 848, 866, 966, 982).
    4. `FieldStringValue` (`src/field.rs:809`): Strictly internal to `src/field.rs` (lines 771, 809, 898, 903, 907). **Zero external consumers** in the entire codebase.
    5. `NoteFieldValue` (`src/note/field.rs:43`): `src/note/field.rs` (definition, `as_str`, `as_date`, `as_date_point`, `as_ref`); `src/note/metadata.rs:97` (`Frontmatter.fields`); `src/note/parser/inline.rs` (`InlineValueParser`); `src/note/model.rs:31` (`Note.inline_fields`); `src/index/tables.rs`, `src/index/store.rs` (serialized inside `Note`); `src/query/value.rs:23, 32` (`QueryFieldValueRef::Owned`); `src/query/` (`eval.rs`, `filter.rs`, `row.rs`, `sort.rs`); `src/template/engine/query.rs:77, 598` (`field_value(NoteFieldValue) -> minijinja::Value`); `src/cli/` (`table.rs`, `show.rs`).
    6. `NoteFieldValueRef` (`src/note/field.rs:179`): `src/note/field.rs` (definition, `kind()`, `rank()`, `compare()`, `append_text()`); `src/query/value.rs:14, 20, 97` (`QueryFieldValueRef::Note`); `src/query/results.rs:28, 210, 243, 250...` (projecting file/list fields); `src/query/sort.rs:23, 370` (`SortKey::from_note_ref`); `src/query/grammar/filter.rs:26, 802`.
    7. `NoteFieldType` (`src/note/field.rs:209`): `src/note/field.rs:209, 235, 254` (definition, total rank order: `Null < Bool < Number < Duration < Temporal < Text < Link < List < Object`); `src/query/sort.rs:22, 403, 414` (`SortKey::kind()` and `SortKey::rank()`). Re-exported in `src/note/mod.rs:47` and `src/lib.rs:155` for crate-internal use. **Zero other consumers**.
    8. `QueryFieldValueRef` (`src/query/value.rs:18`): `src/query/value.rs` (definition, `to_owned_value`, `append_text`, `text`); `src/query/row.rs` (`field(&self, path) -> QueryFieldValueRef`); `src/query/results.rs` (resolving metadata, file, list fields); `src/query/sort.rs:344` (`SortKey::from_value_ref`); `src/query/grammar/filter.rs` (filter evaluation); `src/template/engine/query.rs` (query row evaluation).
  - **Frontmatter Conversion Chain**:
    `src/note/metadata.rs:67-73` confirms:

    ```rust
    let parsed = yaml::parse(self.as_str())?; // -> noyalib::Value
    let field_value = FieldValueRef::from(raw_value); // -> FieldValueRef
    fields.insert(key, NoteFieldValue::from(field_value)); // -> NoteFieldValue
    ```

    The conversion chain is byte-for-byte `noyalib::Value -> FieldValueRef -> NoteFieldValue`.
  - **Distinct Conversion Paths from YAML, TOML, JSON**:
    1. *YAML*:
       - Note frontmatter: `crate::yaml::parse` (`src/yaml.rs:28`) -> `noyalib::Value` -> `FieldValueRef` (`src/field.rs:759`) -> `NoteFieldValue` (`src/note/field.rs:532`).
       - Template filter `from_yaml`: `crate::yaml::parse` (`src/template/engine/yaml.rs:67`) -> `noyalib::Value` -> `Value::from_serialize(&parsed)`.
       - Template filter `to_yaml`: minijinja `Value` -> `noyalib::to_string` (`src/template/engine/yaml.rs:42`).
    2. *TOML*:
       - Config: `toml::from_str` (`src/config/raw.rs`) -> `RawConfig`.
       - Schemas: `toml::from_str` (`src/schema/raw.rs`) -> `RawSchemaFieldDef`, where `options` deserializes via `FieldValue::deserialize` (`src/field.rs:686`), which delegates to `FieldValueRef::deserialize_with(..., DateTimeCoercion::Passthrough)`.
       - Schema select values files: `toml::from_str::<RawSchemaSelectFieldValues>` (`src/schema/fields/select.rs:275`) -> `entries.into_iter().map(FieldValue::from)`.
    3. *JSON*:
       - Schema select values files: `serde_json::from_str::<RawSchemaSelectFieldValues>` (`src/schema/fields/select.rs:277`) -> `entries.into_iter().map(FieldValue::from)`.
    4. *Minijinja Value Conversion*:
       - Query rows: `field_value(NoteFieldValue) -> Value` (`src/template/engine/query.rs:598-614`).
       - Schema select options: `Value::from_serialize(&object)` where `object` is `IndexMap<String, FieldValue>` (`src/template/engine/schema.rs:261`).
  - **Where DateTimeCoercion::Classify is actually enabled**:
    `DateTimeCoercion::Classify` is enabled in **exactly one place in production**: `impl From<noyalib::Value> for FieldValueRef<'static>` at `src/field.rs:772` (`FieldStringValue::new(Cow::Owned(s)).classify(DateTimeCoercion::Classify)`).
    In contrast, the serde deserialization path for `FieldValueRef` (`src/field.rs:966`) and `FieldValue` (`src/field.rs:691`) hardcodes `DateTimeCoercion::Passthrough`! `FieldValueRef::deserialize_with(..., DateTimeCoercion::Classify)` is called **only in unit tests** (`src/field.rs:1490, 1512`).
  - **Lossy Conversions**:
    1. `Int -> f64`: `FieldValueRef::Int(i) => NoteFieldValue::Number(i as f64)` (`src/note/field.rs:545`) uses `i as f64` (clippy `cast_precision_loss` suppressed). Large 64-bit integers (> 53 bits) lose precision. Similarly, YAML numbers in `FieldValueRef::from(noyalib::Value)` convert unconditionally via `n.as_f64()` (`src/field.rs:770`), losing integer identity before `NoteFieldValue` is even constructed. `FileField::Size` casts `u64` to `f64` (`src/query/results.rs:269`).
    2. *Month/Year Precision*: `From<DatePoint> for NoteFieldValue` (`src/note/field.rs:161-173`) explicitly documents: *"dropping the written precision"*; `DatePoint::Calendar { date, .. }` drops month/year precision, defaulting day to 01. `FieldStringValue::classify` (`src/field.rs:835-838`) also discards `precision`.
    3. *Durations*: `DurationValue::parse` normalizes to rounded nominal seconds (`src/note/field.rs:56-58`). In minijinja conversion (`src/template/engine/query.rs:604`), durations lose their type and become plain strings (`dv.to_string()`).
    4. *Links*: In `From<FieldValueRef> for NoteFieldValue` (`src/note/field.rs:552`), only wikilinks are parsed (`Link::parse_wikilink`); standard Markdown links `[text](url)` in frontmatter remain plain strings! In `SortKey` (`src/query/sort.rs:380-382`) and minijinja (`src/template/engine/query.rs:607`), link text and aliases are dropped, retaining only the target string.

---

### Claim 3: Temporal Coupling and Ambient Zone on `main`

- **Status**: `VERIFIED`
- **Evidence**:
  - `QuerySet` (`src/query/results.rs:464-468`):

    ```rust
    pub struct QuerySet {
        base: Arc<Vec<QueryRow>>,
        plan: ExecutionPlan,
        cache: std::sync::OnceLock<Arc<Vec<QueryRow>>>,
    }
    ```

    `QuerySet` holds **no `TimeZone`** field whatsoever. Neither does `ExecutionPlan` (`src/query/plan.rs:33`) or `QueryRow` (`src/query/results.rs:33`).
  - `compare_temporal` does **not exist** anywhere in `src/` (0 occurrences across git grep).
  - **Non-test call sites of `local_zone()`** (total 24 direct sites):
    - `src/temporal/datetime.rs:128`: `DateTimeValue::from_local`
    - `src/temporal/datetime.rs:192`: `DateTimeValue::local_datetime`
    - `src/temporal/datetime.rs:342`: `impl TryFrom<DateValue> for DateTimeValue` (local-midnight promotion)
    - `src/temporal/point.rs:780`: `local_zone_for_calendar`
    - `src/query/grammar/filter.rs:386, 399, 411, 435, 640, 646, 673, 699, 761` (9 call sites in filter evaluation: date addition, subtraction, duration diff, startOf, endOf, shift, and component extraction)
    - `src/template/engine/date.rs:121, 132, 184, 254, 346, 449, 474, 697, 799, 928, 987` (11 call sites in template date functions and filters)
  - **Indirect callers reaching `local_zone()`**:
    - `src/file.rs:97`: `FileMeta::derive_day` calls `DateValue::try_from(created_at)`, which calls `DateTimeValue::try_from(time)?.local_date()` (`src/temporal/date.rs:189-191`), reaching `local_zone()`.
    - `src/note/field.rs:297, 299`: `NoteFieldValueRef::compare` promotes cross-kind `Date` vs `DateTime` via `DateTimeValue::try_from(*a)`, reaching `local_zone()`.
    - `src/query/sort.rs:376`: `SortKey::from_note_ref` for `Date` values calls `DateTimeValue::try_from(value)`, reaching `local_zone()`.
  - **Other ambient clock/`now` reads**:
    - `src/template/engine/date.rs:162`: `jiff::Timestamp::now()` in `DateAnchor::evaluate` (`date.now()`, `date.today()`, `date.tomorrow()`, `date.yesterday()`).
    - `src/template/engine/date.rs:1010`: `jiff::Timestamp::now()` in `is_past` filter.
    - `src/template/engine/date.rs:1026`: `jiff::Timestamp::now()` in `is_future` filter.
  - **Frequency of `local_zone()` in Sort/Filter Paths**:
    - In `SortKey` sorting: called **once per row** during pre-projection for `Date` values (`src/query/sort.rs:376`). However, if two values compare via `NoteFieldValueRef::compare` (tie-breaking or unindexed sort), comparing a `Date` and a `DateTime` invokes `DateTimeValue::try_from` inside the comparison closure, reaching `local_zone()` **once per comparison** (`O(N log N)` times).
    - In filter evaluation: called **once per row evaluated** for any filter containing temporal arithmetic, shift, startOf, endOf, or component extraction (`filter.rs:386-761`).
  - **Where `ZoneGuard`-style test overrides are required**:
    - `src/temporal/` unit suites (`datetime.rs:55, 621, 745...`, `date.rs:1033, 1050`, `point.rs:789, 885...`, `zone.rs:42, 54, 69`, `mod.rs:79, 93`).
    - `src/note/field.rs:923, 1041`: `a_bare_date_promotes_to_local_zone_midnight_for_ordering`.
    - `src/query/grammar/filter.rs:1340, 1561, 1609`: testing date filtering across midnight and pinned zones.
    - `src/template/engine/date.rs:1134, 1139, 1346, 1360, 1374, 1454, 1468, 1490, 1506, 1623, 1754, 1768, 1784, 1799, 1993, 2175, 2268`.
    - `tests/integration/template_render.rs:13, 513, 532, 552`: integration tests verifying template rendering under injected time zones.
  - **Why `field.rs` and `file.rs` import `temporal`**:
    - `src/field.rs:39`: imports `DatePoint` (used in `FieldStringValue::classify` to parse ISO dates/times during deserialization) and `DateValue`, `DateTimeValue` (variants of `FieldValue` and `FieldValueRef`).
    - `src/file.rs:34, 36`: imports `DatePoint` (used in `derive_day` via `DatePoint::parse_prefix` to extract leading date from note basename) and `DateValue` (stored in `FileMeta.day`).

---

### Claim 4: Schema ↔ Note Decoupling, Runtime Validators, Date Format, and Taxonomies

- **Status**: `VERIFIED`
- **Evidence**:
  - `src/schema/` references to `note` types: **Zero**. No file in `src/schema/` imports `note`, `crate::note`, or any type from `src/note/`.
  - **Runtime validator of notes against schemas**: Does **not exist anywhere**. Neither CLI (`src/cli/`), template engine (`src/template/`), nor indexer (`src/index/`) validates note fields against a schema. The schema subsystem is used solely for schema inheritance/resolution, exposing schema metadata into templates (`schema.get`), and resolving file field queries into source selectors.
  - **`SchemaDateField.format` behavior**: Does **nothing in production**. `SchemaDateField` (`src/schema/fields/date.rs:12`) stores `format: Option<String>`. It is parsed from TOML (`date.rs:48`), but its only accessor `pub(crate) fn format(&self) -> Option<&str>` is marked `#[cfg(test)]` (`date.rs:20`). It is dead configuration, never read by production code.
  - **The Four Type Taxonomies & Mappings**:
    1. `RawSchemaFieldType` (`src/schema/raw.rs:204`): 6 variants (`Input`, `Select`, `Boolean`, `Number`, `Date`, `File`).
    2. `SchemaFieldTypeTag` (`src/schema/fields.rs:241`): 6 variants (identical names). Mapped 1:1 from `RawSchemaFieldType` via `From` (`src/schema/fields.rs:250`).
    3. `SchemaFieldType` (`src/schema/fields.rs:150`): 6 variants carrying resolved option structs. Maps to `SchemaFieldTypeTag` via `kind(&self)` (`src/schema/fields.rs:215`).
    4. `NoteFieldType` (`src/note/field.rs:209`): 9 variants (`Null`, `Bool`, `Number`, `Duration`, `Temporal`, `Text`, `Link`, `List`, `Object`). Used exclusively for canonical sorting rank in `note::field` and `query::sort`.
    - **Mapping between Schema types and NoteFieldType**: **Zero mapping exists**. The schema type system and note field type system are completely disjoint.

---

### Claim 5: Config Sections, Fingerprints, and Tasks Setting Blast Radius

- **Status**: `VERIFIED` / `MOVING (07/17)`
- **Evidence**:
  - **Production Consumers by Section**:
    1. `root`: CLI commands (`src/cli/index.rs:39`, `list.rs:62, 84`, `table.rs:61, 83`, `task.rs:192, 216`, `mod.rs:342, 423, 476, 491`); `IndexerService` (`src/index/service.rs:46`); `TemplateEngine` (`src/template/engine.rs:125`); `TemplateService` (`src/template/service.rs:228`); `ConfigService` (`src/config/service.rs:222`).
    2. `templates`: `src/template/loader.rs:313-314` (`local_template_dir`, `global_template_dir`); `src/template/service.rs:294` (`output_dir`); `src/template/writer.rs:16, 324` (`output_dir`).
    3. `schemas`: CLI (`src/cli/mod.rs:477, 492` for `class_field_name` and `directory`); `IndexerService` (`src/index/service.rs:49` for `class_field_name`); `TemplateEngine` (`src/template/engine.rs:126, 131` for `class_field_arc` and `directory`).
    4. `frontmatter`: `IndexerService` (`src/index/service.rs:48`) clones and passes to `MarkdownParserInput`. Inside `src/note/parser.rs:381`, **only `tags_name()` is read**. All other frontmatter fields (`title_name`, `aliases_name`, `date_created`, `date_modified`) have zero production callers outside tests!
    5. `tasks`: `IndexerService` (`src/index/service.rs:47`) clones and passes to `MarkdownParserInput`. Inside `src/note/parser.rs:527-528`, `tag_filters()` and `statuses()` are read during list-item parsing. CLI task commands execute queries over the index and do not read `config.tasks()`.
  - **`Config::hash()` and Drift Status**:
    - `src/config/model.rs:246-253`: `Config::hash()` is explicitly annotated:

      ```rust
      #[cfg_attr(
          not(test),
          expect(
              dead_code,
              reason = "no current caller outside tests; reserved for future index cache invalidation"
          )
      )]
      pub(crate) fn hash(&self) -> Blake3Digest
      ```

    - On `main` (`0c43149d`), `Config::hash()` is unused outside tests. No per-section digest exists, and redb stores only `CACHE_FORMAT_VERSION = b"1"` (`src/index/store.rs:38`).
    - *MOVING (07/17)*: Active in-flight implementation in worktree `.worktrees/config-digest-sections` (`49c5ac39`) introduces `src/config/model/digest.rs` (`ConfigDigests` with per-setting digests for root, templates, schemas, frontmatter, tasks), recomposing `Config::hash()` from bundles. Issue 17 will wire this into index-owned `IndexFingerprint` and an 8th `meta` redb table.
  - **Files Changed to Add One New `tasks` Setting**:
    Minimum **4 to 5 files**:
    1. `src/config/raw.rs`: add raw field with serde default to `RawTaskConfig`.
    2. `src/config/model.rs`: add resolved field to `TaskConfig`, getters, defaults, and feed into hasher/digest.
    3. `src/config/builder.rs`: validate and merge local/global raw values into `TaskConfig`.
    4. `src/note/parser/input.rs`: expose accessor if new setting is read by parser.
    5. `src/note/parser/task.rs` or `list.rs`: consume the new setting during parsing.

---

### Claim 6: Note Parse Pipeline, Double-Pass pulldown-cmark, Spans, and GLOSSARY Claims

- **Status**: `PARTIAL`
- **Evidence**:
  - **Two Passes of pulldown-cmark**: `VERIFIED`. `parse_markdown` (`src/note/parser.rs:106-115`):
    - First pass: `Parser::new_ext(input.src(), MARKDOWN_OPTIONS).into_offset_iter()` (`parser.rs:109`), where `MARKDOWN_OPTIONS` enables YAML metadata, wikilinks, and footnotes (`parser.rs:68-70`).
    - Second pass: `collect_headings(input.src(), &ctx.line_index)` (`parser.rs:113`), which creates a second parser `Parser::new_ext(src, HEADING_OPTIONS).into_offset_iter()` (`parser.rs:122`), where `HEADING_OPTIONS` adds `ENABLE_MATH` (`parser.rs:73`).
  - **List-Item Text Lexing Frequency**: `VERIFIED`. List-item text is lexed at least **3 times**:
    1. `pulldown_cmark::Parser` emits text/code/tag events.
    2. `super::lexer::scan_metadata(&text, mode)` in `ItemFrame::flush_scan_buffer` (`src/note/parser/list.rs:402`) tokenizes the scan buffer for tags and inline fields.
    3. `super::task::TaskScan::scan` in `ItemFrame::finish` (`src/note/parser/list.rs:420`), which calls `tokenize_item_text(raw, TaskFieldEmojis::Include)` (`src/note/parser/task.rs:33`), tokenizing the raw display text a third time.
    (Additionally, `InlineValueParser::parse` parses each inline field value).
  - **Spans Discarded**: `VERIFIED`. `ByteSpan` is used only transiently during event dispatch (`src/note/parser.rs:111, 241`). In `Note` (`src/note/model.rs:24-33`), zero byte spans are retained. `Heading` (`src/note/heading.rs:13`) and `ListItem` (`src/note/lists.rs:45`) retain only 1-indexed `SourceLine` numbers.
  - **Redb Serialization**: `VERIFIED`. `Note` derives `Serialize, Deserialize` (`src/note/model.rs:23`). In `src/index/codec.rs:33`, `encode_row` uses `postcard::to_extend(value, mem::take(buf))`. `IndexStore` writes this byte buffer directly into the `NOTES` table (`src/index/store.rs:1416, 1565`).
  - **GLOSSARY Claims Checked Against Code**:
    1. *Heading slug dedup at parse time*: **REFUTED**. `src/note/GLOSSARY.md:50` claims: *"Duplicate slugs within a note are deduplicated at parse time (`slug-1` suffixes)"*. In code, `Heading` has only `level`, `text`, and `line` (`src/note/heading.rs:10-14`). Slugs are **not computed, not stored, and not deduplicated** at parse time anywhere in `src/note/`.
    2. *Footnote Reference & Footnote Definition nodes*: **REFUTED**. `src/note/GLOSSARY.md:84-96` claims footnote reference and definition nodes are parsed and cross-referenced by the index. In code, `Event::FootnoteReference` simply appends raw text to the scan buffer (`src/note/parser.rs:254`), and no footnote AST nodes or fields exist in `Note`.
    3. *Callout nodes*: **REFUTED**. `src/note/GLOSSARY.md:98-105` defines Callout nodes (`> [!TYPE] content`). In code, **zero** callout types or handlers exist in `src/`.

---

### Claim 7: Ambient Statics and Environment Variable Reads

- **Status**: `VERIFIED`
- **Evidence**:
  - **Enumerate LazyLock / static Process-Global State**:
    - `src/dirs.rs`:
      - `static HOME: LazyLock<PathBuf>` (lines 62, 69, 82)
      - `pub(crate) static CONFIG_HOME: LazyLock<PathBuf>` (lines 92, 103, 116)
      - `static STATE_HOME: LazyLock<PathBuf>` (lines 127, 139, 152)
      - `static TRACES_STATE_DIR: LazyLock<PathBuf>` (line 162)
      - `pub(crate) static TRACKED_CONFIGS: LazyLock<StateDirRoot>` (line 177)
      - `pub(crate) static TRUSTED_CONFIGS: LazyLock<StateDirRoot>` (line 189)
    - `src/env_vars.rs`:
      - `pub(crate) static CEILING_DIRS: LazyLock<Vec<PathBuf>>` (line 8)
      - `pub(crate) static IGNORED_DIRS: LazyLock<HashSet<String>>` (line 20)
    - `src/yaml.rs`:
      - `static YAML_CONFIG: LazyLock<ParserConfig>` (line 18)
    - `src/temporal/`:
      - `static DATETIME_PARSER: DateTimeParser` (`datetime.rs:26`)
      - `static PERIOD_EDGE_LOOKUP: phf::Map` (`calendar.rs:80`)
      - `static UNIT_LOOKUP: phf::Map` (`unit.rs:47`)
      - `static TEMPORAL_UNITS: [UnitSpec; 7]` (`unit.rs:191`)
      - `static ZONE_OVERRIDE: RwLock<Option<TimeZone>>` (`mod.rs:251`, test support)
      - `static ZONE_LOCK: Mutex<()>` (`mod.rs:254`, test support)
      - `thread_local! static ZONE_GUARD: RefCell<Option<ZoneGuard>>` (`mod.rs:247`, test support)
    - `src/note/parser/input.rs`:
      - `static DEFAULT_TASK_CONFIG: LazyLock<TaskConfig>` (`input.rs:12`, test support)
      - `static DEFAULT_FRONTMATTER_CONFIG: LazyLock<FrontmatterConfig>` (`input.rs:16`, test support)
    - `src/cli/cwd.rs`:
      - `static CWD_TEST_LOCK: Mutex<()>` (`cwd.rs:56`, test support)
  - **`std::env::*` reads outside `src/cli/`**:
    Exist in **only two files**:
    1. `src/dirs.rs:193`: `std::env::var_os(key)` inside `non_empty_var` (reads `HOME`, `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`, `XDG_CONFIG_HOME`, `APPDATA`, `XDG_STATE_HOME`, `LOCALAPPDATA`, `TRACES_STATE_DIR`).
    2. `src/env_vars.rs:9, 28`: `std::env::var_os("TRACES_CEILING_DIRS")` and `std::env::var("TRACES_IGNORED_DIRS")`.
  - **Consumers and Test Overrides**:
    - `HOME`: Overridden at compile time via `#[cfg(test)] static HOME = LazyLock::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test"))` (`src/dirs.rs:81-83`). No runtime test override exists.
    - `CONFIG_HOME`, `STATE_HOME`, `CEILING_DIRS`, `IGNORED_DIRS`: Zero runtime test override mechanism; cached on first access for the duration of the test process.
    - `TRACKED_CONFIGS`, `TRUSTED_CONFIGS`: Tests bypass statics by constructing explicit `StateDirRoot` fixtures (`src/file_tracker.rs`).
    - `ZONE_OVERRIDE`: Overridden dynamically in tests via `ZoneGuard::set(&str)` (`src/temporal/mod.rs:98-120`).

---

### Claim 8: Root-Primitive Homes and Consumer Topology

- **Status**: `VERIFIED`
- **Evidence**:
  - **Consumer Census for 9 Root Primitives**:
    1. `strsim` (`src/strsim.rs`): Consumed by **2 modules** (`src/query/grammar/field.rs:24, 147` for query field typo suggestions; `src/schema/model.rs:6, 142` for schema field typo suggestions). **Not single-consumer**.
    2. `yaml` (`src/yaml.rs`): Consumed by **2 modules** (`src/note/metadata.rs:12, 58` for frontmatter parsing; `src/template/engine/yaml.rs:67` for template `from_yaml` filter). **Not single-consumer**.
    3. `dirs` (`src/dirs.rs`): Consumed by **1 subsystem** (`src/config/discovery.rs:46`, `src/config/tracker.rs:26`, `src/config/trust.rs:174`, plus helper `src/file_tracker.rs:20`). Subsystem-bound to `config`.
    4. `env_vars` (`src/env_vars.rs`): Consumed by **2 modules** (`src/config/discovery.rs:45, 158` for ceiling/ignored directories; `src/index/service.rs:328` for scanner directory pruning). **Not single-consumer**.
    5. `delimiter` (`src/delimiter.rs`): Consumed by **1 module** (`src/note/links.rs:303` for wikilink closing; `src/note/parser/lexer.rs:209` for inline field closing). **Single-consumer** (`note`).
    6. `dirtree` (`src/dirtree.rs`): Consumed by **5 modules** (`file_tracker.rs:20, 149`, `src/config/discovery.rs:17, 157`, `src/index/service.rs:22, 327`, `src/schema/service.rs:19, 223`, `src/template/loader.rs:37, 173, 288`). Broadly shared filesystem walker.
    7. `hash` (`src/hash.rs`): Consumed by **2 modules** (`src/file_tracker.rs:20, 439` for path hashes; `src/config/trust.rs:16, 207, 273` and `src/config/model.rs:27, 254` for config hashes, plus in-flight `digest.rs`). **Not single-consumer**.
    8. `lexer` (`src/lexer.rs`): Consumed by **1 module** (`src/query/grammar/` via `expr.rs:13, 77`, `filter.rs:27, 56`, `source.rs:21, 117`). Note parser has its own independent lexer (`src/note/parser/lexer.rs`). **Single-consumer** (`query::grammar`).
    9. `position` (`src/position.rs`): Consumed by **2 modules** (`src/note/` in `heading.rs:5`, `lists.rs:27`, `parser.rs:55`, `parser/list.rs:15`, `parser/task.rs:307`; `src/query/` in `results.rs:24, 142`), plus `src/lexer.rs:36`. **Not single-consumer**.

- **Summary of Single-Consumer Primitives**:
  - Strictly single-consumer: `delimiter` (only `note`), `lexer` (only `query::grammar`).
  - Subsystem-bound: `dirs` (only `config` / `file_tracker`).
  - Survey claims that `strsim`, `yaml`, `env_vars`, and `hash` were single-consumer are **REFUTED**.

---

## Errata (QA review before commit, 2026-10-18)

Claim 8 above is **PARTIAL**, not `VERIFIED`. Re-checked against `main` (`0c43149d`):

- `delimiter` is **not** single-consumer: besides `note` (`links.rs:303`, `parser/lexer.rs`, `parser/marker.rs`), the root `task` module imports it (`src/task.rs:32`, used at `:227` to reject delimiter symbols as task status markers).
- `lexer` is **not** single-consumer: besides `query::grammar`, `lexer::lex_prefix` is called from `temporal` (`temporal/point.rs:147`, `temporal/duration.rs:270`) and from `note::parser::inline` (`inline.rs:235`).
- The remaining parts of the claim (`dirs` = `config` + `file_tracker`; `strsim`, `yaml`, `env_vars`, `hash` refuted as single-consumer) were re-checked for `dirs` only and stand.

Consequence: every primitive in the claim except `dirs` has two or more consumers, which strengthens the "shared foundations, not misplaced single-owner modules" reading used by the crate-topology ticket.
