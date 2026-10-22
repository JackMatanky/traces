# Ticket 27 pass 2 — A1 codebase inventory (independent re-research)

Scope: what the CURRENT Traces codebase actually contains that ticket 27
(symbols/outline, folding, selection ranges, document links, template links,
CodeLens, code actions, semantic tokens, documentHighlight) would consume, plus
the design-only seams from settled tickets 10/13/14/26 that 27 would ride on.
Independence note: this pass deliberately did not read
`research/27-*.md`, `research/27-pass2/*` (sibling files), or any prior 27
research.

Method: `codegraph_explore` (MCP) for first orientation passes, then direct
file reads (explore returned symbol maps but deferred bodies for these files).
All line cites are from the working tree at report time.

---

## 1. Note AST and parser (`src/note/**`)

### 1.1 The `Note` model

`Note` (`src/note/model.rs:24-33`) holds exactly seven components:

```rust
path: PathBuf, frontmatter: Option<Frontmatter>, lists: Box<[ListItem]>,
headings: Box<[Heading]>, outlinks: Box<[Link]>,
inline_fields: IndexMap<FieldKey, Box<[NoteFieldValue]>>, tags: Box<[Tag]>
```

Accessors: `frontmatter()` (:84), `lists()` (:91), `headings()` (:98),
`outlinks()` (:105), `inline_fields()` (:122, `pub(crate)`, test-only caller),
`fields()` (:142, `pub(crate)`, frontmatter-then-inline iterator),
`get()` (:164), `tags()` (:179), `list_items()` (:185), `tasks()` (:191 —
filters `lists` to `kind().is_task()`), `descendants()` (:202). `Note` is
`Serialize`/`Deserialize` (postcard-persisted in the redb `NOTES` table) —
any new AST field is a persistence-format concern unless `#[serde(skip)]`ed
(test pin: postcard round-trip at model.rs:574-626).

### 1.2 Headings — level, line, no span, two-pass

`Heading` (`src/note/heading.rs:10-14`): `level: u8`, `text: String`
(markup-stripped, built by `push_text`), `line: SourceLine` (1-indexed line of
heading start). No byte span, no end line, no slug field.

Collection is a **second, separate parse** (`collect_headings`,
`src/note/parser.rs:118-162`): the main metadata pass runs with
`MARKDOWN_OPTIONS` and treats `CmarkTag::Heading` as a plain text block
(`handle_start_tag`, parser.rs:281-283 → `start_text_block`); then
`parse_markdown` (parser.rs:106-115) re-runs `Parser::new_ext` with
`HEADING_OPTIONS` (same options + `ENABLE_MATH`, parser.rs:73) solely to build
headings. From `into_offset_iter()` only `range.start` is kept, converted to a
line via `line_index.line_at(BytePos::saturating_from(range.start))`
(parser.rs:140). Heading text accumulates `Event::Text`/`Event::Code`
fragments and turns soft/hard breaks into a space (parser.rs:148-157).

Because collection keys off structural `Start(Heading)` events, headings
nested in block quotes/list items are also collected (test
`collects_headings_inside_blockquotes_and_list_items_without_losing_metadata`
is in the codegraph relationship list for `Heading`).

### 1.3 pulldown-cmark consumption & ENABLE flags

