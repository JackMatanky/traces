# Source span/position model for the Note AST

Type: grilling
Blocked by: 39
Status: resolved

## Question

Grounded fact (from direct investigation of `src/note/`): **most semantic
entities in the parsed `Note` AST carry no source position today.**
Specifically:

- Links (`src/note/links.rs:26`, `Link { target, text, kind, embedded }`): no
  line/byte position.
- Tags (`src/note/model.rs:29`, `Vec<Tag>`): no location.
- Inline Fields (`src/note/field.rs:30`, `IndexMap<FieldKey,
  Vec<NoteFieldValue>>`): no location.
- Frontmatter (`src/note/metadata.rs:19,50`): parsed via `yaml-serde` into an
  `IndexMap`; raw text and spans discarded.
- Headings are not retained in the AST at all today (only tags/fields found
  inside them are collected).
- The one exception: List Items/Tasks track a 1-indexed `SourceLine` (not a byte
  offset) via `ListItemPosition` (`src/note/lists.rs:285`).
- `src/position.rs` already defines the shared `BytePos`/`SourceLine` newtype
  vocabulary the rest of the codebase would extend.
- `src/note/parser/line.rs`'s `LineIndex` already does O(log n) `BytePos` to
  `SourceLine` conversion via a precomputed line-start table, and
  `pulldown-cmark` (the underlying event-stream parser,
  `src/note/parser.rs:108`) natively emits byte-offset spans per event. The raw
  data needed to attach spans is therefore available at parse time, even though
  it is currently discarded.

LSP hover/definition/references/rename/completion-context-detection all **need**
precise ranges. Decide:

- Whether spans get added to the `Note` AST model itself (making `Link`, `Tag`,
  inline-field entries, and reconstructed heading nodes span-aware
  structurally), versus a parallel LSP-only span index built during/alongside
  parsing without touching the shared `Note` model, versus re-deriving positions
  on demand by re-scanning text.
