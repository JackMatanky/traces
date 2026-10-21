# How should open editor buffers relate to the persisted index?

Type: grilling
Status: open
Blocked by: 19, 20, 21

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

See [ticket 16's Answer](./16-schema-note-typing-and-validation.md). The check
runs on an in-memory parsed buffer without persistence; the index never
depends on schemas.
