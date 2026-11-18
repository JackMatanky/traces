# Web precedent for ticket 32: path equivalence and portability

Each URL below was fetched for this prep. `[V]` marks a fact in the fetched
source. `[H]` marks an implication for ticket 32.

## A. Unicode and case equivalence (C4)

1. **Deno advisory GHSA-8xpq-cjcf-3wh9 (CVE-2026-49401).** [V] Deno's
   permission check compared requested paths to deny rules at the raw-byte
   level. APFS treats NFD, case and ligature spellings as one file, so a
   script reached a denied path by respelling it. Fixed in 2.7.14 by
   normalizing both sides to NFC and case-folding before comparing
   (`https://github.com/denoland/deno/security/advisories/GHSA-8xpq-cjcf-3wh9`).
   Lesson for 32: C4 — a rule that compares bytes while the file system folds
   spellings is a real bypass class. The Unicode rule and the case rule are
   security decisions, not preferences.

2. **APFS normalization behavior.** [H] Default APFS volumes are
   normalization-insensitive and case-insensitive; ext4 and NTFS are not
   (ticket 32 scenario table; Deno advisory wording).
   Lesson for 32: C4 — the stored form must state whether keys fold; folding
   keys collide ignorable-codepoint names (gitoxide `protect_hfs` evidence).

## B. Order-preserving portability (C5, C6)

3. **SQLite4 key encoding.** [V] `memcmp`-ordered encodings with prefix
   termination for text
   (`https://sqlite.org/src4/doc/trunk/www/key_encoding.wiki`). Lesson for 32:
   C6 — a UTF-8 `/`-separated form sorts byte-wise. The folder range and the
   release order check then agree by construction. The `notes/` versus
   `notes-x.md` divergence disappears only if the persisted order is byte order
   and the in-memory checks use the same comparison.

4. **CockroachDB canonical encodings.** [V] One encoding per type makes the
   datastore architecture-neutral and platform-independent
   (`https://www.cockroachlabs.com/blog/cockroachdb-cross-platform-portability/`).
   Lesson for 32: C5 — a portable form lets ticket 25 drop the platform-family
   identity part, at the cost of one format epoch.

## C. Confinement architectures (C1, C2)

5. **rust-analyzer VFS path model.** [V] `VfsPath` is an abstract path;
   `FileId`s are interned; the VFS does no I/O itself
   (`https://rust-lang.github.io/rust-analyzer/vfs/struct.VfsPath.html`; source
   fetched this session).
   Lesson for 32: C8 — separating the path identity from filesystem access is
   a proven shape for overlaying hosts. Ticket 28's buffer keys can ride the
   same identity.

6. **cap-std handle cost.** [V] True confinement passes `&Dir` to every
   operation; macOS and older Linux fall back to O(components) descriptor opens
   (research 09; cached crate structure).
   Lesson for 32: C2 — the accept-per-boundary stance must name which
   boundaries would use handles if closed, and the measured cost of closing
   them.

## D. Symlinked state (C9)

7. **Ticket 22 attack record.** [V] Two repositories sharing one `.traces`
   through a symlink collide under config-path keying; root keying keeps
   grants separate (research `22-prep/j-attack-unified-config-store.md`,
   section 2.6).
   Lesson for 32: C9 — the F11 open question ("may `.traces` be a symlink?")
   interacts with trust keying. The spec needs a decision or a named owner.
