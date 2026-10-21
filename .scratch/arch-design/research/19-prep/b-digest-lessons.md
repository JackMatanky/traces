# Digest Lessons: Coercion, Typing, Equality, and Ordering in PKM Tools

- **Topic:** Empirical mining of local PKM reference digests (`docs/digests/`) for Wayfinder Ticket 15.
- **Status:** Complete.
- **Deliverable:** `.scratch/arch-design/research/19-prep/b-digest-lessons.md`.

---

## 1. Obsidian Dataview: Dynamic Typing, Heuristic Coercion, and Universal Total Order

### 1.1 Ingestion and Frontmatter Classification
- **Greedy heuristic sniffing:** `parseFrontmatter` (`src/data-index/resolver.ts`, `obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3650-3693`) [V] recursively coerces untyped frontmatter strings. For any string, it attempts:
  1. `EXPRESSION.date.parse(value)` -> Luxon `DateTime` [V: 3679-3680].
  2. `EXPRESSION.duration.parse(value)` -> Luxon `Duration` [V: 3682-3683].
  3. `EXPRESSION.embedLink.parse(value)` -> `Link` [V: 3685-3686].
  4. Fallback to raw `string` [V: 3688].
- **Complete absence of frontmatter literal string escape:** The YAML parser strips quotes before `parseFrontmatter` runs [V: 3679]. `title: "2025-01-15"` and `title: 2025-01-15` produce the identical JS `string`, which `EXPRESSION.date.parse` eagerly converts to a `DateTime` [V: 3679-3680]. Dataview provides ZERO quote-based escape mechanism in frontmatter [V]. To treat such a field as text in queries, users MUST use explicit cast functions like `string(field)` (`docs-digest.txt:3853`) [V].
- **Inline fields use literal escape:** In inline field definitions (`key:: value`), quotes (`"..."`) invoke the Parsimmon string literal parser (`src-digest.txt:7818-7826`), preserving the text [V].

### 1.2 Universal Total Order (`Values.compareValue`)
- **Total ordering across arbitrary types:** `Values.compareValue(val1, val2, linkNormalizer)` (`src-digest.txt:5470-5564`) [V] establishes a total order over all possible values:
  - **Null handling:** `null` and `undefined` are equated and placed strictly before any non-null value (`null < non-null`) [V: 5472-5476].
  - **Mismatched types:** When `wrap1.type != wrap2.type`, comparison falls back to alphabetical order of the internal type tag: `wrap1.type.localeCompare(wrap2.type)` [V: 5487].
  - **Resulting type rank:** `"array" < "boolean" < "date" < "duration" < "link" < "number" < "object" < "string"` [V: 5487].
  - **Intra-type ordering:**
    - `number`: Native JS `<` and `==` on IEEE-754 doubles [V: 5493-5496].
    - `string`: `localeCompare` [V: 5491-5492].
    - `boolean`: `false < true` [V: 5500-5501].
    - `date`: Luxon `<` and `.equals()` [V: 5523-5527].
    - `duration`: Luxon `<` and `.equals()` [V: 5529-5533].
    - `link`: Normalized `link.path`, then `link.type`, then `subpath` existence and text [V: 5503-5521].
    - `array`: Lexicographic element-by-element comparison, then length [V: 5535-5541].
    - `object`: Sorted key list comparison, then key-by-key value comparison [V: 5543-5558].

