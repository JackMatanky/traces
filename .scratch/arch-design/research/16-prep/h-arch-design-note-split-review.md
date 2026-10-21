# Arch-Design Review: Note Representation Split, Source Spans, and Schema Validation

A rigorous, full-text architectural review of the arch-design effort for ticket 16 preparation. Evaluates the note document model, source span retention, and schema validation against decisions, research evidence, and live codebase state.

- Date: 2026-10-20
- Target Deliverable: `.scratch/arch-design/research/16-prep/h-arch-design-note-split-review.md`
- Status: Research input for ticket 16 grilling. Planning only; no code modified.

---

## 1. Decisions, Leans, and Open Questions in Arch-Design

### 1.1 Decision Inventory (DECIDED / LEANING / OPEN)

| Topic | Status | Source Citation | Exact Quote / Evidence | Summary |
|---|---|---|---|---|
| **LSP as Committed Consumer** | **DECIDED** | `issues/13:37-43` [V] | "The redesign must make the shared core ready for the LSP as the next major consumer and for MCP as a likely near-term consumer... Editor-facing behavior must support source-aware features such as diagnostics, navigation, and completion." | Core must support source-aware editor features. |
| **Zero-Span Design Rejected** | **DECIDED** | `issues/13:67-68` [V] | "- **Keep the CLI-only, zero-span design:** rejected because the LSP is the next major consumer and needs source-aware editor capabilities." | Status-quo zero-span note model cannot stand. |
| **Shared vs Split Note Models** | **OPEN** | `issues/13:45-49, 71-73` [V] | "The prior LSP decision to add spans to the shared `Note` AST and reject a parallel model is reopened: the design must compare a shared model against separate editor and indexing representations. The exact model and ownership of source spans belong to [19]... One shared span-aware note model vs separate editor and index models: neither is selected here." | Representation split is formally open for ticket 19. |
| **`Link` vs `NoteLink` Split** | **DECIDED** | `issues/14:87-91, 484-490` [V] | "Agreed direction — Link split: use distinct `Link` and `NoteLink` types with fields belonging to each type... Root `Link`: `Link { target, display: Option<String>, embedded: bool }`... `NoteLink` (syntax kind and source span) is the editor-side occurrence type and is defined by ticket 19." | Semantic `Link` is root; `NoteLink` (with span) is editor-side. |
| **Frontmatter Direct AST Parse** | **DECIDED** | `issues/14:124-128, 492-498` [V] | "Agreed: keep Noyalib's compatibility parser as the frontmatter boundary, then convert its owned AST directly into the shared semantic model, removing `FieldValueRef` from that conversion. Do not add a custom `Deserialize` target... Frontmatter conversion: keep the Noyalib compat parser and convert its `Value` directly into `NoteFieldValue`." | Direct AST conversion; no custom Serde visitor on YAML path. |
| **Noyalib CST for Values** | **DECIDED (REJECTED)** | `issues/14:201-224, 467-470` [V] | "Mechanism reviewed and rejected (Round 1): Noyalib 0.0.57 exposes `cst::parse_document_with_config` and `Document::span_at(path)`... CST path builds a green tree, a typed `Value` and a `SpanTree` per note (+59% parse time)... Path-based CST span lookup is unsound for some keys... Direct Serde visitor or CST for quote style on the YAML path: rejected." | CST rejected for parse-time classification and values. |
| **Infallible Note Parser** | **DECIDED (FROZEN)** | `issues/12:30; 19:16` [V] | `issues/12:30`: "Vault-format parsing (frontmatter, `Key:: Value` inline fields, wikilinks, tasks, tags): **Frozen** (Same files parse to the same meaning)." `issues/19:16`: "infallibility contract retained." `src/note/parser.rs:106` [V]: `parse_markdown` returns `Note` directly. | Markdown parser is strictly infallible; never returns `Result`. |
| **Lossless CST for Vault Index** | **LEANING (REJECTED)** | `issues/19:24-25; research/10:53, 98-99` [V] | `issues/19:24-25`: "lossless tree (expected to be rejected — attempt to refute that)." `research/10:98-99`: "Traces is a query engine and template processor over Markdown notes, not a refactoring IDE. Lossless CSTs... carry heavy pointer overhead and cannot be efficiently persisted in redb." | Full lossless CST (Rowan) rejected for core vault index. |
| **Transient vs Persisted Spans** | **OPEN** | `issues/13:83-85; 19:9-10` [V] | "Source positions must be available to editor features, but whether they belong in one shared model or a separate transient representation remains open for ticket 19." | Persisted index spans vs ephemeral editor spans is open. |
| **Multi-Valued Inline Fields** | **OPEN / LEANING** | `issues/12:44; 15:204-205; 19:12-13` [V] | `issues/12:44`: "Known defects: ... multi-valued inline fields read via `.first()` only: **Not preserved** (Fixing each is a change-table row)." `research/10:119-120`: collapse to `List` or expose full slice. | Leaning toward Dataview-style collapse or slice exposure. |
| **Frontmatter Schema Override** | **OPEN** | `issues/15:42-43, 104-105; 16:41-45` [V] | "Frontmatter classification stays quote-insensitive... A frontmatter value that must stay text uses the schema override (ticket 16)... frontmatter classification stays schema-free and quote-insensitive; the schema override that keeps a date-shaped value as text is decided here." | Schema override keeping date as text to be decided in 16. |
| **Schema Select String Adoption** | **DECIDED** | `issues/14:481-482; 15:122-124` [V] | "Select values that name links stay strings; ticket 16 interprets them at the consumer boundary with the shared wikilink parser... A `Link` row adopts a literal as its folded target... This settles schema select string vs `Link` matching for ticket 16." | Select strings match note `Link`s via adaptive target adoption. |

