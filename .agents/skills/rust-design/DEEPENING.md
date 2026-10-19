# Deepening: Seam Transformations & Empirical Trial Loop

---

## 1. Recursive Depth & Seam Mechanics

Deepening concentrates coherent behavior, policy, state, and invariants behind a simpler seam:

> **A module is recursively deep when its internal decomposition concentrates knowledge behind meaningful seams, and those seams make their parent simpler and deeper rather than merely adding layers.**

```text
+------------------------------------------------------------------------------+
| Parent Seam: Unified Domain Concept (Minimal Caller Knowledge Burden)        |
+------------------------------------------------------------------------------+
                                          |
                                          v
+------------------------------------------------------------------------------+
| Deep Internal Child A                    | Deep Internal Child B             |
| - Owns state machine and lifecycle       | - Owns wire encoding and framing  |
| - Hides all transition validation rules  | - Hides ring-buffer memory pool   |
+------------------------------------------------------------------------------+
```

### The Recursive Depth Test

When extracting or reviewing internal child module $C$ within parent $P$:

1. **Local Depth:** Does $C$'s seam provide substantial capability relative to the small interface it exposes to $P$?
2. **Depth Propagation:** Does $P$'s implementation become shorter, simpler, and higher-level? Did internal states, dependencies, and ordering rules vanish from $P$?
3. **The Anti-Layering Rule:** If $P$ still coordinates $C$'s internal sequence, inspects $C$'s internal states, or maps $C$'s errors directly upward, $C$ is a shallow layer, not a recursively deep module.
4. **Sibling Isolation:** When $P$ contains multiple children, ensure their knowledge is independent. Cross-sibling changes indicate misplaced boundaries or unowned shared invariants. Contain lower-level child types and errors below $P$'s seam.

---

## 2. Core Seam Operations

### Consolidate Fragmented Responsibility
Consolidate when callers currently assemble one concern from several shallow pieces.
- **Signals:** Fixed call sequences across call sites, shared invariants spread across helpers, repeated orchestration loops, and intermediate types existing solely to cross shallow seams.
- **Test:** The consolidation must remove knowledge obligations from callers.

### Collapse Shallow Seams
Collapse a seam whose interface costs roughly as much knowledge as the behavior it hides:
- **Signals:** Forwarding wrappers, one-to-one delegation layers, redundant facades, and traits with no meaningful substitution need.
- **Rule:** If the module disappeared, would its complexity reappear in callers, or would the abstraction simply vanish? If it vanishes, delete it.

### Redesign a Real Seam
Keep the responsibility but change the boundary when callers learn the wrong things:
- Raise parameters or results to domain semantics.
- Absorb ordering requirements internally.
- Hide storage, serialization, or network representations.
- Enforce invariants internally via types.
- Narrow visibility (`pub(crate)`, `pub(super)`).

### Relocate Ownership
Move behavior when another module owns the state, invariant, lifecycle, or policy that gives it meaning. Code enforcing an invariant must have direct authority over the state involved.

### Replace Rather than Layer
Replace rather than incrementally repair when the current abstraction preserves the wrong responsibility or seam:
- Identify callers to migrate, behavior to preserve, and tests whose seam changes.
- Account for superseded types, wrappers, and dependencies.

---

## 3. The Deletion Dividend

Every redesign search includes subtraction. Removal is a primary candidate transformation, not deferred cleanup. A deeper module must pay a **Deletion Dividend**:

1. **Superseded Seams:** Delete redundant intermediate abstractions.
2. **Redundant Representations:** Eliminate parallel models or duplicate conversions.
3. **Duplicated Policy:** Consolidate distributed business rules into single owners.
4. **Unused Dependencies:** Remove crates eliminated from `Cargo.toml`.

---

## 4. Empirical Trial-and-Revert Protocol

Architectural refactoring requires rapid prototyping without accumulating accidental technical debt:

1. **Sandboxed State:** Ensure the working tree is clean or create an isolated Git worktree or branch (`improve/<run-id>-<trial-id>`).
2. **Explicit Drop Conditions:** For Breakthrough mode, state the boundary condition that aborts the trial before editing (for example: "If prototype requires unsafe pointer casts or fails to reduce heap allocations by at least 30%, drop it").
3. **Attempt Limits:** Cap iterative compilation attempts to at most 3 refinements per candidate.
4. **Clean Rollback:** If the candidate fails tests, breaks external API stability, or triggers its drop condition, revert immediately with `git reset --hard` or worktree disposal. Do not leave broken scaffolding in the workspace.

---

## 5. Active Graveyard & Convergence Gates

### The Active Architectural Graveyard
The Active Graveyard (`.design/.../graveyard.md`) is an active filter for subsequent discovery, not passive post-run documentation:
- Record every abandoned bet with:
  - **Hypothesis ID:** ID from `ledger.json`.
  - **Concept & Intended Gain:** What architectural transformation was attempted.
  - **Concrete Failure Evidence:** Compiler errors, regression deltas, benchmark timings, or API leakages.
  - **Rejection Reason:** Why the approach was fundamentally flawed or unviable.
- Subsequent scout passes consult `graveyard.md` to avoid proposing variants of known dead ends.

### Search Termination & Convergence Gates
The search loop iterates until one of three stopping conditions is met:
1. **Convergence Gate:** All candidate hypotheses with Expected Architectural Value `EAV >= 1.0` have been evaluated and either committed or cleanly rejected.
2. **Plateau Gate:** Two consecutive trials fail to produce an accepted change. The orchestrator triggers an adversarial 10-lens re-scouting pass. If re-scouting produces no new hypotheses above `EAV >= 1.0`, terminate the search.
3. **Budget Gate:** The allocated run budget (time limit or maximum trial count) is exhausted.
