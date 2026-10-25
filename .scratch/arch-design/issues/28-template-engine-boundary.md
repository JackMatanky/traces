# What is the template engine boundary with query, schema, temporal, and the host?

Type: grilling
Status: open
Blocked by: 7, 26, 27, 18, 16

## Question

Decide how the template engine relates to the rest:

- Decide between typed host objects and stringified values at the minijinja
  boundary (`src/template/engine/query.rs:604`).
- Decide whether the `QuerySet` bridge wrapper adds anything over the query
  layer's own lazy set. The question is only about the wrapper. `research/01`
  verified the bridge: chained methods clone only the `Arc<Vec<QueryRow>>` +
  plan, NOT the rows. The scout's "clones the row vec" claim is REFUTED.
- Decide whether to drop the `State` temp-cache hack for `Schema` access.
- Decide whether schema loading is eager or lazy.
- Decide whether render-scoped index caching is needed.
- Decide on error-span preservation across callbacks.
- Decide whether `engine/*` retains duplicated domain logic after temporal/query
  decisions.
- Decide the engine's relationship to the writer and dialog seams.

(Writer/path confinement is decided in the path and capability ticket.)

Facts: `survey/template.md` (H3 refuted by `research/01` claim 7),
`survey/schema.md` H6, `.scratch/template-service/module-design-review.md`,
`docs/adr/0001`, `0003`, `0005`.

Alternatives: typed objects; plain data with explicit conversion helpers; keep
as is. Include template-visible API stability from the compat decision.

Inputs: `research/07`, query core, temporal boundary, value model, compat
envelope.

This ticket graduates these fog items: template micro-optimisations (AST cache,
regex caches).

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). Two adapters remain: natural
serde for schema literals and the hand-written runtime adapter. `Int` renders
`5` where `Number(f64)` rendered `5.0` (measured), an intentional change.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Template
comparisons and date handling use the shared relation module, not minijinja's
own comparison. The natural-language date grammar (ticket 27) is shared with
`date()` and `date_add` filters. `now`/`today` come from the execution context.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The
template service stays free of `note`. An app-layer use case runs an advisory
check between `render()` and `write()` (parse rendered frontmatter, resolve
class, call the schema check); it is skipped without frontmatter or class and
also runs under `--dry-run`. The schema stays a source of values, not of prompts
(ADR-0006).

Under `--dry-run` no output path is computed, so the post-render check cannot
apply path bindings (frontmatter and tag bindings still apply).

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md). A
render samples one temporal frame; nested queries and any lazy results keep it
after the render state expires. Do not let minijinja's display, iteration, `len`
or indexing callbacks convert a typed materialization or zone error into a
missing value or a blank cell; render/write is all-or-nothing. A template escape
set that survives rendering retains the same frame. Civil `file.cdate`/`mdate`
presentation uses the render's pinned zone; filename-derived `file.date` does
not need one. `DialogProvider` receives an adapter-selected mode and honors
authored defaults unattended but cannot invent answers; an exhausted preset is
an error. Random UUID creation remains an explicit separate entropy capability,
not a hidden request-global service.
