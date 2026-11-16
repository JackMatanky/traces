# Second-round red team and audits for ticket 28

This file condenses the second-round review of the ticket 28 designs. It also
records later confirmed corrections. Each claim keeps its source mark.
The mark [V] means the reviewer saw the fact in code, a digest or a
measurement. The mark [H] means a hypothesis. The mark [P] means a design
proposal, not current code.
The first-round files are `g1-probes.md`, `h1-stress-designs.md` and
`h2-redteam.md` in this directory.

Sources: the reports of RedTeam28, AuditSimplicity, AuditUxConventions and
AuditImplementability. The prototype source and raw logs are in
`/tmp/rt28-proto/` (`proto28.rs`, `fuzz.rs`, `rt28-run1.txt`, `rt28-run2.txt`).
That directory is not in the repository. The repository does not hold the source
of the 0.148 ms figure [V, AuditImplementability, defect 11].

The review sections keep historical proposals [P]. They are not all adopted
rules. The "Reversals" table and the ticket 28 Answer state the final rules.

## Prototype method (RedTeam28)

All timings below measure the original prototype baseline [V]. They do not
measure the later resolution-context lifecycle.

- The prototype ran in a throwaway worktree. The worktree is removed.
- Build profile: `cargo test --profile bench` (opt-level 3, thin LTO). No
  RUSTFLAGS. Machine: Apple arm64 with 12 rayon threads [V].
- The shipped `[profile.release]` uses opt-level "z" (`Cargo.toml:276-281`)
  [V]. The bench profile uses opt-level 3 and thin LTO (`:269-274`) [V].
- Each figure is the median of 9 runs after 2 warmups [V].
- Each targeted result equals a from-scratch build. The assertion compared the
  path, inlinks and note of every entry [V].
- `FileEntry::note` is `Arc<Note>` or `Box<Note>` behind a feature. The runs
  used only `Box<Note>`. The cost of a heavy-note entry clone is not measured
  [H].
- No run used `RAYON_NUM_THREADS=1`. The single-thread cost of the
  full rebuild is not measured [H].
- Corpus: five link slots for each note. The generator's nominal 30% class is
  not a measured resolution rate. The mix is 8% unique
  wikilink, 7% markdown path, 13% shared-stem wikilink and 2% `[[hub]]`.
- Corpus at 50k: 48,927 resolved edges. One hub has 4,787 inlinks. Six big
  stems have about 700 candidates each (trie). 40 small stems are flat.
- Corpus at 5k: 4,861 edges. One hub has 450 inlinks. Big stems have about 70
  candidates.
- A separate "hot" corpus has 4,570 `index.md` candidates at 50k and 457 at 5k.

## W28a: cost of a derived generation after one buffer edit

Labels: (A) is the literal full rebuild. (B1) rebuilds the map from entries,
makes a fresh resolver and runs `patch_and_diff`. (B2) is (B1) with a cached
resolver. (C) is the targeted patch.

| Operation (ms, median) | 50k | 5k |
| --- | --- | --- |
| Edit 1 buffered note: (A) resolver plus `InlinkMap::with_resolver` | 113.8 | 12.7 |
| (A) extra: gather `&[Note]` from Arc entries (deep clone) | +59.3 | +6.1 |
| (B1) | 13.1 | 1.09 |
| (B2) | 4.83 | 0.42 |
| (C) targeted patch | 0.148 | 0.027 |
| Edit that adds a hub link (4,787 inlinks at 50k): (A) | 113.4 | 12.8 |
| (B1) | 13.3 | 1.07 |
| (B2) | 5.02 | 0.435 |
| (C) | 0.218 | 0.037 |
| Edit 20 buffers: (A) | 112.8 | 12.7 |
| (B2) | 5.82 | 0.75 |
| (C) | 0.620 | 0.355 |

Caveats: (A) in the first row has the notes already gathered. The gather needs
a contiguous `&[Note]`, which the Arc spine does not give [V].

Components of (C) [V]:

- Old targets: `resolve_note` on the base note takes 5.2 us. New targets take
  5.4 us.
- Entry deep clone: 0.9 us (4.5 KB).
- Hub inlink list clone: 69 us at 50k and 4.9 us at 5k.
- `Vec<Arc<FileEntry>>` spine clone: 134 us at 50k and 13 us at 5k. This is
  about 90% of (C) at 50k.
- Heap retained for each derived generation, at 50k / 5k: 400 KB / 49 KB for
  one note, 655 KB / 80 KB with the hub and 919 KB / 352 KB for 20 buffers.
- Keystroke path: parse of a 1 KB note takes 20 us. Parse of an 88 KB note
  takes 3.25 ms. Parse plus (C) takes 171 us at 50k and 45 us at 5k.

Per-base caches. The host builds a cache once for each base generation and only
when a buffer exists [V]:

