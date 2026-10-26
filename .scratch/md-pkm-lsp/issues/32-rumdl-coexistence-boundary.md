# rumdl coexistence: explicit capability-ownership boundary

Type: grilling
Status: open
Blocked by: 03, 25, 27
Assigned: agent

## Question

Synthesize the final coexistence contract once diagnostics (25) and structural/document-link capabilities (27) are settled. Ground it in the rumdl research (ticket 03). That research already confirms rumdl ships `enableLinkCompletions`/`enableLinkNavigation` settings. rumdl intends those settings to cede link intelligence to "another language server (for example a PKM/notes LSP)".

Decide and document the shipped product contract:

- Build the complete ownership table. rumdl owns linting, formatting, and fence-language completion. Traces owns PKM link, wikilink, tag, schema, query, and template intelligence. Generic structural intelligence (folding, symbols, selection ranges per ticket 27) goes to whichever server's research (tickets 03, 05) shows is the stronger or more complete implementation, or split by sub-capability if warranted.
- Write the concrete recommended client configuration. Both servers register for the `markdown` language. Document rumdl's `enableLinkCompletions`/`enableLinkNavigation` set to `false` as a companion setting that a clean joint setup needs. Decide whether Traces detects rumdl's presence and warns or guides the user when those settings are not disabled. Detection may be impossible: check what an LSP server can actually detect. Servers likely cannot see each other, so this probably becomes user-facing setup documentation instead of runtime detection.
- Confirm that diagnostic-code namespacing does not collide. Cross-check ticket 25's decision against rumdl's `MDxxx`/`inline-config` codes.
- Confirm formatting ownership: Traces registers zero `documentFormattingProvider`/`documentRangeFormattingProvider` capability. That cedes 100% of formatting to rumdl, per the product goal's clean split.

## Inputs from 25 (2026-10-15)

- **Post-cession ownership table (25 Q8-R/Q9-R/Q10-R/Q13-R2)**: rumdl owns structure/style rules, standard-link file existence **including image targets** (MD057), same-document `#frag` anchor validation (MD051), footnote undefined/unused/duplicate (MD066), heading-level lint (MD001/MD041), and heading symbols (`enableSymbols` cession). Traces owns wikilinks (resolution, diagnostics, completion, navigation), **cross-file `file.md#heading` anchors** (rumdl does not own them: probed, claimed by 25), footnote self-reference + blockquoted-definition (unruled by MD066), frontmatter parse diagnostics (`frontmatter.missing-delimiter`, Error, owned by 19, with the Q13-R2 coexistence note: MD041 Warning always fires; MD065 Warning fires on the missing-closing shape only; disabling the rule suppresses it), and all `schema.*`/`query.*`/`template.*`/`tasks.*` codes.
- **Namespacing confirmed (25 Q6-R)**: Scheme C: one `source: "traces"`, hierarchical `<feature>.<rule>` codes, and pull `identifier: "traces"`. rumdl emits `source: "rumdl"` plus `MDxxx`/`inline-config` codes. The two vocabularies are disjoint, so there is zero collision by construction. `traces-tasks` is retired.
- **Hint asymmetry line**: 23's A1 is Hint; rumdl has no Hint level. The joint-setup docs must not promise severity symmetry across the two servers.
- **Transport attribution facts**: Traces' push/pull gate is capability-driven (25 Q1-R/Q2-R; the detection seam is owed to 29). rumdl's push/pull branch is its own implementation. There is no cross-server coordination beyond the documented `enableLink*` companion settings (15's joint-setup block, amended by 25 to make it the primary config path).

## Inputs from 26 (2026-10-18)

- **Standard-link rename boundary**: 26 rewrites url/fragment only at reference and definition sites; display-text positions **refuse**. Confirm this boundary against rumdl's ownership.
- **Heading rename rewrites same-document standard-link fragments `(#h)`** as well as same-file wikilink anchors and cross-file `file.md#h`. Traces rewrites text in rumdl's validation domain (MD051) without claiming validation (the 26<->32 line).
- **Comparison-key scope**: `match_key` (NFC + casefold ladder, byte-exact-first) covers **wikilink path/stem matching only**. Standard-link fragments and slug generation stay byte-exact (GitHub/MD051 contract).
- **Live divergences to document in the coexistence design**: Obsidian's filename/stem normalization, and NFD heading anchors. NFKC is rejected everywhere (rationale corrected: 15 revoked title matching, so the Obsidian-title argument was stale).

## Inputs from 27 (2026-10-22)

- VS Code built-in duplication posture: built-in markdown is a full LSP with no per-feature disable for the symbol providers and no global kill switch. Accept duplicates as default (markdown-oxide practice: install and say nothing; Traces documents). Document `outline.show*` per-kind filters honestly (cannot split two `String` heading sources; state the caveat). Provide Traces-side per-feature config gates shaped like rumdl's `enableSymbols` for proven-duplicative capabilities. Document whole-extension disable as the heavy option only. Companion guidance: set rumdl `enableSymbols = false` when Traces provides the outline.
- Rejected levers: SymbolKind dodge (zero precedent, breaks kind pins), and Marksman-style VS Code identity-gating auto-cession (deletes Traces' task outline exactly where users see it).
- The empirical overlap check for folding/highlights is deferred to a check during implementation ([27 pass 2, Part XI open question 7](../research/27-structural-symbols-editor-intelligence-pass2.md)).
