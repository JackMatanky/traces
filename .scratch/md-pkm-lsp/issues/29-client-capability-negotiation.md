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
