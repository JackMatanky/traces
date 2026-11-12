# Grilling guide for ticket 32: path and capability model

Purpose: question order, evidence, options, and proposals per settlement
bullet. Do not answer for the user. Keep STE rules in the Answer.

## Order

1. Root scoping and the confinement owner (bullet 1). Everything else uses
   this owner.
2. The persisted path form: bytes, separator, case, Unicode (bullet 4).
3. Sort order of the form (bullet 6); portability of store and journal
   (bullet 5).
4. TOCTOU stance per boundary (bullet 2).
5. Template inspection and `DryRun` consistency (bullet 3).
6. `DirTree` and `file_tracker` usage (bullet 7).
7. Edge scenarios as the wrap-up check.

## Measured and adversarial evidence

Two deeper evidence files back this guide:

1. `g1-probes.md` (shared): full sort of 50k paths costs 34.7 ms in
   `Path` order versus 4.8 ms in byte order (medians of 9). Byte order
   diverges from
   `Path` order on the hyphen sibling. Yet the `[prefix, prefix+1)`
   byte range with the last byte incremented (`/` to `0`) keeps the
   sibling out of a folder scan. An end bound of `prefix + \x01`
   excludes every child. Pin the increment rule.
2. Normalization duty costs, measured (g1-probes.md, medians of 9,
   50k paths, 10% non-ASCII). A check-only NFC pass costs 1,288.7 µs
   (26 ns per path). A full normalize pass costs 16,423 µs (328 ns per
   path). A lowercase fold pass costs 1,625 µs (33 ns per path). The
   check-then-normalize-when-needed shape is affordable at every
   reconcile site. The write-time normalize adds less than 1% to a
   full build.
3. `h2-redteam.md`: attacks with defenses. Four decide questions.
   First, the non-UTF-8 name decision, the highest-cost open question;
   do not treat it as settled. Second, NFC comparison at every
   reconcile site (the Obsidian NFD regression class). Third, the
   end-bound increment law. Fourth, the case-stability rule for
   case-insensitive file systems.

## Per-bullet evidence and options

### Owner of confinement

Evidence: research 09 refined direction is a `ProjectRoot` boundary
authority, not an OS handle wrapper [H]. `cap-std` handles and `openat2`
do not fit a local CLI [H]. IWE checks escape in one place [V]. The trust
and tracker stores live outside the vault (research 22-prep a3) [V].

Options:

- A. `ProjectRoot` type owns canonical resolution, safe join, and file
  writing. Interior types stay plain. This is D1 of h1.
- B. `ProjectRoot` plus a validated `RelPath` newtype through persisted
  structures (D2). The newtype defines the portable form for free.
- C. Capability handles everywhere (D3). Rejected as disproportionate;
  cite when the user asks "why not close it".

Proposal: B. It satisfies the owner bullet and the form bullet with one
type. A leaves the form undefined and forces a second decision later.

### Persisted form

Evidence: Unix raw bytes, Windows wide units, incompatible codecs
(`src/path.rs:340-437`) [V]. Non-UTF-8 names cannot round-trip as UTF-8
[V]. Ticket 25 binds the store to a platform family until the form is
portable [V]. NFC and NFD are different files until a rule exists [V].
The Deno advisory proves byte-compare against a folding file system is a
bypass class [V]. Gitoxide guards reject ignorable-codepoint tricks [V].

Options for the four rules:

- Bytes: UTF-8 with `/` separators (portable, drops the platform family,
  new format epoch) versus OS bytes (round-trips non-UTF-8 names, store
  stays machine-local).
- Case: never fold keys. The comparison key of the src glossary stays a
  name-matching rule, never a store rule [V].
- Unicode: write NFC as the canonical spelled form; never fold at read.
  Folding collides ignorable-codepoint names [V].
- Rejected names: adopt `protect_hfs`-style admission checks for
  ignorable-codepoint and Windows-reserved names rather than a crate
  dependency [P].

The cost to state: a scan that meets a non-UTF-8 name skips it and emits
one notice. Ticket 16 needs the behavior row.

### Sort order and portability

