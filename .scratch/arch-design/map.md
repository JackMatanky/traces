# Map: Traces strategic architectural redesign

Label: `wayfinder:map`.
Tracker: local markdown (`docs/agents/issue-tracker.md`).
Tickets: `issues/NN-<slug>.md`.
Mission brief: [mission.md](./mission.md).

## Destination

This map delivers architectural decisions for Traces. The decisions are
coherent and backed by evidence. They are sufficient to write a comprehensive
redesign specification. The decisions cover boundaries, component
responsibilities, and invariant ownership. They cover domain models,
interfaces, algorithms, and data-structure changes. They identify transferable
comparative lessons, foundational investments, and dependency constraints.
Externally observable capabilities and correctness are preserved. This map does
not implement the redesign. It ends at hand-off to specification.

## Notes

- Domain: Rust 2024 CLI, ~94k LoC single crate, template-driven PKM over
  markdown vaults. Read `GLOSSARY-MAP.md` and the per-module `GLOSSARY.md` for
  vocabulary. Existing ADRs are in `docs/adr/` and are fallible context
  (ADR-0009 appears twice).
- Code and tickets disagree by design. The tickets describe the target. The
  code is not yet refactored. Each resolved ticket ends with a "Code delta"
  section that lists the change. Ticket 36 orders those deltas. `path:line`
  citations in tickets refer to the working tree near `cb15b21c`. Ticket-to-ticket
  conflicts are defects. Ticket-to-code gaps are not.
- Ticket renumbering (2026-10-30): the journal measurement and the live-buffer
  ticket moved next to the index tickets. The `research/*-prep/` files and git
  history use the old numbers. The old-to-new map follows.
  - 37 to 26, 34 to 27, 26 to 28, 27 to 29.
  - 28 to 30, 29 to 31, 30 to 32, 31 to 33.
  - 32 to 34, 33 to 35, 35 to 36, 36 to 37.
  - Tickets 1 to 25 and 38 did not move.
- Ticket renumbering (2026-11-09): the replay prototype ticket moved next to
  the index tickets, so the index family reads 24 to 29. Only git history uses
  the old numbers. The old-to-new map follows.
  - 29 to 30, 30 to 31, 31 to 32, 32 to 33.
  - 33 to 34, 34 to 35, 35 to 36, 36 to 37.
  - 37 to 38, 38 to 39, 39 to 29.
  - Tickets 1 to 28 did not move.
- Ticket renumbering (2026-11-09, second pass): the replay prototype moved
  next to the index tickets. The replay prototype is 27, the live-buffer
  ticket is 28, and query core is 29. Only git history uses the old numbers.
  The old-to-new map follows.
  - 27 to 28, 28 to 29, 29 to 27.
  - Tickets outside 27 to 29 did not move.
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
  evaluate, not a decision. The crate-topology and workspace-host tickets must
  test it against keep-as-is.
  Separating the CLI makes it an outward-facing consumer of the core. The core
  crate `pub` surface is then justified by the CLI boundary. This follows the
  visibility rule in the compatibility-envelope answer.
  Sequencing: the CLI split depends on first deciding what the CLI owns today but should not own. Examples
  include composition, scaffolding, query-source normalisation, and task-filter
  synthesis. See the workspace-host ticket.
- Every decision ticket's resolution names: alternatives considered (incl.
  retain), invariant owner, what becomes removable, what remains unresolved, and
  affected downstream tickets.
- Ticket prose (2026-10-25): the `Question` sections of open tickets follow
  ASD-STE100 Simplified Technical English. They use verb-first instructions,
  vertical decision lists, and sentences of 20 words or fewer. The `Inputs from …`
  sections, `Facts:` evidence, and research files stay outside STE.
  Definitions of key terms:
  - `check`: inspect current code now.
  - `verify` or `re-verify`: adversarial claim testing.
  - `validate`: schema validation.
  - `audit`: ticket 36's coverage sweep.
  - `reconcile`: ticket 36's consistency pass.
  - `ratify`: accept the landed design unchanged.
  Never swap `index`, `store` (redb), or `cache`.
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
  (`git show 3d934849^:.scratch/arch-redesign/map.md`). It was deleted at
  `3d934849`. Treated as fallible input only. This map was charted independently
  from code.