### 1.2 Analysis of Decisions and Leans

1. **Splitting Note Types [OPEN, leaning split]**:
   Ticket 13 explicitly reopened the prior LSP decision (`.scratch/md-pkm-lsp/issues/20`) that mandated a single shared `Note` AST (`issues/13:45-47` [V]). Ticket 14 further segregated semantic values from editor occurrences by splitting `Link` (root semantic data: target, display, embedded) and `NoteLink` (editor occurrence carrying syntax kind and span), assigning `NoteLink` strictly to the editor-side model (`issues/14:87-91, 484-490` [V]).
2. **Span Retention Policy [OPEN]**:
   Ticket 13 ruled that editor features require source positions (`issues/13:43, 67-68` [V]), but whether spans belong in the shared `Note` or a separate transient representation was explicitly deferred to ticket 19 (`issues/13:83-85` [V]).
3. **Noyalib CST & Frontmatter Spans [DECIDED (REJECTED)]**:
   Ticket 14 investigated Noyalib 0.0.53/0.0.57 CST (`cst::parse_document_with_config` and `Document::span_at`) and rejected it for runtime value classification (`issues/14:201-224, 467-470` [V]). CST parsing adds +59% CPU overhead per note (2.6x on notes with dates) and path-based span lookup is unsound for dotted keys, tags (`!!str`), block scalars (`|`), and duplicate keys (`issues/14:283-288` [V]). Frontmatter parsing remains direct AST conversion (`issues/14:492-498` [V]).
4. **Lossless Tree Alternative [LEANING (REJECTED)]**:
   Research 10 concluded that Roslyn-style green/red trees (`rowan`) carry heavy pointer overhead and cannot be efficiently persisted in redb (`research/10:53, 98-99` [V]). Ticket 19 records this as expected to be rejected (`issues/19:24-25` [V]).
5. **Infallible Parser [DECIDED (FROZEN)]**:
   Vault parsing compatibility is frozen (`issues/12:30` [V]). `parse_markdown` never returns `Result` or panics (`src/note/parser.rs:106` [V]; `issues/19:16` [V]).
