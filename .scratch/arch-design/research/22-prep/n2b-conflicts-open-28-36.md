# Decision Conflicts and Open Issues (28-36)

This document maps decisions Q1-Q15 to open tickets 28-36, analyzing conflicts and alternatives.

## Ticket Analysis Tables

### Ticket 28: Template Engine Boundary

| Decision | Ticket Text (quote + line) | Class | Consequence | Handoff Wording |
| :--- | :--- | :--- | :--- | :--- |
| `TemplateService` vs `&Config` | "Decide between typed host objects and stringified values at the minijinja boundary" (line 11) | Soft Conflict | `TemplateService` holding `&Config` violates consumer-owned policy. | Decouple `TemplateService` from `&Config` via narrowed injection. |

### Ticket 29: Path and Capability Model

| Decision | Ticket Text (quote + line) | Class | Consequence | Handoff Wording |
| :--- | :--- | :--- | :--- | :--- |
| Confinement | "Decide one owner for the invariant 'all file access stays inside the project root':" (line 9) | Gap | Unowned path identity and TOCTOU stance. | Establish single `ProjectRoot` handle; centralize validation. |

### Ticket 30: Workspace Host and Composition

| Decision | Ticket Text (quote + line) | Class | Consequence | Handoff Wording |
| :--- | :--- | :--- | :--- | :--- |
| Composition | "Where does service composition live... and what is the CLI's remit?" (line 9) | Hard Conflict | CLI owning composition prevents crate split. | Extract `WorkspaceHost`; CLI only handles IO/parsing. |

### Ticket 31: Error and Diagnostic Architecture

| Decision | Ticket Text (quote + line) | Class | Consequence | Handoff Wording |
| :--- | :--- | :--- | :--- | :--- |
| Error Owns | "Decide whether libraries own structured diagnostics (code, spans, help), or the CLI flattens them." (line 11) | Soft Conflict | `src/cli/error.rs` bloating; library errors lose structure. | Implement `ToDiagnostic` for library types; flatten only at CLI layer. |

### Ticket 32: Crate Topology and Public Surface

| Decision | Ticket Text (quote + line) | Class | Consequence | Handoff Wording |
| :--- | :--- | :--- | :--- | :--- |
| Split | "Decide the crate topology and the visibility story." (line 9) | Gap | Visibility baseline bloat; backdoor testing. | Split CLI crate; narrow visibility; enforce DAG. |

### Ticket 33: Reconciliation

| Decision | Ticket Text (quote + line) | Class | Consequence | Handoff Wording |
| :--- | :--- | :--- | :--- | :--- |
| Audit | "Reconcile all resolved decisions codebase-wide" (line 9) | Fine | Potential drift in implementation. | Re-run drift checks; confirm specification consistency. |

### Ticket 34: Live Buffer Consistency

| Decision | Ticket Text (quote + line) | Class | Consequence | Handoff Wording |
| :--- | :--- | :--- | :--- | :--- |
| Overlay | "what source-of-truth and consistency rules should apply when an editor has unsaved content that differs from the persisted vault and index?" (line 9-11) | Soft Conflict | Stale reads from persisted vault vs unsaved buffer. | Implement full-document sync for LSP; use in-memory overlay. |

### Ticket 35: Schema Internals Simplification

| Decision | Ticket Text (quote + line) | Class | Consequence | Handoff Wording |
| :--- | :--- | :--- | :--- | :--- |
| Simplification | "Decide which of the current internals still earn their cost" (line 17) | Gap | 12k lines of schema bloat; excessive cloning. | Collapse field-type enums; replace/simplify inheritance graph. |

### Ticket 36: Body Section Schemas

| Decision | Ticket Text (quote + line) | Class | Consequence | Handoff Wording |
| :--- | :--- | :--- | :--- | :--- |
| Body Schema | "Decide whether schemas should also describe the body." (line 10) | Gap | Inconsistent note structure in body sections. | Implement ordered, non-backtracking section matcher. |

## Alternative Architectures for HostInputs/ConfigService

### Alternative 1: CLI-Heavy Composition

The CLI crate holds `ConfigService` and `HostInputs` composition.

- **Pros**: Simple integration, mimics current behavior.
- **Cons**: CLI is bloated; Core cannot be used without CLI-owned configuration logic; complicates crate splitting (ticket 32).

### Alternative 2: Core-Hosted Service (Recommended)

Core (or a new `ServiceHost` crate) owns `ConfigService` and `HostInputs` composition.

- **Pros**: Clean separation of concerns; CLI is thin, only argument parsing and output presentation; Core/Core-lib can be reused for MCP/LSP/other consumers.
- **Cons**: Requires extracting composition logic and formalizing the boundary; slightly more complex initial wiring.

## Summary

This audit of tickets 28-36 reveals several architectural gaps and conflicts, notably regarding crate topology (32), composition ownership (30), and path confinement (29). Implementing the Core-Hosted Service architecture (Alternative 2) is critical to satisfying the crate-split requirements and reducing CLI bloat. All decisions must be reconciled against the specification, ensuring ADR-0004 and the visibility rule are upheld.

Path: `.scratch/arch-design/research/22-prep/n2b-conflicts-open-28-36.md`
