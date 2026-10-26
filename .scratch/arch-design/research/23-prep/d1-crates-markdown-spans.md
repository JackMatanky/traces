# Markdown parser facts: `pulldown-cmark` 0.13.4

I replaced the first version. It was a placeholder with no content. This version
covers `pulldown-cmark` only. The repository uses this crate (`Cargo.toml:136`).

## Summary

- [V] `Parser::into_offset_iter` yields `(Event, Range<usize>)` for each event
  (`pulldown-cmark-0.13.4/src/parse.rs:1434,2174-2175`). Byte offsets are free.
- [V] `src/note/parser.rs:108-111` already uses the offset iterator. It passes
  each range to `handle_event` as a `ByteSpan`. The `Note` stores no span.
- [V] The crate emits `Event::InlineMath` and `Event::DisplayMath` only with
  `Options::ENABLE_MATH` (`src/lib.rs:716-718`).
- [V] The first pass uses `ENABLE_YAML_STYLE_METADATA_BLOCKS`,
  `ENABLE_WIKILINKS` and `ENABLE_FOOTNOTES`. The second pass adds `ENABLE_MATH`
  (`src/note/parser.rs:68-73`). Math is the only difference.
- [V] The crate gives a heading `id`, `classes` and `attrs` only with
  `ENABLE_HEADING_ATTRIBUTES` (`src/lib.rs:163`). The crate does not make a
  slug.
- [V] The crate emits footnote events with `ENABLE_FOOTNOTES`:
  `Tag::FootnoteDefinition` and `Event::FootnoteReference`
  (`src/lib.rs:211-214,573-577`). The first pass already has this option.
- [V] `ENABLE_GFM` turns on GFM alert blockquotes: `[!NOTE]`, `[!TIP]`,
  `[!IMPORTANT]`, `[!WARNING]` and `[!CAUTION]` (`src/lib.rs:719-722`). [H]
  Obsidian callouts have more kinds. Check if a parser option fits callouts
  before ticket 23 models them.

## Why the repository has two passes

- [V] The tests pin two behaviours:
  - `# $x^2$` gives an empty heading text (`src/note/parser.rs:725-748`).
  - `Status:: $draft$ #work` keeps `$draft$` in the field value
    (`src/note/parser.rs:772-785`).
- [H] A single pass with `ENABLE_MATH` changes the second behaviour. The parser
  then emits the math span as a math event, not as text. I did not run this
  case.
- [H] A single pass without `ENABLE_MATH` changes the first behaviour. The
  heading text then contains the `$` characters. I did not run this case.
- [P] Collect the heading text in pass 1. Run pass 2 only for a note that has a
  `$` in a heading line. A heading with no `$` cannot contain math. See
  `g-probe-results.md` for the measured effect.
- [H] A differential test proves the parity. It compares both methods on the
  corpus and on generated headings.

## Sketch: spans from the offset iterator [P]

- A heading span is the range of `Event::Start(Tag::Heading { .. })`. The range
  covers the whole heading, including the marker.
- The extent of a heading section runs to the start of the next heading with a
  level equal to or lower than the current level, or to the end of the source. A
  pass over the heading list computes this. No new parse is needed.
- A list item span is the range of `Event::Start(Tag::Item)`.
- A link span is the range of `Event::Start(Tag::Link { .. })`. A wikilink has a
  link type for wikilinks. Check the exact `LinkType` value before you rely on
  it.

## Not checked

- `comrak`, `markdown` (markdown-rs), `rowan`, `text-size`, `line-index`,
  `lsp-types` and `ropey`. No file in this directory checks them.
- The memory cost of the crate per event.
- The UTF-16 mapping for LSP columns. `e3-web-spans-positions-perf.md` covers
  this from web sources.
- The `LinkType` variants for wikilinks.

## Inconsistencies found while rewriting

- The first version said single-pass parsing "is possible if we customize `Tag`
  processing". The crate has no such hook. The real issue is the `ENABLE_MATH`
  option.
- The first version listed `comrak` and `text-size` with no analysis.
- `src/note/parser.rs:77-78` says the second pass enables "math and footnote
  parsing". The first pass already enables footnotes. Only math differs.
