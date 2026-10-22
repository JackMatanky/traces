# Ticket 27 pass 2, A2: Rust crate/API substrate: tower-lsp-server + ls-types/gen-lsp-types, pulldown-cmark, ropey

Research only. No production code read or written outside what this pass allowed; citations point at
`rust-docs-mcp` crate caches under `.rust-docs/cache/crates/…`, which mirror crates.io source
(0.23.0 and ls-types 0.0.6 are docs-generated caches; 0.24.0-rc.1, pulldown-cmark 0.13.4 and
gen-lsp-types 0.11.0 are source-only caches read directly).

What this answers for ticket 27: exactly what the three crate surfaces **demand** of a
structural-symbols / editor-intelligence implementation (documentSymbol, workspace/symbol,
documentHighlight, foldingRange, selectionRange, documentLink, codeLens, semanticTokens,
codeAction), where those demands interact with settled decisions (tickets 09/11/12/32), and
where crate choice is already forced vs still free. Everything labeled "recommendation" is
ecosystem inference, not an API requirement.

---

## 1. The LSP types crate pairing is version-locked, pick a lane, not a crate

`tower-lsp-server` 0.23.0 (the settled framework, ticket 09) and its types crate are a
**forced pairing**; the rc line is a different pairing:

| tower-lsp-server | types dep (Cargo.toml) | re-export | notes |
|---|---|---|---|
| 0.23.0 (released 2025-12-07) | `[dependencies.ls-types] version = "0.0"` | `pub use ls_types;` (lib.rs:76) | lock resolves latest 0.0.x (cached: 0.0.6). `ls-types` CHANGELOG: 0.0.1→0.0.5 purely additive (registration-options fixes, client-cap additions), no removals (`ls-types/0.0.6/source/CHANGELOG.md`). |
| 0.24.0-rc.1 (unreleased) | `[dependencies.gen-lsp-types] version = "0.11"` | `pub use gen_lsp_types;` (lib.rs:82) | `gen-lsp-types` latest release is 0.11.0. CHANGELOG of tower-lsp-server notes only 0.23.0 as stable in this cache. |

You cannot mix: the `LanguageServer` trait's method signatures use the paired crate's types
(`server.rs` imports), so switching types crates means switching tower-lsp-server lines too.

**Recommendation (not API demand):** stay on 0.23.0 + ls-types 0.0.6, matching every settled
ticket's citations. Revisit at 0.24 final. The rc's gains (§1.3) are real but not needed by
ticket 27's method set.

### 1.1 `Uri` type differs between the lanes, a concrete fork in the road

- **ls-types 0.0.6** (0.23 lane): `Uri` = newtype over `fluent_uri::Uri<String>`, always
  (`ls-types/.../src/uri.rs:14`). tower-lsp-server 0.23's `UriExt` offers
  `to_file_path`/`from_file_path` (ls-types 0.0.1 changelog notes `to_file_path`/`from_file_path` on `Uri`).
- **gen-lsp-types 0.11.0** (0.24-rc lane): `Uri` is feature-dependent
  (`gen-lsp-types/.../src/generated/common.rs:28-68`): **no features = plain
  `pub struct Uri(pub String)` newtype**; `url` feature → `url::Url`; `fluent-uri` feature →
  `fluent_uri::Uri<String>`. tower-lsp-server 0.24-rc depends on gen-lsp-types with **no
  default features** (its `Cargo.toml` only forwards `url-url`/`url-fluent-uri` if opted in).

Consequence if Traces ever moves lanes: path conversion helpers and any `Uri` hashing change
behavior unless the feature is pinned. Ticket 27 handlers all receive
`TextDocumentIdentifier { uri }`, the type of `uri` is decided by this lane choice, not by
ticket 27.

### 1.2 Type-shape parity check for ticket 27's types (0.0.6 vs 0.11.0)

Same names, same wire shapes, two Rust-shape differences found by direct comparison:

- **CodeAction result nesting**: ls-types `CodeActionResponse = Vec<CodeActionOrCommand>`
  (tower-lsp-server 0.23 signature
  `code_action(...) -> Result<Option<CodeActionResponse>>`, `server.rs:1036`). gen-lsp-types
  models the *item* as `enum CodeActionResponse { Command, CodeAction }` (`enum_ors.rs:208`)
  and the result as `Option<Vec<CodeActionResponse>>` (`requests.rs:732-737`,
  `CodeActionRequest::Result`). Identical JSON; different Rust type nesting.
