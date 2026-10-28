# Grilling guide for ticket 24

This guide helps the grilling of
`issues/24-index-read-model-and-query-execution.md`. Each recommendation is a
proposal [P]. The user decides. `g-probe-results.md` holds the measurements.
`h1-stress-test-designs.md` holds the stress tests and the list of
inconsistencies. The sources of each fact are in `a1`, `b1`, `c1`, `c2`, `e1`
and `e2`.

## What changed since the ticket was written

- [V] A cold query with a `Tag` or `Path` source decodes only the matching
  notes. A query with no source, a `Not` source or a filter only decodes every
  note. At 50,000 notes the first group takes 117 ms to 127 ms and 35 MB to 52
  MB. The second group takes 245 ms to 259 ms and 389 MB to 393 MB.
- [V] A process that opens a held `index.redb` fails at once. The real CLI exits
  with code 1. A host that holds the store open (research 11, advice 8) makes
  every CLI call fail.
- [V] One added file makes the next query decode every note: 499 ms and 395 MB
  at 50,000 notes. The path-set change is a read-model problem as well as a
  refresh problem.
- [V] Two of the four multimap tables have no query reader.
- [V] A meta-facet scan costs about 21 ms at 50,000 notes on a stand-in record.
  A multimap lookup costs 0.14 ms. A scan of whole records costs 33 ms to 42 ms
  before any decode.
- [V] A flat basename scan costs about 200 ms more than the trie at 5,000
  same-stem candidates. It costs the same up to about 1,000.

## Order of the questions

The ticket puts topic A first. The evidence says that the answer to topic A
depends on the answer to topic D (the facet split). Use this order:

1. Consumers and query shapes (topic A, first part).
2. The facet split (topic D, first part).
3. One plan or two (topic A, second part).
4. The generation model and the row source (topics B and C).
5. The failed-persist rule and a second process (topic B).
6. The tables, the link graph and the other stored facts (topic D).
7. Totality (topic E).
8. `CandidateTrie`, `indextree` and the store split (topic F).
9. The change-table rows and the closing checklist.

## Question 1: Which consumers and which query shapes need what?

Ask: list the consumers. For each, name the query shapes and the vault size that
the consumer must serve without a full decode.

Facts:

- [V] The cold CLI runs `list`, `table` and `task`. `traces index` rebuilds. The
  template engine calls `refresh` once for each render and keeps the index in
  the minijinja `State` (`a1`, section A).
- [V] The LSP is committed and the MCP server is likely (ticket 17). Neither
  exists in the code.
- [V] `traces task` has no source today. It decodes every note.

Options and proposals:

- [P] Treat the cold CLI and the host as different consumers. The CLI needs low
  memory and a fast start. A host holds a snapshot and needs one generation for
  each request.
- [P] Name the shapes that must avoid a full decode: a tag source, a folder
  source, a class source and a filter on frontmatter. A task filter on status
  needs the items. A source-less `list` needs paths only.
- [H] A user with 50,000 notes runs shape-less queries often. Ask the user which
  shapes they run on their own vault.

## Question 2: How does the store split the `Note`?

Ask: does the store keep a whole `Note`, independent facets or extracted facts?
If facets, which split and what loads when?

Facts:

- [V] The stored bytes on the documentation corpus are: list items 77.7%,
  outlinks 14.1%, headings 5.6%, frontmatter 0.5%, tags 0.03% and the rest 2.1%
  (`research/23-prep/g-probe-results.md`).
- [V] Ticket 23 says four facets are independent: meta, headings, links and
  items.
- [V] A decoded note takes about 7 KB at 50,000 synthetic notes of 2.1 KB. The
  ratio is larger than the 2 times that ticket 23 measured on documentation
  notes (`g-probe-results.md`).

Options:

| id | Option | Effect |
|----|--------|--------|
| D1 | One whole `Note` record (today). | A filter on frontmatter decodes all bytes. |
| D2 | Four facet tables keyed by path. | A filter on frontmatter decodes about 3% of the bytes [H]. A task query reads items and heading ranges. A path-set change reads outlinks only [H]. |
| D3 | D2 and the item text in a fifth table. | A task filter on status, date or priority can skip the text of rows that fail [H]. A printed row needs the text. |
| D4 | Extracted fixed columns (tags, class, key scalars). | Fastest filters. A new schema for each queried field [H]. |

[P] D2. D3 only if the facet decode shows that item text dominates a task query
at 50,000 notes. D4 is too early.

Gate before the choice [P]: decode the real meta facet and the real outlinks
facet at 50,000 notes. The stand-in probe used strings only. The real
`NoteFieldValue` and the dates (parsed from text today) can cost more. rkyv
would remove the decode. It also freezes the schema and needs a validation pass
(`e2`). Facets with postcard are the middle path.

Write each note in one transaction for all facets. One generation then never
holds facets of two parses.

## Question 3: One execution plan or two?

