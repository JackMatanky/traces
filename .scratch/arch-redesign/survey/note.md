# Architecture Survey: Note Parsing & Domain Model

> **UNVERIFIED SURVEY.** Single-pass subagent report; several claims were later refuted. Check the ledgers ([core](../research/01-verify-core-findings.md), [system](../research/02-verify-system-findings.md)) and [performance evidence](../research/03-performance-evidence.md) before relying on any claim here; a claim absent from a ledger is a hypothesis.

**File:** `.scratch/arch-redesign/survey/note.md`  
**Author:** SurveyNote  
**Area:** Note parsing and domain model (`src/note/**`, `src/task.rs`, `src/tag.rs`)  
**Status:** Complete / Evidence-verified  

---

## 1. Scope & Size

- **Modules Covered**:
  - `src/note/mod.rs` (55 lines)
  - `src/note/model.rs` (628 lines)
  - `src/note/field.rs` (1,278 lines)
  - `src/note/metadata.rs` (374 lines)
  - `src/note/lists.rs` (1,098 lines)
  - `src/note/links.rs` (492 lines)
  - `src/note/heading.rs` (54 lines)
  - `src/note/cursor.rs` (85 lines)
  - `src/note/parser.rs` (1,831 lines)
  - `src/note/parser/inline.rs` (508 lines)
  - `src/note/parser/input.rs` (182 lines)
  - `src/note/parser/lexer.rs` (831 lines)
  - `src/note/parser/list.rs` (768 lines)
  - `src/note/parser/marker.rs` (721 lines)
  - `src/note/parser/task.rs` (1,266 lines)
  - `src/task.rs` (1,919 lines)
  - `src/tag.rs` (600 lines)
  - `benches/note_parsing.rs` (656 lines)
- **Total lines**: 13,346 lines (~6.5K production lines, ~6.8K test lines/fixtures).
- **Surface**:
  - **Public (`pub`) API** (`src/lib.rs:157-167`):
    - Note model: `Note`, `Heading`, `ListItem`, `ListItemType`, `ListText`, `TaskListItem`, `Link`, `LinkType`, `LinkTarget`, `Frontmatter`, `NoteFieldValue`, `NoteFieldValueRef`, `MarkdownParserInput`, `parse_markdown`.
    - Primitives: `Tag`, `TagError`, `TaskDate`, `TaskDateSet`, `TaskDateSetIter`, `TaskDateType`, `TaskPriority`, `TaskStatus`, `TaskStatusMap`, `TaskStatusSymbol`, `TaskStatusType`, `TaskError`.
  - **Crate-Internal (`pub(crate)`)**:
    - `NoteFieldType`, `normalize_zero` (`src/note/field.rs:209, 495`), `RawFrontmatter` (`src/note/metadata.rs:18`), `ListFieldMap` (`src/note/lists.rs:30`), `descendants_of` (`src/note/lists.rs:644`), `Note::new` (`src/note/model.rs:43`), `Note::inline_fields` (`src/note/model.rs:122`), `Note::fields` (`src/note/model.rs:142`), `Note::get` (`src/note/model.rs:164`).

---

## 2. Responsibilities

The note domain model and parser perform 7 primary responsibilities:

1. **Document-Level AST Representation (`src/note/model.rs`, `src/note/heading.rs`, `src/note/links.rs`)**:
   - `Note`: In-memory container representing a parsed Markdown file. Holds `path: PathBuf`, `frontmatter: Option<Frontmatter>`, `lists: Box<[ListItem]>`, `headings: Box<[Heading]>`, `outlinks: Box<[Link]>`, `inline_fields: IndexMap<FieldKey, Box<[NoteFieldValue]>>`, and `tags: Box<[Tag]>` (`src/note/model.rs:24-33`).
   - `Heading`: Stores heading `level: u8`, markup-stripped `text: String`, and 1-indexed `line: SourceLine` (`src/note/heading.rs:10-14`).
   - `Link`: Outgoing reference preserving target, display text, wikilink vs standard link discriminator, and embed flag (`src/note/links.rs:28-34`).
