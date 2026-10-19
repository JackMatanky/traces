# Which future consumers and scale points must the architecture be shaped for?

Type: grilling
Status: claimed
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

The redesign must make the shared core ready for the LSP as the next major
consumer and for MCP as a likely near-term consumer. It should make it easy to
add MCP-specific seams and multi-workspace machinery, but does not require
implementing MCP or multi-workspace support speculatively. The shared core owns
domain semantics; protocol and host-specific behavior belongs at consumer seams.

Editor-facing behavior must support source-aware features such as diagnostics,
navigation, and completion. General note write-back is not a requirement of this
redesign. The prior LSP decision to add spans to the shared `Note` AST and
reject a parallel model is reopened: the design must compare a shared model
against separate editor and indexing representations. The exact model and
ownership of source spans belong to [What is the note document model: spans,
facts, items, multi-value metadata?](19-note-document-model.md).

Performance decisions must consider vault sizes of 50, 200, 1,000, 5,000,
10,000, 20,000, and 50,000 notes. These are important measurement points, not a
blanket requirement that every operation be optimized equally at every size.
Select operation-specific ranges and algorithm break-even points based on
evidence; vaults can exceed 50,000 notes.

Keep analysis and domain logic synchronous by default. Async belongs at host or
transport seams unless a concrete workload shows that the core needs it.

Evaluate architectural proposals by independent changeability, low coupling,
fast and reliable verification, failure isolation, and recoverability. These
should reduce change complexity, lead time, regression risk, and recovery costs;
DORA metrics are downstream outcomes, not direct architecture targets.

### Alternatives considered

- **Keep the CLI-only, zero-span design:** rejected because the LSP is the next
  major consumer and needs source-aware editor capabilities.
- **Treat MCP as fully committed and build its multi-workspace host now:**
  rejected; make those additions easy, but wait for concrete MCP requirements.
- **One shared span-aware note model vs separate editor and index models:**
  neither is selected here. Reopen the earlier LSP choice and resolve the
  representation in ticket 19.
- **Require general write-back, or require one universal performance target:**
  rejected; neither is needed to meet the agreed consumer and performance goals.
- **Make the core async by default:** rejected; use a synchronous core and add
  async at host/transport seams only when required.

### Invariants and remaining work

- Shared domain semantics belong to the core; protocol-specific behavior belongs
  to the outward-facing consumer seam.
- Source positions must be available to editor features, but whether they belong
  in one shared model or a separate transient representation remains open for
  ticket 19.
- Index and editor-buffer consistency is now a sharp decision and is tracked in
  [How should open editor buffers relate to the persisted
  index?](30-live-buffer-and-persisted-index-consistency.md).
- Operation-specific performance budgets and measurements remain with the
  relevant index, query, parser, and template decisions, including tickets
  19-24.
- Affected downstream decisions include note modeling (19), index read model
  (20), freshness and invalidation (21), workspace host (26), and error and
  diagnostic architecture (27). Crate topology (28) must account for the
  committed LSP and likely MCP consumers.

### Removable speculative scope

This decision does not justify a general note-editing API, an async core, or
multi-workspace machinery without concrete requirements. It also does not
require every operation to meet one performance threshold at every vault size.

Status: resolved
