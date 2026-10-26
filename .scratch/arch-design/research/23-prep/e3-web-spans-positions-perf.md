# Web precedent: spans, positions and performance

This file covers LSP positions, span size, persistence of spans, LSP sync mode,
frontmatter spans and string interning. I did not open the URLs. Each `[V]` tag
is the claim of the agent that wrote the first version, unless I say that I
checked it. `g-probe-results.md` has my measurements.

## Summary

1. [V] LSP 3.17 added `general.positionEncodings`. UTF-16 is the baseline. A
   server must fall back to UTF-16 when the client does not offer UTF-8
   (<https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification>).
   The agent says Neovim and Helix support UTF-8 and VS Code uses UTF-16.
2. [V] The rust-analyzer `line-index` crate keeps sorted line starts and a side
   table for lines with wide characters
   (<https://rust-lang.github.io/rust-analyzer/src/line_index/lib.rs.html>).
   Ruff has an ASCII fast path (`ruff_source_file::line_index.rs:49-67`).
3. [V] `into_offset_iter` yields `(Event, Range<usize>)`. The agent says it adds
   no heap allocation and little CPU time.
4. [V] A `TextRange` of two `u32` values is 8 bytes. `Range<usize>` is 16 bytes.
   The 4 GiB limit is normal in rust-analyzer and Ruff
   (<https://github.com/astral-sh/ruff/issues/5630>).
5. [V] IWE sets `TextDocumentSyncKind::FULL`. I checked this in the digest
   (`docs/digests/iwe-org-iwe-src-digest.txt:38755`). Marksman reads the sync
   kind from a setting and supports both kinds
   (`docs/digests/lsp_artempyanykh-marksman-digest.txt:11123-11130`). I found no
   evidence for rumdl and markdown-oxide.
6. [V] The agent says rust-analyzer, ripgrep and Tantivy do not persist spans.
   They recompute them. Source for rust-analyzer:
   <https://github.com/rust-lang/rust-analyzer/issues/4712>.
7. [V] The probe measures the raw `pulldown-cmark` pass at 245 MB/s on this
   corpus. The full `parse_markdown` runs at 51 MB/s (`g-probe-results.md`).
8. [V] The agent cites `rust_serialization_benchmark`: `rkyv` gives 1.2 ns
   unvalidated access and 10 µs read, with 1,011 KB for the `log` data set.
   `postcard` gives 724 KB and 2.33 ms. `bitcode` gives 1.46 ms and has no
   format stability across major versions.
9. [H] A YAML CST costs 50% to 60% more parse time. The Noyalib figure is +59%
   from ticket 18. The probe measures `Spanned<Value>` at +181% to +225%. It
   measures a line scan at 0.5% of the parse time (`g-probe-results.md`).
10. [V] The agent says Tree-sitter incremental parsing for markdown suffers from
    global effects, such as reference links and unmatched emphasis.

## LSP positions and the line index

- [V] Before LSP 3.17, `Position.character` counts UTF-16 code units.
- [V] rust-analyzer `LineIndex` holds `newlines: Box<[TextSize]>`,
  `line_wide_chars: IntMap<u32, Box<[WideChar]>>` and `len`. A binary search
  (`partition_point`) gives the line. Only lines with non-ASCII characters have
  a side-table entry. The agent says a lookup takes less than 50 ns.
- [V] Ruff `LineIndex` has `IndexKind::Ascii` and `IndexKind::Utf8`. When a file
  has no non-ASCII byte, the column is the byte distance.
- [P] Build one `LineIndex` for each document change. Convert a byte span to an
  LSP `Range` only when the server formats a response.
- [V] The repository already has a `LineIndex` that gives a line from a byte
  position (`src/position.rs:27-33`). [H] It has no column method. I read only
  its doc comment (`src/position.rs:27-31`), not the whole `impl`.

## Span size

| Type | Bytes | Source |
|------|-------|--------|
| `Range<usize>` | 16 | std |
| `TextRange` (two `u32`) | 8 | `ruff_text_size`, `rowan` |

- [V] The repository already has `BytePos(u32)` and `ByteSpan`
  (`src/position.rs:171-190`). The `u32` limit is 4 GiB.
- [H] The agent says a PKM note is rarely larger than a few megabytes.

## Persisted spans

- [V] ripgrep recomputes match spans from the text. Tantivy stores terms and
  positions but computes highlights from the stored text. rust-analyzer never
  writes syntax or spans to disk.
- [H] For this index, a persisted span adds bytes to every note blob. The probe
  measures +4.3% bytes for spans on headings, items and links. A decode runs at
  about 1 ns for each byte. The added cost is small. A transient span still
  avoids a format change.

## Serialization formats

- `postcard` (current): compact, Serde-compatible. Research 11 gives 18.5 ms for
  5,000 notes.
- `rkyv`: zero-copy access. The payload is about 40% larger on the benchmark
  (1,011 KB against 724 KB). A struct change needs a migration.
- `bitcode`: fast Serde decoding (1.46 ms against 2.33 ms for `postcard`). It
  promises no format stability across major versions.
- [P] Keep `postcard` with an explicit format version. Consider `rkyv` only if
  note decoding stays the bottleneck.

## LSP sync mode

| Server | Sync mode | Source |
|--------|-----------|--------|
| Marksman | `Full` or `Incremental`, from a setting [V] | `docs/digests/lsp_artempyanykh-marksman-digest.txt:11123-11130` |
| IWE | `FULL` [V] | `docs/digests/iwe-org-iwe-src-digest.txt:38755` |
| rumdl | `Full` (agent) | not found in the digest |
| Markdown Oxide | `Full` (agent) | not checked |

## Reparse cost

Numbers from research 11, as the agent quotes them (Apple Silicon M4 Pro, raw
`pulldown-cmark`):

| Note size | Time |
|-----------|------|
| 1 KB | 3.2 µs |
| 5 KB | 25.6 µs |
| 10 KB | about 50 µs |
| 50 KB | 36.8 µs (fast path) to about 250 µs |

- [V] The probe measures the full `parse_markdown` at 217 µs for a mean 11 KB
  note. This is about 4 to 5 times the raw parse time. A reparse of one editor
  buffer still takes much less than 1 ms for a typical note.
- [H] The agent says a keystroke budget is 50 to 100 ms, and a 60 fps frame is
  16.6 ms.

## Frontmatter spans

- [V] The agent says the Noyalib CST adds +59% (5.33 µs for `noyalib::from_str`,
  3.75 µs for the AST conversion of 10 fields, more than 14 µs with the CST).
  The source is ticket 18 and research 11.
- [V] `marked-yaml` builds an AST with spans on `yaml-rust2` and `hashlink`
  (`crates.io/crates/marked-yaml`).
- [H] The agent says `yaml-language-server` uses the `eemeli/yaml` CST for key
  and value ranges (<https://github.com/redhat-developer/yaml-language-server>).
- [P] Parse values with the fast parser. For an LSP request that needs a key
  span, scan the frontmatter slice for `^([a-zA-Z0-9_-]+):` and for `- (.*)`.
  The probe measures the scan at 0.5% of the parse time.

## String interning

- [V] `SmolStr` is 24 bytes and stores up to 23 bytes inline. rust-analyzer
  wraps it in `Name(SmolStr)` (<https://github.com/rust-analyzer/smol_str>).
- [H] The agent says 10,000 tasks allocate 10,000 `String` values for the status
  and cost 234 KB or more. A `String` header is 24 bytes. 10,000 headers are
  240,000 bytes (234 KiB). This is the size of the headers, not of a heap
  allocation.
- [V] `TaskStatus` holds `symbol`, `name: String` and `kind`
  (`src/task.rs:50-55`).
- [P] Option 1: store the `char` symbol (`TaskStatusSymbol`) or a one-byte
  discriminant, and look up the name in `TaskStatusMap`. Option 2: use `SmolStr`
  or a `u32` symbol for tags.

## Lessons for ticket 23

- C1: [P] Keep an index `Note` without spans. Build an editor view from the
  buffer on demand.
- C2: [P] Use two `u32` offsets. Use a `LineIndex`. The repository already has
  `ByteSpan`, `BytePos` and `LineIndex`.
- C2, C12: [P] Spans in the store are not a size problem on the probe corpus
  (+4.3%). Choose by staleness and by format changes.
- C3: [P] Do not use a YAML CST on the indexing path. Scan for key spans on
  demand.
- C7: [P] Remove the owned `String` from `TaskStatus`. The probe confirms that
  `ListItem` is 160 B. I did not check the 128 B figure.
- C12: [P] Keep `postcard` and add a format version.
- C13: [P] Use full-document sync with a reparse. IWE does this.
- C16: [P] Raw parsing runs at 245 MB/s on the probe corpus. The full parse runs
  at 51 MB/s. Decoding runs at about 1 GB/s.

## What to measure

Status after the probe (`g-probe-results.md`):

1. `into_offset_iter` against the plain `Parser`: not measured.
2. `TextRange` (8 B) against `Range<usize>` (16 B): not measured. A proxy gives
   the varint size of postcard.
3. `LineIndex` build time: not measured.
4. Frontmatter line scan against the YAML parser: measured.
5. Task status enum against an owned `String`: not measured.

## Not checked

- The memory of a `LineIndex` for 1,000 open buffers.
- Line scan with emoji in frontmatter tags. The UTF-16 offset may differ.
- All URLs in this file.

## Inconsistencies found while rewriting

- The first version says all of Marksman, IWE and rumdl report `Full`. The
  digest shows a setting for Marksman. It shows no evidence for rumdl.
- The first version says "full sync ... is universally adopted by all major
  markdown LSP servers". The Marksman setting contradicts "all".
- The first version quotes research 11 for a 50 KB note: "36.8 µs (fast path) to
  about 250 µs". 36.8 µs for 50 KB is 1.4 GB/s. The same list gives 195 MB/s for
  5 KB. The numbers do not fit one throughput. [H] The 36.8 µs value is for
  another case.
- The first version says the parse time is "less than 50 µs for typical notes".
  This is the raw `pulldown-cmark` time. The full parse of a mean 11 KB note
  takes 217 µs.
- The first version says "Persisting spans ... directly exacerbates the 97%
  bottleneck". The probe measures +4.3% bytes. The effect exists but is small.
- The first version says the Noyalib CST was "investigated in ticket 23". The
  source is ticket 18.
- The first version says `ListItem` balloons to 160 bytes because of the
  `String` status. A `String` is 24 bytes inside the item. The 160 bytes come
  from the whole layout. The probe measures `TaskListItem` as 72 bytes.
- The first version says "Spans MUST NOT be persisted". This is a proposal, not
  a fact.
