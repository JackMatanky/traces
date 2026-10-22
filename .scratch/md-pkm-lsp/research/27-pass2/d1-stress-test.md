# D1 stress test: adversarial review of ticket 27's provisional decision set (pass 2)

Scope: the 16 provisional decisions (D1-D16) distilled from the pass-2 consolidation
(`research/27-structural-symbols-editor-intelligence-pass2.md`, Parts II-VIII and XI) for
[ticket 27](../../issues/27-structural-symbols-and-editor-intelligence.md). Method: each decision
was re-checked against its cluster report (`research/27-pass2/b1`-`b7`), the substrate reports
(`a1-codebase.md`, `a2-crates.md`), and the live tickets it touches (11, 15, 16, 17, 22, 23, 25,
26, 28, 29, 31, 32, 33, 41). Verdicts: **SAFE** (ratify as written), **WEAK** (sound design,
missing text/pins/gates listed below), **CONFLICT** (contradicts a settled ticket or another
decision; must be adjudicated before ratification), **BLOCKED** (cannot ratify until an
unresolved prerequisite lands). No decision is BLOCKED.

## 1. Verdicts

| # | Decision (as distilled) | Verdict | One-line reason |
|---|---|---|---|
| D1 | documentSymbol outline = headings + tasks as section children (Array kind), dual shape | WEAK | Orphan/preamble attachment and section-end corner unstated; perf row re-anchor needed (F7, F1) |
| D2 | foldingRange E-lite (headings/fences/frontmatter/callouts/lists), gate on 17's CalloutBlock | WEAK | 17's `is_foldable: bool` cannot encode Obsidian's three-state; mapping is open, not gated (F2); collapsedText typed-crate question (F9) |
| D3 | selectionRange S1 trimmed (header + block tiers, minimal inline), position-local | SAFE | Spec invariant noted (a2-crates:130-132); zero-peer-precedent is acceptable for an additive feature |
| D4 | documentLink Option A: emit every resolvable class, ranges = 26's Occurrence spans, eager, `resolveProvider: false` | SAFE | VS Code `_union` dedup requires span overlap with the built-in's hrefRange; verify at implementation (F4b) |
| D5 | documentLink drops unresolved internals; 25's diagnostic + 15's create-note carry the affordance | CONFLICT | Contradicts pass 1's reading of standing constraint #1 (Part X item 2) and strands users who set `links.wikilink.unresolved = off` (F3) |
| D6 | ambiguous links: deterministic first-candidate target + count tooltip | WEAK | 26/15 sign-off still outstanding (Part XI item 4); Neovim `tooltipSupport = false` makes the ambiguity signal invisible there (F5) |
| D7 | semantic tokens T1: standard kinds + `unresolved`/`ambiguous` state modifiers, T2b fallback | WEAK | VS Code modifier-theming spike unverified (Part XI item 5); three pins must survive into the decision text (F11) |
| D8 | debounced refresh (semantic tokens, CodeLens), hand-rolled on tower-lsp-server 0.23.0 | WEAK | Refreshes are server->client *requests*; 0.23.0 `Client` has no methods; must share the mechanism with 25's diagnostic refresh and gate on `refreshSupport` (F4) |
| D9 | staged CodeLens: documentHighlight now; note-level eager counts (Option C) then heading lazy resolve (Option A) | WEAK | Zero-reference suppression policy, stale-`data` guard, and never-pre-serialized-`Location[]` pin dropped in distillation (F10) |
| D10 | four code-action kinds: convert.toWikilink/toStandardLink (Unique-only, disabled-with-reason), extract.note (eager CreateFile), rewrite.cycleStatus | WEAK | `codeAction.disabledSupport` gate and CreateFile ownership seam unstated (F8) |
| D11 | accept VS Code built-in duplication; whole-extension disable as escape hatch | WEAK | Honest but the escape hatch is a heavy hammer; Marksman's client-identity cession precedent not weighed (F12) |
| D12 | workspace/symbol: eager full `Location`, do not advertise resolve, cap 128, empty query = first-128 by (path,line) | SAFE | Matches b7:43 (resolve pays for VS Code only) and r-a's named 128 constant (b7:109); fairness question parked to 29 |
| D13 | perf rows: doc symbols <5/<10, workspace/symbol <50ms@20K, folding <30/<10, selection <10/<5, documentLink <=5/<=10 | CONFLICT | Mis-cites 33's Document-symbols row twice and contradicts 33:53's index-served strategy with a parse-served design (F1) |
| D14 | template refs on both channels: goto-definition independent of 22's spans; documentLink gated on 22's exact spans | SAFE | Matches 22:70's Q24(b) flag and Part XI item 10; minijinja version mismatch is 22's problem, carried as risk (F13b) |
| D15 | outward records: 23 Section 6 amendment, 16 tag-kind, 11 ropey correction | WEAK | Record set incomplete: missing 29, 32, 33, 22, 41, 15/26, 17 (F6) |
| D16 | static `initialize` capabilities + initialize-time config gates (rumdl model); dynamic registration reserved for file watchers | WEAK | Runtime-disable semantics and config live-reload ownership (31, open) unstated (F13) |

