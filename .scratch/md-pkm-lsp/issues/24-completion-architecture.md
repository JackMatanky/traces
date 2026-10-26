# Completion architecture: context detection & dispatch

Type: grilling
Blocked by: 15, 16, 19, 21, 22, 23
Status: resolved

## Question

Once the per-feature completion decisions (tickets 15 links, 16 tags, 19
frontmatter/inline-fields, 21 query, 22 template) are each settled, decide the
*unifying* completion architecture:

- Context-detection strategy: given a cursor position, how does the LSP decide
  which of these contexts applies: "inside a wikilink target", "inside a tag",
  "inside frontmatter YAML", "inside an inline field value", "inside a query-DSL
  string inside a template call", "inside a template helper-namespace call".
  Choose between a single dispatcher walking outward from cursor position
  through (span-aware, per ticket 11) AST context, or per-context independent
  trigger-character registration (`completionProvider.triggerCharacters`), or
  both layered.
- Trigger-character set: consult `docs/refs/lsp_spec.md`'s Completion Request
  section for how `triggerCharacters`/`triggerKind` work. Enumerate the full set
  Traces needs (`[`, `#`, `:`, `.`, etc.). Check for collisions with rumdl's own
  registered link-completion trigger characters (`(`, `#`, `/`, `.`, `-` per the
  rumdl research, ticket 03). Decide whether Traces needs to coordinate/document
  which characters it expects rumdl's `enableLinkCompletions` to be turned off
  for.
- `CompletionItem` resolution strategy: cheap initial list
  (`textDocument/completion`) vs expensive detail filled lazily
  (`completionItem/resolve`). This is ergonomically relevant for showing full
  Schema Field Definition docs only on resolve, not in the initial list.
- Snippet support (`insertTextFormat: Snippet`) for structured insertions (e.g.
  completing a wikilink with the cursor left inside `[[|]]`).

## Cleanup at resolution

Research layout (decided 2026-10-07): the consolidated research file is
[`research/24-completion-architecture.md`](../research/24-completion-architecture.md)
(repo convention: one research file per ticket); the 12 evidence files it cites
by `tag:line` lived in `research/units/`. Merging those 12 into fewer files
would invalidate ~216 line-relative citation edges (130 in the consolidation, 85
unit-to-unit, plus audits), so **merging is deferred**: at resolution, delete
(or archive) `research/units/` and keep the consolidation as the single
permanent research record, the same shape as tickets 15 to 23.

> **Executed 2026-10-07**: `research/units/` deleted after the resolution
  commit; evidence is > recoverable from git history (`61cb98ab`, `428afe65` +
  cleanup commit). The consolidation's
> shorthand table now names the files without links.

## Answer

Resolved 2026-10-07 via one grilling round over six decisions, after four
adversarial stress-test passes and one client-ordering check sweep. All findings
are dispositioned in [research §11](../research/24-completion-architecture.md)
(the authoritative record; §11.2 supersedes §0).

