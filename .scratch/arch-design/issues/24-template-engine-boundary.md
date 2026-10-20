# What is the template engine boundary with query, schema, temporal, and the host?

Type: grilling
Status: open
Blocked by: 7, 22, 23, 14, 12

## Question

Decide how the template engine relates to the rest: typed host objects vs
stringified values at the minijinja boundary
(`src/template/engine/query.rs:604`); the `QuerySet` bridge (verified by
`research/01`: chained methods clone only the `Arc<Vec<QueryRow>>` + plan, NOT
the rows — the scout's "clones the row vec" claim is REFUTED; the question is
only whether the bridge wrapper adds anything over the query layer's own lazy
set); `Schema` access without the `State` temp-cache hack; schema loading eager
vs lazy; render-scoped index caching; error-span preservation across callbacks;
whether `engine/*` retains duplicated domain logic after temporal/query
decisions; and the engine's relationship to the writer and dialog seams.
(Writer/path confinement is decided in the path and capability ticket.)

Facts: `survey/template.md` (H3 refuted by `research/01` claim 7),
`survey/schema.md` H6, `.scratch/template-service/module-design-review.md`,
`docs/adr/0001`, `0003`, `0005`.

Alternatives: typed objects; plain data with explicit conversion helpers; keep
as is. Include template-visible API stability from the compat decision.

Inputs: `research/07`, query core, temporal boundary, value model, compat
envelope.

Graduates fog: template micro-optimisations (AST cache, regex caches).

## Inputs from the value model decision

See [ticket 14's Answer](./14-value-model.md). Two adapters remain: natural
serde for schema literals and the hand-written runtime adapter. `Int` renders
`5` where `Number(f64)` rendered `5.0` (measured), an intentional change.

## Inputs from the coercion and comparison decision

See [ticket 15's Answer](./15-coercion-and-comparison-policy.md). Template
comparisons and date handling use the shared relation module, not minijinja's
own comparison. The natural-language date grammar (ticket 23) is shared with
`date()` and `date_add` filters. `now`/`today` come from the execution context.