- **Registration-options completeness**: ls-types 0.0.6 is **missing**
  `DocumentLinkRegistrationOptions`, `DocumentHighlightRegistrationOptions`,
  `CodeActionRegistrationOptions`, `FoldingRangeRegistrationOptions`,
  `WorkspaceSymbolRegistrationOptions` (verified absent: full `src/` grep). It has
  `DocumentSymbolRegistrationOptions` (lib.rs:2019) and `TextDocumentRegistrationOptions`
  (lib.rs:1941). gen-lsp-types has all of them (generated from the metamodel):
  `FoldingRangeRegistrationOptions` (structures.rs:378),
  `DocumentHighlightRegistrationOptions` (:3188), `CodeActionRegistrationOptions` (:3521),
  `WorkspaceSymbolRegistrationOptions` (:3612), `DocumentLinkRegistrationOptions` (:3770).
  On 0.23, dynamic registration for those methods must pass
  `Registration::register_options: Option<serde_json::Value>` (lib.rs:1919, field type
  `Option<Value>`) with a hand-built options JSON, workable but hand-rolled.

  Note gen-lsp-types' `TextDocumentRegistrationOptions` serializes `documentSelector: null`
  explicitly rather than skipping it (its own test asserts this,
  `gen-lsp-types/.../src/lib.rs` test `nullable_field`), a wire-conformance nuance, not
  relevant to ticket 27's methods.

### 1.3 What the 0.24-rc lane adds (for the eventual-move note)

Same `LanguageServer` method names, lines shifted (~556-761 in server.rs). New in rc:
`inlineCompletion`, `rangesFormatting`, `textDocument/content`, and server→client refresh
requests exposed on `Client`: `code_lens_refresh` (client.rs:288), `semantic_tokens_refresh`
(:318), `folding_range_refresh` (:467), the last two are directly ticket-27-relevant
(`workspace/semanticTokens/refresh`, `workspace/foldingRange/refresh`, both LSP 3.18,
`gen-lsp-types/.../requests.rs:118-125` `FoldingRangeRefreshRequest`). **tower-lsp-server
0.23.0's `Client` has no folding-range or semantic-token refresh methods**, if ticket 27
wants push-refresh (server notifies client to re-request after an index swap), on 0.23 it
would have to go through lower-level request plumbing; nothing in the settled decisions needs
it (N=1 sequential dispatch, ticket 12).

---

## 2. `LanguageServer` trait, exact signatures ticket 27 implements (0.23.0)

All from `.rust-docs/cache/crates/tower-lsp-server/0.23.0/source/src/server.rs`. Every trait
method is `async fn` with a default body returning
`Err(Error::method_not_found())` + `error!` log, i.e. **every method is opt-in**; unimplemented
methods simply answer "method not found", which clients treat as unsupported. The
`#[rpc(name = "…")]` attribute fixes the wire name (all are stable LSP names, no renames).

