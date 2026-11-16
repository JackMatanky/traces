# Non-UTF-8 file names: precedent, prevalence and IWE and Obsidian traces

Goal: give the grill for ticket 32 the evidence on non-UTF-8 file names.
This file condenses two source reports. They are `agent://NonUtf8Precedent`
(sections 1 to 5 below) and `agent://IweObsidianNonUtf8` (sections 6 to 9).
The reports made no repo edit. This file adds no new claim.

Marks: [V] means the source report read the line or URL that is cited.
[H] means a hypothesis or an inference. [H] also marks closed-source
behavior. The IWE report ran no program. It read every IWE outcome from code.
Evidence limits are in section 10.

Upstream source-line references are unpinned source-report citations.
This copy edit does not recheck those upstream lines [H].
Code-derived runtime outcomes are hypotheses, not measured behavior [H].
The ticket 32 Answer records the final decision. This file records its inputs.

## 1. Comparison table: what each tool does with a non-UTF-8 name

| Tool | Behavior | Representation | Source |
| --- | --- | --- | --- |
| git | Keeps the name. `status`, `ls-files` and `diff` print octal such as `"caf\351.md"`. `core.quotePath=false` stops the quoting of bytes above 0x80. `-z` prints the raw name and a NUL. | Byte string | [V] git-scm.com/docs/git-config (`core.quotePath`); git-scm.com/docs/git-ls-files (`-z`) |
| gitoxide, `gix-path` | Keeps the name on Unix. On other systems, a lone surrogate makes the UTF-8 conversion fail. | `BStr`; raw bytes on Unix | [V] gix-path `src/convert.rs:53-55,76` |
| ripgrep | Keeps the name. A path argument can be invalid UTF-8. `-0` prints a NUL after each path. | Raw bytes through `Vec::from_path_lossy` | [V] `crates/printer/src/util.rs:304`; `crates/core/flags/defs.rs:2086,4981` |
| `ignore`, `bstr` | Keeps the name. Gitignore matching uses `OsStrExt::as_bytes` on Unix. | `&Path` | [V] ignore@0.4.33 `src/pathutil.rs:102-162`; bstr@1.13.1 `src/ext_slice.rs:480-496` |
| fd | Keeps the name. A pipe on Unix gets raw bytes. A terminal gets a lossy and sanitized name. | `OsStr` | [V] sharkdp/fd `src/output.rs:80-107`; PR 2197 |
| jj | Skips the name. `RepoPath` is UTF-8 by construction. Before commit `54c4b49` (Aug 2026), one such file failed the whole snapshot. Now the snapshot skips the path and reports it in `SnapshotStats`. | UTF-8 string | [V] docs.rs/jj-lib `RepoPathComponent`; issues 9774 and 9002; commit 54c4b494 |
| Mercurial | Keeps the name. File names are "binary data in an unknown encoding". | Bytes | [V] wiki.mercurial-scm.org/EncodingStrategy section 4 |
| Python | Keeps the name as lone surrogates. Bytes 0x80 and above map to U+DC80 to U+DCFF. Bytes below 0x80 stay as they are. | `str` (PEP 383) | [V] peps.python.org/pep-0383 |
| Go | Keeps the name, because a `string` is a byte sequence. The source report did not fetch a Go source for this. `json.Marshal` replaces invalid bytes with U+FFFD. | `string` | [H] for the keep; [V] golang/go `src/encoding/json/encode.go:82-83` |
| Nushell | Skips the name and prints a message. `Value::String` is a Rust `String`. Issue 11100 is open since 2023. On 0.117 (2026-11-14) `ls` omits the file with a message, and `glob *` still returns it. | UTF-8 string | [V] github.com/nushell/nushell/issues/11100 |
| rust-analyzer | Skips the name without a message. `Utf8PathBuf::from_path_buf(..).ok()?` is in `vfs-notify` at lines 211 and 324. PR 16889 enforced UTF-8 paths. Then issue 16914 showed that a file named `a\xFFz.txt` caused an `unwrap` panic in the watcher path. | `AbsPathBuf` over camino | [V] rust-analyzer `crates/vfs-notify/src/lib.rs`; `crates/paths/src/lib.rs:1`; issue 16914 |
| ty, `ruff_db` | Reports an error and continues. The walker emits a `Severity::Error` diagnostic: "`{path}` is not a valid UTF-8 path". The walk continues with `WalkState::Continue`. | `SystemPathBuf` over `Utf8PathBuf` | [V] astral-sh/ruff `crates/ty_project/src/walk.rs:305-310,370`; `ruff_db/src/system/os.rs:342-355` |
| Classic `ruff check` | Not confirmed. The grep of `ruff_workspace/src/resolver.rs` showed no handling. | n/a | [H] |
| camino | Rejects the name by design. Its README lists "core system utility", "legacy compatibility" and "disk recovery" as cases where you should not use it. | `Utf8PathBuf` | [V] camino-rs/camino README; camino@1.2.6 is in the local registry |
| `relative-path` | Returns the error `FromPathErrorKind::NonUtf8`. | UTF-8 `/` path | [V] relative-path@1.9.3 `src/lib.rs:696,1068` |
| Cargo | Rejects the name. The rust-analyzer PR author says "Cargo already requires this". The source report read no Cargo source. | n/a | [V] as a maintainer quote (rust-analyzer PR 16889); [H] for the Cargo behavior |
| Deno | Dropped the name without a message through `into_string().ok()?`. Since PR 34623 (2026-05) it lists the name with U+FFFD. A lossless Buffer path is "a larger refactor". The PR notes that the lossy name is a different file from the real one. | `String` | [V] github.com/denoland/deno/pull/34623 |
| Node, VS Code | Node returns UTF-8 strings by default. It returns a `Buffer` only with `encoding:'buffer'`. The Deno PR calls the default "lossy U+FFFD". The report found no VS Code explorer behavior. | JS string, `Uri` string | [V] nodejs.org/api/fs.html; Deno PR 34623; [H] that VS Code inherits Node |
| markdown-oxide | Keeps the `PathBuf`. `.to_str()` gives `None` for a non-UTF-8 directory, so the code does not exclude it. A file needs the `.md` extension. Derived name strings drop. | `PathBuf` | [V] `src/vault/mod.rs:36-60,444,1563`; [H] for the consequence |
| IWE | Keeps `(String, PathBuf)`. The key uses `to_string_lossy` on each component. Two names that differ only in invalid bytes get one key. | Lossy key plus `PathBuf` | [V] `crates/liwe/src/fs.rs:29-40`; [H] for the collision winner |
| mdBook | `SUMMARY.md` decodes only `%20` today. PR 3162 (open) proposes full percent-decoding. | `PathBuf` | [V] rust-lang/mdBook PR 3162 |
| Obsidian | Closed source. The help text says Markdown link destinations must be URL-encoded. Wikilinks are raw text. The help text does not say what happens with an invalid name. | JS strings | [V] obsidianmd/obsidian-help "Internal links"; [H] for the representation |
| Foam, Logseq, Zettlr, zk, Hugo, Zed | Not found in the searched material. The search covered GitHub issues for "non-UTF-8", "invalid UTF-8" and file name terms in each repo, and an Obsidian forum search. It found no hit. | n/a | [H] |

