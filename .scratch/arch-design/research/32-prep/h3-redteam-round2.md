# Second red team and three audits: ticket 32 under attack

Goal: record what the second red team and the three audits found against the
ticket 32 decision set. This file condenses four source reports:
`agent://RedTeam32`, `agent://AuditSimplicity`, `agent://AuditUxConventions`
and `agent://AuditImplementability`. It keeps only the ticket 32 parts. It
adds no new claim. The first red team is `h2-redteam.md`. The shared probes are
in `g1-probes.md`. Both files stay unchanged.

Marks: [V] means the source report read the line or measured the value.
[H] means a hypothesis, an inference or a figure that the source report did not
re-measure. [P] means a proposal.

The source reports contain historical proposals. Only the final-decision
statements record the grilling result. The ticket 32 Answer is authoritative.
Upstream source-line references are unpinned source-report citations.
This copy edit does not recheck those upstream lines [H].
The source tallies of 8 call sites and 63 hits in 6 files need correction [H].
The checked tree has 5 production calls to `SafeRelativePath::parse` and 6 test
calls [V]. `RelativePath` has 32 whole-word uses in 6 files [V].
The source tally of 31 `SafeRelativePath` hits spans 6 files, not 5 [V].

Machines: macOS arm64 with APFS. Linux ran in a podman VM on ext4 and overlay,
with a release build of HEAD `fe0cae9d`. The auditors used the release binary
of 9 November 2026 in a temp vault. No Windows or HFS+ data exists.

The report `agent://RedTeam32` gives six verdicts: W32a to W32f. It has no
verdict W32g. The ranked list in section 10 carries its extra findings.

## 1. Register of reversals of the first-round recommendation

The first-round recommendation is the proposal set of `z-grilling-guide.md`
and `h2-redteam.md`. The table lists each item that a later report or the user
reversed. The last column gives the final decision of the grilling session.

| # | First-round item | Reversed by | Final decision |
| --- | --- | --- | --- |
| 1 | The key sorts by plain byte comparison (guide, section "Sort order and portability"). | W32d (section 7). Plain byte order moves 8 of 13 rows in `list`, `table` and `task`. | The key is the path with `/` replaced by byte 0x00, compared as bytes. Stdout order stays. No change row for order. |
| 2 | The folder range is `[prefix, prefix0)` with the last byte incremented. | The NUL form changes the delimiter. | The folder range is `[prefix\0, prefix\x01)`. A test pins the end-bound rule. |
| 3 | Admission rejects ignorable codepoints, Windows device names and 8.3 names (guide, "Rejected names"; the edge table). | W32c (section 4): the premise is partly wrong, and today the binary indexes nine such names. | No rejection by character class. The scan matches `.traces` only, and it ignores case. |
| 4 | A skeleton check (HFS+ ignorables, trailing dot or space, `:stream`, 8.3) against `.traces`, the ignored dirs, `TRACES~N` and `GIT~N` (the W32c proposal and v2). | AuditSimplicity 8; the APFS probe; user override 4. | Dropped. Admission uses the Comparison Key for `.traces` only. Other ignored dirs keep exact case. |
| 5 | Admission matches `.traces` and all ignored dirs in a case-insensitive way, with an ASCII fold plus NFC (v3). | AuditImplementability B5, B6 and defect 7. | `.traces` only, with the Comparison Key function. `Target/` and `Node_Modules/` stay exact on Linux. |
| 6 | Allow a symlinked `.traces`, with root-keyed trust as the guard (guide, edge table). | W32a tested an outside-root target only. | The recorded rule rejects a symlinked `.traces` [P]. An in-root target succeeds today. Section 15 corrects the "no change" premise. |
| 7 | Validate template output at admission and again at open (guide, "TOCTOU stance"). | W32e (section 8): the final component is already safe. | The second validation at open is dropped. The window in intermediate directories is accepted and documented. |
| 8 | "Never fold at read. NFC only on new writes." (guide, "Unicode"). | W32a (section 6): a query in another spelling finds 0 rows. | Exact listed bytes first. On a miss, a Comparison Key map gives the listed spelling. Keys never change. |
| 9 | Normalize at every comparison site (`h2-redteam.md`, attack 2 against the portable form). | The W32a rule: the key keeps the listed spelling, and a lookup map answers other spellings. | No normalization at reconcile. Files that Traces creates are written NFC. |
| 10 | Keep the first-seen spelling and emit a notice on a case flip (`h2-redteam.md`, attack 2 against byte-order ranges). | The final edge rule. | The key is the listed spelling. A flip is a delete plus a create. No notice. The racy rescan catches it. |
| 11 | Non-UTF-8 names: open between skip and OS bytes (guide). | Both audits, W32b, the NonUtf8Precedent report. | Skip with a notice in git-style quoted octal. No reserved U+FDD0. |
| 12 | Reserve U+FDD0 at admission so that an escape can come later (W32b proposal; AuditUxConventions). | AuditSimplicity 9; v3. | Dropped. A later escape costs one epoch bump, and the bump is free before release. |
| 13 | A `stat_semantics` tag in `META` outside the wipe identity (W32f proposal; v2). | AuditSimplicity 4; v3. | Dropped. The platform family leaves the wipe identity. The canonical root stays. |
| 14 | The ban on `Path::cmp` uses clippy `disallowed-methods` (v3). | AuditImplementability section (a)2 and defect 4. A scratch crate showed that clippy cannot name the method. | Types enforce the ban. No `PathBuf` or `Path` appears in a sorted structure. |
| 15 | The sort costs 4.5 ms against 31.6 ms at 50k (v2). | AuditUxConventions section "Re-measured". | About 10 ms for a string comparator, 4.5 ms for stored keys. Recheck under the shipped `opt-level = "z"`. |
| 16 | A new name such as `RelPath` (guide option B; W32 weakness 4). | W32 weakness 4: the name `RelativePath` exists with 63 hits in 6 files. | Reuse `RelativePath` with new rules. |
| 17 | The v2 text did not forbid `Borrow<str>` or `Deref` on `RelativePath`. | AuditUxConventions contradiction 2. | No `Borrow<str>` and no `Deref<Target = str>`. The later Windows correction removes the public zero-cost `as_path()` promise (section 15). |

Audit proposals that the user did not adopt:

- AuditSimplicity 7: drop the comparison-key map and the `icu_normalizer`
  dependency. The final decision keeps the map. AuditUxConventions marked the
  same item KEEP.
- AuditSimplicity 11: ship the confined inspection namespace only. The final
  decision keeps two namespaces and names them so that a user cannot confuse
  them.
- AuditSimplicity 2: the API break needs no change-table row. The final
  decision records row 1, because the public API baselines change
  (AuditImplementability section "Baselines").

## 2. Verdict summary for the second red team

| Item | Subject | Verdict |
| --- | --- | --- |
| W32a | Query spelling and case | Confirmed, with extra findings and probes. |
| W32b | Skip against escape for non-UTF-8 names | Partly confirmed. The "visible regression" premise is refuted for the CLI. |
| W32c | Overbreadth of admission by character class | Confirmed. The 8.3 premise is partly wrong. |
| W32d | Byte order against in-memory order | Confirmed, and wider than stated. |
| W32e | TOCTOU | Partly confirmed. The window is smaller than stated. |
| W32f | Platform family in the store identity | Partly confirmed. Removal is safe if the comparison rule changes. |

## 3. Live bug found: `.Traces` self-indexing [V, measured]

- `is_ignored_dir` is an exact, case-sensitive match (`env_vars.rs:43`).
- On a case-insensitive volume, a case variant of `.traces`, `.git` or
  `node_modules` defeats the prune. The index then holds its own store.
- The dead check `path != index_db` in `service.rs:430-435` compares strings,
  so it also misses a case variant.
- Measured on APFS: after `.traces` became `.Traces`, the scan indexed
  `.Traces/config.toml` and `.Traces/index.redb`. The store file changed size on
  every `list` (98304, then 94208, then 98304).
- Ticket 25 owns a test that nothing under `.traces/` enters the scan. The test
  must include case variants.
- The final fix is a plan. One run reproduced the bug.

## 4. W32c: admission by character class

What `gix-validate` rejects. The red team read `gix-validate/src/path.rs` on
main [V]:

