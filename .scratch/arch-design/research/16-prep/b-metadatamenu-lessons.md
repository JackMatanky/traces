# Metadata Menu Lessons: Schemas, Typing, Validation, and Invalidation in PKM

- **Topic:** Empirical audit of Obsidian Metadata Menu (`mdelobelle-metadatamenu`) across docs, source, and architecture digests for Wayfinder Ticket 16 (Schema Note Typing and Validation).
- **Status:** Complete research deliverable.
- **Deliverable:** `.scratch/arch-design/research/16-prep/b-metadatamenu-lessons.md`.
- **Sources audited:**
  - `docs/digests/obsidian_mdelobelle-metadatamenu-docs-digest.txt` (96 KB, 2,175 lines) [cited as `docs:`]
  - `docs/digests/obsidian_mdelobelle-metadatamenu-src-digest.txt` (1 MB, 24,868 lines) [cited as `src:`]
  - `docs/digests/obsidian_mdelobelle-metadatamenu-digest.txt` (1.1 MB, 28,624 lines) [cited as `repo:`]
  - Traces baseline: `src/schema/GLOSSARY.md`, `.scratch/arch-design/survey/schema.md`, `issues/16-*.md`.

---

## 1. Domain Model: FileClass, Mapping, and Field Kinds

### 1.1 FileClass Definition & Options

- **Storage & Structure:** A `fileClass` is a standard Markdown file stored in a designated vault folder (`settings.classFilesPath`, default `fileClasses/`) [docs: 1151-1153; src: 14193-14204]. The file name (without `.md`) defines the class identifier [docs: 1153].
- **Configuration in Frontmatter:** Schema options and field definitions live entirely within the note's YAML frontmatter [docs: 1159; src: 14148-14160]. The note body is ignored by the plugin [docs: 1159].
- **FileClass Frontmatter Keys:**
  - `limit`: Number of rows per page in table views (defaults to `settings.tableViewMaxRecords`) [docs: 1416-1418; src: 14046, 14066, 14171].
  - `mapWithTag`: Boolean; when `true`, notes containing tag `#<className>` automatically inherit this class [docs: 1387-1392; src: 14047, 14172].
  - `tagNames`: String or array of strings; explicit tag aliases that map to this fileClass [docs: 1393-1398; src: 14049, 14173].
  - `filesPaths`: String or array of folder path prefixes; files in these directories map to the class [docs: 1399-1404; src: 14050, 14174].
  - `bookmarksGroups`: String or array of bookmark group names that map to the class [docs: 1405-1409; src: 14051, 14175].
  - `parent` / `extends`: Single string naming a parent fileClass [docs: 1250-1337; src: 14053, 14149, 14162]. **Single inheritance only**; multiple parents are unsupported [src: 18347-18355].
  - `excludes`: Array of field names omitted from ancestor inheritance [docs: 1370-1386; src: 14052, 14163-14170, 14271-14285].
  - `icon`: Lucide icon name for the note button (defaults to `settings.fileClassIcon` / `file-spreadsheet`) [docs: 1410-1415; src: 14048, 14176, 14206-14221].
  - `savedViews`: Array of table view configurations (filters, sorters, column orders) [docs: 1458-1472; src: 14054, 14177].
  - `favoriteView`: Name of default view to display [src: 14055, 14178].
  - `fieldsOrder`: Array of field IDs governing insertion and column sequence [docs: 1627-1635; src: 14056, 14179, 14234-14258].
  - `version`: Version string (`x.y`), auto-incremented by the system on frontmatter write [docs: 1420-1423; src: 14294-14321].

### 1.2 Note-to-Class Matching & Priority Resolution

Notes associate with zero, one, or multiple fileClasses. When a field name collides across sources, Metadata Menu evaluates definitions in strict priority order [docs: 1171-1179; src: 18580-18644]:

1. **Frontmatter attribute:** Key `fileClass` (configurable via `settings.fileClassAlias`), accepts string or array [docs: 1180-1209; src: 18548-18574].
2. **Tag match:** `mapWithTag: true` or `tagNames` [docs: 1210-1224; src: 18441-18475].
3. **Folder path match:** `filesPaths` folder prefixes [docs: 1225-1231; src: 18477-18495].
4. **Bookmark group match:** `bookmarksGroups` [docs: 1232-1236; src: 18507-18523].
5. **Dataview query match:** Global `fileClassQueries` configured in plugin settings [docs: 1237-1241; src: 18525-18542].
6. **Global fileClass fallback:** `settings.globalFileClass` applies to notes lacking any matched class [docs: 1242-1245; src: 18629-18636].
7. **Preset fields:** Global unclassified vault fields defined in plugin `data.json` [docs: 1178, 2115-2122; src: 18637-18644].

