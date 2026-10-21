# Codebase Inventory: Schema Subsystem, Consumers, and Note Typing

- **Topic:** Code-verified inventory for Ticket 20 (Schema note typing and validation).
- **Target:** `.scratch/arch-design/research/20-prep/a-code-inventory.md`.
- **Status:** Complete. Ground truth measured against `HEAD` (2026-10-20). Tags: `[V]` verified, `[H]` inference, `[P]` proposal.

---

## 1. Executive Summary & Git Drift (`207cc3fb..HEAD`)

- `src/schema/`: **0 lines changed** between `207cc3fb` and `HEAD` [V: `git diff 207cc3fb..HEAD --stat -- src/schema`].
- `src/note/`: **0 lines changed** between `207cc3fb` and `HEAD` [V: `git diff 207cc3fb..HEAD --stat -- src/note`].
- `src/index/service.rs`: 34 lines changed, purely rustdoc comment markdown cleanup (`[IndexError::Scan]` to `` `IndexError::Scan` ``); zero logic drift [V: `git diff 207cc3fb..HEAD -- src/index/service.rs`].
- `src/template/`: MiniJinja 3 upgrade; `src/template/engine/schema.rs` switched from `set_temp` cache key to `State::get_extension::<SchemaServiceCache>()` [V: `src/template/engine/schema.rs:82-93, 160-166`].
- **Core finding:** `src/schema` (12,129 LoC total; 4,580 prod, 7,549 tests) is a passive metadata parser and graph compiler. It has **zero dependencies** on `note`, `index`, `query`, or `template` [V: `src/schema/mod.rs:1-52`]. In production, only two features are ever consumed:
  1. Graph hierarchy (`extends`) for `@class` expansion in queries [V: `src/file_class_expander.rs:15-38`].
  2. In Minijinja templates, `schema.get(name).field(fname)` exclusively returns `select` value lists or `file` filter `SourceSelector` objects. All other field types (`input`, `boolean`, `number`, `date`) return `Value::from(())` (`None`) [V: `src/template/engine/schema.rs:186-201`].
- In production, field options for `number` (`min`, `max`, `step`) and `date` (`format`) have **zero production accessors** (annotated `#[cfg(test)]`) [V: `src/schema/fields/number.rs:22-41`, `src/schema/fields/date.rs:20-23`]. They are parsed, validated, and thrown away.

---

## 2. Schema Module Map & Measured Line Counts

Line counts measured with AST-aware split between production code and test items (`#[cfg(test)]` helper functions + `mod tests`). The earlier survey claimed 4,756 prod / 7,373 test; measuring individual `#[cfg(test)]` helpers shows production code is actually **4,580 LoC (37.8%)** and tests are **7,549 LoC (62.2%)** [V].

| File | Total LoC | Prod LoC | Test LoC | % Prod | Subsystem Role |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `src/schema/mod.rs` | 52 | 50 | 2 | 96.2% | Crate re-exports, docs, `GLOBAL_SCHEMA_NAME = "global"` [V: 48-52] |
| `src/schema/model.rs` | 328 | 144 | 184 | 43.9% | `Schema` model, fields map, `suggest_field` [V: 11-148] |
| `src/schema/name.rs` | 306 | 159 | 147 | 52.0% | `SchemaName`, `SchemaNameRef`, validation [V: 11-163] |
| `src/schema/service.rs` | 982 | 297 | 685 | 30.2% | `SchemaService` facade, `load_verbose`, queries [V: 36-308] |
| `src/schema/builder.rs` | 1,700 | 372 | 1,328 | 21.9% | DAG load, field merge, Kahn sort integration [V: 30-372] |
| `src/schema/raw.rs` | 414 | 239 | 175 | 57.7% | Serde DTOs (`RawSchema`, `RawSchemaFieldDef`) [V: 28-264] |
| `src/schema/error.rs` | 488 | 200 | 288 | 41.0% | `SchemaError`, `SchemaWarning`, `SchemaConstruction` [V: 18-200] |
| `src/schema/graph.rs` | 509 | 226 | 283 | 44.4% | `SchemaGraph` CSR container & traversal facade [V: 15-228] |
| `src/schema/graph/adjacency.rs` | 734 | 355 | 379 | 48.4% | CSR graph arrays (`child_offsets`, `targets`) [V: 28-355] |
| `src/schema/graph/builder.rs` | 203 | 139 | 64 | 68.5% | Kahn topological sort algorithm over CSR [V: 27-139] |
| `src/schema/graph/cycle.rs` | 395 | 278 | 117 | 70.4% | Tarjan iterative DFS cycle detection [V: 18-278] |
| `src/schema/fields.rs` | 464 | 264 | 200 | 56.9% | `SchemaFieldDef`, `SchemaFieldType`, `TypeTag` [V: 53-275] |
| `src/schema/fields/builder.rs` | 886 | 303 | 583 | 34.2% | Field builder, `$ref` resolution, ancestor bounds [V: 40-310] |
| `src/schema/fields/parser.rs` | 918 | 307 | 611 | 33.4% | Imperative parser for untyped options bag [V: 28-307] |
| `src/schema/fields/address.rs` | 273 | 168 | 105 | 61.5% | `FieldAddress` (`schema.field.opt`) diagnostic paths [V: 14-174] |
| `src/schema/fields/error.rs` | 655 | 245 | 410 | 37.4% | 5 nested error enums for option parsing [V: 32-245] |
| `src/schema/fields/date.rs` | 152 | 45 | 107 | 29.6% | `SchemaDateField` (option: `format`) [V: 12-55] |
| `src/schema/fields/number.rs` | 340 | 101 | 239 | 29.7% | `SchemaNumberField` (options: `min`, `max`, `step`) [V: 12-125] |
| `src/schema/fields/file.rs` | 298 | 100 | 198 | 33.6% | `SchemaFileField` (options: `folders`, `ext`, `class`) [V: 21-124] |
| `src/schema/fields/select.rs` | 2,032 | 588 | 1,444 | 28.9% | Select values cache, external JSON/TOML reader [V: 28-615] |
| **Total** | **12,129** | **4,580** | **7,549** | **37.8%** | **20 files; 62.2% test code** [V] |

