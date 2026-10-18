# Which future consumers and scale points must the architecture be shaped for?

Type: grilling
Status: open
Blocked by: none

## Question
Which forward-looking consumers and scales are real design pressure for this redesign, and which are speculation to be refused?

Evidence of intent in the repo: `.scratch/md-pkm-lsp/` (an LSP map: live buffer overlay, spans, cancellation, multi-root), planned MCP frontends (`survey/cli.md` H1), edit-in-place of notes (task toggling), multi-workspace daemons (`survey/config-dialog.md` H2). Scale: vault sizes (1k? 20k? 100k notes), cold-start CLI vs long-lived process, single-user vs multi-root.

Decide: (a) is a headless/long-lived host (LSP/MCP) a committed second consumer or optional? (b) are note mutations (write-back to markdown) in scope, which would require spans and a different document model? (c) target vault sizes for performance decisions; (d) is async required anywhere or does everything stay synchronous with rayon?

Why it is a decision: it determines whether spans, overlays, ambient-context removal, and a workspace host are foundational investments or speculative abstractions.

Type: HITL. Grill the user; do not answer for them.
