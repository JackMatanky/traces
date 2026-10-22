# B7: Client support matrix, registration strategy, shared infrastructure, rumdl coexistence

Pass-2 re-research (cluster 7 of a3-decomposition.md). Independent of `research/27-structural-symbols-editor-intelligence.md`.
Date: 2026-10-22.

## 0. Method

- Every client/server claim below was verified this session against upstream source fetched live via `gh api .../contents/<path>` (base64-decoded) at these refs:
  - `microsoft/vscode-languageserver-node` @ master: the client library VS Code extensions and the built-in markdown server use.
  - `microsoft/vscode` @ master: editor-side registries and built-in markdown extension.
  - `neovim/neovim`: `runtime/lua/vim/lsp/protocol.lua` + `runtime/doc/lsp.txt` @ master, and tags `v0.11.0` / `v0.12.0` for feature-introduction bisects.
  - `helix-editor/helix` @ master **and** tag `25.07.1` (latest release, 2025-07-18) for release-vs-head checks.
  - `zed-industries/zed` @ master: `crates/lsp/src/lsp.rs`, `assets/settings/default.json`, `crates/project/src/lsp_store/*`.
  - `rvben/rumdl` @ main (= 0.3.0 per its Cargo.toml): `src/lsp/server.rs` (the local 0.2.79 digest lacks this file).
- Labels used throughout: **[source/API demand]** = what our own crates/spec/clients require; **[ecosystem practice]** = what peer implementations do (evidence, not obligation).
- Premise checks owned by this cluster: **P9, P10, P11, P13, P14, P15**, verdicts in section 8.

---

## 1. Core artifact: feature x client support matrix

Legend: ✅ advertised **and** actively queried by the editor; ⚠️ supported but off-by-default / opt-in / experimental; ❌ never sent (feature absent from client); ⏳ merged upstream but **not in any release yet**.

Cites are `file:line` in the ref above; "protocol.lua" = neovim `runtime/lua/vim/lsp/protocol.lua` (master); "client <file>.ts" = vscode-languageserver-node `client/src/common/<file>.ts`; "helix client.rs" = `helix-lsp/src/client.rs`.

