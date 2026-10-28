# Prior tickets 01-15: inputs to ticket 24

This record reports earlier findings. It does not choose a read model. Citations
are relative to this file. `[V]` identifies a checked earlier source; `[H]`
identifies an inference or an untested claim; `[P]` identifies an earlier
proposal, not an accepted decision. The source revision matters: research 01 and
11 predate the config-drift change.

## A. Consumers and execution paths

- [V] Research 01 found two paths: `run_from_store` builds an index of candidate
  notes; `run` scans all in-memory entries. `Not` reads all stored paths.
  Source: `../01-verify-load-bearing-claims.md:56-61`. Topic A; also E.
- [V] Research 11 found that the cold CLI calls `run_from_store`. It reads
  candidate notes from redb after source selection. Source:
  `../11-performance-evidence.md:55-59`. Topic A.
- [V] Research 07 found that dynamic template queries need an index held for one
  render. Its minijinja `State` stores that index. Source:
  `../07-template-bridge-precedent.md:53-72`. Topic A; also B.
- [V] The template `QuerySet` already holds a plan and an `Arc` to base rows.
  Chaining does not copy rows. Source:
  `../01-verify-load-bearing-claims.md:75-81`;
  `../../issues/06-research-query-plan-precedent.md:24`. Topic A; also C.
- [V] Dataview selects candidates from in-memory tag sets, folder tries, and
  link maps before it reads page data. Obsidian Tasks scans all tasks in memory.
  Source: `../06-query-plan-precedent.md:24-33`. Topic A; also D.
- [V] DataFusion distinguishes exact, inexact, and unsupported pushdown. It
  checks rows again after an inexact prune. Source:
  `../06-query-plan-precedent.md:7-10,40-44`. Topic A; also D.
- [H] Research 06 proposed one predicate language with an indexable subset. This
  is not a decision from tickets 01-15. Source:
  `../06-query-plan-precedent.md:64-69`. Topic A.
- [V] IWE evaluates queries against an in-memory graph. An `and` clause
  intersects graph candidate sets before scalar checks. IWE has no database
  pushdown. Source: `../13-iwe-query-schema-and-agent-write-gating.md:16-43`.
  Topic A; also B.
- [V] IWE has a CLI, an LSP, and an MCP server over shared core operations. Its
  read queries live in the app library. Source:
  `../12-iwe-core-and-hosts-architecture.md:9-56`;
  `../../issues/12-research-iwe-core-and-hosts-architecture.md:67-77`. Topic A;
  also F.
- [V] IWE's CLI rebuilds its in-memory graph for every call; its servers build
  once and update the graph. It has no persistent cache. Source:
  `../12-iwe-core-and-hosts-architecture.md:85-97`. Topic A; also B.
- [V] IWE's query document has YAML or JSON form. Ticket 13 sends the
  host-facing serialization question to ticket 26. Source:
  `../../issues/13-research-iwe-query-schema-and-agent-write-gating.md:58-66`.
  Topic A; also F.

## B. Read model, generation, and failure

- [V] Research 05 found that cache corruption is a miss in Cargo, Ruff, rustc,
  and rumdl. It also found that redb can change data and fingerprint in one
  transaction. Source: `../05-freshness-fingerprint-precedent.md:46-55,68-77`.
  Topic B.
- [V] Research 05 records a redb `meta` fingerprint for config drift. Its
  opening note says that this proposal landed after the research baseline.
  Source: `../05-freshness-fingerprint-precedent.md:1-3,91-94`. Topic B; also D.
- [H] Research 04 proposed separate host, workspace, and temporal contexts. It
  proposed one pinned time and zone per query. Later ticket 21 refines the zone
  rule. Source: `../04-execution-context-precedent.md:89-100`;
  `../../map.md:328-340`. Topic B.
- [V] IWE's LSP puts an open buffer directly in its only graph and ignores disk
  events for that key. It has no persisted generation to reconcile. Source:
  `../12-iwe-core-and-hosts-architecture.md:103-122`;
  `../../issues/12-research-iwe-core-and-hosts-architecture.md:78-83`. Topic B.
- [V] IWE measures a 705 ms cold load at 20,000 synthetic notes on an M3 Pro.
  Its corpus has about 700 B per note. This is not a direct comparison with
  Traces. Source: `../12-iwe-core-and-hosts-architecture.md:147-160`;
  `../../issues/12-research-iwe-core-and-hosts-architecture.md:82-89`. Topic B.