- Research deviation: this tracker is local markdown. Research findings land
  in `research/NN-*.md` on the working tree and are linked from the ticket. No
  `research/<name>` git branches are pushed.
- Late precedent research (2026-10-21): IWE and Fileclass were first read from
  source after ticket 20 resolved ([core and
  hosts](./issues/12-research-iwe-core-and-hosts-architecture.md), [query,
  schema and
  gating](./issues/13-research-iwe-query-schema-and-agent-write-gating.md),
  [Fileclass](./issues/14-research-fileclass-model-binding-and-validation.md)).
  Open tickets carry an "Inputs from the IWE (and Fileclass) research" section;
  ticket 20 and 19 carry a dated comment. Digests live in `docs/digests/`.
- Renumbering (2026-10-21): the four IWE/Fileclass research tickets moved to
  12 to 15, after the other research tickets. Earlier tickets 12 to 32 became 16
  to 36 (add 4). The prep folders were renamed to `research/19-prep/` and
  `research/20-prep/` (formerly `15-prep`, `16-prep`). Their ticket references
  were updated by hand. Only git history uses the old numbering.
- Provisionally withstand scrutiny (scouts, unverified; ticket 20 overturned
  the `FileClassExpander` seam):
  - `DatePoint`/`DifferenceEngine` and `DurationValue` nominal identity.
  - Schema Kahn linearisation + ancestor-bounded `$ref` + whole-directory reload.
  - Redb derived-cache design (`Durability::None`, auto-recreate, fail-open).
  - Infallible note parser, `MarkerAccumulator`, single-pass Logos item lexing,
    flat list hierarchy.
  - ADR-0004 CLI error isolation, `CommandOutcome`/`UserAbort`, `CwdGuard`,
    stdout/stderr split.
  - Acyclic 10-tier module DAG; layered test isolation.
  - `LineIndex`/`FieldKey`/`Blake3Hasher` framing.
  The reconciliation ticket (ticket 36) re-tests these items.

## Decisions so far

<!-- one line per closed ticket: [title](link): gist -->

- [Which scout claims survive adversarial
  verification?](./issues/01-research-verify-load-bearing-claims.md): of 14
  claim groups, 12 are confirmed. 1 group is partially confirmed with a
  correction. ISO-string coercion runs only through `From<noyalib::Value>` or
  explicit `Classify`, not default deserialize. 1 group is partly refuted.
  Template `QuerySet` chaining does NOT clone rows; it clones an `Arc` and plan.
  The unconfined-path and DryRun parts of that claim hold. New facts:
  `Config::hash` and `ConfigDigests` were unused in production at charting.
  `ConfigDigests` now feeds the index fingerprint; `Config::hash` stays dead.
  Multi-valued inline fields are write-only. Every output command panics on
  EPIPE. `i64` is lost to `f64` at the note boundary. ADR-0009 (index refresh)
  landed at `207cc3fb`. All other ADRs hold.
- [How do mature engines model dynamic values, and what do they pay for
  it?](./issues/02-research-value-model-precedent.md): no comparable keeps an
  owned and `Ref` enum mirror (Polars `AnyValue<'a>` plus `into_static`).
  Ordering belongs in an external comparator (SQLite `sqlite3MemCompare`,
  DuckDB), not the enum `Ord`. `i64/f64` compare losslessly. Direct
  serde-visitor deserialization removes the 3-stage chain. Boxing `Link` shrinks
  the enum from 64 B to 32 B.