Pattern across the table:

- Tools that handle bytes keep bytes at the I/O boundary.
- Tools with a UTF-8 type converge on "skip and report". They do not fail the
  whole run. jj went from fatal to skip. rust-analyzer went from panic to skip.
  Nushell warns and omits. ty reports an error and continues.
- Deno shows the lossy-listing trap. The shown name no longer opens the file.

## 2. Representation options

| Option | Round trip | Injective | Sort equals raw byte order | Pitfall |
| --- | --- | --- | --- | --- |
| UTF-8 string, skip invalid names | Valid names only | Yes | Yes | The index loses names. |
| Lossy U+FFFD | No | No. `a\xFF` equals `a\xFE`. | Not applicable | Deno PR 34623. `display_portable` in Traces is already lossy (`src/path.rs:39-43`) [V]. |
| `OsStr::as_encoded_bytes` | Same Rust version and target only | Within one platform | Raw order on Unix [H] | Std docs call the format "unspecified, platform-specific, self-synchronizing superset of UTF-8". Non-UTF-8 sub-slices are "opaque and only comparable within the same Rust version built for the same target platform". Storing them in a file "will likely result in incompatible byte slices" [V] doc.rust-lang.org/std/ffi/struct.OsStr.html |
| WTF-8 | Ill-formed UTF-16 on Windows only | Yes | Code-point order [H] | It "represents ... text from systems such as JavaScript and Windows that use UTF-16 internally" [V] wtf-8.codeberg.page. It does not cover invalid bytes on Unix. |
| PEP 383 surrogateescape | Yes, bytes to `str` to bytes | PEP 383 calls conflicts with foreign data "out of scope" | No. U+DC80 to U+DCFF sort by code point [H]. | A Rust `str` cannot hold a surrogate [H]. UTF-8 encoding gives "nonsensical data" [V PEP]. A JSON `\uDEAD` has "unpredictable" behavior [V] RFC 8259 section 8.2. |
| Percent escape (`%FF`) | Yes | Only if a literal `%` also becomes `%25`. Then every name with `%` changes its key. | No. The key sorts as its own bytes. | Wheeler weighed `%` against `=` for this reason [V] dwheeler.com essay, about line 2990. |
| Noncharacter escape (U+FDD0 plus 2 hex digits) | Yes | Only if a real U+FDD0 in a name is escaped or rejected | The key sorts as its own bytes. Prefix ranges survive per component [H]. | Unicode says noncharacters are "intended explicitly for internal use". Corrigendum 9 removed the interchange ban, so they are valid in strings. [V] unicode.org/faq/private_use.html |
| Visible-quote escape (rclone: `0xFE` becomes `‛FE`) | Yes | The source report did not verify that the quote character is itself escaped. | Same as the row above | [V] rclone.org/overview/#invalid-utf8 |
| Base64 of the whole name | Yes | Yes | No | The name is unreadable. Folder prefix ranges break unless the code encodes each component [H]. |

