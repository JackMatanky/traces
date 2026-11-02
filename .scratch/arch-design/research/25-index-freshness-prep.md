# Research for [Does the landed index freshness model stand?](../issues/25-index-freshness-and-invalidation.md)

This is preparation for the open grilling ticket. It does not record a decision.
[V] marks checked evidence. [H] marks a limitation or inference. [P] marks an
option.

## Decision boundaries

- [V] [What counts as the externally observable surface the redesign must
  preserve?](../issues/16-grilling-compatibility-envelope.md) freezes index
  results against a fresh parse. The cache format and refresh method can change.
  See lines 65-85.
- [V] [Who owns config-derived types, trust/tracking state, and the config
  lifecycle?](../issues/22-config-ownership-and-lifecycle.md) makes
  `IndexPolicy::fingerprint()` the sole policy fingerprint source. It excludes
  settings that the index does not read. See lines 92-115 and 150-159.
- [V] The read model fixes facets, writer checks, read transactions and pinned
  generations. See [What is the index read model, and is there one query
  execution path or two?](../issues/24-index-read-model-and-query-execution.md).
  It chooses `Durability::None` for the first redb adapter. See lines 405-450.
- [V] The note model needs a complete source-snapshot check for a closed-file
  edit. See [What is the note document model: spans, facts, items, multi-value
  metadata?](../issues/23-note-document-model.md). A byte-layout or fact change
  needs a cache identity change.
- [V] IWE has no persisted index. See [How does IWE structure one core shared by
  a CLI, an LSP, an MCP server and a
  daemon?](../issues/12-research-iwe-core-and-hosts-architecture.md). Its
  cold-load result cannot settle this cache policy. See lines 67-88.

## A. Policy changes

[V] `RefreshPlan::collect` compares a persisted fingerprint and a scan. A drift
causes a full rebuild. See `src/index/refresh.rs:337-358` and
`src/index/service.rs:208-230`.

[P] Keep a full rebuild when an effective ignore setting changes. A prior scan
lacks paths that the old policy ignored. Test any narrower repair against every
stored facet and key before adopting it.

[P] Use `IndexPolicy::fingerprint()` for policy input only. Keep the engine and
application format as separate cache-identity components. This does not create a
second config digest. Schema binding and the host zone apply after index facts
are read.

[H] An unwritable cache can make a cold read repeat a full repair. A host can
keep only a generation whose policy and source inputs remain valid. A cold read
must still parse fresh if it cannot use that generation. Do not make a failed
repair a persistent freshness signal.

## B. Store and parser identity

[V] The landed `META_VERSION` is a manual byte. On an incompatible version,
`IndexStore::open` removes the old database before it builds a replacement. See
`src/index/store.rs:41,166-189,927-979`.