### 1.3 Null Semantics, Truthiness, and Filtering
- **Unified comparison relation for Sort and Filter:** Dataview registers `BinaryOpHandler.compare("*", (a, b) => Values.compareValue(a, b))` (`src-digest.txt:6168`) [V]. Relational expressions (`<`, `<=`, `>`, `>=`, `=`, `!=`) and sorting clauses (`SORT field ASC/DESC`, `src-digest.txt:8276-8291`) [V] execute the EXACT same comparator [V].
- **Cross-kind filter evaluation:** Because `compareValue` is total, cross-type comparisons never error: `12 < "hello"` evaluates to `true` (since `"number" < "string"`); `12 = "12"` evaluates to `false` [V: 5487, 6128].
- **Null filter trap:** Because `null < non-null`, an unset field evaluates as `null <= date(today)` -> `true` [V: 5475]. The official documentation explicitly warns users of this trap (`docs-digest.txt:3794-3803`) [V] and advises adding explicit type guards: `WHERE typeof(due) = "date" AND due <= date(today)` or `WHERE due AND due <= date(today)` [V: 3799-3802].
- **Truthiness:** `Values.isTruthy(field)` (`src-digest.txt:5572-5600`) [V] governs `WHERE` (`src-digest.txt:8251`) [V]. Falsy values: `null`, `undefined`, `0`, `""`, `false`, link with empty path, date with 0 epoch millis, duration of 0 seconds, empty object, and empty array [V: 5576-5599].

### 1.4 Link Model and Equality
- **Link identity excludes display and embed:** `Link.equals(other)` (`src-digest.txt:5780-5784`) [V] defines semantic equality as:
  `this.path == other.path && this.type == other.type && this.subpath == other.subpath`.
  Neither `display` (alias) nor `embed` (`!`) participates in equality [V].
- **Link comparison excludes display:** In `compareValue` (`src-digest.txt:5507-5509`) [V], Dataview explicitly states: *"We can't compare by file name or display, since that would break link equality. Compare by path."* It compares normalized `path`, `type`, and `subpath` only [V].
- **Frontmatter link syntax:** YAML links MUST be quoted: `key: "[[Link]]"` (`docs-digest.txt:1870-1875`) [V]. Unquoted links fail YAML parsing [V].

### 1.5 Temporal Typing and Duration Comparison
- **Single temporal type (Luxon `DateTime`):** Dataview collapses `Date` and `DateTime` into Luxon `DateTime` instances (`src-digest.txt:3663, 7903-7945`) [V]. Pure dates (`YYYY-MM-DD`) are assigned `00:00:00.000` in the ambient local timezone (`src-digest.txt:7904-7906`) [V]. Date and DateTime compare on the exact same timeline via epoch milliseconds [V: 5523-5527].
- **Duration normalization:** `normalizeDuration` (`src-digest.txt:13850-13853`) [V] calls `dur.shiftToAll().normalize()`. Calendar units (years, months, days) are compared using fixed conversion factors (1 month = 30 days, 1 year = 365 days) without requiring a reference anchor date [V].

### 1.6 Explicit Cast Functions
- `date(any)` (`src-digest.txt:6865-6909`) [V]: Parses string via ISO-8601; extracts date from a `Link` via display, path, or `file.day` [V: 6872-6888]; accepts format strings (`DateTime.fromFormat`) [V: 6899]. Returns `null` on failure (DuckDB `TRY_CAST` semantics) [V: 6869].
- `dur(string)` (`src-digest.txt:6912-6921`) [V]: Parses duration strings; returns `null` on failure [V: 6916].
- `number(string)` (`src-digest.txt:6945-6954`) [V]: Regex extraction; returns `null` on failure [V: 6950].
- `string(any)` (`src-digest.txt:6971`) [V]: Converts any value to internal string representation [V].
- `fdefault(v, fallback)` (`src-digest.txt:7305-7308`, `docs-digest.txt:4613-4627`) [V]: Returns `fallback` if `Values.isNull(v)`, else `v` [V].
- `choice(bool, left, right)` (`src-digest.txt:7333-7336`) [V]: Ternary branch on `Values.isTruthy(bool)` [V].

---

## 2. Obsidian Tasks: Discrete Temporal Typing and Inverted Null Sort

### 2.1 Date Typing and Regex Representation
- **Date-only vs Timestamps:** Obsidian Tasks strictly separates date-only task fields (due, scheduled, start: `YYYY-MM-DD`) from timestamps (created, done, cancelled: `YYYY-MM-DD HH:mm`) (`obsidian_obsidian-tasks-src-digest.txt:26244-26245`) [V].
- **Internal Moment wrappers:** Fields are stored as `moment.Moment` instances [V: 5490, 26795]. Date-only fields are anchored to `startOf('day')` [V: 5086, 26334] to avoid timezone and time-of-day pollution during queries.

