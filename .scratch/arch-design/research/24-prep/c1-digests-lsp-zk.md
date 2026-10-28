# Digest evidence for ticket 24: zk and Markdown language servers

This file records precedents, not decisions. Each finding names the concepts
that it informs. The citations use line numbers in the digest files.

## A. Consumers and execution paths

1. **C1, C17 — one indexed query port.** [V] `Notebook` holds a `NoteIndex`
port. The SQLite adapter serves it. The CLI opens `.zk/notebook.db` for it.
`docs/digests/zk-src-digest.txt:17267-17298,18326-18366,12573-12585,11112-11153`.

   Lesson for ticket 24: A — Compare the needs of the CLI and the hosts before
   adding a second query path.

2. **C2, C8 — SQL columns.** [V] `NoteDAO` selects only an ID, a minimal note,
or a full note. The full selection adds body, raw content, dates, tags, and a
snippet. The minimal and full methods read rows from the same `findRows` method.
`docs/digests/zk-src-digest.txt:9264-9315,9384-9393,9701-9732`.

   Lesson for ticket 24: A — A single query builder can select different amounts
   of data for different consumers.

3. **C2, C9 — SQL filters.** [V] An exact text filter uses `LIKE`; an FTS filter
uses `MATCH`. A regex filter uses `REGEXP`. A tag filter uses `collections` and
`notes_collections`. Link filters use IDs from `links`. The builder combines
predicates with `AND` and adds sorting and a limit.
`docs/digests/zk-src-digest.txt:9404-9471,9475-9495,9514-9559,9664-9732`.

   Lesson for ticket 24: A — Specify the plan for each source shape. Do not
   infer all-shape speed from one selective filter.

4. **C1, C18 — Selective loads.** [V] `FindMinimal` and `Find` scan only the SQL
result rows. zk keeps a map of opened `Notebook` objects, not a full vault of
decoded notes in that map. The LSP keeps text for opened documents in a separate
map.
`docs/digests/zk-src-digest.txt:9264-9315,18730-18755,18766-18796,3192-3209`.

   Lesson for ticket 24: A — Separate the cost of notebook handles and open
   buffers from the cost of all decoded notes.

## B. Read model, generation, and failure

5. **C3, C8 — SQLite facts.** [V] `notes` has path, title, lead, body, raw
content, word count, checksum, and timestamps. Later migrations add metadata and
filename. `links` has source and target IDs, href, title, relation text, and
snippet data. Collections and their note associations hold tags.
`docs/digests/zk-src-digest.txt:8247-8278,8301-8328,8332-8340,8379-8384`.

   Lesson for ticket 24: D — A relational precedent splits text, links, and
   tags, but its schema does not fix the `Note` facet split.

6. **C9, C10 — SQLite graph.** [V] Triggers update `notes_fts` after a note
insert, delete, or update. The `links` relation stores source and target IDs. zk
resolves old links again when new paths arrive.
`docs/digests/zk-src-digest.txt:8265-8297,11239-11293`.

   Lesson for ticket 24: D — Keep a secondary relation only when a named query
   or update uses it.

7. **C5, C15 — Failed writes.** [V] `Notebook.IndexWithCallback` passes the
index task to `NoteIndex.Commit`. SQLite rolls the transaction back when the
callback returns an error. The index task logs parse and per-note write errors
and continues its path diff.
`docs/digests/zk-src-digest.txt:18372-18396,11441-11449,11948-11969,17421-17462`.

   Lesson for ticket 24: B — Define whether a logged note-write error can leave
   an incomplete generation that the query can see.

8. **C3, C18 — Change scan.** [V] `Indexed` reads the path and modified time
from the SQLite index. The task walks the notebook and compares the walk with
indexed paths. It parses added and modified notes. It removes deleted notes.
`docs/digests/zk-src-digest.txt:8980-8984,9042-9076,17410-17455`.

   Lesson for ticket 24: B — Keep the scan manifest separate from the cost of
   decoding all note records.

9. **C4, C8 — zk records a rebuild flag after a schema migration.** [V] The
migration sets `reindexingRequiredKey` in SQLite metadata. The next index task
makes a full reindex if it sees this flag. The task clears the flag after the
path diff and the link update.
`docs/digests/zk-src-digest.txt:8388-8410,11452-11471,17346-17362,17460-17479`.

   Lesson for ticket 24: B — A format change needs a clear boundary between old
   facts and the new generation.