6. **Multi-Valued Fields [OPEN, leaning collapse/slice]**:
   `Note.inline_fields` stores `IndexMap<FieldKey, Box<[NoteFieldValue]>>` (`src/note/model.rs:31` [V]), but query resolution only reads `first()` (`src/query/results.rs:328` [V]). Ticket 12 marks this defect unpreserved (`issues/12:44` [V]); ticket 19 settles element-wise classification and representation (`issues/19:12-13, 43` [V]).

### 1.3 Hidden Coupling: Ticket 16 vs Ticket 19

- **Ticket 19's Mandate** (`issues/19:9-16` [V]): Decides the internal shape of parsed `Note`: byte-span policy; whether list/task items remain children of `Note` or separate entities; duplication of item fields/tags into note collections; multi-valued inline field semantics; per-task status representation (interned symbol vs owned string); single vs dual markdown pass.
- **The Bidirectional Coupling**:
  - *Coupling 16 -> 19*: Ticket 16 asks whether schemas drive validation and what module owns the validator (`issues/16:11-14` [V]). If ticket 16 specifies a validator `validate(&Note, &Schema) -> Vec<SchemaDiagnostic>` where diagnostics require `ByteSpan` (essential for LSP `Diagnostic.range`, `research/10:59` [V]), it **forces ticket 19** to attach byte spans to the shared `Note` struct. This forecloses ticket 19 from choosing a span-free index model (`NoteFacts`).
  - *Coupling 19 -> 16*: If ticket 19 runs first and strips spans from `Note` to optimize the index cache, ticket 16 is left unable to generate byte-located diagnostics without re-parsing the file or inventing an uncoordinated sidecar.
- **Resolution Order**:
  - Ticket 16 does NOT need to wait on Ticket 19 if Ticket 16 defines the validation interface against an **abstract source-aware target** rather than the concrete index `Note` struct.
  - Specifically, Ticket 16 decides: (1) validation is advisory, on-demand, and non-gating; (2) validator signature takes an editor-facing document / buffer target with field spans; (3) schema text-override semantics.
  - Ticket 19 then resolves whether that editor-facing target is a unified `Note` or a separate `NoteDocument` paired with a span-free `NoteFacts`.

---

## 2. Evidence on Span Costs from Research 10 & 11

### 2.1 Span Coordinate Primitives & LineIndex

- **[V] `ByteSpan` is 8 bytes**: `struct ByteSpan { start: u32, end: u32 }` (`src/position.rs:18-35` [V]; `research/10:52` [V]). It is isomorphic to `ruff_text_size::TextRange`. It replaces 48-byte coordinate structs like `markdown-rs` `unist::Position` (`Point { line, col, offset }` = 24B; `start + end` = 48B; `Option<Position>` = 56B; `research/10:43-45` [V]). `ByteSpan` is **84% smaller** (`research/10:78` [V]). For a 50,000-node vault, storing `Position` wastes 2.4 MB on redundant line/column numbers (`research/10:101` [V]).
- **[V] `LineIndex` is 4 bytes per line**: Stores `line_starts: Vec<TextSize>` plus an ASCII/UTF-8 flag (`research/10:57` [V]). Line lookups take $O(\log n)$ binary search. LSP UTF-16 character offsets are computed on demand in $O(\text{line\_len})$ (`research/10:58, 79` [V]). Batch indexing pays **zero UTF-16 conversion overhead**.
- **[V] LineIndex must not be persisted**: `LineIndex` is an ephemeral document utility used during indexing/LSP; storing it in redb would waste space (`research/10:89` [V]).

### 2.2 Parser Execution Overhead

