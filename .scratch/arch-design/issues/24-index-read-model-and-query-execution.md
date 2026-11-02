# What is the index read model, and is there one query execution path or two?

Type: grilling
Status: resolved
Blocked by: 11, 17, 21, 23, 1, 12

## Question

`QueryService` has two execution models. The first is an in-memory linear scan
over `WorkspaceIndex`. The second is the redb secondary-table pushdown
(`run_from_store`). The pushdown builds a partial `WorkspaceIndex` only to make
rows. redb keeps four multimap tables only for the pushdown.

Topic A frames the other topics. The plan of topic A depends on the facet split
of topic D. Decide the facet split first.

### A. Consumers and execution paths

- List each consumer of the index. Use the inventory in the Facts section.
- Decide if the consumers use one execution path or two.
- Decide the plan for each query shape. Include these source shapes: `All`,
  `Not`, `Path`, `Tag`, `Class`, and a filter with no source.
- Decide which consumer needs the store to answer a query without a load of
  all notes. State the query shape and the vault size.
- Decide if the cold CLI and a long-lived host (ticket 17) use the same read
  model.

### B. Read model shape and generation

- Decide the shape of the read model. The options are an immutable all-entries
  snapshot, a lazy store-backed view, and columnar facts.
- Decide the unit of one coherent generation (ticket 21). A query reads one
  generation. A refresh from a different policy never mixes rows of two
  generations.
- Decide how a long-lived host moves to a new generation. It can replace the
  snapshot, or it can update the snapshot in place.
- Decide one rule for a failed persist. `refresh` fails open. `refresh_store`
  fails closed. Apply the rule to all consumers.
- Decide how one process admits an index writer when another process holds the
  store (tickets 21 and 23).

### C. Rows

- Decide what a `QueryRow` borrows from. Ticket 28 owns the row shape. This
  ticket owns the source of the borrow.
- State what a row needs: the `Note` facets, the `FileMeta`, the inlinks, the
  item identity `item_idx` and the document order (ticket 28).

### D. What the store persists

- Decide what the store persists. The options are a whole `Note` with postcard,
  independent facets, and extracted facts.
- If you choose facets, choose the split of meta, headings, links and items.
  Choose what loads when. A task query needs the heading ranges above the task
  (ticket 23).
- Decide if the item text stays in the main record or moves out of it.
- Decide which secondary tables stay. For each table, name the query that needs
  it. Each candidate set must agree with the filter relation (ticket 19).
- Decide how the store keeps the inbound link graph (`links`). It can persist
  the graph, or the refresh can compute it in memory.
- Decide the `FileMeta` fields. Keep the source timestamps `created_at` and
  `modified_at`. Decide if the store persists the filename period or derives it
  at query time (tickets 21 and 29).
- Decide the encoding of temporal values. Today the store keeps text and parses
  it on every load. A compact binary form is an option.
- Decide if the store keeps the parser notices. Ticket 23 decided that the bulk
  index path collects them. No code does so today (tickets 23 and 33).
- Decide where the lookup "has a known explicit class" lives (ticket 20).
- Decide if the class axes index inline fields. Today they index frontmatter
  values only.
- State the constraint on the persisted path form. Ticket 31 owns the form.
- Decide if any field gets an order-preserving key table for a native range
  filter or sort. Name the query and the measured gain. Ticket 19 forbids
  collation keys, so string sorts stay in memory. Each candidate set must
  agree with the filter relation.

### E. Totality

- Decide if the totality of `WorkspaceIndex` is a type-enforced invariant. It
  holds one entry for each file, sorted by path.
- Decide if `SortedByPath::assumed_sorted` checks the order in release builds.
- Decide if a partial index type exists. Today the pushdown builds a partial
  `WorkspaceIndex`.

### F. Index internals

- Decide if `CandidateTrie` and `indextree` stay.
- Decide if the store engine and the repository split into two types.
- Decide the redb settings that the store sets: the cache size, the page size
  and the durability. The default cache is 1 GiB. `create_db` keeps it.

### Boundaries with other tickets

- Ticket 25 decides the format version rule, the parser identity, the racy
  timestamps and the visibility of heals. This ticket decides the facet split
  and the lazy decode. Ticket 25 decides if a refresh keeps an in-memory note
  cache. The facet split also sets what a path-set change decodes. One added
  file costs 499 ms and 395 MB at 50,000 notes today.
- Ticket 28 decides the row type, the view enum, the pushdown protocol and the
  membership operation.
- Ticket 31 decides the persisted path form and the identity of a root.
- Ticket 32 decides the held workspace and the writer admission for a root.
- Ticket 27 decides how an open buffer overlays the persisted generation.
- Ticket 33 decides how a notice shows.
- Ticket 34 decides the visibility of `IndexStore` and `WorkspaceIndex`.

### Compatibility rules

- Index results must equal the results of a fresh parse. This rule is Frozen
  (ticket 16).
- The `index.redb` layout, the versioning, the refresh and the drift strategy
  are Free (ticket 16).
