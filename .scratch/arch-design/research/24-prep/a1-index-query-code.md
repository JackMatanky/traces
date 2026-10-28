# Index and query code inventory

[V] This record describes the current `src/` code. It makes no design choice.

## A. Consumers and execution paths

[V] The CLI uses one shared store entry point for `list`, `table`, and `task`.
The template engine uses a render-local memory index.

| API or constructor | Production call site and purpose | Evidence |
|---|---|---|
| `IndexerService::refresh` | The template query cache loads one index per render. | [V] `src/template/engine/query.rs:195-227` |
| `IndexerService::refresh_store` | The CLI synchronizes the store before a query. | [V] `src/cli/mod.rs:335-365` |
| `IndexerService::rebuild` | `traces index` rebuilds and persists the full index. | [V] `src/cli/index.rs:38-48`; `src/index/service.rs:249-253` |
| `QueryService::run_from_store` | The CLI runs the prepared builder against the store. | [V] `src/cli/mod.rs:470-487` |
| `QueryService::run` | The template query engine runs against its cached index. | [V] `src/template/engine/query.rs:195-227` |
| `WorkspaceIndex::assemble` | The build and unchanged refresh assemble all files, notes, and links. | [V] `src/index/service.rs:94-104`; `src/index/service.rs:194-205` |
| `WorkspaceIndex::assemble` | A changed refresh assembles the reconciled rows. | [V] `src/index/refresh.rs:531-540` |
| `WorkspaceIndex::assemble` | A load assembles all persisted rows. | [V] `src/index/service.rs:403-406` |
| `WorkspaceIndex::assemble` | The store query assembles only candidate files and notes. | [V] `src/query/service.rs:130-169` |
| `WorkspaceIndex::new` | `assemble_internal` converts the sorted files into entries. | [V] `src/index/entry.rs:80-103` |
| `WorkspaceIndex::for_test` | This constructor is behind a test gate, not a production call. | [V] `src/index/entry.rs:47-77` |

[V] `refresh_query` serves `list` and `table` through page mode. It serves
`task` through task mode. See `src/cli/list.rs:85-90`, `src/cli/table.rs:84-89`,
`src/cli/task.rs:221-225`, and `src/cli/mod.rs:335-365`.

[V] `QueryService::sync_and_run` also calls `refresh_store` and
`run_from_store`, but a compile gate limits that path to tests and test
utilities. See `src/query/service.rs:186-195`.

### IndexStore methods and production callers

| Method | Production caller and function | Evidence |
|---|---|---|
| `open` | `RefreshPlan::collect` opens the cache beside a file scan. `IndexerService::persist` and `load` open it too. | [V] `src/index/refresh.rs:340-350`; `src/index/service.rs:365-366`; `src/index/service.rs:403-406` |
| `read_notes_batch` | The store query gets candidate notes. | [V] `src/query/service.rs:137-165` |
| `read_files_batch` | The store query gets candidate file metadata. | [V] `src/query/service.rs:137-165` |
| `read_links_for_targets` | The store query gets the candidates' inbound links. | [V] `src/query/service.rs:137-165` |
| `paths_with_tag` | `SourceResolver` selects `Tag`. | [V] `src/query/service.rs:393-397` |
| `paths_with_file_class` | `SourceResolver` selects `Class`. | [V] `src/query/service.rs:397-415` |
| `paths_in_folder` | `SourceResolver` scans paths for `All`, empty `And`, `Not`, and `Path`. | [V] `src/query/service.rs:348-355`; `src/query/service.rs:367-370`; `src/query/service.rs:389`; `src/query/service.rs:417-424` |
| `read_all` | `IndexerService::load` loads the full snapshot. | [V] `src/index/service.rs:403-406` |
| `read_all_notes` | An unchanged refresh loads all notes. A changed pass merges persisted and new notes. | [V] `src/index/service.rs:194-205`; `src/index/refresh.rs:557-593` |
| `read_files_and_fingerprint` | `RefreshPlan::collect` reads files and fingerprint in one read transaction. | [V] `src/index/refresh.rs:340-351`; `src/index/store.rs:528-545` |
| `read_all_links` | The refresh plan reads the old links. An unchanged refresh reads all links. | [V] `src/index/service.rs:194-205`; `src/index/service.rs:222-229` |
| `persist` | The service writes a rebuild. `PendingApply::apply` writes an incremental update. | [V] `src/index/service.rs:375-385`; `src/index/refresh.rs:153-168` |