| Method | Signature (0.23.0) | line |
|---|---|---|
| `initialize` | `async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult>` | ~501 |
| `document_highlight` | `async fn document_highlight(&self, _: DocumentHighlightParams) -> Result<Option<Vec<DocumentHighlight>>>` | 568 |
| `document_link` | `async fn document_link(&self, _: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>>` | 595 |
| `document_link_resolve` | `async fn document_link_resolve(&self, _: DocumentLink) -> Result<DocumentLink>` | 609 |
| `code_lens` | `async fn code_lens(&self, _: CodeLensParams) -> Result<Option<Vec<CodeLens>>>` | 634 |
| `code_lens_resolve` | `async fn code_lens_resolve(&self, _: CodeLens) -> Result<CodeLens>` | 645 |
| `folding_range` | `async fn folding_range(&self, _: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>>` | 660 |
| `selection_range` | `async fn selection_range(&self, _: SelectionRangeParams) -> Result<Option<Vec<SelectionRange>>>` | 679 |
| `document_symbol` | `async fn document_symbol(&self, _: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>>` | 701 |
| `semantic_tokens_full` | `async fn semantic_tokens_full(&self, _: SemanticTokensParams) -> Result<Option<SemanticTokens>>` | 726 |
| `semantic_tokens_full_delta` | `async fn semantic_tokens_full_delta(&self, _: SemanticTokensDeltaParams) -> Result<Option<SemanticTokensFullDeltaResponse>>` | 747 |
| `semantic_tokens_range` | `async fn semantic_tokens_range(&self, _: SemanticTokensRangeParams) -> Result<Option<SemanticTokens>>` | 773 |
| `code_action` | `async fn code_action(&self, _: CodeActionParams) -> Result<Option<CodeActionResponse>>` | 1036 |
| `code_action_resolve` | `async fn code_action_resolve(&self, _: CodeAction) -> Result<CodeAction>` | 1054 |
| `symbol` (workspace/symbol) | `async fn symbol(&self, _: WorkspaceSymbolParams) -> Result<Option<WorkspaceSymbolResponse>>` | 1220 |
| `symbol_resolve` | `async fn symbol_resolve(&self, _: WorkspaceSymbol) -> Result<WorkspaceSymbol>` | 1240 |

API-demand notes:

- **`None` vs `Some(vec![])`**: all list results are `Option<Vec<_>>` / `Option<…Response>`;
  returning `None` and returning `Some(vec![])` are both valid per the types, but
  `DocumentSymbolResponse`/`WorkspaceSymbolResponse` are **untagged** enums over
  `Vec<SymbolInformation>` (Flat) vs `Vec<DocumentSymbol>`/`Vec<WorkspaceSymbol>` (Nested)
  (`gen-lsp-types/.../enum_ors.rs:662-671-1973-1982`; ls-types has the same untagged
  `DocumentSymbolResponse`, documented in tower-lsp-server server.rs:701 "Flat vs Nested").
  Serde untagged = tries Flat first. Fine for serialization (both flatten to a JSON array of
  objects); be aware `PartialEq`/pattern-matching must handle both variants.
- **selection_range invariant** is documented on the trait method itself (server.rs:672-673):
  result `i` must contain `params.positions[i]` within `result[i].range`. The framework does
  not enforce it; a buggy handler produces client-side selection weirdness, not an error.
- **code_lens/document_link resolve split**: `resolve` handlers receive the full unsaved
  `CodeLens`/`DocumentLink` the client got from the list call, including its `data` field
  (`ls-types/.../code_lens.rs` `CodeLens { range, command, data: Option<Value> }`,
  `document_link.rs` `DocumentLink { range, tooltip, target, data }`). The API **demands**
  that resolve-state travel in `data` (JSON), not server-side maps keyed by id, the
  framework gives no request-scoped state handle between list and resolve.
- **`document_link.target: Option<Uri>`**, `Uri` is the lane-decided type (§1.1);
  `DocumentLink.tooltip: Option<String>` requires the client's `tooltipSupport` client
  capability (`document_link.rs:11-24 DocumentLinkClientCapabilities`).

### 2.1 `ServerCapabilities` fields + options structs (ls-types 0.0.6)

From `ls-types/0.0.6/source/src/lib.rs` unless noted. All are `Option<…>`, **advertise = fill
the field in `initialize`'s `InitializeResult.capabilities`** (`InitializeResult` at lib.rs:1558
{ capabilities, server_info }).