### 2.2 Comparison and Sort Order (`compareByDate`)
- **Inverted null sorting (`non-null < null`):** In `compareByDate(a, b)` (`src-digest.txt:5150-5174`) [V]:
  - `a !== null && b === null` returns `-1` (`a < b`) [V: 5151-5152].
  - `a === null && b !== null` returns `1` (`a > b`) [V: 5154-5155].
  - In Tasks ascending sort, scheduled/due items appear FIRST; undated tasks sort to the BOTTOM [V: 5151-5155]. This directly inverts Dataview's `null < non-null` policy [V].
- **Invalid dates sort before valid dates:**
  - `a.isValid() && !b.isValid()` returns `1` (`a > b`) [V: 5161-5162].
  - `!a.isValid() && b.isValid()` returns `-1` (`a < b`) [V: 5163-5164].
  - Invalid dates sort before valid dates, isolating corrupt metadata [V].
- **Task equality:** Tasks equality checks `compareByDate(...) === 0` across `startDate`, `scheduledDate`, and `dueDate` (`src-digest.txt:24646-24654`) [V].

---

## 3. Obsidian Metadata Menu: Schema-Bound Validation over Lexical Storage

### 3.1 Field Types and Lexical Storage
- **22 discrete field types:** `FieldType` (`obsidian_mdelobelle-metadatamenu-src-digest.txt:6375-6422`) [V] defines `Input`, `Number`, `Select`, `Cycle`, `Boolean`, `Date`, `DateTime`, `Time`, `Multi`, `File`, `MultiFile`, `Media`, `MultiMedia`, `Canvas`, `CanvasGroup`, `CanvasGroupLink`, `Formula`, `Lookup`, `JSON`, `YAML`, `Object`, `ObjectList`.
- **Purely lexical storage in Markdown:** All fields exist as raw Markdown text (frontmatter or inline) [V: 19696-19785]. Notes are parsed into CST/line nodes (`LineNode.ts`, `Note.ts:20019-20038`) [V]. Values are updated in-place via text rewriting (`app.vault.modify`) [V: 20034].

### 3.2 Advisory Validation vs Write-Gating
- **Advisory index:** Indexing and Dataview integration evaluate `validateValue(type)(fieldVM)` (`src-digest.txt:6751-6775`, `Field.ts:2570-2586`) [V]. Non-conforming fields emit `isValid: false` as diagnostic metadata but are NEVER excluded from queries or indexing [V].
- **Modal write-gating:** Strict validation is applied only at the modal form submission boundary (`Number.ts:10632`, `Date.ts:8469`, `File.ts:8669`) [V]. Direct Markdown file edits are never constrained [V].

### 3.3 Link Representation, Quotes, and Date/Select Matching
- **Forced quotes for frontmatter links:** In `renderValueString` (`src-digest.txt:19696-19708`) [V]:
  - If a value starts with `[[` or `![[`, it is automatically enclosed in double quotes: `"${_rawValue}"` [V: 19698-19699].
  - Multi-file links in YAML format as `"${item}"` or list items `- "[[Link]]"` [V: 19749, 19551].
  - To prevent YAML parsing from corrupting booleans and numbers, unquoted literals are emitted only if `parseYaml` preserves them; otherwise double quotes are applied [V: 19707].
- **Select value matching:** `validateValue` for `Select` checks string membership: `AbstractList.getOptionsList(managedField).includes(managedField.value)` (`src-digest.txt:11332-11334`) [V].
- **Link unwrapping in Date validation:** If a `Date` field contains a wikilink (e.g. `[[2025-01-15]]` or `[[daily/2025-01-15|Today]]`), `AbstractDate.validateValue` (`src-digest.txt:12406-12424`) [V] strips `[[` and `]]`, strips display alias `|...`, takes the filename (`split("/").last()`), and validates it against `dateFormat` using Moment [V: 12411-12422].
- **File target resolution:** `AbstractFile.validateValue` (`src-digest.txt:8669-8678`) [V] matches links against vault file paths using `metadataCache.getFirstLinkpathDest` (`src-digest.txt:12768-12776`) [V].

