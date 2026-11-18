# Digest evidence for ticket 28: buffer and overlay precedents

This file records precedents for the grilling of ticket 28. It does not decide
the ticket. Each finding names the concept that it informs. Citations use line
numbers in the digest files under `docs/digests/`.

## A. The overlay question (C1, C2)

1. **markdown-oxide replaces the vault under a lock.** [V] Its backend holds
   `Arc<RwLock<Option<Vault>>>`. A query runs inside the read guard. An edit
   builds a new vault and swaps it under the write guard
   (`docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt:1670-1678,1726-1744,1861-1898`).
   Lesson for 28: C2 — a whole-vault swap gives coherence but a rebuild between
   guards stops readers.

2. **markdown-oxide debounce counter.** [V] Each edit increments `diag_gen`. A
   delayed task exits when a newer edit changed the counter
   (`docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt:1660-1668,1800-1833`).
   Lesson for 28: C2 — a counter orders work, it does not make a snapshot
   atomic.

3. **marksman state revision.** [V] `State` holds a workspace and an integer
   revision. `Doc.withText` parses new text into a new document index. One
   `MailboxProcessor` serializes state mutations
   (`docs/digests/lsp_artempyanykh-marksman-digest.txt:12148-12174,11382-11452`).
   Lesson for 28: C1 — one serialized mutator keeps buffer facts coherent.

4. **rumdl worker takes buffer text.** [V] The index worker reads open-document
   text for a scan and reads disk for other files. An open file stays in the
   scan even when discovery excludes it
   (`docs/digests/lsp_rvben-rumdl-src-digest.txt:36941-36955,36975-36995`).
   Lesson for 28: C1 — an overlay query can mix buffer and disk facts by path.

5. **zk syncs buffers into SQLite.** [V] zk keeps text for opened documents in
   a separate map and syncs the diff index to unsaved buffers
   (`docs/digests/zk-src-digest.txt:3192-3209`; `zk-docs-digest.txt` diff-index
   sections).
   Lesson for 28: C1 — a persisted index can observe buffers without writing
   them.

## B. Save, close and external change (C4, C6)

6. **IWE editor ownership.** [V] `didOpen` and `didChange` write the buffer
   into the one graph. Watcher events for an open key are dropped until
   `didClose` re-reads the file (research 12,
   `crates/iwes/src/router/server.rs`).
   Lesson for 28: C6 — dropping external events for open keys is a proven
   ownership rule, but it hides disk truth while open.

7. **IWE close drops unsaved content.** [V] A `didClose` of an unsaved new
   document drops the content (`crates/iwes/tests/reconcile_test.rs:69-78`).
   Lesson for 28: C4 — the close rule needs an explicit adopt, change or refuse
   decision.

8. **fileclass write clobber.** [V] A `processFrontMatter` write destroyed
   user data once. The incident is recorded in the md-pkm-lsp map as the first
   external write-side hazard.
   Lesson for 28: C4 — a save join must prove the store holds the saved bytes
   before the overlay entry goes.

## C. LSP persistence facts (C9)

9. **md-pkm-lsp ticket 13.** [V] No studied LSP persists a workspace index;
   redb is a warm-start cache. The editor buffer is authoritative for open
   documents
   (`.scratch/md-pkm-lsp/issues/13-lsp-persistence-and-caching-strategy.md`).
   Lesson for 28: C9 — the store lock rule and a buffer overlay can coexist if
   the overlay never enters the store.

10. **md-pkm-lsp ticket 14.** [V] Transient replacement, not a layered overlay:
    `RwLock<HashMap<Uri, BufferState>>`, on-demand parse, per-request inlink
    patch, `didSave` rides the refresh typestate
    (`.scratch/md-pkm-lsp/issues/14-live-buffer-vs-filesystem-overlay.md`).
    Lesson for 28: C2 — candidate shape for the overlay, but it predates the
    settled generation model. Check each claim against tickets 24 and 25.