- **[V] pulldown-cmark spans are free**: `Parser::into_offset_iter()` is already called in production (`src/note/parser.rs:109` [V]). In `ParserInner::next_event_range` (`src/parse.rs:2410-2428` [V]; `research/10:48` [V]), pulldown-cmark *already* computes `start..end` for all events. `Parser::next()` simply discards the range (`src/parse.rs:2403` [V]). Keeping `ByteSpan` adds **0.0% parser CPU overhead** (`research/10:48, 87` [V]).
- **[V] Heading pass wastes 20% to 38% CPU**: `collect_headings` runs a redundant second pulldown-cmark pass solely to strip math syntax (`src/note/parser.rs:118-162` [V]). It accounts for **20.2%** (1 KB note), **26.0%** (5 KB note), and **38.0%** (50-heading note) of total note parse time (`research/11:160-165, 195` [V]). Fusing it saves 20–35% parser CPU.
- **[V] Noyalib CST cost**: Parsing YAML frontmatter with Noyalib CST adds **+59% parse time** per note, and 2.6x on notes with dates (`issues/15:64-65` [V]; `research/11:227-230` [V]).

### 2.3 Memory Footprint & Deserialization Bottleneck

- **[V] `ListItem` footprint**: Struct size is **160 bytes** (`src/note/lists.rs:41-50` [V]; `research/11:170` [V]). Currently stores `line: SourceLine` (4B) and `parent: Option<SourceLine>` (8B), but discards byte offsets. Each task item embeds `TaskStatus` with owned `name: String` (24B) (`src/task.rs:53` [V]), causing 10,000 heap string allocations ($\ge 234$ KB) in a 10k-task vault (`research/11:181-185` [V]). Interning status symbols shrinks `ListItem` from 160 to ~128 bytes (`research/11:200` [V]).
- **[V] The 97.3% note deserialization cliff**: On path-set changes at 5,000 notes, total refresh takes 35.17 ms (`research/11:85` [V]). Note deserialization from redb (`store.read_all_notes()`) takes **18.48 ms (97.3% of in-memory work)**, while link re-resolution (`InlinkMap::new`) takes only **0.52 ms (2.7%)** (`research/11:91-95, 106-107` [V]).
- **[H] Impact of spans on index cache**: Serializing extra span fields into redb's `NOTES` table via `postcard` (`src/index/store.rs:42, 1051` [V]) directly expands record size. In an 18.5 ms deserialization bottleneck, adding 8 bytes per heading, list item, link, and frontmatter value directly regresses vault refresh and query startup.

---

## 3. Concrete Candidate Shapes for Note Representation Split

### 3.1 Code Inventory: Existing Note Types and Redb Persistence

- **Existing Note Model (`src/note/model.rs:23-33` [V])**:

  ```rust
  pub struct Note {
      path: PathBuf,
      frontmatter: Option<Frontmatter>,                          // IndexMap<FieldKey, NoteFieldValue> (no spans)
      lists: Box<[ListItem]>,                                    // ListItem has line: SourceLine, parent: Option<SourceLine>
      headings: Box<[Heading]>,                                  // Heading has line: SourceLine (no byte span)
      outlinks: Box<[Link]>,                                     // Link has target, text, kind, embedded (no span)
      inline_fields: IndexMap<FieldKey, Box<[NoteFieldValue]>>,  // No spans
      tags: Box<[Tag]>,                                          // No spans
  }
  ```

- **Existing Line Fields in Code**:
  - `Heading.line: SourceLine` (`src/note/heading.rs:13` [V]) = `NonZeroU32` (4 bytes).
  - `ListItem.line: SourceLine` and `ListItem.parent: Option<SourceLine>` (`src/note/lists.rs:45-46` [V]) = 4 bytes and 8 bytes.
- **Redb Persistence (`src/index/tables.rs:20-80` [V]; `src/index/codec.rs:27-54` [V])**:
  - `FILES`: path `&[u8]` -> serialized `FileMeta` (postcard).
  - `NOTES`: path `&[u8]` -> serialized `Note` (postcard). **Whole `Note` is serialized!**
  - `LINKS`: multimap target path `&[u8]` -> source path `&[u8]`.
  - Secondary multimap tables: `PATHS_BY_TAG`, `PATHS_BY_FILE_CLASS`, `TAGS_BY_PATH`, `FILE_CLASSES_BY_PATH`.
  - `META`: `version` (b"1"), `fingerprint` (raw blake3 bytes).

