# Obsidian digest evidence for ticket 24

This record describes precedents. It does not decide the ticket. Each source is
a digest under `docs/digests/`. The line numbers below refer to the digest, not
to the original source file.

## Topic A. Consumers and execution paths

1. **C1: One index serves several consumers.** [V] Dataview's `pagePaths` uses its source resolver. Its `page` and `pages` methods read the same in-memory `pages` map. The list and table query functions also use that index (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:2383-2419,8475-8488,8517-8531`).
   Lesson for ticket 24: Topic A — check whether all consumers can share one read model.

2. **C2: Source selection precedes filtering.** [V] Dataview first resolves source paths into rows. It then applies `where`, `sort`, and `limit` operations to those rows (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4715-4733,8233-8255,8256-8307`).
   Lesson for ticket 24: Topic A — separate source candidate selection from row filters in the comparison.

3. **C2: A tag starts from an inverse map.** [V] A tag source calls `tags.getInverse`. A folder source walks the vault folder. A negated source starts with every Markdown path and removes matching paths (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4600-4617,4671-4678,4275-4279`).
   Lesson for ticket 24: Topic A — measure tag, folder, and negation shapes separately.

4. **C2: Links do not always use the same lookup.** [V] Dataview uses its inverse link map for an unresolved target. For a resolved incoming target, it scans Obsidian's `resolvedLinks`. For an outgoing target, it reads that target's resolved links (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4625-4649`).
   Lesson for ticket 24: Topic A — identify the actual lookup cost for each link direction.

## Topic B. Read model and generation

5. **C3, C9: Memory maps.** [V] `FullIndex` holds `pages`, tag maps, exact-tag maps, a link map, and a folder index. The `IndexMap` holds both forward and inverse sets (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4033-4054,4466-4477`).
   Lesson for ticket 24: Topic B — compare the memory cost of in-memory inverse sets with store tables.

6. **C4: A revision marks changes, not an immutable generation.** [V] `touch` increments `revision` and calls `onChange`. File reloads replace page and inverse-map entries before `touch`; deletion and rename also call `touch` (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4102-4106,4116-4129,4180-4197,4237-4251`).
   Lesson for ticket 24: Topic B — do not treat a change counter as proof of an atomic snapshot.

7. **C4, C15: Readiness follows an asynchronous initial load.** [V] Dataview starts a reload for each Markdown file. It sets `initialized` and emits `dataview:index-ready` only after those reloads finish (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4132-4134,4148-4171`).
   Lesson for ticket 24: Topic B — state when a consumer can use an incomplete read model.

8. **C5, C8: Startup cache.** [V] Dataview stores per-file metadata in IndexedDB through localforage. On the first load, it accepts a cached record only when the cache time and version pass its checks. It otherwise imports the file (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3771-3809,4200-4225`).
   Lesson for ticket 24: Topic B — distinguish cache reuse from query execution against the store.

9. **C8: Rebuild clears the persisted cache.** [V] Dataview's `reinitialize` drops the IndexedDB instance and reloads all Markdown files. Initial loading also removes cached keys for missing files (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4136-4146,4173-4177,3812-3820`).
   Lesson for ticket 24: Topic B — define the rebuild path and removal of stale records.

10. **C15: Missing metadata.** [V] Dataview skips an uncached file when Obsidian has no metadata entry. A later `resolve` event can load that file. `resolveMarkdownData` returns no rows when `pages` has no entry (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4209-4224,4110-4112,4702-4712`).
    Lesson for ticket 24: Topic B — make partial-index behavior explicit instead of calling it a full snapshot.

11. **C15, C20: Paths and pages can differ during loading.** [V] Dataview's `pagePaths` returns matching source paths. Its `pages` method omits a path if `page` finds no page metadata (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:2383-2419`).
    Lesson for ticket 24: Topic B — check the observable result when a path exists before its record.

## Topic C. Rows

