# Stress test — Q3 resolve split & Q6 server ranking vs client re-sort

Stress test, pre-grilling, 2026-10-07. Adversarial pass over the two draft ticket-24 decisions
(Q3: resolve split / lazy documentation; Q6: server sorts by `sortText`, clients own relevance)
against the verified client reality in `24-zed-client-reality.md` and the ls-types/lsp-types source
on disk. Every finding below cites a unit `tag:line` or a file path and ends with a concrete change
that leans *strengthen*, not hedge.

---

## Q3 — resolve split / lazy `documentation`

### ST-R1 — HIGH — The proposed p95 ≤5 ms resolve budget is ~5× too loose for the client it is
justified by; a 20-item Zed burst burns 100 ms of the same sequential gate the keystroke
completion request needs.

- **Claim**: Zed fires resolve as *serial, detached, uncancellable* batches of ~20 items (one
  in flight at a time, await-then-next). Ticket 12 gives the server `concurrency_level(1)` — every
  queued resolve still *executes* and then gets discarded as stale (`issues/12:36`, `latency:74`).
  20 × 5 ms = 100 ms of pure head-of-line block on the typing path, against ticket 33's
  <20 ms completion row (`issues/33`). The draft's own justification ("Zed fires ~20-item serial
  resolve batches") is an argument for a *tighter* number, not the 5 ms it proposes.
- **Evidence**: `24-zed-client-reality.md` Q5 (serial detached batches, no cancellation);
  `24-completion-resolve-lazy-detail.md:163-168` (Helix `block_on`, nvim-cmp 80 ms — the
  *accept* path is blocking in two of five clients, so gate time is on the user's fingers either
  way); `24-completion-latency-architecture.md:74` ("a slow resolve delays the next" request).
- **Change**: p95 **≤1 ms**, stretch ≤0.5 ms (validate `data`, hash-lookup, clone, serialize one
  ~200 B item — all µs-class work; the 5 ms is slack, not need). Add a second budget row to ticket
  33: **burst of 20 resolves ≤20 ms total gate time**. Enforce a hard per-resolve deadline with
  fail-open (return item unchanged) rather than letting one bad resolve own the gate.

### ST-R1b — MED — D5's rationale ("clients serialise resolve and cancel the previous") is factually
false for Zed, the primary target — so the server needs the deadline/coalescing the rationale
disclaims.

- **Claim**: The draft defers the hard-timeout/deadounce machinery to "clients do it for us." VS
  Code does (cancel-on-refocus, `suggestWidget.ts:399-428`); Zed does **not** — no cancellation,
  overlapping detached batches, every queued item executes under stale-discard. Neovim debounces
  (`adaptive_debounce`) but only cancels *pending* requests, not in-flight execution.
- **Evidence**: `24-completion-resolve-lazy-detail.md:2.1` table (Zed row empty for
  cancel/debounce); `24-zed-client-reality.md` (resolve never cancelled).
- **Change**: state explicitly in D5 that server-side deadline + fail-open is **required** because
  the Zed cancel path does not exist; drop the "clients serialise and cancel" sentence.

### ST-R2 — HIGH — "Defer documentation or ship prose every keystroke" is a false dichotomy: with
`isIncomplete: false` the client filters locally with zero server traffic, and Zed *skips resolve
entirely* for items that already have documentation — so eager docs is a resolve-storm kill switch,
not a per-keystroke cost.

- **Claim**: Zed re-filters open sets client-side (no re-request, no serialization) and, on the
  select/focus path, `completionItem/resolve` is skipped for any item that already carries
  documentation (`info_complete` symmetry in Neovim core, `:618`). Deferring documentation is
  therefore the *only* thing that makes Zed fire the ~20-item serial burst (ST-R1) — shipping it
  eagerly collapses the burst to the single always-selected item. The cost side is mis-modelled:
  eager docs costs one payload per *list generation*, not per keystroke, for every closed set (C6
  schema keys, C9–C13 task/date/emoji/callout/footnote — all well under the 100 cap ⇒
  `isIncomplete: false`). Only the open/capped contexts (C1 wikilinks in a large vault, C4 tags,
  C17 file lists — `isIncomplete: true`) re-serialize per keystroke, and those are exactly the
  contexts where deferral is already in the per-context matrix.
