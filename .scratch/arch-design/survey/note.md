# Architectural Survey: Note Domain & Parsing Subsystem

> **Survey baseline: `2dc3250d`. Re-verification note (main @ `207cc3fb`).** Only line numbers moved: `src/index/service.rs` fileClass tests are now `mod class_field` (`service.rs:1732`), `visit_values` is `src/index/store.rs:1749`, `src/lib.rs` lines after 190 are +1; `src/task.rs` changed only at `TaskStatusMap::iter`. The note parser, model and `TaskStatus` shape are untouched. Authoritative corrections: `reverification.md`.

## 1. Responsibilities & seams

- **What it owns**: In-memory Markdown note document model (`Note`, `Heading`, `ListItem`, `ListText`, `ListItemType`, `TaskListItem`, `Link`, `LinkType`, `LinkTarget`, `Frontmatter`, `NoteFieldValue`, `NoteFieldValueRef`); infallible parsing via `parse_markdown` (`src/note/parser.rs:106`); pulldown-cmark event traversal; frontmatter YAML extraction via `noyalib` (`src/note/metadata.rs:57`); streaming task marker accumulation (`src/note/parser/marker.rs:15`); Logos-based item tokenization for inline fields, task emojis, dates, and tags (`src/note/parser/lexer.rs:14`); inline field recursive-descent value parsing (`src/note/parser/inline.rs:51`); hierarchical list item tracking and subtask completion rollup (`src/note/parser/list.rs:30`, `src/note/parser/task.rs:171`); outgoing link/wikilink extraction and target splitting (`src/note/links.rs:28, 113`); shared task and tag domain primitives (`src/task.rs`, `src/tag.rs`).
- **Public Surface (`src/lib.rs:157-167`)**:
  - Note model: `Note`, `Heading`, `ListItem`, `ListItemType`, `ListText`, `TaskListItem`, `Link`, `LinkType`, `LinkTarget`, `Frontmatter`, `NoteFieldValue`, `NoteFieldValueRef`, `MarkdownParserInput`, `parse_markdown`.
  - Primitives: `Tag`, `TagError`, `TaskDate`, `TaskDateSet`, `TaskDateSetIter`, `TaskDateType`, `TaskPriority`, `TaskStatus`, `TaskStatusMap`, `TaskStatusSymbol`, `TaskStatusType`, `TaskError`.
  - Crate-internal surface (`pub(crate)`): `Note::new` (`src/note/model.rs:43`), `Note::inline_fields` (`src/note/model.rs:122`), `Note::fields` (`src/note/model.rs:142`), `Note::get` (`src/note/model.rs:164`), `ListItem::fields` (`src/note/lists.rs:188`), `descendants_of` (`src/note/lists.rs:644`), `NoteFieldType` (`src/note/field.rs:209`), `RawFrontmatter` (`src/note/metadata.rs:18`).
- **Callers**:
  - `src/index/service.rs:261-277`: `IndexerService::parse_note` is the **only production caller** of `parse_markdown`.
  - `src/index/inlinks.rs:77, 351`: `InlinkResolver::new` reads `note.outlinks()` and `note.path()`.
  - `src/index/store.rs:42, 490, 1051`: Serializes/deserializes `Note` to/from redb `NOTES` via `postcard`.
  - `src/index/entry.rs:138, 161`: `FileEntry` holds `Option<Box<Note>>`.
  - `src/query/service.rs:296`: `QueryService::run` calls `note.lists()` for `lists`/`tasks` sources.
  - `src/query/results.rs:84, 161, 202, 314, 324-332`: `QueryRow` reads `note.get(key)`, `note.tags()`, `note.lists()`, `item.fields()`, `item.tags()`.
  - `benches/note_parsing.rs:40, 103`: Criterion parsing suite measuring `parse_markdown` across small/medium/large notes, scaling, workloads.