- Exit codes and the stdout and stderr split are Frozen. If the failure rule
  changes an exit code, add a change-table row.
- Reduce visibility wherever possible (ticket 16). Use `pub` only for an
  outward-facing component.
- The map Notes record a steer to split the CLI into its own crate. The CLI is
  then an outward-facing consumer. State the read-model surface that the CLI
  uses.

### Evidence gate

Decide only after you see the evidence. Research 11 gives these numbers on an
Apple M4 Pro for a rare-tag query:

- A 20k-note cold query takes 60 ms with the store pushdown. It takes 74 ms
  with a full load.
- Peak RSS is 17 MB with the pushdown. It is 39 MB with a full load.
- The redb commit floor of about 25 ms is the open and close lifecycle. It is
  not durability.
- On a path-set change at 5,000 notes, note decoding takes 18.48 ms. Link
  re-resolution takes 0.52 ms.

Both paths are fast. The contested resource is memory and code. The code is the
four tables, `SourceResolver` and the partial index.

Measure these before you decide:

- The seven vault sizes of ticket 17: 50, 200, 1,000, 5,000, 10,000, 20,000
  and 50,000 notes. Research 11 covers 1,000, 5,000 and 20,000. The prep probe
  covers 1,000 to 50,000 on a synthetic vault. Repeat it on a real vault.
- The query shapes of topic A. Research 11 measured one rare tag only. The prep
  probe measured every shape except `Class`.
- The decode cost and the encoded size of each facet. Ticket 23 estimates that
  a decoded note takes about twice its encoded size. The prep probe suggests
  about 7 KB for a note of 2.1 KB. A scan of 50,000 stand-in meta values
  takes 21 ms. Decode the real meta and outlinks facets at 50,000 notes.
- The host memory at 50,000 notes (ticket 23).
- A writer in each of two processes. The prep probe measured a second reader
  and a second opener on macOS only.
- The compact temporal encoding (ticket 18).
- The boxed and the unboxed task payload (ticket 23).

Ticket 17 does not need equal optimization at every size. Choose the range
of each operation from evidence.

### Fixed inputs

Do not reopen these decisions.

- Index results equal a fresh parse (ticket 16).
- The store is a derived cache. A lost or corrupt store heals by a rebuild.
- The persisted content is the facts-only `Note`. It has no byte span,
  footnote, callout, slug or external URL. `NoteDoc` never enters the store
  (ticket 23). Topic D decides the physical layout.
- The schema never enters the index or the fingerprint. Class binding expands
  at query time (ticket 20).
- The store never holds a collation sort key (ticket 19).
- Naive datetimes are civil. The cache holds no zone (ticket 19).
- `index` owns `IndexPolicy`. It is the only source of the fingerprint.
  `ConfigDigests` is deleted (ticket 22).
- `NoteFieldValue` keeps derived tagged serde and the 80 B size test
  (ticket 18).
- The core stays synchronous by default (ticket 17).
- Store crates are Rust-native only. C and C++ engines are out of scope.

### Facts

- `survey/index.md` §2-3 and H1/H7, and `survey/query.md` §3.
- `src/query/service.rs:130-170` (`run_from_store`) and `:336-427`
  (`SourceResolver`).
- `src/index/entry.rs:80-103` and `src/index/sort.rs:48-57`
  (`SortedByPath::assumed_sorted`, debug-only check).
- `src/index/tables.rs`: eight tables. They are `files`, `notes`, `links`,
  `paths_by_tag`, `paths_by_file_class`, `tags_by_path`, `classes_by_path` and
  `meta`.
- Commit floor of about 34 ms in `benches/index_store.rs:113-115`. Research 11
  measures 25 ms.
- Consumers of the index [V: `survey/index.md` §1]:
  - The cold CLI commands `list`, `table` and `task` call `refresh_store` and
    `run_from_store`.
  - `traces index` calls `rebuild`.
  - The template engine calls `refresh` once for each render. It caches the
    `WorkspaceIndex` in the minijinja `State`.
  - Tickets 17 and 20 add the LSP, the MCP server and `traces check`.
- `SourceResolver` reads the path list of the whole store for source `All`, for
  `Not`, for an empty `And` and for each `Path` atom (`src/query/service.rs:353,
  370, 389, 418`) [V]. A query with no source therefore decodes every note on
  the store path. Only `Tag` and `Class` atoms use a secondary table [V]. The
  prep probe measured the shapes (see below).
- `refresh` logs a persist failure and returns the in-memory index.
  `refresh_store` returns `IndexError::Store` (`src/index/service.rs:259-264,
  285-293`) [V]. The two paths fail in different ways.
- `IndexStore::create_db` has no special case for a store that another process
  holds. Every redb error that is not corruption becomes `StoreError::Redb`
  (`src/index/store.rs:894-921`) [V]. A second process fails at once with
  `DatabaseAlreadyOpen`. The real CLI exits with code 1 and prints "Database
  already open. Cannot acquire lock." [V: prep probe]. `ReadOnlyDatabase` does
  not open beside a writer. The redb `experimental-multiprocess` mode lets a
  second process read and write. It refuses `Durability::None`. Research 11
  advises that a host holds an open `IndexStore`. A held store makes every CLI
  call fail.
