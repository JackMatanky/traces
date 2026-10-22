# B3: `textDocument/documentLink` (incl. template references as links)

**Pass 2 (fresh re-research), Cluster 3 / F4 + F5 (merged as F4.6), 2026-10-22.**

Scope: source-verify peer emission sets, the Traces emission-option space, unresolved/ambiguous
policies, resolve laziness, template-reference precedent, client matrix, position plumbing, and
cost rows for ticket 33. Read-only except this file.

**Method / independence**: I did not read `research/27*.md` outside `research/27-pass2/` (per the
pass-2 protocol). Read: `a1-codebase.md`, `a2-crates.md`, `a3-decomposition.md` (task + premises),
issues 15/22, `research/15`, `research/22`, `research/26`, tool digests/boundary files. Claims are
labelled: **[API]** = LSP spec/framework demand; **[SRC]** = verified in peer source; **[ECO]** =
observed ecosystem practice; **[PROPO]** = proposal for 27 to decide (not decided here).
Local sources carry file:line cites; upstream fetches are dated **2026-10-22** (raw GitHub/gh API).

---

## 0. Premise verdicts

| Premise | Verdict | Evidence |
|---|---|---|
| P1 rumdl registers zero documentLink | **CONFIRMED at capability level** | Digest lacks `src/lsp/server.rs` (digest gap); fetched upstream: capabilities block `rvben/rumdl src/lsp/server.rs:479-546` sets sync/codeAction/formatting/symbols(gated)/diagnostics/completion(gated)/definition+references+hover+rename (gated `enable_link_navigation`)/workspace, then `..Default::default()` → `document_link_provider: None`. |
| P2 Markdown Oxide zero documentLink | **CONFIRMED** | zero matches in `lsp_rvben…`-style src+docs digests (`markdown-oxide` digests), re-grepped. |
| P3 iwe zero documentLink | **CONFIRMED** | zero matches in iwe digest; `tool-iwe.md:341-365` capability inventory lists no documentLink. |
| P6 VS Code built-in MS service default-on overlap | **CONFIRMED** | built-in `vscode.markdown-language-features` runs `vscode-markdown-languageserver` over IPC (`extension.ts` serverModule block) and document links are live by default (`src/test/documentLink.test.ts`: `executeLinkProvider` on `[b](b.md)` → navigates). **No per-feature disable setting exists**, full `package.json` config property list has no `markdown.server.*`/documentLink toggle; only whole-extension disable. |
| P7 client affordances | **CONFIRMED with corrections** | Neovim: 0.12+ only; Zed: merged 2026-05-26; Helix: master-only/unreleased; VS Code: target-less links render a *dead* underline (§4.1). |
| P11 zk `:2446/:3289` "semantic tokens" | **MISREAD confirmed** | those lines are `internal/adapter/handlebars/helpers/prepend.go` (a FILE header) and a `document.GetLines` helper, nothing to do with semantic tokens or documentLink; line-drift citation from prior research. |

---

## 1. [API] Spec facts (LSP 3.18, `documentLink.md` fetched 2026-10-22)

- Client capability `textDocument.documentLink`: `dynamicRegistration?`, `tooltipSupport?` (since 3.15).
  Tooltip is *advisory*, spec says if provided it is shown in a string that includes trigger
  instructions (`{0} (ctrl + click)`), wording varies by OS/settings/localization.
- Server capability `documentLinkProvider?: { resolveProvider?: boolean }`; registration options =
  `TextDocumentRegistrationOptions + DocumentLinkOptions` (so dynamic registration is spec-legal).
- `DocumentLink = { range, target?, tooltip?, data? }`. `target?` doc: *"If missing, a resolve
  request is sent later."* `data` is *"preserved … between a DocumentLinkRequest and a
  DocumentLinkResolveRequest."*
- Resolve round-trip: `documentLink/resolve` takes the full `DocumentLink` (incl. `data`),
  returns `DocumentLink`. **Unlike CodeLens, the spec carries no "creation is cheap, resolve later"
  guidance**, the two-stage shape is implied by `target?`/`data` only.
- No protocol concept exists for "this link is broken/ambiguous", the response is a bare array;
  absence of the link, absence of `target`, or `tooltip` are the only expressive channels.

---

## 2. [SRC] Peer emission sets

