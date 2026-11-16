# What owns root confinement and path identity?

Type: grilling
Status: resolved
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

## Answer

Decision index:

| Decision | Subject |
| --- | --- |
| D1 and D2 | Confinement owner, path type and API |
| D3 and D4 | Persisted form, identity, sort and folder range |
| D5 | Listed spelling and Comparison Key lookup |
| D6 and D7 | Admission and non-UTF-8 names |
| D8 and D9 | TOCTOU, template inspection and the two confinement exceptions |
| D10 and D11 | Symlinked `.traces`, traversal and state-home stores |

The sections before this Answer record historical decision inputs.
This Answer records the rules, including the corrections from the final review.

### Decisions recorded in the grilling session (2026-11-14 and 2026-11-15)

The maintainer confirmed the initial decisions after two red-team rounds and
three audits. The final review (2026-11-16) then settled each choice it reopened.
The project is pre-release. The `.redb` layout is Free and the
code wipes it at will. Ticket 16 freezes stdout payload shapes, exit codes and
template and query capabilities. The change table at the end of this Answer
records each change that touches that envelope.

Evidence files, all under `../research/32-prep/`:

- [`g1-probes.md`](../research/32-prep/g1-probes.md): the shared probes for
  tickets 28, 29 and 32. It holds the first sort and normalization costs.
- [`h2-redteam.md`](../research/32-prep/h2-redteam.md): the first red team.
- [`h3-redteam-round2.md`](../research/32-prep/h3-redteam-round2.md): the
  second red team, three audits and the register of reversals.
- [`e3-non-utf-8-precedent.md`](../research/32-prep/e3-non-utf-8-precedent.md):
  the precedent, prevalence and IWE and Obsidian traces for non-UTF-8 names.

Marks: [V] means a source line, a URL or a measurement backs the claim. [H]
means a hypothesis or an inference. [P] means a proposal. Measurements come from
one macOS arm64 machine with APFS, unless the text says Linux.
The recorded rules are design decisions [P], not completed code changes.
The measurements retain the source reports' [V] marks unless stated otherwise.
Unpinned upstream line references are not rechecked in this copy edit [H].

Each decision below names the alternatives (retain included), the rule and its
invariant owner, the code that the rule makes removable, and what stays open.

#### D1. One owner for root confinement (settle 1; Question bullets 1 and 2)

Alternatives:

- Capability handle (`cap-std`, `openat2` everywhere). Research 09 [H] judges
  it disproportionate for a local CLI. True immunity costs an open for each
  path component on macOS and older Linux, and Windows needs NT calls.
- Newtype propagation only, with no owner. A caller can still pass a raw
  string to a join, so the escape returns at the next boundary [H].
- Boundary validation with one audited file system facade. This is the chosen
  form.
- Retain `SafeRelativePath`. It validates with `strict-path`, discards the
  canonical result and returns a plain join (`src/path.rs:190`). It also does
  I/O inside a path type, and it ties persisted data to one host [H].

Rule:

- `ProjectRoot` owns canonical root resolution, safe join and file writes.
  These three duties are the confinement I/O. No other type joins a root with
  a relative path, except the configured output-directory join in D9.
- The type has no capability handle. Research 09 prices the handle as too costly
  for this tool [H].
- `ProjectRoot` is, or contains, the `WorkspaceIdentity` of tickets 22 and 32.
  Ticket 22:182 forbids a workspace object in core. The relationship is this:
  core holds no workspace object. Core structs carry only pure `RelativePath`
  values. `ProjectRoot` sits at the I/O boundary and produces and consumes
  them.
- The derivation of the identity is settled in ticket 22. This ticket does not
  change it.
- State-home stores keep raw `PathBuf`. The trust store and the tracker store
  live under the OS state home, outside the vault. The vault invariant does not
  cover them (see D11).
- Error topology stays visible. `fold_confinement` maps a confinement error to
  "escape" or "unverifiable" (`src/path.rs:316-333`). `RelativePath` is pure.
  `ProjectRoot` does the I/O.

Invariant owner: `ProjectRoot` owns confinement of vault file access, with the
two named exceptions in D9. `RelativePath` owns lexical validity. A stored
value contains no `..`, NUL or empty component.

Code made removable: `SafeRelativePath` and its tests
(`src/path.rs:159-190`, `:558+`). The source reports give 8 call sites [H].
Their impact list is `schema/fields/select.rs:258`, `template/writer.rs:313`,
`template/engine/file.rs:81`, `template/engine/path.rs:165`,
`config/model.rs:300`, `lib.rs`, `file.rs` and `path.rs`.
This list mixes callers, definitions and other affected files.
The checked tree has 5 production calls and 6 test calls [V].
The production calls are the five numbered source locations above.

Unresolved: the crate and module home of `ProjectRoot` (tickets 33 and 35).
Whether the new method keeps the `strict-path` dependency (`Cargo.toml:144`) is
an implementation choice [H].

#### D2. Newtype and API rules (settle 1; Question bullets 2 and 3)

Alternatives:

- A new name such as `RelPath` or `VaultPath`. The name `RelativePath` already
  exists (`src/path.rs:64`). The source tally is 63 hits in 6 files [H].
  A new name leaves two similar types. AuditUxConventions asked for a clean break.
  The checked tree has 32 whole-word uses in 6 files [V].
- Retain `RelativePath(PathBuf)`. It allows `.` components and holds an
  OS-specific form.
- Plain `String` or `PathBuf` with validation only at the boundary. It leaves
  the persisted form undefined.
- Reuse the name `RelativePath` with new rules. This is the chosen option.
  The first recommendation aimed to leave 1,681 `.path()` calls mostly untouched.
  The later native-boundary correction removes that promise.

Rule:

- `RelativePath` is a normalized, `/`-joined UTF-8 `String`. It is pure and
  does no I/O.
- `parse(&str)` rejects rooted input before normalization, including leading
  `/` and `//`. Reject NUL and every `..` component.
- Remove `.` components and repeated or trailing separators. The normalized
  result needs at least one named component and contains no empty component.
- Thus `a//b` becomes `a/b`, `./a/./b` becomes `a/b`, and `a/` becomes `a`.
  Empty input, `.`, `./`, `/a`, `//a` and `a/../b` fail.