- [Where do mature systems put type coercion and validation, and how do they
  report violations?](./issues/03-research-coercion-validation-precedent.md):
  coercion belongs to query binding and explicit casts (DuckDB `CAST` versus
  `TRY_CAST`, Polars `strict`), not parsing. On-demand advisory note validation
  never gates indexing. Ticket 20 later chooses span-free violations. Ticket 19
  keeps a fixed cross-kind lattice for sorting, while cross-bracket relational
  filters do not match.
- [How do mature CLIs make ambient environment (clock, zone, dirs, env)
  explicit?](./issues/04-research-execution-context-precedent.md): jj and uv split
  context into host environment, workspace, and temporal parts. Do not use one
  large context structure. Sample `now` once and acquire the zone only if needed
  (ticket 21 refinement). Snapshot cwd for workspace discovery. The research
  proposed interactive-only prompts. Ticket 21 permits authored unattended
  defaults but rejects invented answers.
- [How do incremental systems fingerprint inputs and decide
  staleness?](./issues/05-research-freshness-fingerprint-precedent.md): input
  completeness is best owned by the type system (Ruff `CacheKey` derive) over
  hand-listed digests. An index-owned fingerprint over consumed config in redb
  `meta` table suffices. Commit `207cc3fb` built this fingerprint, so this is
  confirmed. Inverted unresolved-stem maps (Obsidian advanced-metadata-cache,
  rumdl) disprove the full-vault rescan on path changes. Git racy-timestamp guard
  was missing from the Traces delta (still missing at `207cc3fb`).
- [How do query engines structure predicate languages, plans, and
  pushdown?](./issues/06-research-query-plan-precedent.md): source selection is
  an indexable subset of predicates, not a separate language (SQLite,
  DataFusion). Pushdown is a 3-state capability on the storage side
  (`TableSource::supports_filters_pushdown`). Dataview prunes through in-memory
  inverted sets. Heavy relational IRs do not fit 1k-20k vaults. `GroupBy` must
  nest or be removed. (One finding on `QuerySet` cloning is superseded; see
  ticket correction.)
- [How do embedded-template hosts bridge domain values and keep date formatting
  coherent?](./issues/07-research-template-bridge-precedent.md): minijinja
  `Object` can carry typed values. It stamps template spans on host errors
  automatically. Bound handles (Typst `Engine`, Rhai context) beat
  `State::set_temp`. Moment tokens are a Templater-compat concern, not a core
  temporal concept (Jiff/chrono use strftime). Moving Moment tokens into
  `temporal` is unjustified.
- [How do mature Rust workspaces structure errors, diagnostics, crate
  boundaries, and test
  seams?](./issues/08-research-errors-topology-precedent.md): a struct error
  facade with hints (jj `CommandError`, uv) removes `private_interfaces`
  suppression. Operational errors stay simple while diagnostics keep spans to
  emission. Rumdl runs 341k LoC in one crate with open modules. No comparable
  has a `testing` backdoor with frozen visibility baselines. Ruff and jj
  downcast `BrokenPipe` to exit 0. A 2-3 crate split (core, test-support, cli)
  is supported. A 10-tier split is not.
- [How do mature tools enforce root confinement and portable path
  identity?](./issues/09-research-capability-path-precedent.md): `cap-std`
  handles give kernel TOCTOU immunity at high cost, disproportionate for a
  single-user CLI. Traces `SafeRelativePath` throws away checked `StrictPath`
  and returns a plain join. Carry a portable UTF-8 `/` relative path in
  persistent structs and confine at the I/O boundary. The template
  path-inspection and DryRun leaks are real.
- [How do markdown/document tools model positions, metadata caches, and
  heading/link
  structure?](./issues/10-research-document-model-spans-precedent.md): spans are
  already free (`into_offset_iter` is used). An 8-byte `ByteSpan` (= ruff
  `TextRange`) with on-demand `LineIndex` is compact. Rumdl proves one pass
  yields slugs. Dataview never rolls task fields into page metadata and squashes
  single occurrences to scalars. Obsidian/Tasks intern status by symbol.
  List items belong as children of the note.
