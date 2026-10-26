# Web precedent: document models, spans, semantic facts and stable ids

This file lists how mature Rust projects separate a position-bearing document
from a position-free summary. I did not open the URLs. Each `[V]` tag is the
claim of the agent that wrote the first version. Check a URL before you quote
it.

## Summary

- [V] rust-analyzer, Ruff, Typst, Taplo, Biome and Oxc separate a lossless or
  position-bearing representation from a span-free or span-light summary. The
  goal is to avoid invalidation after whitespace edits.
- [V] The rust-analyzer `ItemTree` is an "invalidation barrier". An edit inside
  a function body or a whitespace change does not change the `ItemTree`. Salsa
  then stops the recomputation early (`hir_def::item_tree`,
  `https://rust-lang.github.io/rust-analyzer/hir_def/item_tree/index.html`).
- [V] Stable identity does not need byte spans. rust-analyzer uses `FileAstId`
  from `AstIdMap`. Typst uses a numbered span that it renumbers locally
  (`docs/dev/architecture.md`).
- [V] The rustc dev guide says a `Span` is volatile. Incremental queries exclude
  it (`rustc-dev-guide/queries/incremental-compilation-in-detail.html`).
- [V] `serde_spanned::Spanned<T>` and `toml::Spanned<T>` use `Range<usize>`.
  This is 16 bytes on a 64-bit system
  (`docs.rs/serde_spanned/1.1.0/src/serde_spanned/spanned.rs.html:27`).
- [V] Ruff, Biome and Oxc use two `u32` offsets. A `TextRange` or `Span` is 8
  bytes (`docs.rs/ruff_text_size`, `docs.rs/oxc_span`).
- [V] rustc packs a span in 8 bytes: `lo_or_index: u32`, `len: u16`, `ctxt:
  u16`. A side table holds the 16-byte `SpanData` for large spans
  (`doc.rust-lang.org/nightly/nightly-rustc/rustc_span/span_encoding/struct.Span.html`).
- [V] The `jsonschema` crate addresses an error with a logical `instance_path`.
  It maps to a byte span at the reporting edge only
  (`docs.rs/jsonschema/latest/jsonschema/paths/struct.Location.html`).
- [V] Tree-sitter shifts unchanged subtrees with an `InputEdit`
  (`docs.rs/tree-sitter/struct.InputEdit.html`).
- [V] Interning replaces a 24-byte `String` with a 4-byte `u32` symbol (`lasso`,
  `ustr`, rust-analyzer `intern::Symbol`).

## Representation splits

### rust-analyzer: `SyntaxNode`, `ItemTree` and HIR

- Rowan holds a lossless syntax tree. Each node carries a relative length.
  Traversal yields an 8-byte `TextRange`.
- `ItemTree` is an arena summary of the item declarations of one file. It omits
  function bodies. Items have no `TextRange`. A `FileAstId<T>` identifies each
  item.
- An edit in a function body changes the tree but not the `ItemTree`. The early
  cutoff of Salsa then keeps all downstream queries.
- HIR is the resolved semantic form. It lowers bodies on demand.
- Cost: a lowering pass (`lower_module_items`) builds the `ItemTree`. The
  grammar, the AST wrappers and the lowering pass all model the same item facts.
  A grammar change touches all three.

### Ruff

- AST nodes hold an 8-byte `TextRange` (`TextSize` is `u32`).
- `Locator` borrows the source text. A rule asks `locator.slice(node.range())`
  for the text. AST nodes hold no text.
- `Indexer` scans tokens and trivia. It computes comment ranges and `noqa` maps
  outside the AST.
- `LineIndex` keeps newline offsets. It converts a byte offset to a line and a
  column only when the code formats a diagnostic.

### Taplo

- Taplo has a Rowan CST. It builds a DOM with `parse_result.into_dom()`. The DOM
  merges dotted keys and checks duplicate keys. Each DOM node keeps access to
  its `TextRange`.
- Taplo reparses on a text change. It does not persist the DOM across sessions.

### Typst

- A `Span` is a `NonZeroU64` (8 bytes). It holds a file id and a `SpanNumber`.
- `Source::edit` replaces bytes and reparses only the changed block.
- New nodes get local span numbers. Unchanged blocks keep their numbers. The
  memoized work of `comemo` stays valid.

## Answers to the five questions

### (a) One type for both

- [V] rustc: early IRs held spans. Each keystroke that shifted offsets changed
  the hash and caused a large rebuild.
- [V] IWE stores raw text for each key beside an arena tree of block nodes
  (`crates/liwe/src/graph.rs`). The writer re-synthesises markdown with
  `pulldown-cmark-to-cmark`. This led to text-loss regressions. The ticket 23
  input on IWE gives the same facts.
- [V] A single tree for indexing and editing stores spans, trivia and layout.
  rust-analyzer issues #20917 and #21302 describe memory use. rust-analyzer
  evicts raw `SyntaxNode` values with an LRU and keeps the `ItemTree` in Salsa.

### (b) A position-free summary

- [V] Cost of the `ItemTree`: a lowering pass, and the same invariants in two
  places.
- [V] Benefit: whitespace, comment and body edits do not invalidate crate-level
  indexing.

### (c) Persistence

- [V] Salsa keeps memoized results in memory. If the lowered summary is equal,
  the propagation stops.
- [V] A disk store of spanned structures goes stale: one edit at line 1 moves
  every stored byte range of the file.