- [V] Research 05 records Git's racy timestamp guard. It also records a missing
  guard in Traces' stat delta. Ticket 25 owns that question. Source:
  `../05-freshness-fingerprint-precedent.md:50-53,76-77`;
  `../../issues/05-research-freshness-fingerprint-precedent.md:9-15`. Topic B.
- [V] Fileclass invalidates a memoized Base query when a `.base` file changes.
  It reads frontmatter through Obsidian's metadata cache. This is a host cache,
  not a redb generation rule. Source:
  `../14-fileclass-model-binding-and-validation.md:127-147`. Topic B.

## C. Rows

- [V] Research 01 found that `QueryRow` compares file positions but ignores
  `item_idx`. Items in one file compare equal for document order. Source:
  `../01-verify-load-bearing-claims.md:69-73`. Topic C.
- [H] Research 06 says that row ownership of `Arc<WorkspaceIndex>` causes the
  partial index on the store path. It proposes direct rows over stored records
  instead. Source: `../06-query-plan-precedent.md:97-106`. Topic C; also E.
- [V] Research 10 found that note-level inline fields hold multiple values, but
  row lookup reads only the first. Item fields also duplicate into note
  collections. Source: `../10-document-model-spans-precedent.md:63-69,85-92`;
  `../01-verify-load-bearing-claims.md:89-94`. Topic C; also D.
- [V] IWE projects frontmatter, system fields, and blocks into `FindMatch` rows.
  Its `retrieve` path expands graph relations for an LLM context. These are
  distinct result shapes. Source:
  `../13-iwe-query-schema-and-agent-write-gating.md:38-43`. Topic C.

## D. Persisted facts, axes, links, and metadata

- [V] Research 01 found that the current store extracts the explicit class from
  a frontmatter field without a schema lookup. Source:
  `../01-verify-load-bearing-claims.md:83-87`. Topic D.
- [V] Research 03 found that validation at index time couples the index to
  schemas. It described separate advisory checks after indexing. This was a
  research hypothesis, not a rule from ticket 03. Source:
  `../03-coercion-validation-precedent.md:84-96`;
  `../../issues/03-research-coercion-validation-precedent.md:19-22`. Topic D.
- [V] Research 11 names four redb tables. They map paths by tag, paths by class,
  tags by path, and classes by path. Its point-query test uses this store path.
  Source: `../11-performance-evidence.md:37-59`. Topic D.
- [V] Research 05 found that a path-set change caused full note decoding and
  link re-resolution. Its inverted-stem map was a proposal. Source:
  `../05-freshness-fingerprint-precedent.md:68-77`;
  `../01-verify-load-bearing-claims.md:63-67`. Topic D.
- [V] Research 10 found that task items are children of notes. A task-only query
  must visit notes or use another index. Source:
  `../10-document-model-spans-precedent.md:75-82,104-105`. Topic D.
- [V] Research 10 found that pulldown-cmark gives byte offsets at no extra parse
  cost. It proposed a separate, temporary line index. Ticket 23 later excludes
  spans from the stored `Note`. Source:
  `../10-document-model-spans-precedent.md:46-59,85-89`; `../../map.md:365-376`.
  Topic D.
- [V] Research 09 found that the current path codec stores OS-native bytes or
  wide units in redb. It proposed portable UTF-8 relative paths, with
  confinement at I/O. Ticket 29 owns the path form. Source:
  `../09-capability-path-precedent.md:53-60,88-102`. Topic D.
- [V] Research 02 found eager temporal and link classification at the note
  boundary. It found that numbers above $2^{53}$ lose precision as `f64` in the
  old note model. Source: `../02-value-model-precedent.md:86-99`. Topic D; also
  A.
- [V] Research 13 found that IWE filters compare typed values; cross-type
  equality is false. It uses ICU text collation for sorting. Index candidates
  must match filter equality, not sort order. Source:
  `../13-iwe-query-schema-and-agent-write-gating.md:45-60`;
  `../../issues/13-research-iwe-query-schema-and-agent-write-gating.md:63-66`.
  Topic D.
- [V] Fileclass gets class bindings from explicit frontmatter, tags, paths,
  bookmarks, and views. Its class merge lets a later, lower-ranked binding
  override an earlier one. A test does not check that collision. Source:
  `../14-fileclass-model-binding-and-validation.md:40-71`;
  `../../issues/14-research-fileclass-model-binding-and-validation.md:75-95`.
  Topic D.
- [V] IWE's body validator takes a recursive section tree, while Traces stores
  flat headings. The validator has no source spans or token counter. Ticket 15
  found poor fit. Source:
  `../15-schematter-validator-and-document-schema.md:24-38,40-62,90-106`;
  `../../issues/15-research-schematter-validator-and-document-schema.md:49-71`.
  Topic D.