---

## 3. Externally-Reachable Schema Surface & Caller Inventory

Every item exposed by `src/schema` with exact production vs test callers outside `src/schema`:

| Item | Defined At | Visibility | Production Callers (`file:line`) | Test-Only Callers (`file:line`) |
| :--- | :--- | :--- | :--- | :--- |
| `SchemaService` | `service.rs:36` | `pub` | `template/engine.rs:60`, `file_class_expander.rs:12, 15` | `file_class_expander.rs:55`, `template/engine/query.rs:84, 625` |
| `SchemaService::load_verbose` | `service.rs:83` | `pub(crate)` | `template/engine.rs:132`, `cli/mod.rs:494` | `cli/mod.rs:48` |
| `warn_schema_construction_diagnostics` | `service.rs:295` | `pub(crate)` | `template/engine.rs:133`, `cli/mod.rs:500` | None |
| `SchemaService::children_of` | `service.rs:106` | `pub` | `template/engine/schema.rs:209` | None |
| `SchemaService::children_names_of` | `service.rs:125` | `pub(crate)` | `file_class_expander.rs:26` | None |
| `SchemaService::descendants_of` | `service.rs:142` | `pub` | `template/engine/schema.rs:220` | None |
| `SchemaService::matches` | `service.rs:171` | `pub` | `file_class_expander.rs:34` | None |
| `SchemaService::warn_unknown_classes` | `service.rs:191` | `pub(crate)` | `file_class_expander.rs:20, 23` | None |
| `Schema` | `model.rs:11` | `pub` | `template/engine/schema.rs:70, 174` (`impl Object`) | `lib.rs:197` |
| `Schema::name` | `model.rs:64` | `pub` | `template/engine/schema.rs:178` | None |
| `Schema::field` | `model.rs:79` | `pub` | `template/engine/schema.rs:183` | None |
| `Schema::suggest_field` | `model.rs:129` | `pub(crate)` | `template/engine/schema.rs:350, 355` | None |
| `SchemaFieldDef::file_filter` | `fields.rs:114` | `pub(crate)` | `template/engine/schema.rs:186` | None |
| `SchemaFieldDef::select_values` | `fields.rs:98` | `pub(crate)` | `template/engine/schema.rs:191` | None |
| `SchemaFileFieldRef` | `fields/file.rs:116`| `pub(crate)` | `template/engine/schema.rs:70, 282-288` | None |
| `SchemaSelectFieldEntry` | `fields/select.rs:135`| `pub(crate)` | `template/engine/schema.rs:70, 258` | `template/engine/schema.rs:1250, 1269` |
| `SchemaError` | `error.rs:39` | `pub(crate)` | `template/error.rs:138`, `cli/error.rs:26` | `template/engine/schema.rs:369` |
| `SchemaService::new` | `service.rs:62` | `pub` (`#[expect(dead_code)]`) | **ZERO production callers** [V: `service.rs:55`] | `file_class_expander.rs:60`, `template/engine/query.rs:631`, `template/engine/schema.rs:380, 1084` |
| `SchemaServiceError` | `service.rs:24` | `pub` | **ZERO production callers** [V] | `lib.rs:197`, `template/engine/schema.rs:1089, 1120, 1235` |
| `SchemaFieldDef` | `fields.rs:53` | `pub` | **ZERO production callers** [V] | `lib.rs:197` (feature `test-utils` only) |
| `GLOBAL_SCHEMA_NAME` | `mod.rs:52` | `pub(crate)` | **ZERO callers outside `src/schema`** [V] | None |
| `SchemaFileError` | `error.rs:85` | `pub(crate)` | **ZERO production callers** [V: `mod.rs:29-34`] | `template/engine/schema.rs:369, 1090` |

