# Precedent Analysis: rumdl, IWE, Markdown Oxide, and Marksman

This document analyzes precedent from four Language Server Protocol (LSP) tools:
rumdl, IWE, Markdown Oxide, and Marksman. The analysis uses local digests to
evaluate design questions Q1, Q5, and Q6.

## 1. Questions (a) to (e)

### (a) Workspace Start-up and Per-File In-Memory Storage

- **rumdl**:
  - The LSP index worker scans markdown files at start-up [V:
    `docs/digests/lsp_rvben-rumdl-src-digest.txt:36958-36975`].
  - The server indexes files in sequence during a full rescan [V:
    `lsp_rvben-rumdl-src-digest.txt:37031-37050`].
  - It maintains an in-memory `WorkspaceIndex` that maps file paths to
    `FileIndex` [V: `lsp_rvben-rumdl-src-digest.txt:36611,37048`].
  - `FileIndex` stores heading facts (`HeadingIndex`), cross-file links
    (`CrossFileLinkIndex`), and a content hash [V:
    `lsp_rvben-rumdl-src-digest.txt:2067,36846,40572`].
  - rumdl does not store full syntax trees, body spans, footnotes, or callouts
    in `WorkspaceIndex` [V: `lsp_rvben-rumdl-src-digest.txt:40566-40580`].
  - The server holds open editor buffers in memory separately [V:
    `lsp_rvben-rumdl-src-digest.txt:36705`].

- **IWE**:
  - The server parses all workspace files at start-up into memory [V:
    `docs/digests/iwe-org-iwe-src-digest.txt:38638-38648,39276-39299`].
  - `Graph::from_state` builds an in-memory graph of all documents [V:
    `iwe-org-iwe-src-digest.txt:54088-54122`].
  - The graph stores node structures in an arena, node line ranges, full file
    content, frontmatter, and reference edges [V:
    `iwe-org-iwe-src-digest.txt:54343-54360`].
  - It stores line ranges (`Range<usize>`) on graph nodes [V:
    `iwe-org-iwe-src-digest.txt:55177`].
  - IWE does not store heading slugs, footnotes, or callout nodes in the graph
    model [V: `iwe-org-iwe-src-digest.txt:64004-64006`].

- **Markdown Oxide**:
  - The server parses all markdown files in the vault at start-up [V:
    `docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt:1767,2001`].
  - It keeps all references, reference targets (headings, tags, files), and file
    text in memory [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:1673,2643-2664`].
  - It stores LSP `Range` data (`line` and `character`) for references and
    headings using `Rope` [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:2666-2677,5956-5961`].

- **Marksman**:
  - The server parses all folder documents at start-up [V:
    `docs/digests/lsp_artempyanykh-marksman-digest.txt:7461-7501,7648-7666`].
  - It stores a `Doc` record for each document in memory [V:
    `lsp_artempyanykh-marksman-digest.txt:7037-7043,7627-7634`].
  - Each `Doc` retains the concrete syntax tree (`Cst`), abstract syntax tree
    (`Ast`), full text rope, and `Index` [V:
    `lsp_artempyanykh-marksman-digest.txt:7054-7064,8034-8043`].
  - Every node wraps `Node<'A>` containing source text, an LSP `Range`, and
    payload data [V: `lsp_artempyanykh-marksman-digest.txt:6194`].

### (b) References and Backlinks Across Unopened Files

- **rumdl**:
  - Cross-file rules (MD051 for anchors, MD057 for links) query the in-memory
    `WorkspaceIndex` [V:
    `lsp_rvben-rumdl-src-digest.txt:10600-10644,36640-36645`].
  - If a target file is indexed, completion and navigation read cached headings
    from `FileIndex.headings` [V: `lsp_rvben-rumdl-src-digest.txt:35769-35788`].
  - If a target file is missing from the index, the server parses the file from
    disk on demand [V: `lsp_rvben-rumdl-src-digest.txt:35775-35786`].

- **IWE**:
  - The server resolves references and backlinks directly from the in-memory
    graph index [V: `iwe-org-iwe-src-digest.txt:39871-39883`].
  - It returns the stored line ranges (`self.graph.node_line_range(*id)`)
    directly [V: `iwe-org-iwe-src-digest.txt:39886-39902`].
  - The server never reparses unopened files to serve references [V:
    `iwe-org-iwe-src-digest.txt:39853-39906`].

- **Markdown Oxide**:
  - The server queries `vault.select_references_for_referenceable` to collect
    backlinks [V: `lsp_feel-ix-343-markdown-oxide-src-digest.txt:2643-2664`].
  - It returns precomputed line and character ranges stored on link objects [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:2666-2677`].
  - The server never reparses unopened files to serve references [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:2262-2272`].

