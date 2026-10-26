# File operations: note rename/move/delete cascading link updates

Type: grilling
Status: open
Blocked by: 13, 26
Assigned: agent

## Question

Settle file-operation handling after ticket 26 (rename mechanics) and ticket 13 (LSP persistence/caching) are settled. Use `docs/refs/lsp_spec.md`, Workspace File Operations section. That section covers `workspace/willRenameFiles`, `workspace/didRenameFiles`, `workspace/willDeleteFiles`, `workspace/didDeleteFiles`, and the matching client capability registration.

Decide these points:

- On note rename or move (client-initiated, for example from the editor's file explorer): does Traces answer `willRenameFiles` with a `WorkspaceEdit`? The edit rewrites every inbound wikilink target and Markdown-link target across the workspace. Use the existing Inlink graph (`src/index/inlinks.rs`) as the starting point for "which files need edits". Apply the edit atomically with the rename.
- On delete: does `willDeleteFiles` produce warnings or diagnostics for now-broken inbound links? There is nothing to rewrite those links *to*. Or does it offer a code action that cleans up in bulk?
- Server-capability registration: register `didCreateFiles`/`didRenameFiles`/`didDeleteFiles` with glob patterns that scope to Markdown files.
- Interaction with the index: decide when a file-operation-triggered `IndexerService::refresh` runs. Compute the cascading `WorkspaceEdit` against the *pre-rename* Inlink graph first. Apply the edit next. Refresh the index last, so it reflects the new state.

## Inputs from 26 (2026-10-18)

- **Ownership split**: 26 owns server-initiated `RenameFile` for `textDocument/rename` on note targets. Edit ordering: old-path edits -> RenameFile -> new-path edits. Gate it on `resourceOperations`. **28 owns only client-initiated `workspace/will*Files` and the delete path.**
- **Occurrence-level sole-candidate rewrite rule**: cascades consume only single-target resolved occurrence sets. The ambiguous-union path never reaches `EditSet` computation, because the spine has two separate entry points.
- **Comparison-key regime for link-text rewrites**: use `match_key` (NFC -> `char::to_lowercase` -> trailing NFC; path keys fold `\` to `/`). Compare byte-exact first, then fall back to fold. Fold-tier ties refuse rename as `Ambiguous { PathTie }`. Keep original spellings in all output. Exclude the file being renamed from its own ambiguity candidates.
- **Case-only file rename** (`foo.md` -> `Foo.md`) = `RenameFile` with **zero text edits**. Offer it in prepare only when rename is in `resourceOperations`.
- **Post-rename refresh**: a 40-file rename yields ~40 watched-file events. Ticket 13's single-file `RefreshPlan` constructor needs a **batch/multi-file constructor**. This is a seam, not something bolted onto 26.
