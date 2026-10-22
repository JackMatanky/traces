# Cluster 1, Document & workspace symbols (F1.1-F1.9)

Subagent: **B1** (pass 2). Independence: did not read pass-1's `27-structural-symbols-editor-intelligence.md` or any `27-pass2/` file other than `a3-decomposition.md` (the plan).

**Source classes used:** local digests (`docs/digests/*`), upstream fetches (repos/paths/lines named inline, fetched 2026-10-22), sibling issues 11/16/23/26/33 (as pins), `a1-codebase.md`/`a2-crates.md`.
**Spec caveat:** `docs/refs/lsp_spec.md` is a stub (899 lines of `{% include_relative %}` directives) → spec claims cite the upstream LSP 3.17 specification pages instead.

---

## 0. Recommendation summary

| # | Decision point | Recommendation | Owner(s) |
|---|---|---|---|
| D1 | Doc response shape | Hierarchical `DocumentSymbol[]` gated on `hierarchicalDocumentSymbolSupport` (absent ⇒ flat `SymbolInformation[]`), exactly rumdl's branch | 27 |
| D2 | Outline content (doc) | Headings (floor) + tasks (23 S3) attached to their heading section by line, nested by list ancestry; tags stay out (16 pin), but Marksman precedent for inclusion must be recorded to 16 | 16/23/27 |
| D3 | Range convention | `range` = whole section (heading line 0-col → EOL of last line), `selectionRange` = heading name text (rumdl pattern); aligns with F2 folding + 23 task containment | 27 (align w/ rumdl) |
| D4 | Kinds | headings String(5), tasks Array(18) (23), notes File(17), tags **PROPERTY(10)** → feeds 16's parked choice | 16 decides tags |
| D5 | Caps | doc 1k soft/5k hard, silent truncate + `tracing::debug`; workspace **128** pushed into collection (band 100-128 = clangd/gopls/iwe/r-a); `SYMBOL_CEILING{1k,5k,128}` vs `REFERENCE_CEILING{5_000}` never merged | 23/16 confirmed |
| D6 | Workspace shape | **Eager full `Location` in 3.17 `WorkspaceSymbol`; do NOT advertise `workspaceSymbol/resolve`**, evidence contradicts 23's parked uri-only+resolve design (resolveSupport = VS Code only) | amend 23 → 29 |
| D7 | Query | fuzzy-subsequence over name+path; array order = rank order (no sortText exists on either shape); empty query → first 128 in (path,line) order | 27 |
| D8 | Entity mix | one merged list (notes+headings+tags+tasks), single cap over merged set; `containerName` = parent context + 16's `"{N} notes"` clusters | 16/23 |
| D9 | Coexistence | rumdl `enableSymbols=false` covers rumdl; **VS Code's built-in markdown LSP also registers both symbol providers and has no per-feature disable setting** → real fork, grill queue | 32/15/23 |
| D10 | Perf rows | doc symbols p95 **<5 ms** headings-only / **<10 ms** with tasks (33's row fits); workspace/symbol p95 **<50 ms @ 20K notes** (fills 23-Amends-33's unfilled row) | 33 |

---

## F1.1 Response-type negotiation