### 1.3 Complete Field Types & Options Matrix

Metadata Menu defines exactly 22 field types (`src/fields/Fields.ts:6375-6447`) [V]:

| Type | Target / Location | Options | Options Source / Storage | Source Reference |
| :--- | :--- | :--- | :--- | :--- |
| `Input` | Frontmatter / Inline | `template` (string template with `{{value}}`) | YAML / JSON `options` | [src: 6400, 8800-8840] |
| `Number` | Frontmatter / Inline | `step` (float), `min` (float), `max` (float) | YAML / JSON `options` | [src: 6401, 10351-10355] |
| `Select` | Frontmatter / Inline | `sourceType` (`ValuesList` \| `ValuesListNotePath` \| `ValuesFromDVQuery`), `valuesList`, `valuesListNotePath`, `valuesFromDVQuery` | YAML / JSON `options` | [src: 6402, 11249, 12858-12883] |
| `Cycle` | Frontmatter / Inline | Same as `Select` (`AbstractList.Options`); cycles values on click | YAML / JSON `options` | [src: 6404, 8250-8260] |
| `Boolean` | Frontmatter / Inline | None; accepts `true`, `false`, `null` | Unconditional | [src: 6405, 7920-7935] |
| `Date` | Frontmatter / Inline | `dateFormat`, `defaultInsertAsLink`, `dateShiftInterval`, `nextShiftIntervalField` | YAML / JSON `options` | [src: 6406, 8400, 12380-12400] |
| `DateTime` | Frontmatter / Inline | `dateFormat`, `timeFormat`, `defaultInsertAsLink`, `dateShiftInterval`, `nextShiftIntervalField` | YAML / JSON `options` | [src: 6407, 8500-8530] |
| `Time` | Frontmatter / Inline | `timeFormat` | YAML / JSON `options` | [src: 6408, 11400-11415] |
| `Multi` | Frontmatter / Inline | Same as `Select` + `display` (`asArray` \| `asList`) | YAML / JSON `options` | [src: 6403, 9870-9885] |
| `File` | Frontmatter / Inline | `dvQueryString` (Dataview query), `customRendering` (JS string) | YAML / JSON `options` | [src: 6409, 8600, 12600-12640] |
| `MultiFile` | Frontmatter / Inline | Same as `File` + `display` (`asArray` \| `asList`) | YAML / JSON `options` | [src: 6410, 10080-10110] |
| `Media` | Frontmatter / Inline | `mediaFolder`, `mediaExtensions`, `embed` (boolean) | YAML / JSON `options` | [src: 6411, 9550-9570] |
| `MultiMedia` | Frontmatter / Inline | Same as `Media` + `display` (`asArray` \| `asList`) | YAML / JSON `options` | [src: 6412, 10280-10300] |
| `Canvas` | Frontmatter / Inline | `canvasPath` (relative path to `.canvas` file) | Root fields only | [src: 6413, 6476, 8020] |
| `CanvasGroup`| Frontmatter / Inline | `canvasPath` | Root fields only | [src: 6414, 6476, 8100] |
| `CanvasGroupLink`| Frontmatter / Inline| `canvasPath` | Root fields only | [src: 6415, 6476, 8210] |
| `Formula` | Frontmatter / Inline | `formula` (JS expression), `autoUpdate` (boolean) | Root fields only | [src: 6416, 6476, 8704-8740] |
| `Lookup` | Frontmatter / Inline | `dvQueryString`, `targetField`, `outputType`, `builtinFunction`, `customFunction`, `autoUpdate` | Root fields only | [src: 6417, 6476, 9300-9420] |
| `JSON` | Frontmatter / Inline | None; raw JSON text parsed via `JSON.parse` | Raw text | [src: 6418, 9080-9110] |
| `YAML` | Frontmatter only | None; raw YAML parsed via `parseYaml` | Raw text | [src: 6419, 6459, 11500] |
| `Object` | Frontmatter only | None; container for child fields mapped by `path` (`parent_id`) | Nested tree | [src: 6420, 6459, 10880] |
| `ObjectList` | Frontmatter only | `displayTemplate` | Nested list | [src: 6421, 6459, 11080] |

