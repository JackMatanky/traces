# Stress-test designs for ticket 32: path and capability model

Rules: candidate designs, requirement checks, failure modes, inconsistencies.
Tags [V]/[H]/[P]. Cite `path:line` or ticket lines. Do not preset the Answer.

## Candidate designs

### D1: boundary-only validation, `ProjectRoot` authority, raw paths inside

One owner (`ProjectRoot`) resolves the canonical root, joins safely, and
writes files. Inside core, structs carry plain relative path strings.

Checks:

- R-research 09: matches the refined recommendation; `cap-std` handles do
  not fit a local CLI [V]. IWE checks escape in one place before I/O [V]
  (research 12, `key_escapes_workspace`).
- Failure mode: the TOCTOU window stays (strict_join validates; a symlink
  swap redirects the next open, `src/path.rs:159-161`) [V]. The stance must
  say which boundaries accept it: note reads (single user), template output
  (untrusted input, maybe close), template inspection (already a leak,
  `src/template/engine/path.rs:161-170`).
- Failure mode: raw strings re-enter. Without a type, a caller can pass an
  absolute path to a join and get silent escape at the next boundary. The
  `ProjectRoot` API must make the raw-string path hard: join takes only
  validated input [H].

### D2: validated newtype propagation (`RelPath` everywhere)

A `RelPath` newtype (lexical validation at construction) flows through
`FileMeta`, index rows, query paths, and template paths. `ProjectRoot`
converts to real paths at I/O.

Checks:

- R-ticket 24/29: persisted rows need a byte form; the newtype defines it.
  Portable UTF-8 with `/` is the relative-path crate shape [V]. Non-UTF-8
  Unix names cannot round-trip [V] (research 09). That loss must be a named
  consequence, and the scan must skip or notice such files.
- R-ticket 25: a portable form drops the platform family from the store
  identity, with a new format epoch [V].
- R-ticket 24 sort-order check: byte order of the UTF-8 form must match the
  in-memory comparison used by the release check and by the folder range.
  SQLite4-style prefix-terminated text gives byte order for free [V]. The
  current `notes/` vs `notes-x.md` divergence disappears only when both
  sides compare the same form [V].
- Failure mode: the glossary conflict. `src/GLOSSARY.md:18-20` says
  "Avoid: relative path" while `RelativePath` exists (`src/path.rs:64`) [V].
  D2 makes the glossary fix mandatory (standing obligation 5).

### D3: capability handle (`Dir`-style) at every I/O

Every file operation takes a root handle. Full `cap-std` plumbing.

Checks:

- R-research 09: prices this as disproportionate for a local CLI [V]. True
  TOCTOU immunity costs an open per path component on macOS and older Linux
  [V]. Windows needs NT calls; macOS has none in std [V].
- Partial closure variant: use `openat2(RESOLVE_BENEATH)` on Linux only.
  Per-platform stance; research 09 says state it per boundary [V].
- Keep D3 as the rejected extreme; cite it when the grilling asks "why not
  close it everywhere".

## Form decision space (bullet 4 of the ticket)

| Option | bytes | portable | non-UTF-8 | order |
| --- | --- | --- | --- | --- |
| OS bytes (today) | raw | no, platform family stays | round-trips | Path::cmp in memory, bytes in store |
| UTF-8, `/`, NFC | utf-8 | yes, drop family | lost, skip+notice | byte order = range order |
| UTF-8, `/`, no fold | utf-8 | yes | lost | byte order; NFC and NFD stay distinct |

- Case rule: folding keys collide ignorable-codepoint and case pairs. The
  Deno CVE-2026-49401 advisory proves byte-compare against a folding file
  system is a bypass class [V]. Recommendation shape: keys never fold; NFC
  on write as canonical spelled form, no lowercase [H].
- The comparison key of the src glossary (NFC, lowercase, NFC) is for
  name matching, never for store keys or sort order [V] (glossary text).
  Keep that separation explicit in the Answer.

## Journal form consistency

- The journal must stay deduplicable across `cp -R` (discard) and `mv`
  (replay) [V] (ticket 27). A portable form makes `mv` replay keep working;
  `cp -R` changes root strings, so the base identity check discards. Same
  behavior as today, now with portable bytes.
- Journal path form must equal the store form; two forms would need two
  epochs and two codecs [H].

## Requirement table

| Req | Source | D1 | D2 | D3 |
| --- | --- | --- | --- | --- |
| one confinement owner | ticket question | ProjectRoot | ProjectRoot + type | handles |
| persisted form portable | t24/t25 | no | yes | yes |
| sort order stated | t24 | memory-only | byte order both sides | byte order |
| TOCTOU stance named | ticket question | accept per boundary | same | closed, costly |
| non-UTF-8 names | research 09 | kept | lost, skip+notice | kept |
| glossary fix | obligation 5 | optional | mandatory | optional |

## Inconsistencies to resolve at grilling

1. Store path type vs row type: the 24-prep guide proposes a store path type
   owning `Arc<FileEntry>` rows [P]
   (`research/24-prep/z-grilling-guide.md:264-266`), but ticket 29 owns the row
   type. The split: ticket 32 owns the path form and its key bytes; ticket 29
   owns the row that hangs off it.
2. Template inspection leak vs the invariant: endorsing absolute-path
   inspection weakens "all file access stays inside the root". Closing it
   breaks `path_exists('/usr/bin/pandoc')` checks [V] (survey H4). The
   honest options: a confined inspection namespace, or an endorsed raw-path
   test type with a documented name.
3. `.traces` as a symlink (F11): root-keyed trust keeps grants separate even
  then [V] (`research/22-prep/j-attack-unified-config-store.md`, 2.6). The
  grilling may allow it with the root key as the guard. Decide or name the
  owner.
4. `DirTree` and `file_tracker`: the tracker canonicalizes and hashes
  (`src/file_tracker.rs:469-483`); `contains` uses `try_exists` only and
  misses symlink swaps [V]. State whether tracker paths keep raw `PathBuf`
  (state-home is outside the vault) or adopt the newtype. The state-home
  stores are not vault-confined; raw `PathBuf` there is acceptable [H].