10. **C4, C15 — marksman replaces the state value after an edit.** [V] `State`
has a workspace and an integer revision. `State.updateFolder` creates a new
workspace and increments that revision. `Doc.withText` parses new text and
builds a new document index. A `MailboxProcessor` gives a request one state
value and serializes state mutations.
`docs/digests/lsp_artempyanykh-marksman-digest.txt:12148-12174,12227-12243,6928-6937,11382-11452`.

    Lesson for ticket 24: B — A request can use one state revision while the
    host prepares the next state.

11. **C4, C15 — Folder load.** [V] Folder loading walks nested directories and
reads each selected Markdown document. `Folder.multiFile` puts the documents in
a path map and builds lookup and connection state. The LSP loads all workspace
folders during initialization.
`docs/digests/lsp_artempyanykh-marksman-digest.txt:7461-7500,7617-7638,7648-7666,11504-11526`.

    Lesson for ticket 24: E — You can scope state totality to the selected
    document set. Policy-excluded files stay outside that set.

12. **C4, C18 — markdown-oxide protects reads with a vault lock.** [V] Its
backend holds `Arc<RwLock<Option<Vault>>>`. A query runs inside the read guard.
An edit updates the vault under the write guard. A watched-file event constructs
a new vault and replaces the old value under that write guard.
`docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt:1670-1678,1726-1744,1751-1775,1861-1898,2244-2245`.

    Lesson for ticket 24: B — A lock can keep one request on one vault version.
    Full rebuilds still stop readers during replacement.

13. **C4, C12 — a diagnostics counter, not an index generation.** [V] Each edit
increments `diag_gen`. A delayed diagnostics task exits when a newer edit
changed the counter. Diagnostics read the vault under one read lock. The server
publishes results for open files.
`docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt:1660-1668,1685-1711,1800-1833`.

    Lesson for ticket 24: B — Do not treat a debounce counter as proof that
    stored rows share one policy generation.

14. **C5, C8 — Optional cache.** [V] A cache read error or invalid JSON becomes
a miss. A failed atomic write logs an error and does not fail the result write
method. The cache writes a temporary file and renames it to the final path.
`docs/digests/lsp_rvben-rumdl-src-digest.txt:345-363,636-674,760-776`.

    Lesson for ticket 24: B — Distinguish a failed derived-cache write from an
    invalid index that a query must not use.

## C. Rows and vault loading

15. **C7, C18 — Lazy notes.** [V] Its vault holds a vector of notes and a root
path. Its documentation recommends `NoteOnDisk` for large vaults because it
reads files on demand. `NoteOnceLock` stores a path and separate lazy cells for
content and properties. The builder collects the selected note handles in a
vector.
`docs/digests/rust_cryptogladi-obsidian-parser-digest.txt:3179-3185,3203-3229,1809-1827,3805-3839`.

    Lesson for ticket 24: C — A row can borrow a note handle, but a handle does
    not make content free to load.

16. **C7, C18 — Basalt paths.** [V] `Note` stores a name and path, not parsed
Markdown. A vault rename operation walks its note entries, then reads each file
that it checks for links.
`docs/digests/rust_erikjuhani-basalt-digest.txt:10925-10942,11087-11119`.

    Lesson for ticket 24: C — A path-only row saves resident memory but moves
    file reads to consumers.

## D. Persisted data and in-memory indexes

17. **C8, C11 — JSON lint cache.** [V] The file path joins a version directory
and two hash parts. The parts are the file content hash and 16 hex characters of
the rules hash. The JSON entry holds the hashes, warnings, an optional
dependency fingerprint, and a timestamp.
`docs/digests/lsp_rvben-rumdl-src-digest.txt:322-329,424-445,575-584`.

    Lesson for ticket 24: D — Do not equate a cached result for one file with
    persisted facts for all workspace queries.

18. **C5, C8 — Cache validity.** [V] A different content hash, config hash,
rules hash, or version makes a miss. If a rule needs cross-file state, an absent
or different dependency fingerprint makes a miss.
`docs/digests/lsp_rvben-rumdl-src-digest.txt:678-713`.

    Lesson for ticket 24: D — A per-file cache needs the cross-file state in its
    validity rule when a result depends on it.

19. **C1, C9 — LSP index.** [V] An `IndexWorker` holds
`Arc<RwLock<WorkspaceIndex>>`, an index state, workspace roots, and open
documents. Its comments say navigation, completion, and rename use this index
even if a diagnostic rule is off. The worker updates one file under the index
write lock. It then asks for a re-lint of dependents.
`docs/digests/lsp_rvben-rumdl-src-digest.txt:36627-36644,36680-36717,36850-36884`.

    Lesson for ticket 24: D — Treat an LSP cross-file index and a disk cache of
    lint answers as different jobs.