- **Marksman**:
  - The server queries an in-memory symbol connection graph (`conn: Conn`) [V:
    `lsp_artempyanykh-marksman-digest.txt:7617-7620,7700-7713`].
  - It returns precomputed `Range` coordinates from stored `Node<'A>` elements
    [V: `lsp_artempyanykh-marksman-digest.txt:6194,8034-8043`].
  - The server never reparses unopened files to serve references [V:
    `lsp_artempyanykh-marksman-digest.txt:7675-7713`].

### (c) Modeling of Footnotes, Callouts, and Heading Slugs

- **Footnotes**:
  - rumdl enables footnote parsing in `pulldown-cmark` [V:
    `lsp_rvben-rumdl-src-digest.txt:30343`].
  - It records `FootnoteRef { id, line, byte_offset }` for every footnote
    reference [V: `lsp_rvben-rumdl-src-digest.txt:30348-30354,33667-33675`].
  - Rules MD066, MD067, and MD068 check footnote definitions and usage [V:
    `lsp_rvben-rumdl-src-digest.txt:78922-78924`].
  - IWE does not enable footnotes in `pulldown-cmark` options [V:
    `iwe-org-iwe-src-digest.txt:64004-64006`].
  - Markdown Oxide does not model footnotes [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:5926-5970`].
  - Marksman explicitly skips footnote parsing because the feature is not
    implemented [V:
    `lsp_artempyanykh-marksman-digest.txt:16132-16134,16997-16998`].

- **Callouts and Alerts**:
  - rumdl detects MkDocs admonitions (`!!! note`, `??? notice`) using regex and
    line scanning [V: `lsp_rvben-rumdl-src-digest.txt:96450-96595`].
  - It records `in_admonition` in `LineInfo` to stop `pulldown-cmark` from
    interpreting indented content as code blocks [V:
    `lsp_rvben-rumdl-src-digest.txt:27996-27999,33470-33507`].
  - It preserves admonition blocks through dedicated `Block::Admonition`
    structures [V: `lsp_rvben-rumdl-src-digest.txt:80800-80809`].
  - IWE uses a regex only to count callouts during vault detection [V:
    `iwe-org-iwe-src-digest.txt:60925,61397`].
  - IWE does not store callouts as separate document nodes [V:
    `iwe-org-iwe-src-digest.txt:65880-65892`].
  - Markdown Oxide has no callout model [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:5926-5970`].
  - Marksman has no callout or alert model [V:
    `lsp_artempyanykh-marksman-digest.txt:8034-8043`].

