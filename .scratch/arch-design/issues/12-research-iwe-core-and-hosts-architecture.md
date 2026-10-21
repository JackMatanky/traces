# How does IWE structure one core shared by a CLI, an LSP, an MCP server and a daemon?

Type: research
Status: resolved
Blocked by: none

## Question

IWE (`iwe-org/iwe`, Rust, workspace of `liwe` core, `iwe` CLI, `iwes` LSP,
`iwec` MCP, `diwe` daemon) is the closest architectural analogue found for the
destination: a markdown-vault core consumed by thin CLI, LSP and MCP hosts. It
was only read for `crates/iwe/docs/schema.md` in round 4 of the ticket 20
prep; its architecture, document model and host seams were never studied. This
ticket closes that gap for the core/host side. The query language, schema,
frontmatter and MCP-write-gating side is ticket 13.

Study (from the digests, see Deliverable): crate boundaries and what each
crate may depend on; what `liwe` exposes publicly versus keeps private; the
document model (`model/document.rs`, `model/tree.rs`, `graph.rs`,
`graph/arena.rs`, `graph/index.rs`, `model/key_index.rs`, `model/inline.rs`,
`model/node_pointer.rs`, `model/projector.rs`, spans/positions); Keys and
cross-references (`docs/keys.md`); how the in-memory graph is built, updated
incrementally and kept consistent with files (`diwe` loader/watcher/fs,
`iwes` base_path and open-buffer handling); how the LSP router/server share
state with the core and handle unsaved buffers; how the MCP crate and the CLI
call the same use cases (the "app layer"); config model and lifecycle
(`model/config.rs`, `docs/configuration.md`); markdown/djot readers and writers
(lossless round-trip? normalisation?); operations (`liwe/src/operations/*`:
extract, inline, rename, attach, squash); error and diagnostic types; test and
benchmark strategy (`docs/benchmark.md`, `crates/*/tests`, `benches`);
CHANGELOGs as evidence of design reversals and regrets.

Focus, against decisions already taken or still open in this map:

- Layering principle of [ticket 20](./20-schema-note-typing-and-validation.md)
  (cli, lsp, mcp are thin hosts of `core`; app layer of use cases over domain
  modules; no one-implementor traits): does IWE confirm, contradict or refine
  it? Where do use cases live and are they functions or a held context?
- [Note document model](./23-note-document-model.md): spans, facts, items,
  children of note versus separate entities, lossless versus lossy, graph versus
  tree, text store separate from structure.
- [Index read model](./24-index-read-model-and-query-execution.md) and
  [freshness](./25-index-freshness-and-invalidation.md): in-memory graph
  rebuilt per process versus persisted cache; how IWE decides staleness.
- [Live buffers](./34-live-buffer-and-persisted-index-consistency.md): how the
  LSP overlays unsaved text on the graph; cancellation; multi-root.
- [Workspace host](./30-workspace-host-and-composition.md),
  [diagnostics](./31-error-and-diagnostic-architecture.md),
  [crate topology](./32-crate-topology-and-public-surface.md): what the CLI
  owns versus the core; public surface; error types; test seams.
- Scale: vault sizes IWE benchmarks and what it pays at each.

Informs: tickets 23, 24, 25, 30, 31, 32, 34 and the reconciliation ticket 33.

Deliverable: `.scratch/arch-design/research/12-iwe-core-and-hosts-architecture.md`
(<= 300 lines). Source: `docs/digests/iwe-org-iwe-*digest.txt`. Strategy: do not
read the 3 MB digests linearly. A pre-split per-file tree of the main digest
exists at `/tmp/iwe/` (recreate with the splitter described in the research
brief if missing); read docs first, then the named source files by symbol;
cite repo path plus symbol or file-relative line for every claim. Structure: (1)
what IWE actually does (verified from source), (2) invariants and owners, (3)
tradeoffs and costs it pays, (4) transferable lessons, (5) lessons that do NOT
transfer and why, (6) what this changes or challenges in the named Traces
decisions. Mark facts [V], interpretation [H]. Do not propose the final
decision.

## Answer

Deliverable: [research 12](../research/12-iwe-core-and-hosts-architecture.md),
read from the IWE digests (five crates, 455 files) and spot-checked against the
sources by the owner. Findings:

- IWE confirms the layering principle of ticket 20: pure `liwe` operations
  return a plain `Changes { creates, updates, removes }`; `diwe` applies them;
  cli, lsp and mcp are thin hosts; no one-implementor trait crosses the seam.
  It differs in one respect: the app layer is its own library crate (`diwe`),
  which is input to ticket 32.
- The LSP has no buffer overlay: open buffers are written into the single
  in-memory graph, open keys ignore watcher events until close. It works
  because IWE has no persisted index; it does not answer ticket 34's
  persisted-index question. Single root, no cancellation, full sync.
- IWE has no persistent cache (20k notes: 705 ms cold load, M3 Pro, synthetic).
  Cost model for the keep-as-is comparison in tickets 24 and 25, not a verdict.
- The tree writer is lossy and its CHANGELOGs record repeated regressions and
  reversals; any Traces writer must splice spans into original bytes (ticket
  23) and never rewrite unchanged bytes (ticket 25).
- Corrections made by the owner to the agent's draft: the "Traces must keep
  redb" and "perfect consistency" claims were softened (they are decisions for
  tickets 24 and 34); the 30,000 notes/s figure is 28,000.

Alternatives, invariants and removable code: not applicable to a research
ticket. Unresolved: whether the app layer is a separate crate (ticket 32).
Downstream: tickets 22, 23, 24, 25, 29, 30, 31, 32, 34, 33 carry an "Inputs"
section.