20. **C3, C4, C15 — In-place scan.** [V] The worker removes stale entries before
the scan ends. It then indexes files one at a time under the index write lock.
It reports building progress. A reader can see an index while that scan is in
progress. `docs/digests/lsp_rvben-rumdl-src-digest.txt:36957-37018,37030-37050`.

    Lesson for ticket 24: B — An in-place scan needs an explicit read rule if a
    query must see only one complete generation.

21. **C4, C20 — Buffer input.** [V] The worker takes text from open documents
for the full scan. It reads other files from disk. An open file stays in the
scan even when normal discovery excludes it, if the file exists.
`docs/digests/lsp_rvben-rumdl-src-digest.txt:36941-36955,36975-36995,37030-37035`.

    Lesson for ticket 24: B — State which generation owns open-buffer facts when
    the disk and editor versions differ.

22. **C8, C9 — delegated workspace ownership.** [V] The caller supplies
`IWorkspace` and `IMdParser`. The workspace interface gives a full document
list, an open-by-URI operation, and change events. It returns a cached document
or opens one. A document has a URI, version, and text operations.
`docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt:1730-1746,2438-2483,18162-18201`.

    Lesson for ticket 24: D — Keep the host's document provider distinct from
    the library's derived caches.

23. **C3, C8 — two lazy cache scopes.** [V] `MdDocumentInfoCache` loads one
document when a feature asks for it. `MdWorkspaceInfoCache` gets all documents
on its first use and registers each one. It computes each value on first access.
A workspace-symbol query asks for all values, then filters symbols.
`docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt:2661-2705,2768-2834,7499-7538`.

    Lesson for ticket 24: D — An all-workspace query can force every lazy value.
    Count this load in the query budget.

24. **C4, C8 — vscode-markdown-languageservice replaces derived cache entries on
events.** [V] A document change or creation updates the workspace cache entry. A
deletion removes it. The per-document cache cancels an old computation before it
replaces the entry.
`docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt:2684-2685,2730-2764,2792-2794,2836-2856`.

    Lesson for ticket 24: B — Per-document invalidation does not alone define a
    coherent generation for a multi-document query.

25. **C9, C10 — SQL relations.** [V] A tag filter reads IDs from the tag
relations. A link filter reads source or target IDs from `links`. Full-text
search reads the FTS relation. zk queries these relations before it selects the
requested columns.
`docs/digests/zk-src-digest.txt:9404-9471,9482-9488,9514-9559,9701-9732`.

    Lesson for ticket 24: D — Name the source shape and result columns before
    you keep a secondary table.

## E. Totality and partial indexes

26. **C15, C18 — Vault handles.** [V] obsidian-parser collects all selected note
handles into `Vault.notes`. Its on-disk note form reads content on demand, while
`NoteOnceLock` caches content and properties on first access.
`docs/digests/rust_cryptogladi-obsidian-parser-digest.txt:3179-3185,3215-3229,1809-1827,3820-3840`.

    Lesson for ticket 24: E — Define totality over entries separately from the
    amount of note content resident in memory.

27. **C15 — marksman has a separate single-file workspace mode.** [V] A
multi-file folder stores a path map of its documents. When the LSP opens a
Markdown file outside a workspace folder, it creates a single-file folder for
that file.
`docs/digests/lsp_artempyanykh-marksman-digest.txt:7622-7638,11638-11674`.

    Lesson for ticket 24: E — A deliberately scoped workspace is not an
    accidental partial result from a whole-vault query.

## F. Index internals

28. **C17 — SQLite adapter.** [V] The core port declares find, link, collection,
and write operations. The SQLite adapter holds the database and the note, link,
collection, and metadata access objects.
`docs/digests/zk-src-digest.txt:17267-17298,11112-11135,11474-11486`.

    Lesson for ticket 24: F — Split an engine from a repository only if each
    part has a distinct job for consumers.

29. **C16, C18 — Worker facts.** [V] The worker builds `FileIndex` from content
and the two cross-file rules. It updates the workspace index for each changed
file and finds its dependent files. The digest does not give a measured
50,000-note budget for this index.
`docs/digests/lsp_rvben-rumdl-src-digest.txt:36627-36669,36850-36884`.

    Lesson for ticket 24: F — Measure a proposed tree or trie against the
    queries that use it. This precedent does not measure that cost.