| Cache | Time 50k / 5k (ms) | Net heap 50k / 5k |
| --- | --- | --- |
| `LinkResolver::new` | 6.4 / 0.60 | 10.5 MB / 1.0 MB |
| Stem to candidates map | 2.7 / 0.27 | not measured |
| Outlink-stem inverted index | 24.1 / 1.95 | 15.5 MB / 1.9 MB |
| Scan alone instead of the index | 10.6 / 1.1 | none |

A rebuild of an `InlinkMap` from entries costs 1.7 / 0.17 ms.

Path-set add (new unsaved file), ms at 50k / 5k. "Full" excludes the clone of
`Vec<FileMeta>` (1.15 / 0.07). The number of source notes in the stem bucket
appears in each case [V]:

| Case (sources in bucket 50k / 5k) | Full | Rebuild stem + index | Merge + index | Merge + index + filtered | Merge + scan |
| --- | --- | --- | --- | --- | --- |
| unique stem, dangling links resolve (50 / 5) | 114.3 / 12.8 | 0.79 / 0.09 | 0.81 / 0.083 | 0.95 / 0.10 | 11.8 / 1.2 |
| `daily`, 698 candidates, trie (4,122 / 393) | 113.0 / 12.8 | 52.6 / 3.9 | 51.1 / 3.8 | 12.0 / 0.93 | 63.2 / 4.9 |
| `cross`, 62 + 1 = 63 candidates, flat (500 / 50) | 113.6 / 12.7 | 12.2 / 1.03 | 12.0 / 1.0 | 7.1 / 0.69 | 23.2 / 2.1 |
| `index`, hot, 4,570 candidates at 50k (3,121 / 310) | 110.7 / 12.5 | 39.9 / 3.4 | 36.9 / 3.17 | 9.4 / 0.76 | 48.5 / 4.2 |

The raw log reports the `Vec<FileMeta>` clone separately [V].
Sources: `/tmp/rt28-proto/proto28.rs:1004-1014,1085-1090` and
`/tmp/rt28-proto/rt28-run1.txt:35-46,76-78,100-111,141-143`.

- "Rebuild stem" rebuilds one `BaseNameIndex` for the candidates of the new
  stem.
- "Merge" keeps the base trie or the flat list. It adds the one new candidate
  with the nearest-distance and tie-count aggregate. It needs a new
  crate-internal accessor `BaseNameIndex::nearest_raw`. Today `nearest` returns
  only `Option<&Path>` (`src/index/trie.rs:46`) [V].
- "Filtered" resolves a source again only if a link whose stem matches the new
  file changes target.
- The `cross` case ends at 63 candidates. The flat-to-trie change (63 to 64)
  is not run. A trie build for 70 candidates costs 0.05 ms. Merge mode never
  changes the representation.
- Trie build alone: 0.5 ms for 698 candidates and 2.4 ms for 4,570. A separate
  probe built 5,000 candidates in three folder shapes in 2.0 to 2.3 ms. The
  shapes are 5,000 sibling folders, 50 by 100 area and project folders, and a
  deep chain. The nearest lookup takes about 0.1 us.

Soundness result [V]:

- A link resolution depends only on the source path, the link text and the path
  set (`resolve` in `src/index/inlinks.rs`). A content edit changes only the
  edges of the edited note. The targeted patch is sound for same-stem and
  ambiguity cases.
- A new path P can change only links whose last-component stem equals the stem
  of P. The exact path, the path with an implied `.md` and the basename
  fallback all share that stem. The stem bucket is a safe superset.
- Randomized check, 300 iterations at 5k [V]: 88 edit-only, 113 add-only and
  98 edit-plus-add iterations, with 312 added paths. One iteration has no edit
  or added path. Links covered `.md` and `.png` extensions, qualified paths,
  anchors and the same stem in different folders. All 300 results equal a
  from-scratch build.
  Sources: `/tmp/rt28-proto/fuzz.rs:260-320` and `rt28-run2.txt:19`.
- The fuzz has no count of equal-distance ties [H].
- A derived generation only adds paths or replaces notes. A buffer close means
  a rebuild from the base. The design never removes a path from a trie. Trie
  aggregates cannot subtract.

Preconditions of the targeted path [V, code]:

- `LinkResolver<'a>` borrows a contiguous `&[FileMeta]`
  (`src/index/inlinks.rs:307-310`). `InlinkMap::with_resolver` takes `&[Note]`.
  The `Arc<FileEntry>` spine gives neither. One option is an owned resolver (a
  `Vec<FileMeta>` clone costs 0.6 ms at 50k). The other option is an API
  change. Otherwise the full path pays the 59 ms gather.
- Targeted soundness needs the base inlinks to equal the resolution of the base
  paths and notes. The resolver cache must live inside the base generation
  object. A host-global cache that serves a newer base corrupts the inlink
  lists without any error.