| Check | What it does |
| --- | --- |
| `protect_hfs` (`is_dot_hfs`) | It strips the HFS+-ignorable codepoints. Then it compares only with `.git`, and with `.gitmodules` for symlinks. It does not reject a name that merely contains ZWNJ, ZWJ, LRM, RLM, U+202A to U+202E, U+206A to U+206F or U+FEFF. |
| `protect_ntfs` | It rejects 8.3 forms only for `git~1` and the `gi7eba` form of `.gitmodules`. It does not reject `file~1.md`. |
| `protect_windows` (on by default) | It rejects device names with an extension, such as `CON.md` and `aux.txt`. It also rejects `<>:"|?*`, control characters, and a trailing `.` or space. |

- gix guards a checkout of untrusted content into a worktree on every
  platform. Traces only reads names that already exist on disk.

Today's behavior [V, measured]. On the APFS volume, the current binary indexes
all nine of these names:

- Persian `می‌خواهم.md` (ZWNJ)
- Hindi `क्‍ष.md` (ZWJ)
- a `U+FEFF` BOM name
- a `U+202E` name
- `file~1.md`
- `CON.md`
- `aux.md`
- `a:b.md`
- `trail.` with a final space

Cost of the first-round rule. It would silently skip all nine names. That is a
ticket 16 regression. It also hits Persian and Indic names that need ZWNJ or
ZWJ for correct spelling.

The HFS+ ignorable-codepoint danger does not exist on APFS [V, measured]. The
name `.z‌z` is not `.zz`. The name `.z\ufeffz` is not `.zz` either. Only legacy
HFS+ ignores those characters.

Narrowest sound rule that the red team proposed [P]:

1. Never reject by character class.
2. Compute a skeleton of each name. Strip the HFS+-ignorable codepoints, strip
   a trailing `.` and space, strip a `:`-stream suffix, then apply NFC and a
   case fold.
3. Compare the skeleton only with the protected names. These are `.traces`, the
   other ignored directories, and the NTFS short forms `TRACES~N` and `GIT~N`.
   The scan treats a match as that protected name.
4. Apply the Windows-name check (`protect_windows`) only under `cfg(windows)`,
   or as an optional "portable output" lint at template output admission. Never
   apply it at scan on Unix or macOS.
5. The cost is a few name compares per entry, with a first-byte prefilter.

The final decision keeps only the first part of rule 3 for `.traces`. See
section 1, rows 3 to 5.

## 5. W32b: skip against escape for non-UTF-8 names

Where an entry name becomes a path string or a key [V, code]:

| Stage | Location | Form |
| --- | --- | --- |
| Walk | `DirTree::descendants` (walkdir) | `OsStr` path. `is_ignored_dir` uses `to_str()`, so a non-UTF-8 name is never ignored. |
| Scan | `service.rs:424-461`, `RelativePath::derive` | `strip_prefix` on `Path`. No UTF-8 requirement. |
| Row payload | `FileMeta` and `Note` path through `path::codec` | Unix raw bytes. Windows wide units. |
| Store key | `PathKey::as_bytes` (`codec.rs:74`) | `as_encoded_bytes()` |
| Key decode | `path_from_bytes` (`codec.rs:89`) | UTF-8, else lossy U+FFFD |
| Folder prefix | `store.rs:1095` | `to_string_lossy()` plus `MAIN_SEPARATOR` |
| Glob match | `source.rs:287` | `display_portable` (lossy) |
| Output | `results.rs:233`; `format.rs:242-257` | Lossy |
| LINKS | `collect_multimap_links` | Raw-byte map |
| Resolver | `inlinks.rs` | `target_parts` is `&str`, then `Path::new` |

