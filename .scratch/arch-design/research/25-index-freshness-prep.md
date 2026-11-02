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

## G. Stress test, round two

Five independent reviewers attacked the provisional answers. The parent checked each claim against the code.
[V] marks a checked fact. [H] marks an inference. [P] marks a proposal. No Rust code ran and no benchmark ran.

### G1. Corrections accepted

| ID | Weakness | Evidence | Revised answer |
| --- | --- | --- | --- |
| C1 | One global commit time un-races rows that no one rechecked. | [H] An incremental commit moves a global time forward. | [P] Store a `racy` bit in each file row. See section G2. |
| C2 | The scan stat and the file read are two separate steps. | [V] `scan_file_metadata` stats at `src/index/service.rs:448-461`. `parse_note` reads later at lines 342-350. | [P] Read from one open handle. Stat that handle before and after the read. |
| C3 | One vanished file fails the whole refresh. | [V] `parse_notes` collects one `IndexResult` (`service.rs:328-334`). | [P] Rescan once after `NotFound`. A second failure is a classified error. |
| C4 | The engine-file scan exclusion is dead code. | [V] `scan` prunes `.traces` through `is_ignored_dir` (`service.rs:427`). | [P] Remove `path != index_db` (`service.rs:435`). Test that nothing under `.traces/` enters the scan. |
| C5 | The 128-bit `commit_nonce` adds a recovery protocol with no gain. | [H] Every refresh compares stored rows with current files. | [P] Drop the nonce. After an uncertain commit, drop the handle and the plan. |
| C6 | The in-process unwritable latch never clears. | [H] A failed open is cheap. A held store refused the open in 171 microseconds (`24-prep/g-probe-results.md:113`). | [P] Remove the latch. Try the write-open at each persist. Print a notice on a state change only. |
| C7 | Store replacement can fail between two steps. | [V] `open` drops the handle, then removes the file (`src/index/store.rs:174-183`). | [P] A replacement failure is a cache miss for reads. `traces index` reports the error. |
| C8 | A late corruption error in `META` skips the rebuild path. | [V] `rebuild_needed` classifies corruption at `begin_read` and the table probes (`store.rs:937-967`). | [P] Classify corruption from `open_table(META)` and `get` as a rebuild (`store.rs:974-983`). |
| C9 | A derived hash has no order. The "never wipe a newer store" rule needs an order. | [H] Two binaries can carry different dependency sets. | [P] Split the identity. See section G9. |

### G2. Racy check, exact rule

- [P] Sample `scan_start` from the wall clock before the directory walk.
- [P] Use a margin of 2 seconds. That margin covers FAT and exFAT timestamps.
- [P] A committed row gets `racy = true` when `mtime >= scan_start - 2 s`.
- [P] The next refresh reads and hashes each racy row.
- [P] A racy row becomes clean only when its mtime is older than the new `scan_start - 2 s`.
- [H] A file with an mtime far in the future stays racy. It costs one hash per refresh.
- [H] A network filesystem with a skewed server clock can defeat the margin.
  Measure this case before you claim support.

### G9. Store identity, revised

- [P] Store four parts in `meta`: an engine tag, an ordered `FORMAT_VERSION`
  integer, a `ParserIdentity` hash and the policy fingerprint.
- [P] `ParserIdentity` is a BLAKE3 hash of the sorted `name@version` pairs of
  normal dependencies in `Cargo.lock`. A `build.rs` step computes it.
- [P] A dependency bump then rebuilds the store. This closes the escape where
  `pulldown-cmark` changes its output and no test fails.
- [P] A higher `FORMAT_VERSION` than the binary knows is a cache miss.
  Do not wipe it. A fresh parse serves the read.
- [P] A lower integer, or an equal integer with a different `ParserIdentity`,
  replaces the store at the next persist.
- [P] Keep a golden-corpus test. Build one note with every field filled. Pin its serialized bytes. A field reorder then fails the test.
- [H] The dependency hash over-invalidates. Each unrelated dependency bump costs one rebuild. Measure the rebuild rate over one month of bumps.

### G10. Host validation cost

[V] The no-op refresh takes 12.0 ms at 50 notes, 24.0 ms at 5,000 and 65.7 ms at 20,000. See section E.
[V] A full stat check before each host query pays that cost on every query.
[P] Replace "check before each host query" with bounded staleness. A host
validates after a watcher event, or when the last check is older than a set
time.
[P] Each query result carries the generation it read. The buffer ticket and the host owner set the time bound.

### G11. Lock-hold rule for hosts