---

## 4. Templater: Ambient Execution Environment and String Template Output

### 4.1 Date Handling and Timezones
- **Ambient environment:** `InternalModuleDate` (`obsidian_silentvoid13-templater-src-digest.txt:2463-2541`) [V] delegates all operations to Obsidian's global `moment()` [V: 2489, 2501, 2509]. All offsets, formatting, and current time calls inherit the system's ambient local timezone [V].
- **Strict format error gating:** In `generate_now` and `generate_weekday`, if a reference date does not parse under `reference_format`, Templater immediately throws `TemplaterError("Invalid reference date format...")` (`src-digest.txt:2490-2492, 2526-2528`) [V], refusing silent degradation.
- **Offset flexibility:** Offsets accept string duration (`moment.duration("P1D")`) or numeric days (`moment.duration(offset, "days")`) [V: 2495-2499].

### 4.2 Number and String Output
- **Template rendering:** Templater compiles a Rust/WASM `Renderer` (`src-digest.txt:3908-3918`) [V]. Commands inside `<% ... %>` evaluate in standard JS context and are coerced to strings via JS `toString()` before insertion into Markdown text [V].

---

## 5. Metadata Cache & Linters: Common Frontmatter Pitfalls

### 5.1 Pitfalls of YAML Unquoted Coercion
- **The unquoted title/number trap:** In `obsidian-advanced-metadata-cache` (`obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt:16454-16471`) [V], `toTitle` explicitly addresses a major user pitfall:
  *"A `number` is accepted alongside a `string` because an unquoted `title: 2026` parses as one, and a user who typed it plainly meant the text. Everything else — a boolean, a nested mapping, `null` — is not a name anybody typed..."* [V: 16457-16460].
  `toTitle` coerces finite numbers to string via `rawTitle.toString()`, while discarding booleans and nulls [V: 16470].
- **YAML regex parsing limits in Obsidian Linter:** `obsidian-linter` (`obsidian_platers-obsidian-linter-digest.txt:244-250, 6434-6440`) [V] notes that parsing YAML keys and values via regex instead of an AST parser creates subtle corruption with multiline strings, embedded colons, and comment strips [V: 247-249].
- **Case sensitivity:** Array deduplication in YAML (`dedupe-yaml-array-values`, `obsidian-linter:6520-6553`) [V] is strictly case-sensitive (`[computer, Computer]` are preserved as distinct) [V: 6541, 6552].

---

## 6. Markdown LSP & ZK Engines: Link Identity and Target Resolution

### 6.1 Marksman (F# Language Server)
- **Wikilink AST and styles:** `WikiLink` (`lsp_artempyanykh-marksman-digest.txt:6231-6272`) [V] decomposes links into `doc` and `heading`. Supports three target link styles (`CompletionHelpers.wikiTargetLink`, `digest:4736-4750`) [V]: `TitleSlug` (heading/title slug), `FileStem` (filename without extension), and `FilePathStem` (canonical relative path without markdown extension) [V].
- **Extension stripping:** `CanonDocPath.mk` (`digest:9966-9969`) [V] strips known markdown extensions (`.md`, `.markdown`) to achieve canonical target identity [V].

### 6.2 Markdown Oxide (Rust Language Server)
- **Configurable extension and case rules:** Exposes `include_md_extension_wikilink: bool` and `case_matching: Case` (`lsp_feel-ix-343-markdown-oxide-src-digest.txt:1176-1180, 1240-1242`) [V].
- **Civil date resolution:** Resolves daily notes via `chrono::NaiveDate` (`src-digest.txt:588-602`) [V], preventing timezone shifts from altering date filenames [V].

