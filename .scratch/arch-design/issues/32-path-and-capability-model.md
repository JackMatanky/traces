# What owns root confinement and path identity?

Type: grilling
Status: open
Blocked by: 9, 12, 16, 21

## Question

- One invariant rules this ticket: all file access stays inside the project
  root. Define one owner for it.
- Evaluate validated newtypes at boundaries against a capability root handle.
- Decide how `FileMeta`, index rows, query paths, template paths, and CLI errors
  carry confined versus raw paths.
- Decide the TOCTOU stance: accept it for a single-user CLI, or close it.
- Make template path *inspection* and `DryRun` consistent with the writer.
- Decide the persisted path form in redb (OS bytes versus normalized UTF-8) and
  cache portability.
- Decide `DirTree` and `file_tracker` path usage.

Facts: `survey/root-primitives.md` H7/§2, `survey/template.md` H4/H5,
`survey/cross-cutting.md` H4, `src/path.rs:64,146,181-190,340-437`,
`src/template/writer.rs`, `research/09`.

Alternatives: capability `ProjectRoot` handle; newtype propagation;
boundary-only validation with a single audited fs facade; keep as is. Include
Windows reality from `research/09`.

Inputs: `research/09`, [ticket 16's
Answer](./16-grilling-compatibility-envelope.md) (compat envelope). Cache
portability is irrelevant if the cache is disposable. Until this ticket gives
one portable path form, ticket 25 binds the store to a platform family.

## Decision inputs from research 9

Read [research 9](../research/09-capability-path-precedent.md) before the
grilling. It prices the options:

- The current codec makes a store machine-local. Unix writes raw OS bytes
  (`src/path.rs:356`), Windows writes wide units (`src/path.rs:390`), and the
  two forms are incompatible (`src/path.rs:340-437`) [V].
- Git keeps internal tree paths as `/`-separated UTF-8 byte strings and converts
  to OS form only at the I/O boundary. Ruff runs its whole database on validated
  UTF-8 paths. The `relative-path` crate models portable UTF-8 paths with fixed
  `/` separators and an explicit base [V].
- The UTF-8 option has a cost: a non-UTF-8 Unix file name has no UTF-8 form and
  cannot round-trip [V].
- `SafeRelativePath::parse` validates with `strict-path` and then discards the
  canonicalized path. It returns the plain join (`src/path.rs:190`) and keeps
  the acknowledged TOCTOU window (`src/path.rs:159-161`) [V]. `strict-path` is a
  point-of-validation check, not an ongoing handle [V].