## 3. Prevalence

The source report found no quantitative survey. Its scope was GitHub issue
searches, the Obsidian forum, vendor docs and man pages. A search on
DuckDuckGo was blocked.

Qualitative signals [V unless marked]:

- The camino README says non-Unicode paths are "vanishingly uncommon". It names
  "legacy codebases ... in encodings like Shift JIS". This is a claim, not data.
- rclone says: "A common source of invalid UTF-8 bytes are local filesystems,
  that store names in a different encoding ... like latin1" (`local/#filenames`).
- The `convmv` man page (Debian testing 2.06) says the tool exists "to switch
  over from old 8-bit locales to UTF-8 locales".
- Nextcloud issue 59056 (2026-03): `files:scan` failed on `\xF1` (Latin-1
  `ñ`) and on `'thefilename'$'\217''_08.avi.jpg'`. The maintainer wrote:
  "Perhaps we should hard fail on this rather than just log and continue".
- jj issues 9774 and 9002 (2026): users hit the case with untracked
  directories whose bad names they did not know. jj issue 8118: ZFS
  `utf8only=on` makes such names impossible to create.
- Nushell issue 11100: an accidentally created file, "pretty rare, but not
  negligible". The reporter could not list or delete it in Nushell.
- rust-analyzer issue 16914: a `0xFF` byte in a test file name inside a git
  repo.
- ZIP APPNOTE: names default to IBM CP437 unless bit 11 (EFS) is set. A legacy
  archive that a tool extracts without conversion is one input path for invalid
  bytes.
- A ripgrep test comment says invalid UTF-16 paths "appear incredibly rare in
  the Windows world".
- Syncthing issue 430 concerns NFC and NFD, not invalid UTF-8. These are
  different cases.

For note vaults, the report found no data. The Obsidian forum returned no issue
about non-UTF-8 file names. The trackers of zk, Foam, Zettlr, Logseq and
markdown-oxide returned none either. This holds for the queries that the report
ran.

Platform scope [V]: APFS refuses non-UTF-8 names (prep file
`e2-web-deep.md`, section "File-system normalization facts"). jj and Deno skip
the matching tests on macOS and Windows. Windows names are opaque `WCHAR`
sequences. An unpaired surrogate is possible but rare.

## 4. Percent-encoding and URI facts

Link side:

- A Markdown link destination can carry any octet as `%XX`. CommonMark does not
  decode it. "A conforming implementation can use a different renderer" [V]
  spec.commonmark.org 0.31.2, section 1.3.
- Wikilinks have no standard escape. The Obsidian docs describe wikilinks as raw
  text [V]. Note content must be UTF-8, so a raw 0xFF byte cannot appear in a
  link.
- Traces has no percent-decoding in `src/` [V, grep]. `LinkResolver::resolve`
  builds `Path::new(path_part)` from the text. It then runs a binary search
  over the indexed files (`src/index/inlinks.rs:365-367`). So `%20` does not
  resolve today.