- The std docs say that `as_encoded_bytes` is not safe to store in a file
  (std `os_str.rs:1049-1060`) [V]. The store keeps it today. On Windows those
  bytes are WTF-8 with `\` separators.

Linux measurement [V]. The setup was a rust:1.96 container and a release
build. The files were `bad\xffname.md`, `bad\xfenam.md`, `d\xff/in.md` and
eight valid names.

- `traces index` printed `indexed 11 file(s)`.
- `traces list` showed 8 pages. The three non-UTF-8 files are silently missing.
- `list --from '#t1'` returned only `normal.md`. Three files carry the tag.
- Cause: tag, class and folder sources return paths through the lossy
  `path_from_bytes`. Then `read_batch` runs `table.get(lossy_bytes)`, finds
  nothing and returns `Ok(None)` (`store.rs:1009-1016`). No notice appears.
- `--from` with a non-UTF-8 argument fails in clap ("invalid UTF-8 was
  detected").
- `A.md` and `a.md` are separate files on ext4. Both are listed. NFC `é.md` and
  NFD `é.md` are also separate and print the same.

Consequence. Skip with a notice is strictly better than today on the cold CLI
path. In-process `QueryService::run` (templates, host) probably lists the names
raw [H, not tested].

The escape that the red team proposed, if ever wanted [P]:

- Reserve the noncharacter U+FDD0 as the introducer. Replace each invalid byte
  `b` with `U+FDD0` plus two hex digits. Valid names pass through
  byte-identical.
- Use `<[u8]>::utf8_chunks()`. It runs only on names that fail `to_str()`.
- Admission rejects a real name that contains U+FDD0, with one notice. That
  makes the map injective.
- Decode lives in one place, `ProjectRoot::os_path(&RelPath)`. It scans for
  `EF B7 90` and rebuilds the bytes with `OsStringExt::from_vec`. Windows needs
  a 4-hex lone-surrogate variant.
- The key sorts as ordinary UTF-8 bytes. Display stays lossy.

Comparison. Skip costs about 5 lines plus a notice. The escape adds a codec, a
property test and a display decision. The escape also changes `list` output
with new rows. That is a bigger ticket 16 change than a stderr notice.

The red team proposed to ship skip with a notice and reserve U+FDD0 from day
one. The final decision ships skip and drops the reservation (section 1, rows
11 and 12).

Other findings [H]:

- Row payloads store the path twice (key plus payload). A new epoch could drop
  the payload copy.
- On Windows today, the key bytes contain `\`. So the range convention is moot
  without a new epoch.

## 6. W32a: query spelling and case

Probes on macOS APFS with the release binary [V per source report].
Later tests ran in temp vaults:

| Probe | Result |
| --- | --- |
| A vault with NFC `Notes/Café.md` and an NFD folder `Zürich/` | `index` stores the creation spelling: NFC `Notes` and NFD `Zu\u0308rich`. APFS readdir returns the creation spelling. |
| `list --from` with NFC `Zürich/` | 0 pages. The NFD spelling `Zu\u0308rich/` found 1. |
| `list --from` with `zürich/`, `ZÜRICH/`, `notes/` and `NOTES/` | All found 0. |
| `--where 'file.folder == "notes"'` | 0 |
| The same folder in the exact spelling | It hits. |
| `A.md` renamed to `a.md` | The next `list` already shows `a.md`, because the CLI rescans. `--from A.md` gives 0. |
| NFD create when NFC exists (and the reverse); `A.md` against `a.md` | One file on APFS. |
| Template `path_exists` and `is_dir_path` with NFC, NFD, `ZÜRICH` and `notes/Café.md` | All `True` (OS-level lookup). |
| `-o notes/new-nfc.md` | It landed in the existing `Notes/`. The spelling stayed. |
| `-o Zürich/b.md` (NFC) | It landed in the existing NFD folder. |
| A symlinked `.traces` (one shared dir, two vault roots) | `index` and `list` fail: `invalid config subdirectory path .traces/schemas/` and `path is outside the root directory`. |
| Two vaults with `index.redb` symlinked to one shared store | Each `list` corrects itself through a rescan. Each alternation rewrites the store (thrash). |
| `.traces` renamed to `.Traces` | Bug: the scan indexes `.Traces/config.toml` and `.Traces/index.redb`. See section 3. |

Findings from the probes:

- Template inspection (OS-level) and index queries (byte-exact) disagree on
  what exists.
- The first-round "allow symlinked `.traces`" is a new behavior. It needs a
  confinement exception for `.traces` itself.
- The current link resolver is exact. `get_by_path` is a `Path::cmp` binary
  search with no fold. So fold-based link matching is new behavior and needs a
  ticket 16 change-table row.
- On HFS+ or on some network volumes, the listing returns NFD for every name.
  NFC writes then list back as NFD. The "listed spelling" rule fits that
  case [H].

Rule that the red team proposed for queries (lookup, with no persisted
secondary index) [P]:

1. Match the exact listed bytes first. This keeps all of today's hits.
2. On a miss, look the query up in a comparison-key map (NFC, lowercase, NFC)
   that the code builds lazily at load. A unique match rewrites the query atom
   to the listed spelling during planning, before the residual filter exists.
3. Several listed spellings (an ext4 `A.md` and `a.md`) give an OR of the listed
   spellings, or an ambiguity notice. Never guess.
4. The fold lookup applies only to path-literal source atoms and link targets.
   String comparisons such as `file.folder == "x"` stay exact (ticket 19, no
   collation). Document this.
5. The key is a lookup aid, never an identity.

Reason against a persisted index: a persisted comparison-key table adds 50k
rows and a format epoch, with no gain.

Cost at 50,000 paths [V]. Setup: probe crate, `icu_normalizer` 2.4, 10%
non-ASCII, release build, 2 warmups, median of 11:

| Measure | 5k | 50k |
| --- | --- | --- |
| NFC check | 0.031 ms | 0.300 ms |
| Lowercase only | 0.152 ms | 1.458 ms |
| Full key (NFC, lower, NFC) | 0.262 ms | 2.527 ms |
| File-level key map build | 0.377 ms | 3.907 ms |
| Folder-only map (about 4.7k folders) | 0.316 ms | 1.583 ms |
| Folder plus ancestors map | 0.513 ms | 2.315 ms |
| 1,000 fold lookups | 0.195 ms | 0.193 ms |

- At 50k, the file map costs 3.9 ms against a full build of about 1.8 s
  (0.2%). The code builds it lazily, on an exact miss or when the link resolver
  exists. The folder map alone covers `--from folder/`.
- New dependency. `Cargo.lock` has no NFC crate. It has only `icu_collections`,
  `icu_provider` and `icu_segmenter`. `icu_normalizer` 2.4 and `caseless` are
  in the local registry cache.
- The final decision adds the single Comparison Key rule, with a covering map
  for files and folders. It rewrites only path-literal and folder atoms.
  Several matches give a union with no diagnostic.

## 7. W32d: byte order against in-memory order

Where the order comes from:

- The sort is `Path` order throughout.
- `SortedByPath` is built with `sorted()` or `assumed_sorted()`
  (`sort.rs:37-55`).
- `read_all` sorts again the rows that it read in redb byte order
  (`store.rs:469-477`).
- `LinkResolver::get_by_path` is a `Path::cmp` binary search
  (`inlinks.rs:400-406`).
- `assemble_internal` is a merge that uses `Path` `<` and `==`
  (`entry.rs:90-96`).

Comparator sites to change [V]: `delta.rs:31,175`; `refresh.rs:280,478,575`;
`inlinks.rs:402`; `entry.rs:90`; `service.rs:442`; `store.rs:1080,1168`;
`query/service.rs:364,392,417` (`sort`, `dedup`, `intersect_sorted`).
AuditImplementability adds `inlinks.rs:88` (`par_sort_unstable`) and
`index/sort.rs`.

The danger. If one site stays on `Path::cmp` while the data is byte-sorted, a
binary search silently misses and the merge drops rows. Release builds have no
check, only `debug_assert`.

Output order of the probed `traces list`:

| Order | Result |
| --- | --- |
| Today (`Path` order) | `a/b.md`, `a b.md`, `notes/a/b.md`, `notes/a.md`, `notes/note.md`, `notes-x.md`, `notes.md` |
| Plain byte order | `a b.md`, `a/b.md`, `notes-x.md`, `notes.md`, `notes/a.md`, `notes/a/b.md`, `notes/note.md` |

- `Path` order keeps the children of a folder together. Byte order interleaves
  them, because `-` (0x2D), `.` (0x2E) and space (0x20) sort below `/` (0x2F).
- The folder range `[prefix/, prefix0)` is sound. The current store already
  uses a byte range (`store.rs:1123-1154`) and sorts the result at `:1168` [V].
- The red team proposed `RelPath: Ord` with the byte order and a ban on
  `Path::cmp` in the index modules. The final decision uses the NUL-form key
  instead of plain bytes (section 1, row 1).

Measurements that the v2 decision text records [V per the v2 text, one machine]:

- 3M random pairs: 0 mismatches between the NUL-form key and `Path::cmp`.
  Plain UTF-8 byte compare mismatched on 73,891 pairs.
- In the probe vault, plain byte order moves 8 of 13 rows in `list`, `table`
  and `task`. Example: `notes/a.md` sorts before `notes-x.md` today.

## 8. W32e: TOCTOU

- The final component is already safe. `commit_with_writer`
  (`writer.rs:366-407`) writes a temp sibling with `create_new` (`O_EXCL`),
  which does not follow a final symlink. It publishes with `rename` for an
  overwrite or with `hard_link` for create-new. `rename` replaces the final
  name. It does not resolve a symlink there.
  `link()` fails with `EEXIST` on any existing name,
  including a dangling symlink. So an `O_NOFOLLOW` on the final component adds
  nothing.
- The remaining window is the intermediate directories. `create_dir_all(parent)`
  and the temp `create_new` follow a directory component that an attacker swaps
  for a symlink between the check and the use.
- The cheapest sound fix that the red team proposed [P]. It keeps the
  no-capability stance and applies inside `commit` only:
  1. Open the confined parent once, relative to a root fd.
  2. Create the temp file and publish with `openat`, `linkat` and `renameat`
     against the parent fd.
  3. Use `rustix::fs`. It is already in `Cargo.toml:164`.
  4. For the parent open, use `openat2(RESOLVE_BENEATH)` on Linux 5.6 and later.
     Fall back on `ENOSYS`.
  5. The macOS SDK defines `O_RESOLVE_BENEATH` (0x1000) and `O_NOFOLLOW_ANY`
     (0x20000000) [V, `sys/fcntl.h`]. The OS supports both. The red team did
     not wire them up.
  6. Do not use `NO_SYMLINKS` or `NOFOLLOW_ANY`. They break in-root symlinked
     directories that `strict-path` accepts today.
  7. Windows has no beneath primitive. It keeps the current path-based check [H:
     cap-std walks components with `FILE_FLAG_OPEN_REPARSE_POINT`].
- Verdict. A second path-based validation is not worth adding. The realistic
  attacker needs concurrent write access inside the vault. To accept and
  document the window is defensible.
- Template inspection. `std::fs::metadata` follows symlinks
  (`engine/path.rs:118`). With `strict_join` in front, an in-root symlink that
  points out is rejected. This is acceptable.
- Also measured: `Notes/../b.md` is rejected lexically, even though it stays
  inside the root. This is a ticket 16 behavior to keep.
- The final decision accepts and documents the window. The `openat` route is a
  recorded later option.

## 9. W32f: platform family in the store identity

What `ctime` means:

- On Unix it is the inode change time. It changes on write, `chmod`, rename and
  link. A copy gets a new value.
- On Windows stable Rust, `ctime` does not exist. `change_time` is unstable
  (`windows_change_time`, issue 121478, `std/os/windows/fs.rs:482`) [V]. So
  Windows compares size plus mtime, as ticket 25 says.

mtime granularity [H]:

| File system | Granularity |
| --- | --- |
| ext4 | ns, but updated at kernel-tick granularity |
| APFS | ns |
| NTFS | 100 ns |
| HFS+ | 1 s |
| FAT | 2 s |
| exFAT | 10 ms |

- The base-identity check does not validate sources. `META` holds the
  incarnation and the revision. They guard the writer transaction (ticket 25
  lines 309-310). The per-file stat triple and the journal source snapshot do
  the validation.
- Cross-family moves. A move to a different volume mostly gives a mismatch
  (different `ctime`, rounded `mtime`), so the files are read again. That is
  safe. The false-valid risk equals the weak rule of Windows itself: equal size
  and mtime with different content. A store moved from Windows to Unix has
  `ctime = None`. That value must compare unequal to `Some`, which forces a
  reparse.
- A real gap. `FileMeta.created_at` is birth time (`file.rs:71`). It differs by
  file system, and it feeds `cdate` query values. A reused row keeps the birth
  time of the old file system. A fresh parse does not. This breaks "stored row
  equals fresh parse".
- The red team proposed a 1-byte `stat_semantics` tag in `META`, outside the
  wipe identity [P]. A mismatch means that the rows are valid but unvalidated.
  Every file is read once. That costs about 395 ms at 50k, against about 1.8 s
  for a full build.
- Racy bit. The 2 s window (`mtime >= scan_start - 2 s`) is borderline on FAT.
  Clock skew on network volumes can defeat it. Add a margin there [H].
- Final decision. The tag is dropped (section 1, row 13). A row without `ctime`
  that a Unix reader finds is stale (AuditImplementability E-3).

## 10. More weaknesses, ranked by cost of being wrong

1. A mixed `Path` and byte order is silent in release (W32d). A wrong binary
   search gives missing links and dropped rows, with no error.
2. A case-variant `.Traces` self-indexes (section 3). It also affects `.Git` and
   `Node_Modules`. The fix needs a case-insensitive match and a ticket 25 test.
3. The ticket 16 change table lacks at least these rows:
   - a folder-order change (W32d);
   - fold-based link resolution (new behavior);
   - a new notice for skipped names;
   - `.traces` as a symlink (today an error);
   - names with ignorable codepoints, `~1`, device names or illegal characters.
4. `RelPath` conflicts with the existing `RelativePath` (`path.rs:64`, 63 hits
   in 6 files; `SafeRelativePath` has 31 hits in 5 files). It also conflicts
   with the glossary entry "Root-Confined Path … Avoid: relative path"
   (`GLOSSARY.md:18-20`).
   - Reuse the name `RelativePath`. Make it a `/`-joined, normalized `String`
     with byte `Ord`. Drop the allowance for `.` components.
   - Replace `SafeRelativePath` with a `ProjectRoot` confinement method.
   - Add `as_path()` as a zero-cost `Path::new(&str)` view. `Path::new` accepts
     `/` on Windows too. This keeps the 1,681 `.path()` call sites mostly
     untouched.
   - `FileMeta` and the `path_codec` re-export are public API (`lib.rs:192`), so
     the break is visible.
   - Fix the glossary: add a "Relative Path" entry (pure and lexical) beside
     "Root-Confined Path".
5. Two vault roots that share one `index.redb` thrash the store. The root must
   stay in the identity, so that a shared store acts like a `cp -R`. The red
   team did not check what identity holds today.
6. Journal path form. Use the store form. After a case-only rename, the old
   spelling facts in the journal delete the old key, and the new key reparses.
   That is a path-set change, about 499 ms at 50k [V, ticket 24]. It is slow
   and correct.
7. Spelling is unstable with a watcher. A holder of the key `A.md` still sees
   `exists()` succeed after an external case-only rename. Only the next scan
   corrects the spelling. This is benign and needs documentation [H].
8. Windows verbatim `\\?\` prefixes and path length. `strict_join` and
   `strip_prefix` may disagree when the root is canonical-verbatim and the
   walkdir paths are not. No NFC dependency exists yet. The red team did not
   probe either point on Windows [H].
9. `file_tracker` and trust stores. Raw `PathBuf` is right. On Unix the tracker
   stores the target as a symlink. On Windows it writes `as_encoded_bytes` to a
   file (`file_tracker.rs:76-79`). This is the same std-docs issue as above. The
   risk is low, because the state is machine-local.

## 11. Cleanup and limits of the second red team

- The red team removed the worktree `.worktrees/rt32`, the podman volumes and
  `/tmp/traces-probe-32`. The podman machine is stopped again.
- It ran `traces untrust` on every probe vault root.
- `traces tracked clean` printed "removed 40 stale tracked entries". The output
  does not separate probe entries from older stale ones. So the 40 can include
  entries from other work.
- The main tree has uncommitted changes under `src/template/` from other work.
  The red team did not touch them.
- Not verified: Windows behavior, HFS+ behavior, and in-process
  `QueryService::run` with a non-UTF-8 name. Those points carry [H].

## 12. Audit 1: AuditSimplicity (complexity and consistency lens)

The auditor read the v2 decision file and grepped tickets 16, 19 and 25. It ran
no measurement. Its marks: [V] means it read the line. [H] means an inference
from the v2 text.

Quoted ticket text:

- Ticket 16:89: "Template/query **capabilities** … Must stay reachable"
  (Frozen).
- Ticket 16:79: the index equals a fresh parse, except for the racy window.
- Ticket 25:208: one format epoch covers parser facts, serialization, facets
  and derived keys.

Per-decision verdicts for ticket 32:

| # | v2 decision | Verdict | Reason and cost |
| --- | --- | --- | --- |
| 1 | `ProjectRoot` owns confinement I/O. No capability handles. | KEEP | It is the smallest option [H]. |
| 2 | Reuse the name `RelativePath`. Drop `SafeRelativePath`. | SIMPLIFY | A pure `/`-joined `String` newtype is right. The API break needs a change row only if stdout shapes change. They do not change [H]. |
| 3 | Persisted UTF-8 `/` path. New epoch. Platform family leaves the wipe identity. | KEEP | A wipe costs nothing before release. Ticket 25:208 already has one epoch [V]. |
| 4 | A 1-byte `stat_semantics` tag outside the wipe identity. | DROP | It is a second invalidation mechanism next to the identity and epoch (25:46-54). It saves about 395 ms once. A shared index is a fork case that the same decision already calls unsupported. Retrofit cost: a `META` byte plus an epoch bump. |
| 5 | The journal path form equals the store form. | KEEP | It follows from decision 3 and adds no machinery [H]. |
| 6 | Sort key: `/` replaced by NUL, compared as bytes. | KEEP | It keeps stdout order, which is frozen [H]. Banning `Path::cmp` at about 14 sites is cheap. Simplify the text only: say "components compare bytewise". |
| 7 | Comparison-key map (NFC, lowercase, NFC) plus `icu_normalizer`. | DROP, with a REVISIT on a small slice | It adds a dependency, a lazy 3.9 ms map, OR-or-ambiguity semantics and a change row. Ticket 16 does not freeze fuzzy matching. Lost behavior: NFC queries against NFD files, and case-fold links. The map is a pure lookup in front of the exact path, so a later add costs little. Check first: does the ticket 19 Link `ComparisonKey` already fold? [H] |
| 8 | Admission by a skeleton against protected names. | SIMPLIFY | Keep one rule: a case-insensitive match of `.traces` and the ignored dirs. This fixes the live bug. Drop the HFS+, `:stream`, `TRACES~N` and `GIT~N` skeleton [H]. If the map is dropped, `eq_ignore_ascii_case` is enough. |
| 9 | Non-UTF-8 names. | DECIDE (a) skip with a notice | Option (b) reserves a character that is dead weight until used. Option (c) is escape machinery with no prevalence data. Option (a) matches jj, Nushell, rust-analyzer and ty [H]. Reduce the ingresses to one function, `RelativePath::from_os`, that returns a `Result`. Lost behavior: a notice instead of indexing. Those files are already missing from tag, class and folder queries [V]. |
| 10 | TOCTOU: accept and document. | KEEP | |
| 11 | Two inspection namespaces. | SIMPLIFY | The raw-path namespace has no named consumer, so it is speculative. Ship the confined namespace only. Ticket 16:89 freezes the "file ops" capability, and the confined default covers it [V]. |
| 12 | A symlinked `.traces` stays an error. | KEEP | It is settled, and F11 goes to ticket 33. |

Ranked defects from this audit that touch ticket 32:

- Contradiction: four equivalence relations over names. They are the listed
  spelling (stored key), the NUL form (sort key), the NFC, lowercase and NFC map
  (query rewrite), and the skeleton (admission). The `.traces` prune is a fifth
  variant. A file can be visible under one relation and not under another, for
  example `Foo.md` and `foo.md` on a case-sensitive file system. The audit
  proposed two relations: exact for everything, plus an ASCII case-insensitive
  match for protected names [H].
- Contradiction: `stat_semantics` against ticket 25 identity. Ticket 25:208 says
  one format epoch covers all derived state. A second tag is a second identity
  mechanism [V].
- Contradiction: "index equals fresh parse" (16:79) against skipped names. Both
  skipped names and overlay-only rows make the persisted index differ from a
  naive parse. The claim needs the scope "indexed files, persisted store" [V].
- Cost without user-visible benefit: the `stat_semantics` tag, the map plus the
  dependency, the skeleton beyond the `.Traces` case, the U+FDD0 reservation,
  and a raw-path template namespace with no named consumer.
- Hole: a `.redb` format epoch bump follows from the new persisted path form but
  is not listed as a duty on ticket 25 [H]. The audit asked whether the
  `.Traces` prune needs the Unicode fold or only ASCII.
- Overclaim: "3M random pairs, 0 mismatches" shows random pairs only. A targeted
  test is needed for a component that is a prefix of another, such as `a/b`
  against `a.b`. NUL sorts below `.`, which agrees with component-wise order.
  The audit asked for an explicit check [H].
- Overclaim: the 395 ms re-read cost justified the `stat_semantics` tag. A user
  pays it once per platform change, and v2 itself calls that case a fork [V].
- Overclaim: "fold-based link resolution is new behavior" has no check against
  ticket 19 `ComparisonKey` [H].

The minimal decision set that the audit proposed for ticket 32 [P]:

- `ProjectRoot` owns confinement I/O.
- A pure `RelativePath` newtype, `/`-joined UTF-8, with `Ord` by the NUL-form
  bytes.
- One format epoch. The platform family leaves the wipe identity. No
  `stat_semantics` tag.
- Key equals the listed spelling. Traces-created files are NFC. Queries compare
  exactly.
- Admission rejects only a case-insensitive `.traces` or ignored-dir match. A
  test covers it.
- Non-UTF-8 names: option (a), through one `RelativePath::from_os` function.
- TOCTOU documented. A single confined template namespace.

What this minimal set loses, and for whom [P]:

- Cross-platform index sharing: a re-read after a wipe.
- Users who type another Unicode spelling or case: an exact miss.
- Non-UTF-8 vault owners: a notice instead of indexing.
- External tools that need raw-path template inspection: no namespace.

Cheap to defer: the comparison-key map, skeleton hardening, U+FDD0, a raw-path
namespace. Expensive to retrofit: the persisted path form and its sort-key
order, the `RelativePath` boundary, and the overlay key type.

The final decision took verdicts 1 to 6, 9, 10 and 12. For verdict 8, it kept
one rule for `.traces` only. It did not take verdicts 7 and 11.

## 13. Audit 2: AuditUxConventions (UX, conventions and performance lens)

Measured today with the 9 November release binary in a temp vault with a fresh
HOME [V]:

- `list` and `table` print rows on stdout and `N page(s) from ROOT` on stderr.
  `task` prints `N task(s) from ROOT` on stderr.
- `--count` exists only on `task`. It prints a bare number and hides the stderr
  summary. `list --count` and `table --count` fail with a clap error, exit 2.
- `--sort` defaults to descending. `--asc` and `--desc` need `--sort`.
- A `--where` syntax error exits 1 with a miette span and a `help:` line.
- An empty result exits 0.
- A fresh vault needs `traces trust` first. The failure message tells the user
  what to do.
- The source has no notice emission outside the stderr count lines. `grep`
  finds no `warning:` text in `src/` [V]. Ticket 34 is open.

Re-measured NUL-form key sort at 50k. Method: synthetic paths with a mix of
`-`, `.`, space and `_` separators, 1 to 4 levels deep, median of 9 runs:

| Case | Measured | v2 claim |
| --- | --- | --- |
| `Vec<PathBuf>` sort | 31 to 32 ms at both opt levels [V] | 31.6 ms |
| Sort of pre-built NUL keys | 4.75 ms at opt-level z; 3.4 ms at opt-level 3 [V] | 4.5 ms |
| Key build plus sort at opt-level z | 13.5 ms [V] | Not mentioned |
| `Vec<String>` sort with a comparator that maps `/` to 0 for each byte | 10 ms at opt-level 3; 22 ms at opt-level z with a naive iterator comparator [V] | Not mentioned |

- The order matched `Path::cmp` in every case: 0 differing positions.
- A `String`-backed `RelativePath` with the NUL-form `Ord` is the comparator
  row, not the pre-built-key row. So the v2 sort claim is 2x to 5x too
  optimistic. It is still 1.4x to 3x faster than `Path::cmp`, so the decision
  stands.
- The probe source is `/tmp/probe` and `local://probe`. The probe uses no
  worktree.