| Capability field | line | Type | Options struct needed |
|---|---|---|---|
| `selection_range_provider` | 1768 | `Option<SelectionRangeProviderCapability>` | enum { Simple(bool), Options(SelectionRangeOptions), RegistrationOptions(SelectionRangeRegistrationOptions) }, selection_range.rs |
| `document_highlight_provider` | 1800 | `Option<OneOf<bool, DocumentHighlightOptions>>` | `DocumentHighlightOptions` (lib.rs:2035, flattens WorkDoneProgressOptions) |
| `document_symbol_provider` | 1804 | `Option<OneOf<bool, DocumentSymbolOptions>>` | `DocumentSymbolOptions` (lib.rs:2005, label + WDP) |
| `workspace_symbol_provider` | 1808 | `Option<OneOf<bool, WorkspaceSymbolOptions>>` | `WorkspaceSymbolOptions` (lib.rs:2042, `resolve_provider` + WDP) |
| `code_action_provider` | 1812 | `Option<CodeActionProviderCapability>` | enum { Simple(bool), Options(CodeActionOptions) }, code_action.rs. **No RegistrationOptions variant in ls-types** |
| `code_lens_provider` | 1816 | `Option<CodeLensOptions>` | `CodeLensOptions { resolve_provider }`, code_lens.rs |
| `document_link_provider` | 1836 | `Option<DocumentLinkOptions>` | `DocumentLinkOptions { resolve_provider, work_done_progress_options }`, document_link.rs |
| `folding_range_provider` | 1844 | `Option<FoldingRangeProviderCapability>` | enum { Simple(bool), Options(StaticTextDocumentColorProviderOptions), FoldingProvider(FoldingProviderOptions) }, folding_range.rs:24. Quirk: the `Options` variant reuses `StaticTextDocumentColorProviderOptions` (same shape: WDP only); use `Simple(true)` or `FoldingProvider(default)` |
| `semantic_tokens_provider` | 1864 | `Option<SemanticTokensServerCapabilities>` | enum { SemanticTokensOptions(SemanticTokensOptions), SemanticTokensRegistrationOptions }, semantic_tokens.rs:456 |

`OneOf` is `Either`-style (lib.rs:1678). `bool` = "supported, no options";
prefer the options-struct variant whenever resolve/dynamic-registration is wanted.

Semantic-tokens specifics (semantic_tokens.rs): `SemanticTokensOptions { legend, range:
Option<bool>, full: Option<bool | SemanticTokensFullOptions> }` (:423; `SemanticTokensFullOptions`
{ Bool, Delta { delta: Option<bool> } } :410, **advertise `Delta { delta: Some(true) }` only if
implementing `semantic_tokens_full_delta`**, which needs a `result_id` bookkeeping scheme).
`SemanticTokensLegend { token_types: Vec<SemanticTokenType>, token_modifiers: Vec<SemanticTokenModifier> }`
(:143) is **server-defined strings**, `SemanticTokenType` is a `Cow<str>` newtype with named
consts (namespace, type, class, enum, function, variable, parameter, property, enumMember,
event, method, keyword, modifier, comment, string, number, regexp, operator, semantic_tokens.rs
consts block); custom token types (e.g. `wikilink`, `tag`, `inline-field`) are legal strings but
client rendering depends on client support, so a legend of standard types + modifiers (e.g.
modifier `deprecated`) is the portable baseline, **that mapping choice is a design decision
(ticket 27), not an API constraint**. Wire format: `SemanticTokens { result_id, data: Vec<SemanticToken> }`
with `SemanticToken { delta_line, delta_start, length, token_type, token_modifiers_bitset }` all
`u32`, custom serde flattening each token into five interleaved ints (:152-158, :247-251) , 
you never hand-build the flat array. `SemanticTokensDelta`/`SemanticTokensEdit` also provided
(:~263-296) for the delta path.

Client capabilities (what to gate on before advertising fancy variants):
`TextDocumentClientCapabilities` (lib.rs:1286) carries per-method caps , 
`document_highlight` (:1307), `document_symbol` (:1311, incl.
`hierarchical_document_symbol_support: Option<bool>`, document_symbols.rs; if absent/false the
server must send the **Flat** `SymbolInformation` form, not nested `DocumentSymbol`),
plus `folding_range`, `selection_range`, `document_link`, `code_lens`, `code_action`,
`semantic_tokens` fields (lib.rs:1333-1460 region; `SemanticTokensClientCapabilities` at
gen-lsp-types structures.rs:9995 / ls-types semantic_tokens.rs, `requests: { id, delta, range }`
declares which of the three token requests the client will send). `CodeActionClientCapabilities`
(code_action.rs:31+): `code_action_literal_support` (required to send `CodeAction`-literal
actions rather than bare `Command`s), `resolve_support` (:68, `CodeActionCapabilityResolveSupport`
:87 listing `edit`/`command`/`title`, the `data`-based lazy-fill contract),
`data_support` (:61), `disabled_support` (:53), `honors_change_annotations` (:78).
`WorkspaceSymbolClientCapabilities.resolve_support` (workspace_symbols.rs,
`WorkspaceSymbolResolveSupportCapability`) gates `symbol_resolve`, but since ticket 27's
workspace-symbol pass (see §5) is read-only/`SymbolInformation`-shaped, resolve is unnecessary.

