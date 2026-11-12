# Digest and source evidence for ticket 32: path precedents

This file records precedents for the grilling of ticket 32. It does not decide
the ticket. Each finding names the concept that it informs. Citations name the
digest or the fetched source.

## A. Confinement models (C1, C2)

1. **cap-std holds a directory handle.** [V] Every filesystem operation takes
   `&Dir`; the kernel resolves beneath the root on Linux 5.6+ and falls back to
   manual component traversal elsewhere (research 09, `cap-primitives` source).
   Lesson for 32: C1 — kernel confinement closes TOCTOU but costs a handle per
   path component and Windows NT calls. Research 09 prices this as
   disproportionate for a single-user CLI.

2. **strict-path is point-of-validation.** [V] `PathBoundary::strict_join`
   canonicalizes with soft-canonicalize, checks the boundary, then returns a
   `StrictPath`. It holds no descriptor, so a later symlink swap reopens the
   window (`strict-path` 0.2.3 source, `path_boundary.rs`).
   Lesson for 32: C2 — the TOCTOU window is a property of check-then-use, not
   of the crate. A decision must state the stance per boundary.

3. **gitoxide lexical plus traversal guards.** [V] `gix_validate` rejects
   escapes and Windows device names; `protect_hfs` blocks ignorable-codepoint
   tricks; `protect_ntfs` blocks 8.3 names and alternate data streams
   (research 09, `gix-validate` and `protect_*` source).
   Lesson for 32: C4 — filesystem-specific name rules are a known, cited
   defense. Traces reads its own vault, not hostile clones; adopt selectively.

4. **Cargo rejects canonicalize for storage flows.** [V] It fails on nonexistent
   paths, resolves symlinks unexpectedly, and emits Windows verbatim prefixes
   (research 09, `cargo-util/src/paths.rs:78-83`).
   Lesson for 32: C4 — a portable form should be lexical, normalized UTF-8 with
   `/`, converted at the I/O boundary.

## B. Persisted form and key order (C4, C5, C6)

5. **SQLite4 key encoding.** [V] Record keys are byte encodings designed so
   `memcmp` sorts keys into their proper order; text ends with 0x00 so a prefix
   sorts first (`https://sqlite.org/src4/doc/trunk/www/key_encoding.wiki`).
   Lesson for 32: C6 — an order-preserving byte form makes folder ranges and
   prefix scans correct by construction.

6. **CockroachDB column encoding.** [V] Each SQL type has a canonical byte
   encoding with an order property; the datastore stays architecture-neutral
   (`https://www.cockroachlabs.com/blog/cockroachdb-cross-platform-portability/`).
   Lesson for 32: C5 — portability comes from one canonical encoding, which
   also removes the platform-family identity part of ticket 25.

7. **Deno advisory GHSA-8xpq-cjcf-3wh9 (CVE-2026-49401).** [V] On macOS APFS a
   denied path was reachable through an NFD spelling or different case, because
   the deny check compared raw bytes while the file system folded them
   (`https://github.com/denoland/deno/security/advisories/GHSA-8xpq-cjcf-3wh9`).
   The fix normalizes both sides to NFC and case-folds before comparing.
   Lesson for 32: C4 — a Unicode normalization rule is a real security need on
   default APFS, not a cosmetic preference. Ticket 32 must state the NFC rule
   and the case rule together.

## C. Editor and vault identity (C8, C9)

8. **IWE portable key.** [V] A note key is extensionless, `/`-separated;
   watchers normalize separators on Windows; workspace escape is checked in one
   place before disk or graph access (research 12, `crates/diwe/src/fs.rs`).
   Lesson for 32: C8 — a portable relative path plus one escape check is a
   shipped pattern.

9. **md-pkm-lsp URI variants.** [V] Percent-encoding, drive-letter and symlink
   variants stay distinct keys; matching decodes paths rather than comparing
   strings (`.scratch/md-pkm-lsp/issues/30-multi-root-vs-project-root.md`).
   Lesson for 32: C8 — alias identity needs canonicalization at admission,
   which tickets 20-22 already settled. Ticket 32 decides the stored form only.

10. **md-pkm-lsp rename semantics.** [V] Case-only rename is a zero-text-edit
    rename; folded ties refuse rename as `PathTie`
    (`.scratch/md-pkm-lsp/issues/28-file-operations-and-cascading-edits.md`).
    Lesson for 32: C4 — case rules interact with rename UX. The stored form
    decides whether `A.md` and `a.md` are one key or two.