## E. Index totality

- [V] The store path makes a partial `WorkspaceIndex`. This breaks the
  documented rule that each regular file has an entry. A `Not` source still
  scans all paths. Source: `../01-verify-load-bearing-claims.md:56-61`. Topic E.
- [V] The research 05 Git precedent uses path-sorted flat entries and
  binary-search prefix ranges. It gives a possible comparison for a total,
  sorted index. Source: `../05-freshness-fingerprint-precedent.md:24-27`. Topic
  E.

## F. Index internals and surfaces

- [V] Research 01 found that only `CandidateTrie` uses `indextree`. The trie
  threshold is 64 same-stem candidates. Source:
  `../01-verify-load-bearing-claims.md:104-107`. Topic F.
- [V] Research 08 found two live layout patterns: a large single crate and
  workspaces with separate CLI and test support. It did not decide the Traces
  crate split. Source: `../08-errors-topology-precedent.md:48-56,84-104`. Topic
  F.
- [V] The map records a steer to examine a separate CLI crate. This would make
  the CLI an outward-facing consumer of the core. The steer is not a decision.
  Source: `../../map.md:33-42`. Topic F.
- [V] The mission asks for invariant ownership and a reason for each boundary.
  It does not call for a store-engine split. Source: `../../mission.md:21-35`.
  Topic F.

## Research 11 measurements and limits

- [V] At 1,000 notes, the cold rare-tag store query took 16.42 ms and 6.48 MB
  RSS. Full load took 17.49 ms and 7.53 MB RSS. At 5,000: store 25.44 ms / 8.89
  MB; full load 28.76 ms / 13.92 MB. At 20,000: store 60.31 ms / 17.34 MB; full
  load 74.22 ms / 38.67 MB. Source: `../11-performance-evidence.md:37-53`.
  Topics A, B, D.
- [V] At 5,000 notes, refresh took 32.23 ms per edit and 35.17 ms per delete. An
  add took 29.01 ms; 100 edits took 34.61 ms. Full note decode took 18.48 ms
  (97.3%); link re-resolution took 0.52 ms (2.7%). Source:
  `../11-performance-evidence.md:74-107`. Topics B, D.
- [V] Flat scan took 1,217.4 ns at 8 candidates and 9,931.3 ns at 64. At 128 and
  512, it took 20,607.9 and 81,429.1 ns. Trie build took 17.66, 48.23, 86.72,
  and 314.23 µs. Trie lookups took about 60 ns; break-even took 15, 5, 4, and 4
  lookups per stem. Source: `../11-performance-evidence.md:115-147`. Topic F.
- [V] The second parser pass took 20.2%, 26.0%, and 38.0% of parse time in three
  sample notes. `ListItem` measured 160 B. A 10,000-task note took 12.29 ms and
  allocated 10,000 status-name strings (at least 234 KB). Source:
  `../11-performance-evidence.md:151-200`. Topic D.
- [V] Ten-field frontmatter parsing took 5.33 µs for YAML and 3.75 µs for value
  conversion. The measured old `NoteFieldValue` and the proposed unified enum
  were each 80 B. Source: `../11-performance-evidence.md:204-244`. Topic D.
- [V] The template test used 5,000 rows. `QuerySet.clone()` took 6.2 ns; chained
  transforms took less than 1 µs. It did not copy rows at each step. Source:
  `../11-performance-evidence.md:248-301`. Topic A; also C.
- [V] Persistence at 1, 50, 200, and 1,000 notes took 25.47, 24.47, 25.45, and
  25.71 ms. `Database::create` took 11.36 ms, drop took 11.98 ms, and a
  `Durability::None` commit took 18.00 µs. A commit on an already open database
  took 13.0 µs with no durability and 4.01 ms with immediate durability. Source:
  `../11-performance-evidence.md:306-364`. Topic B; also F.
- [V] Setup: Apple M4 Pro, 24 GB RAM, APFS, macOS Darwin 27.0.0,
  nightly-2026-10-01, release `-O3` with project LTO. Isolated child processes
  measured high-water RSS with `getrusage`. Timers used warmups and medians of
  5-20 iterations; the cold rare-tag table used five isolated runs. Source:
  `../11-performance-evidence.md:5-9,27-33,37-46`. Topic A; also B.
- [V] Scope: The cold table tests one rare tag at 1,000, 5,000, and 20,000
  notes. It does not test `All`, `Not`, `Path`, `Class`, or no source. It does
  not test 50,000 notes, second-process access, host memory, facet sizes,
  temporal encoding, or boxed task payloads. Source:
  `../11-performance-evidence.md:37-53,74-80,115-123,204-229,306-341`;
  `../../issues/24-index-read-model-and-query-execution.md:129-140`. Topics A,
  B, D.

