# Tooling: Analyzer Matrix & Execution Protocol

---

## 1. Resource & Worktree Isolation (SSOT)

To eliminate resource contention and workspace contamination:

- **Strict Scout Read-Only Rule:** Parallel subagents (such as the 10 read-only scouting lenses in Step 2) operate strictly in **read-only mode**. Scouts are **forbidden** from running `cargo build`, `cargo test`, or diagnostic scripts, eliminating Cargo target directory lock contention. Only the main orchestrator executes CLI tools or test suites.
- **Sandboxed Prototyping Worktrees:** Run non-trivial implementation trials in an ephemeral Git worktree or branch (`improve/<run-id>-<trial-id>`). This prevents incomplete experiments from dirtying the primary development worktree.

---

## 2. Three-Tier Fallback Ladder

Follow this three-tier capability ladder across diverse host environments:

1. **Tier 1: Automated Script (preferred):** `uv run .agents/skills/rust-design/scripts/probe.py` (probe), `measure.py <target> [out]`, `diff.py <base> <final>`.
2. **Tier 2: Configured environment tasks (fallback):** inspect `mise tasks`, `just --list`, `cargo make`, or `Makefile`; run configured aliases (`modules:tree`, `mess`, `crap`, `orphans`).
3. **Tier 3: Standard Cargo toolchain (universal minimal):** structure via `cargo check`, LSP references, manual inspection; quality via `cargo clippy --all-targets -- -D warnings`; testing via `cargo test`; dependencies via `cargo tree` / `cargo tree -d`. Mark unavailable metrics `not measured (tool unavailable)`.

---

## 3. Tool Guidance & Execution Matrix

| Tool | Capability & Invocation | Evidence & Limit |
| :--- | :--- | :--- |
| `codegraph` | `codegraph status -j`, `codegraph files -j --filter <path>` | Whole-crate node/edge density and per-file symbol counts via SQLite AST. |
| `rustgraph` | `rustgraph structure --json` | AST-aware code navigation fallback when `codegraph` is unavailable. |
| `cargo-public-api` | `cargo public-api` / `cargo public-api --diff-git-checkouts <base> <head>` | Mathematical count of public API surface (IKL); automated breaking-change diff. |
| `cargo-modules` | `cargo-modules tree`, `cargo-modules dependencies --lib` | Module hierarchy and dependency edges. Cycle detection can flag re-exports. |
| `cargo-crap` | `cargo-crap` (requires test coverage data) | Change Risk Anti-Pattern scores (cyclomatic complexity combined with test coverage). |
| `messrust` | `messrust` | Advisory maintainability hotspots and tangled responsibilities. |
| `jscpd` | `jscpd --format rust --reporters console <paths>` | Token-clone sites and duplicate token clusters. A clone is a lead, not repeated policy. |
| `cargo tree` | `cargo tree`, `cargo tree -d` | Built-in Cargo dependency hierarchies and duplicate crate resolution. |

Before anything else, resolve external crate semantics through authoritative crate documentation (local documentation tools or `docs.rs`) rather than recalled memory or guessing from source.

---

## 4. Choose Evidence by Question

| Question | First Evidence | Escalate When Needed |
| :--- | :--- | :--- |
| Who calls this and where does state live? | CodeGraph/LSP for flow; definitions and references | Inspect relevant source and callers for semantic ownership |
| Is a seam bypassed or dependency wrong? | Graph paths, imports, and intended entry points | `cargo-modules dependencies` for structural cycles |
| Does a representation permit invalid states? | Enumerated state and transition model | Behavior tests entering intended seam |
| Will a change cross parent seams? | Frozen change scenario and graph impact | Inspect affected responsibilities and source paths |
| Did a redesign leave obsolete structure? | Changed interfaces, references, dependencies | `cargo tree -d` and orphan checks |
| Does the implemented seam preserve behavior? | Project build and tests through intended seam | Scoped mutation checks for uncertain policy |

---

## 5. Audit Views Protocol

At module scope and above, obtain these three views before declaring an audit complete:
1. **Structure & Topology:** Run `measure.py` (or `cargo-modules tree` alongside CodeGraph). Confirm material edges in source.
2. **Callers & Ownership:** Use LSP definitions/references and inspect source for intended entry paths, bypasses, visibility, state, and policy.
3. **Hotspots & Risk:** Inspect CRAP risks, complexity hotspots, and token clones. Identify tests entering each intended seam.

---

## 6. Empirical Benchmark Protocol

When optimizing computational scaling or hot-path latency:
1. **True Performance Baseline:** Run benchmarks before modifying code. Record absolute numbers and store them in the run record.
2. **Alternating Iterations:** Compare trial performance against the baseline by alternating runs across commits to isolate background noise and thermal throttling.
3. **Never Hack Benchmarks:** Only iterate library and internal implementation code. Never alter benchmark harness parameters to simulate gains.
4. **Independent Iterations:** Ensure benchmark runs do not leak state or caches across iterations unless persistent caching is the explicit capability under test.