- Cargo warns against `std::fs::canonicalize` for path storage flows: it fails
  on nonexistent paths, resolves symlinks unexpectedly, and introduces Windows
  verbatim `\\?\` prefixes [V].
- Full `cap-std` handles and Linux `openat2(RESOLVE_BENEATH)` do not fit a local
  CLI tool [H]. The refined direction is a `ProjectRoot` boundary authority that
  owns canonical root resolution, safe joining, and file writing, not an OS
  handle wrapper [H].
- Confinement belongs at the I/O boundary, not as a viral type [H]. Persistent
  structs should carry a purely relative, normalized UTF-8 path newtype [H].
  Reason: `SafeRelativePath` joins to an absolute root and couples persisted
  data to one host [H].

These are recommendations with cited comparables. This ticket decides; none of
them is settled.

## Scope the roots before deciding

Do not treat all config I/O as vault-confined. The invariant covers I/O inside
the project root. Prior decisions put other stores elsewhere:

- The trust store and the tracker store live under the OS state home
  (`dirs::TRUSTED_CONFIGS`, `dirs::TRACKED_CONFIGS`), not under the vault [V]
  (`../research/22-prep/a3-inventory-trust-tracking-init.md`).
- The global config file lives in the config home.
- Template path inspection accepts absolute paths
  (`src/template/engine/path.rs:161-170`), and the template default output joins
  `Config::output_dir` without confinement (`src/template/writer.rs:328-336`).
  These are standing leaks. This ticket either endorses or closes each one. They
  are not evidence that config I/O is vault-confined.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). Class
binding `paths` are vault-relative globs with portable `/` separators and `!`
exclusions. They match against the carried relative path of the note. The glob
dialect compiles `books/` to `books/**`. `check_note` takes a path on each side.
A path can be absent in stdin or dry-run mode. In that case, path bindings do
not apply.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md). IWE
identifies a note by a portable, extensionless, `/`-separated key. Watcher keys
normalized to `/` on Windows (iwec CHANGELOG). IWE checks workspace escape in
one place before touching disk or graph (`crates/diwe/src/fs.rs`,
`key_escapes_workspace`, `workspace_document_path`) [V]. This matches the
portable-path lesson of research 9. IWE offers no capability handle.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md). The
adapter supplies a typed OS start path and a fixed base for relative overrides.
Distinguish canonical root identity for workspace admission/index coordination
from a note's portable vault-relative path. Also distinguish it from the
confined filesystem capability.

Multi-root readiness does not let two roots aliasing one directory write
independent indexes. Resolve symlink/alias identity and TOCTOU policy here and
in tickets 25/33. Do not presume a registry or new capability.

## Inputs from the config ownership decision

See [ticket 22's Answer](./22-config-ownership-and-lifecycle.md).

- Trust keys by the canonical workspace root. Keying by config path is unsafe
  [V]. When two repositories share one `.traces` directory through a symlink,
  both resolve to one key [V]. Trusting one then silently trusts the other [V]
  (`../research/22-prep/j-attack-unified-config-store.md`, section 2.6).
- `WorkspaceIdentity` derives at open time from the absolute path,
  `fs::canonicalize`, and symlink-escape checks. That derivation is settled;
  this ticket decides which path form the store and the journal carry, not how
  the identity derives.
- The `HostInputs` adapter takes raw `PathBuf`s and does not invent path
  capability types [V] (`../research/22-prep/h-redteam-and-interface-check.md`).
  The capability type is this ticket's to name.
- Open question from prep: may `.traces` itself be a symlink?
  (`../research/22-prep/o-stress-test-findings.md`, F11.) Decide it here, or
  hand it back with a named owner.

## Inputs from the freshness decision

See [ticket 25's Answer](./25-index-freshness-and-invalidation.md).

- Unix and Windows write different path bytes into the store
  (`src/path.rs:349-391`). Until this ticket gives one portable form, the store
  identity holds a platform family. A store does not move between families.
  Remove the platform family when the form is portable.
- This ticket has no Unicode normalization rule. Until it has one, NFC and NFD
  names are different files.
- The store keeps the persisted path form from this ticket. A change of the form
  changes the stored rows and needs a new format epoch.

## Inputs from the journal decisions

See [ticket 26's Answer](./26-measure-index-recovery-journal.md) and [ticket
27's Answer](./27-index-replay-prototype-and-fault-tests.md).

- Ticket 26 hands over "the path form in the journal"
  (`issues/26-measure-index-recovery-journal.md:83`). The journal records the
  base store identity and per-file metadata with the original scan start. It
  records facts for replay, old derived keys to delete, and link edges of
  unchanged files. Every one of those is a path-keyed structure.
- Ticket 27 landed move and copy evidence: "`cp -R` reparses. `mv` replays." A
  copy does not fail merely because its root path changed. A changed
  inode-change time on a Unix copy can fail the source snapshot check, and the
  journal then discards and parses fresh.
- The journal writes to a fixed temp name under `.traces/`, beside the
  engine-specific journal name. The journal name and its path form stay open:
  "The path form in the journal. Ticket 32 owns the path model."
  (`issues/27-index-replay-prototype-and-fault-tests.md:451-452`).
- The journal portability answer must agree with the platform-family rule of
  ticket 25. A journal that moves between platform families needs the same
  portable path form as the store.

## Inputs from the note document and index read model decisions

See [ticket 24's Answer](./24-index-read-model-and-query-execution.md).

- Ticket 24 gives this ticket the persisted path form and the root identity.
- A folder `Path` query uses a path range over the `files` facet. The persisted
  bytes must sort in the order that the range needs.
- Check the path order at index build and at store load, in release builds too.
- The candidate facet layout keys tables by path
  (`../research/24-prep/z-grilling-guide.md:89-92`). The guide proposes a store
  path type whose rows own `Arc<FileEntry>` [P]
  (`../research/24-prep/z-grilling-guide.md:264-266`). Decide the path type
  here; the row type belongs to ticket 29.
- Sorted-path facts: the release order check costs 1.7 ms at 50,000 paths with
  `Path` comparison and 0.12 ms with `OsStr` comparison [V]. The binary search
  in `LinkResolver::get_by_path` and the merge in `assemble_internal` need the
  order (`../research/24-prep/z-grilling-guide.md:259-262`).

## Shared contracts from tickets 28 and 29

Both tickets are open. Read them for shared contracts only.

- Ticket 28 ([question](./28-live-buffer-and-persisted-index-consistency.md))
  compares a closed-file edit against the full source snapshot at commit, and
  stores unsaved buffers with `NoteDoc::scan` and `parse_with_doc`. It needs a
  path identity that survives a generation pin.
- Ticket 29 ([question](./29-query-core-and-plan.md)) enumerates store paths for
  `All`, `Not`, and source-less shapes, uses a path range for folder sources,
  and tag keys for tag sources. Its row type owns `Arc<FileEntry>`, not a whole
  index.
- A change to store key ordering re-sorts both tickets' scans. Record the sort
  order as part of the portable form.

## Standing obligations

1. Admission identity (tickets 20, 21): aliases and alternate spellings resolve
   to one canonical path at admission. This ticket may not revise those gates.
2. Confinement boundaries (tickets 08, 11, 13, 20, 21): notes, queries, and
   schemas stay inside the root. Templates get a safe relative path. Outputs get
   an explicit, prompted boundary.
3. Compat envelope (ticket 16): a path-identity or codec change that alters
   index results or parse outcomes is a regression unless a change table records
   it. The `.redb` layout is Free (wiped once already). The trust gate behaviour
   stays frozen.
4. Error topology: `fold_confinement` maps confinement errors to escape versus
   unverifiable (`src/path.rs:316-333`). `RelativePath` is pure;
   `SafeRelativePath` does I/O (`src/path.rs:159-161`, `:181`). Keep that split
   visible in the decision (`survey/root-primitives.md` section 3).
5. Glossary conflict: `src/GLOSSARY.md:18-20` mandates "Avoid: relative path",
   which contradicts `RelativePath` (`src/path.rs:64`)
   (`survey/root-primitives.md` section 3). A decision that keeps a relative
   path type must correct that glossary entry.

## Edge and transition scenarios to decide

Each scenario feeds specific question bullets. None is decided here.

- **Case-only rename.** `notes/A.md` renames to `notes/a.md` on a
  case-insensitive volume. One inode, two spellings, two keys under
  string-derived keys. The journal replay window holds facts under the old
  spelling [H]. Feeds the identity and form bullets.
- **NFC and NFD pair.** On Linux ext4 and on NTFS the two spellings are two
  files. On default macOS APFS both spellings address one file [H]. A vault that
  moves can turn one file into two. Feeds the Unicode rule.
- **Non-UTF-8 Unix name.** A Unix file name with invalid UTF-8 bytes has no
  UTF-8 form. The UTF-8 option cannot round-trip it [V] (research 9). The
  current byte codec can. Feeds the form bullet.
- **Windows keys after a move.** A vault written on Windows holds wide keys. The
  same vault on WSL or a Linux mount sees one opaque byte string per note.
  Journal move and copy events written before the move must stay deduplicable
  [H] (ticket 27). Feeds the form and portability bullets.
- **Ignorable codepoints.** Names that differ only by ignorable characters (ZWNJ
  `0x200C`, directional marks `0x202A-0x202E`, BOM `0xFEFF`) are distinct files
  to the file system [V] (gitoxide `protect_hfs`, research 9). A
  normalization-folding key would collide them. Feeds the Unicode rule.
- **8.3 short names.** `PROGRA~1` style names get boundary treatment from
  `strict-path` (`Cargo.toml:144`) and rejection in hostile trees from gitoxide
  `protect_ntfs` [V] (research 9). Feeds the Windows reality instruction.
- **Symlinked `.traces`.** Two repositories share one `.traces` directory
  through a symlink. Root-keyed trust keeps the grants separate [V]
  (`../research/22-prep/j-attack-unified-config-store.md`, section 2.6). Whether
  `.traces` may be a symlink is open (F11). Feeds the alias bullet.
- **TOCTOU swap.** `strict_join` validates; a concurrent symlink swap redirects
  the next I/O (`src/path.rs:159-161`) [V]. Feeds the TOCTOU stance, per
  boundary type: note reads, template inspection, template output.
- **Template inspection of an external tool path.** A template calls
  `path_exists('/usr/local/bin/pandoc')` [H]. Today that succeeds, which is the
  documented leak. Closing it breaks that use; endorsing it weakens the
  invariant (`survey/template.md` H4 and its counter). Feeds the inspection
  consistency bullet.
- **Journal on a copied vault.** `cp -R` to a new root changes every path string
  and the inode-change times. The journal must detect this cheaply and discard
  (ticket 27). `mv` keeps the strings and changes the parent; the journal may
  replay. Feeds the form and journal bullets.
- **Path-set change during replay.** One added file makes the next query decode
  every note: 499 ms and 395 MB at 50,000 notes [V]
  (`../research/24-prep/z-grilling-guide.md:19-21`). State which path-set
  assumption replay makes: it writes facts only for files it recorded. Feeds the
  form and journal bullets.

## Evidence limits

- The 24 probe ran on macOS only. Windows and Linux file locking, a real
  personal vault, and Class source cost are not measured [V]
  (`../research/24-prep/g-probe-results.md`, "Not measured").
- Ticket 27 ran 63 macOS and 62 Linux cases. Torn-write ordering is untested at
  block level. The 11-facet layout, notice replay, and the 85%-and-2x claims are
  unproved.
- Research 9 recommendations carry [H] marks with cited comparables. They are
  decision inputs, not settled constraints.
- Survey hypotheses keep their counters on record: H4 (legitimate external tool
  paths) and H5 (dry-run prompt noise) in `survey/template.md`. Research 9
  section 6 strongly affirms both [V]; the counters stay.
- Cost numbers in this ticket are magnitude evidence, not contracts.

## Owner boundaries

- Tickets 20, 21, and 22 own admission identity, execution context, and trust
  keying. This ticket consumes their answers.
- Ticket 24 needs the persisted path form and the root identity from here.
- Ticket 25 owns store identity semantics. A portable form lands as change-table
  rows there and a new format epoch.
- Tickets 26 and 27 own journal cost and replay mechanics. This ticket owns the
  journal's path form and the move/copy distinction.
- Ticket 33 composes the workspace host and shares the alias and TOCTOU policy
  with this ticket.
- Ticket 34 owns replay and discard notices
  (`issues/26-measure-index-recovery-journal.md:84`).
- Tickets 28, 29, and 35 consume the settled model. Ticket 29 co-owns the row
  type with the 24 guide proposal.
- Ticket 16 requires a change-table row for every behaviour change here,
  including any new journal file on disk
  (`issues/27-index-replay-prototype-and-fault-tests.md:453-454`).

## What the grilling must settle

1. One owner for root confinement, with the named capability type or the
   documented split.
2. The TOCTOU stance per boundary: note reads, template inspection, template
   output.
3. Template inspection and `DryRun` consistency with the writer.
4. The persisted path form: OS bytes versus normalized UTF-8, the separator
   rule, the case rule, and the Unicode normalization rule.
5. Whether the store and the journal are portable across platform families, with
   the ticket 25 and ticket 16 change-table rows that follow.
6. The sort order of the persisted form, so the folder path range and the build
   and load order checks stay correct.
7. `DirTree` and `file_tracker` path usage, including whether the state-home
   stores keep raw `PathBuf`.