- Options actually set (parser.rs:68-70):
  `ENABLE_YAML_STYLE_METADATA_BLOCKS | ENABLE_WIKILINKS | ENABLE_FOOTNOTES`.
  `HEADING_OPTIONS` adds `ENABLE_MATH` (parser.rs:73). **GFM is NOT enabled**
  (so `Tag::BlockQuote(Some(BlockQuoteKind))` never fires — matches ticket
  17's finding that callout-kind detection would need `ENABLE_GFM`).
- pulldown-cmark 0.13.4, `default-features = false` (Cargo.toml:136).
- `into_offset_iter()` IS used (parser.rs:108-109, 121-122); the full
  `ByteSpan` per event reaches `handle_event` (parser.rs:241), but is
  immediately reduced: `handle_start_tag` receives only `span.start()`
  (parser.rs:247) and the only consumers of position data are (a) list-item
  start lines and (b) the footnote-reference scan-text slice
  `ctx.input.src()[span.to_range()]` (parser.rs:254). Event end offsets are
  otherwise **discarded**; nothing span-bearing survives into the `Note`.

### 1.4 Tasks / checkboxes

`ListItem` (`src/note/lists.rs:41-50`): `text: ListText` (raw+clean dual
repr), `kind: ListItemType`, `depth: u8`, `line: SourceLine`,
`parent: Option<SourceLine>`, `is_ordered: bool`,
`fields: Option<Box<ListFieldMap>>`, `tags: Box<[Tag]>`. Hierarchy is
reconstructed from `depth` + parent **line** (lists.rs:1-16 module docs) —
there are no child containers and no byte spans. `ListItemType` (lists.rs:234)
classifies Plain / Checkbox / Task; `TaskListItem` (lists.rs:348) carries
`TaskStatus` (`src/task.rs:51`), `TaskDateSet` (task.rs:498), priority,
precomputed subtree-completion state. `start_item` computes the item's line
from the event start byte (parser.rs:519-522). Marker classification lives in
`src/note/parser/marker.rs`; task extraction in
`src/note/parser/task.rs`.

### 1.5 Block quotes / callouts / code blocks / frontmatter / tags / inline fields

- Block quotes: a bare `CmarkTag::BlockQuote(_)` arm that only does list
  nesting bookkeeping (`resolve_pending_marker` + `start_nested_block`,
  parser.rs:288-291). No callout model, no first-line scan, no
  `CalloutBlock` — ticket 17's design (its own Question line 73-81) is
  entirely unimplemented.
- Code blocks: `BlockContext::CodeBlock` (parser.rs:168-174, 444-452);
  excluded from metadata scanning; event content is otherwise dropped
  (no stored code-block nodes — folding ranges for fences would need either
  a re-scan or new capture).
- Frontmatter: buffered as raw text during `MetadataBlock`
  (parser.rs:394-409) → `RawFrontmatter(String)`
  (`src/note/metadata.rs:21`) → parsed via **noyalib** YAML 1.2
  (`yaml::parse`, `src/yaml.rs:27-29`; `ParserConfig::serde_yaml_compat`)
  into `Frontmatter { fields: IndexMap<FieldKey, NoteFieldValue> }`
  (metadata.rs:96-99). **No spans map** on `Frontmatter` yet (ticket 20's
  parallel `spans` design is unimplemented), and `noyalib` `Spanned` is not
  used by the note parser (plain `from_str_with_config`).
- Tags: `Tag(Box<str>)` newtype (`src/tag.rs:23`), validated `#`-identifiers;
  no span. Body tags are accumulated document-order from the lexer
  (`lexer::scan_metadata`), then frontmatter tags from the configured
  `tags_name` key are appended after body tags (parser.rs:378-382).
- Inline fields: `IndexMap<FieldKey, Vec<NoteFieldValue>>` accumulated from
  the same lexer passes (paragraph/heading flush at parser.rs:468-479, list
  items via `ListTracker`); per-item fields/tags ride on `ListItem`.
  `NoteFieldValue` is `src/note/field.rs:43`. No spans anywhere.

### 1.6 Inline lexer

`src/note/parser/lexer.rs` (logos-based, task emojis/priority/tags/fields) —
internal scan buffers only; token spans are used ephemerally and never
surfaced on the AST.

### Constraints / opportunities (ticket-27 features riding the AST)

- Document symbols need heading spans (name range + section range). Today
  only a start `SourceLine` exists (`heading.rs:13`); the end line/byte range
  of a section is not representable. Ticket 11 (resolved design) promises
  flat `Heading` with span; the code has the node but not the span. The
  two-pass collection (parser.rs:113) is also the seam where a merged
  single-pass span-preserving heading capture would land.
- Folding ranges (headings' sections, list nesting, fences, callouts,
  frontmatter block) have *no* structural span substrate in the AST:
  sections have no end boundary, code blocks store nothing, callouts don't
  exist, frontmatter stores only a parsed map (raw text length is known via
  `RawFrontmatter` but discarded after parse). List nesting depth exists
  (`ListItem::depth`, lists.rs:44) but item end lines don't.
- Selection ranges are the purest "no substrate" feature: nothing nested
  (inline spans, list nesting, heading inline contents) retains byte ranges.
  Would need the ticket-11 span retrofit or an on-demand re-scan.
- Task outline (23's S3 design: Array-kind, line-granular ranges, hierarchy
  via `parent: Option<SourceLine>`) can be *approximately* built today from
  `Note::tasks()` + `depth`/`parent`; byte-precise ranges wait on 11/41.

---

## 2. Position model (`src/position.rs`)

- `LineIndex` (`src/position.rs:33`, `pub(crate)` struct; impl :35-62):
  exactly **two** methods — `new(source)` (:39) precomputes line starts, and
  `line_at(BytePos) -> SourceLine` (:57) binary-searches via
  `partition_point`. No `line_to_byte`, no column API, and
  **`byte_to_utf16_cu` does NOT exist** (repo-wide grep for
  utf16/UTF-16: zero hits in `src/`). Ticket 11's and 26's
  `byte_to_utf16_cu`/convert-module designs are unimplemented; there is
  currently no UTF-16 conversion capability anywhere in the codebase.
- `ByteSpan` (:79, `pub(crate)`): half-open byte range with `start()`/`end()`/
  `to_range()`; `From<Range<usize>>` etc.
- `BytePos` (:190, `pub(crate)`): `u32` newtype, `MAX`, `saturating_from`
  (:211), `From<u32>`, `From<BytePos> for u32/usize`, `TryFrom<usize>`.
- `SourceLine` (:270, `pub`): `NonZeroU32`, 1-indexed, `new(u32) -> Option`.
- `Spanned<T>` (:359, `pub(crate)`): value + `ByteSpan` with value-first
  ordering (PartialOrd at :454). Currently used by the query parser, not the
  note parser.
- Everything except `SourceLine` is `pub(crate)` — widening (11's decision
  mentions `pub(crate)` for LineIndex, but the LSP work will need either
  `pub(crate)` module-internal use or widening, depending on 34's crate
  split).

### Constraints / opportunities

- Any 27 range computation must currently live inside the crate (all position
  types are `pub(crate)`). UTF-16 conversion for client positions does not
  exist; 26's negotiated-encoding `convert` module (its Q7/round answer) is
  the designed home and nothing in code contradicts it.
- `LineIndex` is rebuilt from scratch per parse (`ParserContext::new`,
  parser.rs:360) — cheap (one pass over newline positions) and available for
  on-demand re-scans in 27 handlers via `ContentResolver::text_of` (14).

---

## 3. Index & link data (`src/index/**`)

### 3.1 WorkspaceIndex / FileEntry

`WorkspaceIndex` (`src/index/entry.rs:22`): immutable `Box<[FileEntry]>`,
sorted by path; `entries()` (:108), `entry_at(RowIndex)` (:124, `pub(crate)`).
`FileEntry` (:136): `file: FileMeta` + `note: Option<Note>` +
`inlinks: Box<[PathBuf]>` with `file()`/`note()` public and
`inlinks()` (:168) **public** — but inlinks are **note-level path lists**
only. `FileMeta` (`src/file.rs:45`): path/format/created/modified/size/day —
no tag/class fields.

### 3.2 LinkResolver & InlinkMap granularity

- `LinkResolver` (`src/index/inlinks.rs:319`) is **`pub(super)`** (index
  module only): basename trie index (`super::trie::BaseNameIndex`) built in
  one O(n) pass (:326-346); `resolve` (:369-405) tries exact path →
  missing-`.md` path → nearest-by-basename (folder-distance; equal-distance
  ties stay unresolved — `nearest_by_basename` returns `Option`). Consumed
  only by inlink construction (`with_resolver` :83-99) and refresh
  (refresh.rs:444, service.rs:100). **Not reachable from outside
  `src/index/`**; 26's design already budgets "expose `LinkResolver` via a
  facade method" as one of two named code changes.
- `InlinkMap` (inlinks.rs:59): `HashMap<PathBuf, Box<[PathBuf]>>` —
  **note-level edges** (target file → sorted deduped source files). Multiple
  outlinks from one source to one target collapse to one edge
  (`with_resolver` dedup, :90-92). No heading-anchor edges, no per-occurrence
  edges; the redb `LINKS` multimap mirrors this (key=target path bytes,
  value=source path bytes, `src/index/tables.rs:33-39`). 26's resolution
  divergence explicitly records this: the inlink graph keeps nearest-picked
  edges while navigation reports ties (ticket 26 answer §2).

### 3.3 Store/query accessors that could feed an outline or symbol picker

`IndexStore` (`src/index/store.rs:151`): `open` (:166), `read_notes_batch`
(:201), `read_files_batch` (:216), `read_links_for_targets` (:235),
`paths_with_tag` (:276), `paths_with_file_class` (:299), `paths_in_folder`
(:315) — all `pub(crate)`. Multimaps: `PATHS_BY_TAG`, `PATHS_BY_FILE_CLASS`,
`TAGS_BY_PATH`, `FILE_CLASSES_BY_PATH` (tables.rs:41-75).

**`unique_tags()` / `unique_file_classes()` / `SchemaService::suggest_class()`
do NOT exist** (repo-wide grep: zero hits). They are ticket 21's
design-prerequisite names only. Anything in 27 wanting "all distinct
tags/classes for workspace symbols" must either add these store methods or
iterate the existing multimaps.

`QueryService` (`src/query/service.rs:52`): `run(&Arc<WorkspaceIndex>,
QueryBuilder)` (:126) and `run_from_store` (:151); `matched_file_rows`
(:318-329) is the linear `0..index.entries().len()` scan the map flags
(cite holds at exactly :318-329). Task rows expand via `item_rows`
(:300-310). `QueryMode::Tasks` rows are the substrate for 23's conditional
workspace/symbol task picker.

### 3.4 Refresh seams

`RefreshPlan` (`src/index/refresh.rs:311`, `pub(super)`): built only by
`collect(root, fingerprint)` (:336) — whole-tree `IndexerService::scan`
joined with store open, `FileDelta::compute` merge-join. The single-file
constructor (13's §3 design: changed path + editor content, no rescan) does
not exist; `RefreshState::Fresh/Stale/Drifted` + `PersistFailed` are the
lifecycle (refresh.rs:113+). `IndexerService` (`src/index/service.rs:36`)
derives from `&Config` (root/tasks/frontmatter/dimensions/fingerprint, :44-55).

### Constraints / opportunities

- Document links, CodeLens reference counts, and documentHighlight all need
  per-occurrence (span) data that the index does not store: inlinks are
  note-level, spans are transient by design (11 §8, tickets 40/41 pending).
  27 will compute document-local spans by parsing the document (via
  ContentResolver) and use the index only for target existence/resolution.
- CodeLens "N references" over headings is NOT answerable from the current
  graph for headings (no heading-anchor edges; stem-tie union is
  best-effort per 26). Only note-level counts exist via
  `FileEntry::inlinks()`/`InlinkMap::inlinks_of`.
- rumdl coexistence for `documentLink` cannot be verified from code (Traces
  has zero protocol code); it remains a 03/32-config question (map baseline:
  rumdl's `enableSymbols`/`enableLinkCompletions`/`enableLinkNavigation`
  documented for ceding).

---

## 4. Config (`src/config/**`)

`Config` (`src/config/model.rs:49-54`): `root: Arc<Path>`, `templates:
TemplateConfig`, `schemas: SchemasConfig`, `frontmatter: FrontmatterConfig`,
`tasks: TaskConfig`. A `[lsp.*]` section would be a new sibling field +
`raw::RawConfig` section + `ConfigBuilder` merge + (per the digests
convention in `src/config/mod.rs:19-27`) a section-qualified digest label —
the exhaustive-destructure convention makes an unlabeled new setting a
compile error by design. Loading pipeline (mod.rs docs :8-17):
`discovery::ConfigLocator` → trust verification (`file`/`trust`/`tracker`) →
`raw::RawConfig` → `ConfigBuilder` merge → `Config`. `ConfigService` is the
administering entry. Ticket 31 owns LSP config-source questions; nothing in
code anticipates an `[lsp]` section (grep for "lsp" in src/: zero code hits).

### Constraints / opportunities

- 27's coexistence toggles (rumdl `enableSymbols`-style ceding) and any
  feature-depth switches must go through this pipeline; the digest convention
  means config additions are loud. Frontmatter config (`tags_name`,
  `aliases_name`, `title_name`, date keys — model.rs:446-490) is the
  existing model for per-feature config sub-structs.

---

## 5. Template subsystem (`src/template/**`)

- `TemplateService` (`src/template/service.rs:38`): `new`, `list_available`
  (:81 — top-level `.md` names from local-before-global dirs, via
  `TemplateLoader::from(&Config)`), `render_to_file`/`render`/`write`.
- `TemplateEngine` (`src/template/engine.rs:75`): minijinja `Environment`
  with `set_debug(true)` (:120) and `set_loader` backed by `TemplateLoader`
  (:121+); `{% include %}`/`{% extends %}` resolve through
  `TemplateLoader::find` (loader.rs:1-21 docs; local→global, dot-prefixed
  names allowed — deliberately not `minijinja::path_loader`). `file.include()`
  is a separate helper path (`engine` docs :10).
- **No static analysis capability**: no `analysis.rs` (module list:
  engine/{cache,date,error,file,num,path,query,schema,string,ui,yaml}.rs),
  no `machinery`/`unstable_machinery` reference anywhere, and the dep is
  **minijinja 3.0.1** (Cargo.toml:131) — ticket 22's design pins a
  `=2.24.0` machinery-gated analysis module; that pin does not match the
  current dependency and would be part of 22's implementation, not present
  today. No `undeclared_variables` use. Every render re-parses (no root AST
  cache; only `engine/cache.rs` exists — render-result cache).
- Template *files* are ordinary `.md` files in template dirs; when those dirs
  are inside the workspace root they are indexed as notes like any other
  file (no special FileFormat for templates).

### Constraints / opportunities

- Template references as document links (27 Question bullet 5) have **zero
  current substrate**: include/extends spans exist only inside minijinja's
  own parse (22's Q20(b)/machinery plan), and existence checks route through
  `TemplateLoader::find` (disk-only today; 22's overlay seam is design-only).
  `TemplateService::list_available` is the existing enumeration API a
  template documentLink hover/target could reuse once 22 lands its analysis
  module.
- Rendering must never run in an LSP handler (ui.* blocks on DialogProvider
  I/O — map baseline; 09's decision: static-analysis-only, non-interactive
  DialogProvider kept open). Nothing in code contradicts this.

---

## 6. Schema / frontmatter model

- `Schema` (`src/schema/model.rs:11-17`): name, `fields:
  IndexMap<FieldName, SchemaFieldDef>`, ancestors/children/descendants sets;
  `Schema::field()` lookup (:80). `SchemaFieldDef`
  (`src/schema/fields.rs:53-57`): `kind: SchemaFieldType`, `required: bool`,
  `multi: bool` only — **no `origin: Option<SchemaName>`, no `description`**
  (ticket 20's design-only additions; goto-def provenance for schema fields
  depends on them). `SchemaFieldType`: Input/Boolean/Select/Number/Date/File
  (:104-110).
- `SchemaService` (`src/schema/service.rs:36`): load-once in-memory registry
  (map baseline holds); implements `FileClassExpander` for queries
  (`src/file_class_expander.rs:15`). `suggest_class()` absent.
- Frontmatter file-class resolution: class field name from
  `SchemasConfig::class_field_name()` (service.rs:51), indexed into
  `PATHS_BY_FILE_CLASS`/`FILE_CLASSES_BY_PATH` multimaps by
  `IndexerService`. `Frontmatter` itself carries no class field — class is
  just a normal frontmatter key under the configured name.
- No runtime validation against Note content (map baseline holds — nothing
  in `src/schema/**` walks notes).

### Constraints / opportunities

- 27's optional "File-Class-field structure" in the outline can read class
  values from `Note::frontmatter()` + config class field today; per-field
  schema provenance (for richer symbol detail) waits on 20's
  `SchemaFieldDef::origin`.

---

## 7. Existing LSP / protocol code

**None.** `Cargo.toml` has no `tower-lsp-server`/`lsp-types`/`lsp-server`
dependency (grep: zero hits), and no `src/` file references LSP types,
semantic-token legends, CodeLens, folding, documentLink, documentHighlight,
`workspace/symbol`, or `documentSymbol` (case-insensitive grep: only
unrelated "scaffold"/"folding" substring hits in GLOSSARY/cli/temporal).
All handler-shaped code is design-only in the ticket graph.

---

## 8. Design-only seams ticket 27 would consume (from settled tickets)

These do NOT exist in code; the citations below are to the ticket files'
own answer text.

### 8.1 Ticket 10 — analysis host

- Sharing: "a single `Arc<WorkspaceIndex>` behind a swap-on-refresh cell
  ... shared read-only across concurrent request handlers"
  (10 answer §"Sharing strategy", issues/10:27).
- Facade: "a thin analysis-host facade wraps `QueryService`/`SchemaService`,
  owning the `Arc<WorkspaceIndex>` swap point and the cached
  `SchemaService`, handed to every request handler" (10:31).
  27's handlers (symbols/folding/links/etc.) are expected to receive this
  facade, not instantiate services.

### 8.2 Tickets 13/14 — overlay & ContentResolver

- `RefreshPlan` gains a single-file constructor (changed path + editor
  content, no scan; `reconcile`/`persist`/`into_index` shared) — 13 §3
  (issues/13:31-33, 45).
- `ContentResolver` facade: "`fn note_of(&self, uri: &Url) -> Cow<'a, Note>`
  (overlay BufferState content parsed on-demand, or WorkspaceIndex's
  persisted Note), `fn text_of(&self, uri: &Url) -> Cow<'a, str>`" — 14 §3
  (issues/14:27). `BufferState { content: String, version: i32 }`, no
  `parsed_note`; overlay maintains its own LinkResolver (map decision 14).
- 23's S3 explicitly says task-outline content is served "via 14's
  ContentResolver + request-start Arc<WorkspaceIndex> snapshot"
  (issues/23:46) — the same serving model 27's document-scope features
  should use.

### 8.3 Ticket 26 — nav spine (Occurrence / NavTarget / Resolution)

- `nav::Occurrence { kind: OccurrenceKind, span: ByteSpan }` with
  payload-carrying kinds (Wikilink/Embed/FrontmatterLink/Heading/Tag/
  FieldKey/...), `nav::NavTarget { kind, doc, name_range, scope_range }`,
  `NavContext` facade exposing `locate/resolve/candidates/references/rename/
  highlight` — 26 answer §1 (issues/26:23).
- Three-state `Resolution = Unresolved | Unique | Ambiguous { StemTie |
  HeadingAnchor | PathTie }` via folder-distance argmin with explicit ties —
  §2 (26:25).
- Consumer matrix incl. "documentHighlight is document-local span matching
  and never errors on target ambiguity" — §3 (26:27); documentLink emits
  for Unique and Ambiguous targets (27's own Inputs-from-26, issue 27:23).
- Semantic-token modifiers three-valued (resolved/unresolved/ambiguous) —
  27's Inputs-from-26 (issue 27:23), amending a plain boolean.
- Position encodings: negotiated utf-8-if-offered-else-utf-16, one `convert`
  module at the protocol boundary, `byte_to_utf16_cu` retained as a thin
  LineIndex method — 26 §7 (26:35).
- Reference counts for CodeLens must use the **inlink graph** (nearest-picked
  edges), not the nav tie-reporting path — "the inlink graph keeps
  nearest-picked edges (backlink counts, 25's change-summary soundness,
  27's counts, 28's rewrite targeting all consume them)" (26:26).
- 5,000-reference ceiling is scoped to the references response path only,
  deliberately distinct from symbol ceilings (27:25).

---

## 9. Per-feature constraints/opportunities summary

1. **Document symbols / outline** — Headings exist with level+text+start-line
   only (heading.rs:10-14); no spans/end-lines, no nesting structure (flat
   list; nesting derivable from level stack). 23 defers *mechanics* (ranges,
   kinds, registration, coexistence) to 27 (23:42-47, 63): task items use
   `SymbolKind::Array`, headings must be String(15), line-granular minimum,
   byte precision = 11/41. `workspace/symbol` task-picker cap+ranking assigned
   to 27/29 (23:50(c)). rumdl cedes heading outline via `enableSymbols=false`
   (15:196-206 — but that's rumdl config, not Traces code).
2. **Folding ranges** — zero substrate for section ends/fences/callouts;
   17 owns callout detection (BlockQuote + first-line scan) and will "expose
   the parse tree; ticket 27 consumes it" (17:81) but none of it exists yet.
   Frontmatter block folding is derivable from the metadata-block event
   span if 27 re-parses rather than reads the AST.
3. **Selection ranges** — nothing in the AST is nested-span-bearing; purest
   re-scan/11-span-dependent feature.
4. **Document links** — 15 blocks 27 "for document-link ranges" (15:26); link
   target splitting (`LinkTarget`, links.rs:207-270) exists; spans do not.
   Coexistence with rumdl's documentLink (if any) is a 03/32 question —
   ticket 03's research is map-cited for the three settings; whether rumdl
   registers documentLink itself is not answerable from Traces code and must
   be checked against rumdl's boundary research at resolution time.
5. **Template references as document links** — no substrate (no machinery,
   no include-span extraction); 22's Q20(b) seam (overlay-aware `find`,
   issues/22:58) and analysis module (22:12) are design-only; minijinja
   version mismatch (3.0.1 vs 22's =2.24.0 pin) noted.
6. **CodeLens** — note-level reference counts computable today
   (`FileEntry::inlinks()`, `InlinkMap::inlinks_of`); heading-level counts
   impossible without new edges or per-document scans; 26 pins the inlink
   graph as the counting substrate (26:26).
7. **Code actions** — create-note action must check `resourceOperations`
   contains `create` (27:26); status-cycle action invariant owned by 23
   (cycle order = config order = sort_text), plumbing owned by 27 (23:47).
8. **Semantic tokens** — three-valued resolution modifiers from 26 (27:23);
   token substrate = spans (11) + spine resolution (26); no existing
   highlight/token code.
9. **documentHighlight** — spine method `highlight(target, doc)` promised by
   26 (26:23); document-local span matching, never errors on ambiguity
   (27:24); needs 11 spans + 26 spine, nothing existing.

---

## 10. Contradiction check vs map.md baseline facts (map lines 46-59)

| Baseline claim (map) | Verdict | Evidence |
| :--- | :--- | :--- |
| Single Cargo package, one binary `traces-pkm` | **Holds** | `src/main.rs` + `pub mod cli`; no workspace `Cargo.toml` members |
| Entirely synchronous, no async fn | **Holds** | No tokio/async-std in Cargo.toml deps (grep); no `async fn` in src |
| Fresh short-lived CLI process, no watcher/daemon | **Holds** | `IndexerService::refresh` whole-tree; no notify dep |
| Note = pulldown-cmark events + logos lexer; single-pass, no incremental re-parse | **Holds, one nuance** | Parser is single-pass per invocation but heading collection is a *second* `Parser::new_ext` pass over the same source (parser.rs:113, 118-162) — "single-pass" is true per-collector, not per-file |
| Spans almost entirely absent; only list items/tasks carry line-only position | **Holds** | links.rs:28-34 (no span), tag.rs:23, metadata.rs:96-99, heading.rs:13 (line), lists.rs:41-50 (line) |
| "Heading does carry a line" (corrected clause) | **Holds** | heading.rs:13 + parser.rs:140 |
| `BytePos`/`SourceLine`, `LineIndex` at src/position.rs:33 | **Holds** | position.rs:33 (LineIndex), :190 (BytePos), :270 (SourceLine) |
| pulldown-cmark emits byte-offset spans today (currently discarded) | **Holds** | parser.rs:108-111 `into_offset_iter`; end offsets discarded, start used only for item lines/footnote slices |
| WorkspaceIndex immutable snapshot, 3+4 redb tables, whole-tree mtime/size diff, rayon parallel | **Holds** | entry.rs:22; tables.rs (7 tables incl. LINKS/PATHS_BY_TAG/...); refresh.rs:311+; service.rs:96-101 |
| QueryService expects Arc<WorkspaceIndex>; miette span-aware QuerySyntaxError | **Holds** | service.rs:126; query/error.rs:134 area (`SourceSpan::from((span.start, span.len()))` at :137) |
| SchemaService load-once; no runtime validator | **Holds** | schema/service.rs:36; no note-walking validation |
| Template rendering interactive/unsafe for analysis; no static analysis; no AST cache | **Holds** | ui.rs DialogProvider helpers; no analysis.rs/machinery; engine re-parses per render |
| rumdl ships its own LSP with the three ceding settings | **Not verifiable from Traces code** — consistent with research corpus; flagging as externally-sourced, unchanged |
| `byte_to_utf16_cu` promised by ticket 11 / retained by 26 | **Design-only — does NOT exist in code** | zero utf16 hits in src/; not a contradiction of the baseline (baseline doesn't claim it exists) but important for 27 planning |
| `unique_tags()`/`unique_file_classes()`/`suggest_class()` (21 prerequisites) | **Design-only — do NOT exist** | zero grep hits; store has `paths_with_tag`/`paths_with_file_class` instead |
| Frontmatter `spans` map (20), `SchemaFieldDef::origin/description` (20) | **Design-only — do NOT exist** | fields.rs:53-57, metadata.rs:96-99 |
| noyalib Spanned for frontmatter spans (11/19) | **Partially landed** | noyalib IS the YAML parser (yaml.rs:27) but `Spanned` is unused by frontmatter; 0.0.53 |
| 22's minijinja `=2.24.0` machinery pin | **Mismatch with code** | Cargo.toml:131 = minijinja 3.0.1, no machinery feature; 22's implementation will need a dep change — worth carrying into 27's template-links risk register |

No outright contradiction with the map's stated baseline was found; the
map's baseline is accurate. The one wording nuance (heading collection is a
second parse pass) does not change any decision. The load-bearing planning
correction: *every span/encoding/analysis seam 27 depends on (11 spans,
`byte_to_utf16_cu`, 14 ContentResolver, 22 analysis, 26 spine, 21 unique_*)
is design-only, none is partially implemented except the noyalib YAML swap
itself*.

---

## Sources

- Ticket 27: `.scratch/md-pkm-lsp/issues/27-structural-symbols-and-editor-intelligence.md`
- Map: `.scratch/md-pkm-lsp/map.md` (baseline facts :46-59; decisions :61-103)
- Code: `src/note/{model,heading,links,lists,metadata,field}.rs`,
  `src/note/parser.rs` (+ `parser/{input,inline,lexer,list,marker,task}.rs`
  as named), `src/position.rs`, `src/tag.rs`, `src/file.rs`,
  `src/index/{entry,inlinks,refresh,service,store,tables}.rs`,
  `src/query/{service,error}.rs`, `src/config/{mod,model}.rs`,
  `src/schema/{model,fields,service}.rs`, `src/template/{engine,loader,service}.rs`,
  `src/yaml.rs`, `Cargo.toml`
- Design-only ticket text: issues `10` (:27,:31), `11` (:33,:37,:47,:62,:68,:78,:80),
  `13` (:10,:31-33,:45), `14` (:27,:42), `15` (:26,:52,:66,:85,:195-206,:242,:256-260),
  `17` (:16-17,:26,:60,:67,:73-87,:96), `21` (map :88), `22` (:12,:16,:43-51,:58,:70),
  `23` (:9,:41-50,:63), `26` (:23,:25,:27,:35,:45)