`WorkspaceClientCapabilities` (lib.rs:1123) holds `symbol` and
`semantic_tokens.refresh_support` (`SemanticTokensWorkspaceClientCapabilities`, gen-lsp-types
structures.rs:9074) / `code_lens.refresh_support` (:9095), only relevant to the 0.24-rc
refresh methods (§1.3), which we don't need on 0.23.

### 2.2 Dynamic registration (optional path)

Mechanism: after `initialize` responds, call `Client::register_capability(Vec<Registration>)`
(tower-lsp-server 0.23 `service/client.rs:103-108` → `client/registerCapability` request).
`Registration { id, method, register_options: Option<serde_json::Value> }` (ls-types lib.rs:1919).
Requires the client's `general.dynamicRegistration` sub-capabilities for each method
(`DynamicRegistrationClientCapabilities`, e.g. `DocumentHighlightClientCapabilities =
DynamicRegistrationClientCapabilities`, document_highlight.rs; `CodeLensClientCapabilities =
DynamicRegistrationClientCapabilities`, code_lens.rs).

**API-demand conclusion:** static capabilities in `InitializeResult` are strictly simpler and
cover everything ticket 27 needs; dynamic registration only pays off if the feature becomes
configurable at runtime (ticket 31 config). If used on ls-types 0.0.6, five of the methods have
no typed RegistrationOptions struct (§1.2), hand-build the `Value`.

### 2.3 Cancellation / dispatch interaction (ticket 12 cross-check)

Verified in `tower-lsp-server/0.23.0/source/src/service/pending.rs`:
`Pending(Arc<DashMap<Id, AbortHandle>>)`, each handler wrapped in `future::abortable`;
on `$/cancelRequest` the handle is aborted and the response is
`Error::request_cancelled()` (JSON-RPC code **-32800**). `transport.rs:99`
`concurrency_level(mut self, max: usize)` sets the `buffer_unordered(N)` width. Ticket 12 chose
`concurrency_level(1)` + stale-result discard, so the abort machinery exists but is effectively
idle (with N=1 a cancel arrives only after the handler already finished, or is handled by the
supersede-in-queue semantics). No ticket-27-specific cancellation work follows from the API.

---

## 3. pulldown-cmark 0.13.4, event/offset API for symbol extraction

Source: `.rust-docs/cache/crates/pulldown-cmark/0.13.4/source/src/`. Traces already pins
`pulldown-cmark = { version = "0.13.4", default-features = false }` (root Cargo.toml:136) , 
with that, only the binary-target deps (getopts) and the HTML renderer (`html` →
pulldown-cmark-escape) are off (Cargo.toml [features]: default = getopts+html, simd gated
separately). **All `Options` flags are runtime bitflags, not cargo features**, disabling
default features costs nothing for parsing (lib.rs:650+).

### 3.1 Spans: `into_offset_iter` is the span source (already used, partially discarded)

- `Parser::into_offset_iter(self) -> OffsetIter` (parse.rs:1434).
- `OffsetIter: Iterator<Item = (Event<'a>, Range<usize>)>` (parse.rs:2163-2175), ranges are
  **byte offsets into the source string**.
- End events carry the node's full span (parse.rs:2188-2192: `item.start..item.end` on pop);
  start events carry the node's opening span. **Tight paragraphs emit nothing** (parse.rs:2181-2186,
  2194-2202), a known sharp edge when reconstructing document structure from events.
- Ticket 11 already decided spans are captured here (`src/note/parser.rs` currently iterates
  `into_offset_iter()` and discards ends, map baseline). Ticket 27's symbol/folding/highlight
  producers consume the same stream; the API **demands** nothing new beyond keeping the
  `(Event, Range)` pairs.

### 3.2 Event vocabulary relevant to ticket 27 (lib.rs line numbers)

