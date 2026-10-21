# Map: Traces strategic architectural redesign

Label: `wayfinder:map`.
Tracker: local markdown (`docs/agents/issue-tracker.md`).
Tickets: `issues/NN-<slug>.md`.
Mission brief: [mission.md](./mission.md).

## Destination

A coherent, evidence-backed set of architectural decisions sufficient to write a
comprehensive redesign specification for Traces: boundaries and component
responsibilities, invariant ownership, what to
decompose/consolidate/redesign/introduce/eliminate, domain models and
interfaces, algorithm and data-structure changes, transferable comparative
lessons, foundational investments, and dependency/ordering constraints.
Externally observable capabilities and correctness are preserved. The redesign
itself is not implemented here; the map ends at hand-off to specification.

## Notes

- Domain: Rust 2024 CLI, ~94k LoC single crate, template-driven PKM over
  markdown vaults. Read `GLOSSARY-MAP.md` and the per-module `GLOSSARY.md` for
  vocabulary; existing ADRs are in `docs/adr/` and are fallible context
  (ADR-0009 appears twice).
- Skills every session consults: `rust-design`, `domain-modeling`, `grilling`
  (HITL tickets), `research` (research tickets), `codebase-design`; project mise
  tasks (`mise run verify|test|bench`) for evidence; codegraph for navigation.
  Benchmarks: release only, never in parallel, no RUSTFLAGS.
- Standing preferences: adversarial and independent; evaluate "keep as is" for
  every candidate; try to disprove the preferred option; distinguish [V]
  verified, [H] hypothesis, [P] proposal; quality is not fewer lines or more
  types. Resolve one ticket per session (research tickets exempt).
- User steer (2026-10-18, after the compatibility-envelope ruling): during this
  wayfinder, consider evolving the codebase toward a workspace layout,
  **starting by separating the CLI** into its own crate. This is a direction to
  evaluate, not a decision; the crate-topology and workspace-host tickets must
  test it against keep-as-is. It makes the CLI a real, outward-facing consumer
  of the core, so the core crate's `pub` surface is justified by the CLI
  boundary (visibility rule in the compatibility-envelope answer). Sequencing:
  the CLI split depends on first deciding what the CLI owns today but should not
  (composition, scaffolding, query-source normalisation, task-filter synthesis;
  see the workspace-host ticket).
- Every decision ticket's resolution names: alternatives considered (incl.
  retain), invariant owner, what becomes removable, what remains unresolved, and
  affected downstream tickets.
- Breadth-first survey (per-subsystem, code-verified by scouts; claims NOT yet
  adversarially verified, see [Which scout claims survive adversarial
  verification?](./issues/01-research-verify-load-bearing-claims.md)):
  [root-primitives](./survey/root-primitives.md),
  [temporal](./survey/temporal.md), [note](./survey/note.md),
  [schema](./survey/schema.md), [index](./survey/index.md),
  [query](./survey/query.md), [template](./survey/template.md),
  [config-dialog](./survey/config-dialog.md), [cli](./survey/cli.md),
  [cross-cutting](./survey/cross-cutting.md).
- Baseline: charted at `2dc3250d`; re-verified against main @ `207cc3fb`
  (config-drift detection merged in between). See
  [reverification.md](./reverification.md) for the per-ticket verdict and the
  claims superseded. Before working a ticket, check `git diff 207cc3fb..main --
  src tests docs` for further drift.
- Prior art: an earlier `arch-redesign` wayfinder effort lived in git history
  (`git show 3d934849^:.scratch/arch-redesign/map.md`); it was deleted at
  `3d934849`. Treated as fallible input only; this map was charted independently
  from code.
- Research deviation: this tracker is local markdown, so research findings land
  in `research/NN-*.md` on the working tree and are linked from the ticket; no
  `research/<name>` git branches are pushed.
- Provisionally withstand scrutiny (scouts, unverified; the `FileClassExpander`
  seam was overturned by ticket 16): `DatePoint`/`DifferenceEngine` and
  `DurationValue` nominal
  identity; schema Kahn linearisation + ancestor-bounded `$ref` +
  whole-directory reload; redb derived-cache design (`Durability::None`,
  auto-recreate, fail-open); infallible note parser, `MarkerAccumulator`,
  single-pass Logos item lexing, flat list hierarchy; ADR-0004 CLI error
  isolation, `CommandOutcome`/`UserAbort`, `CwdGuard`, stdout/stderr split;
  acyclic 10-tier module DAG; layered test isolation;
  `LineIndex`/`FieldKey`/`Blake3Hasher` framing. The reconciliation ticket
  re-tests these.

## Decisions so far

<!-- one line per closed ticket: [title](link): gist -->

