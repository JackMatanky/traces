# Metrics: Automated Gauges & Semantic Evaluation Rubrics

---

## 1. Automated Tool Metrics

The following metrics are deterministically extracted by `measure.py` and compared via `diff.py`:

| Metric Dimension | Tool / Source | Threshold / Target | Evaluation Purpose |
| :--- | :--- | :--- | :--- |
| **Source Lines of Code (SLoC)** | `measure.py` | Net reduction | Non-comment, non-blank lines of Rust source code. |
| **Doc Comment Lines** | `measure.py` | Maintained/expanded | Preserved doc comments (`///`, `//!`) enforcing anti-gaming. |
| **Public API Footprint (IKL)** | `cargo-public-api` | Stable or compressed | Publicly exported items (`pub fn`, `pub struct`, `pub trait`). |
| **Elevated CRAP Functions** | `cargo-crap` | Reduced (>8.0) | Complexity squared times uncoverage cubed plus complexity. |
| **High CRAP Functions** | `cargo-crap` | Eliminated (>15.0) | Critical refactoring hazard requiring tests entering seam. |
| **Maintainability Hotspots** | `messrust` | Reduced | Advisory maintainability risk and tangled responsibilities. |
| **Cyclic Dependency Edges** | `cargo-modules` | 0 cycles | Cross-module circular dependency edges. |
| **Duplicate Crate Versions** | `cargo tree -d` | 0 duplicates | Redundant crate versions in dependency graph. |
| **Graph Edge Density** | `codegraph`/`rustgraph`| Reduced | Total caller and dependency edges across target scope. |
| **Max File SLoC** | `measure.py` | `<= 500` SLoC | God-module risk; indicates entangled invariants. |
| **Module / File Count** | `measure.py` | Cohesive count | Track structural decomposition and layer thinning. |
| **Unbounded Channels / Queues** | `measure.py` | 0 unbounded | Static scan for unbounded memory buffers and channels. |
| **Non-Test Unwraps / Panics** | `measure.py` | 0 on public paths | Crash hazards outside test modules. |

---

## 2. Claim Routing Table

| Claim Type | Applicable Gauges | Primary Evidence Source |
| :--- | :--- | :--- |
| Seam width & caller burden | `IKL`, `PS` | Public API items, `cargo-public-api`, LSP references |
| Boundary integrity & encapsulation | `BR`, `GED` | Import paths, `cargo-modules dependencies`, re-exports |
| Impact of changes across seams | `PD` | Frozen change scenario evaluated against seam boundaries |
| Structure made obsolete & simplified | `DD`, `SLoC` delta | Removed types, collapsed modules, `cargo tree -d` |
| Risk & test fragility | `CRAP` | `cargo-crap`, test coverage entering seam |
| Invariant & state representation | `ISR`, `ITE` | Typestates, enum state machines, compile-time invariants |
| Fault isolation & runtime buffering | `BBR`, `SPR` | Static channel scan, unwrap scan, `cargo-crap` |

---

## 3. Semantic Architectural Rubrics

Tool counters provide raw evidence, not architectural verdicts. Evaluate candidate seams against these semantic rubrics:

### Interface Knowledge Load (IKL)
Count the independent facts a caller must know to use the seam correctly:
- **Types:** Domain concepts and representations exposed.
- **Invariants:** Preconditions and validity constraints the caller must enforce.
- **Ordering:** Required lifecycle sequences or call ordering.
- **Errors:** Failure variants the caller must discriminate and handle.
- **Leaks:** Storage, network, or serialization representations crossing the seam.
*Target:* Minimize caller obligations while maximizing capability.

### Parent Simplification (PS)
Evaluate what disappeared from the parent module when extracting child $C$:
- Child concepts and internal representations removed from parent.
- State machines and validation rules absorbed behind child seam.
- Child dependencies eliminated from parent imports.
*Target:* The parent's implementation becomes shorter and higher-level.

### Seam Bypass Ratio (BR) & Change Propagation Distance (PD)
- **Seam Bypass (BR):** Inspect whether callers bypass parent interfaces to reach child submodules directly via deep imports. *Target:* Zero bypass paths.
- **Change Propagation Distance (PD):** Count intended architectural seams crossed by a minimal correct change for a representative scenario. *Target:* Changes remain contained behind local seams.

### Invariant & State Guarantees (ISR & ITE)
- **Invalid States Representable (ISR):** Zero invalid states permitted by struct field combinations at construction ("parse, don't validate").
- **Illegal Transitions Expressible (ITE):** Zero illegal state transitions permitted by method signatures (enforced via typestates or sealed enums).

### Fault Isolation & Runtime Stability (BBR & SPR)
- **Bounded Buffering Ratio (BBR):** All message channels, thread pools, and queues have explicit capacity bounds to prevent out-of-memory cascades.
- **Seam Panic Ratio (SPR):** Zero unhandled `.unwrap()`, `.expect()`, or `panic!` calls on public and cross-crate entry points.

---

## 4. Anti-Gaming Guardrails (SSOT)

Size metrics must measure **Source Lines of Code (SLoC)**: non-comment, non-blank lines of Rust code.

- **Comment Preservation Rule:** Stripping, deleting, or compressing doc comments (`///`, `//!`), inline explanations, or rustdoc examples to artificially reduce line counts is strictly prohibited. Any proposal that decreases comment-to-code ratios without justification fails verification.
- **Semantic Deletion Dividend:** The deletion dividend evaluates the removal of architectural complexity (dead types, obsolete traits, collapsed wrappers, unused dependencies), not explanatory prose.
- **Isolated Semantic Gauges:** Never combine metric indicators into an aggregate score. Treat each indicator as an isolated semantic dimension.

---

## 5. Progression Scoreboard Schema

In multi-trial iterative refactorings, track metrics across checkpoints using the Scoreboard format generated by `diff.py --scoreboard`:

```markdown
## Multi-Trial Progression Scoreboard

| Trial / Step | SLoC | Doc Comments | CRAP > 8.0 | Public API | Graph Edges | Tests | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| baseline | 1,850 | 420 | 5 | 34 | 5,120 | 48 passed | baseline |
| trial-01-soa | 1,720 | 430 | 3 | 34 | 4,800 | 48 passed | kept |
| trial-02-typestate | 1,680 | 445 | 1 | 28 | 4,200 | 48 passed | kept |
```