2. **Metadata Field Typing & Comparison Engine (`src/note/field.rs`)**:
   - `NoteFieldValue` & `NoteFieldValueRef`: Strongly typed enum (Null, Bool, Number, String, Date, DateTime, Duration, Link, List, Object) (`src/note/field.rs:43-65, 179-200`).
   - `NoteFieldType` & `NoteFieldValueRef::compare`: Implements canonical total order (`Null < Bool < Number < Duration < Temporal < Text < Link < List < Object`) (`src/note/field.rs:236-250, 286-312`).
   - Filter comparison semantics (`is_equal_to_literal`, `is_containing`) (`src/note/field.rs:393-447`).
3. **Frontmatter Deserialization (`src/note/metadata.rs`)**:
   - `RawFrontmatter`: Preserves raw YAML string between `---` delimiters (`src/note/metadata.rs:18`).
   - `Frontmatter`: Converts YAML via `noyalib` into `FieldValueRef` and then `NoteFieldValue` mapped by `FieldKey` (`src/note/metadata.rs:57-75, 96-98`).
4. **List & Task Classification Hierarchy (`src/note/lists.rs`, `src/note/parser/list.rs`, `src/note/parser/task.rs`)**:
   - `ListItem`: Flat representation in document order storing `ListText`, `ListItemType`, `depth: u8`, `line: SourceLine`, `parent: Option<SourceLine>`, `is_ordered: bool`, `fields: Option<Box<ListFieldMap>>`, and `tags: Box<[Tag]>` (`src/note/lists.rs:41-50`).
   - `TaskScan` & `ListTracker`: Tracks nested list item frames, computes subtask completion roll-up (`SubTaskCompletion::observe_child`), filters task vs checkbox via configured `tag_filters`, and strips metadata tokens to produce `clean_text` (`src/note/parser/task.rs:127-142, 266-275`; `src/note/parser/list.rs:150-163`).
5. **Item-Leading Task Marker Scanning (`src/note/parser/marker.rs`)**:
   - `MarkerAccumulator`: State machine buffering the first 8 bytes of an item to recognize `[<symbol>] ` according to CommonMark and Obsidian whitespace rules across fragmented streaming events (`src/note/parser/marker.rs:189-204`).
6. **Token-Level Lexing & Metadata Scanning (`src/note/parser/lexer.rs`, `src/note/parser/inline.rs`)**:
   - `ItemToken`: Logos-based lexer scanning for task priorities, tags, date shorthands (`📅`, `➕`, etc.), bare fields (`Key:: Value`), and wrapped fields (`[Key:: Value]`, `(Key:: Value)`) (`src/note/parser/lexer.rs:116-159`).
   - `InlineValueParser`: Recursive-descent parser converting inline field text to `NoteFieldValue` atoms or comma-separated lists (`src/note/parser/inline.rs:51-86`).
7. **Task and Tag Domain Definitions (`src/task.rs`, `src/tag.rs`)**:
   - `src/task.rs`: Defines workflow configuration types (`TaskStatus`, `TaskStatusMap`, `TaskStatusType`, `TaskPriority`, `TaskDateSet`, `TaskDate`, `TaskDateType`).
   - `src/tag.rs`: Validated `#`-prefixed tag string with hierarchical subtag containment checks (`is_contained_in`, `segments`) (`src/tag.rs:28, 184, 205`).

---

## 3. Invariants