- `FileMeta` persists `created_at`, `modified_at`, `size` and `day`
  (`src/file.rs:45-53`) [V]. `FileMeta::derive_day` falls back to `created_at`.
  `DateValue::try_from(SystemTime)` converts it in the current local zone
  (`src/file.rs:88-98`, `src/temporal/date.rs:178-191`) [V]. The persisted `day`
  therefore holds a zone-derived date.
- The store path and the memory path disagree when no class expander is
  attached. The store path matches the exact class. The memory path matches
  nothing (`src/query/service.rs:402-406`, `src/query/grammar/source.rs:228-229`)
  [V]. Ticket 20 says that production always attaches an expander. Two paths
  still cost one more place to keep the same rule.
- Two of the four multimap tables have no query reader. `tags_by_path` and
  `classes_by_path` serve the incremental writer only
  [V: `research/24-prep/a1-index-query-code.md`, section D].
- `IndexerService::load` has no production caller. The CLI calls
  `refresh_store`. The template engine calls `refresh` [V].
- Prep probe on a synthetic vault of 2.1 KB notes
  (`research/24-prep/g-probe-results.md`) [V]:
  - At 50,000 notes, a query with a `Tag` or `Path` source takes 117 ms to
    127 ms and 35 MB to 52 MB.
  - At 50,000 notes, a query with no source, a `Not` source or a filter only
    takes 245 ms to 259 ms and 389 MB to 393 MB.
  - The memory ratio between these two groups is 9 to 1 at 20,000 notes. The
    ratio depends on the note content.
  - One added file makes the next query take 499 ms and 395 MB at 50,000
    notes, even for a rare tag.
- Research 11 measured `CandidateTrie` for 8 to 512 same-stem candidates. It
  recommends removal. A prep probe used 5,000 same-stem candidates. A flat scan
  adds about 200 ms to a full build on 12 cores
  [V: `research/24-prep/h1-stress-test-designs.md`, section 10]. The trie serves
  link resolution by basename. It does not serve a source lookup.

### Alternatives

- Use a pure in-memory snapshot. Drop the four tables.
- Use an in-memory snapshot with in-memory inverted sets that the load builds
  (the Dataview pattern). The store keeps `files`, `notes` and `meta` only.
- Keep the dual path. Formalise a pushdown trait.
- Use store-backed lazy rows.
- Split the stored `Note` into facets. Load a facet on demand.
- Use one plan over a facet store. The facet scan replaces the multimap tables.
- Keep two forward tables as `Inexact` pruning over a facet store. Drop the two
  reverse tables.
- Keep as is.

### Resolution must name

- The alternatives considered, including the retain option.
- The owner of each invariant.
- What becomes removable.
- What remains unresolved.
- The change-table rows (ticket 16).
- The downstream tickets that change: 25, 26, 27, 28, 30, 31, 32, 33 and 34.

This ticket graduates these fog items: `CandidateTrie` and `indextree`
retention, the store engine and repository split, and inline-field class
extraction.

Re-verified (main @ `39e96041`). `src/` equals `16f67ef0`. Since `207cc3fb`, the
only code change in `src/index/` and `src/query/` is the `DirTree` walk seam in
`scan`, plus one test. The rest is doc-comment reflow. Therefore the fabricated
partial `WorkspaceIndex` and the `Not` full scan stand. The cold path reads the
fingerprint in the plan's existing read transaction. If the fingerprint
drifts, the cold path rebuilds fully in memory before it serves.

Prep: `research/24-prep/`. The grilling guide is `z-grilling-guide.md`. The
probe numbers come from a synthetic vault.

## Answer

### Decisions recorded in the grilling session (2026-10-29)

The maintainer confirmed this design after the adversarial review.
The evidence is in `research/24-prep/g-probe-results.md` and
`i1-store-candidates-and-designs.md`.

- The index owns a domain-level port to the store. It names operations such as
  candidate paths for a source, facets for a set of paths, an apply of changes
  and the meta values. A cold query uses one read snapshot for candidate
  selection, all selected facets and fingerprint validation. Query code
  never names an engine. This resolves topic F for the store engine and
  the repository.
- Five rules keep the port portable:
  1. No engine type crosses the store boundary. This includes errors. The error
     wraps the engine error as an opaque source.
  2. The persisted bytes are engine-neutral: postcard facets and plain byte
     keys. Ticket 31 still owns portable path identity and key ordering.
  3. Store identity records the engine tag, platform family, format epoch,
     extractor fingerprint and policy fingerprint (ticket 25). Ticket 25 owns
     the store identity and invalidation rules.
  4. A conformance suite checks that every adapter returns the same rows as a
     fresh parse. The probe check is its seed.
  5. Open cost, durability and locking stay inside the adapter.
- The engine choice is reversible. The first adapter is redb, because the
  repository uses it. The store is a derived cache, so a switch needs a rebuild
  and no data migration.
