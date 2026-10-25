# Research: Index and note substrate, candidate guarantee inventory (ticket 03)

Breadth-first survey of `src/index/` and `src/note/` asking which observable behaviors are *intentional
guarantees* vs incidental. Entry format follows [issue 01](../issues/01-surface-workflows.md): guarantee
(observable contract), then one meta line: class (invariant / state transition / workflow / error semantics /
edge case), verdict with evidence, pinned today (an existing test enforces it), Gherkin-suitability.

**Sources**: `src/index/*` and `src/note/*` (`//!` docs and inline tests), `tests/integration/{persistence,refresh}.rs`,
`tests/e2e/{index,tracked}.rs`, `src/index/GLOSSARY.md`, `src/note/GLOSSARY.md`, `docs/adr/0009-index-refresh.md`,
`docs/adr/0009-filter-date-literals.md`, `docs/adr/0005-template-query.md`,
`.scratch/arch-design/research/05-freshness-fingerprint-precedent.md`, open arch-design tickets 23/24/25/32/34/35/36.
Citation: `file:line`, this repo.

**Test counts confirmed**: `src/index` = **208** `#[test]`, `src/note` = **304** `#[test]` (161 in `src/note/*.rs`
+ 143 in `src/note/parser/*.rs`). Both ticket figures exact.

**Structural note**: no ADR covers `src/note/` parsing; ADR-0009 (index refresh) is still `status: proposed`
(`docs/adr/0009-index-refresh.md:5`) although its behavior landed (`research/05` re-verification note).
Note-level guarantees rest on GLOSSARY + `//!` docs + tests only.

---

## A. Persistence-level guarantees (redb store, refresh, drift)

**P01 · Refresh-on-read.** Any query-path command syncs the store before serving; edits/deletes/renames need no
explicit `traces index`, and a cold `list` creates `.traces/index.redb` itself.
- workflow · intentional · **pinned**: `tests/e2e/index.rs:19,52,109,225`; production path `src/cli/mod.rs:344`
  · Gherkin: good scenario candidate (stdout-observable).

**P02 · Query-visible delta.** After refresh, the new tag resolves, the removed tag vanishes, rewritten task text
returns; observing must not mutate.
- state transition · intentional · **pinned**: `tests/integration/refresh.rs:31` ("stale-secondary-index")
  · Gherkin: good scenario candidate.

**P03 · Path-set change forces full link recompute.** Any add/delete/rename invalidates every wikilink resolution:
ghost rows pruned, an ambiguous `[[foo]]` re-resolves after its sibling dies, a renamed target keeps inlinks,
unchanged notes still contribute inlinks.
- state transition · intentional · **pinned**: `src/index/service.rs:118-120`, `src/index/refresh.rs:412-460,477-485`,
  `tests/integration/refresh.rs:98`, `tests/e2e/index.rs:52` · Gherkin: good scenario candidate (file ops +
  `table --column inlinks`).

**P04 · Content-only delta backdates links.** An edit that leaves the outlink target set unchanged reports
`links_modified == 0` and skips inlink recompute; relabeling text with same targets also backdates.
- invariant · intentional · **pinned**: `src/index/service.rs:1354,1407` · Gherkin: poor fit (report counts
  internal, see P24).

**P05 · Freshness = full `FileMeta` equality (path, size, mtime, format).** Size **or** mtime change triggers
reparse; if both match, the store is served as-is, content hash never consulted.
- state transition · intentional · **pinned**, both directions: `src/index/delta.rs:42-48`, `src/file.rs:45-53`;
  reparse tests `src/index/service.rs:1452` (size), `:1480` (mtime, same size); invisibility of a same-size
  mtime-restored edit pinned as canary `tests/e2e/index.rs:147` ("unobservable-reload") · Gherkin: good scenario
  candidate, mtime variant needs filetime-style setup.