### 6.3 VS Code Markdown Language Service (TypeScript)
- **Target resolution cascade:** `resolveLinkTarget` (`lsp_microsoft-vscode-markdown-languageservice-digest.txt:4416-4470`) [V]:
  1. Internal URI normalization [V: 4417-4422].
  2. Workspace existence stat; if absent, tries appending `.md` via `tryAppendMarkdownFileExtension` [V: 4446-4455].
  3. Fragment parsing: checks line number fragments, then heading text [V: 4467-4475].

### 6.4 ZK (Go CLI Notebook)
- **Anchor stripping:** `FindIdsByHref` (`zk-src-digest.txt:9218-9262`) [V] strips subpath `#anchor` first: `strings.SplitN(href, "#", 2)[0]` [V: 9221].
- **Ranked database matching:** Prioritizes exact path match with `.md` extension [V: 9226-9232], followed by filename substring `LIKE %href%` [V: 9237], path substring `LIKE %href%` [V: 9243], and path prefix `href%` [V: 9256].

### 6.5 Rumdl (Rust Markdown Linter)
- **Wikilink pipe masking:** `TableUtils::mask_pipes_in_wikilinks` (`lsp_rvben-rumdl-src-digest.txt:32621-32635`) [V] masks `|` characters inside wikilinks (`[[target|display]]`) prior to markdown table cell splitting, ensuring table columns are not fractured by link aliases [V].

---

## 7. Rust Obsidian Libraries: Frontmatter and Link Handling

### 7.1 obsidian-export (`rust_zoni-obsidian-export`)
- **Direct Serde YAML passthrough:** Defines `pub type Frontmatter = serde_yaml::Mapping;` (`rust_zoni-obsidian-export-digest.txt:1873-1898`) [V]. Performs zero custom type classification or promotion; all YAML scalars remain raw `serde_yaml::Value` [V].

### 7.2 obsidian-parser (`rust_cryptogladi-obsidian-parser`)
- **Generic typed properties:** Defines `pub trait Note { type Properties: Clone; ... }` (`rust_cryptogladi-obsidian-parser-digest.txt:350-379`) [V]. Users provide custom Serde structs (e.g. `NoteInMemory<CustomProperties>`), falling back to `HashMap<String, serde_yml::Value>` [V: 350, 362-368].

### 7.3 obsidian-cli (`rust_mcky-obsidian-cli`)
- **String flattening:** `yaml_to_string_map` (`rust_mcky-obsidian-cli-digest.txt:422-448`) [V] flattens all YAML properties into `String`: numbers and booleans are stringified via `.to_string()`, and sequences are joined with commas [V: 432-439].

---

## 8. Summary Matrix: Tool x Question