| # | Feature | VS Code | Neovim | Zed | Helix |
|---|---------|---------|--------|-----|-------|
| 1 | `documentSymbol` | ✅ hierarchical (`documentSymbol.ts`: `hierarchicalDocumentSymbolSupport = true`); providers merged via `LanguageFeatureRegistry.ordered()` (`src/vs/editor/common/languageFeatureRegistry.ts:126-131`) | ✅ hierarchical (protocol.lua:570); `gO` default mapping (`runtime/doc/lsp.txt`) | ⚠️ capable but **default off**: `"document_symbols": "off"` (`default.json:1455`, tree-sitter is the default) | ✅ **flat `SymbolInformation` only**: `hierarchical_document_symbol_support: Some(false)` (helix client.rs:742); feature gated on server cap (client.rs:366-369) |
| 2 | `workspaceSymbol` | ✅ **only client advertising `resolveSupport {location.range}`** (`workspaceSymbol.ts:48`) | ✅ but **no `resolveSupport`** (protocol.lua:615-620), so server must return full `Location`s | ✅ project palette; `resolve_support: None` (zed `lsp.rs` workspace-symbol caps) | ✅ no resolve field set (helix client.rs capability block) |
| 3 | `foldingRange` | ✅ `lineFoldingOnly = true`, kind valueSet comment/imports/region, workspace `refreshSupport = true` (`foldingRange.ts`) | ✅ `lineFoldingOnly = true`, `collapsedText = true` (protocol.lua:470-477), `refreshSupport = true` (:646-648); but **default use is tree-sitter folding**: `vim.lsp.foldexpr()` is opt-in (lsp.txt:906-927, :1218-1241) | ⚠️ capable (`line_folding_only = false`, `collapsed_text`, kinds; zed `lsp.rs`) but **default off**: `"document_folding_ranges": "off"` (`default.json:1448`); **no folding `refreshSupport`** advertised | ❌ `FoldingRange` absent from `LanguageServerFeature` enum (helix client.rs:317-406), so Helix never requests it |
| 4 | `selectionRange` | ✅ (`selectionRange.ts`; drives smart-select) | ✅ (protocol.lua:596-598); `v_an`/`v_in` defaults + `vim.lsp.buf.selection_range` (lsp.txt) | ❌ zero occurrences of `textDocument/selectionRange` in the repo; expand-selection is tree-sitter | ❌ absent from feature enum; expand-selection is tree-sitter |
| 5 | `documentLink` | ✅ `tooltipSupport = true` (`documentLink.ts`); `resolve` wired iff server advertises it; built-in markdown server also provides links (`src/server.ts` capability block) | ✅ **since 0.12 only**: v0.12.0 lsp.txt:91 "`gx` handles `textDocument/documentLink`"; absent at v0.11.0; `tooltipSupport = false` (protocol.lua:529-532) | ✅ **default on**: `"lsp_document_links": true` (`default.json:1747`); `tooltip_support = true`; in-flight `documentLink/resolve` (`crates/project/src/lsp_store/document_links.rs`) | ⏳ **master only**: `DocumentLinks` feature + client caps + `resolve_document_link` all present (client.rs:373, :730, :1284-1310); **not in 25.07.1** (latest release) |
| 6 | `codeLens` | ✅ `resolveSupport {command}`, workspace `refreshSupport = true` (`codeLens.ts:43-45`) | ✅ `grx` default mapping; `resolveSupport {command}` (protocol.lua:464-468); `refreshSupport = true` (:637-639); `vim.lsp.codelens` module (lsp.txt:2158) | ⚠️ capable (resolve + refresh + dedicated proto) but **default off**: `"code_lens": "off"` (`default.json:451`) | ❌ absent from feature enum |
| 7 | `semanticTokens` | ✅ `range: true`, `full.delta: true`, **`multilineTokenSupport = false`**, `overlappingTokenSupport = false`, `serverCancelSupport = true`, `augmentsSyntaxTokens = true`, workspace `refreshSupport = true` (`semanticTokens.ts` fill block) | ✅ core `vim.lsp.semantic_tokens` module (lsp.txt:2809); `multilineTokenSupport = true`, `augmentsSyntaxTokens = true` (protocol.lua:435+); `refreshSupport = true` (:628-630); highlight-token API flagged **experimental** (lsp.txt:906) | ⚠️ capable (full delta + range + refresh) but **default off**: `"semantic_tokens": "off"` (`default.json:1440`) | ❌ absent from feature enum (the vendored `helix-lsp-types` merely carries the protocol types) |
| 8 | `codeAction` | ✅ richest set: `isPreferred`, `disabled`, `data`, `resolveSupport {edit, command}` (`codeAction.ts:40-46`) | ✅ literal + `isPreferred` + `data` + `resolve {edit, command}` + `disabled` + `honorsChangeAnnotations` (protocol.lua:449-463); **the only one of our eight where nvim allows `dynamicRegistration = true`** | ✅ literal + data + resolve rich (zed `lsp.rs` code-action caps) | ✅ gated on advertised kinds (helix client.rs:356-362) |
| 9 | `documentHighlight` | ✅ (`documentHighlight.ts:34`); built-in markdown has `markdown.occurrencesHighlight.enabled` | ✅ (protocol.lua:561-563); CursorHold-driven (lsp.txt `lsp-highlight` section) | ✅ dynamic (zed `lsp.rs:952`); **default-on** with `"lsp_highlight_debounce": 75` (`default.json:363`) | ✅ since 25.07.1 (client.rs:351 in tag; handler `helix-term/src/handlers/document_highlight.rs`; `lsp.auto_document_highlight` config) |