### What the spec/API demands
- `textDocument/documentSymbol` returns `DocumentSymbol[]` (hierarchical) or, **deprecated**, `SymbolInformation[]` flat; the client capability `documentSymbol.hierarchicalDocumentSymbolSupport` (3.10) gates which a server may send. Spec: <https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_documentSymbol> and `#documentSymbolClientCapabilities`.
- `workspace/symbol` returns `WorkspaceSymbol[]` (3.17, `location` = `{uri}` **or** `{location: Location}` union, structurally proven usable by MS, see F1.7) or deprecated `SymbolInformation[]`. Spec `#workspace_symbol`.
- Flat document symbols lose `detail` and arbitrary depth: `SymbolInformation` carries only one `containerName` string (23:45's degradation note; spec `#symbolInformation`).

### What peers emit (source-verified)
| Peer | documentSymbol | workspace/symbol | Cite |
|---|---|---|---|
| MS markdown-languageservice | **hierarchical only** (never flat; its flat conversion exists only internally for workspace) | `WorkspaceSymbol[]` w/ embedded full Location | ms-digest `:4712-4719` (`#tocToDocumentSymbol`), `:7514-7564` (`MdWorkspaceSymbolProvider`) |
| rumdl | hierarchical **or** flat, branch on capability read at initialize, **default false** (absent ⇒ flat; conservative spec conformance) | flat `SymbolInformation` | upstream `src/lsp/server.rs:439-445` (read+store), `:1506-1512` (branch), rumdl-digest `:40476` / `:40483` |
| Marksman | hierarchical gated on detected client support; flat `SymbolInformation` otherwise | flat `SymbolInformation` | marksman-digest `:11147-11149` (capability block), `:12519-12545` (`headingToDocumentSymbol`) |
| Markdown Oxide | `DocumentSymbolResponse::Nested` | flat `SymbolInformation` | moxide-digest (src/symbol.rs `:2951-3100`), upstream `src/vault/mod.rs:443` |
| iwe | **flat for both** (deprecated `SymbolInformation`, depth encoded as em-space indent in name, depth < 4) | flat, `.take(100)` | iwe-digest `:39582-39740`, `:41390-41440` |
| zk | **none**, capability block has no documentSymbol/workspace symbol at all | none | zk-digest `:4245-4290` |

### Which clients actually send hierarchical (verified at source)
| Client | `hierarchicalDocumentSymbolSupport` | workspace resolveSupport | Cite |
|---|---|---|---|
| VS Code (languageclient) | **true** | **`{properties:["location.range"]}`: only one of the four** | vscode-languageserver-node `client/src/common/documentSymbol.ts:75`, `workspaceSymbol.ts:48` |
| Neovim ≥0.10 | **true** | none | `runtime/lua/vim/lsp/protocol.lua` master `:570` (v0.10.0 tag `:803`); workspace symbol caps `:606-611` |
| Zed | **true** | **None** | `crates/lsp/src/lsp.rs:1105-1106`, `:906-909` |
| Helix | **false** → flat required | none | `helix-lsp/src/client.rs:742` |

Helix's `document_symbols` (`client.rs:1601-1619`) returns `lsp::DocumentSymbolResponse` (lsp-types enum parses both shapes, but a compliant server must send Flat given `:742`).

### Options & recommendation
- **Options:** (a) hierarchical-only (breaks Helix, silent empty outline); (b) flat-only (works everywhere, loses tree+detail for the 3 hierarchical clients, iwe-style); (c) capability-gated dual (rumdl pattern).
- **Source/API demands:** the capability exists precisely to be branched on; flat is the mandatory floor.
- **Ecosystem does:** every hierarchical-capable Markdown peer ships (c); only iwe ships flat-only.
- **→ Recommend (c):** one builder producing the tree, a mechanical flatten (`containerName` = nearest parent heading, rumdl-digest `:40491`) for the fallback. Registration static (`DocumentSymbolOptions` only carries workDoneProgress; see a2).

---

## F1.2 Outline content scope

### Findings
- **Heading tree is the floor everywhere.** No Markdown peer puts frontmatter fields or File-Class attributes in either symbol response (MS walks ToC only + optional link definitions; rumdl walks headings only, rumdl-digest `:40476-40497`; moxide headings; Marksman headings; zk nothing; iwe notes+sections). **Premise confirmed: zero precedent for frontmatter/File-Class in outline or picker.**
- **The one multi-entity doc-outline precedent, and it contradicts 16's pin:** Marksman nests **tags as children of their containing heading** in the document outline (`tagToDocumentSymbol` marksman-digest `:12508-12518`, wired through `headingToDocumentSymbol`'s `toDocumentSymbol` mapping `H→heading, T→tag` at `:12526-12535`), kind String, `range = selectionRange = tag.range`. Its workspace symbols append heading + tag `SymbolInformation` (`:12594-12615`). → 16's "document symbols omit tags" is a *choice against* the only PKM peer precedent, not the default; record this to 16 (option space: (i) omit (outline = document structure; tags reachable via the workspace symbol picker); (ii) Marksman-style positional children (tags navigable in outline; cost: outline density + String-kind collision with headings)).
- **MS interleaves link definitions into the heading tree**, `parent.children.push(additionalSymbols.shift()!)` at the position they occur (ms-digest `:4690`), VS Code's server passes `includeLinkDefinitions: true` unconditionally (vscode-markdown-languageserver `src/server.ts:151`). Precedent: non-heading entities *can* appear in a Markdown outline, positioned by source order, kind Constant. Also a coexistence-noise source (F1.8).
- **Tasks (23 S3):** no Markdown peer emits task/checkbox symbols at all: **Traces would be first**. Closest structural precedents: iwe's mixed notes+sections list (multiple entity classes in one response) and MS's positional interleaving (two entity classes merged in one tree).
- **"Hideable classes" mechanism (23's Array-vs-String rationale):** VS Code's built-in `outline.show*` per-kind settings (`outline.showStrings`, `outline.showArrays`, …) are the **only** hide axis, names and prefixes are not filterable. Kind choice therefore *is* the UX contract: tasks=Array ⇒ users can hide tasks while keeping headings (and vice-versa) **iff** the classes use different kinds. Ecosystem support for the argument: none of the peers need it (single class), so this is API-grounded (the settings exist) rather than imitation-grounded.

### Attachment options for tasks (F1.3 co-owned, mechanics here)
| Option | Precedent | Tradeoff |
|---|---|---|
| (a) children of heading section (by line containment) + list-ancestry nesting within | MS link-def position (`:4690`); matches 23's `note.lists()` ancestor stack | tree depth = heading level + list depth; flat fallback keeps only one containerName level (heading) |
| (b) heading-less tasks at root, tasks under nearest *task* ancestor only (literal 23 wording) | none | top-level tasks orphaned from their section in the tree; outline loses "where am I" |
| (c) separate root-level task list | iwe flat siblings | two top-level classes side by side; simplest, but degrades the outline to two lists |

- **Source/API demands:** `DocumentSymbol.children` is arbitrary and unbounded, any nesting is legal; only `selectionRange ⊆ range` must hold (F1.4).
- **Ecosystem does:** tree-by-source-position (MS, Marksman) is the idiom; nobody has done two interleaved *hierarchies*.
- **→ Recommend (a):** heading section contains tasks on its lines; within a section, tasks nest by list ancestry per 23's rule. Novel but protocol-trivial; feeds F2's shared section walk.

---

## F1.3 Heading↔task interleaving mechanics

### Findings
- **Closest precedent is positional merge, not dual hierarchy:** MS interleaves link definitions by source position into the level-stack tree (ms-digest `:4684-4704`: the `while (parent && entry.level <= parent.level)` stack is the same algorithm 11's stack derivation uses); Marksman merges tag nodes into heading children by CST position (marksman-digest `:12526-12535`). **No peer nests one hierarchy (tasks) inside another (headings), this part is novel**, but it is plain `children` composition, nothing protocol-level.
- **The range contract that makes both nest:**
  - Section range (D3): `(heading.line, 0)` → `(last_line, EOL_utf16)` where `last_line` = line before the next same-or-higher heading (rumdl `HeadingSymbol` convention, rumdl-digest `:40476+`; MS `sectionLocation` = next same-or-higher heading line - 1, ms-digest `:1983-2260` ToC, used at `:4716-4717`).
  - Task leaf (23): `line..line+1`, i.e. `(line,0)-(line+1,0)`: line-granular, no char offsets needed.
  - Intra-symbol invariant `selectionRange ⊆ range` holds trivially for both.
  - **Corner:** a task on the section's *last* line ends at `(last+1, 0)` which exceeds the parent's `(last, EOL)` end. Parent↔child range containment is *not* a spec requirement (spec requires only intra-symbol `selectionRange ⊆ range`; trees come from `children` arrays, not ranges) and no client validates it; cosmetic at worst (outline row highlight extends a few columns). Options: accept; or end sections at `(next_heading_line, 0)` (exclusive-end style, differs from rumdl/MS by one line at fold time, but folding is F2's own computation anyway). **→ accept rumdl/MS convention verbatim (migration coexistence beats a non-issue corner); note the corner for the grilling doc.**
- EOL char offset needs line length → one `LineIndex` lookup via 26's `convert` module (cheap, per open document).
- **Shared-walk factoring (cross-cutting §4 input):** MS's `MdTableOfContentsProvider` is *one* computation feeding symbols (`:4712-4719`), folding, anchors, and hover (`:5804` reuses the same name builder). rumdl has no folding so its section fn feeds symbols only. Traces: one section-end walk serves F1.3 ranges, F2 heading folds, F3 header tier.

---

## F1.4 range / selectionRange conventions

### Peer matrix (source-verified)
| Peer | range | selectionRange | Cite |
|---|---|---|---|
| rumdl | whole section `(L,0)→(next_same_or_higher-1, EOL)` | **heading name text**, excludes `#`s, closing `#`s, trailing `{#custom-id}` | rumdl-digest `:40476+` (tests `:40666-40954`, incl. `test_document_symbols_tree_and_section_ranges`) |
| MS | whole `sectionLocation` | **equal to range** (section!) | ms-digest `:4712-4719` |
| Marksman | `h.data.scope` (section) | `h.range` (declaration range) | marksman-digest `:12519-12524` |
| moxide | heading line | equal (heading line) | upstream `src/vault/mod.rs:1294-1326` (`MDHeading::new`: regex match over raw text, skips frontmatter via `FRONTMENTER_RE`; **line-granular, no section computation, outlier**), moxide-digest `src/symbol.rs:2951-3100` |
| iwe | line-level (flat; depth in name indent, no nesting) | n/a (flat, no nesting semantics) | iwe-digest `:39582-39740` |
| Marksman tags (doc) | tag token range | equal | marksman-digest `:12508-12518` |

- **Spec demands:** only `selectionRange ⊆ range` (3.17 `#documentSymbol`).
- **Ecosystem does:** section-range majority (rumdl, MS, Marksman); selectionRange = *name text* only rumdl; MS trivially sets both equal (coarsest).
- **→ Recommend rumdl's pair:** `range` = section (needed for outline row highlight-to-body, task containment D2, and F2 reuse); `selectionRange` = name text (goto-symbol lands the cursor on the title, not col 0; exclude trailing hashes exactly like rumdl). Custom-ID exclusion is a rumdl-specific flavor detail, Traces only needs it if `{#id}` extensions parse (a1: not in scope today; leave a named constant seam).
- Workspace-symbol location range: MS uses **`selectionRange` (heading line)** per entry (ms-digest `:7543-7556` `#toSymbolInformation` → `location.range = symbol.selectionRange`); Marksman uses `h.range` (`:12590`). → workspace entries point at the heading line, not the section, matches Traces' line-granular pins.

---

## F1.5 SymbolKind choices

| Entity | Peers | Cite | Traces |
|---|---|---|---|
| Heading | **String(5)**, rumdl, MS, Marksman; outliers: moxide STRUCT(23) in doc / KEY(22) in workspace (upstream `vault/mod.rs:443`: *inconsistent with itself*), iwe OBJECT | rumdl `:40510-40528`; ms `:4714`; marksman `:12519` | **String** (15/23 pin; ecosystem majority) |
| Note/file | moxide File(17); iwe NAMESPACE | `vault/mod.rs:443` | **File(17)** |
| Tag (workspace) | Marksman String(5) + `"Tag: "` name prefix; moxide CONSTANT(21) | marksman `:12474-12484`, `:12594-12615`; `vault/mod.rs:443` | **PROPERTY(10)** → feeds 16's parked choice (see below) |
| Task | **no peer, none exist** | zk/MS/rumdl/Marksman/moxide/iwe all lack tasks | **Array(18)** (23 pin; uncontradicted) |
| Link def (MS only) | Constant(14) | ms `:4690` region | n/a (not in Traces v1 outline) |

- **Source/API demands:** all of {5,10,17,18,21} legal everywhere; every surveyed client sends `symbolKind.value_set = SymbolKind::all()` (Helix `client.rs:596-597`, Neovim, Zed), so no client filters kinds at capability level. Spec offers no Markdown guidance (kinds are file-structure iconography).
- **Ecosystem does:** headings ⇒ String is 3-of-4 consensus; hideability via VS Code `outline.show<Kind>` is the only class-filter mechanism, and it makes kind a UX contract (F1.2).
- **Tag kind recommendation (16's parked PROPERTY-vs-STRING):** String collides with headings ⇒ `outline.showStrings:false` (needed to suppress VS Code's built-in duplicate, F1.9) would kill tags *and* headings together; moxide avoids collision via CONSTANT. **PROPERTY(10)** = distinct icon + distinct hide-toggle + field-ish semantics (frontmatter origin) without Marksman's name-decoration crutch. API: nothing demanded; ecosystem: split (String-vs-CONSTANT). **→ recommend PROPERTY; decision stays 16's.**
- `DocumentSymbol.detail` exists **only** on the hierarchical shape; `SymbolInformation`/`WorkspaceSymbol` have no detail field → 23's ≤64-char task detail survives only in the doc outline (flat Helix fallback and all workspace entries lose it). State this in 23's degradation note.

---

## F1.6 Result capping

### Peer caps (source-verified)
| Scope | MS | rumdl | Marksman | moxide | iwe | r-a | clangd | gopls |
|---|---|---|---|---|---|---|---|---|
| documentSymbol | none | none | none | none | none (depth <4 only) | none | n/a | n/a |
| workspace/symbol | **none** | **none** (`server.rs:1515-1524`, no truncation anywhere) | none | none | **100** (`iwe-digest:41390-41440` `.take(100)`) | **128** (`config.rs:1074` `workspace_symbol_search_limit` default 128; enforced *in* `symbol(search, limit)`: handlers/request.rs:676-800) | **100** (`--limit-results`, clangd docs) | **100** (`maxSymbols = 100`, `gopls/internal/lsp/source/workspace_symbol.go`) |

### Post-cap behavior precedent
- Truncate **silently** (r-a/clangd/gopls/iwe all silently take-N; nobody logMessages per request; no Markdown peer truncates at all so no counter-precedent).
- **Protocol fact:** `textDocument/documentSymbol` has **no partialResult**; partial-result exists only for workspace/symbol, references, diagnostics, etc. So for doc outline, truncation is the *only* mechanism; for workspace/symbol we follow 26's deliberate non-advertisement of partialResultProvider and also truncate (consistent, one mechanism).
- **→ Recommend:** silent truncate + one `tracing::debug!` count (never `logMessage`, client-modal noise per request).

### Ceiling families (confirming 23/16/26's separation)
- Three ceilings, **two families**, one module:
  - `SYMBOL_CEILING`: doc soft 1_000 / doc hard 5_000 (23) + workspace 128 (16).
  - `REFERENCE_CEILING`: 5_000 (26), different failure mode (graph traversal cost vs response size), never unified (26 pin: "deliberately distinct, never unify", a3 §0).
- **Shared workspace cap reconciliation:** one merged list (notes+headings+tags+tasks, F1.7) → the 128 cap applies **once to the merged response**, ranking first then cutting (r-a model), not per-class quotas (quotas = over-engineering; empty-query fairness covered in F1.7).
- **Doc caps have no peer precedent**, they are Traces-original, justified by the PKM worst case (a single journal note with thousands of tasks), not imitation. State that honestly in the ticket rather than faking precedent.

---

## F1.7 workspace/symbol specifics

### Shape & resolve
- **Who emits what:** rumdl/moxide/Marksman/iwe → legacy flat `SymbolInformation` with full `Location` (rumdl `:40583-40604` `container_name: None`; Marksman `:12486-12499`); MS → 3.17 `WorkspaceSymbol[]` **with the full-Location union branch** `{location:{uri,range}}` + `containerName: None` + kind String, location range = heading selectionRange (ms-digest `:7514-7564`). **No Markdown peer uses uri-only+resolve.**
- **Resolve support reality:**
  - VS Code: `workspace.symbol.resolveSupport = {properties:["location.range"]}` (workspaceSymbol.ts:48), **only client of the four**.
  - Neovim/Zed/Helix: absent (protocol.lua `:606-611`; lsp.rs `:906-909`; client.rs `:594-599`).
  - **Premise correction:** Marksman's `workspaceSymbol/resolve` hit (digest `:3220`) is the Ionide framework's *routing table*, not an override, upstream `Marksman/Server.fs` has no resolve implementation.
  - r-a (upstream `handlers/request.rs:676-800`): eager `WorkspaceSymbolLocation::Location`, `data: None`, no symbol-resolve registration observed; the `data`-carry/`symbol_resolve` path was *not* found.
- **23's parked design (uri-only + resolve + full-Location fallback when resolveSupport absent) vs evidence:** the "fallback" is what 3 of 4 target clients *and* both MS and r-a do as the primary path; resolveSupport is VS Code-only; per-symbol eager cost at cap 128 is a URL build + already-computed line range (µs each, the ranges are produced by the ranking pass anyway). Resolve would save ~8 KB and buy one client a round-trip.
  - **Source/API demands:** either shape is legal; `WorkspaceSymbolOptions.resolveProvider` only if a resolve handler exists (29: never advertise first, iwe stub = anti-pattern).
  - **Ecosystem does:** eager full Location everywhere (MS, r-a); nobody in Markdown-land resolves.
  - **→ Recommend: 3.17 `WorkspaceSymbol`, `location = {location: Location}` eager, `resolveProvider` not advertised.** Single path, current spec type, MS/r-a precedent. **Flag as proposed amendment to 23 §6 (input to 29).** Alternative priced: uri-only+resolve branching per capability = rumdl-style two-shape response + stale-data guard (24) for one client's benefit, leanest fails.

### Query matching, empty query, ranking
| Peer | Matching | Empty query | Cite |
|---|---|---|---|
| rumdl | lowercase **substring** on name | returns **all** (uncapped → the hazard 16's cap exists to prevent) | upstream `server.rs:1520-1523`; test "empty query returns all" rumdl-digest `:40666+` |
| Marksman | **subsequence** on name (incl. `"H1: "`/`"Tag: "` prefixes → class queryable) | all (uncapped) | marksman-digest `:12609-12615` |
| moxide | **nucleo fuzzy** | all (uncapped) | moxide-digest `src/symbol.rs` |
| iwe | skim fuzzy + BM25 | top-100 by rank | iwe-digest `:41390-41440` |
| MS | `fuzzyContains(lower(name))` | all (uncapped) | ms-digest `:7533-7537` |
| r-a | fuzzy + `#`/`*` query markers, matches name **and path**, limit 128 | limit-clipped | handlers/request.rs:676-800 |
| gopls | `symbolMatcher` default FastFuzzy (fzf-inspired), `symbolStyle Dynamic` qualifier selection | `maxSymbols=100` | gopls `doc/settings.md`, `internal/lsp/source/workspace_symbol.go` |

- **Clients all send `""` on picker open** (Helix explicit comment `client.rs:1717` "empty string to get all symbols"; Neovim/VS Code quick-picks behave the same) → empty query **must** return a sane list.
- **Source/API demands:** no `sortText` on `WorkspaceSymbol`/`SymbolInformation` (spec `#workspaceSymbol`/`#symbolInformation`: neither has sortText) ⇒ **array order is the ranking contract** for Neovim/Helix/Zed; VS Code re-scores client-side regardless. No field carries rank either.
- **Ecosystem does:** fuzzy/subsequence has fully displaced substring (substring = rumdl alone, weakest PKM ergonomics, "tmr mtg" style abbreviation queries are the PKM norm per how gopls/r-a/iwe converged).
- **→ Recommend:** subsequence/fuzzy over `name` **and vault-relative path** (r-a precedent; PKM queries often target paths, `daily/2026-10`), score-descending array order; **cap enforced during collection** (r-a `symbol(query, limit)` model, *not* materialize-then-truncate, which is moxide's anti-pattern at scale).
- **Empty query:** deterministic first-128 by `(path, line)`: O(scan) early-exit, stable, zero ranking cost. **Option (grill):** rank by inlink count (26's nearest-picked graph, O(1) lookup) = "most important 128 notes first" (iwe's BM25-for-empty spirit, but PKM-native). v1 = (path,line); inlink-rank is a cheap v2, open question below.

### Entity composition & containerName
- **Mixing precedent:** moxide mixes files+headings+tags+aliases in one workspace response (`vault/mod.rs:443` File/CONSTANT/KEY mapping); iwe mixes notes+sections; Marksman mixes headings+tags. **Traces' four-class mix (notes+headings+tags+tasks) = moxide-shaped, no tasks anywhere.**
- **containerName:** rumdl `None` (`:40604`), MS `None`, Marksman `None`: Markdown peers leave it empty; code-LSPs (gopls `symbolStyle`, r-a module paths) fill it with parent context. iwe embeds parent in the *name* (`Title↖Parent`, iwe-digest), Marksman embeds class in the *name* (`"H1: …"`, `"Tag: …"`).
- 16 pins dedup clusters: one symbol per same-name note group with `containerName = "{N} notes"` and location = first note lexically; **no peer precedent for count-in-containerName** (it's a Traces original; containerName is a free string, API demands nothing).
- **→ Recommend:** `containerName` = vault-relative parent context (path for notes; parent heading for headings/tasks; tags: none (leave container empty or vault path)) + 16's `"{N} notes"` for clusters. Name construction stays **clean text** (rumdl/moxide style): workspace entries carry kind icons that distinguish classes; Marksman's prefixes are only needed when kinds collide, ours don't (D4). Level not needed in flat lists (goto lands on the line regardless); MS's `"# Title"` (ms-digest `:4720`) is the alternative if level-visibility is wanted; it's what stock VS Code users see today (migration familiarity argument).

---

## F1.8 Coexistence with rumdl's `enableSymbols`

### What rumdl actually ships (so Traces can be described as strict upgrade)
- **Document outline: headings only**, hierarchical with children, section ranges, flavor-aware (setext/ATX), custom-ID exclusion from name range (rumdl-digest `:40476-40497`; tests `:40666-40954`).
- **Workspace: headings only**, flat `SymbolInformation`, lowercase substring, `containerName: None`, **uncapped** (upstream `server.rs:1515-1524`).
- **Gate:** `enable_symbols` config (default true), returns `Ok(None)` when off (same `server.rs:1515-1518`); documented in rumdl docs digest `:2580-2680`.
- **Strict-upgrade claim:** Traces doc = headings ∪ tasks ⊋ rumdl headings (strict); Traces workspace = notes ∪ headings ∪ tags ∪ tasks ⊋ rumdl headings, *provided* section ranges and name conventions align (D3 = rumdl verbatim) so the migration period where both servers are enabled shows identical trees rather than divergent ones.

### Two-outline reality (verified)
- **VS Code built-in markdown is itself a client/server pair**: vscode-markdown-languageserver registers **`documentSymbolProvider: true` (server.ts:117)** and **`workspaceSymbolProvider: true` (:123)**; VS Code merges all providers of a request type → duplicate outline rows + duplicate Ctrl+T entries whenever Traces is added.
- **No per-feature disable exists:** `markdown.server.enabled` is **not a real setting** (verified absent from `markdown-language-features/package.json` on main/1.94/1.85). Suppressions available: (1) disable the whole "Markdown Language Features" extension (heavyweight: kills preview too), (2) per-kind outline filter `[markdown]: {"outline.showStrings": false}`, which also kills Traces' String headings (kind collision), keeping only Traces' Array tasks, (3) accept duplicates, (4) Traces headings switch to a non-String kind (contradicts 15/23 pins). **This is grill-queue item #1 (feeds 15/23/32).**
- **Zed:** no built-in markdown LSP (grammar-only) but **merges symbol responses from every server that declares the provider** (`crates/project/src/lsp_store/document_symbols.rs`: `missing_servers_to_query`, per-server `DocumentSymbolsData`, refresh/evict; workspace fan-out at `lsp_store.rs:9399` accepts Flat + WorkspaceSymbolList) → Traces + rumdl-with-symbols = duplicates in Zed too. Joint setup: `enableSymbols=false` suffices for Zed.
- **Neovim/Helix:** no built-in markdown LSP; multi-client merging varies (nvim `buf_request` collects from all clients; Helix picker, not verified at source, low risk once `enableSymbols=false`).
- **Joint-setup text (feeds 32):**
  1. rumdl: `enableSymbols = false` (docs digest `:2580-2680`), now the *documented, intended* cession (the config exists precisely for a second LSP).
  2. VS Code: state the built-in duplication honestly + the four mitigations above (recommend (1) for outline-centric users, (2) as the tasks-only fallback; (4) is a Traces config decision pending grill).
  3. Zed/Neovim/Helix: step 1 alone.

---

## F1.9 Degradation & serving

- **Flat fallback (Helix only):** tree → flat list; `containerName` = nearest parent heading (rumdl `:40491` pattern); `detail` lost (SymbolInformation has none, 23:45 confirmed as API fact, not just a note); task→heading association survives only as one containerName level (grandparent lost, acceptable; Marksman's doc/workspace name divergence (`"Tag: "` prefix only in workspace, marksman `:12474` vs `:12508`) is precedent for shape-dependent naming if we ever want it).
- **Serving:** open docs → per-`(uri,version)` snapshot (24): heading list is always-resident metadata (a1: flat `Heading { level, line, text }`), tasks from the cached parse tree (`note.lists()` ancestor stack, 23). Cold docs → `ContentResolver::text_of` re-parse (11-Amended-by-26), bounded by T1's 100 ms coalesced refresh; staleness of workspace/symbol ≤ last index build (no push exists, see below).
- **No refresh notifications exist for symbols:** `workspace/symbol/refresh` is absent from the spec and the reference implementation (vscode-languageserver-protocol `protocol.ts` enumerates FoldingRange/SemanticTokens/InlineValue/InlayHint/Diagnostic/CodeLens/TextDocumentContent refreshes only, grep verified). Clients re-request on their own triggers (outline re-request on edit; picker re-request per open/keystroke). → pull-only, nothing to gate under T1.
- **Shape of computation for the <10 ms row:** heading path = metadata walk (O(H), zero parse) → **<5 ms sub-row**; task path = cached-AST walk → the existing **<10 ms** row. Cold-doc re-parse measured separately (no row yet, feeds ContentResolver bench, open question for 33).

### Proposed 33 rows (cluster's contribution)
| Row | Target (p95) | Basis |
|---|---|---|
| `documentSymbol`: headings only, warm | **< 5 ms** | always-resident metadata walk; sub-row of 33's <10 ms |
| `documentSymbol`: with tasks (23 S3), warm | **< 10 ms** | 33's existing row; heading walk + cached-parse task walk |
| `documentSymbol`: cold (not-yet-open doc) | *measure, no row yet* | `ContentResolver::text_of` re-parse (11-Amended-by-26) |
| `workspace/symbol`: cap 128, 20K notes | **< 50 ms** | **fills 23-Amends-33's unfilled row**; collect-with-limit over resident name index; validate via a bench sibling to `bench_query_tasks_density` |
| `workspace/symbol`: empty query | same row | deterministic early-exit at 128 |

---

## Matrix rows for C7 merge

**Client support (F1 features):**

| Feature | VS Code | Neovim ≥0.10 | Zed | Helix | Note |
|---|---|---|---|---|---|
| documentSymbol hierarchical | yes | yes | yes | **no (flat only)** | both shapes mandatory |
| documentSymbol flat | n/a (native hierarchical) | n/a | n/a | **yes** | |
| workspace/symbol (any shape) | yes | yes | yes | yes (sends `""` on open) | empty-query path is load-bearing |
| workspace/symbol resolve | **yes** (`location.range`) | no | no | no | → don't advertise |
| containerName rendered | quick-pick secondary text | picker-dependent | yes (list detail) | kind+name list | soft claim, clients vary |

**Registration (proposal to 29):** static only (`DocumentSymbolOptions`, `WorkspaceSymbolOptions` without `resolveProvider`); `dynamicRegistration` exists per-feature in the spec but **no surveyed peer dynamically registers symbols** (MS/rumdl/Marksman/moxide all static) → static, consistent with 26's partialResult non-advertisement; deliberate non-advertisement list: `workspaceSymbol/resolve`, `partialResultProvider`.

**Performance:** the five rows in F1.9. All under `concurrency_level(1)` (26): workspace/symbol at 50 ms must therefore **not** run on the hover path's queue mid-typing; it runs on picker-open only (client-driven), which is already off-keystroke.

---

## Premise-check results (assigned to Cluster 1)

| Premise | Result |
|---|---|
| P1 rumdl has documentSymbol **and** workspace/symbol providers (non-zero) | **TRUE**: `server.rs:1506-1524`; gated by `enable_symbols` |
| rumdl "empty query returns all" test | **TRUE** (uncapped, the 16-cap hazard) |
| MS/other Markdown peers put frontmatter/File-Class fields in outline | **FALSE: none do** |
| Marksman implements `workspaceSymbol/resolve` (digest :3220 hit) | **FALSE: Ionide framework routing table, not an implementation** |
| VS Code built-in markdown is a real LSP server registering symbols | **TRUE**: `server.ts:117`/`:123` |
| `markdown.server.enabled` setting exists (per-feature disable) | **FALSE: no such setting; only extension-level disable or per-kind `outline.show*` filters** |
| `docs/refs/lsp_spec.md` usable as spec source | **FALSE: stub; upstream 3.17 spec cited instead** |
| zk / moxide symbol providers (zero/non-zero) | zk **zero** (`:4245-4290`); moxide **non-zero** (Nested doc + flat workspace) |
| rust-analyzer eager-Location + no symbol-resolve (per 23's input) | **CONFIRMED at handlers/request.rs:676-800** (data:None, `WorkspaceSymbolLocation::Location`); resolve registration not found |
| `workspace/symbol/refresh` exists in spec | **FALSE**: no symbol refresh request anywhere in reference protocol impl |

---

## Open questions (grill queue, none resolvable from source alone)

1. **VS Code built-in duplication (D9):** disable built-in extension vs `outline.showStrings:false` (kills Traces headings, keeps Array tasks) vs kind-switch Traces headings off String (breaks 15/23 pins) vs accept duplicates. Feeds 15/23/32. **Biggest real-user-facing fork in this cluster.**
2. **23 §6 amendment:** uri-only+resolve+fallback → eager full Location + no resolve (evidence: resolveSupport VS Code-only; no Markdown peer resolves; µs cost at cap 128). Needs 23 accept + 29 ratify.
3. **16's tag decisions:** PROPERTY vs String (recommendation above + Marksman-String + moxide-CONSTANT option space) *and* tags-in-doc-outline (Marksman precedent `:12508` contradicts 16's pin, record, don't decide here).
4. **Empty-query ranking:** deterministic (path,line) [v1 rec] vs inlink-count popularity (26 graph, O(1)), PKM-native but unvalidated.
5. **Uniform 128 cap fairness:** empty query returns first-128 in path order ⇒ files crowd out headings/tags of later paths. Options: accept (picker is a page, not an inventory) vs score-boost notes vs per-class minimums (over-engineering). Recommend accept + note.
6. **Section-end corner (F1.3):** last-line task leaf exceeds parent range by one line, accept (rec; non-normative) vs exclusive-end `(next,0)` convention.
7. **Doc caps 1k/5k have no precedent**, Traces-original; fine, but say so explicitly rather than citing peers.
8. **Cold-doc symbol row** for 33 (re-parse cost), needs a measurement, not research.