- [Which scout claims survive adversarial
  verification?](./issues/01-research-verify-load-bearing-claims.md): of 14
  claim groups, 12 confirmed, 1 partially confirmed with a correction
  (ISO-string coercion runs only via `From<noyalib::Value>`/explicit `Classify`,
  not default deserialize) and 1 partly refuted (template `QuerySet` chaining
  does NOT clone rows, it clones an `Arc`+plan; the unconfined-path and DryRun
  parts of that claim hold). New facts: `Config::hash`/`ConfigDigests` unused in
  production at charting (now `ConfigDigests` feeds the index fingerprint;
  `Config::hash` stays dead), multi-valued inline fields are write-only, every
  output command panics on EPIPE, `i64` is lost to `f64` at the note boundary.
  ADR-0009 (index refresh) was unimplemented at charting and has since landed
  (`207cc3fb`); all other ADRs hold.
- [How do mature engines model dynamic values, and what do they pay for
  it?](./issues/02-research-value-model-precedent.md): no comparable keeps an
  owned/`Ref` enum mirror (Polars `AnyValue<'a>` + `into_static`); ordering
  belongs in an external comparator (SQLite `sqlite3MemCompare`, DuckDB), not
  the enum's `Ord`; i64/f64 compare losslessly; serde-visitor direct
  deserialization removes the 3-stage chain; boxing `Link` shrinks the enum 64 B
  to 32 B.
- [Where do mature systems put type coercion and validation, and how do they
  report violations?](./issues/03-research-coercion-validation-precedent.md):
  coercion belongs to query binding / explicit casts (DuckDB `CAST` vs
  `TRY_CAST`, Polars `strict`), not parsing; note validation should be an
  on-demand advisory pass emitting span-located diagnostics
  (jsonschema/taplo/Metadata Menu), never index-gating; cross-kind order must be
  a fixed lattice (no ambient zone).
- [How do mature CLIs make ambient environment (clock, zone, dirs, env)
  explicit?](./issues/04-research-execution-context-precedent.md): jj/uv split
  context rather than one god struct (host environment / workspace / temporal);
  injecting a pinned zone+now per evaluation deletes `ZoneGuard`; snapshot cwd
  once; scope dialog to interactive commands.
- [How do incremental systems fingerprint inputs and decide
  staleness?](./issues/05-research-freshness-fingerprint-precedent.md): input
  completeness is best owned by the type system (Ruff `CacheKey` derive) over
  hand-listed digests; an index-owned fingerprint over consumed config in redb's
  `meta` table suffices (this is what `207cc3fb` then built, so it is confirmed
  rather than open); inverted unresolved-stem maps (Obsidian
  advanced-metadata-cache, rumdl) disprove the full-vault re-read on path
  change; git's racy-timestamp guard is missing from Traces' delta (still
  missing at `207cc3fb`).
- [How do query engines structure predicate languages, plans, and
  pushdown?](./issues/06-research-query-plan-precedent.md): source selection is
  an indexable subset of predicates, not a separate language
  (SQLite/DataFusion); pushdown is a 3-state capability on the storage side
  (`TableSource::supports_filters_pushdown`); Dataview prunes via in-memory
  inverted sets; heavy relational IRs do not fit 1k-20k vaults; `GroupBy` should
  nest or be removed. (One finding on `QuerySet` cloning is superseded, see
  ticket correction.)
- [How do embedded-template hosts bridge domain values and keep date formatting
  coherent?](./issues/07-research-template-bridge-precedent.md): minijinja
  `Object` can carry typed values and stamps template spans on host errors
  automatically; bound handles (Typst `Engine`, Rhai context) beat
  `State::set_temp`; Moment tokens are a Templater-compat concern, not a core
  temporal concept (Jiff/chrono use strftime), which challenges moving them into
  `temporal`.
- [How do mature Rust workspaces structure errors, diagnostics, crate
  boundaries, and test
  seams?](./issues/08-research-errors-topology-precedent.md): struct error
  facade with hints (jj `CommandError`, uv) removes the `private_interfaces`
  suppression; operational errors stay simple while diagnostics keep spans to
  emission; rumdl runs 341k LoC in one crate with open modules; no comparable
  has a `testing` backdoor with frozen visibility baselines; Ruff and jj
  downcast `BrokenPipe` to exit 0; a 2-3 crate split (core / test-support / cli)
  is the supported shape, a 10-tier split is not.
- [How do mature tools enforce root confinement and portable path
  identity?](./issues/09-research-capability-path-precedent.md): `cap-std`
  handles give kernel TOCTOU immunity at high cost, disproportionate for a
  single-user CLI; Traces' `SafeRelativePath` throws away the checked
  `StrictPath` and returns a plain join; carry a portable UTF-8 `/` relative
  path in persistent structs and confine at the I/O boundary; the template
  path-inspection and DryRun leaks are real.