- `Tag` (lib.rs:154): `Paragraph`, `Heading { level, id, classes, attrs }`, `BlockQuote(Option<BlockQuoteKind>)`,
  `CodeBlock(CodeBlockKind)`, `List(Option<u64>)`, **note: 0.13 dropped `ListType`; ordered-ness
  is `Some(start)`, bullet is `None`**, `Item`, `FootnoteDefinition(CowStr)`,
  `Table(Vec<Alignment>)`, `TableHead`, `TableRow`, `TableCell`, `Emphasis`, `Strong`,
  `Strikethrough`, `Link(LinkType, CowStr dest_url, CowStr title)`, `Image(...)`,
  `HtmlBlock`, `MetadataBlock(MetadataBlockKind)`, `DefinitionList`, `DefinitionListTitle`,
  `DefinitionListDefinition`, `Superscript`, `Subscript`, `FootnoteReference`, `WikiLink(CowStr)`.
- `TagEnd` (lib.rs:384): mirror ends, e.g. `Heading(u8)`, `List(bool /*is_tight*/)`,
  `BlockQuote`, `CodeBlock`, `Link`, `WikiLink`.
- `Event` (lib.rs:524): `Start(Tag)`, `End(TagEnd)`, `Text`, `Code`, `Html`, `InlineHtml`,
  `FootnoteReference`, `SoftBreak`, `HardBreak`, `Rule`, `TaskListMarker(bool)`.
- Helpers: `HeadingLevel` (lib.rs:432, as u8 via `into()`), `BlockQuoteKind`
  (Note/Tip/Important/Warning/Caution, lib.rs:135), `MetadataBlockKind`
  (YamlStyle/PlusesStyle, lib.rs:146), `CodeBlockKind` (Indented/Fenced(CowStr), lib.rs:108),
  `LinkType` incl. `WikiLink { has_pothole: bool }`.
- `Options` flags (lib.rs:650-736): `ENABLE_TABLES(1<<1)`, `ENABLE_FOOTNOTES(1<<2)`,
  `ENABLE_STRIKETHROUGH(1<<3)`, `ENABLE_TASKLISTS(1<<4)`, `ENABLE_SMART_PUNCTUATION(1<<5)`,
  `ENABLE_HEADING_ATTRIBUTES(1<<6)`, `ENABLE_YAML_STYLE_METADATA_BLOCKS(1<<7)`,
  `ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS(1<<8)`, `ENABLE_OLD_FOOTNOTES`,
  `ENABLE_MATH(1<<10)`, `ENABLE_GFM(1<<11)` (blockquote alert tags), `ENABLE_DEFINITION_LIST(1<<12)`,
  `ENABLE_SUPERSCRIPT(1<<13)`, `ENABLE_SUBSCRIPT(1<<14)`, **`ENABLE_WIKILINKS(1<<15)`**.

**API-demand vs design:** the crate gives zero-cost byte spans per event and native wikilink
tokenization (`ENABLE_WIKILINKS` + `Tag::WikiLink`), Traces' existing `Options` choice (what
`src/note/parser.rs` enables) is the single point where symbol coverage (e.g. do `[[…]]`
fragments appear as symbols?) is decided; the crate itself imposes no constraint. Heading `id`
(attributes) is populated only with `ENABLE_HEADING_ATTRIBUTES`, relevant to `#anchor`
link targets and symbol detail.

---

## 4. ropey 1.6.1, position conversion API (and a ticket-11 discrepancy)

Source: `.rust-docs/cache/crates/ropey/1.6.1/source/src/rope.rs`. All conversions are
O(log N) tree walks; panicking methods have `try_` counterparts returning `Result`.

| Need | Method(s) | Location |
|---|---|---|
| byte → char | `byte_to_char(usize) -> usize` (mid-char byte floors to char start; one-past-end OK) | rope.rs:634 |
| char → utf16 CU | `char_to_utf16_cu(usize) -> usize` | rope.rs:706 |
| utf16 CU → char | `utf16_cu_to_char(usize) -> usize` (mid-CU floors to char) | rope.rs:727 |
| char count in utf16 | `len_utf16_cu()` | rope.rs (id 15/248) |
| line ↔ byte/char | `byte_to_line` :653, `line_to_byte` :745, `line_to_char`, `char_to_line` | rope.rs |
| fallible | `try_byte_to_char` :61-area (rope.rs:1422+ region), `try_char_to_utf16_cu`, `try_utf16_cu_to_char`, `try_line_to_byte` :1495, etc. | rope.rs |