- Facet names are `files`, `fields`, `tags`, `classes`, `headings`,
  `outlinks`, `links`, `tasks`, `lists`, `notices` and `meta`. The `meta`
  table keeps the format epoch, extractor fingerprint, policy fingerprint,
  engine tag, platform family, written-by version, store incarnation and
  revision (ticket 25). The `tags` table merges body and frontmatter tags.
- Query latency takes priority over write cost. Persist range keys for
  eligible effective note-level scalar fields, not a fixed list of field
  names. Frontmatter wins over inline, including frontmatter `null`.
  Several inline values become a list, not several scalar keys (ticket 23).
  Keep task-item metadata in a separate scope. Index supported task
  dates and status. Scan other task-item fields until keys have a
  proved and measured benefit (Q21). Keep the original `Note::lists`
  index as `item_idx` in each task candidate. A candidate lookup only
  narrows. The plan still evaluates the source and full filter on each
  survivor, then runs ordered transforms at their original positions.
  Only push a predicate before `limit`, `flatten` or another transform if
  equivalence is proved for that row mode (ticket 28).
- The planner may use a range key only if its candidate set contains
  every row matching the filter (ticket 19). Prove this for each
  operator, row kind, temporal frame and field precedence. Unsupported
  mappings scan the relevant facet. Examples include complements,
  missing or null values, and adaptive or period comparisons.
  Ticket 28 owns the pushdown capability protocol.
- Also prove exact `Int`/`Float` comparisons above 2^53, signed zero,
  infinity, adaptive quoted literals and Link target matching
  (ticket 19). Do not cast `i64` to `f64` or treat a raw string key
  as a Link comparison key. Unsupported operations scan.
- Range keys serve only proved candidate filtering (Q22). Keep sorts
  in the ordered query plan, including its tie and missing-value
  rules. Ticket 19 forbids persisted string collation keys. Numeric
  or temporal sort pushdown needs full-row parity and a measured gain.
- The probe measured tag keys and one status-plus-rating key at 50,000 notes.
  It did not measure generic field keys, task keys or full row materialization.
  The maintainer kept broad scalar keys as a goal in Q18, conditional on
  proved candidate soundness and measured full-query benefit over facet
  scans. Measure a real vault and a task-heavy shape, cache bytes, and
  one-note replacement with stale-key deletion. If broad keys lose, revisit
  their scope instead of shipping them for their own sake.
- Use one execution plan for the cold CLI and each long-lived host. The
  CLI reads store-backed candidates. A host reads its pinned generation.
  Candidate access may differ, but source, filter and transform rules do
  not.
- For `All`, `Not` and a source-less query, enumerate `files` paths,
  then narrow with safe predicates when available. A folder `Path` uses a
  path range. Other path patterns scan paths. `Tag` uses tag keys.
  `Class` checks explicit class keys or the bound tag and path atoms
  from ticket 20. A query that returns every full note still reads every
  selected note. Indexing cannot avoid the output cost.
- Keep `lists` as the complete ordered sequence, including tasks. The
  `tasks` facet is a derived duplicate subset keyed by the original
  `Note::lists` index. Update both in one transaction. Task-only scans do not
  need to decode plain items. Full task rows still load any list context and
  note facets that they use. The synthetic 50,000-note probe scanned tasks in
  12 ms, versus 54-55 ms for all list items. Neither run built query rows.
  Confirm the choice on assembled rows, task-heavy input and one-note
  replacement before fixing the layout.
- A host publishes an immutable generation of shared entries. A refresh
  replaces entries if their note, file metadata or inlinks change. Each
  request pins one generation. A row owns its matching `Arc<FileEntry>`, not
  the whole index. The cold CLI builds entries only for survivors. Ticket 28
  owns the final row API.
- `WorkspaceIndex` remains total and sorted, with one entry per indexed
  file. The cold CLI builds selected `FileEntry` rows directly, not a
  partial `WorkspaceIndex`. The CLI crate consumes the index-owned
  domain port. It never imports a storage engine.
- Keep item text with each task or list record. The selective two-stage probe
  saved about 1 ms by splitting core and text. Revisit the split if task
  range keys fail to narrow broad reads.
- Treat an unusable derived cache (locked, corrupt or unwritable) as
  a miss under these rules:
  - Do not bypass config trust or root validation. The CLI loads
    trusted config before a query (`src/cli/mod.rs:190-204,248-253`).
  - A read query parses source files and prints one stderr cache notice.
    It succeeds only if the fresh parse and query succeed.
  - A host may publish a coherent updated in-memory generation after
    failed persistence only from a validated pinned base. It must not
    fetch unchanged rows from a store that may have changed.
  - `traces index` fails closed. A held store can now serve a fresh
    query and exit 0 instead of 1. Record the change under ticket 16.
- A host opens the store only to commit a write batch, then closes it. Host
  reads come from the pinned generation and do not touch the store. A CLI
  process in another terminal can query if it can parse the source files.
  Each batch pays one store open (about 13 ms measured). This supersedes
  research 11 advice 8, which held an open store in the host.