Verdict W28a: PARTLY. The 105 ms figure of the first round holds only for a
literal full rebuild. That rebuild costs about 173 ms at 50k with the gather.
The existing patch path (B1) costs 13 ms at 50k. The targeted patch (C) costs
0.15 to 0.62 ms [V].

RedTeam28 proposal [P]: use the targeted patch. Keep the resolver and a lazy
stem index in the base generation. Use a full rebuild as a fallback.

The final rule adopts the targeted patch and the owned resolver. Path-set
changes need a full re-resolve. Exact-base changes can also need a full
context rebuild. The final rule does not adopt the stem index.

## W28b to W28f verdicts (RedTeam28)

W28b: CONFIRMED, and wider than the first round stated.

- Rule Q16 said "an existing edited file drops at close". Rule Q9 said "keep
  until proof". After an edit, a save and a close, a query flips back to the
  stale base row until the next refresh.
- The Q9 proof "the row matches the saved file (stat triple)" has three holes:
  1. A terminal edit can supersede the save. The row then never matches and
     the entry leaks.
  2. A racy row can match a stat triple while its content is stale (ticket 25
     line 247).
  3. A refresh that changes nothing may publish no generation. The condition
     "a refresh publishes" then never occurs. Ticket 25 does not say that
     clearing a racy bit commits [H].
- Corrected rule:
  - At the close of a dirty buffer, replace the overlay entry with the
    last-saved snapshot if a save happened since the last proof. Otherwise drop
    the entry. A clean buffer keeps its entry.
  - A closed entry drops at the first completed scan with
    `scan_start >= save_time + 2 s`. The row need not match. Disk wins once the
    buffer is closed.
  - The proof keys on the scan epoch, not on a published revision. `Note`
    equality with the saved snapshot may end the wait early.
  - An open buffer keeps its entry across a save (rule Q19).

W28c: PARTLY. The problem is a position-correctness problem, not only a
staleness problem. A completion or hover request after `didChange` must see
that edit.

- rust-analyzer has no explicit debounce. It applies changes in batches before
  the next snapshot
  (`.scratch/md-pkm-lsp/research/14-buffer-overlay-model.md:79`) [V].
  Its non-retrying requests return `ContentModified` after cancellation
  (`.scratch/md-pkm-lsp/research/x-rust-lsp-implementation-patterns.md:402`)
  [V].
- markdown-oxide runs `Vault::update_vault` on each `didChange` under the
  write lock. It then calls `schedule_diagnostics`. The 300 ms debounce
  applies only to diagnostics
  (`docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt:1659-1668,1726-1744,1876-1890,2236-2242`)
  [V].
- IWE writes the buffer into the graph at once. It batches only its BM25
  rebuild after 300 ms (ticket 28, "Inputs from the IWE research") [V].
- md-pkm-lsp ticket 14 section 4 already designed a per-request inlink patch
  with a cached `LinkResolver`.
- The measured keystroke path is 45 us at 5k and 171 us at 50k. IWE stalled on
  a heavy per-keystroke index rebuild. Traces has no such rebuild.
- Corrected rule: publish the buffer note and the derived generation on each
  `didChange` with a latest-wins worker, not a timer. Debounce only the schema
  check and the diagnostics. A position request pins buffer version v. If the
  client version is newer, the request waits for the in-flight publish (at most
  one parse) or returns `ContentModified`. It never answers with positions from
  an older version. An 88 KB note parses in 3.25 ms. Larger notes may coalesce.

W28d: PARTLY.

- Beyond the spine clone, the measured patch cost grows with the number of
  buffers [H]. It adds about 0.03 ms for each buffer above the 0.134 ms spine
  floor (0.62 ms for 20 at 50k). The spine clone itself grows with the number
  of notes [V].
- Real costs of the base cache and the heap:
  - The per-base cache build costs 6.4 ms plus 10.5 MB for the resolver. The
    inverted index costs 24 ms plus 15.5 MB. Build the index lazily, or use
    the 10.6 ms scan.
  - The host can reuse the resolver across bases when the path set is
    unchanged (`RefreshPlan::is_paths_unchanged`, `src/index/refresh.rs:472`).
  - Each derived generation retains 0.4 to 0.9 MB of heap. In-flight requests
    multiply that figure.
  - A hub touch clones 245 KB (4,787 `PathBuf`, 69 us) for each keystroke.
- The earlier review estimated about 0.6 ms for a derived rebuild after a base
  refresh [H]. The context amendment does not preserve that estimate.
  Each exact-base change can need a full context rebuild.
  An unchanged path set can still reuse the immutable resolver component.

W28e: PARTLY.

- The identity `(incarnation, revision, overlay_seq)` is safe only if
  `overlay_seq` is process-monotonic and never reused. A reset for each base,
  or a return to 0 when all buffers close and a restart at 1, makes memo keys
  collide. The value 0 must mean "no overlay".
- Ticket 27's macOS-suite result shows a weakened incarnation-plus-revision
  check accepting a same-revision forked journal [V]. It does not isolate a
  revision-only check.