---

## 2. Validation in Metadata Menu: Empirical Audit

### 2.1 The Gating Myth: Zero Enforcement at Write / API / Read

- **Search for Gating:** Grepping the entire repository codebase reveals **zero blocking validation passes** on note files during indexing or programmatic writing [V: `repo: 1-28624`].
- **Unchecked API Writes (`postValues`):**
  - `postValues(plugin, payload, fileOrFilePath, ...)` (`src/commands/postValues.ts:3375-3404`) [V] creates or updates note frontmatter/body lines.
  - Verification: `postValues` validates only that each payload item contains an `indexedPath` (`src: 3383-3386`) [V]. It performs **zero type checking, zero schema conformance checks, zero enum validation, and zero range validation** before persisting values to disk [V].
- **Unchecked Storage Parsing (`LineNode` & `Note`):**
  - When loading frontmatter in `LineNode.defineExistingFieldsAndValues` (`src/note/lineNode.ts:19393-19477`) [V], attributes are assigned directly to `existingField.value`:
    `this.value = frontmatter[field.name]` (`src: 19471`) [V].
  - No coercion or validation occurs [V]. If a schema defines `rating: Number` and frontmatter contains `rating: "banana"`, `existingField.value` is stored as `"banana"` without error [V].
  - Unmapped attributes: If an attribute in frontmatter is absent from the note's schema fields, it is silently skipped in `defineExistingFieldsAndValues` [V: 19416]. It is **not** an error, emits no warning, and preserves original file contents intact [V].

### 2.2 Write-time Interactive Modal Gating (`valueModal.save()`)

Validation in Metadata Menu is strictly confined to interactive UI entry modals:

- **Blocking Modal Save:** When a user enters data via a field's edit modal, `save()` invokes `validateValue(this.managedField)` [V].
  - In `Number.ts:10532-10539` [V]:

    ```typescript
    if (!validateValue(this.managedField)) {
        const { min, max } = this.managedField.options
        this.errorField.show();
        this.errorField.setText(`value must be numeric${min ? " and >= " + min : ""} ${max ? " and <= " + max : ""}`)
        this.numberInput.inputEl.setAttr("class", "is-invalid")
        return // Blocked: will not save
    }
    ```

  - In `Select.ts:11261-11279` [V]: The suggest modal restricts input to allowed options. If a user forces a new value (`onAdd`), Metadata Menu mutates the underlying schema setting via `addNewValueToSettings()` [V: 11265] before saving, thereby maintaining enum validity.
  - In `AbstractDate.ts:12406-12424` [V]: Moment parsing enforces format matching: `moment(cleanValue, options.dateFormat).isValid()`.

### 2.3 Advisory-Only Read Surface (`fileFields.ts` and `isValid`)

- **API Field Inspection:** Metadata Menu exposes an analysis endpoint: `fileFields(fileOrFilePath)` (`src/commands/fileFields.ts:2605-2615`) and `namedFileFields` (`src: 2617-2629`) [V].
- **Advisory `isValid` Flag:**
  - `FieldInfo.getInfos()` (`src/commands/fileFields.ts:2570-2586`) computes:
    `isValid: !fieldVM.value || validateValue(fieldVM.type)(fieldVM)` (`src: 2580`) [V].
  - **Empty fields are unconditionally valid:** If `fieldVM.value` is falsy (`""`, `null`, `undefined`), `isValid` evaluates to `true` [V]. Metadata Menu cannot report a missing or blank field as invalid [V].
  - **Advisory consumption:** This boolean is consumed purely by external scripts or the API; it never blocks indexing, note rendering, or file modification [V].

### 2.4 Missing Fields vs Invalid Fields vs Extra Unmapped Fields

- **Missing Fields (`<missing>`):**
  - Computed on-demand in `FieldsModal.ts:4505-4510` [V] by subtracting `existingFields` from `note.fields`.
  - Display: Displayed in the note modal as `<missing>` with a `list-plus` insertion button (`src: 4591, 4614`) [V].
  - Insertion: Bulk inserted into frontmatter or at a line via `insertMissingFields` (`src/commands/insertMissingFields.ts`) [V].
  - Never an error; completely benign [V].
