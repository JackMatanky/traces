# Who owns diagnostics, and what do library errors carry?

Type: grilling
Status: open
Blocked by: 8, 20, 30, 12, 13

## Question

Decide the error architecture:

- Decide whether libraries own structured diagnostics (code, spans, help), or
  the CLI flattens them. The CLI file is `src/cli/error.rs`, 1.8k LoC, and
  matches 4 levels into 7 subsystems. Only `QueryError` preserves spans.
- Decide between per-module error enums (28 in 14 modules) and fewer
  cross-cutting types.
- Decide on the `private_interfaces` suppression.
- Decide where schema validation diagnostics (if any) land.
- Decide on minijinja error-span loss.
- Choose the presentation approach: per-subsystem presentation modules, a
  `ToDiagnostic` trait, or miette on library types. ADR-0004 explicitly keeps
  miette out of libraries. Challenge ADR-0004 on that point.

Facts: `survey/cli.md` H3/H7, `survey/cross-cutting.md` §3 (error table),
ADR-0004, `src/query/error.rs`, `src/cli/error.rs:39-52`.

Inputs: `research/08`, schema validation decision, workspace host decision.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Two advisory
stderr notices need a mechanism (stdout and exit codes unchanged): a count of
rows excluded by a bracket mismatch, and a build-time warning for a date-shaped
but invalid quoted literal. A parse-time `NaN`-to-`Null` warning is a third.
Ordinary type mismatches in filters do not error at evaluation; a bare invalid
date literal stays a syntax error. Ticket 21 requires typed
environmental/materialization errors instead of blanket no-error semantics.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). 20 pins
span-free, severity-free `Violation` records with stable codes and a bounded
`expected`. This ticket owns the diagnostic envelope, severities, per-code
overrides and the registry (md-pkm-lsp 25 is a fallible input). Type problems
are never `Error` by default (ticket 19). Gating is a host policy over codes.

Violations carry a layer (`class` or `global`) and an `origin`; the adapter
derives a distinct code family `schema.global.*`, which this ticket is asked to
rank below class violations by default. A violation also carries a `hint` from
the field's `description`. A broken schema is reported separately from note
violations.

Guardrail channels (20, agent guardrail) reuse one violation envelope across the
lint exit code, the MCP `isError` result, the LSP and the notice on read. Open:
whether a stored "violation keys at last check" baseline (for a delta without
git) is a diagnostic concern or a read-model concern, and the behaviour of a
gate when the schema is broken (fail closed, distinct code).

## Inputs from the IWE and Fileclass research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md),
[13](../research/13-iwe-query-schema-and-agent-write-gating.md) and
[14](../research/14-fileclass-model-binding-and-validation.md). IWE has no
central error type: narrow enums in `liwe` (`OperationError`) and plain
`Result<T, String>` in `diwe`, adapted per host (LSP `ResponseError`, CLI
strings) [V]; the string results lose structure, so it is no model for the
library error type. Its violation record is richer: header breadcrumb, message,
`hint` from the nearest `description`, JSON-Pointer `schemaPath` and failing
keyword, with exit 0, 1 and 2 for clean, violations and a broken schema (an
unreachable schema entry is a load-time exit 2) [V]. It has no severities and no
code registry. Fileclass violations are `{ ok, message }` only, shown as a
coloured indicator (`src/fields/validate.ts`) [V], so ticket 20's coded
violations and this ticket's registry go beyond both.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md).
Distinguish a nonmatching row or nullable creation timestamp from missing
required temporal context, failed lazy result materialization, presentation
failure, and an unanswered unattended prompt. The latter failures must cross
query/template boundaries as typed errors and be translated by each host without
silently dropping rows or emitting partial success. Keep the frozen CLI exit
codes and successful stdout payloads; update the redesign change table and
asserted diagnostics for new failure paths. A memoized failure on a lazy result
must remain observable on every terminal accessor.