- A digest of (path, LSP version) is not unique, because `didOpen` can restart
  the version after a close.
- Simplest safe rule: name a derived generation by the base pair plus a
  process-wide `derived_id: u64`. Key memos on the generation identity (the Arc
  pointer) or on the full triple.

W28f: PARTLY.

- The 499 ms and 395 MB path-set cliff (ticket 24 line 98) is the decode of
  stored notes [V]. It does not apply to a derived generation that the host
  builds from in-memory `Arc<FileEntry>` values that already hold notes
  (ticket 24 lines 364-368). It would apply if host entries were lazy facets
  (ticket 24 line 547), because the stem bucket needs the outlinks of every
  note.
- The real cost is link re-resolution. A full re-resolve takes 113 ms at 50k.
  The targeted merge takes 0.8 ms (unique stem) to 12 ms (worst filtered
  bucket, 4,122 sources). The cost follows the stem fan-in, not N.
- The cliff does hit later. When the user saves the new file, the base refresh
  sees a path-set change. `RefreshPlan::reconcile` then takes the
  `merge_refreshed_notes` branch and reads all stored notes
  (`src/index/refresh.rs:429-455`).
- Ticket 24 line 397 says that the trie build at 5,000 same-stem candidates
  took 224 ms. RedTeam28 measured 2.0 to 2.3 ms for the build alone. The 224 ms
  figure is probably the whole `InlinkMap::new` over 5,000 sources [H]. Keep
  the trie decision and relabel the number.

## More weaknesses (RedTeam28), ranked by cost of being wrong

1. Base publish races a derived rebuild [H]. A derived task built from base G1
   can publish after base G2. It then rolls back every non-buffer change and
   moves the revision backward. Rule: publish a derived generation only by
   compare-and-swap on the current base id. Discard and rebuild on a mismatch.
   Never derive from the previous derived generation. Replay and fork events
   rely on this rule. The cache lifetime ties to the base generation.
2. The rule "a rename re-keys the entry" is unsound under LSP [H]. The client
   owns the document identity. After an external `mv a.md b.md` the client
   still sends `didChange` for `a.md`. That message recreates the old key next
   to the re-keyed one. Key overlays by client URI. A watcher rename affects
   only disk. Expect a duplicate or ghost row until close, with one notice.
3. Ghost rows [H]. A ghost joins link resolution like an added path. Links that
   fell to another same-stem candidate flip back to the ghost. The CLI shows
   the opposite. Keep the row only for a dirty buffer. A clean buffer drops its
   entry at once after an external delete. A ghost never persists. The only
   permanent leak risk is the pending-proof entry (W28b).
4. The admission predicate is missing [V for the scanner, H for the gap]. The
   scanner skips ignored directories, `.git`, the index file and symlinks. It
   keeps only regular files (`src/index/service.rs:412-448`, test
   `skips_symlinks_entirely` at line 829). A buffer that the scan skips would
   show in the host and never get proof from a refresh, so its entry leaks.
   The overlay must also exclude non-`.md` files (`FileFormat::Other`),
   `untitled:` URIs, paths outside the root, non-UTF-8 names and names that
   admission rejects. Rules Q23 and Q24 were silent.
5. Stale pre-edit answer for position requests. W28c has the fix:
   `ContentModified` (the Answer names the core outcome `Stale`).
6. `FileMeta` of overlay rows is undefined [H]. A dirty buffer or a new file
   needs a rule for `file.size`, `file.mtime`, `created` and `day`. Suggested:
   size is the buffer bytes. The mtime and created values stay from the base
   row (`None` for a new file).
7. Parse notices of the buffer must replace the persisted notices of the base
   for that path in the derived entry [H].
8. A template engine inside the host sees the overlay (rule Q8) [H]. A template
   that writes a file persists output from unsaved text. This conflicts with
   "never write a fact from an unsaved buffer" only if the output is indexed
   later. State it.
9. Two LSP hosts on one vault each hold their own overlay and never see the
   other overlay [H]. Document it with S9.
10. Old derived generation held at close [H]. The request finishes on its
    immutable generation (S7). A registry that keeps old derived generations
    (for example a result cache keyed by the triple) leaks about 0.4 to 0.9 MB
    each at 50k. After a buffer closes, the positions of a request refer to a
    closed document. The client discards them.
11. Journal interplay (tickets 26 and 36) [V, ticket 26 input]. A derived
    generation is never persisted, so no buffer fact enters the journal by
    construction. A replay changes the revision, so the compare-and-swap of
    weakness 1 discards in-flight derived work. A `cp -R` fork changes the
    incarnation, so derived generations must not carry over.
12. Ticket 25 racy bit. The proof of rule Q9 must wait for a non-racy row or
    for `Note` equality. Stat equality alone is not enough (W28b) [H].

## Scenario holes (RedTeam28)