[V] `IndexStore::poison_note_row` has a test gate.
`IndexStore::write_fingerprint_bytes` has a test gate too. Neither has a
production call site. See `src/index/store.rs:360-380`.

[V] The store candidate resolver selects all file paths for `All`, `Not`, and
empty `And`. It scans all paths for a `Path` atom. See
`src/query/service.rs:348-424`.

[V] `Tag` and `Class` use secondary lookup tables. A filter without a source
uses `All`, so it selects the whole path set before row filtering. See
`src/query/service.rs:137-169`, `src/query/service.rs:393-424`, and
`src/cli/mod.rs:419-427`.

## D. Secondary tables and metadata

| Table | Writer | Reader and query shape |
|---|---|---|
| `files` | [V] Full rebuild `WriteTarget::Files` at `src/index/store.rs:1654-1677`; incremental upsert and delete at `src/index/store.rs:1406-1442`, `src/index/store.rs:1487-1499`. | [V] `paths_in_folder` serves `All`, `Not`, empty `And`, and `Path` at `src/index/store.rs:315-324`, `src/query/service.rs:348-424`. The query reads candidate metadata at `src/query/service.rs:147-165`; refresh reads all files at `src/index/store.rs:539-545`. |
| `notes` | [V] Full rebuild `WriteTarget::Notes` at `src/index/store.rs:1654-1677`; incremental write and delete at `src/index/store.rs:1422-1442`, `src/index/store.rs:1517-1529`. | [V] Every selected note needs a row for page, list, or task queries at `src/query/service.rs:139-169`, `src/query/service.rs:197-208`. Full refresh and load use `read_all_notes` or `read_all` at `src/index/service.rs:194-205`, `src/index/service.rs:403-406`. |
| `links` | [V] Full rebuild `write_links` at `src/index/store.rs:697-719`, `src/index/store.rs:1654-1677`; incremental `apply_inlink_delta` at `src/index/store.rs:1578-1603`. | [V] Store queries read links by selected target at `src/query/service.rs:151-165`. Refresh and load read all edges at `src/index/store.rs:486-510`, `src/index/store.rs:580-604`. A query of `inlinks` reads the row graph at `src/query/results.rs:171-174`. |
| `paths_by_tag` | [V] Full axis write and incremental axis update at `src/index/store.rs:1310-1349`, `src/index/store.rs:1552-1575`; the tag axis maps here at `src/index/store.rs:1717-1727`. | [V] `paths_with_tag` reads the table for a `Tag` source at `src/index/store.rs:269-290`, `src/query/service.rs:393-397`. |
| `paths_by_file_class` | [V] The same forward axis write at `src/index/store.rs:1310-1349`, `src/index/store.rs:1552-1575`; the class axis maps here at `src/index/store.rs:1717-1727`. | [V] `paths_with_file_class` reads the table for a `Class` source at `src/index/store.rs:292-306`, `src/query/service.rs:397-415`. |
| `tags_by_path` | [V] Full reverse axis write and incremental update at `src/index/store.rs:1352-1372`, `src/index/store.rs:1552-1575`; the tag axis maps here at `src/index/store.rs:1730-1739`. | [V] Only the writer reads old values to remove one path's stale forward entries at `src/index/store.rs:1445-1484`. No query source reads this reverse table. |
| `classes_by_path` | [V] Full reverse axis write and incremental update at `src/index/store.rs:1352-1372`, `src/index/store.rs:1552-1575`; the class axis maps here at `src/index/store.rs:1730-1739`. | [V] Only the writer reads old values to remove one path's stale forward entries at `src/index/store.rs:1445-1484`. No query source reads this reverse table. |
| `meta` | [V] Rebuild and incremental persist call `write_meta` at `src/index/store.rs:795-804`, `src/index/store.rs:830-850`; it stores format version and fingerprint at `src/index/store.rs:671-686`. | [V] Store opening checks the schema at `src/index/store.rs:166-185`. Refresh reads the fingerprint with the files at `src/index/store.rs:539-569`. No query source reads this table. |

[V] The class writer reads configured frontmatter values, not inline fields. The
tag writer reads note tags and their segments. See
`src/index/store.rs:1742-1769`.

### Reads of FileMeta values