**P06 · Racy-timestamp blind spot.** An edit holding size and mtime inside one filesystem tick is missed: no
`racy` handling in `src/index` (grep), `delta.rs` untouched; tests mask it with a 15 ms sleep and a
`clippy::disallowed_methods` expect (`src/index/service.rs:1471-1490`).
- edge case · intentional-adjacent defect, **open decision** (arch ticket 25 item 4) · **not pinned** (no test
  asserts the miss) · Gherkin: poor fit (timing); unit/property if a content-hash fallback lands.

**P07 · RefreshReport semantics.** Three variants; count accessors are 0 for non-`FilesChanged`; drift reports
its cause, never counts; first build into a virgin store = `FilesChanged`; second refresh and post-heal refresh =
`Unchanged`.
- state transition · intentional · **pinned**: `src/index/refresh.rs:32-50,61-110`; `src/index/service.rs:1996,2029,2046`
  · Gherkin: poor fit as stated (internal enum; observable projection is P01/P08).

**P08 · Config drift heals on the next refresh.** Changing an index-relevant setting (e.g. `class_field`) changes
query results on the next spawned query with no `traces index` between; drift wins over simultaneous file changes;
the refresh after the heal reports `Unchanged`.
- state transition · intentional · **pinned**: `tests/e2e/index.rs:248`; `src/index/service.rs:2010`
  (`lets_drift_win_over_file_changes`), `:1862` (class axes healed), `:1996`; drift checked first
  `service.rs:208-218` · Gherkin: good scenario candidate (config edit to query output).

**P09 · Fingerprint composition.** Exactly 10 of 15 config settings enter `IndexFingerprint`; `new` destructures
`ConfigDigests` with no rest pattern so a new setting fails to compile until handled; root/template/schema-dir
excluded so moving the project does not rebuild; a `syn` source-scan test pins the convention.
- invariant · intentional · **pinned**: `src/index/fingerprint.rs:12-18,139-141,158-201`, exclusion test `:313`,
  scan `:505-549` · Gherkin: poor fit (compile-time/source property).

**P10 · Fingerprint persisted atomically with rows.** The raw fingerprint rides in the `meta` table in the same
write transaction as the rows; read on rebuild, replaced on rebuild, recorded at first incremental persist;
damaged bytes read as unreadable.
- invariant · intentional · **pinned**: ADR-0009 `:60-61`; `src/index/store.rs:557,671`; `store.rs:1971,2001,2030`
  · Gherkin: poor fit (inside transactions).

**P11 · Drift vocabulary: virgin never drifts, missing fingerprint = `Unrecorded`.** Rows with no readable
fingerprint fail closed as `DriftCause::Unrecorded` without naming settings; a rowless store never drifts.
- state transition · intentional · **pinned**: `src/index/fingerprint.rs:226`, `src/index/GLOSSARY.md:34-50`,
  ADR-0009 `:46-48`; `store.rs:1963,2030`, `service.rs:2060` · Gherkin: good scenario candidate (corrupt the meta
  row, observe rebuild).

**P12 · `load()` is as-persisted: no freshness, no drift.** `load()` returns disk state even under changed config
and has no production caller.
- invariant · intentional · **pinned**: ADR-0009 `:91-93`; `src/index/service.rs:397-403`
  (`expect(dead_code, ...)`); `service.rs:2093` · Gherkin: poor fit (test-only seam that `tests/integration/*`
  observes through).

**P13 · Fail-open vs fail-closed refresh.** `refresh()` absorbs persist failure by logging and materializing
in-memory; `refresh_store()` propagates `IndexError::Store` because callers read the store with no fallback.
- error semantics · intentional · **pinned**: `src/index/service.rs:255-307`, `src/index/mod.rs:48-61`,
  `src/index/refresh.rs:689` · Gherkin: poor fit (needs injected write failure).

**P14 · Corruption self-heals at open/query.** Garbage in `.traces/index.redb` never panics: next query exits 0
with correct output, recreates a valid database, accepts writes; refresh after corruption upserts everything and
deletes nothing.
- error semantics · intentional · **pinned**: `tests/e2e/index.rs:195` ("panic-on-corruption"),
  `tests/integration/persistence.rs:249`; `src/index/store.rs:894` · Gherkin: good scenario candidate.