| Tool | Q1: Promotion Seam & Storage | Q2: Literal String Escape | Q3: Int vs Float Semantic Eq | Q4: Date vs DateTime & Timezone | Q5: Cross-Kind Total Order | Q6: Link Eq & Order | Q7: Select vs Link Match | Q8: Query Literal Sniffing | Q9: Duration Comparison | Q10: Collation & Case | Q11: Strictness & Error Mode |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Dataview** | Ingestion classifier (`parseFrontmatter`); eager typed memory index [V] | **None in frontmatter** (quote insensitive); inline uses quotes [V] | All IEEE f64; no distinction [V] | Unified Luxon `DateTime`; ambient local zone at parse [V] | `compareValue`: type-name alphabetical order; `null < non-null` [V] | Path + type + subpath only; `display`/`embed` excluded [V] | Explicit `date(link)` parses path/alias or resolves file [V] | Parsimmon grammar; no sort-time sniffing [V] | `normalizeDuration` shifts via 30d/mo, 365d/yr approximations [V] | JS `localeCompare`; case-sensitive default [V] | Soft fallback: `null` on failed cast (DuckDB `TRY_CAST` style) [V] |
| **Tasks** | Line regex parse; Moment wrappers [V] | N/A (line regex based) [V] | Floating point / integer math [V] | Strict separation (`YYYY-MM-DD` vs `HH:mm`); daily notes use `startOf('day')` [V] | `compareByDate`: **`non-null < null`** (inverted); invalid < valid [V] | N/A | N/A | N/A | `moment.duration` [V] | N/A | Corrupt dates flagged as `invalid`; sorted first [V] |
| **Metadata Menu** | Schema-bound (`fileClass`); purely lexical storage [V] | Quotes forced when serializing strings matching scalars/links [V] | Form validation on floats/ints [V] | Configurable format (`dateFormat`); Moment parse [V] | N/A | Normalized vault path via `getFirstLinkpathDest` [V] | Unwraps `[[...]]` and strips alias to match dates/options [V] | N/A | Shift durations (`1 day`) [V] | Case-sensitive options match [V] | Advisory validation (`isValid`); modal write-gating [V] |
| **Templater** | Template runtime; JS execution [V] | JS strings [V] | Standard JS numbers [V] | Global ambient `moment()`; local timezone leak [V] | N/A | N/A | N/A | N/A | `moment.duration` [V] | N/A | Strict: throws `TemplaterError` on invalid reference format [V] |
| **Advanced Metadata Cache** | Frontmatter name index; on-demand read [V] | Coerces unquoted `number` (e.g. 2026) to title string [V] | N/A | N/A | N/A | Frontmatter links indexed for backlinks [V] | N/A | N/A | N/A | Case-insensitive title deduplication [V] | Silent drop for booleans/nulls [V] |
| **LSP / ZK / Rumdl** | Static AST / SQLite index [V] | Respects quotes in frontmatter parser [V] | N/A | Naive date resolution (`chrono::NaiveDate` in Oxide) [V] | N/A | Strips `.md`, strips `#subpath`, resolves via SQLite or stat [V] | N/A | N/A | N/A | Configurable smart case / exact case [V] | Unresolved links produce advisory diagnostics [V] |
| **Rust Crates** | Serde YAML passthrough / generic Deserialize [V] | Handled by `serde_yaml` parser [V] | `serde_yaml::Number` (i64/u64/f64) [V] | Serde string passthrough [V] | N/A | Custom wikilink regex [V] | N/A | N/A | N/A | Standard Rust string cmp [V] | Serde deserialization errors [V] |

*Note on Digest Precedent:* Questions Q3 (exact Rust `i64` vs `f64` comparison without float truncation) and Q7 (Traces-specific schema select string matching against runtime `Link` values) have NO direct native implementation in JS tools, because JS engines collapse numbers into IEEE-754 doubles and lack unified schema/note link models [V]. Precedent for Q3 derives from database engines (SQLite `sqlite3IntFloatCompare`, PostgreSQL, DuckDB; see `research/03` and `c-external-precedent.md`) [V].

---

## 9. Implications for Ticket 15

### Q1: Promotion Seam & Lexical Storage with Typing-on-Demand
- **Findings:** Dataview proves that eager ingestion-time heuristic promotion (`parseFrontmatter`) creates permanent ambiguity (loss of string identity) and user surprises [V: 3679-3688]. Metadata Menu proves that purely lexical storage with on-demand parsing and advisory validation is robust and eliminates parsing data loss [V: 19696-19708, 6751-6775].
- **Options for Ticket 15:**
  1. *Keep as is (Heuristic Ingestion Promotion):* Convert frontmatter strings to `Date`/`DateTime`/`Duration` in `From<noyalib::Value>` using fixed precedence. Simple, but inherits Dataview's date/string collision flaw.
  2. *Schema-Bound Promotion (Recommended):* Store frontmatter strings lexically as `String` in the note read model; promote to `Date`/`DateTime`/`Duration` only when bound to a typed schema field or an explicit query cast (`date()`, `dur()`).
  3. *Typing-on-Demand with Cached Scalars:* Store raw strings, parse lazily on query/schema access, and cache in the query evaluation context.