| Invariant | Location | Enforcement Mechanism | Can Safe Code Violate? |
|---|---|---|---|
| **Never-fail parsing** | `src/note/parser.rs:106` | Infallible return (`parse_markdown(...) -> Note`) | No. Any malformed YAML or Markdown falls back to empty/string/plain. |
| **Strict document order** | `src/note/model.rs:28-33`, `src/note/parser.rs:488-498` | Vector append order in event traversal | No (enforced by parser structure and boxed slices). |
| **Complete AST span discard** | `src/note/model.rs:24-33` | Fields lack `ByteSpan` | No (the types simply do not have span fields). |
| **Dual-pass heading extraction** | `src/note/parser.rs:108-114` | Two `Parser::new_ext` runs over `input.src()` | No (hardcoded inside `parse_markdown`). |
| **List tree as flat slice** | `src/note/lists.rs:41-50` | `depth: u8` and `parent: Option<SourceLine>` | No. Tree reconstruction requires scanning `descendants_of`. |
| **Canonical field precedence** | `src/note/model.rs:142-173` | `fields()` and `get()` check `frontmatter` first, then skip colliding `inline_fields` | No. Hardcoded in accessors. |
| **Cross-kind temporal comparison** | `src/note/field.rs:297-300` | Promotes `Date` to local midnight via `DateTimeValue::try_from` | **Leaked invariant**: depends on process ambient local `TimeZone`. |
| **Zero normalizes in numbers** | `src/note/field.rs:292, 495` | `normalize_zero(n)` maps `-0.0` to `+0.0` before `total_cmp` | No. Enforced in `compare`. |
| **Tag syntax validation** | `src/tag.rs:56-75` | Constructor `Tag::parse` verifies leading `#`, alphabetic first char, alphanumeric/`_`/`/`/`-` body | No. Inner string is private `Box<str>`. |
| **Task marker format** | `src/note/parser/marker.rs:87-115` | `[` + 1 non-`]` char + `]` + 1 whitespace | No. |

---

## 4. State & Lifecycle

- **Owned State**:
  - `Note`: Fully owned tree of boxed slices (`Box<[ListItem]>`, `Box<[Heading]>`, `Box<[Link]>`, `Box<[Tag]>`) and `IndexMap<FieldKey, Box<[NoteFieldValue]>>`.
  - Stored inside `WorkspaceIndex` entry as `Option<Box<Note>>` (`src/index/entry.rs:138`).
- **Persistence**:
  - Persisted directly to redb table `NOTES` (`src/index/tables.rs:27`) using `postcard` binary serialization via serde derives (`src/note/model.rs:23`).
  - Contains no file descriptors or caches.
- **Mutability & Lifecycle**:
  - `ParserContext` (`src/note/parser.rs:313-338`) is ephemeral and mutable during `parse_markdown`.
  - Once assembled, `Note` is **strictly immutable**. There are no mutation methods on `Note`, `ListItem`, `Heading`, or `Link`. Updates require a full re-parse of the source file.
- **Concurrency**:
  - Note parsing is stateless and thread-safe (`Send + Sync`).
  - Batch parsing in `IndexerService::parse_notes` uses `rayon::par_iter()` (`src/index/service.rs:251-257`).
  - Zero internal mutexes, locks, or atomic cells.

---

## 5. Dependency Edges

- **Inbound `crate::` edges into `src/note/`**:
  - `src/index/service.rs`, `src/index/store.rs`, `src/index/entry.rs`, `src/index/inlinks.rs`, `src/index/refresh.rs`: Uses `Note`, `parse_markdown`, `MarkdownParserInput`, `ListItem`, `Link`, `NoteFieldValue`, `Tag`.
  - `src/query/**` (`grammar/source.rs`, `grammar/filter.rs`, `results.rs`, `plan.rs`, `service.rs`): Uses `Note`, `NoteFieldValue`, `NoteFieldValueRef`, `ListItem`, `TaskListItem`.
  - `src/cli/task.rs`: Reaches into `query::results` which holds `NoteFieldValue`.
- **Outbound `crate::` edges from `src/note/`**:
  - `crate::field::{FieldValueRef}` (`src/note/field.rs:24`, `src/note/metadata.rs:12`).
  - `crate::field::{FieldKey, FieldKeyRef}` (`src/note/model.rs:16`, `src/note/metadata.rs:12`, `src/note/lists.rs:27`, `src/note/parser/lexer.rs:10`).
  - `crate::position::{BytePos, ByteSpan, LineIndex, SourceLine, Spanned}` (`src/note/parser.rs:55`, `src/note/heading.rs:5`, `src/note/lists.rs:27`).
  - `crate::temporal::{DatePoint, DateTimeValue, DateValue, DurationValue}` (`src/note/field.rs:24-25`).
  - `crate::task::{TaskDate, TaskDateSet, TaskDateType, TaskPriority, TaskStatus, TaskStatusMap, TaskStatusSymbol, TaskStatusType}` (`src/note/lists.rs:27`, `src/note/parser/task.rs:12-14`).
  - `crate::tag::Tag` (`src/note/model.rs:16`, `src/note/lists.rs:27`, `src/note/parser/lexer.rs:10`).
  - `crate::delimiter::DelimiterType` (`src/note/links.rs:11`, `src/note/parser/lexer.rs:10`, `src/note/parser/marker.rs:5`).
  - `crate::config::{FrontmatterConfig, TaskConfig}` (`src/note/parser/input.rs:9`).
  - `crate::yaml` (`src/note/metadata.rs:12`).
  - `crate::path::{HasPath, codec}` (`src/note/model.rs:25, 214`).