**P15 · Format/schema evolution = wipe-and-rebuild at open, not `Drifted`.** `META_VERSION` mismatch
(`src/index/store.rs:41`, hand-bumped `b"1"`), wrong-typed table, missing `NOTES`, outdated marker, legacy
`cache_format`, old str-key `files` schema all recreate; all-tables-absent is fresh, not rebuild.
- state transition · intentional in code, **contradicted by ADR (D-04)** · **pinned**: `src/index/store.rs:872,927-1004`;
  tests `store.rs:1818,1839,1864,1909,2793,2864` · Gherkin: good scenario candidate, brittle to redb bumps.

**P16 · Row-level error policy.** Undecodable row skipped with warning, siblings survive; genuine read failure
propagated, never silently omitted; corrupted previous `notes` row fails `refresh` open.
- error semantics · intentional · **pinned**: `src/index/store.rs:968-986`; `store.rs:2875,2896,2978` · Gherkin:
  poor fit (poisoned row bytes).

**P17 · Persist replaces, never appends.** Rebuild overwrites every row; incremental persist deletes a removed
file's `notes` row from disk; links/files absent from the new set drop.
- invariant · intentional · **pinned**: `service.rs:1140,1186`, `store.rs:2548,2561` · Gherkin: good scenario
  candidate (delete file, query shows nothing).

**P18 · Path sort order is an invariant of every materialized view.** Entries, delta slices and store reads are
ascending by path, but `assumed_sorted` only `debug_assert`s, so release builds enforce nothing.
- invariant · intentional · **pinned in debug**: `src/index/sort.rs:48-57`, `src/index/delta.rs:10-14`,
  `src/index/entry.rs:33-41`; `store.rs:2738`, `service.rs:485`, `tests/integration/refresh.rs:117-129` · Gherkin:
  poor fit (`list` stdout is the ordering observable).

**P19 · Non-Unicode paths survive byte-exactly.** Keys use `Path::as_encoded_bytes` and round-trip; lossy
`path_from_bytes` (`src/index/codec.rs:93`) is confined to key-decode lookups, never `LINKS` edges.
- invariant · intentional · **pinned**: `src/index/codec.rs:68`; `store.rs:2598,2613,2635,2675,2711` · Gherkin:
  poor fit (platform fixtures).

**P20 · Inlink graph derivation.** Repeated outlinks in one note dedupe to one edge; sources sorted ascending;
self-links safe; unresolvable outlinks ignored; notes with no inbound links omitted; attachments are valid targets;
every target has >=1 source.
- invariant · intentional · **pinned**: `src/index/inlinks.rs:1-7`; `inlinks.rs:863,875,890,903,940,971,995`
  · Gherkin: good scenario candidate via `table --column inlinks`.

**P21 · Wikilink resolution order.** Exact relative path, then implied `.md`, then nearest basename by folder (LCA)
distance; qualified paths skip basename fallback; equal-distance ties stay unresolved; URL/anchor-only never
resolves.
- invariant · intentional · **pinned**: `src/index/inlinks.rs:10-17`, `src/index/GLOSSARY.md:89-95`;
  `inlinks.rs:488,502,512,552,614,625,753,782,796` · Gherkin: good scenario candidate (ambiguous basename fixture).

**P22 · Scan scope.** `.git`, symlinks and the store's own `.traces/index.redb` skipped; root's own name ignored;
empty root = no files; unreadable directory = `IndexError`; indexing never rewrites source markdown.
- invariant · intentional · **pinned**: `service.rs:738,797,811,826,843,853,678` · Gherkin: good scenario
  candidate for skip rules, unit for errors.

**P23 · Store plumbing facts.** DB at `.traces/index.redb` (`src/index/mod.rs:45`); directory at that path
rejected (`store.rs:2760`); unwritable parent = IO error (`store.rs:2777`); rows are postcard bytes, not TOML text
(`store.rs:2162,2947`).
- invariant · intentional · **pinned** as cited · Gherkin: poor fit (encoding).

