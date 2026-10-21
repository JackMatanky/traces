# Completion architecture: context detection & dispatch

Type: grilling
Blocked by: 15, 16, 19, 21, 22, 23
Status: resolved

## Question

Once the per-feature completion decisions (tickets 15 links, 16 tags, 19 frontmatter/inline-fields, 21 query, 22 template) are each settled, decide the *unifying* completion architecture:

- Context-detection strategy: given a cursor position, how does the LSP decide which of "inside a wikilink target", "inside a tag", "inside frontmatter YAML", "inside an inline field value", "inside a query-DSL string inside a template call", "inside a template helper-namespace call" applies — a single dispatcher walking outward from cursor position through (span-aware, per ticket 11) AST context, or per-context independent trigger-character registration (LSP `completionProvider.triggerCharacters`), or both layered.
- Trigger-character set: consult `docs/refs/lsp_spec.md`'s Completion Request section for how `triggerCharacters`/`triggerKind` work, and enumerate the full set Traces needs (`[`, `#`, `:`, `.`, etc.) checking for collisions with rumdl's own registered link-completion trigger characters (`(`, `#`, `/`, `.`, `-` per the rumdl research, ticket 03) — decide whether Traces needs to coordinate/document which characters it expects rumdl's `enableLinkCompletions` to be turned off for.
- `CompletionItem` resolution strategy: cheap initial list (`textDocument/completion`) vs expensive detail filled lazily (`completionItem/resolve`) —ergonomically relevant for e.g. showing full Schema Field Definition docs only on resolve, not in the initial list.
- Snippet support (`insertTextFormat: Snippet`) for structured insertions (e.g. completing a wikilink with cursor left inside `[[|]]`).

## Cleanup at resolution

Research layout (decided 2026-10-07): the consolidated research file is
[`research/24-completion-architecture.md`](../research/24-completion-architecture.md) (repo
convention: one research file per ticket); the 12 evidence files it cites by `tag:line` lived in
`research/units/`. Merging those 12 into fewer files would invalidate ~216
line-relative citation edges (130 in the consolidation, 85 unit↔unit, plus audits), so **merging is
deferred**: at resolution, delete (or archive) `research/units/` and keep the consolidation as the
single permanent research record — the same shape as tickets 15–23.

> **Executed 2026-10-07**: `research/units/` deleted after the resolution commit; evidence is
> recoverable from git history (`61cb98ab`, `428afe65` + cleanup commit). The consolidation's
> shorthand table now names the files without links.

## Answer

Resolved 2026-10-07 via one grilling round over six decisions, after four adversarial stress-test
passes and one client-ordering verification sweep — all findings dispositioned in
[research §11](../research/24-completion-architecture.md) (the authoritative record; §11.2
supersedes §0).

1. **Context detection** — layered hybrid dispatcher: L1 region → L2 line predicates → L3
   first-match to exactly one completer (`None` ⇒ `[]`); protocol hints never classify (contract
   line, not a layer). Return contract = `ContextKind` + replace range + filter word. One content
   snapshot per `(uri, version)` shared by completion/hover/diagnostics (contract 15). Bare-`[`
   seeds `[]` with `isIncomplete: true`; inline-field completion requires following key-char or
   `::` shape; `- [` + non-checkbox char declines the checkbox layer (answers 23's owed
   ambiguity). Body `:` completes only in inline-field bracket shape or whole-line bare fields
   (strict-19); all other prose colons decline on a ≤0.3 ms miss path. C11 claimed: daily-note
   date completion inside `](`, date-shaped guard, 18's ±7 set, word-char/`Invoked` entry.
   Callout types offer at `[` (`[!?` predicate). Code veto fence-language-aware from day one
   (21-Phase-2 `traces-query` seam).