- **Slugs for Headings**:
  - rumdl does not store slugs in its index [V:
    `lsp_rvben-rumdl-src-digest.txt:40572`].
  - It derives anchor slugs on demand using `DocumentAnchors::headings` with
    `OnceLock` per renderer style [V:
    `lsp_rvben-rumdl-src-digest.txt:27110-27131`].
  - It calculates duplicate disambiguation suffixes (`-1`, `-2`) dynamically in
    memory [V: `lsp_rvben-rumdl-src-digest.txt:27121-27126`].
  - IWE does not compute or store heading slugs [V:
    `iwe-org-iwe-src-digest.txt:55237-55240`].
  - Markdown Oxide matches headings by raw text rather than computed slugs [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:2702-2704`].
  - Marksman computes slugs with `Heading.slug` and stores a multimap
    `headingsBySlug` [V: `lsp_artempyanykh-marksman-digest.txt:8037,8059-8060`].

### (d) Startup Parse and Persistent Cache

- **rumdl**:
  - In CLI mode, rumdl processes multiple files in parallel using Rayon [V:
    `lsp_rvben-rumdl-src-digest.txt:1697-1703`].
  - In CLI mode, it persists file lint results to
    `.rumdl_cache/{version}/{hash}.json` [V:
    `lsp_rvben-rumdl-src-digest.txt:327-330,425-445`].
  - It also persists the workspace cross-file index using
    `WorkspaceIndex::save_to_cache` [V:
    `lsp_rvben-rumdl-src-digest.txt:1670-1673,2170-2180`].
  - In LSP mode, the server executes a sequential scan across files in a
    background worker [V: `lsp_rvben-rumdl-src-digest.txt:37031-37050`].
  - The LSP server does not read a persistent disk cache at start-up [V:
    `lsp_rvben-rumdl-src-digest.txt:36958-37012`].

- **IWE**:
  - When the workspace contains 128 or more files, IWE builds documents in
    parallel using Rayon [V: `iwe-org-iwe-src-digest.txt:54103-54116,54409`].
  - When the count is under 128 files, it parses documents in sequence [V:
    `iwe-org-iwe-src-digest.txt:54103-54108`].
  - IWE provides no persistent disk cache [V:
    `iwe-org-iwe-src-digest.txt:38638-38648`].

- **Markdown Oxide**:
  - The server constructs the entire vault synchronously at start-up [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:1767,1895-1898`].
  - It provides no persistent disk cache [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:2001-2004`].

- **Marksman**:
  - The server reads all folder documents sequentially at start-up [V:
    `lsp_artempyanykh-marksman-digest.txt:7461-7501`].
  - It provides no persistent disk cache [V:
    `lsp_artempyanykh-marksman-digest.txt:7648-7666`].

### (e) Bugs, Regressions, and Scaling Complaints

- **Markdown Oxide**:
  - Synchronous diagnostics on each keystroke caused severe server stalls [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:1659-1668`].
  - Edits caused diagnostics tasks to accumulate, filled the message queue,
    blocked stdin, and wedged the server [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:1662-1667`].
  - Low default request concurrency (4 in tower-lsp) caused deadlocks when slow
    requests saturated worker slots [V:
    `lsp_feel-ix-343-markdown-oxide-src-digest.txt:1652-1656`].
  - Any file watcher event triggers a full, blocking disk reparse of the entire
    vault [V: `lsp_feel-ix-343-markdown-oxide-src-digest.txt:1895-1898,2245`].

- **Marksman**:
  - Marksman stores the complete CST, AST, and text in memory. This design
    increases memory use on large vaults [H].

- **rumdl**:
  - Avoids storing syntax trees or document text in `WorkspaceIndex` to limit
    memory growth [V: `lsp_rvben-rumdl-src-digest.txt:40566-40580`].

## 2. Table of Lessons

The table below summarizes the lessons and gives verdicts for Q1, Q5, and Q6.

| Tool | Precedent and Digest Line | Finding | Verdict for Q1, Q5, Q6 |
|---|---|---|---|
| **IWE** | `iwe-org-iwe-src-digest.txt:39871-39906` | Backlinks and references read stored line positions from the in-memory graph. The host never reparses unopened files. | **Supports Q1**. Proves that fast reference navigation needs pre-stored positions rather than on-demand reparse [P]. |
| **Markdown Oxide** | `lsp_feel-ix-343-markdown-oxide-src-digest.txt:2643-2677` | References resolve from stored ranges without file reparsing. | **Supports Q1**. Backlinks across unopened files cannot wait for disk I/O [P]. |
| **Marksman** | `lsp_artempyanykh-marksman-digest.txt:6194,7617-7620` | All nodes store ranges in an in-memory graph. References resolve from memory. | **Supports Q1**. Proves that cross-file queries need in-memory coordinates [P]. |
| **rumdl** | `lsp_rvben-rumdl-src-digest.txt:1670-1673,2170-2180` | Persists the cross-file workspace index to disk cache to accelerate start-up. | **Supports Q1**. Loading pre-indexed data from disk is faster than full reparsing [P]. |
| **rumdl** | `lsp_rvben-rumdl-src-digest.txt:30061-30368` | Single markdown pass extracts links, headings, and footnotes without a second parse. | **Supports Q5**. Second parser pass is unnecessary overhead for documents without math [P]. |
| **IWE** | `iwe-org-iwe-src-digest.txt:64004-64006` | Single pass extracts blocks and inlines. No dual-pass parsing exists. | **Supports Q5**. Confirms that dual-pass parsing is an architectural smell [P]. |
| **rumdl** | `lsp_rvben-rumdl-src-digest.txt:27110-27131` | Headings do not store precomputed slugs. Slugs and numeric suffixes resolve on demand. | **Supports Q6**. Proves that storing slugs inflates records without benefit [P]. |
| **rumdl** | `lsp_rvben-rumdl-src-digest.txt:30348-30354,33667-33675` | Captures `FootnoteRef` with line and byte offset from `pulldown-cmark`. | **Supports Q6**. Proves that storing footnote references enables definition checks and navigation [P]. |
| **Marksman** | `lsp_artempyanykh-marksman-digest.txt:16132-16134` | Omitting footnotes caused broken references and skipped tests. | **Supports Q6**. Confirms that footnote references belong in the core model [P]. |
| **rumdl** | `lsp_rvben-rumdl-src-digest.txt:27996-27999,96450-96595` | Admonition blocks need explicit recognition to stop `pulldown-cmark` from treating contents as code blocks. | **Supports Q6**. Proves that callouts need dedicated parser support [P]. |
| **Markdown Oxide** | `lsp_feel-ix-343-markdown-oxide-src-digest.txt:1659-1668` | Synchronous re-computation of cross-file references on every edit causes message queues to overflow and wedged the server. | **Neutral for Q1**. Warns that in-memory index updates need debouncing [P]. |
| **IWE** | `iwe-org-iwe-src-digest.txt:54103-54116` | Rayon parses files in parallel when file count exceeds 128. | **Supports Q1**. Demonstrates parallel batch processing limits start-up overhead [P]. |

## 3. Not Checked

- Memory consumption of 50,000 files in Marksman on 64-bit Linux.
- Behaviour of rumdl on non-UTF-8 footnote references.
- Precise CPU cost of Regex callout detection in IWE across large repositories.