- [What do the existing benchmarks and targeted new measurements say about the
  consequential performance
  claims?](./issues/11-research-performance-evidence.md): on Apple M4 Pro, a
  cold 20k-note query is <75 ms either way. Redb secondary tables halve peak RSS
  (17 vs 39 MB). On path-set change, 97% of cost is deserializing notes (18.5 ms
  @5k) and only 2.7% is link re-resolution (0.5 ms). The inverted-stem index is
  unjustified; the primary lever is note caching/laziness. Flat
  nearest-candidate scan beats `CandidateTrie` below ~5 same-stem lookups (trie
  and `indextree` are removable). The second pulldown-cmark pass takes 20-38% of
  parse time. Status interning cuts 10k `String` allocs and shrinks `ListItem`
  from 160 to ~128 B. A unified value enum measures 80 B (no growth). `QuerySet`
  chaining is free (already lazy). The ~25 ms redb commit floor stems from
  `Database::create` and drop lifecycle, not durability. Re-measured at
  `207cc3fb` with an A-B-A control: the merged drift check adds no detectable
  cost (median +0.7% vs 1.7% noise).
- [How does IWE structure one core shared by a CLI, an LSP, an MCP server and a
  daemon?](./issues/12-research-iwe-core-and-hosts-architecture.md): confirms
  ticket 20 layering. Pure operations return a plain `Changes` set. Hosts apply
  it, with no one-implementor traits. The app layer lives in its own crate
  (`diwe`, for ticket 35). The LSP writes open buffers into the one in-memory
  graph. It drops watcher events for open keys. This works only because IWE has
  no persisted index. No persistent cache: 20k notes cold in 705 ms (M3 Pro,
  synthetic). This provides a cost model for tickets 24 and 25, not a verdict.
  Its lossy tree writer regressed repeatedly. Writers must splice spans and never
  rewrite unchanged bytes (tickets 23, 25).
- [What do IWE's query language, document schema, search and MCP write path
  teach
  Traces?](./issues/13-research-iwe-query-schema-and-agent-write-gating.md):
  IWE query value semantics converge with ticket 19. Its query is a
  serialisable document (question for ticket 29). Schema binds by host-config
  globs, with no inheritance. The MCP gate is always on and all-or-nothing. It
  validates whole pending documents with no delta rule (supports ticket 20 delta
  rule). It requires `expect` count guards on query mutations. Body schemas use
  an ordered greedy section matcher through an external crate (ticket 15).
  Corrects earlier claims: no frontmatter `type` binding, no strict mode switch,
  and reserved-prefix fields are validated.
- [What does Fileclass's source teach about class resolution, typed fields and
  validation?](./issues/14-research-fileclass-model-binding-and-validation.md):
  confirms `global` as the lowest-precedence layer for every note and per-class
  origin. Its code lets a lower-ranked binding override a frontmatter class
  (docs state the opposite). This supports ticket 20 suppression and conflict
  reporting. Its validation is weaker than ticket 20 (`step` is UI-only, `[]`
  satisfies `required`, custom date formats unchecked). 28 field types, about
  ten distinct (ticket 37). No body or buffer model.
- [What are `schematter-validator` and the document-schema
  spec?](./issues/15-research-schematter-validator-and-document-schema.md):
  IWE validator crate (about 1,200 lines, `jsonschema` dependency, one
  implementation, draft spec) is a poor fit. It brings a second schema dialect,
  a recursive section tree, and no spans. Its matcher semantics are reusable as a
  design over Traces flat headings (ticket 38). No dependency decision remains.
- [What counts as the externally observable surface the redesign must
  preserve?](./issues/16-grilling-compatibility-envelope.md): sole consumer, so
  almost everything is Migratable through the specification change table (no
  standing register). Frozen: vault-format parsing, index result equivalence
  with a fresh parse, exit codes, and stdout/stderr split. Also frozen: stdout
  payload shapes, trust-gate behaviour, and template/query capabilities. Free:
  dialect parity with Dataview/Templater, `index.redb` layout and refresh
  strategy, trust symlink layout, unasserted wording, and the Rust library
  surface. Known defects (closed-pipe panic, `i64` to `f64`, `.first()`-only
  multi-values) are not preserved. Visibility rule: reduce visibility wherever
  possible. Limit `pub` to outward-facing components (widening requires an
  outward-facing justification).