[V] A second process receives `DatabaseAlreadyOpen` and the CLI exits 1 today (`24-prep/g-probe-results.md:106-123`).
[V] A cold store query with a tag or path source takes 117 to 127 ms at
50,000 notes. A fresh parse takes 1,801 ms
(`g-probe-results.md:11-14,67-68`).
[H] A host that holds the store open between queries makes every CLI call 14 times slower at 50,000 notes.
[P] A host opens the store only to refresh and persist. It serves queries from its pinned in-memory generation.
[P] The workspace-host owner must meet this rule. Record the exit code change from 1 to 0 in the change table.

### G12. Policy alternation

[V] A fingerprint mismatch rebuilds the whole store. Two policies at one root rebuild the store at each switch.
[H] Realistic triggers are `TRACES_IGNORED_DIRS` set in one shell, and a debug binary next to an installed binary.
[H] One switch costs a fresh parse plus a facet write. The parse took 797 ms at 20,000 notes. The write took 520 to 587 ms at 50,000 notes.

| Option | Benefit | Cost |
| --- | --- | --- |
| A. One store, last writer rebuilds (landed) | No new code | Repeated rebuilds on alternation |
| B. One file per identity, with cleanup | No thrash | Disk growth and a cleanup rule |
| C. Only the config policy persists | No thrash | Overrides parse fresh on every call |

[P] Keep A until a measured alternation cost justifies B or C.

### G13. Per-axis repair

[H] Per-axis repair adds a second invalidation state machine. No measurement shows a need for it.
[P] Rebuild fully on any fingerprint change. Allow per-axis repair only after a measured rebuild cost exceeds a stated limit.

### G14. Windows change time

[H] Stable Rust `std` exposes no change time on Windows. A same-size edit with a restored mtime then passes the stat check.
[P] Option a: accept this gap and add `traces index --verify`, which hashes every Markdown file. Option b: read `ChangeTime` through the Win32 API. Option c: hash every Markdown file on each refresh.
[H] Option c costs one hash per file. The hash of 1 KiB took 15.8 microseconds and 1 MiB took 757 microseconds. No vault-wide time exists.

### G15. Journal scope

[H] Every refresh derives rows from sources. A lost commit loses only the redo work.
[P] Measure the journal against that redo cost in [the journal measurement ticket](../issues/37-measure-index-recovery-journal.md).

### G16. Claims rejected or contained

| Claim | Verdict | Reason |
| --- | --- | --- |
| Validate only candidate notes (lazy hint) | Rejected | A newly tagged note is not in the stored candidate set. |
| Directory digests skip clean subtrees | Rejected | An in-place edit leaves the directory mtime unchanged. |
| No stored facts, parse on every query | Rejected | The fresh parse took 797 ms at 20,000 notes (`g-probe-results.md:67`). |
| Watcher journal | Rejected | It adds a daemon and drops events under load. |
| A second process holds the store during replacement | Contradicted | `ExclusiveWriter` refuses the second open. Only the gap in G7 remains. |
| `Durability::None` rollback serves stale rows | Contained | Facets and `meta` commit in one transaction. Older rows carry older stats and reparse. |
| NFC and NFD names churn the delta | Rejected | A changed name is a rename. Path identity belongs to the path ticket. |
| Evicted cloud files hang the refresh | Contained | Only rows with a changed stat are read. A fresh parse reads the same files. |
| Persisted range keys store civil dates for `file.mdate` | Handoff | The read-model decision persists keys for note-level fields only. Keep timestamp keys as UTC instants. |
| The same-size edit canary asserts stale output | Accepted | Record it as a change-table row. Flip the assertion to the new text. |

### G17. Missing before resolution

| Item | Status |
| --- | --- |
| Invariant owners | [P] Stat guard in `src/index/delta.rs`. Identity in `src/index/store.rs`. Replan in `src/index/service.rs`. Host admission and unlatch rule go to the host ticket. |
| Removable code | [V] `path != index_db` (`service.rs:435`). [P] `ConfigDigests` destructuring after the config cutover. [P] The unconditional wipe in `open`. |
| Change-table rows | Same-size edit with restored mtime. Exit code 1 to 0 on a held store. Fallback notice. Newer store: wipe becomes fresh parse. `NotFound` retry. |
| Handoffs | Query ticket: candidate-set freshness signal. Path ticket: path form in keys. Host ticket: lock-hold rule and unlatch. Diagnostics ticket: reason codes for notices. Buffer ticket: generation pair. Journal ticket: redo-cost comparison. |