- **Required Fields:**
  - **Metadata Menu has zero concept of `required` or `isRequired`** [V: `repo: 1-28624`]. A search for `required` returns zero hits across the entire codebase and docs. All schema fields are strictly optional [V].
- **Extra Unmapped Fields:**
  - YAML fields in a note that do not match any mapped fileClass or preset field are completely ignored by Metadata Menu [V: 19416]. They render normally in Obsidian Properties and are never flagged as extraneous [V].

### 2.5 Implementation Flaws in Type Validators

Audit of `validateValue` implementations revealed critical design shortcuts:

- **`Number.validateValue` Bypass (`src/fields/models/Number.ts:10632-10641`):**

  ```typescript
  export function validateValue(managedField: IFieldManager<Target, Options>): boolean {
      const { min, max } = managedField.options;
      if (!min || !max) return true // BUG: Returns true if either bound is missing!
      const fMin = parseFloat(`${min}`);
      const fMax = parseFloat(`${max}`);
      const fValue = parseFloat(`${managedField.value}`);
      return (!isNaN(fValue) && (isNaN(fMin) || fValue >= fMin) && (isNaN(fMax) || fValue <= fMax))
  }
  ```

  [V] If a `Number` field has no `min` or no `max`, `validateValue` returns `true` without checking `!isNaN(fValue)`. A value of `"invalid_text"` evaluates as valid!
- **Unimplemented Stub Validators:**
  `validateValue` in `Formula.ts:8791`, `Lookup.ts:9478`, `JSON.ts:9204`, and `Object.ts:10921` unconditionally return `true` or log `"Not implemented"` (`AbstractList.ts:13264`) [V].

---

## 3. Reading, Indexing, and the Performance Cost Story

### 3.1 FieldIndex Architecture & Invalidation Lifecycle

- **Central Index (`FieldIndex.ts:18068-18718`):**
  - Manages inheritance graphs (`fileClassesAncestors`), class definitions (`fileClassesFields`), and file-to-class mappings (`filesFileClasses`, `filesFields`) [V: 18765-18810].
- **Lifecycle Events:**
  - `vault.on("modify")`: Adds file to `changedFiles` [V: 18112-18121].
  - `metadataCache.on("resolved")`: When pending cache tasks hit 0, if any modified file was in `classFilesPath`, triggers `metadata-menu:fileclass-indexed`; runs `indexFields()`; triggers `metadata-menu:indexed` [V: 18139-18150].
  - `vault.on("delete")` / `vault.on("rename")`: Purges paths and runs `fullIndex()` [V: 18124-18136].
  - `bookmarks.on("changed")`: Rebuilds index if bookmarks plugin is active [V: 18099-18108].
  - `dataview:metadata-change`: Triggers formula and lookup re-evaluations [V: 18160-18171].

### 3.2 The Value Indexing Disaster: Why MM Abandoned Eager Note Value Indexing

- **The Performance Wall:** Early versions of Metadata Menu attempted to maintain an in-memory index of every note's parsed field values across the entire vault [docs: 1735; repo: 2820-2850].
- **The Removal (v0.6.4):** In release 0.6.4, the author explicitly removed values indexing:
  *"Performance: queries resolution are made asynchronously; new setting to scope the plugin to the frontmatter only; removed values indexing"* [docs: 1732-1736; repo: 2840] [V].
- **Current Strategy (Lazy on-demand):**
  - `FieldIndex` stores **zero note field values** [V: 18765-18810]. It stores only the schema definitions assigned to each file path (`filesFields: Map<string, Field[]>`) [V: 18784].
  - Values are parsed from disk **lazily on-demand** only when a user interacts with a note (`Note.buildNote` inside `FieldsModal`, `postValues`, or API calls) [V: 19844-19854].

### 3.3 What MM Actually Caches vs Computes on Demand

- **Eagerly Cached in Memory:**
  - Inheritance DAGs (`fileClassesAncestors`) [V: 18855].
  - Resolved field definitions per class (`fileClassesFields`) [V: 18844].
  - File-to-class associations based on tags, paths, bookmarks, and queries [V: 18857-18860].
  - External note option lists (`valuesListNotePathValues`) [V: 18856].
- **Computed Lazily / On-Demand:**
  - Note frontmatter AST / lines (`Note.buildNote`, `LineNode`) [V: 19844].
  - Table view rows: Rendered page-by-page via Dataview API or `Note.buildNote` [docs: 1416, 1658-1662]. Table view auto-rebuilding on schema change was explicitly disabled in v0.6.11 to stop freezing the UI; users must manually click a "refresh" button [docs: 1473-1475, 1658-1659].

