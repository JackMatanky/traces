# Research: precedent for execution context, configuration ownership, and composition roots

Type: research
Status: resolved

## Question

Gather transferable lessons for tickets [Ambient environment](12-ambient-environment-and-execution-context.md) (explicit context vs ambient statics), [Config ownership](13-config-ownership-and-lifecycle.md) (who owns config section types; reload/trust lifecycle), and [Workspace host](17-workspace-host-and-composition.md) (headless host/application layer). Note the in-flight config digests/drift work (map Notes) is fixed input; do not research digest design.

**General/Rust comparables** (read source; rust-docs-mcp for crate APIs; `web_search` only to locate): cargo's `GlobalContext` (explicit context threaded vs statics; how it owns env, cwd, config, shell) and cargo config resolution; uv/ruff per-crate settings structs and how option types are owned vs composed; gitoxide `Repository`/`ThreadSafeRepository` handle model; jj `Workspace`/`Repo`/transaction layering; jiff/`time`/`chrono` clock and zone injection (testability without global guards); ripgrep/fd handling of env and cwd.

**Local digests** (`docs/digests/`; very large — **search/grep first, read only matched ranges**; `*-src-digest.txt` = source, `*-docs-digest.txt` = docs, unsuffixed = whole repo; check structure before choosing):

- `rumdl-src` digest (and `rumdl` whole-repo digest only if needed): config discovery and precedence, how per-rule options are typed and owned, how config is passed to rules (context vs globals), workspace/crate layout, and how the CLI and its long-running server share a core.
- `obsidian_silentvoid13-obsidian-templater-src-digest.txt` only if a lesson on how a host exposes capabilities/settings to a template runtime is needed (see also [Query and template precedent](07-research-query-ir-template-boundary-precedent.md)).

## Deliverable

`.scratch/arch-redesign/research/05-context-config-composition-precedent.md`: at most ~8 lessons, each tagged to the ticket(s) it informs and graded `VERIFIED` (source, cite file/section), `ECOSYSTEM-CONSENSUS`, or `INFERENCE`. Each carries a **transferability test** against Traces' constraints (single-user synchronous CLI with a possible long-lived frontend; `index → config` fixed by in-flight work; trust gating of local config; small per-process floors measured in research 03). Include **negative lessons**. Do not recommend a design; do not modify production code or docs. Append an `## Answer` (5–8 lines) to THIS ticket and set `Status: resolved`. Do NOT edit `map.md`.

## Exact digest file names (`docs/digests/`)

`lsp_rvben-rumdl-src-digest.txt` (rumdl source), `lsp_rvben-rumdl-docs-digest.txt` (config docs), `lsp_rvben-rumdl-digest.txt` (whole repo; avoid unless needed), `obsidian_silentvoid13-templater-src-digest.txt` and `-docs-digest.txt`. The `lsp_` prefix is a grouping artifact; LSP behavior is out of scope — use rumdl only for CLI config/discovery/context patterns.

## Answer

Eight precedent lessons across Cargo, Ruff, Gitoxide, Jujutsu, Jiff, Ripgrep, and Rumdl were synthesized and verified in `.scratch/arch-redesign/research/05-context-config-composition-precedent.md`. For [Ambient environment](12-ambient-environment-and-execution-context.md), Cargo's `GlobalContext` and Jiff's immutable `(now, zone)` prove that an explicit context eliminates 24 `local_zone()` sites, process LazyLocks, and test locks (`ZoneGuard`, `CwdGuard`) with zero runtime overhead. For [Config ownership](13-config-ownership-and-lifecycle.md), Ruff's tiered settings demonstrate that domain modules can own their settings types (`note → TaskConfig`), severing `note → config` upward edges while remaining fully compatible with Issue 07's exhaustive destructuring for fail-closed config digests. For [Workspace host](17-workspace-host-and-composition.md), Gitoxide (`ThreadSafeRepository`) and Jujutsu (`ReadonlyRepo`) establish that a headless workspace host should own immutable Arc snapshots (`Arc<WorkspaceIndex>`, `Arc<Config>`), serving lightweight operation handles without redb load/persist floors. Rumdl (`DocumentRun`) confirms that parameterizing document analysis by explicit buffer and path roles unifies CLI, stdin, and LSP without disk writes. Finally, Templater and Rust 2024 CWD precedents supply critical negative constraints: never leak host objects to template runtimes, and never query ambient CWD inside library code.