- Resolvers differ on `%E9`:
  - `url@2.5.8` on Unix decodes to raw bytes, `%FF` included [V
    `src/lib.rs:3088-3102`].
  - On Windows the same crate needs valid UTF-8 [V `src/lib.rs:3172-3174`].
  - `vscode-uri` keeps an invalid triplet as the literal text `%E9` [V
    `src/uri.ts:666-685`]. So a Foam-style round trip loses data [H].
  - JS `decodeURIComponent` throws `URIError` on a malformed sequence [V MDN].
    The Obsidian behavior is unknown [H].
  - mdBook decodes only `%20` today [V PR 3162].
- Verdict of the source report: the argument "no link can reach such a note"
  is weaker for Markdown links in principle. It still holds for wikilinks and
  for every resolver that does not decode to bytes.
- A decode to bytes makes `%` ambiguous for real names. The mdBook PR text says
  that `question%3F.md` could not load `question?.md`.

URI side (LSP):

- An LSP `DocumentUri` is a string whose content "can be parsed as a valid URI"
  under RFC 3986 [V] LSP 3.17 section "URI".
- RFC 8089 section 4: file systems store names as "arbitrary sequences of
  octets". UTF-8 before percent-encoding is a SHOULD. "A decision not to use
  percent-encoded UTF-8 is outside the scope of this specification." [V]
- On Unix, `Url::from_file_path` percent-encodes raw bytes
  (`url@2.5.8 src/lib.rs:2968-2971`). `to_file_path` reverses it (`:3090-3102`)
  [V].
- Correction to the ticket premise: an LSP URI string can name such a file,
  because `file:///v/caf%E9.md` is ASCII. The failure is in client-side
  handling. JS clients decode with loss [H].

JSON and MCP side:

- JSON text must be UTF-8 [V RFC 8259 section 8.1]. MCP says "JSON-RPC messages
  MUST be UTF-8 encoded" [V] modelcontextprotocol.io 2025-06-18 transports.
- A raw byte cannot appear in a JSON string. Go and similar libraries coerce it
  to U+FFFD [V]. `Path::serialize` in `serde` returns the error "path contains
  invalid UTF-8 characters" [V serde_core@1.0.228 `src/ser/impls.rs:915`]. An
  unpaired-surrogate escape is "unpredictable" [V RFC 8259 section 8.2].
- So an MCP host can name such a file only through an escape or a lossy form
  inside a valid string.

## 5. Design pressure: skip, escape or enable

Status quo (Linux). The file is in the index but absent from tag, class and
folder results. `list` shows it nowhere. `--from` with such a name stops in
clap [V]. This is neither skip nor enable. In clap, `String` and `to_str()`
value parsers give `ErrorKind::InvalidUtf8` [V clap_builder@4.6.7
`value_parser.rs:884,932`; `error/kind.rs:351`]. `PathBufValueParser` takes an
`OsString`. ripgrep accepts invalid path arguments [V].

What "skip" needs:

- A single predicate (`to_str()`) must cover every OS-path ingress [P].
  rust-analyzer enforced it in the walker and still crashed in the watcher path
  [V issue 16914]. The scan exists today. Watcher events and LSP `didOpen`
  URI mapping are future ingresses [P]. The CLI `--from` and outlink targets
  already hold UTF-8 text (ticket 32, D7).
- A notice that prints the name. The options seen: lossy text (ty), a count in
  statistics (jj), and git-style quoted octal. A lossy print cannot tell two
  bad names apart.
- A change-table row for ticket 16, because the skipped files are a behavior
  change.
- A reserved U+FDD0 for a later escape adds a second skip class: valid UTF-8
  names that contain U+FDD0. Without that rejection, a real name
  `…U+FDD0 E9` equals an escaped `\xE9` [H].
- Skip makes the key form portable (UTF-8, `/` separators). That removes the
  platform family from the identity rule of ticket 25 [H].

What "escape" needs:

- A hybrid key. Valid names stay byte-identical to the skip design. Only
  invalid names use the escape [H]. Injectivity needs the reserved-character
  rule above.
- Two forms. The storage form (U+FDD0 plus hex) is a noncharacter, and printing
  it is probably not safe. Display needs a visible form (rclone `‛FE`, git
  octal). The terminal also needs sanitizing (fd does this [V]).
- A form that a user can type for `--from` and for path literals. Users do not
  type U+FDD0. The CLI parses an `OsString`, applies the escape and compares
  keys.
- An unescape step at the I/O boundary: key to `OsString` to `open()`. The step
  must refuse to give `/` or NUL. PEP 383 refuses bytes below 0x80 for security
  [V]. Escaping only bytes of 0x80 and above is the same rule [H].
