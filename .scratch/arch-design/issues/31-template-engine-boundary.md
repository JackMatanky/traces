# What is the template engine boundary with query, schema, temporal, and the host?

Type: grilling
Status: open
Blocked by: 7, 16, 18, 19, 20, 21, 22, 23, 24, 25, 28, 29, 30, 32

## Question

Decide how the template engine relates to the rest. Answer each numbered
question separately. Use
[ticket 36's decision-record rules](./36-reconcile-decisions-and-audit-coverage.md#decision-records-and-evidence-gates).
Keep decision evidence separate from implementation gates. A question that ends
with "Needs 30" waits for the answer of ticket 30. See "Dependencies on open
tickets" for the other open tickets.

### Values and operators

1. Decide between typed host objects and stringified values at the minijinja
   boundary (`field_value` in `src/template/engine/query.rs`).
   The decision covers `Date`, `DateTime`, `Duration` and `Link`. Needs 30.
2. Decide which template operators on host values obey the shared relation
   module of ticket 19. The candidates are `==`, `<`, `sort`, `unique` and `in`.
   Needs 30.
3. Decide the `to_yaml` output of each typed host value.
   Skip this question if question 1 keeps strings.

### Query bridge

4. Decide whether a `QuerySet` bridge wrapper adds anything over the query
   layer's own lazy set. The question covers only the wrapper. `research/01`
   verified the bridge: chained methods clone only the `Arc<Vec<QueryRow>>` and
   the plan, NOT the rows.
   The scout's "clones the row vec" claim is REFUTED.
   Ticket 29 later fixed the lazy set contract and the shared memo.
   Today no wrapper type exists. See "Facts verified on main" for the orphan
   rule.
5. Decide the channel that carries lazy errors through minijinja callbacks.
   Tickets 21 and 29 need every terminal to return or propagate the memoized
   typed error. See the live-buffer, query-core and path inputs for the
   terminals.
6. Decide the minijinja form of a group key.
   Ticket 29 needs a typed and lossless key. Needs 30.
7. Decide whether templates get `take_groups(n)`.
   Ticket 29 adds it only if a template needs it.
8. Decide how `with_children`, `with_descendants` and the schema file-field
   filters build a source when the query IR is immutable (ticket 29).

### Render state and services

9. Decide the render-scoped carrier.
   It lets a callback reach the temporal frame, pinned generation and services.
10. Decide how templates reach the schema service.
    The old string-keyed cache already uses a typed `State` extension.
    Ticket 37 later decides its internals.
11. Decide whether `file.write_to()` keeps its `State::set_temp` slot
    (`WRITE_TO_KEY`) or returns its path another way.
12. Decide whether schema loading is eager or lazy.
13. Decide which index path a render uses.
    Today it calls `IndexerService::refresh` and loads the whole generation.
    A host supplies the pinned generation of ticket 28.
    A cold render can use the read path of tickets 24 and 25.
    Ticket 33 owns host composition, not a second engine-side read contract.
14. Decide whether render-scoped index caching is needed.
15. Decide what `TemplateService` takes from config. Today it holds `&Config`.
    The engine reads these values:
    - The root.
    - The class field.
    - The schema directory.
    - The output directory.
    The composition root supplies the shared `IndexPolicy`.
    Ticket 25 excludes filename-period policy unless the index consumes it.
16. Decide the entropy source of the `uuid()` function.
    Ticket 21 needs an explicit entropy capability.

### Errors

17. Decide how callbacks preserve error locations and typed causes.
    Cover the template span that the VM stamps.
    Cover query clause identities and atom spans.
    Ticket 34 owns the presentation.

### Writer, dialog and inspection seams

18. Decide the public shape of the render and write split.
    The app layer runs ticket 20's advisory check between `render()` and
    `write()`.
19. Decide how the advisory check uses the final output path after any collision
    prompt. Keep preview and write path resolution consistent.
20. Decide the spelling of the confined inspection tests and raw-path tests.
21. Decide whether the raw-path test accepts only absolute paths.
    Ticket 32 keeps this rule as a proposal [P].
22. Decide the engine-side dialog contract.
    Cover the typed prompt error for an unattended missing answer.
    Cover the mode that the adapter selects. Ticket 33 decides the threading.

### Cost and duplication

23. Decide whether `engine/*` retains duplicated domain logic after the temporal
    and query decisions. Needs 30.
24. Decide whether to cache compiled templates (the AST cache).
25. Decide whether to cache compiled regex patterns.

Writer and path confinement are decided in the path and capability ticket
(ticket 32). This ticket decides only the template-side names and adapters.

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

## Dependencies on open tickets

- Ticket 30 (listed blocker). These bullets depend on its answer: typed objects,
  relation operators, the group key form and duplicated domain logic. The
  `date()` and `date_add` filters also use its natural-language grammar.
  Ticket 30 decides the `date.format` dialect and the home of the Moment
  translator. Bullets that do not depend on ticket 30 can start first.
- Ticket 33 (not a blocker). It owns the composition root, the held workspace
  and the dialog threading. Decide the engine-side contract in terms of three
  per-render inputs: a pinned generation, a temporal frame and a dialog
  provider. Every host option can supply them.
- Ticket 37 (blocked by 36, which waits for this ticket). It owns the test-only
  `SchemaService::new` and the template `Schema` handle. This ticket decides how
  templates reach a schema. Ticket 37 decides the internals.
- Ticket 34 (blocked by this ticket). It owns the diagnostic envelope and the
  display of template errors. This ticket keeps the typed cause chain intact.
- Ticket 35 (blocked by 33 and 34). It owns the crate cut and the visibility of
  `TemplateService`, `render()` and `write()`. Choose adapters that compile
  under any cut.
- Ticket 36 (audit). It audits the fallible lazy-result API across template
  callbacks.
- Ticket 38 is open. If it adopts body checks, this ticket must accept its
  logical locator and input contract. A numeric host debounce does not block
  that contract. Keep template skeleton generation separate.

## Facts verified on main

Facts below come from `main` at `cf6d79bd`. [V] marks a fact checked in code.
[H] marks a hypothesis.

- The survey baseline is stale. `reverification.md` says `src/template` is
  untouched. Since `207cc3fb`, 20 files in `src/template` changed [V: `git
  diff`]. Many commits are documentation. These changes alter structure or
  behavior:
  - minijinja 3 typed extensions replace the string-keyed `set_temp` cache for
    the index and for `Schema`.
  - The Moment translator moved to `src/temporal/token.rs`. The engine wrapper
    `pattern_to_strftime` calls it.
  - Duration rendering moved to `src/template/engine/duration.rs`.
  - Template writes became atomic (`commit_with_writer` in
    `src/template/writer.rs`).
- `src/template/engine/date.rs` has 2,512 lines [V]. Production code is about
  880 lines. The 2.5k LoC figure in the survey includes test code.
- Three mechanisms carry render state today [V]:
  - `cached()` in `src/template/engine/cache.rs` stores the `WorkspaceIndex` in
    a typed `State` extension.
  - `SchemaServiceCache` in `src/template/engine/schema.rs` stores the
    `Arc<SchemaService>` in a typed extension. `schema.get()` seeds it.
  - `file.write_to()` stores its path with `State::set_temp` and the string key
    `WRITE_TO_KEY`. `TemplateEngine::render` reads it with `get_temp`.
- `TemplateEngine::new` loads schemas once with `SchemaService::load_verbose`.
  It builds `IndexerService::from(config)` [V]. A registry-wide schema error
  fails `TemplateService::new` with `TemplateError::SchemaLoad`. A field-level
  error excludes that schema and logs it.
- `TemplateEngine::render` compiles the source on each call with
  `template_from_named_str`. It renders with an empty context [V].
  `src/template/engine/string.rs` compiles a regex on each `regex_replace` and
  `regex_match` call [V].
- The CLI renders one template for each process [H: `survey/template.md` H6].
  The measured base render costs 28 to 30 ms at 5,000 rows (`research/11`
  section 8). Template parse time below 50 us is a scout claim [H].
- `field_value` in `src/template/engine/query.rs` converts `Duration`, `Date`,
  `DateTime` and `Link` to strings. `Link` becomes its target. `Number(f64)`
  becomes a float, so `5` renders as `5.0` [V].
- No wrapper type exists for `QuerySet`. `impl Object for QuerySet`, `QueryRow`
  and `SourceSelector` sit on the query types in `src/template/engine/query.rs`
  [V]. The orphan rule would forbid this impl if `query` and `template` became
  separate crates [H]. Ticket 20 reproduced E0117 for a similar case, and a
  newtype compiled.
- `Object::get_value`, `enumerate`, `enumerator_len`, `is_true` and `render`
  cannot return an error in minijinja 3.0.1. Only `call` and `call_method`
  return `Result` [V]. `QuerySet::get_value`, `enumerate` and the `count` method
  use this infallible shape today [V].
- minijinja 3.0.1 supports an invalid value, `Value::from(Error)`. The VM checks
  it in `Instruction::Iterate` and in list unpacking. `Serialize for Value`
  checks it when it iterates [V]. The doc says a consumer should use
  `ValueIter::checked` to fail on it.
- `process_err` in minijinja 3.0.1 stamps the template name and span on a host
  error only if `err.line().is_none()` [V]. `research/07` verified this in
  2.24.0. A host error that already has a line keeps that line.
- `RenderFailureKind::classify` walks the error source chain. It downcasts to
  `DialogError`, `QueryError`, `IndexError` and `io::Error` [V]. The CLI then
  reads `error.range()` and `error.template_source()` in `render_error_location`
  (`src/cli/error.rs`) [V].
- `QueryRow::get_value` returns `None` when the field path is bad. A
  `QueryError::FieldPath` never becomes a render error [V]. `FileFields` and
  `ListFields` convert values with no error path. Ticket 21 makes civil
  `file.cdate` need a zone, so these callbacks need an error channel.
- `uuid()` calls `Uuid::new_v4()` in `src/template/engine.rs`. `date.now()` and
  `date.today()` read the ambient clock (`DateTimeValue::now`,
  `DateValue::today`) [V].
- `PathOps` registers three minijinja tests: `path_exists`, `is_file_path` and
  `is_dir_path`. They accept absolute paths as they are [V].
- `TemplateService::write` returns `Previewed` for `DryRun` before it resolves
  any output path. Under `CommitPolicy::CreateNew`,
  `TemplateWriteTarget::resolve` prompts for a new path when the output exists,
  `-o` is unset and the provider is interactive [V].
- `TemplateLoader::list_available` lists only top-level stems. `find` accepts
  nested paths [V: `survey/template.md` section 3]. The survey lists a recursive
  listing as an open decision. No ticket owns it. It is a tactical item for
  `findings-ledger.md`.
- Tests that pin template behavior: `tests/features/template/*.feature`,
  `tests/integration/template_render.rs` and `benches/template_render.rs` (a
  sweep up to 1,000 notes) [V].

## Template-visible changes that resolved tickets already decide

Record each change in the specification change table. These items are inputs,
not open questions.

- `Int` renders `5`, not `5.0` (ticket 18, measured).
- Frontmatter and inline Markdown and embed links become `Link` values (ticket
  16 and ticket 18). With the current adapter, a template prints the target and
  no longer prints `[a](b.md)` [H].
- `group_by(x)` returns groups, not sorted rows (ticket 29).
- `file.day` is retired. `file.date` comes from the filename only. A missing
  date or creation time is `Null`. `file.cdate` never falls back to the
  modification time (ticket 21). `FileField::ACCESSOR_NAMES` still lists `day`
  [V].
- An unattended prompt with no authored default fails (ticket 21). The scenario
  in `tests/features/template/template-prompts.feature` pins today's fallback
  answers and must change.
- Terminal access to a result set becomes fallible (tickets 21 and 29).
- A multi-valued inline field becomes a list (ticket 16 known defect and ticket
  23) [H].
- The default inspection test stops accepting absolute paths (ticket 32).
- Relational filters across brackets stop matching (ticket 19). This applies to
  `.where()` strings.

## Inputs from the compatibility decision

See [ticket 16's Answer](./16-grilling-compatibility-envelope.md).

- Template and query capabilities are Frozen: query notes, lists and tasks,
  schema lookup, dialogs, file operations, date arithmetic, `--from` and
  `--where`. They must stay reachable.
- Template function and object spelling is Migratable. Heuristic string-to-date
  promotion is Migratable. Each change needs a row in the change table.
- Dataview and Templater parity is Free. Moment tokens are a Templater
  compatibility feature (`research/07`).
- The Rust library surface is Free. The visibility rule still applies: widen a
  `pub` item only for an outward-facing need.

## Inputs from the forward-consumer decision

See [ticket 17's
Answer](./17-grilling-forward-consumers-and-design-pressure.md).

- Keep the core synchronous. Put async at the host and transport seams.
- The LSP is the next committed consumer. The MCP server is likely. A template
  run inside a host must not need CLI globals (ticket 21).
- Operation-specific performance budgets belong to the template decisions too.
  Measured: a template loads the whole generation, about 50 ms at 5,000 notes
  and 255 ms at 50,000 notes (ticket 29). Its decode is single-threaded.
- Async dialog bridging for a long-lived host stays in the map fog. This ticket
  does not decide it.

## Inputs from the config decision

See [ticket 22's Answer](./22-config-ownership-and-lifecycle.md).

- `TemplateService` holds `&Config`. The template engine builds an indexer
  (`template/engine.rs`).
- `Config` no longer holds the ignore and period policy.
- The composition root builds `IndexPolicy` one time. The CLI and the template
  engine share it. Ticket 24 confirms the rule.
- `init` takes plain data and does not take a `DialogProvider` into core. The
  template still takes one (ticket 21).
- The template directory can be anywhere under the root. Trust covers the whole
  root. Init adds no starter templates.
- Local config needs local trust. Global config does not use the trust gate
  (ticket 22). This matters for the output directory (ticket 32).

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). Two adapters remain: natural
serde for schema literals and the hand-written runtime adapter. `Int` renders
`5` where `Number(f64)` rendered `5.0` (measured), an intentional change.

Gap: the YAML filters are a third seam. `to_yaml` serializes any template value
with `noyalib`. `from_yaml` converts a parsed value with `Serde`. Ticket 18 does
not list them [V: `src/template/engine/yaml.rs`]. A plain `Object` serializes as
its display string, and a `Seq` object serializes its items [V: minijinja
3.0.1]. Decision 1 must keep `to_yaml` working for typed values.

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
cannot invent answers, and an exhausted preset is an error. Random UUID creation
remains an explicit separate entropy capability, not a hidden request-global
service.

Ticket 21 leaves these items to this ticket: the minijinja carrier, the template
carrier shapes, captured escaping sets, and fallible formatting. Research
verified two carriers in minijinja 3.0.1 [V: `research/21-prep/brief-r3.md`]:

- A hidden root-context key (`$frame`) reaches callbacks through
  `State::lookup`, including `include` and isolated `import`.
- A lazily seeded `State` extension gives one sample for each render. The engine
  must hold a sampler.

`render_captured` builds its own `State`. A caller cannot seed a separate
`State` [V: `research/21-prep/brief.md`]. Research 07 names bound handles (Typst
`Engine`, Rhai `NativeCallContext`) as a third option [H]. A prior design in
`.scratch/temporal-core/design-nl-dates.md` proposes an "Invalid date" string
fallback for `date.relative()`. That fallback conflicts with the fail-closed
rule above. Do not adopt it.

## Inputs from the note document and index read model decisions

See [ticket 23's Answer](./23-note-document-model.md) and [ticket 24's
Answer](./24-index-read-model-and-query-execution.md).

- Ticket 23 asks whether a template reads external links or outlinks. Answer
  [V]: no template path reads `Note::outlinks`. `FieldPath` has `Inlinks` and no
  outlinks accessor. External URLs resolve to no file, so they never create an
  inlink (the test `ignores_outlinks_that_do_not_resolve_to_any_file` in
  `src/index/inlinks.rs`). Removing them from `Note::outlinks` changes no
  template output [H]. Close this check.
- Ticket 24 names this ticket as an index-facing consumer. Use the index-owned
  domain port. Do not import a storage engine.
- `WorkspaceIndex` stays total. The cold CLI builds entries only for the
  selected rows. Ticket 24 removes the split failure rules for read queries.
  Today the template calls `refresh`, which logs a persist failure and returns
  the in-memory index. `refresh_store` fails with `IndexError::Store`. [V:
  ticket 24 Facts].
- A read with an unusable derived cache falls back to a fresh parse and exits 0
  with one stderr notice (ticket 24).

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

See [ticket 28](./28-live-buffer-and-persisted-index-consistency.md), [ticket
29](./29-query-core-and-plan.md) and [ticket
32](./32-path-and-capability-model.md).

- A host render reads its pinned derived generation, including admitted buffers.
  The cache must not outlive its generation (ticket 28 code delta, row "Template
  index cache"). A template run inside a host can write output that derives from
  unsaved text. The index sees that output only through its saved file.
- Expose `{key, rows}` through a distinct `GroupedQuerySet`. Preserve the typed
  key and the `QuerySet` rows.
- Ticket 29 fixes the accessors: `len` is the group count, `row_count` is the
  distinct row count and `entry_count` is the membership count. It adds no plan
  stage after grouping. A grouped set has no `sort`, `limit`, `where` or `count`
  method.
- Preserve fallible terminals, clause identities and atom spans through
  callbacks. The terminals are iteration, indexing, formatting and grouping.
  `QuerySet::count` is a terminal too.
- Return typed replacement errors for grouped `sort`, `limit`, `where` and
  `count` calls. Name the permitted operation in each error.
- Ticket 29 asks whether a template needs `take_groups(n)`. It names ticket 31
  as the owner of the group value form and of the lazy-error channel.
- Class names and depth become immutable in the IR. `with_children` mutates them
  in place today (`set_class_depth` in `src/template/engine/query.rs`).
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

## Inputs from the schema internals ticket

See [ticket 37](./37-schema-internals-simplification.md). Ticket 37 decides the
test-only `SchemaService::new` and the template `Schema` handle. It waits for
ticket 36. Settle template access to a schema here first.

## Inputs from research and survey

- `research/07` [V]: minijinja `Object` carries typed values. The VM stamps
  template spans on host errors. These claims hold in 3.0.1. The claim that
  bound handles beat `State::set_temp` is [H]. Typed extensions have replaced
  `set_temp` for the index and `Schema`.
- `research/07` [V]: Moment tokens are a Templater compatibility concern. The
  translator now lives in `temporal::token`. Ticket 30 decides its home.
- `research/01` claim 7 [V]: `QuerySet` chaining clones an `Arc` and a plan.
  `research/11` measured 0.0 ms of row duplication at 5,000 rows and 6.2 ns for
  `QuerySet.clone()`.
- `survey/template.md` open decisions: typed values, Moment move, lazy query
  builder (refuted), path confinement (decided by 32), `DryRun` validation
  (decided by 32), recursive listing (tactical), lazy schema load. Hypotheses H6
  (AST cache) and H8 (regex cache) are the graduated fog items.
- `survey/template.md` preserved fog: LSP intelligence for `record.file.*` in
  template buffers. It stays fog.
- `survey/schema.md` H6: a detached `Schema` forces a `State` slot. The
  alternatives are a bound handle, a shared `Arc` of the hierarchy, and keep as
  is. `survey/schema.md` H7 [H]: a whole-directory schema load takes under 1 ms
  for 100 schemas.
- `.scratch/template-service/module-design-review.md` (dated 2026-09-07) is
  stale in places. It names `chrono` and `Local::now()`. It proposes product
  features: `date.relative()`, `ui.interactive`, multi-field template sort,
  `file.rfc3339`. These features are outside this map unless a decision needs
  them. It advises against caching the `Environment` across renders, because the
  CLI renders once.
- ADR-0001: the `ui` namespace routes through `DialogProvider`. `--no-input`
  swaps the provider. ADR-0003 keeps selection by index. Ticket 21 revises only
  the non-TTY fallback. ADR-0005: `SourceSelector` crosses minijinja by
  downcast. Terminal filters accept a `QuerySet` and return a `String`.
  ADR-0006: the schema supplies values, never prompts.

## Checks before resolution

- Record the alternatives, invariant owners, removable code and unresolved
  items. Include keep-as-is.
- Prove the lazy-error channel through iteration, indexing, `len`, truth tests,
  display and serialization. Include direct and nested query results.
- Exercise a failed materialization and an unavailable zone. Each affected
  terminal must stop rendering with the typed cause intact.
- Do not treat `Value::from(Error)` as proof for every callback. Some callbacks
  return a boolean or a count instead of a `Value`.
- Prove typed numeric equality, temporal comparison, group keys and `to_yaml`
  without a lossy string conversion.
- Exercise includes, imports and an escaping result set. Each must keep the
  selected frame and generation.
- Exercise a collision rename that changes class binding. Run the advisory check
  against the resolved destination.
- Separate new prototypes from current code facts. Keep a cache only if the
  measured workload supports it. Define its key, lifetime and size bound.
- Give ticket 33 the render inputs and path-check sequence. Give ticket 34 the
  typed causes and template and query locations.
- Give tickets 35 and 37 the outward-facing handle requirements. Do not wait for
  their internal module placement.
- Give ticket 36 the stale survey claims and the template-visible change rows.

This review changes no Answer in tickets 30, 33 or 38. Their open choices stay
open. No cache, callback adapter or body-check feature is adopted here.