Verdicts for ticket 32:

| v2 decision | Verdict | Detail |
| --- | --- | --- |
| `ProjectRoot` owns confinement I/O. No capability handles. | KEEP | It fits the AGENTS rules and the TOCTOU analysis of W32e [H]. |
| Reuse the name `RelativePath`, `String`-backed. | REVISIT (name and `Ord`) | Reuse of a name for a different type is a silent semantic break. `src/path.rs:64` is a `RelativePath(PathBuf)` that allows `.` components. `pub(crate)` uses sit at `pub-tokens.txt:153,871,1409` [V]. The only stated reason for reuse is fewer renames. A clean break would use a new name such as `VaultPath`. If `Ord` follows the NUL form, `Ord` disagrees with `str` byte order. So no `Borrow<str>` is allowed: `Borrow` needs `Ord`, `Eq` and `Hash` to agree, and `BTreeMap<RelativePath, _>::get(&str)` would silently miss. Also forbid `Deref<Target=str>`. Give only `as_str()` and `as_path()`. Ticket 35 must state this. A property test checks `Ord`, `Eq` and `Hash`. |
| Persisted path UTF-8 with `/`. New epoch. `stat_semantics` tag. | KEEP | The layout is pre-release. The 395 ms figure is a ticket 24 number that the audit did not re-measure [H]. |
| Sort key with `/` replaced by NUL. | SIMPLIFY the claim, KEEP the rule | State about 10 ms for a string comparator and 4.5 ms for stored keys. Say which one the in-memory `Ord` uses. "`Path::cmp` banned in index modules" needs a mechanism: a clippy `disallowed-methods` entry or a test. A grep rule is not enough. |
| Listed spelling kept. Comparison-key map for queries. | KEEP | Exact match comes first, so no regression. The map costs 3.9 ms at 50k, built lazily [V per W32a; not re-measured]. Ambiguity says "OR or an ambiguity notice". Pick one. For a CLI, an OR that lists the spellings in a stderr notice is best. Fold-based link resolution needs a change-table row. The dependency record says that `icu_normalizer` exists because of query-time fold. |
| Admission by a skeleton against protected names. | KEEP | It is the narrowest sound rule. It fixes the `.Traces` bug [V per W32a; not reproduced]. Name the lint code for Windows device names so that ticket 34 can register it. |
| Non-UTF-8: undecided. | DECIDE (a), reserve U+FDD0 | Before release, the current behavior is a silent drop from tag, class and folder queries on Linux, so skip with a notice is a strict improvement. Option (c) adds new rows to `list` and a display decision. Option (b) costs one check and keeps the door open. An open decision blocks the work "at every ingress". |
| TOCTOU accepted and documented. | KEEP | `rustix` is already a dependency, so the `openat` upgrade stays cheap later. |
| Two inspection namespaces. | KEEP | Name them so that users cannot confuse "confined" with "raw" in templates. |
| A symlinked `.traces` stays an error. | KEEP | |

