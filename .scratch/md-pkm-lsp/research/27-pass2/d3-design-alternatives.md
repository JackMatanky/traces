# Ticket 27 pass 2, design-decision review D3: alternatives against the current code

Method: codegraph_explore over the live tree (`.codegraph/` present at repo root,
used throughout), plus direct ticket text reads. Scope: the ten provisional
design decisions in the pass-2 consolidation, each re-checked against what the
code actually does today, biased toward the smallest change that keeps (or
deepens) the shared-walk design. All file:line cites are working-tree current.

---

## 1. Per-decision verdicts

### D1. Shared structural walk: one parse pass feeding folding/selection/symbols/documentLink

**Verdict: DEEP DESIGN CONFIRMED, with a placement correction: the walk is an
LSP-side free function over raw text, not a Note method and not an extension of
`collect_headings`.**

Evidence:

- `parse_markdown` (src/note/parser.rs:106-115) already iterates
  `into_offset_iter()` and passes the FULL `ByteSpan` to `handle_event`
  (parser.rs:111), but `handle_start_tag` receives only `span.start()`
  (parser.rs:264-268); every end offset is discarded. Nothing span-bearing
  survives into `Note` (heading.rs:10-14: `{level, text, line}` only;
  links.rs:28-34: `Link { target, text, kind, embedded }` no position at all;
  lists.rs:41-50: `ListItem` line+depth+parent-line only).
- Headings are already a SECOND parse pass (`collect_headings`,
  parser.rs:118-162, re-running `Parser::new_ext` with `HEADING_OPTIONS`).
  The CLI index path therefore has no walk to "extend": merging 27's needs
  into `collect_headings` would couple LSP-only features into the postcard
  -persisted `Note` (model.rs:24-33, round-trip test at :574-626) and raise
  persistence-format questions 11 §8 explicitly wants to avoid.
- The cheapest correct shape: a new module-level function
  `walk(src: &str) -> DocStructure` (block ranges + inline spans, one
  `into_offset_iter()` pass with `ENABLE_WIKILINKS` already on,
  parser.rs:68-70), living on the LSP side of the seam 34 is deciding
  (34:10, "LSP protocol DTOs at the protocol boundary"). Consumers fold over
  the returned structure; cache it per `(uri, version)` next to 14's
  `BufferState { content, version }` (issues/14:27) so folding, selection,
  symbols, links, highlight, and tokens share ONE parse per document version.
  This matches the confirmed peer precedents (rumdl `HeadingSymbol` fan-out,
  MS ToC provider, r-a one Analysis) without touching the CLI parse at all.
- It is genuinely one walk, not four: the alternative (heading sections from
  `Note::headings()` line-scan, fences via re-scan, links via a third pass)
  is strictly more parses of the same text, the thing the walk exists to
  eliminate.

### D2. Outline tasks-as-children: direct walk vs QueryService

**Verdict: SIMPLER ALTERNATIVE EXISTS (and removes a visibility problem).**

Evidence:

- `QueryService` rows pay `QueryRow` materialization with a documented
  capacity-overshoot dance (src/query/service.rs:224-236, :280-307), and
  `QueryMode`/`QueryBuilder` are `pub(crate)` (src/query/builder.rs:21, :55)
  while `QueryService::run` is `pub`. A future `traces-lsp` crate split
  (34:10) would hit that wall immediately.
- The data needed for task outline children is all `pub` today:
  `Note::tasks()` (src/note/model.rs:191), `ListItem::depth` (lists.rs:195),
  `ListItem::parent` (lists.rs:210), `ListItem::line` (lists.rs:202),
  `ListItem::clean_text` (lists.rs:173). Section attachment is a line-range
  join against the walk's heading ranges. Zero new accessors required.
- Serving model matches 23's own S3 text (issues/23:46: ContentResolver +
  request-start snapshot). Workspace-scope task picking (if kept) walks
  `index.entries()` (src/index/entry.rs:108, pub) and `FileEntry::note()`
  (entry.rs:161, pub) directly, the same precedent 23 already chose to avoid
  row materialization.

Seam: a function in the documentSymbol handler module, e.g.
`task_symbols(note: &Note, sections: &[(SourceLine, SourceLine)])`, consuming
the walk's heading ranges and `Note::tasks()`.