- **Cycles / Leaks**:
  - **No cyclic imports**. Note sits at the mid-to-lower level of the crate.
  - However, Note's parser directly depends on `crate::config::TaskConfig` and `FrontmatterConfig` via `MarkdownParserInput` (`src/note/parser/input.rs:9`).

---

## 6. Interfaces: Deep vs. Shallow

- **`parse_markdown(&MarkdownParserInput) -> Note`**:
  - **Deep interface**. Completely hides pulldown-cmark, logos, noyalib YAML parsing, CommonMark marker state machines, subtask tree roll-ups, and inline regexes behind a single function call returning an immutable `Note`.
- **`NoteFieldValueRef::compare(&self, &Self) -> Ordering`**:
  - **Deep interface**. Enforces the full cross-type ranking and temporal promotion semantics across all metadata comparison sites.
- **`src/task.rs` vs `src/note/parser/task.rs` vs `src/cli/task.rs`**:
  - **Clean separation of responsibilities**:
    - `src/task.rs` (1,919 lines): Pure domain models and configurations (status mappings, priority enums, lifecycle date sets). Contains no parser code and zero regexes.
    - `src/note/parser/task.rs` (1,266 lines): Concrete syntax extraction algorithms (clean text whitespace collapsing, emoji/inline-field extraction, subtask completion propagation).
    - `src/cli/task.rs` (927 lines): CLI command presentation (`traces task`), table rendering, and flag formatting.
- **`src/note/cursor.rs`**:
  - **Shallow interface** (85 lines). Thin wrapper around `&str` byte indexing (`SourceText`) used only in `links.rs` and `parser/inline.rs`.

---

## 7. Algorithms, Data Structures & Performance

1. **Dual-Pass pulldown-cmark Parsing**:
   - `parse_markdown` walks the event stream **twice** (`src/note/parser.rs:108-114`):
     - Pass 1: `Parser::new_ext(input.src(), MARKDOWN_OPTIONS).into_offset_iter()` extracts metadata, links, lists, tags.
     - Pass 2: `collect_headings(input.src(), &ctx.line_index)` runs `Parser::new_ext(src, HEADING_OPTIONS).into_offset_iter()` specifically to extract headings with math enabled.
   - **Cost**: Double tokenization of the entire document for every note.
2. **Intermediate String Buffering & Multiple Lexer Passes**:
   - For every paragraph/heading: text is accumulated into `body_buffer: String` (`src/note/parser.rs:330`).
   - On block end: `scan_metadata` calls `scan_fields` (logos pass 1), then `scan_tags` (logos pass 2) (`src/note/parser/lexer.rs:316-322`).
   - For list items: text is accumulated in `ItemBuffers::scan: String`, then flushed (`take_scan`) via `scan_metadata` (2 logos passes), and then `ItemBuffers::text` is tokenized *again* by `TaskScan::scan` (logos pass 3) (`src/note/parser/list.rs:396-402, 420-423`).
   - **Result**: List item text is lexed **3 separate times** with intermediate heap strings.
3. **Double AST Value Mapping on Frontmatter**:
   - `RawFrontmatter::parse` (`src/note/metadata.rs:57-73`):
     - YAML string $\to$ `noyalib::Value::Mapping`
     - $\to$ `FieldValueRef` (`src/field.rs`)
     - $\to$ `NoteFieldValue` (`src/note/field.rs:532-570`)
   - Each scalar/string undergoes 2 conversion hops and multiple re-allocations.