Ranked defects from this audit that touch ticket 32:

- Contradiction: the NUL-form `Ord` against `Borrow<str>`. Any `Borrow<str>`,
  `Deref<Target=str>` or `HashMap` or `BTreeMap` lookup by `&str` breaks the std
  contract. The v2 text did not forbid these impls.
- Contradiction: "output order stays" against "key is the spelling as listed".
  On macOS, a case-only rename changes the key, and so the order, until the next
  scan. W32 weakness 7 says that only the next scan corrects the spelling [H].
  The v2 text did not call this a documented limitation.
- Hole: no Windows check of `Path::cmp`. RedTeam29 has no Windows ordering
  check. The NUL-key equivalence claim is macOS only [V]. Cross-platform
  separator behavior is untested. The persisted form uses `/`. The original
  advice "say that `as_path()` on Windows accepts `/`" is superseded. Section 16
  moves native conversion into `ProjectRoot`.
- Hole: the public API baseline. `tests/baselines/pub-tokens.txt` lists
  `RelativePath` and `SafeRelativePath`. A rename or a type change shows in the
  baseline diff. The change table needs one row.
- Overclaim: "4.5 ms against 31.6 ms" is the pre-built key case. A `String`
  `Ord` costs 9 to 10 ms at opt-level 3 and 10 to 22 ms at opt-level z. The
  ratio is 1.4x to 3x, not 7x.
- Overclaim: the release profile. The ticket 28 probes used a bench profile at
  opt-level 3. RedTeam29 and RedTeam32 used a release build. The shipped profile
  is `opt-level = "z"` (`Cargo.toml:269-281`). Opt-level z makes
  iterator-heavy comparators 2x slower in the audit run. The other numbers are
  not re-measured under "z".
- Overclaim: macOS APFS only for the path probes. Linux has a podman check of
  non-UTF-8 listing only. No Windows or HFS+ data.