- An LSP mapping: key to raw bytes to `Url::from_file_path` works on Unix [V].
  On Windows, the decode needs valid UTF-8 [V], so the mapping is Unix only.
  JS clients lose the round trip [H].
- MCP: output and input both carry the escaped string. The confinement checks
  of ticket 32 must run after the unescape.

What "enable" needs (full, byte-faithful):

- A key type of `OsString` or bytes, or an escaped UTF-8 string. Raw
  `as_encoded_bytes` in the store keeps today's machine-local store and the
  platform family. The std docs say not to store it across versions [V].
  Traces already writes platform-specific codecs (`src/path.rs:340-437`) [V
  per ticket].
- All steps in "escape", plus lossless JSON (impossible without an escape).
- A decision on percent-decoding in link targets. Traces has none today. A
  decoder changes how valid names with `%` resolve.
- A rule for each Windows lone surrogate.
- Links to such notes resolve only through percent-encoded Markdown links, and
  only if a decoder exists. Wikilinks cannot reach them. The notes stay
  queryable by tag, class and folder.
- Cost: `OsStr` comparison costs 0.12 ms against 1.7 ms for `Path` at 50k
  paths [V ticket 24 probe]. A `String` key also compares as bytes, so the
  report expects the same fast path [H].

Facts for the grill, with no ranking:

- Skip, escape and enable differ mostly in how many layers hold two path forms.
  Skip has one (UTF-8). Escape has storage, display and input forms. Enable adds
  bytes at the I/O boundary.
- The makefile problem applies to Traces directly. Notes name other notes by
  text. Mercurial and camino both give this as the reason why UTF-8 only is
  defensible. Mercurial, git and ripgrep take the byte route.
- No prevalence data exists for vaults. Every incident the report found is a
  Linux or WSL case with a legacy encoding or an accidental file. The reports
  of those incidents called each one a tooling failure.

Not verified by the source report:

- classic `ruff check` discovery;
- Zed, Hugo, zk, Foam, Logseq and Zettlr behavior on invalid names;
- the Obsidian runtime handling of `%XX` octets and invalid names;
- `fluent-uri` LSP crates. They are not in the local registry. The lock file
  has no `url` or LSP crate. So only the `url` crate behavior is verified.

## 6. IWE and Obsidian: comparison table

The IWE claims come from `iwe-org/iwe` master, fetched 2026-11-15. The `liwe`
CHANGELOG top entry was 0.33.0 of 2026-11-11. The report had no shell. It
executed nothing.

| Tool | Behavior for `a\xFFz.md` | Mechanism | Evidence |
| --- | --- | --- | --- |
| IWE (CLI, LSP, MCP) | The tool loads the file at startup under the lossy key `a�z`. The first watcher event or `didClose` drops it. A write through that key hits a different file. No step gives a message. | `to_string_lossy` on each component builds the key. The key goes back to a path as `base.join(key + ".md")`, so U+FFFD becomes a literal name. | [V] `crates/liwe/src/fs.rs:29-40`; `crates/diwe/src/watcher.rs` `path_to_key`; `crates/iwes/src/router/server.rs:114-145` |
| IWE, two names that differ only in invalid bytes | One name overwrites the other in the load map. No error and no log. | Both names give the same `String` key. `.collect::<State>()` fills a `HashMap<String,String>`. | [V] `crates/diwe/src/fs.rs` `new_for_path`; [H] for the winner (readdir order and last insert wins) |
| Obsidian desktop, Linux | Not documented. A developer says "obsidian uses UTF8. The filename encoding is managed by your OS." | Electron and Node `fs` probably decode names as UTF-8 and replace invalid bytes. Closed source. | [V] forum.obsidian.md/t/71622 (WhiteNoise, 2023-11-20); [V] Node docs; [H] for the vault adapter internals |
| Obsidian, files that collide after its own canonicalization (NFC, NBSP) | Reproduced for NFC and NFD, and for NBSP. It was not reproduced for invalid bytes. One note shows. The other is invisible. Edits go only to the kept file. No message appears. | The vault index canonicalizes the path to NFC and NBSP to space, so two disk names share one entry. | [V] forum.obsidian.md/t/95629 posts 4 and 5 (user-measured, v1.14.5, Windows 11; not vendor-confirmed) |
| Obsidian Sync | Skips files whose name the receiving OS cannot store. It says "Unable to download file with illegal name". The page does not mention invalid UTF-8. | A check on the receiving side | [V] obsidian-help `en/Obsidian Sync/Status icon and messages.md:87-99` |
| Logseq | The report found no note-name issue. One open PR keeps a literal `%` when a percent-decode is not valid UTF-8 (`50%ba.png`). That PR is for image paths. | A decode per segment that keeps the literal on failure | [V] github.com/logseq/logseq/pull/13563 |
| Foam | Not found in the searched material. The search was GitHub issues for `"invalid utf-8"` in `foambubble/foam`. The only hit was an unrelated Dependabot PR. | n/a | [V] search result |