[V] SQLite separates file-header versions from the application's user version.
redb also has its own file format. Neither library version identifies Traces'
parser facts. See [SQLite database
header](https://www.sqlite.org/fileformat.html#the_database_header) and [redb
design](https://github.com/cberner/redb/blob/master/docs/design.md).

[P] Compare one explicit application version with a separate parser-facts
version. A single version is smaller if its bump rule also covers parser
meaning. A package hash rebuilds after unrelated code changes and can miss
inputs outside its hash.

[P] Check the engine, application format and policy before any cached row can
reach a caller. Classify a missing record, an incompatible record, corruption,
an I/O error and a held lock. The read path parses fresh on a cache miss.
`traces index` reports a failure instead of serving old rows.

[V] `IndexerService::scan` excludes `.traces/index.redb` by exact path, not the
whole `.traces` directory. See `src/index/service.rs:421-433,810-822`. [H] An
engine-specific cache filename would become a scanned file. Exclude each cache
file if the engine gets a new filename. Another option keeps one filename but
handles unreadable old-engine bytes before adapter open.

## C. Source changes and recovery

[V] `FileDelta::compute` compares `FileMeta` values, not content. The existing
e2e case expects stale content after a same-size edit with restored mtime. See
`src/index/delta.rs:21-65` and `tests/e2e/index.rs:129-180`. That expectation
conflicts with the frozen fresh-parse result rule.

[V] [Git's racy-index
documentation](https://git-scm.com/docs/racy-git/2.43.0.html#_racy_git) hashes
equal-stat files when their mtime reaches the index time. It saves a racy marker
across index updates. Its reasoning assumes the edited file cannot get an
earlier mtime. A restored or backdated mtime breaks that assumption.

[H] [How do incremental systems fingerprint inputs and decide
staleness?](../issues/05-research-freshness-fingerprint-precedent.md) recommends
the Git guard. Its claim about closing the same-timestamp gap does not cover the
restored-mtime case. A stat shortcut or a timestamp-only guard cannot meet
unrestricted fresh-parse parity.

[P] For strict parity, compare the bytes of Markdown files that supply parsed
facts. Record a digest of the same bytes the parser consumed. Compare path
membership and query-relevant metadata for every scanned regular file. The
scanner includes attachments, but the parser reads content only for
Markdown-classified files. See `src/index/service.rs:328-355,421-458,609-638`.

[P] Check the complete input snapshot again before a write commit and on reopen.
A recovered `Durability::None` database can lack the last write. A matching
policy fingerprint alone cannot establish source freshness. See [redb durability
documentation](https://docs.rs/redb/4.3.0/redb/enum.Durability.html) and
`src/index/store.rs:357`.

[H] A second writer can change a file after the final check. A redb transaction
cannot commit a filesystem snapshot. Define the request's snapshot boundary
before promising parity during concurrent external writes. A stronger guarantee
needs writer coordination or a filesystem snapshot.

## D. Generation and process races

[V] The read-model decision requires a writer check of the plan's base
incarnation, revision and fingerprint. See [What is the index read model, and is
there one query execution path or
two?](../issues/24-index-read-model-and-query-execution.md). One read
transaction must cover candidate selection, facets and validation. See lines
421-450.

[P] On a base mismatch, discard the plan and scan again. Do not relabel another
policy's rows. Check source inputs before publishing the new generation. A read
request must use one validated generation.

[V] The macOS two-process probe got `DatabaseAlreadyOpen` for a second process.
The real CLI exited with code 1. Experimental redb multiprocess mode allowed a
second reader but refused `Durability::None`. See [the measured
probe](24-prep/g-probe-results.md#a-second-process-and-the-store), lines
106-144. Local rust-docs for redb 4.3.0 documents the same concurrency limits in
`ConcurrencyMode` and `WriteTransaction::set_durability`.

[H] There is no measured Linux or Windows two-process result. The existing CI
runs e2e on both systems but does not test this lock sequence. See
`.github/workflows/ci.yml:120-141`. Do not infer measured behavior from macOS or
the API documentation.

[H] redb documents that a failed commit can have an uncertain outcome. Check the
revision after a failure before applying the same delta again. See local
rust-docs for redb 4.3.0, `WriteTransaction::commit`.

## Source comparisons from digests

[V] rumdl keys cached warnings by file content, config, rules and a dependency
fingerprint. It writes a temporary file before replacing the cache entry. See
`docs/digests/lsp_rvben-rumdl-src-digest.txt:320-357`. [H] Its cache key shows
input classes, not Traces' complete input set.

[V] IWE tests `write_file_if_changed` with equal bytes and unchanged mtime. Its
test also checks that changed bytes produce a write. See
`docs/digests/iwe-org-iwe-src-digest.txt:2190-2215`. This write rule does not
detect a change by another process that restores an old mtime.

## E. Cost and query shape

[V] The older 5,000-note path-set probe spent 18.48 ms decoding notes and 0.52
ms resolving links. See [research 11](11-performance-evidence.md), lines 74-111.
The inverted stem index proposed in earlier research targets the smaller part.
It does not fix the measured decoding cost.

[V] The existing 50,000-note facet prototype spent 145 ms scanning whole records
and 54 to 55 ms scanning list facets. Its task-only scan took 12 ms. Writes took
300-330 ms for whole records and 520-555 ms for the first facet split. See [the
facet probe](24-prep/g-probe-results.md), lines 259-275. These are synthetic
data and a throwaway layout.

[V] That probe also timed staged queries, but it did not build result rows or
call `QueryService`. See [the staged pipelines](24-prep/g-probe-results.md),
lines 302-326. It cannot establish assembled-result time or host memory after
the decided facet cutover.

The next two tables use current release-mode Criterion quick runs on macOS.
`mise` ran both benchmarks. The hash fixture drops file pages before each
sample. The refresh fixture uses warm pages and the current stat-only delta. See
`benches/hash.rs:50-95` and `benches/index_refresh.rs:162-198`.

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

[H] Neither run measures end-to-end content checks on a vault. The existing
refresh benchmark has no 50,000-note case. Do not extrapolate a vault result by
multiplying a single-file time. Measure assembled queries and memory before
choosing a host note cache or a finer invalidation scheme.

## F. Visibility and remaining proof

[P] A successful cache repair need not print a notice. One failed read that
falls back to fresh parsing needs the single stderr notice from the read-model
decision. `traces index` must report its failure. The diagnostics decision owns
the notice text. Keep any check watermark and previous violation keys outside
the cache identity.

These experiments remain before an optimization decision:

1. Measure Linux and Windows two-process opens under the chosen redb mode.
2. Measure full source checks on all seven required vault sizes and on realistic
   file sizes.
3. Measure assembled rows, read bytes and memory with the decided facet layout.
4. Define source-snapshot admission when unrelated processes write files.
5. Check the cache version whenever parser facts, keys or required facets
   change.

The open ticket must settle the correctness policy in the grilling. Missing
measurements do not justify a claimed speed gain. They also do not turn an
unavailable platform experiment into a verified fact.

## G. Stress test, round two

Five independent reviewers attacked the provisional answers. The parent checked
each claim against the code. [V] marks a checked fact. [H] marks an inference.
[P] marks a proposal. No Rust code ran and no benchmark ran.

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
- [P] A racy row becomes clean only when its mtime is older than the new
  `scan_start - 2 s`.
- [H] A file with an mtime far in the future stays racy. It costs one hash per
  refresh.
- [H] A network filesystem with a skewed server clock can defeat the margin.
  Measure this case before you claim support.

### G9. Store identity, revised

[P] Superseded by section H. The maintainer confirmed the replacement in
section H and ticket 25. The confirmed design lists a fixed set of extractor
crates, not every dependency. It adds a corpus fingerprint and an ordered epoch.

- [P] Store four parts in `meta`: an engine tag, an ordered `FORMAT_VERSION`
  integer, a `ParserIdentity` hash and the policy fingerprint.
- [P] `ParserIdentity` is a BLAKE3 hash of the sorted `name@version` pairs of
  normal dependencies in `Cargo.lock`. A `build.rs` step computes it.
- [P] A dependency bump then rebuilds the store. This closes the escape where
  `pulldown-cmark` changes its output and no test fails.
- [P] A higher `FORMAT_VERSION` than the binary knows is a cache miss. Do not
  wipe it. A fresh parse serves the read.
- [P] A lower integer, or an equal integer with a different `ParserIdentity`,
  replaces the store at the next persist.
- [P] Keep a golden-corpus test. Build one note with every field filled. Pin its
  serialized bytes. A field reorder then fails the test.
- [H] The dependency hash over-invalidates. Each unrelated dependency bump costs
  one rebuild. Measure the rebuild rate over one month of bumps.

### G10. Host validation cost

[V] The no-op refresh takes 12.0 ms at 50 notes, 24.0 ms at 5,000 and 65.7 ms at
20,000. See section E. [V] A full stat check before each host query pays that
cost on every query. [P] Replace "check before each host query" with bounded
staleness. A host validates after a watcher event, or when the last check is
older than a set time. [P] Each query result carries the generation it read. The
buffer ticket and the host owner set the time bound.

### G11. Lock-hold rule for hosts

[V] A second process receives `DatabaseAlreadyOpen` and the CLI exits 1 today
(`24-prep/g-probe-results.md:106-123`). [V] A cold store query with a tag or
path source takes 117 to 127 ms at 50,000 notes. A full build (parse and
persist) takes 1,801 ms (`g-probe-results.md:11-14,67-68`). [H] A host that
holds the store open between queries makes every CLI call 14 times slower at
50,000 notes. [P] A host opens the store only to refresh and persist. It serves
queries from its pinned in-memory generation. [P] The workspace-host owner must
meet this rule. Record the exit code change from 1 to 0 in the change table.

### G12. Policy alternation

[V] A fingerprint mismatch rebuilds the whole store. Two policies at one root
rebuild the store at each switch.

[H] Realistic triggers are `TRACES_IGNORED_DIRS` set in one shell, and a debug
binary next to an installed binary.

[H] One switch costs a full build: parse and persist. It took 797 ms at 20,000
notes and 1,801 ms at 50,000 (`g-probe-results.md:66-68`). The parse-only part
is unmeasured.

| Option | Benefit | Cost |
| --- | --- | --- |
| A. One store, last writer rebuilds (landed) | No new code | Repeated rebuilds on alternation |
| B. One file per identity, with cleanup | No thrash | Disk growth and a cleanup rule |
| C. Only the config policy persists | No thrash | Overrides parse fresh on every call |

[P] Keep A until a measured alternation cost justifies B or C.

### G13. Per-axis repair

[H] Per-axis repair adds a second invalidation state machine. No measurement
shows a need for it.

[P] Rebuild fully on any fingerprint change. Allow per-axis repair only after a
measured rebuild cost exceeds a stated limit.

### G14. Windows change time

[H] Stable Rust `std` exposes no change time on Windows. A same-size edit with a
restored mtime then passes the stat check.

[P] Option a: accept this gap and add `traces index --verify`, which hashes
every Markdown file. Option b: read `ChangeTime` through the Win32 API. Option
c: hash every Markdown file on each refresh.

[P] Choose option a. The maintainer chose it.

#### Windows API findings

- [V] Rust issue #121478 (`windows_change_time`) is still open. The method needs
  a nightly feature, so it breaks `rust-version = "1.96"` (`Cargo.toml:15`).
- [V] On a desktop MSVC target the std method returns `None`. `WIN32_FIND_DATAW`
  has no `ChangeTime`, so `DirEntry::metadata()` cannot supply it.
- [V] A caller must open a handle for each file to read `ChangeTime`. This needs
  about 100 lines of unsafe FFI. `Cargo.toml:256` sets `unsafe_code = "deny"`.
- [H] A handle open for each file slows a walk by 5 to 20 times. A research
  agent estimated this from Git for Windows reports. No one measured it.
- [V] A process without elevated rights can set `ChangeTime` with
  `SetFileInformationByHandle`. It is not as strong as Unix `ctime`.
- [V] FAT and exFAT keep no `ChangeTime`. An SMB server that is not Windows
  often returns `LastWriteTime`. A handle open can hydrate a cloud placeholder.
- [V] restic sets `ChangeTime = ModTime` on Windows
  (`internal/fs/stat_windows.go:34-36`).

#### How other tools detect change

[H] A research agent read these tools. The parent re-read only restic.

| Tool | Signals | Hashes content when | Racy handling |
| --- | --- | --- | --- |
| Git | size, mtime, ctime, inode | The stat differs, or the entry is racily clean | It zeroes the stored size for a racy entry |
| Mercurial | size, mtime, mode | The stat differs or the mtime is ambiguous | `MTIME_SECOND_AMBIGUOUS` flag |
| restic | Unix: size, mtime, ctime, inode. Windows: size, mtime | The stat differs, or with `--force` | None |
| Borg | ctime, size, inode | The files-cache misses | None |
| rsync | size, mtime | With `-c` | None |
| Syncthing | size, mtime, permissions | The stat differs | `modTimeWindowS` |
| Bazel | ctime, mtime, size, inode | The key changes | Checks before a remote upload |
| ccache | mtime, ctime, size | The stat differs | Refuses direct mode for a file changed this second |
| Cargo, Ruff | mtime | Never by default | None |
| rumdl | Content hash | Every run | Not needed |

[V] Unix tools that use `ctime` catch a restored mtime. Windows tools accept the
gap.

#### Hashing cost

[V] Measured on an Apple M4 Pro (12 cores, APFS, warm cache, release build). The
fixture holds notes sampled from 18,226 local Markdown files. The median is 5.7
KB. The mean is 13.2 KB.

| Notes | Stat walk | Read and hash, 12 threads | Read only, 12 threads | Stat walk plus 1% hash |
| ---: | ---: | ---: | ---: | ---: |
| 1,000 | 2.85 ms | 7.5 ms | 8.0 ms | 3.1 ms |
| 5,000 | 11.9 ms | 37 ms | 38 ms | 12.2 ms |
| 20,000 | 45 ms | 151 ms | 151 ms | 47 ms |
| 50,000 | 113 ms | 395 ms | 378 ms | 118 ms |

- [V] A single thread takes 1,833 ms to hash 50,000 notes. The 12-thread run
  uses 4.3 s of CPU.
- [V] The `open`, `read` and `close` calls dominate. Hashing adds 18 ms at
  50,000 notes.
- [V] `mmap` was no faster. `F_NOCACHE` matched the warm run, but this is not a
  verified cold cache.
- [H] Windows cost is unmeasured.

### G15. Journal scope

[H] Every refresh derives rows from sources. A lost commit loses only the redo
work.

[P] Measure the journal against that redo cost in [the journal measurement
ticket](../issues/26-measure-index-recovery-journal.md).

### G16. Claims rejected or contained

| Claim | Verdict | Reason |
| --- | --- | --- |
| Validate only candidate notes (lazy hint) | Rejected | A newly tagged note is not in the stored candidate set. |
| Directory digests skip clean subtrees | Rejected | An in-place edit leaves the directory mtime unchanged. |
| No stored facts, parse on every query | Rejected | A full build took 797 ms at 20,000 notes (`g-probe-results.md:66-67`). It includes persistence. A parse-only run is unmeasured, so the real fallback latency is lower. |
| Watcher journal | Rejected | It adds a daemon and drops events under load. |
| A second process holds the store during replacement | Contradicted | `ExclusiveWriter` refuses the second open. Only the gap in G7 remains. |
| `Durability::None` rollback serves stale rows | Contained | Facets and `meta` commit in one transaction. Older rows carry older stats and reparse. |
| NFC and NFD names churn the delta | Not settled | A changed name is a rename, but ticket 31 has no normalization rule (a search found none). Round three added an explicit handoff. |
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

## H. Stress test, round three: store identity

Five reviewers attacked the "Store identity" decision: a fingerprint over an
embedded corpus plus a rebuild on any major or minor version change. The parent
checked the load-bearing claims against code. Section H supersedes the
manual-`FORMAT_VERSION` and lock-pin design in G9. The decision text is in
[ticket 25](../issues/25-index-freshness-and-invalidation.md), "Store identity".

### H1. Findings that changed the design

| Finding | Verdict | Evidence | Effect |
| --- | --- | --- | --- |
| The fingerprint covers only extractor output. `FileMeta`, links, tag maps, key codecs and table layout can change unseen. | Accepted | Eight tables in `src/index/tables.rs` today. Ticket 24 (resolved) replaces them with 11 facets (`24:307-310`). The corpus hash touches `Note` only. | Hash one probe record per facet except `meta`, and one range key per scalar key encoding. A test fails if a table other than `META` has none. |
| A hash has no order. The store cannot tell a newer store from an older one. | Accepted | A hash only supports equality. | Add an ordered `u32` epoch. The `index:bless` task sets it. |
| "Major or minor" fails for `0.x`, and no release process exists. | Accepted | `version = "0.1.0"`. Two tags exist and both are `merge-safety/*`. | Drop the version floor. List the extractor crates in the fingerprint instead. |
| Unix and Windows serialize paths differently. | Verified | `src/path.rs:349-391`. | Platform family joins the identity. Ticket 31 may remove it. |
| An unzoned datetime depends on the host zone. | Verified, already decided | `src/temporal/datetime.rs:133-136`. Ticket 19 makes naive datetimes civil. | The corpus uses no unzoned datetime until that lands. |
| `logos` and `noyalib` are not owned by one module. | Verified | `logos`: `src/lexer.rs`, `src/note/parser/lexer.rs`, `src/query/grammar/`, `src/temporal/`. `noyalib`: `src/yaml.rs`, `src/field.rs`, `src/note/metadata.rs`, `src/template/engine/yaml.rs`. | Confine only `pulldown-cmark`, `redb` and `postcard`. Cover the rest by the crate list. |
| The repository already enforces boundaries with `git grep` in `mise` tasks. | Verified | `mise.toml:148-190`, `check:layers`. | Add `check:boundaries` in the same style. |
| Computing the fingerprint at startup adds latency to commands that skip the index. | Accepted | `--version`, `--help` and completions never open the store. | Compute once, lazily, on the first store open. |
| After a rollback, an older binary meets a newer store. It needs a way out. | Accepted | A cache miss forever means a fresh parse per query. `traces index` already rebuilds in full (`src/cli/index.rs:38-48`). | Use bare `traces index`. The notice names it. No new flag. The first draft proposed `--rebuild`; the check against ticket 24 and the code removed it. |

### H2. Claims rejected or contained

| Claim | Verdict | Reason |
| --- | --- | --- |
| "Zero version tags exist." | Contradicted | Two tags exist. Neither marks a release. The conclusion holds. |
| Add store-only record types to decouple the wire format. | Rejected | Tickets 18 and 24 keep derived serde for the cache. The probe rows detect a shape change. |
| A wire fixture test at `src/note/lists.rs:1013` warns about this. | Unverified | A search for its comment text found nothing. Dropped from the evidence. |
| Hash `src/note/**` in `build.rs`. | Rejected | A comment or `cargo fmt` change would force a rebuild. |
| Per-row extractor stamp with lazy reparse. | Rejected | The inverted tables (`PATHS_BY_TAG`) would miss unparsed notes. A read would need a write lock. |
| Layered facts with separate identities. | Rejected | The split saves about 117 ms at 50,000 notes. A full build takes 1,801 ms. |
| Content-addressed rows. | Rejected | A scan hashes every file: 395 ms against 117 ms. It does not solve parser change. |
| Skip the persisted store for small vaults. | Rejected | It creates a second query path. |
| Use the epoch in the file name so versions never meet. | Rejected | Orphan files need a cleanup rule. The engine tag is a different case: it changes rarely, and the grilling round of ticket 25 already chose one cache file per engine (`25:213`). |
| Rebuild on any mismatch with no order. | Rejected | An old editor host would alternate rebuilds with a new CLI after an upgrade. |
| Store a `build.rs` git hash for developer builds. | Rejected | Every commit would force a rebuild. |

### H3. Unverified

- The runtime cost of the corpus pass. Reviewers estimated 1 to 3 ms. No one
  measured it.
- Whether `serde-reflection` can trace the tagged serde types.
- Whether a `redb` major upgrade reports `Corrupted` or a magic mismatch. If it
  does, `create_db` deletes the file. The engine tag in the file name avoids
  this. Reads must also stop deleting.
- What `cargo install` without `--locked` puts in `Cargo.lock` at build time.
- Windows behavior of every rule above.

### H4. Check against earlier decisions

Three reviewers compared section H and the "Store identity" text with tickets
16 to 38, ADRs and glossaries. The parent verified each row below.

| Earlier decision | What the first draft said | Resolution |
| --- | --- | --- |
| `24:299-300`: the fingerprint names the engine and the format. Ticket 25 owns the rule. | "Ticket 24 owns the engine tag." | Wrong. Ticket 25 owns the engine tag. It lives in `META` and in the file name. |
| `24:431-438`: each store has an incarnation and a revision. | The `META` list omitted them. | Add a sentence. They stay in `META`. They are not identity parts. |
| `24:307-310`: 11 facet tables, including `meta`. | "One probe row for every table in `TABLES`." | Exempt `META`: it holds the fingerprint. Probe facets and scalar key encodings. |
| `24:374-384`: an unusable cache is a read miss with one stderr notice. `traces index` fails closed. | A higher epoch was a cache miss with an unspecified exit code. | A higher epoch follows that rule. A read exits 0. |
| `24:309`: the `meta` table keeps the format version. | A new "epoch" name. | The epoch is that format version. |
| `src/cli/index.rs:38-48`: `traces index` always rebuilds. | A new `--rebuild` flag. | Remove the flag. |
| `25:213`: one cache file per engine. | H2 said ticket 24 chose it. | Ticket 25 chose it. The engine tag now names the file. |
| `25:229`: keep successful automatic heals quiet. | Ticket 33 would write notice texts for a platform rebuild. | Rebuilds stay quiet. Only a higher epoch gets a notice. |
| `22:99`, `24:170`: `IndexPolicy::fingerprint()` is the only source of "the fingerprint". `src/index/GLOSSARY.md:52-60`: "Index Fingerprint" is the config digest. | A second "Fingerprint" in `META`. | Call the new hash the extractor fingerprint. The glossary entry is itself stale: it contradicts `25:28` and describes digests that ticket 22 deletes. Rename it "Policy Fingerprint". |
| ADR-0009 (`proposed`) and `src/index/GLOSSARY.md:34-50`: `Fresh`, `Stale`, `Drifted`, `ConfigDrift`. | The first draft reused the state and added a cause. | ADR-0009 overstates the code: a format mismatch wipes at open and never reaches `Drifted`. The name "ConfigDrift" is wrong for engine or epoch causes. Keep one rebuild state. Rename it and its cause type. A higher epoch is a read miss, not a refresh state. |
| `23:367,385`: a stored `Note` change bumps `META_VERSION`. | Stale text in ticket 25. | The epoch meets the rule. The text is updated. |
| `29:14`, `34:20` (open): they may prune `speedate` or `noyalib`. | A fixed crate list that fails the build. | Keep the failing build. Add a handoff: update the list when pruning. |
| `34` (open): a workspace split. | `build.rs` reads `Cargo.lock`. `check:boundaries` greps paths. | Hand off to ticket 34. The lock path moves and the task becomes crate boundaries. |
| `16:67-78`: every observable CLI change goes in the change table. | `--verify` was missing from the rows. | Add it. |
| `22:110`: no time zone in the fingerprint. | Not stated. | Holds. The corpus has no unzoned datetime. |
| `18`, `24`: derived serde stays for the cache. | No store-only record types. | Holds. |
| `19`, `21`: naive datetimes are civil. The cache holds no zone. | Corpus rule on unzoned datetimes. | Holds. It applies until ticket 19 lands in code. |

One reviewer claim was wrong: `pulldown-cmark` types do not leak across
`src/note/`. A search of `src/` found `pulldown_cmark` only in
`src/note/parser.rs`.

### H5. Audit of all tickets

Four reviewers audited tickets 01 to 38, the ADRs and the glossaries. They
treated every ticket as fallible, ticket 25 included. The parent verified the
findings that could change ticket 25. Reviewer findings that contradict a
maintainer-confirmed ticket without new evidence are logged, not adopted.

| Finding | Verdict | Effect |
| --- | --- | --- |
| The 1,801 ms figure is a full build, not a parse-only fallback. | Verified. `g-probe-results.md:66-68` says "build from nothing". Ticket 24 said so at `24:451-453`. | Rebuild cost stays valid. Fallback latency is unmeasured. Four brief rows are relabeled. |
| Ticket 16 freezes "index equals a fresh parse" (`16:74`). A stat check cannot guarantee it. The landed canary test pins stale output. | Verified. | Ticket 16 overstates the guarantee. Add a ticket 16 correction to ticket 25. |
| The committed fingerprint differs between Unix and Windows because path bytes differ. | Partly wrong. postcard writes a `u16` below 128 as one byte, so an ASCII path gives the same bytes on both. [INFERENCE: not run on Windows] | Require an ASCII path in the corpus. Run the identity test on Windows CI. |
| `FileMeta.day` depends on the host zone. | Verified. `src/file.rs:88-98`, `24:214-217`. | The synthetic file probe sets `day` by hand. Until tickets 21 and 24 remove `day`, a zone change leaves stale `day` values outside the identity. Record it. |
| The corpus rule "no unzoned datetime" conflicts with "every enum variant". | Wording only. | Say "every variant that exists". Add the civil variant when ticket 19 lands. That bumps the epoch. |
| A future-dated mtime stays racy forever. | Valid, minor. | Correct but costs a re-read on each refresh. Record it. |
| `traces index` from an older binary silently replaces a newer store. | Valid, minor. | Keep the behavior. Add one stderr notice. |
| A read-only vault falls back to a parse because `create` needs write access. | Plausible. [INFERENCE] | Open question for the implementation: try a read-only open. |
| Ticket 31 has no Unicode normalization rule. | Verified. A search found nothing. | The handoff in G16 pointed at nothing. Add an explicit handoff. |
| Civil datetimes must deserialize without a zone lookup. | Valid forward constraint. | Add a handoff to tickets 19 and 29. |
| The 11-facet split costs 85% more write time and doubles the file size (`AuditIndexCore`). | Not verified. I did not re-derive the figures. | Log for ticket 24. It also raises the rebuild cost that ticket 25 relies on. |
| Switch the engine to LMDB or SQLite. | Not adopted. | Ticket 24 compared engines and keeps the choice reversible (`24:304`). |
| Corruption should fail closed. | Not adopted. The reviewer misread. | Ticket 24 prints a stderr notice. It does not fall back silently. |
| Cancel ticket 26. Close ticket 38. | Judgment call. | Raise with the maintainer. Do not decide here. |
| The host cannot both hold no handle and serve a pinned generation. | Mostly wrong. | Ticket 24 serves from memory. The real cost is memory: 467 MB at 50,000 notes. Log for ticket 32. |