12. **C7: Data rows.** [V] Each Markdown row gets a file-link identity and serialized page data. An absent page gives an empty row list. The query engine then passes these rows to its shared operation loop (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4702-4733,8233-8248`).
    Lesson for ticket 24: Topic C — identify the owner of each row's data and identity.

## Topic D. Stored facts and lookup axes

13. **C8, C14: Field sources.** [V] The parser adds frontmatter properties to a field map. It merges Markdown inline fields into that map. It also reads inline fields on list items (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3343-3359,3391-3405,3480-3499,3580-3588`).
    Lesson for ticket 24: Topic D — decide whether a class lookup sees only frontmatter or also body fields.

14. **C9: Inverse sets update with a file.** [V] `IndexMap.set` replaces a file's values and adds inverse entries. `delete` removes inverse entries. `finish` updates tags, exact tags, and links after parsing a file (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4494-4526,4237-4251`).
    Lesson for ticket 24: Topic D — include the refresh cost of each retained secondary axis.

15. **C10: One backlink cache stores forward and reverse relationships.** [V] Advanced Metadata Cache holds `backlinksMap` and `linksMap` in memory. It removes a source file's old edges before it reads Obsidian's parsed links and adds new edges (`obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt:6371-6385,6518-6536,6640-6687,6723-6746`).
    Lesson for ticket 24: Topic D — compare recomputation of inlinks with persistence of the graph.

16. **C10: Cache miss.** [V] Advanced Metadata Cache returns after Obsidian's metadata cache gives no entry. It listens for `changed`, `modify`, `create`, `delete`, and `rename` events to queue work (`obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt:6505-6515,6566-6589,6663-6675`).
    Lesson for ticket 24: Topic D — specify how a missing parsed entry becomes visible later.

17. **C10: Pending work.** [V] The advanced backlink cache processes queued actions until the queue is empty. Its safe backlink method waits for that processing. Its ordinary method queues processing after a read (`obsidian_mnaoumov-obsidian-advanced-metadata-cache-src-digest.txt:6395-6409,6424-6435,6543-6545,6606-6638`).
    Lesson for ticket 24: Topic D — define whether a read waits for pending invalidations.

18. **C8, C15: Task states.** [V] Obsidian Tasks keeps one task array and a `Cold`, `Initializing`, or `Warm` state. It loads Markdown files after layout readiness and again after the first metadata resolution (`obsidian_obsidian-tasks-src-digest.txt:9971-9995,10002-10010,10035-10053,10093-10107`).
    Lesson for ticket 24: Topic D — state both cache readiness and the meaning of a partial initial load.

19. **C8, C12: Read failure.** [V] Tasks reads a file only if its metadata lists task items. A failed file read retains the prior tasks. An unchanged parsed task list causes no update. A changed list replaces that file's tasks and notifies subscribers (`obsidian_obsidian-tasks-src-digest.txt:10232-10311`).
    Lesson for ticket 24: Topic D — distinguish a read failure from a confirmed empty task list.

20. **C8: Task heading.** [V] Tasks finds a preceding heading by walking the cached heading list up to the task line. It returns no heading if the heading cache is absent (`obsidian_obsidian-tasks-src-digest.txt:10373-10387`).
    Lesson for ticket 24: Topic D — include heading ranges when a task row needs its heading.

21. **C13: Explicit class.** [V] Metadata Menu reads the configured class alias from the note's frontmatter. It accepts an array or a comma-separated string. It maps recognized class names to that file's fields (`obsidian_mdelobelle-metadatamenu-src-digest.txt:18544-18573`).
    Lesson for ticket 24: Topic D — separate a known explicit class from an unknown class name.

22. **C13: Other bindings.** [V] Metadata Menu builds bindings for tags, folder paths, bookmarks, and Dataview query results. It stores the bound class names by note path (`obsidian_mdelobelle-metadatamenu-src-digest.txt:18413-18436,18441-18474,18477-18494,18525-18541`).
    Lesson for ticket 24: Topic D — do not mistake derived bindings for explicit note classes.

