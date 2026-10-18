# Template engine boundary: adapter or domain owner?

Type: grilling
Blocked by: 02, 16, 17, 07

## Question

The template subsystem (~13K lines) is described as a minijinja adapter, but reportedly owns substantial domain behavior: a Moment-token date-format translator and anchor arithmetic (`engine/date.rs`, 2.5K lines), minijinja `Object` wrappers re-exposing `QuerySet`/`QueryRow`/`FileFields`/`ListFields` plus presentation filters (`engine/query.rs`, 2K), schema reflection implemented in the engine via `State` temp-storage downcasting (`engine/schema.rs`), prompt option extraction (`engine/ui.rs`), render-scoped service caches, and template output writing with collision prompts. Templates declare no parameters (ADR-0001: interactive helpers run mid-render), roots are re-parsed on every render, and a language server needs *static* analysis that never executes helpers. Decide what the **template engine's boundary** is: what stays in the minijinja adapter, what capabilities it receives from the workspace host ([Workspace host](17-workspace-host-and-composition.md)), whether the domain logic it currently carries moves to the owning subsystems, how render-scoped state is passed (State temp storage vs explicit context), how compilation/caching and static analysis are structured, and where interactive I/O sits.

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify system findings](02-research-verify-system-findings.md) §5 lands): [template §2, §5–§9, §11](../survey/template.md), [temporal §9.3](../survey/temporal.md), [schema §11.6](../survey/schema.md). Treat ADR-0001/0003/0005 as claims; md-pkm-lsp ticket 22 (static analysis via `minijinja::machinery`, never execute templates) as an input constraint.

**Verified (ledgers landed 2026-10-18)** — [02 §5](../research/02-verify-system-findings.md): `TemplateEngine::new` is a second composition root (see [Workspace host](17-workspace-host-and-composition.md)); `State` temp-storage caches `query.index_cache` (`query.rs:100`) and `schema.service_cache` (`schema.rs:90`) exist; root templates are re-parsed on every render (`engine.rs:190-193`); chained query calls clone an `Arc` shallowly (the survey's "deep clone" claim is **refuted**), though materializing a plan with transforms copies the base row vector ([03 §7.2](../research/03-performance-evidence.md)). In `template/engine/date.rs` (2,486 lines incl. tests) only the Moment-token translation (~140 lines, `date.rs:482-623`) has no `temporal` counterpart; anchor math (`start_of`, `end_of`, `shift`, `with_weekday`, `DateDiff`) already delegates to `temporal::DatePoint` — so the "template duplicates temporal" claim is mostly **refuted**; the real leak is ambient clock/zone use ([Ambient environment](12-ambient-environment-and-execution-context.md)). `template::engine::query` does hold query-domain logic (`set_class_depth` AST rewrite, `query.rs:499-515`, plus presentation filters) and `engine::schema` synthesizes query ASTs (`file_field_source`, `schema.rs:277-309`). Rendering cost beyond the refresh floor is ≈ 0.8 ms at 1K notes ([03 §7.3](../research/03-performance-evidence.md)), so compile caching is a static-analysis/LSP concern, not a CLI latency one.

## Alternatives (not exhaustive)

- Thin adapter over host-provided capabilities: engine owns only registration, value conversion, and render orchestration.
- Keep helper namespaces owning light domain wrappers but move reusable semantics (date formatting, schema reflection, presentation) into their subsystems.
- Compiled-template artifact with separate analysis and execution paths (shared parse).
- Explicit render context object replacing `State` temp-storage downcasting.

## Done when

The answer states the engine's responsibilities and exclusions, the capability interface it consumes, per-render state handling, compile/analysis structure, what code moves or is deleted, how synchronous interactive I/O is contained, and which fog items (date-format dialect, presentation) become specifiable.
