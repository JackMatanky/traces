# B4: Reference-count CodeLens, inlay-hint alternative, documentHighlight (Cluster 4)

Pass-2 research for ticket 27 (F6 + documentHighlight borrowed from F7.8 per dispatch note).
Decomposition: `research/27-pass2/a3-decomposition.md` §Cluster 4 (8 sub-questions) + documentHighlight.

**Independence**: written without reading pass-1 `research/27-*.md` or any other pass-2 cluster report.
Inputs: ticket issues 12/26/27/29/33, `research/26-*.md`, `research/33-*.md`, `docs/digests/*`,
`research/27-pass2/{a1,a2,a3}.md`, local `docs/refs/lsp_spec.md` (stub; spec text fetched upstream).

**Evidence labels**: **[S]** source-verified (fetched code/docs this session) · **[W]** web/secondary
(claimed, not read at source) · **[E]** ecosystem practice (idiom, not a hard requirement) ·
**[D]** LSP-spec demand (3.18 unless noted) · **[L]** local Traces code/ticket.

---

## 1. Premise checks (a3 §4 items owned by this cluster)

| # | Premise | Verdict | Evidence |
|---|---------|---------|----------|
| P4 | Marksman + moxide ship reference-count CodeLens, "default-on" | **Confirmed for capability; "default-on" = no server-side off switch** | **[S]** Marksman `CodeLensProvider = Some { ResolveProvider = None }` (digest :11168); no config knob anywhere in digest (grep `lens.*config` → only library/dispatch lines); moxide `code_lens_provider: Some(CodeLensOptions { resolve_provider: None })` (:2059-2061), no `code_lens` setting in its settings struct (only `semantic_tokens`/`inlay_hints` exist). Both advertise unconditionally; **rendering** is a client decision. |
| P9 | Zed renders CodeLens (since ~2026), supports inlay + documentHighlight | **Confirmed** | **[S]** `code_lens` setting exists (`crates/editor/src/editor_settings.rs:65`), enum `CodeLens { #[default] Off, On, Menu }` (`zed-settings-editor.rs:575-583`), default `"off"` (Zed default-settings JSON :451); editor has full machinery (`crates/editor/src/editor.rs`: `mod code_lens`, `refresh_code_lenses`, `document_highlights_task`, inlay hints, `toggle_code_lens` call :10460); lsp_store registers server→client `CodeLensRefresh`/`InlayHintRefresh`/`DocumentHighlightRefresh` handlers (`zed_lsp_store.rs:1187-1215, :4865-4870`). **[W]** enablement via PR #54100 closing #11565 (~2026-04), flicker follow-ups #56047/#57790. |
| P10 | Helix: no CodeLens (PR #5063 open since 2022); inlay support status | **Confirmed: PR #5063 "feat(lsp): code lenses" is OPEN**; inlay + documentHighlight **supported, both default off** | **[S]** PR page state badge `State--open` + `"state":"OPEN"` JSON; `helix-lsp/src/client.rs` has `document_highlight_provider` (:362), `inlay_hint_provider` (:392), workspace+textDocument inlay client caps (:604, :726), **zero `code_lens` hits**; `helix-view/src/editor.rs:633-658`: `display_inlay_hints: false`, `auto_document_highlight: false`, `inlay_hints_length_limit: None` defaults; book `lsp.md:31,36-37` ("Inlay hints | enable with `display-inlay-hints`", toggles incl. `auto-document-highlight`). |
| P15 | VS Code TS/Java reference-CodeLens default-off | **Confirmed at source** | **[S]** TS: `new ResourceUnifiedConfigValue<boolean>(Config.enabled, false)` + provider only registers under `requireHasModifiedUnifiedConfig(Config.enabled, …)` (`referencesCodeLens.ts:34, :114-119`); Java: `java.referencesCodeLens.enabled: false`, `includeFields: false`, `java.implementationCodeLens: none` (redhat vscode-java `package.json`, parsed). |
| P14 | iwe's `inlay_hints_used` + 100 ms-delayed refresh idiom | **Confirmed at source** | **[S]** `iwe-org-iwe-src-digest.txt:38887, :38901-38909 (`delay_send` spawns thread, sleeps 100 ms, sends), `:38985-39001` (`key_set_changed` → `refresh_inlay_hints()` → only if `inlay_hints_used.load()`), `:39111` (`inlay_hints_used.store(true)` when a request arrived). |
| C1 (added) | Peers ship documentHighlight | **Only Microsoft's library** | **[S]** MS: `MdDocumentHighlightProvider` (:3630-3791) exposed on `IMdLanguageService.getDocumentHighlights` (:1786); Marksman/moxide/rumdl/iwe/zk digests: **0** `document_highlight`/`DocumentHighlight` hits each (grep counts = 0); Marksman capability block omits `DocumentHighlightProvider` (dispatch entry :3174 is library boilerplate). |

**Note on VS Code markdown coexistence for *these* features**: VS Code's built-in
markdown-language-features registers **none** of CodeLens/documentHighlight/inlayHints;
`registerMarkdownLanguageFeatures` lists exactly diagnostics, fileReferences, drop/paste, rename,
paste-links (`extensions/markdown-language-features/src/extension.shared.ts:108-123` **[S]**);
`extension.ts`, `client/client.ts` contain no `register*Highlight*|register*CodeLens*|register*Inlay*`
calls **[S]**. So the built-in MS service cannot double-render our lens/highlight/hints (it can for
documentLink/hover/definition, Cluster 3's premise P6).

---

## 2. Peer precedent at source (the two named servers)

### 2.1 Marksman: `Marksman/Lenses.fs` (digest :8158-8219)

- **Attach points**: every heading (from `doc.Index.headings`) + link definitions, i.e. *all*
  referenceable elements per document **[S]** `Lenses.forDoc` called per `textDocument/codeLens`
  request (:11999-12011: `withState` → `State.tryFindFolderAndDoc` → `Lenses.forDoc`).
- **Eager per-element count**: `buildReferenceLens` runs
  `Dest.findElementRefs false folder doc el |> Array.ofSeq` per element at list time **[S]** (:8176).
- **Zero-suppressed**: tests snapshot only headings with refs; `## No ref` produces no lens
  **[S]** (`Tests/LensesTests.fs` :15564-15582: 3 headings → 2 lenses, `"1 reference"`, `"2 references"`).
- **Payload**: title `humanRefCount` ("1 reference" / "N references") (:8174);
  `data: None`; `ResolveProvider = None` in capabilities (:11168). Command
  `marksman.findReferences` carries pre-serialized `FindReferencesData { Uri; Position; Locations }`
  **only if** client advertises experimental `codeLensFindReferences`
  (`SupportsLensFindReferences`, :12101-12105); otherwise `Arguments = None` (test :15586-15600).
  Lens ranges = heading text spans e. `(0,0)-(0,7)` for `# Doc 1`.
- **executeCommand wart**: `override this.WorkspaceExecuteCommand` returns dummy `JToken.FromObject(0)`
  with comment *"We provide a dummy implementation here because 'showing references' need to be
  handled by the client and it's a bit too much hassle to do this for every client"* (:12013-12019),
  **while advertising `ExecuteCommandProvider = Some { commands = Some [||] }` (empty list)** (:11170), yet the command is handled but never declared. **[S]**
- **Refresh**: never calls `workspace/codeLens/refresh`; only LSP-library plumbing exists
  (abstract member default `ignoreNotification` :1665-1667; forwarding member :1725) **[S]**.
- **Docs claim**: "✅ Code Lens with '# references' on headings" (digest :1234) **[S]**.
- **Config**: none (no lens on/off, no per-kind filters). **[S]**

### 2.2 Markdown Oxide: `src/codelens.rs` (digest :864-946)

- **Attach points**: `vault.select_referenceable_nodes(Some(path))`: file (lens at
  `(0,0)-(0,1)`), headings, indexed blocks, footnotes (docs :2199 "files at the top of the file,
  headings, indexed blocks, footnotes, …") **[S]**.
- **Eager**: `select_references_for_referenceable` per referenceable at list time; **zero-filtered**
  (`.filter(|(_, references)| !references.is_empty())`) **[S]**.
- **Payload**: full `Location[]` pre-serialized into `Command.arguments`
  (`FindReferencesData { uri, position, locations }`), `data: None`, no resolve
  (`resolve_provider: None` :2059) **[S]**. Titles: `"1 reference"`/`"N references"`,
  `"1 reference to file"`/`"{n} references to file "` (trailing space in source) **[S]**.
- **Command routing receipt**: `moxide.findReferences` is *"handled client-side in VSCode via
  `editor.action.peekLocations`. For other editors (e. kakoune-lsp) that send the command back via
  `workspace/executeCommand`, we parse the pre-computed locations and **show the first one**"*
  (:2337-2360). On non-VS Code clients the affordance silently degrades to "jump to first" **[S]**.
  Worse: **issue #343 / PR #363 (closed)**: the server's executeCommand catch-all routed
  `moxide.findReferences` to the daily-note date parser, erroring
  `Could not parse journal format ("moxide.findReferences") as a valid uri` **[S]** (issue body
  fetched). Fix logs a warning and returns `Ok(None)`.
- **Client-floor receipt**: docs state *"we provide a code lens with the count of references to
  headings and files … Note this does not work on Zed and Helix yet"* (docs digest :1596-1597) **[S]**;
  now stale for Zed (post-#54100), but the server's own docs acknowledged the floor.
- **Refresh**: server never sends `workspace/codeLens/refresh`; only `semantic_tokens_refresh` on
  settings change (:1747, :1796) **[S]**. Docs instead ship a **client-side** idiom: capability-gated
  `vim.lsp.codelens.refresh` on autocmds (`codelens_supported(bufnr)` checks
  `server_capabilities.codeLensProvider`, docs digest :132-158 / :859-886) **[S]**, presented as
  "(optional) Enable Code Lens".
- **Open churn receipts**: issues #427 ("Neovim: Proper way to enable codelens in 0.12?") and #469
  ("Update README to reflect NeoVim 0.12+ codelens pattern") are both still **open**; CodeLens setup docs
  repeatedly break across Neovim versions **[S]** (search API).

### 2.3 Server-side absence matrix (digest greps this session, counts = 0)

| Server | CodeLens | documentHighlight | inlay hints |
|---|---|---|---|
| Marksman | **yes** (:11168) | no | no |
| Markdown Oxide | **yes** (:2059) | no | **yes** (`inlay_hints` setting default **true** :1180/:1243; `inlay_hint_provider: OneOf::Left(true)` :2042; handler :2475-2580 emits **embed-block transclusion previews** (`←preview` style), range-filtered, gated on `block_transclusion`, **not** ref counts) **[S]** |
| Microsoft md-ls | no (0 hits) | **yes** (library) | no (0 hits) |
| iwe | no (0; capability block lacks `code_lens_provider`, :38735-38790) | no (0) | **yes** (`inlay_hint_provider: Some(OneOf::Left(true))` :38762) |
| rumdl | no | no | no (0 hits) |
| zk | no | no | no |

iwe anti-pattern pin: advertises `inline_value_provider: Some(OneOf::Left(true))` (:38765) but
`handle_inline_values` returns `vec![]` (:39719) **[S]**; never advertise an empty provider.

---

## 3. Spec demands [D]

**CodeLens** (3.18 `language/codeLens.md`, fetched upstream):
- `interface CodeLens { range; command?: Command; data?: LSPAny }`: **`command` is optional**;
  `data` is *preserved between a code lens and a code lens resolve request* (:92-109). Command-less
  lens + `data` + `codeLens/resolve` is the sanctioned lazy pattern (rust-analyzer's exact use).
- `CodeLensOptions.resolveProvider?: boolean` (:45-50); `codeLens/resolve` "resolve[s] the command
  for a given code lens item" (:116).
- Client capability `resolveSupport` **since 3.18** (:18-24): older clients may not resolve at all;
  a server that *requires* resolve to show any count breaks on pre-3.18 clients (it is unverified whether
  Neovim's built-in client resolves; verify at implementation time; Zed's implementation does async resolve **[W]**).
- `workspace/codeLens/refresh` (3.16): server→client, *"Note that the client still has the freedom to
  delay the re-calculation … if an editor is currently not visible"* (:134) and refreshSupport
  *"should be used with absolute care … for situation where a server … detects a project wide
  change"* (:146-152). The spec itself frames refresh as an exceptional, project-wide hammer.

**documentHighlight** (3.18 `documentHighlight.md`): kept separate from `references` because it
*"is allowed to be more fuzzy"*; `Read`/`Write` for symbol matches, `Text` for fuzzy/textual (:4-6);
kinds `Text=1 | Read=2 | Write=3` (:87-104); errors explicitly permitted (:108); partial result
supported. Registration: `documentHighlightProvider: boolean | DocumentHighlightOptions`.

**inlayHint** (3.17+): `textDocument.inlayHint` + `inlayHint/resolve` with
`resolveSupport.properties` gating late resolution of `tooltip/textEdits/location/command`
(:148-233); `workspace/inlayHint/refresh` mirrors codeLens refresh language (:293) incl.
"client still has the freedom to delay".

---

## 4. Count semantics for Traces (26 coupling) [L]

- **Substrate gap (a1 §3.2/§5 item 6)**: `InlinkMap` is **note-level only**
  (`HashMap<PathBuf, Box<[PathBuf]>>`, no heading-anchor/per-occurrence edges; redb `LINKS` mirrors
  it). Note-level counts computable **today** via `FileEntry::inlinks()` / `InlinkMap::inlinks_of`;
  **heading-level counts impossible without new edges or per-document scans**. The inlink graph is
  the pinned counting substrate (26 answer §2).
- **Nearest-picked divergence (26)**: *"the inlink graph keeps nearest-picked edges (backlink counts,
  25's change-summary soundness, **27's counts**, 28's rewrite targeting all consume them);
  navigation reports ties"* (issue 26 §2, research/26:347/§B4). So a heading count is a **graph
  count, not a resolution count**: equal-distance ties contribute nothing (tie-er notes produce no
  inlink edge today, issue 26 §5), so display counts can be *lower* than `textDocument/references`
  unions (which union every candidate). This divergence must be documented, not silently reconciled.
- **5,000 cap interplay (26 Q7)**: the soft cap is scoped **structurally to the
  `textDocument/references` response path only** (two spine entry points; `EditSet` never truncated;
  named constant fed to 27/29). A CodeLens payload is a *different* path: a moxide-style
  pre-serialized `Location[]` per lens would **bypass the capped driver entirely**: a hub note with
  20k backlinks would serialize an uncapped argument list, and Neovim's measured failure class is
  client-side: *"Neovim's handler bufloads every distinct referenced file (… 2.93s client stall at
  ~100 refs while the server answered in <300ms)"* (research/26:318). **Count-only display + a
  capped references call on demand** sidesteps both.
- **Zero-suppression is universal** (Marksman tests, moxide filter, iwe `refs_counter_hints`
  `if inline_refs > 0`) **[S]**: absence of a lens/hint means "0 refs", "feature off", "client
  doesn't render", and "unresolved", all indistinguishable at a glance. If Traces ships counts, decide
  explicitly whether 0 renders as `0 references` (more honest, noisier); no peer does.
- **Hub-note worst case**: counting via the graph is O(edges of target), which is cheap; the danger is only
  payload materialization (Location[]) and per-heading eager scans over huge documents (moxide and
  Marksman both scan the whole doc's referenceables on every list request; fine at doc scope,
  never workspace scope).

---

## 5. Eager vs lazy: the three sourced models

| Model | Discovery (list) | Count work | Receipt |
|---|---|---|---|
| **Eager** (Marksman, moxide) | ranges + full counts + payload | per visible doc, every re-request | works on *every* client incl. pre-3.18; cost repeats on each provide (clients re-request after every edit, as moxide's nvim autocmds do on TextChanged-class events); no resolve dependency |
| **Lazy resolve** (VS Code TS, rust-analyzer) | spans only, `command: None`, `data` for RA | count at `codeLens/resolve`, **viewport-gated by client** | VS Code: provide debounce 250 ms, resolve debounce 250 ms, `RunOnceScheduler(() => this._resolveCodeLensesInViewport(), …)`; only *visible* lenses are resolved (`codelensController.ts:38-57, :372` **[S]**); RA: cheap annotation pass creates `HasReferences { data: None }` *before any db query* (config-gated early return, `request.rs:1687`), resolve re-derives with **stale-version guard** (`from_proto.rs:121`: `url_file_version != data.version → Ok(None)`) **[S]** |
| **Hybrid** | note-level count eager (O(1) map lookup), heading spans lazy | split | no direct peer; follows substrate (note counts free today, heading counts costly until edges exist) |

RA config nuance **[S]** (`config.rs:335-367`): `lens_enable: true` but reference-count lenses for
adt/enumVariant/method/trait are **default false**; even the IDE that ships CodeLens keeps
*reference* lenses opt-in.

---

## 6. Client matrix (four editors) × three features

| | VS Code | Neovim | Zed | Helix |
|---|---|---|---|---|
| **CodeLens render** | `editor.codeLens` default **true** (`editorOptions.ts:6205-6208` **[S]**); but reference-count *providers* default-off (TS/Java §1) | built-in `vim.lsp.codelens` (enable/get/run; `grx` default map; `LspCodeLens` hl group) **[S]** `runtime/doc/lsp.txt:84,626-635,2158+` | `code_lens: "off"` default; also `Menu` mode (lens shown as code actions) **[S]** | **unsupported** (PR #5063 open) |
| **Lens resolve** | viewport-gated, 250 ms debounces **[S]** | client requests resolve per protocol | async resolve with blank placeholders **[W]** #54100 | n/a |
| **Server→client refresh support** | yes (`workspace.codeLens.refresh`) | yes | **yes** (`CodeLensRefresh` handler `zed_lsp_store.rs:1201-1208`) | (n/a for lens; inlay refresh sent: `client.rs:604`) |
| **Inlay hints** | `editor.inlayHints.enabled` default **'on'** (`editorOptions.ts:3289-3294` **[S]**) | `vim.lsp.inlay_hint.enable` (0.10+) **[S]** | `inlay_hints.enabled: false` default; `edit_debounce_ms: 700`, `scroll_debounce_ms: 50` **[S]** default-settings JSON :845-863 | `display-inlay-hints` default **false** **[S]** editor.rs:634/656 |
| **documentHighlight** | supported (editor feature; debounce-class) | `vim.lsp.buf.document_highlight` + CursorHold autocmd idiom **[S]** | supported; `lsp_highlight_debounce: 75` ms setting **[S]** :361-363; issues/PRs #42793, #61865, #47556 show active support | `auto-document-highlight` default **false** **[S]** editor.rs:636/657, book lsp.md:36-37 |

**Sparse-floor verdict**: CodeLens floor is *worst* (Helix none, Zed/Helix default-off, VS Code
provider-level default-off convention). documentHighlight floor is *highest* (all four implement,
two opt-in by config). Inlay sits between (all four implement, Zed+Helix default off).

**Multi-server coexistence receipt [S]**: Zed issue #61865: "Document highlights query only the
first capable language server"; duplicate/competing markdown servers produce first-wins behavior
for highlights; two lens-emitting servers (e. moxide + Marksman both installed) have no such
arbiter and would double-render. Registration policy (32 coexistence boundary) should assume Traces
is *a* markdown server, not *the* one.

---

## 7. Option space (a3 F6 fork A-F)

| Option | Shape | Pros | Cons / receipts |
|---|---|---|---|
| **A (lazy, default-off-ish)** | list: spans only (`command: None`, `data` = version+anchor); resolve: count | viewport-bounded work (VS Code/RA model); aligns with RA/TS default-off reference-lens precedent; cheap discovery | needs `resolveProvider: true` + 3.18 `resolveSupport` on client for full UX; older clients show bare/blank lenses (RA/Zed placeholder flicker receipts **[W]**); resolve must re-check doc version (RA stale guard) |
| **B (eager zero-suppressed, peer-shaped)** | counts computed at list time, no resolve | identical UX on every CodeLens client; matches both PKM peers (parity argument); no resolve round-trip latency | cost = per-visible-doc scan every re-request; heading counts need substrate first (a1 §4); hub-note Location[] payload must be avoided anyway (§4) → so "eager" should still mean *eager count, never eager Location[]* |
| **C (note-level only)** | single lens at `(0,0)-(0,1)` with backlink count (moxide file lens precedent) | substrate exists **today** (InlinkMap O(1)); never wrong under 26 semantics (graph == note edges); tiny | weaker value (one number per file); heading counts are the interesting part (ticket 27:17) |
| **D (out)** | no CodeLens | zero client/perf risk; MS/zk/rumdl/iwe all here | loses the flagship PKM affordance both peers ship; ticket 27:17 explicitly weighs it |
| **E (inlay hints)** | `‹3›` inline (iwe `refs_counter_hints` shape) at line start, `resolveProvider` for tooltip→refs list | the **only** option that reaches **Helix**; doc-local range → client debounces handle cost; iwe precedent incl. refresh gating | no reference-count precedent beyond iwe; weaker affordance (no peek-locations affordance; mostly non-interactive unless resolve tooltip used); Zed/Helix inlay default-off too → floor win is thin; moxide shows inlay can *also* carry transclusion previews → semantic competition for the same slot |
| **F (both)** | lens + inlay | covers lens-less Helix | double rendering on clients that support both (VS Code/Nvim) with no capability signal to discriminate (servers can't see Zed's `code_lens: off` display setting); needs a user config knob per client → reject unless config-forced |

**Zed's `Menu` mode** (lens as code actions) is a fourth presentation worth remembering for the
recommendation: it degrades gracefully in narrow editors and is a pure client concern **[S]**.

---

## 8. Refresh discipline (F6 sub-question 7)

- **[D]** Spec: refresh is "absolute care", project-wide, client may delay.
- **[E]/[S] Peers**: Marksman and moxide **never** send `workspace/codeLens/refresh` (§2); moxide
  pushes refresh onto the *client* via docs autocmds; iwe gates `workspace/inlayHint/refresh` on
  (a) hints actually requested (`inlay_hints_used`) and (b) `key_set_changed`, debounced by an
  internal 100 ms sleep, and only fires after FS events / `codeAction/resolve` (:38981, :39204);
  RA fires refresh (semantic-tokens/code-lens/inlay/diagnostics, each gated on client support) when
  the DB *"became_quiescent || state_changed"* (`main_loop.rs:545-563`) **[S]**.
- **Why Traces might need it at all**: counts are cross-file: an edit in note B changes the count
  shown in open note A, and A's client will *not* re-request on its own. Proposal:
  1. Track whether any lens-capable editor has the server's lenses (iwe's `inlay_hints_used`
     analogue is easy: set on first `textDocument/codeLens` serve, clear never (or clear on
     config disable).
  2. Fire `workspace/codeLens/refresh` only when the **inlink set of an open document changed**,
     debounced ≥250 ms (align with VS Code's provide/resolve debounce), and never during active
     typing (quiescence gate à la RA).
  3. Gate on client `workspace.codeLens.refreshSupport`; swallow errors (29's refresh policy
     precedent: "unconditional with swallowed error" for diagnostics, 29:17).
  4. If Option B counts are cheap enough, still prefer refresh-on-graph-change over
     refresh-on-every-edit (iwe's `key_set_changed` is exactly this: only when the key→edge
     structure actually moved).

---

## 9. documentHighlight (dispatch-added; normally Cluster 5)

- **Precedent**: Microsoft library only: document-local, ToC + link provider: cursor on a header
  yields header (kind `Write`) + same-document fragment links (`Read`); cursor on a link yields
  other links with same path/fragment; external links, reference definitions
  (def `Write`, uses `Read`) covered; returns `[]` on no match, cancellation-token checked,
  **never errors** (:3630-3791, tests :8813+) **[S]**. Matches ticket 27:24 pins
  ("document-local span matching, never errors on ambiguity") and 26:23's promised
  `NavContext.highlight(target, doc)` spine method (a1 item 9).
- **[D]** Spec explicitly allows fuzzy/textual matching (`Text` kind): perfect for unresolved
  wikilinks and ambiguous textual forms without touching the resolver's ambiguity rules.
- **Cost**: document-local span match: no cross-file work; bounded by doc size; same class as
  document symbols (`<30ms/<10ms` in 33).
- **Registration**: `documentHighlightProvider: true` (static); no resolve, no refresh needed for v1
  (staleness is client-scoped to the open doc; edits re-request naturally).
- **Client floor**: highest of the three features (§6). VS Code's built-in markdown does **not**
  register it (§1 note), so no built-in double-highlight for markdown specifically.
- **Recommendation**: implement: cheapest of the three, spec-sanctioned fuzziness absorbs Traces'
  unresolved/ambiguous cases, and it partially delivers the "cursor-on-heading → see this note's
  occurrences" loop that CodeLens counts only advertise.

---

## 10. Registration, serving, coexistence

- **Static registration** of `CodeLensOptions { resolve_provider }` + `documentHighlightProvider`
  (+ `inlay_hint_provider` only if Option E ships), decided from Traces config at initialize
  (conditional registration precedent: VS Code TS registers the provider only when the user has
  set the enable flag; the server-side equivalent is config-gated advertise). **[E]**
- If any lens ships a command: **advertise it in `executeCommandProvider.commands`** (avoid
  Marksman's empty-list wart) **and** make the server's executeCommand handler route unknown
  commands safely (moxide #343 receipt). Prefer *no* executeCommand at all: count-only lens with
  client-side command convention (VS Code `editor.action.showReferences` / Neovim's lens run) or
  resolve-time command injection (VS Code TS builds the command in `resolveCodeLens` **[S]**).
- **Serving**: note-level = `InlinkMap` lookup; heading-level = per-doc scan until heading edges
  exist (a1 §4; the new-edge option should be its own design decision, not a rider here). Counts
  never call the references driver; "show N references" *clicks* go through the capped
  `textDocument/references` path (26's cap constant) **[L]**.
- **Coexistence**: no conflict with VS Code's built-in markdown for these three (§1); conflict with
  a second user-installed markdown LSP is real (double lenses; Zed highlights first-server-wins).

---

## 11. Latency budget rows (ticket 33 format)

| Operation | Target (p95) | Stretch | Rationale |
|---|---|---|---|
| `textDocument/codeLens` (open doc; note-level count + heading spans) | < 10 ms | < 3 ms | note count = O(target's inlinks); spans from in-memory heading index; **no workspace scan, no per-heading ref enumeration at discovery** (a1 substrate constraint) |
| `codeLens/resolve` (one heading's count) | < 20 ms | < 5 ms | per-heading count via inlink-implicated files with per-file early-stop discipline; viewport-gated by client (VS Code/RA model) so only visible headings pay |
| Eager-all count materialization (only if Option B heading counts are chosen) | amortized into index cold (< 3 s / 1000 files) | n/a | build counts with the graph, never on the request path; incremental after single-file edit ≤ 10 ms (matches 33's re-index row) |
| `textDocument/documentHighlight` (open doc) | < 10 ms | < 3 ms | document-local span match; same class as document symbols (< 30 / < 10 ms), cheaper (no workspace query) |
| `workspace/codeLens/refresh` emission | ≤ 1 per 250 ms per graph change, 0 during typing | n/a | spec "absolute care"; VS Code 250 ms provide/resolve debounce precedent; iwe 100 ms internal delay, RA quiescence gate |
| Option E only: `textDocument/inlayHint` (visible range) | < 30 ms | < 10 ms | client-side debounces dominate UX (Zed defaults 700 ms edit / 50 ms scroll); server must stay well under |
| Lens-count correctness vs references | count-only response work O(edges); full `Location[]` never on lens path | n/a | Neovim bufload stall class (2.93 s @ ~100 refs, research/26:318); uncapped payload bypasses 26's cap discipline |

---

## 12. Performance/failure receipts (honest labels)

- **[S]** moxide #343/#363: pre-serialized CodeLens command misrouted by server's own
  executeCommand catch-all (closed; fix ships).
- **[S]** moxide docs: "does not work on Zed and Helix yet" (stale for Zed; Helix still true).
- **[S]** moxide #427/#469 open: Neovim CodeLens enablement docs churn across 0.12.
- **[S]** Marksman #197 (closed): 2,800-note vault → *"roughly 1 minute and 8 seconds for changes to
  populate in the diagnostics"*: workspace-scale slowness in a markdown LS at 3k files.
  **Label: diagnostics path, not lens-specific**; no lens-specific perf issue found
  (searches: `codelens`, `"code lens"`, `performance/slow/in:title`).
- **[W]** Zed flicker follow-ups #56047/#57790 post-#54100; generic forum advice to disable
  markdown CodeLens in large vaults (AI-summarized search results, **not citable as receipts**).
- **[S]** RA: reference lenses default off + "before any db query" early return + stale-version
  resolve guard: the perf-conscious IDE posture.

---

## 13. Recommendation (proposal for the ticket to accept/modify)

**Staged, substrate-honest:**

1. **documentHighlight: implement now** (§9): cheapest, highest client floor, spec-fuzzy,
   spine method already promised by 26.
2. **CodeLens: Option C first, Option A/B for headings later**: ship the note-level count lens
   (eager, count-only, zero-suppressed like peers, at `(0,0)-(0,1)`, no executeCommand), then add
   heading lenses as **A-style lazy resolve** (command-less + `data` + resolve with doc-version
   guard) once heading-level edges/scans are budgeted. Do **not** pre-serialize `Location[]`
   (moxide payload shape rejected on §4 grounds); "show references" reuses the capped driver.
   Server advertises the capability unconditionally *only if* discovery stays this cheap;
   otherwise config-gated advertise (TS/Java default-off precedent). Client matrix (§6) means the
   ticket should treat Zed-off/Helix-none as the expected floor, not a bug.
3. **Refresh: gated + debounced** (§8): iwe's used-flag + change-flag idiom with a ≥250 ms debounce
   and quiescence gate; never unconditional project-wide on keystrokes.
4. **Option E (inlay): defer; keep as Helix-gap escape hatch**: its unique win is Helix, which is
   also opt-in (`display-inlay-hints`), and it competes for the same visual slot moxide already uses
   for transclusion previews. Revisit only with a config knob, never advertised simultaneously with
   lenses (Option F rejected: no capability signal discriminates client display settings).

**Open questions for the orchestrator/ticket**:
- Count for `0` renders as nothing (peer parity) vs `0 references` (honesty)? Zero-suppression
  coupling documented in §4.
- Heading counts: new heading-level edges vs per-doc scan: index-level decision (touches 26/38),
  this cluster only proves the request-path shape either way.
- Displayed count follows the **graph's nearest-pick** (26 divergence) with a docs note, or is
  suppressed when resolution would report ties? (26 records ties as client-visible for navigation.)
- Neovim pre-3.18 clients: confirm `codeLens/resolve` behavior before relying on Option A
  (label: verification owed at implementation).

---

## Appendix: citation index (primary anchors)

- Marksman: digest `lsp_artempyanykh-marksman-digest.txt` :8158-8219 (Lenses.fs), :11168
  (capability), :11999-12019 (handlers + dummy executeCommand), :12101 (experimental gate),
  :15553-15600 (zero-suppression tests), :1234 (docs claim), :1665-1667/:1725/:3171-3172
  (refresh plumbing only).
- moxide: src digest :864-946 (code_lens), :2059-2061 (capability), :2116-2120 (handler),
  :2337-2365 (command fallback + misroute fix), :2475-2580 (inlay handler), :1180/:1243/:2042
  (inlay setting default true), :1747/:1796 (semantic refresh only); docs digest :132-158,
  :609, :859-886, :1596-1597, :2199; issues #343/#363 (executeCommand misroute), #427/#469 (open).
- iwe: :38735-38790 (capability block; `inlay_hint_provider` :38762, `inline_value_provider`
  :38765), :38887/:38901-38909/:38985-39001/:39111 (refresh gating), :39697-39710
  (`refs_counter_hints` zero-suppressed `‹N›`, `container_hint` `↖text`), :39719 (empty
  inlineValues stub).
- VS Code: `extensions/typescript-language-features/.../codeLens/referencesCodeLens.ts`
  (default-off :34, provide :42, resolve :51-74, label :76-80, conditional registration :114-119);
  `src/vs/editor/contrib/codelens/browser/codelensController.ts` (250 ms debounces, viewport
  resolve :38-57/:372); `codelensWidget.ts:300-322`; `src/vs/editor/common/config/editorOptions.ts`
  :6205-6208 (`editor.codeLens` true), :3289-3294 (inlay `'on'`); markdown extension
  `extension.shared.ts:108-123` (no lens/highlight/inlay registration).
- rust-analyzer (fetched `/tmp/ra`): `request.rs:1687`, `annotations.rs:203`, `to_proto.rs:1759`,
  `from_proto.rs:121`, `config.rs:335-367`, `main_loop.rs:545-563`.
- Zed: `editor_settings.rs:65`, `zed-settings-editor.rs:575-587`, default-settings JSON :361-363
  (:451, :845-863), `editor.rs:19/:1027/:10460/:1066`, `zed_lsp_store.rs:1187-1215/:4865-4870/
  :6011-6037`; issues #11565/#54100/#56047/#57790/**#61865**/#42793 **[S/W as marked]**.
- Helix: `helix-lsp/src/client.rs:362/:392/:604/:726/:1240-1248/:1422-1431`; `helix-view/src/
  editor.rs:633-658`; `book/src/lsp.md:31-37`; PR **#5063 state = open**.
- Neovim: `runtime/doc/lsp.txt` :84, :608-635, :1585-1594, :2158-2208, :2422-2428.
- LSP 3.18 upstream (`raw.githubusercontent.com/microsoft/language-server-protocol` gh-pages):
  `language/codeLens.md` (:45-50, :92-109, :116, :134, :146-152, resolveSupport :18-24),
  `language/documentHighlight.md` (:4-6, :87-104, :108), `language/inlayHint.md` (:293, :148-233).
- Local: `research/27-pass2/a1-codebase.md` §3.2/§5 items 6,9; `a3-decomposition.md` Cluster 4 +
  P4/P9/P10/P14/P15; `research/26-*.md` :294-320 (cap/graph pins), :347/:366; issue 26 §2/§5;
  `research/33-*.md` §Interactive latency (row format), :117-169; issue 27:10/:17.