### 3.2 Concrete Candidate Shapes Compared

| Dimension | Shape A: Single Shared `Note` with Spans | Shape B: Dual Model (`NoteFacts` Index + `NoteDocument` Editor) | Shape C: Lossless CST (Rowan) + Facts | Shape D: Span-Free `Note` + Buffer Reparse |
|---|---|---|---|---|
| **Index Persistence (`redb NOTES`)** | Serializes `Note` with `ByteSpan` on all elements | Serializes `NoteFacts` (span-free, stripped) | Serializes `NoteFacts` | Serializes `Note` (status quo line fields) |
| **Editor / LSP Model** | Same `Note` struct | `NoteDocument` (spans, `NoteLink`, field spans) | Full Rowan CST (`SyntaxNode`) | Ephemeral reparse on open buffer |
| **Validator Consumes** | `&Note` | `&NoteDocument` | `&SyntaxNode` / CST DOM | `(&str, &Schema)` or `(&Note, &str, &Schema)` |
| **Diagnostic Spans Source** | Directly from `Note` fields | Directly from `NoteDocument` spans | From CST `text_range()` | From raw buffer validation scan |
| **Redb Deserialization Impact** | **Regresses**: +15–25% record size; worsens 18.5 ms cliff | **Improves**: -10–20% record size; speeds up 18.5 ms cliff | Improves 18.5 ms cliff | Neutral (unchanged from today) |
| **Parser Invocations** | 1 pass (fused) | 1 parser emits `NoteDocument`, projects to `NoteFacts` | 2 separate parsers | 1 pass index; 1 pass per active editor keystroke |
| **What Ticket 16 Decides** | Validator takes `&Note` | Validator takes `&NoteDocument` | Rejected | Validator takes `(&str, &Schema)` |
| **What Ticket 16 Defers** | Defers AST span fields to 19 | Defers `NoteFacts` layout to 19 & 20 | N/A | Defers all AST details to 19 |

### 3.3 Evaluation of Shapes

- **Shape A (Single Shared `Note` with Spans)**:
  - *Mechanism*: Replace `SourceLine` on `Heading` (`src/note/heading.rs:13` [V]) and `ListItem` (`src/note/lists.rs:45` [V]) with `ByteSpan` (8B). Add `span: ByteSpan` to `Link` and frontmatter fields.
  - *Failure Mode*: Pollutes redb index cache with editor byte spans that CLI queries never use. Direct violation of Ousterhout's information hiding: index persistence pays for editor concerns.
- **Shape B (Dual Representation: `NoteFacts` Index + `NoteDocument` Editor) [RECOMMENDED]**:
  - *Mechanism*: Single parser engine can emit `NoteDocument` (spans on all syntax occurrences, `NoteLink`, field spans). For batch indexing, `NoteDocument` projects into compact `NoteFacts` (no spans, interned statuses, no display/embed trivia).
  - *Validator*: `SchemaValidator::validate(&NoteDocument, &Schema) -> Vec<SchemaDiagnostic>`.
  - *Failure Mode*: Two structs to maintain; conversion code required.
- **Shape C (Lossless CST)**:
  - *Verdict*: **REJECTED**. Rowan CSTs cannot be efficiently serialized in redb (`research/10:53, 98-99` [V]); Noyalib CST is unsound for YAML scalars (`issues/14:283-288` [V]).
- **Shape D (Shared Model + On-Demand Reparse)**:
  - *Mechanism*: Keep `Note` span-free. Schema validation runs as a text-linter on raw frontmatter string: `validate_frontmatter_str(yaml_str: &str, schema: &Schema)`.
  - *Failure Mode*: Disconnects validation from parsed note semantics; fails if inline fields also require schema validation.

---

## 4. Real Subsystem Topology and Dependency Edges

