# Stress test of candidate designs for ticket 24

This file tests candidate designs against the evidence of this prep. It decides
nothing. Each fact has a tag. `[V]` means checked in code, a probe or a source.
`[H]` means a hypothesis. `[P]` means a proposal. The probe numbers come from
`g-probe-results.md`. The code facts come from `a1-index-query-code.md` and from
the code at `main` (`39e96041`).

## 1. Requirements

| id | Requirement | Source |
|----|-------------|--------|
| R1 | An index result equals the result of a fresh parse. | Ticket 16 (Frozen) |
| R2 | A cold CLI query stays fast at the seven vault sizes. No size has a budget. | Ticket 17 |
| R3 | A long-lived host (LSP, MCP) answers a request from one coherent generation. | Tickets 17 and 21 |
| R4 | The CLI and a host can run on one vault at the same time. | Tickets 17 and 21 |
| R5 | A query that needs only the meta facet does not decode items or links. | Ticket 23 |
| R6 | One rule decides each result. A second path does not hold a second rule. | Ticket 16, finding in the ledger |
| R7 | The code that serves the design is small. Each table has a named query. | Ticket 24 |
| R8 | A failure has one rule for all consumers. | Ticket 24 |
| R9 | An observable change has a row in the change table. | Ticket 16 |
| R10 | The memory of a host at 50,000 notes stays acceptable. | Ticket 23 |

## 2. What the evidence says

1. [V] Source pruning helps a subset of shapes only. A `Tag` or `Path` source at
   50,000 notes takes 117 ms to 127 ms and 35 MB to 52 MB. A query with no
   source, a `Not` source or a filter only takes 245 ms to 259 ms and 389 MB to
   393 MB (`g-probe-results.md`).
2. [V] The real CLI cannot run while another process holds the store. It exits
   with code 1 and the message "Database already open. Cannot acquire lock."
3. [V] One added file makes the next query decode every note: 499 ms and 395 MB
   at 50,000 notes, even for a rare tag.
4. [V] Two of the four multimap tables have no query reader. `tags_by_path` and
   `classes_by_path` serve only the incremental writer
   (`a1-index-query-code.md`, section D).
5. [V] A scan of 50,000 stored meta values of 150 B takes 6.7 ms to 7.6 ms in
   redb. A scan of 50,000 whole records of 1,350 B takes 33 ms to 42 ms. A
   multimap lookup of 5% of the paths takes 0.14 ms. This is I/O with no decode.
6. [V] A stand-in meta facet (82 B, 2 tags, 5 fields) decodes and filters in 21
   ms at 50,000 notes and in 8.5 ms at 20,000 notes (throwaway probe, postcard).
   The real facet can cost more, because dates parse from text today.
7. [V] A new generation made by a clone of a `Vec<Arc<Entry>>` takes 120 µs at
   50,000 entries (throwaway probe).
8. [V] A release-mode check that a path list is sorted takes 1.7 ms at 50,000
   paths with `Path` comparison and 0.12 ms with `OsStr` comparison.

## 3. Candidate designs for the paths and the read model

Notation: "full decode" means that every note decodes. "Floor" means the cost
that does not depend on the query shape (117 ms at 50,000 notes).

| id | Design | Cold CLI at 50,000 notes | Host memory | Code left | Main failure |
|----|--------|--------------------------|-------------|-----------|--------------|
| S0 | Keep the dual path and the partial index. | tag 117 ms; no source 247 ms | whole snapshot | most | Two rules. Shapes without a source decode everything. |
| S1 | One memory path. Drop the four tables. The store keeps whole notes. | every shape 247 ms and 393 MB [V] | whole snapshot | least | A rare tag costs 130 ms and 357 MB more [V]. |
| S2 | Memory path with in-memory sets built at load (Dataview). | same as S1 | whole snapshot | less | The load builds the sets after a full decode. A memory scan of 50,000 entries is cheap [H]. |
| S3 | Facet store. One plan: scan the meta facet, filter, load the other facets for the rows that pass. No multimap tables. | tag about 117 ms plus 21 ms [H]; no source about the same | facets on demand | small | New facet tables. The real meta decode can cost more than the stand-in. |
| S4 | Facet store and keep the two forward tables as `Inexact` pruning. | tag 117 ms; filter only about 138 ms [H] | facets on demand | medium | Two plans remain. Two tables stay in sync. |
| S5 | Columnar facts: fixed columns for tags, class and key scalars. | filter only below 138 ms [H] | small | large | A new schema for every queried field. Too early [H]. |
| S6 | A lazy view over one held read transaction, with no total index. | like S3 | small | medium | A row borrows from the transaction. The lifetime reaches the template engine and the host. |
| S7 | Replace redb with SQLite (zk precedent). | not measured | not measured | rewrite | A dependency and format swap. The layout is Free but the swap is out of scope for ticket 24 [H]. |