- [Which forward-looking consumers and scales must the architecture be shaped
  for?](./issues/17-grilling-forward-consumers-and-design-pressure.md): LSP is
  the next committed consumer; MCP is likely. Support source-aware editor
  features. Keep the core synchronous by default. Assess operation-specific
  performance across the seven target vault sizes. Reopen the note-model choice
  in ticket 23. Resolve open-buffer/index consistency in ticket 27.
- [What is the single dynamic value model, and who owns
  it?](./issues/18-value-model.md): two owned types and no borrowed mirror enum.
  The runtime `NoteFieldValue` (`Int(i64)`/`Float(f64)`, `Link`, derived tagged
  serde, 80 B kept) serves notes, queries, and templates. Ticket 20 later
  requires its owner to sit below both note and schema (ticket 35 chooses its
  exact placement). A schema-local literal type (strings stay strings) replaces
  the schema `FieldValue`. Only a root `Link` (`target`, normalized `display`,
  `embedded`) is shared. `NoteLink` (syntax kind, span) is the editor-side type
  for [the note document model](./issues/23-note-document-model.md). Frontmatter
  keeps the Noyalib compat parser and converts its AST directly. Huge integers
  become `Float` instead of dropping the block. Classification is
  quote-insensitive in frontmatter and shares one parser set with inline fields.
  `FieldValueRef`, `DateTimeCoercion`, and double conversion are removable.
  Ticket 19 later makes naive datetimes civil, so zone input affects evaluation,
  not the persisted note value. Handoffs: [coercion
  policy](./issues/19-coercion-and-comparison-policy.md), [execution
  context](./issues/21-ambient-environment-and-execution-context.md), temporal
  cache encoding ([index read
  model](./issues/24-index-read-model-and-query-execution.md), [index
  freshness](./issues/25-index-freshness-and-invalidation.md)), view enum
  benchmark ([query core](./issues/29-query-core-and-plan.md)).
- [Where do coercion and cross-type comparison live, and what is the
  policy?](./issues/19-coercion-and-comparison-policy.md): `PartialEq` stays
  structural. One value-semantics module owns filter equality and bracketed
  order separately from a total sort order. Across brackets, `==` evaluates to
  false, and relational operators produce no match. Numeric values compare
  exactly. Temporal point or period filter membership is not sort equality. Sort
  sniffing is deleted. Unicode collation (`icu_collator` root, numeric) is
  sort-only; `NaN` becomes `Null`. Naive datetimes become civil and the cache
  holds no zone. Classification stays quote-insensitive. Quoted query literals
  adapt per row using pre-parsed candidates, including `CalendarPeriod` for
  date-shaped comparison. Period literals match whole spans with an exclusive
  upper bound. Two periods match only by normalized identity. Typed temporal and
  environmental failures are distinct from nonmatching data. Link equality uses
  the folded target, while `date(link)` reads the target. Two advisory stderr
  notices are retained. Handoffs: [query core](./issues/29-query-core-and-plan.md),
  [temporal boundary](./issues/30-temporal-boundary-and-dialects.md) (natural
  language grammar, `DateConfig`), [execution
  context](./issues/21-ambient-environment-and-execution-context.md), [schema
  typing](./issues/20-schema-note-typing-and-validation.md), [index
  model](./issues/24-index-read-model-and-query-execution.md).