23. **C14: One fileclass index reads frontmatter for the explicit alias.** [V] FileclassIndex reads the alias from `getFileCache(file)?.frontmatter`. It obtains tags from `getAllTags(cache)` and separately records the parent folder and bookmark groups (`obsidian_mdelobelle-fileclass-src-digest.txt:11441-11453`).
    Lesson for ticket 24: Topic D — state which class axes use body data and which use frontmatter.

24. **C13, C20: Resolution filters unknown names.** [V] The fileclass resolver adds a class name only when the registry knows it. It checks frontmatter names before tags, paths, bookmark groups, and base-view names (`obsidian_mdelobelle-fileclass-src-digest.txt:12608-12652`).
    Lesson for ticket 24: Topic D — test known and unknown explicit classes as separate source cases.

25. **C8, C17: Another plugin separates class indexing from disk storage.** [V] FileclassIndex clears and rebuilds in-memory maps from class-note frontmatter. Its source says it does not use IndexedDB. Metadata Menu uses IndexedDB for a `fileClassView` store, not for its `FieldIndex` maps (`obsidian_mdelobelle-fileclass-src-digest.txt:11258-11265,11296-11305,11327-11360`; `obsidian_mdelobelle-metadatamenu-src-digest.txt:5274-5277,5281-5315,18839-18863`).
    Lesson for ticket 24: Topic D — separate persistent UI state from the index read model.

## Topic E. Totality

26. **C15: Tasks defers files with no Obsidian cache entry.** [V] Its `indexFile` returns when `getFileCache` gives no entry. Its `resolved` listener starts another vault load after the first metadata resolution (`obsidian_obsidian-tasks-src-digest.txt:10093-10107,10217-10235`).
    Lesson for ticket 24: Topic E — do not infer one indexed entry per file from a file enumeration.

## Topic F. Index internals

27. **C17: Derived maps.** [V] `FileclassIndex.rebuild` clears class maps and scans class notes. It then builds inheritance and binding maps and emits `fileclass:indexed`. A missing class-note cache entry supplies no frontmatter during that scan (`obsidian_mdelobelle-fileclass-src-digest.txt:11327-11346,11349-11376`).
    Lesson for ticket 24: Topic F — state which component owns each derived map and its rebuild event.

## Further evidence by topic

28. **C19, Topic A: API surface.** [V] Dataview exposes `pagePaths`, `page`, and `pages` as public plugin methods. `pagePaths` selects paths, and `page` reads a page from the in-memory map (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:2383-2419`).
    Lesson for ticket 24: Topic A — name the read-model operations that external consumers need.

29. **C5, Topic B: Cache write.** [V] Dataview calls `storeFile` after it finishes an imported page. It does not await that call in `import`; page indexing finishes independently of the cache write (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4229-4251`).
    Lesson for ticket 24: Topic B — specify whether a failed cache write can leave a usable in-memory result.

30. **C11, Topic D: Date fields.** [V] Dataview's page parser stores the file creation time, modification time, size, and `day`. It finds `day` in date fields or the filename (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:3399-3409,3631-3648`).
    Lesson for ticket 24: Topic D — test a filename-derived day separately from file timestamps.

31. **C12, Topic D: Parse errors.** [V] Tasks passes a reporting callback to its file parser. During initialization, the callback gives a notice with the file path and task line. It also logs the error (`obsidian_obsidian-tasks-src-digest.txt:10314-10322,10325-10355`).
    Lesson for ticket 24: Topic D — define how parser notices survive a cache load.

32. **C16, Topic F: Folder lookup.** [V] Dataview names its folder helper `PrefixIndex`, but `get` walks the vault folder tree. It builds a set of matching file paths for each call (`obsidian_blacksmithgu-obsidian-dataview-src-digest.txt:4255-4279`).
    Lesson for ticket 24: Topic F — measure folder lookup before keeping a separate path trie.
