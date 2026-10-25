# rumdl coexistence: explicit capability-ownership boundary

Type: grilling
Status: open
Blocked by: 03, 25, 27
Assigned: agent

## Question

Synthesize the final coexistence contract once diagnostics (25) and structural/document-link capabilities (27) are settled, grounded in the rumdl research (ticket 03) — which already confirms rumdl ships `enableLinkCompletions`/`enableLinkNavigation` settings specifically intended to cede link intelligence to "another language server (for example a PKM/notes LSP)".

Decide and document as the shipped product contract:

- The complete ownership table: linting/formatting/fence-language-completion → rumdl; PKM link/wikilink/tag/schema/query/template intelligence → Traces; generic structural intelligence (folding, symbols, selection ranges per ticket 27) → whichever server's research (tickets 03, 05) shows is the stronger/more complete implementation, or split by sub-capability if warranted.
- The concrete recommended client configuration: both servers registered for the `markdown` language, with rumdl's `enableLinkCompletions`/`enableLinkNavigation` set to `false` documented as a required companion setting for a clean joint setup — decide whether Traces should detect rumdl's presence and warn/guide the user if these aren't disabled (possible, but check what's actually detectable from inside an LSP server — likely nothing, since servers don't see each other; more likely this becomes user-facing setup documentation rather than runtime detection).
- Diagnostic-code namespacing confirmed non-colliding (cross-check ticket 25's decision against rumdl's `MDxxx`/`inline-config` codes).
- Formatting ownership: confirm Traces registers zero `documentFormattingProvider`/`documentRangeFormattingProvider` capability at all, ceding 100% of formatting to rumdl, per the product goal's clean split.

## Inputs from 25 (2026-10-15)

- **Post-cession ownership table (25 Q8-R/Q9-R/Q10-R/Q13-R2)**: rumdl owns structure/style rules, standard-link file existence **including image targets** (MD057), same-document `#frag` anchor validation (MD051), footnote undefined/unused/duplicate (MD066), heading-level lint (MD001/MD041), and heading symbols (`enableSymbols` cession). Traces owns wikilinks (resolution, diagnostics, completion, navigation), **cross-file `file.md#heading` anchors** (unowned by rumdl — probed; claimed by 25), footnote self-reference + blockquoted-definition (unruled by MD066), frontmatter parse diagnostics (`frontmatter.missing-delimiter`, Error — 19, with Q13-R2 coexistence note: MD041 Warning always fires, MD065 Warning fires on the missing-closing shape only, suppressible only by disabling the rule), and all `schema.*`/`query.*`/`template.*`/`tasks.*` codes.
- **Namespacing confirmed (25 Q6-R)**: Scheme C — one `source: "traces"` + hierarchical `<feature>.<rule>` codes + pull `identifier: "traces"`; rumdl emits `source: "rumdl"` + `MDxxx`/`inline-config` codes. Zero collision by construction (distinct `source` + disjoint code vocabularies); `traces-tasks` retired.
- **Hint asymmetry line**: 23's A1 is Hint; rumdl has no Hint level. The joint-setup docs must not promise severity symmetry across the two servers.
- **Transport attribution facts**: Traces' push/pull gate is capability-driven (25 Q1-R/Q2-R, detection seam owed to 29); rumdl's push/pull branch is its own implementation — no cross-server coordination beyond the documented `enableLink*` companion settings (15's joint-setup block, amended by 25 to make it the primary config path).

## Inputs from 26 (2026-10-18)

- **Standard-link rename boundary**: 26 rewrites url/fragment only at reference and definition sites; display-text positions **refuse**. Confirm this boundary against rumdl's ownership here.
- **Heading rename rewrites same-document standard-link fragments `(#h)`** as well as same-file wikilink anchors and cross-file `file.md#h`: Traces rewrites text in rumdl's validation domain (MD051) without claiming validation (the 26<->32 line).
- **Comparison-key scope**: `match_key` (NFC + casefold ladder, byte-exact-first) covers **wikilink path/stem matching only**; standard-link fragments and slug generation stay byte-exact (GitHub/MD051 contract).
- **Live divergences to document in the coexistence design**: Obsidian's filename/stem normalization and NFD heading anchors. NFKC is rejected everywhere (rationale corrected: 15 revoked title matching, so the Obsidian-title argument was stale).

## Inputs from 27 (2026-10-22)

- VS Code built-in duplication posture (built-in markdown is a full LSP with no per-feature disable for the symbol providers and no global kill switch): accept duplicates as default (markdown-oxide practice: install and say nothing; Traces documents), document `outline.show*` per-kind filters honestly (cannot split two `String` heading sources, state the caveat), provide Traces-side per-feature config gates shaped like rumdl's `enableSymbols` for proven-duplicative capabilities, document whole-extension disable as the heavy option only, companion guidance: set rumdl `enableSymbols = false` when Traces provides the outline. Rejected levers: SymbolKind dodge (zero precedent, breaks kind pins), Marksman-style VS Code identity-gating auto-cession (deletes Traces' task outline exactly where users see it). Empirical overlap check for folding/highlights deferred to implementation verification ([27 pass 2, Part XI open question 7](../research/27-structural-symbols-editor-intelligence-pass2.md)).
