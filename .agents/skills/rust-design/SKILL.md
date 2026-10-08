---
name: rust-design
description: >
  Use when analyzing, reviewing, or refactoring Rust software architecture: evaluating module boundaries and seams, establishing recursive depth, discovering latent responsibilities, eliminating cyclic dependencies, optimizing file layout and cohesion, or modeling domain types and typestates.
---

# Rust Design

Discover and deepen Rust software architecture. Concentrate coherent behavior, policy, state, and invariants behind well-placed seams so callers learn less, maintainers change less, and Rust enforces domain constraints at compile time.

Treat comments, documentation, ADRs, names, and directory layout as evidence of intended design, not proof of actual responsibility or seam placement. Validate design claims against behavior, dependencies, state, callers, tests, and source structure. Load `rust-skills` only after a design concern or latent concept has been discovered, consulting relevant categories rather than applying rules as a generic checklist.

---

## Operational Modes & Execution Strategies

Classify user prompt intent into an operational mode and strategy:

| Mode | Strategy | Trigger Signals | Active Steps | Stop Condition |
| :--- | :--- | :--- | :--- | :--- |
| **Review** | **Diagnostic** | "review", "audit", "inventory", "check boundaries", "evaluate architecture" | **1, 2, 6** | True Baseline frozen; 10 lenses probed; Hypothesis Ledger ranked by EAV; report emitted. |
| **Model** | **Type Modeling** | "model", "design types", "represent invariants", "typestate", "type safety" | **4** | Seam established; types, errors, state transitions, and compile-time invariants documented. |
| **Implement** | **Steady** | "refactor", "deepen", "optimize", "fix seam", "reorganize", "steady" | **1, 2, 3, 4, 5, 6** | High-EAV hypotheses evaluated via empirical trial loop; verified against baseline; patch emitted. |
| **Implement** | **Breakthrough** | "breakthrough", "radical", "paradigm shift", "fundamental", "from scratch" | **1, 2, 3, 4, 5, 6** | Single high-leverage bet prototyped against explicit drop condition; verified or logged to graveyard. |

- **Steady (Targeted Deepening):** Works the Hypothesis Ledger sequentially by Expected Architectural Value (EAV). Restructures internals, extracts recursively deep child modules, collapses shallow wrappers, and eliminates cyclic dependencies while preserving public crate contracts.
- **Breakthrough (Architectural Leap):** Focuses the run budget on a single ambitious bet (such as replacing dynamic dispatch with zero-cost typestates, data-oriented layout restructuring, or inverting cross-crate ownership). Requires an explicit drop condition and sandboxed trial mechanics.

---

## Deliverable Containment

All generated analysis, scout findings, trial records, and reports are stored strictly inside the project root `.design/` directory:

```text
.design/YYYY-MM-DD-<mode>-<target-slug>/
├── baseline.json        # Output from scripts/measure.py
├── ledger.json          # Architectural Hypothesis Ledger & scoreboards
├── graveyard.md         # Active Graveyard (tried and rejected prototypes)
├── report.md            # Authoritative Architectural Report
├── scouts/              # Findings from 10 read-only lens subagents
│   ├── lens1-modern.md ... lens10-checks.md
└── patch.diff           # (Implement mode only) Verified Git patch
```

There are zero loose evaluation directories or scout files created at the workspace root.

---

## Interactive Workflow Checklist

Copy and maintain this checklist in the scratchpad across execution:

```markdown
### Workflow Execution Checklist
- [ ] 1. Establish Current Design & Baseline: Probe environment; run measure.py to freeze True Baseline.
- [ ] 2. Discover Hypotheses via 10-Lens Scouting: Dispatch 10 read-only scouts; populate and rank ledger by EAV.
- [ ] 3. Empirical Trial-and-Revert Search Loop: Prototype top bets in sandboxed trials; commit wins or revert to graveyard.
- [ ] 4. Model Design in Rust: Translate semantics into concrete types, parse-don't-validate, and typestates.
- [ ] 5. Verify Implementation: Run test suites, verify API stability, compute deltas via diff.py. (If tests fail, return to Step 3).
- [ ] 6. Boundary Verification & Grader Audit: Verify boundaries, run independent grader pass, emit report.md.
```