---

## 4. Dead, Inert, and Test-Only Surface

1. **`SchemaService::new`** (`src/schema/service.rs:52-64`): Marked `#[cfg_attr(not(any(test, feature = "test-utils")), expect(dead_code))]`. Production loads exclusively via `load_verbose`.
2. **`SchemaServiceError`** (`src/schema/service.rs:24-34`): Only exists for `SchemaService::new`. In production, `load_verbose` returns `Result<SchemaConstruction, SchemaError>`.
3. **`SchemaFieldDef` public export** (`src/schema/mod.rs:35-36`): Only exported under `#[cfg(any(test, feature = "test-utils"))]`. Production never names this type directly.
4. **Number bounds** (`src/schema/fields/number.rs:20-41`): `min()`, `max()`, `step()` are all annotated `#[cfg(test)]`. No production code can read them.
5. **Date format** (`src/schema/fields/date.rs:18-23`): `format()` is annotated `#[cfg(test)]`. No production code can read it.
6. **`SchemaFieldDef::is_required` & `is_multi`** (`src/schema/fields.rs:128-137`): Used only within `src/schema/builder.rs` and unit tests. ADR-0006 explicitly acknowledges: *"required/multi are declared now but inert until the deferred MCP guardrail stage"* [V: `docs/adr/0006-file-class-schemas.md:110-111`].
7. **`GLOBAL_SCHEMA_NAME`** (`src/schema/mod.rs:52`): Constant `"global"` is only referenced internally in `src/schema/graph/adjacency.rs:53`, `src/schema/builder.rs:144`, and schema unit tests.
8. **`SchemaFileError`** (`src/schema/mod.rs:29-34`): Exported under `#[cfg(test)]` solely so `template::engine::schema` unit tests can pattern-match load failures.

---

## 5. Field Model & Consumption Reality

### Schema Field Types & Options Matrix

| Field Type | TOML Options Accepted | Defaults | Required / Multi Allowed | Who Consumes in Production? | Status Today |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `input` | None [V: `fields.rs:152`] | None | `required: false`, `multi: false` [V: `raw.rs:180-181`] | **Nothing** (returns `Value::from(())` in templates) [V: `template/engine/schema.rs:192`] | Parsed, ignored |
| `boolean` | None [V: `fields.rs:154`] | None | `required: false`, `multi: false` [V] | **Nothing** (returns `Value::from(())` in templates) [V] | Parsed, ignored |
| `number` | `min: f64`, `max: f64`, `step: f64` [V: `fields/number.rs:12-16`] | `None`, `None`, `None` | `required: false`, `multi: false` [V] | **Nothing** (accessors are `#[cfg(test)]`; returns `()` in templates) [V: `number.rs:22-41`] | Parsed, validated, ignored |
| `date` | `format: String` [V: `fields/date.rs:12-14`] | `None` | `required: false`, `multi: false` [V] | **Nothing** (accessor is `#[cfg(test)]`; returns `()` in templates) [V: `date.rs:20-23`] | Parsed, validated, ignored |
| `select` | `values`: inline array or external file path [V: `fields/select.rs:28-40`] | `[]` | `required: false`, `multi: false` [V] | `template/engine/schema.rs:191-201` (`.field(name)` returns list of entries) | **Consumed** |
| `file` | `folders: Vec<String>`, `ext: String`, `class: Vec<String>` [V: `fields/file.rs:21-25`] | `[]`, `None`, `[]` | `required: false`, `multi: false` [V] | `template/engine/schema.rs:186-190` (`.field(name)` returns `SourceSelector`) | **Consumed** |

### Semantics of Structural Directives

