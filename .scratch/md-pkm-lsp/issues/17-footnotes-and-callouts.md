# Footnotes & callouts

Type: grilling
Status: resolved
Blocked by: 01, 02, 05

## Question

Two smaller, related extended-Markdown-syntax scope decisions. Base them on
Markdown Oxide/Marksman (tickets 01, 02) and the generic-Markdown baseline
(ticket 05), since footnotes are closer to "generic Markdown extension" than
PKM-specific.

Footnotes (`[^1]` / `[^1]: definition`):

- Does Traces' note parser recognize footnote syntax at all today? Check
  `src/note/`: the earlier scout pass did not cover it, so check directly.
- If in scope: definition/references/hover linking a footnote reference to its
  definition, diagnostics for undefined/unused footnotes, and rename.

Callouts (`> [!NOTE]`, `> [!WARNING]`, etc., an Obsidian/GFM-alert-style
blockquote extension):

- Does the parser recognize callout syntax today?
- If in scope: completion for callout-type keywords, diagnostics for
  unrecognized callout types (against a fixed or configurable vocabulary).
  Folding-range behavior for callout bodies (feeds into ticket 27).

Decide scope (in/out) and depth for both, independently. They do not need to
share an implementation approach, only a shared research grounding.

## Answer

### Parser status today

- **Footnotes**: pulldown-cmark emits `Event::FootnoteReference` and
  `Tag::FootnoteDefinition` behind `Options::ENABLE_FOOTNOTES`, but Traces does
  not set this flag. The parser matches `FootnoteReference` at
  `src/note/parser.rs:225` only to reject task-list markers; no footnote content
  is extracted.
- **Callouts**: pulldown-cmark emits `Tag::BlockQuote(Some(BlockQuoteKind))`
  behind `Options::ENABLE_GFM`, but Traces does not set this flag. No callout
  handling exists.

### Footnotes: in scope

**Scope**: In scope. pulldown-cmark provides native support; Markdown Oxide and
md-lsp prove LSP value; the inverted-index implementation is cheap and
integrates cleanly with the existing parser.

**Depth: all seven operations**:

| Operation | Value | Complexity |
|---|---|---|
| Go-to-definition (`[^ref]` -> `[^ref]: def`) | High | Low (inverted index lookup) |
| Find references (def -> all refs) | High | Low (inverted index lookup) |
| Diagnostics: undefined references | Medium | Low (key set difference) |
| Completion: suggest labels when typing `[^` | Medium | Low (static list from index) |
| Hover: preview definition content | Low-medium | Low (index lookup + text slice, ~200 char truncation) |
| Rename: update label in refs + def | Medium | Low (file-local find-replace, simpler than heading/file rename; confirmed trivial by Panache, md-lsp, mdsmith) |
| Diagnostics: unused definitions | Low | Low (optional polish) |

**Implementation**:

- Enable `Options::ENABLE_FOOTNOTES` in `parse_markdown()`.
- Add `FootnoteIndex` to `ParserContext`: two `HashMap`s (definitions: label
  maps to range, references: label maps to vec of ranges), built during the
  single parse pass.
- `FootnoteRef` and `FootnoteDef` AST nodes carry `Range<BytePos>` spans (from
  `into_offset_iter()`, currently discarded).
- Diagnostics: undefined references (key in refs but not defs).
  Unused definitions (key in defs but not refs), duplicate definitions (same
  label defined twice: first wins, warn on second).
- Rename: file-local only (CommonMark footnotes are document-scoped); find the
  label range in each `Reference::Footnote` and `Referenceable::Footnote`, then
  replace label text.
- Spans stay transient and are never persisted in redb.

**Edge cases handled**:

- `[^]` (empty label): invalid, parser will not emit events.
- Self-referencing: valid but meaningless, optional warning.
- Definitions inside blockquotes/lists: parsed by pulldown-cmark, but some
  renderers do not support them. Flag as a diagnostic.
- Nested footnotes: the index handles this naturally (each label is independent).

### Callouts: in scope, limited

**Scope**: In scope, but limited to completion and folding-range data.
Diagnostics are skipped by design.

**Depth**:

| Operation | Value | Complexity |
|---|---|---|
| Completion: 13 canonical types + aliases + config extension | High | Low (static vocabulary + alias map + TOML config) |
| Folding-range data for ticket 27 | Medium | Low (detect blockquote boundaries) |
| Hover: type description | Nice-to-have | Low (static map) |
| Diagnostics for unknown types | **Skip** | N/A: contradicts Obsidian behavior; no tool in the ecosystem flags this; custom types via CSS make any list permanently incomplete |

**Implementation**:

- Enable `Options::ENABLE_GFM` in `parse_markdown()`.
- Detect callouts in the `BlockQuote` start handler: scan the first
  `Event::Text` after `Event::Start(CmarkTag::BlockQuote)` for
  `>\s*\[!([a-zA-Z]+)\]`.
- `CalloutBlock` AST node: `header_line`, `body_start`, `body_end`,
  `nesting_depth`, `type_identifier` (original text), `canonical_type` (resolved
  from vocabulary, `None` if unknown), `is_foldable` (`+`/`-` modifier).
- Vocabulary: 13 canonical types with alias resolution (case-insensitive).
  `hint` -> `tip`, `tldr` -> `abstract`, `done` -> `success`, etc.
- Config extension via `.traces/config.toml`:

  ```toml
  [callouts]
  extra_types = ["released", "code", "target"]
  ```

- Ticket 17 detects/parses callouts and exposes the parse tree; ticket 27
  consumes it for `FoldingRange` generation

**Why skip diagnostics**:

- Obsidian treats unknown types as valid (fallback to note rendering).
- Custom callout types via CSS make any "known type" list permanently incomplete.
- No LSP in the ecosystem provides callout diagnostics (Markdown Oxide, rumdl,
  VS Code markdown-ls all skip).
- Malformed syntax (`> [!NOTE` missing bracket): Obsidian treats it as "not a
  callout", not "broken callout"; no tool flags it.

**13 canonical types with aliases**:

note (default), abstract (summary, tldr), info, todo, tip (hint, important),
success (check, done), question (help, faq), warning (caution, attention),
failure (fail, missing), danger (error), bug, example, quote (cite)

### Shared

- Both use spans from `into_offset_iter()` (already available, currently
  discarded).
- Neither needs new redb tables: transient, recomputed per parse.
- Both feed into ticket 27 (structural/editor intelligence) for folding ranges.
- The parser stays pulldown-cmark with full re-parse per edit (matches all
  peers).

### Amended by 25 (2026-10-15)

- **MD066 cession (25 Q9-R)**: rumdl MD066 covers footnote undefined-reference,
  unused-definition, and duplicate-definition. Those three diagnostics are
  **dropped from 17** (docs guidance: users who want them back enable MD066;
  `tags: [Unnecessary]` remains the idiomatic unused-def rendering in docs).
  Footnote **self-reference** and **blockquoted-definition** stay Traces-owned
  (MD066 does not rule them; live probe). The footnote hover idea loses any
  dependency on the ceded diagnostics; hover stays only where Traces still owns
  the rule.

## Amendment from 27 (2026-10-22)

- `CalloutBlock.is_foldable` ("+/- modifier", 17:74) cannot carry Obsidian's
  three-state semantics (no modifier = collapsible/expanded; `-` = starts
  collapsed but still collapsible; `+` = not collapsible). 27's folding mapping
  records "foldable = NOT `+`"; the `-` initially-collapsed state is
  inexpressible on the tower-lsp-server 0.23 lane (no initially-collapsed
  FoldingRange field), so it is dropped server-side as a client-side concern.
  The owner should either widen `is_foldable` to a three-state enum or document
  the bool as exactly "foldable = NOT `+`" ([27 pass 2, Part
  XII.3](../research/27-structural-symbols-editor-intelligence-pass2.md)).
- Related correction: 17's handoff note that ticket 27 consumes `CalloutBlock`
  for FoldingRange generation is superseded; 27's folding E-lite runs on its own
  walk, never the Note AST. `is_foldable` keeps its meaning for 17's own
  consumers; 27's mapping rule (foldable = NOT `+`) is applied on 27's walk.