| Value | Reader and purpose | Evidence |
|---|---|---|
| `day` | The `file.day` query field gets the stored date or null. The scanner first prefers a filename date, then the creation date. | [V] `src/query/results.rs:307-312`; `src/file.rs:66-98` |
| `created_at` | `file.ctime` and `file.cday` use it. Both fall back to `modified_at`. The day derivation also uses it. | [V] `src/query/results.rs:271-289`; `src/file.rs:71-97` |
| `modified_at` | The created fields use it as fallback. `file.mtime` and `file.mday` use it directly. | [V] `src/query/results.rs:271-305` |
| `size` | `file.size` returns its numeric byte count. A refresh compares the complete `FileMeta` value to find changed files. | [V] `src/query/results.rs:262-269`; `src/index/delta.rs:23-46`; `src/file.rs:44-52` |

[V] The store serializes the whole `FileMeta`, including these four values. See
`src/file.rs:44-53` and `src/index/store.rs:1654-1677`.

## F. CandidateTrie and indextree

[V] `BaseNameIndex` selects a flat search for fewer than 64 paths with one
basename. It selects `CandidateTrie` at 64 paths. See `src/index/trie.rs:22-60`.

[V] `CandidateTrie` uses `indextree::Arena` and `NodeId` to find the nearest
folder candidate for a basename link. See `src/index/trie.rs:12`,
`src/index/trie.rs:100-139`, and `src/index/inlinks.rs:362-406`.

[V] `LinkResolver::new` groups file paths by basename and builds each
`BaseNameIndex`. It also uses a sorted file slice for exact path lookup. See
`src/index/inlinks.rs:322-342`, `src/index/inlinks.rs:400-415`.

[V] A fresh build and a changed path set call this resolver to compute inbound
links. Test fixtures call it through `InlinkMap::new`. See
`src/index/service.rs:94-104`, `src/index/refresh.rs:434-452`, and
`src/index/entry.rs:55-76`.

[V] The trie serves link target resolution, not a `Tag`, `Class`, `Path`, or
`All` source lookup. See `src/index/inlinks.rs:362-406` and
`src/query/service.rs:393-426`.

## D. Parser notices

[V] The current `ParserContext` has no notice collection field. `parse_markdown`
returns a `Note` only, and the indexer collects `Vec<Note>`. See
`src/note/parser.rs:312-338`, `src/note/parser.rs:106-115`, and
`src/index/service.rs:328-355`.

[V] Malformed YAML gets logged and becomes empty frontmatter. Invalid tag
candidates leave the tag iterator. Neither becomes a stored notice. See
`src/note/metadata.rs:154-188`, `src/note/parser.rs:374-408`, and
`src/index/store.rs:1654-1677`.

## E. SortedByPath and order users

| Construction | Order user | Evidence |
|---|---|---|
| `assumed_sorted(scan)` and `assumed_sorted(parsed)` in build | `WorkspaceIndex::assemble_internal` merges sorted file and note lists. | [V] `src/index/service.rs:84-104`; `src/index/entry.rs:80-103` |
| `assumed_sorted(scan)` in refresh | `FileDelta::compute` merges current and persisted file lists. | [V] `src/index/refresh.rs:340-351`; `src/index/delta.rs:23-65` |
| `sorted(decoded rows)` and `assumed_sorted(empty rows)` in store reads | Refresh and load receive path-sorted rows. | [V] `src/index/store.rs:463-471`; `src/index/store.rs:486-545` |
| `assumed_sorted(complete notes)` and `assumed_sorted(merged notes)` | Refresh assembles rows and merges note paths. | [V] `src/index/refresh.rs:298-305`; `src/index/refresh.rs:557-593`; `src/index/refresh.rs:531-540` |
| `assumed_sorted(candidate files)` and `assumed_sorted(candidate notes)` | The temporary store index assembles matching rows. | [V] `src/query/service.rs:139-169`; `src/index/entry.rs:80-103` |
| `sorted` and `assumed_sorted` in test fixtures | Fixture assembly gets sorted files and notes. | [V] `src/index/entry.rs:55-77`; `src/index/service.rs:1324-1328` |

[V] `SortedByPath::assumed_sorted` checks order only in debug builds. Its binary
search needs sorted paths. See `src/index/sort.rs:42-56`,
`src/index/sort.rs:77-85`.

[V] `LinkResolver::get_by_path` also uses binary search on file paths.
`SourceResolver::intersect_sorted` uses sorted candidate paths. See
`src/index/inlinks.rs:409-415`, `src/query/service.rs:429-449`.

