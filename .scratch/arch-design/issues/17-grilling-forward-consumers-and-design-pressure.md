# Which future consumers and scale points must the architecture be shaped for?

Type: grilling
Status: resolved
Blocked by: none

## Question

Which forward-looking consumers and scales are real design pressure for this
redesign, and which are speculation to be refused?

Evidence of intent in the repo: `.scratch/md-pkm-lsp/` (an LSP map: live buffer
overlay, spans, cancellation, multi-root), planned MCP frontends
(`survey/cli.md` H1), edit-in-place of notes (task toggling), multi-workspace
daemons (`survey/config-dialog.md` H2). Scale: vault sizes (1k? 20k? 100k
notes), cold-start CLI vs long-lived process, single-user vs multi-root.

Decide:

- (a) is a headless/long-lived host (LSP/MCP) a committed second consumer or
  optional?
- (b) are note mutations (write-back to markdown) in scope, which would require
  spans and a different document model?
- (c) target vault sizes for performance decisions;
- (d) is async required anywhere or does everything stay synchronous with rayon?

Why it is a decision: it determines whether spans, overlays, ambient-context
removal, and a workspace host are foundational investments or speculative
abstractions.

Type: HITL. Grill the user; do not answer for them.

## Answer

### Decision

Make the shared core ready for the LSP as its next major consumer. Prepare
for MCP as a likely near-term consumer. Make it easy to add MCP-specific
seams and multi-workspace machinery. Do not implement MCP or multi-workspace
support speculatively. The shared core owns domain semantics. Consumer seams
own protocol and host-specific behavior.

Support editor features that need source positions: diagnostics, navigation
and completion. General note write-back is not a requirement of this redesign.
Reopen the prior LSP decision to add spans to the shared `Note` AST and reject
a parallel model. Compare a shared model with separate editor and indexing
representations. Ticket 23 decides the exact model and ownership of
[source spans](23-note-document-model.md).

Consider vault sizes of 50, 200, 1,000, 5,000, 10,000, 20,000 and 50,000
notes in performance decisions. Treat them as measurement points. Do not
require equal optimization for every operation at every size. Select
operation-specific ranges and algorithm break-even points from evidence.
Vaults can exceed 50,000 notes.

Keep analysis and domain logic synchronous by default. Put async at host or
transport seams unless a concrete workload shows that the core needs it.

Evaluate proposals by independent changeability, low coupling, fast and
reliable verification, failure isolation and recoverability. These properties
reduce change complexity, lead time, regression risk and recovery costs.
DORA metrics are downstream outcomes, not direct architecture targets.

### Alternatives considered

- **Keep the CLI-only, zero-span design:** rejected because the LSP is the next
  major consumer and needs source-aware editor capabilities.
- **Treat MCP as fully committed and build its multi-workspace host now:**
  rejected; make those additions easy, but wait for concrete MCP requirements.
- **One shared span-aware note model vs separate editor and index models:**
  neither is selected here. Reopen the earlier LSP choice and resolve the
  representation in ticket 23.
- **Require general write-back, or require one universal performance target:**
  rejected; neither is needed to meet the agreed consumer and performance goals.
- **Make the core async by default:** rejected; use a synchronous core and add
  async at host/transport seams only when required.

### Invariants and remaining work

- Shared domain semantics belong to the core; protocol-specific behavior belongs
  to the outward-facing consumer seam.
- Source positions must be available to editor features, but whether they belong
  in one shared model or a separate transient representation remains open for
  ticket 23.
- Index and editor-buffer consistency is now a sharp decision and is tracked in
  [How should open editor buffers relate to the persisted
  index?](34-live-buffer-and-persisted-index-consistency.md).
- Operation-specific performance budgets and measurements remain with the
  relevant index, query, parser, and template decisions, including tickets
  23-28.
- Affected downstream decisions include note modeling (23), index read model
  (24), freshness and invalidation (25), workspace host (30), and error and
  diagnostic architecture (31). Crate topology (32) must account for the
  committed LSP and likely MCP consumers.

### Removable speculative scope

This decision does not justify a general note-editing API, an async core, or
multi-workspace machinery without concrete requirements. It also does not
require every operation to meet one performance threshold at every vault size.
