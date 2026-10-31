# Research for [Does the landed index freshness model stand?](../issues/25-index-freshness-and-invalidation.md)

This is preparation for the open grilling ticket. It does not record a decision.
[V] marks checked evidence. [H] marks a limitation or inference. [P] marks an option.

## Decision boundaries

- [V] [What counts as the externally observable surface the redesign must preserve?](../issues/16-grilling-compatibility-envelope.md) freezes index results against a fresh parse. The cache format and refresh method can change. See lines 65-85.
- [V] [Who owns config-derived types, trust/tracking state, and the config lifecycle?](../issues/22-config-ownership-and-lifecycle.md) makes `IndexPolicy::fingerprint()` the sole policy fingerprint source. It excludes settings that the index does not read. See lines 92-115 and 150-159.
- [V] The read model fixes facets, writer checks, read transactions and pinned generations. See [What is the index read model, and is there one query execution path or two?](../issues/24-index-read-model-and-query-execution.md). It chooses `Durability::None` for the first redb adapter. See lines 405-450.
- [V] The note model needs a complete source-snapshot check for a closed-file edit. See [What is the note document model: spans, facts, items, multi-value metadata?](../issues/23-note-document-model.md). A byte-layout or fact change needs a cache identity change.
- [V] IWE has no persisted index. See [How does IWE structure one core shared by a CLI, an LSP, an MCP server and a daemon?](../issues/12-research-iwe-core-and-hosts-architecture.md). Its cold-load result cannot settle this cache policy. See lines 67-88.

## A. Policy changes

[V] `RefreshPlan::collect` compares a persisted fingerprint and a scan. A drift causes a full rebuild. See `src/index/refresh.rs:337-358` and `src/index/service.rs:208-230`.

[P] Keep a full rebuild when an effective ignore setting changes. A prior scan lacks paths that the old policy ignored. Test any narrower repair against every stored facet and key before adopting it.

[P] Use `IndexPolicy::fingerprint()` for policy input only. Keep the engine and application format as separate cache-identity components. This does not create a second config digest. Schema binding and the host zone apply after index facts are read.

[H] An unwritable cache can make a cold read repeat a full repair. A host can keep only a generation whose policy and source inputs remain valid. A cold read must still parse fresh if it cannot use that generation. Do not make a failed repair a persistent freshness signal.

## B. Store and parser identity

[V] The landed `META_VERSION` is a manual byte. On an incompatible version, `IndexStore::open` removes the old database before it builds a replacement. See `src/index/store.rs:41,166-189,927-979`.