### 2.1 MS vscode-markdown-languageservice, the maximally-capable pole

Digest: `docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt`.

- Emission inventory `MdLinkComputer.getAllLinks` (`:4030-4053`): inline links+images
  (`#getInlineLinks`, incl. images *nested in link text* and reference links inside image text),
  reference-style **uses** (`#getReferenceLinks`), reference **definitions**
  (`#getLinkDefinitions` `:4214-4244`, emitted as links when `createHref` parses; existence not
  checked; unparseable hrefs dropped `:4231-4233`), autolinks (`#getAutoLinks`, external-only),
  HTML `src`/`href` (`#getHtmlLinks`). `NoLinkRanges.compute(tokens)` (`:4036-4037`) suppresses
  everything inside code blocks/inline code/HTML.
- **Range shape = `link.source.hrefRange`**, the destination only, not the whole `[text](…)` span
  (`#toValidDocumentLink` `:4517-4560`).
- Eager/lazy split (`#toValidDocumentLink`): External → `target` eager, no tooltip. Internal →
  `target: undefined`, `tooltip: 'Follow link'`, `data: link` (full MdLink). Reference **use** →
  **dropped if no definition**; else eager target (open-at-definition command/URI) +
  `tooltip: 'Go to link definition'`.
- Resolve (`resolveDocumentLink` `:4390-4414` + `#resolveInternalLinkTarget` `:4447-4494`):
  revives `data`; `stat`-based existence with `.md`-extension tack; **never drops**, non-existent
  file still returns `{kind:'file', uri}` (no position). Fragment handling: `#L<line>,<col>` via
  `parseLocationInfoFromFragment`, else TOC lookup (`toc.lookByLink`, HTML-id fallback `:4479-4492`)
  → open-at-position. Targets become VS Code `command:` URIs or `file://…#L…` URIs
  (`#createOpenAtPosCommand` `:4564-4586`).
- Cache: per-doc versioned `MdDocumentInfoCache` (`:4341-4355`); `provideDocumentLinks` =
  `coalesce(links.map(toValidDocumentLink))`.

### 2.2 zk, the lean/eager pole

Digest: `docs/digests/zk-src-digest.txt`.

- Parser (`:3357-3363`): goldmark + Footnote + WikiLinkExt only (no Linkify, no TagExt).
- `DocumentLinks()` (`:3406-3463`) walks the AST and emits **only** `*ast.Link` (skips empty href,
  `file:///`, `magnet:?`; percent-decodes; range via `markdown.GetLinkPosition` = full link span)
  and `*extensions.WikiLink` (no scheme filter; raw `StartOffset/EndOffset` range). **Not emitted**:
  images (`ast.Image` not cased), autolink footnote refs, HTML, definitions, embeds beyond
  WikiLink-shaped ones. Enabled footnotes are parsed but not emitted.
- Handler `TextDocumentDocumentLink` (`:4418-4456`): external URLs eager; internal links **only if
  `noteForLink` resolves**, `if target != ""` gate means **unresolvable internal links are
  silently dropped**. No `tooltip`, no `data`.
- **Advertise/impl mismatch confirmed**: `capabilities.DocumentLinkProvider = &protocol.
  DocumentLinkOptions{ResolveProvider: boolPtr(true)}` (`:4256-4258`) but **no
  `handler.DocumentLinkResolve` exists anywhere** (only `CompletionItemResolve` `:4362`), a client
  that calls resolve would hit an unimplemented method. Harmless in practice because all emitted
  links carry eager targets.
- **Three channels, same span**: `DocumentLinkAt` (`:3340-3343`, re-parses `DocumentLinks()` per
  call, no cache) is reused by Hover (`:4383`), Definition (`:4459-4487`), and the link list.
  Also: zk's Hover returns the *target file's full contents*, not our model, but shows how cheap
  peers treat the shared span.

### 2.3 Marksman: **correction to prior research**

- The `textDocument/documentLink` + `documentLink/resolve` hits at digest `:3175/:3177` are
  **not** Marksman registration, they sit in `defaultRequestHandlings ()` of the *vendored Ionide
  LanguageServerProtocol framework* (verified upstream: `LanguageServerProtocol/
  LanguageServerProtocol.fs:242-244`). The framework's default member is
  `default __.TextDocumentDocumentLink(_) = notImplemented` (`LanguageServerProtocol/Server.fs:506-508`).