- [Does the schema subsystem validate and type note data, and where does that
  capability live?](./issues/20-schema-note-typing-and-validation.md): advisory,
  on-demand check. No parse-time typing, no read-path gating, and schema never
  enters the index or fingerprint. `schema` owns the pure, span-free rule over
  resolved effective fields. The runtime value type sits below `schema` and
  `note` (placement to 34). Gating is a host policy over stable violation codes
  carrying a layer (`class` or `global`). Layering principle: cli, lsp, and mcp
  are thin hosts of core. Core app layer composes domain modules with plain data
  and no one-implementor traits. `FileClassExpander` and its crate-root impl are
  deleted for a plain class hierarchy. Multiple classes resolve
  order-independently. Differing declarations are reported once and left
  unchecked; ancestors are reduced away. Multiple `extends` parents are kept.
  `global` becomes the lowest-precedence layer for every note. Class binding by
  tag and path (class-owned `[binding]`) supplies classes to notes without an
  explicit class. This expands at query time so the index stays schema-free.
  `required` is kept. `step` is checked. `date.format` is a hint until 29.
  `origin`, `description`, and `parent_order` are added. Template check is a
  post-render advisory use case. No design depends on harness hooks. The
  portable guarantee is detection and feedback through tiered channels (lint
  command, git, read notice, LSP). Prevention operates only by placement
  (isolated copy or inbox, outside destination). Prevention also applies where
  MCP tools own every write, with host-run guards over the stateless 2026-07-28
  protocol. `traces check` takes paths and splits schema errors from violations. Delta rule applies. A class-set change prompts the user. Date-shaped text: no override, no
  quote convention. Body-section schemas form a separate ticket. Handoffs:
  [query core](./issues/29-query-core-and-plan.md), [note document
  model](./issues/23-note-document-model.md), [temporal
  boundary](./issues/30-temporal-boundary-and-dialects.md), [template
  boundary](./issues/31-template-engine-boundary.md),
  [diagnostics](./issues/34-error-and-diagnostic-architecture.md), [crate
  topology](./issues/35-crate-topology-and-public-surface.md), [schema
  internals](./issues/37-schema-internals-simplification.md), [body
  sections](./issues/38-body-section-schemas.md).
- [What is the explicit execution context, and what ambient state is
  eliminated?](./issues/21-ambient-environment-and-execution-context.md):
  adapters snapshot OS inputs at workspace opening. Core workspaces own
  independent policy/config generations, rechecking local trust at each
  request admission. One frame per query/render/request pins clock, week
  rule, and lazy zone resolution. Lazy results memoize errors. A civil
  `CalendarPeriod` (start and `CalendarUnit`) supplies filename-only `file.date`,
  which never falls back to `file.cdate`. Creation fields never fall back to
  mtime. Whole-period comparisons use the instant's local civil date. Period
  identity and sorting remain distinct. Authored defaults work unattended;
  missing answers fail. Multi-root construction is supported without a
  registry. Same-root index writer coordination stays with tickets
  24/25/32/33. Handoffs: [config](./issues/22-config-ownership-and-lifecycle.md),
  [read model](./issues/24-index-read-model-and-query-execution.md),
  [freshness](./issues/25-index-freshness-and-invalidation.md),
  [query](./issues/29-query-core-and-plan.md),
  [temporal](./issues/30-temporal-boundary-and-dialects.md),
  [templates](./issues/31-template-engine-boundary.md),
  [paths](./issues/32-path-and-capability-model.md),
  [host](./issues/33-workspace-host-and-composition.md),
  [errors](./issues/34-error-and-diagnostic-architecture.md),
  [topology](./issues/35-crate-topology-and-public-surface.md),
  [reconciliation](./issues/36-reconcile-decisions-and-audit-coverage.md).