- **What callers must know**:
  - `parse_markdown` is **strictly infallible** (`src/note/parser.rs:106`): malformed YAML frontmatter, broken links, invalid field syntax, and truncated task markers fall back silently to plain text or empty maps.
  - `Note` is **strictly immutable**: once parsed, no mutation methods exist on `Note`, `ListItem`, `Heading`, or `Link`. Modifying any note requires a full re-parse.
  - `Note` completely discards AST byte spans and offsets (`src/note/model.rs:24-33`): callers cannot locate elements in the raw file or perform in-place edits. Only `Heading` and `ListItem` preserve 1-indexed `SourceLine`.
  - `Note::get(key)` returns only the *first* value of an inline field (`src/note/model.rs:172`), giving precedence to frontmatter (`src/note/model.rs:165`). Subsequent values in `IndexMap<FieldKey, Box<[NoteFieldValue]>>` are ignored.

## 2. Invariants

- **Infallible Parsing**: `parse_markdown(&MarkdownParserInput) -> Note` never returns `Result` or panics (`src/note/parser.rs:106`). Frontmatter YAML errors log via tracing and yield empty frontmatter (`src/note/metadata.rs:92-94`); invalid inline fields fall back to `NoteFieldValue::String` (`src/note/parser/inline.rs:85`); unclosed task markers flush as plain text (`src/note/parser/marker.rs:23, 75`). Enforced: construction / runtime.
- **Strict Document Order**: `headings`, `lists`, `outlinks`, `tags`, and `inline_fields` preserve appearance order in source (`src/note/model.rs:28-33`, `src/note/parser.rs:379-391`). Enforced: runtime vector append order in event traversal and `IndexMap` entry ordering.
- **Complete AST Span Discard**: Neither `Note`, `Link`, `Frontmatter`, `Tag`, nor `NoteFieldValue` store byte offsets or spans (`src/note/model.rs:24-33`). Only `Heading.line` (`src/note/heading.rs:13`) and `ListItem.line` / `parent` (`src/note/lists.rs:45-46`) preserve 1-indexed line numbers. Enforced: type system.
- **Dual-Pass pulldown-cmark Parsing**: Every note is parsed twice by pulldown-cmark: Pass 1 with `MARKDOWN_OPTIONS` for metadata (`src/note/parser.rs:108`), Pass 2 with `HEADING_OPTIONS` (adds `ENABLE_MATH`) in `collect_headings` (`src/note/parser.rs:113, 121-123`) to strip math delimiters from heading text without breaking math-enclosed link extraction in pass 1 (`src/note/parser.rs:935`). Enforced: runtime control flow.
- **Flat List Hierarchy Representation**: List trees are flattened into a contiguous slice `Box<[ListItem]>` (`src/note/model.rs:28`). Nesting is encoded via `depth: u8` and `parent: Option<SourceLine>` (`src/note/lists.rs:44-46`). Descendant traversal requires linear scanning via `descendants_of` (`src/note/lists.rs:648`). Enforced: data representation.
- **Canonical Field Key Identity & Case Normalization**: Field keys are normalized to lowercase alphanumeric canonical form (`FieldKey::canonical`, `src/field.rs:44-58`). Accessors (`Note::get`, `Frontmatter::get`) normalize queried strings before hash lookup (`src/note/model.rs:170`, `src/note/metadata.rs:113`). Enforced: type construction (`FieldKey`).
- **Metadata Precedence: Frontmatter > Inline Fields**: `Note::fields()` and `Note::get()` check frontmatter first; inline fields sharing a canonical key with a frontmatter field are shadowed (`src/note/model.rs:145-155, 164-169`). For inline fields, only the first occurrence is returned by `get()` (`src/note/model.rs:172`). Enforced: runtime accessor logic.
- **Ambient Timezone Leak in Temporal Ordering**: Cross-kind comparison between `Date` and `DateTime` promotes `Date` to midnight via `DateTimeValue::try_from(*a)` (`src/note/field.rs:297-300`), which invokes the host OS local timezone (`src/temporal/datetime.rs:58`). Total ordering depends on process-ambient timezone. Enforced: nowhere (leaked invariant).
- **Zero Normalization in Numeric Equality**: Float comparisons normalize `-0.0` to `+0.0` before `total_cmp` (`src/note/field.rs:292, 495-498`), ensuring IEEE-754 signed zero does not partition identical numbers. Enforced: runtime helper.
- **Tag Syntax Invariant**: `Tag` validates a leading `#`, non-empty alphabetic first body char, and alphanumeric/`_`/`/`/`-` segments (`src/tag.rs:56-75`). Cannot be constructed with invalid syntax. Enforced: type construction (`Tag::parse`).
- **Delimiter Prohibition in Task Status Symbols**: Task status symbols cannot be any paired delimiter character (`[`, `]`, `(`, `)`, `{`, `}`) (`src/task.rs:227`, `src/delimiter.rs:136`). Enforced: runtime check on `TaskStatusMap::insert`.
- **Double Storage of List Metadata**: All inline fields and tags on list items are stored on `ListItem.fields` / `ListItem.tags` (`src/note/lists.rs:48-49`), and ALSO duplicated into `Note.inline_fields` and `Note.tags` (`src/note/parser.rs:488-498`). Enforced: runtime parser flush.

