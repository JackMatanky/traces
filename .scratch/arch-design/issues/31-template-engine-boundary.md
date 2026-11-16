# What is the template engine boundary with query, schema, temporal, and the host?

Type: grilling
Status: open
Blocked by: 7, 16, 18, 19, 20, 21, 22, 23, 24, 25, 28, 29, 30, 32

## Question

Decide how the template engine relates to the rest:

- Decide between typed host objects and stringified values at the minijinja
  boundary (`src/template/engine/query.rs`).
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

### Errors

Facts: `survey/template.md` (H3 refuted by `research/01` claim 7),
`survey/schema.md` H6, `.scratch/template-service/module-design-review.md`,
`docs/adr/0001`, `0003`, `0005`, `0006`.

Alternatives: typed objects; plain data with explicit conversion helpers; keep
as is. Include template-visible API stability from the compat decision (ticket
16: capabilities are Frozen, spelling is Migratable). For each bullet, name
"keep as is" and say what withstands scrutiny.

Inputs: `research/07`, query core, temporal boundary, value model, compat
envelope.

This ticket graduates these fog items: template micro-optimisations (AST cache,
regex caches). The map fog item "Template internals" also names lazy schema
load. This ticket graduates that part too.

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). Two adapters remain: natural
serde for schema literals and the hand-written runtime adapter. `Int` renders
`5` where `Number(f64)` rendered `5.0` (measured), an intentional change.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Template
comparisons and date handling use the shared relation module, not minijinja's
own comparison. The natural-language date grammar (ticket 30) is shared with
`date()` and `date_add` filters. `now`/`today` come from the execution context.

The handoff does not say which operators it covers. Today `QueryRow` has a
`custom_cmp` that uses `cmp_document_order`. Dates are strings, so minijinja
compares them as strings. A typed object alone does not prove the shared
relation contract. Check each operator that the adapter exposes.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The
template service stays free of `note`. An app-layer use case runs an advisory
check between `render()` and `write()`. The check parses rendered frontmatter,
resolves the class, and calls the schema check. The check skips execution
without frontmatter or class. The check also runs under `--dry-run`. The schema
stays a source of values, not of prompts (ADR-0006). Ticket 20 rejects static
template analysis, because frontmatter is raw text to the template AST.

Ticket 20 lists the wiring of this check as a handoff to this ticket. Today
`render()` and `write()` are `pub(crate)` in `src/template/service.rs`.

Today `--dry-run` computes no output path. Thus its post-render check cannot
apply path bindings. Ticket 32 now decides that `DryRun` resolves the output
with the writer rule (D9). The target is therefore known. Path bindings can
apply under `--dry-run`. Keep the frontmatter and tag checks. One question
remains: the collision prompt can change the final path after the check.
`TemplateWriteTarget::resolve` runs it at write time [V]. See the section
"Writer, dialog and inspection seams" in the Question.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md). A
render samples one temporal frame. Nested queries and any lazy results keep this
frame after the render state expires. Never let minijinja callbacks convert a
typed materialization or zone error into a missing value or blank cell.
Iteration, display, `len` and indexing callbacks must fail closed. The
render/write operation is all-or-nothing. A template escape set that survives
rendering retains the same frame.

Civil `file.cdate`/`mdate` presentation uses the pinned zone of the render.
Filename-derived `file.date` does not need one. `DialogProvider` receives an
adapter-selected mode and honors authored defaults unattended. `DialogProvider`
cannot invent answers, and an exhausted preset is an error. Random UUID
creation remains an explicit separate entropy capability, not a hidden
request-global service.

## Inputs from the note document and index read model decisions

See [ticket 23's Answer](./23-note-document-model.md) and
[ticket 24's Answer](./24-index-read-model-and-query-execution.md).

- Check whether a template reads external links or outlinks. Ticket 23 removes
  external URLs from the stored `Note::outlinks`.
- Ticket 24 names this ticket as an index-facing consumer. Use the index-owned
  domain port. Do not import a storage engine.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- A host opens the store only to refresh and to persist. It serves reads from
  its pinned generation.
- A read that meets a higher format epoch parses fresh, gives one stderr notice
  and exits 0. A rebuild heal is quiet.
- The template engine builds an `IndexerService` today, so it reaches the same
  fingerprint (ticket 22).
- Tickets 26 and 27 hand nothing to this ticket [V: search of both tickets].

## Inputs from the live-buffer, query-core and path decisions

See [ticket 28](./28-live-buffer-and-persisted-index-consistency.md),
[ticket 29](./29-query-core-and-plan.md) and
[ticket 32](./32-path-and-capability-model.md).

- A host render reads its pinned derived generation, including admitted buffers.
- Expose `{key, rows}` through a distinct `GroupedQuerySet`. Preserve the typed
  key and the `QuerySet` rows.
- Preserve fallible terminals, clause identities and atom spans through
  callbacks. The terminals are iteration, indexing, formatting and grouping.
  `QuerySet::count` is a terminal too.
- Return typed replacement errors for grouped `sort`, `limit`, `where` and
  `count` calls. Name the permitted operation in each error.
- Choose names for the confined default inspection test and the raw-path test.
  Ticket 32 keeps the capability. The spelling is open (D9, change row 21).
- Apply the writer's output-directory rule to `DryRun`. Local values pass local
  trust admission. Global values obey ticket 22's global-config policy. A
  runtime `-o` or `file.write_to()` value stays confined. `DryRun` and the
  prompt show the resolved path.
- Keep output-path resolution consistent between inspection, preview and
  writing. Show preview diagnostics without changing successful stdout payloads.
  The test `Dry-run previews rendered content without writing a file` pins the
  stdout payload.
- Do not expose a generic public unchecked join. The template I/O seam owns the
  admitted-config exception (ticket 32).