- S2: a refresh that changes nothing may never publish, so the overlay never
  drops. Use the scan epoch.
- S4: the outcome differs for a clean and a dirty buffer after an external
  delete (weakness 3).
- S5: weakness 2.
- S7: weakness 5 (`ContentModified`).
- S8: weakness 1 (compare-and-swap).
- S9: two hosts or two windows.

Not done by RedTeam28: timing of deletion and ghost removal, the `Arc<Note>`
variant and the single-thread full rebuild.

## Audit findings for ticket 28

Three audits reviewed the v2 decision set. The v2 text is the first complete
form of the ticket 28 rules. The final rules differ from it only where this
file or the ticket 28 Answer says so.

### AuditSimplicity (lens: complexity and consistency)

The audit read the v2 text. It ran no measurement. Its marks are [V] for lines
it read and [H] for inference.

| Item | Verdict | Content |
| --- | --- | --- |
| 22 | KEEP | Every host query sees open buffers. The CLI sees the store only. A language server that ignores unsaved edits is wrong. The rule is a new surface, so ticket 16 stays intact. |
| 23 | SIMPLIFY | Debounce, then do a full derive: 114 ms at 50k and 13 ms at 5k, at most once every 300 ms, off the request thread. No targeted patch and no resolver cache. Behavior lost: edits show about 300 ms after the last keystroke, with CPU spikes above about 20k notes [H]. The retrofit of a targeted patch is moderate and local to one module. |
| 24 | DROP | The audit called publish per keystroke, a latest-wins worker, compare-and-swap, `derived_id` and "never derive from a derived generation" overlapping guards. It proposed one owner thread that holds the current generation. Keep the rule that a position request pins version v and gets a stale outcome. |
| 25 | DROP and replace | The save and close rule has a 2 s constant, three buffer states and a dependency on scan epochs. Replace it with: close drops the entry, save marks it clean, the watcher refreshes the base. Behavior lost: after a close, queries see the older base for up to one scan interval [H]. |
| 26 | REVISIT | The overlay is keyed by client URI. The base is keyed by `RelativePath`. Link resolution and the admission predicate take a `RelativePath`, so a conversion is needed. The text does not name the owner. Outside-root, non-UTF-8 and case-variant URIs fail the conversion. Fix: convert once at `didOpen` and keep a side map. |
| 27 | SIMPLIFY | Keep the ghost row for a dirty buffer after an external delete. A clean buffer drops at once. "Watcher rename affects disk only" is a statement of non-handling. "Two hosts" is a statement of non-sharing and needs no code. |
| 28 | KEEP | `file.size` is the buffer bytes. The mtime stays from the base. |
| 29 | KEEP | The schema check runs on the debounced parse. |

Ranked defects that name ticket 28:

- Contradiction 1: URI-keyed overlay against `RelativePath`-keyed base. Neither
  the link resolver nor the admission predicate can take a URI [H].
- Cost without user-visible benefit:
  - Item 5: compare-and-swap, `derived_id` and publish per keystroke. A
    debounced full derive costs 13 ms at 5k.
  - Item 6: the 2 s drop rule with the scan-epoch proof.
  - Item 7: the per-base resolver cache with crate-internal API changes.
- Hole 1: who converts URI to `RelativePath`, and what happens for outside-root,
  non-UTF-8 and case-variant URIs [H].
- Overclaim 2: the overlay timing of 0.148 ms at 50k is for one note. The 20
  buffer figure of 0.62 ms is fine. The heap retained for each derived
  generation (0.4 to 0.9 MB) multiplies if in-flight queries pin old
  generations. That multiplication is not measured [H].

The audit lists these "expensive to retrofit" items. The overlay key type
(`RelativePath` against URI) is one of them: a later re-key touches the link
resolver, admission and the ghost rules [H].

The audit also proposes a minimal ticket 28 set [H]:

- Hosts see buffers.
- One worker does a debounced full derive.
- The overlay key is a `RelativePath`, converted once at `didOpen`.
- A close drops the entry.
- A dirty deleted buffer keeps a ghost row.
- A position request pins a version and gets a stale outcome.

### AuditUxConventions (lens: user experience, conventions, performance)

The audit ran the binary of 9 Nov in a temp vault with a fresh HOME [V]. It
re-measured no overlay figure. It judged the ticket 28 figures as follows.