Lessons from the table:

- [V] S1 is safe up to about 5,000 notes. The cost of a full decode at 5,000
  notes is +12 ms and +36 MB. At 20,000 notes it is +46 ms and +145 MB. At
  50,000 notes it is +130 ms and +358 MB.
- [H] The tables only help the cold CLI. A host holds a snapshot. A scan of
  50,000 entries for one tag takes about 1 ms to 2 ms in memory. A set saves
  less than that.
- [H] S3 depends on the facet split of topic D. Without the split, the only
  other way to avoid the full decode is a pruning table (S0 or S4). The answer
  to topic A therefore depends on the answer to topic D. Ask topic D first.
- [V] S0 has two rules today. The store path matches the exact class when no
  expander is attached. The memory path matches nothing
  (`src/query/service.rs:402-406`, `src/query/grammar/source.rs:228-229`).
  Ticket 20 records this hazard. It says that production always attaches an
  expander, so it is not a current bug. Any design with a pruning table needs
  one residual filter. Then the table only prunes. It never decides.
- [H] S4 is the S3 design plus the tables as an optimisation. Add the tables
  only if the facet scan fails its gate: more than about 35 ms at 50,000 notes
  for a tag or class source.

## 4. Candidate rules for a failed persist (topic B)

Today `refresh` logs the failure and returns the in-memory index. Today
`refresh_store` returns `IndexError::Store` (`src/index/service.rs:259-264,
285-293`) [V].

| id | Rule | Effect | Precedent |
|----|------|--------|-----------|
| F1 | Keep two rules. | A template render survives a locked store. A CLI query fails. | Today [V] |
| F2 | Fail closed everywhere. | A host that holds the store blocks the template engine in another process. | zk rolls the transaction back when the index callback fails. It logs a per-note write error and continues [V: `c1`, finding 7] |
| F3 | Fail open for the derived cache. A query uses a fresh parse when the store is unusable. `traces index` fails closed. | A held store costs one full parse and no exit code. | rumdl, Dataview and Tasks treat a cache failure as a miss [V: `c1`, finding 14; `c2`, findings 8 and 29] |
| F4 | Fail open with a notice. Like F3, but the CLI prints that the cache was not used. | One more line on stderr. | [P] |

- [V] A full parse at 50,000 small notes takes about 1.8 s (`traces index`),
  because the build parses everything and persists. At 5,000 notes it takes 0.22
  s. This is the price of F3 and F4 when a second process holds the store [H].
- [V] A change from F1 to F3 or F4 changes an exit code from 1 to 0 for a held
  store. Ticket 16 freezes exit codes. A change-table row is a must.

## 5. Candidate rules for a second process (topic B)

| id | Rule | Evidence | Cost |
|----|------|----------|------|
| W1 | A host holds the store open all the time. The CLI fails. | [V] This is the real behaviour today. | Research 11 advises this to avoid the 25 ms floor. It makes every CLI call fail. |
| W2 | A host opens the store for one write and closes it. The CLI retries for a short time. | [V] A second `create` fails in 171 µs. [H] A write commit is short. | The 25 ms floor falls on each write. A burst of edits needs batching. |
| W3 | A host never opens the store. Only the CLI writes it. | [P] | The host parses on start and holds memory only. A cold host start costs a full build (1.8 s at 50,000 notes). |
| W4 | Use the redb multi-process mode (`experimental-multiprocess`, `ConcurrencyMode::MultiWriter`). | [V] A second reader opened in 330 µs and saw each commit. A second writer committed. | The feature is experimental. It refuses `Durability::None`. Each commit is 2-phase (8 ms to 11 ms in a debug build). |
| W5 | One index daemon owns the store. | [H] | A new process and protocol. Ticket 17 keeps the core synchronous. |
| W6 | Combine F3 and W2. | [P] | The CLI never fails because of a host. The host pays the floor per write. |

- [V] `ReadOnlyDatabase` does not help while a writer holds the file. The redb
  docs say that it "may not be opened concurrently with a `Database`".
- [H] W2 with F3 gives the smallest change. It needs no new dependency flag. It
  does not fix the cost of a held store for a long CLI query. Test it with a
  host that writes at each keystroke batch.
- [H] W4 removes the conflict. It adds an experimental flag to a Frozen
  guarantee (results equal a fresh parse). Check redb crash behaviour before any
  use.

## 6. Candidate generation models for a host (topic B)