**P24 · Production observation seam.** CLI queries run `refresh_store()` plus store-backed execution;
`refresh()`/`load()` serve tests/benches. Integration tests observe through a test-only seam, e2e through
production.
- workflow · intentional (documented) · `src/cli/mod.rs:344`, `src/index/service.rs:397-403`,
  `src/index/mod.rs:11-17` · Gherkin: design constraint, prefer spawned-command scenarios for visible behavior
  (ticket 10).

**P25 · `index` populates the tracked-config store.** `tracked list` / `tracked clean` only see entries after a
load-triggering command such as `index`.
- workflow · intentional · **pinned**: `tests/e2e/tracked.rs:3-8,21,42` · Gherkin: good scenario candidate
  (borderline: config domain, ticket 01).

---

## B. Parsing-level guarantees (`src/note`)

**N01 · `parse_markdown` is infallible.** Returns `Note`, never `Result`; malformed YAML frontmatter yields empty
fields; absent/whitespace-only frontmatter yields none.
- error semantics · intentional · **pinned**: `src/note/parser.rs:106`, `parser.rs:808,836`, `metadata.rs:230`;
  "infallible parser" named in arch ticket 23 · Gherkin: good scenario candidate (malformed `---` still queries).

**N02 · Frontmatter mapping semantics (YAML = `noyalib` via `crate::yaml`).** Top-level sequence = `NotMapping`;
invalid `FieldKey`s skipped; last duplicate key wins; merge keys stay literal; YAML 1.1 ambiguous scalars stay
strings; ISO date value = typed Date; `get` matches the canonical key.
- edge case · intentional · **pinned**: `src/note/metadata.rs:56-70,238,257-362` · Gherkin: good scenario
  candidate (outline over YAML fixtures), crate-owned (C-10).

**N03 · Headings.** ATX and setext in document order with (level, markup-stripped display text, source line);
levels 1-6; headings inside blockquote/list retained; empty display text retained; math option only for headings.
- invariant · intentional · **pinned**: `src/note/parser.rs:68-72`, `parser.rs:649,659,692,713,731,751,772`
  · Gherkin: good scenario candidate.

**N04 · Lists.** Strict pre-order document order; depth/line/parent chain through arbitrary nesting (grandchildren
beyond 2 levels); top-level siblings distinct lines with no parent; soft breaks kept in item text.
- invariant · intentional · **pinned**: `parser.rs:998,1015,1027,1054,1066,1133,1229` · Gherkin: good scenario
  candidate (outline to query rows).

**N05 · Inline-field extraction and scoping.** Declared-form extraction; fields scoped to their own item, never
siblings/parents; parent-before-child order; body-vs-list document order preserved; code fences and inline code
excluded.
- invariant · intentional · **pinned**: `parser.rs:1343,1379,1424,1546,1579,1587,1607,1622,1659`,
  `parser/list.rs:600` · Gherkin: good scenario candidate (query by field value).

**N06 · Inline-field syntax forms.** Bare `Key:: Value`, `[Key:: Value]`, `(Key:: Value)`; multi-word keys only
delimiter-bounded; empty value allowed; value trimmed; matches ordered by position in one pass.
- invariant · intentional · **pinned**: `src/note/parser.rs:88-90`, `parser/lexer.rs:347,366,375,413,577,606,724,753`
  · Gherkin: good scenario candidate (outline of forms).

**N07 · Typed inline values.** `null` case-insensitive; non-null keywords, NaN and Infinity rejected; comma lists;
quoted strings with escapes; Dataview link/wikilink/embed values (commas inside targets survive); HTML links stay
text; durations; ISO dates need valid calendar and a boundary; tags need `#`.
- invariant · intentional · **pinned**: `parser/inline.rs:301,318,360,372,391,414,427,447,460,468,476`,
  `parser/lexer.rs:443,457,471,481,536,546,556,624` · Gherkin: good for visible subset, unit for rejects.

**N08 · Tags.** Body and list-item extraction; hierarchical (`#projects/active`); mid-word `#` and non-alpha
initials skipped; code regions excluded; level isolation; tags survive into clean text unless filtered.
- invariant · intentional · **pinned**: `parser.rs:1553,1560,1572,1622,1643`, `parser/lexer.rs:808,819,825`,
  `parser/task.rs:443`, `parser/list.rs:691` · Gherkin: good scenario candidate (`list --from #tag`).

