# How do mature Rust workspaces structure errors, diagnostics, crate boundaries, and test seams?

Type: research
Status: resolved
Blocked by: none

## Question

Traces has 28 `thiserror` enums in 14 modules funnelled into a 1.8k-line `CliError` that matches 4 levels deep (`src/cli/error.rs`), uses `#[expect(private_interfaces)]` to embed `pub(crate)` errors in a public enum (`:39-50`), drops structured diagnostics for all subsystems except query, and exposes 45 (now 46, after `207cc3fb`) private types through `traces::testing` for 12 benches + integration tests (`src/lib.rs:177-210`) guarded by 1,528 (now 1,550) frozen visibility tokens and API baselines. Single crate, 93k LoC, strictly acyclic 10-tier module DAG. See `survey/cli.md`, `survey/cross-cutting.md`, ADR-0004.

Study actual source of: rustc (`rustc_errors::Diag`, `Diagnostic` derive, error codes), Ruff (`ruff_db::diagnostic`, per-crate error types, `ruff_linter` vs `ruff_workspace` vs `ruff` bin crate split, how benches/test fixtures access internals — `ruff_benchmark`, `*_test` crates), Cargo (`CargoResult`/anyhow + `cargo-util`, `cargo-test-support` crate), uv (`uv-*` crate split, error conversion, `miette`-free presentation), gitoxide (`gix-*` per-crate errors, `gix` facade, `gix-testtools`), rumdl (single crate? how it exposes internals to benches), jj (`jj-lib` vs `jj-cli` error mapping `CommandError`). Add others.

Focus: who owns diagnostic *codes and presentation* (library vs CLI); whether library errors carry structured spans/help or are flattened to strings; crate-split criteria actually used (compile time, dependency isolation, API stability, reuse by a second binary like an LSP) versus cost (visibility churn, test-support crates); how internals are exposed to benches/tests (test-support crate, `#[doc(hidden)]`, feature) and the stability debt of each; whether a 100k-LoC single crate is typical or an outlier.

Informs: error and diagnostic architecture, crate topology and public surface, workspace host and composition.

Deliverable: `.scratch/arch-design/research/08-errors-topology-precedent.md` (<= 300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp `get_item_source`, or GitHub raw files) and cite repo path + symbol/line for every claim. Structure: (1) what each comparable actually does (verified from source), (2) the invariants and who owns them, (3) tradeoffs and costs they pay, (4) transferable lessons for Traces, (5) lessons that do NOT transfer and why, (6) what this changes or challenges in the Traces hypotheses named above. Mark facts [V], interpretation [H]. Do not propose the final decision; that belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Deliverable completed at [.scratch/arch-design/research/08-errors-topology-precedent.md](../research/08-errors-topology-precedent.md).

Investigated real source across all 7 comparables (rustc, Ruff, uv, Cargo, gitoxide, rumdl, jj) with verified citations (`repo path + symbol:line`). Key findings:

1. Diagnostic codes/spans are owned by libraries (rustc, Ruff, rumdl), while operational failures are handled via `anyhow` cause chains or struct facades (`jj::CommandError`, `uv::hints_for_error`). No comparable funnels 28 domain enums into a monolithic 1.8k-line CLI enum.
2. A struct facade with `Arc<dyn Error>`/`Box<dyn Error>` and hint collections (jj pattern) eliminates `#[expect(private_interfaces)]`, 4-level deep pattern matching, and stack bloat without losing centralized exit codes.
3. A 100k-LoC single crate is not an outlier (rumdl has 341k SLoC in `src/` as a single crate), but combining a single crate with frozen `pub-tokens.txt` baselines and a 45-type `test-utils` backdoor is an outlier; comparables either make internals public (rumdl) or isolate test utilities into a dedicated workspace test crate (Cargo, jj, gitoxide, Ruff).
4. In-band `BrokenPipe` detection is standard in production CLI runners (Ruff, jj), mapping to exit 0 without diagnostics.