[V] SQLite separates file-header versions from the application's user version. redb also has its own file format. Neither library version identifies Traces' parser facts. See [SQLite database header](https://www.sqlite.org/fileformat.html#the_database_header) and [redb design](https://github.com/cberner/redb/blob/master/docs/design.md).

[P] Compare one explicit application version with a separate parser-facts version. A single version is smaller if its bump rule also covers parser meaning. A package hash rebuilds after unrelated code changes and can miss inputs outside its hash.

[P] Check the engine, application format and policy before any cached row can reach a caller. Classify a missing record, an incompatible record, corruption, an I/O error and a held lock. The read path parses fresh on a cache miss. `traces index` reports a failure instead of serving old rows.

[V] `IndexerService::scan` excludes `.traces/index.redb` by exact path, not the whole `.traces` directory. See `src/index/service.rs:421-433,810-822`. [H] An engine-specific cache filename would become a scanned file. Exclude each cache file if the engine gets a new filename. Another option keeps one filename but handles unreadable old-engine bytes before adapter open.

## C. Source changes and recovery

[V] `FileDelta::compute` compares `FileMeta` values, not content. The existing e2e case expects stale content after a same-size edit with restored mtime. See `src/index/delta.rs:21-65` and `tests/e2e/index.rs:129-180`. That expectation conflicts with the frozen fresh-parse result rule.

[V] [Git's racy-index documentation](https://git-scm.com/docs/racy-git/2.43.0.html#_racy_git) hashes equal-stat files when their mtime reaches the index time. It saves a racy marker across index updates. Its reasoning assumes the edited file cannot get an earlier mtime. A restored or backdated mtime breaks that assumption.

[H] [How do incremental systems fingerprint inputs and decide staleness?](../issues/05-research-freshness-fingerprint-precedent.md) recommends the Git guard. Its claim about closing the same-timestamp gap does not cover the restored-mtime case. A stat shortcut or a timestamp-only guard cannot meet unrestricted fresh-parse parity.

[P] For strict parity, compare the bytes of Markdown files that supply parsed facts. Record a digest of the same bytes the parser consumed. Compare path membership and query-relevant metadata for every scanned regular file. The scanner includes attachments, but the parser reads content only for Markdown-classified files. See `src/index/service.rs:328-355,421-458,609-638`.

[P] Check the complete input snapshot again before a write commit and on reopen. A recovered `Durability::None` database can lack the last write. A matching policy fingerprint alone cannot establish source freshness. See [redb durability documentation](https://docs.rs/redb/4.3.0/redb/enum.Durability.html) and `src/index/store.rs:357`.

[H] A second writer can change a file after the final check. A redb transaction cannot commit a filesystem snapshot. Define the request's snapshot boundary before promising parity during concurrent external writes. A stronger guarantee needs writer coordination or a filesystem snapshot.

## D. Generation and process races

[V] The read-model decision requires a writer check of the plan's base incarnation, revision and fingerprint. See [What is the index read model, and is there one query execution path or two?](../issues/24-index-read-model-and-query-execution.md). One read transaction must cover candidate selection, facets and validation. See lines 421-450.

[P] On a base mismatch, discard the plan and scan again. Do not relabel another policy's rows. Check source inputs before publishing the new generation. A read request must use one validated generation.

[V] The macOS two-process probe got `DatabaseAlreadyOpen` for a second process. The real CLI exited with code 1. Experimental redb multiprocess mode allowed a second reader but refused `Durability::None`. See [the measured probe](24-prep/g-probe-results.md#a-second-process-and-the-store), lines 106-144. Local rust-docs for redb 4.3.0 documents the same concurrency limits in `ConcurrencyMode` and `WriteTransaction::set_durability`.

[H] There is no measured Linux or Windows two-process result. The existing CI runs e2e on both systems but does not test this lock sequence. See `.github/workflows/ci.yml:120-141`. Do not infer measured behavior from macOS or the API documentation.

[H] redb documents that a failed commit can have an uncertain outcome. Check the revision after a failure before applying the same delta again. See local rust-docs for redb 4.3.0, `WriteTransaction::commit`.

## Source comparisons from digests

[V] rumdl keys cached warnings by file content, config, rules and a dependency fingerprint. It writes a temporary file before replacing the cache entry. See `docs/digests/lsp_rvben-rumdl-src-digest.txt:320-357`. [H] Its cache key shows input classes, not Traces' complete input set.

[V] IWE tests `write_file_if_changed` with equal bytes and unchanged mtime. Its test also checks that changed bytes produce a write. See `docs/digests/iwe-org-iwe-src-digest.txt:2190-2215`. This write rule does not detect a change by another process that restores an old mtime.

## E. Cost and query shape

[V] The older 5,000-note path-set probe spent 18.48 ms decoding notes and 0.52 ms resolving links. See [research 11](11-performance-evidence.md), lines 74-111. The inverted stem index proposed in earlier research targets the smaller part. It does not fix the measured decoding cost.

[V] The existing 50,000-note facet prototype spent 145 ms scanning whole records and 54 to 55 ms scanning list facets. Its task-only scan took 12 ms. Writes took 300-330 ms for whole records and 520-555 ms for the first facet split. See [the facet probe](24-prep/g-probe-results.md), lines 259-275. These are synthetic data and a throwaway layout.

[V] That probe also timed staged queries, but it did not build result rows or call `QueryService`. See [the staged pipelines](24-prep/g-probe-results.md), lines 302-326. It cannot establish assembled-result time or host memory after the decided facet cutover.

The next two tables use current release-mode Criterion quick runs on macOS. `mise` ran both benchmarks. The hash fixture drops file pages before each sample. The refresh fixture uses warm pages and the current stat-only delta. See `benches/hash.rs:50-95` and `benches/index_refresh.rs:162-198`.

| Source file | `Blake3Digest::from_path` time |
| --- | ---: |
| 1 KiB | 15.841 microseconds |
| 1 MiB | 757.54 microseconds |

| Note count | Existing no-op `WorkspaceIndex::refresh` time |
| ---: | ---: |
| 50 | 12.000 ms |
| 200 | 11.997 ms |
| 1,000 | 12.852 ms |
| 5,000 | 24.021 ms |
| 10,000 | 39.214 ms |
| 20,000 | 65.745 ms |

[H] Neither run measures end-to-end content checks on a vault. The existing refresh benchmark has no 50,000-note case. Do not extrapolate a vault result by multiplying a single-file time. Measure assembled queries and memory before choosing a host note cache or a finer invalidation scheme.

## F. Visibility and remaining proof

[P] A successful cache repair need not print a notice. One failed read that falls back to fresh parsing needs the single stderr notice from the read-model decision. `traces index` must report its failure. The diagnostics decision owns the notice text. Keep any check watermark and previous violation keys outside the cache identity.

These experiments remain before an optimization decision:

1. Measure Linux and Windows two-process opens under the chosen redb mode.
2. Measure full source checks on all seven required vault sizes and on realistic file sizes.
3. Measure assembled rows, read bytes and memory with the decided facet layout.
4. Define source-snapshot admission when unrelated processes write files.
5. Check the cache version whenever parser facts, keys or required facets change.

The open ticket must settle the correctness policy in the grilling. Missing measurements do not justify a claimed speed gain. They also do not turn an unavailable platform experiment into a verified fact.