- **`$ref` overrides** (`src/schema/fields/builder.rs:252-277`): Bounded to `global` schema or transitive ancestors in `self.ancestors`. Target address is `schema.field` or `global.field`. Child can override type or merge options. Arbitrary cross-hierarchy references yield `RefOutOfBounds` [V: 268].
- **`extends`** (`src/schema/builder.rs:125-295`): Multi-parent inheritance. Fields merge topologically (Kahn order); first-listed parent wins conflicts [V: 180-192].
- **`excludes`** (`src/schema/raw.rs:37`, `src/schema/builder.rs:170-176`): List of field names explicitly suppressed from inherited parents.
- **`global` pool** (`src/schema/builder.rs:144-156`): Reserved schema stem (`global.toml`). Cannot inherit (`extends` forbidden), cannot be extended, cannot be a note `fileClass`. Any `required = true` field in `global` degrades to `false` with `SchemaWarning::StrayGlobalRequired` [V: `fields/builder.rs:152`].
- **Canonical Field Uniqueness** (`src/schema/builder.rs:306-323`): No two effective fields on a schema may produce the same `FieldKey::canonicalize(name)` (prevents ambiguous matching).

### External Select Values (`src/schema/fields/select.rs:208-289`)

- **Formats:** JSON or TOML file containing `entries = [ ... ]` [V: 274-278].
- **Confinement:** Confined to `.traces/schemas/` directory via `SafeRelativePath` [V: 260-264].
- **Caching:** In-memory `SelectValuesFileCache` during `load_verbose` [V: 212-250].
- **Shapes:** Bare string entries (`"active"`) or structured objects (`{ id = "1", title = "One", order = 10 }`).
- **Dynamic Key Mapping:** Attributes `value` (key to use as value), `label` (key for display), `order` (key for sorting) [V: 95-115].
- **Reality:** 2,032 LoC (16.8% of `src/schema`), 3 error enums, float total ordering. Used only in template prompts when author passes `schema.get('foo').field('bar')` to `ui.select`.

---

## 6. Consumers Deep-Dive

### (a) Template Engine & `ui` Prompts (`src/template/engine/`)

- `src/template/engine/ui.rs`:
  - Exposes 4 global methods: `ui.text_input(label, default)`, `ui.select(label, items, kwargs)`, `ui.confirm(label, default)`, `ui.multi_select(label, items, kwargs)` [V: `src/template/engine/ui.rs:34, 79-130`].
  - **`ui.rs` has ZERO knowledge of schemas or field definitions.** It never imports `src/schema` [V].
  - There is NO `ui.date` and NO `ui.number` [V: `ui.rs:34`].
  - How do prompts type their input? They don't. `ui.text_input` prompts for raw text; `ui.select` extracts `.label` from item objects (or uses raw string) via `DEFAULT_ATTRIBUTE = "label"` [V: `ui.rs:38`].
- `src/template/engine/schema.rs`:
  - Registers `schema.get(name)` which returns a Minijinja `Object` wrapping `Schema` [V: 119-140].
  - `schema.name` -> string [V: 178].
  - `schema.children()` -> list of child `Schema` objects [V: 209].
  - `schema.descendants()` -> list of descendant `Schema` objects [V: 220].
  - `schema.field(name)`:
    - If `field.file_filter()`: returns `SourceSelector` (usable in `query.from(...)`) [V: 186-190].
    - If `field.select_values()`: returns list of `{ value, label }` [V: 191-200].
    - Else: returns `Value::from(())` (`None`) [V: 192].

### (b) CLI Schema Commands & Output Shapes

- **There are NO `traces schema` CLI commands.** The survey claim that CLI calls `load_verbose` to service schema inspect commands is **FALSE** [V: `src/cli/mod.rs:149-173`].
- CLI subcommands are strictly: `init`, `trust`, `untrust`, `index`, `list`, `table`, `task`, `template`, `completions`, `tracked` [V: 150-172].
- In `src/cli/mod.rs:470-488`, `load_schema_service` is called inside `run_query_builder_from_store` **only when a query contains class selectors** (`has_classes`) to attach `FileClassExpander` to `QueryService` [V: 479-480].

### (c) `src/file_class_expander.rs`

- Bridges `query` and `schema` without cross-module dependency [V: 4-6].
- Implements `FileClassExpander` for `SchemaService`:
  - `Exact`: calls `self.warn_unknown_classes(classes)` [V: 19-21].
  - `Children`: expands via `self.children_names_of(class)` [V: 22-29].
  - `Descendants`: expands via `self.matches(classes)` [V: 30-36].

### (d) `src/index/service.rs` FileClass Extraction (`mod class_field`)

- Tests are at `src/index/service.rs:1732-1923` [V].
- Production logic is in `src/index/store.rs:1761-1773` (`IndexDimension::visit_values`) [V]:

  ```rust
  let values = note
      .frontmatter()
      .into_iter()
      .flat_map(|fm| fm.get_values(key))
      .filter_map(crate::NoteFieldValue::as_str);
  for value in values {
      with_lowercased(value, &mut visit)?;
  }
  ```