- Overclaim: the normalization corpus has 10% non-ASCII. A CJK, Indic or
  accent-heavy vault pays more, because ASCII fast paths skip. At 3x, the 3.9 ms
  map costs 12 ms. This does not change a decision.
- Overclaim: "fixed" for `.Traces` is a plan, not a measured fix. The
  reproduction is [V per W32a]. The audit did not rerun it.

The audit found no reason to drop a v2 decision for ticket 32. It would reopen
two items: the `RelativePath` name with its `Ord` contract, and (for ticket 29)
the group-by CLI details.

Cleanup: the audit made no worktree and left `/tmp/probe`, `/tmp/tv` and
`/tmp/tvhome` as scratch. It used an isolated HOME. It changed nothing in the
repo tree.

## 14. Audit 3: AuditImplementability (implementation map, ambiguities)

Plan verdict: the plan is implementable in outline. One decision was wrong
(path-set patching, which belongs to ticket 28). One cross-ticket rewrite was
unrecorded (platform family). A few decisions assumed code that does not exist.
The audit ran one compile check in a scratch clippy crate. It deleted the crate.

### 14.1 Implementation map: `RelativePath` and `ProjectRoot` [V]

Touched code:

- `src/path.rs`. `RelativePath(PathBuf)` already exists as `pub(crate)`
  (`path.rs:64`) and accepts `.` components. `ConfigSubDir`
  (`config/model.rs:278`), `TemplatePathInput` (`template/path.rs:38`) and
  `scan_file_metadata` use it.
- `SafeRelativePath` has 8 call sites: `schema/fields/select.rs:258`,
  `template/writer.rs:313`, `template/engine/file.rs:81`,
  `template/engine/path.rs:165`, `config/model.rs:300`, `lib.rs`, `file.rs`,
  `path.rs`.
- `FileMeta.path` and `Note.path` are both `PathBuf` with `path::codec`.
  `FileMeta::derive_day`, `BaseNameRef::from_path` and `FolderRef` work on
  `&Path`.
- `HasPath::path() -> &Path` (`path.rs:57`) is implemented by `FileMeta`,
  `FileEntry` and `Note`. `SortedByPath` depends on it.
- `index/{entry,inlinks,sort,refresh,delta,store,codec,trie,service}.rs`.
- `query/grammar/source.rs` (`GlobPattern::is_match`, `display_portable`),
  `query/format.rs`, `query/results.rs`, and `query/service.rs`
  (`SourceResolver` uses `Vec<PathBuf>`, `acc.sort()` and `intersect_sorted`).
- `lib.rs:192` (`path::codec as path_codec`), the benches
  (`index_codec.rs:47` uses `traces::testing::path_codec`; `common/notes.rs`;
  `index_inlinks.rs`; `common/project.rs`), integration tests and
  `index-paths.feature`.

Impact size [V]: codegraph `impact` on `path` gives 358 affected symbols. Most
are tests.

API changes:

- `RelativePath::{parse(&str), from_os(&OsStr), as_str, as_path}`.
- `ProjectRoot::{relativize(&Path), join, write}`.
- `HasPath::path() -> &RelativePath`.
- `FileMeta::path()`, `Note::path()`, `MarkdownParserInput::new(&Path, …)` and
  `FileEntry::inlinks() -> &[PathBuf]` change type.
- `PathKey` and `path_from_bytes` in `index/codec.rs` get NUL-form keys.
- `collect_folder_paths` (`store.rs:1090`) changes its range to
  `[prefix\0, prefix\x01)`.

Baselines [V]:

- `pub-tokens.txt` is a `git grep` of every `pub…` line in `src/`
  (`mise.toml:145`), so it covers `pub(crate)` too. 163 lines mention `Path`.
  The new signatures sort again.
- `public-api-default.txt` has 16 touched lines: `FileMeta::path` (:1392),
  `Note::path` (:1743) and `MarkdownParserInput::new` (:1687).
- `public-api-test-utils.txt` has 183 lines with `Path`. The three
  `testing::path_codec` lines (:280-282) disappear.
- Visibility. If `FileMeta::path()` returns `&RelativePath`, the type must
  become `pub` or sit in a private module. That is a widening under the ticket
  16 rule. It belongs to ticket 35.

Tests that break [V]:

- Non-UTF-8 fixtures: `service.rs:888/1300`, `store.rs:2342/2750/2772/2812`,
  `benches/index_codec.rs:67`.
- Path codec tests (`path.rs:726-780`) and the `SafeRelativePath` tests
  (`path.rs:558+`).
- `preserves_path_sorted_order_for_hyphenated_sibling_folders`
  (`service.rs:488`).
- `FileMeta::for_test<P: Into<PathBuf>>` fixtures across the suite.

New tests required:

- Rejection of NUL, backslash and empty components.
- A property test that `Ord`, `Eq` and `Hash` agree.
- Per-OS skip tests.
- The NUL form against `Path::cmp` on valid Unicode.
- Folder-range boundary cases: `notes`, `notes.md`, `notes-x/`.

### 14.2 NUL sort key and the `Path::cmp` ban

- The ordering proof is sound on paper [V by reasoning]. NUL sorts below every
  other byte, so a shorter component sorts first, as `Path::cmp` does.
- A lint cannot express the ban. The audit ran clippy 1.98 in a scratch
  crate [V]:
  - `std::path::Path::cmp` gives "does not refer to a reachable function".
    Clippy cannot name a trait-impl method by its impl type.
  - Only `core::cmp::Ord::cmp` fires, and it fires on every type.
  - `v.sort()`, `a < b` and derived `Ord` on `Target<'a>(&'a Path)`
    (`inlinks.rs:410`) stay silent.
  - `clippy.toml` is crate-wide, not per module.
- Sites that a lint cannot see [V]: `acc.sort()` and `intersect_sorted` in
  `query/service.rs:364/392/417`; `note.path() < file.path()` in `entry.rs:90`;
  `flat_edges.par_sort_unstable()` in `inlinks.rs:88`.
- Real enforcement is type-based: no `PathBuf` or `Path` in any sorted
  structure.
- The v3 cost line "10 ms or 4.5 ms (stored keys)" does not say where the
  stored keys live (ambiguity B4).

### 14.3 Parse rules and ambiguities that touch ticket 32

| # | Ambiguity | Proposed one-line resolution |
| --- | --- | --- |
| B1 | Does `RelativePath::parse` reject NUL, `\`, empty components and `..`? The `Ord` and `Eq` agreement depends on rejecting NUL. | `parse` normalizes `.` and `//`. It rejects `..`, NUL and empty components. Only `from_os` treats `\` as a separator, on Windows only. |
| B2 | Is `from_os` per component or for the whole path? Who strips the root? | `ProjectRoot::relativize(&Path) -> Result<RelativePath, Skip>` calls `from_os` for each component. |
| B3 | What is the quoted-octal form on Windows, where names are WTF-16? | Print lone surrogates as `\uXXXX` and valid text as UTF-8. State the Windows rule. |
| B4 | Is the NUL-form key stored inside `RelativePath` or computed on the fly? | `RelativePath` holds one `String`. `Ord` maps `/` to NUL on the fly. Stored keys are transient sort keys at scan and load only. |
| B5 | Two fold functions: admission uses ASCII fold plus NFC, and the comparison key uses `to_lowercase`. | Use the single Comparison Key function everywhere, or document the ASCII fold as a strict subset. |
| B6 | Does admission apply to env-configured ignores and to config discovery? | Match case-insensitively for `.traces` only. Other ignored dirs keep exact case. Add a change row. |
| B7 | Does the fold map cover files only or folders too? Globs or literals? | Add `PathAtom::{Literal, Folder, Glob}` to the IR. The map covers files and folders. Globs are never rewritten. |
| B8 | Where does the fold map live on the CLI cold path and in the overlay? | The code builds it lazily from `files` keys for each pinned generation (`OnceLock`). An overlay-added path invalidates it. |
| B9 | `normalize_source_input` probes the real file system (`cli/mod.rs:440-472`). That differs on case-insensitive and case-sensitive volumes. | Move it into core and resolve it through the index fold map. Tell ticket 33 that it no longer decides this. |
| B20 | The overlay key for an unsaved new file: which spelling, when the base lists nothing? | Use the URI spelling. The 2 s scan proof drops it, whatever the real spelling is. |

(Ambiguities B10 to B19 and B21 belong to tickets 28 and 29.)

### 14.4 IR path atoms, fold map and link resolution [V]

