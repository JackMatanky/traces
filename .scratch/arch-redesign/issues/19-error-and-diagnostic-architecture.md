# Errors and diagnostics: ownership, layering, and span unification

Type: grilling
Blocked by: 14, 17, 18, 08

## Question

The crate defines 42 error types across subsystems, and `CliError` (`cli/error.rs`, 22 variants, 1.8K lines including tests) centralizes every domain error's miette code, help text, and source-chain classification — domain modules are meant to stay `miette`-free (ADR-0004). Template errors erase minijinja detail into strings, query diagnostics reportedly carry `miette` spans, and positions appear as `BytePos`/`ByteSpan`, logos `Range<usize>`, and `miette::SourceSpan`. A language-server frontend needs protocol-free diagnostics with precise ranges; the CLI needs terminal reports and exit codes. Decide the **error and diagnostic architecture**: layering conventions (when to wrap, flatten, or carry sources), who owns presentation mapping (centralized facade vs per-subsystem diagnostic descriptions consumed by frontends), the diagnostic/span model shared by CLI and a language server, and how errors cross the host seam from [Workspace host](17-workspace-host-and-composition.md).

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify system findings](02-research-verify-system-findings.md) §6–7 lands; note the surveys disagree on `CliError` size): [cli/dialog §8, §12.1](../survey/cli-dialog.md), [cross-cutting §8](../survey/cross-cutting.md), [schema §8, §11.2](../survey/schema.md), [root primitives §8–9](../survey/root-primitives.md). Inputs: ADR-0004; md-pkm-lsp tickets 20/25 (diagnostic sourcing, `schema.*` codes).

**Verified (ledgers landed 2026-10-18)** — [02 §6–7](../research/02-verify-system-findings.md): `CliError` has **22** variants (`cli/error.rs:58-231`; the "52+" claim is refuted); the crate has **42** error types (verified); the survey's "six-layer" propagation example is **refuted** (`FieldAddressError` never converts into `SchemaFieldBuilderError`, and `CliError::TemplateRender` does not exist — the variant is `TemplateInstantiate`), so layering depth must be re-measured from real conversion chains, not assumed. No non-CLI production code writes stdout/stderr. **Verified in this review (not in the ledgers):** `QueryError` implements `miette::Diagnostic` (`query/error.rs:66`) and `QuerySyntaxError` derives it (`query/error.rs:111`), whereas ADR-0004 says the CLI seam alone owns `miette` codes and help; the CLI survey's "no domain module implements `Diagnostic`" claim is therefore **refuted**, and ADR-0004's "review confirms domain modules do not implement miette diagnostics" is currently violated by `query`. `GLOSSARY-MAP.md` omits the `CLI → Schema` edge (real, `cli/mod.rs:494`).

## Alternatives (not exhaustive)

- Keep the central CLI facade; split internally by diagnostic family (as ADR-0004 allows) and add a separate LSP mapping.
- A protocol-free `Diagnostic` description (code, severity, span, message, help) produced by subsystems (no `miette`), rendered by frontends.
- Error-layering convention: leaf errors preserved as sources; one context-adding wrapper per subsystem boundary; no re-stringification.
- A single span/position type (and one conversion edge) across note, query, template, and diagnostics.

## Done when

The answer states the layering convention (grounded in real conversion chains), the owner of codes/help/severity, the shared span model, how template/minijinja and query errors keep their detail, what changes among the 42 types, and the amendment (if any) to ADR-0004.