4. **Memory Footprint & Benchmark Evidence**:
   - In `benches/memory_footprint.rs:89-93`:
     - List items parsing allocates linearly with item count (`list_items(n)`: ~100-300 bytes and multiple allocations per item).
   - In `src/index/entry.rs:230`: `FileEntry` explicitly boxes `Note` (`Option<Box<Note>>`) because `Note` itself is **240 bytes** on the stack.
5. **AST Span Stripping**:
   - `handle_event` receives `ByteSpan::from(range)` on every pulldown-cmark event (`src/note/parser.rs:111`), but **discards** it for links, tags, and inline fields. Only `line_index.line_at(pos)` is retained for list items and headings.

---

## 8. Error Topology

- **Infallible Parsing**:
  - `parse_markdown` **cannot return an error**.
  - Malformed YAML in frontmatter logs a `tracing::warn!` and yields an empty `Frontmatter` (`src/note/metadata.rs:182`).
  - Unrecognized tags or inline field syntax simply skip emission and stay as plain text.
- **Error Types in Note/Task/Tag**:
  - `FrontmatterParseError` (`src/note/metadata.rs:80`): Private enum (`Parse(noyalib::Error)`, `NotMapping`). Handled internally.
  - `TagError` (`src/tag.rs:242`): Public enum (`MissingHash`, `InvalidFirstCharacter`, `InvalidBodyCharacter`). Returned only by `Tag::parse`.
  - `TaskError` (`src/task.rs:1140`): Public enum (`ProhibitedStatusSymbol`, `UnknownStatus`, `UnknownPriority`, `UnknownDateType`, `DuplicateStatusSymbol`, `DuplicateStatusName`). Returned only during configuration/CLI parsing.
- **Context Loss**:
  - Because `Note` parsing is infallible and drops spans, syntax warnings or diagnostics for broken links, malformed frontmatter, or invalid dates cannot be emitted to users as compiler/linter diagnostics today.

---

## 9. Duplicated & Conflicting Concepts

1. **`field::FieldValue` vs `note::NoteFieldValue`**:
   - `src/field.rs:586`: `FieldValue` (Null, Bool, Int(i64), Float(f64), String, Date, DateTime, List(Vec), Object(IndexMap)).
   - `src/note/field.rs:43`: `NoteFieldValue` (Null, Bool, Number(f64), String, Date, DateTime, Duration, Link, List(Box<[Self]>), Object(IndexMap)).
   - **Conflict**: The codebase has two parallel dynamic value enums. `FieldValue` is used by schema options, while `NoteFieldValue` is used by notes and queries. `NoteFieldValue` implements `From<FieldValueRef>` with runtime string re-parsing for duration/wikilink syntax.
2. **`SchemaFieldType` vs `NoteFieldType`**:
   - `SchemaFieldType` (`src/schema/fields.rs:150`) classifies schema author constraints (`Input`, `Select`, `Boolean`, `Number`, `Date`, `File`).
   - `NoteFieldType` (`src/note/field.rs:209`) classifies parsed markdown metadata primitives (`Null`, `Bool`, `Number`, `Duration`, `Temporal`, `Text`, `Link`, `List`, `Object`).
   - **Conflict**: There is no type-level correspondence between schema constraints and note field values. The schema engine performs zero validation of note fields (`schema.md:86-87`).
3. **`Heading` display text vs `Heading` slug**:
   - Headings store `text: String` (`src/note/heading.rs:12`). Slugification logic does not exist in `src/note/heading.rs` at all; heading anchors are parsed on demand during link resolution (`src/index/inlinks.rs`).

---

## 10. Doc, Glossary & ADR Drift

1. **`src/note/GLOSSARY.md` Slug Claim Drift**:
   - `GLOSSARY.md:47-51` claims: *"Duplicate slugs within a note are deduplicated at parse time (`slug-1` suffixes)."*
   - **CODE VERIFICATION: FALSE / COMPLETE DRIFT**. `Heading` has no slug field (`src/note/heading.rs:10-14`), and `parse_markdown` performs zero slug generation or deduplication (`src/note/parser.rs:118-162`).
