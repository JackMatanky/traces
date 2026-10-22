# Client capability negotiation & graceful degradation

Type: grilling
Blocked by: 09

## Question

Once the framework/runtime (ticket 09) is chosen, decide the capability-negotiation strategy per `docs/refs/lsp_spec.md`'s `initialize`/`ClientCapabilities`/`ServerCapabilities` sections:

- Which server capabilities are unconditionally registered (statically, at `initialize` response time) vs dynamically registered later (`client/registerCapability`) based on what the client actually advertises supporting.
- Minimum viable client: what happens for a client with a sparse `ClientCapabilities` (e.g. no `workspace/didChangeWatchedFiles` support, no pull-diagnostics, no `positionEncoding` negotiation) — does every feature degrade gracefully, or are some features simply unavailable and the server reports that clearly.
- `positionEncoding` negotiation (LSP 3.17+, ties to ticket 11's UTF-8/UTF-16 decision): request `utf-8` when the client advertises support, falling back to `utf-16` otherwise.
- Version/capability floor: does Traces target LSP 3.17 baseline features only, or does it opportunistically use 3.18 features (per `docs/refs/lsp_spec.md`'s "What's new in 3.18") where advertised, decide the policy once, not per-feature ad hoc.

## Inputs from 25 (2026-10-15)

- **Capability fields to advertise** (diagnostics domain, from 25's transport decision): `textDocument.diagnostic` (pull, Phase A document-level; workspace-level with 30), `workspace.diagnostics.refresh_support` (server-initiated `workspace/diagnostic/refresh`, unconditional with swallowed error), and pull `data` support (LSP 3.18 `data` field for resultId-carrying items; 27/28 will pass action payloads through it).
- **Push gate depends on pull-capability detection at initialize (25 Q2-R)**: push is disabled when the client pulls (rumdl/ty pattern); publishing non-empty to a pull-capable client clobbers pull. 29 owns the detection seam (initialize result inspection) that the diagnostics module's push gate reads. Empty-publish-once invalidation still runs on gate transitions.
- Hint severity rendering note (23's A1): whether the client's Problems panel lists Hint-level diagnostics is client-side and unverified (carried since 25 §6.2 T3); 29's minimum-viable-client matrix should record Hint visibility as a client property, not a server requirement.

## Inputs from 26 (2026-10-18)

- **Capability fields 26 consumes** (advertise/derive accordingly): `hover.contentFormat` (Markdown iff offered, else stripped PlainText of the same body), `prepareSupport` (gates `RenameOptions { prepare_provider: true }`; plain `true` otherwise), `general.positionEncodings` (utf-8-if-offered-else-utf-16, never utf-32), `workspace.workspaceEdit.documentChanges` + `resourceOperations` + `failureHandling` (two-tier serializer: `documentChanges` iff advertised **OR** ops present; resource ops offered in prepare only when genuinely applicable; changeAnnotations skipped v1; `metadata.isRefactoring` emitted regardless).
- **Deliberate non-advertisement**: do NOT advertise `partialResultProvider` for references until the `partialResultToken` streaming path lands (26 records the upgrade path, does not use it). iwe's advertised-but-stubbed inline-values and identity-resolve-without-extra-data are the anti-pattern this discipline exists to avoid (amended 2026-10-21 by iwe/fileclass research, [tool-iwe](../research/tool-iwe.md) Part 6).
- Rename may exceed 100ms and emits workDone `$/progress` under `concurrency_level(1)` (26 amends 12 on this point).

## Inputs from 27 (2026-10-22)

- Capability fields for all nine 27 methods (documentSymbol, workspaceSymbol, foldingRange, selectionRange, documentLink, codeLens, codeAction, semanticTokens, documentHighlight), all static initialize-time gates on 27's flat `[lsp]` config table. Deliberate NON-advertisement list: `workspaceSymbol/resolve`, documentSymbol partialResult, `semanticTokens/full.delta`, `semanticTokens/range`, documentLink/resolve (resolveProvider:false), codeLens/resolve (absent in Wave 1; advertised in the Wave 2 heading-lens stage). Refresh gating: each of `semanticTokens/refreshSupport` and `codeLens/refreshSupport` gates its server-to-client refresh request (no folding refresh is sent; Zed lacks it). 27 uses no dynamic registration except the file watchers 29 owns. Runtime-toggle semantics for 31: initialize-time gates decide advertisement; runtime flips make handlers return empty results or method_not_found, sticky-until-restart per 22's reset_engine precedent.
