---
name: rust-design
description: >
  Use when analyzing, reviewing, or refactoring Rust software architecture: evaluating module boundaries and seams, establishing recursive depth, discovering latent responsibilities, eliminating cyclic dependencies, optimizing file layout and cohesion, or modeling domain types and typestates.
---

# Rust Design

Deepen Rust architecture: concentrate behavior, policy, state, and invariants behind well-placed seams so callers learn less and Rust enforces domain constraints at compile time.

Treat comments, docs, ADRs, names, and layout as evidence of intended design, not proof of responsibility or seam placement. Validate claims against behavior, dependencies, state, callers, tests, and source structure. Load `rust-skills` only after a design concern is discovered; consult relevant categories rather than applying rules as a generic checklist.

---

## Operational Modes & Execution Strategies

Classify user prompt intent into an operational mode and strategy:

| Mode | Strategy | Trigger Signals | Active Steps | Stop Condition |
| :--- | :--- | :--- | :--- | :--- |
| **Review** | **Diagnostic** | "review", "audit", "inventory", "check boundaries", "evaluate architecture" | **1, 2, 6** | True Baseline frozen; 10 lenses probed; Hypothesis Ledger ranked by EAV; report emitted. |
| **Model** | **Type Modeling** | "model", "design types", "represent invariants", "typestate", "type safety" | **4** | Seam established; types, errors, state transitions, and compile-time invariants documented. |
| **Implement** | **Steady** | "refactor", "deepen", "optimize", "fix seam", "reorganize", "steady" | **1, 2, 3, 4, 5, 6** | High-EAV hypotheses evaluated via empirical trial loop; verified against baseline; patch emitted. |
| **Implement** | **Breakthrough** | "breakthrough", "radical", "paradigm shift", "fundamental", "from scratch" | **1, 2, 3, 4, 5, 6** | Single high-leverage bet prototyped against explicit drop condition; verified or logged to graveyard. |

- **Steady (Targeted Deepening):** Works the ledger sequentially by EAV: restructure internals, extract deep child modules, collapse shallow wrappers, eliminate cyclic dependencies; preserve public crate contracts.
- **Breakthrough (Architectural Leap):** Spend the run budget on one ambitious bet (e.g. zero-cost typestates, data-oriented layout, inverted cross-crate ownership); requires an explicit drop condition and sandboxed trial.

---

## Grilling Follow-Up (All Modes)

After any run produces its deliverable (report, model, or patch), offer a `grill-me` session (user-invoked only; never auto-chain) to stress-test design theses, top `ledger.json` items, or proposed types.

---

## Deliverable Containment

All analysis, scout findings, trial records, and reports go under the project root `.design/` directory:

```text
.design/YYYY-MM-DD-<mode>-<target-slug>/
├── baseline.json        # measure.py
├── final.json
├── final-diff.md
├── ledger.json          # Hypotheses
├── graveyard.md         # Rejected prototypes
├── report.md
├── scouts/
│   ├── lens1-modern.md ... lens10-checks.md
└── patch.diff           # Implement only
```

No loose evaluation directories or scout files at the workspace root.

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
2. **Rank by Expected Architectural Value (EAV):** Score every candidate in `ledger.json` per [`DISCOVERY.md`](DISCOVERY.md) §4 (formula + consequential triggers). Consequential means `EAV >= 1.0` or any listed trigger.
3. **Closed Deferral Enum:** **REQUIRED:** defer only with the three variants defined in [`DISCOVERY.md`](DISCOVERY.md) §5 (Closed Deferral Reason Enum); maximum 3 per review without explicit user approval.

**Done when:** All 10 lenses report into `.design/.../scouts/`; `ledger.json` populated and sorted by descending EAV; every tool finding has a recorded disposition (investigated, dismissed, or Closed Deferral Enum max 3).

### 3. Empirical Trial-and-Revert Search Loop

**REQUIRED:** Read [`DEEPENING.md`](DEEPENING.md) §4 and §5 before the first trial; sandbox rules, drop conditions, attempt limits, rollback, graveyard format, and gate definitions live only there.

1. **Select Candidate:** In Steady mode, select the top-ranked item from `ledger.json`. In Breakthrough mode, select the single highest-leverage fundamental bet.
2. **Sandboxed Trial:** Work in a clean worktree/branch; for Breakthrough bets state the drop condition before editing.
3. **Iterate & Refine:** Apply candidate transformations within the attempt limit.
4. **Commit or Revert:** Keep only on green tests + stable public API + positive scoreboard delta; otherwise revert immediately and log the post-mortem to `.design/.../graveyard.md`.
5. **Termination Gates:** Convergence, Plateau (two consecutive failures → re-scout), or Budget; definitions in [`DEEPENING.md`](DEEPENING.md) §5.

**Done when:** All selected hypotheses empirically evaluated and committed or reverted; Deletion Dividend lists superseded types, wrappers, dependencies; Graveyard records rejected bets with concrete failure evidence.

### 4. Model Design in Rust

**REQUIRED:** Read [`MODELING.md`](MODELING.md) before proposing any type. In one line: concrete types first, invalid states unconstructable (`ISR = 0`), typestates only when transitions are correctness-critical (`ITE = 0`), panic-free `Result` seams, POLS layout with no junk drawers.

**Done when:** Proposed types, ownership, errors, and traits directly serve intended seams; invariants enforced at construction or compile time; type ceremony balanced against caller burden.

### 5. Verify Implementation

1. **Test Verification:** Verify required behavior through the intended seam using project test suites. If tests fail, return to Step 3 for refinement or rollback.
2. **Public API Stability:** Run `cargo public-api` diff checking to verify zero accidental breaking changes.
3. **Calculate Deltas (Low Freedom):** Run the diff calculator to generate comparative markdown tables:
   ```bash
   uv run .agents/skills/rust-design/scripts/measure.py <target-path> .design/.../final.json
   uv run .agents/skills/rust-design/scripts/diff.py .design/.../baseline.json .design/.../final.json
   ```
4. **Anti-Gaming Guardrails:** **REQUIRED:** Read [`METRICS.md`](METRICS.md) §4 before scoring any delta; comment preservation and semantic deletion dividend rules live only there.

**Done when:** Intended behavior verified through tests entering seam; public API diff verified clean; SLoC deltas and comment preservation verified; superseded structure accounted for in Deletion Dividend.

### 6. Boundary Verification & Adversarial Grader Audit

1. **Boundary Reconsideration:** Inspect downward for newly visible concepts and upward for parent or sibling simplifications.
2. **Adversarial Grader Audit:** Run an independent grader pass against the Done Criteria of active steps to ensure zero vacuous completions or unjustified deferrals.
3. **Emit Architectural Report:** Compile `.design/YYYY-MM-DD-<mode>-<target-slug>/report.md` using [`report-template.md`](report-template.md).

**Done when:** Inspected boundaries and parent/child relationships documented; all findings have definitive dispositions; final report written to `.design/.../report.md` with zero vacuous completions.