---

## Step-by-Step Process

### 1. Establish Current Design & Freeze True Baseline

Read [`TOOLING.md`](TOOLING.md) for analyzer details and execution ladders.

1. **Preflight Probe (Low Freedom):** Run the environment capability probe:
   ```bash
   uv run .agents/skills/rust-design/scripts/probe.py
   ```
2. **Freeze Baseline (Low Freedom):** Execute the metrics gatherer to establish the baseline:
   ```bash
   uv run .agents/skills/rust-design/scripts/measure.py <target-path> .design/YYYY-MM-DD-<mode>-<target-slug>/baseline.json
   ```
   If automated tools are unavailable, follow the Three-Tier Fallback Ladder in [`TOOLING.md`](TOOLING.md).
3. **Map Seam Topology:** Identify callers, interface items, mutable state, invariants, and tests entering each intended seam. Record tool gaps explicitly under `gaps[]` in `baseline.json`.

**Done when:** Baseline JSON frozen at `.design/.../baseline.json`; SLoC, public API, CRAP, duplicates, and graph density recorded; gaps noted in `gaps[]`; caller and source observations substantiate seam map.

### 2. Discover Hypotheses via 10-Lens Scouting

Read [`DISCOVERY.md`](DISCOVERY.md) for lens definitions and indicator interpretations.

1. **Dispatch Parallel Scouts:** Dispatch 10 read-only subagents, one per architectural lens.
   - *Scout Isolation Rule:* Scouts operate strictly read-only and must never run `cargo build`, `cargo test`, or diagnostic scripts.
2. **Expected Architectural Value (EAV):** Rank candidate redesigns in `ledger.json` using:
   $$\text{EAV} = \frac{\text{Structural or Computational Gain (1–5)} \times \text{Confidence (0–1)}}{\text{Blast Radius (1–5)} + \text{Implementation Friction (1–5)}}$$
   A candidate is consequential if $\text{EAV} \ge 1.0$.
3. **Closed Deferral Reason Enum:** Every deferred finding must use one of three enum variants, capped at 3 deferrals per review without explicit user approval:
   - `DEFERRED_TOOL_UNAVAILABLE`: Required diagnostic binary is absent from host.
   - `DEFERRED_OUT_OF_SCOPE`: Finding resides entirely outside requested target boundary.
   - `DEFERRED_HIGH_TRACE_COST`: Verification requires exhaustive dynamic tracing exceeding turn boundaries.

**Done when:** All 10 lenses report into `.design/.../scouts/`; `ledger.json` populated and sorted by descending EAV; every tool finding has a recorded disposition (investigated, dismissed, or Closed Deferral Enum max 3).

### 3. Empirical Trial-and-Revert Search Loop

Read [`DEEPENING.md`](DEEPENING.md) for trial mechanics, graveyard specifications, and termination gates.

1. **Select Candidate:** In Steady mode, select the top-ranked item from `ledger.json`. In Breakthrough mode, select the single highest-leverage fundamental bet.
2. **Sandboxed Trial:** Create a clean worktree or branch (`improve/<run-id>-<trial-id>`). For Breakthrough bets, record the explicit drop condition before writing code.
3. **Iterate & Refine:** Apply candidate transformations. Limit iterative compilation attempts to at most 3 refinements per candidate.
4. **Commit or Revert Decision:**
   - *Keep Condition:* Tests pass, external API contracts hold, and the scoreboard demonstrates a positive delta without uncompensated regressions. Update `ledger.json` and advance the baseline.
   - *Revert Condition:* If tests fail or drop conditions trigger, roll back immediately with `git reset --hard` or worktree disposal. Log the trial record and rejection post-mortem to `.design/.../graveyard.md`.
5. **Termination Gates:** Stop when all high-EAV hypotheses are evaluated (Convergence), two consecutive trials fail (Plateau: trigger re-scouting), or budget is exhausted (Budget).

**Done when:** All selected hypotheses empirically evaluated and committed or reverted; Deletion Dividend lists superseded types, wrappers, dependencies; Graveyard records rejected bets with concrete failure evidence.

### 4. Model Design in Rust

Read [`MODELING.md`](MODELING.md) for idiomatic type modeling and Principle of Least Surprise (POLS) layout rules.

