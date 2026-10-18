# Workspace host: is there a headless application layer, and what does it own?

Type: grilling
Blocked by: 12, 13, 15, 16, 05

## Question

Service composition is scattered: `cli/mod.rs` wires `ConfigService`, `IndexerService`, `QueryService`, and (conditionally) `SchemaService` through private free functions per command; `TemplateService` constructs its own engine holding `Arc<IndexerService>`, `Arc<SchemaService>`, and `Arc<dyn DialogProvider>` and acts as a second composition root; the glue `FileClassExpander` is built by whichever caller needs it. `cli` cannot be reused by another frontend (it reads argv and cwd and prints to stdout). A long-lived frontend (language server, possible MCP server) needs one place that owns configuration, trust, the index snapshot and its refresh, schema registry, and query/template capabilities. Decide whether the redesign has a **headless workspace host/application layer**, what it owns, what its API looks like at the seam level (snapshot acquisition, refresh, capability access, lifecycle), how it shares state across threads (the core is synchronous; rayon inside), and how `cli`, `template`, and future frontends consume it. Include the ownership of user interaction (`DialogProvider`) and cancellation/abort semantics at this seam.

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify system findings](02-research-verify-system-findings.md) §5–6 lands): [cli/dialog §2, §4, §11–12](../survey/cli-dialog.md), [template §4–5](../survey/template.md), [cross-cutting §5–6](../survey/cross-cutting.md). Inputs: md-pkm-lsp tickets 09 (runtime: transport-only async, sync core), 10 (analysis host = thin facade over `Arc<WorkspaceIndex>` swap), 12 (concurrency/cancellation). A host that merely re-exports existing services is a shallow wrapper; the question is whether a real invariant (single coherent snapshot + config + schema generation; refresh ordering; trust state) belongs to it.

**Verified (ledgers landed 2026-10-18)** — [02 §5–6](../research/02-verify-system-findings.md): construction sites — `ConfigService` `cli/mod.rs:227`; `IndexerService` `cli/index.rs:41`, `cli/mod.rs:344`; `QueryService` `cli/mod.rs:477`; `SchemaService` `cli/mod.rs:494`; `TemplateService` `cli/template.rs:140`. `TemplateEngine::new` (`engine.rs:109-168`) is a verified **second composition root**: it constructs `SchemaService`, `IndexerService`, and three `QueryService` instances. No non-`cli` production code writes to stdout/stderr (logging uses `tracing`); `normalize_source_input` does filesystem `stat`s (`cli/mod.rs:451-463`); `cli/task.rs` builds query strings. `CliError` has 22 variants. [03 §8](../research/03-performance-evidence.md): per-process floors are small (start ≈ 4.5 ms, config+trust ≈ 0.3–1.5 ms), so a host's value is state coherence and reuse across frontends, not CLI startup cost; template render time is >95% the redb refresh floor.

## Alternatives (not exhaustive)

- No new layer: extract the existing CLI orchestration into a reusable module with no new state.
- A `Workspace`/`Session` value owning config generation, index snapshot, and schema registry, handing out cheap read handles; CLI creates one per process, a server keeps one alive.
- Capability traits implemented by the host and consumed by template/query (inversion), vs concrete services passed down.
- Where the template renderer's dialog/IO needs sit relative to the host (render-scoped context vs host-owned).

## Done when

The answer states whether the host exists and the invariants it owns, its seam-level API (not signatures to implement), threading/state-sharing contract, lifecycle and reload semantics consistent with [Ambient environment](12-ambient-environment-and-execution-context.md)/[Config ownership](13-config-ownership-and-lifecycle.md)/[Index read model](15-index-read-model.md), what leaves `cli`, how `template` consumes it, and the effect on error ownership ([Errors and diagnostics](19-error-and-diagnostic-architecture.md)) and crate topology ([Crate topology](20-crate-topology-and-public-surface.md)).
