# Who owns diagnostics, and what do library errors carry?

Type: grilling
Status: open
Blocked by: 8, 12, 13, 20, 34

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

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Three
advisory stderr notices need a mechanism while stdout and exit codes stay
unchanged. The first is a count of rows excluded by a bracket mismatch. The
second is a build-time notice for a date-shaped but invalid quoted literal. A
parse-time `NaN`-to-`Null` notice is the third.

Ordinary type mismatches in filters do not error at evaluation. A bare invalid
date literal stays a syntax error. Ticket 21 requires typed
environmental/materialization errors instead of blanket no-error semantics.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). 20 pins
span-free, severity-free `Violation` records with stable codes and a bounded
`expected`. This ticket owns the diagnostic envelope, severities, per-code
overrides and the registry (md-pkm-lsp 25 is a fallible input). Type problems
are never `Error` by default (ticket 19). Gating is a host policy over codes.

Violations carry a layer (`class` or `global`) and an `origin`. The adapter
derives a distinct code family `schema.global.*`. This ticket ranks that family
below class violations by default. A violation also carries a `hint` from the
field's `description`. Report a broken schema separately from note violations.

Guardrail channels (20, agent guardrail) reuse one violation envelope across the
lint exit code, the MCP `isError` result, the LSP and the notice on read. An
open item asks whether a stored baseline of violation keys is a diagnostic or
read-model concern. Another open item decides gate behavior when the schema is
broken (fail closed, distinct code).

## Inputs from the IWE and Fileclass research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md),
[13](../research/13-iwe-query-schema-and-agent-write-gating.md) and
[14](../research/14-fileclass-model-binding-and-validation.md). IWE has no
central error type. It uses narrow enums in `liwe` (`OperationError`) and plain
`Result<T, String>` in `diwe`. Hosts adapt them to LSP `ResponseError` or CLI
strings [V]. String results lose structure, so IWE provides no model for
library errors.

Its violation record is richer: header breadcrumb, message, `hint` from the
nearest `description`, JSON-Pointer `schemaPath` and failing keyword [V]. Exit
codes are 0, 1 and 2 for clean, violations and a broken schema. An unreachable
schema entry is a load-time exit 2 [V]. IWE has no severities and no code
registry.

Fileclass violations are `{ ok, message }` only, shown as a coloured indicator
(`src/fields/validate.ts`) [V]. Ticket 20's coded violations and this ticket's
registry go beyond both models.

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

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- A read that meets a store with a higher format epoch parses fresh, gives one stderr notice, and exits 0. Make the notice travel as a notice and not as an error.
- A rebuild heal gives no notice. Report its cause in the refresh report only.
- Ticket 25 renames `RefreshReport::ConfigDrift` and `DriftCause` and adds identity causes: engine, platform, epoch, extractor and damaged `META`. This ticket gives the cause texts.
- `traces index` from an older binary replaces a newer store. It gives one stderr notice first.

## Inputs from the note document and index read model decisions

See [ticket 23's Answer](./23-note-document-model.md) and [ticket 24's Answer](./24-index-read-model-and-query-execution.md).

- `parse_markdown` returns notices beside `Note`. A parse-time `NaN` becomes `Null` with a notice.
- This ticket owns how parser notices show. It also owns the bulk-index notice path.
- Ticket 24 stores parser notices in the `notices` facet.
- A cache failure on a read is one stderr notice and exit 0. See ticket 24 and the freshness inputs above.