| Item | Verdict | Content |
| --- | --- | --- |
| Every host query sees open buffers | KEEP | It fits rust-analyzer and markdown-oxide. Document it in one help paragraph. |
| Derived generation with targeted patch | KEEP | The corpus has 5 outlinks for each note and about 30% resolving, so it is lighter than a link-dense vault. A 3x error leaves 0.5 ms for one buffer and 2 ms for 20. The spine clone (134 us, about 90% of the cost) changes a decision only if it grows 100x. The resolver (6.4 ms and 10.5 MB at 50k) is built once for each base. A 3x error costs 20 ms once. |
| Publish per change, latest-wins, debounce only diagnostics, version pin | KEEP | It matches markdown-oxide and IWE [V, from RedTeam28 and digests]. A query result in an open editor view can change while the user types. That is intended. Documentation must state it, because the CLI answers differently for the same file. |
| `derived_id` and compare-and-swap | KEEP | The identity failure is shown in W28e [V]. |
| Save and close rules with the 2 s proof | SIMPLIFY the explanation | State where the constant comes from (the racy margin of ticket 25). On FAT and network volumes the window is borderline [H, RedTeam32]. |
| Overlay keyed by client URI, ghost row | KEEP | The ghost is a visible surprise: links flip back to the ghost while the CLI shows the opposite. The notice says "file deleted on disk; showing unsaved buffer". |
| External change to an open file | KEEP | The rust-analyzer behavior here is [H], not verified. |
| Schema check, `file.size`, two hosts | KEEP | Document that two hosts keep independent overlays. |

Gaps and overclaims:

- Dedup: "one notice" has no key. Define one notice for each (URI, divergence
  kind) until the buffer closes or the state changes [V, gap]. Ghost,
  divergence, bracket-mismatch, skipped-name and ambiguity notices share the
  problem.
- LSP re-push rule: nothing says whether the client gets a query result again
  after each publish. Without a rule, a stale display is possible [H].
- The ticket 28 probes used the bench profile (opt-level 3). The shipped
  profile is opt-level "z" (`Cargo.toml:276-281`). The audit measured that "z"
  slows iterator-heavy comparators by about 2x. The overlay figures are not
  re-measured under "z" [V].
- The audit found a self-reference in the ticket 34 `Blocked by` line [V,
  historical]. The current header no longer lists ticket 34 [V].
  Six kinds of notice, including the ticket 28 notices, depend on that ticket.
- Verdict: the audit found most identity and publish rules of ticket 28 sound.

### AuditImplementability (lens: implementation map and conflicts)

Overlay implementation map [V, code]:

- `WorkspaceIndex` holds `entries: Box<[FileEntry]>` (`src/index/entry.rs:22`).
  The prototype assumed `Box<[Arc<FileEntry>]>`.
- `FileEntry { file, note: Option<Box<Note>>, inlinks: Box<[PathBuf]> }`
  (`src/index/entry.rs:135-138`).
  `QueryRow` holds `Arc<WorkspaceIndex>` plus a `RowIndex`
  (`src/query/results.rs:37-42`) [V].
- `WorkspaceIndex` owns no `InlinkMap` and no resolver. `assemble_internal`
  moves the inlinks into the entries and drops the map
  (`src/index/entry.rs:79-102`).
- `LinkResolver<'a>` borrows `&'a [FileMeta]` and keys `BaseNameRef<'a>`
  (`src/index/inlinks.rs:307-310`). An `Arc<FileEntry>` spine is not a
  contiguous `[FileMeta]`. `get_by_path` binary-searches with `Path::cmp`. The
  resolver needs an owned redesign before it can live in a generation object.
- Targeted patch [H]: the repository does not hold the source of the 0.148 ms
  figure. The cost depends on `Box<Note>`. A target
  entry whose inlinks change forces a deep `Note` clone unless `note` becomes
  `Arc<Note>` or `inlinks` becomes a separate `Arc`. An edit that adds or
  removes links costs more, and the cost grows with the number of outlink
  targets.
- No host exists. Core can supply only a pure `derive(base, edits)`. The
  serialized owner belongs to ticket 33.

Ambiguities named for ticket 28:

| Id | Ambiguity | Proposed one-line resolution |
| --- | --- | --- |
| B19 | `ContentModified` is an LSP code in a core-level decision. | Name a core outcome `Stale`. The host maps it. |
| B20 | What overlay key spelling does an unsaved new file get when the base lists nothing? | Use the URI spelling. The 2 s scan proof drops the entry whatever the real spelling is. |
| B21 | A new or removed overlay path "uses `patch_and_diff`". | See the correction below. State "full re-resolve" and measure again. |

Correction to the v3 text. The v3 text and line 29 of the v2 text said that a
path-set change uses `patch_and_diff`. That statement is wrong [V, High]:

- The existing refresh calls `patch_and_diff` only when `is_paths_unchanged`
  holds (`src/index/refresh.rs:413-421`). The comment at `:468-471` says that
  link resolution for unedited notes stays valid "only in this case".
- A path add or a path remove takes the `else` branch (`:430-454`). That branch
  runs `LinkResolver::new` plus `InlinkMap::with_resolver` over all notes.
- An added file can change the wikilink resolution of other notes (nearest
  match and ties).
- The 13 ms figure belongs to a different code path (B1 above).
- `patch_and_diff` needs an `InlinkMap` that `WorkspaceIndex` does not hold.
- Required statement: a path-set change in the overlay is a full re-resolve.
  Measure it again.