## 3. Verified observations

- [V] Two pulldown-cmark passes per note: `parse_markdown` traverses the source twice via `Parser::new_ext` (`src/note/parser.rs:108, 121`), once for metadata and once for `collect_headings`.
- [V] GLOSSARY claims for Slug, Footnote Reference/Definition, and Callout are refuted by code: `src/note/GLOSSARY.md:45-53` claims heading slugs are deduplicated at parse time (`slug-1`); `Heading` has no slug field (`src/note/heading.rs:10-14`). Lines 84-96 claim footnote reference and definition nodes exist; `Note` has no footnotes field (`src/note/model.rs:24-33`) and `Tag::FootnoteDefinition` is unhandled (`src/note/parser.rs:252`). Lines 98-105 claim callouts are parsed; `Note` has no callouts field.
- [V] Quadruplicate value representations across the crate: `FieldValue` (`src/field.rs:596`), `FieldValueRef` (`src/field.rs:717`), `NoteFieldValue` (`src/note/field.rs:43`), and `NoteFieldValueRef` (`src/note/field.rs:179`). Frontmatter parsing performs a 3-stage conversion chain: `noyalib::Value` -> `FieldValueRef` -> `NoteFieldValue` (`src/note/metadata.rs:71-72`).
- [V] Unused public/internal API methods: `Note::inline_fields` (`src/note/model.rs:122`) and `Note::fields` (`src/note/model.rs:142`) carry `dead_code` expect attributes and have zero callers outside unit tests.
- [V] Multi-valued inline fields are inaccessible to queries: `Note.inline_fields` allocates `IndexMap<FieldKey, Box<[NoteFieldValue]>>` (`src/note/model.rs:31`), but `Note::get` (`src/note/model.rs:172`) and `QueryRow::resolve_metadata_ref` (`src/query/results.rs:328, 332`) only ever inspect `values.first()`.
- [V] Every task item allocates an owned String for status name: `TaskListItem` contains `TaskStatus` (`src/note/lists.rs:351`), which holds `name: String` (`src/task.rs:53`). For a vault with thousands of tasks, `"Todo"` and `"Done"` strings are allocated repeatedly on the heap.
- [V] List items are 160 bytes each: `ListItem` struct size is 160 bytes (`src/note/lists.rs:41-50`), largely due to `TaskListItem` (~88 bytes) inside `ListItemType` and `ListText` (48 bytes: `raw` + `clean`).
- [V] Upward dependency edge from `note` to `config`: `src/note/parser/input.rs:9` imports `FrontmatterConfig` and `TaskConfig` from `crate::config`, creating an inverted dependency from a core document parser to the high-level configuration subsystem.
- [V] Complete loss of byte spans: `Note`, `Link`, `Frontmatter`, `Tag`, and `NoteFieldValue` hold no byte ranges or offsets (`src/note/model.rs:24-33`). Only `Heading` (`src/note/heading.rs:13`) and `ListItem` (`src/note/lists.rs:45`) retain 1-indexed `SourceLine`.
- [V] Double storage of list item fields and tags: Every inline field and tag on a `ListItem` is stored in `ListItem.fields` / `ListItem.tags` (`src/note/lists.rs:48-49`) and ALSO cloned into `Note.inline_fields` and `Note.tags` (`src/note/parser.rs:493-496`).
- [V] Redb table NOTES stores full serialized Note: `IndexStore` serializes `Note` with `postcard` directly into redb `NOTES` (`src/index/store.rs:42, 1051`).
- [V] Delimiter classification coupling: `src/delimiter.rs` defines `DelimiterType`, used by `src/note/links.rs:303` (wikilinks), `src/note/parser/lexer.rs:154` (wrapped fields), `src/note/parser/marker.rs:30` (task markers), and `src/task.rs:227` (status symbol validation).

