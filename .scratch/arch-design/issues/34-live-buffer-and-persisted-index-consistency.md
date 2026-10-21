# How should open editor buffers relate to the persisted index?

Type: grilling
Status: open
Blocked by: 23, 24, 25, 12

## Question

For committed long-lived LSP and likely MCP consumers, what source-of-truth and
consistency rules should apply when an editor has unsaved content that differs
from the persisted vault and index? Decide which reads use the buffer, how
queries and links observe edits, and how close/save/refresh transitions
reconcile buffered and indexed state.

Resolve after the note document model, index read model, and
freshness/invalidation policies are decided. Keep protocol-specific behavior
outside the shared semantics unless it creates a real variation.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The check
runs on an in-memory parsed buffer without persistence; the index never
depends on schemas.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md). IWE's LSP
has no overlay: `didOpen` and `didChange` write the buffer into the one
in-memory graph (full-document sync), `open_documents` records open keys, and
watcher events for an open key are dropped until `didClose`, when the file is
re-read from disk (`crates/iwes/src/router/server.rs`) [V]. The CHANGELOGs show
why: last-write-wins let disk changes overwrite unsaved buffers (v0.17.0,
reversed to editor ownership) and rebuilding the BM25 index per keystroke
stalled the server (v0.18.0, now batched after 300 ms idle) [V]. It supports a
single root and no `$/cancelRequest` [V]. [H] Because IWE has no persisted
index, this settles only the in-process view of buffers; the persisted-index
question of this ticket is not tested by it.