- **Key name:** Configurable via `[schemas] class_field = "..."` (default `"class"`) [V: `src/config/model/schemas.rs:40-42`, `src/index/service.rs:54-56`].
- **Lookup:** Uses `Frontmatter::get_values(key)` which queries `fields.get(&FieldKeyRef::new(key))` [V: `src/note/metadata.rs:130, 138`]. This automatically handles case-folding and punctuation via `FieldKey::canonicalize` (e.g. `Class`, `class`, `CLASS` all match) [V: `src/field.rs:324-343`].
- **Values:** Handles both scalar strings (`class: novel`) and list of strings (`class: [sci_fi, novel]`) [V: `src/note/metadata.rs:140-149`]. Non-string values (numbers, dates, links) are filtered out silently by `filter_map(NoteFieldValue::as_str)` [V: `src/index/store.rs:1768`].
- **Normalization:** Lowercased before writing to redb multimap `paths_by_file_class` via `with_lowercased` (zero-alloc for ASCII lowercase) [V: `src/index/store.rs:1770, 1781-1798`].
- **Index decoupling:** The indexer **never reads `.traces/schemas/`**. It extracts whatever string is in frontmatter under the configured `class_field` name.

### (e) References in Note / Query / Config

- `src/note/`: **Zero references to schema.**
- `src/query/`: References `FileClassExpander` trait (`src/query/service.rs:36-39`), not `SchemaService`. Query parses `@book`, `class(book)`, `class(book, children)`, `class(book, descendants)` [V: `src/query/grammar/source.rs:250-280`].
- `src/config/`: `SchemasConfig` holds `directory` (default `.traces/schemas`) and `class_field` (default `"class"`) [V: `src/config/model/schemas.rs:12-42`].

---

## 7. Field-Type Enums & Note-Value Variants Mapping Matrix

Four field-type enums coexist across the codebase:

1. `RawSchemaFieldType` (`src/schema/raw.rs:204`): serde TOML tag (`input`, `select`, `boolean`, `number`, `date`, `file`).
2. `SchemaFieldTypeTag` (`src/schema/fields.rs:241`): diagnostics tag (same 6 variants, no payload).
3. `SchemaFieldType` (`src/schema/fields.rs:150`): resolved schema options payload (`Number(SchemaNumberField)`, `Date(SchemaDateField)`, `File(Arc<SchemaFileField>)`, `Select(Arc<SchemaSelectField>)`).
4. `NoteFieldType` (`src/note/field.rs:209`): comparison lattice rank (`Null:0 < Bool:1 < Number:2 < Duration:3 < Temporal:4 < Text:5 < Link:6 < List:7 < Object:8`).

### Validation Compatibility Matrix: `SchemaFieldType` vs `NoteFieldValue`

| Schema Type | Validates Variant | Gaps & Edge Cases in Current Code |
| :--- | :--- | :--- |
| `Input` | `NoteFieldValue::String` [V: `note/field.rs:51`] | Frontmatter parses unquoted date/temporal strings as `Date`/`DateTime`, or unquoted numbers as `Number`. If schema says `input`, note has `Date` or `Number` variant. Coercion or advisory check required. |
| `Boolean` | `NoteFieldValue::Bool` [V: `note/field.rs:47`] | Strings `"true"`/`"false"` in YAML are parsed as Bool, but unquoted YAML 1.1 `yes`/`no` stay Strings in YAML 1.2 (`noyalib`). |
| `Number` | `NoteFieldValue::Number(f64)` [V: `note/field.rs:49`] | Ticket 18 decided `Int(i64)` / `Float(f64)` split. Schema `min`/`max`/`step` are `f64`. Quoted `"42"` stays String. Bounds are currently inaccessible in prod. |
| `Date` | `NoteFieldValue::Date` or `DateTime` [V: `note/field.rs:53-55`] | `SchemaDateField` accepts `format: Option<String>`, but `NoteFieldValue` parses ISO `YYYY-MM-DD` (`Date`) and RFC 3339 (`DateTime`). Non-ISO dates stay `String`. Does `Date` schema allow `DateTime` values? |
| `Select` | `NoteFieldValue::String`, or `NoteFieldValue::Link` [V: `note/field.rs:51, 60`] | Select entries are stored as `FieldValue` literals. If note contains wikilink `[[Target]]` (`NoteFieldValue::Link`), matching against select entry string `Target` requires the adaptive-literal rule from Ticket 19 [V: `19-coercion-and-comparison-policy.md:43-45`]. |
| `File` | `NoteFieldValue::Link` or `String` [V: `note/field.rs:51, 60`] | Validating `file` requires resolving the link target against `IndexStore` and checking `folders`, `ext`, and `class`. Cannot be validated in-memory by `note` alone; requires `WorkspaceIndex`. |
| *(None)* | `NoteFieldValue::Duration` [V: `note/field.rs:58`] | **Schema has no `duration` type.** |
| *(None)* | `NoteFieldValue::Object` [V: `note/field.rs:64`] | **Schema has no `object` or nested map type.** |
| `multi: true` | `NoteFieldValue::List` [V: `note/field.rs:62`] | In schema, `multi` is a boolean flag on `SchemaFieldDef`, not a type variant. If `multi: true`, note must have `List`. If `multi: false`, note having a 1-element list is a common YAML pattern. |