- Marksman's own `ServerCapabilities` (digest `:11145-11180`) does **not** set
  `DocumentLinkProvider`, and Marksman never overrides `TextDocumentDocumentLink` (gh code search:
  zero hits outside the vendored lib).
- **Net: Marksman serves zero documentLinks.** The "implemented but unadvertised" reading from
  pass 1 was wrong, it is framework boilerplate, not an implementation.

### 2.4 rumdl: see §0/P1 (capability-level confirmation; digest alone was insufficient because
`src/lsp/server.rs` is absent from the digest, enumerate `^FILE:` markers before trusting digest
absence for rumdl again).

### 2.5 Markdown Oxide / iwe, zero, per §0 (P2/P3).

### 2.6 Spectrum

Lean pole = zk (2 syntax kinds, eager, existence-gated drop, no tooltip/data, phantom resolve
advert). Max pole = MS (6 generators, lazy internals + data + resolve, versioned cache, l10n
tooltips, reference-definition jumps, HTML). **No peer ever emits an ambiguous-target link**
(zk drops unresolved; MS doesn't resolve existence at emission), and **only MS emits target-less
links** (backed by resolve + data). Marksman serves nothing; rumdl/moxide/iwe serve nothing.

---

## 3. [ECO] Template-LSP survey (web, 2026-10-22)

- **jinja-lsp (uros-5)**, cloned `/tmp/jinja-lsp`: zero `document_link` matches anywhere;
  capabilities (`jinja-lsp/src/channels/lsp.rs:66-104`): definition, references, codeAction,
  executeCommand, documentSymbol, hover, completion. **No documentLink.** Template navigation =
  goto-definition only.
- **jinja-ls (noamzaks)**, `packages/server/src/server.ts:102`:
  `documentLinkProvider: { resolveProvider: false }` **and** `definitionProvider: true`;
  `documentLinks.ts` emits `target: uri` **only when `uri !== undefined`** (existence-gated,
  unresolved dropped, eager, no resolve). **Same include/extends/import span served on BOTH
  channels.** The one real precedent for F4.6.
- **fjkit**, not a findable project (searches return nothing relevant); not a template LSP
  candidate. Do not cite it as one.
- **Practice (n=1 + n=1)**: template refs either on **both** channels (eager, existence-gated) or
  on definition only. Nobody serves documentLink alone for templates.

---

## 4. [ECO] Client matrix (all verified 2026-10-22)

### 4.1 VS Code (+ built-in markdown overlap, P6)

- **Built-in markdown is a real LSP** (`vscode-markdown-languageserver`, IPC worker) and emits
  documentLinks for standard links by default; **no per-feature setting** can silence just that
  (full config property list checked). Only disable-whole-extension.
- **Multi-provider merge**: `src/vs/editor/contrib/links/browser/getLinks.ts`, `LinksList`
  constructor merges all providers; `_union` **drops the older link when ranges intersect or
  touch** → the **later-registered provider wins** for overlapping ranges. Registration order ≈
  activation order (built-in markdown activates first, third-party Traces later), so Traces would
  likely *override* the built-in for identical spans, **but order is not contractual**; do not
  depend on which target wins, depend on spans overlapping so only one underline ever renders.
- **Target-less behavior**: decorations are created for *every* returned link, url or not
  (`links.ts:160-177`); hover shows `tooltip ?? 'Follow link' (ctrl + click)` as plain text when
  url is absent (`links.ts:363-395`); click → `Link.resolve` → provider `resolveLink` or
  `reject('missing')` (`getLinks.ts:34-54`) → **dead underline: looks interactive, does nothing**.
- Client caps (vscode-languageclient `documentLink.ts:37-41`): `dynamicRegistration: true`,
  `tooltipSupport: true`. Resolve called only when `target` is undefined (lazy).

### 4.2 Zed

- **PR #56011 "Support LSP document links" merged 2026-05-26** (closes #33587, completed).
- Setting: `lsp_document_links: true` default, *"Whether to query and display LSP
  `textDocument/documentLink` links"*; `"lsp_document_links": false` turns it off
  (`assets/settings/default.json:1744-1748`).
- Client caps (`crates/lsp/src/lsp.rs:1118-1121`): `dynamic_registration: true`,
  `tooltip_support: **true**`.
- Full resolve flow: `LspDocumentLink { range, target, tooltip, data, resolved }` cached **per
  server** (`crates/project/src/lsp_store/document_links.rs:28-37`); resolve request built from
  cached link incl. `data`, gated on upstream `resolveProvider` (`:426-455`); result merged back
  (`:497-504`). Refresh handler wired (`lsp_store.rs:4869`).

### 4.3 Neovim

- **0.12+ only**: `release-0.12` `runtime/doc/lsp.txt:91` *"`gx` handles
  `textDocument/documentLink`"*; `runtime/doc/news-0.12.txt:232` same; **zero matches in
  `release-0.11`** (both fetched HTTP 200). Latest stable **v0.12.6 (2026-10-09)**.
- Client caps (`runtime/lua/vim/lsp/protocol.lua:531-534`): `dynamicRegistration: false`,
  `tooltipSupport: **false**`.
- **gx behavior** (`runtime/lua/vim/ui.lua:237-271`): `buf_request_sync` per keypress (no cache),
  collects links whose range contains the cursor **and `document_link.target` is non-nil**;
  `file://` → filename. **No `documentLink/resolve` call anywhere** → **target-less links are
  invisible to Neovim's gx**; resolve must not be load-bearing for gx reachability.

### 4.4 Helix

- **Master-only**: PR #15235 "feat(lsp): documentLink highlights" merged **2026-03-14** (supersedes
  closed #15215; body: reuses `goto_file`, preferring LSP links, falls back to native logic).
  Latest release **25.07.1 (2025-07-18)** → **no shipped release has documentLink**.
- Client caps (`helix-lsp/src/client.rs:730-733`): `dynamic_registration: false`,
  `tooltip_support: **false**`. Feature gated on server advertising (`client.rs:373`); methods
  `text_document_document_link` + `resolve_document_link` exist (`:1284-1310`).
- Behavior: 250 ms debounced fetch on document change, cached per doc
  (`helix-term/src/handlers/document_links.rs:14-60`), rendered as **editor highlight decorations**
  (`helix-term/src/ui/editor.rs:497` `doc_document_link_highlights`), consumed by navigation.

### 4.5 Matrix + sparse-floor implication

| Client | documentLink | resolve | tooltipSupport | affordance | since |
|---|---|---|---|---|---|
| VS Code | yes (+ built-in md provider) | lazy, only if `target` missing | **true** | hover underline + ctrl/cmd+click; **target-less = dead underline** | current |
| Zed | yes | full on-demand resolve w/ cache | **true** | clickable links, setting to disable | 2026-05-26 merge |
| Neovim | yes (gx only) | **never called** | **false** | `gx` opens target at cursor; **target-less ignored** | 0.12 (0.11: none) |
| Helix | yes (highlights + goto_file) | supported, gated `resolveProvider` | **false** | underline highlights | master only (rel. ≤25.07.1: none) |

Sparse floor: Helix-release and Neovim-0.11 users get **nothing** from documentLink but always
still get goto-definition → for template refs this is a concrete argument for serving **both**
channels (§7).

---

## 5. [PROPO] Emission-set option space for Traces

Premises: 26 pins *emit for `Resolution::Unique` **and** `Resolution::Ambiguous`, never error on
ambiguity* (26 research, Inputs-from-27 row); 15 blocks 27 "for document-link ranges"
(issue 15, Blocks line); ticket wording pushes *"every resolvable link/embed target"* (a3 F4.2);
coexistence rule = parseable-syntax-disjoint from rumdl (now trivial, rumdl emits none).

| Syntax class | MS emits | zk emits | built-in VS Code overlap | Recommendation |
|---|---|---|---|---|
| Wikilinks `[[t]]` / `[[t#h]]` | no | **yes** | no (built-in doesn't parse wikilink syntax) | **emit**, Traces/15-owned syntax, zero overlap |
| Embeds `![[t]]` | no | no (Image/WikiLink `!` not cased) | no | **emit**, no peer precedent; anchor on 26 occurrences |
| Cross-file `file.md#heading` | yes (internal lazy) | yes | **yes → dedup collision** | **emit** (ticket wording); accept VS Code range-dedup; spans must overlap built-in's hrefRange to collapse to one underline |
| Same-doc `#anchor` | yes | yes (as internal) | **yes → dedup** | **emit**; rumdl MD051 owns *diagnosis*, we own *pointer* (documentLink renders ambiguity, never errors, 26 read-path rule) |
| Frontmatter wikilinks | no | no | no | **v1 out**, feeds only after 26 A16 extraction seam exists (a3 line 119) |
| Images `![](x.png)` | yes | no (`ast.Image` uncased) | yes (built-in) | **emit** (file URI, eager); low collision cost |
| External/absolute URLs | yes eager | yes eager | yes | **emit** eager; never unresolved |
| Autolinks `<https://…>` | yes (external) | yes (core parses as Link; bare-URL Linkify *not* enabled in zk's doc parser) | yes | **emit** (cheap) |
| Reference uses `[t][r]` | yes, dropped if no def | no | yes | **emit iff definition exists** (MS rule, dangling ref is 25's diagnostic territory, not a pointer) |
| Reference definitions `[r]: url` | yes (href jump) | no | yes | **emit**, click-to-jump to href; ms-precedented, cheap |
| HTML `src`/`href` | yes | no | yes | **v1 out**, no HTML extraction in Traces AST; divergence from MS is fine (built-in covers it in VS Code) |
| Template `{% include/extends %}` | n/a | n/a | n/a | **§7**, both channels, gated on span source + overlay existence |

**Lean vs maximally-capable balance.** zk-lean (2 kinds, drop everything unresolvable) forfeits the
ticket's "every resolvable link" wording and the ambiguous-emit pin; MS-max (lazy+resolve+data)
pays complexity we don't need (§6) and is invisible to Neovim anyway. **[PROPO]** target the
*middle*: eager targets for everything in the table (resolution already computed by 26's spine),
MS-style drop rules where a definition/target is required, zk-style honesty on ambiguity via
tooltip (§6). Two decision knobs remain: (a) standard-link inclusion (recommend **include** dropping them deprives Neovim/Helix/Zed users who have *no* built-in markdown provider; VS Code's
merge dedups overlaps anyway); (b) **range shape**, MS uses hrefRange, zk uses the full link
span; **[PROPO]** emit the span 26's Link Occurrence already carries (single source of truth for
27/26/24), and verify it overlaps the built-in's hrefRange so VS Code `_union` collapses them.

---

## 6. [PROPO] Unresolved / Ambiguous / tooltip / resolve-laziness

**Unresolved internal.** Options: (i) **drop** (zk; no double affordance, 25's
`links.wikilink.unresolved` + 15's "Create note" action *are* the unresolved affordance);
(ii) emit target-less (MS-style), but MS only does this with `data` + resolve behind it, and the
client evidence is hostile to target-less: VS Code dead underline, **Neovim gx silently ignores
it**, Helix highlights something unopenable, Zed caches it and needs our resolve. **[PROPO]
= drop unresolved internals (incl. non-existent template refs), always emit externals.**
Diagnostics (25) and code actions (15) carry the unresolved UX; documentLink stays a pointer.

**Ambiguous (Unique+Ambiguous emit, never error, 26 pin).** No peer precedent (nobody emits
ambiguous links). The link cannot carry N targets: (i) `target` = deterministic first candidate
(per 26's comparison-key/folder-distance pick) + tooltip "3 matches, `gd`/references to
disambiguate"; (ii) target-less + tooltip (falls into every dead-affordance problem above);
(iii) drop (violates the pin). **[PROPO] = (i)**, flagged for sign-off: "ties stay undisambiguated"
(26 D1) governs references/diagnostics; a *navigation* pick-first with honest tooltip does not
change stored resolution state, but 26+15 must confirm the wording.

**Tooltip.** Capability-gated (§1): VS Code/Zed show it; Neovim/Helix advertise false and ignore
it. MS precedent: `'Follow link'` (internal), `'Go to link definition'` (reference use).
**[PROPO]**: ambiguous → `"N candidates, use references"`; unique internal → optional `'Follow
link'`; externals → none (MS omits them too). Never rely on tooltip for correctness (2 clients
drop it).

**Resolve laziness.** Price both:
- *Eager, `resolveProvider: false`*: resolution already exists per 26's spine (`Occurrences +
  Resolution` during locate); a note has tens of links; **Neovim requires eager targets** (never
  calls resolve); Helix/Zed/VS Code all work; zk/jinja-ls precedent; zero round-trip on gx's
  critical path (nvim does a *synchronous* documentLink request per keypress).
- *Lazy, `resolveProvider: true` + `data`*: only pays if target computation needs workspace
  I/O (MS stats files, opens docs, TOC lookups). Our targets come from an in-memory index +
  overlay, no I/O to hide. VS Code/Zed/Helix support it; **Neovim still can't reach lazy links**,
  so laziness can never be load-bearing for unique targets.
**[PROPO] v1 = `resolveProvider: false`, eager targets everywhere.** Revisit only if a future
target class needs deferred workspace lookup, and even then keep unique targets eager.

---

## 7. [PROPO] Template references as document links (F4.6 / ticket's fifth bullet)

- **Precedent**: jinja-ls = **both channels**, eager, existence-gated (§3); jinja-lsp =
  definition only; fjkit nonexistent. Layering argument holds: documentLink = pointer
  (click/gx/gf), definition = keyboard (`gd`), zk already serves documentLink+definition+hover
  from one `DocumentLinkAt` for its link spans, so dual-channel is idiomatic, not redundant.
- **[PROPO] both channels**, matching jinja-ls: emit `{% include %}`/`{% extends %}` spans as
  documentLinks *and* serve definition at those positions. Sparse-floor §4.5 makes this near-
  mandatory: Helix-release / nvim-0.11 users lose documentLink but keep `gd`.
- **Existence (22 Q20(b) seam)**: documentLink cannot diagnose → apply the §6 Unresolved rule:
  exists-on-disk∪overlay-buffer set (virtual children included) → emit with file target; missing →
  **drop** (25's diagnostics carry non-existence). The overlay set must be consulted at emission
  or we false-drop unsaved buffer-only templates.
- **Dynamic targets** (`{% extends layout %}`, 22 Q14 honest false-negative): target not statically
  computable → **drop** (consistent with Unresolved-drop; a target-less emit would be dead in
  3 of 4 clients). Definition at position remains the resolution path for dynamic cases.
- **Span fidelity / sequencing**: 22's machinery-gated parse + span tracker gives exact byte spans;
  until it lands, template spans would be line-granular → **[PROPO] gate template documentLinks on
  22's span source**; definition (position lookup) can ship earlier since it doesn't need an
  emission-time span list. Do not ship line-granular underlines.
- The Q24(b) flagged line from 22 (include/extends spans as documentLink candidates + overlay-aware
  existence + re-evaluate-the-edge clause) is hereby **answered by this section**, restated at 27's
  resolution per 22's timing-fallback clause.

---

## 8. Position plumbing & cost rows (for ticket 33)

- **Plumbing**: full `Range`s flow through 26's `convert` (UTF-16 per client `offset_encoding`);
  `LineIndex` from `ContentResolver::text_of`; ranges derived from 26's Link Occurrence spans
  (one source, no second span computation); emission list cached per (uri, version) like 24's
  snapshot reuse (mirrors MS versioned cache, Helix debounce, Zed per-server cache).
- **Latency rows [PROPO]** (anchors: doc symbols <10 ms, hover <2 ms, find-refs ≤20 ms; links per
  doc = tens; walk is cheaper than symbol tree assembly):

  | Row | p95 warm | p95 cold | Notes |
  |---|---|---|---|
  | `textDocument/documentLink` (per doc) | **≤5 ms** | ≤10 ms (cold-parse path, same anchor as doc symbols) | nvim `gx` issues this **synchronously per keypress** → warm-cache serving is a UX requirement, not an optimization; Helix/Zed/VS Code debounce/cache on their side |
  | `documentLink/resolve` | n/a v1 | n/a | `resolveProvider: false`, no row exists; a future lazy class adds one ≤2 ms |

- **Ceilings**: never merged into `SYMBOL_CEILING` (1k/5k) or `REFERENCE_CEILING` (5,000) different families. Per-doc link counts are tens (MS/zk have **no** link cap); **[PROPO] v1 no
  cap**, and if 33 wants one it should be a new `LINK_CEILING` row (e.g. 1k/doc, truncate + log),
  decided there, not here. `concurrency_level(1)` risk: documentLink shares 26's spine pass serve from cache so the on-path cost is a lookup, never a re-walk.

---

## 9. Coexistence rows (feeds 32)

| Provider | rumdl | VS Code built-in | Traces action |
|---|---|---|---|
| documentLink | **none** (P1, capability-verified) | active by default for standard links (P6); **no per-feature toggle** | no new joint-setup toggles needed for rumdl; for VS Code accept range-dedup overlap (§5), optionally document that disabling built-in markdown extension removes *its* links, not ours |
| definition/hover/refs/rename | gated `enableLinkNavigation` (still "on" by default per docs digest :2606-2646) | built-in markdown too | (26's lane; noted here only because zk-style multi-channel reuse means documentLink can share its spans) |

---

## 10. Decision list for ticket 27 (open questions this report hands off)

1. Emission set: **Option A (all resolvable links, incl. standard)** vs syntax-disjoint Option B recommendation A (§5).
2. Range shape: 26 Occurrence span (recommended) vs MS-style hrefRange vs zk-style full span;
   must overlap built-in's span for VS Code dedup (§5).
3. Unresolved internals: **drop** (recommended) vs target-less emit (§6).
4. Ambiguous targets: **deterministic first + count tooltip** (recommended, needs 26/15 sign-off)
   vs target-less vs drop (§6).
5. `resolveProvider: false` + eager v1 (recommended) vs data-carrying lazy (§6).
6. Template refs: **both channels**, existence-gated, dynamic→drop, span-source-gated (§7);
   Q24(b) answered here.
7. Frontmatter wikilinks / HTML links: v1 out, gated on A16 seam / N/A respectively (§5).
8. Reference uses + definitions: emit per MS rules (§5), confirm vs 15's settled model.
9. 33 rows: documentLink p95 ≤5/≤10 ms; LINK_CEILING question deferred (§8).

---

## Sources

**Local**: `docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt`
(:4030-4053, :4214-4244, :4341-4494, :4517-4586, :18802 NoLinkRanges, :1528-1549 API);
`docs/digests/zk-src-digest.txt` (:3340-3463, :3357-3363, :4256-4258, :4383, :4418-4487);
`docs/digests/lsp_artempyanykh-marksman-digest.txt` (:3175/:3177 dispatch, :11145-11180 caps);
`docs/digests/lsp_rvben-rumdl-src-digest.txt` + `-docs-digest.txt` (:2606-2646 toggles);
`docs/digests/lsp_rvben-rumdl…` FILE-list check; `tool-rumdl-boundary.md`, `tool-iwe.md:341-365`;
`docs/refs/lsp_spec.md:649` (stub); issues 15 (Blocks line), 22 (Q24(b) Wiring para), research
15/22/26, a1/a2/a3 (pass2).

**Upstream (fetched 2026-10-22)**: LSP 3.18 `language/documentLink.md` (microsoft/
language-server-protocol gh-pages); vscode `src/vs/editor/contrib/links/browser/{getLinks,links}.ts`,
`extensions/markdown-language-features/{package.json,src/test/documentLink.test.ts,src/extension.ts}`;
vscode-languageserver-node `client/src/common/documentLink.ts`; zed PR #56011 + merged 2026-05-26,
`assets/settings/default.json:1744-1748`, `crates/lsp/src/lsp.rs:1118-1121`,
`crates/project/src/lsp_store/document_links.rs`; neovim `release-0.12`/`release-0.11`
`runtime/doc/lsp.txt`, `news-0.12.txt:232`, `runtime/lua/vim/lsp/protocol.lua:531-534`,
`runtime/lua/vim/ui.lua:237-271`; helix PR #15235 (merged 2026-03-14) / #15215 (closed),
releases API (latest 25.07.1), `helix-lsp/src/client.rs:373/:730-733/:1284-1310`,
`helix-term/src/handlers/document_links.rs`, `helix-term/src/ui/editor.rs:497`;
rumdl upstream `rvben/rumdl src/lsp/server.rs:469-546`; marksman upstream vendored
`LanguageServerProtocol/{LanguageServerProtocol.fs:242-244, Server.fs:506-508}`; jinja-lsp
(uros-5) `src/channels/lsp.rs:66-104`; jinja-ls (noamzaks) `server.ts:102`, `documentLinks.ts`.
