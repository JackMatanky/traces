# File operations: note rename/move/delete cascading link updates

Type: grilling
Status: open
Blocked by: 13, 26
Assigned: agent

## Question

Once rename mechanics (ticket 26) and the LSP persistence/caching strategy (ticket 13) are settled, decide file-operation handling per `docs/refs/lsp_spec.md`'s Workspace File Operations section (`workspace/willRenameFiles`, `workspace/didRenameFiles`, `workspace/willDeleteFiles`, `workspace/didDeleteFiles`, and the corresponding client capability registration):

- On note rename/move (client-initiated, e.g. via the editor's file explorer): does Traces respond to `willRenameFiles` with a `WorkspaceEdit` that rewrites every inbound wikilink/Markdown-link target across the workspace (using the existing Inlink graph, `src/index/inlinks.rs`, as the starting point for "which files need edits"), applied atomically alongside the rename?
- On delete: does `willDeleteFiles` produce warnings/diagnostics for now-broken inbound links (there's nothing to rewrite them *to*), or a code action offering bulk clean-up?
- Server-capability registration: `didCreateFiles`/`didRenameFiles`/`didDeleteFiles` registration filters (glob patterns) scoped to Markdown files.
- Interaction with the index: does a file-operation-triggered `IndexerService::refresh` need to run before or after computing the cascading `WorkspaceEdit` (ordering matters: the edit must be computed against the *pre-rename* Inlink graph, then applied, then the index refreshed to reflect the new state).

## Inputs from 26 (2026-10-18)

- **Ownership split**: 26 owns server-initiated `RenameFile` for `textDocument/rename` on note targets (edit ordering: old-path edits -> RenameFile -> new-path edits), gated on `resourceOperations`; **28 owns only client-initiated `workspace/will*Files` and the delete path.**
- **Occurrence-level sole-candidate rewrite rule**: cascades consume only single-target resolved occurrence sets; the ambiguous-union path structurally never reaches `EditSet` computation (two separate spine entry points).
- **Comparison-key regime for link-text rewrites**: `match_key` (NFC -> `char::to_lowercase` -> trailing NFC; path keys fold `\` to `/`), **byte-exact-first with fold-fallback**; fold-tier ties refuse rename as `Ambiguous { PathTie }`; original spellings in all output; the file being renamed is excluded from its own ambiguity candidates.
- **Case-only file rename** (`foo.md` -> `Foo.md`) = `RenameFile` with **zero text edits**, offered in prepare iff rename is in `resourceOperations`.
- **Post-rename refresh**: a 40-file rename yields ~40 watched-file events; ticket 13's single-file `RefreshPlan` constructor needs a **batch/multi-file constructor** (seam, not bolted onto 26).