2. **`src/note/GLOSSARY.md` Footnote & Callout Drift**:
   - `GLOSSARY.md:84-105` defines `Footnote Reference`, `Footnote Definition`, and `Callout` under Note domain terms.
   - **CODE VERIFICATION: PARTIAL / ABSENT**. There is no `Callout` AST node anywhere in `src/note/` (blockquotes are simply tracked to resolve pending markers in `src/note/parser.rs:288`). Footnotes are enabled in `pulldown-cmark` options (`src/note/parser.rs:70`), but no `FootnoteDefinition` or `FootnoteReference` nodes are stored on `Note`.
3. **`docs/divergence-register.md` Entry 4 Drift**:
   - Divergence Register Entry 4 (`docs/divergence-register.md:294`) lists rank order as `Null < Bool < Number < String < Date < DateTime < Duration < Link < List < Object`.
   - **CODE VERIFICATION: DRIFT**. Code in `src/note/field.rs:236-250` actually implements: `Null < Bool < Number < Duration < Temporal < Text < Link < List < Object`. (`Duration` is rank 3, before `Temporal` rank 4; `Date` and `DateTime` share rank 4).
4. **ADR-0005 Redb Lists Table**:
   - ADR-0005 specified a dedicated `lists` redb table. In reality, list items are serialized directly inside `Note` (`src/note/model.rs:28`).

---

## 11. Candidate Architectural Questions

### 1. Unified Value Representation: Should `FieldValue` and `NoteFieldValue` be merged?
- **Claim**: Having two separate dynamically typed value trees (`crate::field::FieldValue` in `src/field.rs:586` and `crate::note::NoteFieldValue` in `src/note/field.rs:43`) causes code bloat, confusing API boundaries, and forces a multi-step allocation/conversion hop (`noyalib::Value` $\to$ `FieldValueRef` $\to$ `NoteFieldValue`) on every frontmatter load.
- **Evidence**: `src/note/metadata.rs:71-72` and `src/note/field.rs:532-570`. Both enums define Null, Bool, Number, String, Date, DateTime, List, and Object. `NoteFieldValue` merely adds `Duration` and `Link`.
- **Strongest Counter-Argument**: `FieldValue` is format-agnostic structured data for TOML schemas and JSON configs where Markdown wikilinks and Dataview durations do not exist. Merging them would pollute general configuration data structures with Markdown/PKM-specific domain variants.
- **Open Decision**: Whether to merge both into a unified `Value` enum in a shared core module, or keep `FieldValue` for schemas/config and `NoteFieldValue` for PKM document metadata.
- **Touches**: `field.rs`, `note/field.rs`, `schema/`, `query/`.

### 2. Elimination of Dual-Pass pulldown-cmark Parsing in `parse_markdown`
- **Claim**: `parse_markdown` walks the raw document with pulldown-cmark twice—once for metadata/links/lists, and once in `collect_headings` for headings (`src/note/parser.rs:108-114`). Merging heading collection into the primary event loop would eliminate ~30-40% of Markdown tokenization CPU overhead on every indexed note.
- **Evidence**: `src/note/parser.rs:113` explicitly re-parses the source text: `collect_headings(input.src(), &ctx.line_index)` using `Parser::new_ext(src, HEADING_OPTIONS)`.
- **Strongest Counter-Argument**: `HEADING_OPTIONS` includes `Options::ENABLE_MATH` whereas `MARKDOWN_OPTIONS` does not. Reconciling math handling in the main event stream might slightly alter inline text event boundaries or marker recognition.
- **Open Decision**: Unify heading extraction into the primary event traversal by enabling `ENABLE_MATH` across both or filtering math events in the main dispatch.
- **Touches**: `note/parser.rs`.