**N09 · Task markers.** Default markers classify to statuses; unknown marker still = incomplete Task; bracket text,
leading inline markup/code and link lookalikes stay plain; no Unicode whitespace after `]`; multibyte symbols OK;
marker recognized across event chunks; buffer overrun flushes plain.
- edge case · intentional · **pinned**: `parser/marker.rs:591,616,627,640,657,690,708`,
  `parser/task.rs:528,552,566,606,640,652,666,692` · Gherkin: good for marker cases, chunk splits are unit material.

**N10 · Task vs checkbox via `tag_filters`.** Empty filter = every marked item is a Task; a marked item must match
a filter, else it is a checkbox; exact match on nested tags, **no prefix match**; any of multiple tags/filters
qualifies; unmarked items stay plain.
- invariant · intentional · **pinned**: `parser/task.rs:704,752,767,783,802,819,835,850` · Gherkin: good scenario
  candidate (config x note matrix).

**N11 · `fully_complete` subtree semantics.** Childless leaf task = true; true iff every descendant task is done or
cancelled; incomplete, in-progress or unknown-marker child = false; plain bullets and non-task checkboxes ignored.
- invariant · intentional · **pinned**: `parser/task.rs:872,890,899,910,921,930,940,957,976,995,1006,1016`
  · Gherkin: good scenario candidate.

**N12 · Task text normalization.** `raw_text` and `clean_text` both kept; clean strips emoji dates (incl.
variation selector), priority, out-of-filter tags and inline task fields **in a prescribed order**; emoji in code
ignored; invalid dates kept intact.
- invariant · intentional · **pinned**: `parser/task.rs:434,451,458,471,483`, `parser/list.rs:670,712,737,747`
  · Gherkin: good scenario candidate (task output shows clean text).

**N13 · Priority and date precedence.** Priority emoji beats inline field; earliest priority emoji wins; explicit
"normal" = `None`; invalid falls through. Task dates: emoji beats inline field; invalid calendar and alphanumeric
terminators rejected; multibyte text never panics.
- edge case · intentional · **pinned**: `parser/task.rs:1046,1077,1085,1103,1135,1161,1170,1179,1191` · Gherkin:
  poor fit (precedence table, unit).

**N14 · Task emoji shorthands are task-item-only.** `📅`-style shorthands extract only from task items, never
plain items or surrounding text; repeated slots keep the first emoji; emoji beats inline field; code regions
excluded.
- invariant · intentional · **pinned**: `parser.rs:1424,1457,1488,1505,1530,1536,1546`, `parser/task.rs:318,355,368`
  · Gherkin: good scenario candidate.

**N15 · `NoteFieldValue` total order (the query-sort contract) and 80-byte budget.** Kind rank; `NaN == NaN`;
`±0` equal; spelling-insensitive durations; bare date = midnight datetime but orders before an afternoon one; list
prefix below its extension; links by target then text; objects by sorted key sets.
- invariant · intentional · **pinned**: `src/note/field.rs:938,971,978,985,996,1007,1040,1058,1070,1096,1130,1147`,
  budget `field.rs:579`, shared with `src/query/sort.rs` (`field.rs:206`) · Gherkin: poor fit (algebra; sort output
  is ticket 04 material).

**N16 · Field collection semantics.** Frontmatter yielded before inline; frontmatter **wins** on duplicate keys;
`Note` postcard round-trip preserves populated collections.
- invariant · intentional · **pinned**: `src/note/model.rs:352,379,574` · Gherkin: good scenario candidate (query
  returns one value).

**N17 · Link shape.** Raw target split at the **first** `#` into path and anchor; path/anchor/basename
classification; `embedded` flag for `![[...]]`; escaped pipe in wikilink target; invalid forms rejected.
- invariant · intentional · **pinned**: `src/note/links.rs:387,407,431,445,458,469,476,489` · Gherkin: good
  scenario candidate (anchor into P21).