Other defects and handoffs that name ticket 28:

- Defect 3 [V, historical]: the `Blocked by` lines listed self-references in
  tickets 31, 34 and 35. Ticket 33 lacked tickets 28 and 32. The current
  headers contain the repairs below [V]. A repair that adds both tickets 31
  and 35 to ticket 34 would create a cycle between tickets 34 and 35 [H].
  - Ticket 31 waits for 7, 16, 18, 28 and 29.
  - Ticket 34 waits for 8, 12, 13, 20, 28, 29 and 31.
  - Ticket 35 waits for 8, 12, 16, 22, 33 and 34.
- Soft cycle between tickets 28 and 33. Both "set the staleness bound". If 33
  waits for 28, ticket 28 states the overlay rules for a parameter B and
  leaves the number to ticket 33. That is a one-way handoff.
- Defect 10 [V]: `LinkResolver<'a>` cannot sit in a generation object. The
  "reuse the resolver across bases" rule depends on the owned redesign.
- Defect 11 [H]: the targeted-patch cost depends on `Box<Note>`.
- Defect 16 [V]: `ContentModified` is an LSP term in a core-level decision.
- The audit judged the "2 s proof tied to the racy margin" sound [V].
- Not checked: the 0.148 ms and 13 ms patch timings, and any Windows behavior.

## Reversals of the first-round recommendation

The first round is `h1-stress-designs.md`, `h2-redteam.md` and
`z-grilling-guide.md`, plus the v1 rules (Q-numbers) that the grilling used.
Each row names the first-round rule and the final rule.

| # | First-round recommendation | Final rule | Reason (source) |
| --- | --- | --- | --- |
| 1 | Rule Q9: the entry stays until a refresh publishes a row that matches the saved file (stat triple). | The entry of a closed buffer drops at the first completed scan with `scan_start >= save_time + 2 s`. The proof keys on the scan epoch. | Three holes: a later terminal edit leaks the entry, a racy row can match with stale content, and a no-change refresh may publish nothing (W28b). |
| 2 | Rule Q16: an edited existing file drops at close. The z-guide says the same: keep the overlay until close, then drop. | A clean buffer keeps its entry. A dirty buffer gets the last-saved snapshot if a save happened since the last proof. Otherwise the entry drops. | A saved-then-closed file flipped back to the stale persisted row (W28b). |
| 3 | h2 attack 1: on host save, drop the overlay entry. | Open buffers keep their entry across a save. | The drop at save opens a window with the stale row. The z-guide asked the grill to decide it. |
| 4 | Rule Q22: name the generation `(incarnation, revision, overlay_seq)`. | Name it by the base pair plus a process-wide monotonic `derived_id: u64`. The value 0 means "no overlay". The value never repeats. | A reset collides memo keys (W28e). |
| 5 | Rule Q23, h1 D1 and the z-guide S5: re-key the entry on a watcher rename. | The overlay keeps the client document URI. A rename never re-maps until close. | The client keeps sending `didChange` for the old name (weakness 2). |
| 6 | The z-guide Group 3: parse on a debounce, never on a keystroke. | Publish the buffer note and the derived generation on each change with a latest-wins worker. Debounce only the schema check and diagnostics. | The keystroke path costs 171 us at 50k (W28c). |
| 7 | The first-round full rebuild of 105 ms for each edit. | Targeted patch for content edits. The original baseline is 0.148 ms at 50k. Path-set changes need a full re-resolve. Exact-base changes can need a full context rebuild. | W28a: the original literal rebuild is about 173 ms with the gather. |
| 8 | v3: a path-set change uses `patch_and_diff` (13 ms). | One full re-resolve off the request thread. The original baseline is 113.8 ms at 50k, about 173 ms with the gather. | The existing refresh calls `patch_and_diff` only for an unchanged path set (AuditImplementability, defect 1). |
| 9 | `ContentModified` for a stale position request. | The core outcome `Stale`. The host maps it to its protocol code. | `ContentModified` is an LSP term (B19). |
| 10 | The z-guide S4: a row that is absent from the generation drops. | A dirty buffer keeps a ghost row until close. A clean buffer drops at once. | Weakness 3. |
| 11 | RedTeam28 weakness 1: publish only by compare-and-swap on the current base id. | One owner serializes generation changes. Compare-and-swap is one allowed way. | The AuditSimplicity item 24: one owner makes the guard structural. |
| 12 | Rules Q23 and Q24 were silent on admission and on the conversion of a URI. | The overlay applies the scanner predicate. `ProjectRoot` maps the URI once to a `RelativePath` at `didOpen`. | Weakness 4 and AuditSimplicity item 26. |
| 13 | h2 defense for the wrapper: an enum row (Generation or Overlay). | A derived generation of normal `Arc<FileEntry>` rows. The plan sees "any pinned generation". | The ticket 29 seam. |
| 14 | The z-guide: one notice for a divergence, without a key. | One notice for each (document, divergence kind) until the state changes or the buffer closes. | AuditUxConventions, gap on dedup. |
| 15 | The first-round S1 to S9 table in the z-guide (candidate rules). | The final outcomes in the Answer. | The reversals above change S2, S4, S5, S7 and S8. |