### Q2: Frontmatter Literal-String Escape
- **Findings:** Dataview has no YAML quote escape [V]. Metadata Menu preserves quotes by forcing `"${val}"` when serializing strings that look like other scalars [V: 19707]. Advanced Metadata Cache explicitly compensates for YAML unquoted numbers (`title: 2026`) by coercing them back to strings [V: 16457-16470].
- **Options for Ticket 15:**
  1. *Keep as is:* Frontmatter classification is quote-insensitive; `title: "2025-01-15"` becomes a `Date`. No YAML quote escape; escape exists only in inline fields (`"..."`) or query explicit cast (`string(title)`).
  2. *Explicit Frontmatter String Escape (YAML Tag):* Support YAML tag `!str 2025-01-15` or special syntax. Rejected by ticket 14 (YAML tags dropped).
  3. *Schema Immunity:* Schema declaration `type = "string"` overrides heuristic classification, treating the value as text regardless of date shape.

### Q3: Int vs Float Semantic Equality and Order
- **Findings:** JS tools treat all numbers as f64 doubles, losing $2^{53}$ integer precision [V]. SQLite (`sqlite3IntFloatCompare`) and Python perform exact cross-type mathematical comparisons [V].
- **Options for Ticket 15:**
  1. *Keep as is (Structural Derived PartialEq):* `Int(1) != Float(1.0)`. In queries, `=` promotes `Int` to `Float` (risking precision loss > $2^{53}$).
  2. *Exact Mathematical Comparator in Query Engine (Recommended):* Derived `PartialEq` remains structural (for fast Rust collections and `Hash` coherence). The query expression evaluator and `NoteFieldValue::compare_numeric` implement SQLite's exact algorithm: compare sign/magnitude, truncate float in integer range, use fractional part for tie-breaking.

### Q4: Date vs DateTime Equality, Timezones, and Determinism
- **Findings:** Dataview collapses Date into Luxon DateTime at `00:00:00` in the ambient local zone, leaking the machine's timezone into index cache persistence [V: 7904-7906]. Tasks strictly isolates date-only fields (`YYYY-MM-DD`) from timestamps (`YYYY-MM-DD HH:mm`) and normalizes with `startOf('day')` [V: 26244, 5086]. Markdown Oxide uses `chrono::NaiveDate` [V: 588].
- **Options for Ticket 15:**
  1. *Keep as is (Ambient Zone Shift):* Naive datetimes are zone-shifted at parse time by ambient local zone; persisted in cache. Diverges across machines; breaks cache portability.
  2. *Strict Naive Separation (Recommended):* `NoteFieldValue::Date` uses `jiff::civil::Date`; `DateTime` uses `jiff::civil::DateTime` (or UTC `Timestamp`). Zero ambient timezone in `Ord`. Comparing `Date` vs `DateTime` in queries projects `Date` to `civil::DateTime` at `00:00:00` wall-clock time without timezone shift.

### Q5: Cross-Kind Rank Order and Equality Policy
- **Findings:** Dataview uses one universal total order (`Values.compareValue`) for both sort and filter, where `null < non-null` and cross-type comparison uses type-name alphabetical order (`"date" < "number" < "string"`) [V: 5487]. Tasks inverts null sort (`non-null < null`) to keep undated tasks at the bottom [V: 5151].
- **Options for Ticket 15:**
  1. *Keep as is (Unified Total Order):* `Null < Bool < Number < Duration < Temporal < Text < Link < List < Object` (`src/query/sort.rs:425-439`). Relational operators in filters (`<`, `>`, `=`) use this same order (so `1 < "text"` is true, `null < date` is true).
  2. *Two Distinct Relations (Recommended):*
     - **Sort Order:** Total order across all types (with configurable or explicit null placement: `NULLS FIRST` vs `NULLS LAST`).
     - **Filter Relation:** Cross-kind comparison between incompatible types (e.g. `Number` vs `String`, `Date` vs `Bool`) evaluates to `false` (or 3-valued `Null`), preventing Dataview's `null <= date(today)` false-match trap.

