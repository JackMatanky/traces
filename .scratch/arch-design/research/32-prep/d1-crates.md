# Crate evidence for ticket 32: path and confinement crates

Crate facts come from `rust-docs-mcp` against the cached versions. Each finding
names the concept that it informs.

## A. Validated newtypes (C1, C4)

1. **strict-path 0.2.3.** [V] `PathBoundary::try_new` canonicalizes the root
   once; `strict_join` joins a candidate and enforces the boundary through
   `canonicalize_and_enforce_restriction_boundary`; the result is a `StrictPath`
   with `strict_read_dir`, `strict_rename`, `strict_copy` and companions.
   Lesson for 32: C1 — a boundary authority already exists in the tree. The
   Traces gap is that `SafeRelativePath::parse` discards the `StrictPath`
   (`src/path.rs:190`). A decision can keep the crate and keep the checked
   path.

2. **relative-path 2.0.2.** [V] `RelativePath` is a validated,
   separator-normalized relative path with `is_normalized`, `strip_prefix` and
   an explicit base for filesystem use. Lesson for 32: C4 — a portable UTF-8
   form with fixed `/` is a mature crate. It cannot carry non-UTF-8 Unix names.
   That is the cost research 09 records.

3. **camino 1.2.6.** [V] `Utf8Path`/`Utf8PathBuf` replace `Path`/`PathBuf` with
   UTF-8 equivalents using OS separators.
   Lesson for 32: C4 — camino is for OS-side UTF-8 paths; the persisted form
   wants relative-path semantics, not camino. Cite both to prevent conflation.

4. **typed-path 0.12.3.** [V] Typed `Utf8UnixPath`, `Utf8WindowsPath` and
   encoding conversions (`with_unix_encoding`, `with_windows_encoding`) over
   the same buffer.
   Lesson for 32: C5 — one buffer with two encodings is a candidate shape for
   a portable store form that converts at the I/O boundary.

## B. Capability handles (C1, C2)

5. **cap-std 4.0.3.** [V] `Dir` wraps an owned descriptor; every operation
   takes `&Dir`; `DirEntry::open_with` opens inside the handle; ambient
   constructors exist but are separate.
   Lesson for 32: C2 — real TOCTOU immunity needs handle plumbing through every
   leaf. Research 09 prices this as disproportionate here; the crate structure
   shows why (the `fs::dir` module is pervasive).

6. **rustix 1.1.5.** [V] `openat2` with `RESOLVE_BENEATH` is available behind
   the fs feature on Linux.
   Lesson for 32: C2 — a per-boundary TOCTOU stance could use openat2 on Linux
   only; Windows needs NT calls and macOS needs manual traversal. A partial
   closure must be stated per platform.

## C. Name validation (C4, C7)

7. **gitoxide guards.** [V] `protect_hfs` rejects ignorable-codepoint
   `.git` tricks; `protect_ntfs` rejects 8.3 names, reserved devices and
   alternate data streams (research 09).
   Lesson for 32: C4 — the ticket's Windows-reality bullet can adopt these
   checks as rejected-name rules at admission rather than as a crate dependency.

8. **path_jail / path-security / pathdiff / path-slash.** [V] Cached crates
   without generated docs; they cover confinement prefixes, separators and
   relative diffs.
   Lesson for 32: C7 — the ecosystem has many small path crates. Check whether
   any replaces the in-house `RelativePath` before the grilling. Otherwise the
   decision keeps the in-house type and fixes the glossary.
