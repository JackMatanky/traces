# What is the index read model, and is there one query execution path or two?

Type: grilling
Status: open
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

- Decide what a `QueryRow` borrows from. Ticket 26 owns the row shape. This
  ticket owns the source of the borrow.
- State what a row needs: the `Note` facets, the `FileMeta`, the inlinks, the
  item identity `item_idx` and the document order (ticket 26).

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
  at query time (tickets 21 and 27).
- Decide the encoding of temporal values. Today the store keeps text and parses
  it on every load. A compact binary form is an option.
- Decide if the store keeps the parser notices. Ticket 23 decided that the bulk
  index path collects them. No code does so today (tickets 23 and 31).
- Decide where the lookup "has a known explicit class" lives (ticket 20).
- Decide if the class axes index inline fields. Today they index frontmatter
  values only.
- State the constraint on the persisted path form. Ticket 29 owns the form.
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
- Ticket 26 decides the row type, the view enum, the pushdown protocol and the
  membership operation.
- Ticket 29 decides the persisted path form and the identity of a root.
- Ticket 30 decides the held workspace and the writer admission for a root.
- Ticket 34 decides how an open buffer overlays the persisted generation.
- Ticket 31 decides how a notice shows.
- Ticket 32 decides the visibility of `IndexStore` and `WorkspaceIndex`.

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
- The downstream tickets that change: 25, 26, 28, 29, 30, 31, 32 and 34.

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
- Ticket 26 optimises a folder glob such as `folder/**` to `paths_in_folder`.
  The remaining `Path` scan is linear.
- A "notes changed since the last check" notice (ticket 20, agent guardrail)
  reads the changed set that the refresh computes. A baseline store sits
  outside the index and its fingerprint. Tickets 25 and 31 share the open
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
- A change of the stored `Note` bumps `META_VERSION`. Index results must equal a
  fresh parse.
- Ticket 26 needs the item identity `item_idx`, the `list.*` fields and the
  document order.
- The bulk index path collects parser notices with their paths. Ticket 31 owns
  how they show.
- Ticket 30 keeps a compact link and heading range index for open buffers. It
  tags the range sets with a source revision. Ticket 34 decides the rest.
- Ticket 23 left these items open: the boxed or unboxed task payload, host
  memory at 50,000 notes, and a two-process open of the store.

## Inputs from the config ownership decision

See [ticket 22's Answer](./22-config-ownership-and-lifecycle.md).

- `index` owns `IndexPolicy`. `IndexPolicy::fingerprint()` is the only source of
  the fingerprint.
- Ticket 25 decides the content, the granularity, the parser version, the
  format version and the racy timestamps.
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
- Ticket 34 owns the consistency of an open buffer and the persisted index.

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
  boundary. Ticket 29 decides the persisted path form and the portability of
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
