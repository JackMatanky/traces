# D4: Ecosystem conventions research for ticket 27 (Markdown/PKM LSP design)

Research subagent report for grilling Q9 (outline contents) and Q11 (VS Code duplication).
Fetched 2026-10-22. Every web source below was fetched on that date unless marked "local".
Labels: **ecosystem practice** = observed behavior of shipped tools; **source/API demand** = stated
in issue trackers, maintainer replies, or API/docs text.

---

## Section 1: Q1 inventory - what node kinds do Markdown/PKM LSPs put in documentSymbol?

| Server / plugin | documentSymbol kinds | workspaceSymbol kinds | Tasks in outline? | Notes |
|---|---|---|---|---|
| Marksman | Headings only (`SymbolKind.String`); tags nested under headings (`String`) | Headings + tags | No | Source code `tagToDocumentSymbol` uses `String` kind (digest :12508-12535, local). `docs/features.md`: "Document symbols from headings / Workspace symbols from headings" (local). In VS Code, both symbol providers are withheld server-side: `DocumentSymbolProvider = Some(First <\| not clientDesc.IsVSCode)` and same for workspace (digest :11146-11148, local). |
| markdown-oxide | Headings only | Files, headings, tags (`SymbolKind.Constant`), aliases | No | Verified in sibling report `b1-symbols.md` (local, 2026-10-22). Tag kind differs from Marksman (`Constant` vs `String`) as class distinction, not collision avoidance (no source found). |
| vscode-markdown-languageservice (VS Code built-in) | Headings only (`String`, name gets `#` per heading repeat level, digest :4680-4725); link definitions (`Constant`) only when caller passes `includeLinkDefinitions` (optional, default off, digest :4613-4635 and :1556) | n/a (built-in uses document symbols only) | No | This is the built-in's own choice to default link definitions off: the one time the built-in faces an outline-duplication-adjacent choice, it picks opt-in. |
| rumdl | Headings only | n/a (no workspace symbols for outline) | No | Setting `enableSymbols` exists specifically to avoid duplicate headings "when another LSP provides the outline"; Neovim example sets it `false` (digest :2608, :2630-2646, local). |
| iwe | Flat `SymbolInformation[]` from graph paths (no `DocumentSymbol` hierarchy); depth via em-space indent in name | Flat file symbols | No | Digest :40424-40460 (local); sibling `b1-symbols.md`. |
| zk | None. No `textDocument/documentSymbol` at all | None | No | Delegates outline entirely to the editor (local digests; `b1-symbols.md`). |
| Obsidian built-in Outline pane | Headings only | n/a | No | Obsidian help page is JS-rendered and could not be fetched; treated as given via forum threads and the Tasks plugin demand pattern below. |
| Obsidian Tasks plugin | n/a (no outline integration) | n/a | No | Demand pattern is heading-scoped task queries (issue #205, discussion #209) and foldable heading groups (discussion #1012), not outline membership. Tasks are consumed via in-note query blocks. |

### Non-Markdown precedents (prose documents with LSPs)

| Tool | What appears in outline | Relevance |
|---|---|---|
| esbonio (reST) | Sections **plus directives** (docs.esbon.io changelog; issue #374 adds directive symbol info) | Closest shipped precedent for a non-heading node class in a prose-document outline. |
| texlab (LaTeX) | Sectioning commands; environments configurable via `texlab.symbols.allowedPatterns` / `ignoredPatterns` (wiki "Configuration", edited 2024-04-27) | Shows a server-side symbol-filter axis (different coexistence problem: filtering your own symbols). |
| Jupyter (jupyterlab-lsp) | Experimental ToC; its markdown-language-server is "ignored by default (#1176)" | Precedent for a companion markdown LSP standing down inside another tool's files. |
| Org | No mainstream Org LSP exists. `lsp-org` only runs language servers inside `#+begin_src` blocks (alpha); conao3/lsp-org-mode has semantic tokens and folding but no `documentSymbol` task support | Negative precedent: even Emacs' 25-year-old outline format has no LSP task symbols. |

---

## Section 2: Q1 verdict - is headings-only the outline convention?

**Yes. Headings-only is the universal convention for Markdown/PKM LSP document symbols.**
All five servers that implement `documentSymbol` (Marksman, markdown-oxide, MS
markdown-languageservice, rumdl, iwe) emit headings only; zk emits nothing. **No Markdown or
PKM LSP puts tasks in the outline.** The only non-heading outline content anywhere in the space
is: MS link definitions (opt-in, `Constant`), Marksman tags (nested, `String`), and esbonio
directives (reST). **Tasks-in-outline would be Traces-original.**

Stated reasons, with labels:

1. **source/API demand**: mjbvz (VS Code maintainer), microsoft/vscode#201669: "VS Code itself
   targets standard markdown which does not include task lists." Task lists are extension
   territory; related requests were closed as extension candidates (#53993, #67596, #74776).
2. **source/API demand**: artempyanykh (Marksman owner), issue #445 comment 2026-01-24: "The
   reason links are not exposed as symbols is that usually a symbol is a *definition* of
   something. A link is a *use* of a symbol and therefore doesn't show up in the symbols because
   it doesn't have it's own identity... unless you consider 'link + line + col' as the identity."
   The issue itself (2025-12-07) is the only outline-expansion feature request found in the space,
   and it asks for links, not tasks.
3. **Negative search result** (ecosystem practice by absence): no feature request or rejection
   found in marksman or markdown-oxide trackers asking for tasks in the outline. The convention
   looks **unexamined for tasks rather than deliberately rejected**: neither stated reason
   logically excludes tasks. A task is a definition (not a use), and Traces *is* the extension,
   so the "standard markdown has no task lists" boundary is precisely the extension Traces
   provides. Tasks are also the one node class with stable identity (file + position + text),
   unlike arbitrary prose spans.

---

## Section 3: Q2 precedent table - how do LSPs mitigate VS Code built-in duplication?

| Ecosystem | Mechanism | Scope | Label |
|---|---|---|---|
| ESLint | `eslint.enable` master switch; `eslint.validate` (per-language validation list); `eslint.format.enable` **default false** (formatter off by default to avoid fighting other formatters); `source.fixAll.eslint` via `editor.codeActionsOnSave` | Per-capability and per-language, all server/client-registered | Ecosystem practice (package.json fetched 2026-10-22) |
| Prettier | `prettier.enable` (default true); `editor.defaultFormatter` per-language to resolve formatter conflicts; `source.fixAll.prettier` | Same shape | Ecosystem practice (package.json fetched 2026-10-22) |
| Python / Pylance | `python.languageServer` = `Default` / `Pylance` / `Jedi` / `None`; third-party servers require setting `None` to avoid conflict (vscode-python#23640) | Whole-server selection, user-side | Ecosystem practice + source/API demand |
| rust-analyzer / clangd / gopls | Nothing. No built-in competitor exists for these languages, so no coexistence mechanism is needed | Negative precedent | Ecosystem practice by absence |
| markdown-oxide VS Code | Install extension; nothing else. No mention of the built-in markdown extension, no disable instructions, no duplication warning in docs or extension README (docs digest :927-968, :2190-2201, local) | Silent acceptance | Ecosystem practice |
| Marksman VS Code | Server-side client detection: withholds `documentSymbol` / `workspaceSymbol` providers entirely when `clientDesc.IsVSCode` (digest :11146-11148). marksman-vscode README gates activation on `.marksman.toml` presence; no built-in disable instructions | Auto-cede whole capability in VS Code; users get no symbols at all | Ecosystem practice (source code) |
| rumdl | `enableSymbols` setting documented as "Set to `false` to avoid duplicate headings when another LSP provides the outline"; Neovim docs set it false (digest :2608, :2630-2646) | Per-setting gate for companion-server coexistence | Ecosystem practice + stated reason |
| VS Code core | `LanguageFeatureRegistry` merges **all** matching providers; tiebreaker only deprioritizes built-ins in score, never excludes them; "exclusive" selectors can overwrite but are client-internal, unreachable via standard vscode-languageclient dynamic registration | API fact: a second provider always shows | Source/API demand (registry source, sibling `b7`) |
| VS Code core | `outline.showFiles` through `outline.showTypeParameters` per-kind booleans, all `LANGUAGE_OVERRIDABLE` (verified in `src/vs/workbench/contrib/outline/browser/outline.contribution.ts`) | Client-side per-kind hide axis; scoped to `[markdown]` via language blocks | Source/API demand |
| VS Code core | Built-in markdown has **no** per-feature kill switches. Only `markdown.validate.*`, `markdown.occurrencesHighlight.enabled`, `markdown.suggest.paths.*`, `markdown.updateLinksOnFileMove.*` (sibling `b1`/`b7`). The complete client lever is whole-extension disable ("Markdown Language Features", Extensions view gear), which also costs preview, path completions, and validation | Whole extension only | Source/API demand |
| texlab | `texlab.symbols.allowedPatterns` / `ignoredPatterns` server-side symbol filtering (wiki, 2024-04-27) | Filtering own symbols, not coexistence | Ecosystem practice |
| Zed / Neovim | No built-in markdown LSP; duplication only possible between extensions (Zed merges servers, sibling `b7`) | Problem is VS Code-specific | Ecosystem practice |

**Kind reassignment ("SymbolKind dodge"): no precedent found.** No server in the space reassigns
a `SymbolKind` to avoid colliding with the built-in outline. The kind splits that exist
(markdown-oxide tags as `Constant`, MS link definitions as `Constant`, headings as `String`
everywhere) are for class distinction, not collision avoidance. And because the built-in and
Traces would both use `String` for headings, no kind filter can separate them: `outline.show*`
hides by kind, not by provider.

---

## Section 4: Q2 ranked lever set for grilling Q11

Ranked by alignment with ecosystem practice and by cost to Traces' headline value:

1. **Accept built-in heading duplicates + document the `outline.show*` filters honestly**
   (moxide is the de facto precedent: install and say nothing; we do better by documenting).
   Caveat to surface in grilling: kind filters cannot split two `String` sources, so the
   documented escape for a user who wants Traces tasks but no duplicate headings is either
   whole-extension disable or nothing. Tasks (`Array`) and MS link definitions (`Constant`,
   built-in default off) are separable; heading-on-heading duplication is not.
2. **Server-side config gates on Traces itself for duplicative capabilities**
   (precedent shapes: Marksman's client detection; rumdl's `enableSymbols`). Explicit config,
   not auto-cession: Marksman's auto-withholding means VS Code users get zero symbols, which
   would delete Traces' headline value. Gate only the provably-duplicative capabilities
   (folding/highlights need the empirical check from `b2` first; outline duplication is the
   confirmed visible one).
3. **Document whole-extension disable of "Markdown Language Features" as the heavy option.**
   Only complete mitigation for symbols, folding, and highlights at once. Cost: preview, path
   completions, built-in validation. No peer documents this, so Traces would be first; that is
   acceptable as long as it is framed as the heavy option, not the default.
4. **Companion-server guidance**: tell users to set rumdl `enableSymbols = false` when Traces
   provides the outline (rumdl already ships the gate; Traces docs should point at it).
   Same spirit as the Jupyter precedent (markdown LSP "ignored by default").
5. **Rejected**: SymbolKind reassignment to dodge collisions (no precedent anywhere; contradicts
   the kind pins already decided in `b1`); exclusive `LanguageFilter` selectors (unreachable via
   standard vscode-languageclient registration).

**Recommendation for Q11**: the ecosystem-endorsed posture is (1) accept duplicates as the
default (moxide practice), (2) document client-side per-kind filters and the heavy whole-extension
option, (3) provide Traces-side config gates shaped like rumdl's `enableSymbols` for any
capability proven duplicative, (4) never auto-cede VS Code symbols the way Marksman does, and
(5) never kind-dodge. This is also the only posture consistent with the already-decided kind
pins and with tasks-in-outline remaining the Traces differentiator.

---

## Section 5: Sources

All web URLs fetched 2026-10-22.

- https://github.com/microsoft/vscode/issues/201669 - mjbvz statement, standard markdown has no task lists (source/API demand)
- https://github.com/microsoft/vscode/issues/74776 - closed as extension candidate (source/API demand)
- https://github.com/artempyanykh/marksman/issues/445 - full thread: FR body 2025-12-07, artempyanykh "symbol is a definition" reply 2026-01-24, author follow-up 2026-02-08 (source/API demand)
- https://raw.githubusercontent.com/microsoft/vscode/main/src/vs/workbench/contrib/outline/browser/outline.contribution.ts - `outline.show*` kinds, all LANGUAGE_OVERRIDABLE (source/API demand)
- https://raw.githubusercontent.com/Microsoft/vscode-eslint/main/package.json - `eslint.enable`, `eslint.validate`, `eslint.format.enable` default false (ecosystem practice)
- https://raw.githubusercontent.com/prettier/prettier-vscode/main/package.json - `prettier.enable`, `prettier.requireConfig` (ecosystem practice)
- https://code.visualstudio.com/docs/python/settings-reference - `python.languageServer` enum (source/API demand)
- https://github.com/microsoft/vscode-python/issues/23640 - third-party server requires `None` (source/API demand)
- https://github.com/latex-lsp/texlab/wiki/Configuration - `symbols.allowedPatterns` / `ignoredPatterns` (ecosystem practice, edited 2024-04-27)
- https://docs.esbon.io - esbonio directive symbols in outline; issue #374 (ecosystem practice)
- https://help.obsidian.md/Plugins/Outline - JS-rendered, title-only fetch; heading-only treated as given (ecosystem practice, corroborated by forum threads)
- https://github.com/obsidian-tasks-group/obsidian-tasks/issues/205 and discussion #209 - heading-scoped task query demand (source/API demand)
- https://github.com/artempyanykh/marksman, https://github.com/feel-ix-343/markdown-oxide, https://github.com/rvben/rumdl, https://github.com/microsoft/vscode-markdown-languageservice, https://github.com/iwe-org/iwe, https://github.com/zk-org/zk - local digests under docs/digests/ (ecosystem practice, local)
- Sibling reports `b1-symbols.md` and `b7-clients-registration-coexistence.md` in this directory (verified 2026-10-22, local)
