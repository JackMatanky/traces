# 27 pass-2 / B2: Folding ranges + selection ranges

Research only. No code changed. Independence: did not read `research/27-structural-symbols-editor-intelligence.md` or any `research/27*.md` outside `27-pass2/`. Inputs: `27-pass2/{a1,a2,a3}`, tickets 11/17 (5/15 via digests), tool digests, upstream LSP 3.18 spec.

Provenance labels used throughout:

- **demand**: required by the LSP API or by an actual client's behavior.
- **ecosystem**: what peer markdown LSP servers emit (practice, not requirement).
- **domain**: Traces/PMK-specific need decided by a ticket (frontmatter, callouts).

---

## 1. Spec facts (LSP 3.18)

`docs/refs/lsp_spec.md` is an include-only stub (no feature sections), so these come from the upstream 3.18 spec text (captured at `~/.local/share/opencode/tool-output/tool_1609de439001nMCbDd6d5WaZhZ`).

### 1.1 `textDocument/foldingRange`

- Server capability: `foldingRangeProvider: boolean | FoldingRangeOptions | null`.
- Params: `FoldingRangeParams { textDocument, workDoneProgressParams, partialResultParams }` (whole document, no positions).
- Result: `FoldingRange[] | null`; partial result `FoldingRange[]`.
- `FoldingRange` fields: `startLine`, `startCharacter?`, `endLine`, `endCharacter?`, `kind?`, `collapsedText?` (since 3.17). **No `selectionRange` field on `FoldingRange`** (verified against 3.18 interface text; earlier assumption was wrong).
- Semantics: the folded area starts *after* `startLine`'s last character and ends *with* `endLine`'s last character; with `lineFoldingOnly` the start line stays visible as the fold header.
- Spec oddity: validity prose says start/end lines must be "bigger than zero and smaller than the number of lines"; real clients do not enforce this (servers emit `startLine: 0` routinely, e.g. section-at-doc-start). Emit `0` freely; note the quirk.
- `kind` value set: `comment`, `imports`, `region` (extensible string; unknown kinds handled gracefully by clients). `imports` has no markdown meaning.
- `collapsedText`: custom label when collapsed; gated by client cap `textDocument.foldingRange.foldingRange.collapsedText` (3.17).
- Client caps: `rangeLimit?` (spec explicitly calls it "a hint, servers are free to follow the limit"); `lineFoldingOnly?` (if set, client **ignores** `startCharacter`/`endCharacter`); `foldingRangeKind.valueSet?`; `collapsedText?`.
- `workspace/foldingRange/refresh` (3.18): server→client, for config-change invalidation of *cached* folds. Not needed when folding is computed per request.

### 1.2 `textDocument/selectionRange`

- Server capability: `selectionRangeProvider: boolean | SelectionRangeOptions | RegistrationOptions`.
- Params: `SelectionRangeParams { textDocument, positions: Position[], ... }` (batch of positions).
- Result: `SelectionRange[] | null`; `SelectionRange { range, parent? }` with the normative constraint **`parent.range` must contain `this.range`** (tree rooted at each requested position, innermost first, parents outward).
- Observed convention (VS Code API docs + MS impl): `result[i]` is the chain for `positions[i]`; editors merge/dedupe chains themselves; empty document / no match → `null` or `[]`, both tolerated.
- No folding/selection interaction in spec; independent requests.

---

## 2. Client consumption matrix (who actually uses what)

