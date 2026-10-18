# Research: precedent for diagnostics, error layering, and crate topology

Type: research
Status: resolved

## Question

Gather transferable lessons for tickets [Errors and diagnostics](19-error-and-diagnostic-architecture.md) (error layering, diagnostic ownership, shared span model) and [Crate topology](20-crate-topology-and-public-surface.md) (one crate vs workspace, public surface, test access without a side-door).

**General/Rust comparables** (read source; rust-docs-mcp for crate APIs): rustc/ruff structured diagnostics and how one diagnostic model is rendered by several frontends; `annotate-snippets`, `codespan-reporting`, `miette` usage patterns (derive-on-domain-types vs a presentation layer); BurntSushi's ripgrep (grep-* crates), cargo (cargo-util, `cargo-test-support` as a test crate), uv/ruff (many-crate workspaces, `*-test` crates), nushell and gitoxide (granular crates): granularity, compile-time effects, dependency-direction enforcement, and how integration tests reach internals; error-handling conventions in large workspaces (thiserror vs anyhow boundaries, context-adding wrappers).

**Local digests** (`docs/digests/`; very large — **search/grep first, read only matched ranges**; `*-src-digest.txt` = source, `*-docs-digest.txt` = docs, `*-tests-digest.txt` = tests, `*-benches-digest.txt` = benches):

- `rumdl-src` digest: its workspace/crate layout and public library surface, how one diagnostic/warning type with fix suggestions is shared by the CLI and other consumers, and error layering; `rumdl-tests` digest to see how it tests across crate boundaries without an internal side-door.
- Skip the Obsidian-plugin digests (TypeScript).

## Deliverable

`.scratch/arch-redesign/research/08-errors-crate-topology-precedent.md`: at most ~8 lessons tagged to [Errors and diagnostics](19-error-and-diagnostic-architecture.md)/[Crate topology](20-crate-topology-and-public-surface.md), graded `VERIFIED` (source, cite), `ECOSYSTEM-CONSENSUS`, or `INFERENCE`, each with a **transferability test** (Traces is one crate with 25 top-level modules, 42 error types, 22 `CliError` variants, a 45-identifier `traces::testing` surface, a strict production DAG) and **negative lessons** (e.g. workspace granularity that suits projects an order of magnitude larger). Do not recommend a design; do not modify production code or docs. Append an `## Answer` (5–8 lines) to THIS ticket and set `Status: resolved`. Do NOT edit `map.md`.

## Exact digest file names (`docs/digests/`)

`lsp_rvben-rumdl-src-digest.txt`, `lsp_rvben-rumdl-tests-digest.txt`, `lsp_rvben-rumdl-benches-digest.txt` (the `lsp_` prefix is a grouping artifact; use rumdl for workspace layout, diagnostics type, and test organization, not LSP behavior). Rust Obsidian tools (`rust_*-digest.txt`) only if a concrete error/crate-layout example is needed.

## Answer

Precedent research across rustc, ruff, rumdl, cargo, uv, ripgrep, and gitoxide is distilled into 8 graded lessons in `.scratch/arch-redesign/research/08-errors-crate-topology-precedent.md`.
For [Errors and diagnostics](19-error-and-diagnostic-architecture.md): the ecosystem standard decouples protocol-free structured diagnostic types (`code`, `severity`, `message`, `span`, `help`, `fixes`) from terminal/LSP emitters; deriving `miette::Diagnostic` on domain errors is an anti-pattern that creates viral dependency coupling; byte-offset spans (`Range<usize>`) should be unified across the pipeline, with line/column coordinates computed lazily at presentation edges; and typed `thiserror` source chains must be preserved without string-erasure.
For [Crate topology](20-crate-topology-and-public-surface.md): Rumdl demonstrates that a single Cargo package with dual targets (`[lib] name = "rumdl_lib"` and `[[bin]] name = "rumdl"`) cleanly serves CLI, LSP, and WASM without multi-crate workspace churn; Cargo and UV isolate test harnesses in dedicated test crates (`cargo-test-support`, `uv-test`) while Ripgrep and Rumdl test via public library APIs and black-box CLI invocation, suggesting (a hypothesis for the crate-topology ticket, not a decision) that Traces' 45-identifier `traces::testing` side-door feature could be eliminated; and Gitoxide illustrates that micro-crate hyper-fragmentation creates severe maintenance overhead without compile-time or runtime benefit for mid-sized codebases whose module graph is already a verified acyclic DAG.