(9 rows because ticket 27's eight features split `documentSymbol`/`workspaceSymbol`.)

### 1.1 Load-bearing cross-cell findings

- **Flat fallback is mandatory, not optional.** Helix (the floor) advertises `hierarchical_document_symbol_support: Some(false)` (helix client.rs:742). Any symbol response must be renderable as flat `SymbolInformation` with `container_name`. rumdl already implements exactly this from one parsed heading model: `document_symbols` (nested), `document_symbols_flat`, `workspace_symbols` all come from `extract_heading_symbols` (rumdl digest `src/lsp/symbols.rs` :40363+; its own doc comment: *"Workspace symbols carry the same heading set the document outline and link navigation use, so all three stay consistent."*). **[source/API demand]**
- **`workspaceSymbol/resolve` is a VS Code-only payoff.** Neovim (protocol.lua:615-620), Zed, and Helix all resolve nothing. Ticket 23's resolve design therefore helps exactly one of four clients; the other three require full `Location`s up front (uris + ranges). **[source/API demand]**
- **Line folding is the norm.** Both VS Code and Neovim advertise `lineFoldingOnly = true`; only Zed is unrestricted (and Zed defaults the whole feature off). Character-granular fold ends are unobservable for the two biggest clients. **[source/API demand]**
- **Multiline semantic tokens: VS Code says no.** `multilineTokenSupport = false` (vscode `semanticTokens.ts`) vs Neovim `true`. If semantic tokens ship (see section 6: MS excludes them from the canonical bar), multiline spans must be split per line for VS Code. Overlap is also `false` in VS Code. **[source/API demand]**
- **Dynamic registration is not required, and is nearly useless for our features on Neovim.** Neovim sets `dynamicRegistration = false` for documentSymbol, foldingRange, selectionRange, documentLink, documentHighlight, codeLens, semanticTokens (protocol.lua cited per row); only codeAction allows it. Static `initialize` capabilities satisfy every client in the matrix. **[source/API demand]**
- **Push-refresh (`workspace/*/refresh`) client support, 3.18:** Neovim advertises foldingRange + codeLens + semanticTokens + inlayHint + diagnostics refresh (protocol.lua:627-648); VS Code advertises foldingRange + codeLens + semanticTokens (per-feature `ensure(...).refreshSupport = true`); Zed advertises codeLens + semanticTokens + inlayHint + diagnostics but **not** foldingRange; Helix refresh caps not verified this pass. Refresh is therefore real, and folding-refresh specifically misses Zed. **[source/API demand]**
- **Position encodings:** VS Code sends `generalCapabilities.positionEncodings = ['utf-16']` only and errors if a server picks anything else (vscode-languageserver-node `client.ts:2170`, `:1488`); Helix prefers UTF-8, then UTF-32, then UTF-16 (helix client.rs:757-761) and follows the server's choice (:410-425); Neovim sends utf-8 first (protocol.lua:357-361); Zed-as-server advertises `[UTF16]` only (zed `lsp.rs`). A UTF-16-only server is safe everywhere; UTF-8 negotiation only pays off for nvim/Helix. Cross-ref: cluster 33 / position-model work. **[source/API demand]**

---

## 2. Registration strategy (premise-adjacent)

**Ecosystem practice: every peer Markdown server is static-first.**

| Server | Feature registration | Notes + cite |
|---|---|---|
| rumdl 0.3.0 | **Static** `initialize` capabilities, gated by init-time config; **one** dynamic registration: file watchers, sent in `initialized` | `src/lsp/server.rs` (fetched from rvben/rumdl@main); gate model: `enable_symbols.then_some(OneOf::Left(true))` etc. |
| Marksman | **Static** + **client detection**: `WorkspaceSymbolProvider = Some(First <\| not clientDesc.IsVSCode)`, same for `DocumentSymbolProvider` (digest :11147/:11149); rename prepare-gated on `SupportsPrepareRename` (:11139) | `ClientDescription.ofParams` inspects InitializeParams |
| MS markdown-ls | **Static** block in `src/server.ts:101-123, 380` | no `registerCapability` anywhere in the fetched server (grep negative) |
| iwe | **Static** block (digest :38732-38792) | incl. `inline_value_provider: Some(true)` advertised |
| markdown-oxide | **Static** block (digest :2047-2100) | settings-gated config; uses a **forked tower-lsp** (Cargo digest :195) |
| rust-analyzer (ref.) | **Static** feature caps; dynamic registration **only** for `workspace/didChangeWatchedFiles` (`crates/rust-analyzer/src/reload.rs:643-651`, `send_request::<RegistrationRequest>`) | feature grep for `registerCapability` in production code: negative |

**Client-side demand:** Neovim's `dynamicRegistration = false` on seven of our eight features (section 1.1) means static advertisement is the only reliable channel there; VS Code and Zed accept static and dynamic equally; no evidence Helix acts on dynamic feature registrations, so treat it as static-only.

**Conclusion [research finding, not a decision]:** static `initialize` capabilities + initialize-time config gates (the rumdl `enable_*` pattern, or Marksman's client detection) satisfies all four clients for all nine features; dynamic registration should be reserved for file watchers, matching every peer. Push-refresh is orthogonal: it needs `Client` refresh methods that **tower-lsp-server 0.23.0 does not have**, only 0.24-rc exposes `code_lens_refresh` / `semantic_tokens_refresh` / `folding_range_refresh` (a2-crates section 1.3, client.rs:288/318/467). markdown-oxide calls `client.semantic_tokens_refresh()` on settings change (digest :1747, :1796), precedent that refresh is used in practice for exactly this "index/config swapped, re-pull" story.

---

## 3. Shared-infrastructure precedent (one parse/model feeds many features)

**[ecosystem practice]**

- **rumdl**: `extract_heading_symbols` produces the `HeadingSymbol` list that feeds (a) nested `document_symbols`, (b) flat `document_symbols_flat` for non-hierarchical clients, (c) `workspace_symbols` over the cross-file index, and the same workspace index feeds link navigation and completions: three protocol surfaces, one parse (digest :40363+, workspace fn :40566; empty query returns *all* symbols, test :40915-40917).
- **MS markdown-languageservice**: clean **library/adapter split**: pure `vscode-markdown-languageservice` library behind the thin `vscode-markdown-languageserver` adapter (adapter spawns in-process: md extension `extension.ts:32-47`). `MdTableOfContentsProvider` is shared: heading-section folding explicitly runs "via a `MdTableOfContentsProvider`" (tool-ms research :22) and fragment/anchor validation uses the same ToC (:32). Feature bar (tool-ms :14): links, doc/workspace symbols, folding, **selection**, completions, references, definitions, rename, code actions, highlights, hover, diagnostics, and **semantic tokens + CodeLens are explicitly out of scope even in Microsoft's canonical implementation** (tool-ms :41), which carries real weight against adding those two speculatively (see P15).
- **rust-analyzer**: one `Analysis` snapshot exposes `file_structure` (crates/ide/src/lib.rs:494), `folding_ranges` (:530), `highlight_related` (:730), `symbol_search` (limit-parameterized, called from `handlers/request.rs:744-747`), semantic tokens: same tree/DB, many views.
- **iwe**: static caps advertise both `document_symbol_provider` and `folding_range_provider` (digest :38750, :38789); folding is really implemented (heading sections, fences, list items; tests :47803-47964), while `inline_value_provider: Some(true)` is advertised (:38766) with no substantive implementation found: an **anti-pattern to avoid** (advertise only what is implemented; P15's honesty rule).
- **markdown-oxide**: counter-precedent: no AST, regex-extraction only (tool-markdown-oxide :9). Its shared substrate is a bag of regexes, not a model. Traces' note/parse model aligns with the rumdl/MS/ra shape instead.

**Implication for Traces [source/API demand]:** the matrix proves the *same* heading/section model must serve nested symbols, flat symbols, workspace symbols, folding (line-only ends), and (if built) links/highlights off one traversal; the rumdl `HeadingSymbol` fan-out is the smallest proven instance of exactly this.

---

## 4. rumdl coexistence substrate (premise P9/P13)

rumdl 0.3.0 `initialize` capability block (fetched `src/lsp/server.rs` @ main):

- **Provides:** textDocumentSync (FULL, willSaveWaitUntil), codeAction (`quickfix`/`source.fixAll`/`source.fixAll.rumdl`), formatting + rangeFormatting, documentSymbol + workspaceSymbol (gated `enable_symbols`), pull diagnostics (identifier `"rumdl"`, `inter_file_dependencies: true`), completion (trigger chars gated `enableLinkCompletions`: backtick always; `(`/`#`/`/`/`.`/`-` only if enabled; `resolve: false`), definition + references + hover + rename(prep) (gated `enableLinkNavigation`), `..Default::default()`.
- **Absent (so no doubling):** documentLink, foldingRange, selectionRange, codeLens, semanticTokens, documentHighlight, inlayHint, executeCommand. Handlers confirm the same set: no handler exists for any of the above.
- **Client caps read:** pull-diagnostics support and `hierarchicalDocumentSymbolSupport`, so it stores a flat fallback and emits `SymbolInformation` for non-hierarchical clients.
- **Config surface** (docs digest :2602-2612): `enableLinting`(true), `enableAutoFix`(false), `enableLinkCompletions`(true), `enableLinkNavigation`(true), `enableSymbols`(true), `linkCompletionContentRoots`, `configPath`, `disableRules`/`enableRules`.

**Doubling against ticket 27's eight [source/API demand]:**

| Feature | Doubling with rumdl? |
|---|---|
| documentSymbol | **Yes** (when `enableSymbols=true`): VS Code outline merges both via `ordered()`; nvim `gO` behavior with two providers not verified |
| workspaceSymbol | **Yes**: both feed VS Code Quick Open / nvim `:lua vim.lsp.buf.workspace_symbol()`; results are merged lists, dedup is a UX concern (cluster 1) |
| codeAction | By method yes, **by kind no**: rumdl emits quickfix/fixAll, 27's planned kinds are disjoint, so the lightbulb unions cleanly (VS Code resolves per-kind; `disableRules` overlap is cluster 5's open risk, out of scope here) |
| foldingRange, selectionRange, documentLink, codeLens, semanticTokens, documentHighlight | **No: rumdl implements none of these** |

**Coexistence levers found:** (1) rumdl's own `enableSymbols` gate is a ready-made "cede symbols to the companion server" switch (P13 confirmed); (2) Marksman proves client-conditional cession in the wild: it disables its symbol providers *for VS Code clients specifically* because built-in markdown already covers them (digest :11147/:11149; tool-marksman :36). Both are precedents for a P10-style decision, not commitments.

---

## 5. Ceilings and constants precedent

- **rust-analyzer**: `workspace_symbol_search_limit: usize = 128`, a **named, config-tunable** constant (crates/rust-analyzer/src/config.rs:1074, wired at request.rs:699, :744-747 `symbol_search(query, limit)`). [ecosystem practice; matches Traces ticket 16's 128 cap, which is **source/API demand** from cluster 1]
- **iwe**: candidate cap `.take(100)` **before** fuzzy scoring in path search (digest :42100-42116): precedent for capping candidates, not final results. [ecosystem practice]
- **rumdl**: **no cap at all**; `workspace_symbols` returns every match and an empty query returns the whole index (digest :40566, test :40915-40917). Absence is also evidence: unbounded workspace symbols is what a naive implementation does. [ecosystem practice]
- **Traces existing named constants**: 5,000-ref cap (ticket 26), 128 workspace-symbol cap (ticket 23); these should stay the canonical names. The other clusters in a3 own them. [source/API demand]
- Not verified this pass: clangd's symbol caps; MS markdown-ls result caps.

---

## 6. Spec mechanics that affect the matrix

- **Resolve pairs:** `documentLink/resolve` is wired by VS Code iff `resolveProvider` (client `documentLink.ts`), implemented by Helix master (client.rs:1302-1310), and in flight in Zed; Neovim's resolve behavior for links is unverified. `codeLens/resolve` is advertised by nvim/VS Code/Zed (`{command}`). `workspaceSymbol/resolve` is VS Code only (section 1). `codeAction/resolve`: all four advertise `edit`/`command` resolution. **Design rule:** resolve is an optimization, never a correctness requirement, except codeAction where nvim/VS Code both expect it to be available when `resolveProvider` is set. [source/API demand]
- **Refresh (`workspace/{foldingRange,codeLens,semanticTokens,inlayHint,diagnostics}/refresh`):** client support per section 1.1; framework gap on 0.23.0 per section 2. markdown-oxide's semantic-tokens-refresh-on-settings-change is the direct behavioral precedent. [source/API demand + ecosystem practice]
- **Partial results:** all nine methods carry `partialResultParams` in 3.18; ticket 26 chose not to implement `partialResultProvider`. No client-side verification done here; with the 128/5,000 caps above, partial results remain optional. [source/API demand, other cluster]
- **Pull diagnostics** are what rumdl uses (identifier `"rumdl"`), already covered by cluster 6; noted only as the one place rumdl and Traces *architecturally* align.

---

## 7. Built-in markdown server (VS Code): the real coexistence baseline

Fetched `microsoft/vscode-markdown-languageserver` `src/server.ts` + md extension `extension.ts`:

- Built-in spawns `vscode-markdown-languageserver` **in-process over IPC** (extension.ts:32-47).
- Its capabilities: diagnostics (pull), codeAction (+resolve), definition, documentLink (+resolve), documentSymbol, **foldingRange**, hover, references, rename (+prepare), **selectionRange**, workspaceSymbol, documentHighlight, and **no codeLens, no semanticTokens** (server.ts capability block; library feature files confirm: `folding`, `smartSelect`, `documentHighlights`, `workspaceSymbols`, and no codelens/semanticTokens file).
- Its settings are per-feature but **have no global kill switch** for symbols/folding/links/highlights (package.json: `markdown.occurrencesHighlight.enabled`, `markdown.suggest.paths.*`, `markdown.validate.*`, `markdown.updateLinksOnFileMove.*`). A user-supplied Traces server on the same language *cannot* turn the built-in one off per-feature; only narrow switches exist. [source/API demand]
- VS Code **merges every provider per feature** (`LanguageFeatureRegistry.ordered()` collects all registered providers for a model: languageFeatureRegistry.ts:126-131; adapters are per-handle: extHostLanguageFeatures.ts:2794 `$provideDocumentLinks(handle, ...)`). Overlapping documentSymbols/folding/links/highlights/actions between built-in markdown and Traces are *merged, not conflicting*; duplicates are a ranking/UX question (P14 confirmed).

---

## 8. Premise-check verdicts (this cluster's: P9, P10, P11, P13, P14, P15)

| Premise | Verdict | Evidence |
|---|---|---|
| P9: rumdl doubles only where it provides features | **Confirmed, narrower than feared** | rumdl has zero of the six non-symbol features; codeAction kinds disjoint (section 4) |
| P10: precedent exists for ceding overlapping features per client | **Confirmed precedent** (not a recommendation) | Marksman disables symbol providers for VS Code clients (digest :11147/:11149); rumdl `enableSymbols` gate (section 4) |
| P11: Helix documentLink status | **Split: merged on master, unreleased** | client.rs:373/:730/:1284 on master; absent from tag 25.07.1 (latest release 2025-07-18) |
| P13: rumdl's init-gate model is reusable | **Confirmed** | `enable_*` gates read at initialize (server.rs; settings digest :2602-2612) |
| P14: VS Code merges multiple providers per feature | **Confirmed** | `LanguageFeatureRegistry.ordered()` (languageFeatureRegistry.ts:126-131); built-in markdown coexists with user servers (section 7) |
| P15: Zed defaults these features off; MS excludes codeLens/semanticTokens | **Confirmed verbatim** | Zed `default.json`: `code_lens` :451, `semantic_tokens` :1440, `document_folding_ranges` :1448, `document_symbols` :1455 (all "off"; `lsp_document_links` :1747 and `lsp_highlight_debounce` :363 are on); MS canonical bar excludes codeLens + semantic tokens (tool-ms :41) |

---

## 9. Open / not verified this pass

- Helix `refreshSupport` client caps; Neovim `documentLink/resolve` call behavior; Neovim `gO` multi-server merge (`buf_request` vs `buf_request_all`); Zed-as-*client* position encodings (Zed-as-server UTF-16-only is verified).
- clangd and MS markdown-ls result caps; whether any editor dedupes identical workspace symbols from two servers.
- iwe's `inline_value_provider` was found advertised without implementation in this pass's grep, but a full no-op confirmation was not performed: treat as "suspected anti-pattern".
- Whether ticket 27 should ship semantic tokens / codeLens at all is **not** this cluster's call, but the two strongest canonical implementations (rumdl: no; MS: explicitly out of scope) plus Zed-off / `multilineTokenSupport=false` economics are recorded above for whoever decides.