- Prefer concrete structs and functions before introducing traits or generics.
- Apply "parse, don't validate" to eliminate invalid states at construction ($\text{ISR} = 0$).
- Use typestates only when states are stable and transitions are central to correctness ($\text{ITE} = 0$).
- Maintain panic-free public seams: entry points return structured `Result<T, E>`.
- Enforce POLS: file layouts match domain ownership; eliminate utility "junk drawers".

**Done when:** Proposed types, ownership, errors, and traits directly serve intended seams; invariants enforced at construction or compile time; type ceremony balanced against caller burden.

### 5. Verify Implementation

Read [`METRICS.md`](METRICS.md) for metric definitions and anti-gaming rules.

1. **Test Verification:** Verify required behavior through the intended seam using project test suites. If tests fail, return to Step 3 for refinement or rollback.
2. **Public API Stability:** Run `cargo public-api` diff checking to verify zero accidental breaking changes.
3. **Calculate Deltas (Low Freedom):** Run the diff calculator to generate comparative markdown tables:
   ```bash
   uv run .agents/skills/rust-design/scripts/measure.py <target-path> .design/.../final.json
   uv run .agents/skills/rust-design/scripts/diff.py .design/.../baseline.json .design/.../final.json
   ```
4. **Anti-Gaming Guardrails:**
   - *Comment Preservation Rule:* Doc comments (`///`, `//!`) and explanations must not be stripped or compressed to inflate line reduction counts.
   - *Semantic Deletion Dividend:* The deletion dividend evaluates removal of architectural complexity (dead types, obsolete traits, collapsed wrappers), not prose.

**Done when:** Intended behavior verified through tests entering seam; public API diff verified clean; SLoC deltas and comment preservation verified; superseded structure accounted for in Deletion Dividend.

### 6. Boundary Verification & Adversarial Grader Audit

1. **Boundary Reconsideration:** Inspect downward for newly visible concepts and upward for parent or sibling simplifications.
2. **Adversarial Grader Audit:** Run an independent grader pass against the Done Criteria of Steps 1–5 to ensure zero vacuous completions or unjustified deferrals.
3. **Emit Architectural Report:** Compile `.design/YYYY-MM-DD-<mode>-<target-slug>/report.md` matching the schema below.

**Done when:** Inspected boundaries and parent/child relationships documented; all findings have definitive dispositions; final report written to `.design/.../report.md` with zero vacuous completions.

---

## Architectural Report Schema

Every execution formats its final output under these mandatory Markdown headings in `report.md`:

```markdown
# Architectural Report: [Scope / Target Name]

## 1. Executive Summary & Strategy
- Mode: [Review | Model | Implement] (Strategy: [Steady | Breakthrough])
- Primary Target: [Target Path or Module]
- Deliverable Path: `.design/YYYY-MM-DD-<mode>-<target-slug>/`
- Architectural Thesis: [Summary of recursive depth and knowledge transformation]

## 2. True Architectural Baseline vs. Final State
<!-- Embed output table directly from: uv run .../diff.py baseline.json final.json -->

## 3. Recursive Depth & Seam Decomposition
- Parent Module Simplification: [What vanished from parent obligations]
- Deep Child Modules Extracted: [Child responsibilities and local depth ratios]
- Depth Propagation Evidence: [Why the parent seam is now deeper]

## 4. Predictability & Layout Refactor
- Cohesion Fixes: [Resolved disjoint reference clusters in files]
- Eliminated Anti-Patterns: [Removed junk drawer files, deep path traversals]

## 5. Deletion Dividend Ledger
- Superseded Types & Functions: [Deleted symbols]
- Eliminated Adapters & Wrappers: [Collapsed layers]
- Removed Dependencies: [Eliminated crates]
- Net SLoC Removed: [Exact count from diff.py]

## 6. Architectural Graveyard (Tried & Rejected)
- Candidate: [Concept]
- Hypothesis: [Intended improvement]
- Rejection Evidence: [Concrete failure metrics or errors]

## 7. Verification & Grader Audit
- Seam Behavior Verification: [Tests executed through intended seams]
- Public API Stability: [Diff verified via cargo-public-api]
- Done Criteria Verification: [Audited by independent pass]
```