Tally: 4 SAFE, 10 WEAK, 2 CONFLICT, 0 BLOCKED.

## 2. Findings (severity-ranked)

**F1 (High) D13's perf rows mis-cite ticket 33 and contradict its serving strategy.**
33:53 records Document symbols target **<10ms** / stretch **<5ms** with strategy "Headings from
in-memory index". Three defects follow. (a) b1-symbols.md's D10 restates the row as "doc symbols
p95 <5 ms headings-only / <10 ms with tasks", reading the target/stretch pair as two workload
variants; the axes are flipped relative to 33's convention. (b) b2-folding-selection.md:134 and
b4-codelens-inlay.md:259 both cite "Document-symbols row `<30ms / <10ms` in 33" as the class
anchor for folding; no such row exists in 33 (33:47-55). (c) 33:53's strategy says headings come
from the in-memory index, but pass 2's design serves document symbols from a per-document parse
via 14's ContentResolver (14:27; 23:46 pins the same serving model for tasks); the row must be
re-anchored to a per-document workload, and tasks-as-children (D1) changes it again. The
workspace/symbol <50ms row (b1 D10) correctly fills 33:117's unfilled slot, but its derivation
must follow 33:117's stated method (bench_query_tasks_density at the 20K anchor with a stated
mean->p95 factor), not a free-floating number.
*Fix:* one corrected perf block owned by 27, filed to 33: target <10 / stretch <5 headings-only,
target <10 / stretch n-a with tasks (re-measured), folding <30/<10 and selection <10/<5 as new
rows, workspace/symbol <50ms@20K derived per 33:117; delete both false `<30/<10` citations.

**F2 (High) D2 gates on 17's `is_foldable: bool`, but the field cannot express Obsidian's
three-state semantics; the mapping is open, not settled.**
17:74 records `CalloutBlock { ..., is_foldable }` with the parenthetical "(+/- modifier)".
b2-folding-selection.md:72 documents the real semantics: no modifier = collapsible/expanded,
`-` = starts collapsed (still collapsible), `+` = not collapsible. A bool cannot carry three
states, and b2:158 itself leaves "confirm with 17 owner" open. The failure mode is an
implementer reading `is_foldable` as "the +/- modifier disables folding" and inverting behavior
on `-` callouts. Separately, `-` (initially collapsed) is not expressible in LSP FoldingRange on
the 0.23 lane at all: there is no "initially collapsed" field, only `collapsedText` (b2:26), so
the mapping collapses to "foldable = not `+`", with initial-collapse dropped (or client-side).
*Fix:* 27 records the mapping rule and files an amendment to 17 (bool -> three-state enum, or
`is_foldable` documented as "not `+`"); D2's gate text names the mapping, not just the handoff.

**F3 (High) D5's unresolved-drop leaves a silent hole when the diagnostic is configured off, and
its affordance handoff depends on two other tickets' lanes.**
25:40 gives every diagnostic key, including `links.wikilink.unresolved`, the override vocabulary
`hint|info|warn|error|off`; 25:47 forbids any registry key from firing on Ambiguous (so ambiguity
gets no diagnostic either). With D5 dropping unresolved documentLinks, severity `off` yields:
no link, no diagnostic, no 15 create-note action (it rides the diagnostic quickfix lane,
b3-document-links.md:255-261), leaving only 26's informational unresolved hover (26:37). That is
a defensible user choice, but the interaction is nowhere stated, and D5 also still contradicts
pass 1's "never drop" reading (Part X item 2), which the consolidation explicitly queued for
grilling adjudication. *Fix:* grilling adjudicates D5 vs pass 1 per Part X item 2; whichever way
it lands, the decision text states the three-way handoff (diagnostic severity default,
create-note quickfix dependency, hover residual) and the `off` = fully silent consequence.