- Check path ordering in release at every index build and store load, with
  `OsStr` comparison (about 0.12 ms at 50,000 paths). Keep
  `SortedByPath::assumed_sorted` for callers that reorder in memory. The
  boundary checks then witness the order that binary search needs.
- Keep `CandidateTrie` and `indextree`. They resolve links by basename. At
  5,000 same-stem candidates the trie build took 224 ms and a flat scan took
  412 to 436 ms. Extend the `index_inlinks` benchmark to 5,000 candidates.
- Persist inbound links for cold `file.inlinks` reads, including targets
  that are not Markdown. A refresh updates links with affected outlinks in
  one generation.
- Persist only parse-time notices with their file facts. The store
  offers a path-scoped notice read for check, CLI and host diagnostics.
  Query-time notices are computed for each query and are not persisted.
  Ticket 33 owns how consumers show both kinds. Keep temporal values
  as text until a compact encoding shows a measured gain.
- Persist the source `FileMeta` timestamps and size, but not the hybrid
  `day` field (ticket 21). Derive a filename period at query time.
  `classes` holds explicit classes only. Class binding stays outside the
  index and inline fields do not supply a class axis (ticket 20).
- For the first redb adapter, keep `Durability::None` and the default
  page size. The store is a rebuildable cache, not crash-durable data.
  On reopen, validate the format, policy and source freshness, and
  rebuild an unusable or stale cache. Ticket 25 must cover a crash that loses
  the last commit and a same-mtime content edit. Format validity alone does not
  establish freshness. Measure cache size on the new layout before fixing it. An
  earlier 8 MB cache saved 65 MB on a worst-shape synthetic query without a
  measured time cost. That probe predates the range tables.
- Still open: the precise range-key encoding and measured write and
  read costs.

### Adversarial review and required corrections

- [V] `RefreshPlan::collect` reads the store fingerprint and scans source
  files before it builds a delta (`src/index/refresh.rs:337-358`). The
  proposed host releases the store between write batches. [H] Another
  policy can rebuild the store after planning but before commit. An
  incremental apply could then label mixed-policy rows as current.
  Use a writer-transaction check of the plan's base generation and
  fingerprint. A failed check needs a complete replan, not an
- The maintainer chose the writer-transaction base check in Q16.
  Give each store incarnation a unique identity. Do not reset a token to a
  reusable value after a rebuild. Every commit advances its revision, even
  under the same policy.

  An incremental plan carries the base identity, revision and fingerprint.
  Compare them inside the write transaction before replacing any row. On
  mismatch, discard the delta and replan against the new base or rebuild.
  Never retag unchanged rows from a different policy.
- [V] The current store has separate read transactions for file and
  fingerprint reads and for other facet reads (`src/index/store.rs:486-545`).
  A cold query must pin one read transaction through candidate selection,
  facet reads and fingerprint validation. A host snapshot cannot lazily
  read facets from a newer store generation.
- [V] `read_notes_batch` can skip corrupt rows after logging a message
  (`src/index/store.rs:201-209`). Define which facets have required
  rows and how an empty optional facet is represented. A missing or
  corrupt required facet must cause a complete cache miss, not a
  silently incomplete result. On fail-open, parse fresh or reuse a
  validated pinned host generation. Do not mix fresh rows with
  unverified store rows.
- [V] The 1.8 s figure in the prior prep includes a full build with
  persistence (`research/24-prep/h1-stress-test-designs.md:98-100`).
  It is not a measured parse-only fallback latency.
- [V] A task-row metadata field can override its note's value
  (`src/query/results.rs:320-333`). Indexing only note fields cannot
  safely prune task-row filters. The Q21 decision scans non-date and
  non-status task-item fields. Current `Note::fields()` enumerates
  inline values while `Note::get()` reads only the first
  (`src/note/model.rs:142-173`). Ticket 23 changes several inline
  values to one list. The key writer must use that future effective
  accessor, not the current iterator.
- [V] `limit` and `flatten` are ordered lazy transforms
  (`src/query/plan.rs:38-58,238-277`). Moving a filter before either can
  change the result. A source `Not` also needs the `files` universe,
  including non-note entries (`src/query/service.rs:348-390`).
- [V] An unknown explicit class does not suppress tag-bound class
  membership (ticket 20). The class source checks a known explicit
  class, the hierarchy and binding exclusions, not merely any
  `classes` key. Source tags describe note membership. Task-row tags describe an
  item, so they need distinct candidate scopes.
- [V] Today list queries include task items in the same document order
  (`src/query/service.rs:246-263,280-305`). The split does not yet say if
  `lists` duplicates tasks. Disjoint facets need an `item_idx` merge;
  duplication needs atomic updates to both. The task-only scan did not
  build rows (`research/24-prep/g-probe-results.md:250-274`).
- [V] Two prototype range tables grew the redb file from 67 MB to
  135 MB (`research/24-prep/g-probe-results.md:345-350`). This does not
  measure indexing every scalar field. Compare full-row queries and
  replacement costs before treating generic keys as a read-speed win.
