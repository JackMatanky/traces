# LSP and Rust PKM digests: note models

This file summarises how LSP tools model a parsed note. The first version had no
citations. I checked the sync-mode claims in the digests. The other claims keep
the tag [H], because I did not check them.

## Summary

- [V] IWE asks for full-document sync: `change:
  Some(TextDocumentSyncKind::FULL)`
  (`docs/digests/iwe-org-iwe-src-digest.txt:38755`).
- [V] Marksman reads a sync kind from its settings. It supports `Full` and
  `Incremental`
  (`docs/digests/lsp_artempyanykh-marksman-digest.txt:11123-11130`).
- [H] The agent says rumdl uses full sync. The rumdl source digest has no match
  for `TextDocumentSyncKind`. I found no evidence.
- [H] The agent says most tools split the parsed index from an editor document.
  The agent gives no citation for each tool.
- [H] The agent says high-performance tools do not persist spans in their index.
  They derive spans when the document loads.
- [H] The agent says an LSP converts byte offsets to UTF-16 columns at the
  protocol edge.
- [H] The agent says frontmatter key spans are rare. Tools scan for the `---`
  delimiters and parse YAML only when needed.

## Comparison table (the agent's claims)

| Tool | Shared or split | Position unit | Persisted spans | Language |
|------|-----------------|---------------|-----------------|----------|
| IWE | split: graph index and document | byte and LSP UTF-16 | no | Rust |
| Marksman | split: CST and index | line and column | no | F# |
| Markdown Oxide | split: vault and metadata | byte (Ropey) | no | Rust |
| rumdl | index worker and document | byte and LSP | no | Rust |
| zk | minimal and full | LSP position | no | Go |

## Findings (the agent's claims)

- C1. IWE keeps a graph index of facts and a `Document`. Markdown Oxide has a
  `Vault` with `NoteInMemory`. Marksman has a `Cst` type, an `Index` for
  headings and links, and a `Doc` with a `Text` (rope) and a `Structure`.
- C2. The LSP uses UTF-16 code units. Rust tools keep byte offsets and convert
  when they answer a request.
- C3. A YAML CST costs a lot of parse time. The 59% figure for Noyalib comes
  from ticket 18, not from these digests.
- C13. A `didChange` event updates an in-memory buffer. The tool then indexes
  the buffer again.

## Lessons for ticket 23

- C1: [P] Check each claim in the digest before you use it as precedent. The
  table says "split". The Marksman finding says one `Doc` holds the text and the
  structure.
- C2, C12: [P] Compute spans on demand from the parse. Do not store them. The
  probe measures +4.3% bytes for persisted spans (`g-probe-results.md`). The
  size is not the strong argument against persisting them. Staleness is.
- C13: [P] Full-document sync plus a reparse is the common pattern. IWE shows
  this (`iwe-org-iwe-src-digest.txt:38755`).

## Not checked

- The memory cost of spans at 50,000 notes.
- The `pulldown-cmark` against `jotdown` comparison.
- Each "split" claim and each position-unit claim in the table.
- The rope claim for Markdown Oxide and Marksman.

## Inconsistencies found while rewriting

- The first version says "Persist byte-spans, convert to UTF-16" in the C2 row.
  The same row recommends "compute spans on demand". The summary says tools do
  not persist spans. The three statements conflict. I kept the on-demand
  statement, because the summary supports it.
- The first version says the Marksman model is "Split (CST + Index)". The
  detailed finding says the `Doc` struct holds the text and the structure. This
  is one document type that holds both.
- The first version says tools use `tower-lsp` types. The repository has no LSP
  dependency (`Cargo.toml`).
- The first version attributes the 59% figure to "successful tools". It is the
  Noyalib CST measurement of ticket 18.
- `e3-web-spans-positions-perf.md` says Marksman uses full sync. The digest
  shows a setting for both kinds.
- The first version gives no citation for any tool claim.