| id | Model | Evidence | Lesson |
|----|-------|----------|--------|
| G1 | Immutable `Arc` snapshot. A refresh builds a new snapshot. A host swaps the pointer. | [V] rust-analyzer and Tantivy do this (`e1`, sources 1 and 10). | A request pins one generation. The old one frees when the last request ends. |
| G2 | Snapshot of `Arc<FileEntry>`. A new generation clones the vector and replaces the changed entries. | [V] 120 µs at 50,000 entries. | The copy is cheap. Entries are shared between generations. This needs `Arc<FileEntry>` in the index. |
| G3 | In-place update under a lock (markdown-oxide, rumdl, IWE). | [V] `c1`, findings 12 and 20; `research/12`. | A reader sees a partial scan, or a full rebuild stops the readers. |
| G4 | Two copies and a flip (`left-right`). | [V] `e1`, source 5. | Double memory. Not suited to a 400 MB snapshot [H]. |

- [H] G2 gives one generation for each query, a cheap swap and no lock on the
  read path. It makes the row type the key question (section 7).
- [V] A change counter such as the `revision` of Dataview or the `diag_gen` of
  markdown-oxide does not make a snapshot atomic (`c2`, finding 6; `c1`, finding
  13).

## 7. Candidate row sources (topic C)

| id | A row borrows from | Evidence | Lesson |
|----|--------------------|----------|--------|
| P1 | `Arc<WorkspaceIndex>` and a position (today). | [V] `src/query/results.rs:45-50`. | The row needs a total index. The store path builds a partial one. |
| P2 | `Arc<FileEntry>` for the row. | [V] G2 uses the same entry. | The store path builds `Arc<FileEntry>` for the candidates only. No partial index. A row sorts by path and `item_idx`. |
| P3 | An owned decoded record. | [H] Copies selected data. | Simple. Memory and copy cost grow with the result size. |
| P4 | A borrow from a read transaction. | [V] LMDB and redb give borrowed bytes that need a live transaction (`e1`, source 13). | A decoded `Note` is not a borrow of the bytes. The lifetime spreads to the template engine. |

- [V] `QueryRow` also needs `inlinks` (`src/query/results.rs:169-174`). Under
  P2, the entry carries its inlinks or a facet that gives them.
- [V] `QuerySet` already shares rows behind an `Arc`
  (`src/query/results.rs:463-467`, research 11, section 8). A row of P2 keeps
  that cheap.

## 8. Candidate store contents (topic D)

### Facets and tables

- [V] The stored share of each part on the documentation corpus is: list items
  77.7%, outlinks 14.1%, headings 5.6%, frontmatter 0.5%, tags 0.03% and the
  rest 2.1% (`research/23-prep/g-probe-results.md`).
- [H] A task query needs items, the heading ranges and the file facts. It does
  not need the outlinks.
- [H] A path-set change needs the outlinks only. The outlinks are 14% of the
  bytes. The facet split can cut the 18.5 ms decode of research 11 at 5,000
  notes by a large factor. The probe shows 499 ms and 395 MB for an add at
  50,000 notes with a whole-record store. This is a cross-ticket gain for ticket
  25.
- [V] Ticket 23 says item text is 93% of the item bytes. A split of item text
  from the item record saves more for a query that reads only the item facts.
- [H] Facet tables multiply the keys. Each facet table holds one row for each
  note. Write one transaction for all facets of one note, so that one generation
  never mixes facets of two parses.

### Link graph

- [V] `links` stores inlink edges. `refresh` and `load` read all edges. A store
  query reads the edges of the selected targets (`a1`, section D).
- [V] Link re-resolution takes 0.52 ms at 5,000 notes and the decode takes 18.48
  ms (research 11).
- [H] If the outlinks facet is cheap to decode, the inbound graph can be
  computed in memory at refresh and per query on the candidates. The table then
  goes. Measure the decode of the outlinks facet at 50,000 notes first.

### `FileMeta` and temporal values

- [V] `FileMeta.day` falls back to `created_at` in the local zone
  (`src/file.rs:88-98`). Ticket 21 removes this hybrid. A persisted `day` is
  therefore a stale field.
- [P] Persist `created_at`, `modified_at` and `size`. Derive the filename period
  at query time with the rule of ticket 21. A filename scan of 50,000 names is
  cheap [H]. Measure it.
- [H] A compact binary date saves the text parse on each load. The meta facet
  scan above decodes strings only. Measure with real dates before any change.

### Notices

- [V] No code collects notices today. `ParserContext` has no field for them
  (`a1`, section D). Ticket 23 decided that the bulk path collects them.
- [P] A notice is a fact of one parse. If the store keeps notices, one facet
  holds them, with the path as the key. If the store drops them, a command such
  as `traces check` reparses the note (ticket 31). Ask ticket 31 first.

### Class lookups

- [V] The class writer reads the configured frontmatter field, not inline fields
  (`src/index/store.rs:1742-1769`).
- [V] Dataview merges frontmatter and inline fields of one note into one map
  (`c2`, finding 13). Metadata Menu reads the explicit alias from frontmatter
  only (`c2`, finding 21).