- [How do markdown/document tools model positions, metadata caches, and
  heading/link
  structure?](./issues/10-research-document-model-spans-precedent.md): spans are
  already free (`into_offset_iter` is used); an 8-byte `ByteSpan` (= ruff
  `TextRange`) with on-demand `LineIndex` is compact; rumdl proves one pass
  yields slugs; Dataview never rolls task fields into page metadata and squashes
  single occurrences to scalars; Obsidian/Tasks intern status by symbol; list
  items belong as children of the note.
- [What do the existing benchmarks and targeted new measurements say about the
  consequential performance
  claims?](./issues/11-research-performance-evidence.md): (Apple M4 Pro) cold
  20k-note query is <75 ms either way, but redb secondary tables halve peak RSS
  (17 vs 39 MB); on path-set change 97% of cost is deserializing notes (18.5 ms
  @5k) and only 2.7% is link re-resolution (0.5 ms), so the inverted-stem index
  is unjustified and the lever is note caching/laziness; flat nearest-candidate
  scan beats `CandidateTrie` below ~5 same-stem lookups (trie + `indextree`
  removable); second pulldown-cmark pass is 20-38% of parse time; status
  interning cuts 10k `String` allocs and `ListItem` 160 to ~128 B; a unified
  value enum measures 80 B (no growth); `QuerySet` chaining is free (already
  lazy); the ~25 ms redb commit floor is `Database::create`/drop lifecycle, not
  durability. Re-measured at `207cc3fb` with an A-B-A control: the merged drift
  check adds no detectable cost (median +0.7% vs 1.7% noise).
- [What counts as the externally observable surface the redesign must
  preserve?](./issues/12-grilling-compatibility-envelope.md): sole consumer, so
  almost everything is Migratable via the spec's own change table (no standing
  register). Frozen: vault-format parsing, index result equivalence with a fresh
  parse, exit codes and the stdout/stderr split, stdout payload shapes,
  trust-gate behaviour, template/query capabilities. Free: dialect parity with
  Dataview/Templater, `index.redb` layout and refresh strategy, trust symlink
  layout, unasserted wording, the Rust library surface. Known defects
  (closed-pipe panic, `i64` to `f64`, `.first()`-only multi-values) are not
  preserved. Visibility rule: reduce visibility wherever possible and limit
  `pub` to outward-facing components (widening allowed only with an
  outward-facing justification).
- [Which forward-looking consumers and scales must the architecture be shaped
  for?][fc]: LSP is the next committed consumer; MCP is likely. Support
  source-aware editor features, keep the core synchronous by default, and assess
  operation-specific performance across the seven target vault sizes. Reopen the
  note-model choice in ticket 19; resolve open-buffer/index consistency in
  ticket 30. [fc]: ./issues/13-grilling-forward-consumers-and-design-pressure.md
- [What is the single dynamic value model, and who owns
  it?](./issues/14-value-model.md): two owned types and no borrowed mirror enum.
  `note::NoteFieldValue` (`Int(i64)`/`Float(f64)`, `Link`, derived tagged serde,
  80 B kept) for notes, queries and templates; a schema-local literal type
  (strings stay strings) replaces the schema `FieldValue`; only a root `Link`
  (`target`, normalized `display`, `embedded`) is shared. `NoteLink` (syntax
  kind, span) is the editor-side type for [the note document
  model](./issues/19-note-document-model.md). Frontmatter keeps the Noyalib
  compat parser and converts its AST directly (huge integers become `Float`
  instead of dropping the block); classification is quote-insensitive in
  frontmatter and shares one parser set with inline. `FieldValueRef`,
  `DateTimeCoercion` and the double conversion are removable. Handoffs: zone
  for naive datetimes ([coercion
  policy](./issues/15-coercion-and-comparison-policy.md), [execution
  context](./issues/17-ambient-environment-and-execution-context.md)), temporal
  cache encoding ([index read
  model](./issues/20-index-read-model-and-query-execution.md), [index
  freshness](./issues/21-index-freshness-and-invalidation.md)), view enum
  benchmark ([query core](./issues/22-query-core-and-plan.md)).