## 4. Hypotheses

- **[H1] Single Unified Value Hierarchy vs Quadruplicate Enums (Rank 1)**:
  - *Thesis*: Unifying `FieldValue` / `FieldValueRef` (`src/field.rs:596,717`) and `NoteFieldValue` / `NoteFieldValueRef` (`src/note/field.rs:43,179`) into a single canonical `Value` / `ValueRef` type hierarchy eliminates the 3-stage `noyalib -> FieldValueRef -> NoteFieldValue` conversion chain (`src/note/metadata.rs:71-72`), removes ~1,500 lines of duplicated enum logic and match arms, and harmonizes schema validation with note fields.
  - *Current vs Alt*: Current maintains separate types for schema options/selects vs note metadata, where `FieldValue` has `Int`/`Float` while `NoteFieldValue` collapses to `f64` but adds `Duration`/`Link`. Alternative: A single `Value` type supporting `Int`, `Float`, `Duration`, `Link`, `Date`, `DateTime`, `String`, `Bool`, `Null`, `List`, `Object`. (Keep as is preserves separate numeric representations).
  - *Removable*: `src/note/field.rs:480-575` conversion logic; `src/field.rs:641-660`; redundant `NoteFieldValue` definitions.
  - *Disprove*: Schema TOML parsing requires strict `i64` precision that cannot tolerate `f64` representation, or query engine performance regresses due to extra variant overhead.
  - *Blast radius*: `src/field.rs`, `src/note/field.rs`, `src/schema/`, `src/query/results.rs`, `src/template/engine/`.
- **[H2] Fuse pulldown-cmark Double Pass into Single Pass (Rank 2)**:
  - *Thesis*: Merging `collect_headings` (`src/note/parser.rs:118-162`) into the primary `handle_event` loop (`src/note/parser.rs:241-258`) eliminates the redundant second pulldown-cmark parse over the entire source text for every markdown file.
  - *Current vs Alt*: Current runs two complete passes of `Parser::new_ext` over `input.src()` because math delimiters (`$`) in headings are stripped only under `HEADING_OPTIONS`. Alternative: Run a single pass with `MARKDOWN_OPTIONS` and strip math delimiters during inline text accumulation or post-process heading text; or run single pass with math enabled and handle outlinks explicitly.
  - *Removable*: `collect_headings` function (`src/note/parser.rs:118-162`); cuts pulldown-cmark parsing invocations by 50% across vault indexing.
  - *Disprove*: Math delimiter handling inside pulldown-cmark interacts with wikilinks in headings in ways that cannot be reconciled without two distinct parser configurations (`src/note/parser.rs:935`).
  - *Blast radius*: `src/note/parser.rs`.