- [Who owns config-derived types, trust/tracking state, and the config
  lifecycle?](./issues/22-config-ownership-and-lifecycle.md): the root types
  `task`, `tag`, and `field` stay as shared vocabulary. `note` owns its parser input, so `note` no longer imports `config`. `ConfigDigests` is deleted. `index` owns
  `IndexPolicy` as the only source of the fingerprint. One exhaustive
  projection builds it from `Config`. Ticket 25 keeps content and granularity.
  Two trust and tracker stores remain. `ConfigService` is the only writer.
  Root-only grants and `MissingBaseline` are removed. The `ConfigFile` typestate
  retains a store-level trust check. Adapters build `HostInputs`.
  `ConfigService` owns its locator. A relative path variable is an error.
  `init` is a core use case, trusting only a workspace that it created. `Config`
  carries a stamp. `ConfigService::admit` is the admission primitive. Core holds
  no workspace object (ticket 33).
- [What is the note document model: spans, facts, items, multi-value
  metadata?](./issues/23-note-document-model.md): Persist one facts type,
  `Note`, with no byte span, footnote or callout. `Note` is an assembled view of
  independent facets: meta, headings, links and items. Add `NoteDoc`: a flat set
  of ranges plus a line index, built by a separate scan from bytes in hand and
  never persisted. Both builders call one rules module. The host owns the text
  and the version. A stored position never drives an edit. A task is a kind of
  list item. It stores the status symbol and kind, not the name. Keep the stored
  note-level tags and inline fields. Two maps and one note-owned effective
  accessor. Several values form a list; ticket 29 adds membership. External URLs
  leave the stored links. The math pass runs only for a heading with `$`.
  Notices return beside the `Note`. Handoffs: [read
  model](./issues/24-index-read-model-and-query-execution.md),
  [freshness](./issues/25-index-freshness-and-invalidation.md),
  [query](./issues/29-query-core-and-plan.md),
  [template](./issues/31-template-engine-boundary.md),
  [host](./issues/33-workspace-host-and-composition.md),
  [errors](./issues/34-error-and-diagnostic-architecture.md),
  [topology](./issues/35-crate-topology-and-public-surface.md), [live
  buffer](./issues/28-live-buffer-and-persisted-index-consistency.md),
  [schema](./issues/37-schema-internals-simplification.md), [body
  sections](./issues/38-body-section-schemas.md).
- [What is the index read model, and is there one query execution path or
  two?](./issues/24-index-read-model-and-query-execution.md): one query plan
  over a facet store and immutable host generations. Cold queries load only selected entries. `WorkspaceIndex` stays total. The index owns an
  engine-neutral port, with redb as the first Rust-native adapter. Store
  snapshots and writer-transaction base checks prevent mixed generations.
  `lists` keeps tasks; `tasks` duplicates them in one atomic write. Note-level
  scalar range keys and task date/status keys need semantic proof and end-to-end
  measurement. Sorts stay in the query plan. An unusable derived cache falls
  back to a trusted fresh parse for read queries, while `traces index` fails
  closed. Ticket 25 owns freshness and crash recovery. Tickets 29, 32, 33, and
  34 own rows, paths, writer admission, and notices.
- [Does the landed index freshness model stand?](./issues/25-index-freshness-and-invalidation.md):
  config drift detection triggers a full rebuild. Store identity uses an engine
  tag, platform family, format epoch, extractor fingerprint, and policy
  fingerprint (`IndexPolicy::fingerprint()`). A higher epoch triggers a read
  fallback to fresh parsing. Crash recovery is a no-op refresh of uncommitted
  files. Ticket 26 (journal measurement) is decoupled. Content comparison
  detects edits when size and mtime match; a file row stores a racy bit.
  ADR-0009 text and the glossary fingerprint definition are fallible and partly
  stale. Handoffs: [live
  buffer](./issues/28-live-buffer-and-persisted-index-consistency.md), [query
  core](./issues/29-query-core-and-plan.md),
  [host](./issues/33-workspace-host-and-composition.md),
  [diagnostics](./issues/34-error-and-diagnostic-architecture.md).