## A. Tests of the dual path

| Test | What it pins | Conditional effect if store query execution disappears |
|---|---|---|
| `store_backed_and_in_memory_queries_agree_across_representative_selectors` | [V] Page row parity for `All`, tags, paths, `Or`, and `Not` at `tests/integration/query.rs:650-695`. | [H] Remove this differential test when only one query path remains. |
| `runs_from_store_matching_in_memory_lists` | [V] List row structure, content, and order at `src/query/service.rs:1241-1299`. | [H] Remove the cross-path comparison when only one query path remains. |
| `run_from_store_evaluates_query_over_only_matching_notes` | [V] A store tag candidate selects one note at `src/query/service.rs:589-612`. | [H] Remove the store-only execution test with its implementation. |
| `resolves_all_selector_to_all_paths`, `resolves_tag_selector_via_multimap`, `incremental_refresh_removes_a_notes_stale_tag_from_the_multimap`, `resolves_class_selector_via_multimap`, `intersect_sorted_returns_common_paths` | [V] Store candidate selection and set intersection at `src/query/service.rs:491-586`. | [H] Remove these resolver-only tests with `SourceResolver`. |
| `preserves_query_rows_and_list_metadata_across_cold_reload` | [V] The persisted index reload preserves list and task rows at `tests/integration/persistence.rs:108-116`, `tests/integration/persistence.rs:148-157`. | [H] Keep this test if cache persistence remains; it tests a different boundary. |

[V] CLI and template tests also compare output across consumers. See
`src/cli/mod.rs:1244-1305`. [H] Keep or adapt them to check visible output if
one execution path replaces two.

## Compatibility. Narrow pub surfaces

[V] `index` exports `FileEntry`, `WorkspaceIndex`, `IndexError`, `IndexResult`,
and `IndexerService`. Its test-gated exports are `DriftCause`, `IndexSetting`,
`InlinkMap`, and `RefreshReport`. See `src/index/mod.rs:34-45`.

[V] `query` exports `QueryBuilder`, four error and result types,
`SourceSelector`, `QueryRow`, `QuerySet`, and `QueryService`. See
`src/query/mod.rs:89-103`.

| Public item | Current narrow use | Evidence |
|---|---|---|
| `IndexerService::for_tests`; `WorkspaceIndex::for_test` | [V] Both constructors have test or test-utility compile gates. | `src/index/service.rs:61-69`; `src/index/entry.rs:47-55` |
| `IndexerService::load`; `IndexerService::persist` | [V] Production `rebuild` calls `persist`; `load` has no CLI caller. Tests and the test project use both. | `src/index/service.rs:249-252`; `src/index/service.rs:365-406`; `src/lib.rs:529-544` |
| `IndexerService::refresh_with_report`; `RefreshReport` | [V] Production `refresh` uses the method. Tests and benches read the report. | `src/index/service.rs:135-157`; `src/index/refresh.rs:31-49`; `tests/integration/refresh.rs:31-42` |
| `DriftCause`; `IndexSetting`; `InlinkMap` | [V] The public re-exports have test gates. | `src/index/mod.rs:37-42` |
| `QueryService::sync_and_run` | [V] Only tests and test utilities can compile this public method. | `src/query/service.rs:172-195`; `tests/integration/query.rs:667-693` |
| `QueryRow::task_text`, `task_completed`, `status_symbol`, `depth`, `line` | [V] `query::format` consumes them to write task lists. Tests also use the values. | `src/query/results.rs:87-153`; `src/query/format.rs:220-255` |
| `QueryRow::task_priority_rank` | [V] `query::sort` consumes it for priority sort. | `src/query/results.rs:108-121`; `src/query/sort.rs:84-92` |
| `WorkspaceIndex::entry_at` | [V] The query row and query service are its two consumers outside its module. | `src/index/entry.rs:112-128`; `src/query/results.rs:64-66`; `src/query/service.rs:312-323` |
| `IndexStore` and its `pub(crate)` readers | [V] Only the index and query modules use this internal type in production. | `src/index/mod.rs:45`; `src/query/service.rs:26-27`; `src/index/refresh.rs:340-350` |

[V] These are use counts, not recommendations to change visibility. The public
`QueryBuilder`, `QueryService::run`, and query result types also serve external
tests and benchmarks. See `src/query/mod.rs:89-103`,
`tests/integration/query.rs:667-693`, and
`src/template/engine/query.rs:195-202`.