**F4 (Medium) D8's refresh mechanism is mis-modeled in three ways.**
(a) `workspace/semanticTokens/refresh` and `workspace/codeLens/refresh` are server-to-client
*requests* (LSP 3.18), not notifications; a2-crates.md:80-89 confirms tower-lsp-server 0.23.0's
`Client` exposes neither (only 0.24-rc adds `code_lens_refresh`/`semantic_tokens_refresh`/
`folding_range_refresh`). Hand-rolling means raw request plumbing plus swallowed errors.
(b) 25:24 already commits to sending `workspace/diagnostic/refresh` unconditionally with a
swallowed error from the beta publisher's `Client` clone; three refreshes now need the same
mechanism, and D8 invents a parallel one. One shared "send refresh request, swallow error"
helper belongs at the client/facade layer, gated on client capability.
(c) Client support is uneven (b7:47): Neovim and VS Code advertise codeLens + semanticTokens +
foldingRange refresh; Zed advertises codeLens + semanticTokens but not foldingRange. Sending
refresh to a client that never advertised `workspace.*.refreshSupport` is wasted work at best.
*Fix:* D8 names refresh as a request, routes through the shared helper (owed to 25 and 29), and
gates each send on the corresponding client workspace capability.

**F5 (Medium) D6's ambiguity tooltip is invisible on the client that most needs eager targets.**
Neovim advertises `tooltipSupport = false` (b7-clients matrix row 5, protocol.lua:529-532) and
its `gx` never calls resolve, so a first-candidate target with a "3 matches" tooltip presents as
an unconditional jump to a possibly-wrong note with zero signal. b3:264-265's tooltip text is the
only channel (no protocol concept for broken/ambiguous links, b3:44-45), and 25:47 blocks a
diagnostic fallback. 26's consumer matrix (26:27) does not cover documentLink emission at all,
and 15 forbids diagnosing ambiguity, so neither has signed off. *Fix:* obtain 26/15 sign-off
(Part XI item 4); document the Neovim limitation explicitly in the decision; consider
first-candidate-plus-no-target only for Neovim if a capability probe is deemed worth it (default:
no, capability is static at initialize so the branch is cheap, but it splits behavior).