2. **Trigger characters** — hardcoded `[ # : @ . ^ > /` at `initialize`; zero config/phase
   machinery (22 and 17 resolved and filed their triggers; `(`/`"` join by code change when
   21-Phase-2 lands). `/` guarded to `[[`-interior (no path-segment menu flicker in Zed).
   Degenerate-path contract: every request — registered char, rumdl-union fan-out, or word char —
   misses in O(line), ≤0.3 ms, never past S2; non-empty responses only inside classified
   contexts; per-trigger histogram row owed to 33. `|` never registered; alias candidates ride
   the `[[target` response (capped, sortText group after the target). `isIncomplete` claimed on
   Traces' side only; 32 documents the rumdl-side ask. Invoked-first contexts enumerated (C6
   frontmatter keys, C16 include/extends names, template roots) for 29's sparse-client floor.
   v1 query-context entry = `Invoked` + `#`/`@`-inside-`{{ }}`.
3. **Resolve split** — `resolveProvider: true`; `detail` always initial; immutable fields never
   deferred; documentation **eager up to measured size N, deferred only above N** (per session
   openness × doc size — eager short docs collapse Zed's serial resolve bursts to the selected
   item); stateless `data` key; server-side deadline + fail-open required (Zed cancels nothing);
   one bounded overlay-only resolve-time read legal (create-note flow keeps docs). Budgets owed
   to 33: resolve p95 ≤1 ms, 20-item burst ≤20 ms gate-time; D7 duration histogram = launch
   gate. Counts/badges mirror into `detail` (Zed never renders `labelDetails`). `itemDefaults` =
   upstream ls-types PR path, not a permanent wall. iwe is the shipped counterpoint that validates
   the split as deliberate: eager completion with identity-resolve is a shipped shape, not an
   oversight (amended 2026-10-21 by iwe/fileclass research, [tool-iwe](../research/tool-iwe.md) Part 6).
4. **Wikilink insert** — the zero-snippets draft was refuted and flipped to option (A):
   `snippetSupport` is verified true in all five primary clients, no-auto-pair is stock Neovim's
   default, and 15 names Neovim the primary PKM demographic. Decision: range covers an existing
   trailing `]]`; snippet `[[T|${1:}]]$0` gated on `snippetSupport`; state-aware plain track for
   gated-off clients (paired state → `[[T|`, alias slot pre-opened; fresh state → `[[T]]`).
   `[[target|alias]]` with cursor in the target declines (pipe guard — prevents broken Markdown).
   S-18 retained as the recorded rule for any future snippet re-adoption. Bracket auto-pair
   check across the four primaries = pre-implementation acceptance gate.
5. **Frontmatter `:`** — keys before (19's three layers, guarded against comment and
   block-scalar/continuation interiors via L1 span marks); values after are a **region with
   owners by key shape**: tag-list keys → 16 (C5), schema-bound keys → 20 (C7/C17), `[[` inside
   frontmatter values → C1's completer with frontmatter-local range (new rung), else decline.
   After-key is a sustained position property; the classifier emits `AfterKey { key }`. Frontmatter
   wins over `{{ }}` depth. Unbound-note value positions decline in v1; C7/C17 ship empty until
   20's completion lands (dispatch returns empty, not erroring).
6. **Ranking** — accept client re-sort, bound to three counterweights: our order absolute at
   empty input via `sortText` **and response-array order** (Helix never reads sortText); `preselect`
   where honored (VS Code yes; Neovim 0.13+ with `completeopt+=preselect`, no-op on 0.12;
   Zed/Helix never); **filterText word-initial builder invariant**, enforced at the item builder
   with one test per context (the sole ranking counterweight in Zed; visibility-critical in
   Neovim's startswith matcher). nvim-cmp carved out — our order is never absolute there
   (source priority/recently-used dominate); accepted. sortText gap-encoding is empty-input-only.

Handoffs to 15/16/18/19/20/23/29/31/32/33/34 and the ls-types upstream PR are enumerated in
research §11.3; pre-implementation verification gates in §11.4. Evidence layer: the 17 files under
`research/units/` (12 research + 4 stress passes + 1 ordering sweep) are deleted at resolution
per the cleanup plan above and recoverable from git history; the consolidation is the permanent
record — same shape as tickets 15–23.