## Conflicts and gaps

- [V] Research 05 proposes an inverted-stem map to remove the path-set cost.
  Research 11 measures 18.48 ms for decoding and only 0.52 ms for link
  resolution. Ticket 24 notes this conflict but leaves the link-graph choice
  open. Source: `../05-freshness-fingerprint-precedent.md:68-77`;
  `../11-performance-evidence.md:89-111`;
  `../../issues/24-index-read-model-and-query-execution.md:331-333`. Topic D.
- [V] Research 05 favors a pure memory path. Research 06 says a cold CLI cannot
  afford a full load. Research 11 measures 74.22 ms and 38.67 MB at 20,000
  notes; the memory limit remains open. Ticket 24 records the time correction.
  Source: `../05-freshness-fingerprint-precedent.md:101-104`;
  `../06-query-plan-precedent.md:79-85`; `../11-performance-evidence.md:44-53`;
  `../../issues/24-index-read-model-and-query-execution.md:327-330`. Topics A,
  B.
- [V] Research 06 proposes a new lazy template builder to prevent row copies.
  Ticket 06 and research 01 correct this: chaining already copies no rows.
  Ticket 24 must not use that cost as a reason to replace a path. Source:
  `../06-query-plan-precedent.md:70-72`;
  `../../issues/06-research-query-plan-precedent.md:24`;
  `../01-verify-load-bearing-claims.md:75-81`. Topic A.
- [V] Research 02 reports a 64 B old note value and 56 B field value. Research
  11 measures 80 B for the old note value and 72 B for the field value on its
  target. Use the measured target when estimating host memory. Source:
  `../02-value-model-precedent.md:92-99`;
  `../11-performance-evidence.md:210-220`. Topic B; also D.
- [V] Research 10 proposes storing spans in redb. The later ticket 23 decision
  excludes spans from the stored facts and puts them in an ephemeral document.
  Ticket 24 fixes this later boundary. Source:
  `../10-document-model-spans-precedent.md:85-92,117-120`;
  `../../map.md:365-376`;
  `../../issues/24-index-read-model-and-query-execution.md:149-153`. Topic D.
- [H] Research 10 proposes separate item fields without a note-level roll-up.
  Ticket 23 keeps note-level tags and inline fields. The row plan must state how
  the page and item rows get item metadata without changing results. Source:
  `../10-document-model-spans-precedent.md:85-92,115-120`;
  `../../map.md:365-376`. Topics C, D.
- [V] Research 05's old config-hash claim predates the landed fingerprint.
  Research 01's old stale-class test also predates it. Ticket 24 already names
  the new index-owned policy. Source:
  `../05-freshness-fingerprint-precedent.md:1-3`;
  `../01-verify-load-bearing-claims.md:1-3,49-54`;
  `../../issues/24-index-read-model-and-query-execution.md:153-159`. Topic B.
- [H] Tickets 01-15 supply no measured two-process redb lock behavior and no
  tested rule for simultaneous writers with different policies. Ticket 24 asks
  for both; its facts mark the lock behavior unchecked. Source:
  `../../issues/24-index-read-model-and-query-execution.md:31-39,190-192`;
  `../11-performance-evidence.md:306-341`. Topic B.
- [H] Tickets 01-15 do not measure the other source shapes or facet decode
  sizes. They do not measure 50,000-note host memory or release sorted-order
  checks. Ticket 24 calls for these measurements. Source:
  `../../issues/24-index-read-model-and-query-execution.md:129-140`;
  `../11-performance-evidence.md:37-53`. Topics A, B, D, E.
- [H] Ticket 14's tag and path class binding comes from Fileclass as precedent.
  It does not test whether Traces' frontmatter-only class axis can select bound
  classes. Ticket 24 delegates class expansion to query time and leaves the
  explicit-class exclusion lookup open. Source:
  `../14-fileclass-model-binding-and-validation.md:40-71`;
  `../../issues/24-index-read-model-and-query-execution.md:253-263`. Topic A;
  also D.
- [H] Research 08 and 12 describe possible crate and app boundaries. They do not
  decide if one `IndexStore` type must split into engine and repository. Ticket
  24 still owns that choice. Source:
  `../08-errors-topology-precedent.md:84-104`;
  `../12-iwe-core-and-hosts-architecture.md:9-56`;
  `../../issues/24-index-read-model-and-query-execution.md:80-83`. Topic F.