**N18 · Wire-format compatibility.** Frozen pre-redesign postcard bytes of a full `TaskListItem` must keep
decoding; the test notes cached postcard indexes carry **no automatic format marker**, making it the tripwire for
field add/remove/reorder on the serialization path.
- invariant · intentional · **pinned**: `src/note/lists.rs:1014-1065` · Gherkin: poor fit (byte fixture),
  migration material (ticket 12).

---

## C. Refactor-hostile guarantees

Open arch-design tickets 22-36; stressing ones: **23** note document model, **24** index read model, **25**
freshness and invalidation, **32** crate topology, **34** live buffer vs persisted index.

1. **redb error taxonomy.** `is_rebuild_trigger` enumerates `redb::TableError` variants (`store.rs:872`);
   corruption detection matches redb's exact string `"Not a redb database: magic number mismatch"`
   (`store.rs:894-925`); e2e corrupts bytes after the 9-byte magic (`persistence.rs:259-264`) and opens with
   `redb::Database::open` (`tests/e2e/index.rs:209`). Breaks on a redb bump (`Cargo.toml:138`, `redb = "4.3.0"`).
2. **`META_VERSION` wipe vs ADR `Drifted`.** Hand-bumped (`store.rs:41`), mismatch wipes at open
   (`store.rs:927-1004`) before any `RefreshReport`; arch ticket 25 item 2 may fold format version into the
   fingerprint, flipping P15's tests to a drift report.
3. **Fingerprint constructor plus source-scan test.** The `syn` scan reads `fingerprint.rs` itself
   (`fingerprint.rs:549`), so any behavior-preserving rewrite of `IndexFingerprint::new` fails it; `ConfigDigests`
   shape is owned by config (arch ticket 22, open).
4. **Four multimap secondary tables and store pushdown.** `PATHS_BY_TAG`, `PATHS_BY_FILE_CLASS`, `TAGS_BY_PATH`
   (`src/index/tables.rs:39-60`) back "bare and prefixed tags resolve identically" and "frontmatter-only tags
   indexed" (`store.rs:2063,2075`). Ticket 24 weighs dropping pushdown ("drop 4 tables"), relocating or deleting them.
5. **Whole-`Note` postcard persistence.** Round trips (P18, N18, `store.rs:2240,2311`) assume `Note` is the stored
   unit; ticket 24 weighs "extracted facts", which would force the frozen byte fixture (`lists.rs:1020`) to be
   re-frozen by design.
6. **Debug-only sort witness.** `assumed_sorted` only `debug_assert`s (`sort.rs:48-57`); ticket 24's read-model
   change (partial index, lazy views) can violate sortedness silently in release.
7. **Metadata-only freshness plus its canary.** E2e `same_size_mtime_restored_edit` (`tests/e2e/index.rs:147`)
   pins current blindness; ticket 25 item 4 (content-hash/racy guard) would invert that assertion. Spec it as a
   known-limitation scenario, not a forever guarantee.
8. **`Note` model shapes.** Frontmatter-wins dedup, item fields/tags mirrored into note-level collections,
   store-all-read-first multi-value fields, owned task-status strings (pinned: N16, `model.rs:379`,
   `lists.rs:1020`) are exactly what ticket 23 is deciding.
9. **pulldown-cmark/logos mechanism tests.** Single-pass event walk (`parser.rs:106-110`), marker-across-chunk
   (`marker.rs:616,627`), logos single-pass (`lexer.rs:753`), heading option split (`parser.rs:68-72`) pin the
   mechanism, not just output; ticket 23 weighs a second markdown pass.
10. **`noyalib` YAML behaviors.** N02's duplicate/merge/1.1-scalar outcomes are crate-owned; swapping YAML
    libraries (or its config) changes them.
11. **`traces::testing` facade and `test-utils` gates.** `RefreshReport`, `DriftCause`, `InlinkMap`, `load()`
    export only under `cfg(any(test, feature = "test-utils"))` (`src/index/mod.rs:36-43`); ticket 32 may re-home
    them, breaking the P12/P24 seams.