- **[H3] Interned Task Statuses vs Owned String Duplication (Rank 3)**:
  - *Thesis*: Replacing `TaskStatus` in `TaskListItem` (`src/note/lists.rs:351`) with `TaskStatusSymbol` or an interned `TaskStatusId` reduces `ListItem` struct size from 160 bytes to ~128 bytes and eliminates thousands of redundant heap allocations of status names (`"Todo"`, `"Done"`).
  - *Current vs Alt*: Current embeds owned `TaskStatus { symbol, name: String, kind }` in every task item. Alternative: `TaskListItem` stores only `symbol: TaskStatusSymbol` (1 byte) or `kind: TaskStatusType` (1 byte), resolving status name from `TaskStatusMap` during display/projection. (Keep as is: self-contained items).
  - *Removable*: `name: String` field from `TaskStatus` inside `TaskListItem`; clones during `finish()` (`src/note/parser/list.rs:424-432`).
  - *Disprove*: Queries or templates frequently project status names in tight inner loops where looking up the symbol in `TaskStatusMap` adds hash map lookup overhead exceeding string dereference.
  - *Blast radius*: `src/note/lists.rs`, `src/task.rs`, `src/query/results.rs`.
- **[H4] Invert Note → Config Dependency via Parser Configuration Object (Rank 4)**:
  - *Thesis*: Decoupling `note` from `config` by passing raw primitives (`&TaskStatusMap`, `&[Tag]`, `&str`) to `MarkdownParserInput` (`src/note/parser/input.rs:9-34`) restores clean architectural layering where domain parsers do not depend on high-level configuration.
  - *Current vs Alt*: `MarkdownParserInput` imports `FrontmatterConfig` and `TaskConfig` from `crate::config`. Alternative: `MarkdownParserInput` takes `statuses: &'a TaskStatusMap`, `task_tag_filters: &'a [Tag]`, `tags_field_name: &'a str`. (Keep as is: convenient container).
  - *Removable*: Upward dependency edge `note -> config` in crate module graph.
  - *Disprove*: Grouping config into `FrontmatterConfig` and `TaskConfig` simplifies passing parameters from `IndexerService` and prevents parameter proliferation.
  - *Blast radius*: `src/note/parser/input.rs`, `src/index/service.rs`.
- **[H5] Deduplicate List Item Metadata from Note Document Collections (Rank 5)**:
  - *Thesis*: Ceasing the duplication of `ListItem.fields` and `ListItem.tags` into `Note.inline_fields` and `Note.tags` (`src/note/parser.rs:488-498`) cuts memory overhead during parsing and resolves ambiguous multi-value ownership.
  - *Current vs Alt*: Current pushes list item fields/tags to both the item and the parent note. Queries on notes inspect both (`src/query/results.rs:324-333`). Alternative: `Note.inline_fields` holds ONLY body-level inline fields (not in lists); `Note.tags` holds body-level tags; `note.fields()` / `note.tags()` accessors chain item collections dynamically or indexer queries items directly.
  - *Removable*: Vector re-allocations in `extend_from_flush` (`src/note/parser.rs:490-498`); duplicated metadata entries in redb `NOTES` table.
  - *Disprove*: File-level queries (`traces list --from "#tag"` or `WHERE field = ...`) rely on fast iteration over `Note.tags` and `Note.inline_fields` without traversing `lists`.
  - *Blast radius*: `src/note/parser.rs`, `src/note/model.rs`, `src/query/results.rs`.
- **[H6] Span-Preserving Parse Pipeline for Surgical Note Mutation & LSP (Rank 6)**:
  - *Thesis*: Retaining `ByteSpan` on `Heading`, `ListItem`, `Link`, and `Frontmatter` enables future note mutation (e.g., toggling task status, updating fields) and LSP features (definition jump, hover) without reparsing and re-serializing the entire file.
  - *Current vs Alt*: Current discards all spans at parse time; only `SourceLine` is retained on headings and lists. Alternative: Add `span: ByteSpan` to `Heading`, `ListItem`, `Link`, `RawFrontmatter`. (Keep as is: minimal footprint for read-mostly index).
  - *Removable*: Need for external text search when mapping query results back to file offsets; enables precise diagnostics.
  - *Disprove*: Adding `ByteSpan` (8 bytes) to every AST node increases serialized `Note` size in redb table and postcard overhead without immediate benefit in a read-mostly CLI tool.
  - *Blast radius*: `src/note/heading.rs`, `src/note/lists.rs`, `src/note/links.rs`, `src/note/metadata.rs`.