- [P] Keep the class axis frontmatter-only. Ticket 20 gives the class set three
  sources: the frontmatter class field, tag bindings and path bindings. It does
  not name inline fields as a source [V]. A class axis over inline fields would
  make the index depend on the inline-field parse.
- [P] "Has a known explicit class" needs the set of known class names. The index
  does not hold the schema (ticket 20). The lookup therefore filters a candidate
  list at query time. With S3, the meta facet scan gives the explicit class
  values and needs no extra table.

## 9. Candidate rules for totality (topic E)

| id | Rule | Cost |
|----|------|------|
| E1 | Keep `assumed_sorted` with a debug-only check. | [V] Nothing in release. A wrong order gives silent wrong results. |
| E2 | Check the order in release. | [V] 1.7 ms at 50,000 paths with `Path` comparison, 0.12 ms with `OsStr` comparison. |
| E3 | Two types: a total `WorkspaceIndex` and a partial `Candidates`. Rows own `Arc<FileEntry>`. | [H] The store path stops fabricating a partial `WorkspaceIndex`. This follows from P2. |

- [H] E2 removes the silent failure for 1.7 ms or less. The check runs once for
  each build or load, not for each query.
- [V] A binary search in `LinkResolver::get_by_path` and the merge in
  `assemble_internal` need the order (`a1`, section E).

## 10. `CandidateTrie` and `indextree` (topic F)

- [V] The trie serves link resolution by basename. It does not serve a source
  lookup (`a1`, section F).
- [V] Research 11 measured flat scan and trie for 8 to 512 same-stem candidates.
  The trie builds in 17.7 µs to 314 µs. A lookup costs 60 ns. Break-even is 4 to
  15 lookups.
- [V] New probe (`traces index` after a full rebuild, 12 cores, notes that all
  link to `[[index]]`). With K notes named `index.md` in K folders:

| K | With the trie (median) | Flat only (median) |
|---|------------------------|--------------------|
| 128 | 32 ms to 34 ms | 33 ms to 34 ms |
| 1,000 | 65 ms to 66 ms | 72 ms to 74 ms |
| 5,000 | 224 ms to 226 ms | 412 ms to 436 ms |

- [V] A flat scan at 5,000 same-stem candidates adds about 190 ms to 210 ms to a
  full build on 12 cores. The trie is a safeguard for a vault of this kind. Such
  a vault is rare.
- [H] The flat-only variant was a throwaway copy with `TRIE_THRESHOLD` set to
  `usize::MAX`. The repository is not changed.
- [H] Deleting the trie also deletes `indextree` and about 500 lines. A
  sorted-candidate search can replace it only if it keeps the tie rule
  (equal-distance candidates give `None`). Prove this with a differential test.

## 11. Store engine and repository split (topic F)

- [V] `IndexStore` has about 3,000 lines (`src/index/store.rs`). It holds the
  open, the schema check, the rebuild trigger, the table writers and the
  readers.
- [V] zk splits a port (`NoteIndex`) from one SQLite adapter and separate access
  objects for notes, links, collections and metadata (`c1`, finding 28).
- [H] A split pays when two consumers need different parts. The consumers are:
  the refresh (writer and bulk reader), the query (point reader) and a host (a
  reader of facets). The facet split of topic D gives the natural seam: a
  `FacetReader` for queries and a `StoreWriter` for the refresh.
- [P] Decide the split after topic D. A split before the facets creates a seam
  that the facets then move.

## 12. Inconsistencies and gaps found in this prep

1. [V] The ticket evidence gate omits the `CandidateTrie` measurement of
   research 11. Topic F asks a question that research 11 and the probe above
   already answer in part.
2. [V] The ticket says the four tables exist for the pushdown. This is true. Two
   of them have no query reader. They keep the forward tables in step.
3. [V] Research 11 advises a held `IndexStore` handle (item 8 of its summary). A
   held handle makes every other `traces` process fail. The research does not
   say this.
4. [V] Research 11 measured a memory ratio of 2.2 to 1 at 20,000 notes. The
   probe on denser notes gives 9 to 1. The ratio depends on the note content.
5. [V] The ticket says "The bulk index path collects [notices]". No code
   collects them today. The sentence describes a decision of ticket 23.
6. [V] The ticket Fixed inputs say that the store persists "one facts-only
   `Note`". Topic D asks if the store keeps whole notes or facets. The fixed
   input fixes the content. Topic D fixes the layout. The wording hides this.
7. [V] `IndexerService::load` has no production caller (`a1`, narrow `pub`
   items). The template path calls `refresh`. The CLI path calls
   `refresh_store`.
8. [V] The `b1` file uses ticket line numbers in several citations. The ticket
   text changes. Use topic names.
9. [V] A path-set change decodes every note even on the store path (probe). The
   boundary with ticket 25 says that ticket 25 decides the refresh note cache.
   The facet split of this ticket changes the size of that cache.