- **Evidence**: `24-zed-client-reality.md:719-723` (docs-skip), `:726-736` (selected item always
  resolved); `24-completion-resolve-lazy-detail.md:2.2(4)` (Neovim `not info_complete`, VS Code
  pre-marked-resolved `suggest.ts:125-126`); `24-completion-architecture.md` §4.3 over-claims
  "withholding shrinks S8 for every keystroke" — only true under `isIncomplete: true`.
- **Change**: invert the default per-context rule from **"defer documentation"** to
  **"ship documentation eagerly up to a measured size budget N (prebuilt previews, short schema
  prose), defer only prose over N"** — chosen by (session openness × doc size), not
  compute-vs-payload. Keep `resolveProvider: true` (Zed/Neovim/Helix gate on *our* capability,
  `resolve:2.2(3)`). Record the measured S8 delta for both arms (S8 serialization remains
  unmeasured — §9.7) and pick N at the grilling, not by default.

### ST-R3 — MED — "Never a per-request scan at resolve time" (D5.3) collides with the unsaved-note
case: a freshly created note has no refresh-built preview artifact, so eager-docs-or-resolve both
fail-open and silently lose documentation on exactly the "create note" flow the design sells.

- **Claim**: Documentation previews are artifacts of `IndexerService::refresh`; a note typed but
  not yet refreshed (or a brand-new note from the create-note action) has none. Under D5.3's
  "resolve-time scan is a bug" rule, resolve must return the item unchanged ⇒ documentation blank
  on the newest, most-relevant item. The rule as written forbids the one legitimate bounded
  fallback: a **bounded overlay/ContentResolver lookup** (first-line slice of the live buffer,
  already resident per ticket 14) — cheap because the buffer is in memory and the slice is O(1),
  not a workspace scan. `v` is the schema version (1), not a vault/refresh version, so it cannot
  detect staleness of the overlay either — and C14 position staleness remains open (ContentModified
  vs version stamp, `resolve:2` spec box).
- **Evidence**: `24-completion-architecture.md` D5.3 + §5; `issues/14` (DocumentStore overlay is
  authoritative while open); `24-completion-resolve-lazy-detail.md` (fail-open semantics; `data`
  = `{"c":…,"v":…}`); `issues/28` create-note flow.
- **Change**: amend D5.3 to permit **one** resolve-time read — bounded, overlay-only
  (DocumentStore/ContentResolver, ≤ first N lines), never the workspace scan D5.3 actually means
  to forbid — and state the fallback explicitly: overlay miss ⇒ eager artifact ⇒ return unchanged
  (fail-open, in that order). Add the `v` staleness question (ContentModified vs version stamp) to
  ticket 29's open list rather than leaving it buried in a unit.

### ST-R4 — MED — Zed advertises `labelDetails` and never reads it — so 16's usage-stat badges and
19's `⟨global⟩`/`⟨inferred⟩` layer badges are invisible in a primary target; the design has no
mirror for them.

- **Claim**: 16 puts counts in `label_details.detail` (`issues/16:49`); 19 visualises layer via
  badges (decomposition §22 row / `issues/19:47`). Zed's capability advertises `labelDetails` but
  the renderer never consumes it ⇒ in Zed the counts/badges simply do not render. The draft's
  "resolve split" is silent on where a *visible* mirror lives.
- **Evidence**: `24-zed-client-reality.md` (labelDetails never read, capability-only);
  `zk-src-digest.txt:4864-4868` (zk mirrors counts into top-level `detail`, ~15 B/item, and every
  surveyed client renders `detail`).
- **Change**: state the mirror rule in the item-shape handoff to 16/19: **counts/layer badges go in
  `detail` (zk-style, ≤~15 B/item) when they must be visible; `labelDetails` is an optional
  enhancement for clients that render it.** This is a §4.3/16/19 amendment, not a Q3 rewrite.

### ST-R5 — LOW (strengthen) — The hybrid in ST-R2 also fixes the D3.3 sparse-client floor the draft
leaves open.

- **Claim**: A client that advertises no `resolveSupport.properties` may not legally receive
  deferred fields; the draft's answer is "still defer documentation" — which means a sparse client
  never shows documentation at all. Eager-until-N shipping means the sparse floor gets the same
  short docs the rich clients get, for free.
- **Evidence**: `24-completion-capability-negotiation.md` D3.3; `29-client-capability-negotiation.md`
  (gating ownership); spec box `[S1]:3-4,119-125` (withholding requires `resolveSupport`).