12. **Disk-is-source-of-truth refresh.** Every P01-P08 guarantee assumes the vault on disk is the only input;
    ticket 34 (open editor buffers) adds a second freshness signal that does not exist today.

---

## D. Docs-vs-code contradictions

1. **Slug.** `src/note/GLOSSARY.md:45-52` defines a GitHub-style slug, byte-exact with no Unicode normalization and
   `slug-1` duplicate suffixes; **no slug code exists in `src/` or `tests/`** (grep hits only unrelated `slug` keys
   in schema-select fixtures). `Heading` carries level/text/line only (`src/note/heading.rs:20-45`). Arch ticket 23
   admits it: "footnotes / callouts / slugs (claimed by the glossary, absent in code)".
2. **Callout.** `src/note/GLOSSARY.md:98-105` specifies 13 Obsidian callout types, aliases and CSS custom types;
   **zero callout references in code or tests** (grep over `src/`, `tests/`, `benches/`).
3. **Footnotes.** `src/note/GLOSSARY.md:84-95` says the parser "emits them independently and the index
   cross-references them"; a footnote reference event only re-injects its source text into the scan buffer
   (`src/note/parser.rs:252-255`), there is no definition/reference record, and **`src/index/` has no footnote
   handling at all**. Related: root `src/GLOSSARY.md:71-79` defines an `Occurrence` carrying "footnote" from a
   "navigation spine's locate step"; no navigation spine exists in `src/`.
4. **ADR-0009 vs format-version handling.** ADR says drift covers "the stored config fingerprint **or format
   version**" as a `Drifted` state (`docs/adr/0009-index-refresh.md:36,40-41`); code wipes at `IndexStore::open`
   before any refresh state exists (`src/index/store.rs:927-1004`), so a version bump never surfaces as
   `ConfigDrift`. Arch ticket 25 item 2 flags the same gap.
5. **ADR status vs landed behavior.** ADR-0009 is `proposed` (`docs/adr/0009-index-refresh.md:5,17`) while
   implemented (`research/05`; arch ticket 25 "has landed"). Do not cite it as accepted authority.
6. **Duplicate ADR number 9.** `docs/adr/0009-filter-date-literals.md` and `docs/adr/0009-index-refresh.md` both
   carry `number: 9`; `map.md` notes "duplicate-numbered 0009s". "ADR-0009" alone is ambiguous (index refresh is
   the file-named one).
7. **Filename/title drift on ADR-0005.** File `0005-template-query.md` has
   `title: redb Index with QueryOps Namespace and Pipeline Terminal Filters` (`:3`); ADR-0009's "ADR-0005 =
   persisted redb index" reference (`:21`) resolves by title, not filename.

*Checked and consistent*: RefreshState/Report vocabulary, drift-wins, virgin-never-drifts, `Unrecorded`,
fingerprint exclusions (project move does not rebuild, `fingerprint.rs:313`), Folder Distance ties-unresolved,
`TaskStatusType` incl. `NonTask` (`src/task.rs:292-306`), malformed-YAML to empty fields, and both test counts
(208/304).

---

## E. Behaviors hostile to Gherkin (constraints for tickets 09-11)

- **Timing/mtime**: P06 racy gap, the 15 ms sleep workaround (`service.rs:1471-1490`), mtime-restore fixtures;
  unit/integration material.
- **Byte-level store surgery**: P14/P16 corruption tests (`0xFF` fill, `poison_note_row_in` `store.rs:393`) are
  not scenario-shaped.
- **Compile/source-time properties**: P09's `syn` scan is not executable behavior.
- **Internal enums/counts**: P04/P07 assert `RefreshReport` fields production only logs (`service.rs:309-326`:
  drift at `info!`, changes at `debug!`; count accessors `dead_code` outside test facades, `refresh.rs:52-59`). No
  user-visible projection exists today; arch ticket 25 item 5 would create one.
- **Process-spawn e2e as the production seam** (P01, P08, P14): slow, but the only place `refresh_store()` is
  exercised; scenario cost must be accepted under nextest (feeds ticket 10).