**Finding, no direct byte→UTF-16 method exists.** `byte_to_utf16_cu` / `utf16_cu_to_byte` /
`line_to_utf16_cu` are absent from the public API (full-crate symbol search: zero hits; the
string-level helper `byte_to_utf16_surrogate_idx` exists in `str_utils` but is not re-exported
as a Rope method). LSP `Position.character` (UTF-16 by default per ticket 11's wire analysis)
therefore requires **composition**: `rope.byte_to_char(b).pipe(rope.char_to_utf16_cu)`, two
O(log N) walks, or, per line,
`line = rope.byte_to_line(b); cu = rope.char_to_utf16_cu(rope.line_to_char(line))
       + (utf16 length of the line prefix up to b)`.

**Ticket-11 cross-check (flag for pass-2 owners):** ticket 11's answer text names
`rope.line_to_utf16_cu(rope.byte_to_line(offset))` as the live-buffer conversion.
**`Rope::line_to_utf16_cu` does not exist in ropey 1.6.1** (and `utf16_cu_to_line` doesn't
either). The decision's intent is implementable, closest exact composition:
`rope.char_to_utf16_cu(rope.byte_to_char(offset))` (equivalently
`rope.char_to_utf16_cu(rope.line_to_char(rope.byte_to_line(offset))) + prefix`), but the
method name in the decision text must be corrected to avoid a compile-time surprise during
implementation. This is an **API-demand correction**, not a design change: ropey remains the
right live-buffer type (ticket 14), and the two-concrete-functions boundary (no `TextBuffer`
trait) stands.

---

## 5. What is forced vs free (ticket-27 decision surface)

**Forced by settled tickets / crate APIs:**
- Framework + types crate + `Uri` type: tower-lsp-server 0.23.0 + ls-types 0.0.x (ticket 09;
  §1 pairing).
- Handler shapes: the §2 table signatures; opt-in via default trait bodies.
- Sequential dispatch N=1, stale-result discard; no framework cancellation work (ticket 12;
  §2.3).
- Span source: pulldown-cmark `into_offset_iter` byte ranges into the AST (ticket 11; §3.1).
- Live-buffer positions: ropey `Rope` (ticket 14) with corrected conversion composition (§4).
- Heading model: flat list + `level: u8`, hierarchy derived on demand (ticket 11).
- Ownership context: rumdl cedes heading symbols (`enableSymbols`) and Traces owns
  wikilink/cross-file-anchor intelligence (ticket 32); standard-link documentLink overlap
  with rumdl's `enableLinkNavigation` (ticket 03) is a **design** question for the pass-2
  symposium, not an API question.

**Free (crate imposes nothing):**
- Which of the nine methods ticket 27 actually implements first (each is independently
  opt-in); whether symbols are Flat or Nested (client-cap gated, §2.1);
- static vs dynamic registration (§2.2);
- semantic-token legend mapping (standard types + modifiers vs custom strings, §2.1);
- codeLens/documentLink lazy-resolve strategy via `data` (API demands data-carried state,
  but which fields are lazy is free);
- folding-range kind mapping (Comment/String/Region consts on `FoldingRangeKind`,
  ls-types folding_range.rs) and collapse-cell/`collapsed_text` (3.18, not in ls-types 0.0.6,
  so unavailable on 0.23 lane without raw field injection);
- refresh/push via `Client` methods, only on 0.24-rc (§1.3).

## 6. Ecosystem notes (recommendations, clearly non-binding)

- tower-lsp-server 0.23 is the ecosystem default per ticket 09's research; the 0.24-rc lane's
  gen-lsp-types is codegen'd from the LSP metamodel and is strictly more complete on
  registration-options types (§1.2), worth adopting at final release, not now.
- pulldown-cmark 0.13's `Event`+`OffsetIter` design is the same primitive markdown-oxide-class
  servers use; no competing markdown parser is needed (ticket 34's packaging ticket owns dep
  bloat, and the parser is already a core dep).
- ropey is the de facto rope (helix, many editors); 1.6.1 is current-in-cache; no alternative
  needed.