- **Change**: fold into the ST-R2 rule; no separate mechanism.

### Item-defaults note (evidence for the draft's "ls-types blocks it" line)

`ls-types` 0.0.6 `CompletionList` = `is_incomplete` + `items` only
(`.rust-docs/cache/crates/ls-types/0.0.6/source/src/completion.rs:414-421`); client-side
`CompletionListCapability.item_defaults` exists (`:200-211`). **lsp-types 0.97.0 has the same
omission** — shared gap, not fork-specific. The field is `data: Option<Value>`-ready
(`:543`) so nothing else blocks the *payload*; only the `CompletionList` struct needs the field.
Tower-lsp-server 0.23.0's `proposed` feature is dangling against ls-types ≥0.0.5 (no `[features]`
section in 0.0.6; 0.0.2 had `proposed = []`), so "enable via feature flag" is not currently a
path either. → record the workaround as an **upstream PR to tower-lsp-community/ls-types** (same
org as the chosen framework, trivial camelCase field) rather than treating it as a permanent wall.
Low severity for now (sub-ms byte savings), but the framing should not be "crate blocks it, period."

---

## Q6 — server ranking vs client re-sort

### ST-K1 — HIGH — Q6's "accept display-order" pre-decides item-shape unit Q1 (file-field relevance
order) in the permissive direction, contradicting three already-resolved tickets that name a server
ordering, and the compensating levers are neither named nor degradation-noted.

- **Claim**: 19 locks "sortText prefixed by layer number for client-side ordering"
  (`issues/19:48`); 20 locks "sort by relevance (recently opened → same folder → alphabetical)"
  (`issues/20:37`); the decomposition owes 22 "local-before-global ordering"
  (`24-part-decomposition.md:17`). Q6 says display order is cosmetic, group labels are cosmetic,
  and only empty-input ordering must be meaningful via `sortText` alone — which makes all three
  of those resolutions partially dead letters. The levers that *do* exist are not stated:
  `preselect` on server rank-0 (works VS Code + Neovim, **absent in Zed/Helix** per
  `24-zed-client-reality.md`), and exact-`filterText` rung-1 match (`sort_exact` sits *above*
  score in Zed's rungs — C17/C6 `filterText` = natural typed form buys absolute first place for
  the exact-prefix case).
- **Evidence**: as cited above; plus `issues/20:37`'s recency class has **no funded data source** —
  "recently opened" is client MRU the server can only partially emulate (didOpen session-scoped,
  cross-session needs a table 13 forbids), so the relevance class itself is under-specified
  independent of Q6.
- **Change**: in Q6, name the three levers explicitly (`sortText` = empty-input order only;
  `preselect` = rank-0 where `preselectSupport`; exact `filterText` = absolute rung-1), and add a
  one-line **degradation note handed to 19/20/22**: "layer prefixes / relevance / local-before-global
  order are display-only in Zed except at empty input; if absolute order matters, encode it in
  `filterText` or `preselect`, or revisit." Also send 20's recency class back for a data-source
  decision (didOpen-only MRU vs drop) before locking.

### ST-K2 — MED — "True for C1–C8" is fragile: Zed's OtherMatch demotion voids sortText authority
for any non-word-initial query, which is the majority of real keystrokes.

- **Claim**: Zed demotes items whose match lands outside word initials (OtherMatch rung, below
  word-initial runs but above the sortText rung) — e.g. typing "roj" against `filterText`
  "projects…/…": no word starts with `r`, so the item is demoted *regardless of sortText*. The
  §3.1 invariant "every context meaningfully ordered by sortText alone at empty input" is the only
  order guarantee that survives; the stronger informal reading ("sortText orders the list") does
  not, and the draft does not say so.
- **Evidence**: `24-zed-client-reality.md` (project.rs:7071-7111 rungs; OtherMatch demotion);
  `24-completion-item-shape-and-ranking.md` §3.1–3.2.
- **Change**: harden the invariant to an explicit **filterText invariant**: the natural typed form
  of every context (C1 target text, C6 field key, C17 path, C4 tag path) must appear word-initial
  in `filterText` so the query stays in the word-initial rungs; add one test per context to S-1..S-15.
  State plainly that group order is absolute *only* at empty input, elsewhere it is best-effort.

### ST-K3 — LOW (blunt) — Sort-text gap encoding is an illusory lever; say so and stop pretending.

