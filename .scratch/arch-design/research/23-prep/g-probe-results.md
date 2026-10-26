# Probe results: parse cost, record size and span cost

This file records measurements for the evidence gate of ticket 23. All numbers
are [V] for the corpus and machine named below. They are not a vault benchmark.

## Summary

- Spans are not the size problem. Spans on headings, items and links add 4.3% to
  the serialized `Note` bytes.
- List item text is 93% of the serialized `ListItem` bytes. `ListItem` records
  are 78% of the serialized `Note` bytes.
- The two `pulldown-cmark` passes take 21% each of the full parse time. The full
  parse also does other work.
- Only 1.18% of the notes have a `$` in a heading line. A guard on heading lines
  can skip the second pass for 98.8% of the notes. [P]
- A line scan finds top-level frontmatter keys in 0.5% of the full parse time.
- Noyalib `Spanned<T>` makes a frontmatter parse 2.8x to 3.3x slower. It gives
  value spans only. Its end offsets include trailing whitespace.
- Decoding a stored `Note` runs at about 1 ns for each byte. This is 3% of the
  parse time.

## Method

- Probe code: a throwaway crate in `/tmp/traces-probe`. It is outside the
  repository. It depends on `traces` with the `test-utils` feature.
- Corpus: all `*.md` files in the repository, except `target`, `.git`,
  `node_modules`, `.design` and `digests`. This is 2,711 files and 29,153 KiB.
  The mean file size is 11,012 B.
- The corpus holds documentation and tickets. It is not a personal vault. It has
  long lists and code samples.
- Machine: macOS, arm64, `cargo build --release`. No other job used the CPU
  heavily during the timing runs [H].
- Timing: the best of 5 trials. Each trial runs the pass 10 times over the whole
  corpus. For frontmatter timing, each trial runs 20 or 50 times.
- `parse_markdown` is the public function. The raw passes use `pulldown-cmark`
  0.13.4 with the same options as `src/note/parser.rs:68-73`.

## Sizes of types

`std::mem::size_of` in the release build:

| Type | Bytes |
|------|-------|
| `Note` | 232 |
| `ListItem` | 160 |
| `TaskListItem` | 72 |
| `Heading` | 32 |
| `NoteFieldValue` | 80 |

`src/index/entry.rs:230` says `Note` is 240 bytes. The measured value is 232
bytes.

## Parse time

Time for the whole corpus:

| Step | Time | Share of full parse |
|------|------|---------------------|
| Full `parse_markdown` | 589.2 ms | 100% |
| Raw pass 1 (`MARKDOWN_OPTIONS`) | 122.0 ms | 21% |
| Raw pass 2 (`HEADING_OPTIONS`) | 123.2 ms | 21% |
| Raw pass 2 on files without `$` | 63.2 ms | 11% |
| Scan of each file for `$` | 1.05 ms | 0.2% |

- The two raw passes use 42% of the full parse time. The event handler, the
  lexer and the field scan use the other 58%.
- 1,942 of 2,711 files (72%) have no `$`. This corpus has many shell and math
  samples. A personal vault can have fewer.
- Heading-like source lines: 33,464. Lines with `$`: 79 (0.24%).
- Notes with a `$` in at least one heading line: 32 of 2,711 (1.18%).

Meaning for the second pass:

- The second pass uses `ENABLE_MATH`. The first pass does not. The test
  `Status:: $draft$ #work` needs the first pass to keep `$draft$` as text
  (`src/note/parser.rs:772-785`).
- The test `# $x^2$` needs an empty heading text (`parser.rs:725-748`).
- [P] Collect the heading text in pass 1. Use pass 2 only for a note that has a
  `$` in a heading line. A heading without `$` has no math, so both passes give
  the same text. Prove this with a differential test over the corpus.
- [P] The saving is at most 21% of the parse time on this corpus.

## Record size

Postcard bytes for the whole corpus: 15,889,119.

| Part | Bytes | Share |
|------|-------|-------|
| `lists` | 12,338,021 | 77.7% |
| `outlinks` | 2,244,265 | 14.1% |
| `headings` | 895,496 | 5.6% |
| `frontmatter` | 80,778 | 0.5% |
| `tags` | 4,822 | 0.03% |
| path, `inline_fields`, framing | 325,737 | 2.1% |

Counts: 31,433 headings, 76,728 list items, 3,785 tasks, 32,088 outlinks, 205
tags and 205 notes with frontmatter.

List items:

- The mean serialized `ListItem` is 161 B. The mean raw text is 144 B.
- Text (raw text plus the clean text when it differs) is 93% of the item bytes.
- The clean text differs from the raw text on 803 items (415,296 B).
- `ListText` stores the clean text only when it differs
  (`src/note/lists.rs:496-502`).

Cost of spans:

- Proxy: one span (start and end) for each heading, list item and link. Headings
  and items use the line start and end. Links use the file midpoint. The proxy
  computes the postcard varint length exactly. It does not compute the real
  offsets.
- 140,249 nodes add 677,374 bytes. This is +4.3% of 15,889,119 bytes.

Decoding:

- Decoding all stored notes takes 16.6 ms. This is 1.05 ns for each byte.
- This is 3% of the full parse time. One note takes 6.1 µs to decode and 217 µs
  to parse on average.

## Frontmatter spans

- 175 files start with `---`. The parser reports frontmatter on 205 notes. The
  probe does not explain the difference [H].
- Line scan for top-level keys over all files: 0.163 ms. On the 175 frontmatter
  files it takes 0.077 ms. The full parse of the same files takes 16.275 ms. The
  scan costs 0.5% of the parse time.
- The line scan finds `key:` lines at column 0. It does not parse YAML. It can
  miss quoted keys, complex keys and flow mappings [H].

Noyalib 0.0.53 with `HashMap<String, Value>` and `HashMap<String,
Spanned<Value>>`:

| Input | Plain | Spanned | Change |
|-------|-------|---------|--------|
| 175 real blocks and 200 synthetic blocks | 9.2 µs | 25.8 µs | +181% |
| 200 typical synthetic blocks | 7.5 µs | 24.5 µs | +225% |

- This is not the exact code path of `src/note/metadata.rs`. The result shows
  the order of the cost [H].
- `Spanned<T>` gives the span of a value, not of a key.
- The end offset includes trailing text. The value `Meeting notes` has the span
  7..21. The text ends at 20. The list element `work` has the span 10..17. The
  text ends at 14. The end offset is the start of the next token.
- A diagnostic that underlines a value needs a trim step [H].

## Not measured

- The cost of an on-demand reparse of one editor buffer (ticket 34).
- A prototype of a span-bearing `Note` in the parser. The size numbers use a
  proxy.
- Key spans from Noyalib.
- The memory use of notes in RAM for each candidate design.
- A repeat run on a personal vault.