---

## 4. Evolution & User Pain Points: What the Settings & Releases Reveal

### 4.1 Features Born from User Pain

- **Multiple FileClasses per Note:** Added in v0.5.x (`docs: 56-57, 1196-1209`). Users wanted notes to belong to multiple categories (e.g. `company` + `pkm`). This introduced duplicate field collisions and complex precedence ordering [docs: 1837; src: 18558-18571].
- **Inherited Field Overrides:** Added in v0.6.4 (`docs: 1729`). Users required children classes to redefine options (e.g. `Select` choices) for an inherited parent field without rewriting the parent class [docs: 1338-1368].
- **Frontmatter-Only Scope Setting:** Added in v0.6.4 (`docs: 1734, 2094-2098`). Scanning large files for inline Dataview syntax (`field:: value`) caused unacceptable typing latency, prompting a setting to restrict MM strictly to YAML [docs: 2096-2097].
- **External Select Value Dictionaries:** Users requested dynamic and shared option lists, resulting in `valuesListNotePath` (shared markdown file) and `valuesFromDVQuery` (Dataview query) [src: 12858-12883].

### 4.2 `globalFileClass` vs Traces `global`

- **Metadata Menu `globalFileClass`:** A concrete fileClass note that applies automatically to all notes in the vault lacking an explicit `fileClass` [docs: 1242-1245, 2141-2147; src: 18629-18636]. It is a default note class, not an abstract schema.
- **Traces `global.toml`:** An abstract, non-inheritable shared pool of field definitions (`GLOSSARY.md:16-20`). Notes cannot be assigned `fileClass = "global"`. Fields in `global` serve purely as base targets for `$ref` overrides (`survey/schema.md:22`).

### 4.3 External Values Files: Markdown Lines vs Traces TOML/JSON Tables

- **Metadata Menu (`valuesListNotePath`):** Implemented in `FieldSetting.getValuesListFromNote` (`src/settings/FieldSetting.ts:21392-21403`) [V]. Reads a `.md` file, splits by newline `\n`, trims whitespace, and returns `string[]` [V]. No keys, no labels, no sort ordering.
- **Traces Select Engine (`fields/select.rs`):** A 2,032-line subsystem supporting structured TOML/JSON tables with `value`, `label`, `order` columns, float ordering, and cache confinement (`survey/schema.md:36, 56-63`). MM's implementation demonstrates that PKM users primarily need flat text lists, making Traces' 2k lines of multi-format dictionary parsing disproportionate to real-world usage [H].

---

## 5. Comparison: Traces Schema Features vs Metadata Menu

### 5.1 Direct Feature-by-Feature Mapping & Consumer Analysis