- **Claim**: Both verified rungs (VS Code, Zed) sit the client fuzzy score *strictly above* sortText
  for any non-empty query, so widening rank gaps in sortText changes nothing while the user is
  typing — only empty-input order. The design's implied "encode priority in sortText gaps" is real
  for C-N13-style "only order guarantee" and dead for query-time reordering.
- **Evidence**: `24-completion-capability-negotiation.md` C-N13 ("sortText is the only order
  guarantee"); `24-zed-client-reality.md` rung order (score above sortText).
- **Change**: one sentence in Q6: gap encoding is for empty-input/initial-population order only.

### ST-K4 — MED — Q6's core ordering claim is verified only for VS Code + Zed; Helix and
Neovim-core/nvim-cmp ordering remain unchecked (item-shape Q7), and Neovim is first-class per
`map.md:44`.

- **Claim**: item-shape Q7 is still open (`24-completion-item-shape-and-ranking.md:854`) and there
  is **no Helix digest at all** (`docs/digests/` has none), yet Q6 generalises "client sorts, we
  supply sortText" across all five clients. If Neovim core (or nvim-cmp's own `sorting.priority`
  interplay with sortText) ignores sortText at empty input, the "every context meaningfully
  ordered at empty input" contract is quietly VS-Code/Zed-only.
- **Evidence**: `24-completion-item-shape-and-ranking.md:854` (Q7 open); `map.md:44` (Neovim
  first-class, Helix = sparse floor).
- **Change**: elevate Q7 to **required before locking Q6**; add Helix ordering (and Neovim-core
  completion.lua ordering) to the same research sweep; until then Q6 is proposed, not accepted.

### ST-K5 — LOW — S-4's stated tiebreak "sortText, then label, then kind" matches VS Code but
contradicts Zed's verified rung order (kind before label); harmless only because sortText is unique
per item (group+rank+label).

- **Evidence**: `24-completion-item-shape-and-ranking.md` S-4; `24-zed-client-reality.md` rungs.
- **Change**: rewrite S-4's tiebreak as "sortText is unique per item by construction (group+rank+label);
  any further tiebreak is unspecified client behaviour" — the current wording invites an
  implementation that encodes priority in kind, which both clients demote below score.

### ST-K6 — LOW — 23's "`preselect` top" silently no-ops in Zed (preselectSupport absent, never
read); the `- [` flow still lands correctly because Zed's empty-query order follows our sortText
and the list is short/closed — record the degradation, don't discover it in implementation.

- **Evidence**: `24-zed-client-reality.md` (preselect absent); `issues/23:47/50` (sort_text =
  config insertion order, preselect top, `Invoked` fallback).
- **Change**: one-line note in the 23 handoff: "`preselect` is honoured by VS Code/Neovim;
  Zed/Helix order by sortText at empty input — config order *is* the visible order there."

---

## Verdict per decision

- **Q3 (resolve split): STRENGTHEN, not weaken.** Keep `resolveProvider: true`, keep the
  stateless-`data` key, keep never-deferring immutable fields and `detail`. Fix three things: (1)
  the resolve budget p95 ≤5 ms → **≤1 ms** with a 20-item burst ≤20 ms gate-time row in ticket 33
  and a hard fail-open deadline (ST-R1/R1b); (2) invert the documentation rule from blanket-defer
  to **size-gated eager-up-to-N, defer only long prose**, chosen per context by session openness ×
  doc size, with N set by a measured S8 arm (ST-R2/R5); (3) permit one bounded overlay-only
  resolve-time read so the create-note flow keeps docs (ST-R3). Mirror counts/badges into `detail`
  for Zed (ST-R4). Item-defaults: upstream `ls-types` PR, not a wall.
- **Q6 (server ranking vs client re-sort): KEEP the display-order acceptance, but it is not yet
  lockable.** The acceptance itself survives the stress (verified for VS Code + Zed; empty-input
  `sortText` order is real and is the contract worth keeping). Before locking: name the three
  compensating levers and their Zed degradations (ST-K1), harden the filterText invariant against
  OtherMatch demotion (ST-K2), say gap-encoding is empty-input-only (ST-K3), and **close the
  Helix/Neovim-core/nvim-cmp ordering gap (Q7)** or narrow Q6's claim to VS Code + Zed until it is
  closed (ST-K4). Hand 19/20/22 the degradation notes and send 20's "recently opened" class back
  for a data-source decision.