Ask: does the query service keep a store path and a memory path, or one plan?

Candidates (`h1`, section 3):

- S0: keep the two paths.
- S1: memory path only.
- S2: memory path with in-memory sets.
- S3: facet store and one plan.
- S4: facet store and two forward tables as `Inexact` pruning.
- S5: fixed columns.
- S6: a lazy view over one read transaction.
- S7: SQLite.

Facts:

- [V] S1 costs +13 ms and +36 MB at 5,000 notes. At 20,000 notes it costs +46 ms
  and +147 MB. At 50,000 notes it costs +130 ms and +358 MB.
- [H] The tables help the cold CLI only. A host scan of 50,000 entries for one
  tag takes about 1 ms to 2 ms.
- [V] The two paths disagree on a class source when no expander is attached.
  Ticket 20 records the hazard and says production always attaches one.

[P] S3 with one residual filter. A pruning table, if any, only prunes. It never
decides a result. Gate: the facet scan at 50,000 notes costs at most about 35 ms
for a tag or class source. If it fails the gate, use S4 and drop the two reverse
tables.

What becomes removable under S3 [H]: four multimap tables, `SourceResolver`, the
partial `WorkspaceIndex`, the sorted-set intersection and the reverse-table
maintenance in the writer.

What stays unresolved: the real facet decode cost, and whether a user accepts
about 20 ms more for a tag source at 50,000 notes.

## Question 4: Generation model and row source

Ask: what is one generation, how does a host move to the next one, and what does
a row borrow from?

Facts:

- [V] A new generation by a clone of `Vec<Arc<Entry>>` takes 120 µs at 50,000
  entries.
- [V] rust-analyzer and Tantivy give each request an immutable snapshot (`e1`,
  sources 1 and 10). markdown-oxide and rumdl update in place under a lock
  (`c1`, findings 12 and 20).
- [V] `QueryRow` holds `Arc<WorkspaceIndex>` and a position today. The store
  path builds a partial `WorkspaceIndex` to satisfy this
  (`src/query/results.rs:45-50`).
- [V] redb and LMDB readers pin pages. A long-held read transaction grows the
  file. The precedent is a short read transaction and a replaced snapshot
  (`e2`).

Options for the generation (`h1`, section 6): G1 `Arc` snapshot, G2 snapshot of
`Arc<FileEntry>`, G3 in-place under a lock, G4 two copies.

Options for the row (`h1`, section 7): P1 `Arc<WorkspaceIndex>` and a position,
P2 `Arc<FileEntry>`, P3 an owned record, P4 a borrow from a read transaction.

[P] G2 with P2. A refresh builds a new vector, shares unchanged entries and
replaces the changed ones. A host swaps one pointer. A request pins one
generation. The store path builds `Arc<FileEntry>` for the candidates only.

Unresolved: the exact content of `FileEntry` after the facet split (facets
loaded on demand inside an entry). Ticket 26 owns the row type. Ticket 24 owns
the source of the borrow.

## Question 5: Failed persist and a second process

Ask: what does a query do when the store is held, corrupt or not writable? What
does a second process see?

Facts:

- [V] `refresh` logs a persist failure and returns the in-memory index.
  `refresh_store` returns an error (`src/index/service.rs:259-264,285-293`).
- [V] A second process fails at once with `DatabaseAlreadyOpen`. The real CLI
  exits with code 1.
- [V] `ReadOnlyDatabase` cannot open a store that a writer holds.
- [V] The redb multi-process mode lets a second process read and write. It is
  behind an `experimental-` flag and refuses `Durability::None`.
- [V] A full parse and build takes about 1.8 s at 50,000 small notes and 0.22 s
  at 5,000 notes.
- [V] rumdl, Dataview and Tasks treat a cache failure as a miss (`c1`, finding
  14; `c2`, findings 8 and 29).
- [V] ruff exited with code 2 when a multi-process race failed a cache write.
  Issue 12284 made this failure non-fatal. Git and Cargo also treat a derived
  cache as disposable (`e2`).
- [V] Ticket 21 does not promise concurrent index writers with different
  policies for one root. Ticket 30 owns writer admission inside a host. This
  question owns what a second process sees.

Rules for a failed persist (`h1`, section 4):

- F1: two rules (today).
- F2: fail closed.
- F3: fail open for the cache.
- F4: fail open with a notice.

Rules for a second process (`h1`, section 5):

- W1: a host holds the store.
- W2: a host opens the store for each write.
- W3: a host never opens the store.
- W4: the redb multi-process mode.
- W5: one index daemon.
- W6: F3 and W2 together.

[P] F4 with W2. A query never fails because the cache is unusable. It parses
fresh and prints one line on stderr. `traces index` fails closed. A host opens
the store for a write and closes it. This asks the host to batch writes, because
open and close cost about 25 ms.

