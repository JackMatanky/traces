# R27: Structural symbols & editor intelligence: consolidated research

Supporting research for [ticket 27](../issues/27-structural-symbols-and-editor-intelligence.md), produced by ten parallel research subagents (A1 codebase review, A2 crate research, A3 concern decomposition, then B1-B7 per-cluster external research), consolidated 2026-10-18. Parts 0-X below integrate the findings, state the leanest-maximally-capable option per feature, and list the open questions the grilling session must resolve; Appendices A1-B7 carry the full per-cluster research reports verbatim so every verified citation stays in this one file. Evidence markers inline; the general methodology rule (source/API demand vs ecosystem adoption as separate labeled judgments) was applied throughout.

## 0. Premise corrections & cross-ticket facts surfaced

These findings correct or sharpen assumptions in the ticket, the map, A3's decomposition, or settled tickets. Each is load-bearing for the grilling session:

1. **The CodeLens premise is false.** A3 (C7) claimed zero evidence of PKM CodeLens presence; in fact Marksman (`Lenses.fs`, digest `:8158-8219`) and Markdown Oxide (`src/codelens.rs`, digest `:864-946`) both ship reference-count CodeLens, **default-on**. MS markdown-ls, rumdl, and zk ship none. The ticket-05 caveat (27:10) therefore cuts *toward* inclusion for PKM parity and *away* from generic-markdown parity (B4 §0).
2. **A3's "Marksman folding/selection handlers" is a misattribution.** The `:3190/:3191` dispatch rows are Ionide's vendored C#-stub table, not live handlers; Marksman's `ServerCapabilities.Default` advertises neither folding nor selectionRange; its registered handlers are unreachable dead code (B2, B7 §2.3). Never copy Marksman's capability block: handlers exist only if the advertised capability exists.
3. **`byte_to_utf16_cu` does not exist** (ropey 1.6.1); the idiom is `byte_to_char` + `char_to_utf16_cu` (consistent with R26 §0.2). The Note AST is **flat** with headings carrying `line`, not spans; span presence for fence/frontmatter extents is design-only pending ticket 11 (A1). CodeLens ranges can ship at line granularity today (`Heading.line`), so CodeLens is the *last* of the eight bullets to need 11's byte spans (B4 lesson 5).
4. **`textDocument/codeLens` is whole-document**: there is no range parameter (LSP 3.18; corrects A1:161). Any per-view filtering is client-side (B4).
5. **rumdl's provider list, enumerated at source** (`src/lsp/server.rs`, main @ 0.3.0): documentSymbol/workspace-symbol (gated `enableSymbols`), codeAction (lint quickfix + fixAll), formatting, pull diagnostics, completion, and standard-link definition/references/hover/rename (gated `enableLinkNavigation`). **No** documentLink, folding, codeLens, codeAction-conversion, or semanticTokens (B3 §3, B7 §2.3).
6. **Semantic tokens: the ticket's working assumption is refuted.** "Syntax highlighting likely covers wikilinks adequately" is false: the VS Code built-in markdown TextMate grammar, Neovim's tree-sitter markdown query, and the upstream MDeiml grammar have **zero** wikilink rules (B6 §3). Three-valued resolution-state coloring has **never shipped** (Markdown Oxide and rust-analyzer are two-way only); Traces would be first. Local `docs/refs/lsp_spec.md:655` stubs the entire semanticTokens section; cite upstream LSP 3.18 only.
7. **tower-lsp-server 0.23.0 re-exports `ls-types` 0.0.6** (not `lsp-types`; stale doc comment), fluent-uri `Uri` newtype, consistent with R26 §0.1; 0.24.0-rc.1 re-exports `gen-lsp-types`. `Registration.register_options` is untyped JSON, so capability option payloads are ours to shape (A2 §2-3).
8. **Zed drops `selectionRange` entirely** (not just Helix): two of four target editors never send the request (B7 §1.2).
9. **MS markdown-ls is the only generic-markdown LSP precedent** (ticket 05 caveat); never treat its omissions (no semantic tokens, no codeLens) as normative; its inclusion of folding/symbols is the positive half of the same coin (B1 lesson 10, B6).
10. **pulldown-cmark 0.13.4 has no wikilink node** (16 `ENABLE_*` flags, none cover `[[...]]`); wikilink/embed occurrences come from ticket 11's planned parser extension, fence/frontmatter extents from Event streams (`Start(Paragraph…)`/`Rule`) or bounded line scans, template spans from ticket 22 (A2 §4).
11. **`WorkspaceIndex__load` is a 15-20 ms fixed floor** and must never run at request time (issue 23's bench, B7 §3.1); 4,000-line reparse measures 18.6 ms against issue 33's <10 ms interactive targets; the tension is unchanged by this research and stays visible in the 33 edits (B7 §3.1, B1 lesson 7).

## Part I, Codebase & crate substrate (A1, A2)

**Traces ground truth (codegraph review, A1).** No ticket-27 feature code exists today. What exists: `LineIndex` with only `new` + `line_at` (no UTF-16 conversion); `LinkResolver` (`pub(super)`) and the `LINKS` table / `InlinkMap` at **note granularity** (no heading-anchor edges yet); `NoteHeading` with `line`, no span; `FileEntry::inlinks()` for backlink queries; miette `QuerySyntaxError` as the only span-aware diagnostic precedent. Design-only, awaiting implementation: the nav spine (26), all LSP handler code, `src/template/analysis.rs`, `ContentResolver::text_of`, the overlay (14). The map's baseline claim (map.md:52) that `byte_to_utf16_cu` exists is wrong (§0.3); the 11↔14↔26 reconciliation R26 §0.3 settled (one LineIndex-backed conversion helper for both disk and overlay) carries over unchanged.

**Crate facts (A2).** All thirteen `LanguageServer` trait methods are surveyed with line cites; the shape we bind to is tower-lsp-server 0.23 + ls-types 0.0.6 (pin the pair). Capabilities and registrations are plain structs whose `register_options` are untyped `Json`, so `codeLensProvider`/`foldingRangeProvider`/`semanticTokensProvider` options are ours. pulldown-cmark 0.13.4 Event/Tag inventory (relevant kinds: `Start(Heading)`, `Start(CodeBlock)`, `Rule`, `Html`) plus the sixteen `ENABLE_*` flags are catalogued for the structural walk; ropey 1.6.1 conversions are `byte_to_char` + `char_to_utf16_cu` only (§0.3). Supplementary candidates surveyed; none required beyond what 24/26 already established. Tooling note: rust-docs-mcp has a server bug (relative-path `cargo metadata`); workaround is reading `.rust-docs/cache/crates/<crate>/<ver>/source/` directly.

## Part II, Outline & workspace symbols (B1)

**Response-type negotiation (C1).** Ship one handler, dual shape: nested `DocumentSymbol[]` when the client advertises `hierarchicalDocumentSymbolSupport: true`, flat `SymbolInformation[]` (with `containerName` = nearest heading) otherwise; never return null (empty Helix picker). rumdl (`commit 2cef95b`) is the reference implementation; Markdown Oxide's always-nested shape works with current editors but is spec-noncompliant. Helix requires the flat path (`hierarchical_document_symbol_support: false`, B7 §1.2).

**Range convention.** rumdl's precision: `range` = whole section (VS Code's containment invariant + click-to-select), `selectionRange` = heading name span; repair-before-send (`selectionRange ⊆ range`, children ⊆ parent) as a cheap invariant check à la typescript-go; VS Code rejects whole trees on violation. Kind: `SymbolKind.String`; name from 23's `clean_text()`. Hash-prefix `## ` naming (MS) is precedent-split, an open grill choice.

**What goes in the outline (C2, NOVEL).** Zero PKM precedent for tasks or File-Class fields in documentSymbol: every markdown server ships headings-only, and org-lsp/Patto refused task-spillover outright. Three options for tasks: (5a) headings-only v1, tasks reachable via workspace/symbol only (safest for the <10 ms p95 contract); (5b) tasks as `Array` children of the nearest heading, `detail` = state, the only option matching 23's VII.4 floor-safe kind plan, but it re-opens 33's documentSymbol p95 row for cold-LRU measurement; (5c) tasks behind 3.16 `labelSupport` opt-in. File-Class fields: top-level symbols with floor-safe kind (`Field`/`Property`) and `detail` = value preview, or workspace/symbol only. Tags stay out (16's decision stands; Marksman's inclusion is why its outline is called noisy).

**C1/C2 data point (amended 2026-10-21 by iwe research):** iwe ships a flat depth<4 document-symbol outline and a 100-cap workspace-symbol search ([tool-iwe](tool-iwe.md) Part 6).

**Caps.** Impose 23's 1k/5k node caps despite no markdown server capping anything: the VS Code freeze data (25K entries → 5.3 s) justifies the divergence. Cap behavior (truncate + `// …N more` node vs kind-filter) is an explicit 27-vs-23 joint decision.

**Workspace/symbol.** Follow 16's shape (unique tags/file-classes as symbols). Lazy `workspaceSymbol/resolve` (3.17) is unused by all markdown peers (novel-but-standard, addresses the 15-20 ms reload floor), **but** B7 §1.2 finds resolveSupport is VS Code-only and recommends eager ranges for everyone (26's spine makes `RangeInFile` cheap): one code path, not two. This paper's recommendation is **eager**; flagged as an open question because B1's resolve idea is the only alternative with a measured motivation (§ Part X.7).