Verification of `survey/cross-cutting.md:35, 96-124` [V] against live source code:

```text
Tier 0-2:  [path, position, hash, tag, temporal, dirtree, strsim, yaml, delimiter, dirs, env_vars]
               ▲                   ▲                  ▲
               │                   │                  │
Tier 3:   [field] ──────────► [file] ──────────► [task]
               ▲                   ▲                  ▲
               │                   │                  │
Tier 4:   [config]                │              [schema] ──► (imports field, file, dirtree, strsim, path)
               ▲                   │                  │
               │                   │                  │   ZERO EDGES between schema and note!
Tier 5:    [note] ─────────────────┴──────────────────┘   (note imports field, config, task, temporal, ...)
               ▲
               │
Tier 6:   [index] ──► (imports note, file, config, path, tag)
               ▲
               │
Tier 7:   [query] ──► (imports note, index, field, file, ...)
               ▲                                      ▲
               │                                      │
Tier 8:   [file_class_expander] ──────────────────────┼──► (bridges query and schema)
          [template] ─────────────────────────────────┴──► (imports note, schema, query, ...)
               ▲
               │
Tier 9:     [cli] ──► (composition root: imports all services)
```

### 4.1 Verified Dependency Edge Facts

1. **`src/schema/` -> `src/note/` [ZERO EDGES]**:
   - `src/schema/` has **0 imports of `crate::note`** (`src/schema/*` [V]).
   - `src/schema/` has **0 uses of `NoteFieldValue`** (`src/schema/*` [V]).
   - `src/schema/` imports only root primitives: `crate::{FieldKey, FieldName, FieldValue}` (`src/field.rs:45, 596` [V]), `crate::BaseNameRef` (`src/file.rs:18` [V]), `crate::{DirTree, DirTreeError}`, `crate::strsim::closest_match`, `crate::path::{PathError, SafeRelativePath}` (`src/schema/builder.rs:30`, `src/schema/service.rs:19`, `src/schema/fields/select.rs:22` [V]).
2. **`src/note/` -> `src/schema/` [ZERO EDGES]**:
   - `src/note/` has **0 imports of `crate::schema`** (`src/note/*` [V]).
   - `src/note/` imports `crate::field::FieldValueRef` (`src/note/metadata.rs:12, 71` [V]), `crate::config::{FrontmatterConfig, TaskConfig}` (`src/note/parser/input.rs:9` [V]), and temporal/position primitives.
3. **`src/field.rs`**:
   - Imports `Blake3Hasher` and temporal types (`src/field.rs:39` [V]). Zero imports of `note` or `schema`.
4. **`src/file_class_expander.rs`**:
   - Explicit bridge module at Tier 8 (`src/file_class_expander.rs:1-13` [V]): implements `query::FileClassExpander` for `schema::SchemaService`. Neither `query` nor `schema` imports the other.

### 4.2 Architectural Consequence for Ticket 16

- `schema` sits at **Tier 4**; `note` sits at **Tier 5**.
- `src/schema/` CANNOT validate `Note` directly without creating a Tier 4 -> Tier 5 cyclic inversion.
- Therefore, schema validation logic MUST live either:
  - In a new bridge module/service at **Tier 6+** (e.g. `src/schema_validator/` or `src/validation/`), mirroring `file_class_expander`.
  - Or inside the consumer at **Tier 8/9** (LSP diagnostics worker or CLI `check` runner).

---

## 5. Downstream Dependencies Across Tickets 20–30