- [V] Postcard is dense. A format change needs a schema version
  (`postcard-schema`, `redb/CHANGELOG.md`).
- [V] A store that holds derived facts (tags, link targets, frontmatter values)
  stays stable when it omits byte spans.

### (d) Stable ids across edits

- [V] `AstIdMap::from_source` walks the tree in preorder. It stores each item in
  an `Arena<SyntaxNodePtr>`. `FileAstId<T>` wraps a `u32` index. The id stays
  the same while items are not added, removed or reordered
  (`docs.rs/ra_ap_span/0.0.270/src/ra_ap_span/ast_id.rs.html`).
- [V] Tree-sitter uses an `InputEdit` to adjust positions.

### (e) Interning

- [V] `intern::Symbol` in rust-analyzer is a 4-byte handle into a global table.
- [V] `lasso::Spur` is a `u32` key. `ThreadedRodeo` supports threads. It can
  freeze into a read-only resolver.
- [V] `ustr::Ustr` is an 8-byte pointer to a leaked string.

### (f) Span sizes

| Type | Bytes | Source |
|------|-------|--------|
| `Range<usize>` (`serde_spanned`, `toml::Spanned`) | 16 | agent |
| `TextRange` (`rowan`, `ruff_text_size`, `biome_rowan`) | 8 | agent |
| `oxc_span::Span` | 8 | agent |
| `rustc_span::Span` | 8 (packed) | agent |
| `typst_syntax::Span` | 8 (`NonZeroU64`) | agent |

- A `u32` offset caps a document at 4 GiB.

## Candidates and precedents

The agent numbers the candidates 1 to 5. This differs from the letters A to D in
`research/20-prep/h-arch-design-note-split-review.md` and from the bullets in
the ticket.

| Candidate | Precedent | Cost found in the precedent |
|-----------|-----------|-----------------------------|
| 1. Shared span-aware `Note` | Ruff AST (`Ranged`) | The agent says saving it to redb means a high deserialization cost and cache invalidation on every edit. |
| 2. Span-free `Note` and `NoteDocument` | rust-analyzer (`ItemTree` and `SyntaxNode`) | A lowering step. Heading, tag and link facts exist in two types. Paragraph edits do not invalidate the `Note`. |
| 3. `NoteFacts` and `NoteDocument` | rustc (`HirId` and `DefMap` against the AST) | A small index record. Editor checks need both types or the index facts. |
| 4. Span-free `Note` and on-demand reparse | Taplo, rumdl | The simplest type system. The cost is a reparse on an editor request. |
| 5. Lossless tree | Rowan (rust-analyzer, Biome, Taplo) | Memory per node. +59% parse time for the Noyalib CST. Ticket 17 sets no general write-back. |

## Lessons for ticket 23

- C1: [P] Precedent supports a position-free summary for a persistent index.
  Candidates 2 and 4 match rust-analyzer and Taplo. If a reparse is fast,
  candidate 4 has the fewest duplicated invariants.
- C2: [P] Use two `u32` offsets (8 bytes), as `ruff_text_size` does. Compute
  `LineIndex` on demand. Do not store line and column on nodes.
- C4: [P] Do not use a byte range as a durable id. Use a traversal index, as
  `FileAstId` does.
- C7: [P] Interning a small string to a `u32` symbol or a `char` removes
  allocations. Research 11 says this shrinks `ListItem` from 160 B to about 128
  B. The probe confirms 160 B (`g-probe-results.md`). I did not check 128 B.
- C12: [P] The 97% decode cost (research 11) is the main argument for a lean
  record.
- C13: [P] The agent says a full reparse of one note takes less than 1 ms. The
  probe supports this for a mean 11 KB note: 217 µs for the full parse. A
  10,000-line note is not tested (`g-probe-results.md`).

## Not checked

- Whether the offset iterator is fast enough for on-demand reparse of a note
  over 10,000 lines.
- The wire size of a spanned `Note` on a 50,000-note vault. The probe gives
  +4.3% on 2,711 notes.
- Whether a line scan of the frontmatter is faster than a spanned YAML parser.
  The probe says yes (`g-probe-results.md`).
- All URLs in this file.

## Inconsistencies found while rewriting

- The agent tags every claim [V]. It cites URLs that I did not open. It also
  tags conclusions, such as "precedents strongly refute Candidate 1", as [V]. A
  conclusion is a proposal. I tagged the lessons [P].
- The stale-offset argument applies to a cache that compares data across edits
  (Salsa). This index rewrites the row of a changed file. The argument against
  persisted spans is weaker for this index than for Salsa. [H] Check if
  `patch_and_diff` or `merge_refreshed_notes` compares whole `Note` values.
  `Note` derives `PartialEq`.
- The numbering of the candidates differs in three places (this file, the `h`
  file and the ticket). Candidates 2 and 3 are almost the same design.
- The file says rumdl "implements `didChange` with full and incremental sync".
  `c1-digests-lsp.md` found no evidence for rumdl in the digest.
  `e3-web-spans-positions-perf.md` says rumdl uses full sync.
- The first version cites
  `.scratch/arch-design/issues/23-note-document-model.md:176-179`. These lines
  belong to an earlier version of the ticket.
- The statement "an edit at line 1 invalidates every stored range" is true. It
  does not show that a per-file row costs more than a per-file reparse.
