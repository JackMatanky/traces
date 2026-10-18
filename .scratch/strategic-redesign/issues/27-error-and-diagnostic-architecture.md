# Who owns diagnostics, and what do library errors carry?

Type: grilling
Status: open
Blocked by: 8, 16, 26

## Question
Decide the error architecture: structured diagnostics (code, spans, help) owned by libraries vs flattened at the CLI (`src/cli/error.rs`, 1.8k LoC matching 4 levels into 7 subsystems; only `QueryError` preserves spans); per-module error enums (28 in 14 modules) vs fewer cross-cutting types; the `private_interfaces` suppression; where schema validation diagnostics (if any) land; minijinja error-span loss; presentation split (per-subsystem presentation modules vs a `ToDiagnostic` trait vs miette on library types — ADR-0004 explicitly keeps miette out of libraries; challenge it).

Facts: `survey/cli.md` H3/H7, `survey/cross-cutting.md` §3 (error table), ADR-0004, `src/query/error.rs`, `src/cli/error.rs:39-52`.
Inputs: `research/08`, schema validation decision, workspace host decision.