| Ticket | Citation (path:line) | Direct Dependency on Ticket 16 Choice? | Presupposes Validation Exists? |
|---|---|---|---|
| **20: Index Read Model** | `issues/20:14-15` [V]: "what the store persists (whole `Note` via postcard vs extracted facts)" | **YES**: If validation runs at index time, redb must persist validation errors. If validation is advisory on-demand, index persists facts without schema coupling. | **NO**: Read model prefers decoupled facts. |
| **21: Index Freshness** | `issues/21:22-24, 30` [V]: "per-axis invalidation... whether a parser/schema identity enters the fingerprint" | **YES**: If validation gates indexing, schema edits must invalidate redb cache. If validation is an on-demand pass, schema edits do NOT invalidate index. | **NO**: Landed fingerprint (`207cc3fb`) already excludes schemas. |
| **22: Query Core & Plan** | `issues/22:9-10` [V]; `issues/15:202-203` [V]: "schema note typing: the schema string override" | **YES**: Query core needs to know if schemas provide runtime typing or if note values stay truthful to parse classification. | **PARTIALLY**: Presupposes text-override rule, not diagnostic validation. |
| **24: Template Boundary** | `issues/24:15-16` [V]: "`Schema` access without the `State` temp-cache hack; schema loading eager vs lazy" | **WEAK**: Relates to passing `Schema` handle to minijinja, not validation. | **NO**. |
| **26: Workspace Host** | `issues/26:9-10` [V]: "Where does service composition live (config load + trust, schema load, index refresh/persist, query execution...)" | **YES**: Host must compose `SchemaService` with the validator and diagnostic emitter. | **YES**: If `traces check` or LSP diagnostics are hosted services. |
| **27: Diagnostic Arch** | `issues/27:5, 12-14` [V]: **BLOCKED BY 16**. "where schema validation diagnostics (if any) land" | **DIRECT**: Ticket 27 cannot be resolved until ticket 16 decides where validation diagnostics live and whether they carry AST spans. | **YES**: Explicitly assumes validation diagnostics exist or are rejected. |
| **28: Crate Topology** | `issues/28:14-15, 39-44` [V]: workspace split starting with CLI; public surface | **YES**: Determines crate boundary placement: if `schema` and `note` are separate crates, validator must live in a higher crate. | **NO**. |
| **29: Reconciliation** | `issues/29:5, 12-16` [V]: codebase-wide reconciliation | **YES**: Formally blocked on 16 (and 14-28). | **NO**. |
| **30: Buffer Consistency** | `issues/30:9-13` [V]: "For committed long-lived LSP... rules when editor has unsaved content" | **DIRECT**: In-editor validation must validate live unsaved buffer text with spans, NOT stale index data from redb. | **YES**: Presupposes validation runs over live buffer representations. |

---

## 6. Candid Assessment of Prior Reviewer Hedging

### 6.1 What Was Previously Argued

The previous reviewer recommended:

1. Retain a **span-free kernel** as a hedge to protect index performance.
2. Assert that **spans belong entirely to ticket 19** and should be excluded from ticket 16.
3. Treat schema validation as a future LSP concern that can be deferred without affecting current design.

### 6.2 Why That Hedge Fails (Adversarial Assessment)

- **Flaw 1: LSP is committed, not speculative**: Ticket 13 explicitly resolved that the LSP is the next committed consumer and that editor features require source-aware capabilities (`issues/13:37-43` [V]). Designing a validator today against a span-free `Note` produces a CLI-only toy that cannot populate LSP `Diagnostic.range` (`research/10:59` [V]).
- **Flaw 2: The False Dichotomy of Spans**: The hedge assumed that introducing spans meant bloating `Note` and regressing redb deserialization (the 18.48 ms cliff in `research/11:93` [V]). But Ticket 13 reopened the dual-model architecture (`issues/13:45-47` [V]) and Ticket 14 split `Link`/`NoteLink` (`issues/14:484-490` [V]). By separating an editor-shape `NoteDocument` (with `ByteSpan`) from an index-shape `NoteFacts` (span-free), we achieve zero-overhead indexing AND byte-accurate validation.
- **Flaw 3: Frontmatter Spans Cannot Be Deferred Indefinitely**: Frontmatter is parsed directly from YAML into `NoteFieldValue` (`src/note/metadata.rs:71-72` [V]). Without spans, a diagnostic for `status: "Invalid"` can only point to the whole file. If ticket 16 ignores spans, any validation diagnostic design will be immediately obsolete when editor integration begins.
- **Flaw 4: Circular Deadlock**: Deferring spans to ticket 19 while trying to decide ticket 16 leaves ticket 16 paralyzed: it cannot specify `validate(&Note, &Schema) -> Vec<Diagnostic>` because it does not know what `Note` provides.