- The IR cannot express "path-literal atoms" today. Source atoms are regex
  globs: `SourceAtom::Path(GlobPattern)` (`source.rs:190`, `:250-290`) matches
  the `display_portable` string. ADR-0008:77-84 says the same. The IR needs a
  Literal, Folder and Glob split. Then the planner can rewrite an atom to the
  listed spelling. The map must also cover folders, not only files.
- `src/GLOSSARY.md:107-115` already defines the Comparison Key as NFC, then
  `char::to_lowercase`, then NFC, with an exact match first, "at index-build and
  query boundaries".
- No NFC normalization dependency exists in `Cargo.toml`. `deny.toml`
  already allows `Unicode-3.0`.
- The home of the map is unclear. `WorkspaceIndex` is only
  `entries: Box<[FileEntry]>` (`entry.rs:21`). The CLI cold path
  (`run_from_store`) has no generation object. It would build the map from
  `files` keys on a miss.
- Link resolution is at risk. `LinkResolver` resolves at index time and persists
  inlinks. If link resolution uses the fold, then the extractor fingerprint
  (`25:312-317`) must list `icu_normalizer`, and the epoch bumps.

### 14.5 Admission today [V]

- `is_ignored_dir(file_name)` is exact-case. It covers `DEFAULT_IGNORED_DIRS`
  and the user env var `TRACES_IGNORED_DIRS` (`env_vars.rs:19-44`). Callers:
  `index/service.rs:427` and `config/discovery.rs:163`.
- `.traces` is ignored by name only. `scan` has the dead `path != index_db`
  check (`:435`).
- The v3 text made the match case-insensitive for `.traces` and "the ignored
  dirs". On Linux that also skips `Target/` and `Node_Modules/`. It changes user
  configured ignores. It changes config discovery if the predicate is shared.
  The `.Traces` fix itself is correct [V].
- Two folds exist: ASCII fold plus NFC for admission, and `char::to_lowercase`
  for the comparison key.

### 14.6 Non-UTF-8 skip through `from_os` [V]

- Ingresses that exist today: scan (`RelativePath::derive`),
  `normalize_source_input`, and the DSL path atom.
- Ingresses that do not exist yet: watcher and `didOpen` (no host code).
- Vacuous ingresses: the CLI `--from` is `Option<String>`, and clap rejects
  non-UTF-8 before `from_os` runs. Outlink targets are `&str`, so `from_os` does
  nothing there. They need lexical normalization instead, which `[[../x]]`
  breaks: today `LinkResolver::resolve` looks up the path as written from the
  root (`inlinks.rs:365-373`).
- Octal quoting has no byte form on Windows. Existing fixtures use the lone
  surrogate `0xD800` (`store.rs:2354`).
- This reverses current, tested behavior. Today the code indexes non-UTF-8 files
  byte-exact (`index/codec.rs:83-94`, `service.rs:1300`, `store.rs:2772`).
- Defect 13: invert the tests and do not delete them. Defect 14: name the real
  ingresses: scan, `ProjectRoot::relativize` and the URI mapping.

### 14.7 Platform-family contradictions with resolved tickets [V]

The v2 and v3 text removes the platform family from the store identity. It
still appears in:

- ticket 24 rule 3 (`:299`) and `:311`;
- ticket 25 "Store identity" (`:299-300`, `:357`);
- ticket 26 `:251`;
- ticket 27 `:306`, and the probe test `rejects_mutated_platform`.

Ticket 32:31 allows the removal "when the form is portable". It still needs
explicit supersession rows in tickets 24, 25, 26 and 27. The `META` field list
changes.

Other supersessions and wording:

- Ticket 24 `:392-395` asks for a release order check with `OsStr` comparison.
  The NUL-form key supersedes it. Say so.
- Ticket 25 `:359` says that a read of a newer epoch parses fresh and does not
  wipe. v3 says a cross-family store is a fork and is wiped. Align the wording.
- Ticket 22 `:182` says "Do not hold a workspace object in core".
  `ProjectRoot` owns canonical root resolution, and tickets 22 and 32 mention
  `WorkspaceIdentity` (`32:119`). State that `ProjectRoot` is the value of
  `WorkspaceIdentity`, or contains it. Neither type exists in `src/`.
- The v2 duty "`stat_semantics` tag to ticket 25" is stale. v3 dropped the tag.
- Ticket 24 relabel: the text at `:396-398` says that the trie build took 224 ms
  and a flat scan took 412 to 436 ms. That feeds "keep `CandidateTrie`". Recheck
  the decision after the relabel.

### 14.8 Glossary conflicts [V]

- `src/GLOSSARY.md:20` says "Avoid: relative path" under Root-Confined Path.
  Add a pure "Relative Path" entry. Edit both.
- `src/GLOSSARY.md:63-68` Path Codec: replace or delete.
- `src/GLOSSARY.md:107-115` Comparison Key: extend it. Add a Canonical Key and
  a NUL-form sort key, so that the three "key" terms stay distinct.
- `src/GLOSSARY.md:98-105` Ambiguity matches "PathTie, never a diagnostic".
- The DSL field `ctime` is the birth-time alias (`query/grammar/field.rs:205`).
  The ticket 25 "ctime" is the Unix inode-change time. Add an Avoid line.
- Map: `.scratch/arch-design/map.md` lines 438-441 name the platform family in
  the identity. Update them. Graduate the fog item "Persistence portability"
  (`:520`).

### 14.9 Change-table candidates for ticket 32 (ticket 16 categories)

| # | Change | Category | Flag |
| --- | --- | --- | --- |
| 1 | New `RelativePath`. `SafeRelativePath` removed. `path_codec` and the `FileMeta` and `Note` path types change. | Free | Visibility call for ticket 35. |
| 2 | Store keys become NUL-form UTF-8 strings. New epoch. The platform family leaves the identity. | Free | Needs supersession rows in tickets 24, 25, 26 and 27. |
| 3 | Non-UTF-8 names are skipped with a stderr notice. They were indexed and listed lossily. | Migratable | Changes stdout rows. Needs a row. |
| 4 | `.Traces` is no longer indexed. | Free (defect fix) | |
| 5 | Case-insensitive ignored dirs (`Target/`, env ignores; discovery if shared). | Migratable | The audit said this changes stdout rows on Linux. The final decision removed this row. |
| 6 | Fold-based path-literal lookup in `--from` and link resolution. | Migratable | It can change `file.inlinks` output. Extractor fingerprint. |
| 7 | Backslash link targets stop resolving on Windows. | Migratable | [H], Windows only. |
| 8 | `normalize_source_input` moves from file system probing to the index. | Migratable | |
| 18 | `ConfigSubDir` and `TemplatePathInput` normalize `./` and reject NUL. | Free | |
| 19 | New dependency `icu_normalizer`. `release` is `opt-level = "z"`. | Free | Recheck sort timings at `z`. |

### 14.10 Cross-platform gaps

Windows:

- E-1 [H, std knowledge]: `Path::cmp` compares WTF-8 bytes, not UTF-16. For
  valid Unicode it equals code-point order, which equals the NUL-form order. The
  audit did not check this on Windows. The 3M-pair match test ran on macOS only.
  Add the property test to Windows CI. CI already runs unit and e2e tests on all
  three OSes. Integration tests run on Linux only.
