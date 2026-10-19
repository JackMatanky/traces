# Discovery: Multi-Lens Scouting & Hypothesis Ledger

---

## 1. Scouting Protocol & Rules

During architectural discovery, the orchestrator dispatches read-only investigations across ten distinct lenses.

- **Strict Scout Isolation Rule:** Scouts inspect source code, documentation, and AST facts. They are forbidden from running builds, tests, or diagnostic scripts. Only the main orchestrator runs CLI tools.
- **Reporting Limit:** Each lens reports up to 5 ranked hypotheses citing exact `file:line` locations, estimated SLoC deletion dividend, and Expected Architectural Value (EAV).
- **Synthesis:** Before hypotheses are committed to `ledger.json`, the orchestrator synthesizes scout findings to identify compounding transformations and resolve conflicts.

---

## 2. The 10 Architectural Lenses

Dispatch parallel subagents, one row per scout; each row is a self-contained directive:

| # | Lens | Target | Inspect | Objective |
| :--- | :--- | :--- | :--- | :--- |
| 1 | Modern Rust & Upstream Capabilities | Upstream features that obsolete hand-rolled machinery | `std::sync::Exclusive`, modern GAT patterns, slice chunking, `portable_simd`, upstream crate capabilities | Replace custom utilities with std/compiler intrinsics |
| 2 | Data-Oriented Layout & Memory Footprint | Padding holes and cache misses | Field ordering, alignment padding, Struct-of-Arrays (SoA) opportunities, hot/cold field segregation | Maximize cache-line saturation, eliminate padding waste |
| 3 | Allocation, Lifecycles & Zero-Copy | Unnecessary heap allocations and defensive copying | Defensive `.clone()`, unbounded per-item allocation, missing arena/bump allocators, borrowed views or `Cow<'a, T>` | Eliminate runtime allocations on hot paths |
| 4 | Seam Placement & Knowledge Asymmetry | Callers knowing implementation mechanisms | Bypassed abstractions, direct field manipulation, leaked storage/serialization representations, loose visibility | Move knowledge behind deep, minimal interfaces |
| 5 | State & Invariant Ownership | Invariants split across callers or runtime-only | Invariants across multiple types, mutable state disconnected from logic, invalid states representable, runtime assertions → typestates | Enforce "parse, don't validate" (`ISR = 0`) |
| 6 | Boundary Crossings & Vertical Leakage | Lower-level details bubbling upward | Child error types in ancestor APIs, low-level config bubbling, sibling circular imports | Isolate internal churn across seams |
| 7 | Concurrency, I/O & Contention | Lock contention, blocking, unbounded queues | Coarse `Mutex` scopes, blocking on async reactors, serialization round-trips, `unbounded_channel()`, panic propagation across tasks | Bound resource consumption, isolate worker failures |
| 8 | Algorithmic Scaling & Hot-Path Complexity | Complexity and scaling bottlenecks | O(n^2) lookups, redundant hashing, repeated traversals, memoization or early-exit opportunities | Reduce big-O scaling on hot paths |
| 9 | Deletion Dividend & Layer Thinning | Redundant, obsolete, or pass-through structure | Forwarding wrappers, redundant facade traits, one-to-one adapters, dead conversions, compatibility scaffolding | Delete architectural complexity, thin layers |
| 10 | Checks That Can't Fail & Test Ergonomics | Fragile, tautological, or over-mocked tests | Default-state assertions, tautologies, over-mocking hiding regressions, missing error-path verification | Verify domain behavior through intended seams |

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

### Closed Deferral Reason Enum

Every deferred finding must use one of three variants, capped at 3 deferrals per review without explicit user approval:

- `DEFERRED_TOOL_UNAVAILABLE`: Required diagnostic binary is absent from host.
- `DEFERRED_OUT_OF_SCOPE`: Finding resides entirely outside requested target boundary.
- `DEFERRED_HIGH_TRACE_COST`: Verification requires exhaustive dynamic tracing exceeding turn boundaries.
