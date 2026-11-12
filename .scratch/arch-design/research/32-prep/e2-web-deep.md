# Deepened web evidence for tickets 28, 29, 32

Round 2 of web research. Targets the three weak areas named in the first
pass: crash semantics, watcher rename semantics, and platform path rules.
Every entry carries a URL; claims made without a fetched page stay [H].

## redb commit and crash semantics (28, 29)

Source: https://github.com/cberner/redb/blob/master/docs/design.md [V]

1. Non-durable commits: after a crash the database rolls back to the last
   durable commit; the file stays consistent. The ticket 27 ENOSPC finding
   (rows vanish under `Durability::None`) is the designed behavior, not a
   corruption path. The adoption question for ticket 36 stays a freshness
   question, never a consistency question.
2. 1PC+C is the default durable strategy: one `fsync`, XXH3_128 checksum
   tree, primary-bit flip. Partial writes are detected and rolled back.
   2PC exists for malicious-input scenarios; an index store that only the
   tool writes does not need it.
3. MVCC: read transactions register with the database, and pages they
   reference are never freed until the reads complete. This is the
   mechanism behind the settled rule "cold query pins ONE read txn".
   A long-lived read transaction pins pages, so holding several open
   blocks reclaim and grows the file.
4. Savepoints cost about 64 kB allocator snapshot per 1 GB of values.
   The journal design of ticket 26 does not need savepoints; cite if the
   grill asks "why not roll back instead of journal".

## Watcher rename semantics (28, 32)

Sources: https://lkml.org/lkml/2014/4/7/77 (inotify rename pairing, Jan
Kara) [V]; https://docs.rs/notify-debouncer-full (DebouncedEvent
`track_id`, `event().flag` rename from/to markers) [V];
https://forum.obsidian.md/t/macos-notes-with-nfd-normalized-filenames-disappear-from-file-explorer-and-search-when-edited-regression-in-1-13-5-1-13-6-watcher-existence-check-compares-raw-nfd-event-name-against-nfc-normalized-listing/117189
[V]

1. inotify: `IN_MOVED_FROM` and `IN_MOVED_TO` are usually consecutive,
   but "usually" is the strongest guarantee; the cookie can be missed and
   the from-event can arrive without its to-event. Any rename re-keying
   rule needs a fallback: an unmatched from-event must degrade to delete +
   rescan, not to a silent drop.
2. macOS FSEvents reports coalesced directory-level events; per-file
   rename pairing is reconstructed, not observed. The debouncer's
   `push_rename_event`/`push_remove_event` machinery does this
   reconstruction, with a FileIdMap for identity.
3. Direct precedent for ticket 32's NFC rule: Obsidian 1.13.5 shipped a
   regression where the watcher compared a raw NFD event name against an
   NFC-normalized listing, so migrated notes disappeared from the file
   explorer on edit. The same class reproduces on Windows for any
   non-NFC spelling (still open at 1.14.5). A store that normalizes on
   write and never folds on read must still compare incoming watcher
   names in the same form as stored keys. Otherwise it inherits this
   bug.

## Platform path rules (32)

Sources: https://learn.microsoft.com/en-us/dotnet/standard/io/file-path-formats
[V]; https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file
[V]; https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlea
[V]; https://help.obsidian.md/Symbolic+links+and+junctions [V]