- E-2: untested. The list is below.
  - Separators in `RelativePath::from_os`.
  - The `\\?\` canonical root with `strip_prefix` (`config/service.rs:271`
    canonicalizes).
  - Trailing dot or space, `:stream`, 8.3 names and `CON` or `NUL`. v3 defers
    these to `cfg(windows)` later.
  - Lone-surrogate file names and their skip notice (B3).
  - Backslash in link targets (change row 7).
- E-3 (identity and stat semantics): v3 drops `stat_semantics` and keeps the
  canonical root in the identity. The same canonical-root string can reach one
  `.traces/index.redb` from two OS families (VM mounts that keep the host path,
  network shares). Then ctime-validated Unix rows meet Windows (no ctime), or
  the reverse. The rule for a row stored without ctime read on Unix was not
  stated. Rule: a missing ctime means the row is stale.
- Windows has no inode-change time. Ticket 25 records the gap.

Linux:

- Non-UTF-8 fixtures need Linux or Windows. macOS APFS cannot create them
  (`service.rs:884`).
- NFC and NFD names are distinct files. The union rule is right. Nothing tests
  two sibling files with such names.
- On case-sensitive trees, `Foo.md` and `foo.md` coexist. Exact first, then
  fold, gives a union. The audit asked for tests of both this case and the
  admission case-fold of `Target/` and similar names.
- `Metadata::created()` can return `None` on some kernels. Ticket 21 handles
  that.

macOS:

- APFS is case-insensitive and keeps normalization. The v3 measurement is APFS
  only.
- HFS+ (external or legacy volumes): the listing returns NFD and ignores
  ignorable codepoints. This is untested. v3 dropped the ignorable-codepoint
  check on APFS evidence.

All platforms:

- Windows CI has no case-insensitive scan test for the `.Traces` fix.
- No test builds a `RelativePath` from a file name that has a literal NUL, an
  empty component or a lone `\`.

### 14.11 Ranked defects of this audit that touch ticket 32

| Rank | Severity | Defect |
| --- | --- | --- |
| 2 | [V] High | Platform-family removal contradicts tickets 24, 25, 26 and 27 with no recorded rows. A stale v2 duty still lists `stat_semantics`. |
| 4 | [V] Medium | The `Path::cmp` ban cannot be written as a clippy lint. Enforce by types. Drop the "banned via disallowed-methods" claim from the Answer. A project-wide `core::cmp::Ord::cmp` ban is too wide. |
| 5 | [V] Medium | `RelativePath::parse` rules are silent on NUL, `\` and empty components. The `Ord`, `Eq` and `Hash` agreement and the key aliasing depend on them. Outlink targets, DSL literals and URIs can carry NUL or `%00`. The property test must generate them. |
| 6 | [V] Medium | The IR cannot express "path-literal atoms". Add a Literal, Folder and Glob split and a fold map for folders. |
| 7 | [V] Medium | Admission case-fold hits every ignored dir and the user env var. It changes Linux stdout, may change config discovery, and uses a second fold function. |
| 13 | [V] Low-medium | v3 reverses existing tested non-UTF-8 behavior. Invert the tests. |
| 14 | [V] Low | "Every ingress" lists ingresses that do not exist or are vacuous. Name the real ones. |
| 18 | [V] Low | The DSL `ctime` and the ticket 25 `ctime` are different. Add a glossary Avoid line. |

Ranks 1, 3 and 8 to 12, 15 to 17 belong to tickets 24, 28, 29, 31, 33, 34 and
35. Rank 3 (the `Blocked by` graph) is partly a ticket 32 duty: ticket 33 gains
28 and 32.

Sound as decided [V]:

- the NUL-form ordering argument and the folder range;
- "key is the spelling as listed", with exact first, then fold;
- the existing `.traces` self-index fix;
- a quiet heal for a rebuild cause.

Not checked by the audit:

- the 0.148 and 13 ms patch timings (ticket 28; the artifact is not in the
  repo);
- Windows behavior;
- whether `public-api-default.txt` lists types beyond the 16 matches that the
  audit counted.

## 15. Factual corrections from the hostile copy edit

### Configured output directory

The writer keeps the configured output-directory exception
(`src/template/writer.rs:328-342`) [V].
The builder can inherit `output_dir` from global config
(`src/config/builder.rs:96-109`) [V].
Ticket 22:187-198 gates local config, but not global config [V].
Thus, "trust-gated `Config::output_dir`" is too broad.
The ticket 32 Answer preserves ticket 22. The corrected endorsement premise
needs maintainer confirmation.

### Symlinked metadata directory

The outside-root failures in section 6 do not prove that every symlink fails.
The current path check tests containment (`src/path.rs:180-190`) [V].
`ConfigSubDir::resolve_against` calls that check
(`src/config/model.rs:296-305`) [V].

Stress32 tested `.traces -> .metadata`, with `.metadata` inside the root.
The `trust`, `index` and `list` commands all exited 0 [V per peer probe].
The list included `.metadata/config.toml`, `.metadata/index.redb` and `note.md`.
The recorded all-symlink rejection therefore changes current behavior.
Its former "No change" label needs maintainer review.

### APFS alias that the Comparison Key does not match

Stress32 supplied this probe [V per peer run].
The copy editor read the probe but did not rerun it.
The probe used the current debug binary, not the earlier release binary.
Its vault was `/tmp/stress32-unicode-owaau404/vault`.

Input:

```python
(root / '.traces').mkdir()
(root / '.traces' / 'config.toml').write_text('')
(root / '.traces').rename(root / '.traceſ')
(root / 'note.md').write_text('# hello\n')
```

The probe isolated `HOME`, `TRACES_STATE_DIR`, `XDG_STATE_HOME`,
`XDG_CONFIG_HOME` and `XDG_DATA_HOME`.
It set each variable to a separate directory under the probe's temp directory.
The listed `.traceſ` was a directory, not a symlink.
`root / '.traces' / 'config.toml'` still existed.

Commands, with the vault as the current directory:

```text
/Users/jack/Documents/41_personal/traces/target/debug/traces trust /tmp/stress32-unicode-owaau404/vault
/Users/jack/Documents/41_personal/traces/target/debug/traces index
/Users/jack/Documents/41_personal/traces/target/debug/traces list
```

The script used the absolute binary path
`/Users/jack/Documents/41_personal/traces/target/debug/traces`.
All commands exited 0.
The `index` stderr was:

```text
indexed 2 file(s) under /private/tmp/stress32-unicode-owaau404/vault
```

The `list` stdout was:

```text
- .traceſ/config.toml
- .traceſ/index.redb
- note.md
```

The `list` stderr was:

```text
3 page(s) from /private/tmp/stress32-unicode-owaau404/vault
```

The Rust probe printed `long_s_lower=Some('ſ')`.
NFC, lowercase, NFC leaves `.traceſ` unchanged [V per peer probe].
Thus, the D6 Comparison Key rule does not exclude this APFS metadata alias.
This evidence does not change the endorsed query lookup rule.
The maintainer now confirms metadata identity exclusion (final review below).

### Open path questions

The final rule rejects rooted input first. It then normalizes repeated and final
separators. The stored result contains no empty component.
Windows treats `/` and `\` as separators
(`std/src/sys/path/windows.rs:49-52`) [V].
It also recognizes a drive prefix
(`std/src/sys/path/windows_prefix.rs:128-138`) [V].
These references use the installed stable Rust source.
The maintainer confirmed the correction after this review.
`RelativePath` stays pure portable text with `as_str()`.
There is no public zero-cost `as_path()` promise.
`ProjectRoot` owns native conversion and native component validation.
Native-unrepresentable names produce typed errors.
The earlier audit recommendations for a zero-cost view are superseded.

Journal fact path values are canonical UTF-8 text with `/` separators.
Database lookup keys use NUL encoding. The store adapter owns that encoding.
Journal paths do not encode their separators as NUL.
Earlier "journal form equals store form" wording refers to path values only.

The final review resolves root relocation in favor of canonical-root binding.
The old "`mv` replays" result remains prototype evidence, not the production
rule.

## 16. Confirmed final review (2026-11-16)

The maintainer confirms these corrections after review of the alternatives.
They supersede conflicting earlier recommendations in this research record.

- Exclude the admitted metadata directory by filesystem identity and subtree.
  Keep query Comparison Key semantics unchanged.
- Permit `.traces` links only to a directory strictly inside the admitted root.
  Reject outside, dangling, cyclic, non-directory and unverifiable targets.
  Never repair a link by removing its target.
- Keep canonical-root cache identity. Rebuild recognized derived artifacts after
  root relocation. Never recursively remove metadata or user-authored files.
- Preserve higher-epoch handling and independent root trust admission.
  A within-vault note rename still uses normal refresh.
- Defer root rebinding until measured relocation costs justify its validation
  and crash-recovery duties.
- Convert portable paths to native I/O targets only through `ProjectRoot`.
  Reject native reinterpretation, including Windows prefixes, traversal,
  devices, streams and trailing aliases. Keep valid Unix text names on Unix.
- Keep `strict-path` until replacement conformance succeeds.
- Keep the configured-output exception under admitted Config provenance.
  Local trust and the ungated global policy remain separate.
- Accept the documented final-component race for note reads and inspection.
  Template publication avoids a pre-existing final destination symlink only
  while its parent remains stable.

The [conventions note](./i-conventions-review.md) checks Git, Obsidian and Cargo
primary documentation. It establishes no universal policy or Windows guarantee.
The macOS NUL-order probe checks 93,636 pairs with zero mismatches and the
17 expected folder children [V per Stress32]. Windows remains untested.
