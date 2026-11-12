# Web precedent for ticket 28: buffer truth and generations

Each URL below was fetched for this prep. `[V]` marks a fact in the fetched
source. `[H]` marks an implication for ticket 28.

## A. Editor-as-truth architectures

1. **rust-analyzer VFS.** [V] The VFS stores changes, not content; files are
   identified by interned `FileId`s; `set_file_contents` merges two changes to
   one file in one cycle; `take_changes` drains them for salsa
   (`https://raw.githubusercontent.com/rust-lang/rust-analyzer/master/crates/vfs/src/lib.rs`).
   Lesson for 28: C2 — a change log plus merge-in-cycle is a proven shape for
   "many edits between refreshes". [H] Traces can map it to overlay entries
   that attach to one generation and are replaced by the next publish.

2. **rust-analyzer host/snapshot.** [V] `AnalysisHost` mutates; `Analysis` is
   an immutable, cheaply cloned snapshot handed to each request (research 24
   prep, source 1).
   Lesson for 28: C2 — the pinned-generation rule from ticket 24 is the same
   pattern; the overlay must name which generation its entries attach to.

## B. PKM overlay precedents

3. **md-pkm-lsp tickets 13 and 14.** [V] Transient replacement map keyed by
   URI; on-demand parse; per-request inlink patch over the persisted baseline;
   `didSave` rides the refresh path
   (`.scratch/md-pkm-lsp/issues/13-...` and `14-...`).
   Lesson for 28: C4 — a save joins the store through the normal refresh, not
   a bespoke write. [H] The racy window from ticket 25 covers the save race.

4. **zk diff-index sync.** [V] zk syncs unsaved buffers into its SQLite diff
   index (`.scratch/md-pkm-lsp`, zk research).
   Lesson for 28: C1 — queries can observe buffers without writing them to the
   store. Ticket 26 forbids journal facts from unsaved buffers.

## C. External change and save races

5. **IWE ownership reversals.** [V] v0.17.0 reversed last-write-wins after disk
   changes overwrote unsaved buffers; v0.18.0 batched the search rebuild after
   300 ms idle (research 12, CHANGELOGs).
   Lesson for 28: C6 — the ownership rule exists because its absence lost user
   data. The Answer should cite both reversals.

6. **Git racy-index guard.** [V] Entries whose mtime reaches the index write
   time are re-read and content-checked; Traces adopts the same rule as the
   `racy` bit with a 2 s margin (ticket 25).
   Lesson for 28: C7 — the S2 save-race scenario resolves through the existing
   racy rule; the Answer must name it rather than invent a new mechanism.

## D. Coherence across processes

7. **redb lock model.** [V] The default `ExclusiveWriter` mode rejects a second
   writable open while a writer holds the file
   (redb `ConcurrencyMode` docs, cached 4.3.0).
   Lesson for 28: C9 — the S9 scenario's "one process waits" outcome follows
   from the default mode. The overlay never enters the store. The host does not
   need the store open to answer.