- Byte offset vs UTF-16 code-unit offset: the LSP wire protocol's default
  `Position` encoding is UTF-16 (confirm current behavior/negotiability via
  `docs/refs/lsp_spec.md`'s position-encoding section) while
  `src/position.rs::BytePos` is UTF-8 bytes. Decide the conversion boundary and
  whether `positionEncoding` capability negotiation (LSP 3.17+) is used to
  request UTF-8 from clients that support it, which avoids conversion entirely
  for those clients.
- Whether this is a breaking change to the `Note`/redb-persisted schema (spans
  stored in the `NOTES` table means an index-format version bump and a
  migration/rebuild-on-load) or spans are computed transiently at LSP-request
  time from raw source text plus existing line-only positions, never persisted.
- Whether frontmatter needs raw-text-relative span reconstruction (since it is
  currently fully opaque after YAML-parse) for frontmatter-field
  hover/completion/diagnostics to work, and how (re-scan the frontmatter block
  text, then map YAML value paths back to source lines).
- Whether the underlying `Note` parser itself stays a full single-pass re-parse
  on every edit (today's model, and per the Markdown Oxide/Marksman/zk research
  a genuinely accepted pattern even in mature PKM LSPs at note-collection scale,
  not just a legacy shortcut) or becomes incrementally re-parseable (e.g. a
  rope-based text representation like `ropey` plus incremental re-lex/re-parse
  of only the changed region, or a tree-sitter-style incremental grammar). This
  question was previously folded silently into ticket 14 as "full re-parse,
  debounced or not" without ever considering true parser-level incrementality as
  an option. Decide it explicitly here, on performance merits (ticket 33) and
  implementation cost, not by default inheritance from today's CLI-batch parsing
  model.

This has no research blocker: it is answerable from the existing codebase alone.
It blocks 15 (links/references), 19 (frontmatter/inline-field intel), 26
(definition/references/hover/rename), and 13 (LSP persistence/caching).

## Answer

### 1. Spans on the AST: extend existing types, don't build a parallel index

Add `Range<BytePos>` fields to `Link`, `Tag`, inline-field entries, and a new
`Heading` AST node. Capture `range.end` from `pulldown-cmark`'s
`into_offset_iter()` (currently discarded at `src/note/parser.rs:106`) and from
`logos` token spans in `InlineTokenLexer`. This is additive: existing consumers
that don't care about spans keep compiling unchanged.

A parallel LSP-only span index was rejected. It duplicates the
position-computation work the parser is already doing in the same pass, and it
violates the standing constraint "reuse existing services rather than building
parallel LSP-only models."

### 2. Heading node: flat list with `level: u8`

Every studied peer (Marksman, Markdown Oxide, Microsoft's generic Markdown
service) stores headings as flat lists with a level field, deriving hierarchy on
demand. No peer builds a parent-child tree at parse time. A flat list is the
smallest API surface, and tree structure can be built from it in O(n) by a
stack-based algorithm whenever a consumer actually needs parent pointers
(symbols view, heading-anchor links). No such consumer was identified that would
materially benefit from a parse-time tree.

### 3. BytePos width: `u32` with `From`/`TryFrom`

`BytePos` narrows from `usize` to `u32` (4 GiB/file cap). `From<u32>` gives
infallible construction; `TryFrom<usize>` gives narrowing from `usize` with
`PositionError` on overflow (the caller uses `.unwrap_or(BytePos::MAX)` at
pulldown-cmark event boundaries). This is consistent with
`SourceLine(NonZeroU32)`: same width philosophy, same safety net.

### 4. LineIndex: move to `src/position.rs`, widen to `pub(crate)`

`LineIndex` relocates from `src/note/parser/line.rs` to `src/position.rs`. It
has zero `note::parser` dependencies (it imports only `BytePos` and
`SourceLine`, both from `position.rs`), and its logic (count newlines, binary
search line starts) is domain-general. It widens from `pub(super)` to
`pub(crate)`. A `byte_to_utf16_cu(&self, offset: BytePos) -> u32` method is
added: it walks characters from the line start and sums `char::len_utf16()`.

The module doc in `position.rs` updates from "only the vocabulary lives here" to
"vocabulary and shared conversion infrastructure."

### 5. Text-buffer conversion: two concrete functions, no trait

The byte to LSP-position conversion happens at the LSP response serialization
boundary, not in the core AST. Two concrete call patterns:

- **File buffers** (closed notes, loaded from disk):
  `LineIndex::line_at(offset)` + `tracker.byte_to_utf16_cu(offset)`, both now in
  `src/position.rs`.
- **Live buffers** (open editor buffers, backed by `ropey::Rope` per ticket 14):
  `rope.line_to_utf16_cu(rope.byte_to_line(offset))`, ropey's native API.

A `TextBuffer` trait was rejected: there are zero polymorphic call sites (each
handler knows its text type from context), there are two implementations with no
third on the horizon (this violates the "Rule of Three" from
`anti-over-abstraction`), and the trait would be shallower than the two concrete
functions it wraps. The "single interface" preference does not apply here,
because the text type is determined by handler context, not by runtime dispatch.

### 6. Frontmatter spans: via `noyalib` `Spanned<T>` (reconciled 2026-09-23)

Frontmatter byte ranges come from `noyalib`'s `Spanned<T>` single-pass value +
position extraction, following the YAML parser swap decided in
[Metadata/frontmatter & inline-field
intelligence](19-frontmatter-and-inline-field-intelligence.md). You get full
YAML structure (nesting, block scalars, anchors, aliases), with ranges relative
to the frontmatter block. The originally-decided hand-rolled Level-2 raw-text
scanner is dropped as redundant: its sole motivation (`yaml_serde` has zero span
capability) disappears once that swap lands. Frontmatter spans therefore land
with ticket 19, not this ticket, and no consumer (19, 20) needs them earlier.

### 7. Parser: stay on pulldown-cmark, full re-parse per edit

Zero peer Markdown/PKM LSP (Markdown Oxide, Marksman, Microsoft's service) uses
tree-sitter-markdown. Its ecosystem positioning is "editor
highlighting/folding," and it is explicitly not recommended where parsing
correctness matters. `pulldown-cmark` is confirmed as "the industry standard in
Rust" for CommonMark. Full re-parse per edit is the accepted pattern across
every studied peer at note-collection scale. `ropey` is adopted for the
live-buffer text representation (ticket 14), not for incremental parsing.

### 8. Persistence: spans stay transient, never in redb

Spans are cheap to recompute (re-parsing a file the LSP just read also
re-derives every span in the same pass), they invalidate on every edit (a poor
cache-hit shape), and persisting them triggers an avoidable index-format version
bump. Only live-editing-session requests (hover, completion, rename) need spans,
and those are against a file whose current text the LSP already has to touch.

### 9. Ropey: scoped to live buffers (ticket 14), not universal

`ropey::Rope` becomes the text representation for live open buffers.
Closed/persisted notes stay as `&str` (today's model). The Note AST carries
`Range<BytePos>` (protocol-agnostic), and the LSP adapter converts using
whichever text representation is in scope. Ropey-everywhere is not foreclosed
architecturally but is not adopted now.

### Amended by 26 (2026-10-18)

- **Position-encoding negotiation decided (closes 11's open Question)**:
  negotiate utf-8-if-offered-else-utf-16 (the first client-preference encoding
  Traces implements; never utf-32; default utf-16), echoed via
  `capabilities.positionEncoding`. One total `convert` module at the protocol
  boundary (`negotiate`, `to_lsp_position`, `to_lsp_range`, `to_byte_pos`; all
  functions clamp, never panic) supersedes §4's `byte_to_utf16_cu`-only method
  and §5's two-function disk/ropey split. `byte_to_utf16_cu` is retained as a
  thin `LineIndex` method so 19/23's prerequisite text stays true. Inbound
  `to_byte_pos` covers the direction 11 never handled. The module doubles as the
  miette bridge (miette SourceSpan = byte offset + len, no adapter crate
  needed).
- **§5/§9 ropey branch deleted**: ticket 14 resolved `BufferState { content:
  String, version: i32 }` (no rope). Disk and overlay both surface as `&str` via
  `ContentResolver::text_of`, so one LineIndex-backed conversion helper serves
  both. This reconciles the 11/14 inconsistency (11's "ropey for live buffers"
  had nothing to split over).
- **§8 sentence amended**: cross-file definition/references/rename DO consume
  spans for closed files, not only live-editing-session requests. Serving model:
  re-parse on demand from `ContentResolver::text_of` for cold files, resident
  spans when cached. 26 does not need persisted spans (41-C is acceptable; 41-B
  is the upgrade path).

## Correction from 27 (2026-10-22)

- 11's answer names `rope.line_to_utf16_cu(...)`; ropey 1.6.1 has no such method
  (full-crate search, zero hits). The correct idiom is
  `rope.char_to_utf16_cu(rope.byte_to_char(offset))`. The decision intent is
  unchanged, and 26 already retained `byte_to_utf16_cu` as a thin LineIndex
  method name. Separately: ticket 27's document-scope features do NOT need 11's
  AST spans (the LSP-side walk produces transient exact spans per request via
  `into_offset_iter()`); 11's spans remain what 26 (cross-file nav) and 41
  (persistence) need ([27 pass 2, Part
  XII.1](../research/27-structural-symbols-editor-intelligence-pass2.md)).