## 7. IWE end-to-end trace

Directory walk:

- `liwe::fs::walk_md_paths` (re-exported from `diwe::fs`) uses
  `ignore::WalkBuilder` with `.follow_links(false).hidden(true)
  .require_git(false)`.
- The walk swallows errors with `entry.ok()?`. The only log call in the
  function is `error!("path doesn't exist")`.
- The extension test compares `OsStr` values and is byte-safe. So `a\xFFz.md`
  passes.

Key construction (`fs.rs:29-40`, confirmed verbatim):

```rust
let key = relative_path
    .with_extension("")
    .components()
    .filter_map(|c| match c {
        Component::Normal(os) => Some(os.to_string_lossy().to_string()),
        _ => None,
    })
    .collect::<Vec<_>>()
    .join("/");
Some((key, path.to_path_buf()))
```

The walk returns `(String, PathBuf)`, so the real path is still available at
this point. Directory components are lossy too.

Document load:

- `diwe::fs::new_for_path` runs `walk_md_paths(..).into_par_iter()
  .filter_map(|(key,path)| fs::read_to_string(&path).ok().map(|c| (key,
  sanitize_content(c)))).collect()`.
- It reads from the real path, so the content is correct. The key is the lossy
  one.
- A file whose content is not valid UTF-8 fails `read_to_string`. The load drops
  it without a message.
- `graph_from_path` (CLI and MCP) and the LSP server both use this load.

Collision:

- `State = HashMap<String,String>` is built with `collect()`. No duplicate
  check, error or log exists.
- `a\xFFz.md` and `a\xFEz.md` both become the key `a�z`. One entry survives.
  Which one depends on readdir order and rayon's ordered collect [H].
- IWE guards other collisions, so this gap is notable. `rename` fails with
  `AlreadyExists` when the new key differs only in case (CHANGELOG, `liwe`
  0.33.0, `iwe` Fixed list). `iwe init` reports "filename case collisions"
  (`iwe` CHANGELOG line 318).

Link resolution:

- Both link types percent-decode with a lossy UTF-8 decode. `Key::from_rel_link_url`
  in `liwe/src/model.rs` runs `percent_decode_str(without_fragment)
  .decode_utf8_lossy()`. It then strips the extension and joins the path to
  the linking document. A leading `/` resolves from the workspace root
  (`docs/keys.md`).
- Wikilinks use `decoded_name` in `liwe/src/model/key_index.rs:319-322`. It is
  the same decode.
- Wikilinks also match by case-folded suffix. `fold` is `to_lowercase`, and an
  exact-case match wins.
- So `[x](a%FFz.md)` decodes to `a�z` and "resolves" to the lossy key. So does
  `a%FEz.md`. A link cannot tell the two files apart.
- IWE does not tell `%20` from other escapes.
- IWE has no NFC or NFD normalization. The `NFC` issue search in IWE gave zero
  hits.

Watcher (`notify`), in `diwe/src/watcher.rs`:

- `path_to_key` builds the same lossy key. `dispatch` reads the real path with
  `read_md_file`.
- It emits `FsChange::Update(key, content)`.
- The router discards the content: `router.rs` has
  `FsChange::Update(key, _) => server.apply_external_update(key)`.
- `server.rs:114-145` then calls `reload_document_from_disk(key)`.
  `base_path.key_to_path(&key)` gives `…/a�z.md` (bytes `EF BF BD`). That file
  does not exist. `read_md_file` returns `None`. Then the arm
  `None if !path.exists() && graph.get_document(&key).is_some() =>
  drop_document(key)` runs.
- Result: the first change or remove event on the real file removes the note
  from the LSP graph. The server never adds a new bad-named file that appears
  while it runs.

LSP URI to key (`iwes/src/router/server/base_path.rs`):