- [V] The existing reverse tag and class tables let writes remove old
  keys (`src/index/store.rs:1445-1484`). Before removing them, read
  the old values from stored facets inside the write transaction, or
  provide another path-based deletion strategy. One transaction must
  replace facets, task copies, range keys, link edges, notices, meta
  and the store revision. Delete every old derived key first.
- [V] The notice facet initially lacked a named reader. The path-scoped
  read above limits persisted notices to parse-time facts. Ticket 31 owns
  portable path identity and sort order. Postcard data alone does not make range
  keys portable.
- [V] The task-only probe has 37,500 tasks among 487,500 items
  (`research/24-prep/g-probe-results.md:284-285`). It measured the
  task subset without full row assembly. If `lists` retains all items,
  a task-heavy vault can duplicate most item bytes in `tasks`.
  Both copies must change in the same write transaction.
  If `lists` excludes tasks, `Lists` must merge both facets by original
  `item_idx`, including parents and nested order. No task-only result
  may renumber the index or omit its parent when a reader asks for it.
  Neither layout avoids loading a full note for a result that needs
  `file.lists` or list-parent context. Measure both on task-heavy input and
  assembled rows. The 12 ms scan proves neither end-to-end speed nor a size
  advantage.

## Inputs from the value model decision

See [ticket 18's Answer](./18-value-model.md). Postcard encodes `Date`,
`DateTime` and `Duration` as text. It parses them again on every cache load. A
compact binary form with `is_human_readable()` is a candidate. No one measured
it. Ticket 18 keeps the derived tagged serde for the postcard cache.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md).

- The cache persists naive datetimes as civil values. It holds no machine zone.
  The cache format changes.
- The store never holds a collation sort key.
- A secondary table can push down a filter. Its candidates must agree with the
  filter relation. Use the numeric value for `Int` and `Float`. Use the folded
  target for links.
- The total sort order does not replace period membership. It does not replace
  the field-versus-field civil-midnight rule.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md).

- Class binding by tag and path expands at query time into the `Class`, `Tag`
  and `Path` atoms. The index stores no derived class. It learns nothing about
  schemas.
- Source selection needs an exclusion for "has a known explicit class". A
  secondary lookup for it avoids the `Not` full scan.
- Ticket 28 optimises a folder glob such as `folder/**` to `paths_in_folder`.
  The remaining `Path` scan is linear.
- A "notes changed since the last check" notice (ticket 20, agent guardrail)
  reads the changed set that the refresh computes. A baseline store sits
  outside the index and its fingerprint. Tickets 25 and 33 share the open
  ownership question.

## Inputs from the note document model decision