- [Where do coercion and cross-type comparison live, and what is the
  policy?](./issues/15-coercion-and-comparison-policy.md): `PartialEq` stays
  structural; one value-semantics module owns loose equality, bracketed
  order for filters (cross-kind `==` false, relational no match, MongoDB-style)
  and a total sort order built with an explicit context. Sort sniffing is
  deleted; Unicode collation (`icu_collator` root, numeric) for sorting only;
  exact `Int`/`Float` comparison, `NaN` to `Null`. Naive datetimes become
  civil and the cache holds no zone. Classification stays quote-insensitive
  (no quote escape: measured too costly and it would silently drop quoted
  dates); quoted query literals adapt per row to Date, DateTime, Duration,
  Link, Number or Bool from pre-parsed candidates, coarse date literals are
  intervals, and a natural-language candidate is a hook. Link equality uses
  the folded target only; `date(link)` reads the target. Two advisory stderr
  notices. Handoffs: [query core](./issues/22-query-core-and-plan.md),
  [temporal boundary](./issues/23-temporal-boundary-and-dialects.md) (natural
  language grammar, `DateConfig`), [execution
  context](./issues/17-ambient-environment-and-execution-context.md), [schema
  typing](./issues/16-schema-note-typing-and-validation.md), [index
  model](./issues/20-index-read-model-and-query-execution.md).
- [Does the schema subsystem validate and type note data, and where does that
  capability live?](./issues/16-schema-note-typing-and-validation.md): advisory,
  on-demand check; no parse-time typing, no read-path gating, schema never
  enters the index or fingerprint. `schema` owns the pure, span-free rule over
  resolved effective fields; the runtime value type sits below `schema` and
  `note` (placement to 28); gating is a host policy over stable violation codes
  that carry a layer (`class` or `global`). Layering principle: cli, lsp and mcp
  are thin hosts of core, whose app layer composes domain modules with plain
  data and no one-implementor traits, so `FileClassExpander` and its crate-root
  impl are deleted for a plain class hierarchy. Several classes resolve
  order-independently (differing declarations are reported once and left
  unchecked, ancestors reduced away); multiple `extends` parents are kept.
  `global` becomes the lowest-precedence layer for every note. Class binding by
  tag and path (class-owned `[binding]`) supplies classes to notes without a
  known explicit class, expanded at query time so the index stays schema-free.
  `required` kept; `step` checked; `date.format` a hint until 23; `origin`,
  `description`, `parent_order` added. Template check is a post-render advisory
  use case; no design depends on harness hooks: the portable guarantee is
  detect and feed back through tiered channels (lint command, git, a notice on
  read, LSP), true gating only where MCP tools own every write; `traces check`
  takes paths, splits schema errors from violations; delta rule; class-set
  change asks. Date-shaped text:
  no override, no quote convention. Body-section schemas are a separate ticket.
  Handoffs: [query core](./issues/22-query-core-and-plan.md), [note document
  model](./issues/19-note-document-model.md), [temporal
  boundary](./issues/23-temporal-boundary-and-dialects.md), [template
  boundary](./issues/24-template-engine-boundary.md),
  [diagnostics](./issues/27-error-and-diagnostic-architecture.md), [crate
  topology](./issues/28-crate-topology-and-public-surface.md), [schema
  internals](./issues/31-schema-internals-simplification.md), [body
  sections](./issues/32-body-section-schemas.md).

## Not yet specified

- **Schema internals**: graduated to [What schema internals survive now that
  schema produces a checker and a class
  hierarchy?](./issues/31-schema-internals-simplification.md).
- **Note parser internals**: status interning, second pulldown-cmark pass,
  `ListItem` footprint, footnote/callout/slug modelling. Sharpen after [What is
  the note document model: spans, facts, items, multi-value
  metadata?](./issues/19-note-document-model.md).
- **Index internals**: `CandidateTrie`/`indextree` retention, store engine vs
  repository split, inline-field class extraction. Sharpen after [What is the
  index read model, and is there one query execution path or
  two?](./issues/20-index-read-model-and-query-execution.md).
- **Template internals**: compiled-AST and regex caches, lazy schema load.
  Sharpen after [What is the template engine boundary with query, schema,
  temporal, and the host?](./issues/24-template-engine-boundary.md).
- **Output formats** (`--format json|csv`) and where formatting lives; **async
  dialog bridging** for a long-lived host. Both hang on the forward-consumer and
  workspace-host decisions.
- **Persistence portability** (cache across OSes) hangs on the compatibility
  envelope and the path model.

## Out of scope

- Tactical defects and mechanical cleanups surfaced by the survey
  (`cmp_document_order` tie-break, EPIPE panic, dependency pruning, glossary
  drift, ADR renumbering, config default asymmetry, etc.): recorded in
  [findings-ledger.md](./findings-ledger.md) for ordinary issues; they do not
  need an architectural decision. Anything that proves to depend on a map
  decision is promoted back into its ticket.
- Implementing the redesign, migrating code, or writing the specification itself
  (the specification is the hand-off from this map).
- New product features (LSP server, MCP server, note write-back, output formats)
  as deliverables. They enter the map only as design-pressure scenarios.