- **[H7] Eliminate Ambient Timezone Leak in Temporal Comparison (Rank 7)**:
  - *Thesis*: Comparing `Date` and `DateTime` using UTC midnight or an explicit timezone passed to the comparison (`src/note/field.rs:297-300`) makes `NoteFieldValueRef::compare` a pure, deterministic function independent of process-ambient OS timezone.
  - *Current vs Alt*: Current calls `DateTimeValue::try_from(*a)` which invokes `Local::now().offset()` / local zone. Alternative: Strict comparison where `Date` is interpreted at UTC midnight or cross-kind ordering is explicitly partitioned (`Date < DateTime` or vice versa).
  - *Removable*: System timezone syscalls and non-deterministic sorting across machines.
  - *Disprove*: Users expect `2025-01-15` to match `2025-01-15T00:00:00` in their local timezone when sorting task due dates.
  - *Blast radius*: `src/note/field.rs`, `src/temporal/datetime.rs`, `src/query/sort.rs`.
- **[H8] Remove Dead Code Accessors and Align Multi-Value Field Model (Rank 8)**:
  - *Thesis*: Either implementing multi-value inline field querying or simplifying `Note.inline_fields` from `IndexMap<FieldKey, Box<[NoteFieldValue]>>` to `IndexMap<FieldKey, NoteFieldValue>` eliminates dead accessor code (`Note::fields`, `Note::inline_fields`) and matches actual query behavior.
  - *Current vs Alt*: Current stores `Box<[NoteFieldValue]>` for every key, but `Note::get` and query resolution only take `values.first()`. Alternative: If multi-values are unsupported, store single values; if supported, expose multi-value accessors to `QueryRow`.
  - *Removable*: `Box<[NoteFieldValue]>` inner indirection; `dead_code` attributes in `src/note/model.rs:113-142`.
  - *Disprove*: Dataview specification requires multiple `key:: val` occurrences to accumulate into a list, and future query syntax (`WHERE contains(key, val)`) will require accessing all values.
  - *Blast radius*: `src/note/model.rs`, `src/note/parser.rs`, `src/query/results.rs`.

## 5. Cross-slice couplings

- **`note -> config`**: `src/note/parser/input.rs:9` imports `FrontmatterConfig` and `TaskConfig`. Inverted dependency where core parsing depends on config structs instead of minimal interfaces.
- **`note -> temporal`**: `src/note/field.rs:10` and `src/note/parser/inline.rs:11` depend heavily on `DateValue`, `DateTimeValue`, `DurationValue`, and `DatePoint`. `NoteFieldValueRef::compare` leaks local timezone via `DateTimeValue::try_from`.
- **`index -> note`**: `IndexerService` is the only production orchestrator of `parse_markdown` (`src/index/service.rs:261`). `IndexStore` persists full `Note` structs into redb `NOTES` via `postcard` (`src/index/store.rs:42, 1051`), coupling the storage engine directly to the AST struct layout.
- **`query -> note`**: `src/query/results.rs:28, 320-415` directly accesses `Note`, `ListItem`, `TaskListItem`, and `NoteFieldValueRef`. Furthermore, query semantics (`is_equal_to_literal`, `is_containing`) are implemented directly inside `src/note/field.rs:393-447` rather than in `src/query/`.
- **`schema <-> note` (decoupled but duplicate)**: `src/schema/` has zero references to `src/note/`, but defines its own duplicate `FieldValue` enum in `src/field.rs:596` with no mapping to `NoteFieldValue`.
- **`root -> note` couplings**: `src/delimiter.rs` (`DelimiterType`) is coupled to wikilinks (`src/note/links.rs`), task markers (`src/note/parser/marker.rs`), and wrapped field callbacks (`src/note/parser/lexer.rs`). `src/task.rs` is placed at the crate root to resolve mutual dependencies between `config`, `note`, and `query`.

## 6. Comparables worth studying

