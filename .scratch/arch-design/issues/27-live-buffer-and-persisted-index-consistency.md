# How should open editor buffers relate to the persisted index?

Type: grilling
Status: open
Blocked by: 23, 24, 25, 12

## Question

LSP is a committed long-lived consumer, and MCP is a likely consumer. What
source-of-truth and consistency rules should apply when an editor has unsaved
content that differs from the persisted vault and index? Decide which reads use
the buffer, how queries and links observe edits, and how close/save/refresh
transitions reconcile buffered and indexed state.

Resolve after the note document model, index read model, and
freshness/invalidation policies are decided. Keep protocol-specific behavior
outside the shared semantics unless it creates a real variation.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The check
runs on an in-memory parsed buffer without persistence. The index never depends
on schemas.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md). IWE's LSP
has no overlay. `didOpen` and `didChange` write the buffer into the one
in-memory graph (full-document sync). `open_documents` records open keys.
Watcher events for an open key are dropped until `didClose`, when the file is
re-read from disk (`crates/iwes/src/router/server.rs`) [V]. The CHANGELOGs show
why. Last-write-wins let disk changes overwrite unsaved buffers (v0.17.0,
reversed to editor ownership). Rebuilding the BM25 index per keystroke stalled
the server (v0.18.0, now batched after 300 ms idle) [V]. It supports a single
root and no `$/cancelRequest` [V]. [H] Because IWE has no persisted index, this
settles only the in-process view of buffers. It does not test the
persisted-index question of this ticket.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- A host checks the sources within a staleness bound. It does not check before each query. This ticket and ticket 32 set the bound.
- A host holds the store only during a refresh and a persist.
- Each query result names the generation that it read. Use the incarnation and the revision as the generation pair.
- A host that meets a store with a higher format epoch never writes it. It serves from its in-memory generation, or it parses fresh if it has none.