| Client | foldingRange | collapsedText | selectionRange | Notes / evidence |
|---|---|---|---|---|
| VS Code | yes | client cap **off** (default false) | yes (Expand/Shrink Selection) | `vscode-languageclient` folding defaults: `rangeLimit: 5000`, `lineFoldingOnly: true`, kind valueSet all three. Built-in `markdown-language-features` (MS service) already supplies **both** features for markdown → Traces providers coexist/merge with built-in ones (verify duplicate-range behavior empirically in Phase B). |
| Neovim ≥ 0.11 | yes | yes (shown by `foldtext()`) | ≥ 0.12 (`an`/`in` visual textobjects, PR #34011) | `vim.lsp.foldexpr/foldtext/foldclose` (PR #31311); `foldclose` special-cases `imports` kind; nvim drops single-line folds client-side; selectionRange is fallback when treesitter is inactive. |
| Zed | yes, **opt-in** since Feb 2026 (PR #48611) | yes (PR #48624) | **not found** (two searches) | Per-language setting `document_folding_ranges: "on"`, default off; prefers LSP over tree-sitter, falls back to tree-sitter on empty result; PR comment: few servers send `collapsedText`, VS Code ignores it. Expand-selection is tree-sitter (`select_larger_syntax_node`, word-first fix for markdown in PR #29184); no `SelectionRangeRequest` in `crates/project/src/lsp_command.rs`. Medium confidence; report as "no evidence", not "absent". |
| Helix | no (issue #1840 open since 2022; draft PR #14593 is user-level) | no | no | Treesitter textobjects only. Floor: features that only exist as LSP calls are dead there. |
| coc.nvim | mirrors VS Code (configs seen: `foldingRange: { rangeLimit, lineFoldingOnly }`) | as configured | yes | Same upstream client library. |

Demand summary: **folding is consumed by 3/4 targets, selection by 2/4** (VS Code + nvim; Zed/Helix currently neither). Both are cheap relative to their value; folding is the clearly higher-demand feature.

---

## 3. Server ecosystem survey

| Server | folding | selection | Shape of what it emits |
|---|---|---|---|
| **MS markdown-language-services** | yes | yes | Folding: header sections (ToC-derived), `<!-- #region -->` blocks (`kind: Region`, nested, labeled), fenced code, lists (`list_item_open`, incl. multi-line items), tables, blockquotes, html blocks (kind `Comment` when starting `<!--`). Line-only (no character offsets). `rangeLimit` 5000, silent truncation. **No frontmatter fold**: test `Should fold fenced code blocks with yaml front matter` is `test.skip`. Blank-line convention: trailing blank lines trimmed so exactly one newline remains before next header (3 dedicated tests). Gate: `isFoldableToken` requires `end > start` line (`html_block`: `> start+1`). Smart select: three tiers: **header** (nested enclosing headers via ToC), **block** (tokens sorted largest→smallest, chained as parents), **inline** (bold/italic/link/inline-code/table cells); returns innermost range with parent chain outward; blockquotes covered at block tier only; no match → `undefined` → coalesced. |
| **iwe** | yes (kind=Region for sections; `collapsedText` used) | no | Sections (end = recursive max end − 1; `collapsedText` = `"#"*level + title`), code/Raw (`collapsedText` = lang), quote, bullet/order lists (`collapsedText` = first item text), table. Line-only (`start_character/end_character = None`). Emits degenerate single-line folds; don't copy that. |
| **Marksman** | **no** | **no** | Capability block (digest :11136-11169) advertises hover/definition/codeAction/doclink/completion/references/rename-ish only; the `:3190` hits are the vendored Ionide library's generic dispatch table, not advertised caps. Maintainer (GitHub Discussion #353, Nov 2024): "not implemented, happy to accept a PR". |
| **zk** | **no** | **no** | Capabilities (:4244-4284): hover, definition, codeAction, documentLink, completion, references, executeCommand. (`OriginSelectionRange` elsewhere is go-to-def WorkspaceEdit, unrelated.) |
| **rumdl** | **no** | **no** | Only "case folding" test hits. |
| **markdown-oxide** | **no** | **no** | `selection_range` hits are `DocumentSymbol` fields. |

Takeaway: the feature set is a **MS-shaped market with almost no adoption**: 1 of 6 servers does folding, 1 of 6 does selection, and both are the same MS service that ships inside VS Code. Marksman/zk/rumdl/markdown-oxide users get these features *only* from Traces if Traces provides them. This flips the usual logic: providing them is differentiation, parity with MS is the ceiling, and no peer's conventions bind us.

---

## 4. Traces-specific inputs

- **Ticket 11 (spans)** decided: `Range<BytePos>` on `Heading`/`Link`/`Tag`/inline-field entries, captured from `into_offset_iter()` (currently discarded), **transient (never persisted in redb)**; position-encoding negotiation `utf-8-if-offered-else-utf-16`, single `convert` module at the protocol boundary (clamp, never panic). Folding/selection consume these spans at request time for closed files too (re-parse via `ContentResolver::text_of`, per 26's serving model).
- **Ticket 17 (callouts)** explicitly declares the handoff: scope "in scope, but limited to completion and folding-range data"; "Folding-range data for ticket 27: Medium / Low (detect blockquote boundaries)"; `CalloutBlock { header_line, body_start, body_end, nesting_depth, type_identifier, canonical_type, is_foldable }` (+`-` modifier); "ticket 27 consumes it for `FoldingRange` generation". GFM enabled via `Options::ENABLE_GFM`. Note: Obsidian's real semantics are three-state (no modifier = collapsible/expanded, `-` = starts collapsed, `+` = not collapsible) vs ticket's `is_foldable: bool` (mapping question for 27, see §9).
- **Ticket 19 (frontmatter spans)** lands *with 19*, not earlier; but a frontmatter fold needs no spans: a line-0 raw scan (`---` … `---`/`...`) suffices, so no cross-ticket dependency.
- **a1**: headings are line-only in the Note AST today; two-pass heading parse; `pulldown-cmark` 0.13.4 `into_offset_iter()` spans available; a1 already recommends "a shared structural walk for folding + outline consumers" (§7 makes that concrete).

---

## 5. Cross-cutting design constraints

1. **Line-only ranges.** `lineFoldingOnly: true` is the default in VS Code's client and the practical norm (MS and iwe both emit line-only). Emit `startCharacter`/`endCharacter: None`. Saves position conversion entirely and matches every consumer tested.
2. **`rangeLimit`.** Spec says hint; MS honors 5000 silently. Honor the client value when present (default 5000) with **priority truncation**: headers → fences/frontmatter → callouts/blockquotes → lists/tables → comments/regions, so the folds users click survive. Deterministic and documented (unlike MS's silent cut).
3. **No degenerate folds.** Skip `start == end` and `end < start` server-side (iwe's single-line folds are a known wart; nvim drops them anyway).
4. **Empty result → `null`.** Spec allows; avoids client-side error paths on headerless/one-line files.
5. **`collapsedText`: send unconditionally.** Precedent: iwe and rust-analyzer send without gating; nvim `foldtext()` renders it, Zed reads it (PR #48624), VS Code ignores it (harmless). Cheapest strings we produce: heading = title (or `"#"*level + title`, iwe-style), fence = lang, list = first item text (iwe), callout = `type_identifier`.
6. **Skip in v1**: `workspace/foldingRange/refresh` (nothing cached cross-request), partial-result/progress (payloads bounded by the 10k-line/file cap), dynamic registration (static is fine).
7. **End-line convention is unstandardized**: MS folding trims trailing blanks (leave one newline), MS ToC `sectionLocation` does *not* (ends at line before next same-or-higher heading, full line length), iwe uses `max_end − 1`. Adopt MS folding's convention (gutter UX, VS Code dominant) for **folding**; adopt MS ToC's raw convention for the selection header tier (no blank trimming; selection wants exact section bounds).
8. **Cancellation**: check `$/cancelRequest` between tiers in selection, per-section in folding; bounded by the 10k-line cap, worst case already small.

---

## 6. Option spaces

### 6.1 Folding: minimal cut (M) vs extended cut (E)

**M: heading sections + fenced code + frontmatter.**

| Piece | Provenance | Coverage | Cost |
|---|---|---|---|
| Heading sections | demand + ecosystem (MS, iwe) | the fold every markdown user hits; drives nvim `foldexpr` | low; heading spans (11) or two-pass parse; end = next heading of `>=` level or EOF; trim trailing blanks (§5.7) |
| Fenced code | demand + ecosystem (MS, iwe) | second universal fold | low; fence events already spanned |
| Frontmatter | **domain** (vaults are frontmatter-heavy) | Obsidian parity; MS *lacks* it (skipped test), our differentiator | low; raw line scan, no dependency on 19 |

**E = M + lists + blockquotes/callouts + tables + html comments (`kind: comment`) + `<!-- #region -->` (`kind: region`).**

| Piece | Provenance | Coverage | Cost |
|---|---|---|---|
| Blockquote + callout body | ecosystem (blockquote: MS) + **domain** (callout: ticket 17 handoff) | PKM killer: fold a `> [!NOTE]` to its header | low; same blockquote path, enrich with `CalloutBlock` when 17 lands (`collapsedText` = type; fallback: fold as plain blockquote day one) |
| Lists | ecosystem (MS, iwe) | moderate; PMK notes use lists heavily | low; `list_item_open` spans |
| Tables | ecosystem (MS, iwe) | low-medium (wide tables fold well) | low; `table_open` spans |
| HTML comments | ecosystem (MS) | low | low |
| `<!-- #region -->` | ecosystem (MS only) | low for PMK; useful for hand-authored grouping | medium; nested state machine, MS-parity nicety |

Everything in E is one more match arm in the same event walk (§7); the only genuinely new machinery is the region state machine and the callout mapping.

### 6.2 Selection ranges: v1 (S1) vs defer (S2)

- **S1: MS-shaped three tiers.** Header tier (cursor's heading spine, nested) + block tier (fence/list/blockquote/table/paragraph, largest→smallest chained) + inline tier (emphasis/strong/link/inline-code; table cells optional). Returns innermost-first with `parent` chain; `null` on no match. **demand** (VS Code Expand Selection, nvim 0.12 `an`/`in`); tiers beyond header/block are **ecosystem** (MS shape). Cost: medium, but position-local: header tier is an index lookup, block tier walks blocks overlapping the position, inline tier walks only inline events at the position; reuses the shared walk.
- **S2: defer.** VS Code users keep built-in MS smart select (no visible loss); nvim 0.12 falls back to treesitter when the markdown parser is active; Zed/Helix unaffected either way. Cost of deferring: small and concentrated (nvim without treesitter markdown).

---

## 7. Recommendation

**Folding: E-lite**: heading sections + fences + frontmatter + blockquotes/callouts + lists in v1; tables, html comments, and `<!-- #region -->` as fast-follows behind the same walk (accept the small MS-parity gap rather than the region state machine in the critical path).

Rationale: every v1 piece traces to a hard driver: headings/fences = universal client UX (**demand**), frontmatter = vault shape (**domain**, and an MS gap), callouts = explicit ticket-17 handoff (**domain**), lists = heaviest PMK block type (**ecosystem** + domain). Regions are the one MS-ism with no Traces driver; deferring costs nothing.

**Selection: S1 in v1, trimmed**: header + block tiers mandatory; inline tier minimal (emphasis/strong/link/inline-code), table cells optional. Rationale: the shared walk makes the marginal cost small, and unlike folding this is a feature *no* peer markdown server ships; 2 of 4 target editors consume it and Traces would be their only markdown source.

**Shared structural walk (concrete):** one `into_offset_iter()` pass over the buffer producing (a) a block tree (sections, fences, blockquotes/callouts, lists, tables, paragraphs) with byte spans and (b) inline spans per section. Folding projects block nodes → `FoldingRange[]` (line-only, priority-truncated at `rangeLimit`, degenerate-filtered, `collapsedText` attached). Selection answers per requested position: locate innermost containing node (inline → block → header spine), chain `parent` outward, dedupe equal adjacent ranges. Document-symbol/outline consumers (a1's note) read the same heading nodes. All three features then share one parse-time cost instead of three walks.

**Wire conventions** (from §5): line-only; honor `rangeLimit` (default 5000) by priority; skip degenerate; `null` when empty; `collapsedText` always; no refresh/partial-result/dynamic-registration; capability objects advertise `foldingRangeProvider: true` and `selectionRangeProvider: true` statically.

**Perf rows for 33** (none exist today; proposed, same class as its Document-symbols row `<30ms / <10ms`):

| Operation | p95 target | stretch | Rationale |
|---|---|---|---|
| foldingRange | < 30ms | < 10ms | whole-document walk over one parse; same class as document symbols |
| selectionRange | < 10ms | < 5ms | few positions, position-local walk with early exit |

---

## 8. Provenance roll-up

| Pick | Label |
|---|---|
| Heading sections, fenced code, line-only output, blank-trim convention, `rangeLimit` honoring | demand (client/API behavior) |
| Lists, tables, blockquotes, html comments, region markers, three-tier selection shape, `collapsedText` strings | ecosystem (MS/iwe practice) |
| Frontmatter fold (MS lacks it), callout fold + `collapsedText` = type, is_foldable mapping | domain (tickets 17/19, vault UX) |
| Providing *either feature at all* (peers ship neither) | demand + differentiation |
| Deferring `workspace/foldingRange/refresh`, partial results, dynamic registration | demand-avoidance (nothing requires them) |

---

## 9. Open questions / risks

1. **VS Code provider overlap**: Traces + built-in `markdown-language-features` both serve markdown for these two features; confirm empirically whether VS Code merges, dedupes, or duplicates ranges; decide whether the extension registers for `markdown` unconditionally or config-gates it.
2. **Callout three-state vs `is_foldable: bool`** (17): Obsidian `-` = start collapsed, `+` = not collapsible; LSP `FoldingRange` has no "initially collapsed" concept (client-side setting), so mapping likely collapses to "foldable + `collapsedText`"; confirm with 17 owner.
3. **Zed `selectionRange`**: no evidence found in two searches; report as unverified, re-check before any claim in the final synthesis.
4. **Heading source**: consume ticket 11's `Heading` spans vs the two-pass parse, whichever lands first in 27's build order; folding must not block on 11's full span model (line-only output tolerates line-granular section ends).
5. **`selectionRange` per-position ordering**: innermost-first + parent chain confirmed by spec (`parent.range` contains `range`) and MS/VS Code practice; position containment of `result[i]` is observed practice, not verifiable spec prose in our fetch; implement it anyway (VS Code relies on it).
6. **Spec quirk**: `startLine: 0` vs "bigger than zero" validity prose; harmless in practice, frontmatter-at-line-0 folds must ship regardless; note in code review docs.

---

## Appendix: evidence index

- Upstream LSP 3.18 (folding §: ~6425-6669; selection §: ~6671-6766): `/Users/jack/.local/share/opencode/tool-output/tool_1609de439001nMCbDd6d5WaZhZ`.
- MS service: folding impl/tests `docs/digests/lsp_microsoft-vscode-markdown-languageservice-digest.txt:4984-5146`, tests `:11444+` (frontmatter test skipped, table/blockquote/region/html tests), smart select `:6805-7260`, ToC `sectionLocation` `:1983-2240` (raw section end = line before next same-or-higher heading).
- iwe `handle_folding_range`: `docs/digests/iwe-org-iwe-src-digest.txt:40011-40120`, tests `:47761+`.
- Marksman caps (no folding/selection): `docs/digests/lsp_artempyanykh-marksman-digest.txt:11136-11169`; Discussion #353 (2024-11).
- zk caps: `docs/digests/zk-src-digest.txt:4244-4284`.
- rumdl: `docs/digests/lsp_rvben-rumdl-src-digest.txt` (zero hits); markdown-oxide: `docs/digests/lsp_feel-ix-343-markdown-oxide-src-digest.txt` (zero hits).
- Clients: VS Code folding defaults (`vscode-languageclient` docs), nvim PRs #31311/#34011, Zed PRs #48611/#48624/#29184, Helix issue #1840.
- Tickets: `issues/11-source-span-and-position-model.md` (spans, encoding, transient policy), `issues/17-footnotes-and-callouts.md` (CalloutBlock, folding handoff), `issues/19-...md` (frontmatter spans with 19).
- Perf: `research/33-lsp-performance-targets.md` (no folding/selection rows exist; Document symbols `<30ms/<10ms` used as class anchor).
