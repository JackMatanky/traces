# Discovery: Multi-Lens Scouting & Hypothesis Ledger

---

## 1. Scouting Protocol & Rules

During architectural discovery, the orchestrator dispatches read-only investigations across ten distinct lenses.

- **Strict Scout Isolation Rule:** Scouts inspect source code, documentation, and AST facts. They are forbidden from running builds, tests, or diagnostic scripts. Only the main orchestrator runs CLI tools.
- **Reporting Limit:** Each lens reports up to 5 ranked hypotheses citing exact `file:line` locations, estimated SLoC deletion dividend, and Expected Architectural Value (EAV).
- **Synthesis:** Before hypotheses are committed to `ledger.json`, the orchestrator synthesizes scout findings to identify compounding transformations and resolve conflicts.

---

## 2. The 10 Architectural Lenses

Dispatch parallel subagents using these self-contained directives:

### Lens 1: Modern Rust & Upstream Capabilities

- **Target:** Upstream language or standard library features that obsolete hand-rolled machinery.
- **Inspect:** `std::sync::Exclusive`, modern GAT patterns, slice chunking, `portable_simd`, or upstream crate capabilities.
- **Objective:** Eliminate custom utilities in favor of canonical standard library or compiler intrinsics.

### Lens 2: Data-Oriented Layout & Memory Footprint

- **Target:** Inefficient memory layout, struct padding holes, and cache misses.
- **Inspect:** Struct field ordering, alignment padding, opportunities for Struct-of-Arrays (SoA) layout, and hot/cold field segregation.
- **Objective:** Maximize CPU cache-line saturation and eliminate padding waste.

### Lens 3: Allocation, Lifecycles & Zero-Copy

- **Target:** Unnecessary heap allocations and defensive copying.
- **Inspect:** Defensive `.clone()` calls, unbounded per-item allocations, missing arena or bump allocators for scoped lifecycles, and opportunities for borrowed views or `Cow<'a, T>`.
- **Objective:** Eliminate runtime allocations on hot paths.

### Lens 4: Seam Placement & Knowledge Asymmetry

- **Target:** Callers knowing too much about implementation mechanisms.
- **Inspect:** Callers bypassing abstractions, direct struct field manipulation, leaked storage or serialization representations, and loose visibility.
- **Objective:** Move knowledge behind deep, minimal interfaces.

### Lens 5: State & Invariant Ownership

- **Target:** Invariants distributed across callers or enforced only at runtime.
- **Inspect:** Invariants split across multiple types, mutable state disconnected from logic, invalid states representable in structs, and runtime assertions that could be compile-time typestates.
- **Objective:** Enforce "parse, don't validate" to eliminate invalid states at construction.

### Lens 6: Boundary Crossings & Vertical Leakage

- **Target:** Lower-level details bubbling upward through module boundaries.
- **Inspect:** Internal child error types exposed in ancestor APIs, low-level configuration bubbling upward, and circular imports between sibling modules.
- **Objective:** Isolate internal churn so changes do not propagate across seams.

### Lens 7: Concurrency, I/O & Contention

- **Target:** Lock contention, blocking operations, and unbounded queues.
- **Inspect:** Coarse `Mutex` scopes, blocking operations on async reactors, serialization round-trips, unbounded channels (`unbounded_channel()`), and uncontained panic propagation across task boundaries.
- **Objective:** Bound resource consumption and isolate worker failures.

### Lens 8: Algorithmic Scaling & Hot-Path Complexity

- **Target:** Computational complexity and scaling bottlenecks.
- **Inspect:** $O(n^2)$ lookups, redundant hashing on hot paths, repeated traversals, and opportunities for memoization or early-exit fast paths.
- **Objective:** Reduce big-O computational and memory scaling on hot paths.

### Lens 9: Deletion Dividend & Layer Thinning

- **Target:** Redundant, obsolete, or pass-through structure.
- **Inspect:** Forwarding wrappers, redundant facade traits, one-to-one adapters, dead type conversions, and compatibility scaffolding.
- **Objective:** Delete architectural complexity and thin unnecessary layers.

### Lens 10: Checks That Can't Fail & Test Ergonomics

- **Target:** Fragile, tautological, or over-mocked tests.
- **Inspect:** Tests asserting default states, tautological assertions, over-mocked boundaries hiding regressions, and missing error-path verification.
- **Objective:** Ensure tests verify domain behavior through intended seams.

---

## 3. The Indicator Hypothesis

Diagnostic indicators (complexity, CRAP, duplication, fan-out, file size) are indicators, not verdicts. Translate raw signals into domain ownership questions:

| Tool Signal | Domain Ownership Question |
| :--- | :--- |
| **High Complexity** | What knowledge is entangled here? |
| **Token Duplication** | Is the same policy or only similar syntax repeated? |
| **High Fan-Out** | Is responsibility broad, or is the seam leaking mechanisms? |
| **Large File (>500 SLoC)** | Is it incohesive, or a single cohesive implementation? |
| **Many Tiny Modules** | Do they hide distinct knowledge, or fragment one responsibility? |

Do not combine indicators into an aggregate score. Treat each gauge as an isolated semantic dimension.

---

## 4. Expected Architectural Value (EAV)

Rank candidate redesigns in `ledger.json` using Expected Architectural Value:

`EAV = (Structural/Computational Gain [1-5] × Confidence [0.0-1.0]) / (Blast Radius [1-5] + Implementation Friction [1-5])`

### Consequential Candidate Triggers

A candidate is consequential if `EAV >= 1.0`, or if it satisfies at least one observable trigger:

1. Changes or bypasses a public API signature, visibility boundary, or trait contract.
2. Alters cross-module or cross-crate dependency direction (introducing or removing edges/cycles).
3. Transfers ownership of mutable state, memory layout, or invariant enforcement across types or files.
4. Reduces big-O computational or memory scaling on identifiable hot paths.
5. Targets a component explicitly named in the user prompt.

---

## 5. Candidate Output & Ledger Schema

Each scout reports findings using this standardized record:

```text
id: HYP-###
lens: [Lens Name]
target: [file:line]
thesis: [Transformational thesis]
current seam and caller knowledge: [What callers must currently know]
proposed direction: [Seam relocation, consolidation, or deletion]
what becomes removable (Deletion Dividend): [Obsolete code]
estimated EAV: [Gain * Confidence / (Blast Radius + Friction)]
drop condition: [Concrete condition triggering immediate rollback]
risk: [Potential implementation or lifetime hazard]
verify: [Specific behavior or invariant needing empirical test proof]
```

Store candidate hypotheses in `.design/YYYY-MM-DD-<mode>-<target-slug>/ledger.json` sorted by descending EAV.