**F6 (Medium) D15's outward-record set is incomplete.**
The distilled set names 23 Section 6 (eager Location amendment, 23:49-51 -> 29 ratification),
16 (tag kind / Marksman tags-in-outline precedent, consolidation section 0.11), and 11 (the
non-compiling `rope.line_to_utf16_cu` citation, a2-crates.md:309-318, consolidation section 0.9;
correct idiom `rope.char_to_utf16_cu(rope.byte_to_char(offset))`). Missing: 29 (capability
fields for all nine methods + deliberate non-advertisement list: workspaceSymbol/resolve,
partialResultProvider, per b1 D6 and 29:24); 33 (F1's corrected rows); 32 (VS Code duplication
decision D11 + rumdl `enableSymbols=false` companion setting, 32:8-16); 22 (Q24(b) restatement
per 22:70's timing fallback clause); 41 (demand statement: 27's document-local features need
transient spans only; 41-C honest-absence is acceptable, a1-codebase.md:248-252 and 41:29);
15/26 (ambiguous-documentLink emission sign-off, F5); 17 (F2's mapping amendment).
*Fix:* expand D15 to the nine-ticket record set above, each with the exact section to touch.

**F7 (Medium) D1 leaves task-outline edge cases unstated.**
23:46 attaches tasks to their nearest heading section by line and to the root otherwise, but
neither 23 nor the D-set states what the root symbol *is*: 17's File(17) kind is named in b1 D4
for notes, yet a first-heading-orphan task (before any heading) and preamble/frontmatter-region
tasks need an explicit container (document root symbol or an implicit "" root). b2's section-end
corner (last-line task leaf can exceed its parent heading's range by one line, Part XI item 8)
is likewise unrecorded in the decision text. *Fix:* state root-attachment = document root
pseudo-symbol (or first-heading fallback, pick one), record the section-end corner as
non-normative, both in D1.

**F8 (Medium) D10 omits two gates.**
(a) `disabled_support` exists on `CodeActionClientCapabilities` (a2-crates.md:193,
code_action.rs:53); disabled-with-reason conversion actions are only rendered as disabled on
clients that advertise it, and Helix filters disabled actions out entirely (b5), so Unique-only
refusal renders as absence there. The capability gate must be stated, and the Helix absence
documented as accepted. (b) `refactor.extract.note` emits `CreateFile { ignore_if_exists: true }`
inside a WorkspaceEdit; 26 owns server-initiated `RenameFile` (28:17) but no ticket owns
code-action-originated `CreateFile` emission or its interaction with 28's future
`didCreateFiles` handling. D10 should state: 27 owns CreateFile emission and the shared
`create in resourceOperations` predicate (b5 already pairs it with 25); 28's file-operation
handling treats it like any client-side create when it lands.
*Fix:* add both sentences to D10.

**F9 (Low) D2's "collapsedText always" may be untypeable on the 0.23 lane.**
b2:26 and b2:132 treat `FoldingRange.collapsedText` as a 3.17 field to send unconditionally,
gated on the client's `collapsedText` capability. a2-crates.md:347 reports collapsed_text as
absent from ls-types 0.0.6 (present in gen-lsp-types for the 0.24-rc lane), meaning the typed
crate cannot carry it without raw-field injection, the same untyped-`Value` discipline already
budgeted for registration options (a2-crates.md:69-71). The two reports disagree; one is wrong.
*Fix:* verify `ls-types 0.0.6` `FoldingRange` at implementation start; if the field is missing,
either inject a raw field (wire-legal, serde-flatten) or defer collapsedText to the 0.24 move.
Record the verification as a D2 implementation note.

**F10 (Low) D9 drops three load-bearing policies from b4.**
(i) Zero-reference suppression: peers universally drop zero-count reference lenses, but b4
explicitly says "decide consciously"; the distilled set neither adopts nor rejects it.
(ii) Stale-`data` guard: CodeLens resolve payloads must carry `{uri, range, version}` and
identity-on-stale (the r-a pattern b4 cites), the same discipline b5 pre-specifies for code
actions. (iii) Never moxide's pre-serialized `Location[]` payload: doing so bypasses 26's 5,000
-reference ceiling and caused a 2.93s Neovim bufload stall at ~100 refs (b4's receipt). All three
exist in b4 and vanish in the D9 summary.
*Fix:* re-attach all three to D9 verbatim.

**F11 (Low) D7 must retain three pins from b6.**
(a) Never filter emitted tokens against the client's advertised `tokenTypes`/`tokenModifiers`
lists: filtering would delete wikilink coloring in VS Code, whose TextMate grammar is verified
wiki-blind (consolidation section 0.4), making tokens the only first-party channel. (b) Emit
single-line, non-overlapping tokens: VS Code advertises `multilineTokenSupport = false` and
`overlappingTokenSupport = false` (b7:45). (c) Do not advertise `full.delta` unless a
`result_id` bookkeeping scheme ships (a2-crates.md:166-167); D7 says full-only, so the negative
advertisement must be explicit. *Fix:* pin all three in D7's text.

**F12 (Low) D11's escape hatch deserves one more comparison.**
b7:132 confirms the VS Code built-in has no per-feature kill switch; whole-extension disable
trades away its diagnostics and documentLinks too. b7:103 records Marksman's live precedent:
disable symbol providers *for VS Code clients specifically* (client-identity gating at
initialize). Traces' differentiation story argues against ceding symbols in VS Code, so
accept-duplicates remains the right call, but the decision should record why the Marksman lever
was rejected (it would delete Traces' task outline and wikilink-aware symbols exactly where the
user can see them). *Fix:* one rejection sentence in D11; no design change.

**F13 (Medium) D16's lifecycle story is incomplete.**
(a) Runtime disable: static capabilities are advertised once at initialize (b7:67), but 31 (open,
31:1-23) will own live config reload; if a user flips an LSP feature toggle via
`workspace/didChangeConfiguration`, the server cannot un-advertise. The opt-out mechanism is the
framework default body: unimplemented methods answer `method_not_found` and clients treat the
feature as unsupported (a2-crates.md:96-98). D16 should state: initialize-time gates decide
*advertisement*; runtime toggles make handlers return empty results (or method-not-found for
strong disable), sticky-until-restart where 22's `reset_engine` precedent applies (22:65).
(b) rumdl's precedent reads gates at initialize only (b7:58); Traces should not promise more
than the precedent until 31 lands. *Fix:* add the two-sentence runtime-semantics clause; file
the live-reload interaction to 31 as an input.

**F13b (Low) Template-links risk register should carry the minijinja version mismatch.**
22 pins `=2.24.0` with `unstable_machinery` (22:24); Traces ships minijinja 3.0.1 with no
machinery feature (a1-codebase.md:303-306, Cargo.toml:131). D14's gating is correct, but the
implementation risk (22 must change the dep before template documentLinks can exist) belongs in
D14's risk notes so 27's template-link work items cite the real blocker.

**F14 (Low) The ticket-11 ropey citation needs a correction record, not just a D15 mention.**
22... correction: ticket 11's answer names `rope.line_to_utf16_cu(...)`; ropey 1.6.1 has no such
method (a2-crates.md:300-318: full-crate search zero hits). The intent is implementable via
`rope.char_to_utf16_cu(rope.byte_to_char(offset))`. D15 lists "11" as a record target but the
distilled text does not say *what* to record; an implementer copying 11's wording gets a
compile error. *Fix:* D15's 11-record = "correct the method name in the decision text; intent
unchanged".

## 3. Overlooked capabilities (not covered by any of D1-D16)

1. **Workspace-symbol vs document-symbol tag asymmetry.** 23:50(c) assigns one shared cap across
   tags/headings/tasks for workspace/symbol, so tags *do* appear in the workspace picker while
   D1/B1 exclude them from the document outline (16 pin). The asymmetry is defensible (hide-axis
   via `outline.show*` is a document-outline affordance only) but appears nowhere in the D-set;
   state it or 29's ratification will trip on it.
2. **Unsaved/new files are absent from workspace/symbol until index refresh.** The workspace walk
   reads resident index metadata (a1:27-33, 23:50's direct `Note::tasks()` walk); a brand-new
   unsaved note exists only in 14's overlay. 13's single-file RefreshPlan constructor is
   design-only (a1:285-286), so there is a freshness window. One sentence in D12 ("workspace
   symbols reflect the last index refresh; document-scope features are overlay-authoritative")
   closes it.
3. **Flat `SymbolInformation` needs a `Location` (uri + range) per symbol and a containerName
   convention for tasks.** Helix's `hierarchical_document_symbol_support: false` makes the flat
   path mandatory (b7:42); b1 D1 covers the branch, but the task-item flat representation
   (range = task line, containerName = heading text or parent task) is not pinned.
4. **selectionRange handler invariant needs a test pin.** The spec requires result `i` to contain
   `positions[i]`; the framework does not enforce it, and a bug produces client-side selection
   weirdness rather than an error (a2-crates.md:130-132). One property test over the shared walk
   belongs to the S1 implementation.
5. **One position-encoding conversion seam for all nine handlers.** Every range the structural
   walk produces is a `BytePos`; all must route through 26's convert module at the protocol
   boundary (26:35; a1:173-176: no `byte_to_utf16_cu` exists in the codebase today). The D-set
   never says "27 handlers share 26's conversion point"; without it, handlers will each
   hand-roll positions.
6. **documentSymbol silent truncation is a user-visible honesty question.** documentSymbol has no
   partialResult (consolidation section 0.12); at the 5k hard cap the outline is silently
   incomplete. `tracing::debug!` is in b1 D5, but a user-facing note (docs or a log at warn)
   is the honest version; flag for the grilling session.
7. **`semantic_tokens_full_delta` must be affirmatively not advertised**, and
   `semantic_tokens_range` likewise (D7 says full-only). Both need the negative-advertisement
   sentence (F11c); this is the overlooked *capability negotiation* item, distinct from the
   legend design.
8. **CodeLens "used-flag" semantics need a definition.** D9's refresh gate keys on a per-client
   used-flag (iwe precedent, b4) but never defines "used": client ever sent
   `textDocument/codeLens` this session, or ever resolved one? The first request is the only
   observable; state it, or the gate is unimplementable.
9. **Quickfix-lane code actions are conditional on 25, not owned by 27's four kinds.** b5 includes
   `quickfix` "conditional on 25's diagnostic-lane actions sharing the provider"; the distilled
   D10 lists four kinds and drops the conditional fifth. Either state "v1 = four kinds; quickfix
   joins when 25 lands its action set" or the provider's kind advertisement (which is static at
   initialize, per D16) will not match what it later returns.
10. **Frontmatter-fold line-0 scan and fence spans are parse-time, not AST-time.** a1:98-101 and
    a1:337-339 confirm code blocks store nothing and frontmatter keeps only a parsed map, so
    D2's E-lite folding must run on the raw `into_offset_iter()` walk (b2's shared walk), never
    on the `Note` AST. b2 says this; the D-set summary does not, and an implementer reading only
    D2 would reach for `Note` and find no fence spans.

## 4. Recommended amendments (per decision)

- **D1**: add root/preamble task attachment rule (F7), section-end corner as non-normative, and
  the perf re-anchor dependency on F1.
- **D2**: record the `is_foldable` mapping rule (foldable = not `+`; `-` initially-collapsed
  inexpressible on 0.23, dropped) and file the 17 amendment (F2); add the collapsedText
  typed-crate verification note (F9); state that folding runs on the raw offset-iter walk, not
  the `Note` AST (overlooked item 10).
- **D3**: no change beyond adopting the invariant test pin (overlooked item 4).
- **D4**: add the implementation-time verification that Occurrence spans overlap the VS Code
  built-in's hrefRange for standard links (F4b; b3:155 `_union` dedup).
- **D5**: pending grilling adjudication (Part X item 2); add the `off`-severity silent-hole
  sentence and the three-way affordance handoff regardless of outcome (F3).
- **D6**: obtain 26/15 sign-off (Part XI item 4); document the Neovim tooltip-invisible
  limitation (F5).
- **D7**: pin never-filter-client-tokenTypes, single-line non-overlap, and no-delta advertisement
  (F11).
- **D8**: restate refresh as a server->client request; route through the shared swallowed-error
  helper (owed to 25/29); gate on client `refreshSupport` (F4).
- **D9**: re-attach zero-reference suppression (decide consciously), stale-`data` version guard,
  and never-pre-serialized-`Location[]` (F10); define the used-flag observable (overlooked
  item 8).
- **D10**: add `disabledSupport` capability gate + Helix-absence note; state 27 owns CreateFile
  emission and the shared `create in resourceOperations` predicate; note 28 compatibility (F8);
  add the conditional quickfix-lane sentence (overlooked item 9).
- **D11**: add the one-sentence rejection of Marksman's VS Code identity-gating lever (F12).
- **D12**: add the overlay-freshness sentence (overlooked item 2) and the tags-asymmetry statement
  (overlooked item 1); keep the fairness question explicitly parked to 29.
- **D13**: replace the perf block per F1 (corrected rows, re-anchored doc-symbol strategy,
  33:117-derived workspace-symbol number); file to 33.
- **D14**: add the minijinja 3.0.1 vs `=2.24.0` risk note (F13b).
- **D15**: expand to the nine-ticket record set (F6); make the 11-record the explicit method-name
  correction (F14).
- **D16**: add runtime-disable semantics (empty-result vs method-not-found) and the 31
  live-reload interaction (F13); add the shared position-conversion seam sentence (overlooked
  item 5).

## Sources

In-repo: `research/27-structural-symbols-editor-intelligence-pass2.md` (Parts II-VIII, X, XI);
`research/27-pass2/` b1-symbols.md, b2-folding-selection.md:26,:72,:132,:134,:158,
b3-document-links.md:44-45,:155,:255-265, b4-codelens-inlay.md:259 and latency section,
b5-code-actions.md, b6-semantic-tokens.md, b7-clients-registration-coexistence.md:42-47,:58,:67,
:103,:109-111,:120,:131-133, a1-codebase.md:27-33,:98-101,:142-144,:173-176,:248-252,:285-286,
:303-306,:337-339,:427-433, a2-crates.md:96-98,:130-132,:166-167,:193,:300-318,:347;
issues 11, 15, 16, 17:74, 22:24,:49,:58,:65,:70, 23:46,:49-51,:50(c), 25:24,:40,:47,
26:27,:35,:37, 28:17, 29:24, 31:1-23, 32:8-16, 33:47-55,:117, 41:29;
`map.md`; Cargo.toml:131.