### Frontmatter Key Matching to Schema Fields

- Schema fields are canonicalized at construction: `FieldKey::canonicalize(name)` rejects ambiguous names [V: `src/schema/builder.rs:306-323`].
- Frontmatter keys are stored as `FieldKey` preserving raw `name: Box<str>` and computed `canonical: Box<str>` [V: `src/field.rs:274-279`].
- Key lookup uses `FieldKeyRef::new(schema_field_name)`, matching if canonical forms match (`Status` matches `status`) [V: `src/field.rs:324-343`].

---

## 8. Note Model Inspection: Inputs for Validation

### What `Note` Exposes Today (`src/note/model.rs:24-33`)

```rust
pub struct Note {
    path: PathBuf,
    frontmatter: Option<Frontmatter>,
    lists: Box<[ListItem]>,
    headings: Box<[Heading]>,
    outlinks: Box<[Link]>,
    inline_fields: IndexMap<FieldKey, Box<[NoteFieldValue]>>,
    tags: Box<[Tag]>,
}
```

1. **Spans and Source Positions:**
   - Frontmatter parsing in `src/note/metadata.rs:56-74` parses YAML via `noyalib::parse` and constructs `IndexMap<FieldKey, NoteFieldValue>`. **Spans and line/column numbers are completely discarded.** [V: `metadata.rs:66-73`].
   - Inline field parsing in `src/note/parser.rs:493-497` groups values into `IndexMap<FieldKey, Vec<NoteFieldValue>>`. **Spans are completely discarded.** [V].
   - **Consequence for validation:** An advisory validator running over parsed `Note` can only emit file-level or field-name diagnostics (e.g. `path: "notes/a.md", field: "due"`). It **cannot** emit line/column spans unless:
     - (a) `Note`/`Frontmatter` is updated to record byte spans per field [P], or
     - (b) The validator re-parses the raw frontmatter text (`raw_frontmatter`) with a span-aware YAML parser on demand [P].
2. **`fileClass` Value on `Note`:**
   - Post-parse, `fileClass` is **not a dedicated field** on `Note`. It must be extracted from `note.frontmatter().get_values(class_field_name)` [V: `src/index/store.rs:1764-1772`].
   - Supports single string or list of strings [V].
3. **Inheritance & Class Chains in Validation:**
   - If a note has `class: novel`, and `novel extends sci_fi extends book`:
   - `SchemaService::get("novel")` resolves the effective schema where `fields` already contains merged fields from `sci_fi`, `book`, and `global` (with `$ref` and `excludes` already applied) [V: `src/schema/builder.rs:125-295`].
   - Validation against `schema.fields()` automatically validates the entire inheritance chain in a single pass [H].

---

## 9. Tests, Fixtures & User-Facing Intent

### Fixtures & Test Coverage Reality

- **`tests/` integration/e2e tests:**
  - `tests/e2e/config.rs:41-60`: tests configuring `[schemas] class_field`.
  - `tests/e2e/query.rs:30-45`: tests `@class*` query expansion through schema graph (`book` -> `sci_fi` -> `novel`).
  - `tests/e2e/template.rs:166-188`: tests template `{{ schema.get('topic').field('category')[0].label }}`.
  - `tests/integration/template_render.rs:606-630`: tests `@class` expansion in `query.from(...)`.
  - **ZERO tests in `tests/` validate note contents against schemas.** [V].
- **Real-world Schema Fixtures:**
  - `examples/.traces/schemas/`: Contains 42 schema files (e.g. `pkm.toml`, `task.toml`, `lib_book.toml`, `global.toml`) and 5 values JSON files (`countries.json`, `cities.json`, etc.) [V].
  - Real-world schemas define extensive `type`, `values`, `folders`, `ext`, `class`, `required`, and `multi` fields.
  - There are **no note fixtures** in `examples/` that pair with these schemas [V].