See [ticket 23's Answer](./23-note-document-model.md).

- `Note` is an assembled view of four independent facets: meta, headings, links
  and items. No facet needs the storage of another facet. This ticket chooses
  the physical split and what loads when.
- Ticket 23 says that no consumer needs all facts of all notes in memory.
- A decoded note takes about twice its encoded size. Link edges are about 4% of
  the record.
- An item stores the status symbol and the kind. It stores no status name. A
  task stores no heading link. A query derives the heading above a task from
  the heading ranges.
- The store keeps no external URL, byte span, footnote, callout or slug.
- A change of the stored `Note` changes the extractor fingerprint or bumps the
  format epoch (ticket 25). Index results must equal a fresh parse.
- Ticket 28 needs the item identity `item_idx`, the `list.*` fields and the
  document order.
- The bulk index path collects parser notices with their paths. Ticket 33 owns
  how they show.
- Ticket 32 keeps a compact link and heading range index for open buffers. It
  tags the range sets with a source revision. Ticket 27 decides the rest.
- Ticket 23 left these items open: the boxed or unboxed task payload, host
  memory at 50,000 notes, and a two-process open of the store.

## Inputs from the config ownership decision

See [ticket 22's Answer](./22-config-ownership-and-lifecycle.md).

- `index` owns `IndexPolicy`. `IndexPolicy::fingerprint()` is the only source of
  the fingerprint.
- Ticket 25 (resolved) decides the store identity (engine tag, platform family,
  format epoch, extractor fingerprint, policy fingerprint), invalidation rules
  and racy timestamps.
- A changed ignore set needs a rescan and a rebuild.
- The composition root builds `IndexPolicy` one time for the CLI and for the
  template engine. Both consumers use the same policy.
- The parser input is built one time for each indexing run. The parser borrows
  it.

## Inputs from the forward-consumer decision

See [ticket 17's Answer](./17-grilling-forward-consumers-and-design-pressure.md).

- The LSP is the next committed consumer. The MCP server is likely.
- Keep the core synchronous by default. Add async at host seams only.
- Vaults can exceed 50,000 notes.
- Evaluate each proposal by independent changeability, low coupling, fast
  verification, failure isolation and recoverability.
- Ticket 27 owns the consistency of an open buffer and the persisted index.

## Inputs from the precedent research

- [Research 06](../research/06-query-plan-precedent.md): Source selection is the
  indexable subset of the predicates. Pushdown is a three-state capability on
  the storage side: `Exact`, `Inexact` and `Unsupported` (DataFusion). A store
  that returns candidates is `Inexact`. Residual filters run on the rows.
  Dataview prunes with in-memory sets of the form `Map<Tag, Set<Path>>`. A
  heavy relational IR does not fit vaults of 1,000 to 20,000 notes [H].
- Research 06 says that `QueryRow` holds `Arc<WorkspaceIndex>`. This forces the
  partial index. In Polars and DataFusion, a row does not own the catalog [H].
- Research 06 says that a CLI cannot afford to decode 20,000 notes on every
  run. Research 11 supersedes this claim: a full load takes 74 ms.
- [Research 05](../research/05-freshness-fingerprint-precedent.md) suggests an
  inverted map of unresolved stems. Research 11 shows that 97% of the cost is
  note decoding. Ticket 25 owns the racy-timestamp guard.
- [Research 09](../research/09-capability-path-precedent.md): Carry a portable
  UTF-8 relative path with `/` in persistent structs. Confine at the I/O
  boundary. Ticket 31 decides the persisted path form and the portability of
  the cache.

## Inputs from the findings ledger

See [findings-ledger.md](../findings-ledger.md). The ledger row on
`SortedByPath::assumed_sorted` folds into topic E. The ledger row on the class
expander shows that two paths can diverge.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md).

- IWE is the measured pure in-memory alternative. It has no persistent cache.
  The CLI parses the whole vault with Rayon on every run.
- Synthetic notes of about 700 B on an M3 Pro [V: `docs/benchmark.md:155-171`]:
  - 5k notes take 144 ms.
  - 10k notes take 329 ms.
  - 20k notes take 705 ms for a cold load. The load takes 36% more with its BM25
    index.
- A field filter at 20k takes 12 ms. Graph filters are super-linear. `--roots`
  takes 60 ms. The unbounded included-by filter reaches about 94% of the corpus
  [V: `docs/benchmark.md:185-208`].
- Servers load one time and update in place (`Graph::update_document`).
- The machine and the corpus differ from ticket 11 (M4 Pro, 20k cold query 60
  to 74 ms). Use these numbers as a cost model for the keep-as-is comparison.
  Do not use them as a head-to-head result.
- The IWE evaluator splits an `and` into graph-walk generators and scalar
  predicates. It intersects the candidate key sets before it runs the scalar
  predicates (`crates/liwe/src/query/eval.rs`, `eval_and`) [V]. This candidate-set
  plan needs no store.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md).

- A query starts from one coherent index generation. It keeps its pinned
  temporal frame across lazy materialization.
- A refresh from a different policy must never mix stored rows of two
  generations. This ticket and ticket 25 own the read and store boundary.
- Two hosts can have different in-memory workspace identities. They can still
  share one store. The identity does not isolate the store.
- Do not assume that redb gives the policy invariant. Check its lock behavior.
- An absent creation time stays `Null`. Do not substitute the modification time.
- Persist source timestamps. Do not persist the zone-derived `cdate` and `mdate`
  under a zone-free fingerprint.
- A filename period is zone-independent. It is the same when the store persists
  it and when a query parses it.
- Ticket 21 removes the `file.day` name and its hybrid of filename and creation
  time. The persisted `FileMeta.day` still holds this hybrid.

## Resolution and handoffs

The answer above settles the design. These handoffs keep the open
evidence and implementation work explicit.

Alternatives considered: a dual path, a full in-memory load, whole-note
records, and a core/text item split. Other options: selected-field
indexes, scan-only tables, a held store handle, removing the trie, and
debug-only order checks. See `research/24-prep/h1-stress-test-designs.md`
and `research/24-prep/g-probe-results.md`.

Invariant owners:

- Index and store: a coherent generation, atomic facet writes, sound
  candidate sets, and the path-order check at load and build boundaries.
- Ticket 25: store identity (engine tag, platform family, format epoch,
  extractor fingerprint, policy fingerprint) and freshness invalidation.
- Ticket 28: row type, residual filter, pushdown protocol and membership.
- Ticket 31: the persisted path form and root identity.
- Ticket 32: host writer admission.
- Ticket 33: how parser notices show.
- Ticket 34: visibility of the index and store port.
- Ticket 27: buffer overlay on a pinned generation.

Remove the four multimap tables after implementation:
`paths_by_tag`, `paths_by_file_class`, `tags_by_path` and `classes_by_path`.
Remove the `SourceResolver` full-store read path, the partial
`WorkspaceIndex`, and the split failure rules for read queries.

Measure the generic field and task range-key layout, including write and
replace costs, before implementation. Measure full row materialization
on a real vault. Check Linux and Windows locking. A compact temporal
encoding needs a measured read gain. Ticket 21 already removes the
hybrid `FileMeta.day`. Ticket 20 keeps inline fields out of class axes.

Change table (ticket 16): on a successful fresh-parse fallback, a held
store query exits 0 rather than 1. The query prints one cache notice on
stderr and keeps query results on stdout. The 1.8 s synthetic build
included persistence, so it is not a measured fallback latency.
The store layout and format version change within the Free envelope.

Downstream tickets: 25 (freshness and format), 28 (plan and rows),
30 (index-facing consumer), and 31 (path form). Also 32 (writer
admission), 33 (notice display), 34 (visibility), 27 (buffer overlay), and
26 (recovery journal).

Fog: keep `CandidateTrie` and `indextree`. Split the store engine from the
repository at the domain port. Class axes stay frontmatter-only.

## Code delta

The code does not implement this ticket. This table lists what the refactor changes. Locations are the working tree on 2026-11-02.

| Area | Today (path:line) | Target (ticket decision) | Depends on |
| --- | --- | --- | --- |
| Store abstraction port | Concrete `IndexStore` in `src/index/store.rs:145` and `src/query/service.rs:125` | Domain-level store port with an adapter boundary; no engine type crosses the boundary | Ticket 34 |
| Persisted facet tables | Single serialized `notes` and `files` tables in `src/index/tables.rs:21,30` | Eleven stored facets: `files`, `fields`, `tags`, `classes`, `headings`, `outlinks`, `links`, `tasks`, `lists`, `notices`, `meta` | Ticket 23, Ticket 25 |
| Redb secondary tables | Four multimap tables `PATHS_BY_TAG`, `PATHS_BY_FILE_CLASS`, `TAGS_BY_PATH`, `FILE_CLASSES_BY_PATH` in `src/index/tables.rs:43-70` | Remove the four multimap tables; query execution uses facet scans and proved range-key lookups | Ticket 28 |
| Query execution paths | Dual path: memory scan in `src/query/service.rs:98` and `run_from_store` in `src/query/service.rs:123` | Single execution plan for CLI and hosts; CLI reads store candidates and hosts read a pinned generation | Ticket 28 |
| Source resolution | `SourceResolver` in `src/query/service.rs:316-406` does full-store scans for `All`, `Not`, and empty expressions | Remove `SourceResolver`; enumerate `files` paths and narrow with safe predicates | Ticket 28 |
| Query row borrow model | `QueryRow` borrows whole `Arc<WorkspaceIndex>` in `src/query/results.rs:38` | Row owns matching `Arc<FileEntry>` rather than the entire index | Ticket 28 |
| Totality of WorkspaceIndex | `WorkspaceIndex::assemble` in `src/index/entry.rs:39` builds partial index for `run_from_store` in `src/query/service.rs:156` | `WorkspaceIndex` stays total and sorted; cold CLI constructs `FileEntry` rows directly | Ticket 28 |
| Release path-order check | Debug-only assertion in `SortedByPath::assumed_sorted` in `src/index/sort.rs:48-54` | Check path order in release builds at index build and store load boundaries | None |
| Store transaction pinning | `read_batch` and `read_files_and_fingerprint` open separate read transactions in `src/index/store.rs:550,1004` | Pin one read transaction across candidate selection, facet reads, and fingerprint verification | Ticket 25 |
| Cache failure handling | `refresh` logs an alert in `src/index/mod.rs:62`; `refresh_store` returns `IndexError::Store` in `src/index/service.rs:285` | Unified cache miss: fallback to fresh parse and print one stderr notice; exit 0 on success | Ticket 16, Ticket 33 |
| Store lock lifecycle | `IndexStore::open` holds open database in `src/index/service.rs:338,407` | Host opens store only to commit write batches; reads come from pinned generation | Ticket 27, Ticket 32 |
| Base generation check | `RefreshPlan::collect` reads fingerprint before scan without writer verification in `src/index/refresh.rs:336-354` | Transactional check compares base identity, revision, and fingerprint before committing delta | Ticket 25 |
| Corrupted row policy | `read_batch` skips corrupt rows in `src/index/store.rs:1018-1026` | Corrupted or missing required facet triggers a complete cache miss rather than partial results | Ticket 25 |
| Inbound links persistence | `read_all_links` reads `LINKS` table in `src/index/store.rs:588` | Persist inbound links graph in `links` facet; refresh updates edges atomically | Ticket 25 |
| FileMeta day field | `FileMeta.day` derives timezone-dependent day in `src/file.rs:52,88-98` | Remove persisted `day` field; derive filename period at query time | Ticket 21 |
| Benchmark candidate scale | `SAME_STEM_CANDIDATE_COUNTS` caps at 128 in `benches/index_inlinks.rs:60` | Extend `index_inlinks` benchmark to 5,000 same-stem candidates to monitor trie scale | None |

Not in code:

- Conformance test suite verifying that every store adapter matches fresh-parse rows.
- Measurement of generic scalar field and task range-key encoding performance on real vaults.
- Measurement of full row materialization against facet scans on large vaults.
- Linux and Windows file lock validation for host-CLI concurrency.

Unclear in the ticket:

- "The precise range-key encoding and measured write and read costs".
- "Confirm the choice on assembled rows, task-heavy input and one-note replacement before fixing the layout".