### D3. documentLink spans: wait for 26's Occurrence, or emit from today's data?

**Verdict: SIMPLER ALTERNATIVE EXISTS. documentLink does NOT need 11 or 26 to
land with exact spans.**

Evidence:

- Current `Link` carries no position at all (links.rs:28-34), so there is
  nothing to emit from the AST. BUT the pulldown-cmark event stream fires
  wikilinks and standard links as `Start(Link)` events with full offset ranges
  (the CLI already classifies `CmarkLinkType::WikiLink` at parser.rs:416-426),
  and `target_parts()` splitting (links.rs:113-126) plus `target()`/`text()`
  are all `pub(crate)`-reachable from an in-crate walk. The D1 walk therefore
  produces exact byte spans per link occurrence immediately, with no line
  -granular compromise for note links at all.
- The pass-2 framing "emit 26's Occurrence span as single source of truth"
  is a unification nicety, not a dependency: the walk IS a span source, and
  26's `Occurrence` can later alias it.
- The 22 "no line-granular underlines" rule (b3-document-links.md:308-310)
  applies to TEMPLATE refs only (that passage is inside the template-refs
  section, gating on 22's machinery spans). It does not gate wikilink or
  standard-link documentLinks, which get exact spans from the walk on day one.
- Resolution of a basename wikilink to a path needs `LinkResolver`
  (src/index/inlinks.rs:319, `pub(super)`); 34:24 already budgets the named
  widening "a LinkResolver facade method (`resolve_link`-style)". One such
  method on 10's analysis-host facade (`resolve_link(from: &Path, raw_target:
  &str) -> Option<PathBuf>`) is the entire resolution seam. Existence check
  for already-path targets can use `InlinkMap::has_target` (inlinks.rs:162)
  or the facade.
- Drop-unresolved works with zero extra machinery: resolver returns `None`
  (inlinks.rs:369-405 already returns None for external URLs, tested at
  inlinks.rs:907) and the walk omits the emission.

### D4. CodeLens Option C substrate: per-note reference counts

**Verdict: DEEP DESIGN CONFIRMED. The substrate is one binary search plus one
`.len()` today.**

Evidence:

- Count refs to note B: binary-search `index.entries()` for B's path (entries
  are path-sorted; `WorkspaceIndex::entries` pub at entry.rs:108) and read
  `FileEntry::inlinks().len()` (entry.rs:165-170, pub). The `InlinkMap` is
  consumed into per-entry inlink slices at assemble time
  (`inlinks.remove(...)`, entry.rs:97-99), so entries are the canonical
  count source; `InlinkMap::inlinks_of` (inlinks.rs:123-125) is the same
  data pre-distribution.
- Zero-reference suppression is the same lookup: `inlinks().is_empty()` costs
  nothing beyond the count lookup. Universal among peers (b4:159).
- Caveat recorded: counts are index-snapshot counts; overlay-buffer edits are
  not reflected until 14's single-file refresh lands (same staleness class as
  every other index-derived feature; acceptable for a v1 lens at `(0,0)-(0,1)`).

### D5. Semantic tokens: enumeration pass, resolver choke point as side product

**Verdict: SIMPLER ALTERNATIVE EXISTS. The choke-point idea fails on
inspection; the D1 walk is the enumeration.**

Evidence:

- The only resolver choke point is `resolve_edges_for` /
  `LinkResolver::resolve_note` (inlinks.rs:303-316, :351-362), which runs at
  INDEX BUILD time over indexed notes only. It (a) never sees the open overlay
  buffer, which is exactly where tokens are requested; (b) covers outlinks
  only, not tags (`Note::tags`, model.rs:179, no resolution concept) or
  inline fields (model.rs:122); (c) runs during CLI refresh, so recording
  token spans there would persist or re-derive per-session state 11 §8 says
  never to persist.
- The D1 walk enumerates wikilinks (link events), tags (the parser's lexer
  already scans them per line; note-level `tags()` has no positions, so the
  walk re-scans or the lexer's ephemeral spans get surfaced into the walk),
  and inline fields (lexer spans, parser/lexer.rs) in the same pass.
- Resolution state v1: classify via the facade `resolve_link` call (Unique
  = Some, Unresolved = None). The three-valued Ambiguous modifier (26's
  `StemTie`) genuinely waits on 26's tie-reporting candidates; the
  two-valued unresolved modifier ships now. `InlinkMap` cannot report ties
  by construction (nearest-pick only, inlinks.rs:364-400; the divergence is
  recorded in 26).

### D6. Folding E-lite from `Heading {level, line}` alone

**Verdict: SIMPLER ALTERNATIVE EXISTS for heading sections, but the other four
fold kinds need the walk anyway, so this is a walk-internal detail, not a
separate design.**

Evidence:

- Scan-until-next-heading-of-level is correct: heading hierarchy is a flat
  list derivable by level stack (11 §2, and the code has exactly that flat
  list, heading.rs:10-14). Section end = next heading with `level <= current`
  minus one line; last heading ends at EOF line, obtainable from the source
  line count (or `LineIndex` line-starts length, position.rs:33-46). No
  structural breakage; the "last-line task leaf exceeds parent by one line"
  corner (consolidation §11.8) is cosmetic.
- What line-only loses: byte-precise section ends. Both VS Code and Neovim
  advertise `lineFoldingOnly` (b2), so line-granular folding is
  spec-sufficient v1; 11's span retrofit (or the walk's byte ranges, which
  exist from day one) upgrades it trivially.
- But fences (no AST node; parser drops code-block content, parser.rs
  444-452), frontmatter block (raw text discarded after parse,
  parser.rs:402-409), lists (no end lines, lists.rs:41-50), and callouts
  (don't exist; parser.rs:288-291 does list bookkeeping only) are only
  available from the walk's event ranges. Since folding ships E-lite anyway,
  all five kinds come from the walk; heading-section line-scan is one match
  arm inside it.

### D7. Config gates: smallest `[lsp]` section shape

**Verdict: DEEP DESIGN CONFIRMED with the existing section recipe, one flat
table.**

Evidence:

- The recipe is fixed and mechanical: add a sibling field to `Config`
  (src/config/model.rs:49-55), a `#[serde(default)] #[serde(deny_unknown
  _fields)]` table on `RawConfig` (src/config/raw.rs:17-30), a
  `TryFrom<RawLspConfig>` merge in `ConfigBuilder` (builder pattern per
  `resolve_tasks`, src/config/builder.rs:186), and a `digests()` pair with
  section-qualified labels (exact precedent: `SCHEMAS_DIRECTORY: &[u8] =
  b"schemas.directory"`, src/config/model/schemas.rs:127, consumed by
  `Config::digests` at model.rs:144). The exhaustive-destructure digest
  convention makes an unlabeled setting a compile error by design
  (src/config/mod.rs:19-27).
- Smallest shape gating all 27 features: ONE `[lsp]` table of flat per
  -feature bools, e.g. `symbols`, `folding`, `selection`, `document-links`,
  `document-highlight`, `code-lens`, `semantic-tokens`, `code-actions`
  (each `Option<bool>` in raw, defaulted in `TryFrom`: lens default false
  per client-floor research b4, the rest default true). This mirrors rumdl's
  `enableSymbols`-style per-feature ceding that 32/29 need, and matches the
  initialize-time static gate model (Neovim sets
  `dynamicRegistration=false` for 7 of 8 features; consolidation §0.8).
  Nested sub-tables per feature would be over-structure: no 27 feature has
  more than a boolean depth decision today (depth switches are 29/32 work).

### D8. Position conversion: minimal LineIndex extension

**Verdict: DEEP DESIGN CONFIRMED. The minimal extension is three methods; no
per-request O(n) walk is needed.**

Evidence:

- `LineIndex` today: `new` precomputes line starts (position.rs:39-46),
  `line_at` binary-searches (position.rs:57-61). Nothing else; zero utf16
  code in `src/` (repo-wide, a1 §2 confirmed).
- Minimal extension serving every 27 feature:
  1. `line_start(line: SourceLine) -> BytePos` (index into the existing
     `Box<[BytePos]>`, position.rs:33);
  2. `byte_to_utf16_cu(pos: BytePos) -> u32` (binary-search the line, then
     walk chars from line start summing `len_utf16`; this is EXACTLY the
     method 11 §4 already specifies, and 26 §7 retains it as "a thin
     LineIndex method");
  3. the inverse `utf16_cu_to_byte(line, cu) -> BytePos` for inbound
     `to_byte_pos` (26's VS Code `offsetAt` semantics).
  Per-conversion cost is O(log n + line length); folding/selection/symbols
  emit O(document) positions each, so total cost is O(document bytes) once
  per request, not O(n) per position beyond the line walk, and the ASCII
  whole-document fast path (26 §7) short-circuits the char walk.
- A per-line utf16 prefix cache is NOT warranted at note scale and should not
  be built v1; 26 already documented the rust-analyzer line-index crate
  upgrade route if hot (33 prices it).
- The `convert` module (`negotiate`, `to_lsp_position`, `to_lsp_range`,
  `to_byte_pos`) wraps these at the protocol boundary as 26 decided; its
  placement is 34:23-pending, and it is the miette bridge for free.

### D9. Refresh plumbing on tower-lsp-server 0.23.0

**Verdict: DEEP DESIGN CONFIRMED. Piggyback on 25's transport; no new lane.**

Evidence:

- 25 Q1-R already settled the push/pull transport (issues/25:23: option B,
  document-level pull with dirty-set/resultIds, push as fallback) and already
  hand-rolls a server-to-client refresh-class request: "workspace
  /diagnostic/refresh unconditional with swallowed error" (issues/25:24).
  `workspace/semanticTokens/refresh` and `workspace/codeLens/refresh` are the
  same protocol shape (server-to-client request), so the missing typed
  helpers in 0.23.0 (consolidation §0.13) are worked around by the identical
  hand-rolled send 25 already plans.
- The "graph changed" broadcast already exists conceptually: 25 Q11-R's facade
  old/new Arc snapshot-diff produces the dirty set plus the referrer set
  (issues/25:33, `FileEntry::inlinks()` at entry.rs:168 is the named input).
  Semantic-tokens refresh (open buffers whose link/tag state flipped) and
  CodeLens refresh (open buffers whose count flipped) are two subscribers of
  that same dirty/referrer emission, debounced on 25's T2 cadence (or b4's
  ≥250 ms for lens) and gated on 29's client-announcement flags.
- Minimal shared shape: a small observer list on the analysis-host facade
  (10:31) that the change-summary path notifies with `(dirty, referrers)`;
  token/lens refresh handlers translate that into per-URI re-pull plus one
  refresh request. Nothing else is required; inlay-hint-style used-flags
  (b4's refresh gate) are read from the capability snapshot 29 owns.

### D10. Over-engineering audit

**Verdict: two over-reach items, one under-reach item.**

1. **documentHighlight does not need 26's spine.** It is document-local span
   matching by definition (26's own consumer matrix, issues/26:27). The D1
   walk produces every link occurrence span; matching same-target (or
   same-resolved-path) occurrences in one document is a filter over the
   walk's link list, spec-sanctioned fuzzy, never errors on ambiguity. The
   spine `highlight()` promise is a unification, not a dependency. Ship it
   with the first walk wave (see landing order).
2. **Selection S1 does not need the 11-span retrofit** (pass-2 called it
   "the purest 11-dependence"; that is wrong once the walk exists). The walk
   carries byte spans per block and per inline event from pulldown-cmark on
   day one; selection nests position-locally over walk ranges. No AST change,
   no persistence question, no 11 wait.
3. **Under-reach: pass-2 gates ALL documentLink emission on 26's Occurrence
   spans.** As shown in D3, only template refs have a real span-source gate
   (22). Wikilink/standard-link documentLinks have exact spans from the walk
   immediately; treating them as 26-blocked needlessly serializes the most
   user-visible feature behind the largest open ticket.

Everything else in pass 2 checked out against the code: dual-shape symbols,
eager workspace symbols, static registration, ceilings split, and the
Option-C-first CodeLens path all match current substrates.

---

## 2. Recommended minimal-change path per decision

| # | Decision | Minimal path | Exact seams |
|---|---|---|---|
| D1 | Shared walk | New free function `walk(src) -> DocStructure` (block ranges + inline spans), one `into_offset_iter()` pass, cached per `(uri, version)` beside 14's `BufferState`; NOT a Note method, NOT merged into `collect_headings` | new module (34-home pending; single-crate `pub(crate)` to start), `pulldown-cmark::Parser::new_ext` with `MARKDOWN_OPTIONS` (parser.rs:68-70), `LineIndex::new` (position.rs:39) |
| D2 | Task outline children | Direct walk: `Note::tasks()` (model.rs:191) + depth/parent (lists.rs:195/:210) joined to walk heading ranges by line; no QueryService | documentSymbol handler; `FileEntry::note()` (entry.rs:161), `WorkspaceIndex::entries()` (entry.rs:108) |
| D3 | documentLink | Walk emits `(ByteSpan, raw_target, kind)` per link event; resolution via ONE new facade method `resolve_link(from, raw_target) -> Option<PathBuf>` (the 34:24-budgeted LinkResolver widening, exposed on 10's analysis facade); drop on `None` | `LinkResolver::resolve` (inlinks.rs:369, currently `pub(super)`), `LinkTarget` split (links.rs:113-126), `CmarkLinkType::WikiLink` classification precedent (parser.rs:416-421) |
| D4 | CodeLens Option C | Count = binary-search entries by path + `FileEntry::inlinks().len()` (entry.rs:168); suppress on `is_empty()`; lens at `(0,0)-(0,1)`, eager, no resolve | `WorkspaceIndex::entries`, `FileEntry::inlinks`; refresh later via D9 observer |
| D5 | Semantic tokens | Enumerate in the walk (link events + tag/field lexer spans); classify v1 via `resolve_link` Some/None (two-valued); three-valued modifiers wait on 26's tie-reporting facade | walk; facade `resolve_link`; legend/modifiers per b6 T1 |
| D6 | Folding E-lite | All five v1 kinds from walk event ranges; heading sections = next-heading-of-≤-level scan over walk heading list; EOF line from source line count; line-granular v1 | walk; `Heading::level/line` (heading.rs:31/:45) |
| D7 | Config gates | One `[lsp]` table, flat `Option<bool>` per feature, defaults in `TryFrom`; digest labels `lsp.<feature>` | `Config` (model.rs:49), `RawConfig` (raw.rs:17), `ConfigBuilder` (builder.rs:186 pattern), `Config::digests` (model.rs:144), `setting()` label precedent (schemas.rs:127) |
| D8 | Position conversion | Add `line_start`, `byte_to_utf16_cu`, `utf16_cu_to_byte` to `LineIndex` (position.rs:33); `convert` module wraps them at the protocol boundary with ASCII fast path; NO prefix cache v1 | `LineIndex` (position.rs:33-62), `BytePos`/`SourceLine` (position.rs:190/:270); 11 §4 already names `byte_to_utf16_cu` |
| D9 | Refresh | Subscribe token/lens refresh to 25's dirty/referrer emission on the analysis facade; hand-rolled `workspace/semanticTokens/refresh` / `workspace/codeLens/refresh` sends mirroring 25's hand-rolled diagnostic refresh; debounce on 25 T2 / b4 ≥250 ms; gate on 29 | 25 Q1-R transport (issues/25:23-24), Q11-R snapshot-diff (issues/25:33), `FileEntry::inlinks` (entry.rs:168) |
| D10 | Highlight/selection | documentHighlight = filter over walk link spans by target equality; selection S1 = position-local nesting over walk ranges; both ship in the first walk wave, no 26/11 wait | walk; `Link::target()` (links.rs:106) |

The single visibility change this whole plan needs is the already-budgeted
`LinkResolver` facade method (34:24). Everything else uses `pub` or
`pub(crate)` surfaces reachable from an in-crate LSP module.

---

## 3. Landing order

**Wave 0 (prerequisites, no feature code):**
1. `[lsp]` config section + digest (D7) and 29's capability snapshot reading it.
2. `LineIndex` three-method extension + `convert` module (D8).
3. The one `resolve_link` facade method on the analysis host (D3 seam, budgeted by 34).

**Wave 1 (walk wave; ships BEFORE 11, 14-overlay, and 26):**
4. `DocStructure` walk module with `(uri, version)` cache (D1).
5. documentSymbol: headings + tasks-as-children, line-granular ranges (D2, D6 machinery).
6. foldingRange E-lite (D6).
7. selectionRange S1 trimmed (D10.2).
8. documentLink: wikilinks, embeds, standard links; exact spans; drop unresolved; `resolveProvider:false` (D3).
9. documentHighlight: same-target matcher over walk spans (D10.1).
10. CodeLens Option C: note-level count from `FileEntry::inlinks()` (D4).
11. Semantic tokens: T1 legend, two-valued unresolved modifier, `full`-only, refresh via D9 lane (D5 v1).

**Wave 2 (waits on named tickets):**
- Three-valued resolution (token Ambiguous modifier, ambiguous-link first
  -candidate tooltips): 26's tie-reporting candidates facade.
- Template-ref documentLinks and goto-def: 22's machinery spans + Q20(b)
  overlay-aware `find` (this is the ONLY genuinely span-gated documentLink
  class, per 22/b3).
- Heading-level CodeLens counts: 26/38 heading-anchor edges, or an interim
  per-doc scan of the target note's walk (b4 priced both).
- Byte-precise non-document consumers (cross-file definition/references
  position mapping over indexed closed files): 11's AST spans, which 26 needs
  and 27 does not.
- Overlay-fresh counts/tokens for dirty buffers: 14's single-file RefreshPlan
  constructor.

Note the inversion vs pass 2: pass 2 treated 11 spans and 26 Occurrence spans
as 27's substrate. The walk makes 27's document-scope features independent of
both; 11/26 remain prerequisites only for cross-file navigation and
three-valued state.

---

## 4. Surprise findings from codegraph

1. **11 is not a 27 prerequisite at all (for document-scope features).**
   Every span 27 needs (blocks, inline events, links) is available
   transiently from `into_offset_iter()` in the walk. Pass 2's "selection is
   the purest 11-dependence" and "documentLink waits on 26's Occurrence
   spans" both dissolve. 11's AST fields remain what 26 (cross-file nav over
   closed files) and 41 (persistence) need, but 27's six document-scope
   features can all ship in Wave 1.
2. **The main parse pass already carries full spans and throws them away.**
   `handle_event` receives `ByteSpan::from(range)` (parser.rs:111) and
   `handle_start_tag` immediately narrows to `span.start()` (parser.rs:264).
   An AST-field route for `Link` would be a ~5-line change (`ActiveLink`
   already exists, parser.rs:325-326), but it would put transient LSP spans
   into the postcard-persisted `Note` (model.rs:34-36, round-trip test
   :574-626), which 11 §8 forbids. The walk route gets the same spans with
   zero persistence-format risk. This is why the walk, not an AST retrofit,
   is the cheap path.
3. **`collect_headings` is already a second parse of every note in every index
   build** (parser.rs:113, :118-162). The "single-pass note" baseline has a
   footnote: per-collector single pass, two collectors. The walk adds a third
   pass but only per-request per-open-document, cached by version; the index
   path is untouched. If 33 ever prices index-build parses, merging heading
   capture into the main pass is an independent optimization with no 27
   coupling.
4. **`InlinkMap` is consumed, not retained, at assemble time** (entry.rs:97:
   `inlinks.remove(...)` distributes sources into `FileEntry`s). Any handler
   wanting counts or existence must go through `FileEntry::inlinks()` /
   binary search, not the map. Also: attachments count as inlink targets
   (tested, inlinks.rs:944-968), so CodeLens counts on image files are
   representable, and external URLs correctly produce no edge
   (inlinks.rs:907) which is exactly the documentLink drop semantics needed.
5. **QueryService is the wrong tool shape for the LSP even setting aside
   performance**: `QueryMode` and `QueryBuilder` are `pub(crate)`
   (builder.rs:21/:55), so a separate `traces-lsp` crate (34's live option)
   could not construct task queries without another widening budget. Direct
   walks over `pub` accessors (`entries`, `note`, `tasks`, `headings`,
   `inlinks`) keep the 34 crate-split option open.
6. **`has_target` vs `resolve_link`**: `InlinkMap::has_target` (inlinks.rs:162)
   is O(1) but keyed by full resolved path; wikilink targets arrive as
   basenames (`LinkTarget::is_basename`, links.rs:268), so existence checks
   for the common Obsidian case REQUIRE the resolver, not the map. The one
   facade method is load-bearing for documentLink, tokens, and highlight
   alike; it should be prioritized in Wave 0.

---

*End of D3 design-alternatives review. Net: the shared-walk deep design stands
and gets cheaper; the walk removes 27's dependency on 11 and 26 for every
document-scope feature; one facade method plus three LineIndex methods are the
only core-code changes Wave 1 needs.*