### Repo Documentation on Schema Intent (ADR-0006)

From `docs/adr/0006-file-class-schemas.md` [V]:
> *"Traces needs a way for template authors to stop hard-coding field value lists (e.g. every select writing out its own options) and instead reference a shared, per-note definition of what fields mean and which values they accept. The prior art is the Obsidian Metadata Menu plugin, whose fileClass system scopes field definitions to classes of notes. Traces is a CLI with a queryable index over markdown notes and no GUI, so the fileClass concept must be adapted to declarative config files consumed by minijinja templates, queries, and (later) an LSP and MCP."* [V: lines 31-38]
>
> *"The schema supplies values only; the template author picks the interactive `ui.*` function."* [V: lines 76-77]
>
> *"Non-list types return None."* [V: line 75]
>
> *"required/multi are declared now but inert until the deferred MCP guardrail stage; LSP and MCP stages are deferred to later phases."* [V: lines 110-111]

---

## 10. Sizing of Candidate Architecture Options

### Candidate A: Keep Passive & Prune Dead Weight

- **Design:** Schemas remain passive metadata providers for templates and `@class` query expansion. Do not validate notes. Delete unused internals.
- **Deletable Surface:**
  1. `src/schema/fields/select.rs` external values cache & files (`SelectValuesFileCache`, `RawSchemaSelectFieldValues`, JSON/TOML reader): **-1,440 LoC** [V].
  2. `src/schema/fields/parser.rs` imperative options parsing (replace with typed serde): **-700 LoC** [H].
  3. `src/schema/fields/error.rs` 5 nested error enums (collapse to simple enum): **-500 LoC** [H].
  4. CSR graph representation (`adjacency.rs`, `cycle.rs`, `graph/builder.rs` = 1,332 LoC) replaced with simple `HashMap<SchemaName, Vec<SchemaName>>`: **-1,100 LoC** [H].
  5. Unused options in `number.rs` (`min`/`max`/`step`) and `date.rs` (`format`): **-350 LoC** [V].
  6. Dead code (`SchemaService::new`, `SchemaServiceError`): **-50 LoC** [V].
- **Net LoC Impact:** **Delete ~3,500 - 4,000 LoC** from `src/schema/` (shrinking 12k to ~8k LoC).

### Candidate B: Advisory Validator (On-Demand / Check Pass)

- **Design:** Keep schema definitions (including `required`, `multi`, bounds). Introduce an on-demand validation pass (CLI command `traces check` / LSP diagnostics / test suite) producing structured diagnostics.
- **New Surface Required:**
  1. `src/schema/validate.rs`: validation engine checking `Note` against `Schema` effective fields:
     - Missing `required` fields.
     - Type compatibility (`NoteFieldValue` vs `SchemaFieldType`).
     - Bounds check for numbers (`min <= n <= max`).
     - Allowed values check for select (with adaptive-literal for links per Ticket 19).
     - File link filter validation (`folders`, `ext`, `class`) via `WorkspaceIndex`.
     - Multi-valued list checking (`multi: true` vs scalar).
     - Sizing: **~500 LoC prod, ~700 LoC tests** [H].
  2. Span tracking in `src/note/` (if diagnostics require line/col spans):
     - Updating `RawFrontmatter::parse` or AST extraction to retain field spans: **~300-400 LoC** [H].
- **Net LoC Impact:** **+800 to +1,600 LoC**. Leaves index and note persistence untouched.

### Candidate C: Index-Time Typed Ingestion

- **Design:** `IndexerService` resolves schemas during note ingestion. Coerces frontmatter/inline fields and stores typed records in Redb.
- **Architectural Changes & Invalidation:**
  1. `IndexerService` must depend on `Arc<SchemaService>` [V: currently zero dependency].
  2. **Cache invalidation nightmare:** `ConfigDigests` fingerprints config files. Schemas (`.traces/schemas/*.toml`) are NOT fingerprinted [V: `src/config/digests.rs`]. Editing any schema file would require either re-indexing the entire vault or tracking schema file mtimes/digests to invalidate notes with matching `fileClass` [H].
  3. Redb tables: would need schema-typed tables or schema-versioned note records.
  4. **Violation policy conflict:** Conflicts with Ticket 19 ("coercion failures are advisory, never gating") and Research 03 ("note validation must never gate the index") [V: `20-prep-context.md:19-20`].
- **Net LoC Impact:** **+2,000 to +3,500 LoC** across `index`, `store`, `config`. High blast radius and fragility.

---