### 6.3 Failure Modes Compared

- **Failure Mode of "Validator on Span-Free Note"**:
  - *Short-term*: Clean and simple in ticket 16.
  - *Long-term*: When LSP is wired up, `validate(&Note)` cannot produce LSP `Range`s. The team is forced to either: (a) re-parse the file with an ad-hoc scanner, (b) bolt on an uncoordinated `spans` sidecar, or (c) rewrite the validator from scratch.
- **Failure Mode of "Validator on Editor-Shape NoteDocument"**:
  - *Short-term*: Requires ticket 16 to accept that `Note` will split (or that validation consumes an abstract source target), anticipating ticket 19.
  - *Long-term*: If ticket 19 unexpectedly rejects the split, the editor validator simply falls back to an on-demand buffer reparse (Shape D, standard in Taplo/rumdl), which still works cleanly and requires zero index cache changes.

---

## 7. Not Verified

1. **[H] Exact memory overhead of `ByteSpan` on `NoteDocument`**: While `ByteSpan` is verified at 8 bytes (`src/position.rs:18` [V]), the total in-memory size of a heap-allocated `NoteDocument` retaining spans on every frontmatter key/value was not measured with a Criterion benchmark.
2. **[H] Single-pass YAML span extraction**: Noyalib 0.0.53's private event scanner was not benchmarked against a custom Logos-based YAML frontmatter scanner for recovering key/value byte spans without the full CST overhead.
3. **[H] Performance of on-demand buffer validation**: The exact latency of validating an open 500-line markdown buffer against 10 schemas on every keystroke was not micro-benchmarked (inferred to be $<50$ µs based on 9 µs frontmatter parse in `research/11:229` [V]).

---

## 8. What This Changes for Ticket 16

1. **Decouple Validation from Index Storage [P]**: Schema validation must be strictly **advisory and on-demand** (following `research/03:110-115` [V] Taplo/Metadata Menu precedent). Validation MUST NOT gate indexing or run during `IndexerService::parse_note`. `index.redb` remains an invariant-free facts cache; `.traces/schemas/` remains excluded from `IndexFingerprint`.
2. **Define Validator Interface on an Abstract Source Target [P]**: Ticket 16 should define the validator signature as:

   ```rust
   pub trait NoteSourceTarget {
       fn frontmatter_fields(&self) -> &IndexMap<FieldKey, NoteFieldValue>;
       fn field_span(&self, key: &FieldKey) -> Option<ByteSpan>;
   }
   pub fn validate_note(target: &impl NoteSourceTarget, schema: &Schema) -> Vec<SchemaDiagnostic>;
   ```

   This unblocks ticket 16 without dictating ticket 19's choice:
   - If ticket 19 chooses Candidate B (`NoteDocument`), `NoteDocument` implements `NoteSourceTarget`.
   - If ticket 19 chooses Candidate D (Span-free `Note` + reparse), an ephemeral buffer adapter implements `NoteSourceTarget`.
3. **Place Validator in a Dedicated Bridge Module (Tier 6+) [P]**: Because `src/schema/` (Tier 4) has zero dependencies on `src/note/` (Tier 5), the validator should NOT be placed inside `src/schema/model.rs`. It belongs in a dedicated analysis/validation service (e.g. `src/schema_validator/` or `src/validation/`), maintaining the clean acyclic module DAG.
4. **Settle Schema Type Override for Text [P]**: Per ticket 15 (`issues/15:104-105` [V]), frontmatter parsing is quote-insensitive. Ticket 16 must ratify the rule that a schema field declared as `type = "string"` overrides heuristic date/duration promotion, preserving text truthfulness without quote hacks.