### Confirmed resolution-context amendment (2026-11-15)

The maintainer accepted this amendment after the later hostile review [P].
It replaces a base-only resolver for derives that retain an overlay-only path.

- `derive(base, current_buffers)` equals a full build over the effective paths
  and notes. It includes every current buffer.
- A private immutable context names the exact validated base snapshot and the
  effective path set: base paths plus admitted overlay paths, including ghosts.
- The context holds the effective resolver and the base-note edges resolved
  through it. A derive replaces those edges with the current buffer edges.
- A base publish or replay invalidates the context. A new or removed effective
  path changes its key.
- Each exact-base change can need a full context rebuild while an overlay-only
  path remains open. No cheap rebuild bound is established.
- A close removes the replacement. A close of a path still in the base needs
  no full re-resolve. A watcher rename never changes the client URI identity.
- A previous derived result never becomes the semantic base.
- `QuerySet` reads an ordinary pinned generation. The context is not public.
- The original timings do not prove this lifecycle. Measure its cost and check
  full-build equality before implementation adoption.

Audit alternatives that the grilling did not adopt:

- AuditSimplicity item 23 (debounced full derive). The original full derive
  costs about 173 ms at 50k with the gather [V]. Its targeted patch costs
  0.148 ms [V]. The ticket keeps the targeted patch. Path-set changes need a
  full re-resolve. Exact-base changes can also need a full context rebuild.
- AuditSimplicity item 25 (close drops the entry). This alternative reopens the
  flip back to the stale base row after a save and a close (W28b).
- AuditSimplicity item 24 (drop compare-and-swap and `derived_id`). The final
  rule keeps `derived_id` (W28e) and keeps one owner. It lets the owner choose
  how to enforce the invariant.

## Open items after round 2

- Not measured: the amended resolution-context lifecycle, the heavy-note entry
  clone, the `Arc<Note>` variant, the single-thread full rebuild, deletion and
  ghost removal, equal-distance ties in the fuzz and the flat-to-trie crossing
  (63 to 64 candidates).
- The figures come from a synthetic corpus at opt-level 3. The shipped profile
  is opt-level "z".
- The repository does not hold the source of the 0.148 ms figure [V].
- No host code exists. The overlay rules are semantics, not a built design.

## Final adversarial review (2026-11-16)

The later review checks the written contract, not a production host.
Four small state models cover effective path resolution, incremental edits,
proof publication order and process-local generation identity [V per Stress28].
They do not measure the amended architecture.

### Confirmed lifecycle corrections

- Apply every LSP text edit in order. Coalesce versioned buffer snapshots only
  after the text state is current.
- Keep one running job and one latest pending snapshot. Fence publication by
  base, overlay state, document session and version.
- Select the required position version before pinning a generation. Never repin.
- Treat unknown cleanliness as dirty. Keep an exact known saved text snapshot,
  not only its parsed `Note`.
- Give every retained closed entry a proof time. For a save, use receipt time.
  Otherwise use the known disk snapshot's observation time.
- An eligible scan validates and installs the coherent base atomically with
  entry removal. Failed scans release nothing. No-change scans need no new revision.
- Remove the `Note`-equality early release. It does not prove byte size or notices.
- A validated deletion supersedes older saved snapshots. A later save creates
  a new proof obligation.
- Separate lexical admission from existing-node checks. A new final component
  needs an admitted parent. One listed path has one active buffer owner.
- Persist validated disk facts only. Derived query generations cannot create
  store or journal batches.
- Scope the generation triple to one host lifetime. Memos own their generations;
  no global generation history survives.

The maintainer confirms these corrections. They retain transparent buffer truth,
immutable query generations and the private effective-path resolution context.
An event history, cross-process overlay service and persistent immutable tree
add duties without a demonstrated need. They are not adopted.

### Measurement scope corrections

The added-file saved-index baseline is 499 ms and 394.9 MB at 50,000 notes
(`research/24-prep/g-probe-results.md:88-95`).
The 5k decoding figure is 18.48 ms, or 97.3% of in-memory reconciliation work
(`research/11-performance-evidence.md:89-95`). It excludes scan and persist.
Neither figure measures deletion or the amended context lifecycle.

The synthetic corpus has five link slots per note. Its nominal 30% class is
not a measured resolution rate. The runs contain 48,927 unique resolved edges
at 50k and 4,861 at 5k [V per checked prototype logs].

Measure amended-context retention and shipped-profile latency before adoption.
Compare each persistent-new-path and ghost sequence against a full graph build.
