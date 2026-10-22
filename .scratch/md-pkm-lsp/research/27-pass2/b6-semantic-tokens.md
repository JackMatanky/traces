# 27 pass-2 / B6: Semantic tokens (Markdown/wikilink/PKM state coloring)

Research only. No code changed. Independence: did not read `research/27-structural-symbols-editor-intelligence.md` or any `research/27*.md` outside `27-pass2/`. Inputs: `27-pass2/{a1,a2,a3}`, tickets 26/29/33, tool digests, upstream LSP 3.18 spec, client/server sources fetched live.

Provenance labels used throughout:

- **demand**: required by the LSP API or by an actual client's behavior.
- **ecosystem**: what peer markdown LSP servers emit (practice, not requirement).
- **domain**: Traces/PMK-specific need decided by a ticket (resolution state, ambiguity).

---

## 1. Premise audit (a3's P8-P15), verified at source

| Premise | Verdict | Evidence |
|---|---|---|
| P8: VS Code's built-in markdown grammar has no wikilink concept | **confirmed** | `microsoft/vscode` `extensions/markdown-basics/syntaxes/markdown.tmLanguage.json` (79,877 B fetched): **0 matches** for `wiki`/`[[`; only `meta.link.*` / `string.other.link.*` scopes. Corroborated by microsoft/vscode#138173 ("Avoid weird highlighting of wikilinks") and kortina/vscode-markdown-notes#121. Plain `[x](y)` links *are* grammar-colored (resolution state is not). |
| P9: tree-sitter markdown doesn't cover wikilinks either | **confirmed with nuance** | nvim-treesitter `queries/markdown/highlights.scm` (2,654 B): no wiki capture (only `link_destination`/`link_label`); neovim core `runtime/queries/markdown/{highlights,injections}.scm`: none; helix `runtime/queries/markdown/highlights.scm`: none; Zed `crates/grammars/src/markdown{,-inline}/highlights.scm`: none. **Nuance**: the upstream inline grammar (MDeiml/tree-sitter-markdown `tree-sitter-markdown-inline/grammar.js`, lines ~100-103/179-196/425-429) *has* a `wiki_link` node gated behind `common.EXTENSION_WIKI_LINK` at build time, so the grammar can parse wikilinks, but no shipped editor query captures it and the extension flag is off in the grammars Zed (`tree_sitter_md`, `crates/grammars/src/grammars.rs:33-34`) and helix (`languages.toml` rev `f969cd3`) use. |
| P10: editors have "partial" semantic-token support | **corrected, spans the full range** | VS Code: full (full+delta+range+cancel+refresh). Neovim: full-ish (full+delta+range, multiline+overlap, **serverCancelSupport=false**, refresh). Zed: full for full/delta (**`range: None`**, so it never sends range requests), cancel+refresh. **Helix: none at all** (see §3). |
| P11: zk (and maybe others) emit semantic tokens | **refuted** | zk digest hits at :2446/:3289 are GPL/install-doc false positives (`zk-src:19202` is a chart color rule). zk has no `SemanticTokensProvider`. Same zero-hit verification for MS markdown-language-service, rumdl, iwe (§4). |
| P12: peer PKM servers color link state | **confirmed, but only two-valued / stateless** | Marksman: kind-only, **no** resolution state (§4). markdown-oxide: two-valued (resolved/unresolved), no ambiguous (§4). |
| P13: rust-analyzer is the custom-type precedent | **confirmed** | `UnresolvedReference` type, `standard_fallback_type`, `nonStandardTokens` toggle; origin PR #4034 (2020, matklad: "quick way to implement unresolved reference diagnostics"; users color via `editor.semanticTokenColorCustomizations`) (§4). |
| P14: nvim/Zed semantic tokens are additive over tree-sitter | **confirmed, with one exception mode** | nvim `augmentsSyntaxTokens = true` always, extmarks at `vim.hl.priorities.semantic_tokens`. Zed: `augments_syntax_tokens = semantic_tokens.use_tree_sitter()` (`crates/project/src/lsp_store.rs:645-647`) → true for `off`/`combined`, **false for `full`** (in `full` mode grammar coloring is replaced entirely). |
| P15: `nav::NavContext::highlight` spine exists as source | taken from a2 | Not re-derived here; token emission must walk the same occurrence/`Resolution` data, not a second parser (§5). |

