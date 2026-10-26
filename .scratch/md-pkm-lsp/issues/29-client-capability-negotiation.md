# Client capability negotiation & graceful degradation

Type: grilling
Status: open
Blocked by: 09
Assigned: agent

## Question

Once ticket 09 fixes the framework and runtime, decide the capability-negotiation strategy. Use `docs/refs/lsp_spec.md`: its `initialize`, `ClientCapabilities`, and `ServerCapabilities` sections.

Decide these points:

- Decide which server capabilities register unconditionally at `initialize` response time. Decide which register later via `client/registerCapability`, based on what the client advertises.
- Decide the minimum viable client behavior. A client may send sparse `ClientCapabilities`: no `workspace/didChangeWatchedFiles` support, no pull-diagnostics, no `positionEncoding` negotiation. Make every feature degrade gracefully, or mark some features unavailable and report that clearly.
- `positionEncoding` negotiation (LSP 3.17+, ties to ticket 11's UTF-8/UTF-16 decision): request `utf-8` when the client advertises support. Fall back to `utf-16` otherwise.
- Set the version and capability floor once. Decide whether Traces targets LSP 3.17 baseline features only, or uses 3.18 features where the client advertises them (see `docs/refs/lsp_spec.md`, "What's new in 3.18"). Do not decide per feature.

## Inputs from 25 (2026-10-15)

- **Capability fields to advertise** (diagnostics domain, from 25's transport decision): `textDocument.diagnostic` (pull; Phase A document-level, workspace-level with 30), `workspace.diagnostics.refresh_support` (server-initiated `workspace/diagnostic/refresh`, unconditional with swallowed error), and pull `data` support (the LSP 3.18 `data` field for resultId-carrying items; 27/28 pass action payloads through it).
- **The push gate depends on pull-capability detection at initialize (25 Q2-R)**: disable push when the client pulls (rumdl/ty pattern). Publishing non-empty diagnostics to a pull-capable client clobbers pull. 29 owns the detection seam (initialize result inspection); the diagnostics push gate reads it. Empty-publish-once invalidation still runs on gate transitions.
- Hint severity rendering note (23's A1): whether a client's Problems panel lists Hint-level diagnostics is a client-side property. Nobody has checked it (carried since 25 §6.2 T3). 29's minimum-viable-client matrix records Hint visibility as a client property, not something the server must provide.

## Inputs from 26 (2026-10-18)

- **Capability fields 26 consumes** (advertise or derive them accordingly): `hover.contentFormat` (Markdown when offered, else stripped PlainText of the same body); `prepareSupport` (gates `RenameOptions { prepare_provider: true }`, plain `true` otherwise); `general.positionEncodings` (utf-8 if offered, else utf-16, never utf-32); and `workspace.workspaceEdit.documentChanges` + `resourceOperations` + `failureHandling` (two-tier serializer: `documentChanges` when advertised **or** ops present; offer resource ops in prepare only when genuinely applicable; skip changeAnnotations in v1; emit `metadata.isRefactoring` in all cases).
- **Deliberate non-advertisement**: do NOT advertise `partialResultProvider` for references until the `partialResultToken` streaming path lands. 26 records that upgrade path but does not use it. iwe's advertised-but-stubbed inline-values, and identity-resolve-without-extra-data, are the anti-pattern this discipline avoids (amended 2026-10-21 by iwe/fileclass research, [tool-iwe](../research/tool-iwe.md) Part 6).
- Rename may exceed 100ms and emits workDone `$/progress` under `concurrency_level(1)` (26 amends 12 on this point).

## Inputs from 27 (2026-10-22)

- Advertise capability fields for all nine 27 methods (documentSymbol, workspaceSymbol, foldingRange, selectionRange, documentLink, codeLens, codeAction, semanticTokens, documentHighlight). All are static initialize-time gates on 27's flat `[lsp]` config table.
- Deliberate NON-advertisement list: `workspaceSymbol/resolve`, documentSymbol partialResult, `semanticTokens/full.delta`, `semanticTokens/range`, documentLink/resolve (resolveProvider:false), and codeLens/resolve (absent in Wave 1; advertised in the Wave 2 heading-lens stage).
- Refresh gating: each of `semanticTokens/refreshSupport` and `codeLens/refreshSupport` gates its server-to-client refresh request. Send no folding refresh; Zed lacks it. 27 uses no dynamic registration except the file watchers that 29 owns.
- Runtime-toggle semantics for 31: initialize-time gates decide advertisement. At runtime, flips make handlers return empty results or method_not_found, sticky-until-restart per 22's reset_engine precedent.