**rumdl coexistence (C12 head).** rumdl's `enableSymbols` defaults true → duplicate outlines until users flip it. 27 documents the cession contract (`tool-rumdl-boundary.md:64`) and recommends `enableSymbols = false` (rumdl's own documented joint pattern); Traces does not gate on `clientInfo` (Marksman's VS Code identity-gating is a curiosity, not a model).

## Part III, Folding ranges & selection ranges (B2)

**C3 folding: ship, minimal-then-extend.** First cut (**C3-min**): heading sections (fold to next same-or-higher level, trailing blanks trimmed, single-heading-no-content produces no fold), fenced code blocks, YAML frontmatter. Line-only ranges, matching VS Code `lineFoldingOnly`; `kind` left unset for all three (MS convention: keeps the client's "Fold all regions" command honest; Panache's Region-stamping is the anti-pattern). Return null when empty (MS and Panache both do). Follow-up (**C3-ext**): list-item folds (lazy continuation to end-of-file, MS-tested), callout/blockquote folds gated on 17. Skip `rangeLimit` handling initially (vault notes are orders of magnitude under VS Code's 5,000 hint); revisit only with evidence. No cancellation machinery: one synchronous O(document) walk under 12's sequential dispatcher.

**C4 selection ranges: omit from the first cut.** Zed and Helix never send the request (§0.8), Neovim prefers treesitter, VS Code already ships a superset for Markdown, and Panache deferred it too. Record the decision; re-open only on a demonstrated target-editor workflow. If ever pursued: block-tier chain only (`paragraph → list item → section`) built as parent pointers from the *same* structural walk; no inline tier (no inline spans in the AST) and never a second ad-hoc walker.

**Shared infrastructure regardless:** land `structural_walk` as the ticket-32 structural-intelligence core (C3 filters it; outline consumes it; C4-B would join as a third consumer); kinds discipline as above; sequencing after 17 (callout extents) and 11's span additions.

## Part IV, Document links & template references (B3)

**Emission set (C5), the non-overlap rule.** Emit documentLinks for exactly what rumdl cannot parse: wikilinks, embeds, cross-file `file.md#heading` links (15's ranges, 25's ownership), and template `{% include %}`/`{% extends %}` spans (C6). Do **not** emit standard relative Markdown links, anchors, or disk-target images: rumdl's navigation owns them; the split is parseable-syntax-disjoint, not toggle-dependent. (Caveat from §0.9: MS's standard-link omission is the only generic precedent; Traces follows it for coexistence reasons, and the decision can be revisited on evidence.)

**Ambiguity and existence (26's predicates).** `Unique` → eager `target` (zk model; resolution is already computed for 26's predicate, no extra I/O). `Ambiguous` → no `target`, tooltip like `"N candidates (gd to choose)"`; the spec's missing-target contract renders-but-does-not-open, the honest UX. Never drop unresolvable links (zk's behavior removes affordance exactly when feedback is needed and violates standing constraint #1). Existence/overlay state surfaces via tooltip; emission is never gated on existence.

**Resolve.** `resolveProvider: false` in v1; adding MS-style `data` + `documentLink/resolve` later is a non-breaking upgrade (clients re-request resolve only when target is missing).

**Template references (C6): both channels, one resolution pass.** No template LSP uses documentLink (jinja-lsp/fjkit use goto-definition); zk uses documentLink only; nobody serves both for the same span. Recommended option **A**: template spans get **both** documentLink (click/gx/gf) and goto-definition (`gd`, and the Helix floor), fed by identical 22-derived spans + the Q20(b) overlay seam, so they cannot drift (one resolution pass, two renderings; C15 discipline). No `Blocked by: 22` edge: expressible as "when 22's spans are available", with definition-only (option B) as the degradation. No double-results risk with rumdl's `enableLinkNavigation` definition provider: rumdl navigates standard links only, position-disjoint; document this rather than adding a toggle.

**Client quirks (feeds C11).** Helix rides its existing `gf` flow (250 ms debounce, PR #15235); `tooltipSupport` is VS Code + Zed only (tooltip enrichment wasted on Neovim/Helix); per-doc version-stamped cache invalidated by didChange (26's extraction path makes recompute cheap).

## Part V, CodeLens reference counts (B4)

**Premise correction first** (§0.1): two PKM peers ship reference-count CodeLens default-on; the generic trio ships none. Attach points (both PKM peers): all heading levels, no precedent filters, plus one file-level lens at line 0.

**Four options:**

- **Option A: lazy, count-only, default-off (recommended if in scope).** `resolveProvider: true`; discovery = document-local heading walk emitting line-granularity ranges (no span dependency) + file-level lens; `data = {uri, version}` with rust-analyzer's stale-version guard; resolve = count from the resident nearest-picked inlink/anchor multimap (26's "27's counts", overlay-adjusted via 14), count-only payload, per-lens "999+" cap, always return a command (Neovim re-resolves command-less lenses forever); config `[lsp.codeLens] references` default **false** with rust-analyzer's early-return-before-any-index-work idiom. Honors the spec's two-stage design, bounds each unit of work under no-cancellation dispatch, costs nothing when off. Cost: zero-count suppression impossible without eager counts ("0 references" shows when enabled).
- **Option B: eager, zero-suppressed, `resolveProvider: false` (PKM-peer shape).** Counts at discovery from the resident multimap (NOT per-element workspace scans; Marksman's 339 µs × headings is the failure receipt), suppress 0/≤1, one round trip. Requires a measured hub-note worst case before committing, and makes heading-anchor edges a T1 obligation (13/41 index-shape decision).
- **Option C: file-level only, v1.** "3 references" to the *note* via today's `InlinkMap`/`FileEntry::inlinks()`, moxide's line-0 convention. The leanest approach that stays maximally useful; no nav-spine or span dependency; loses the per-heading novelty that is the ticket's headline example; also the only variant buildable before 26's machinery lands.
- **Option D: out of scope (legitimate).** The only 27 bullet with zero generic-markdown precedent, display-only, best-effort under 26's ambiguity semantics, invisible on the Helix floor, competing with Obsidian's backlinks-panel convention. Revisit when a backlinks panel/hover exists anyway, 33 has measured count costs, or a hub-note auditing workflow demands inline counts.
- **Option E: inlay hints (iwe precedent, 2026-10-21).** iwe displays backlink count + parent title as inlay hints at the note top, refreshing via `workspace/inlayHint/refresh` pushed ~100ms-delayed (spawned sleep before the push) and only if inlay hints were previously used — a reusable gating idiom for tickets 12/33's refresh discipline. Inlay hints display the same counts CodeLens was proposed for, at lower UI cost; unlike CodeLens (explicitly Helix-absent per the Part VIII matrix, PR open since 2022), inlay hints carry no recorded Helix-floor exclusion in this file at all (the client matrix simply has no inlay-hints row — an absence of evidence, not evidence of absence; [tool-iwe](tool-iwe.md) Part 6).

**Discipline regardless of option.** Never copy moxide's payload shape (all locations pre-serialized per lens); count-only `data` + stale-version guard fits 12's stale-result-discard model. Refresh: default none (both PKM peers manage client-side) or capability-gated, T1-coalesced `workspace/codeLens/refresh` on settled index swaps (r-a template); never per-keystroke. Counts must be documented best-effort for stem-tie ambiguity (26:31). CodeLens may never carry information available only there: supplementary to hover/references/backlinks (Helix has no CodeLens at all, PR open since 2022).

## Part VI, Code actions (B5)

**One provider, four kinds.** Single registration serves C8 + C9 + status-cycle: `quickfix` create-note-from-broken-wikilink (25's `data` channel), `refactor.convert.*` (C8), `refactor.extract.note` (C9), `source.traces.statusCycle` (23-invariant). `resolveProvider: false` v1 (Marksman pattern) unless C9's edit grows expensive; `dataSupport` needed regardless for the diagnostic-derived quickfix.

**C8, markdown-link ↔ wikilink conversion (NOVEL).** No surveyed LSP does wikilink conversion; the only adjacent precedent is IWE-style extract-to-note (C9). Recommended defaults: both-way direction pair; path style + extension config (MS/Better-ML analog); alias-preserving both directions; bang-preservation for embeds; reference-style links out of scope v1; per-occurrence scope v1 (bulk variant later, 5000-occurrence tier <50 ms).

**C9, extract selection to new note.** Collision policy (suffix vs refuse vs candidate-actions), link shape (`[[stem|text]]` vs `[[stem]]` vs first-line alias), and destination (same-dir v1) are grill choices; eager edit construction (precedent uniformly eager, budget fits); from-template stays deferred to 28; undo = client-side with preview as the safety, compensating command is 28-priced v2.

**Safety invariants to write into the ticket.** Pre-validation mandatory (no rollback exists, 26:83-85); **create-first** op ordering (abort semantics; `create` gate shared with 25 via one core predicate, 29 plumbing); refuse empty/frontmatter/fence selections; never `overwrite: true`.

**Perf rows proposed** (Part IX table). Couplings to restate: 25/26 (transport + predicates + refusal matrix), 28 (file-ops boundary, undo-compensation pricing), 29 (registration), 32 (boundary table), 33 (rows). Open verification gap: VS Code multi-file WorkspaceEdit undo-grouping behavior, belongs to C11's editor sweep.

## Part VII, Semantic tokens (B6)

**The value story flips on the syntax baseline** (§0.6): in VS Code and Neovim out of the box, `[[wikilink]]` gets *no structural coloring whatsoever*, because no grammar has a wikilink rule. C10 would not be "state nuance on a colored baseline"; it would be the **only first-party coloring channel for wikilinks**, where even uniform entity coloring is already an upgrade. Ranked unique values: (1) first-ever coloring of wikilinks at all in VS Code/Neovim, (2) **ambiguity visibility**: 15 deliberately emits no diagnostic for `Ambiguous`, so tokens are the only vehicle for that state, (3) ambient resolved/unresolved without squiggle noise (a second channel over 15's Information diagnostic; real but incremental), (4) create-note discoverability reinforced visually.

**Precedent.** Markdown Oxide is the direct precedent, config-free: two-way state mapped onto two *standard* types (`DECORATOR` resolved / `COMMENT` unresolved), modifiers 0, default-on, `semantic_tokens_refresh` pushed after every vault rebuild. rust-analyzer has a single custom type (`UNRESOLVED_REFERENCE`) and is actively trying to *reduce* non-standard types (issue #13066). clangd's mature design *escaped* semantic tokens for region-shaped state. **Three-valued has never shipped**; keep the legend dumb to survive that novelty.

**Legend design.** 26 mandates three-valued modifiers, but modifiers are a bitfield (three mutually exclusive bits is legal-but-odd), and per-entry client cost is: standard type = themed everywhere; custom string = themed nowhere without per-editor artifacts (VS Code companion extension, Neovim `hi link`, Zed `semantic_token_rules`). Options: **L1** three standard types (config-free, MO-proven, slight semantic oddity), **L2** entity standard type + 3 custom state modifiers (matches 26's wording; state differentiation needs per-editor config), **L3/L4** custom types (invisible until configured; not recommended). This paper's recommendation: **L1** as the leanest maximally-capable choice, with 26's "modifiers" wording treated as the intent (three-valued state on the wire) rather than the literal bitfield. **Flagged conflict**: B7 lesson 4 recommends modifiers-only legend + full/delta transport; B6's dedicated analysis (standard-type config-free argument + zero-grammar finding) supersedes it for legend choice, and the transport disagreement (delta vs full-only) resolves to **full-only** below.

**Transport and refresh.** Full-document only; **no delta, no range** (delta buys nothing at PKM scale: ~0.02 ms/10 KB parse, 5,000-ref ceiling is workspace-wide and distinct; MO's shape is the lean precedent). Compute on request from the current overlay (14's freshness falls out free). **No proactive refresh in v1**: cross-file staleness persists until next edit; if refresh ships later, debounce behind index-mutation events, never per-keystroke (the 200 ms client debounce plus 12's single sequential lane makes MO's push-after-every-vault-rebuild "absolute care" territory). Advertise `serverCancelSupport: false` (12's sequential truth). Tokens must stay single-line and non-overlapping (VS Code ignores anything else, even though Neovim/Zed accept both). Gate emission on the client advertising any known tokenTypes (29's matrix). Entities v1: wikilinks + embeds; tags join with 16's index (second pass). Client reach: VS Code (theme-dependent; `configuredByTheme`), Neovim (live on any colorscheme for standard types), Zed (default-off, must flip `semantic_tokens`), Helix (never; ignored). Ship per-editor config snippets in docs.

**Honest out case (O4, recorded for the grill).** Helix ignores tokens; Zed default-off; VS Code third-party themes may disable them; only 1 of ~6 surveyed servers colors resolution state; unresolved duplicates 15; no 33 row exists yet. The counterweight: the comparison class is "markdown users with no coloring at all", and MO chose default-on.

## Part VIII, Client matrix, registration & shared layer (B7)

**Support matrix (C11, verified at source).** Eight features × four editors:

| Feature | VS Code | Neovim | Zed | Helix (floor) |
|---|---|---|---|---|
| documentSymbol (hierarchical) | yes | yes (`gO`) | yes | **flat only** |
| workspace symbol | yes + lazy resolveSupport | yes, eager ranges needed | yes | yes |
| foldingRange | yes (`lineFoldingOnly`) | yes (0.11+, `collapsedText`) | yes (`collapsedText`) | **no** |
| selectionRange | yes | yes (0.11+) | **no** | **no** |
| documentLink | yes + tooltip | yes (`gx`) | yes + tooltip + resolve | yes (2026-03, via `gf`) |
| codeLens | yes + resolve + refresh | yes (eager resolve-at-display) | yes (since 2026-04) | **no** |
| semanticTokens | full+delta+range | full+delta, overlapping/multiline ok | full+delta (mode default-off) | **no** |
| codeAction | yes | yes | yes | yes |

Quirks that bind design: Neovim never advertises workspace-symbol resolveSupport and refuses multiline/overlapping tokens; Helix documentSymbol must be dual-shaped and its link UX is `gf`; lazy workspace-symbol resolve is VS Code-only (Part II recommends eager for all); tooltip is wasted on Neovim/Helix; the four Helix-absent handlers must be capability-skipped no-ops, not errors (r-a "lens config off, return before any DB query" pattern). No feature is dead weight; no feature may require the sparse floor to send anything it won't.

**Registration (C13): all eight static, gated by initialize-time config flags** (the rumdl model). Dynamic registration is a VS Code-only nicety for these features (Neovim: codeAction only; Zed/Helix: none), none needs mid-session re-binding, and the spec forbids dual registration. Keep the r-a gated-`didSave` pattern in reserve for the future vault-watcher (`didChangeWatchedFiles`). Anti-patterns to avoid: Marksman's unreachable handlers, and Marksman's VS Code identity-gating (Traces wants its own outline everywhere).

**Shared layer (C15): one resolve pipeline, many thin protocol adapters.** Bind all eight consumers to 26's spine (`Occurrence`/`OccurrenceKind`/`NavTarget`/`TargetKind` + `locate`/`resolve`/`candidates`/`references`/`rename`/`highlight` on the `NavContext` facade) and nothing else: folding = heading Occurrence ranges; documentLink = link Occurrences with Resolution targets; codeLens = placeholders from occurrence pass, resolve = Resolution counts; semanticTokens = Occurrence-kind classifier + Resolution state mapping; symbols = heading/symbol slice of the same index; codeAction = 26's actions verbatim. **Ceilings**: one constants module, two named families that must never be merged: `SYMBOL_CEILING` (1k/5k nodes, 23) and `REFERENCE_CEILING` (5,000, 26). **Predicates** in core, never in DTOs (standing constraint #3): `resolvable`, `create_supported` (25's `create` check), `ambiguous`, plus the `Resolution` → token-classifier mapping. One shared capability snapshot at the protocol boundary drives: hierarchical-vs-flat shape, `collapsedText`/tooltip emission, Helix early-returns, and fallback when the client never asked.

## Part IX, Consolidated leanest-maximally-capable recommendations

Per-feature summary for the grilling session (each row is a recommendation, not a decision; grill before accepting):

| # | Feature | Leanest-maximally-capable call | Depends on |
|---|---|---|---|
| 1 | documentSymbol | **IN.** One handler, dual shape (hierarchical per client cap, flat for Helix); headings-only v1 (5a); `range` = whole section, `selectionRange` = heading span; `String` kind; 1k/5k caps with explicit truncation; no tags | 21 `clean_text`, 23 kind plan, 33 row, rumdl `enableSymbols=false` documented |
| 2 | workspace/symbol | **IN.** Tags-as-unique-symbols per 16; **eager ranges for all clients** (one code path); fill the UNFILLED p95 row in 33 | 16, 23 caps, 33 row |
| 3 | foldingRange | **IN, C3-min**: headings + fences + frontmatter, line-only, kind unset, null when empty, no rangeLimit handling; C3-ext later (lists; callouts gated on 17) | 11 span additions (or bounded line scan), `structural_walk` (32) |
| 4 | selectionRange | **OUT of v1 (C4-A)**: two of four editors never send it; re-open on demonstrated demand; never a second walker | 32's walk if revived |
| 5 | documentLink | **IN.** Wikilinks/embeds/`file.md#heading`/template spans only; Unique eager target; Ambiguous no-target + count tooltip; `resolveProvider: false`; never drop, never error | 26 predicates, 15 ranges, 22 spans (templates) |
| 6 | template refs (C6) | **IN, option A**: both documentLink and goto-definition over one resolution pass | 22 spans, 29 advertisement |
| 7 | codeLens | **IN or D, grill hard.** If in: Option A (lazy, count-only, default-off) unless the grill picks C (file-level, buildable before 26) or B (eager zero-suppressed, needs hub-note measurement); refresh none-or-T1-coalesced | 26 spine (A/B), InlinkMap (C), 33 rows |
| 8 | codeAction | **IN.** One provider, four kinds; C8 both-way conversion; C9 extract eager; pre-validation mandatory; create-first ordering; `create` gate shared with 25; never `overwrite: true` | 25/26 transport, 28 file-ops boundary, 29, 33 rows |
| 9 | semanticTokens | **IN, lean shape**: L1 legend (three standard types, config-free, MO-proven) + full-only request-time compute from overlay; no delta/range; no refresh v1; `serverCancelSupport: false`; ambiguity coloring is the unique payload; per-editor config snippets in docs | 26's `Resolution`, 14 overlay, 29 capability gate, 33 row |
| 10 | registration | **IN (C13)**: all eight static, initialize-time config flags (rumdl model); no dynamic registration | 29 owns advertisement |
| 11 | shared layer | **IN (C15)**: bind to 26's spine; `SYMBOL_CEILING` ≠ `REFERENCE_CEILING` in one constants module; predicates in core; one capability snapshot | 26, 23, 25 |

**Proposed latency rows for issue 33 §3** (planning budgets, not measurements; cluster proposes, 33 prices; B7 §3.5 + B4 lesson 3 + B5 §4 merged; B4's numbers supersede B7's tentative codeLens row):

| Capability | Target p95 | Stretch | Strategy |
|---|---|---|---|
| Workspace symbols | <5 ms | <2 ms | Resident index only (no `__load`); 23's bench shows 0.665-1.77 ms at 20-100K rows; 18 ms @ 1M rows is out of stated PKM scope, document the degradation |
| Document symbols | <10 ms (existing row; re-price if 5b tasks ship) | <5 ms | Heading index slice + overlay; cold-LRU measurement required before tasks-in-outline |
| Folding ranges | <10 ms | <5 ms | Shares the documentSymbol heading walk; honor client `rangeLimit` when introduced |
| Selection ranges | <5 ms | <2 ms | Only if C4 revived; ancestor chain, no vault access |
| Document links (full) | <10 ms | <5 ms | Per-doc outgoing-link extraction already on 26's didChange path |
| Document link resolve | <5 ms | <2 ms | Only if resolve ships; path materialization only (25's create-target logic) |
| CodeLens discovery | <10 ms | <5 ms | Line-granularity heading walk; placeholders only under Option A |
| CodeLens resolve (per lens) | <5 ms | <1 ms | Single-target count from resident multimap, `REFERENCE_CEILING`-capped |
| CodeLens eager-all (only if Option B) | ≤20 ms | n/a | Find-refs class; requires measured hub-note worst case before default-on |
| Semantic tokens (full) | <20 ms | <10 ms | One classifier pass over parse spans + resolver lookups; full-only keeps zero resultId bookkeeping |
| codeAction (Invoked) | <10 ms | n/a | Range→occurrence lookup + in-memory resolution + one TextEdit; strictly cheaper than rumdl's re-lint floor |
| codeAction (Automatic) | <3 ms | n/a | Construction skipped/downsized; measures the lightbulb-scan tax |
| codeAction/resolve (if C9 uses resolve) | <20 ms | n/a | Name generation + collision scan + skeleton build |
| codeAction bulk (if shipped) | <50 ms | n/a | 26's rename-scale budget tier |

All rows ride the existing per-document request budget; none touches the workspace tier. The 18.6 ms reparse vs. <10 ms hover tension from 23 stays flagged in the same section of 33.

## Part X, Open questions for the grilling session

1. **Tasks in the outline**: 5a (headings-only, tasks via workspace/symbol; preserves the performance contract), 5b (tasks as `Array` children; matches 23's VII.4 kind plan; forces a cold-LRU re-measurement and a 33 row re-price), or 5c (`labelSupport` opt-in)?
2. **File-Class fields in the outline** at all, or is workspace/symbol their natural home (16 gave tags that answer)?
3. **Cap behavior** for the 27-vs-23 joint decision: truncate with a `// …N more` node, or kind-filter?
4. **CodeLens option**: A (lazy default-off), B (eager zero-suppressed, measured), C (file-level v1, honest upgrade path), or D (out, revisit on triggers)? Note the eager-vs-lazy zero-suppression coupling and that C is the only variant buildable before 26's machinery exists.
5. **Semantic-token legend**: L1 (three standard types, config-free, this paper's recommendation; treats 26's "modifiers" as intent) vs L2 (literal 26 modifiers + entity type, state needs per-editor config)? Does a VS Code companion extension ever enter scope (the only path to "just works" beyond standard types)? Tags in v1 entities or deferred to 16?
6. **selectionRange**: confirm C4-A (omit) rather than C4-B (block-tier later).
7. **Workspace/symbol**: eager ranges for all (B7, this paper) vs `workspaceSymbol/resolve` novelty (B1:131, addresses the 15-20 ms reload floor but is VS Code-only in practice)?
8. **Heading naming in flat lists**: MS-style `## ` hash prefix or clean text only?
9. **rumdl joint setup**: confirm `enableSymbols = false` documentation as the single required toggle; test both states.
10. **Semantic tokens in/out**: does the zero-grammar-coverage finding (strongest value argument) beat the honest-out case (O4: Helix-never, Zed-default-off, 15 overlap, no precedent for three-valued)? If in, is "no refresh v1" acceptable staleness?
11. **Folding sequencing**: is C3-min acceptable post-11, or must fence/frontmatter extents block on span work?
12. **CodeAction defaults**: C8 both-way direction and C9 collision policy/link shape need explicit picks from B5's option lists.

## Sources

**In-repo**: Appendices A1-B7 below (each with its own Sources section); issues 11/12/14/15/16/17/21/22/23/24/25/26/27/28/29/31/32/33/35; `map.md`; `research/tool-rumdl-boundary.md`, `research/26-definition-references-hover-rename.md`, `research/33-lsp-performance-targets.md`, `research/24-completion-architecture.md`, `research/x-lsp-performance-best-practices.md`; local digests under `docs/digests/` (Marksman, Markdown Oxide, MS markdown-ls, rumdl, zk).

**Upstream (fetched by subagents)**: LSP 3.18 spec sections for all eight features + registerCapability/semanticTokens (local vendored spec stubs these; upstream authoritative); microsoft/vscode-languageserver-node@main capability builders; neovim/neovim@master runtime docs + protocol (PRs #31311, #30781, #32704, commits f864b68c, 6072153796d1); zed-industries/zed@main `lsp.rs`/`lsp_store.rs` (PR #46356, issues #7450/#11565/#58864); helix-editor/helix@master (`client.rs`, `commands.rs:1382-1440`, PR #15235, issue #8438); rust-lang/rust-analyzer@master (`capabilities.rs`, `semantic_tokens.rs`, `request.rs:1687-1750`, `main_loop.rs:545-556`, issue #13066); rust-lang/rust-analyzer issue #16441; clangd features docs + `SemanticHighlighting.h`; rumdl `src/lsp/server.rs` @ 0.3.0; VS Code markdown TextMate grammar; nvim-treesitter `queries/markdown/highlights.scm`; MDeiml/tree-sitter-markdown `grammar.json`; gopls v0.12 scalability blog (via R26); Obsidian Backlinks docs.

*End of findings. Research consolidation complete; ready for the ticket-27 grilling session.*

---

# Appendices: full cluster research reports


---

<a id="appendix-a1" />

## Appendix A1: A1: Codebase review for ticket 27 (structural/editor intelligence)

## A1: Codebase review for ticket 27 (structural/editor intelligence)

Research fan-out part A1 of 6: current-state review of the Traces codebase relevant to ticket 27 (document symbols/outline, folding ranges, selection ranges, document links, template references as links, CodeLens, code actions, semantic tokens). Every claim cited `file:line`. Companion parts (A2-A6) handle crates, external-LSP research, and consolidation.

Orientation sources: `.scratch/md-pkm-lsp/map.md:1-163` (Destination, Notes, baseline facts, Decisions so far), tickets 27/26/11/13/14/25/22/20/23, glossary `src/GLOSSARY.md:70-115`.

---

## (a) What exists and how it works

### A1.1 Note AST (`src/note/**`)

**`Note`** is a flat record: `path`, `frontmatter: Option<Frontmatter>`, `lists: Box<[ListItem]>`, `headings: Box<[Heading]>`, `outlinks: Box<[Link]>`, `inline_fields: IndexMap<FieldKey, Box<[NoteFieldValue]>>`, `tags: Box<[Tag]>` (`src/note/model.rs:24-33`). Accessors: `headings()` `:98`, `outlinks()` `:105`, `tasks()` (filters `kind().is_task()`) `:191-193`, `descendants(parent_idx)` (depth-based contiguous scan) `:202-211`.

**`Heading`: flat node, `level: u8`, markup-stripped `text: String`, `line: SourceLine` (1-indexed line of heading start only, **no byte span**) (`src/note/heading.rs:10-14`, accessors `:31-47`). Collected by a **second parser pass** (`collect_headings`, `src/note/parser.rs:118-162`) using `HEADING_OPTIONS` (= MARKDOWN_OPTIONS + `ENABLE_MATH`, `:73`) so math is stripped from display text; the heading's `line` is derived from `range.start` of the Heading event (`:140`). Handles setext headings and headings inside blockquotes/lists (test names at `:659`, `:751`).

**`ListItem`: flat, document order: `text: ListText` (raw + clean dual repr), `kind: ListItemType` (`Plain | Checkbox | Task(TaskListItem)`), `depth: u8`, `line: SourceLine`, `parent: Option<SourceLine>`, `is_ordered`, optional inline `fields`, `tags` (`src/note/lists.rs:41-50`; builders `with_depth` `:85`, `with_parent` `:93`). Hierarchy is reconstructed from depth/parent, not child containers (module doc `:4-7`). Nesting depth and parent linkage exist today, usable for list-nesting folding and task-outline nesting, but only at **line granularity**.

**`Link`: `target: String`, `text: Option<String>` (alias/display), `kind: LinkType (Markdown | Wikilink)`, `embedded: bool` (`src/note/links.rs:28-34`). **No position of any kind.** `target_parts()` splits at first `#` into `Link | AnchorOnly | PathWithAnchor` (`:113-126`). Wikilink parse helpers `parse_wikilink`/`parse_wikilink_prefix` (`:62-101`).

**`Frontmatter`: bare `fields: IndexMap<FieldKey, NoteFieldValue>` (`src/note/metadata.rs:96-97`); **no spans, no raw text retained**. YAML parsed via noyalib `from_str_with_config` with `serde_yaml_compat` preset (`src/yaml.rs:5-29`), plain `Value`, **`Spanned<T>` extraction not wired** (ticket 19's design).

**Parser plumbing: `parse_markdown` runs `pulldown-cmark` `Parser::new_ext(...).into_offset_iter()` and converts every event range to `ByteSpan` (`src/note/parser.rs:106-114`); `handle_event` receives the full `ByteSpan` (`:241`) but **only `span.start()` is forwarded** to `handle_start_tag` (`:247`); links record `(link_type, dest_url)` with no span (`:271-278`); code blocks record nothing but block state (`:279`, `BlockContext::CodeBlock` `:172`); footnote references push their source text into the scan buffer via `span.to_range()` (`:252-255`), **no footnote index**. `MARKDOWN_OPTIONS` already enables `ENABLE_YAML_STYLE_METADATA_BLOCKS | ENABLE_WIKILINKS | ENABLE_FOOTNOTES` (`:68-70`). Blockquotes only affect list-nesting state (`:288-291`), **no callout type detection anywhere in `src/`** (`rg callout` → zero hits). Frontmatter is buffered as raw YAML text (`metadata_buffer`, `:322`, `start_metadata_block` `:395-398`) then parsed; the buffer's byte extent is not retained on the resulting `Frontmatter`.

**Inline lexer: the `logos` lexer already produces `Spanned<ItemToken>` (`src/note/parser/lexer.rs:43-49`, `Spanned::from_usize_range`); spans drive task-shorthand scanning (`src/note/parser/task.rs:24-37`) and code-span exclusion (`src/note/parser/list.rs:212-264`) but are **discarded before reaching `Note`**.

### A1.2 Position model (`src/position.rs`)

- `LineIndex(Box<[BytePos]>)`, precomputed line starts, `line_at(BytePos) -> SourceLine` via `partition_point` (`src/position.rs:33-61`). **No `byte_to_utf16_cu` method exists yet** (rg: zero hits in `src/`), despite 11 §4/26 §7 design text.
- `ByteSpan`, `start`/`end` `BytePos` newtype, full serde + `Default`, invariant `start <= end` (`:79-93`, conversions `:132-169`).
- `BytePos(u32)`, saturating `saturating_from(usize)` (`:190-213`), `TryFrom<usize>` fallible (`:246-253`).
- `SourceLine(NonZeroU32)`, 1-indexed, hand-rolled serde (`:270-355`).
- `Spanned<T> { value, span: ByteSpan }`, **no serde derives**; span-inclusive `Eq`/`Hash`, manual value-then-span `Ord` (`:359-380`; ordering pins cited in research/40 at `:853`/`:863`). Production users are transient token streams only (research/40 `:33`).

### A1.3 Navigation spine (ticket 26), design-only; zero code

`rg "Occurrence|NavTarget|NavContext" src/` → **only `src/GLOSSARY.md`** (`:72-105`: Occurrence, NavTarget, Resolution, Ambiguity/StemTie/HeadingAnchor/PathTie, Comparison Key). There is **no `nav` module, no `Resolution` enum, no `locate`/`candidates`/`references_at`/`rename_at`/`highlight`** in `src/`. Key design commitments from ticket 26 that gate/shape 27:

- Spine facade `NavContext` exposes `locate`/`resolve`/`candidates`/`references`/`rename`/`highlight`; `nav::locate` classifies **navigation entities only** (26:23).
- Three-state `Resolution { Unresolved | Unique | Ambiguous{kind, candidates} }`; distance = LCA folder distance; `AmbiguityKind = StemTie | HeadingAnchor | PathTie` (26:25).
- Consumer matrix: definition all-candidates; hover bounded; references capped union; prepareRename/rename refuse `-32803`; **documentHighlight is document-local span matching and never errors on ambiguity** (26:27; echoed in ticket 27's Inputs-from-26).
- References: 5,000 soft cap scoped to the references response path only, named constant "fed to 27/29"; graph-vs-nav divergence, **the inlink graph keeps nearest-picked edges; "backlink counts, 25's change-summary soundness, 27's counts, 28's rewrite targeting all consume them"** (26:25, 26:31).
- Position encoding: negotiate utf-8-if-offered-else-utf-16, one `convert` module at the protocol boundary; `byte_to_utf16_cu` retained as a thin `LineIndex` method (26:7 / ticket 11 Amended-by-26 `:78`).
- Two named changes to existing code, budgeted: **expose `LinkResolver` via a facade method and expose `paths_with_tag` the same way** (26:23).
- `ls-types` ↔ `tower-lsp-server` pairing pinned (26:45); neither crate is in `Cargo.toml` today.

Ticket 27's own Inputs-from-26 block (`27-…md:21-26`) additionally pins: documentLink emits for Unique **and** Ambiguous (never errors); semantic-token modifiers three-valued (resolved/unresolved/ambiguous); documentHighlight registration belongs to 27; 5,000-cap kept distinct from symbol caps; create-note action checks `create` in `resourceOperations` (shared predicate with 25's code-action path).

### A1.4 Link resolution & backlink data (`src/index/**`)

- **`InlinkMap(HashMap<PathBuf, Box<[PathBuf]>>)`: target → canonically-sorted deduplicated source paths (`src/index/inlinks.rs:59`, construction invariants `:53-58`). Populated per `FileEntry` at assemble time (`src/index/entry.rs:97-99`); `FileEntry::inlinks()` public (`entry.rs:167-170`, cited by 25 Q11 as load-bearing). **This is the file-level backlink data a reference-count CodeLens could consume today: but it is file-level only; per-heading counts need 26's occurrence machinery (not built).
- **`LinkResolver`: `files: &[FileMeta]` + `basename_index: FxHashMap` (`inlinks.rs:319-322`); `resolve()` tiers: exact path → missing-`.md` path → nearest-by-basename (folder-distance trie, `src/index/trie.rs:22`, `:49`) (`inlinks.rs:369-401`). **Returns `Option`, equal-distance ties collapse to `None` (unresolved)** (`:367-368` comment "Equal-distance basename ties stay unresolved"). This is exactly the `ties_resolve_to_none` behavior 26 keeps as the Unique-side spec while adding three-state classification on top (26:25). `resolve_edges_for` builds `(target, source)` edges sharing one resolver (`:303-316`).
- **redb tables: `LINKS` multimap (`src/index/tables.rs:34`, spec `:110`); `read_links` indexed lookup per target with target/source filtering (`src/index/store.rs:215-230`, `:476`, `:537`); `InlinkMap` re-collection from the table (`store.rs:1079`). Seven tables total, no spans/raw text persisted (map.md:53; research/13 confirms table list).
- **`WorkspaceIndex`: `entries: Box<[FileEntry]>` sorted by path, `assemble` merge-joins files/notes/inlinks (`entry.rs:22-24`, `:39-45`, `:80-103`); `entries()` public (`:108`), `entry_at(RowIndex)` (`:124`).
- **`unique_tags()` / `unique_file_classes()`, do not exist** (rg over `src/`: zero hits). They are ticket 21 prerequisites (map.md:88), not yet implemented; anything workspace-symbol-shaped over tags/classes waits on them.
- **`IndexerService`: `build`/`refresh`/`refresh_with_report`/`load` (`src/index/service.rs:74`, `:107`, `:125`, `:309`); sole production `parse_markdown` caller at `:276` (the 25 Q15-R2 BrokenLink side-channel would wrap exactly this call, design-only).

### A1.5 Refresh & overlay seams (tickets 13/14), design-only; seams located

- **`RefreshPlan`: fields `store/current_files/persisted_files/delta` (`src/index/refresh.rs:260-265`); **only constructor is `collect(root)` (whole-tree scan + store open in `rayon::join`)** (`:281-299`); `reconcile(modified_notes, persisted: InlinkMap)` patches inlinks when paths unchanged (`:338-359`). **No single-file constructor exists: 13's answer (13:31-45) and 26's batch-constructor note (26:33) are design-only.
- **Buffer overlay / ContentResolver: entirely design (ticket 14:23-33): `RwLock<HashMap<Uri, BufferState{content: String, version: i32}>>`, `ContentResolver::note_of/text_of` returning `Cow`, per-request inlink adjustment with cached `LinkResolver`, overlay-keyed `Uri` (ls-types' fluent-uri, 14:46). **None of this exists in `src/`.** For 27: folding/symbols/documentLink served against an open document will need `ContentResolver::text_of` (13/14 seam) plus a per-request `LineIndex`.

### A1.6 Diagnostics + code-action lane (ticket 25), design-only

No `publishDiagnostics`, no diagnostic registry, no code-action types anywhere in `src/` (no LSP code at all, see A1.9). Design commitments that touch 27:

- Protocol-free diagnostic type, plain fold functions, `ls-types` conversion only at the boundary (25:26); `source: "traces"` + hierarchical `<feature>.<rule>` codes + `enum DiagnosticRule` with `const ALL` (25:28, 25:40).
- **BrokenLink side-channel** (25:40): shared internal parse returns `(Note, Vec<BrokenLinkRef>)`; `parse_markdown` wraps and discards; sole production caller `src/index/service.rs:276` unaffected. This is the substrate a "convert link / create note" **code action** would read, currently even `Parser::new_with_broken_link_callback` is unused (research/25:705-713, 788-789; code has no `broken` hits in parser.rs).
- β single-lane publisher generalized to all diagnostics (25:25); T2 ~50ms publish lane.
- Create-note code action: zero-match unresolved wikilinks get Information diagnostic + "Create note" action (map.md:83 ticket 15; 27's input: offering side must check `create` in `resourceOperations`, a client capability read at `initialize`, no capability plumbing exists yet).
- 26 §9 (26:39): status-cycle code action invariant belongs to 23, **"27 owns plumbing"** (23:47); "steps over symbols vs kinds" is 27's open sub-question (23:47).

### A1.7 Template include/extends seam (ticket 22), `src/template/analysis.rs` does NOT exist

`ls src/template/` → `engine`, `error`, `loader`, `mod`, `path`, `service`, `writer`, **no `analysis.rs`** (22:33's design: shared engine-adjacent module at exactly that path, gated `unstable_machinery`, golden-span tests). What exists: `TemplateLoader::load` backs minijinja `{% include %}`/`{% extends %}` (`src/template/loader.rs:4`, `:19-23`, `:214-217`), with find/precedence logic at `:77`/`:85-99` (cited by 22 Q13/22:16) and dot-prefixed default handling (`:23`). Existence = disk-only today; 22 Q20(b) overlay-aware discovery (`Option<Arc<RwLock<…>>>` seam, CLI passes `None`) is design-only (22:58). **No include/extend reference spans exist anywhere: minijinja parses at render time only; there is no static AST or span extraction. Template rendering is interactive/unsafe for analysis (map.md:56).

### A1.8 Schema/Frontmatter model (tickets 19/20), outline-relevant fields absent

- `SchemaFieldDef { kind, required, multi }` only (`src/schema/fields.rs:53-57`), **no `origin`, no `description`** (20's design: `origin: Option<SchemaName>` + `description` for hover/goto-definition provenance, map.md:87; 26:45 consumes `SchemaFieldDef::origin`).
- `Schema` (`src/schema/model.rs:11`), **no `parent_order: Vec<SchemaName>`** (20 design).
- `Frontmatter` has **no `spans` map** (20's design: parallel `#[serde(skip)] spans: ByteSpan` map, map.md:87).
- `SchemaService` is load-once in-memory (map.md:55); no runtime field-value validator (schema validation never runs against Note content, so File-Class fields have **no per-field presence/validity signal** to hang on in an outline beyond the raw frontmatter map).
- Implication for "File-Class fields in an outline": today `Note::frontmatter().fields()` (`metadata.rs:121`) is the only field enumeration; no spans means no ranges; no schema attribution means no kind/decoration differentiation. All 20-model upgrades are design-only.

### A1.9 LSP/protocol/registration code, none

- `ls src/` → no `lsp` module; `rg tower_lsp|lsp_types|lsp-server` over `src/` + `Cargo.toml` → **zero hits**. `Cargo.toml` deps present: `pulldown-cmark 0.13.4`, `logos 0.16.1`, `noyalib 0.0.53` (`Cargo.toml:129-136`); **absent**: `ropey`, `tower-lsp-server`, `ls-types`, `unicode-normalization`, `fuzzy-matcher`, `camino` (all design-only new deps from 11/14/15/26).
- Config: **no `[lsp]` section** in `src/config/model.rs` (rg `lsp` → zero hits); ticket 31 (config integration) open; 25 defers project `[lsp.diagnostics]` to 31 (25:29).
- Perf harness: `benches/` contains `index_refresh.rs`, `note_parsing.rs`, `query_execution.rs`, etc., **no `lsp_latency.rs`** (33's design, map.md:82). `bench_query_tasks_density` (23's picker substrate) lives in the query benches family (23:50-52 references; density bench named in 23's Amends-33 block).

### A1.10 QueryService/IndexStore structures reusable by outline/picker

- `matched_file_rows`, linear scan over **every** `WorkspaceIndex` entry per query (`src/query/service.rs:319-330`), the reuse-vs-performance tension named at map.md:34.
- **Task rows exist**: `QueryMode::Tasks` → `tasks()` → `item_rows(index, source, |item| item.kind().is_task())` (`service.rs:210`, `:258-286`); `QueryRow` (`src/query/results.rs:44`). This is the row substrate 23's conditional workspace/symbol task picker builds on (23:49-55), residency + cap 128 + ranking assigned to 27/29 (23:51).
- `TaskStatusMap`/status config: `src/task.rs:119`+ (symbol→status config; cycle order invariant feeds 23's status-cycle code action, 23:47).

---

## (b) What does NOT exist yet (design-only, awaiting implementation after this planning effort)

| Designed in | Item | State in code |
|---|---|---|
| 26 (resolved) | Entire nav spine: `Occurrence`, `NavTarget`, `Resolution`, `AmbiguityKind`, `NavContext`, `locate/candidates/references_at/rename_at/highlight`, `EditSet`, comparison-key helper, `-32803` taxonomy | Glossary terms only (`src/GLOSSARY.md:70-115`); zero code |
| 11 (resolved, partial) | Spans on `Link`/`Tag`/inline-field entries; `byte_to_utf16_cu`; `convert` module (26 §7) | `Heading` node + `BytePos(u32)`/`ByteSpan`/`Spanned`/`LineIndex`-in-position.rs **do exist**; everything else absent |
| 13 | Single-file `RefreshPlan` constructor; batch constructor (26:33) | Only `collect(root)` (`refresh.rs:281`) |
| 14 | `BufferState`, overlay map, `ContentResolver`, inlink reconciliation | Nothing |
| 15 | Three-state link resolution over `candidates()`, heading-anchor multi-match, broken-link side-channel in parse, document-link ranges | `LinkResolver` is `Option`-returning nearest-only (`inlinks.rs:367-401`) |
| 16 | `paths_with_tag` facade (budgeted in 26:23); tag document symbols omitted by design; unique-tags workspace symbols | `unique_tags()` absent |
| 17 | Callout detection (blockquote first-line scan, 13 canonical types, `[callouts] extra_types` config); footnote inverted index (definitions+references HashMaps), `ENABLE_FOOTNOTES` already on | Callouts: zero code. Footnotes: option enabled (`parser.rs:70`), events discarded to scan buffer (`:252-255`) |
| 19/20 | Frontmatter `Spanned` extraction; `Frontmatter.spans`; `SchemaFieldDef.origin/description`; `Schema.parent_order`; `[schemas.diagnostics]` | None |
| 21 | `IndexStore::unique_tags()`, `unique_file_classes()` | None |
| 22 | `src/template/analysis.rs`, machinery-gated static parse with spans, overlay-aware `find`/`list_available` | Loader runtime seam only (`loader.rs:214-217`) |
| 23 | S3 outline (ancestor-stack task nesting, bool rollup), status-cycle code action, workspace/symbol task picker with cap 128 | Row substrate (`item_rows`) only |
| 25 | Diagnostic registry (`enum DiagnosticRule`), β publisher, code-action lane, BrokenLink wrap of `service.rs:276` | Nothing |
| 09/12 | `tower-lsp-server` transport, `ls-types`, `concurrency_level(1)`, cancellation | Nothing |
| 33 | `lsp_latency.rs` bench, inverted index for navigation | Nothing |

Also open: **ticket 41** (spanned-occurrence persistence, B-vs-C), `Spanned` cannot cross the redb boundary today (no serde derives; research/40:30); 11-Amended-by-26 says range-serving paths re-parse cold files via `ContentResolver::text_of` anyway (11:80), which 27's documentLink/CodeLens/semantic-token serving modes inherit.

---

## (c) Constraints / concerns / opportunities per ticket-27 decision bullet

### C1. Document symbols / outline

- **Exists**: flat `Heading { level, text, line }` (`heading.rs:10-14`), exactly the 11 §2 shape ("flat list with level, tree derived on demand", 11:37-39). `ListItem.depth`/`parent` + `clean_text()` (`lists.rs:173-175`, `:195-212`) support 23's S3 task outline (23:41-47). `Note::descendants()` gives subtree walks (`model.rs:202-211`).
- **Constraint**: headings/lists carry **line-only positions: `DocumentSymbol.range`/`selectionRange` need full LSP `Range`s (byte→UTF-16 conversion per 26 §7). Either 11's spans land first, or symbols ship line-anchored and convert at line granularity (degraded `character` values). Selection range of a symbol is the headline pain.
- **Constraint**: 23's S3 explicitly defers **mechanics to 27** ("registration, code-action plumbing, VS Code built-in-provider coexistence = 27/29"; "heading nesting → 27 blocked on 11's Heading", 23:46; Heading now exists). Tag symbols omitted (16, map.md:84); workspace symbols expose unique tags (needs `unique_tags()`, absent).
- **Opportunity**: hierarchy derivable O(n) via ancestor stack over the flat list (23:41); no parse-time tree needed. File-Class fields in the outline would need 19/20's frontmatter spans + schema attribution, clean optional add-on, currently unbuildable.
- **Coexistence**: rumdl ships its own heading outline with `enableSymbols` documented for ceding (map.md:57), Traces' document symbols need the same ownership-boundary treatment (32); VS Code's built-in Markdown outline is a second coexistence surface flagged by 23:46.
- **Workspace/symbol**: 23's conditional picker needs one shared cap + one `SymbolKind`/`containerName` policy across tags (16)/headings (27)/tasks (23), assigned to 27/29 (23:51); 16's PROPERTY-vs-STRING unresolved (23:51). Keep distinct from 26's 5,000 references cap (27 inputs: "keep the two consciously distinct").

### C2. Folding ranges

- **Key opportunity**: LSP `FoldingRange` is **line-based only** (`startLine`/`endLine`), the existing line-only positions may be *sufficient* for heading sections, list nesting, and frontmatter without waiting on byte spans. Heading sections: next heading of ≤ level ends the range, computable from the flat `headings()` list (`heading.rs:10-14`). List nesting: `depth`/`parent` already reconstructed (`lists.rs:4-7`). Frontmatter: block start/end lines not currently retained (buffer extent discarded, `parser.rs:322`, `:395-398`), needs one line-pair captured at `start_metadata_block`/`end_metadata_block`.
- **Constraint**: code fences are **not retained in the AST** at all (`BlockContext::CodeBlock` is transient state, `parser.rs:172`, `:279`), fence folding needs either a new retained node (11 didn't plan one) or a line-scan at serve time. Callout folding is gated on 17's detection (which is design-only) and 17 only committed to "completion + folding-range data for ticket 27" (map.md:85), i.e. 17 supplies the data, 27 the protocol shape.
- **Opportunity**: cheap to ship early (no encoding conversion), high client value (VS Code folds natively), zero rumdl overlap (rumdl's boundary is lint/format; map.md:57 lists no folding ownership, verify against ticket 03 digest during consolidation).

### C3. Selection ranges

- **Constraint**: `SelectionRange` requires full nested `Range`s through inline spans, the deepest hard case for 27. The AST retains **no inline spans** (links/tags/fields spanless; `handle_start_tag` forwards only `span.start()`, `parser.rs:247`); `into_offset_iter()` already computes per-event byte ranges that are discarded (`:106-114`), so a serve-time (or parse-time) span tree is available at the same cost as 11's spans.
- **Opportunity/concern**: selection ranges are pure-document-local math (no resolution, no index), zero dependency on 26's spine, but they want 11's spans or an on-demand re-scan of the request line. Microsoft's baseline includes selection ranges (map.md:67), in-scope per the generic bar. Consider deferring depth (client expands through what you return; a shallow start still works).

### C4. Document links (`textDocument/documentLink`)

- **Data exists but spanless**: every `Link` in `Note.outlinks` (`links.rs:28-34`) plus embeds (`embedded` flag) is a documentLink candidate; parser has the byte range per Link event and drops it (`parser.rs:271-278` gets `span.start()` only, the full `ByteSpan` was already computed at `:111`). Frontmatter-embedded wikilinks are first-class occurrences in 26 (A16, 26:23) but are **not extracted as outlinks today**.
- **Resolution state**: documentLink must emit for `Unique` **and** `Ambiguous`, never erroring (27 inputs), requires 26's three-state `candidates`/`classify`, not today's `Option` resolver (`inlinks.rs:369`). Unresolved links: decide whether they emit at all (a link with no target can't be clicked; likely emit only resolvable ones, matches "every **resolvable** link/embed target" in the Question).
- **rumdl coexistence, evidence gap**: ticket 03's research (tool-rumdl-boundary) **never mentions `documentLink`** (rg zero hits across research + issues). The Question's "check ticket 03 for whether it also registers documentLink" is currently unanswered, rumdl owns link *completion* + *navigation* (hover/def/refs/rename) with toggles, but its documentLink registration status is unverified in the local corpus. Consolidation should close this (rumdl digest/src digest exist: `docs/digests/lsp_rvben-rumdl*.txt`).
- **Position plumbing**: documentLink ranges are full `Range`s → needs the 26 §7 `convert` module + encoding negotiation; per-document `LineIndex` from `ContentResolver::text_of` (14 seam) for open buffers.

### C5. Template references as document links

- **Nothing to build on**: `src/template/analysis.rs` absent (22's Q20(b) seam, 22:58); no include/extends spans anywhere; loader resolves names at render time only (`loader.rs:214-217`). 22 derives existence via `find` + overlay seam (design-only).
- **Decision latitude**: 27's Question deliberately carries this as optional input with no `Blocked by: 22` edge (27:16; 22:70 confirms the flagged-line protocol and timing fallback). If 27 resolves template-links in scope, it gains an edge on 22; if it defers to go-to-definition (26's spine handles template identifiers as non-renamable v1, 26:39 "template/query identifiers non-renamable v1 (receipts into 21/22)"), no edge.
- **Constraint**: template name spans require 22's machinery-gated `machinery::parse` + span tracker (design-only, exact-pin `=2.24.0`); until then any template documentLink would be a whole-file range or line-range hack, not worth it.

### C6. CodeLens (reference-count above headings/notes)

- **Data reality check**: the only backlink data in code is **file-level** `InlinkMap` (`inlinks.rs:59`) / `FileEntry::inlinks()` (`entry.rs:167-170`), "3 references" per *note* is cheap; per-*heading* counts require heading-occurrence scanning (26 spine, unbuilt) plus 15 D2's anchor matcher (multi-match pre-dedup slugs, 26:25).
- **26's count semantics are already pinned for 27**: "the inlink graph keeps nearest-picked edges (backlink counts, … **27's counts**, … consume them)" (26:25), i.e. CodeLens counts read the nearest-picked graph, **not** the tie-reporting navigation union. Stem-tie er notes produce no inlink edge today (26:31), so counts are knowingly best-effort for ambiguous targets.
- **Cost question in the ticket** ("requires backlink computation eagerly for every visible heading") maps onto: CodeLens is pull-per-visible-range (`textDocument/codeLens` + optional `resolve`); counts for the visible file's headings = scan that file's occurrences × workspace inlinks for heading anchors. The 5,000-cap is references-path-only (27 inputs), CodeLens needs its own conscious budget decision.
- **Evidence**: MS's canonical service deliberately omits CodeLens (research/tool-ms-markdown-language-service.md:15, :41), one data point per ticket 27's caveat (27:10). Marksman/Markdown Oxide CodeLens status = external-research agent's lane.

### C7. Code actions (beyond quick-fixes elsewhere)

- **Existing substrate**: none in code. The designed lane is 25's registry + BrokenLink side-channel wrap of `src/index/service.rs:276` (25:40), the natural source of "link is unresolved" context for "convert link ↔ wikilink" / "extract selection to new note".
- **Shared predicate**: create-note offering must check `create` in client `resourceOperations` (27 inputs; 26:33 gates resource ops on `resourceOperations` alone), one predicate shared with 25's code-action path; capability plumbing comes with 29.
- **Ownership seams already drawn**: status-cycle code action = 23's invariant, **27 owns plumbing** (23:47, "steps over symbols vs kinds" open); create-note-from-template depth deferred until 22+28 (map.md:101); daily-note creation is diagnostic+action only (map.md:89). "Extract selection to new note" is new work touching 28 (file operations), no existing seam.
- **Constraint**: actions producing edits should follow 26's `EditSet` protocol-free shape (26:33) so the edge owns Uri/TextEdit/capability gating.

### C8. Semantic tokens

- **Nothing exists**; the three-valued modifier model (resolved/unresolved/ambiguous) rides 26's `Resolution` (27 inputs), i.e. semantic tokens are **structurally downstream of the nav spine**; no spine ⇒ no honest tokens.
- **Value case to test**: syntax highlighting already covers wikilink *syntax* via grammars (ticket premise, 27:19); the unique win is resolved-vs-unresolved-vs-ambiguous *state*, which is exactly what 25's diagnostics will already surface as diagnostics (unresolved = `links.wikilink.unresolved` Information/Warning; ambiguous = no diagnostic by 15's rule, 25 Amended-by-26 `:47`). Overlap risk: tokens duplicating what diagnostics already color.
- **Cost shape**: needs spans per link occurrence (same as documentLink), plus per-token resolution calls, potentially O(links × candidates) per visible range unless resolved states are cached per (uri, version) snapshot (24's per-(uri,version) content snapshot is the designed cache home, 26:37).
- **Evidence**: MS's service omits semantic tokens entirely (research/tool-ms-markdown-language-service.md:15); the ticket's caveat (27:10) forbids treating that as a ceiling, but it is the only generic-Markdown data point.

### Cross-cutting constraints for all eight

1. **Everything spans-adjacent waits on, or double-pays for, 11: document links, selection ranges, semantic tokens, symbol ranges, CodeLens ranges need byte spans + the 26 §7 `convert` module; folding ranges (line-based) can precede them.
2. **Nothing LSP-shaped exists**: tower-lsp-server/ls-types unpicked-into-code (09 resolved but not implemented), no session/capability/registration skeleton, every bullet above assumes the 09/12/14/25 lane exists; module home pending 34 (26:23, 25:26).
3. **Performance**: 33's budgets (inverted index for gotodef/find-refs now; `lsp_latency.rs` owed), documentLink/CodeLens/semantic tokens all multiply per-request resolution; the designed mitigation is 24's per-(uri,version) snapshot + 26's occurrence-resolved union patterns. `concurrency_level(1)` sequential dispatch (12) means a slow CodeLens pass blocks hover, budget consciously.
4. **Serving cold files**: 11-Amended-by-26's serving model (re-parse on demand via `ContentResolver::text_of`; resident spans when cached; 41-C acceptable, 11:80) applies to every 27 feature that reads ranges of closed files (CodeLens on scrolled-into-view files, workspace symbol ranges).
5. **Coexistence ownership**: rumdl's `enableSymbols`/`enableLinkCompletions`/`enableLinkNavigation` triad (map.md:57) covers outline + link nav; 27's documentLink and symbols both need explicit 32-boundary decisions; rumdl's documentLink status is an **open evidence gap** (C4).

---

## (d) Contradictions with the map's baseline facts

No substantive contradictions found. Two precision notes:

1. **map.md:52 says "only list items/tasks carry a line-only position" and calls out `Heading` as carrying a line: code agrees: `Heading { level, text, line }` (`heading.rs:10-14`), `ListItem { line, depth, parent }` (`lists.rs:41-50`); `Link`/`Tag`/inline fields/frontmatter spanless (`links.rs:28-34`, `model.rs:31-32`, `metadata.rs:96-97`). The ticket 11 Question's older grounding ("Headings are not retained in the AST at all today", 11:14) is **stale relative to current code: Heading nodes exist with line positions; ticket 27 can treat "headings as first-class AST nodes" (27:12's precondition) as **already satisfied at line granularity**, with byte spans still owed by 11.
2. **map.md:52 says `byte_to_utf16_cu` is among the plumbing `src/position.rs` "already supplies": it does **not** exist yet (`rg byte_to_utf16` → zero hits); 11 §4 designs it and 26 §7 supersedes it with a `convert` module while retaining it as a thin method (11:78). Treat the method as design-only; `LineIndex::line_at` + `BytePos`/`ByteSpan`/`Spanned` are the real existing plumbing (`position.rs:33-61`, `:79-138`, `:359-380`).

Everything else checked out: single package/binary (Cargo.toml, `src/main.rs`); fully synchronous (no async deps); `pulldown-cmark` event stream + logos lexer, single-pass (`parser.rs:106-114`, `lexer.rs:43-49`); `WorkspaceIndex` immutable snapshot, 7 redb tables, whole-tree mtime diff refresh, no watcher (`entry.rs:22-24`, `tables.rs`, `refresh.rs:281-299`); `QueryService` linear scan (`query/service.rs:319-330`); miette span-aware query errors (`src/query/error.rs`, per map.md:54); `SchemaService` load-once, no runtime validation (map.md:55); no template static analysis (map.md:56, confirmed, `analysis.rs` absent); rumdl ships LSP with the three coexistence toggles (map.md:57, research/tool-rumdl-boundary.md).

---

<a id="appendix-a2" />

## Appendix A2: A2 — Crate Research (ticket 27: structural symbols / editor intelligence)

## A2 — Crate Research (ticket 27: structural symbols / editor intelligence)

**Scope**: tower-lsp-server 0.23.0, ls-types 0.0.6, pulldown-cmark 0.13.4, ropey 1.6.1, supplementary candidates.
**Method note (deviation)**: The rust-docs MCP tool cache for pulldown-cmark 0.13.4 was left half-broken by a server bug (the `generate dependency metadata` step invokes `cargo metadata` with a relative manifest path that resolves against an unknown cwd, failing even when the file exists — see [Cache-bug appendix](#appendix-cache-tool-bug)). Per instruction, the already-cached local source under `.rust-docs/cache/crates/pulldown-cmark/0.13.4/source/` was read directly with Read/Grep instead. All other crates were cited via the rust-docs tools where available. No crate source was cloned or fetched; no production code or ticket/map file was modified.

---

## 1. tower-lsp-server 0.23.0 — `LanguageServer` trait signatures

All from `.rust-docs/cache/crates/tower-lsp-server/0.23.0/source/src/server.rs`. Every method is a default-impl returning `Err(Error::method_not_found())`, so implementing only what you need is fine.

| Method | RPC name | Signature | Line |
|---|---|---|---|
| `document_highlight` | `textDocument/documentHighlight` | `async fn document_highlight(&self, params: DocumentHighlightParams) -> Result<Option<Vec<DocumentHighlight>>>` | 568 |
| `document_link` | `textDocument/documentLink` | `async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>>` | 595 |
| `document_link_resolve` | `documentLink/resolve` | `async fn document_link_resolve(&self, params: DocumentLink) -> Result<DocumentLink>` | 609 |
| `code_lens` | `textDocument/codeLens` | `async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>>` | 634 |
| `code_lens_resolve` | `codeLens/resolve` | `async fn code_lens_resolve(&self, params: CodeLens) -> Result<CodeLens>` | 645 |
| `folding_range` | `textDocument/foldingRange` | `async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>>` | 660 |
| `selection_range` | `textDocument/selectionRange` | `async fn selection_range(&self, params: SelectionRangeParams) -> Result<Option<Vec<SelectionRange>>>` | 679 |
| `document_symbol` | `textDocument/documentSymbol` | `async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>>` | 701 |
| `semantic_tokens_full` | `textDocument/semanticTokens/full` | `async fn semantic_tokens_full(&self, params: SemanticTokensParams) -> Result<Option<SemanticTokensResult>>` | 726 |
| `semantic_tokens_full_delta` | `textDocument/semanticTokens/full/delta` | `async fn semantic_tokens_full_delta(&self, params: SemanticTokensDeltaParams) -> Result<Option<SemanticTokensFullDeltaResult>>` | 747 |
| `semantic_tokens_range` | `textDocument/semanticTokens/range` | `async fn semantic_tokens_range(&self, params: SemanticTokensRangeParams) -> Result<Option<SemanticTokensRangeResult>>` | 773 |
| `code_action` | `textDocument/codeAction` | `async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>>` | 1036 |
| `code_action_resolve` | `codeAction/resolve` | `async fn code_action_resolve(&self, params: CodeAction) -> Result<CodeAction>` | 1054 |

Notes from source docs:
- `document_symbol` returns either `DocumentSymbolResponse::Flat` (list, no hierarchy) or `DocumentSymbolResponse::Nested` (hierarchy) — server.rs:693–699.
- `DocumentLink::tooltip` requires client `document_link.tooltip_support` (LSP 3.15+) — server.rs:587–593.
- `code_action` supports lazy resolution: compute only the `edit` on `code_action_resolve` — server.rs:1030.
- Refresh push methods exist: `Client::code_lens_refresh` (client.rs:269), `Client::semantic_tokens_refresh` (client.rs:300).

### Dynamic registration
`Client::register_capability(&self, Vec<Registration>) -> jsonrpc::Result<()>` and `unregister_capability` — `.rust-docs/cache/crates/tower-lsp-server/0.23.0/source/src/service/client.rs:103,127`.

---

## 2. Version-drift finding

- **tower-lsp-server 0.23.0 depends on / re-exports `ls-types` 0.0.6, NOT `lsp-types`.** Confirmed:
  - `Cargo.toml:88–89`: `[dependencies.ls-types] version = "0.0"`
  - `Cargo.toml.orig:21,29`: `proposed = ["ls-types/proposed"]`, `ls-types = "0.0"`
  - `Cargo.lock:303,702`: package `ls-types`
  - `src/lib.rs:75` doc-comment still *says* "A re-export of [`lsp-types`](https://docs.rs/lsp-types) for convenience" — **stale doc**, the crate is `ls-types`. (Consolidation-pass naming risk; see §7.)
- **tower-lsp-server 0.24.0-rc.1 uses `gen-lsp-types`** instead (per changelog/manifest drift). The changelog (0.23.0 `CHANGELOG.md:23`) documents the switch from the unmaintained `gluon-lang/lsp-types` to the community fork `tower-lsp-community/ls-types`, with a planned codegen future (`CHANGELOG.md:27`).

---

## 3. ls-types 0.0.6 — registration & capability types

Cites are into `.rust-docs/cache/crates/ls-types/0.0.6/source/src/`.

### ServerCapabilities fields (lib.rs)
| Feature | Field (type) | Line |
|---|---|---|
| selectionRange | `selection_range_provider: Option<SelectionRangeProviderCapability>` | 1768 |
| documentHighlight | `document_highlight_provider: Option<OneOf<bool, DocumentHighlightOptions>>` | 1800 |
| documentSymbol | `document_symbol_provider: Option<OneOf<bool, DocumentSymbolOptions>>` | 1804 |
| codeAction | `code_action_provider: Option<CodeActionProviderCapability>` | 1812 |
| codeLens | `code_lens_provider: Option<CodeLensOptions>` | 1816 |
| documentLink | `document_link_provider: Option<DocumentLinkOptions>` | 1836 |
| foldingRange | `folding_range_provider: Option<FoldingRangeProviderCapability>` | 1844 |
| executeCommand | `execute_command_provider: Option<ExecuteCommandOptions>` | 1852 |
| semanticTokens | `semantic_tokens_provider: Option<SemanticTokensServerCapabilities>` | 1864 |

### `OneOf` (lib.rs:1678)
```rust
pub enum OneOf<A, B> { One(A), Two(B) }
```
Used to express "simple bool OR options object" for several providers.

### Registration option types
| Type | File:Line | Notes |
|---|---|---|
| `Registration` | lib.rs:1919 | `pub register_options: Option<Value>` at **lib.rs:1929** — **untyped `serde_json::Value`** (see §7) |
| `DocumentSymbolOptions` | lib.rs:2005 | `{ label: Option<String>, work_done_progress }` |
| `DocumentSymbolRegistrationOptions` | lib.rs:2019 | flattens `TextDocumentRegistrationOptions.document_selector` + options |
| `DocumentHighlightOptions` | lib.rs:2035 | `{ work_done_progress_options }` |
| `DocumentLinkOptions` | document_link.rs:22 | |
| `CodeLensOptions` | code_lens.rs:14 | |
| `CodeActionProviderCapability` | code_action.rs:12 | `Simple(bool) | Options(CodeActionOptions)` |
| `CodeActionOptions` | code_action.rs:352 | |
| `FoldingRangeProviderCapability` | folding_range.rs:21 | `Simple(bool) | FoldingProvider | Options` |
| `FoldingRangeCapability` | folding_range.rs:60 | client cap; `collapsed_text: Option<bool>` at folding_range.rs:66 (request field `Option<String>` at 154) |
| `SemanticTokensServerCapabilities` | semantic_tokens.rs:456 | full / full-delta / range / full+range variants |
| `SemanticTokensOptions` | semantic_tokens.rs:423 | |
| `SemanticTokensLegend` | semantic_tokens.rs:143 | `{ token_types, token_modifiers }` |

### SemanticTokenType / Modifier — custom-token support
`SemanticTokenType` wraps `Cow<'static, str>` (semantic_tokens.rs:16). Both `SemanticTokenType` and `SemanticTokenModifier` expose:
```rust
pub const fn new(tag: &'static str) -> Self
```
(semantic_tokens.rs:46 for type; :89, :118 for the modifier/other wrappers). Predefined consts are provided; custom strings are allowed and participate in the legend — useful for Note-specific token types (heading, tag, footnote-ref, callout-kind).

### Position encoding
- `PositionEncodingKind` — lib.rs:248, `pub const fn new(tag: &'static str)` at lib.rs:268; variants `UTF8`, `UTF16` (default), `UTF32`.
- `GeneralClientCapabilities.position_encodings: Option<Vec<PositionEncodingKind>>`.
- `Position { line: u32, character: u32 }` — lib.rs:186 `pub const fn new(line, character)`.

### SymbolKind
Consts `FILE = 1` … (full set present). See `lib.rs` `SymbolKind`.

### Per-feature client-capability structs
| Struct | File:Line |
|---|---|
| `CodeActionClientCapabilities` | code_action.rs:31 — fields: `dynamic_registration`, `code_action_literal_support`, `is_preferred_support`, `disabled_support`, `data_support`, `resolve_support: Option<CodeActionCapabilityResolveSupport>`, `honors_change_annotations` |
| `CodeActionCapabilityResolveSupport` | code_action.rs:87 — `{ properties: Vec<String> }` |
| `SemanticTokensClientCapabilities` | semantic_tokens.rs:338 |
| `SemanticTokensClientCapabilitiesRequests` | semantic_tokens.rs:397 |
| `TextDocumentClientCapabilities` (per-request sub-caps) | lib.rs — document_highlight:1306, document_symbol:1311, code_action:1342, code_lens:1346, document_link:1350, folding_range:1367 |

---

## 4. pulldown-cmark 0.13.4

### Crate-feature quirk
- Repo's Note parser uses `default-features = false`.
- Published crate **excludes `/specs/**` and `/third_party/**`**, so the `gen-tests` feature's `build.rs` panics at line 41 ("Could not find the 'specs' directory") — this is what broke the rust-docs all-features doc-gen. Relevant `Cargo` features for runtime parsing (`html`, `serde`) are unaffected.

### `Event` / `Tag` / `TagEnd` inventory (docs.json + source src/lib.rs)
```
Event:
  Start(Tag<'a>), End(TagEnd), Text(CowStr), Code, InlineMath, DisplayMath,
  Html, InlineHtml, FootnoteReference, SoftBreak, HardBreak, Rule,
  TaskListMarker(bool)

Tag:
  Paragraph
  Heading { level: HeadingLevel, id: Option<CowStr>, classes: Vec<CowStr>, attrs: Vec<(CowStr, Option<CowStr>)> }
  BlockQuote(Option<BlockQuoteKind>)
  CodeBlock(CodeBlockKind)
  HtmlBlock
  List(Option<u64>)          // Some(start) for ordered, None for bullet
  Item
  FootnoteDefinition(CowStr)
  DefinitionList / DefinitionListTitle / DefinitionListDefinition   // ENABLE_DEFINITION_LIST
  Table(Vec<Alignment>) / TableHead / TableRow / TableCell          // ENABLE_TABLES
  Emphasis, Strong, Strikethrough, Superscript, Subscript
  Link { link_type: LinkType, dest_url: CowStr, title: CowStr, id: CowStr }
  Image { link_type, dest_url, title, id }
  MetadataBlock(MetadataBlockKind)

TagEnd mirror:
  Heading(HeadingLevel), List(bool)  // bool = is_tight
  … same leaf names as Tag variants

HeadingLevel: H1..H6
CodeBlockKind: Indented | Fenced(CowStr)
LinkType: Inline, Reference, ReferenceUnknown, Collapsed, CollapsedUnknown,
          Shortcut, ShortcutUnknown, Autolink, Email, WikiLink { has_pothole: bool }
MetadataBlockKind: YamlStyle | PlusesStyle
BlockQuoteKind: Note | Tip | Important | Warning | Caution   (callouts — gated by ENABLE_GFM)
```

### `Options` — exact field names (src/lib.rs:646–736, bitflags `u32`)
| Flag | Bit | Gates |
|---|---|---|
| `ENABLE_TABLES` | 1<<1 | Table/… events |
| `ENABLE_FOOTNOTES` | 1<<2 | FootnoteReference, FootnoteDefinition (GFM syntax) |
| `ENABLE_STRIKETHROUGH` | 1<<3 | `~~x~~` |
| `ENABLE_TASKLISTS` | 1<<4 | `Event::TaskListMarker(bool)` |
| `ENABLE_SMART_PUNCTUATION` | 1<<5 | ligatures/smart quotes |
| `ENABLE_HEADING_ATTRIBUTES` | 1<<6 | Heading `id`/`classes`/`attrs` |
| `ENABLE_YAML_STYLE_METADATA_BLOCKS` | 1<<7 | `---` metadata |
| `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS` | 1<<8 | `+++` metadata |
| `ENABLE_OLD_FOOTNOTES` | (1<<9)|(1<<2) | old footnote syntax (implies ENABLE_FOOTNOTES) |
| `ENABLE_MATH` | 1<<10 | InlineMath/DisplayMath |
| `ENABLE_GFM` | 1<<11 | **BlockQuoteKind callouts ([!NOTE] etc.)** (lib.rs:721) |
| `ENABLE_DEFINITION_LIST` | 1<<12 | DefinitionList events |
| `ENABLE_SUPERSCRIPT` | 1<<13 | `^x^` |
| `ENABLE_SUBSCRIPT` | 1<<14 | `~x~` |
| `ENABLE_WIKILINKS` | 1<<15 | `LinkType::WikiLink` |

Internal helper: `Options::has_gfm_footnotes()` (lib.rs:740) = `ENABLE_FOOTNOTES && !ENABLE_OLD_FOOTNOTES`.

### Offset / span semantics (src/parse.rs)
- `Parser::into_offset_iter(self) -> OffsetIter<'input, F>` — parse.rs:1434.
- `OffsetIter` (parse.rs:2163) wraps the parser; `Iterator::Item = (Event<'a>, Range<usize>)` (parse.rs:2175). Ranges are **byte offsets** into the source string.
- Tight paragraphs emit no events (skipped at parse.rs:2181–2184), so `End` events still carry their span.
- `OffsetIter::reference_definitions(&self) -> &RefDefs<'_>` — parse.rs:2169; also `Parser::reference_definitions`.
- `OffsetIter` doc (parse.rs:2155–2161): "Generates tuples where the first element is the markdown event and the second is the corresponding range in the source string."

### Constructors (docs.json)
`Parser::new(&str)`, `new_ext(&str, Options)`, `new_with_broken_link_callback`, plus `into_offset_iter` and `reference_definitions`.

---

## 5. ropey 1.6.1 — byte/char/line/UTF-16 conversions

Cites into `.rust-docs/cache/crates/ropey/1.6.1/source/src/rope.rs`. All panicking variants have `try_` counterparts.

| Method | Signature | Line |
|---|---|---|
| `byte_to_char` | `(usize) -> usize` | 634 |
| `byte_to_line` | `(usize) -> usize` | 653 |
| `char_to_byte` | `(usize) -> usize` | 670 |
| `char_to_line` | `(usize) -> usize` | 689 |
| `char_to_utf16_cu` | `(usize) -> usize` | 706 |
| `utf16_cu_to_char` | `(usize) -> usize` | 727 |
| `line_to_byte` | `(usize) -> usize` | 745 |
| `line_to_char` | `(usize) -> usize` | 763 |
| `line` | `(usize) -> RopeSlice` | 821 |
| `try_char_to_line` | fallible variant | 1448 |

`char_to_utf16_cu` / `utf16_cu_to_char` cover LSP's default UTF-16 code-unit encoding (`PositionEncodingKind::UTF16`). Combined with `Position { line: u32, character: u32 }` (ls-types lib.rs:186), conversions are: `line` → `line_to_char` → + `utf16_cu_to_char`-family for `character`.

---

## 6. Supplementary crate candidates

| Crate | Cached | One-line API note |
|---|---|---|
| **logos 0.16.1** | yes (tools work) | `logos::Logos` derive + `Lexer`, `lexer_with_extras` — fast keyword/token lexer; candidate for a lightweight Note syntax highlighter. |
| **winnow 1.0.4** | yes | `winnow::parser::Parser`, `ModalParser`, `RecoverableParser` — composable byte-level parsers with error recovery. |
| **nom 8.0.0** | yes | classic combinator parser; heavier/error-prone than winnow for new code. |
| **tree-sitter 0.27.1** | yes | `tree_sitter::Parser` + `ParseOptions`; incremental re-parse via old tree (`Tree::edit`); gives full CST for structural symbols — **parse()/Tree::edit() signatures not yet extracted (unverified)**. |
| **text-size 1.1.1** | half-broken | `TextSize`/`TextRange` interop with rowan-style ranges; **tool queries fail** (cache-bug, see appendix) — treat signatures as unverified. |
| **indextree 4.9.2** | yes (repo dep) | arena-indexed tree (`Id`, `Node`) — candidate for Note document tree. |
| **comrak 0.56.0** | half-broken | CommonMark+GFM AST alternative to pulldown; **tool queries fail** (no `dependencies.json`) — unverified. |
| **lsp-textdocument 0.5.0** | half-broken | text-doc sync helper over lsp-types; **tool queries fail** — unverified. |

Anything marked unverified above should be confirmed against docs.rs or a fresh cache before use in the consolidation pass.

---

## 7. API awkwardness / risk flags

1. **Untyped dynamic registration** — `Registration.register_options: Option<Value>` (ls-types lib.rs:1929). There is no compile-time link between a method's registration options (e.g. `FoldingRangeCapability`) and the `Value` you serialize into `register_options`; a typo surfaces only at runtime on the client. Mitigation: centralize one `to_value` per feature.

2. **`ls-types` vs `lsp-types` naming risk** — tower-lsp-server 0.23.0's `lib.rs:75` doc still claims to re-export `lsp-types`, but the actual crate is `ls-types` (0.0.6). Any migration/consolidation code that greps for `lsp_types::` will miss, and `ls-types` is pre-1.0 (`0.0.x`) so breaking changes are expected. 0.24.0-rc.1 moves again to `gen-lsp-types` — pin the exact version before consolidating.

3. **Sync/async + resolve-pair pattern** — every lazy feature requires implementing both the list method and a `*_resolve` method (`code_action`/`code_action_resolve`, `code_lens`/`code_lens_resolve`, `document_link`/`document_link_resolve`). Omitting the resolve half yields `method_not_found` only when the client actually resolves.

4. **Missing hooks** — there is no server-side hook for "document symbols changed" push except the generic refresh (`code_lens_refresh`, `semantic_tokens_refresh`); documentSymbol has no refresh method. Symbol trees must be recomputed on change or cached by document version.

5. **Semantic tokens are three surfaces** — `full`, `full/delta`, `range` (server.rs:726/747/773). Implementing only `full` is valid but loses incremental-update bandwidth; `SemanticTokensLegend` must be negotiated via `SemanticTokensOptions` (semantic_tokens.rs:423).

6. **UTF-16 default** — unless the client advertises `position_encodings: [UTF8, …]` and the server picks a non-default, `character` is UTF-16 code units; ropey's `char_to_utf16_cu`/`utf16_cu_to_char` are the required bridges (rope.rs:706/727).

7. **pulldown byte-offset vs LSP line/char** — `OffsetIter` ranges are byte offsets (parse.rs:2175); converting to `Range { start: Position, end: Position }` requires ropey line/utf16 conversions for every event. Budget for this in the structural-symbol mapper.

---

## Appendix — cache tool bug (rust-docs-mcp)

**Symptom**: `cache_crate` and all tool queries for crates cached after ~Sep 13 (comrak 0.56.0, text-size 1.1.1, lsp-textdocument 0.5.0, pulldown-cmark 0.13.4 post-repair) fail with:
```
Failed to generate dependency metadata: error: manifest path
`.rust-docs/cache/crates/<crate>/<ver>/source/Cargo.toml` does not exist
```
even when the file exists on disk and `cargo metadata` succeeds from the shell.

**Root cause (identified, not fixed per instruction)**: the "generate dependency metadata" step in rust-docs-mcp invokes `cargo metadata` with a *relative* manifest path (`.rust-docs/cache/...`) that is resolved against a cwd that is not the repo root (server process cwd *is* repo root; the spawned cargo's cwd differs). All 6 rust-docs-mcp processes run with cwd = repo root; the failure is in the tool's own cargo invocation, not the cache contents. Crates cached Sep 13 (logos, tower-lsp-server, ropey, lsp-types) have a valid `dependencies.json` + `docs.json` and still work; later crates never completed the step.

**Mitigation applied**: for pulldown-cmark 0.13.4, the local source was read directly (Read/Grep) as instructed. A manual `dependencies.json` (`cargo metadata --format-version 1 --manifest-path <abs>`) and dummy `specs/` + `third_party/*` stubs were written to `.rust-docs/cache/crates/pulldown-cmark/0.13.4/` to attempt repair; these did not restore tool access (the tool re-runs the step regardless) but left the source tree readable. **No further time was spent on this.**

**Unverified items (flagged)**:
- pulldown-cmark `Options` — field names now confirmed from source (§4); the earlier docs.json `{"tuple":[null]}` stripping is moot.
- tree-sitter `Parser::parse` / `Tree::edit` exact signatures.
- text-size, comrak, lsp-textdocument — all tool-queries blocked; treat as unverified.

---

<a id="appendix-a3" />

## Appendix A3: Ticket 27, decomposition into researchable concerns (A3)

## Ticket 27, decomposition into researchable concerns (A3)

Produced: 2026-10-18. Step 3 of 6 (hub decomposition). PLANNING ONLY.
Scope: ticket 27's eight-bullet Question plus its cross-cutting concerns. Each concern below is a
*self-contained research question* for a downstream research subagent, deliberately unanswered here.

## Grounding & constraints carried into every cluster

- **Ticket 05 caveat**: `research/tool-ms-markdown-language-service.md` is the map's only researched
  generic-Markdown precedent. "MS has no semantic tokens/CodeLens" is one data point, not a ceiling.
  Each cluster should actively look for a *second* implementation (Marksman, Markdown Oxide, other
  editors' built-in Markdown tooling) before treating any MS omission as normative.
- **Target editors** (map Notes): VS Code + Neovim + **Zed (first-class)** first-class; **Helix is the
  sparse floor**. Any client-support claim must cover all four (ticket 24's sweep is the seed).
- **Inputs from 26 (ticket §"Inputs from 26")**: documentLink emits for `Unique` **and** `Ambiguous`
  targets (never errors on ambiguity); semantic-token modifiers are **three-valued**
  (resolved/unresolved/ambiguous) per 26's `Resolution` enum; spine exposes `highlight(target, doc)`
  for documentHighlight; the 5,000-reference ceiling is deliberately distinct from symbol ceilings;
  the create-note action must check `create` in `resourceOperations` (shared predicate with 25).
- **Optional input from 22**: include/extends spans as documentLink candidates + overlay-aware
  template-name existence (22's Q20(b)/Q24(b) flagged line; no `Blocked by` edge).
- **23's S3 outline**: 23 owns outline *content* (Array hideable-from-String headings, ancestor-stack
  nesting, bool rollup v1, `clean_text()` names, 1k/5k node caps, ContentResolver serving); **27 owns
  symbol mechanics/coexistence** (23's explicit deferral). 16: tags are omitted from document symbols,
  exposed as unique workspace symbols instead. `workspace/symbol` p95 row in 33 is proposed but
  **unfilled**; cap+ranking across tags/headings/tasks was assigned to 27/29 by 23.
- **Performance**: 33's latency table has a Document-symbols row (<10ms / <5ms) but **no rows** for
  folding, selection ranges, document links, CodeLens, semantic tokens, clusters must propose rows
  or explicitly mark "derive from existing rows".
- **Local spec limitation (verified)**: `docs/refs/lsp_spec.md` (899 lines) contains base protocol,
  lifecycle, capability narrative, and change log only, the per-feature sections are unfetched
  `{% include_relative %}` stubs. Researchers needing `documentLink`/`foldingRange`/`selectionRange`/
  `documentSymbol`/`semanticTokens`/`codeLens`/`codeAction` definitions must use the upstream LSP 3.18
  spec (primary source) and cite it as such.
- **rumdl facts already verified for 27** (digest greps by the hub, worth re-confirming in-cluster):
  rumdl registers `documentSymbol` (nested heading tree, `enableSymbols`, src digest :40368-40615);
  **zero** `textDocument/documentLink` provider and **zero** CodeLens code in rumdl's source digest;
  wikilinks permanently out of rumdl's scope; link diagnostics (MD057/MD051) not gated by LSP toggles.
- **Research output convention**: findings go to `.scratch/md-pkm-lsp/research/<cluster-slug>.md`
  (one file per cluster), citing sources per the `/research` skill.

---

## Concerns

### C1: Heading document-symbol shape (GENERAL)

- **Question**: How do existing Markdown LSPs shape `textDocument/documentSymbol` output for heading
  documents, hierarchical `DocumentSymbol` tree vs flat `SymbolInformation`, range vs selectionRange
  conventions for headings, `containerName`/`detail`/`kind` choices, fallback for clients without
  `hierarchicalDocumentSymbolSupport`, and the `workspace/symbol` counterpart, and what shape should
  Traces adopt given 23 already fixed the *content* rules?
- **Sources**: `docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt`
  (`documentSymbols.ts`, `includeLinkDefinitions` option); `lsp_artempyanykh-marksman-digest.txt`
  (:12524 `headingToDocumentSymbol`, :11149 VSCode-specific provider gating, :12109 hierarchical
  fallback); `lsp_feel-ix-343-markdown-oxide-src-digest.txt` (:2411, :3027 nested-tree mapping);
  `lsp_rvben-rumdl-src-digest.txt` (:40368-40615 symbol-tree builder); upstream LSP 3.18
  `documentSymbol` spec; `research/tool-ms-markdown-language-service.md`,
  `research/tool-marksman.md`; issues/23 §5 (S3 content rules), issue/16 (tags excluded).
- **Why it matters**: document/workspace symbols are a rumdl-ceded gap (`enableSymbols` documented
  for cession); 33 already budgets <10ms; mechanics are 27's explicit ownership.
- **Couplings**: C2 (what enters the tree), C12 (rumdl ownership), C14 (latency row), C15 (shared
  symbol cap with tags/tasks), ticket 29 (registration).

### C2: PKM-aware outline: tasks + File-Class fields (NOVEL)

- **Question**: Do other editors/LSPs ever surface *task rows* and *schema/File-Class field summaries*
  inside a Markdown document outline, how do they nest them (parent/child task depth via
  `ListItemPosition.parent`, tasks under headings), and what lessons apply to Traces' S3 outline
  (23's content rules: rollup vs counts, `clean_text()` names, 1k/5k node caps) without violating 16's
  "no tags in document symbols" and 33's residency assumptions?
- **Sources**: issues/23 §5-6 + `research/23-task-pkm-semantics.md` (outline content decision,
  `Amends 33` residency); issue/16; `docs/digests/obsidian_obsidian-tasks-*` (task-tree UI precedent),
  `vim_obsidian-nvim-digest.txt`, `vim_ymich9963-mdnotes-nvim-digest.txt`; VS Code Tasks tree /
  `tree-sitter` task views (web search); `rust-docs-mcp` not applicable; web: "task outline LSP
  documentSymbol".
- **Why it matters**: the flagship PKM differentiator vs rumdl/MS; failure mode is an outline that is
  either useless (flat, uncapped) or slow (recomputing tasks per request instead of resident rows).
- **Couplings**: C1 (tree mechanics), C14 (node caps/perf), C15 (shared cap/ranking across
  tags/headings/tasks assigned by 23 to 27/29), ticket 41 (span persistence feeding 26/27).

### C3: Folding ranges (GENERAL)

- **Question**: What folding-range sets do Markdown implementations actually emit (heading sections,
  list nesting, code fences, frontmatter/YAML block, blockquotes/callouts, link-reference blocks) and
  with which `FoldingRangeKind`s, and what does the evidence say about cost/quality tradeoffs
  (whole-section ranges, comment-flood capping, cancellation-token discipline)?
- **Sources**: `lsp_microsoft-vscode-markdown-languageservice-digest.txt` (`folding.ts:25-37` detail in
  `research/tool-ms-markdown-language-service.md`); Marksman digest (`:3190` foldingRange handler);
  rumdl src digest (does rumdl fold? likely not, verify); upstream LSP 3.18 `foldingRange`;
  issue/17 (callout detection → folding-range data for 27); issue/19 (frontmatter spans);
  VS Code built-in Markdown folding + `vscode-markdown` repo (web) as second data point.
- **Why it matters**: pure-syntax feature, no PKM semantics, cheapest of the eight bullets, but
  17 only promises *data*; 27 must decide the provider and the callout/frontmatter inclusion.
- **Couplings**: C4 (same structural walk), C12 (rumdl non-overlap), C14 (latency).

### C4: Selection ranges (GENERAL)

- **Question**: How do implementations compute nested selection expansion through Markdown structure
  (inline spans → paragraph → list item → heading section), which nodes do they treat as boundaries,
  and what does the LSP `selectionRange` parent-chain contract require, plus how do the four target
  editors exercise it (VS Code word/expand-selection, Neovim `vim.lsp` selection-range keymaps, Zed,
  Helix)?
- **Sources**: MS digest (`smartSelect.ts:41-55`); Marksman digest (`:3191` handler, `SelectionRange`
  field usage at :12518-12577); upstream LSP 3.18 `selectionRange`; `research/tool-ms-…`;
  client docs for the four editors (web / Neovim `:h lsp`, Zed docs, Helix book).
- **Why it matters**: rarely used but trivially cheap once the structural AST from 11 exists; deciding
  *not* to ship it must be evidence-based, not assumed.
- **Couplings**: C3 (shared structure walk), C11 (client exercise), C14 (latency row).

### C5: Document links, including rumdl overlap (GENERAL)

- **Question**: Which Markdown LSPs register `textDocument/documentLink` and exactly what do they emit
  (which link kinds, inline, reference-definition, images, anchors, tooltips, `target` URIs,
  `resolve`-on-demand vs eager), how do clients render them, and, given rumdl ships **no**
  documentLink provider (digest-verified) while owning standard-link completion/navigation, what is
  the non-overlapping emission set for Traces (wikilinks/embeds per 15 + cross-file `file.md#heading`
  per 25; standard links owned by rumdl's completion/navigation) and the correct coexistence wording?
- **Sources**: rumdl src/docs digests (re-verify absence of documentLink registration; the
  `DocumentLinks` type at :85967 is MD057 lint-internal, not a provider, do not confuse);
  `lsp_artempyanykh-marksman-digest.txt:3175`; MS digest (documentLinks feature in
  `research/tool-ms-…` "Document Links" list); `research/tool-rumdl-boundary.md`; issue/15 (link
  model, ranges), issue/25 (ownership table), issue/03; upstream LSP 3.18 `documentLink` (incl.
  `resolve` support and `documentLink/resolve` capability); web: "LSP documentLink markdown".
- **Why it matters**: ticket 15 fixed *ranges* but 27 owns provider mechanics + the rumdl toggle
  story; mis-emitting standard links duplicates rumdl's UX and breaks the clean-split product goal.
- **Couplings**: C6 (template refs as links), C10 (same resolution state data), C12 (toggle mapping),
  26's predicate (emit for `Unique` + `Ambiguous`, never error).

### C6: Template references as document links vs go-to-definition (NOVEL)

- **Question**: How do template-aware language servers (Jinja/Liquid/Handlebars/Hugo/Nunjucks LSPs,
  if any exist) expose `include`/`extends`/`import` targets, as document links, go-to-definition,
  or both, and what should Traces do with 22's derived include/extends spans + overlay-aware
  existence check: emit `documentLink` ranges (with existence reflected how?), route to
  go-to-definition, or split the two? What client-visible signal covers a missing/overlay-only
  template name?
- **Sources**: issue/22 (Q20(b)/Q24(b) flagged line, `find`+`list_available` seam,
  `research/22-template-intelligence.md`, include/extends existence & overlay discovery); upstream
  `documentLink` spec (tooltip/target semantics); web searches: "Jinja language server goto
  definition include", "Hugo template LSP", "handlebars LSP partials", "template include go-to
  definition LSP"; `minjinja` machinery docs (local `docs/refs/minjinja_syntax.md`) for include
  syntax shapes only.
- **Why it matters**: explicitly scoped into 27 by 22's flagged line (no blocking edge); the
  documentLink-vs-definition split is a genuine product decision no PKM tool has settled; overlay
  (unsaved-buffer) awareness per standing constraint #1.
- **Couplings**: C5 (mechanics), C9 (missing-template creation actions later), ticket 22 (data
  source), 26's definition/refusal matrix.

### C7: CodeLens reference counts above headings/notes (NOVEL)

- **Question**: Which LSPs ship reference-count CodeLens (rust-analyzer, TypeScript/Java servers are
  the known mechanism precedents), how do they compute and invalidate counts cheaply, lazily, and
  under which client capabilities (`codeLens.refreshSupport`), do **any** Markdown/PKM servers do it
  (verify absence in Marksman/Markdown Oxide/MS/rumdl digests, currently zero evidence of presence),
  and does the value justify eagerly computing backlink counts for every visible heading under 33's
  budgets?
- **Sources**: issues/01, `research/tool-markdown-oxide.md`, `research/tool-marksman.md`,
  `research/tool-ms-…` (absence claims, re-verify), rumdl src digest (absence, verified);
  issue/26 (references machinery: occurrence-resolved union, 5,000 cap, **not** reusable per-lens);
  issue/33 (backlink recomputation <5ms, inlink multimap, visibility/lazy evaluation patterns);
  upstream LSP 3.18 `codeLens` (+ `codeLens/resolve`); web/source: rust-analyzer `codelens.rs`
  (references lens), VS Code TS server codelens implementation, `workspace/codeLens/refresh`
  usage in the wild.
- **Why it matters**: ticket 05 flags CodeLens as deliberately absent in the one generic precedent,
  either a real lesson about cost/benefit for prose documents, or a gap Traces can fill better;
  the answer is the ticket's explicit "value vs cost" decision.
- **Couplings**: C1 (lens attach points = outline nodes), C14 (perf: eager backlinks), C11 (client
  support: Zed/Helix CodeLens rendering must be checked), 26 (count source), ticket 29 (registration).

### C8: Code action: Markdown-link ↔ wikilink conversion (NOVEL)

- **Question**: Do any LSPs/editors offer link-format conversion as a code action (MS markdown's
  link-organization/extraction actions, Obsidian plugins, "convert to wikilink" extensions), what
   exact edit shape do they use (whole-target rewrite vs display-preserving), and how should Traces
  compose it with 26's ownership rules (display-text positions refuse; url/fragment rewrite only at
  reference/definition sites) and 25's pull-code-action transport (`data` payload passthrough)?
- **Sources**: MS digest (`languageFeatures` code actions, link definition organization/extraction);
  issue/25 (code-action transport, `identifier: "traces"`, pull actions), issue/26 (rename/edit
  boundaries, `EditSet`), issue/28 (file ops boundaries), upstream LSP 3.18 `codeAction`
  (+ `resolve`); `obsidian_*` digests (any convert action?), treat as ~3 months stale per map caveat;
  web: "convert markdown link to wikilink", VS Code extension marketplace, Obsidian plugins.
- **Why it matters**: pure PKM/interop affordance with real daily-use value (migrating vaults);
  correctness hinges on 26's refusal matrix, not on the edit mechanics.
- **Couplings**: C9 (same provider/registration), C5 (link kinds it may touch), 25/26/28 (machinery),
  26's `resourceOperations` predicate where creation results.

### C9: Code action: extract selection to new note (NOVEL)

- **Question**: What precedent exists for "extract selection to new file/note" refactoring actions
  (rust-analyzer/TS/Python extract-method and move-to-file refactorings; Obsidian/PKM plugins;
  VS Code's "extract to" scope handling), how do they shape the resulting `WorkspaceEdit`
  (insert-link-at-selection + create-file-in-one transaction, undo semantics, naming/slug
  generation), and how must Traces gate them (`create` in `resourceOperations`, 26's shared
  predicate with 25; `isRefactoring` metadata; destination-path choice UX)?
- **Sources**: issue/26 (`create` predicate, `EditSet`, serializer tiers), issue/25 (action
  transport), issue/28 (note creation/move/delete ownership, the file-op boundary), issue/15
  (wikilink insertion shape), issue/18 (template/creation precedents), issue/22 (from-template
  variant is deferred pending 22+28, confirm it stays deferred); upstream LSP 3.18 `codeAction`
  + `WorkspaceEdit.createFile`; web/source: rust-analyzer `move_file`/extract refactors, TS server
  "move to new file", VS Code refactoring docs.
- **Why it matters**: highest-risk action of the set (it writes files); the gating predicate is
  already mandated by 26, but edit composition, naming, and undo story are undecided.
- **Couplings**: C8 (same provider), ticket 28 (create/rename cascade ownership), 26/25
  (transport + predicates), ticket 29 (capability advertisement).

### C10: Semantic tokens for wikilink resolution state (NOVEL)

- **Question**: Has any LSP encoded *resolution state* (not just syntax) into semantic tokens, e.g.
  clangd's inactive regions, unresolved-reference coloring in Java/C++ servers, `unresolved`
  token-type experiments, and given 26 mandates three-valued modifiers (resolved / unresolved /
  ambiguous), (a) is the marginal value over TextMate/tree-sitter highlighting real for prose
  Markdown, (b) which legend design (token type vs modifier vs both) survives across VS Code,
  Neovim (virtual styles), Zed, Helix (does Helix even apply semantic tokens?), and (c) what refresh
  strategy (full vs range vs delta tokens) fits Traces' sync core?
- **Sources**: upstream LSP 3.18 `semanticTokens` (legend, `range`/`full/delta`, refresh, local
  spec file lacks this section, use upstream); VS Code "Semantic Highlighting" docs + Markdown
  TextMate grammar / Neovim treesitter-markdown / Zed markdown grammar (web) as the syntax baseline
  the ticket says "likely covers adequately"; issue/26 (`Resolution` enum, `highlight(target, doc)`
  spine); `research/tool-ms-…` (deliberate absence, one data point only, per 05 caveat);
  issue/24's client sweep; web: "semantic tokens unresolved reference", "LSP semantic tokens
  markdown", rust-analyzer/clangd semantic-token source.
- **Why it matters**: the ticket's one capability that would *uniquely* enable something syntax
  highlighting cannot do, deciding in-scope vs out hinges on cross-editor feasibility + genuine
  value; three-valued state is mandated by 26 if tokens ship at all.
- **Couplings**: C5 (same `Resolution` data), C11 (client matrix, likely the deciding factor),
  C13 (registration), 29 (capability advertisement), 14 (live-buffer freshness of token state).

### C11: Client capability/support matrix for the eight features (GENERAL)

- **Question**: For each of documentSymbol, foldingRange, selectionRange, documentLink, codeLens,
  codeAction, semanticTokens (and workspace/symbol), which of VS Code, Neovim (core + nvim-cmp
  ecosystem), **Zed**, and **Helix** actually render/support it, under which `ClientCapabilities`
  fields, with which known bugs/quirks, and what is the sparse-client floor behavior for each?
- **Sources**: `research/24-completion-architecture.md` §11 (verified client sweep, extend its
  method to these features); Zed docs/source (zed.dev docs, `zed` repo LSP feature support, first
  class per map Notes, previously unverified areas must be closed); Neovim `:h lsp` + nvim-lspconfig
  docs; Helix book `lsp.md` (feature matrix); VS Code language-client docs; upstream LSP 3.18
  `ClientCapabilities` per feature; web searches per editor for each feature.
- **Why it matters**: converts eight in/out/depth decisions into evidence, e.g. CodeLens in Helix or
  semantic tokens in Zed may be absent, changing cost/benefit; also seeds 29's degradation matrix.
- **Couplings**: every concern (its in/out/depth verdict), C13 (static vs dynamic follows
  capabilities), ticket 29 (owns the negotiation policy).

### C12: rumdl coexistence mapping for this ticket's features (GENERAL)

- **Question**: Enumerating from rumdl's *source and docs* (not assumptions): which LSP providers does
  rumdl register for each of the eight features (documentSymbol: yes, `enableSymbols`;
  documentLink: none, re-verify; CodeLens: none; folding/selection/codeAction/semanticTokens:
  determine), which are toggle-gated vs always-on, and what ownership table + recommended
  `init_options` companion settings follow for Traces' joint-setup documentation (feeding ticket 32)?
- **Sources**: `docs/digests/lsp_rvben-rumdl-src-digest.txt` (registration surface, grep provider
  impls; symbol tree :40368), `lsp_rvben-rumdl-docs-digest.txt` (`docs/lsp.md` 1885-2158 settings
  table), `research/tool-rumdl-boundary.md`, issue/03, issue/25 (post-cession ownership table),
  issue/32 (consumer of this output), issue/15 (joint-setup block).
- **Why it matters**: ticket 32 is blocked on 27; this cluster's mapping is the primary input.
  Accidentally duplicating rumdl's outline or emitting standard-link duplicates would break the
  product's clean split.
- **Couplings**: C1, C5 (direct overlap candidates), C14 (rumdl perf characteristics as a yardstick),
  ticket 32 (downstream), ticket 25 (diagnostic cession already settled).

### C13: Static vs dynamic capability registration for these providers (GENERAL)

- **Question**: What do mature servers (rust-analyzer, MS markdown-language-server, Marksman, rumdl)
  advertise statically at `initialize` vs register dynamically via `client/registerCapability`, when
  does dynamic registration actually buy anything for the eight features (e.g. toggling symbols with
  rumdl settings, adapting to sparse clients), and what should Traces' default be given ticket 29
  owns the overall negotiation policy?
- **Sources**: upstream LSP 3.18 (`client/registerCapability`, per-feature registration options;
  local `docs/refs/lsp_spec.md:896` has the change-log narrative only); Marksman digest (:3175-3191
  dispatch table, :11149 VSCode-specific gating), rumdl src digest, MS `server.ts` (from
  `research/tool-ms-…`), issue/29 (question + 26's consumed capability fields), issue/12
  (`concurrency_level(1)`, dynamic registration is a notification, ordering-safe).
- **Why it matters**: determines whether features can be enabled/disabled at runtime (directly
  relevant to rumdl joint setups) and what the sparse-client floor looks like per feature.
- **Couplings**: C11 (client capabilities drive it), C12 (rumdl toggling scenario), ticket 29
  (policy home), C10/C7 (the two most likely dynamic-only candidates).

### C14: Performance budgets & invalidation for the new providers (GENERAL)

- **Question**: What latency/invalidation discipline do peer LSPs apply to documentSymbol, folding,
  selection, documentLink, CodeLens, and semantic tokens (on-demand vs precomputed, cancellation
  tokens, CodeLens/semantic-token *refresh* notifications, range/delta token requests), and what
  should 33's table gain: rows (or explicit "derive from Document symbols <10ms") for each of these
  features, plus filling the **unfilled `workspace/symbol` p95 row** (23's condition) and honoring
  23's 1k/5k node caps and the deliberate separation of 26's 5,000-reference ceiling from symbol
  ceilings?
- **Sources**: issue/33 (latency table §3, `Amends 33` from 23, memory tiers), `research/33-…`,
  `research/x-lsp-performance-best-practices.md`, `research/x-rust-lsp-implementation-patterns.md`;
  MS digest (cancellation-token patterns), rumdl benches digest (rumdl's own perf yardstick),
  issue/23 (task-row residency, `bench_query_tasks_density` numbers for the unfilled row), issue/26
  (reference-path ceiling), issue/12 (cancellation = stale-result discard; on-demand computation
  must be cheap because there is no mid-request interruption).
- **Why it matters**: standing constraint #5, performance is first-class; several features
  (CodeLens backlinks, workspace/symbol refill, semantic tokens over large docs) are exactly the
  "reuse vs performance" tension named at map:34.
- **Couplings**: C7 (eager backlink cost, the make-or-break input), C1 (symbol caps), C15 (ceiling
  discipline), issue 41 (span persistence mode affects store-assembled serving).

### C15: Shared predicates & ceilings across providers (GENERAL)

- **Question**: 26's inputs mandate shared predicates (documentLink emits on `Unique`+`Ambiguous`;
  three-valued token modifiers; `create` resourceOperations gate; reference ceiling ≠ symbol
  ceilings). How do mature LSPs (rust-analyzer, clangd, ty) structure such cross-provider invariants
 , one facade/enum consumed by every protocol handler vs per-handler logic, so that
  documentLink/documentHighlight/semanticTokens/documentSymbol can't drift apart, and what is the
  minimal Traces-side expression of that discipline at the protocol boundary (standing constraint #3:
  DTOs never define core semantics)?
- **Sources**: issue/26 (`Research Parts VII-X`, `Occurrence`/`NavTarget`/`Resolution` spine,
  consumer matrix), issue/12 (sequential dispatch, one renderer feeding), 26's Inputs blocks in
  issues/25/28/29/32/41; `research/x-rust-lsp-implementation-patterns.md`,
  `research/07-rust-lsp-framework-crates.md`; rust-analyzer source via web/repo (facade patterns).
- **Why it matters**: the eight features all consume 26's resolution state; without an explicit
  shared-predicate map, each provider re-derives ambiguity handling and the two ceiling classes get
  silently unified.
- **Couplings**: C5, C6, C10 (direct consumers), C14 (ceilings), C13 (where predicates gate
  advertisement), ticket 41 (span-serving mode).

---

## Research clusters (7)

Each cluster = one subagent session (~100K tokens), clean boundary, findings to
`.scratch/md-pkm-lsp/research/<slug>.md`.

### Cluster 1: Outline, heading symbols + PKM outline
**Members**: C1, C2.
**Sources**: MS digest `documentSymbols.ts`; Marksman digest :11149/:12109/:12524-12583;
Markdown Oxide src digest :2411/:3027; rumdl src digest :40368-40615; upstream LSP 3.18
`documentSymbol`/`workspace/symbol`; issues 23 §5-6, 16, 33 §3; `research/23-…`, `research/tool-*`;
obsidian-tasks + obsidian-nvim digests; web (task outline precedents, `hierarchicalDocumentSymbolSupport`).

### Cluster 2: Folding & selection ranges
**Members**: C3, C4.
**Sources**: MS digest `folding.ts`/`smartSelect.ts` (+ `research/tool-ms-…` details); Marksman
digest :3190/:3191, :12518; rumdl src digest (verify absence); upstream LSP 3.18 `foldingRange`/
`selectionRange`; issue/17 (callout folding data), issue/19 (frontmatter spans); VS Code built-in
Markdown folding (web); four-editor expand-selection behavior (web).

### Cluster 3: Document links, rumdl overlap, template references
**Members**: C5, C6, C12.
**Sources**: rumdl src/docs digests (provider-surface grep; settings table :2118-2158),
`research/tool-rumdl-boundary.md`, issues 03/15/25/32; Marksman digest :3175; MS digest document
links; upstream LSP 3.18 `documentLink`/`documentLink/resolve`; issues 22 (Q20(b)/Q24(b)) +
`research/22-…`; web: Jinja/Liquid/Handlebars/Hugo template LSPs; 26's documentLink predicate.

### Cluster 4: CodeLens reference counts (value vs cost)
**Members**: C7.
**Sources**: absence checks in issues/01/02 + `research/tool-markdown-oxide.md`,
`research/tool-marksman.md`, `research/tool-ms-…`, rumdl src digest (verified absent); rust-analyzer
`codelens.rs` + VS Code TS-server codelens (web/source); upstream LSP 3.18 `codeLens` +
`workspace/codeLens/refresh`; issue/26 (references machinery + 5,000 cap), issue/33 (backlink <5ms,
residency), issue/12 (no mid-request cancellation → lazy/on-demand necessity); four-editor CodeLens
support (feeds Cluster 7).

### Cluster 5: Code actions: link conversion + extract-to-note
**Members**: C8, C9.
**Sources**: MS digest code actions (link organization/extraction); upstream LSP 3.18 `codeAction`
(+ `resolve`, `createFile`); issues 25 (pull-action transport, `data`), 26 (`EditSet`,
`resourceOperations`/`create` predicate, `isRefactoring`, serializer tiers), 28 (file-op ownership),
15 (wikilink insertion), 18/22 (creation/template precedents, confirm deferred variant);
rust-analyzer/TS move-to-file & extract refactors (web); Obsidian/VS Code conversion plugins (web,
treat Obsidian digests as ~3mo stale).

### Cluster 6: Semantic tokens for wikilink resolution state
**Members**: C10.
**Sources**: upstream LSP 3.18 `semanticTokens` (local spec file lacks it, cite upstream); VS Code
Semantic Highlighting docs; TextMate markdown grammar + Neovim treesitter-markdown + Zed markdown
grammar (syntax baseline, web); clangd inactive-regions / unresolved-reference precedents (web);
issue/26 (`Resolution`, three-valued modifiers, `highlight(target, doc)`); `research/tool-ms-…`
(absence, one data point per 05 caveat); issue/24 client sweep + Zed/Helix semantic-token support
(cross-check Cluster 7).

### Cluster 7: Client matrix, registration strategy, shared predicates, perf budgets
**Members**: C11, C13, C14, C15.
**Sources**: `research/24-completion-architecture.md` §11 (method to extend); Zed/Neovim/Helix/VS
Code official docs (per-feature support, Zed first-class, Helix floor); upstream LSP 3.18
(`ClientCapabilities` per feature, `client/registerCapability`, per-feature registration options);
local `docs/refs/lsp_spec.md:896` (change-log narrative); Marksman digest :3175-3191/:11149, MS
`server.ts`, rumdl src (what peers register statically); issues 29, 12, 33 §3 (table + unfilled
`workspace/symbol` row), 23 (`Amends 33`, 1k/5k caps, density-bench numbers), 26 (spine + ceiling
discipline + Inputs blocks); `research/x-lsp-performance-best-practices.md`,
`research/x-rust-lsp-implementation-patterns.md`, `research/07-…`.

---

<a id="appendix-b1" />

## Appendix B1: B1 Research: Outline & document symbols (Cluster 1, ticket 27)

## B1 Research: Outline & document symbols (Cluster 1, ticket 27)

Scope: ticket 27 Cluster 1, C1 (heading document-symbol shape, GENERAL) and C2 (PKM-aware outline: tasks + File-Class fields, NOVEL). Research only; no decisions made. Per `A3-decomposition.md` Grounding & constraints, ticket 23 owns outline *content* rules (Array hideable-from-String headings, ancestor-stack nesting, bool rollup v1, `clean_text()`); ticket 27 owns the *mechanics* (protocol shape, negotiation, caps, residency hooks). Ticket 05's caveat applies throughout: MS markdown-ls is the only generic precedent, so every "nobody does X" claim below was checked against additional implementations (Marksman, Markdown Oxide, rumdl) and PKM-adjacent servers found via web search.

Primary sources: LSP 3.18 spec (fetched microsoft.github.io; local `docs/refs/lsp_spec.md` is unfetched `{% include_relative %}` stubs, `documentSymbol` at `docs/refs/lsp_spec.md:654`, `workspaceSymbol` at `:675`, so spec quotes cite the live page, saved at `~/.../tool-output/tool_14e84a257001b9sLtUYS1ygZg3`); repo digests under `docs/digests/`; ticket files under `.scratch/md-pkm-lsp/`; web sources cited inline.

---

## C1, Heading document-symbol shape (GENERAL)

### 1. The spec rules that actually bind us

LSP 3.18 (live spec page, `textDocument/documentSymbol` + `workspace/symbol` sections):

- "Servers should whenever possible return `DocumentSymbol`"; `SymbolInformation` is deprecated ("this type is deprecated... use `DocumentSymbol`"). Flat `SymbolInformation[]` remains the required fallback response type when the client doesn't declare `hierarchicalDocumentSymbolSupport`.
- Negotiation: `DocumentSymbolClientCapabilities.hierarchicalDocumentSymbolSupport` (added 3.10, PR #538; KamasamaK on microsoft/language-server-protocol#1215: "If `hierarchicalDocumentSymbolSupport` is not `true`, then the server **must** provide `SymbolInformation`. If it is `true`, the server should prefer `DocumentSymbol`.").
- Structural rules: `DocumentSymbol.name` must be non-empty and not all-whitespace; **`selectionRange` must be contained in `range`** (the only hard nesting invariant at the symbol level); children's `range`s should be within the parent's `range` (VS Code enforces, see §5).
- `symbolKind.valueSet`: if absent, "the client only supports the standard kinds `File`..`Array`" (1-18). Unknown kinds → clients must degrade gracefully, but a floor-safe kind never triggers the question.
- `DocumentSymbolClientCapabilities.labelSupport` / `DocumentSymbolOptions.label` (3.16): allows multiple named outline trees per document ("Outline", "Timeline", ...). Servers not using it are unaffected; worth knowing because a second provider appearing in the same tree is a coexistence event, not an error.
- `workspace/symbol`: relaxed matching, clients "shouldn't use prefix, substring, or other constraints"; empty query may return everything. `WorkspaceSymbol.resolve` (3.17, `workspaceSymbol/resolve`) lets servers send URI-only results and resolve location lazily; `partialResultToken` supported. `WorkspaceSymbol` (name + kind + location) is recommended over `SymbolInformation` (which adds deprecated `containerName`, but note VS Code still renders `containerName` as the grey path suffix, so "deprecated" ≠ "unused"; see rumdl/Marksman both populating it).

### 2. Hierarchical vs flat: what each markdown server does

| Server | Nested (hierarchical) | Flat fallback | Negotiation |
|---|---|---|---|
| MS vscode-markdown-languageservice | `DocumentSymbol[]` only, `provideDocumentSymbols` always returns nested (`docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt:4632`) | **Unverified**: adapter (`vscode-markdown-languageserver/src/server.ts`) is not in the digest; the library API has no flat mode | Unverified at adapter level; VS Code sets the flag true, so untested in practice |
| Marksman | Nested with full tree; also emits **tags as symbol children** (digest `:12480-12600`, `:12524 headingToDocumentSymbol`) | Flat `SymbolInformation` with `containerName` = parent heading, names like `H1: E`, `Tag: t1` (`:11149`, `:12106-12109`, tests `:17599-17640`) | **Gated on client identity, not just capability**: `DocumentSymbolProvider = Some(First <| not clientDesc.IsVSCode)`, Marksman *cedes* document symbols to VS Code's built-in Markdown provider; both providers advertised otherwise (digest `:11756`) |
| Markdown Oxide | Always `Nested` (`SymbolKind::STRUCT`, digest `:2960-3110`) | **None**, ignores the client capability entirely | Ignored (pragmatic; works because real clients either accept nested or flatten) |
| rumdl | Nested; range = whole section, selectionRange = heading **name span**; flat fallback `document_symbols_flat` with `containerName` = nearest enclosing heading (`docs/digests/lsp_rvben-rumdl-src-digest.txt:40363-40930`) | Yes, `SymbolInformation[]` | Negotiated from `hierarchicalDocumentSymbolSupport`; both providers in `initialize` (commit 2cef95b message; source `src/lsp/server.rs` not in digest) |

Traces position (issue 16, `.scratch/md-pkm-lsp/issues/16-tags-and-hierarchical-tags.md:87-95`): tags excluded from documentSymbol ("noisy trees"), exposed only as workspace symbols (`Tag: #{name}`, PROPERTY/STRING, deduped, containerName `"{N} notes"`). This *diverges* from Marksman (tags in-tree) but matches rumdl/MOX (headings only). Ceding to rumdl's own outline (`tool-rumdl-boundary.md:64` + rumdl's `enableSymbols`, default true, documented "Set to `false` to avoid duplicate headings when another LSP provides the outline", negotiated at server start) is a separate coexistence question, see §6.

### 3. Range / selectionRange conventions (the thing clients actually break on)

| Server | `range` | `selectionRange` |
|---|---|---|
| MS | Whole section (= `sectionLocation.range`) | **Equal to `range`** (digest `:4590-4730`) |
| Marksman | Whole section (`h.data.scope`) | = `h.range` (heading line) (digest `:12524`, `:11756`) |
| Markdown Oxide | = `heading.range` (heading line only, **no section scope**) | Equal to `range` (digest `:3090`) |
| rumdl | Whole section | Heading **name** span only (excludes `{#custom-id}`), the most precise of the four (commit 2cef95b) |

So: only rumdl follows the spec's suggested intent ("selectionRange is the range that is used to select the symbol... when navigating", i.e. the name). MS/MOX make them equal (legal, just less precise); Marksman uses the heading line. Nesting is universally "child heading nests under nearest preceding smaller level" (MS handles level-skipping via a parent-pop loop, digest `:4590-4730`; section = next header of ≥ level or EOF, `:2195-2221`).

### 4. Names and kinds

- Heading kind is `SymbolKind.String` in MS, Marksman, and rumdl, universal among markdown servers. MOX uses `STRUCT` (outlier). No markdown server uses `File` for headings.
- MS name = `'#'.repeat(level) + ' ' + text` with inline markup stripped (digest `:4590-4730`), the hash prefix is MS-only; rumdl/MOX use plain text; Marksman flat mode uses `H1: E` style. rumdl uses `(untitled)` for empty headings (spec requires non-empty name). MS adds optional link-definition symbols as `Constant` kind named `[label]` when `includeLinkDefinitions` is set (digest `:4590-4730`; the option exists in the library API, `:4613`, and MS's own tests pass it, `:10391`, but whether the shipping markdown-language-server turns it on is unverified).

### 5. Client failure modes and enforcement (why the rules above matter)

- **VS Code rejects the entire outline** when a child's range escapes the parent's, or `selectionRange ⊄ range`: tishlang/tish commit 57c8215 ("VS Code rejects the entire document outline with 'selectionRange must be contained in fullRange'"); Stack Overflow 55846146 ("your children symbols' range MUST BE included in their parent's range or VS Code will not display any symbol in Outline view, even if your symbols tree structure and selectionRanges are correct"). Whole-tree failure, not per-symbol failure.
- typescript-go hardened rather than trusted clients: #2351/#2658, source `packages/vscode-ts/src/outline.ts:432`, `range.contains(selectionRange) ? selectionRange : range` (repair-before-send).
- The two-range design exists because providers historically disagreed on `range` semantics (vscode#34968, origin thread).
- **Helix does not advertise `hierarchicalDocumentSymbolSupport` (flat floor) but flattens nested responses itself** (zuban#459 citing helix `nested_to_flat`, lsp.rs:374-386; helix#5048). Two consequences: (a) a `null` response when the flag is false yields an *empty* picker in Helix, return flat `SymbolInformation[]`, never null; (b) returning nested anyway happens to survive Helix, but OmniSharp is filed as buggy for ignoring the capability, the norm is to respect it (OmniSharp issue re: capabilities not respected; lsp-devtools capability matrix lists Neovim ≥0.6.1, eglot ≥29.1, VS Code ≥1.65.2 as advertising the flag).
- Neovim's original design discussion (neovim#11664): early preference for flat lists ("No, flat list is better probably"), later dispatching on response type, modern Neovim (≥0.6.1) sends the flag true, but the flat path is exercised by Helix and older clients.
- **VS Code multi-provider behavior**: multiple providers each get their own `OutlineGroup` (vscode#60641, `outlineModel.ts`); `_compact()` merges only when there is exactly one group. Quick Outline (`Ctrl+Shift+O`) flattens nested symbols into a dotted path list via `asListOfDocumentSymbols` (vscode#165583, cited in ticket 23's research), so both shapes must read well: tree in Outline view, flattened paths in Quick Outline.
- VS Code's built-in Markdown support targets "standard markdown which does not include task lists" (mjbvz, vscode#201669), see C2: no client-side expectation that a markdown outline contains tasks.

### 6. Workspace/symbol shape (Traces issue 16 alignment)

- rumdl: case-insensitive substring over indexed workspace headings; empty query returns all; no result cap visible in `symbols.rs` (digest `:40363-40930`).
- Markdown Oxide: flat `to_symbol_informations` over all referenceable nodes; empty query → full list; nucleo fuzzy scoring sorted desc; **no cap** (digest `:2960-3110`).
- Marksman: flat `SymbolInformation` with `containerName` (parent heading) (digest `:12106-12109`).
- Traces issue 16 already fixes: `Tag: #{name}`, PROPERTY/STRING, deduped, containerName `"{N} notes"`. No markdown server caps result counts; ticket 23's node caps (§C2.5) and the unfilled `workspace/symbol` p95 row (issue 33, `.scratch/md-pkm-lsp/issues/33-performance-targets-and-strategy.md:45-53`) are where Traces must decide independently. `workspaceSymbol/resolve` (3.17) is unused by all four markdown servers but is a natural fit for deferred location resolution against issue 33's LRU (23's residency model: metadata always resident, task rows/AST bodies in a 256-entry LRU with 15-20ms reload floor).

### 7. Performance baseline

- Issue 33 p95 table (`.scratch/md-pkm-lsp/issues/33-performance-targets-and-strategy.md:45-53`): "Document symbols <10ms/<5ms, headings from in-memory index", filled. **`workspace/symbol` row absent (unfilled).**
- Measured: documentSymbol ~16ns/item over the heading index (ticket 23 research XIII.1); tasks query 0.665ms @ 20K files/60K rows, 18ms @ 1M (23 research XIV.1).

---

## C2, PKM-aware outline: tasks + File-Class fields (NOVEL)

### 1. The gap (verified, not assumed)

No markdown server examined includes tasks or frontmatter fields in `documentSymbol`, MS (library-level, digest `:4590-4730`), Marksman (`:12480-12600`), MOX (`:2960-3110`), rumdl (`symbols.rs`, digest `:40363-40930`) all emit headings only (MS optionally link definitions). This matches ticket 23's tasks-as-symbols precedent survey (`.scratch/md-pkm-lsp/research/23-task-pkm-semantics.md` Part VII: absent everywhere *by design*, LSP has no task concept, so nobody surfaces tasks as symbols). zk provides no symbols at all (`tool-zk.md:13-14`). VS Code's own Markdown team scopes their outline to "standard markdown" without task lists (mjbvz, vscode#201669), i.e., C2 is genuinely novel, and the *absence* of precedent is itself the finding: nothing constrains Traces, but nothing validates it either.

### 2. Closest precedents (web, non-markdown or nonstandard)

- **claude-code-vault-keeper** (Claude Code vault LSP): returns documentSymbol trees where each file is a `SymbolKind.Object` with one child per frontmatter key plus heading children, the only direct precedent for **File-Class fields as outline symbols**.
- **Panache** (Quero/Rust, PKM-oriented Markdown viewer): document symbols include headings + tables + figures + a YAML frontmatter shape summary; workspace symbols carry parent-heading container, precedent for "more than headings in a PKM outline" without tasks.
- **nadle** (GitHub Actions runner, issue #589): task registrations surfaced as `SymbolKind.Function`; range = call site, selectionRange = task-name literal, `detail` = task type, precedent for **tasks-as-Function with detail**.
- **sanemark** (Rust/tower-lsp-server): `detail: Some("H1")`, range == selectionRange == heading bytes, `children` always `Some(...)`, precedent for `detail`-as-level metadata.
- **sylmark** (PKM LSP, Go): dynamic workspace symbols (symbol sets recompute from vault state), precedent for PKM-flavored workspace/symbol, not document symbols.
- **markymark**: symbols over headings + tags + code refs, the "everything indexed is a symbol" anti-pattern relative to Traces issue 16's deliberate tag exclusion.
- **org-lsp**: deliberately declares agenda/todo task management out of LSP scope; **Patto** delivers task lists via a custom command/panel (`:LspPattoTasks`), not documentSymbol, the two "PKM/task" servers that *exist* both declined to put tasks in the outline. That is the strongest counter-precedent available and should be weighed as design signal, not dismissed.

### 3. SymbolKind choices

- Headings: `String` (MS/Marksman/rumdl consensus), keep.
- Tasks: ticket 23 research VII.4 recommends `Array` (18) as the floor-safe kind (1-18 always supported without `symbolKind.valueSet`); mixed kinds within one outline are legal (spec has no uniformity rule; vscode#67596 shows clients tolerating mixed trees). `Array` is also the kind 23's hide-toggle design already assumes (hiding "Tasks" = hiding `Array` in the outline).
- File-Class fields: uncharted. `Field` (9) is floor-safe; `Property` (7) too. `Object` (19) is *outside* the 1-18 floor and requires `symbolKind.valueSet` negotiation, vault-keeper uses it, which only works because Claude Code's client advertises the extended set. If ticket 27 wants the file node itself as a symbol (e.g. `Object`), that's a valueSet-dependent choice; a floor-safe alternative is no file node at all (fields as top-level children of the document, which is what a flat outline of a frontmatter-only file naturally looks like).
- 23's VII.4 also notes `detail` is unused by heading peers (MS/Marksman/MOX/rumdl leave it null for headings), so `detail` is free real estate for tasks (`"TODO"`, due date) or fields (value preview) without diverging from markdown-server convention.

### 4. Nesting

Spec allows arbitrary child nesting (children are just nested `DocumentSymbol`s). Options the evidence supports:
- Tasks under their nearest ancestor heading (matches 23's ancestor-stack rule for headings; the section range already contains the task's line, so VS Code's containment invariant holds for free).
- Tasks as top-level siblings (matches how Quick Outline flattens anyway, vscode#165583, dotted paths would read `Heading / Task` vs `Task`, a readability difference either way).
- Frontmatter fields cannot nest under a heading (frontmatter precedes H1); they must be top-level or under a synthetic file node. Marksman's Emacs quirk (adds a `"."` node when a heading has children, digest `:12524` region) is a cautionary tale about synthetic nodes confusing clients.

### 5. Caps, node budgets, residency

- VS Code freezes on huge outlines: ≥250K nodes → 4-30s UI freeze; a 25K-entry symbol list caused a 5.3s incident (ticket 23 research VII.5, vscode#67596-adjacent evidence). 23 sets node caps 1k (normal) / 5k (large).
- All four markdown servers cap nothing (§C1.6), Traces adding caps is a divergence, but a defensible one given the freeze data.
- Issue 33 residency: metadata always resident; task rows in the 256-entry LRU; request-time reload has a 15-20ms floor, meaning a documentSymbol response that includes tasks may exceed the <10ms p95 target when the file's AST body is cold. This is the sharpest C1∩C2 interaction: **the document-symbol p95 budget in issue 33 was filled for headings-only** (from the in-memory index); adding tasks to the outline converts documentSymbol from an in-memory-index-only request to one that can touch the LRU. Options: keep documentSymbol headings-only in v1 (tasks appear only in workspace/symbol via issue 16's path), or include tasks and re-fill the p95 row, or include tasks only when the body is resident (response shape varies with cache state, fragile, likely reject).

### 6. Boundary with ticket 23 (from A3 + map)

23 owns: which headings are hidden (Array-hiding rule), ancestor-stack nesting, bool rollup v1, `clean_text()`/name computation. 27 owns: protocol DTO shape, hierarchical negotiation + flat fallback, kind mapping, caps, the LRU/residency hook points, client-compat enforcement (repair-before-send vs validate). The name C2 symbols get must come from 23's `clean_text()`, not a 27-local stripper (MS's inline-markup strip and rumdl's `{#id}` exclusion show this logic is non-trivial and easy to get inconsistent between outline and other features).

### 7. Editor-facing risks (C2-specific)

- Per-`SymbolKind` hide toggles (23 research VII.4): if tasks are `Array` and headings `String`, users can hide either in VS Code's outline settings, a feature, but also means a task-less outline is a legitimate user configuration, not a bug.
- Quick Outline flattening (vscode#165583): mixed-kind trees become dotted paths; `detail` is not shown in Quick Outline, so task state (`TODO`/`DONE`) only surfaces in the Outline view's `detail` column where the client renders it (VS Code does; Helix's flat picker likely doesn't).
- Helix flat floor (§C1.5): tasks and fields in the flat fallback get `containerName` = nearest heading, frontmatter fields have no heading container, so `containerName` would be the file/basename or absent; issue 16's `"{N} notes"` pattern is the closest in-house precedent for a synthetic container label.

---

## Lessons for ticket 27 (options, not decisions)

1. **Response-type negotiation**: ship both shapes, nested `DocumentSymbol[]` when `hierarchicalDocumentSymbolSupport: true`, flat `SymbolInformation[]` with `containerName` = nearest heading otherwise; never return null (empty Helix picker, zuban#459). rumdl is the reference implementation of exactly this (commit 2cef95b). Alternative (MOX-style always-nested) works with current target editors but is spec-noncompliant and files-as-buggy behavior (OmniSharp).
2. **Range convention**: adopt rumdl's precision, `range` = whole section (needed for VS Code's containment invariant and for click-to-select anywhere in the section), `selectionRange` = heading name span only; repair-before-send (`selectionRange ⊆ range`, children ⊆ parent) as a cheap invariant check à la typescript-go `outline.ts:432`, since VS Code rejects whole trees on violation (tish 57c8215, SO 55846146).
3. **Heading kind/name**: `SymbolKind.String`, name from 23's `clean_text()`; decide whether to adopt MS's `## ` hash-prefix naming (helps in flat lists, noisy in trees), precedent split, no default.
4. **Tags stay out of documentSymbol** (issue 16 decision stands; Marksman's inclusion is the only counter-example and it doubles as the reason their outline is called noisy).
5. **Tasks in the outline (C2)**: three options with real tradeoffs, (a) headings-only v1, tasks only via workspace/symbol (safest for the <10ms p95 row; matches every markdown server and the org-lsp/Patto refusal precedent); (b) tasks as `Array` children of nearest heading, `detail` = state, and re-fill issue 33's documentSymbol p95 row for the cold-LRU case; (c) tasks via a separate provider/label (3.16 `labelSupport`) so users opt in per editor. (b) is the only one that matches ticket 23's VII.4 kind plan; (a) is the only one that preserves the existing performance contract without re-measurement.
6. **File-Class fields in the outline (C2)**: top-level symbols, floor-safe kind (`Field`/`Property`), `detail` = value preview; avoid an `Object` file node (kind 19 is outside the 1-18 floor; vault-keeper only works because its client advertises extended kinds). Alternative: fields as workspace/symbol only, keeping documentSymbol purely structural.
7. **Caps**: impose 1k/5k node caps (23's numbers) even though no markdown server caps anything, the VS Code freeze data (25K entries → 5.3s) justifies the divergence; state the cap behavior (truncate + `// …N more` node? filter by kind?) as an explicit 27-vs-23 joint decision since 23 owns the hide rules.
8. **rumdl coexistence**: rumdl's `enableSymbols` (default true) means duplicate outlines until users flip it or Traces documents the toggle; 27 should document the cession contract (`tool-rumdl-boundary.md:64`) and consider Marksman's identity-gating only as a curiosity, Traces should not gate on `clientInfo`, but should test with both rumdl-symbols-on and -off.
9. **Workspace/symbol**: follow issue 16's shape; add the missing p95 row to issue 33 as part of 27's perf work; consider `workspaceSymbol/resolve` (3.17) to defer location materialization against the LRU, unused by all markdown peers, so it's novel-but-standard, and it directly addresses the 15-20ms reload floor.
10. **No Semantic Tokens/CodeLens spillover**: MS's canonical implementation ships neither (tool-ms-markdown-language-service.md key takeaway), keep Cluster 1 scoped to symbols; carries the same weight as ticket 05's original bar.

Open questions for the grilling/review pass: does (5a) or (5b) win, and does re-measuring documentSymbol-with-tasks land inside the LRU reload floor or force the p95 row up? Do File-Class fields belong in the outline at all, or is workspace/symbol their natural home (issue 16 already gave tags that answer)?

---

<a id="appendix-b2" />

## Appendix B2: B2: Folding ranges (C3) and selection ranges (C4), external implementations

## B2: Folding ranges (C3) and selection ranges (C4), external implementations

Research for ticket 27 (structural intelligence providers). Every claim below is
cited; local digests use `path:line`, web sources use URL, the LSP spec was fetched
once and cached (`tool_14e7fe773001BlwulVbV0cuGIY`, line numbers from that capture).

**Correction to A3**: A3's grounding cited "Marksman digest :3190/:3191 handlers" for
folding/selection. Those lines are the **Ionide `FSharpLanguageServer.Shim Layer`
default dispatch table** vendored inside the digest (the library's `RequestHandling.fs`
stubs `textDocument/foldingRange`/`selectionRange` with `ignore`), not a Marksman
implementation. The conclusion (Marksman implements neither) stands on different
evidence below; the citation in A3 should be re-pointed.

---

## 1. The protocol contract (LSP 3.18)

### 1.1 `textDocument/foldingRange`

- Server capability: `foldingRangeProvider?: boolean | FoldingRangeOptions |
  FoldingRangeRegistrationOptions` (spec capture, folding section around :7588-7600;
  also the [3.17 spec](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)).
- `FoldingRange { startLine, startCharacter?, endLine, endCharacter?, kind?,
  collapsedText? }`. `kind` is an open string; the standardized namespace is
  `comment`/`imports`/`region` only
  ([protocol types 3.18.3](https://cdn.jsdelivr.net/npm/vscode-languageserver-protocol@3.18.3/lib/common/protocol.foldingRange.d.ts)).
  `collapsedText` since 3.17 (custom collapse label).
- Client capabilities that matter to a server
  ([vscode-languageserver-protocol 3.18.3 d.ts](https://cdn.jsdelivr.net/npm/vscode-languageserver-protocol@3.18.3/lib/common/protocol.foldingRange.d.ts)):
  - `rangeLimit`: "a hint, servers are free to follow the limit".
  - `lineFoldingOnly`: the client **ignores** `startCharacter`/`endCharacter` and folds
    whole lines only.
  - `foldingRangeKind.valueSet` (3.17): the kinds the client knows; unknown kinds are to
    be handled gracefully.
  - `foldingRange.collapsedText` (3.17).
- `workspace/foldingRange/refresh` exists since 3.18 (server nudges the client to
  re-request).

### 1.2 `textDocument/selectionRange`

- The request returns `SelectionRange[]`, **one entry per requested position, same
  order**; `positions[i]` must be contained in `result[i].range`
  ([lsp-types `request.rs`](https://docs.rs/lsp-types/latest/src/lsp_types/request.rs.html),
  spec capture :7870-7895).
- `SelectionRange { range, parent? }`: "parent.range must contain this.range"; the chain
  is a linked list from the innermost (returned) node outward.
- "Typically, but not necessary, selection ranges correspond to the nodes of the syntax
  tree" (lsp-types doc comment, same source).
- The client capability is only `dynamicRegistration`
  ([lsp-devtools selectionRange page](https://lsp-devtools.readthedocs.io/en/latest/capabilities/text-document/selection-range.html));
  there is no rangeLimit and no options field, so clients cannot hint at a preferred
  granularity.

### 1.3 What VS Code actually sends

`vscode-languageclient` `FoldingRangeFeature.fillClientCapabilities` (mirrored verbatim
in [coc.nvim's vendored copy](https://code.jaenis.ch/Vim/coc.nvim/raw/branch/deps/src/language-client/foldingRange.ts)):

```ts
capability.dynamicRegistration = true
capability.rangeLimit = 5000
capability.lineFoldingOnly = true
capability.foldingRangeKind = { valueSet: [Comment, Imports, Region] }
capability.foldingRange = { collapsedText: false }
```

This matches the user-captured initialize payload from clangd issue #20815 (VS Code
1.87.1) exactly. Consequences for Traces:

- **`lineFoldingOnly: true`**: Traces never needs character-precision folding ranges;
  line-only spans (all ticket 11's planned span model gives us) are sufficient for the
  most common client.
- **`rangeLimit: 5000`**: a hint; MS's own service truncates at it (section 3.1).
- **`collapsedText: false`**: no need to emit custom collapse labels.

---

## 2. What consumers actually do (editor matrix)

| Editor | Folding via LSP | Selection via LSP | Evidence |
|---|---|---|---|
| **VS Code** | Yes; the bundled markdown extension runs `vscode-markdown-languageservice` (`extensions/markdown-language-features/src/extension.ts`, `startServer()`, fetched from `microsoft/vscode` main) | Yes; same service's smart select | Primary consumer of both features |
| **Neovim** | Yes, opt-in: `vim.lsp.foldexpr()` (PR [#33060](https://github.com/neovim/neovim/pull/33060), 0.11); the docs' example defaults to `vim.treesitter.foldexpr` and only prefers the LSP `foldexpr` when `supports_method('textDocument/foldingRange')`; supports filtering by kind and `collapsedText` | Yes: `vim.lsp.buf.selection_range(direction, timeout)` (PR [#34011](https://github.com/neovim/neovim/pull/34011)) mapped to visual-mode `an`/`in`; docs: treesitter parent-node first, LSP selection_range only as fallback | Treesitter-first, LSP second |
| **Zed** | Yes, since 2026-02: PR [#48611](https://github.com/zed-industries/zed/pull/48611) `textDocument/foldingRange`; per-buffer-version cache, 30 ms debounce, in-flight dedup; the `document_folding_ranges: "on"/"off"` setting is **off by default**; falls back to tree-sitter/indent when the LSP returns nothing; honors `workspace/foldingRange/refresh` | **No**: repo-wide search of `crates/project/src/lsp_command.rs` finds `selection_range` only as the `DocumentSymbol` field (across multiple recent commits); no `SelectionRangeRequest` anywhere in the repo; expand-selection is tree-sitter (`SelectLargerSyntaxNode`) | Server-side selection ranges are invisible in Zed |
| **Helix** | **No**: issue [#1840](https://github.com/helix-editor/helix/issues/1840) open; PR #14593 still open/draft as of 2026-01 | **No**: expand selection is tree-sitter `parent-node` only | Neither feature reaches Helix |
| **Emacs (eglot)** | dynamicRegistration supported ([lsp-devtools](https://lsp-devtools.readthedocs.io/en/latest/capabilities/text-document/selection-range.html)); folding UI varies by frontend | Same | Thin evidence |
| **coc.nvim** | Yes; the client feature mirrors vscode-languageclient ([source](https://code.jaenis.ch/Vim/coc.nvim/raw/branch/deps/src/language-client/foldingRange.ts)) | Yes (same lineage) | Consumes both |

**Net**: folding reaches every major client (as an opt-in or default-off feature);
selection ranges reach VS Code (richly) and Neovim (as a fallback under treesitter), and
reach **neither Zed nor Helix**.

---

## 3. C3: folding ranges, what implementations emit

### 3.1 VS Code markdown language service (the reference implementation)

`src/foldingProvider.ts`, local digest
`docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt:4984-5150`;
tests at `:11391-11670`.

Three range sources, one walk of the markdown-it token stream:

1. **HTML comments** `<!-- #region name -->` / `<!-- #endregion -->` get `kind: Region`.
2. **Heading sections**, from the same TOC the outline uses: start is the heading's
   line, end is the end of its section (content of all deeper headings), with
   **trailing blank lines trimmed**. Headings get **no kind**.
3. **Every block token**: `fence`, `list_item_open`, `table_open`, `blockquote_open`
   are emitted with **no kind**; `html_block` gets `kind: Comment` when the block starts
   with `<!--`, otherwise none.

Cross-cutting behaviors:

- **`rangeLimit` = 5000** hard-coded: "if the number of folding ranges exceeds the
  rangeLimit, truncate the list", with no ranking and no smallest-first preference
  (digest :5050-5070 region).
- **`lineFoldingOnly` respected implicitly**: the provider always emits line bounds.
- An empty result returns `null` (not `[]`).
- **No frontmatter folding**: the only frontmatter mention in the whole test suite is a
  `test.skip` for "fenced code blocks with yaml front matter" (digest :11590).
- Tests pin subtle behavior worth copying (digest :11391+):
  - a heading fold excludes trailing blank lines;
  - a list with lazy continuation folds to **end of file**;
  - multi-line fences/HTML/tables/blockquotes fold; single-line constructs do not.

### 3.2 Panache (Rust; Pandoc/Quarto Markdown LSP, the closest analog to Traces)

`src/lsp/handlers/folding_ranges.rs` (357 lines including tests; added in commit
[`7ce6ce2`, 2026-02-13, +370/-2 across 4 files](https://github.com/jolars/panache/commit/7ce6ce27a4abe2df6c6e087a1bab0222a1ea3f38);
source fetched from
[docs.rs 3.6.1](https://docs.rs/crate/panache/latest/source/src/lsp/handlers/folding_ranges.rs)).
Feature list: [panache.bz/guide/lsp.html](https://panache.bz/guide/lsp.html)
("Code folding: Headings, Code blocks, Fenced divs, YAML frontmatter, HTML blocks").

Algorithm (single walk of `document.blocks()` over the rowan CST):

- Collects heading `(level, start_offset)` pairs (levels come from a salsa symbol
  index), then for each heading finds the **next heading with `level <= own`** (or EOF)
  as the section end; `end_pos = end_offset - 1`; emits only if `start_line < end_line`.
  The fold **starts on the heading line itself**, the same choice MS makes.
- Emits `CODE_BLOCK`, `FENCED_DIV`, `YAML_METADATA`, `HTML_BLOCK`/`_RAW`/`_DIV`
  immediately, with the same multi-line guard.
- **Everything gets `kind: Some(FoldingRangeKind::Region)`**, including code blocks and
  frontmatter. No `start_character`/`end_character` (line-only).
- The handler is fully synchronous: "Build folding ranges synchronously (SyntaxNode is
  not Send)", reading `snap.parsed_tree(&uri)` and `snap.line_index(&uri)` from the
  state snapshot, with no cancellation checks and no async.
- Emits `None` when the range list is empty.
- **Deliberately does NOT fold** list nesting, blockquotes, or tables, only the five
  kinds above (verified against both the source match arms and the docs feature list).
- Tests cover heading hierarchy, code blocks, fenced divs (`::: {.callout}`), YAML
  frontmatter, `<details>`/comment/`<div>` HTML blocks, and the negative cases (inline
  HTML, plain paragraphs, single-heading-no-content producing an empty result).

### 3.3 Quarto (wraps the MS service)

Quarto's bundled LSP registers `foldingRangeProvider: true` and
`selectionRangeProvider: true` and delegates both to
`vscode-markdown-languageservice` (`quarto-dev/quarto`, `apps/lsp`, fetched earlier in
this research run). Data point: the MS service is the shared substrate for a second
shipping product, so its behaviors (sections 3.1, 4.1) are not one editor's quirks.

### 3.4 Implementations that ship neither feature

| Server | Evidence |
|---|---|
| **Marksman** | `mkServerCaps` sets no `FoldingRangeProvider`/`SelectionRangeProvider` (`…marksman-digest.txt:11144-11170`); `Server.fs` overrides no folding/selection handlers (`:10995-12032`); `docs/features.md` (`:1218-1351`) lists every supported method and omits both. The `:12518` `SelectionRange` hits are DocumentSymbol fields. |
| **Markdown Oxide** | The `ServerCapabilities` struct literal ends `..Default::default()` with no folding/selection entries (`…markdown-oxide-src-digest.txt:2024-2105`); the only `selection_range` hit is DocumentSymbol (`:3095`). |
| **rumdl** | No `FoldingRange`/`SelectionRangeParams` identifiers in the src/tests/docs digests; the docs settings table has no folding/selection option. Caveat: the src digest omits `src/lsp/server.rs` (`…rumdl-src-digest.txt:38447` lists it as a module but its body is not captured), so capability advertisement is not directly verified; absence-of-handler-type plus docs silence is strong but not airtight. |
| **zk** | Zero folding/selection matches across all four digests. |
| **Panache (selection)** | Coverage table: `textDocument/selectionRange` is "❌ Planned: structural smart-select" ([docs](https://panache.bz/guide/lsp.html#lsp-specification-coverage)). Even a feature-rich, actively-developed Rust Markdown LSP deferred it. |

### 3.5 Range-emission matrix (C3)

| Range source | MS mdLs | Panache | Notes for Traces |
|---|---|---|---|
| Heading sections | yes, no kind, blanks trimmed | yes, kind=Region, includes heading line | Both fold to the next same-or-higher level. The intersection is the must-have. |
| Fenced code blocks | yes (`fence`) | yes | The intersection is the must-have. The Traces AST lacks fence extents (A1), so this needs a ticket 11 span extension or a line scan. |
| HTML comment regions | yes, kind=Region | (HTML block becomes Region) | The MS-only `#region` syntax; low PKM value. |
| HTML blocks | yes, Comment if `<!--` | yes, Region | Low value in vaults. |
| Lists (nesting, lazy continuation) | yes (`list_item_open`; continuation folds to EOF per tests) | **no** | MS-only; Traces already has the ticket 23 list walk. |
| Blockquotes / callouts | yes (`blockquote_open`) | **no** | The Traces-specific twist: ticket 17 callout extents would let us fold callout bodies semantically. |
| Tables | yes (`table_open`) | **no** | Marginal; tables are usually short in PKM. |
| YAML frontmatter | **no** (only a `test.skip`) | **yes** | Panache-only; high value for vaults whose notes open with 10+ lines of frontmatter. |

### 3.6 Kinds, caps, cancellation (C3 cross-cutting)

- **Kind vocabulary**: the standardized set is `comment`/`imports`/`region` and nothing
  else. MS leaves structural folds unkindled and reserves `Region` for explicit
  `#region` comments and `Comment` for HTML comments. Panache stamps `Region` on
  everything. The consequence of Panache's choice is that VS Code's "Fold all regions"
  command swallows the whole document; with MS's choice, heading folds are still
  foldable but excluded from kind-targeted commands. `imports` is meaningless for
  Markdown.
- **Capping**: only MS implements one (truncate at `rangeLimit`, no ranking). Zed
  instead caches per buffer version, debounces 30 ms, and supports server refresh. A
  prose vault note with 500 headings produces roughly 500 heading folds plus a handful
  of fences, an order of magnitude under VS Code's 5000 hint, so a cap is defensive
  only.
- **Cancellation**: MS checks the token between its four computation stages (TOC,
  blocks, inline, links) because it also awaits a link cache. Panache never checks:
  synchronous snapshot read plus one tree walk. Traces under ticket 12
  (`concurrency_level(1)`, sequential dispatch, stale-result discard, no
  CancellationToken infrastructure) must behave like Panache: no I/O, no async, one
  O(document) pass, done. Budget evidence: the ticket 23 list walk is 0.069 ms per 10 KB
  of prose, so a folding pass of similar shape is comfortably sub-millisecond on real
  notes.

---

## 4. C4: selection ranges, the one real implementation

### 4.1 VS Code markdown language service `smartSelect.ts`

`src/features/smartSelect.ts`, local digest `…-digest.txt:6805-7263`; tests
`:15294-16274`.

The chain is built **outside-in as parent pointers**, and the innermost node is
returned:

1. **Header range**, from the TOC: the heading line through the start of its first
   child, an "intermediate" range between the heading and the full section. Present
   when the position is inside a section.
2. **Block ranges**: every block token containing the position, sorted largest-first
   (paragraph, list item, nested list, table, blockquote, fence, ...). `heading` tokens
   are explicitly excluded (`isBlockElement` skips them because the TOC already
   provided the header range).
3. **Inline ranges**, innermost-first: table cell, bold, italic, bold-italic combo,
   link, inline code. Each becomes the parent of the previous.

Cross-cutting behaviors:

- `parent` is the next-larger range; every range contains the position; the client
  receives a single `SelectionRange` (VS Code requests one position at a time).
- **Cancellation is checked between stages** (4 stages), not inside them.
- Link ranges come from the **cached** `linkProvider.getLinks` result, so no separate
  link pass runs per keystroke.
- Tests (`:15294+`) pin chains like word, link text, link, list item, section, and
  table cell, table, blockquote, section.

### 4.2 Why nobody else shipped it

- Panache: explicitly "Planned: structural smart-select" (section 3.4); the feature
  lost the cut against formatting/diagnostics/navigation in a feature-rich server.
- Marksman, Markdown Oxide, rumdl, zk: no (section 3.4).
- Zed: no client support at all (section 2), so a server implementation is invisible
  there.
- Helix: no client support (section 2).
- Neovim: treesitter-first; LSP selection_range is only the fallback when treesitter is
  inactive for the language, and Markdown treesitter grammars are common in Neovim
  setups.
- VS Code: already served by the bundled markdown extension (section 3.3), so a second
  provider for `.md` would compete with it rather than fill a gap.

### 4.3 What a Markdown selection chain can even offer

From the MS chain, the Markdown-specific layers are: heading section, list item
(nested), paragraph, blockquote/table/fence, link/bold/italic/code/cell. Of these, the
current Traces AST (ticket 11) retains **no inline spans** and no code-fence/frontmatter
extents (A1 lines 133-142), so the inline tier (section 4.1 stage 3) is unavailable
without either an AST extension or a re-tokenization pass. The block tier (stages 1-2)
is computable from the ticket 11 heading list plus the ticket 23 list walk plus
paragraph boundary detection.

---

## 5. Cost/quality tradeoffs observed

| Decision | Who | Tradeoff |
|---|---|---|
| Whole-document pass, truncate at limit | MS | Simple, with order-dependent quality loss (no smallest/most-useful-first ranking); fine in practice at 5000 |
| No limit at all | Panache | Correct for prose; relies on document size staying small |
| Cache + debounce + refresh instead of capping | Zed (client) | Moves cost off the request path; the server is still O(document) |
| Stamp one kind on everything | Panache | Cheap; pollutes kind-targeted commands |
| Leave structural folds unkindled | MS | Kind commands stay meaningful; slightly "dumber" responses |
| Inline tier needs source spans | MS | Requires a tokenizer that retains inline extents; the expensive half of selection |
| Sync, no cancellation | Panache | Viable only because the pass is sub-ms; MS cancels between stages because its stages await caches |
| Deferred the whole feature | Panache (selection) | Shipping 8 navigation features first beat smart-select; consumers did not revolt |

---

## 6. One structural walk for both (and for the rest of ticket 32)

Both providers consume the same facts: heading levels and line spans, block boundaries
(code fences, list items with nesting, blockquotes/callouts, frontmatter, tables), and
paragraph boundaries. Evidence that this is the natural shape:

- MS derives the TOC, folding, document outline, **and** the selection header tier from
  one cached table-of-contents pass (digest index.ts capability surface `:1475-1807`;
  the folding provider and smartSelect both start from the TOC).
- Panache's folding handler reads the same `parsed_tree` + `line_index` snapshot every
  other handler uses, and heading levels come from the shared salsa symbol index.
- Traces-specific: ticket 23's list walk (ancestor stack, 0.069 ms per 10 KB) and
  ticket 11's heading list are already the two biggest inputs; ticket 17 will expose
  callout extents; ticket 14's overlay resolver supplies text uniformly.

Design implication for ticket 32 (ownership split still undecided): a single
`structural_walk(document) -> Vec<StructuralEvent>` (heading, list item, fence,
blockquote, frontmatter, paragraph, each with line span plus depth) is the deep module;
the C3 and C4 handlers become thin filters over it, and document symbols/outline
(already in Traces) are a third consumer. This keeps each handler trivially O(events)
and keeps the "cancellation equals keep it cheap" contract of ticket 12 enforceable in
one place.

---

## 7. The evidence-based case for omitting either feature

**Omit C4 (selection ranges), the strong case:**

1. Two of the four first-class target editors cannot consume it at all (Zed has no
   client support, section 2; Helix has none, section 2).
2. The third (Neovim) prefers treesitter and uses the LSP only as a fallback (section 2).
3. The fourth (VS Code) **already has** the best-in-class Markdown implementation
   bundled (sections 3.1, 3.3), and for `.md` files Traces cannot beat it without first
   buying inline spans it does not have (section 4.3).
4. The most comparable independent implementation deferred it (Panache, section 3.4)
   with no user-visible regression reported.
5. The Traces-specific cost: the inline tier requires AST work (the ticket 11
   extension), the most invasive option in this whole ticket, and a block-only chain
   would be a subset of what VS Code already provides.

**Omit C3 (folding), the weak case:**

1. Every editor family consumes folding (section 2); Zed ships it since 2026-02 and
   Neovim since 0.11.
2. It is the cheaper of the two: line-based, so ticket 11's line spans suffice, no
   inline data needed, one sub-ms pass; Panache shipped the whole thing in 357 lines.
3. Traces has vault-specific wins no generic server offers: frontmatter folds (only
   Panache does this), callout folds gated on ticket 17, and consistency with Traces's
   own heading semantics; meanwhile in VS Code the built-in markdown provider supplies
   the generic folds already, and multiple folding providers **merge** there.
4. The only honest argument against is "not now", specifically sequencing it after
   ticket 17 (callout extents) so the first cut can include callouts.

---

## 8. Lessons for ticket 27: concrete options

**C3 (folding), recommended: ship, minimal-then-extend.**

- **C3-min (first cut)**: heading sections (fold to the next same-or-higher level,
  trailing blanks trimmed, single-heading-no-content produces no fold), fenced code
  blocks, and YAML frontmatter. Line-only, matching VS Code `lineFoldingOnly`. `kind`
  left unset for all three (the MS convention, which keeps the client's "Fold all
  regions" command honest; Panache's Region-stamping is the anti-pattern). Return
  `None`/null when empty. Sources: ticket 11 headings plus the ticket 23 walk;
  fence/frontmatter extents need the small ticket 11 span additions or a bounded line
  scan.
- **C3-ext (follow-up)**: list-item nesting folds (lazy continuation folds to end of
  file, per the MS tests) and callout/blockquote folds gated on ticket 17.
- **Caps**: skip `rangeLimit` handling initially; vault notes are orders of magnitude
  under VS Code's 5000 hint. Revisit only with evidence.
- **Cancellation**: none (the Panache model); the pass is one synchronous O(document)
  walk under the ticket 12 sequential dispatcher, and the ~100 ms budget is kept by
  construction, not by tokens.

**C4 (selection ranges), recommended: omit from the first cut; optional block-only tier later.**

- **C4-A (default)**: omit. Zed and Helix cannot use it, Neovim prefers treesitter,
  VS Code already ships a superset for Markdown, and Panache deferred it too. Record
  the decision and re-open only if a target editor workflow demands it.
- **C4-B (if pursued later)**: block-tier chain only, `paragraph -> list item (nested)
  -> section`, built as parent pointers from the same structural walk (section 6),
  innermost returned; no inline tier (no inline spans in the AST, section 4.3); one
  position per request is enough, since Neovim and VS Code request one at a time in
  practice. Do not attempt MS's inline parity: it is unreachable without the AST
  extension and redundant in VS Code.
- Either way, **do not** implement selection before the shared walk exists; building it
  as a second ad-hoc walker is the duplication that section 6 exists to prevent.

**Shared infrastructure, regardless of the C4 decision:**

1. Land `structural_walk` (or the equivalent) as the ticket 32 structural-intelligence
   core; C3 filters it, symbols/outline consume it, and C4-B (if ever) joins as a third
   consumer.
2. Kinds: MS discipline, unset for structure, `comment` reserved for folded comment
   blocks if and when they are emitted; never emit `imports`; `region` only for literal
   `#region`-style markers if Traces ever gets them.
3. Empty results: return null/no provider result rather than `[]` (both MS and
   Panache).
4. Sequencing: after ticket 17 (callout extents) and the ticket 11 span additions;
   folding is then a single cheap handler, and ticket 32's ownership question answers
   itself via the shared walk.

---

## Sources

- Local digests:
  `docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt` (folding
  :4984-5150, smartSelect :6805-7263, folding tests :11391-11670, smartSelect tests
  :15294-16274, capability surface :1475-1807, frontmatter `test.skip` :11590);
  `…marksman-digest.txt` (caps :11083-11170, overrides :10995-12032, features.md
  :1218-1351, Ionide dispatch :3190-3191, see the correction at the top);
  `…markdown-oxide-src-digest.txt` (:2024-2105 capabilities, :3095);
  `…rumdl-{src,docs,tests}-digest.txt` (:38447 module list); zk digests (absence).
- LSP spec capture: `tool_14e7fe773001BlwulVbV0cuGIY` (~:7540-7900).
- [vscode-languageserver-protocol 3.18.3 folding d.ts](https://cdn.jsdelivr.net/npm/vscode-languageserver-protocol@3.18.3/lib/common/protocol.foldingRange.d.ts);
  [coc.nvim vendored vscode-languageclient foldingRange.ts](https://code.jaenis.ch/Vim/coc.nvim/raw/branch/deps/src/language-client/foldingRange.ts);
  [lsp-types request.rs](https://docs.rs/lsp-types/latest/src/lsp_types/request.rs.html);
  [lsp-devtools selectionRange](https://lsp-devtools.readthedocs.io/en/latest/capabilities/text-document/selection-range.html).
- Panache: [LSP guide and coverage table](https://panache.bz/guide/lsp.html);
  [folding_ranges.rs source (docs.rs 3.6.1)](https://docs.rs/crate/panache/latest/source/src/lsp/handlers/folding_ranges.rs);
  [commit 7ce6ce2](https://github.com/jolars/panache/commit/7ce6ce27a4abe2df6c6e087a1bab0222a1ea3f38).
- Editors: [Zed PR #48611](https://github.com/zed-industries/zed/pull/48611); Zed
  `crates/project/src/lsp_command.rs` (search-verified absence of SelectionRange);
  [Neovim PR #33060](https://github.com/neovim/neovim/pull/33060),
  [PR #34011](https://github.com/neovim/neovim/pull/34011);
  [Helix issue #1840](https://github.com/helix-editor/helix/issues/1840);
  vscode `extensions/markdown-language-features/src/extension.ts` (fetched from main);
  Quarto `apps/lsp` capability registration (fetched earlier in this run).
- Traces context: `.scratch/md-pkm-lsp/issues/{11,12,14,17,23,32}-*.md`;
  `A1-codebase-review.md` C3/C4 lines 133-142; `A3-decomposition.md` (correction above).

---

<a id="appendix-b3" />

## Appendix B3: Ticket 27, Cluster 3 (B3): Document links, template references, rumdl mapping

## Ticket 27, Cluster 3 (B3): Document links, template references, rumdl mapping

Produced: 2026-10-18. Covers A3 concerns **C5, C6, C12**. PLANNING ONLY; no production code touched.

**Method notes.** **V** = verified this session against local digests or cached sources; **W** =
verified against a web primary source (upstream repo/docs/spec, read 2026-10-18); **I** = inference
from verified facts. Rumdl upstream claims cite `rvben/rumdl` `main` at version **0.3.0** (read
2026-10-18); local digests are an older snapshot and are cited where they corroborate. LSP spec
citations use the upstream **LSP 3.18** specification (primary source; the local
`docs/refs/lsp_spec.md` stubs per A3's grounding note).

---

## 0. Grounding corrections (facts that amend A3 / earlier claims)

1. **Rumdl's provider list is now fully enumerated from source** (the A3 digest gap, "digest
   cannot prove a negative" for `server.rs`/`navigation.rs`, is closed by reading upstream
   `src/lsp/server.rs` capabilities block directly, W). Full list in §3. Zero documentLink, zero
   folding, zero selection-range, zero CodeLens, zero semantic tokens: **confirmed at the
   advertisement site**, not just by module-list absence.
2. **A3 C12's citation "settings table ~:1885-2158" is wrong.** The rumdl docs settings table is
   at `docs/digests/lsp_rvben-rumdl-docs-digest.txt:2606-2608`; the joint-setup example (all three
   toggles false) is at :2637-2642. Cite those.
3. **Markdown Oxide contradicts A3 C7's "zero evidence of presence" for CodeLens** (flagged here
   for Cluster 4, out of B3's scope): it registers `code_lens` (src digest :882, :2116),
   `semantic_tokens_full` (:2446, :3289) and `document_symbol`. B3 re-verified it has **no**
   `documentLink` (`document_link` count 0) and **no** folding/selection providers
   (`folding_range` count 0; the single `selection_range` hit at :3095 is a heading-node field,
   not a provider).
4. **Helix documentLink support is master-only, unreleased.** `goto_file_impl` on master prefers
   LSP document links and awaits `documentLink/resolve` in a background job
   (`helix-term/src/commands.rs`, `selection_overlaps_document_link`,
   `resolve_document_link_request`; W, read 2026-10-18). The latest release, **25.07.1**
   (2025-07-18), has `goto_file_impl` at :1318 but **zero** document-link code (W). Released
   Helix is sparse-floor; master Helix is fine.
5. **Zed document links shipped 2026-05-26** via PR/issue #56011, closing feature request #33587
   (opened 2025-06-28) (W). Current Zed (first-class per map Notes) renders LSP document links.
6. **Neovim handles `textDocument/documentLink` via `gx` since 0.12** ("`gx` handles
   `textDocument/documentLink`", `runtime/doc/lsp.txt`, present in the v0.12.0 tag, absent in
   v0.10-v0.11.x; W). Current stable is 0.12.6 (2026-10-09) (W). No native underline
   decoration; `lsplinks.nvim` adds underline/status rendering on top (W).
7. **Rumdl's `DocumentLinks` type is MD057-internal** (`md057_existing_relative_links`,
   `for_document`, src digest :78974, :85936-85975): a lint data structure, not an LSP provider.
   Do not conflate (A3 already flagged this; re-confirmed V).

---

## 1. C5: Document links

### 1.1 The protocol contract (LSP 3.18, primary)

- `textDocument/documentLink` returns `DocumentLink[] | null`; each link is
  `{ range, target?, tooltip?, data? }` (spec, "Document Link Request"; W). `range` is the
  clickable range (per ticket 15: the target/url span, not the whole link syntax).
- `target?: URI`: **"If missing, a resolve request is sent later."** This is the entire basis of
  the lazy-resolve pattern; a server without `resolveProvider` that omits `target` yields a link
  clients render but cannot open.
- `tooltip?` (since 3.15): the client appends its own trigger instructions
  (`"{0} (ctrl + click)"`); requires client `tooltipSupport` (VS Code 1.65.2+, per lsp-devtools
  matrix; W).
- `data?`: preserved verbatim between the request and `documentLink/resolve`.
- Server capability: `documentLinkProvider?: DocumentLinkOptions` with `resolveProvider?`;
  dynamic registration supported (`DocumentLinkRegistrationOptions`); partial results supported.
- Client capabilities: `dynamicRegistration?`, `tooltipSupport?` only.

### 1.2 What Markdown/PKM LSPs actually emit

| Server | Registers documentLink? | Emission set | target strategy | Notes |
|---|---|---|---|---|
| **MS markdown-language-service** | yes | inline links (incl. images nested in link text), reference links, reference *definitions*, autolinks (external), HTML `src`/`href` attrs | **internal → lazy** (`target: undefined` + `data`, tooltip "Follow link"); **reference link → definition site**, tooltip "Go to link definition"; external → eager | per-doc versioned link cache; `NoLinkRanges` suppresses code blocks; `#toValidDocumentLink` at :4517, tooltips :4529/:4544 (V) |
| **Marksman** | yes + **`documentLink/resolve`** (dispatch :3175/:3177) | not visible in digest | not visible | emission internals are F# behind the dispatch table; registration is certain, behavior is not (digest limitation, stated honestly) (V) |
| **zk** | yes, `ResolveProvider: true` (:4256) but **no resolve handler registered** (full handler list checked) | standard `ast.Link` **and `extensions.WikiLink`** (:3404-3463, :3451) | **eager**; internal links emitted **only if the target note resolves** (unresolvable internal links silently dropped, :4434-4454); skips `file:///` and `magnet:?`; percent-decodes | advertise/impl mismatch is a client-visible wart (resolve requests would error); PKM-wikilink-as-documentLink precedent (V) |
| **rumdl** | **no** (§0-1) | n/a | n/a | owns standard-link *completion/navigation* instead (§3) (V+W) |
| **Markdown Oxide** | **no** | n/a | n/a | (V) |

**Template LSPs do not use documentLink at all** (feeds C6): both jinja-lsp forks
(alex-oleshkevich 0.4.0: "go-to-definition (macros, blocks, templates, imports), find references";
uros-5: "Goto Definition … templates") and fjkit-lsp (route/template reference tables) expose
include/extends/import exclusively as **definition/references** (W: PyPI/GitHub READMEs).

### 1.3 Client rendering, all four target editors

| Client | documentLink support | Visual affordance | Trigger |
|---|---|---|---|
| **VS Code** | first-class since protocol birth; negotiates `dynamicRegistration` + `tooltipSupport` (1.65.2+) | underline + cmd+click; tooltip on hover | native |
| **Neovim ≥ 0.12** (current stable 0.12.6) | core `gx` consults documentLink first, falls back to regex path detection | **no underline by default**; `lsplinks.nvim` adds underline + status | `gx` |
| **Zed** (since 2026-05-26, #56011) | yes | clickable link ranges | cmd+click |
| **Helix released (≤ 25.07.1)** | **no** | n/a | n/a |
| **Helix master** | `goto_file` prefers LSP links when selection overlaps a link range; resolves lazy links in a background job | no underline (on-demand only) | `gf` |

**Sparse-floor conclusion**: Helix-release users get nothing from documentLink; they still get
go-to-definition if the server registers it (C6's argument for registering definition for template
spans). Neovim users need `gx` (or a plugin); `gd` via definition registration also works.

### 1.4 Non-overlapping emission set for Traces (recommendation, I)

- **Emit**: wikilinks, embeds, cross-file `file.md#heading` links (ticket 15's ranges; ticket 25's
  ownership), template `{% include %}`/`{% extends %}` spans (C6). Exactly the targets rumdl
  cannot parse: wikilinks are permanently out of rumdl's scope (issue 32), and template syntax is
  not Markdown.
- **Do not emit**: standard relative Markdown links, anchors, and disk-target images: rumdl's
  completion/navigation owns them; emitting them duplicates UX and breaks the clean-split goal
  (A3 C5's core warning).
- **Ambiguity predicate (26's input)**: emit for `Unique` and `Ambiguous`, never error. For
  `Ambiguous`, omit `target` and set `tooltip` to e.g. `"N candidates (gd to choose)"`; the spec's
  missing-target contract renders-but-does-not-open, which is the honest UX. Do **not** copy zk's
  drop-unresolvable behavior: it silently removes affordance exactly when the user needs
  feedback, and conflicts with standing constraint #1 (overlay-only templates must stay visible).
- **Eager vs lazy**: v1 **eager** `target` for `Unique` (zk model, simplest; resolution is already
  computed for the 26 predicate, no extra I/O). Defer `data`+`documentLink/resolve` (MS model)
  until a measured need; advertising `resolveProvider: false` and adding resolve later is a
  non-breaking upgrade (spec: clients re-request resolve only when target is missing).
- **Perf row**: documentLink returns per-current-document occurrences only (a note has tens of
  links, not thousands); propose "derive from Document symbols <10ms" for 33's table. The 5,000
  references ceiling (26) governs *references* requests and must not be conflated with link counts.

---

## 2. C6: Template references as document links vs go-to-definition

**Evidence**: no template LSP uses documentLink (§1.2); the convention across jinja-lsp/fjkit-lsp
is goto-definition (+ references). The PKM convention (zk) is documentLink for note references.
The ecosystem split is not contested; it is *layered*: **documentLink = pointer affordance
(click/gx/gf), definition = keyboard command (`gd`)**. MS serves internal doc links via documentLink
only; zk via documentLink only; jinja via definition only. **Nobody in the corpus serves both for
the same span**: that is the genuine open product decision A3 named.

**Options**:

- **A (recommended): both, fed by the same 22-derived spans + the Q20(b) overlay seam.**
  documentLink for clickability (VS Code/Zed/Neovim-gx), goto-definition for keyboard and for the
  Helix sparse floor. Cost is one extra provider over spans 22 already derives; both consume the
  identical resolution result, so they cannot drift (C15's shared-predicate discipline: one
  resolution pass, two renderings).
- **B: definition only (jinja precedent).** Cheaper; loses click affordance in VS Code/Zed for
  template spans. Defensible if 22's spans are late/optional (A3: no `Blocked by` edge).
- **C: documentLink only.** Not recommended: breaks `gd` keyboard flow that every template-LS
  user expects.

**Existence state (overlay-aware, per standing constraint #1)**: documentLink can only express
state through `tooltip` and target-absence; the client-visible signal is **tooltip text**
(`"missing template"`, `"overlay-only template"`) plus, when C10 ships, 26's three-valued semantic
modifiers. Do not gate *emission* on existence (unlike zk): emit the link, annotate the state.

**Open question for the hub**: does option A's definition registration risk double-results with
rumdl's `definition_provider` (`enableLinkNavigation`)? In practice no: rumdl navigates standard
Markdown links only (its navigation module parses Markdown links; wikilink/template positions are
outside its parse), so the two servers' answer sets are position-disjoint. Document this
rather than introducing a toggle.

---

## 3. C12: rumdl coexistence mapping (feeds ticket 32)

Authoritative enumeration: upstream `src/lsp/server.rs` capabilities block, `main` @ 0.3.0 (W,
read 2026-10-18), corroborated by local digests (V). Rumdl registers:

`text_document_sync` (FULL, `will_save_wait_until`), `code_action` (QUICKFIX, SOURCE_FIX_ALL,
`source.fixAll.rumdl`), `document_formatting`, `document_range_formatting`,
`document_symbol` (**gated** `enable_symbols`), `workspace_symbol` (**gated**), pull
`diagnostic_provider` (identifier "rumdl", inter-file, no workspace diagnostics), `completion`
(trigger chars `` ` `` always; `( # / . -` only when `enable_link_completions`, dynamically
unregistered when off, docs :2472-2478), `definition`/`references`/`hover`/`rename`(+prepare)
(**all gated** `enable_link_navigation`), `workspace_folders`. Settings table:
docs digest :2606-2608; joint-setup example :2637-2642.

**Per-feature ownership table for ticket 27's eight features**:

| Feature | rumdl | Gate | Traces role | Joint-setup action |
|---|---|---|---|---|
| documentSymbol | **yes** (heading tree) | `enableSymbols` | PKM outline (23's content), **direct overlap** | set `enableSymbols = false` (rumdl's own documented pattern, :2608/:2639) |
| workspace/symbol | **yes** | `enableSymbols` | tags-as-unique-symbols (16), overlap in mechanism | same toggle covers it |
| documentLink | **no** | n/a | wikilinks/embeds/`file.md#heading`/template spans | **none needed**: pure gap-fill |
| foldingRange | **no** | n/a | headings/lists/fences/callouts/frontmatter | none needed |
| selectionRange | **no** | n/a | structural expansion | none needed |
| codeLens | **no** | n/a | Cluster 4's decision | none needed |
| codeAction | **yes** (lint quickfix + fixAll only) | always | conversion + extract-to-note (`refactoring` kind) | none needed; kinds are disjoint, clients merge actions from multiple servers |
| semanticTokens | **no** | n/a | wikilink resolution state (C10) | none needed |
| definition/references/hover/rename | **yes** for *standard links* | `enableLinkNavigation` | templates/wikilinks (position-disjoint) | keep rumdl's toggle **true**; document position-disjointness (§2) |
| completion | **yes** (paths/anchors/fence langs) | `enableLinkCompletions` | ticket 24's architecture owns the joint story | (documented in 24, not here) |
| diagnostics | **yes** (lint) | always-on (rule toggles via `disableRules`/`enableRules`) | 25's cession already settled | settled by 25 |
| documentHighlight | **no** | n/a | 16 tags + link occurrences (26's `highlight`) | none needed |

**Headline for 32**: of the eight features, **only documentSymbol/workspace-symbol needs a joint
toggle** (`enableSymbols = false`). Everything else Traces ships in 27 fills a rumdl-ceded gap
with zero coexistence machinery; navigation overlap is position-disjoint and needs documentation,
not toggles. Rumdl's linting/formatting are always-on and orthogonal.

---

## 4. Lessons for ticket 27: concrete options

- **C5-1 (emission set)**: adopt the non-overlap rule §1.4: wikilinks/embeds/`file.md#heading`/
  template spans only; never standard links. Rationale: rumdl parses only standard links; the split
  is parseable-syntax-disjoint, not toggle-dependent.
- **C5-2 (ambiguity)**: `Unique` → eager `target`; `Ambiguous` → no `target` + count tooltip;
  missing/overlay → tooltip + (later) token modifier. Never drop, never error (26's predicate).
- **C5-3 (resolve)**: `resolveProvider: false` in v1; the upgrade path (MS's `data`+resolve) is
  non-breaking and unneeded while resolution is already computed eagerly.
- **C5-4 (perf)**: propose 33's documentLink row as "derive from Document symbols <10ms"; keep
  26's 5,000-reference ceiling conceptually separate from per-document link counts.
- **C6-1**: template spans get **both** documentLink and go-to-definition (option A), both fed by
  one resolution pass over 22's spans + Q20(b) overlay seam (C15's shared predicate at work).
- **C6-2**: existence/overlay state surfaces via tooltip now, semantic modifiers when C10 ships;
  emission is never gated on existence (anti-zk).
- **C6-3**: no `Blocked by: 22` edge remains correct: C6-1 is expressible as "when 22's spans are
  available"; the definition-only fallback (option B) is the degradation, not a blocker.
- **C12-1**: write ticket 32's mapping from §3's table; the single required joint setting is
  `enableSymbols = false`; include rumdl's own three-toggle joint example (docs :2637-2642) as the
  "Traces absent" baseline and our table as the "Traces present" delta.
- **Cross-cluster flags**: (i) Cluster 4 must re-check Markdown Oxide CodeLens/semantic tokens
  (§0-3 contradicts A3's premise); (ii) Cluster 7's client matrix can adopt §1.3's four-editor
  documentLink rows verbatim, with the Helix released-vs-master distinction preserved; (iii)
  Cluster 3's C12 output should not re-litigate diagnostics (settled by 25) or completion (owned
  by 24).

## Sources

- LSP 3.18 spec, "Document Link Request" / "Document Link Resolve Request"
  (microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification; W).
- rumdl: `rvben/rumdl` `main` @ 0.3.0, `src/lsp/server.rs` (capabilities :481-550) (W);
  `docs/digests/lsp_rvben-rumdl-src-digest.txt` (:38447-38469 modules, :78974/:85936-85975 MD057)
  and `lsp_rvben-rumdl-docs-digest.txt` (:2367-2650 docs/lsp.md, :2472-2478 trigger chars,
  :2606-2608 settings, :2637-2642 joint setup) (V).
- MS markdown-language-service `src/languageFeatures/documentLinks.ts`
  (`lsp_microsoft-vscode-markdown-languageservice-digest.txt` :4045-4055 emission sets,
  :4517-4544 `#toValidDocumentLink` + tooltips) (V).
- Marksman dispatch (`lsp_artempyanykh-marksman-digest.txt` :3175/:3177) (V).
- zk (`zk-src-digest.txt` :3340-3463 DocumentLinks incl. WikiLink, :4256 capability,
  handler absence verified) (V).
- Markdown Oxide (`lsp_feel-ix-343-markdown-oxide-src-digest.txt` :882/:2116 code_lens,
  :2446/:3289 semantic tokens, zero document_link/folding) (V).
- Neovim `runtime/doc/lsp.txt` (v0.12.0 tag vs v0.10/v0.11 absence; stable 0.12.6 release) (W);
  `icholy/lsplinks.nvim` (W).
- Zed: `zed-industries/zed` #33587 → #56011 (closed 2026-05-26) (W).
- Helix: master `helix-term/src/commands.rs` (`goto_file_impl` document-link path,
  `resolve_document_link_request`); 25.07.1 tag (zero document-link code; latest release) (W).
- Template LSPs: `alex-oleshkevich/jinja-lsp` 0.4.0 (PyPI), `uros-5/jinja-lsp` (crates.io),
  `fjkit-lsp` (PyPI) READMEs (W).
- lsp-devtools documentLink capability matrix (VS Code 1.65.2 tooltipSupport/dynamicRegistration) (W).
- Tickets: 27 (Question, Inputs from 26), 22 (Q20(b), `research/22-template-intelligence.md`
  :542-561 seam), 15/25 (ranges/ownership), 32 (consumer), A3 decomposition (Cluster 3, grounding).

---

<a id="appendix-b4" />

## Appendix B4: B4: CodeLens reference counts above headings/notes (C7, Cluster 4)

## B4: CodeLens reference counts above headings/notes (C7, Cluster 4)

Research fan-out part B4 (cluster 4 of ticket 27's decomposition, `A3-decomposition.md:150-169,347-354`).
PLANNING ONLY: no production code touched. Every claim below is cited to a primary source
(upstream LSP 3.18 spec, server/client source code, this repo's digests, or the map's own tickets).

---

## 0. Headline finding: the cluster's premise is wrong

A3's C7 says: "do **any** Markdown/PKM servers do it (verify absence in Marksman/Markdown
Oxide/MS/rumdl digests, **currently zero evidence of presence**)". Re-verified against the digests:
**two of the four PKM/Markdown servers in the corpus ship reference-count CodeLens today**:
Marksman and Markdown Oxide, while the two *generic* ones (Microsoft's service, rumdl) plus zk do
not. The ticket's "informed by whether Marksman/Markdown Oxide do this" (issue 27:17) therefore has
a positive answer: **the PKM-native precedent exists and is on by default in both servers that have
it.** The absence claim in `A3-decomposition.md:154-155` (and the softer "Marksman/Markdown Oxide
CodeLens status = external-research agent's lane" at Appendix A1 of this file (:162))
should be corrected at consolidation.

---

## 1. Absence/presence checks (re-verified, not trusted from summaries)

`rg -ci "codelens|code lens"` over every digest in `docs/digests/` (a file with no printed count has
zero matches):

| Server | Result | Evidence |
|---|---|---|
| **Marksman** | **PRESENT**: capability, handler, implementation, tests, README bullet | `lsp_artempyanykh-marksman-digest.txt:11168` (`CodeLensProvider = Some { ResolveProvider = None }`), `:11999-12011` (handler), `:8158-8219` (`Lenses.fs`), `:15553-15651` (`LensesTests.fs`), `:1234` / docs digest `:256` ("✅ Code Lens with '# references' on headings") |
| **Markdown Oxide** | **PRESENT**: `codelens.rs` module, capability, handler, nvim + docs guidance | `lsp_feel-ix-343-markdown-oxide-src-digest.txt:864-946` (`src/codelens.rs`), `:2059-2061` (`code_lens_provider: Some(CodeLensOptions { resolve_provider: None })`), `:2116-2119`, `:2337-2360`; docs digest `:609` ("Inline reference counts for files and headings"), `:1596`, `:2199` |
| Microsoft `vscode-markdown-languageservice` | **ABSENT** (zero matches across the full-repo digest) | `docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt` (0 matches); claim corroborated by `research/tool-ms-markdown-language-service.md:15,41` |
| rumdl | **ABSENT** (zero matches in src, docs, tests, benches digests) | `lsp_rvben-rumdl-{src,docs,tests,benches}-digest.txt` (0 matches) |
| zk | **ABSENT** (zero matches in `zk-{src,docs,}-digest.txt`) | 0 matches |

So the generic-vs-PKM split is real: **no generic Markdown LSP ships CodeLens; both PKM LSPs that
were checked do.** Ticket 05's caveat (issue 27:10, one data point, not a ceiling) cuts the other
way here: the PKM side has *two* data points, both positive.

---

## 2. Upstream LSP 3.18 mechanics (primary: the spec online; local `docs/refs/lsp_spec.md:651` is only an unfetched include stub)

Fetched from `https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/`
(the "Code Lens", "Code Lens Resolve", "Code Lens Refresh" sections and `ClientCapabilities`):

- `textDocument/codeLens`: **params carry only `textDocument`, no range**: a request is
  *whole-document*, not visible-range. (Correction to `A1-codebase-review.md:161`, which says
  "CodeLens is pull-per-visible-range": that is a *client rendering* behavior, not the protocol
  shape. The server is asked for every lens in the file.)
- `CodeLens { range, command?, data? }`, with the normative note: *"A code lens is _unresolved_ when
  no command is associated to it. **For performance reasons the creation of a code lens and
  resolving should be done in two stages.**"* The doc comment's own example of a code lens is
  literally "the number of references". `range` *"should only span a single line"*.
- `CodeLensOptions { resolveProvider?: boolean }` (server capability); `codeLens/resolve` resolves a
  single lens. `CodeLensClientCapabilities.resolveSupport.properties` (client, since 3.18)
  declares which properties are resolved lazily; VS Code and Neovim both declare `['command']`.
- `workspace/codeLens/refresh` (since 3.16): server→client request; spec warns *"this event is
  global and will force the client to refresh all code lenses … should be used with absolute care"*;
  the client may delay recomputation for invisible editors. Client capability:
  `workspace.codeLens.refreshSupport`.

Implication: the protocol *designed* for exactly this feature to be lazy, and reference counts are
its canonical example. Eager computation is legal but is the deviation, not the default.

---

## 3. Mechanism precedents (server/client source, fetched live)

### 3.1 rust-analyzer: the canonical lazy reference-count lens

Source fetched from `github.com/rust-lang/rust-analyzer` (master, 2026-10):

- **Discovery is document-local and cheap**: `crates/ide/src/annotations.rs`: `annotations()` walks
  file definitions (`visit_file_defs`) and inserts `AnnotationKind::HasReferences { pos, data: None }`
  with **no count computed**; `resolve_annotation()` is where `find_all_refs` runs. Config gates per
  item kind.
- **LSP handlers** (`crates/rust-analyzer/src/handlers/request.rs:1687-1750`):
  `handle_code_lens` early-returns *"before any db query!"* when `lens_config.none()`
  (`:1694-1697`); `handle_code_lens_resolve` deserializes `CodeLensResolveData` and calls
  `resolve_annotation`.
- **Lens building** (`crates/rust-analyzer/src/lsp/to_proto.rs:1759-1911`): at discovery,
  `command` is `None` and `data` carries `{version, kind}`; at resolve, `command =
  show_references(reference_title(n), uri, position, locations)` with
  `reference_title` = "1 reference"/"N references" (`:2086-2088`). Note rust-analyzer does **not**
  suppress zero: a resolved lens with an empty location list titles "0 references".
- **Staleness guard** (`crates/rust-analyzer/src/lsp/from_proto.rs:121-155`): resolve data carries
  the file version; if `url_file_version(uri) != data.version` the resolve returns `None`, so the
  lens stays unresolved. This is issue 12's stale-result-discard applied per-resolve.
- **Refresh** (`crates/rust-analyzer/src/main_loop.rs:545-556`): when `became_quiescent ||
  state_changed`, it sends `CodeLensRefreshRequest` **gated on the client capability**
  (`crates/rust-analyzer/src/lsp/capabilities.rs:507-509`: reads
  `workspace.codeLens.refreshSupport`), alongside semantic-token/inlay/diagnostic refreshes.
- **Reference-count lens defaults are OFF** (`crates/rust-analyzer/src/config.rs:347-361`):
  `lens_references_adt_enable`, `lens_references_method_enable`,
  `lens_references_enumVariant_enable`, `lens_references_trait_enable` all default **`false`**
  (settings `rust-analyzer.lens.references.*`); `lens_implementations_enable` defaults `true`
  (`:342`). Master switch `lens_enable` default `true` (`:339`).

### 3.2 VS Code TypeScript: lazy resolve, viewport-only resolution, default OFF

Source: `microsoft/vscode` `extensions/typescript-language-features/src/languageFeatures/codeLens/`:

- `baseCodeLensProvider.ts`: discovery = one cached `navtree` request → referenceable name spans
  (no counts). Cancels/errors map to placeholder commands (`cancelledCommand` with empty title).
- `referencesCodeLens.ts`: `Config.enabled = 'referencesCodeLens.enabled'`, initialized **`false`**
  (`_enabled = ... (Config.enabled, false)`), so **reference-count CodeLens is opt-in in VS Code's
  own TypeScript support**; a `showOnAllFunctions` knob defaults `false` too. `resolveCodeLens`
  issues a `references` request per lens with `lowPriority: true` and `cancelOnResourceChange`,
  then labels "1 reference"/"N references"; zero → `command: ''` (renders, not clickable).
- `implementationsCodeLens.ts`: same shape, also default `false`.
- Registration re-fires `onDidChangeCodeLenses` when the setting changes
  (`changeEmitter.fire()`), i.e. config-change refresh is client-side; no `workspace/codeLens/refresh`
  needed for that case.

Client (`microsoft/vscode-languageserver-node` `client/src/common/codeLens.ts`):

- `:41-46` `fillClientCapabilities`: `textDocument.codeLens.dynamicRegistration = true`,
  `resolveSupport = { properties: ['command'] }`, `workspace.codeLens.refreshSupport = true`.
- `:48-54` on `CodeLensRefreshRequest`, it fires every provider's `onDidChangeCodeLensesEmitter`
  → the client re-requests `textDocument/codeLens`.

Editor scheduling (`microsoft/vscode` `src/vs/editor/contrib/codelens/browser/codelensController.ts`):

- `:55-56` provide and resolve each debounced at **min 250ms**.
- `:200-232` on model content change: dispose stale lens view-zones, `scheduler.schedule()`
  (re-provide), and **cancel pending/active resolve requests**.
- `:233-246` focus → schedule provide; blur → cancel; scroll/layout →
  `_resolveCodeLensesInViewportSoon()`**, and **counts are only resolved for lenses in the
  viewport**, while discovery is whole-document.

### 3.3 Java (VS Code Java / jdt.ls): same pattern, default OFF

Source: `redhat-developer/vscode-java` `package.json`:

- `java.referencesCodeLens.enabled` **default `false`** (`:1543-1549`);
  `java.referencesCodeLens.includeFields` default `false` (`:1550-1556`);
  `java.implementationCodeLens` enum `none|types|methods|all`, **default `"none"`**
  (`:1557-1574`).

### 3.4 `workspace/codeLens/refresh` in the wild

- **Sender**: rust-analyzer (§3.1, capability-gated, on state change/index quiescence), verified in
  source. Spec's own framing is "project-wide change" (config or index).
- **Handlers**: VS Code (`codeLens.ts:50-54`, fires provider change events); Neovim
  (`runtime/lua/vim/lsp/codelens.lua:434-446`, `M.on_refresh` re-requests per active provider);
  Zed supports it (issue history below), having *fixed* a bug where refresh did not re-query
  already-open docs (`zed-industries/zed#58864`, closed completed 2026-06-28).
- Marksman: **never sends it**: only the Ionide LSP library's default no-op exists
  (`lsp_artempyanykh-marksman-digest.txt:1667` `default __.WorkspaceCodeLensRefresh() = ignoreNotification`,
  no override anywhere). Markdown Oxide: zero `refresh` hits in its src digest; its docs instead
  tell nvim users to run `vim.lsp.codelens.refresh()` from `TextChanged/InsertLeave/CursorHold/BufEnter`
  autocmds (`lsp_feel-ix-343-markdown-oxide-docs-digest.txt:130-158`), i.e. **client-side polling
  instead of server push**.

---

## 4. The two Markdown/PKM implementations in detail

### 4.1 Marksman: eager counts, no resolve, zero-suppressed, headings + link definitions

`lsp_artempyanykh-marksman-digest.txt`, `Marksman/Lenses.fs` (`:8158-8219`):

- `forDoc` = `doc.Index.headings` **and** `doc.Index.linkDefs` (so also "[foo]: /url" definition
  lines get "2 references"), mapped through `buildReferenceLens`.
- `buildReferenceLens` calls `Dest.findElementRefs false folder doc el` **per element on every
  `textDocument/codeLens` request** (fully eager, `resolveProvider = None`), and returns `None`
  when `refCount = 0`, so this is **zero-count suppression**. Title via `humanRefCount`:
  "1 reference"/"N references".
- Range = the element's full CST range (heading span), single line.
- Command = `marksman.findReferences` with a **dummy server-side `workspace/executeCommand`
  handler** that returns `0` (`:12013-12018`): comment: *"Code lenses need an associated command…
  'showing references' need to be handled by the client and it's a bit too much hassle to do this
  for every client."* Location lists are attached to the command arguments **only** if the client
  advertises the non-standard experimental capability `codeLensFindReferences`
  (`:12101-12105`, `:8182-8194`).
- Cost evidence: Marksman's own BenchmarkDotNet harness (`:1080-1170`) measures a single
  `findElementRefs` at **9.6µs @10 docs, 50.7µs @50 docs, 339.5µs @250 docs** (folder of N docs each
  containing N links → 62,500 links at N=250). Linear in workspace size, computed once **per heading
  per request**: a document with 40 headings at the 250-doc size ≈ **13.6ms of eager counting per
  `codeLens` request**, and it scales linearly beyond. This is the concrete caution against naive
  per-element scans; Traces' inlink multimap design (33:41) is structurally better.
- Capability is advertised **unconditionally** (`:11168`), unlike workspace/document symbols, which
  Marksman gates on `not clientDesc.IsVSCode` (`:11147-11150`).

### 4.2 Markdown Oxide: eager counts + precomputed locations, no resolve, file/heading/block/footnote

`lsp_feel-ix-343-markdown-oxide-src-digest.txt`, `src/codelens.rs` (`:864-946`):

- `code_lens(vault, path, params)`: for every `Referenceable` in the document
  (`vault.select_referenceable_nodes(Some(path))`) compute
  `select_references_for_referenceable(...)`, eager, `resolve_provider: None` (`:2059-2061`).
- Attach points: `Referenceable::File` → range `(0,0)-(0,1)` (top of file, title
  "N references to file"); headings / indexed blocks / footnotes → `referenceable.get_range()`
  (docs digest `:2199`: "references code lens for files at the top of the file, headings, indexed
  blocks, footnotes, ...").
- **Zero-count suppressed** (`.filter(|(_, references)| !references.is_empty())`).
- Command `moxide.findReferences` with **every reference location pre-serialized into the command
  arguments** (`FindReferencesData { uri, position, locations }`), so the response payload is
  O(total references) across all lenses. VS Code handles the command client-side via
  `editor.action.peekLocations`; other editors (kakoune-lsp) send it back through
  `workspace/executeCommand`, where the server opens the **first** location via `window/showDocument`
  (`:2337-2360`).
- No server-side refresh; nvim users are told to drive `vim.lsp.codelens.refresh()` themselves
  (docs digest `:130-158`).

**Design takeaways from both PKM peers**: eager + `resolveProvider: false` + zero-suppression +
"1 reference/N references" titles + a command that must exist for every lens (client-handled), with
the *real* navigation payload either client-side (peekLocations) or a server `executeCommand`
fallback. Both avoid `workspace/codeLens/refresh` and compensate client-side. Both are cheap only
because their workspaces are small or their scans are fast; Marksman's own bench shows the linear
wall.

---

## 5. Client support matrix (for C11; VS Code + Neovim + Zed first-class, Helix floor, map.md:44)

| Client | Renders CodeLens | `codeLens/resolve` | `workspace/codeLens/refresh` | Notes / citations |
|---|---|---|---|---|
| **VS Code** | yes | yes (viewport-only) | yes (advertises `refreshSupport = true`, handles by re-requesting) | `client/src/common/codeLens.ts:41-54`; provide/resolve 250ms debounce + content-change invalidation `codelensController.ts:55-56,200-246` |
| **Neovim** | yes (`vim.lsp.codelens`, virtual text above the line) | yes, **resolves every unresolved lens at display time**, with stale guards (buffer version + lens identity) | yes, `vim.lsp.codelens.on_refresh` registered handler | `runtime/lua/vim/lsp/codelens.lua:130-157,195-211,434-446`; default caps `runtime/lua/vim/lsp/protocol.lua:464-469` (`resolveSupport.properties={'command'}`), `:637-639` (`workspace.codeLens.refreshSupport=true`) |
| **Zed** | **yes (since ~Apr 2026)** | yes (fixed remote resolve bug 2026-06) | yes (bug "refresh doesn't re-query open docs" closed completed 2026-06-28) | Feature request `zed#11565` "LSP textDocument/codeLens: CodeLens with count of reference/implementations" closed 2026-04-22; `zed#58864` (refresh) closed 2026-06-28; rendering/folding fixes `zed#63673`, PR "Hide code lens' blocks in folds"; jdt.ls lens click-through PR #64101 (closed) |
| **Helix** | **no** | no | no | `book/src/lsp.md` has zero "code lens" mentions; PR #5063 "feat(lsp): code lenses" **open since 2022-12-08, not merged** (updated 2026-09-29); issue #7957 "gopls code lenses unsupported?" open |

Sparse floor: **Helix will not display any lens**: a feature depending on CodeLens for correctness
or discoverability is invisible there. Zed (primary target) support is new (<6 months) with a
recent bug history around refresh/dynamic registration, so favor **static** advertisement (feeds
C13/29) over post-init dynamic registration (`zed#58864` documented the dynamic-registration
re-query failure mode).

---

## 6. Analysis

### 6.1 Eager vs lazy count computation (the resolve round-trip)

Three live shapes, all with primary sources:

1. **Lazy (rust-analyzer, VS Code TS)**: discovery is document-local (syntax/AST walk, cached
   navtree), counts computed in `codeLens/resolve`. Protocol-preferred ("two stages … for
   performance reasons"). Costs arrive *per lens, on demand, viewport-first in VS Code*, each
   individually stale-checkable, the right fit for issue 12's "cancellation = stale-result
   discard; mid-request interruption impossible" model, because a resolve is a *small* unit of work
   that either fits its budget or is discarded when the buffer version moves.
2. **Eager (Marksman, Markdown Oxide)**: counts + zero-suppression at discovery,
   `resolveProvider: None`. One request, exact titles, no resolve chatter; but the full
   cross-file count cost lands inside a single `textDocument/codeLens` handler that, under
   `concurrency_level(1)` sequential dispatch (issue 12:20-30), **blocks hover/completion for its
   whole duration** (`A1-codebase-review.md:182`). Marksman's own benchmark is the receipt for how
   that grows.
3. **Client-side refresh instead of server push (both PKM peers + moxide's nvim docs)**: never
   send `workspace/codeLens/refresh`; let the client re-request on its own edit/focus triggers or
   autocmds. Accepts staleness for *cross-file* count changes (the common case for backlinks!).

Two protocol facts that constrain the design:

- **Lazy cannot cleanly suppress zero counts.** The lens list exists before counts are known. A
  resolved lens must return *some* command title or Neovim will re-resolve it forever
  (`codelens.lua:201-211` re-enters `resolve` whenever `not lens.command`), and VS Code simply won't
  render a command-less lens. So "suppress 0 (and 1)", which both PKM peers do, **requires eager
  counts**, or it requires displaying "0 references" (rust-analyzer/TS behavior).
- **`textDocument/codeLens` is whole-document** (no range param), so "only compute for what's
  visible" is impossible server-side; lazy resolve is the only lever that approximates it (and VS
  Code does exactly that).

### 6.2 Refresh notification discipline

Reference counts are invalidated by **other files' edits**: the exact case client-side edit
triggers cannot see (VS Code's content-change re-provide only fires for the open document,
`codelensController.ts:200-232`). Options, in increasing fidelity/cost:

- **None (PKM-peer baseline)**: rely on client re-request on edit/focus/close-reopen, plus nvim
  autocmds (moxide pattern). Cross-file counts go stale until the user edits the document. Honest
  and shippable; matches both existing Markdown implementations.
- **Capability-gated, coalesced push**: send `workspace/codeLens/refresh` only when
  `workspace.codeLens.refreshSupport` is advertised and an index swap actually changed inlink
  state, i.e. piggyback on 13/T1's 100ms coalesced refresh (map.md:32) rather than per-file,
  honoring the spec's "absolute care / global event" warning. rust-analyzer's trigger
  (`became_quiescent || state_changed`, `main_loop.rs:545-556`) is the template: refresh when the
  workspace settles, not per keystroke.
- **Never on the hot lane**: refresh is an outbound request; under tower-lsp-server it must be sent
  from the background/publisher lane (25's β lane idiom), never from inside a handler.

### 6.3 Per-heading attach points and thresholds

- **Which nodes**: Marksman = every heading with ≥1 ref **+ link definitions**; moxide = file (line
  0) + headings + indexed blocks + footnotes; rust-analyzer = item name ranges with per-kind gates.
  For Traces: headings are the only anchor kind that exists in the AST today (`Heading { level,
  text, line }`, `src/note/heading.rs:10-14`, line-only, cited `A1-codebase-review.md:15`);
  file-level counts exist *today* via `InlinkMap`/`FileEntry::inlinks()`
  (`src/index/inlinks.rs:59`, `src/index/entry.rs:167-170`, `A1-codebase-review.md:51`).
- **Range shape**: spec wants a single-line range. Heading line numbers are already available, so a
  CodeLens range `(line-1, 0)-(line-1, 0)` (LSP 0-indexed from `SourceLine`'s 1-indexed) ships
  **without waiting on ticket 11's byte spans**, degraded `character` values only, same degradation
  class C1's symbols accept (`A1-codebase-review.md:127`). Do not block CodeLens on spans.
- **Levels**: no precedent filters by heading level (Marksman/moxide attach to *all* headings).
  Level filtering would be a Traces-specific noise knob; counts are cheap per heading once the
  inlink neighborhood is resident, so level filtering is a *clutter* decision, not a cost decision.
- **Thresholds**: suppression of 0 (both PKM peers) and optionally of 1 is a clutter decision that,
  per §6.1, forces eager counting. A cheap alternative: **count cap on display**: titles like
  "999+ references" (bounded work and bounded payload), consistent with keeping 26's 5,000
  references ceiling deliberately separate from any CodeLens budget
  (issue 27:25; `A1-codebase-review.md:161`).

### 6.4 Cost model under 33's budgets, with no mid-request cancellation

Budget context (issue 33 §3, rows at `:49-55`, plus `:41`): **no CodeLens row exists**: A3:27-28
requires this cluster to propose one. Related rows: Document symbols <10ms/<5ms; Find-refs
<20ms (≤5,000 results); **Backlink recomputation <5ms, O(link-count) not O(vault-size)**
(`33-performance-targets-and-strategy.md:41`). Issue 12: `concurrency_level(1)`, handlers run
inline, cancellation = stale-result discard, *"with N=1 and <100ms handler budgets, there is no
mid-execution cancellation"* (`12-concurrency-and-cancellation-model.md:30-38`).

Component costs:

- **Discovery (per request)**: walk `Note::headings()`, emit one lens each, document-local, same
  class as document symbols. *Proposed row: <10ms p95 (derive from Document symbols), stretch <5ms.*
- **Count, file-level** (today's data): `FileEntry::inlinks().len()`, resident multimap lookup,
  effectively free; consistent with 33:41.
- **Count, per-heading** (needs 26's occurrence machinery + 15's anchor matcher): for target file
  F, count anchor occurrences resolving to each heading across F's inlink sources:
  O(Σ inlink sources' anchor occurrences) = **O(neighborhood link count)**, which is exactly 33:41's
  shape, *provided* counts are read from a maintained inlink/anchor multimap (26:25 pins that
  "27's counts consume [nearest-picked] edges") and **not** by re-scanning the workspace per
  heading (Marksman's shape: 339µs × headings at 62.5k links, §4.1).
  *Proposed row: resolve <5ms p95 per lens (count-only), stretch <1ms; eager-all variant ≤20ms
  (find-refs class) only if the feature defaults on.*
- **Payload**: count-only lens data (`{uri, version}` for staleness, à la rust-analyzer's
  `CodeLensResolveData`) is O(headings). Copying moxide's precomputed-`locations`-in-arguments shape
  is O(total references) and re-imports the ceiling problem 27's inputs deliberately scoped to the
  references path (27:25).
- **Worst case**: hub/MOC notes with hundreds of inlinks × hundreds of headings. Eager-all is the
  only shape that can blow a handler budget under N=1; lazy resolve bounds each unit of work and
  lets a slow one be discarded when the version moves. Cold-file serving: counts for a
  scrolled-into-view *closed* file ride 11's `ContentResolver::text_of` re-parse path
  (`A1-codebase-review.md:118,183`), same cost class as other range-serving 27 features.
- **Overlay (13/14)**: counts must be overlay-adjusted for the open document (14's per-request
  inlink adjustment, `A1-codebase-review.md:61`), or the lens will disagree with hover/go-to-def on
  the same link, a correctness requirement, not an optimization.

### 6.5 Config / gating options (precedents)

| Server | Setting | Default |
|---|---|---|
| rust-analyzer | `rust-analyzer.lens.enable`; `…lens.references.{adt,method,trait,enumVariant}.enable` | master `true`, **each references variant `false`** (`config.rs:339-361`) |
| VS Code TS | `typescript.referencesCodeLens.enabled`, `…showOnAllFunctions` | **`false`** |
| VS Code Java | `java.referencesCodeLens.enabled`, `…includeFields`, `java.implementationCodeLens` | **`false`**, `false`, `"none"` |
| Marksman | none | always advertised |
| Markdown Oxide | none (server-side) | always advertised; nvim display opt-in via user autocmds |

**Every general-purpose implementation with a setting defaults reference counts OFF; both PKM
implementations have no setting and default them ON.** rust-analyzer additionally short-circuits
the handler "before any db query" when the lens config is empty
(`request.rs:1694-1697`), the gating idiom to copy for Traces' config predicate (feeds 29/31).

Coexistence: rumdl has zero CodeLens code (§1), so **no rumdl toggle needed** for this feature
(different from symbols/documentLink in C1/C5). VS Code's built-in Markdown support runs Microsoft's
service, which also has none (§1) → no built-in-provider overlap either.

### 6.6 Honest value assessment for prose documents

**For:**

- Both PKM LSPs market it: moxide docs call "Inline reference counts for files and headings" a
  feature and its changelog bills "a code lens with the count of references to headings and files…
  more easily [view] information on backlinks" (docs digest `:609`, `:1596`); Marksman ships it as
  a README bullet (`:1234`). User demand in editors is real: Zed's request for
  reference/implementation count lenses (`zed#11565`) ran 2024→2026 and closed 2026-04-22.
- The count is genuinely novel information in an editor: "which/many notes point at this section"
  is not exposed by syntax highlighting, outline, or hover (hover shows target info *at the link*,
  not inlinks *at the heading*).
- Traces has the semantics already pinned: counts read the nearest-picked inlink graph (26:25), so
  the number means something specific and consistent with rename/rewrite targeting.

**Against:**

- **The PKM-native surface for this exact question already exists and is richer**: Obsidian's
  Backlinks core plugin shows, per active note, *Linked mentions* and *Unlinked mentions* with
  context, filtering, sorting, and an optional in-document backlinks section at the bottom of the
  note (Obsidian help, `obsidian.md/help/plugins/backlinks` / `obsidianmd/obsidian-help
  en/Plugins/Backlinks.md`). It is a *sidebar panel with context*, not an inline per-heading
  number, i.e. the PKM convention answers "what links here?" with a list, not a count. Per-heading
  counts only beat that panel for hub/MOC/structure notes where you're auditing section wiring; for
  prose reading they are visual noise above every heading (CodeLens adds a full line per lens).
- **The generic bar omits it**: Microsoft's canonical service, rumdl, and zk all have none (§1),
  and every settings-bearing general-purpose implementation defaults it **off** (§6.5): the
  ecosystem's revealed preference is "ship it, don't show it by default".
- It is **display-only**: it adds no capability find-references (26, <20ms row) or a future
  backlinks panel doesn't already provide; hover/`textDocument/references` already answer the
  underlying question on demand.
- Per-heading counts are **knowingly best-effort**: stem-tie er-notes produce no inlink edge today
  (26:31, `A1-codebase-review.md:160`), so a displayed "2 references" can under-count ambiguous
  targets, a correctness wart users will read as a fact.
- Cost lands on the one lane where Traces has no slack: N=1 sequential dispatch with no mid-request
  cancellation (12), inside the reuse-vs-performance tension (map.md:34), for a feature with no
  latency row in 33 yet.

**Who actually uses it**: the honest answer from the evidence is *IDE users on symbol-heavy code*
(default-off everywhere) and *PKM users on hub/index notes* (default-on in the two PKM LSPs). The
prose-document marginal reader is not the beneficiary.

---

## 7. Options for ticket 27 (value vs cost)

**Option A: Lazy, count-only, default-off (recommended if in scope).**
`codeLensProvider { resolveProvider: true }`; discovery = document-local heading walk emitting
line-granularity ranges (no span dependency) + one file-level lens at line 0; `data = {uri, version}`
with rust-analyzer's stale-version guard at resolve; resolve = count from the resident
nearest-picked inlink/anchor multimap, count-only payload, per-lens cap ("999+"), always return a
command (avoid Neovim's command-less re-resolve loop); refresh = capability-gated and coalesced
with T1's index-swap events (or none in v1, PKM-peer baseline); config `[lsp.codeLens] references`
default **false** with the rust-analyzer-style early return. *Why*: honors the spec's two-stage
design, bounds each unit of work under no-cancellation dispatch, matches every generalist's
default, costs nothing when off. *Cost*: zero-count suppression impossible without eager counts, so
the lens shows "0 references" when enabled (or accept that and skip suppression).

**Option B: Eager, zero-suppressed, `resolveProvider: false` (PKM-peer shape).**
Counts computed during discovery from the resident multimap (NOT per-element workspace scans,
Marksman's 339µs×headings receipt), suppress 0 (and optionally ≤1), one round trip, exact titles,
no refresh strictly required (client re-request covers open-doc edits). *Why*: simplest client
behavior, both PKM peers do it, cheapest total work when the multimap makes counts O(neighborhood).
*Cost*: whole count cost inside one N=1 handler, so it **requires a proposed 33 row (≤10-20ms) and
a measured hub-note worst case before committing**; per-heading multimap maintenance then becomes a
T1 obligation ("<5ms backlink recomputation" must include heading-anchor edges, an index-shape
decision touching 13/41).

**Option C: File-level only, v1 ("3 references" to the *note*).**
Use today's `InlinkMap`/`FileEntry::inlinks()` (`inlinks.rs:59`), moxide's file-lens convention
(range at line 0), defer per-heading lenses until 26's occurrence machinery + 15's anchor matcher
exist and are measured. *Why*: **the leanest performant approach that stays maximally useful**:
resident data, O(inlinks), no nav-spine dependency, no span dependency, still answers the dominant
PKM question ("how many notes link here?"), leaves an honest upgrade path. *Cost*: loses the
per-heading novelty that is the ticket's headline example ("3 references" above a heading).

**Option D: Out of scope (legitimate, evidence-backed).**
No CodeLens at all; document the decision as: the only 27 bullet with **zero generic-Markdown
precedent** (MS/rumdl/zk), default-off in every generalist that has it, display-only, per-heading
counts best-effort under 26's ambiguity semantics, invisible on the Helix floor, a fresh/buggy
surface on Zed pre-2026, and competing with Obsidian's backlinks-panel convention for the same
user need. Revisit when (a) a backlinks panel/hover surfaces inlink data anyway, (b) 33 has
measured count costs, or (c) a user-facing hub-note auditing workflow demands inline counts. This
frees 27's latency budget for folding/selection/documentLink/semantic tokens, which *do* sit on the
generic bar (issue 27:10-19).

Sequencing note regardless of option: CodeLens is the *last* of the eight bullets to need 11's byte
spans (line-only ranges suffice, §6.3) but the *first* to need 26's spine + 14's overlay-aware
inlinks for per-heading correctness, so Option C is also the only variant buildable before those
land.

---

## Lessons for ticket 27 (concrete)

1. **Correct the premise**: Marksman (`Lenses.fs`, digest `:8158-8219`) and Markdown Oxide
   (`src/codelens.rs`, digest `:864-946`) both ship reference-count CodeLens. Replace
   `A3-decomposition.md:154-155`'s "zero evidence of presence" and `A1:162`'s "external-research
   agent's lane" with these citations; the ticket 05 caveat (27:10) now cuts *toward* inclusion for
   PKM, *away* for generic parity (MS/rumdl/zk all absent).
2. **Decide eager vs lazy explicitly, knowing the zero-suppression coupling**: lazy resolve cannot
   hide zero counts (Neovim re-resolves command-less lenses forever; VS Code won't render them).
   "Suppress 0/1" ⇒ eager counts ⇒ a mandatory 33 latency row + hub-note measurement before
   default-on.
3. **Propose 33 rows** (A3:27-28 obligation): CodeLens discovery <10ms p95 (derive from Document
   symbols), count resolve <5ms p95/lens from resident multimap (stretch <1ms), eager-all variant
   ≤20ms (find-refs class), and keep CodeLens's own cap decision consciously separate from 26's
   5,000 references ceiling (27:25).
4. **Never copy moxide's payload shape** (all locations pre-serialized per lens); count-only
   `data = {uri, version}` + rust-analyzer's stale-version resolve guard is the pattern that fits
   12's stale-result-discard model.
5. **Attach points**: all heading levels (both PKM peers do; no precedent filters) + one file-level
   lens at line 0; ranges can ship at **line granularity today** (`Heading.line`,
   `src/note/heading.rs:10-14`), so do not gate on ticket 11's spans.
6. **Counts must read 26's nearest-picked inlink graph** (26:25, "27's counts"), be overlay-adjusted
   via 14's inlink reconciliation, and be documented as best-effort for stem-tie ambiguity (26:31),
   same honesty as `A1:160`.
7. **Refresh**: default to *none* (both PKM peers manage client-side) or capability-gated,
   T1-coalesced `workspace/codeLens/refresh` on settled index swaps (rust-analyzer template,
   `main_loop.rs:545-556` gated by `capabilities.rs:507`); never per-keystroke; spec says "absolute
   care" (global event).
8. **Config**: gate with an early return before any index work (rust-analyzer
   `request.rs:1694-1697` idiom); default follows the generalist precedent (**off**) if the
   feature is lazy, or the PKM-peer precedent (**on**) only if Option B's measured budget holds;
   feed the predicate to 29/31 (dynamic registration also enables rumdl-joint-setup toggling, but
   prefer **static** advertisement given Zed's dynamic-registration re-query bug history).
9. **Client matrix input for C11**: VS Code (yes, viewport-resolve, refresh-capable), Neovim (yes,
   eager resolve-at-display, refresh handler), Zed (yes since 2026-04, refresh fixed 2026-06),
   Helix (**no**, PR open since 2022), so CodeLens can never carry information available *only*
   there; it must be supplementary to hover/references/backlinks.
10. **rumdl coexistence (C12)**: no overlap: zero CodeLens code in rumdl's digests; no toggle
    needed, say so explicitly in the 32 boundary table.

---

## Sources

**Local corpus / map**
- Appendix A3 of this file (C7, Cluster 4, grounding header)
- Appendix A1 of this file (:51, :61, :118, :157-162, :180-184)
- `.scratch/md-pkm-lsp/issues/27-…md:10-26`; `map.md:21-34,44`; `issues/26-…md:25,31`;
  `issues/33-…md:41,45-55,117-131`; `issues/12-…md:20-38`
- `research/tool-ms-markdown-language-service.md:15,41`; `research/tool-marksman.md:14`;
  `research/tool-markdown-oxide.md:16,18`
- Digests: `lsp_artempyanykh-marksman-{digest,docs-digest}.txt`,
  `lsp_feel-ix-343-markdown-oxide-{src,docs}-digest.txt`,
  `lsp_microsoft-vscode-markdown-languageservice-digest.txt`, `lsp_rvben-rumdl-*`, `zk-*`

**Upstream spec (fetched)**: LSP 3.18 spec, sections Code Lens / Code Lens Resolve / Code Lens
Refresh + `ClientCapabilities`: `microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/`

**Server source (fetched raw)**
- rust-analyzer: `crates/ide/src/annotations.rs`;
  `crates/rust-analyzer/src/handlers/request.rs:1687-1750`;
  `crates/rust-analyzer/src/lsp/to_proto.rs:1759-1911,2086-2088`;
  `crates/rust-analyzer/src/lsp/from_proto.rs:121-155`;
  `crates/rust-analyzer/src/main_loop.rs:545-556`; `crates/rust-analyzer/src/lsp/capabilities.rs:502-519`;
  `crates/rust-analyzer/src/config.rs:335-367,1541-1627,2707-2727`
- VS Code: `extensions/typescript-language-features/src/languageFeatures/codeLens/{baseCodeLensProvider,referencesCodeLens,implementationsCodeLens}.ts`;
  `src/vs/editor/contrib/codelens/browser/codelensController.ts:55-56,200-246`
- vscode-languageserver-node: `client/src/common/codeLens.ts:41-54`
- vscode-java: `package.json:1543-1574`
- Neovim: `runtime/lua/vim/lsp/codelens.lua:130-157,195-211,434-446,493-501`;
  `runtime/lua/vim/lsp/protocol.lua:464-469,637-639`
- Helix: `book/src/lsp.md` (no code lens), PR #5063, issue #7957
- Zed: issues #11565 (closed 2026-04-22), #58864 (closed 2026-06-28), #63673, PR #64101

**Docs**: Obsidian help, Backlinks core plugin (`obsidian.md/help/plugins/backlinks`;
`github.com/obsidianmd/obsidian-help` `en/Plugins/Backlinks.md`)

---

<a id="appendix-b5" />

## Appendix B5: Research: 27 Cluster 5 — Code actions (C8 link conversion, C9 extract-to-note)

## Research: 27 Cluster 5 — Code actions (C8 link conversion, C9 extract-to-note)

**Date**: 2026-10-18. **Status**: research input for the grilling on
[issues/27-structural-symbols-and-editor-intelligence.md](../issues/27-structural-symbols-and-editor-intelligence.md),
decomposed as C8/C9 in Appendix A3 of this file (lines 171–204, 356–363).
**Method**: local corpus mining (digests, research/24–26, A1), upstream LSP 3.18 spec fetched this
session, primary web precedent (IWE, Pylance, VS Code docs) fetched this session, earlier-session
precedent surveys marked [G·].

**Evidence markers**: **[S]** = LSP 3.18 spec text fetched this session (upstream
microsoft.github.io/language-server-protocol; the local `docs/refs/lsp_spec.md` has unmaterialized
stubs for codeAction — cannot be cited); **[D f:l]** = local digest (file:line); **[G]** = primary
upstream fetched this session; **[G·]** = primary source named but fetched earlier in this research
task (2026-10 cluster fan-out); **[R f:l]** = local `research/`/`issues/`/`A*` file; **[?]** = memory
only, UNVERIFIED.

---

## §0 Findings at a glance

1. **No LSP does wikilink ↔ Markdown conversion as a code action.** Verified absent in MS
   markdown-language-service (only two code-action providers, both link-*definition* hygiene
   [D :1752-1753]), Marksman (TOC + create-missing-file [D :11943-11955]), Markdown Oxide
   (create-unresolved-file only [D :709-830]), rumdl (lint fixes only [D :38285]), zk
   (link-format config governs *generation*, not conversion [D :1678-1700]). Conversion is
   editor-plugin/CLI territory ([G·] mii-key/obsidian-links, Wikilinks-to-MDLinks,
   obsidian-links-converter, Better-Markdown-Links, olconv, Foam fork) — novelty of C8 confirmed;
   correctness hinges on 26's refusal matrix, not edit mechanics (A3:184).
2. **Closest LSP precedent for C9 is IWE's "link" code action** ([G] iwe.md/docs/code-actions/link/):
   create note from word/selection, replace text with link, template-controlled key
   (`{{id}}`/`{{slug}}`/`{{title}}`/`{{today}}`), Markdown|WikiLink output, numeric collision
   suffixes. Obsidian Note Composer/Note Refactor ([G·]) cover the PKM-native shape incl. the
   telling detail that Note Refactor Plus ships an *explicit undo command* — clients do not
   auto-reverse file creation.
3. **Transport**: spec makes `CodeAction.data` opaque passthrough guaranteed since 3.16 [S] —
   25 already adopted this as the lazy quick-fix channel (25:136). All Markdown-LSP precedent is
   *eager* (full edit in the codeAction response; Marksman advertises `ResolveProvider = Some false`
   [D :11136]); only TS-server's two-phase move-to-new-file [G·] matches resolve-on-demand. Eager is
   fine when cheap (rumdl re-lints per request [D :38285]); C8 is trivially eager (one TextEdit);
   C9's create-file edit is the only candidate for resolve.
4. **C9 is the highest-risk action** (writes files). Spec + 26 pin the mechanics: ordered
   `documentChanges` (resource ops only legal there [S]; Neovim never advertises it but applies it —
   26:81, two-tier serializer 26:213 B2), `failureHandling: abort` leaves pre-failure ops executed
   [S] so **pre-validation is mandatory, no client rolls back** (26:83-85), gate on `create` ∈
   `resourceOperations` (27 inputs from 26; N6 26:240; A1:167 — one shared predicate with 25),
   `isRefactoring` metadata buys VS Code's preview/confirm (26:85).
5. **From-template creation variant stays deferred** — map.md:101 deferred it pending 22+28; 22
   resolved 2026-09-30, **only 28 remains** (open, unblocked by 13/26). Confirmed: deferral stands.
6. **Ownership seams** (A1:166-168, 23:47): status-cycle code action = 23's invariant (cycle order =
   config insertion order = completion sort_text order), **27 owns plumbing**; "steps over symbols
   vs kinds" is 27's open sub-question; create-note offering side checks `create` (shared with 25);
   extract-to-note is new work touching 28 (file-ops boundary).
7. **33 has no code-action latency rows** — cluster proposes (§4), 33 prices.

---

## §1 Transport: LSP 3.18 codeAction machinery + precedent matrix

### 1.1 Spec facts [S]

- `textDocument/codeAction` request carries `CodeActionContext`: `diagnostics` (the pull-diagnostics
  set the client currently shows), `only` (client/preference kind filter — VS Code sends the
  lightbulb menu's requested kinds), `triggerKind` (1 = Invoked, 2 = Automatic — the lightbulb's
  passive scan). Servers may return `Command` or `CodeAction` literals.
- `CodeAction`: `title` (required), `kind` (hierarchical string, `quickfix`/`refactor`/`source` roots
  plus dotted children), `diagnostics`, `edit` (WorkspaceEdit), `command`, `isPreferred`,
  `disabled` ({reason} — action offered but greyed with explanation), `data` (opaque to client,
  returned verbatim on `codeAction/resolve`; servers "shouldn't alter existing attributes" when
  resolving), `tags` (3.18: `Empty`, `Deprecated`).
- Registration `CodeActionOptions`: `codeActionKinds`, `resolveProvider`. Client capabilities:
  `codeActionLiteralSupport`, `resolveSupport`/`dataSupport`, `isPreferredSupport`,
  `honorsChangeAnnotations`, `disabledSupport`, `tagSupport`.
- `WorkspaceEdit`: `changes` (map uri→TextEdit[], *cannot* carry resource ops) vs `documentChanges`
  (ordered array; resource ops — CreateFile/RenameFile/DeleteFile — legal only here). The sequence is
  **ordered and atomic-on-invalid**: an invalid operation fails the whole edit. `failureHandling`
  (abort/reportsFailure/undo): **abort = operations before the failure stay executed** — the spec's
  own recovery text concedes partial application is client-defined.
- `CreateFile`: options `overwrite` / `ignoreIfExists`, `annotationId`. No content payload — a
  created file must then be populated by a following `TextDocumentEdit` in the same sequence.

### 1.2 25's adopted model (cluster input)

- Pull diagnostics with `identifier: "traces"`, dotted `<feature>.<rule>` codes, `enum
  DiagnosticRule` registry (25 research §5.4).
- **`Diagnostic.data` is the spec-guaranteed lazy quick-fix channel** (survives publish→codeAction
  since 3.16; 25:136-137) — 26 N6 (26:240) hands the create-note offering side to 27/25 as a shared
  predicate; A1:167 confirms no capability plumbing exists yet (29 owns advertisement).
- BrokenLink side-channel (25:788, A1:68): shared parse returns `(Note, Vec<BrokenLinkRef>)`,
  sole production caller `src/index/service.rs:276` unaffected — the substrate "convert link /
  create note" actions read.

### 1.3 Precedent matrix (how real servers wire codeAction)

| Server | Kinds offered | Eager / resolve | Edit shape | Notable detail |
|---|---|---|---|---|
| MS markdown-ls | `refactor.extract.linkDefinition` [D :7589], remove-def quickfix [D :7746] | **eager** (full edit in response) | TextEdit[] via in-house `WorkspaceEditBuilder` [D :18329] | Disabled-action-with-reason pattern ("Not on link" [D :7595], "Link is already a reference" [D :7603]); `organizeLinkDefinitions` exposed as a *service method*, not a code action [D :1776, :5243]; only two providers registered [D :1752-1753] |
| Marksman | source (TOC), quickfix (create-missing-file) [D :11943-11955] | **eager**, `ResolveProvider = Some false` [D :11136] | TOC: single-doc `changes` map; create: **CreateFile-only** WorkspaceEdit [D :4264-4265] | Per-action config toggles `code_action.toc.enable`, `code_action.create_missing_file.enable` [D :5499-5506]; routes `codeAction/resolve` method but doesn't advertise resolveProvider [D :11136] |
| Markdown Oxide | "Create Unresolved File" [D :709-830] | eager, `CodeActionProviderCapability::Simple(true)` [D :2049] | Ordered `DocumentChanges::Operations`: CreateFile (`ignore_if_exists`, no overwrite [D :822]) then `TextDocumentEdit` with `version: None` appending a heading [D :825-831] | Destination routed to `daily_notes_folder` for formatted filenames else `new_file_folder_path` [D :756-762]; snippets emitted ungated (rejected pattern per 24) |
| rumdl | per-warning quickfix + `source.fixAll.rumdl` [D :38385-38387] | **eager — re-runs the whole lint per codeAction request** [D :38285] | TextEdit | `is_preferred: Some(true)` on fixAll [D :38407]; range-overlap matching on warning span |
| TS server | `refactor.move.newFile` [G·] | **two-phase**: cheap action list → expensive `getEditsForAction` (≈ codeAction + resolve) | new-file edits + `renameFilename`/`renameLocation` putting user into rename on the new name; `allowTextChangesInNewFiles` client preference gates new-file edits (old editors broke) — capability-gating precedent | |
| Pylance | Move symbol to new file / to…, Extract Method/Variable, Convert import to relative/absolute (+ "Convert all…" variants) [G] | client-extension, eager | new file named after symbol **in the same directory as source** [G] | Refactor Preview checkboxes + Apply/Discard (Rename Module) [G] |
| pylsp+rope | `refactor.*` incl. rename-module emitting `CreateFile` + importer edits [G·] | resolve | documentChanges w/ CreateFile | only Python LSP emitting CreateFile end-to-end (secondary digest, o2-scalpel design notes) |

**Takeaways for Traces.**
(a) Lazy `data` is only worth it where computation is expensive; C8 (one TextEdit from in-memory
resolution) is eager; C9 (name generation + collision scan + new-note skeleton) is the sole
resolve candidate — and even IWE/Obsidian-adjacent precedent computes eagerly. A resolve-capable
registration costs nothing to advertise later, but v1 can honestly ship `resolveProvider: false`
(Marksman pattern) if C9's edit stays cheap.
(b) The `data` channel *is* needed for one thing regardless: quickfix actions derived from
diagnostics (create-note from a broken-wikilink diagnostic) — 25's channel, spec-guaranteed.
(c) MS's disabled-action-with-reason is the spec-blessed way to say "the action exists but not
here" — an alternative to silent refusal at 26's refusal positions (§2.4).
(d) `context.triggerKind == Automatic` should skip expensive action construction (the spec's own
lightbulb-scan affordance) — perf lever for §4.
(e) rumdl's eager re-lint-per-request is the anti-pattern floor: Traces' index-resident resolution
makes eager C8 strictly cheaper.

### 1.4 Serializer + capability gates (carry-over, restated for this cluster)

- Two-tier rule from 26:213 B2: emit `documentChanges` **iff advertised OR resource ops present**
  (rust-analyzer-proven Neovim deviation — Neovim never advertises `documentChanges` but applies it,
  26:81); gate *offering* on `resourceOperations` alone.
- Version policy (26:83): `Some(current)` for open docs, `null` for closed (disk-master; never 0).
  VS Code rejects the **entire** edit set on version mismatch for open docs; Neovim soft-skips the
  first mismatched doc (**partial rename, no rollback**). Pre-validation mandatory (26:85).
- `isRefactoring` metadata on multi-file cascades → VS Code preview/confirm; Neovim users get no
  rollback, the preview is their only safety (26:85). C9 should set it unconditionally.
- Change annotations skipped v1 (26:85).

---

## §2 C8: Markdown-link ↔ wikilink conversion

### 2.1 Ecosystem shape: plugins and CLI, not LSPs

Verified-absent list (§0.1). What exists instead ([G·] unless noted):

| Tool | Directions | Notable behaviors |
|---|---|---|
| mii-key/obsidian-links (VS Code + Obsidian) | wiki ↔ markdown / HTML / autolink | `<>` wrapping for targets with spaces; append-extension option; bulk convert variants |
| Wikilinks to MDLinks (Obsidian) | wiki → markdown, toggle | single-note scope |
| obsidian-links-converter | wiki ↔ markdown | **skips frontmatter and fenced code**; preserves aliases/anchors; refuses external URLs |
| Better Markdown Links | both | relative / absolute / shortest-path styles; demote embeds; **refuses ambiguous names rather than guessing** |
| Foam fork | wiki → markdown | bulk |
| olconv (CLI) | both | round-trip path-loss warning; skips code spans/fences |
| Links plugin | matrix wiki/markdown/HTML/autolink | |
| eetann/markdown-language-server (VS Code) | n/a (creation) | emits **multiple candidate actions** for one wikilink insertion (`[[foo.md]]`, `[[foo.md|Title]]`, `[[foo.md#ninja|Title]]`) — the per-candidate-action pattern for ambiguity |
| zk | n/a | `link-format` config (`markdown`/`wiki`/custom template `[[
{{metadata.id}}|{{title}}]]` [D zk:1700]) governs *generation/insertion* only — no conversion action |

Editor-plugin behavior is the design corpus; MS's `preferredMdPathExtensionStyle` config ([D
:1438], append vs drop extension for *generated* paths) is the in-LSP analog for path-style
configuration.

### 2.2 Edit shape: alias-preserving, both directions

From the plugin corpus, the canonical conversions:

- Standard → wiki: `[text](target.md)` → `[[target|text]]` when text ≠ filename, `[[target]]` when
  text is the filename (every alias-preserving plugin does this). Path→stem mapping must strip
  `.md` and resolve relative prefixes through 26's resolution.
- Wiki → standard: `[[target]]` → `[target](relative/path.md)` — path style (shortest/relative/
  absolute) and extension (append/drop) are **config**, mirroring Better-ML styles + MS
  `preferredMdPathExtensionStyle`.
- Embeds: `![[x]]` → `![](x.md)` (or `![x](x.md)`); 15 narrowed embeds to `![[..]]` form, so the
  bang-preservation rule is narrow. Better-ML's "demote embeds" is an explicit option there.
- Fragment links `[[#heading]]` → `[](#heading)`: intra-doc, resolution-free, trivially safe.

### 2.3 Cases the ticket must enumerate (each with precedent-backed default)

1. **Frontmatter / fenced-code / code-span positions**: multiple plugins skip them (converter,
   olconv); 26's occurrence model already excludes non-occurrence positions — conversion rides the
   same exclusion. Default: no action offered.
2. **External URLs** in standard links: no note target — refuse (converter precedent).
3. **Reference-style** `[text][ref]` + definition blocks: MS's extract/remove-link-definition
   actions ([D :7585, :7746]) are the existing hygiene layer; converting reference-style (rewrite
   the definition target + collapse to inline) is *not done by any surveyed tool*. Default: out of
   scope v1; future work can compose with MS-style definition ops.
4. **Cursor on display text**: 26's rename refusal matrix refuses display-text positions because
   rename rewrites the *target*, not the token. Conversion rewrites the **whole link token** —
   refusing display-text positions for conversion is not required by the same logic. Options:
   (a) offer at any position inside the link token (whole-token rewrite makes it safe), or
   (b) align with rename for uniformity. Present both; (a) is precedent-consistent (plugins act on
   "the link under cursor" without caring which part).
5. **Ambiguous target** (26's Ambiguous resolution — documentLink emits for Ambiguous too, per 27's
   inputs): options, all precedent-backed: (a) refuse (rename-consistent, Better-ML-aligned);
   (b) one action per candidate (eetann pattern); (c) convert text form only, leave target
   unresolved. (b) is the richest but multiplies menu entries; (a) is the most consistent with 26.
6. **Broken/unresolved target**: converting form doesn't change resolvability — offer (it's a pure
   text rewrite); combined with 25's create-note quickfix the user gets the full path
   (convert-then-create or create-then-convert).

### 2.4 Transport + kinds

- Kind: `refactor.convert.wikilinkToMarkdown` / `refactor.convert.markdownToWikilink` (or one
  `refactor.convert.linkFormat` with direction in the title). `isPreferred` on the canonical
  direction if config names one (rumdl precedent for isPreferred, §1.3).
- Eager single-`TextEdit` edit; no resolve needed; no `data` needed except when the action is
  offered from a diagnostic (then 25's channel).
- Bulk "convert all in document" variant: Pylance "Convert all…" precedent; must respect 26's
  rename-scale budgets (byte-tiered, 5,000-occurrence cap kept distinct — 27 inputs) if ever
  workspace-scoped. v1: per-occurrence only, document-wide optional.
- Disabled-action option: at refusal positions (display-text under option (b) of §2.3.4; ambiguous
  under (a)), MS's `disabled: { reason }` pattern ([D :7594-7603]) surfaces the refusal in the menu
  instead of hiding the action — requires client `disabledSupport` (VS Code has it; verify C11
  matrix).

---

## §3 C9: Extract selection to new note

### 3.1 Precedent

- **IWE "link" code action** ([G] iwe.md/docs/code-actions/link/ — fetched this session): the only
  LSP implementing selection→note-via-code-action. Mechanics: cursor-word or single-line selection;
  creates note keyed by template (`{{id}}` numeric, `{{slug}}`, `{{title}}` filename-sanitized,
  `{{today}}`); replaces source text with `[text](key)` or `[[key]]` per `link_type` config; new
  note seeded with `# <title>` heading; **collision handling = numeric suffix** (`concept.md` →
  `concept-1.md`); multiple independently-configured link actions can coexist; templates control
  the creation folder. Limits: single-line only; no undo story in docs.
- **Obsidian Note Composer** (core) [G·]: `note-composer:extract-heading` (move heading+section to
  new note, replace with link), `merge-file`, `split-file`; template support for the new note.
  PKM-native UX baseline.
- **Note Refactor** plugin [G·]: first-line-as-filename with illegal-char stripping (`: \ /`),
  content-only extraction prompt, vault-root creation. **Note Refactor Plus** adds destination
  folders, templates, frontmatter — and ships an **explicit one-step undo command** (restores
  source + deletes created file): direct evidence clients do not auto-reverse file creation, and
  the community's answer is a compensating command.
- **Dendron** [G·]: "Extract text to new note" as a refactor lightbulb; "Add missing note for
  wikilink declaration" broken-link fix; "Convert Link" as a *command* (prompt-based disambiguation
  — alias/note-name/hierarchy/prompt/destination choices) — the prompt-based UX that LSP code
  actions structurally cannot do (no mid-resolve interaction).
- **lehmacdj/wiki-language-server** [G·]: `wiki.createNoteFromSelection` via
  `workspace/executeCommand` — the command-not-action alternative when editor UI for destination
  picking is needed (Pylance "Move symbol to…" file picker is the same escape hatch).
- **TS server** [G·] / **Pylance** [G]: language-agnostic move-to-new-file (§1.3); Pylance names
  the file after the symbol **in the source file's directory**; rust-analyzer's extract-function is
  in-file only, no creation [G·].

### 3.2 Edit composition (spec + 26 pinned)

Ordered `documentChanges` ([S], §1.1), MD-Oxide's exact shape as in-corpus proof ([D :817-831]):

1. `CreateFile { uri: new_note, options: { ignoreIfExists } }` — first, so a create failure aborts
   *before* any source edit (abort semantics: pre-failure ops stay [S] — putting create first makes
   the common failure mode clean: nothing applied). `ignoreIfExists` vs `overwrite`: collision
   policy decides (§3.3); never `overwrite` by default.
2. `TextDocumentEdit { source, version: Some(current) }` — replace selection with the inserted link
   (§3.4).
3. `TextDocumentEdit { new_note, version: None }` — seed heading + extracted body (CreateFile has no
   content payload [S]; MD-Oxide appends `# heading` with the `\n\n` vs `` discrimination [D
   :812]).

Pre-validation mandatory (26:85): version check on source, collision check on target, selection
non-empty and inside an extractable range (not frontmatter/fence — same exclusion as C8). Half-state
risk is real but ordered-create-first reduces it to "orphan empty note on late edit failure",
recoverable by the user; delete-compensation is 28's territory, not v1.

### 3.3 Naming and destination (options, all precedent-backed)

- **Key generation**: sanitized selection heading (Note Refactor illegal-char strip; IWE `{{title}}`
  sanitization) — v1 default. Alternatives: first-line-of-selection, slugged title, date-keyed
  (IWE `{{today}}`).
- **Collision**: (a) numeric suffix `stem-1` (IWE; keeps the action single-shot), (b) refuse with
  reason (26's rename-ambiguity consistency), (c) multiple pre-computed candidate actions with
  distinct names (eetann pattern). (a) is IWE-proven; (b)/(c) are more 26-consistent. Decide.
- **Destination folder**: v1 = source file's directory (Pylance precedent, zero config); config
  extension later: dedicated folder (Note Refactor Plus), daily-notes folder routing (MD-Oxide
  :756-762), template-controlled (IWE, Note Refactor — ties to the deferred from-template variant).
- **Ambiguous stem collision** (new note's stem would collide with an existing *ambiguous* stem):
  same decision surface as C8 §2.3.5 — suffix keeps 26's ambiguity semantics intact (the new note
  is Unique by construction if suffixed).

### 3.4 Link insertion shape + note content

- Inserted link: `[[stem|selected text]]` vs `[[stem]]` vs `[[stem|first line]]`. Obsidian replaces
  with note-title link; IWE wiki-mode inserts bare `[[key]]`; 24's completion snippet shape
  `[[T|${1:}]]$0` (24:60) shows the alias-first convention in this codebase. Long selections argue
  for first-line alias or bare stem (a 500-word alias is hostile). Present as ticket decision with
  the three shapes.
- New note content v1: `# <heading>` + extracted body (IWE/MD-Oxide shape). From-template stays
  deferred (§0.5). Frontmatter seeding: 28's/20's territory, not v1.
- Refuse: empty selection; selection spanning whole file (that's a move — 28); selections inside
  frontmatter/fences.

### 3.5 Gating and safety (checklist)

- `create` ∈ `resourceOperations` before *offering* (shared predicate — 27 inputs, N6 26:240,
  A1:167); Neovim two-tier serializer makes the op actually deliverable there (26:213 B2).
- `isRefactoring: true` (26:85) → VS Code preview/confirm dialog; this *is* the undo story's first
  half. Second half: honest statement that undo of file creation is client-side and currently
  nonexistent (Note Refactor Plus evidence); a compensating `workspace/executeCommand` undo is a
  possible v2 (Dendron/prompt-UX escape hatch) — priced under 28, not 27.
- Trigger discipline: action is `refactor.*`, so VS Code shows it under Refactor menu, not
  quickfix-auto-apply — accidental auto-application is structurally prevented (fix-all consumers
  only apply `quickfix` kinds).
- Perf: name + collision scan is O(candidates in index) — index-resident, eager-safe; budget in §4.

### 3.6 Status-cycle code action (cluster-5 sibling, 23/27 boundary)

- 23 defines the invariant (cycle order = config insertion order = completion sort_text order;
  23:47); 27 owns the plumbing; "steps over symbols vs kinds" stays 27's open sub-question (23:47).
- Substrate exists: `TaskStatusMap`/status config `src/task.rs:119+` (A1:95).
- Shape: eager, single-doc `changes` edit, kind `source.traces.*` (or `quickfix` if ever
  diagnostic-derived — 26 §9 sketch); no file ops, no resolve, no `data`. Trivially cheap; no
  precedent gap — closest analogs are rumdl's per-warning actions (eager recompute) and MS's
  hygiene actions (eager single-doc edits).

---

## §4 Proposed performance rows for 33 (cluster proposes; 33 prices)

33 has no code-action rows; propose, following 33's interactive-threshold framing:

| Row | Budget (p95) | Basis |
|---|---|---|
| `codeAction` (eager, per open-doc request, triggerKind=Invoked) | < 10 ms | C8 conversion = range→occurrence lookup + in-memory resolution + one TextEdit; rumdl's floor is a full re-lint per request and ships fine — Traces is strictly cheaper |
| `codeAction` (triggerKind=Automatic) | < 3 ms | construction skipped/downsized per §1.3(d); measures the lightbulb-scan tax |
| `codeAction/resolve` (if C9 uses resolve) | < 20 ms | name generation + collision scan + skeleton build; still inside one frame at 60fps |
| `codeAction` on 5,000-occurrence document (bulk variant, if shipped) | < 50 ms | 26's rename-scale budget tier, byte-tiered analog |

No new lanes: all four ride the existing per-document request budget; none touches the workspace
tier.

---

## §5 Lessons for ticket 27 — concrete options for the grilling

1. **One provider** serves C8 + C9 + status-cycle (single registration, 29 owns advertisement).
   Kind map: `quickfix` create-note-from-broken-wikilink (25's `data` channel),
   `refactor.convert.*` (C8), `refactor.extract.note` (C9), `source.traces.statusCycle`
   (23-invariant). `resolveProvider`: false v1 (Marksman pattern) unless C9's edit grows expensive;
   `dataSupport` needed regardless for the diagnostic-derived quickfix.
2. **Capability gates**: `create` predicate shared with 25 (one function, 29 plumbing);
   documentChanges-per-26-B2; optional `disabledSupport` if adopting MS's disabled-action pattern
   at refusal positions.
3. **C8 decisions to make**: direction pair both-ways (default) vs one-way config; path style +
   extension config (MS/Better-ML analog); alias-preserving both directions (default);
   bang-preservation for embeds (default); reference-style out of scope v1 (default);
   display-text offering (option a whole-token vs b rename-aligned refusal); ambiguity
   (refuse vs per-candidate actions vs text-only); bulk variant scope (per-occurrence v1).
4. **C9 decisions to make**: collision policy (suffix vs refuse vs candidate-actions); link shape
   (`[[stem|text]]` vs `[[stem]]` vs first-line alias); destination (same-dir v1 default);
   eager vs resolve (eager default — precedent is uniformly eager, budget fits §4);
   `isRefactoring` unconditional (default); from-template stays deferred until 28 (confirmed);
   undo = client-side, preview is the safety, compensating command is 28-priced v2 (default
   framing).
5. **Safety invariants to write into the ticket**: pre-validation mandatory (no rollback exists,
   26:83-85); create-first op ordering (abort-semantics rationale, §3.2); refuse empty/frontmatter/
   fence selections; never `overwrite: true`.
6. **Perf**: adopt §4 rows into 33's ledger.
7. **Open verification gaps (do not claim without citation)**: VS Code multi-file WorkspaceEdit
   undo-grouping behavior — UNVERIFIED, belongs to C11's editor-capability sweep; four-editor
   codeAction rendering/menus matrix — C11/Cluster 7, note coupling only (A3 C9→C11).
8. **Couplings to restate on the ticket**: C5 (link kinds conversion may touch), C9↔C8 (same
   provider), 28 (file-ops boundary, undo-compensation pricing), 25/26 (transport + predicates +
   refusal matrix), 29 (registration + capability advertisement), 33 (rows).

---

*End of findings. Cluster 5 research complete; ready for grilling.*

---

<a id="appendix-b6" />

## Appendix B6: B6: Semantic tokens for wikilink resolution state (C10, NOVEL)

## B6: Semantic tokens for wikilink resolution state (C10, NOVEL)

**Scope**: Evidence for ticket 27's in/out/depth decision on C10 (`A3-decomposition.md:207-217`):
has any LSP encoded *resolution state* into semantic tokens, does the three-valued legend
(26's mandate: resolved / unresolved / ambiguous, `A3:16`, `27:21-26`) survive across the
four target editors, and what refresh strategy fits Traces' sync core.
**Method**: upstream primary sources throughout. LSP 3.18 spec (web; local `docs/refs/lsp_spec.md:655`
is an unfetched `{% include %}` stub, so upstream is authoritative), VS Code extension docs + the
built-in markdown TextMate grammar source, clangd feature docs + source, rust-analyzer docs,
nvim-treesitter query source, Zed/Helix issue trackers, and the local digests for
Markdown Oxide / Marksman / MS markdown language service / rumdl / zk. No production code or
ticket/map file modified; only this file written.

---

## 1. Spec essentials (LSP 3.18 `semanticTokens`)

Source: <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/#semanticTokens>
(local gap: `docs/refs/lsp_spec.md:655` proves our vendored spec lacks this whole section, so
everything below is upstream, per `A3:216-218`).

- Feature since 3.16; legend = ordered `tokenTypes[]` + `tokenModifiers[]`; every emitted token
  references legend entries **by index**. Clients announce which types/modifiers they know
  (`textDocument.semanticTokens.capabilities.tokenTypes/tokenModifiers`); **custom strings are
  legal** (a client that doesn't know a string simply can't theme it).
- Wire format: relative encoding, 5 uint32 per token (deltaLine, deltaStart, length, tokenType,
  modifiers bitfield). Multiline tokens and overlapping tokens are capability-gated
  (`multilineTokenSupport`, `overlappingTokenSupport`); deltaStart/length are in the negotiated
  position encoding.
- Three request shapes: `full`, `full/delta` (server returns edits vs. previous `resultId`),
  `range` (spec: intended for fast initial render; the server must *also* implement full, or
  use it when full is too expensive). One registration method covers all variants:
  `textDocument/semanticTokens` (not per-variant).
- `workspace/semanticTokens/refresh`: global "please re-request" request; the spec says it must
  be used "with absolute care" and notes clients (e.g. VS Code) may delay it until editors
  become visible. Cheap for the server (no payload); the cost lands on the client as
  re-request + redraw.
- `augmentsSyntaxTokens` (3.17): false means the client uses semantic tokens *instead of* syntax
  highlighting; true means layered on top. Irrelevant in practice for our case: the markdown
  grammar emits nothing for wikilinks anyway (see section 3), so tokens are the only coloring
  channel for them either way.
- `serverCancelSupport` (3.17): whether the client may send `$/cancelRequest` mid-flight.
  Traces' honest answer is **false**; issue 12 pins `tower-lsp-server` `concurrency_level(1)`
  sequential execution with no mid-request cancellation (`12-…md`).

## 2. Precedents: who encodes resolution state in semantic tokens

| Server | Entity kinds | Resolution state? | Mechanism | Source |
|---|---|---|---|---|
| **Markdown Oxide** | links (any) | **YES, 2-way** | unresolved = standard type `COMMENT`, resolved = standard type `DECORATOR`, modifiers always 0 | src digest `:2080-2105`, `:3279+` |
| **rust-analyzer** | identifiers | **YES, 2-way** | custom type `UNRESOLVED_REFERENCE`; resolved uses normal kinds; skips emission entirely when file is unlinked | <https://rust-lang.github.io/rust-analyzer/rust_analyzer/lsp/semantic_tokens/types/index.html>; <https://users.rust-lang.org/t/are-you-happy-with-the-current-state-of-rust-syntax-highlighting/49399> |
| **clangd** | C++ identifiers | **YES, but as stopgap** | `InactiveCode` kind maps to LSP type `comment`: one **zero-width token per line** ("line style rather than a token style"), only for clients lacking the dedicated inactive-regions push | <https://clangd.llvm.org/features> (Kinds table); <https://codebrowser.dev/llvm/clang-tools-extra/clangd/SemanticHighlighting.h.html> (`:57-59`); <https://reviews.llvm.org/D67536> |
| **Marksman** | wikilinks/reflinks/tags | **NO** | entity-kind only: WikiLink/RefLink map to `class`, Tag maps to `enumMember`, `Modifiers = 0u` always ("We don't use modifiers in the encoding currently") | src digest `:10877-11000`, `:11161-11166` |
| **MS markdown LS** | none | none | deliberately no semantic tokens | `tool-ms-markdown-language-service.md:15,:41`; grep of `lsp_microsoft-vscode-markdown-languageservice-digest.txt` for `semanticTokens` returns 0 hits |
| **rumdl** | none | none | grep of src+docs digests: 0 `semanticTokens` hits | this pass, `lsp_rvben-rumdl-{src,docs}-digest.txt` |
| **zk** | none | none | grep of docs+src digests: 0 `semantic token` hits | this pass, `zk-{docs,src}-digest.txt` |
| **JDT LS (Java)** | none | **NO evidence** | unresolved refs surface as error diagnostics/annotations, not tokens | websearch this pass; refuted for absence |

Three findings that matter more than the table:

1. **Markdown Oxide is the direct precedent and it is config-free.** It avoided the
   custom-legend problem entirely by mapping the two states onto two *standard* types
   (`DECORATOR` / `COMMENT`) that every theme already styles, and it ships
   `semantic_tokens = true` by default (docs digest `:346-350`, `:1146-1150`: "even if this is
   turned off, special semantic tokens will be sent for the unresolved links, allowing you to
   visually identify unresolved links"; tokens are deliberately a diagnostic-independent
   channel). It pushes `client.semantic_tokens_refresh()` after every vault rebuild
   (src digest `:1746-1747`, `:1795-1796`).
2. **clangd's mature design *escaped* semantic tokens.** State-in-tokens was the first attempt
   (D67536); the shipped solution for capable clients is a dedicated server-to-client push
   (`textDocument/inactiveRegions`, implemented in
   <https://github.com/clangd/vscode-clangd/blob/master/src/inactive-regions.ts>), with `comment`
   tokens kept only as fallback. The lesson: when state spans *regions* rather than identifiers,
   token encoding gets awkward fast. Wikilink resolution state is per-identifier, so we are in
   the Markdown Oxide shape, not the clangd shape; the precedent still warns against inventing
   exotic token semantics.
3. **Even rust-analyzer treats custom types as a smell**: issue #13066
   (<https://github.com/rust-analyzer/rust-analyzer/issues/13066>) is an active effort to
   *reduce* its non-standard token types. Don't bet the legend on custom strings surviving
   client fashion.

## 3. Syntax baseline: TextMate/tree-sitter coverage of wikilinks is **zero**

This flips the ticket's working assumption ("syntax highlighting likely covers adequately",
`A3:210-211`). Verified by fetching grammar sources:

- **VS Code built-in markdown TextMate grammar**
  (<https://raw.githubusercontent.com/microsoft/vscode/main/extensions/markdown-basics/syntaxes/markdown.tmLanguage.json>):
  grep for `[[` yields exactly 2 hits, both inside the `bold`/`italic` begin-regexes (nested
  bracket bookkeeping, lines ~2783, ~2943); **no wikilink rule exists**. Frontmatter *is*
  covered: the `frontMatter` rule (~line 2662) injects `source.yaml`.
- **Neovim treesitter markdown** (nvim-treesitter
  `queries/markdown/highlights.scm`,
  <https://raw.githubusercontent.com/nvim-treesitter/nvim-treesitter/master/queries/markdown/highlights.scm>):
  captures `link_destination` / `link_label` (standard links only); **no wikilink capture**.
- **Upstream MDeiml/tree-sitter-markdown grammar**
  (<https://raw.githubusercontent.com/MDeiml/tree-sitter-markdown/master/tree-sitter-markdown/src/grammar.json>):
  node inventory contains `link_label`, `link_destination`, `link_title`,
  `link_reference_definition`; **no `wiki*` node at all**. No editor using this parser can even
  *see* `[[...]]` structurally without a fork/extension.
- Zed's markdown grammar wikilink coverage: **not verified this pass** (grammars repo paths
  404'd); treat as open question for C11's sweep, likely MDeiml-derived and hence also zero.

**Consequence**: in VS Code and Neovim out of the box, `[[wikilink]]` gets *no structural
coloring whatsoever*; not even a uniform "this is a link" paint. Users currently color wikilinks
via third-party TextMate grammar extensions / obsidian.nvim. So C10 is not "add state nuance on
top of a colored baseline"; it would be the *only* first-party coloring channel for wikilinks.
That is the strongest value argument in this whole ticket, and it also means even the "resolved"
state has value: uniform entity coloring (Marksman-style) is already an upgrade.

## 4. Client reality (the deciding matrix)

### VS Code (first-class)
- `editor.semanticHighlighting.enabled` defaults to `configuredByTheme`: built-in themes
  (Dark+ etc.) have it on; **third-party themes may be off**.
  <https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide>
- Theming path: theme `semanticTokenColors` selectors, then fallback TextMate scope map, then
  nothing. **Custom types/modifiers get no scope-map entry and no default color.** For them to
  be styleable "for free", a *VS Code extension* must contribute them in package.json
  (`semanticTokenTypes` with optional `superType` for theme inheritance,
  `semanticTokenModifiers`, `semanticTokenScopes`); the server cannot do this alone.
- Standard types/modifiers are themed by all decent themes out of the box. This is why Markdown
  Oxide's DECORATOR/COMMENT trick works config-free here.

### Neovim (first-class)
- Core implementation `runtime/lua/vim/lsp/semantic_tokens.lua`
  (<https://github.com/neovim/neovim/blob/master/runtime/lua/vim/lsp/semantic_tokens.lua>):
  default `debounce = 200` ms; redraws via extmarks; supports full + delta.
- Highlight groups: `@lsp.type.<type>.<ft>`, `@lsp.mod.<mod>.<ft>`,
  `@lsp.typemod.<type>.<mod>.<ft>`. nvim defines default links for the *standard* types
  (e.g. `@lsp.type.class` links to `@type`); **custom strings resolve only if the colorscheme or
  user defines the group**, otherwise the token paints nothing.
- Net: standard-type mappings are live immediately on any colorscheme; custom legend entries
  need shipped doc snippets (`hi link @lsp.mod.…`) or a colorscheme PR.

### Zed (first-class, but default-off)
- Ships semantic tokens since Stable 0.224.4 (PR #46356
  <https://github.com/zed-industries/zed/pull/46356>, tracking issue #7450 closed 2026-02-18
  <https://github.com/zed-industries/zed/issues/7450>; impl
  `crates/project/src/lsp_store/semantic_tokens.rs`).
- Per-language setting `semantic_tokens: "off" | "combined" | "full"`, **default `"off"`
  (tree-sitter only)**; custom `token_type`+`token_modifiers` styling goes through
  `semantic_token_rules` in settings; language attach/override mechanics per
  <https://zed.dev/docs/configuring-languages>.
- Net: capability exists, but a Zed user must flip a setting *and* (for custom legend entries)
  write rules. Standard-type mappings at least render once "combined"/"full" is on.

### Helix (sparse floor, confirmed dead)
- **Helix does not apply semantic tokens at all.** Open feature request #8438
  (<https://github.com/helix-editor/helix/issues/8438>); discussion #5589 states Helix is
  "waiting on semantic tokens to be implemented"
  (<https://github.com/helix-editor/helix/discussions/5589>); the Helix book's
  configurable-features list (<https://docs.helix-editor.com/languages.html>) has no
  semantic-tokens entry (wire types exist in `helix-lsp-types/src/semantic_tokens.rs` but are
  unused). Tokens would be silently ignored, consistent with ticket 05's sparse-floor framing;
  the state story in Helix is grammar + diagnostics only, same as today.

## 5. Legend design (type vs modifier vs both, three-valued)

26 mandates three-valued *modifiers* (`A3:16`, `27:21-26`). Spec constraints on that mandate:

- Modifiers are a **bitfield**, so "three states" as modifiers means three mutually exclusive
  bits: legal, but odd, since clients can style `mod.a`, `mod.b`, `mod.a+mod.b` (the last never
  emitted). Precedents avoid this: everyone ships 0 or few modifiers; state-as-*type* (Markdown
  Oxide, rust-analyzer) is what shipped servers actually do.
- Client support cost per entry (from section 4): **standard type/modifier = themed everywhere;
  custom string = themed nowhere without per-editor work** (VS Code companion extension or user
  theme; Neovim highlight link; Zed `semantic_token_rules`).

Concrete legend options:

| # | Design | Config-free result | Three-valued? | Precedent |
|---|---|---|---|---|
| L1 | State as **standard-type mapping**: resolved maps to `decorator`, unresolved maps to `comment`, ambiguous maps to a third standard type (e.g. `type` or `interface`) | **Yes; all states color on every theme, Zed-full, Neovim** | Yes, at the cost of semantic type oddity ("a link is a comment when broken") | Markdown Oxide (2-way) |
| L2 | Entity as standard type (wikilink maps to `class`, like Marksman) + 3 custom state **modifiers** | Base coloring yes (wikilinks visible as classes); **state differentiation needs per-editor config** | Yes | 26's mandate; Marksman has the type half |
| L3 | 3 **custom types** (`tracesLinkResolved` etc., optional `superType`) | No (invisible until configured) | Yes | rust-analyzer's single custom type; MO docs describe users configuring colors |
| L4 | 2 custom types, ambiguous falls back to "no token" or merges with unresolved | Partial | No (two-valued) | rust-analyzer skips unlinked files |

Honest read: L1 is the only design that lands three-valued coloring on screen **without Traces
shipping editor-side artifacts**; L2 matches 26's wording and degrades to today's Marksman UX
(uniform wikilink color) until users configure state colors; L3/L4 punt the value question to
per-editor config and re-import the exact problem L1 exists to dodge. A hybrid worth naming:
L2's type+modifier shape for the *wire* (machine-readable, matches 26) while documenting
L1-style standard-type fallbacks for clients that ignore unknown modifiers; the wire is one
legend, so pick one.

## 6. Refresh strategy vs sync core (issues 12/14/33)

- Client-side re-request pressure already exists: Neovim re-requests on change (200 ms debounce),
  VS Code re-requests on change. A keystroke in the edited doc refreshes its own tokens without
  any server push. The gap is **cross-file staleness**: rename/create/delete in doc B leaves open
  doc A's colors wrong until A is edited. Markdown Oxide's answer: push
  `semantic_tokens_refresh` after every vault update (src digest `:1746-1747`); simple, and the
  spec-sanctioned mechanism, but "absolute care" territory, since every visible markdown editor
  re-requests, riding Traces' **single sequential lane** (issue 12).
- Recommendation shape: v1 emits full-document tokens computed **on request** from the current
  overlay (issue 14 freshness falls out for free; the resolver sees the same overlay the request
  path sees); **no proactive refresh in v1**, documented staleness-until-next-edit for
  cross-file changes; if refresh ships later, debounce it behind index-mutation events (per
  issue 25/26 pipeline), never per-keystroke. `Client::semantic_tokens_refresh` already exists in
  tower-lsp-server (`A2` section 1: `client.rs:300`).
- **Skip delta and range.** Delta buys nothing at PKM scale: notes parse at ~0.02 ms/10 KB
  (issue 14), the 5,000-ref ceiling is workspace-wide (27 inputs), and full recompute per request
  keeps zero resultId bookkeeping on the sequential server. Range exists for "full too expensive"
  clients; Marksman advertises range=true without delta (src digest `:11161-11166`), MO
  advertises `full: true, range: false` (src digest `:2080-2105`); the lean precedent is MO's
  shape: **legend + full only**.
- Cost model: per request = walk parse tree for wikilink/embed spans, then resolver lookup
  (hash index + trie, `A1` section LinkResolver); token sort + relative encode is O(n log n) in
  link count, trivial per note. The real occupancy cost is lane-time under editor churn, same
  class as documentSymbol, which issue 33 already budgets (<10 ms / <5 ms interactive).
  **Issue 33 has no semantic-tokens row today** (`33-…md`); if C10 ships, add one anchored to
  documentSymbol, keeping the 5,000-ref ceiling consciously distinct (27 inputs).

## 7. Overlap with issue 15: what is actually unique

- Issue 15 already gives **unresolved** wikilinks an Information-severity diagnostic (registry
  key like `links.wikilink.unresolved`, default hint/info, policy includes `off`) plus a
  create-note code action (`15-…md`, `map.md:83`). Ambient coloring of unresolved links is thus
  a **second channel for half of C10's information**: real, but incremental; editors routinely
  show the same fact as squiggle + color (rust-analyzer does exactly this).
- **Ambiguity has no channel at all**: 15 deliberately emits *no* diagnostic for `Ambiguous`
  (client disambiguates; "Ambiguous is not Unresolved", `15-…md`, `26` pins ties to None). If
  Traces wants users to *see* ambiguous links at a glance, semantic tokens (or an editor
  extension) are the only vehicle. This is the genuinely novel slice of C10.
- Unique values, ranked: (1) first-ever coloring of wikilinks at all in VS Code/Neovim (section 3;
  zero grammar coverage), (2) ambiguity visibility (no other channel), (3) ambient
  resolved/unresolved state without squiggle noise, (4) create-note discoverability reinforced
  visually. Non-values: Helix (ignored), Zed default config (ignored until toggled), users on
  third-party VS Code themes with semantic highlighting off (ignored).

## 8. Options for ticket 27 (with the honest out-of-scope case)

**O1: IN, lean (recommended baseline)**. Legend = standard entity type + state, full-document
only, no delta/range, no proactive refresh in v1; entities = wikilinks + embeds (tags second
pass, 16's index); gate emission on client advertising any known tokenTypes (initialize-time
capability read, shared with 29's matrix); ship per-editor config snippets in docs (Neovim
`hi link`, Zed `semantic_token_rules`, VS Code theme snippet). Legend choice: L2 (26's mandated
modifiers, base type `class`), accepting that state differentiation is opt-in config while
wikilinks get first-party coloring everywhere immediately.

**O2: IN, config-free**. Same transport as O1 but legend L1 (states mapped to standard types,
Markdown Oxide style). Three states, three standard types; zero per-editor artifacts; cost is
semantic type oddity and slight theme-coupling (a theme that styles `comment` dim does the right
thing by accident). Best "it just works" story; slightly worse machine-readability for future
clients.

**O3: IN, unique-slice only**. Tokens **only** for `Ambiguous` (optionally unresolved too, but
that duplicates 15). Smallest legend, pure novelty, honest uniqueness; odd UX, since only one
class of link is colored and users cannot tell resolved links apart from plain text.

**O4: OUT of 27 (defer, not reject)**. 26's spine (`Resolution` three-valued enum,
`highlight(target, doc)`, documentLink emitting for Unique+Ambiguous) still ships for C5/C8;
tokens are a pure emission layer on top, so deferring costs little and can be revisited as a
standalone ticket after 29's client matrix lands. The out case rests on: Helix ignores tokens
entirely; Zed ships them default-off; VS Code third-party themes may disable them and custom
legend entries need a *companion VS Code extension* to theme reliably (server alone cannot);
only 1 of ~6 surveyed servers (MO) actually colors resolution state, and MS's deliberate absence
plus rust-analyzer's custom-type cleanup trend (#13066) suggest the ecosystem does not consider
this load-bearing; unresolved duplicates 15's diagnostic channel; issue 33 has no budget row for
it. **Counterweight** (why O4 is weaker than it looks): section 3's zero-grammar-coverage finding
means the comparison class is not "code servers that already color identifiers" but "markdown
users with no coloring at all", where MO's default-on tokens are the only mainstream precedent
and they chose to ship them.

**Leanest maximally-capable design** (synthesis): O2's legend (three standard types, config-free,
MO-proven) + O1's transport (full-only, request-time compute from overlay, no refresh, no delta),
emitting wikilinks + embeds only, gated on advertised capabilities, with the ambiguity state as
the headline feature (sole channel) and the 15 overlap stated openly. That is the entire feature:
legend + one walk of the parse spans + relative encoding; everything else is already built
(26's resolver states, 14's overlay, 25's registry severity as the gated fallback story).

**Open questions for C11/27**: Zed markdown grammar wikilink coverage (likely zero, verify);
whether we ever ship a tiny VS Code companion extension (contributes types + default colors, the
only path to VS Code "just works" beyond standard-type mappings); whether tags (16) join v1
entities; whether `augmentsSyntaxTokens=false` clients (if any target editor sets it) change the
story (they should not; section 3).

## 9. Lessons for ticket 27 (checklist)

1. Local spec is a stub for this feature (`docs/refs/lsp_spec.md:655`); cite upstream 3.18 only.
2. Two-way resolution state in tokens has shipped (MO, rust-analyzer); three-valued has **not**;
   we would be first, so keep the legend dumb (standard types) to survive that novelty.
3. The syntax baseline covers **zero** wikilinks in VS Code + Neovim; the value story is "color
   at all", with state as the differentiator. The ticket's "likely covers adequately" assumption
   is refuted for this feature.
4. Server capability is not the user-visible outcome: Zed default-off, Helix never, VS Code
   theme-dependent. Design for graceful degradation (MO trick) instead of guaranteed styling.
5. Client re-request pressure (200 ms debounce / on-change) plus the single sequential lane (12)
   means: full-only, no delta, request-time compute, refresh deferred and debounced.
6. The unique, non-duplicative payload is **ambiguity coloring** (15 gives ambiguity no
   diagnostic); unresolved coloring is a bonus channel over 15's Information diagnostic.
7. If shipping: add a semantic-tokens row to issue 33's budget table (anchor to documentSymbol's
   <10 ms/<5 ms), keep it distinct from 26's 5,000-ref ceiling, and advertise false for
   serverCancelSupport (12's sequential truth).

---

<a id="appendix-b7" />

## Appendix B7: B7: Editor × feature matrix, static vs. dynamic registration, performance

## B7: Editor × feature matrix, static vs. dynamic registration, performance
## budgets, shared predicates (C11, C13, C14, C15)

Research for ticket 27. Method extends research/24 §6: every cell is verified
against primary sources (editor source, editor docs, or the LSP 3.18 spec), not
recollection. Web captures from this session live in
`/var/folders/9w/3qn47_qj3m9b27gkxwr5_k9m0000gn/T/opencode/`
(`vsc/` = microsoft/vscode-languageserver-node@main, `nvim-lsp.txt` /
`nvim_protocol.lua` = neovim@master runtime docs + generated protocol types,
`zed-lsp.rs` / `zed-lsp_store.rs` = zed@main, `helix-*.md` + `/tmp/hx*` =
helix@master, `/tmp/ra_*` = rust-lang/rust-analyzer@master,
`lsp318/` = spec markdown). Line numbers below refer to those captures unless a
digest path is given. "Sparse floor" = ticket 27's Helix-only-minimal stance
(A3 "Grounding & constraints").

---

## 1. C11: Editor × feature support matrix

Eight features in scope (A3 cluster 7): documentSymbol (nested), workspace
symbol, foldingRange, selectionRange, documentLink, codeLens, semanticTokens,
codeAction. First-class editors: VS Code, Neovim, Zed; sparse floor: Helix.

### 1.1 The matrix

| Feature | VS Code | Neovim | Zed | Helix (sparse floor) |
|---|---|---|---|---|
| documentSymbol (hierarchical) | yes, `hierarchicalDocumentSymbolSupport: true`, plus `tagSupport`, `labelSupport` | yes, hierarchical cap true; default map `gO` (:1598; mapping since 0.11, neovim/neovim#30781) | yes, hierarchical | **flat only** (`hierarchical_document_symbol_support: Some(false)`, helix-lsp/src/client.rs:585-760); still renders outlines from `SymbolInformation[]` |
| workspace symbol | yes + `resolveSupport: { locationRange: true }` (lazy resolve) | yes; **no resolveSupport** (needs full `location.range` every result) | yes (`WorkspaceSymbolRequest`, lsp_store.rs:9390, cap check :9375) | yes (`WorkspaceSymbols` feature) |
| foldingRange | yes; `lineFoldingOnly: true`, `workspace.refresh` | yes since **0.11** (`vim.lsp.foldexpr/foldtext/foldclose`, PR #31311 merged 2024-11-29; capability `lineFoldingOnly: true`, `collapsedText: true`, refresh true) | yes; `line_folding_only: false`, `collapsedText: true` | **no** (absent from `LanguageServerFeature`, helix-core/src/syntax/config.rs:303) |
| selectionRange | yes (smart-select expand) | yes since **0.11** (`vim.lsp.buf.selection_range`, `v_an`/`v_in` fallback, :1721) | **no** (no `SelectionRangeRequest` anywhere in lsp.rs / lsp_store.rs) | **no** (absent from feature enum) |
| documentLink | yes + `tooltipSupport` | yes; `gx` dispatches it (:88; 0.11 "allow gx to function for markdown links", commit f864b68c) | yes; `tooltip: true`, resolve support, `RefreshDocumentLinks` handled | yes since 2026-03 (PR #15235); **integrated into `gf`**: `goto_file_impl` prefers LSP links, falls back to path/URL detection (helix-term/src/commands.rs:1382-1440); 250 ms debounce fetch |
| codeLens | yes + `resolveSupport`, `workspace.refresh` | yes (`vim.lsp.codelens` :2157, `grx` run; `resolveSupport` announced 0.11 commit 6072153796d1; refresh true) | yes; `CodeLensResolve`, `RefreshCodeLens` handled (lsp_store.rs:1179) | **no** |
| semanticTokens | yes; `range: true`, `full: { delta: true }`, `serverCancelSupport`, `augmentsSyntaxTokens: true`, workspace refresh; **overlapping: false, multiline: false** | yes since 0.9; `range: true`, `delta: true`, overlapping/multiline **true**, `serverCancelSupport: false`, refresh true | yes (dynamic; `settings::SemanticTokens` global mode); `requests.range: None` (**full only**), `delta: true`, overlapping/multiline true, `serverCancel: true`, `augments_syntax_tokens` honored (:862) | **no** |
| codeAction | yes + `resolveSupport` (edit\|command), `isPreferred`, `disabled`, `data`; dynamicRegistration **true** | yes + resolve (0.11 #32704); dynamicRegistration **true** (the one feature Neovim marks dynamic) | yes + resolve | yes (`CodeAction` feature) |

### 1.2 Quirks and traps (row-for-row, 24 §6 style)

- **Neovim is the odd resolver**: it does *not* support `workspace/symbol`
  resolve and sets `serverCancelSupport: false`, but advertises
  overlapping/multiline semantic tokens that VS Code refuses. A token stream
  must stay single-line and non-overlapping (VS Code ignores anything else);
  never rely on multiline tokens even though Neovim/Zed accept them.
- **Zed is the only non-hierarchical-blind modern editor that drops
  selectionRange entirely**; Helix drops it too. Two of four editors never send
  the request, so keep the handler an O(depth) ancestor walk, no caching.
- **Helix never sends folding/selection/semanticTokens/codeLens.** These four
  handlers must be *capability-skipped no-ops* there, not errors (the
  rust-analyzer "lens config off, return before any DB query" pattern,
  `/tmp/ra_req.rs:1694-1697`).
- **Helix documentSymbol must be dual-shaped**: hierarchical when
  `hierarchicalDocumentSymbolSupport` is true, flat `SymbolInformation[]`
  otherwise (rumdl precedent: `docs/digests/lsp_rvben-rumdl-src-digest.txt:40480-40500`).
- **Helix link UX is `gf`, not a dedicated key**: our documentLink results ride
  on the editor's existing goto-file flow; tooltip enrichment is wasted on
  Helix and Neovim (`tooltipSupport` false both).
- **Lazy workspace-symbol resolve is VS Code-only** (resolveSupport
  `locationRange`). Neovim/Zed/Helix need full ranges up front. Ticket 26's
  spine makes the range cheap (`RangeInFile` materialized from `NavTarget`),
  so resolve eagerly for everyone and treat VS Code's resolve as an optional
  no-op path; do not build a second code path for one client.

### 1.3 Sparse-floor consequence

With Helix at the floor, every Traces feature still has at least one consumer:
documentSymbol/workspaceSymbol/documentLink/codeAction on all four editors;
folding/selection/codelens/semanticTokens on the three first-class editors.
No feature is dead weight; no feature may *require* the sparse editor to send
anything it won't.

---

## 2. C13: Static vs. dynamic registration

### 2.1 What the spec actually requires

- Capabilities are either advertised in `InitializeResult.capabilities`
  (**static**) or pushed later via `client/registerCapability` (**dynamic**).
  The server **must not** register the same capability both statically and
  dynamically for the same document selector
  ([spec, registerCapability](https://microsoft.github.io/language-server-protocol/gh-pages/_specifications/lsp/3.18/language/registerCapability.md)).
- Dynamic registration is opt-in per feature: the client advertises
  `dynamicRegistration: true` in that feature's `ClientCapabilities`; if absent
  or false, a dynamic registration is a protocol error / ignored.
- `StaticRegistrationOptions.id` exists so a statically-advertised capability
  can later be unregistered; none of the surveyed servers use it.

### 2.2 What the four editors actually advertise

| Feature | VS Code | Neovim | Zed | Helix |
|---|---|---|---|---|
| codeAction | true | true | true | false |
| codeLens / documentLink / foldingRange / semanticTokens | true | **false** | **false** | false |
| documentSymbol / selectionRange / workspace symbol | true | false | false | false |

(Columns: `*.dynamicRegistration` in each client's capability builder:
vsc/protocol.codeLens.d.ts + friends; nvim_protocol.lua; zed-lsp.rs:840-960;
helix-lsp/src/client.rs.)

Only VS Code broadly supports dynamic registration. Neovim supports it for
exactly one feature (codeAction); Zed/Helix for none of the eight.

### 2.3 Peer registration models (all verified this session)

- **rumdl**: everything negotiated at initialize; "flags like `enableSymbols`
  are negotiated when the server starts", restart required
  (`docs/digests/lsp_rvben-rumdl-docs-digest.txt:2646`). Static only.
- **rust-analyzer**: all eight-feature capabilities static in one builder
  (`crates/rust-analyzer/src/lsp/capabilities.rs:48-141`:
  `selection_range_provider: true` :48, `document_symbol_provider` :76,
  `workspace_symbol_provider` :77, `code_action_provider` :78,
  `code_lens_provider` :79, **`document_link_provider: None`** :101,
  `folding_range_provider` :103, `semantic_tokens_provider` :141). Its only
  dynamic registrations are sync-shaped: `didSave` gated on
  `did_save_text_document_dynamic_registration()` (main_loop.rs:188, 220-266;
  capabilities.rs:323-331), watched-files likewise gated.
- **Marksman**: dispatch table registers folding/documentLink/codeLens
  handlers (`docs/digests/lsp_microsoft-vscode-marksman...digest.txt:3170-3190`)
  but the `ServerCapabilities.Default` block advertises *neither* those nor
  selectionRange (:11145-11170), so the handlers are unreachable dead code. It
  also disables Document/WorkspaceSymbolProvider **for VS Code specifically**
  (`First <| not clientDesc.IsVSCode`) so the editor-native outline wins.
- **MS markdown-language-service**: static providers bound on the connection
  (`docs/digests/lsp_microsoft-vscode-markdown-languageservice...digest.txt:1767`).

### 2.4 Decision-shaped conclusion for ticket 27

Register **all eight statically**, gated by config flags negotiated at
initialize (the rumdl model). Dynamic registration buys nothing here: three of
four target editors won't accept it for these features, none of these features
needs per-workspace re-binding mid-session, and the spec forbids dual
registration. Keep the r-a *pattern* in reserve for the one capability where it
matters (vault-watcher, `didChangeWatchedFiles`), gated on client caps the
way r-a gates `didSave`. Anti-pattern to avoid (Marksman): shipping handlers
that the advertised capabilities omit.

---

## 3. C14: Performance budgets, invalidation, proposed latency rows

### 3.1 The constraints that bind these numbers

- **Cancellation is stale-discard only** (ticket 12): no mid-request
  interruption. Every on-demand handler must therefore be cheap
  *uninterrupted*; budget for the full walk, not a cancelled partial one.
  (Peers do support true cancellation: r-a via salsa revision/`salsa::Cancelled`,
  research/x §3.2; MS threads a `CancellationToken` through every provider and
  checks it between sub-walks, digest :5021-5075; we can adopt the *shape*
  later if issue 12's scope changes.)
- **Ceilings from upstream tickets**: 23's 1k/5k node caps and the workspace
  numbers below; 26's **5,000-reference ceiling** for reference-class work.
  Distinct budgets, distinct constants (see §4.2).
- **Issue 23 measurements** (research/23 §XIII.1, density bench):
  workspace/symbol-shaped tasks query **0.665 ms @ 60K rows (≈20K files)**,
  **1.77 ms @ 100K rows**, **18.08 ms @ 1M rows**; realistic PKM ≈ 2-4 ms;
  `WorkspaceIndex__load` is a **15-20 ms fixed floor** and must never run at
  request time; `documentSymbol`-shaped walk ≈ **16 ns/item, 50K items =
  0.81 ms**; **4,000-line reparse = 18.6 ms** vs. issue 33's <10 ms interactive
  targets (tension remains flagged, unchanged by this research).

### 3.2 Invalidation model per feature (cheap by construction)

| Feature | Work source | Invalidation |
|---|---|---|
| foldingRange | heading walk (subset of the documentSymbol pass) | none; recompute per request, no cache, no refresh unless we choose to push |
| selectionRange | ancestor chain over the syntax tree at cursor | none; O(depth), never cached |
| documentLink | per-doc outgoing-link extraction already in 26's didChange path | version-stamped per-doc cache; invalidated by didChange |
| codeLens | placeholders derived from 26's occurrence pass; heavy part in `codeLens/resolve` | placeholder cache version-stamped; `workspace/codeLens/refresh` on *index* mutation only |
| semanticTokens | single classifier pass over doc tokens; delta patches on incremental didChange | document version; delta from previous version |
| workspace symbol | resident index query (no `__load`) | index is live; no staleness |
| documentSymbol | heading index slice | live index + open-buffer overlay |

### 3.3 CodeLens laziness pattern (verified, rust-analyzer)

`handle_code_lens` returns **placeholders carrying a `data` payload**; the
lens-config-off path returns before any DB query (`/tmp/ra_req.rs:1687-1697`);
`handle_code_lens_resolve` does the expensive per-lens computation from `data`
and clears it (:1707-1737). Clients that support it are re-nudged via
`workspace/codeLens/refresh` (all three modern editors advertise
refreshSupport; Zed explicitly handles `RefreshCodeLens`,
lsp_store.rs:1179). This is the shape for Traces reference-count lenses:
placeholder from the cheap occurrence pass, count computed on resolve, refresh
on index change, never per keystroke.

### 3.4 Output caps (peer-verified)

MS folding assembles three sub-walks (regions, headers via cached TOC, blocks)
and slices the result to the client's `rangeLimit`
(digest :5021-5032). The spec treats `limit` as a hint. Mirror this: honor
`FoldingRangeParams.limit` and `DocumentSymbolParams` depth caps; cap
workspace-symbol results at 23's node ceilings; cap reference-derived
lens/token work at 26's 5,000-reference ceiling.

### 3.5 Proposed latency rows for issue 33 §3 (fill the gap)

Issue 33's §3 table has a row for "Document symbols" (<10 ms) but **no rows
for the ticket-27 features and an UNFILLED workspace/symbol p95**. Proposed
additions, in the table's own shape:

| Capability | Target | Stretch | Strategy |
|---|---|---|---|
| Workspace symbols | **<5 ms** | <2 ms | Resident index only; 23's bench shows 0.665-1.77 ms at 20-100K rows, so p95 <5 ms has measured headroom; 18 ms @ 1M rows is out of stated PKM scope, document the degradation |
| Folding ranges | **<10 ms** | <5 ms | Heading walk shares the documentSymbol pass; honor client `rangeLimit` |
| Selection ranges | **<5 ms** | <2 ms | Ancestor chain from syntax tree at cursor; no vault access |
| Document links (full) | **<10 ms** | <5 ms | Per-doc outgoing-link extraction already on the didChange path (26) |
| Document link resolve | **<5 ms** | <2 ms | Path materialization only (25's create-target logic) |
| CodeLens (placeholders) | **<20 ms** | <10 ms | Occurrence-pass slice; heavy work deferred to resolve |
| CodeLens resolve (per lens) | **<10 ms** | <5 ms | Single-target reference count from inverted index, ceiling-capped |
| Semantic tokens (full) | **<20 ms** | <10 ms | One classifier pass; delta on incremental sync keeps steady-state near zero |

Notes for the issue-33 edit: these are *planning budgets*, not measurements;
word them like the existing rows ("Strategy" column), and keep the 18.6 ms
reparse vs. <10 ms hover tension visible in the same section.

---

## 4. C15: Shared predicates and ceilings

### 4.1 How mature peers structure the shared layer

- **rust-analyzer**: protocol-free queries in `analysis::*`; thin `handle_*`
  adapters; **one** capabilities builder (`lsp/capabilities.rs`) and **one**
  semantic-tokens legend module (`lsp/semantic_tokens.rs`, a single
  `declare_enum!`-generated `SupportedType`/`SupportedModifier` set plus
  delta-computation helpers, verified this session).
- **clangd**: one `Symbol`/`Ref` + `SymbolRole` bitmask consumed by
  symbols, refs, and codeAction alike (research/26 Part II).
- **MS markdown-language-service**: one TOC cache + one link resolver shared
  by folding, symbols, links, smart-select, codeAction providers
  (`getDocumentSymbols` even takes `includeLinkDefinitions`, digest :1556;
  header folding reuses `tocProvider`, :5050-5064). Every provider takes a
  `CancellationToken`.
- **Marksman**: the same `TableOfContents`/doc map drives symbols + links.
- **ty / ruff-server** (research/35): salsa query invariants shared across
  request handlers; capabilities static.

The invariant across all five: **one resolve pipeline, many thin protocol
adapters; capabilities in one module; ceilings as named constants.**

### 4.2 Mapping onto ticket 26's spine (the Traces answer)

Ticket 26 already decided the shared layer: `Occurrence` / `NavTarget` /
`Resolution` plus a `NavContext` facade (`locate`/`resolve`/`candidates`/
`references`/`rename`/`highlight`). C15's job is to bind the eight new
consumers to that spine and nothing else:

| Feature | Derived from spine as |
|---|---|
| foldingRange | heading `Occurrence` ranges (kind `region` for explicitly marked sections) |
| selectionRange | ancestor chain over the heading tree the spine already walks |
| documentLink | link `Occurrence`s with `Resolution` targets; tooltip = target title |
| codeLens | **Ambiguous** reference occurrences as placeholders; `resolve` = `Resolution` candidates (26's create-note action split) |
| semanticTokens | `Occurrence`-kind classifier: Unique→link token; Ambiguous→link + the 3-valued `Resolution` modifier (resolvable/warning) |
| documentSymbol / workspaceSymbol | heading/symbol slice of the same index the spine queries |
| codeAction | 26's actions verbatim |

**Ceilings**: one constants module, two distinct families (do not merge):
`SYMBOL_CEILING` (1k/5k nodes, from 23; feeds documentSymbol/workspace-symbol
truncation and folding `limit`) and `REFERENCE_CEILING` (5,000, from 26;
feeds lens counts, reference tokens, rename batching). Per-issue-23's warning,
1k nodes ≠ 5k refs; the names must make the mix-up impossible.

**Predicates**: defined in core, never in DTOs (standing constraint #3):
`resolvable(target, doc)`, `create_supported(resource_operations)` (from
25's `create` check), `ambiguous(ref)`, `resolution → token modifier` mapping.
The protocol boundary performs a single row-for-row mapping spine→DTO per
consumer (26's consumer matrix), plus one shared capability snapshot from §2
that drives: hierarchical-vs-flat documentSymbol shape, whether to emit
`collapsedText`/tooltips, whether any given handler early-returns (Helix
sparse floor), and fallback when the client never asked.

---

## 5. Lessons for ticket 27 (options, not decisions)

1. **Register statically, gate with initialize-time config flags** (rumdl
   model). Dynamic registration is a VS Code-only nicety for these features
   (§2.2); skip it, and keep the gated-watcher pattern (r-a `didSave`) in mind
   only for future `didChangeWatchedFiles`.
2. **One documentSymbol handler, dual shape**: hierarchical or flat per
   `hierarchicalDocumentSymbolSupport` (rumdl :40480-40500). Helix requires it.
3. **Eager workspace-symbol ranges**: resolveSupport is VS Code-only; 26's
   spine makes ranges free. One code path, not two.
4. **Semantic tokens**: full+delta, range optional-but-cheap; keep tokens
   single-line/non-overlapping (VS Code); legend = 3-valued Resolution
   modifiers only, declared in one module (r-a `lsp/semantic_tokens.rs` shape).
5. **Folding**: headings-only is safe everywhere (`lineFoldingOnly` true or
   absent); emit `collapsedText` (Neovim+Zed consume it).
6. **CodeLens**: placeholders + resolve (r-a pattern); `workspace/codeLens/
   refresh` on index mutation only; Helix gets the no-op path.
7. **Selection range**: cheapest handler in the set, but only two of four
   editors ever call it; no caching, no cleverness.
8. **Never copy Marksman's capability block**: handlers exist only if the
   advertised capability exists; and do *not* disable features for VS Code the
   way Marksman does; Traces wants its own outline everywhere.
9. **Feed §3.5's rows into issue 33 §3** and keep the 4k-line-reparse tension
   flagged in the same breath.

---

## 6. Confidence and open verifications

- High confidence: all matrix cells, registration rules, r-a capability
  lines, MS folding/token discipline, Helix `gf` integration; each verified
  against the captures listed at the top this session.
- Medium (verify at implementation): Neovim `selection_range` exact minor
  version (docs say 0.11-era; the foldexpr/gx/codelens-resolve attributions
  are pinned to commits/PRs above); r-a's `RefreshCodeLens` outbound path
  (Zed's *handling* of it is verified; r-a's `lsp-ext` crate moved, re-locate
  before citing in an ADR).
- Not verifiable this session: `vscode-markdown-languageserver`'s own
  `server.ts` (GitHub 404, repo renamed/moved); its registration facts ride
  on the existing research/tool-ms-markdown-language-service.md digest.
- Helix document-link debounce (250 ms) and PR number (#15235) come from the
  earlier session's digest, re-confirmed by this session's handler presence in
  commands.rs:1382-1440.

## Sources

- LSP 3.18 spec: foldingRange, selectionRange, documentLink, codeLens,
  codeAction, documentSymbol, workspace/symbol, semanticTokens,
  registerCapability; raw markdown fetched to
  `/var/folders/9w/.../T/opencode/lsp318/` from
  `raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.18/...`
- microsoft/vscode-languageserver-node@main (capability builders, `vsc/`).
- neovim/neovim@master `runtime/doc/lsp.txt`, `runtime/lua/vim/lsp/protocol.lua`,
  `runtime/doc/news-0.11.txt`, PR #31311, commits f864b68c / 6072153796d1 /
  #30781 / #32704.
- zed-industries/zed@main `crates/lsp/src/lsp.rs`,
  `crates/project/src/lsp_store.rs`.
- helix-editor/helix@master: `helix-core/src/syntax/config.rs:303`,
  `helix-lsp/src/client.rs` (585-760), `helix-term/src/commands.rs:1382-1440`,
  `helix-term/src/keymap/default.rs`, book `lsp.md`.
- rust-lang/rust-analyzer@master: `src/lsp/capabilities.rs`, `src/lsp/
  semantic_tokens.rs`, `src/handlers/request.rs`, `src/main_loop.rs`.
- Local digests: `lsp_rvben-rumdl-docs-digest.txt:2646`,
  `lsp_rvben-rumdl-src-digest.txt:40480-40500`,
  `lsp_microsoft-vscode-marksman...digest.txt:3170-3190, 11106-11170`,
  `lsp_microsoft-vscode-markdown-languageservice-digest.txt:1556, 5021-5075`.
- In-repo research: 23 §XIII.1, 24 §6/§11, 26 Part II + consumer matrix,
  33 §3, 35, x-lsp-performance-best-practices §3.2, A3-decomposition,
  issues 12/26/27/33.