## 11. Findings that Change the Survey

The following claims in `.scratch/arch-design/survey/schema.md` are **incorrect or stale** against `HEAD`:

1. **Line counts:** Survey §3 claimed `12,129 total lines: 4,756 lines production code (39.2%), 7,373 lines unit test code (60.8%)`.
   - *Correction:* Measuring individual `#[cfg(test)]` helper methods shows **4,580 lines production (37.8%)** and **7,549 lines test (62.2%)** [V]. In `select.rs`, prod is 588 lines (not 616) and test is 1,444 lines (not 1,416) [V].
2. **CLI Schema Commands:** Survey §1 claimed: `Cli (src/cli/mod.rs:490-502): Calls load_verbose to service schema inspect commands and passes it to query/template pipelines.`
   - *Correction:* **There are no schema inspect commands in Traces CLI.** The CLI has no `schema` subcommand [V: `src/cli/mod.rs:149-173`]. `load_verbose` in `src/cli/mod.rs:490-502` is called exclusively when running page queries (`list` / `table`) that contain `@class` selectors, to attach `FileClassExpander` to `QueryService` [V: `src/cli/mod.rs:478-480`].
3. **Template UI Prompts:** Survey §1 & §4 hypothesized schemas inform interactive prompts.
   - *Correction:* `src/template/engine/ui.rs` has **zero knowledge of schemas** [V]. There is no `ui.date` and no `ui.number`. Prompts accept generic items/labels; templates only pass schema select entries to `ui.select`.
4. **Field Options Consumption:** Survey implied field options are actively modeled.
   - *Correction:* `number` options (`min`, `max`, `step`) and `date` option (`format`) have **no production accessors** (annotated `#[cfg(test)]`) [V: `number.rs:22-41`, `date.rs:20-23`]. When `.field(name)` is called on a schema in a template, non-list fields (`input`, `boolean`, `number`, `date`) unconditionally return `Value::from(())` (`None`) [V: `template/engine/schema.rs:192`].
5. **Class Extraction Location:** Survey §3 cited `src/index/service.rs:1656-1840`.
   - *Correction:* Unit tests are at `src/index/service.rs:1732-1923` (`mod class_field`). The actual production extraction runs in `src/index/store.rs:1761-1773` (`IndexDimension::visit_values`), extracting values via `note.frontmatter().get_values(key)`, filtering by `NoteFieldValue::as_str`, and lowercasing via `with_lowercased` [V].

---

## 12. Not Verified

1. **Performance of re-parsing frontmatter on demand:** We did not benchmark the CPU cost of re-parsing frontmatter YAML with a span-preserving parser during an advisory `traces check` run vs storing spans in `Frontmatter` permanently.
2. **External values file usage in wild vaults:** We cannot verify whether any real user vaults actively use the external `.json`/`.toml` select values files feature in `src/schema/fields/select.rs`, beyond the bundled `examples/` fixtures.
3. **MCP / LSP roadmap timing:** Whether the "deferred MCP guardrail stage" mentioned in ADR-0006 will arrive in the next milestone or remains indefinite future art.

---

## 13. Questions this Raises for the Grilling

1. **If schemas remain passive (Option A), should we immediately delete the ~3.5k lines of dead complexity?**
   - Specifically: delete external select values file cache (`select.rs:208-289`), replace the CSR adjacency graph (`1,332` LoC) with a simple adjacency map, and drop unread `min`/`max`/`step`/`format` options.
2. **If we add an advisory validator (Option B), does it report file-level diagnostics or must `Note` gain span tracking?**
   - Currently, `Note` and `Frontmatter` completely discard YAML byte offsets and line numbers. Does validation require changing the note parser / `noyalib` integration, or is an on-demand re-parse sufficient for a future `traces check` / LSP pass?
3. **How should a validator handle the absence of schema types for rich note values?**
   - Notes natively support `Duration` and `Object` (`src/note/field.rs:58, 64`), but schemas have neither. Does having a note field not defined in the schema trigger an "unknown field" warning, or are schemas open/permissive by default (Dataview style)?
4. **Should schema field definitions override frontmatter classification (Ticket 19 link)?**
   - Ticket 19 asked: if a frontmatter value is `2024-01-01` (sniffed as `Date`), but the schema defines `type = "input"`, does the validator expect a string, or does schema typing steer frontmatter parsing away from temporal sniffing?
5. **Should `file` field validation require an active index?**
   - `file` fields restrict targets by folder, extension, and class. Validating a note's file link cannot be done in isolation—it requires querying `IndexStore`. Should validation be decoupled into pure local structural checks vs whole-vault referential checks?