**Net finding**: no target editor can show wikilink/tag/field resolution state today (grammar-blind + tree-sitter-query-blind), and 4 of 6 peer markdown servers ship no tokens at all. Semantic tokens are the *only* LSP surface whose entire purpose here is **state visibility** (navigation is already ticket 26's `documentLink`).

---

## 2. Spec facts (LSP 3.18)

`docs/refs/lsp_spec.md:655` is a Jekyll include stub (`{% include_relative language/semanticTokens.md %}`), so all cites are upstream 3.18 (`https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/`, stripped at `/tmp/lsp318.txt`):

- **Legend** (lines ~14660-14770): `SemanticTokensLegend { tokenTypes: string[], tokenModifiers: string[] }`; "must be provided up-front on registration and capture **all possible** token types and modifiers"; fixed at `initialize`, not per-request. Types = index lookup; modifiers = **bit flags** (value `3` → `[tokenModifiers[0], tokenModifiers[1]]`). `tokenType < 65536`.
- **Extension is legal** (line 14300): "The protocol defines a set of token types and modifiers but **clients are allowed to extend these and announce the values they support** in the corresponding client capability." → custom types/modifiers are a designed extension point; the client's `tokenTypes` array is an announcement, and (per §3) no fetched client *enforces* it on render.
- **Encoding**: relative format, 5 ints per token (`deltaLine`, `deltaStart`, `length`, `tokenType`, `tokenModifiers`); `deltaStart`/`length` must use the position encoding negotiated at `initialize` (ticket 26's `convert` module applies unchanged).
- **`SemanticTokensClientCapabilities`** (~15344-15480): `requests { range, full({delta}) }`, `tokenTypes[]`, `tokenModifiers[]`, `formats[]`, `overlappingTokenSupport?`, `multilineTokenSupport?`, `serverCancelSupport?` (3.17: server may return `ServerCancelled`, client retriggers), `augmentsSyntaxTokens?` (3.17: true = both syntax+semantic color; false = semantic only; **undefined = behavior unspecified**).
- **Range vs full** (~15886-15888): range exists "to allow for flicker free scrolling and semantic coloring of a minimap" for *range-only* servers; a range-only server "might not render a minimap correctly or might even decide to not show any semantic tokens at all". Full-only is the normal degraded-safe path (markdown-oxide works this way in production). If a client advertises both but the server provides neither-capable-thing, the client may show nothing; advertising must match implementation (ticket 29's discipline).
- **`workspace/semanticTokens/refresh`** (~15946-15995): server→client; client asks the server to recompute tokens for visible editors; "the client still has the freedom to delay the re-calculation if … an editor is currently not visible"; global, "should be used with absolute care"; response `void`, error = code+message.
- Spec says nothing about *which* types a server "should" use for a given language: legend content is convention (ecosystem), not normative.

---

## 3. Client rendering matrix (who consumes, and what happens by default)

| Client | Advertised `tokenTypes` / `modifiers` | Requests | multiline / overlap / cancel / refresh | Rendering default | Custom types by default |
|---|---|---|---|---|---|
| **VS Code** (`vscode-languageserver-node` `client/src/common/semanticTokens.ts:54-101`) | **23 standard only** (incl. `decorator`, `label`) / 10 standard | `range: true`, `full: {delta: true}` | false / false / **true** / true | `editor.semanticHighlighting.enabled` = `configuredByTheme`; **shipped themes (Dark+, Light+) enable it → stock VS Code renders standard-type tokens out of the box** (vscode-docs `semantic-highlight-guide.md`) | No rule → no semantic style → **grammar color preserved** (`augmentsSyntaxTokens=true`); custom type/modifier visible only after `editor.semanticTokenColorCustomizations` rule (RA PR #4034 workflow) |
| **Neovim** (`runtime/lua/vim/lsp/protocol.lua`, capability block) | **23 standard, `label` missing** (ns/type/class/enum/iface/struct/typeParam/param/var/prop/enumMember/event/fn/method/macro/keyword/**modifier**/comment/string/number/regexp/operator/decorator) / 10 standard | `range: true`, `full: {delta: true}` | true / true / **false** / true (`workspace.semanticTokens.refreshSupport=true`) | `vim.lsp.semantic_tokens` **on by default** (`semantic_tokens.lua:1027`), 200ms debounce (`:829`), extmarks at `vim.hl.priorities.semantic_tokens + delta` (`:700`), groups `@lsp.type.X.<ft>` → `@lsp.mod.Y.<ft>` → `@lsp.typemod.X.Y.<ft>` (`:728-731`) | **Applied by name unconditionally, with no filtering against the advertised list**; undefined group → invisible. Default links for standard `@lsp.type.*` documented in `runtime/doc/lsp.txt` ("LSP servers may also use off-spec types") |
| **Zed** (`crates/lsp/src/lsp.rs:467-511, 1048-1061`) | 26 types incl. **custom** `label`, `lifetime`; custom modifier `constant` / 11 | **`range: None`**, `full: {delta: true}` | true / true / true / true (`refreshSupport` :942-944) | **`semantic_tokens` per-language setting default `"off"`** (`docs/src/semantic-tokens.md`; modes `off`/`combined`/`full`) | Mapped **by name from the server's legend** (`lsp_store/semantic_tokens.rs:670-693`); rules from theme + `semantic_token_rules`; `assets/settings/default_semantic_token_rules.json` styles `label` (:189) but has **no `unresolvedReference`** → RA's flagship type is unstyled here too |
| **Helix** |  |  |  | tree-sitter only | **No semantic-token support whatsoever**: full `ClientCapabilities` literal (`helix-lsp/src/client.rs:586+`, both `workspace` and `text_document` blocks) contains **no `semanticTokens` field**; PR #6102 ("Semantic tokens highlighting") closed, never merged; no handling in `helix-lsp/src/{lib,client}.rs` |
| **coc.nvim** | mirrors VS Code in configs observed | mirrors | (unverified) | mirrors upstream vscode-languageserver protocol; **low confidence, not source-verified** | same as VS Code |

Demand summary: state-coloring is *consumable* in **2/4 targets out of the box** (VS Code themes, Neovim defaults), *possible but unconfigured* in Zed (default off; and in `full` mode `augments=false` means our tokens replace grammar, see §10.6), and *impossible* in Helix. All clients render **by name**, ignoring the announced `tokenTypes` list, so emitting an un-announced custom type degrades to "no color", never to an error.

---

## 4. Server ecosystem survey

| Server | Ships tokens? | Legend | State encoding | Transport | Notes |
|---|---|---|---|---|---|
| **Marksman** (`docs/digests/...marksman-digest.txt` Semato.fs :10877-10990) | **yes** | `class` (WikiLink + RefLink), `enumMember` (Tag), modifiers `[]` (:11163) | **none**: purely syntactic occurrence coloring | `full: true` + `range: true` (:11887/:11902) | `tokenModifiers = 0u` always; **multiline tokens filtered out** (`isSingleLine`); manual delta encoding; **no `workspace/semanticTokens/refresh` calls** (digest hits :1497+ are vendored Ionide stubs) |
| **markdown-oxide** (`...markdown-oxide-digest.txt` `src/tokens.rs` :6240-6320) | **yes**, gated by config `semantic_tokens` (default `true`; capability-advertisement *forces* it on via `set_override` :5040-region) | advertises `DECORATOR`+`COMMENT` types and **`DECLARATION`+`DEPRECATED` modifiers (:5054-5061) but `token_modifiers_bitset: 0` on every token**: an advertise/impl gap (exactly 29's iwe-style anti-pattern) | **two-valued**: `token_type: if is_unresolved {1} else {0}` (0=decorator(resolved), 1=comment(unresolved)); computed from a per-request `path_unresolved_references(vault, path)` set; every reference in the file (`vault.select_references`) is tokened | `full: Bool(true)`, **`range: Some(false)`**, no delta | Calls `workspace/semanticTokens/refresh` after vault update and after `reconstruct_vault` (gated on the setting); docs: unresolved links get tokens "even if `unresolved_diagnostics` is turned off": state coloring as an independent feature |
| **MS markdown-language-services** | **no** |  |  |  | zero `SemanticToken` hits in digest |
| **rumdl** | **no** |  |  |  | zero `semantic` hits in digest |
| **iwe** | **no** |  |  |  | zero `semantic_token` hits in digest |
| **zk** | **no** |  |  |  | P11 refuted (§1) |
| **rust-analyzer** (precedent, not markdown) | **yes** | many standard + **custom** incl. `UnresolvedReference` (`crates/rust-analyzer/src/lsp/semantic_tokens.rs`; PR #4034 origin) | state-as-**custom-type** (`unresolvedReference`) | full+delta (diff via `diff_tokens`), range | `config.rs:415-467`: `nonStandardTokens: bool = true` (when false: `standard_fallback_type` maps or drops, `mods.standard_fallback()` drops non-standard modifiers), `comments_enable` doc: *"In some editors (e.g. vscode) semantic tokens override other highlighting grammars"*; `to_proto.rs:749-807`: **augment-aware filtering**: when the client augments, skip modifier-less syntax-class highlights (Comment/Keyword/String/Operator/…) that would only restate grammar color; **multi-line ranges split into per-line single-line tokens** (:793-801) |

Takeaway: **2 of 6** markdown peers ship tokens; neither encodes three-valued resolution; **no peer uses token modifiers at all** (Marksman and markdown-oxide both ship bitset 0); the state-as-custom-type idea exists only in rust-analyzer. Traces would be the first markdown LSP to color *ambiguity*, for which ticket 26 built an entire taxonomy (`StemTie | HeadingAnchor | PathTie`).

---

## 5. Traces-specific inputs

- **Nothing exists today**: `grep -ri "semantic_token|SemanticTokens" src/` → zero hits. Greenfield feature; `docs/refs/lsp_spec.md` unusable (stub).
- **Ticket 26** (`issues/26-...md`): spine `classify` derives `Resolution = Unresolved | Unique(target) | Ambiguous { kind, candidates }`, **the exact three-valued state to color**. Only link-shaped occurrences plus `DailyDate` can be Ambiguous; tags/fields/schema/footnotes are Unique/Unresolved only (negative pin). Token ranges should mirror 26's occurrence ranges (embed `!` stays outside the token, alias side decision, code blocks excluded by AST). Hover budget `<2ms/<1ms` (supersedes 33's hover row) sets the latency class we live in.
- **Ticket 29**: capability discipline: advertise only what we implement (markdown-oxide's advertised-but-never-set modifiers and iwe's stubbed providers are the named anti-patterns); deliberate non-advertisement precedent (`partialResultProvider`); 29 owns the initialize-time inspection our emission gate reads.
- **Ticket 33**: **no semantic-tokens row exists**; one is proposed in §8. Anchors available: single-file reparse `<10ms`, structural diff `<1ms`, debounce `100ms` (provisional, 25 owns), document symbols `<10ms/<5ms`, diagnostics warm p95 `≤10ms`. Metadata/occurrences always resident (~2MB @20K) → token emission is index-walk only, no parse.
- **a1/a2**: occurrence spans + `nav::NavContext::highlight` spine are the intended source; tokens must be a projection of the same walk (one parse-time cost shared with folding/symbols per b2's shared-walk pattern).

---

## 6. Cross-cutting design constraints

1. **Emission gate = client announced `textDocument.semanticTokens` at all** (29): tokens are pull-based, so the gate is mostly declarative: advertise `semanticTokensProvider` in `initialize` only if the client announced; never send unsolicited token payloads; `workspace/.../refresh` only if `refreshSupport`.
2. **Legend is fixed at registration** (spec §2) covering every kind×state we may ever emit; it cannot extend later per-request; choose the full vocabulary now, gate *emission* per client.
3. **Emit by name regardless of the client's `tokenTypes` list**: all three capable clients render/ignore by name (verified nvim :728-731 unconditional formatting; Zed server-legend map :670; VS Code themed-by-name, medium confidence). Filtering to the announced list would *delete the feature in VS Code* (its list is standard-only) with zero client-side benefit. Keep rust-analyzer's fallback mapping (`standard_fallback_type` → drop) in the design as a dormant safety valve, not the default path.
4. **Single-line, non-overlapping tokens**: VS Code advertises both `false`; split per line (RA `to_proto.rs:793-801` pattern) and skip/filter multiline spans server-side (Marksman `isSingleLine` pattern). Wikilinks/tags are line-local in practice; the 10k-line/file cap bounds the worst case.
5. **Position encoding** rides 26's `convert` module (utf-8-if-offered-else-utf-16); `deltaStart`/`length` are encoding-sensitive.
6. **No reliance on `ServerCancelled`** (nvim `serverCancelSupport=false`); normal `$/cancelRequest` between tiers/batches is enough.
7. **Augment-aware restraint** (RA `to_proto.rs:764-779` lesson): don't emit tokens that merely restate what the grammar already colors with no added state (e.g. a plain `[x](y)` with state=resolved in VS Code); that buys churn and double-styling, not information. Emit when we add information: wikilinks/tags/fields/footnotes/daily-dates (grammar-blind) and **any occurrence whose state ≠ resolved**.
8. **Refresh is genuinely required** (not cargo-cult): creating/deleting note B flips `[[b]]` occurrences in *other* buffers from unresolved↔resolved without any edit to those buffers; clients only re-request on their own edits. Debounced (25's tiers), errors swallowed (25's diagnostics-refresh pattern), only when `refreshSupport`; markdown-oxide does exactly this (source-verified), nvim's `semantic_tokens.lua` comments document the clangd refresh pattern as the sanctioned re-sync mechanism. Spec: client may delay refresh for invisible editors (free backpressure).
9. **Payload bound**: emission is per-document, O(occurrences in that file), never workspace-wide; bounded by 10k-line cap + per-line occurrence density. No spec-defined payload cap (unlike 26's 5,000-refs cap); pathological many-tags-per-line notes are an open question (§10.7).
10. **Dynamic registration / delta / range**: static capability only in v1; add-ons are additive later (§7.2).

---

## 7. Option spaces

### 7.1 Legend axis: what carries the state?

Three candidate shapes (kind map below is provisional: links/embeds→`class`, tags→`enumMember`, frontmatter/inline fields→`property`, footnotes/daily-dates→`label`-ish; refine in review):

| Option | Encoding | Default visibility (VS Code / nvim / Zed) | Pros | Cons |
|---|---|---|---|---|
| **T0 kind-only (leanest)** | standard types by kind, state ignored | kind color renders by default in VS Code/nvim | zero config; marksman parity | **ships the ticket's whole point for nothing**: marksman already colors links statelessly; state (the only thing clients can't get elsewhere) stays invisible |
| **T1 standard types + custom state modifiers** (recommended) | type = kind (standard, always themed); modifiers = bit0 `unresolved`, bit1 `ambiguous`; resolved = 0 | kind color by default; state color appears once user themes `class.unresolved` / nvim `@lsp.typemod.class.unresolved` | kind stays correct-by-default (augment preserved); **one modifier pair covers every kind** (incl. DailyDate-ambiguous, tags-unresolved); nvim's `@lsp.typemod.<type>.<mod>` chain exists precisely for this; legend stays small; no peer precedent = our differentiation, and lossless if the modifiers are ignored | state invisible until themed (docs snippet required); VS Code custom-modifier theming must be spike-verified (§10.1) |
| **T2 state-as-custom-type (RA-style)** | e.g. `unresolvedReference`, `ambiguousReference`, else kind type | unstyled → grammar color (i.e. invisible state) until user themes, and Zed's default rules file confirms even RA's type isn't styled by default | strongest precedent (RA PR #4034, moxide's `comment`-for-unresolved is the standard-type cousin) | custom type **replaces** the kind type → kind color lost for stateful tokens; N kinds × states = legend sprawl; same "must be themed" cost as T1 with less composition |
| **T2b state-primary standard types (moxide-style)** | state picks the standard type: unresolved→`comment`, resolved→kind type | **zero-config state visibility** (unresolved renders comment-colored in VS Code/nvim defaults) | the only option that colors state *out of the box* | loses kind color exactly when state matters; `ambiguous` has no good standard carrier; inconsistent semantics per kind; moxide precedent shows it's acceptable in practice |

- **T1 + a T2b-style escape hatch for `unresolved` is the synthesis to evaluate in Phase B**: if VS Code custom-modifier theming fails verification, fall back to mapping `unresolved` onto a standard type (T2b) for un-themed clients rather than losing the signal.

### 7.2 Transport axis (leanest vs maximally capable)

| Piece | v1 (leanest, recommended) | Later (capable) | Why |
|---|---|---|---|
| `full` | **yes, `full: true`** |  | only request all four clients send; cost = one occurrence walk over a resident snapshot |
| delta | **no** | add `{delta: true}` if §8 bench shows full > budget | RA/VS Code/nvim/Zed all speak it; but at ≤10k-line docs full is doc-symbol-class (~ms); delta is result_id bookkeeping we don't need yet. Adding later is additive (29-safe) |
| `range` | **no** | optional for minimap-scroll micro-optimization | spec frames range as the range-only-server path (§2); markdown-oxide runs `range: Some(false)` in production; Zed never even asks |
| `workspace/.../refresh` | **yes, debounced, swallow-error** (constrained #8) |  | *only* mechanism for cross-file state flips; 3/3 capable clients advertise support |
| dynamic registration | no | no | static at initialize is fine (29: register iff client announced) |
| server-cancel reliance | no (nvim can't) | no |  |

Rejected as over-building: per-request legend negotiation (spec forbids), workspace-scope token APIs (don't exist), emitting tokens inside code blocks (26 excludes them), re-parsing on token requests (ride 25's snapshot).

---

## 8. Recommendation

**Ship T1 with leanest transport.** Standard kind types + `unresolved`/`ambiguous` modifier bits, three-valued straight from 26's `Resolution` (tags/fields/footnotes just never take the `ambiguous` bit), `full: true` only, debounced refresh, grammar-blind occurrences always + any state ≠ resolved, single-line non-overlapping via 26's spans and convert module, static capability gated on client announcement.

Rationale across the three lenses:

- **demand**: the state exists nowhere in any client's grammar/tree-sitter (P8/P9 verified at source), ticket 26 already materializes `Resolution` per occurrence, and navigation is separately solved by `documentLink`, so this feature's *entire* value is state visibility, which T0 would throw away while T1 keeps default kind coloring intact.
- **ecosystem**: 2/6 peers ship tokens; neither uses modifiers or ambiguity; RA proves the "custom vocabulary + user theming + docs snippet" delivery model survives five years in the flagship editor; markdown-oxide proves refresh-after-vault-change and proves state tokens are wanted independently of diagnostics.
- **cost**: emission is an occurrence-walk over resident index metadata (no parse), the cheapest interactive class we have; the only cross-buffers cost is refresh, which we debounce and clients may delay.

**What we deliberately do not build (v1)**: delta, range, dynamic registration, server-cancel, tokens for grammar-covered syntax (RA's augment-aware skip), anything for Helix (status quo: no color, navigation via 26's links).

**Perf rows proposed for ticket 33** (none exist today; anchors = its document-symbols `<10ms/<5ms` row and diagnostics warm `≤10ms`):

| Operation | p95 target | stretch | Strategy |
|---|---|---|---|
| `semanticTokens/full` per visible doc (warm) | **≤10ms** | ≤5ms | occurrence walk on 25's content snapshot + O(1) `Resolution` lookups on always-resident index; no reparse (snapshot already parsed), delta-free |
| emission after `didChange` | rides 25's 100ms debounce + T1 100ms snapshot; **≤5ms** at request time | ≤2ms | zero server work until the client's post-settle request; no per-keystroke tokens |
| `workspace/semanticTokens/refresh` fan-out | **debounced ≥100ms, coalesced 1× per change burst**, errors swallowed |  | bounded by open editors × ≤10ms full; client may delay for invisible editors (spec) |
| (recorded, deferred) `full/delta` | ≤2ms |  | RA `diff_tokens` pattern; only if the full row regresses in `lsp_latency.rs` |

**Provenance roll-up**

| Pick | Label |
|---|---|
| Capability gating, single-line/non-overlapping, position-encoding sensitivity, legend-fixed-at-registration, refresh semantics | demand (spec §2) |
| Kind type map (class/enumMember/property), full-only transport, refresh-after-vault-change, single-line filter, custom-type vocabulary | ecosystem (Marksman/markdown-oxide/rust-analyzer practice) |
| Three-valued state, `ambiguous` modifier, ambiguity-only-for-link-shapes + DailyDate, embed/alias/code-block token boundaries | domain (ticket 26 `Resolution`, `AmbiguityKind`) |
| Emit-by-name over list-filtering; augment-aware restraint | demand + ecosystem (verified client code paths; RA `to_proto.rs` lesson) |
| Deferring delta/range/dynamic-registration/server-cancel | demand-avoidance (29: advertise only what's implemented; nothing requires them) |

---

## 9. Open questions / risks

1. **VS Code custom-modifier theming** (§7.1): verify in Phase B that `editor.semanticTokenColorCustomizations.rules` accepts selectors like `class.unresolved` / `*.*.ambiguous`, and that bare-type rules still match modified tokens (if bare rules stop matching, T1 degrades kind colors for stateful tokens → fall back to T2b for unresolved). Same spike for nvim `@lsp.typemod.class.unresolved` group chain.
2. **Zed default-off + `full` mode**: stock Zed shows nothing until the user flips `semantic_tokens`; a user choosing `full` gets `augments=false`, and we only token entity occurrences → **bold/italic/code/headings lose grammar color entirely** in that mode. Mitigations: document `combined` as the supported mode; verify whether a Zed language config/extension can set per-language `semantic_tokens: combined` for markdown (unverified).
3. **Helix floor**: no semantic tokens ever; wikilinks stay uncolored (status quo: helix's grammar query has no wiki capture either). Accept and document; 26's documentLink covers navigation.
4. **State-primary vs kind-primary default UX**: moxide's unresolved→`comment` gets zero-config visibility we otherwise can't match; decide with a side-by-side rendering test (Phase B matrix) before locking §7.1.
5. **VS Code by-name rendering** for off-list types is medium-confidence (inferred from RA behavior + docs; VS Code client code shows standard-only *advertisement*, not render-time filtering); re-verify in the same Phase B spike.
6. **coc.nvim** row unverified (assumed mirror of VS Code); cheap to confirm before final synthesis.
7. **Payload pathology**: a pathological note (`#a #b #c …` thousands per line) has no spec-level cap; decide whether to reuse 26's constant + `window/logMessage` truncation pattern or accept the 10k-line cap as sufficient.
8. **Embed `!` and alias side**: mirror 26's rename range decisions (token covers `[[...]]`, `!` outside, alias side as display); confirm with 26's owner, don't silently diverge.
9. **Frontmatter vs code-block boundaries**: frontmatter field keys = token (property); fenced code interiors = never (26's AST excludes); inline code = never.
10. **markdown-oxide's exact `range: Some(false)` semantics** and whether its `DECORATOR` legend choice was deliberate design or accident; read `CHANGELOG`/issues before copying any of its conventions.

---

## Appendix: evidence index

- Upstream LSP 3.18 semantic-tokens sections: `/tmp/lsp318.txt` (legend/encoding :14660-14770; client-extension statement :14300; `SemanticTokensClientCapabilities` :15344-15480; range/minimap/flicker :15886-15888; refresh :15946-15995). Source URL in §2; local strip is transient (`/tmp`).
- VS Code grammar: `microsoft/vscode` `extensions/markdown-basics/syntaxes/markdown.tmLanguage.json` (full fetch, 0 `wiki`/`[[` hits); issues microsoft/vscode#138173, kortina/vscode-markdown-notes#121.
- tree-sitter: MDeiml `tree-sitter-markdown-inline/grammar.js` (`EXTENSION_WIKI_LINK`, lines ~100-103/179-196/425-429); nvim-treesitter `queries/markdown/highlights.scm`; helix `languages.toml` + `runtime/queries/markdown/highlights.scm`; Zed `crates/grammars/src/grammars.rs:33-34`, `crates/grammars/src/markdown{,-inline}/highlights.scm`.
- VS Code client: `vscode-languageserver-node` `client/src/common/semanticTokens.ts:54-101` (capability block); rendering: vscode-docs `semantic-highlight-guide.md` (`configuredByTheme` default, built-in themes enable).
- Neovim: `runtime/lua/vim/lsp/protocol.lua` (textDocument.semanticTokens capability: 23 types no `label`, 10 modifiers, range+delta, overlap/multiline true, cancel false, augments true; workspace refreshSupport true); `runtime/lua/vim/lsp/semantic_tokens.lua` (`:1027` enabled, `:829` debounce 200, `:700` priority, `:728-731` group formatting, `:665-688` staleness/clangd-refresh commentary); `runtime/doc/lsp.txt` (group chain, off-spec types, default links).
- Zed: `crates/lsp/src/lsp.rs` (`SEMANTIC_TOKEN_TYPES` :467-495, `SEMANTIC_TOKEN_MODIFIERS` :497-511, capability :1048-1061 incl. `range: None`, refresh :942-944); `crates/project/src/lsp_store.rs:645-647` (augments = `use_tree_sitter()`); `crates/project/src/lsp_store/semantic_tokens.rs` (:96 capability probe, :206 delta response, :354 delta request, :670 legend-name map, :997 delta codec); `docs/src/semantic-tokens.md` (default `off`, modes, `semantic_token_rules`); `assets/settings/default_semantic_token_rules.json` (:189 `label`; no `unresolvedReference`).
- Helix: `helix-lsp/src/client.rs` :586+ full `ClientCapabilities` literal (no `semanticTokens` anywhere, both blocks printed); PR #6102 closed/unmerged.
- Marksman: `docs/digests/lsp_artempyanykh-marksman-digest.txt` (`Semato.fs` :10877-10990, legend :11163, handlers :11887/:11902, tests :17269).
- markdown-oxide: `docs/digests/lsp_feel-ix-343-markdown-oxide-digest.txt` (`src/tokens.rs` :6240-6320 incl. `select_references`/`path_unresolved`/delta fold, capability :5046-5066, refresh :4695-4765, config gate :344/:608).
- Absences: MS service digest (0 `SemanticToken`), rumdl digest (0), iwe digest (0), zk digest false positives (:2446/:3289; `zk-src:19202`).
- rust-analyzer: `crates/rust-analyzer/src/lsp/semantic_tokens.rs` (`SupportedType` incl. `UnresolvedReference`, `standard_fallback_type` :193, `standard_fallback` :264, builder); `crates/rust-analyzer/src/lsp/to_proto.rs:749-816` (augment skip :764-779, non-standard fallback :786-791, per-line split :793-801, delta diff :809-816); `crates/rust-analyzer/src/config.rs:415-467` (toggles + "semantic tokens override other highlighting grammars" doc); PR #4034.
- Tickets: `issues/26-definition-references-hover-rename.md` (Resolution, AmbiguityKind, ranges, hover `<2ms`), `issues/29-client-capability-negotiation.md` (non-advertisement discipline), `issues/33-performance-targets-and-strategy.md` (no token row; anchors used in §8).
- Traces codebase: `grep -ri "semantic_token|SemanticTokens" src/` → 0 hits; `docs/refs/lsp_spec.md:655` stub.