| Traces Feature | Metadata Menu Feature | MM Implementation & Consumer | Divergence / Architectural Contrast |
| :--- | :--- | :--- | :--- |
| `extends = ["parent"]` (Multi-parent DAG) | `extends: parent` (Single inheritance) | `FileClass.getAttributes` (`src: 14267-14288`). Consumed by `FieldsModal` and `insertMissingFields`. | Traces implements a multi-parent DAG (Kahn's sort, Tarjan cycles). MM supports only single-parent linear chains. |
| `$ref` overrides | Implicit override by identical `name` | `ancestorsAttributes` iteration (`src: 14287`). Child fields with same name replace ancestor fields. | Traces uses explicit `$ref = "ancestor.field"` with fine-grained option overrides. MM uses wholesale field replacement. |
| `global` definition pool | `globalFileClass` / `presetFields` | `settings.globalFileClass` (`src: 18629`) & `presetFields` (`src: 18637`). Applied to unassigned notes. | Traces `global` is a non-assignable abstract pool. MM `globalFileClass` is a full fallback class assigned to notes. |
| `excludes = ["field"]` | `excludes: [field]` | Frontmatter array (`src: 14163-14170`). Ancestor fields in `excludes` are skipped during inheritance. | **Identical concept and syntax.** Both systems discard named parent fields during inheritance resolution. |
| External select files (`.toml`/`.json`) | `valuesListNotePath` (`.md` lines) | `FieldSetting.getValuesListFromNote` (`src: 21392`). Splits markdown note by line. | Traces parses structured TOML/JSON tables with labels/sorts. MM reads plain newline-separated text lines. |
| `required = true` | *None* | *No counterpart.* MM treats all fields as optional; missing fields are advisory UI indicators (`<missing>`). | Traces schema parser models `required`, but never enforces it on notes. MM has no `required` property. |
| 6 core field kinds | 22 field kinds | `fieldTypes` (`src: 6424-6447`). Drives UI modals, formatters, and table controls. | Traces models basic types (`input`, `select`, `boolean`, `number`, `date`, `file`). MM includes complex widgets (`Canvas`, `Formula`, `Lookup`, `Object`). |
| Canonical name uniqueness (`FieldKey`) | Arbitrary string matching | Case-sensitive string matching (`field.name === yamlAttr`). Collisions resolved by array iteration order. | Traces enforces canonical name uniqueness across separators/case (`FieldKey`). MM suffers collision ambiguity. |

### 5.2 Asymmetries: What Each System Lacks

- **What Metadata Menu Has That Traces Lacks:**
  - Rich interactive write modals for 22 types (`src/fields/models/*`).
  - Native table views with dynamic filtering, sorting, and inline editing [docs: 1432-1475].
  - Persistent calculated fields (`Formula` and `Lookup` writing back to files) [src: 8684, 9300].
  - Arbitrary hierarchical nesting in frontmatter (`Object`, `ObjectList` with path indexing) [docs: 1753-1795].
  - Multiple note-to-class binding vectors (tags, directories, bookmark groups, queries) [docs: 1171-1179].
  - Programmatic diagnostic analysis (`fileFields` returning advisory `isValid`) [src: 2580].
- **What Traces Has That Metadata Menu Lacks:**
  - Multi-parent DAG inheritance with cycle detection (`src/schema/graph/`).
  - Bounded, acyclic `$ref` resolution bounded to ancestors or global.
  - Strict canonical field name uniqueness (`FieldKey::canonicalize`).
  - Deterministic compiled Rust CLI architecture without runtime DOM overhead.

---

## 6. Architectural Lessons for Traces Ticket 16

### 6.1 (a) Advisory vs Gating

- **Empirical Proof:** Metadata Menu is the primary inspiration for fileClasses in Obsidian, and it is **100% non-gating** [V]. It never prevents a note from being indexed, searched, or displayed because of a schema violation [V].
- **Lesson for Traces:** Ticket 16 MUST NOT make schema validation a gate for note indexing or query ingestion [P]. A vault is an open-world filesystem; treating schema violations as fatal errors breaks valid user workflows. Validation must be an on-demand, advisory diagnostic pass [P].

### 6.2 (b) Typing at Write vs Read

- **Typing at Write:** In MM, type checking is strictly an interactive write-time affordance (`valueModal.save()`) [V: 10532]. The modal prevents invalid numbers or dates from being entered through the UI, but does not police the underlying markdown file.
- **Typing at Read:** At read time, notes are ingested permissively as raw YAML/text [V: 19471].
- **Lesson for Traces:** Type enforcement belongs at write boundaries (`traces new`, interactive CLI prompts, LSP completions), not at storage or ingestion boundaries [P]. Storage and indexing must remain resilient and permissive.

### 6.3 (c) Schema-as-UI-Hints vs Schema-as-Contract

- **MM's Stance:** MM treats schemas entirely as **UI hints and autocompletion templates** [docs: 28-30, 75-96]. Fields serve to generate buttons, dropdowns, and insertion boilerplate.
- **Risk of Over-Engineering:** Traces built 12k LoC of strict schema compilation (Tarjan DFS, Kahn sort, CSR graphs) for a system that currently provides zero runtime behavior [V: `survey/schema.md:35-37`]. If schemas do not validate notes, they are passive prompt metadata.
- **Lesson for Traces:** Ticket 16 must decide whether Traces schemas remain pure UI hints (like MM) or become structured contracts for query execution and advisory diagnostics [P].

### 6.4 (d) Multi-Class & Inheritance Conflicts

- **MM's Fatal Flaw:** MM deduplicates fields across multi-class bindings using randomly generated field IDs (`field.id`) rather than canonical field names (`src: 18564, 18618`) [V]. When two mapped classes define a field named `status`, both IDs are retained in `filesFields`. During note parsing, the first matching class claims the frontmatter key; the second class reports `status` as `<missing>` [src: 4505-4510, 19415-19477] [V]. This caused persistent user bugs and required patch workarounds [docs: 1837].
- **Lesson for Traces:** Traces' decision to enforce unique canonical field names (`FieldKey::canonicalize`, `survey/schema.md:24`) is fully vindicated. Multi-class or multi-parent field merging MUST resolve by canonical field name, not arbitrary internal IDs [P].

### 6.5 (e) Dealing with Schema-Violating Data in Notes

- **MM's Handling:**
  - Extra unmapped fields: Ignored completely, preserved as-is [V: 19416].
  - Mismatched types: Displayed as raw text in modals and tables; reported as `isValid: false` in API [V: 2580, 4594].
  - Missing fields: Rendered as `<missing>`; bulk insertion provided via `insertMissingFields` [V: 4591, 4733].
- **Lesson for Traces:** Ticket 16 should adopt MM's exact tri-state classification for note validation:
  1. *Valid Field:* Matches schema definition.
  2. *Missing Field:* Declared in schema, absent in note (advisory diagnostic / quick-fix insertion).
  3. *Unmapped Field:* Present in note, undeclared in schema (open-world data; strictly allowed, non-diagnostic).
  4. *Type Violation:* Present in note, conflicts with schema type (advisory diagnostic with source span; query treats as null or coerces per Ticket 15) [P].

### 6.6 (f) Known Pitfalls, Limitations, and Anti-Patterns

- **Eager Value Indexing Chokes Vaults:** MM attempted full value indexing and had to rip it out in v0.6.4 due to crippling latency [docs: 1735]. Traces must never build an eager in-memory index of all note values across all schemas [P].
- **Single Inheritance Friction:** MM users frequently hit limits with single `extends`, forcing the developer to invent complex tag, path, and query mappings to combine classes [docs: 1171-1179]. Traces' multi-parent DAG is structurally superior.
- **Over-Engineered External Dictionaries:** MM's flat newline-delimited text files (`valuesListNotePath`) satisfied 95% of user needs [src: 21392]. Traces' 2,032 lines of multi-format external TOML/JSON tables in `select.rs` is confirmed accidental complexity [H].

---

## 7. Not Verified

1. **Table View Cell Editing under Concurrency:** Not verified how Metadata Menu resolves race conditions when multiple table cells in `fileClassTableView` are edited simultaneously while Obsidian's background indexer runs.
2. **Dataview First-Index Race Condition:** Release 0.6.5 notes claim *"dataview first index not causing competing queries recalculation anymore"* [docs: 1723], but the underlying synchronization lock mechanism was not verified in source.
3. **Canvas Performance on Large Files:** Not verified how Canvas node parsing behaves on vaults with hundreds of `.canvas` boards, beyond the presence of `getCanvasesFiles` in `FieldIndex.ts:18289-18313`.

---

## 8. Questions This Raises for the Grilling

1. **Passive Metadata vs Advisory Validator:** Metadata Menu proves a PKM tool can achieve massive adoption while schemas remain 100% passive UI hints. Should Traces leave `src/schema` completely disconnected from note parsing (Status Quo), or introduce a dedicated advisory diagnostic pass (`traces check` / LSP diagnostics)?
2. **The `required` Illusion:** Metadata Menu completely omitted `required` fields because in PKM, notes are drafted iteratively and rarely complete upon creation. Should Traces delete `required` from `RawSchemaFieldDef`, or keep it strictly for advisory lint warnings?
3. **Pruning `select.rs` (2,032 lines):** Metadata Menu supports external select values with 12 lines of code reading newline-separated notes (`FieldSetting.getValuesListFromNote`). Should Traces delete its 2,032-line structured TOML/JSON external select table cache and standardize on flat note lists or inline arrays?
4. **Interactive Write Gating vs Batch Gating:** MM blocks invalid inputs in its interactive UI modals (`valueModal`), but allows arbitrary raw text in files. If Traces implements write-time gating, should it apply only to interactive CLI commands (`traces new`, template prompts), while batch indexing remains strictly permissive?
5. **Handling Multiple `fileClass` Frontmatter Values:** If a note contains `fileClass: [project, task]`, and both schemas define `priority`, does first-listed win (Traces DAG convention), or should field collision across independent classes be reported as a validation diagnostic?

---

*Ponytail note: Researched and audited all 3 digests; avoided implementing any validator or editing schemas. Skipped tracing mobile-specific UI code and legacy v1 fileClass migration scripts.*