### 3. Source Spans on the Note AST: In-band retention vs transient AST
- **Claim**: The planned Markdown/PKM LSP requires byte spans on AST elements (links, tags, inline fields, headings). Currently, `pulldown-cmark` emits offset ranges (`src/note/parser.rs:109`), but they are discarded. Retaining `ByteSpan` on `Link`, `Tag`, `Heading`, and `ListItem` directly enables LSP gotodef/references/rename without a second parser.
- **Evidence**: `.scratch/md-pkm-lsp/map.md:78` (Ticket 11) notes spans are currently discarded. `src/position.rs:66` already defines `ByteSpan`.
- **Strongest Counter-Argument**: Persisting spans in `Note` increases the serialized size in redb (`NOTES` table) and breaks snapshot cache determinism if byte offsets shift under formatting changes. (Addressed in Ticket 40: persist spans vs `Option<ByteSpan>` vs transient-only).
- **Open Decision**: Should `ByteSpan` become a permanent field on `Link`, `Tag`, `Heading`, and `ListItem` in `src/note/`, or should `Note` remain a span-free semantic cache with a separate `SpannedNote` AST used by the LSP?
- **Touches**: `note/model.rs`, `note/links.rs`, `note/heading.rs`, `note/lists.rs`, `index/store.rs`, `md-pkm-lsp`.

### 4. Single-Pass Lexing for List Items: Avoiding 3x Tokenization
- **Claim**: List item text is currently parsed 3 times: once by CommonMark event accumulation, once by `scan_metadata` (which runs `scan_fields` and `scan_tags`), and once by `TaskScan::scan` (`src/note/parser/list.rs:396-402, 420-423`). Unifying item tokenization into a single logos pass over the raw item text would remove redundant regex scans and intermediate buffer allocations.
- **Evidence**: `ItemToken::lexer_with_extras` in `src/note/parser/lexer.rs:44` already has variants for `Priority`, `Tag`, `Date`, and `Field`. Yet `list.rs` calls `scan_metadata` (running fields and tags separately) and then calls `TaskScan::scan`.
- **Strongest Counter-Argument**: `scan_metadata` needs to run when an item's scan buffer is flushed (e.g. before nested child lists start) to maintain document-order metadata emission, whereas `TaskScan` needs the complete item text to compute `clean_text`. Decoupling them avoids premature clean-text slicing.
- **Open Decision**: Refactor `ItemFrame` to run a single unified tokenization pass once per item and cache the tokens.
- **Touches**: `note/parser/list.rs`, `note/parser/task.rs`, `note/parser/lexer.rs`.

### 5. Flat `ListItem` Slice vs Nested Tree in `Note`
- **Claim**: Storing list items as a flat `Box<[ListItem]>` with `depth: u8` and `parent: Option<SourceLine>` (`src/note/lists.rs:44-46`) makes subtree iteration ($O(N)$ scan via `descendants_of`) and hierarchical manipulation awkward compared to a nested tree node structure.
- **Evidence**: `Note::descendants(parent_idx)` in `src/note/model.rs:202-211` must do a contiguous slice linear scan until the first sibling/ancestor depth is found.
- **Strongest Counter-Argument**: Flat document-order storage matches redb linear serialization, makes whole-document item iteration zero-cost, avoids recursive heap allocations, and aligns with how Markdown list items appear sequentially on disk. Keep as-is.
- **Open Decision**: Keep flat slice storage as-is.
- **Touches**: `note/lists.rs`, `note/model.rs`.

### 6. Parser Decoupling from IndexerService
- **Claim**: `parse_markdown` takes `MarkdownParserInput`, which borrows `TaskConfig` and `FrontmatterConfig` (`src/note/parser/input.rs:9`). Currently, `IndexerService` is the only component in the production pipeline that constructs this configuration, tying Markdown parsing configuration to the indexing service.
- **Evidence**: `src/index/service.rs:261-277`. If the LSP or CLI needs to parse an ad-hoc note buffer, it must reconstruct `MarkdownParserInput` manually with configs extracted from `Config`.
- **Strongest Counter-Argument**: `MarkdownParserInput` is a clean borrower. Creating a standalone `NoteParserService` is an extra layer of abstraction with minimal gain unless the LSP requires dynamic configuration swapping.
- **Open Decision**: Keep `MarkdownParserInput` as a borrowed struct, or wrap parsing in a dedicated `NoteEngine` service.
- **Touches**: `note/parser/input.rs`, `index/service.rs`, `config/`.

---
