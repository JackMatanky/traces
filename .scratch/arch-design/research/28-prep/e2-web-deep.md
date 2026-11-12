# Deepened web evidence for ticket 28

The shared round-2 file lives at research/32-prep/e2-web-deep.md. Sections
that load this ticket:

1. redb commit and crash semantics: a `Durability::None` commit rolls back to
   the last durable commit on crash; the store stays consistent, only fresh.
   Ticket 27's ENOSPC row-loss is the designed rollback. The grill should frame
   the Durability question as freshness loss, never data corruption.
2. MVCC page pinning: a read transaction pins every page it references. The
   settled "cold query pins ONE read txn" rule is load-bearing for store growth;
   a second open read path would bloat the file. This closes the "why exactly
   one txn" question with a mechanism.
3. Watcher rename semantics: rename pairing is "usually" consecutive on inotify
   and reconstructed on macOS. The overlay's rename handling must degrade an
   unmatched from-event to delete + rescan. The Obsidian NFD regression shows
   the same need on the name side. The overlay must compare watcher names
   against stored keys in the same form on both sides, or rows vanish.
4. rust-analyzer VFS change merge: per-file precedence delete > create collapse
   rules are the algebra for the disk-event versus buffer-edit race in groups 5
   and 6 of the guide.
5. redb page reclaim: deletes return pages to the freelist; the file keeps its
   length until explicit compaction, which is a stop-the-writers operation.
   Relevant if the grill asks what a long-lived overlay session does to store
   growth.

No ticket-28-specific new claims beyond these pointers.