What this costs: an exit code changes from 1 to 0 for a held store. Add a
change-table row (ticket 16). A cold CLI beside a busy host can pay a full
parse. Revisit W4 when redb stabilises the mode.

Research 11, advice 8 (hold an open `IndexStore`) conflicts with W2. Resolve it
in the answer. A host that holds the handle gets 18 µs commits. It blocks every
CLI call.

## Question 6: Tables, link graph and stored facts

Ask, for each stored thing: which query or update needs it?

| Thing | Named reader | Proposal [P] |
|-------|--------------|--------------|
| `paths_by_tag` | a `Tag` source | Drop under S3. Keep under S4. |
| `paths_by_file_class` | a `Class` source | Drop under S3. Keep under S4. |
| `tags_by_path` | the writer only | Drop with the forward tables. |
| `classes_by_path` | the writer only | Drop with the forward tables. |
| `links` (inbound edges) | `file.inlinks` and the refresh patch | Keep until the outlinks facet is measured. |
| `meta` (format, fingerprint) | open and refresh | Keep. |
| `FileMeta.day` | a hybrid of filename and creation time | Drop. Ticket 21 removes the hybrid. |
| `FileMeta` timestamps and size | `file.ctime`, `file.mtime`, `file.size` and the refresh delta | Keep as optional source timestamps. |
| temporal text | parsed on each load | Defer a compact form until the facet decode is measured. |
| notices | none today | Do not persist in the first cut. Ask ticket 31. |
| "has a known explicit class" | the meta facet at query time | No table. |
| class axis over inline fields | none | Frontmatter only. Ticket 20 names three class sources and inline fields are not one. |

Constraint for ticket 29: the persisted path form is a relative UTF-8 path with
`/` (research 09). This guide does not choose it.

## Question 7: Totality

Ask: is "one entry for each file, sorted by path" enforced by a type?

Facts:

- [V] `assumed_sorted` checks the order in debug builds only
  (`src/index/sort.rs:42-56`).
- [V] A release check of a sorted list takes 1.7 ms at 50,000 paths with `Path`
  comparison. It takes 0.12 ms with `OsStr` comparison.
- [V] A binary search in `LinkResolver::get_by_path` and the merge in
  `assemble_internal` need the order.

[P] Check the order in release when you build or load an index (E2). Give the
store path its own type, with rows that own `Arc<FileEntry>` (E3). The partial
`WorkspaceIndex` then disappears.

## Question 8: `CandidateTrie`, `indextree` and the store split

Facts:

- [V] The trie serves basename link resolution. It does not serve a source
  lookup.
- [V] Research 11 recommends deleting it. Its test used 8 to 512 candidates.
- [V] New probe, K notes named `index.md` that all link to `[[index]]`: the full
  build takes 32 ms to 34 ms with and without the trie at K = 128. At K = 1,000
  it takes 65 ms with the trie and 72 ms to 74 ms without. At K = 5,000 it takes
  224 ms with the trie and 412 ms to 436 ms without.

Options: delete the trie and `indextree` (about 500 lines), keep them, or
replace them with a sorted search that keeps the tie rule.

[P] Keep the trie. Extend the `index_inlinks` benchmark to 5,000 candidates. The
removal saves 500 lines but removes a safeguard against a real vault shape
(folder notes with one name). Revisit if ticket 25 makes a path-set change rare.
This reverses the advice of research 11, because the new probe covers a larger
candidate count.

Store engine and repository split [P]: decide after Question 2. A split before
the facets creates a seam that the facets move. The natural seam is a facet
reader for queries and a store writer for the refresh.

## Question 9: Change-table rows and the closing checklist

Rows to expect (ticket 16):

- A held store changes from an error (exit code 1) to a fresh parse.
- A persisted `day` field is gone. Ticket 21 and ticket 27 own the visible
  result.
- The `index.redb` layout and `META_VERSION` change. These are Free.

Closing checklist from the ticket:

- The alternatives, with the retain option.
- The owner of each invariant.
- What becomes removable.
- What stays unresolved.
- The change-table rows.
- The downstream tickets (25, 26, 28, 29, 30, 31, 32 and 34).

## Corrections to the evidence

- [V] Research 11 measured a memory ratio of 2.2 to 1 on 20,000 notes. The new
  probe gives 9 to 1 on denser notes. Use a range.
- [V] Research 11 says the 25 ms floor is the open and close of the database.
  This stays true. A held handle removes it and blocks other processes.
- [V] Two of the four multimap tables have no query reader. They keep the
  forward tables in step. A design without the forward tables drops them too.
- [V] Earlier prep files cite ticket line numbers. Use topic names.

## Not done in this prep

- A real meta facet and a real outlinks facet decode at 50,000 notes.
- A real vault. The probe corpus is synthetic.
- Linux and Windows file locking.
- A host with a writer in each of two processes under the redb multi-process
  mode.
- The inline-field class axis. No code reads inline fields for classes.