- **Obsidian Dataview (`src/data-model/value.ts`, `src/data-import/inline-field.ts`)**: Informs **[H1]**, **[H5]**, **[H8]**. How Dataview parses inline fields, whether list item fields are rolled up to page metadata, and how multi-valued keys (`key:: a`, `key:: b`) are unified into arrays.
- **rumdl (`src/rules/md051.rs`, heading slug derivation)**: Informs **[H2]** and GLOSSARY slug refutation. How Markdown linter rumdl extracts heading slugs without running multiple parser passes.
- **rustc / Ruff AST span representation (`ruff_source_file::TextRange`, `SimpleAST`)**: Informs **[H6]**. How production CLI compilers and linters preserve compact 32-bit `TextRange` byte spans without bloating AST memory footprint.
- **Polars / DataFusion (`ScalarValue`, `Arrow DataType`)**: Informs **[H1]**. Canonical single value hierarchy for query and data processing engines, avoiding fragmented parallel enums.
- **Tasks (Obsidian Tasks plugin, `src/Task.ts`)**: Informs **[H3]**. How Tasks models task status symbols, emoji dates, and priority without duplicating status metadata per task instance.

## 7. Open decisions

- **Decision: Unify Field Value Hierarchies vs Keep Schema and Note Separate**:
  - Should `FieldValue` (`src/field.rs`) and `NoteFieldValue` (`src/note/field.rs`) be consolidated into a single unified `Value` enum across the entire crate?
- **Decision: Multi-Value Inline Field Query Semantics vs First-Wins Flattening**:
  - Should Traces officially support multi-valued inline fields in queries (e.g. projecting or filtering all values), or formalize first-wins semantics and simplify `IndexMap<FieldKey, Box<[NoteFieldValue]>>` to `IndexMap<FieldKey, NoteFieldValue>`?
- **Decision: AST Span Preservation vs Zero-Span Model**:
  - Should `Note` adopt compact byte spans (`ByteSpan`) for headings, list items, links, and frontmatter to prepare for in-place file modifications and LSP diagnostics, or maintain the zero-span model to keep redb persistence compact?
- **Decision: Single-Pass Heading and Math Extraction**:
  - Can pulldown-cmark configuration be reconciled into a single pass that extracts headings and metadata simultaneously without breaking math-enclosed wikilinks?
- **Fog (cannot yet state sharply)**:
  - How footnote references/definitions and callouts should be modeled when implemented: whether they belong as top-level fields on `Note` or nested within a generalized block stream.
  - The boundary between `ListItem` and `Note`: whether list items should remain part of the persisted `Note` in redb or be indexed into a dedicated index table if task queries scale to tens of thousands.

## 8. Withstands scrutiny

- **MarkerAccumulator Byte-Level Streaming State Machine (`src/note/parser/marker.rs:189-204`)**:
  - *Soundness*: Buffering the first 8 bytes across streaming pulldown-cmark text events cleanly reconciles CommonMark task list parsing with Obsidian arbitrary-character task markers (`[?]`, `[-]`, `[/]`). Handles broken events without allocating strings.
- **Single-Pass Logos Item Tokenization (`src/note/parser/lexer.rs:116-159`, `src/note/parser/task.rs:128-153`)**:
  - *Soundness*: Tokenizing priority emojis, date shorthands, tags, and inline fields in a single Logos pass using look-behind callbacks (`char_before`) and computing `clean_text` by direct slice concatenation without regexes or intermediate string allocations is fast and well-tested.
- **Infallible Parser Seam (`src/note/parser.rs:106`)**:
  - *Soundness*: `parse_markdown` never failing is an essential invariant for a personal knowledge management CLI. Malformed user notes must never abort indexing or crash the tool; degradation to plain text is the correct behavior.
- **Flat List Hierarchy with Contiguous Descendant Slices (`src/note/lists.rs:41-50, 644-649`)**:
  - *Soundness*: Encoding list trees as a flat array in document order with `depth: u8` and `parent: Option<SourceLine>` avoids pointer chasing, simplifies postcard serialization, and makes descendant iteration cache-friendly via simple linear scans (`descendants_of`).