- Only `RelativePath::from_os(&OsStr)` and
  `ProjectRoot::relativize(&Path) -> Result<RelativePath, Skip>` treat `\` as a
  separator. They do this on Windows only. `relativize` calls `from_os` for each
  component.
- The text API is `as_str()`. There is no public zero-cost `as_path()` promise.
  `ProjectRoot` owns native path conversion and native component validation.
  Native-unrepresentable names produce typed errors. `RelativePath` stays pure
  portable text. The maintainer confirmed this correction after the Windows review.
- Native conversion rejects roots, prefixes, traversal and component
  reinterpretation before joining the root. On Windows this includes device,
  stream and trailing-alias interpretations. Never open another file as a fallback.
- Keep `strict-path` for the initial cutover. Remove it only after the replacement
  passes the confinement conformance cases. A lexical prefix check is insufficient.
- `RelativePath` has no `Borrow<str>` and no `Deref<Target = str>`. The `Ord`
  of the type maps `/` to NUL, so it differs from `str` order. `Borrow` needs
  `Ord`, `Eq` and `Hash` to agree.
- A property test checks that `Ord`, `Eq` and `Hash` agree. The generator
  produces NUL, `%00`, empty components and a lone `\`.
- `HasPath::path()` returns `&RelativePath`.
- `RelativePath` holds one `String`. The NUL-form key is not stored in the
  type (see D4).

How each structure carries confined and raw paths:

| Place | Form | Owner of the rule |
| --- | --- | --- |
| `FileMeta`, `Note`, `FileEntry` path | `RelativePath` (pure) | `RelativePath` |
| Persisted path values and journal facts | Canonical UTF-8 text with `/` separators. Journal paths contain no NUL separators. | `RelativePath`; journal consumer |
| Database lookup keys | NUL-form encoding of the canonical path value (D4). | Store adapter through the ticket 24 port |
| Query path literal and folder atoms | Typed text. Planning resolves it to a listed `RelativePath` through the fold lookup (D5). Glob atoms stay text. | Ticket 29 IR |
| Template path input (`TemplatePathInput`) and config subdirectory (`ConfigSubDir`) | `RelativePath::parse`. It normalizes `./` and rejects NUL (change row 18). | `RelativePath` |
| Any vault file read or write | `ProjectRoot` joins and confines. D9 exempts the raw-path inspection test and the configured output-directory join. | `ProjectRoot`; ticket 31 exposes the template exceptions. |
| Trust store and tracker store | Raw `PathBuf` | Ticket 22 |
| Confinement errors | `fold_confinement` keeps the escape and unverifiable split. Message text belongs to ticket 34. | Ticket 34 |

Public API break: `FileMeta::path`, `Note::path`, `MarkdownParserInput::new` and
`FileEntry::inlinks` change type. The `path_codec` re-export (`src/lib.rs:192`)
goes away. The baselines `pub-tokens.txt`, `public-api-default.txt` (16 lines)
and `public-api-test-utils.txt` (the three `testing::path_codec` lines) change.
If `FileMeta::path()` returns `&RelativePath`, the type must become `pub` or sit
in a private module. That is a widening under the ticket 16 rule. Ticket 35
decides it.

Glossary duty (obligation 5): `src/GLOSSARY.md:20` says "Avoid: relative path".
That text conflicts with the type. Correct it and add a pure "Relative Path"
entry beside "Root-Confined Path". The full glossary list is in "Resolution and
handoffs".

Invariant owner: `RelativePath` for lexical rules. Ticket 35 for visibility.

Code made removable: the `.`-allowing `RelativePath(PathBuf)` body, the
`HasPath::path() -> &Path` signature (`src/path.rs:57`), and the use of
`PathBuf` for `FileMeta.path` and `Note.path`.

Unresolved: the visibility of `RelativePath` (ticket 35). Native Windows
conformance remains an acceptance gate, not a claimed result.

#### D3. Persisted form, identity and portability (settle 4 and 5)

Alternatives:

- OS bytes (retain today's codec). The Unix codec writes raw OS bytes
  (`src/path.rs:356`). The Windows codec writes wide units (`src/path.rs:390`).
  The two forms are incompatible (`src/path.rs:340-437`). The store stays
  machine-local and keeps the platform family. The std docs say that
  `as_encoded_bytes` is not safe to store in a file (`os_str.rs:1049-1060`) [V].
- UTF-8 with `/`, NFC keys, with a normalization step at every reconcile.
  The first red team asked for this. The final rule keeps the listed spelling
  instead (D5).
- UTF-8 with `/`, and the listed spelling. This is the chosen option.
- UTF-8 with `/` plus a `stat_semantics` tag in `META`, outside the wipe
  identity. A mismatch keeps the rows and treats them as unvalidated. The tag
  saves about 395 ms once [H]. It is a second invalidation mechanism. The user
  dropped it.

Rule:

- The persisted path is a UTF-8 `String` with `/` separators. This needs a new
  format epoch.
- The platform family leaves the wipe identity. The canonical root stays.
  Two roots sharing a store cause identity mismatch and repeated rebuilds [V].
- A vault-root rename or move invalidates the old-root store and journal.
  Rebuild recognized derived artifacts under the existing epoch rules.
  Never recursively remove `.traces`, notes, config, schemas or templates.
- There is no `stat_semantics` tag. A pre-release wipe is free.
- A stored row without `ctime`, read on Unix, is stale. Ticket 25 states this
  rule.
- Journal fact path values are canonical UTF-8 text with `/` separators.
  Database lookup keys use NUL encoding. The store adapter owns that encoding.
  The journal does not encode path separators as NUL.
- Ticket 27's "`mv` replays" result describes its prototype, not this production
  rule. This decision supersedes its path-change exception.
- A within-vault note rename uses normal refresh rules. It does not change
  the canonical root.
- Root rebinding is not adopted. Reconsider it after measured relocation costs
  justify independent admission, writer exclusion, complete source proof and
  a crash-safe store/journal identity transition.
- Replay writes facts only for files that the journal recorded.
- Today's saved-index added-file baseline is 499 ms and 394.9 MB at 50,000 notes
  (`research/24-prep/g-probe-results.md:88-95`) [V].
  This is not a measurement of replay, deletion or the amended implementation.
- The folder-range, sort and release check rules are in D4.

Invariant owner: the store adapter writes and reads NUL-encoded lookup keys.
Ticket 25 owns the identity fields and the epoch. This ticket owns the
canonical path value. Journal facts carry that value without NUL separators.

Code made removable:

- The platform codecs in `src/path.rs:340-437` and the `path_codec` re-export.
- The lossy decode `path_from_bytes` (`src/index/codec.rs:89`) and the byte
  codec for non-UTF-8 names (`src/index/codec.rs:83-94`).
- The platform family field in `META`, and the probe test
  `rejects_mutated_platform`.

Unresolved:

- The cause text for a store that a changed canonical root invalidates. The
  refresh report carries it (tickets 25 and 34). A rebuild heal stays quiet.
- The row payload stores the path twice, once in the key and once in the
  payload. A new epoch could drop the payload copy [H].

#### D4. Sort and range key (settle 6)

Alternatives:

- Retain: `Path::cmp` order in memory and raw bytes in the store. A binary
  search that uses the wrong order silently misses rows, and release builds do
  not check it.
- Plain UTF-8 byte order (first-round proposal). It moves 8 of 13 rows in
  `list`, `table` and `task` in the probe vault. Example: `notes/a.md` sorts
  before `notes-x.md` today, and after it in plain byte order. Plain UTF-8 byte
  compare mismatched `Path::cmp` on 73,891 of 3M random pairs [V].
- `OsStr` comparison. The release check costs 0.12 ms against 1.7 ms for `Path`
  at 50,000 paths [V ticket 24]. It still leaves the order question open.
- A stored NUL-form key. A sort of 50,000 pre-built keys costs 3.4 ms at
  opt-level 3 and 4.75 ms at opt-level z. Build plus sort costs 13.5 ms at
  opt-level z [V AuditUxConventions].
- The NUL form, computed on the fly in `Ord`. This is the chosen option.

Rule:

- The sort key of a path is the path with `/` replaced by the byte 0x00. NUL
  never occurs in a file name. The comparison is bytewise.
- 3M random pairs gave 0 mismatches against `Path::cmp` [V]. The order of
  stdout stays identical to today, so no change row is needed for order.
- A folder range is `[prefix\0, prefix\x01)`. The end-bound rule is a pinned
  invariant. A test covers the boundary cases `notes`, `notes.md` and
  `notes-x/`.
- Reason for the range [V by reasoning]: children of `notes/` start with the
  bytes `notes\0`. A sibling such as `notes-x.md` or `notes.md` has a byte above
  0x01 after `notes`. So the range keeps all children and excludes the siblings.
  On plain bytes, an end bound of `prefix + \x01` excludes every child. The
  reason is that `/` (0x2F) sorts above 0x01 [V g1-probes.md]. In the NUL
  form, the delimiter is NUL, so `\x01` is the right bound.
- `RelativePath` holds one `String`. `Ord` maps `/` to NUL on the fly.
  Transient sort keys exist only at scan and load. Persisted store keys also
  use the NUL form (D3).
- Costs at 50,000 paths [V]: `Vec<PathBuf>.sort()` takes 31.6 ms. A `String`
  comparator that maps `/` to 0 takes about 10 ms at opt-level 3. Pre-built keys
  take 4.5 ms.
- The shipped profile is `opt-level = "z"` (`Cargo.toml:269-281`). An
  iterator-heavy comparator is about 2x slower there (22 ms in the audit run).
  Recheck the timings under the shipped profile (change row 19).
- The release check at index build and at store load compares NUL-form keys.
  This supersedes the `OsStr` check of ticket 24 (`24:392-395`). The check cost
  of the NUL form is not measured.
- The `Path::cmp` ban is a type rule. No `PathBuf` or `Path` appears in a sorted
  structure. A clippy `disallowed-methods` entry cannot enforce it. Clippy 1.98
  cannot name a trait-impl method: `std::path::Path::cmp` gives "does not refer
  to a reachable function". Only `core::cmp::Ord::cmp` fires, and it fires on
  every type. `v.sort()`, `a < b` and a derived `Ord` on `Target<'a>(&'a Path)`
  stay silent [V].
- Windows: `Path::cmp` order on valid Unicode equals code-point order [H]. Add
  the property test to Windows CI. Integration tests run on Linux only today.

Sites to change: `delta.rs:31,175`; `refresh.rs:280,478,575`; `inlinks.rs:402`;
`inlinks.rs:88` (`par_sort_unstable`); `entry.rs:90`; `service.rs:442`;
`store.rs:1080,1168`; `query/service.rs:364,392,417`; `index/sort.rs`.

Invariant owner: `RelativePath::Ord` is the single definition of order. The
type rule keeps other orders out.

Code made removable: the `Path::cmp` comparators at the sites above, and the
`Path` re-sort of rows after a byte-range read (`store.rs:469-477`,
`:1158-1168`) [V]. The debug-only assert in `SortedByPath::assumed_sorted`
(`src/index/sort.rs:48-54`) stays only for callers that reorder in memory.

Unresolved: type-based enforcement of the byte order across the public API
(ticket 35). The Windows order check (CI).

#### D5. Case and Unicode rule, with the fold map (settle 4)

Alternatives:

- Retain: an exact match only. A query in another spelling finds 0 rows
  (probes below). Template inspection says "exists" for the same name.
- Fold the key: lowercase or NFC in the stored key. A folded key collides names
  that the file system keeps apart (NFC and NFD on ext4 and NTFS, case pairs on
  ext4).
- Normalize to NFC at every reconcile site (first red team). The row would look
  deleted and re-added each refresh when the file system lists NFD. This is the
  Obsidian regression class.
- A persisted comparison-key table. It adds 50k rows and an epoch for no gain
  (W32a).
- Exact match only, with NFC for files that Traces creates
  (AuditSimplicity 7). It drops the dependency and the map. The user kept the
  map.
- The listed spelling as the key, plus a lookup map for queries. This is the
  chosen option.

Rule:

- The key is the spelling that the file system lists. The code never rewrites
  it. APFS returns the creation spelling (probe). Traces writes the files that
  it creates in NFC.
- A query in another spelling matches in two steps. First, it matches the exact
  listed bytes. On a miss, it looks up the Comparison Key map. The Comparison
  Key is NFC, then `char::to_lowercase`, then NFC (`src/GLOSSARY.md:107-115`).
- Each pinned generation builds the map lazily with a `OnceLock`. The input is
  the `files` keys. The map covers files and folders.
- A unique match rewrites a path-literal atom or a folder atom to the listed
  spelling at planning time. Several matches return the union of the
  candidates, with no diagnostic. The glossary says that Ambiguity (PathTie) is
  normal and "never a diagnostic" (`src/GLOSSARY.md:98-105`).
- Glob atoms are never rewritten.
- String comparisons such as `file.folder == "x"` stay exact. Ticket 19 makes
  strings byte-exact.
- The Comparison Key is a lookup aid. The code never applies it to redb keys,
  sort order, I/O paths or overlay keys.
- Fold-based link resolution is new behavior. Today `LinkResolver::get_by_path`
  is an exact `Path::cmp` search. IWE and Obsidian resolve wikilinks without
  case, with an exact-case match winning [V]. Change row 6 records it. The
  extractor fingerprint lists the normalization dependency. The epoch bumps
  (ticket 25, `25:312-317`).
- New dependency: an NFC crate. `icu_normalizer` 2.4 is in the registry cache.
  `Cargo.lock` has none today. The dependency record says that it exists
  because of query-time and link-time fold.
- `normalize_source_input` (`src/cli/mod.rs:440-472`) probes the real file
  system today. It moves into core and resolves through the fold map (change
  row 8). Ticket 33 decides who calls it. Ticket 29 settled synthesis.
- The IR needs a path-atom split before the rewrite can exist: today
  `SourceAtom::Path(GlobPattern)` is a regex glob over `display_portable`
  (`src/query/grammar/source.rs:190`, `:250-290`; ADR-0008:77-84). Ticket 29
  owns `PathAtom::{Literal, Folder, Glob}`.

Probe evidence on macOS APFS [V]:

- A vault with NFC `Notes/Café.md` and an NFD folder `Zürich/` is stored with
  the creation spelling.
- `list --from` with NFC `Zürich/` finds 0 pages. The NFD spelling finds 1.
- `zürich/`, `ZÜRICH/`, `notes/` and `NOTES/` all find 0.
- Template `path_exists` says `True` for all of these. Inspection and queries
  disagree today.

Costs at 50,000 paths [V] (`icu_normalizer` 2.4, 10% non-ASCII, medians of 11):

| Measure | Time |
| --- | --- |
| NFC check | 0.300 ms |
| Lowercase only | 1.458 ms |
| Full key (NFC, lowercase, NFC) | 2.527 ms |
| File key map build | 3.907 ms |
| Folder-only map | 1.583 ms |
| 1,000 lookups | 0.193 ms |

The file map costs 0.2% of a full build of about 1.8 s [V]. A corpus with more
non-ASCII text pays more. At 3x, the map still costs about 12 ms [H].

Invariant owner: the single Comparison Key function in core owns the fold. The
planner owns the rewrite. The store never sees a folded key.

Code made removable: the file system probe in `normalize_source_input`
(`src/cli/mod.rs:440-472`).

Unresolved:

- The home of the map on the CLI cold path. It builds from `files` keys on a
  miss.
- The owner of the `normalize_source_input` calls (ticket 33).
- Ticket 29 owns the settled `PathAtom` split (D5). Its implementation remains.

#### D6. Admission: which names the scan skips (settle 4; obligation 1)

Alternatives:

- Reject by character class, in the style of `gix-validate` (`protect_hfs`,
  `protect_ntfs`, `protect_windows`).
  - Measured today: the binary indexes nine such names [V].
  - They are Persian ZWNJ `می‌خواهم.md`, Hindi ZWJ, a BOM name, U+202E,
    `file~1.md`, `CON.md`, `aux.md`, `a:b.md` and `trail.` with a final space.
  - This rule would skip all nine. That is a ticket 16 regression.
  - It hits Persian and Indic names that need ZWNJ or ZWJ for correct spelling.
- A skeleton check against protected names only. The skeleton strips HFS+
  ignorables, a trailing dot or space and a `:stream` suffix. Then it applies
  NFC and a case fold. The protected names are `.traces`, the other ignored
  dirs, `TRACES~N` and `GIT~N`. APFS does not ignore those codepoints:
  `.z\u200Czz` is not `.zz` [V]. The user dropped this rule.
- Match `.traces` and every ignored directory without case. It hides `Target/`
  and `Node_Modules/` on case-sensitive Linux. It also changes user ignores and
  config discovery, which share the predicate (`env_vars.rs:19-44`;
  `index/service.rs:427`; `config/discovery.rs:163`).
- Retain the exact, case-sensitive match. This leaves a live bug (below).
- Exclude the admitted metadata directory by filesystem identity and prune its
  complete subtree. Keep the protected-name check as a separate lexical rule.

Live bug [V, measured on APFS]. After `.traces` became `.Traces`, the scan
indexed `.Traces/config.toml` and `.Traces/index.redb`. The store changed size on
every `list` (98304, 94208, 98304 bytes). The cause is the exact match in
`is_ignored_dir`. The lexical `path != index_db` check
(`service.rs:430-435`) does not establish filesystem identity.

Rule:

- Resolve the admitted `.traces` directory once. Exclude every listed directory
  with that filesystem identity before entering its subtree.
- Keep the protected `.traces` name check without case. The query Comparison
  Key remains a lookup aid, never an identity proof.
- Use a transient device/inode identity on Unix. Prove the corresponding native
  identity mechanism on Windows before cutover.
  Persist neither identity in path keys.
- Other ignored directories and `TRACES_IGNORED_DIRS` keep exact case.
- The scan never rejects a name by character class.
- The scan applies no HFS+ ignorable, `:stream` or 8.3 skeleton. The 8.3 forms
  `TRACES~N` and `GIT~N` are not special.
- Native Windows conversion rejects device, stream and trailing-alias
  reinterpretation before I/O. These checks are mandatory, not optional lint.
  Ticket 34 can add an advisory output-name lint separately.
- Ticket 25's metadata-exclusion test covers `.Traces`, physical `.traceſ`,
  and an admitted in-root metadata target under another listed name.
- Facts from `gix-validate` [V per source report]: `protect_hfs` compares a name
  with `.git` only after it removes ignorables. `protect_ntfs` guards `.git`
  short names, not general 8.3 names such as `file~1.md`.
  `protect_windows` rejects device names, illegal characters and a trailing dot
  or space. gix guards a checkout of untrusted content. Traces reads names that
  already exist.
- This rule preserves the admission gates of tickets 20 and 21. It repairs
  metadata self-indexing without broadening query case equivalence.

Invariant owner: `ProjectRoot` supplies metadata identity and confinement.
The scanner owns identity-based subtree exclusion and the separate lexical
ignore rules.

Code made removable: retain `path != index_db` until the replacement exclusion
proves that no metadata artifact enters the scan.

The APFS `.traceſ` counterexample is recorded in `h3-redteam-round2.md`,
section 15 [V per Stress32]. The maintainer confirms identity-based exclusion.
Other ignored directories keep exact-name matching.

Unresolved: the Windows identity conformance test and optional lint code
(ticket 34). HFS+ behavior remains unmeasured.

#### D7. Non-UTF-8 file names (settle 4; Question bullet 6)

Alternatives:

- Retain: index the bytes. Today the code stores raw bytes in the key, but tag,
  class and folder queries lose the file (below).
- Skip with a notice. This is the chosen option.
- A hybrid escape. Each invalid byte becomes U+FDD0 plus two hex digits. Valid
  names stay byte-identical. It needs a codec, a display form, a typeable input
  form and a property test. It adds new rows to `list` output.
- Reserve U+FDD0 at admission now, so that an escape can come later. It adds a
  second skip class for valid names that hold U+FDD0. The user dropped it. An
  epoch bump before release is free.
- Lossy U+FFFD in the key. Two names that differ only in invalid bytes collide
  (IWE does this).
- Raw OS bytes in the key. The store stays machine-local and keeps the platform
  family.

Rule:

- The scan skips a name that is not valid UTF-8. It prints one notice that shows
  the name in git-style quoted octal. On Windows, lone surrogates print as
  `\uXXXX`, and valid text prints as UTF-8.
- One function does the conversion: `RelativePath::from_os` through
  `ProjectRoot::relativize`. The real ingresses use it: the scan
  (`RelativePath::derive`, `src/index/service.rs:424-461`),
  `ProjectRoot::relativize`, and the host URI mapping (later).
- The CLI `--from` is `Option<String>`, and clap rejects non-UTF-8 input before
  the function runs. Outlink targets are `&str`. For both, the ingress is
  vacuous. The watcher and `didOpen` do not exist yet, because no host code
  exists.
- No U+FDD0 is reserved.
- The revisit trigger is a user report of a legacy-encoded vault. A host that
  must name such a note is the other trigger.
- This reverses current tested behavior. The code indexes non-UTF-8 files
  byte-exact today (`src/index/codec.rs:83-94`, `service.rs:1300`,
  `store.rs:2772`, `benches/index_codec.rs`). Invert those tests. Do not delete
  them. Change row 3 records the change.

Evidence on Linux today (ext4 container, release build of HEAD) [V]:

- `traces index` prints `indexed 11 file(s)`. `list` shows 8 pages. The 3
  non-UTF-8 files are silently missing.
- Tag, class and folder sources return paths through the lossy
  `path_from_bytes` (`src/index/codec.rs:89`). Then `read_batch` runs
  `table.get(lossy_bytes)` and returns `Ok(None)` (`store.rs:1009-1016`). No
  notice appears.
- `list --from '#t1'` returned only `normal.md`, although three files carry the
  tag.
- `as_encoded_bytes` is documented as not safe to store in a file
  (`os_str.rs:1049-1060`).

Precedent (condensed from [the evidence
file](../research/32-prep/e3-non-utf-8-precedent.md)):

| Tool | Behavior | Source |
| --- | --- | --- |
| jj | Skips the path and reports it. `RepoPath` is UTF-8. Before commit `54c4b49`, one such file failed the whole snapshot. | [V] commit 54c4b494; issues 9774, 9002 |
| Nushell | Skips the name and prints a message. | [V] nushell issue 11100 |
| rust-analyzer | Skips the name without a message. A watcher panic (issue 16914) showed that the predicate must cover every ingress. | [V] `crates/vfs-notify/src/lib.rs` |
| ty | Reports an error and continues the walk. | [V] `ruff/crates/ty_project/src/walk.rs:305-310,370` |
| Deno | Lists the name with U+FFFD since PR 34623. The lossy name is a different file. | [V] deno PR 34623 |
| git, ripgrep, fd, Mercurial | Keep the bytes. git prints quoted octal. | [V] git docs; ripgrep `printer/src/util.rs:304`; fd `output.rs:80-107` |
| IWE | Keeps a lossy key. Two names that differ only in invalid bytes silently overwrite each other in a `HashMap`. A write through the key hits a different file. | [V] `crates/liwe/src/fs.rs:29-40` |
| Obsidian | Closed source. A developer says the name encoding is managed by the OS. | [V] forum 71622; [H] for the internals |

Prevalence: no data exists for note vaults [V]. A Markdown link can carry raw
bytes as `%XX`. Traces has no percent-decoding in `src/`. Wikilinks cannot
express a raw byte. An LSP `DocumentUri` can name such a file in percent-encoded
form. JSON and MCP need UTF-8.

Invariant owner: `RelativePath::from_os` owns "a `RelativePath` is valid
UTF-8". The scan owns the notice trigger. Ticket 34 owns the notice text and the
dedup key.

Code made removable: the lossy `path_from_bytes` decode and its
`table.get(lossy_bytes)` miss path for names that the scan never admits.

Unresolved: the notice envelope and text (ticket 34). The URI mapping for a
non-UTF-8 document (ticket 28 and the host). Percent-decoding of link targets
(not decided; no decoder exists).

#### D8. TOCTOU stance per boundary (settle 2; Question bullet 4)

Alternatives:

- Close it everywhere with capability handles (`cap-std`). Research 09 [H]
  judges it disproportionate.
- Close it for template output only. Open the confined parent once and use
  `openat`, `linkat` and `renameat` through `rustix`. This is a recorded later
  option.
- Validate a second time at open (first-round proposal). It adds little,
  because the attacker needs concurrent write access inside the vault. The user
  dropped it.
- Accept and document. This is the chosen option. It is also the retain case:
  `strict_join` validates, and a symlink swap redirects the next I/O
  (`src/path.rs:159-161`) [V].

Rule by boundary:

| Boundary | Stance | Reason |
| --- | --- | --- |
| Note reads | Accept races in every component, including the final name. | The path facade is not a sandbox against a concurrent vault writer. |
| Template output | Accept parent and temp-name races. D9 retains the configured output-directory exception. With a stable parent, publication does not follow a pre-existing final destination symlink. | `commit_with_writer` (`src/template/writer.rs:366-407`) creates a temp sibling with `create_new` (`O_EXCL`) and publishes with `rename` or `hard_link`. This does not protect a mutable parent. |
| Template inspection | The confined default accepts the window. D9 exempts the separately named raw-path test. | `std::fs::metadata` follows symlinks (`engine/path.rs:118`). With `strict_join` in front, an in-root symlink that points out is rejected [V]. |
| State-home stores | Out of scope. They are not vault-confined. | They hold machine-local state. Ticket 22 owns the documented tracker `try_exists` gap. |
| Windows | Keeps the path-based check. | Windows has no beneath primitive [H]. |

Recorded later option [P]:

- Open the confined parent once.
- Use `openat`, `linkat` and `renameat` through `rustix`. It is already a
  dependency (`Cargo.toml:164`).
- Use `openat2(RESOLVE_BENEATH)` on Linux 5.6 and later. Fall back on `ENOSYS`.
- Use `O_RESOLVE_BENEATH` (0x1000) on macOS [V, `sys/fcntl.h`].
- Do not use `NO_SYMLINKS` or `NOFOLLOW_ANY`. They break in-root symlinked
  directories that the check accepts today.

Also measured: `Notes/../b.md` is rejected lexically, even though it stays
inside the root. This ticket 16 behavior stays.

Invariant owner: `ProjectRoot`. Ticket 33 shares the alias and TOCTOU policy.

Code made removable: none.

Unresolved: the handle-relative I/O upgrade remains an alternative. Revisit it
for hostile concurrent writers or a multi-user host. A second path check
does not close the race.

#### D9. Template inspection and `DryRun` (settle 3; Question bullet 5)

Alternatives:

- A: endorse absolute-path inspection as one raw-path test with a documented
  exception to the invariant.
- B: close it. Inspection takes only vault-relative paths. External tool
  checks such as `path_exists('/usr/local/bin/pandoc')` stop working. This
  breaks a capability that ticket 16 freezes.
- C: two namespaces. The default test is confined. A separately named test
  takes raw paths. This is the chosen option.
- Confined namespace only (AuditSimplicity 11). It has the smaller surface. The
  user kept C, because absolute-path inspection is a frozen capability.
- Retain: absolute paths accepted in one test (`src/template/engine/path.rs:161-170`).
  This is the standing leak.

Rule:

- The default inspection tests are confined to the root. `DryRun` resolves the
  output with the writer rule, including the configured output-directory exception.
  Confined inspection and output use the same `ProjectRoot` confinement method.
- A separately named raw-path test covers external tools. Name the two tests so
  that a user cannot confuse "confined" with "raw".
- All vault file access stays inside the root, except for two named cases.
  The raw-path test reads external state and writes nothing.
  The configured output directory is the second exception.
- The change row 21 records the change. The capability stays.
- The default output-directory join stays. `TemplateWriteTarget::trusted` can
  keep an absolute path or a path outside the root
  (`src/template/writer.rs:328-342`) [V]. The user endorsed this named exception.
  Local config values need local trust. Global config is not trust-gated
  (ticket 22:187-198) [V]. The builder can inherit `output_dir` from global config
  (`src/config/builder.rs:96-109`) [V]. This Answer changes no trust policy.
- A runtime `-o` or `file.write_to()` value stays confined through `ProjectRoot`.
  `DryRun` and the prompt show the resolved path.
  A test must show that an untrusted local config never reaches the join.
- The template I/O seam owns this admitted-config exception. Do not expose
  a generic public unchecked join. Runtime and prompt path text never uses it.
- A confined inspection can select `ProjectRoot` itself without constructing
  an empty `RelativePath`. A normal file output needs a named path.

Invariant owner: `ProjectRoot` for the confined rule. Ticket 31 owns the
template adapter that exposes the names.

Code made removable: the absolute-path branch in the default inspection test
(`src/template/engine/path.rs:161-170`), which moves to the raw-path test.

Unresolved:

- The spelling of the two test names (ticket 31).
- The first-round proposal that the raw test takes an absolute path only, and
  that a relative input is an error, is a proposal [P]. The grilling did not
  decide it.

#### D10. Symlinked `.traces` (Question bullets 1 and 4; alias scenario)

Alternatives:

- Allow a symlinked `.traces` outside the root, with root-keyed trust as the
  guard (first-round proposal). This needs a new confinement exception.
- Retain the current containment check. It permits an in-root target [V].
- Reject every symlinked `.traces`. The review rejects this alternative because
  it removes a contained capability without solving physical metadata aliases.

Rule:

- Keep a symlinked `.traces` when its resolved directory stays strictly inside
  the admitted root. The maintainer confirms this rule after the review.
- Reject outside-root, dangling, cyclic, non-directory and unverifiable targets
  with typed errors. Reject a metadata target equal to the vault root.
- Never remove the target as a repair action.
- Exclude the resolved metadata directory by identity and subtree (D6).
  Blanket symlink rejection would not stop the physical `.traceſ` alias.
- This decision adds no exception for a symlinked `.traces`. D9 names the two
  template exceptions.
- Finding F11
  ([`o-stress-test-findings.md`](../research/22-prep/o-stress-test-findings.md))
  goes to ticket 33 with a named owner.
- Two vaults that share one `.traces` through a symlink: root-keyed trust keeps
  the grants separate (ticket 22 [V]). The store is a fork (D3).

Invariant owner: `ProjectRoot` (confinement). Ticket 33 owns F11.

Code made removable: none. Row 22 records the retained contained capability
and the metadata self-index repair.

Unresolved: ticket 33 wires F11 under this settled confinement policy.

#### D11. `DirTree` and `file_tracker` (settle 7; Question bullet 7)

Alternatives:

- Give the state-home stores the `RelativePath` type. They hold absolute
  machine-local paths, so a vault-relative type does not fit.
- Keep raw `PathBuf` in the state-home stores, and use `RelativePath` for vault
  paths. This is the chosen option.

Rule:

- Vault paths use `RelativePath`.
- State-home stores (trust and tracker) keep raw `PathBuf`. The vault invariant
  does not cover them.
- `DirTree` walks the vault and yields OS paths (`src/dirtree.rs:71`). The scan
  converts each entry once through `ProjectRoot::relativize` (D7). Code above
  that boundary sees `RelativePath` only.
- The tracker canonicalizes and hashes paths (`src/file_tracker.rs:469-483`).
  Its `contains` check uses `try_exists` only, and it misses a symlink swap
  (`src/file_tracker.rs:117,149,362`). Ticket 22 owns this gap. The decision is
  to document it.
- On Unix, the tracker stores the target as a symlink. On Windows, it writes
  `as_encoded_bytes` to a file (`src/file_tracker.rs:76-79`). This has the same
  std-docs issue as the store codec. The risk is low, because the state is
  machine-local [H].

Invariant owner: ticket 22 for the state-home stores. `ProjectRoot` for vault
paths.

Code made removable: none.

Unresolved: the `try_exists` symlink gap (ticket 22).

### What the grilling must settle: where each item is answered

| Item | Answer |
| --- | --- |
| 1. One owner for root confinement, with the named type | D1: `ProjectRoot`. No capability handle. D2: `RelativePath` is the pure path type. |
| 2. TOCTOU stance per boundary | D8: accepted and documented for note reads, template output and inspection. |
| 3. Template inspection and `DryRun` | D9: a confined default and a raw-path test. `DryRun` retains the configured output-directory exception. |
| 4. Persisted form, separator, case and Unicode rule | D3: UTF-8 with `/`. D5: listed spelling and a fold map. D6 and D7: admission and non-UTF-8 names. |
| 5. Portability of store and journal, with change rows | D3: the platform family leaves the identity. The canonical root stays. Rows 2 and 3. |
| 6. Sort order of the persisted form | D4: the NUL-form key. |
| 7. `DirTree` and `file_tracker` | D11: state-home stores keep raw `PathBuf`. |

### Edge and transition scenarios

| Scenario | Rule |
| --- | --- |
| Case-only rename (`notes/A.md` to `notes/a.md`) | The key is the listed spelling. A flip is a delete of the old key plus a create of the new key. The racy rescan catches it. The journal facts under the old spelling delete the old key. The new key reparses. That is a path-set change: about 499 ms and 395 MB at 50,000 notes [V ticket 24]. Queries still hit through the fold map. Until the next scan, a holder of the old key still sees `exists()` succeed, and the order can differ [H]. This is a documented limit. |
| NFC and NFD pair | Keys keep the listed spelling. The fold map answers other spellings. A unique match is a rewrite. Several matches are a union. Traces writes NFC. On ext4 and NTFS the two names are two files. On default APFS they are one file [H]. |
| Non-UTF-8 Unix name | Skip with a notice in quoted octal (D7). |
| Windows keys after a move | Portable path values remain UTF-8. A changed canonical root rebuilds recognized derived state. Native Windows conversion must preserve component meaning or return a typed error. |
| Ignorable codepoints (ZWNJ, U+202A to U+202E, BOM) | The scan accepts and indexes the names as today. They are distinct files to the file system. The fold map is a lookup aid only. |
| 8.3 short names and device names | The scan accepts and indexes them as today (D6). |
| Symlinked `.traces` | Permit a strictly in-root directory target. Reject outside-root or unverifiable targets. Exclude metadata by filesystem identity. Ticket 33 wires F11. |
| TOCTOU swap | Accept note-read and inspection races, including the final component. With a stable parent, template publication does not follow a pre-existing final destination symlink. Parent races remain accepted. |
| Template inspection of an external tool path | Two namespaces (D9). |
| Journal on a copied or moved vault | A different canonical root invalidates the old-root cache and journal under existing epoch rules. The old `mv` replay result is prototype evidence only. Never transfer trust grants. |
| Path-set change during replay | Replay writes facts only for files that it recorded (D3). |

### Standing obligations: how this Answer meets each

| Obligation | How this Answer meets it |
| --- | --- |
| 1. Admission identity (tickets 20 and 21) | This Answer revises no admission gate. It fixes one scan predicate: `.traces` matches without case (D6). |
| 2. Confinement boundaries (tickets 08, 11, 13, 20 and 21) | `ProjectRoot` confines notes, queries, schemas and template paths. The raw-path inspection test and configured output-directory join are named exceptions (D9). Local trust and global config obey ticket 22. The output boundary stays explicit and prompted. |
| 3. Compat envelope (ticket 16) | The change table below lists each change. The `.redb` layout is Free. The trust gate behavior stays frozen. The order of stdout stays frozen. |
| 4. Error topology | `fold_confinement` keeps the escape and unverifiable split (`src/path.rs:316-333`). `RelativePath` is pure. `ProjectRoot` does the I/O. |
| 5. Glossary conflict | Correct `src/GLOSSARY.md:20`. Add a pure "Relative Path" entry (see "Resolution and handoffs") [P]. |

### Adversarial review and required corrections

Two red-team rounds and three audits attacked the first-round recommendation.
The first round is [`h2-redteam.md`](../research/32-prep/h2-redteam.md). The
second round and the audits are in
[`h3-redteam-round2.md`](../research/32-prep/h3-redteam-round2.md). That file
has the full register of 17 reversals, the probe tables and the audit verdicts.

Reversals of the first-round recommendation:

| First-round item | Final decision |
| --- | --- |
| Plain byte order for the key | The NUL-form key. Plain byte order moves 8 of 13 rows in the probe vault. No change row for order. |
| Folder range `[prefix, prefix0)` | `[prefix\0, prefix\x01)`, with a boundary test. |
| Admission by character class (Q13) | No rejection by class. The scan matches `.traces` only. Nine names that the binary indexes today stay indexed. |
| A skeleton check against `.traces`, ignored dirs, `TRACES~N` and `GIT~N` | Dropped. APFS does not ignore those codepoints. |
| Case-insensitive match of all ignored dirs (v3) | `.traces` only. `Target/` and `Node_Modules/` keep exact case on Linux. |
| Reject every symlinked `.traces` | Retain strictly in-root targets. Reject external or unverifiable targets. Identity-based metadata exclusion repairs self-indexing. |
| Validate template output a second time at open | Dropped. A second path check cannot close concurrent mutation races. |
| "Never fold at read" | A fold map on a miss. The key never changes. |
| Normalize at every comparison site | None. Traces writes NFC only for files that it creates. |
| Keep the first-seen spelling and print a notice on a flip | The key is the listed spelling. A flip is a delete plus a create. No notice. |
| Non-UTF-8 names: open | Skip with a notice in quoted octal. |
| Reserve U+FDD0 for a later escape | Dropped. A later epoch bump is free before release. |
| A `stat_semantics` tag in `META` | Dropped. The platform family leaves the identity. The canonical root stays. |
| Ban `Path::cmp` with clippy `disallowed-methods` | A type rule. Clippy cannot name the method. |
| Sort costs 4.5 ms against 31.6 ms | About 10 ms for the string comparator. 4.5 ms for stored keys. |
| A new name such as `RelPath` | Reuse `RelativePath`, with no `Borrow<str>` and no `Deref`. |

Audit proposals that the user did not adopt:

- Drop the fold map and the dependency (AuditSimplicity 7).
- Ship only the confined inspection namespace (AuditSimplicity 11).

Findings that this Answer fixes:

- The live `.Traces` self-index bug (D6).
- The `RelativePath::parse` rules are silent on NUL, `\` and empty components
  (D2).
- The IR cannot express path-literal atoms (D5).
- The platform-family removal contradicted tickets 24, 25, 26 and 27 with no
  recorded rows (see "Resolution and handoffs").
- A row without `ctime` on Unix has no rule (D3).
- A lint cannot express the `Path::cmp` ban (D4).
- Template inspection and queries disagreed on what exists (D5 and D9).

### Resolution and handoffs

The Answer records the design and its final-review corrections. The handoffs
keep the remaining work explicit.

Invariant owners:

- `ProjectRoot`: confinement I/O and the TOCTOU stance.
- `RelativePath`: lexical validity, the sort order and the UTF-8 rule.
- The Comparison Key function: the fold. It is a lookup aid only.
- Store port (ticket 24): key reads and writes in the form of this ticket.
- Ticket 25: store identity, format epoch, extractor fingerprint and the
  `.traces` scan test.
- Tickets 26 and 27: journal cost and replay. This ticket owns only the journal
  path form.

Supersession duties. This Answer records the rows. It does not rewrite the
decisions of the other tickets:

| Ticket and line | Text today | Duty |
| --- | --- | --- |
| 24 rule 3 (`:299`) | Store identity records the engine tag, platform family, format epoch and two fingerprints. | The platform family leaves the identity. The canonical root stays. |
| 24 (`:311`) | `meta` keeps the platform family. | Change the `meta` field list in the same way. |
| 24 (`:392-395`) | The release order check uses `OsStr` comparison. | The NUL-form key check supersedes it. |
| 25 "Store identity" (`:299-300`) | Unix and Windows write different path bytes. A store does not move between families. | Remove the platform family. A changed canonical root invalidates the store and journal (D3). |
| 25 (`:355-357`) | A different platform family is a rebuild. | Remove the platform family from the rebuild rule. Keep the canonical root in the identity. |
| 25 (`:359-362`) | A higher epoch parses fresh and does not wipe. | Keep this rule. It also covers a store that a newer binary wrote. |
| 25 (`:312-317`) | The extractor fingerprint hashes crate versions and probe records. | List the normalization dependency. The epoch bumps with fold-based link resolution. |
| 25 (new rules) | None. | A stored row without `ctime` that a Unix reader finds is stale. The scan test covers `.Traces`, the physical `.traceſ` alias and an admitted in-root metadata target. The scan excludes the metadata directory by filesystem identity (D6). |
| 26 (`:251`) | The journal holds the platform family in the base identity. | Drop the platform family. Journal fact paths use canonical UTF-8 `/` text, not NUL-encoded lookup keys. |
| 27 (`:306`, `:313-316`) | The identity tests include the platform family. The probe has `rejects_mutated_platform` (`research/27-probe/replay/tests/fault.rs:842`). A copy is not rejected only because its path changed. | Drop the platform test. A changed canonical root rebuilds recognized derived state and discards the old-root journal. The `mv` replay result stays prototype evidence (D3). |
| 22 (`:182`) | Core holds no workspace object. | D1 states that `ProjectRoot` is, or contains, the `WorkspaceIdentity`. |

Duties for other tickets:

- Ticket 24: shares `Note` and `inlinks` through `Arc` in `FileEntry`, and owns
  the owned `LinkResolver`. These come from tickets 28 and 29. The path type
  owns key bytes only. The row type belongs to ticket 29.
- Ticket 25: see the table above.
- Ticket 28: `ProjectRoot` maps the client URI once to a `RelativePath`. It
  looks up the listed spelling in the base. An outside-root, non-UTF-8 or
  admission-skipped document gets diagnostics only. The overlay never uses the
  fold key.
- Ticket 29: the order statement of D4, the `PathAtom` split of D5, and rows
  that own `Arc<FileEntry>`.
- Ticket 31: the spelling of the two inspection names (D9).
- Ticket 33: F11 (D10), the `normalize_source_input` caller (D5), the shared
  alias and TOCTOU policy, and `WorkspaceIdentity`. Ticket 33 gains the blockers
  28 and 32.
- Ticket 34: the notice for a skipped name (quoted octal), the fold-map union
  (no notice), the lint code for Windows device names, and the dedup key.
- Ticket 35: the visibility of `RelativePath`, the public API baselines and
  the type-based enforcement of the byte order. It also picks the bench home.
- Ticket 36: the Windows CI property test and the durable benches.

Glossary duties:

- `src/GLOSSARY.md:20`: correct "Avoid: relative path". Add a pure "Relative
  Path" entry beside "Root-Confined Path".
- `src/GLOSSARY.md:63-68`: replace or delete the Path Codec entry.
- `src/GLOSSARY.md:107-115`: extend the Comparison Key. Add a Canonical Key and
  a NUL-form sort key, so that the three "key" terms stay distinct.
- `src/GLOSSARY.md:98-105`: Ambiguity (PathTie, never a diagnostic) stays
  consistent with the union rule.
- Add an Avoid line for the DSL `ctime`, the birth-time alias
  (`src/query/grammar/field.rs:205`). Ticket 25 `ctime` is the inode-change
  time.

Map duties (done on 2026-11-16 in `.scratch/arch-design/map.md`):

- Add a "Decisions so far" line for ticket 32.
- Lines 438-441 name the platform family in the identity. Update them.
- Graduate the fog item "Persistence portability" (about line 520).
- Record the blocked-by repairs:
  - Ticket 30: 14, 19, 21, 22 and 29.
  - Ticket 31: 7, 16, 18, 28, 29, 30 and 32.
  - Ticket 33: add 29 to its existing blockers.
  - Ticket 34: 8, 12, 13, 20, 28, 29, 31 and 32.
  - Ticket 35: 8, 12, 16, 22, 33 and 34.
  - The ticket 34 header previously listed ticket 34 itself.

### Change table (ticket 16)

Row numbers follow the list that tickets 28, 29 and 32 share. Row 5 is removed:
only `.traces` matches without case, and row 4 covers it. Rows 9 to 17 and 20
belong to tickets 28 and 29.

| # | Change | Category | Note |
| --- | --- | --- | --- |
| 1 | New `RelativePath`. `SafeRelativePath` is removed. `path_codec` and the `FileMeta` and `Note` path types change. | Free | Ticket 35 decides visibility. |
| 2 | NUL-form database keys and slash-separated path values. New epoch. The platform family leaves identity; the canonical root stays. Root relocation rebuilds derived state. | Free | Supersedes ticket 27's path-change replay exception. Higher-epoch rules stay unchanged. |
| 3 | Non-UTF-8 names are skipped with a stderr notice. They were indexed and listed lossily. | Migratable | Changes stdout rows. Invert the tests. |
| 4 | Metadata aliases, including `.Traces` and physical `.traceſ`, leave the scan through identity-based exclusion. | Free | Defect repair. Other ignored directories keep exact names. |
| 6 | Fold-based path-literal lookup in `--from`, and fold-based link resolution. | Migratable | Can change `file.inlinks` output. Extractor fingerprint and epoch. |
| 7 | Backslash link targets stop resolving on Windows [H]. | Migratable | Windows only. |
| 8 | `normalize_source_input` moves from file system probing to the index fold map. | Migratable | |
| 18 | `ConfigSubDir` and `TemplatePathInput` normalize `./` and reject NUL. | Free | |
| 19 | New dependency for NFC normalization. The `release` profile is `opt-level = "z"`. | Free | Recheck the sort timings. |
| 21 | Template inspection: a confined default plus a separately named raw-path test. `DryRun` retains the configured output-directory exception. | Migratable | The inspection capability stays. Local trust and global config obey ticket 22. |
| 22 | Keep contained `.traces` directory symlinks. Reject external or unverifiable targets. Exclude the metadata target by identity. | Free | The capability stays. The exclusion repairs a metadata self-index defect. F11 goes to ticket 33. |

No change row is needed for stdout order. The NUL-form key keeps today's order.

### Evidence limits for this Answer

- Only macOS APFS has path-behavior probes [V]. Linux has checks only for
  non-UTF-8 listing and for ext4 case and normalization distinctness (podman,
  release build).
- No Windows or HFS+ data exists. The Windows `Path::cmp` order is not checked
  [H].
- The `icu_normalizer` cost used a corpus with 10% non-ASCII text. A CJK,
  Indic or accent-heavy vault pays more. At 3x, the map still costs about
  12 ms [H].
- One auditor measured the sort at opt-level 3 and opt-level z. The red team
  that measured 31.6 ms and 4.5 ms did not. Recheck under the shipped profile.
- The mtime granularity facts (ext4, NTFS 100 ns, HFS+ 1 s, FAT 2 s) are [H].
- The `.Traces` fix is a plan. One run reproduced the bug.
- The `.traceſ` alias result comes from one APFS probe with an existing debug
  binary [V per Stress32]. No Linux, Windows or HFS+ result exists.
- The 499 ms and 395 MB figures at 50,000 notes come from ticket 24 [V].
- The NUL-form check cost at build and load is not measured.
- Non-UTF-8 prevalence in note vaults is unknown. The IWE trace is code
  reading, not a run. The Obsidian behavior is closed source [H].
- Cost numbers in this ticket are magnitude evidence, not contracts.

### Code delta

The code does not implement this ticket. This table lists what the refactor
changes. The Today column records the working tree on 2026-11-15 [V].
The Target column records the ticket decisions [P].

| Area | Today (path:line) | Target (ticket decision) | Depends on |
| --- | --- | --- | --- |
| Confinement owner | `SafeRelativePath` in `src/path.rs:159-190`: I/O in a path type, plain join at `:190`, TOCTOU comment at `:159-161`. Five production calls (D1) [V]. | `ProjectRoot` confinement method. Delete `SafeRelativePath`. | Ticket 33, ticket 35 |
| Relative path type | `RelativePath(PathBuf)` in `src/path.rs:64`. It allows `.` components. `HasPath::path() -> &Path` in `src/path.rs:57`. | A `/`-joined UTF-8 `String` with `as_str()`. No public zero-cost `as_path()` promise. `ProjectRoot` converts and validates native components, with typed errors. `HasPath::path() -> &RelativePath`. | Ticket 35 |
| Parse rules | `ConfigSubDir` in `src/config/model.rs:278`. `TemplatePathInput` in `src/template/path.rs:38`. | `RelativePath::parse` normalizes `./`. It rejects `..`, NUL and empty components. | None |
| Persisted path | Codecs in `src/path.rs:340-437`. `PathKey::as_bytes` in `src/index/codec.rs:74`. `path_from_bytes` in `src/index/codec.rs:89`. Re-export in `src/lib.rs:192`. | Canonical UTF-8 `/` path values. The store adapter owns NUL-encoded lookup keys. Delete the codecs and the re-export. New epoch. | Ticket 25 |
| Store identity | Platform family in `META` (`24:299`, `24:311`, `25:299-300`, `25:355-357`). | Remove the platform family. Keep the canonical root. | Ticket 25 |
| Order and comparison | `Path::cmp` at `delta.rs:31,175`, `refresh.rs:280,478,575`, `inlinks.rs:88,402`, `entry.rs:90`, `service.rs:442`, `store.rs:1080,1168`, `query/service.rs:364,392,417`, and `index/sort.rs`. | `RelativePath::Ord` in NUL form. No `Path` in a sorted structure. | Ticket 29 |
| Release order check | Debug-only assert in `SortedByPath::assumed_sorted` (`src/index/sort.rs:48-54`). | Check the NUL-form order at index build and store load, in release builds too. | Ticket 24 |
| Folder range | `collect_folder_paths` in `src/index/store.rs:1090`. Lossy prefix at `:1095`. Byte range at `:1123-1154`. Result sort at `:1168`. | `[prefix\0, prefix\x01)` with a boundary test. | Ticket 24 |
| Admission | `is_ignored_dir` in `src/env_vars.rs:19-44`. Callers at `src/index/service.rs:427` and `src/config/discovery.rs:163`. Dead check at `src/index/service.rs:430-435`. | Match `.traces` without case through the Comparison Key. Other dirs keep exact case. | Ticket 25 |
| Non-UTF-8 names | Scan at `src/index/service.rs:424-461`. Byte-exact tests at `src/index/codec.rs:83-94`, `service.rs:888,1300`, `store.rs:2342,2750,2772,2812` and `benches/index_codec.rs:67`. | Skip with a notice. Invert the tests. | Ticket 34 |
| Fold map and links | No map. `LinkResolver::get_by_path` is an exact search (`inlinks.rs:400-406`). `normalize_source_input` probes the file system (`src/cli/mod.rs:440-472`). | A Comparison Key map for each pinned generation. Fold-based link resolution. The extractor fingerprint lists the dependency. | Tickets 25, 29, 33 |
| IR path atoms | `SourceAtom::Path(GlobPattern)` in `src/query/grammar/source.rs:190`, `:250-290`. | `PathAtom::{Literal, Folder, Glob}`. | Ticket 29 |
| Dependencies and profile | No NFC crate in `Cargo.lock`. `opt-level = "z"` in `Cargo.toml:269-281`. | Add `icu_normalizer` 2.4. Recheck the sort timings. | None |
| TOCTOU | `commit_with_writer` in `src/template/writer.rs:366-407`. `rustix` in `Cargo.toml:164`. | No change. Document the window. Keep the `openat` option. | None |
| Template inspection | Absolute paths accepted in `src/template/engine/path.rs:161-170`. `fs::metadata` at `:118`. Confined writer rule at `src/template/writer.rs:303-323`. Trusted output join at `:328-342`. | A confined default and a separately named raw-path test. `DryRun` retains the configured output-directory exception. | Ticket 31 |
| Traversal and trackers | `DirTree` in `src/dirtree.rs:71`. Tracker in `src/file_tracker.rs:76-79,117,149,362,469-483`. | The scan converts once through `ProjectRoot::relativize`. State-home stores keep raw `PathBuf`. | Ticket 22 |
| Glossary | `src/GLOSSARY.md:20`, `:63-68`, `:98-105`, `:107-115`. | The edits in "Resolution and handoffs". | None |
| Public API baselines | `pub-tokens.txt`. `public-api-default.txt` (`FileMeta::path` :1392, `Note::path` :1743, `MarkdownParserInput::new` :1687). `public-api-test-utils.txt:280-282`. | Regenerate. The three `testing::path_codec` lines disappear. | Ticket 35 |
| Journal | Path form open (`issues/27-index-replay-prototype-and-fault-tests.md:451-452`). Identity text at `26:251` and `27:306`. | Journal fact paths use canonical UTF-8 `/` text, not NUL-encoded lookup keys. Drop the platform test `rejects_mutated_platform`. | Tickets 26, 27 |

Not in code:

- The `ProjectRoot` type and the `WorkspaceIdentity` type. Neither type exists
  in `src/`.
- The fold map, the NFC dependency and the NUL-form `Ord`.
- A test that builds a `RelativePath` from a name with a literal NUL, an empty
  component or a lone `\`.
- The property test that `Ord`, `Eq` and `Hash` agree.
- A Windows CI case-insensitive scan test for `.Traces`.
- A test with two sibling files whose names are an NFC and NFD pair.
- A Linux test with `Foo.md` and `foo.md` side by side.

Remaining handoffs and open choices:

- "Decide how `FileMeta`, index rows, query paths, template paths, and CLI errors
  carry confined versus raw paths." The Answer fixes the form for each type
  except the CLI error text. Ticket 34 owns that text.
- "The journal name and its path form stay open." Journal fact paths use
  canonical UTF-8 `/` text. The journal name is not a path-form question.
  This ticket does not decide the journal name.
- The names of the two template inspection tests.
- Native Windows identity and component conformance checks.
- Keep `strict-path` until replacement conformance succeeds.
- D9 preserves local trust and ticket 22's separate global-config policy.

### Final review amendments (2026-11-16)

The maintainer confirms native conversion in `ProjectRoot`, identity-based
metadata exclusion, contained metadata links and rebuild after root relocation.
These decisions replace the earlier all-symlink rejection and unconditional
`mv` replay claims.

The [conventions review](../research/32-prep/i-conventions-review.md) compares
Git metadata pointers, Obsidian symlink advice and Cargo dependency paths.
It does not establish one universal convention or cross-platform runtime proof.

Keep quiet cache healing quiet. Record root-change causes in the refresh report
or existing logs. Rebuilding derived state never authorizes deleting user files.
Trust at a new root needs independent admission, even if future rebinding
reuses facts.