- `document_key` calls `maybe_url_to_key`. That calls `Url::parse`,
  `canonical()`, `Url::to_file_path()`, `strip_prefix(base)` and
  `relative_path_key` (lossy for each component). The code uses `Url`, not
  manual parsing.
- `file:///v/a%FFz.md` gives a raw-byte `PathBuf` on Unix, then the key `a�z`.
  So `didOpen` maps to the lossy key. It replaces whichever file won the
  collision.
- `didClose` calls `reload_document_from_disk`, so the note drops (same path as
  above).
- `didSave` does nothing useful. The server advertises `include_text: false`.
- `iwes/src/main.rs` builds `base_path: workspace_path.to_string_lossy()
  .to_string()`. A non-UTF-8 workspace root is mangled. `walk_md_paths` then
  logs "path doesn't exist".

Writing, rename and create (key to disk):

- `workspace_document_path` guards `..`, a root path, and empty or `.`-prefixed
  segments. It then returns `base.join(format!("{key}.{ext}"))`.
- `apply_changes` removes or creates through that path. For a lossy key, the
  remove target does not exist, so the code skips it. The create succeeds at
  the U+FFFD name. So a `rename` of a lossy-keyed note creates a new file and
  leaves the original [H: derived from code, not run].
- `iwe update` in body mode (`crates/iwe/src/main.rs:3807-3843`):
  `read_to_string(&file_path).unwrap_or_default()` gives an empty string.
  `std::fs::write(&file_path, …)` creates a new `a�z.md`. Then it prints
  `Updated 'a�z'`.
- `update` in frontmatter mode (`main.rs:3959-3960`) runs
  `read_to_string(&file_path).ok()?`. It skips the document without a message.

CLI printing:

- `iwe find` and similar commands print `doc.key` (`println!("{}", doc.key)`,
  for example `main.rs:1890`) with U+FFFD.
- `iwe new` and `iwe create` print `doc.path.display()`. That output is lossy
  too.

Panics:

- The router wraps each request in `catch_unwind` and returns an internal error.
- `url_to_key` has `.expect("valid URI")` and `.expect("URI is a file URL")`.
  They can panic on bad input. Only non-file URIs trigger them [H].

IWE tracker and CHANGELOG:

- The report searched `utf-8`, `unicode`, `encoding`, `percent`,
  `normalization`, `filename`, `lossy` and `NFC`. It did not search `invalid`
  or `non-UTF`.
- No issue is about invalid UTF-8 names.
- Related items: issue 179 (Danish characters in names, closed by PR 185, `tøj`
  round trip), issue 243 (base path with spaces, PR not merged), and issue 339
  (root-absolute links and fragments, now fixed per `docs/keys.md`).
- CHANGELOG: `%23` in a link path is part of the key (`liwe` and `iwe` 0.33.0).
  Wikilink lookup ignores case.
- The docs (`docs/keys.md`, `spec.md` grep) say nothing about non-UTF-8 names.

What IWE does with `a\xFFz.md`:

1. It indexes the file as `a�z` at startup, with correct content.
2. It lists the file as `a�z`. Every command that writes through the key
   targets a file that does not exist.
3. In LSP, the first file system event or `didClose` drops the note from the
   graph.
4. No notice appears anywhere.

For two names that differ only in invalid bytes, one overwrites the other in
the load map, without a message.

## 8. Obsidian findings

Documented policy:

- The help says Obsidian "will respect the filename limitations of the
  operating system you create the note on" (`Manage notes.md`).
- Forum staff, 2021-08-15 (topic 22543): "we are OS specific now" and "If you
  create a file with a different casing outside obsidian … It's your files."
- The same topic shows that `[[Testing/Test links upper- and lowercase]]` and
  the `Test Links…` spelling both resolve to one file (v0.12.12).

Non-UTF-8 names:

- Not documented. The forum searches `invalid utf-8 filename`, `non-UTF-8 file
  names linux`, `filename encoding latin-1 OR mojibake` and `replacement
  character` returned no relevant topic.
- Scope: `forum.obsidian.md/search.json` only. The report has no search of the
  `obsidian-releases` GitHub issues.

Node behavior:

- `fs.readdir` takes `encoding`, with `'utf8'` as default. With `'buffer'` it
  returns `Buffer` names (`doc/api/fs.md` around lines 1886-1899).
- `buffer.md` line 146: invalid UTF-8 decodes to U+FFFD.
- Obsidian desktop almost certainly uses the default, so names are lossy [H].
  This is an inference, not a measured Obsidian result.