### Q6: Link Equality and Ordering
- **Findings:** Dataview's `Link.equals` and `compareValue` check ONLY normalized `path`, `type`, and `subpath` (`src-digest.txt:5780-5784, 5507-5509`) [V]. `display` (alias) and `embedded` (`!`) are strictly EXCLUDED from link equality and order [V]. ZK and VS Code strip fragments/anchors during file target resolution [V]. Rumdl masks pipes in wikilinks to prevent markdown table corruption [V: 32623].
- **Options for Ticket 15:**
  1. *Target-Only Semantic Equality (Recommended):* `Link::semantic_eq` compares target path and anchor/subpath (case-folded/normalized). `display` and `embedded` are ignored in semantic equality and query sort.
  2. *Full Structural Equality:* Derived `PartialEq` includes `embedded` and `display`. Rejected for queries: two links to the same file with different aliases would not match.

### Q7: Schema Select String vs Note Link Matching
- **Findings:** Metadata Menu matches links against file paths by resolving `destFile.path` [V: 8669-8678]. When a date field contains a wikilink, it strips `[[...]]` and aliases to extract the literal date text [V: 12411-12422]. Dataview's `date(link)` inspects `display`, then `path`, then target file [V: 6872-6888].
- **Options for Ticket 15:**
  1. *Automatic Unwrapping (Recommended):* When matching a schema `select` option (string) against a note `Link`, or when matching a string filter against a `Link`, compare the string against `link.target` and `link.display` (if present).

### Q8: Query Literal Promotion and Sort-Time Sniffing
- **Findings:** Dataview uses constructor functions (`date(...)`, `dur(...)`) in query literals and Parsimmon grammar [V: 4858-4859, 7810-7970]. It performs NO sort-time heuristic string sniffing (`TextShape::classify`) [V: 8276-8291].
- **Options for Ticket 15:**
  1. *Keep sort-time sniffing (`TextShape::classify`):* Sniffs strings at sort time to order date-like strings chronologically. Risky: non-uniform collation across rows.
  2. *Grammar-Level Typed Literals (Recommended):* Query parser produces typed literals (`Date`, `Duration`, `Number`). Eliminate runtime `TextShape` sniffing in `src/query/sort.rs:462`.

### Q9: Duration Comparison and Calendar Units
- **Findings:** Dataview uses Luxon's `shiftToAll().normalize()` with fixed approximations (30 days/month, 365 days/year) to totally order durations without a reference anchor date [V: 13850-13853].
- **Options for Ticket 15:**
  1. *Fixed-Ratio Normalization (Recommended):* Normalize `jiff::Span` / `DurationValue` using standard civil approximations (1 mo = 30d, 1 yr = 365.25d) when evaluating relational `<`/`>` in sort or queries without a reference date.
  2. *Incomparable Calendar Spans:* Return `None` for comparisons between calendar months and days without reference date. Accurate, but breaks total ordering in sort.

### Q10: String Collation and Unicode Normalization
- **Findings:** Dataview uses JS `localeCompare` [V: 5492]. Markdown Oxide supports configurable case matching (`Smart`, `Exact`) [V: 1179]. Advanced Metadata Cache normalizes link names case-insensitively [V: 15567, 16401].
- **Options for Ticket 15:**
  1. *Ord uses Ordinal/Unicode Scalar Byte Order:* Fast, deterministic, zero-allocation Rust `cmp`.
  2. *Case-Insensitive Query Option:* Provide explicit query functions (`lower()`, `icontains()`) or case-folding collation for filters.

### Q11: Strictness and Error Mode on Failed Coercion
- **Findings:** Dataview uniformly adopts DuckDB `TRY_CAST` semantics: failed coercions (`date("invalid")`, `number("foo")`) return `null` [V: 6869, 6950]. Metadata Menu uses advisory validation (`isValid`), restricting hard errors to interactive write modals [V: 2570, 10533]. Templater throws hard errors on invalid date format strings [V: 2490].
- **Options for Ticket 15:**
  1. *Silent Null Fallback (TRY_CAST Model - Recommended):* Heuristic coercion and explicit cast functions return `Null` on parse failure. Filters treat `Null` as falsey.
  2. *Diagnostic Reporting:* Return `Null` for evaluation, but attach an advisory diagnostic to the query result when strict mode is active.
