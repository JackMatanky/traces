# How do mature Rust workspaces structure errors, diagnostics, crate boundaries, and test seams?

Type: research
Status: resolved
Blocked by: none

## Question

Traces has 28 `thiserror` enums in 14 modules. They funnel into a 1.8k-line
`CliError` that matches 4 levels deep (`src/cli/error.rs`). It uses
`#[expect(private_interfaces)]` to embed `pub(crate)` errors in a public enum
(`:39-50`). It drops structured diagnostics for all subsystems except query. It
exposes 45 (now 46, after `207cc3fb`) private types through `traces::testing`
for 12 benches + integration tests (`src/lib.rs:177-210`). The exposure is
guarded by 1,528 (now 1,550) frozen visibility tokens and API baselines. Single
crate, 93k LoC, strictly acyclic 10-tier module DAG. See `survey/cli.md`,
`survey/cross-cutting.md`, ADR-0004.

Study these comparables:

- rustc (`rustc_errors::Diag`, `Diagnostic` derive, error codes)
- Ruff (`ruff_db::diagnostic`, per-crate error types)
  - crate split: `ruff_linter` vs `ruff_workspace` vs `ruff` bin
  - bench/test internals access: `ruff_benchmark`, `*_test` crates
- Cargo (`CargoResult`/anyhow + `cargo-util`, `cargo-test-support` crate)
- uv (`uv-*` crate split, error conversion, `miette`-free presentation)
- gitoxide (`gix-*` per-crate errors, `gix` facade, `gix-testtools`)
- rumdl (single crate? how it exposes internals to benches)
- jj (`jj-lib` vs `jj-cli` error mapping `CommandError`)

Add others.

Focus:

- Who owns diagnostic codes and presentation: library or CLI?
- Do library errors carry structured spans/help, or are they flattened to
  strings?
- What crate-split criteria are actually used (compile time, dependency
  isolation, API stability, reuse by a second binary like an LSP)?
- What do they cost (visibility churn, test-support crates)?
- How are internals exposed to benches/tests (test-support crate,
  `#[doc(hidden)]`, feature)?
- What stability debt does each exposure carry?
- Is a 100k-LoC single crate typical or an outlier?

Informs: error and diagnostic architecture, crate topology and public surface,
workspace host and composition.

Deliverable: `.scratch/arch-design/research/08-errors-topology-precedent.md` (<=
300 lines). Study real source (shallow-clone into a temp dir, rust-docs-mcp
`get_item_source`, or GitHub raw files) and cite repo path + symbol/line for
every claim. Use this structure:

1. What each comparable actually does (checked from source).
2. The invariants and who owns them.
3. The tradeoffs and costs they pay.
4. Transferable lessons for Traces.
5. Lessons that do NOT transfer, and why.
6. What this changes or challenges in the Traces hypotheses named above.

Mark facts [V], interpretation [H]. Do not propose the final decision; that
belongs to the decision tickets that list this ticket in Blocked by.

## Resolution

Deliverable completed at
[.scratch/arch-design/research/08-errors-topology-precedent.md](../research/08-errors-topology-precedent.md).

Investigated real source across all 7 comparables (rustc, Ruff, uv, Cargo,
gitoxide, rumdl, jj) with checked citations (`repo path + symbol:line`). Key
findings:

1. Libraries own diagnostic codes and spans (rustc, Ruff, rumdl). Other failures
   travel through `anyhow` cause chains or struct facades (`jj::CommandError`,
   `uv::hints_for_error`). No comparable puts 28 domain enums in one 1.8k-line
   CLI enum.
2. A struct facade with `Arc<dyn Error>`/`Box<dyn Error>` and hint collections
   (jj pattern) removes `#[expect(private_interfaces)]`. It also removes 4-level
   deep pattern matching and stack bloat. It keeps centralized exit codes.
3. A 100k-LoC single crate is not an outlier (rumdl has 341k SLoC in `src/` as a
   single crate). Combining a single crate with frozen `pub-tokens.txt`
   baselines and a 45-type `test-utils` backdoor is an outlier. Comparables
   either make internals public (rumdl), or isolate test utilities into a
   dedicated workspace test crate (Cargo, jj, gitoxide, Ruff).
4. In-band `BrokenPipe` detection is standard in production CLI runners (Ruff,
   jj), mapping to exit 0 without diagnostics.
