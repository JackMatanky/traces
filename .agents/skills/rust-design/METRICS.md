# Metrics: Automated Gauges & Semantic Evaluation Rubrics

---

## 1. Automated Tool Metrics

The following metrics are deterministically extracted by `measure.py` and compared via `diff.py`:

| Metric Dimension | Tool / Source | Threshold / Target |
| :--- | :--- | :--- |
| **Source Lines of Code (SLoC)** | `measure.py` | Net reduction |
| **Doc Comment Lines** | `measure.py` | Maintained/expanded |
| **Public API Footprint (IKL)** | `cargo-public-api` | Stable or compressed |
| **Elevated CRAP Functions** | `cargo-crap` | Reduced (>8.0) |
| **High CRAP Functions** | `cargo-crap` | Eliminated (>15.0) |
| **Maintainability Hotspots** | `messrust` | Reduced |
| **Cyclic Dependency Edges** | `cargo-modules` | 0 cycles |
| **Duplicate Crate Versions** | `cargo tree -d` | 0 duplicates |
| **Graph Edge Density** | `codegraph`/`rustgraph`| Reduced |
| **Max File SLoC** | `measure.py` | `<= 500` SLoC |
| **Module / File Count** | `measure.py` | Cohesive count |
| **Unbounded Channels / Queues** | `measure.py` | 0 unbounded |
| **Non-Test Unwraps / Panics** | `measure.py` | 0 on public paths |

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
Evaluate what disappeared from the parent module when extracting the child:
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

In multi-trial iterative refactorings, track metrics across checkpoints using the Scoreboard format generated by `diff.py --scoreboard` (abbreviated example: header plus baseline row):

```markdown
## Multi-Trial Progression Scoreboard

| Trial / Step | SLoC | Doc Comments | CRAP > 8.0 | Public API | Graph Edges | Tests | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| baseline | 1,850 | 420 | 5 | 34 | 5,120 | 48 passed | baseline |
```