Evidence: the persisted bytes must sort in the order the folder range
needs (ticket 24) [V]. The release check costs 1.7 ms with `Path`, 0.12 ms
with `OsStr` at 50k [V]. SQLite4 text keys prefix-terminate, so byte order
is range order [V]. The `notes/` versus `notes-x.md` divergence disappears
only when both sides compare the same form [V].

Proposal: the form sorts by plain byte comparison; the build and load
checks compare the same bytes; the folder range is a byte range. This
makes the in-memory `Path::cmp` check removable, or re-expressed over the
newtype. Ticket 29 consumes the order statement.

Journal: the journal path form equals the store form. `mv` replays, `cp
-R` discards through the base identity check (ticket 27) [V]; both keep
working under portable bytes.

### TOCTOU stance per boundary

Evidence: `strict_join` validates, and a symlink swap redirects the next
open (`src/path.rs:159-161`) [V]. Point-of-validation is not a handle [V].
Cargo warns against `canonicalize` for storage flows [V].

Options per boundary:

- Note reads: accept (single user, own machine).
- Template output: validate at admission and again at open; no handle.
- Template inspection: see next bullet.

### Template inspection and DryRun

Evidence: inspection accepts absolute paths (`src/template/engine/path.rs:161-170`)
[V]; the writer confines through `SafeRelativePath` (`src/template/writer.rs:303-336`)
[V]; survey H4 counters that external tool checks are legitimate [V].

Options:

- A. Endorse absolute inspection as a named raw-path test type. The
  invariant gets a documented exception.
- B. Close it: inspection takes only vault-relative or state-home paths;
  external checks break.
- C. Two namespaces: confined by default; an explicit raw test type with a
  distinct name for external paths.

Proposal: C. It keeps the writer and inspection consistent without
breaking the `pandoc` use case.

### DirTree and file_tracker

Evidence: the tracker canonicalizes and hashes (`src/file_tracker.rs:469-483`)
[V]; `contains` uses `try_exists` only [V]; state-home stores are outside
the vault [V].

Proposal: vault paths use the newtype; state-home stores keep raw
`PathBuf` (they are not vault-confined). Fix or document the `try_exists`
symlink gap under ticket 22's tracker ownership.

## Edge scenario answers to check with the user

| Scenario | Candidate rule |
| --- | --- |
| case-only rename | keys never fold; two spellings stay two keys; the racy rescan catches the flip |
| NFC/NFD pair | both spellings stored as written; NFC only on new writes; no fold on read |
| non-UTF-8 name | skip + one notice (UTF-8 form); or OS bytes keep it (form decision) |
| Windows keys after move | portable form makes the store readable cross-family; needs the new epoch |
| ignorable codepoints | never fold; admission rejects HFS tricks |
| 8.3 names | reject at admission (protect_ntfs rule) |
| symlinked `.traces` | allow with root-keyed trust as the guard (F11) |
| TOCTOU swap | accepted for note reads; admission re-check for template output |
| template inspection | two-namespace rule |
| journal on copy | base identity check discards; `mv` replays |
| replay path-set | replay writes facts only for files it recorded |

## Cross-ticket consistency checks

1. 32-24: the folder range and the release order check use the form's byte
   order; state the removal of the `Path::cmp` check if adopted.
2. 32-25: dropping the platform family needs change-table rows and a new
   format epoch; `index:bless` bumps it.
3. 32-29: rows own `Arc<FileEntry>`; the path type owns key bytes only
   (the 24-prep guide proposal is scoped here [P]).
4. 32-28: the buffer overlay keys on the portable form; rename re-keying
   waits on this ticket.
5. 32-22: trust keying stays root-canonical; `WorkspaceIdentity`
   derivation is settled; this ticket only picks the store form.
6. 32-16: every behavior change (skipped non-UTF-8 files, new epoch,
   inspection namespace) needs a change-table row.

## Change-table candidates for the Answer

- Store rows move to portable UTF-8 keys; new format epoch; one-time wipe
  or migration.
- Non-UTF-8 names: skip with notice (new behavior).
- Template inspection namespace rule (if C adopted).
- `.traces` symlink allowance (if adopted).
- Glossary correction: retire the "Avoid: relative path" entry if the
  newtype lands (obligation 5).