Plugin API (`obsidianmd/obsidian-api` `obsidian.d.ts`):

- `TAbstractFile.path: string` and `DataAdapter.list(normalizedPath):
  Promise<ListedFiles>` are JS strings. They cannot express invalid bytes.
- `normalizePath` "uses forward slashes, removes duplicate, leading and
  trailing slashes, replaces non-breaking spaces with regular spaces, and
  applies Unicode NFC normalization".
- `getFirstLinkpathDest(linkpath, sourcePath)` resolves links to a `TFile`.

Canonicalization is the closest precedent to a collision. Forum 95629
(2025-06 and 2026-10-27, v1.14.5, Windows):

- A non-NFC name is canonicalized inside the app. Post 4: `getFullRealPath(F.path)`
  returns the NFC path while the disk holds the NFD name.
- If both spellings exist, the vault shows one note and "the last file written
  wins". The other file "appears nowhere".
- NBSP against space behaves the same way. The edit goes only to the kept file.
  No message appears.
- Forum 117189 (2026-08): in 1.13.5 and 1.13.6 the watcher compared a raw NFD
  event name with an NFC listing. Edited NFD-named notes vanished from the
  explorer until a restart.

Forbidden characters:

- The Sync skip-message page lists `/ \ : * ? " < > |`, a trailing space or
  period, the Windows reserved names `CON PRN AUX NUL COM1-9 LPT1-9`, `# ^ [ ]`
  for link-friendly names, and multiple periods or emoji for some Android
  devices.
- The internal links page says `# | ^ : %% [[ ]]` "may not work as a link".
- The rename popup (forum 120767) says: "Links will not work with file names
  containing any of these characters: `# ^ [ ] |`".
- Sync deletes a file on Windows that another device renamed to include
  `< > |`. The rejection shows in the Sync log.

Links:

- The help says Markdown destinations must be URL-encoded ("blank spaces
  become `%20`"). Wikilinks are raw text. Folder paths use `/`.
- The Obsidian handling of `%E9` or other non-UTF-8 octets is not documented.
  Forum searches for `decodeURI` and `%C3 encoded accents` found nothing [H].

Sync and Publish:

- Sync skips by the receiving OS, as above.
- The Publish limitations page does not mention file names.
- Mobile: only the Android note above (multiple periods, emoji).
- The Obsidian handling of invalid bytes is [H] throughout.

## 9. What the evidence implies for skip, escape and enable

The two reports give evidence only. They do not rank the options.

- Lossy keys are not a stable middle ground. IWE shows both failures:
  - A silent collision. IWE guards case collisions but not this one.
  - A key that does not round-trip to the disk path. Writes miss the real file.
    The LSP graph drops the note on the first event. `update` and `rename` act
    on a different path.
  - Obsidian canonicalization (NFC, NBSP) has the same two failures: a
    shadowed file and a wrong write target. The forum reports call both bugs.
    The report found no vendor reply.
- Skip. Obsidian Sync already skips unstorable names and logs them as
  "Skipped". The IWE failures come from names that the tool loads but cannot
  map back. They do not come from skipped names. A skip must apply at every
  ingress, because the IWE startup walk, watcher, `didOpen` and link decode each
  repeat the lossy conversion.
- Escape. No tool in this survey escapes invalid bytes into keys. Logseq PR
  13563 keeps a literal `%` when a decode is not valid UTF-8, but only for
  asset paths.
- Enable. Obsidian and IWE both store keys as UTF-8 strings. Neither key type
  can hold invalid bytes. IWE retains the real `PathBuf` during the walk [V].
  `TFile.path` in Obsidian is a JS string [V].
- Link resolution. IWE decodes with loss, so `%FF` and `%FE` resolve to one
  key. The Obsidian behavior is unknown. A scheme that gives distinct links to
  distinct bad names must decode to bytes, not text.
- Prevalence. Neither the IWE tracker nor the Obsidian forum shows a report of
  this case.

## 10. Evidence limits

- No prevalence data exists for vaults. All incidents are Linux or WSL cases.
- The IWE report ran no program. It read the code [V] and inferred the runtime
  outcomes [H].
- The Obsidian runtime behavior is closed source and has no reproduction [H].
- Not checked: Foam and Logseq note-name handling, Electron on Linux, and the
  `obsidian-releases` GitHub issues.
- The Linux listing facts for Traces come from `agent://RedTeam32`. See
  `h3-redteam-round2.md`.