1. **Context detection**: layered hybrid dispatcher. L1 region, then L2 line
   predicates, then L3 first-match to exactly one completer (`None` returns
   `[]`). Protocol hints never classify: that is a contract line, not a layer.
   The return contract is `ContextKind` + replace range + filter word. One
   content snapshot per `(uri, version)` is shared by
   completion/hover/diagnostics (contract 15). Bare-`[` seeds `[]` with
   `isIncomplete: true`. Inline-field completion needs the following key-char or
   `::` shape. `- [` + a non-checkbox char declines the checkbox layer (this
   answers 23's owed ambiguity). Body `:` completes only in the inline-field
   bracket shape or whole-line bare fields (strict-19); all other prose colons
   decline on a ≤0.3 ms miss path. C11 claimed: daily-note date completion
   inside `](`, with a date-shaped guard, 18's ±7 set, and word-char/`Invoked`
   entry. Callout types offer at `[` (the `[!?` predicate). The code veto is
   fence-language-aware from day one (the 21-Phase-2 `traces-query` seam).
2. **Trigger characters**: hardcoded `[ # : @ . ^ > /` at `initialize`; zero
   config/phase machinery (22 and 17 resolved and filed their triggers; `(`/`"`
   join by code change when 21-Phase-2 lands). `/` is guarded to `[[`-interior
   (no path-segment menu flicker in Zed). Degenerate-path contract: every
   request (registered char, rumdl-union fan-out, or word char) misses in
   O(line), ≤0.3 ms, and never past S2. Non-empty responses come only inside
   classified contexts. A per-trigger histogram row is owed to 33. `|` is never
   registered; alias candidates ride the `[[target` response (capped, sortText
   group after the target). `isIncomplete` is claimed on Traces' side only; 32
   documents the rumdl-side ask. Invoked-first contexts are enumerated (C6
   frontmatter keys, C16 include/extends names, template roots) for 29's
   sparse-client floor. v1 query-context entry = `Invoked` + `#`/`@`-inside-`{{
   }}`.
3. **Resolve split**: `resolveProvider: true`. `detail` is always initial.
   Immutable fields are never deferred. Documentation is **eager up to measured
   size N, and deferred only above N** (by session openness × doc size; eager
   short docs collapse Zed's serial resolve bursts to the selected item). The
   `data` key is stateless. A server-side deadline + fail-open is needed (Zed
   cancels nothing). One bounded overlay-only resolve-time read is legal (the
   create-note flow keeps docs). Budgets owed to 33: resolve p95 ≤1 ms, 20-item
   burst ≤20 ms gate-time; the D7 duration histogram is the launch gate.
   Counts/badges mirror into `detail` (Zed never renders `labelDetails`).
   `itemDefaults` is an upstream ls-types PR path, not a permanent wall. iwe is
   the shipped counterpoint that validates the split as deliberate: eager
   completion with identity-resolve is a shipped shape, not an oversight
   (amended 2026-10-21 by iwe/fileclass research,
   [tool-iwe](../research/tool-iwe.md) Part 6).
4. **Wikilink insert**: the zero-snippets draft was refuted and flipped to
   option (A). `snippetSupport` is confirmed true in all five primary clients,
   no-auto-pair is stock Neovim's default, and 15 names Neovim the primary PKM
   demographic. The range covers an existing trailing `]]`. Snippet
   `[[T|${1:}]]$0` is gated on `snippetSupport`. Gated-off clients get a
   state-aware plain track (paired state gives `[[T|`, with the alias slot
   pre-opened; fresh state gives `[[T]]`). `[[target|alias]]` with the cursor in
   the target declines (the pipe guard prevents broken Markdown). S-18 stays as
   the recorded rule for any future snippet re-adoption. The bracket auto-pair
   check across the four primaries is a pre-implementation acceptance gate.
5. **Frontmatter `:`**: keys before (19's three layers, guarded against comment
   and block-scalar/continuation interiors via L1 span marks). Values after are
   a **region with owners by key shape**: tag-list keys go to 16 (C5),
   schema-bound keys go to 20 (C7/C17), `[[` inside frontmatter values goes to
   C1's completer with a frontmatter-local range (a new rung), and everything
   else declines. After-key is a sustained position property; the classifier
   emits `AfterKey { key }`. Frontmatter wins over `{{ }}` depth. Unbound-note
   value positions decline in v1; C7/C17 ship empty until 20's completion lands
   (dispatch returns empty, not an error).
6. **Ranking**: accept client re-sort, bound to three counterweights. Our order
   is absolute at empty input via `sortText` **and response-array order** (Helix
   never reads sortText). `preselect` applies where honored (VS Code yes; Neovim
   0.13+ with `completeopt+=preselect`, no-op on 0.12; Zed/Helix never). The
   **filterText word-initial builder invariant**, enforced at the item builder
   with one test per context, is the sole ranking counterweight in Zed
   (visibility-critical in Neovim's startswith matcher). nvim-cmp is carved out:
   our order is never absolute there (source priority/recently-used dominate);
   that is accepted. sortText gap-encoding is empty-input-only.

Handoffs to 15/16/18/19/20/23/29/31/32/33/34 and the ls-types upstream PR are
enumerated in research §11.3; pre-implementation check gates are in §11.4. The
evidence layer is the 17 files under `research/units/` (12 research + 4 stress
passes + 1 ordering sweep); they are deleted at resolution per the cleanup plan
above and are recoverable from git history. The consolidation is the permanent
record, the same shape as tickets 15 to 23.