- [What does a durable index repair journal cost and
  save?](./issues/26-measure-index-recovery-journal.md): cost screen measured,
  no journal chosen. Against the loose saved-work upper bound no candidate
  rejects. At 50,000 notes the external facts journal costs 76 ms per command
  against a 1,332.9 ms bound for rebuilds. The bound counts the persist that
  a replay still pays, so this is not proof that a journal saves. The in-store
  journal table costs more than the external file at every measured size (477
  ms added per rebuild). Break-even rates for edits are provisional and
  unmeasured. No-change commands never pass. A kill before the last rebuild
  sync leaves an empty store in the sample of 9 of 15 stop points. The next
  open costs 17 to 37 ms. The claims of 85 percent more write time and 2 times
  size for a facet layout stay untested. The screen split rows round-robin,
  not the decided layout. The `None` versus `Immediate` choice stays with
  ticket 36. Handoff:
  [replay prototype](./issues/27-index-replay-prototype-and-fault-tests.md).
- [Does the index replay prototype survive fault tests, and what does
  recovery cost?](./issues/27-index-replay-prototype-and-fault-tests.md):
  the re-targeted throwaway probe passed 42 of 42 release-profile fault
  tests on macOS against the real `.traces/index.redb` layout. It proved
  the ticket-25 fork hole. Incarnation plus revision is not enough; the
  digest backstop is required. It also proved full identity-mutation
  handling, signal safety, hostile-body safety, and real-`ENOSPC`
  behavior.
  Measurement found
  no winning policy. No-journal `Immediate` wins the write path. A
  retained journal wins the 50,000-note read-only refresh. Replay
  decode alone runs about 60 times the 53.3 ms write budget. Real
  `ENOSPC` under `Durability::None` is silent sparse-write loss with no
  catchable error, a new adoption blocker. Ticket 27 resolved without an
  adoption verdict. Handoffs: [ticket
  36](./issues/36-reconcile-decisions-and-audit-coverage.md) (adoption)
  and ticket 33 (freshness bound data). Probe assets live on branch
  `scratch/t27-probe-worktree`.

## Not yet specified

- **Schema internals**: graduated to [What schema internals survive now that
  schema produces a checker and a class
  hierarchy?](./issues/37-schema-internals-simplification.md).
- **Note parser internals**: graduated. [Ticket
  23](./issues/23-note-document-model.md) decides status interning, the second
  pass, the footprint, and footnote, callout and slug modelling. Only the
  measurements it lists remain.
- **Index internals**: graduated to [the index read model
  decision](./issues/24-index-read-model-and-query-execution.md). Exact key
  encoding, task-heavy and real-vault measurements, and cross-platform locking
  remain evidence gates.
- **Template internals**: compiled-AST and regex caches, lazy schema load.
  Sharpen after [What is the template engine boundary with query, schema,
  temporal, and the host?](./issues/31-template-engine-boundary.md).
- **Output formats** (`--format json|csv`) and where formatting lives; **async
  dialog bridging** for a long-lived host. Both hang on the forward-consumer and
  workspace-host decisions.
- **Persistence portability** (cache across OSes) hangs on the compatibility
  envelope and the path model.
- **Agent workflows around the check** (validated CLI write commands, isolated
  session or inbox with a gated merge, per-folder instruction snippets): product
  features outside the destination. Only their pressure on core (a change-set
  check with a base and destination path) is recorded in [ticket
  20](./issues/20-schema-note-typing-and-validation.md) and [ticket
  33](./issues/33-workspace-host-and-composition.md).

## Out of scope

- Tactical defects and mechanical cleanups surfaced by the survey
  (`cmp_document_order` tie-break, EPIPE panic, dependency pruning, glossary
  drift, ADR renumbering, config default asymmetry, etc.): recorded in
  [findings-ledger.md](./findings-ledger.md) for ordinary issues. They do not
  need an architectural decision. Anything that proves to depend on a map
  decision is promoted back into its ticket.
- Implementing the redesign, migrating code, or writing the specification itself
  (the specification is the hand-off from this map).
- New product features (LSP server, MCP server, note write-back, output formats)
  as deliverables. They enter the map only as design-pressure scenarios.