1. Windows normalization: trailing dots and spaces are stripped from each
   component; the `\\?\` device-path prefix skips normalization. Two
   stored spellings can name one file, so an admission check must reject
   or canonicalize reserved-trailing components, not only the 8.3 names.
2. `GetFinalPathNameByHandle` resolves the final path from an open
   handle. This is the Windows-side counterpart of `openat2` with
   `RESOLVE_BENEATH`. The honest close of the TOCTOU window is
   handle-based, which research 09 already rejected as the wrong scale
   for this project. The accepted stance stays: lexical validation plus
   re-validation at the output boundary, with the race documented.
3. Obsidian officially supports symlinked and junctioned vault folders on
   all three platforms, but documents that sync tools and the config
   folder must stay inside the real tree. This supports the F11 stance
   (allow a symlinked `.traces`, guard by root-keyed trust) with a
   shipping precedent. It adds one duty. A vault reached through a
   symlink must resolve its real path for trust keying. Otherwise two
   spellings of the same vault get two trust identities.
4. Windows case sensitivity: NTFS volumes are case-insensitive and
   case-preserving by default. Since Windows 10 1803, a per-directory
   attribute flag makes one directory case-sensitive (the WSL doc is the
   reference). A vault can therefore mix both behaviors across
   directories. This is the platform fact behind the h2 case-stability
   rule. Keep the first-seen spelling as row identity, fold case only
   for link matching, and emit a notice on a flip. Source:
   https://learn.microsoft.com/en-us/windows/wsl/case-sensitivity [V]

## File-system normalization facts (32)

Sources: https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/APFS_Guide/FAQ/FAQ.html
[V]; https://bugs.openjdk.org/browse/JDK-8296141 and
https://bugs.openjdk.org/browse/JDK-8366911 (JDK normalization history)
[V]

1. APFS accepts only valid UTF-8 names and preserves both case and
   normalization on disk in all variants. Case-insensitive variants
   normalize case for lookup only; the stored spelling stays. HFS+ (the
   old default) normalized names toward a variant of NFD on
   case-insensitive volumes, which is where the macOS NFD folklore
   comes from.
2. The JDK story confirms the platform moved: JDK 8296141 added macOS
   name normalization for HFS+ compatibility, and JDK 8366911 removed it
   again (fixed for JDK 26) because APFS made it unnecessary. A tool
   written today should not normalize on read against macOS; it should
   normalize on write (NFC admission) and compare consistently.
3. Consequence for the portable key form: NFC-write + never-fold keys
   match APFS behavior exactly, and the 8-bit-clean byte order of the
   key matches the preserved spelling. The non-UTF-8 name question
   shrinks to Linux + Windows; APFS refuses such names at creation.
   Source: APFS FAQ, "How does Apple File System handle filenames?" [V]

## redb page reclaim and compaction (28, 29, 36)

Source: https://github.com/cberner/redb/blob/master/docs/design.md [V];
`Database::compact` doc string in redb 4.3.0 [V]

1. Pages are copy-on-write after commit. A delete moves the page into
   a pending-free state recorded in a freed tree, and a later
   transaction reclaims it. This matches the g1 finding: file size
   does not shrink after a table clear. The pages return to the
   allocator's freelist, but the file keeps its length.
2. Explicit compaction takes the writer byte plus an exclusive header
   lock, refuses to run while a transaction is in progress, and rewrites
   the file shorter. So "compact" is a stop-the-writers operation; the
   ticket 36 durability discussion should treat it as an offline
   maintenance command, not a query-time path.
3. Non-durable commits may only free pages allocated by earlier
   non-durable commits. This is one more reason the ticket 27 ENOSPC
   row-loss stays a freshness question: the rollback never frees durable
   pages.

## Editor overlay precedent detail (28)

Sources: https://github.com/rust-lang/rust-analyzer/issues/3715 (Better
VFS discussion, watchman-scale experience) [V];
https://github.com/rust-lang/rust-analyzer/issues/3107 (live diagnostics
driven by open buffers) [V]

1. The rust-analyzer VFS separates the loader (filesystem watcher) from
   the in-memory set of files. File events follow one order:
   delete wins, and a create after a delete is
   one update. This is the same merge method the
   `wrapper` design of h1 needs for disk-event versus buffer-edit races.
2. The scale experience recorded in issue 3715 argues for a flat
   Vec-based file table and interner rather than per-file handles; the
   settled `Arc<Vec<FileEntry>>` generation already follows it.

## What stays unproven

1. redb figures come from the design document at master, not from a crash
   test on our workload; the ticket 27 probe results remain the only
   measured crash evidence.
2. FSEvents coalescing behavior is cited from notify-debouncer-full's
   documented design, not measured here; no macOS machine test ran.
3. The Obsidian NFD thread confirms the bug class but not its exact
   internal cause beyond the reporter's analysis; treat the mechanism as
   reported, the lesson as design-relevant [H for mechanism].
