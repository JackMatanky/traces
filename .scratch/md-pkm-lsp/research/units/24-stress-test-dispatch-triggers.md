# Stress test: ticket-24 draft recommendations — Q1 (context dispatch) & Q2 (v1 trigger set)

What's tested: the draft Q1 (variant-(c) layered hybrid classifier) and Q2 (`[ # : @ . ^ >` static
flat trigger set, `(`/`"` deferred, `>`/`^` in, `.` phase-gated) against every resolved ticket Answer
(15/16/17/19/20/21/22/23), open tickets 32/33, and the C1–C17 context inventory. Date 2026-10-07.
**Stress test, pre-grilling.**

Citation shorthand for research units per `research/24-completion-architecture.md:10-25`
(`dispatch`, `trigger`, `latency`, `zed`, `novel`, …); issues cited as `issues/NN:line`.

---

## ST-D — findings on Q1 (context-detection / dispatch architecture)

### ST-D1 — **severity: high** — "memoized per (uri, version)" never fires on the typing path; per-keystroke L1 is a fresh O(document) scan plus a full re-parse, and the recommendation's caching language papers over it.

- Every keystroke is a new document version, so a `(uri, version)` memo is invalidated before the
  first completion request that would use it — the memo only helps *same-version* repeats (resolve +
  completion, hover + completion), never steady typing. The classifier re-runs L1 per request
  (`research/24-completion-architecture.md:129-133`, i.e. `dispatch §7`; `dispatch:236-244`), and S1
  (content access) is a full `pulldown-cmark` re-parse per version because "parser stays
  pulldown-cmark with full re-parse per edit" (`map.md:51`, `map.md:78`) — with the measured
  worst case **18.6 ms** on a 51.2 KB long line (`issues/23:56`, `latency:183` S1 row). Under
  `concurrency_level(1)` that cost sits on the shared dispatch gate ahead of `didChange`/hover/
  diagnostics (`map.md:79`, `latency` §6). The §5 budget's "L1 memoized per version + L2 O(line)
  ≤1 ms" (`research/24-completion-architecture.md:518-519`) therefore budgets a cache hit that the
  typing path cannot take.
- **Suggested change (strengthen):** fold S1+S2 into **one per-version parsed snapshot** —
  `Note` + `LineIndex` + region table (frontmatter range, code ranges, `{{`/`}}` depth column)
  computed once per `(uri, version)` on first request and shared by completion/hover/diagnostics —
  and make the *target* design an incrementally-maintained region table spliced at `didChange`
  (frontmatter delimiters, fence toggles, depth deltas are all local), with the full scan as the
  correctness fallback. Hand 33 a named **degenerate-path row: S2 miss p99 ≤ 0.3 ms, never touches
  S3–S8** (`latency:197`) plus the snapshot-vs-rescan measurement question.

### ST-D2 — **severity: high** — P3's default-body `word:` rung contradicts 19's Answer and would fire inline-field-key completion on every prose "Note:" — a non-empty ~10 ms response on the single dispatch gate.

- 19's settled trigger text: the `:` trigger "must be in frontmatter region" (`issues/19:82`,
  verbatim: "must follow a valid YAML key pattern … must NOT be followed by another colon … **must
  be in frontmatter region**"). The consolidation's ladder step P3.8 then says "default body: `#` ⇒
  C4 tag; **word-boundary `:` per 19's regex ⇒ C8-adjacent**" (`research/24-completion-architecture.md:115`,
  `dispatch:199`) — and 19's own regex `^[a-zA-Z_-][a-zA-Z0-9_-]*\s*$` matches the line text before
  a colon in ordinary prose ("Note:", "Warning:", "TODO:"). A non-empty response is not a cheap miss:
  it runs S3–S8 (~10 ms of the ~11 ms typical budget, `research/24-completion-architecture.md:416-431`)
  on the one sequential handler slot.
- **Suggested change (strengthen):** adopt strict-19 in the decision text — body `:` completes only
  (a) inside an open `[`-bracket inline-field position (C8) or (b) the whole-line bare-field shape
  `Key::` (optional `>`/list prefix, per 19's own bare-field rule `issues/19:92`); any other
  word-boundary colon **declines** (S2 early exit). State it as a guard contract, not an
  implementation note — this is the single highest-volume trigger in the set.

### ST-D3 — **severity: med** — The ladder has no answer for the first `[` keystroke, and C8 would wrongly claim the start of every wikilink/footnote/checkbox.

- At the moment the *first* `[` of `[[`, `[^`, `- [`, or `[k::` is typed, the text is a bare `[`;
  rungs 1–3 (`[[`, `[^`, task) cannot match yet, so first-match-wins falls to rung 4 (inline field).
  19 says `[` trigger "only offer completion for inline fields" (`issues/19:81`) — so typing a
  wikilink in a vault with many schema fields pops field keys for one keystroke, then flips to
  wikilink targets. 23 also owes 24 the related `- [t` ambiguity (`decomposition` constraint row for
  23; `issues/23:60` "→ 24: … `[`-context ambiguity at `- [t`") and Q1/Q2 answer neither.
- **Suggested change (strengthen):** pin the tie-break explicitly in the decision: cursor directly
  after a bare `[` with nothing classifiable ⇒ **`[]` with `isIncomplete: true`** (session seed —
  see ST-D5); C8 requires a following key-char or `::` to appear; the `- [t` case declines the
  checkbox layer and falls through to C8 only on `::` shape. This is also the honest vehicle for
  C8/C9/C12 entry on clients whose word-char re-query is off (ST-T7).

### ST-D4 — **severity: high** — C11 (daily-note date in a link target) has neither an entry vehicle nor a completer under Q1/Q2, so 18's deferral is *not* satisfied — contradicting the consolidation's own handoff table.

- 18's resolved Answer: "**Completion dispatch deferred to ticket 24**" (`issues/18:38`), and the
  decomposition lists "daily-note date in link target" as a C-table context owned via the `](` rung
  (`decomposition` unit-1 context list; `dispatch:142` C11). The consolidation's §10 then asserts
  "18 … Its deferral is satisfied — date completion in `](` lives in the P3 ladder/C1–C17 … no
  further input owed" (`research/24-completion-architecture.md:666`). But Q2 defers `(` (so nothing
  opens a session inside `](`), P3.5 "recognize, then **decline**-or-C3" (`dispatch:192-193`) is the
  hedge, and §9.2 leaves `](` ownership to 32 (`research/24-completion-architecture.md:583-589`).
  Under both live options — (a) rumdl keeps `](` completion or (b) flag off and accept the gap —
  Traces declines `](`, and C11 dies silently with it. 18's ±7-day hybrid completion
  (`issues/18:28`) then has no delivery path.
- **Suggested change (strengthen):** claim C11 now, narrowly: inside `](`, guard on a date-shaped
  target prefix (empty or `^\d{4}-\d{2}(-\d{2})?`), candidates = 18's algorithmic ±7 set, entry via
  word-char/`Invoked` (Zed: `zed` Q3(b)); the date-shape guard makes any rumdl overlap
  non-duplicating (different item shapes; Zed does no LSP-vs-LSP dedup, `zed` Impact 2). If the
  grilling refuses the claim, record an explicit amendment to 18 ("completion descoped; dispatch
  question closed as declined") — do not carry "deferral satisfied" into the resolution.

### ST-D5 — **severity: med** — The cap-conditional `isIncomplete` rule ("produced > cap, nothing else") collides with two requirements that need session seeding; as written it silently kills 23's stage-2 flow.

- 23 (resolved) specifies the emoji completion's stage-2 date list is delivered "via
  `triggerSuggest`/isIncomplete session" after accepting `📅 ` (`issues/23:30`) — that requires a
  `isIncomplete: true` response from a list that is *under* the cap. The consolidation locks
  `isIncomplete = (candidates_after_dedup > cap)`, "**nothing else**" (`research/24-completion-architecture.md:297-299`,
  contract 8 at `:516-518`), with only degrade-contexts as an exception — and stage-2 is not a
  degrade context. The same strictness breaks ST-D3's bare-`[` seed. Neovim's core client also
  doesn't execute completion-item commands (`research/24-completion-architecture.md:279-282`), so
  `triggerSuggest` is not the portable vehicle.
- **Suggested change (strengthen):** define a second, enumerated legal source of
  `isIncomplete: true` — "session-seeding responses" (declined-but-open positions such as bare `[`,
  and 23's post-`📅 `-accept stage-2) — as a closed list in the decision text, and hand 23 the
  confirmation that the vehicle is the incomplete session, not `command`. Alternative for 23 if the
  grilling dislikes seeding: fold the top-K date values into the initial `📅 ` item's list (one
  response, no second round trip — strictly better under the 20 ms budget).

### ST-D6 — **severity: med** — C13's callout predicate requires `[!`, but `!` is unregistered — in Zed the menu can only open after the user has typed the first type-letter; Obsidian-style "pop at `> [!`" is unreachable.

- The ladder's callout rung is "`> ` + `[!` ⇒ callout (C13)" (`dispatch:195`), and the trigger unit's
  guard is `^\s{0,3}>\s*\[!` (`trigger` §4 `>` row). In Zed, typing a non-word unregistered char
  "**sends no request and closes the menu**" (`zed` Q3(c), `completions.rs:399-401,1527-1541`): `>`
  misses (no `[!` yet), space closes, `[` misses (still no `!`), `!` closes, first letter finally
  opens the filtered list. 17's resolved depth is "Completion: 13 canonical types + aliases" (`issues/17:66`)
  — an unfiltered type-picker is the feature; first-letter-only entry degrades it to guess-what-to-type.
- **Suggested change (strengthen):** relax C13's predicate to `^\s{0,3}>\s*\[!?$` with the cursor
  between the bracket(s) and the type text, so the `[` keystroke (registered) opens the full 13-type
  list; accept the one-keystroke close at `!` (next word char re-opens filtered); do **not** register
  `!` (sentence-final exclamations are common in prose).

### ST-D7 — **severity: low** — Q1's summary omits the dispatcher's return contract, which three other decisions hang off.

- The decision must pin "classifier returns `ContextKind` **+ replace range + filter word**", not just
  a kind — item shape (`range.start` is part of the item contract), `filterText` co-design, and the
  resolve `data.c` enum all consume it (`research/24-completion-architecture.md:504-505,522-524`,
  `item-shape:820-822`). Grilling on "which architecture" can easily drop this line and break §3/§4.

### ST-D8 — **severity: low** — L1's code veto must be fence-language-aware from day one or 21-Phase-2 fenced `traces-query` blocks will require re-litigating P0.

- 21's Phase 2 defers fenced-code-block query contexts (`issues/21:59`), and P0 is "code veto,
  first, unconditionally" (`dispatch:160-162`). When Phase 2 lands, a ` ```traces-query ` fence must
  classify as *query world*, not code — i.e. L1's code regions are "fenced/indented/inline **except
  a language tag on the allowlist**". Naming that seam in the Q1 decision costs one sentence and
  avoids restructuring the precedence ladder later.

---

## ST-T — findings on Q2 (v1 trigger set)

### ST-T1 — **severity: high** — Sub-call (a) is backwards: 22 is *resolved* and has filed the `.` trigger request as a named owed input; presenting `.` as speculative contradicts 22's Answer.

- 22's resolved research records the owed input verbatim: "24 owns shared completion + trigger
  reconciliation; 22 supplies data + staging proposal + **trigger `.` request**" (`research/22-template-intelligence.md:175-177`),
  repeated as a constraint on 24 ("22 … **files a `.` trigger request**", `decomposition` constraints
  row for 22). Helper/namespace-member completion is a phase-1 deliverable, not speculative: the
  degrade ladder lists "filter/test/global/**namespace completion**" as features that are *on* in the
  healthy state and only off under construction-failure (`research/22-template-intelligence.md:418-419`),
  backed by the hand-authored ~25-entry phase-1 table + `enumerate()`-derived members
  (`research/22-template-intelligence.md:376,125-130`). "Drops out unless 22's template-helper
  completion ships" (`Q2 sub-call (a)`) inverts the dependency: 22 shipped the *decision*; only the
  implementation ordering is 24's to sequence.
- **Suggested change (strengthen):** register `.` **unconditionally** in the v1 set. The cost of
  registering it before 22's completer is live is exactly one O(line) miss-path check
  (helper-namespace regex on line-to-cursor, `trigger` §4 `.` row, "miss path must be near-zero");
  a phase-variant trigger set buys ~nothing and costs a release-to-release behavior delta plus a
  bigger test matrix. If a knob is wanted at all, make it a user-facing
  `[lsp.completion] trigger_overrides`, not a build-phase flag.

### ST-T2 — **severity: high** — The `:` decision is a *guard* decision, not a registration decision — the registration is cheap, the response shape is the latency event; Q2 states the former and not the latter.

- Registration fires one request per *typed* char (not per completion request; sessions are
  re-queried per keystroke only under `isIncomplete: true` or Zed's word-char `Invoked`
  (`zed` Q3(b),(e)) — the cap-conditional rule keeps most sessions complete ⇒ local filter only).
  But `:` is typed constantly in prose (times, labels, `Note:`), and if the guard admits them the
  response is non-empty → S3–S8 (~10 ms) on the single sequential gate per occurrence
  (see ST-D2; budget `research/24-completion-architecture.md:416-431`; gate `map.md:79`).
- **Suggested change (strengthen):** move the strict-`:`-guard (ST-D2) *into* Q2's decision text as
  the contract for every registered char: "**every trigger char's degenerate path is an O(line),
  ≤0.3 ms, S3–S8-free decline; a non-empty response is only ever produced inside a classified
  context**" — and hand that row to 33's `lsp_latency.rs` (per-trigger miss-path histogram). Also
  record the Zed fan-out precision (ST-T6) so the audit covers rumdl-triggered requests too.

### ST-T3 — **severity: med** — Excluding `/` costs a verified menu-close at every wikilink path segment, and the stated rationale (rumdl territory) is wrong for `[[`-interior paths.

- Zed: unregistered non-word char ⇒ "no request to any server … and closes the menu" (`zed` Q3(c)).
  So `[[projects/act|` → `[[projects/` **closes** the popup at each `/`; the session re-opens on the
  next word char (`Invoked`, subject to ST-T7's `trigger_in_words` caveat). Segment-wise path
  completion is core PKM UX and it flickers on the primary target editor. The exclusion rationale —
  "`/` only meaningful inside `](path` — territory 15 cedes to rumdl" (`trigger` §5) — does not
  apply inside `[[`: rumdl's `/` path structurally requires `](` earlier on the line
  (`trigger` §1.3, "rumdl structurally never completes wikilinks"), so the two `/` contexts are
  disjoint by the same argument §2.3 uses for `#`.
- **Suggested change (strengthen):** either (i) **add `/`** with a cheap line-local guard ("cursor
  inside `[[…`", decline elsewhere — under 32's recommended flags only Traces pays the volume;
  under default flags both servers get every slash, both decline fast), or (ii) keep it out but
  write the cost into the decision ("menu closes at each path segment in Zed/VS Code-class clients;
  session resumes on next word char") so the grilling chooses the flicker knowingly. (i) is the
  capability-forward choice; the volume is one O(line) decline per slash in prose/URLs.

### ST-T4 — **severity: med** — Q2 lists `|` under "never register" without stating the delivery vehicle, and for Zed that vehicle is the *only* one that exists.

- Verified: typing `|` in Zed sends **no request and closes the menu** (`zed` Q3(c)); VS Code likewise
  won't re-query on an unregistered non-word char (`research/24-completion-architecture.md:600-605`,
  §9.5). So alias completion (`[[target|alias]]`) must ride the `[[target` response itself. §9.5
  leaves it as an open question (register `|` vs alias list at target time); Q2's "never `|`"
  silently picks "not registered" and answers nothing about the list.
- **Suggested change (strengthen):** state the vehicle in the decision: **C1's completion response
  includes alias candidates for the resolved target (capped, in `sortText` group after the target
  itself); no `|` registration; no `isIncomplete` tail required.** Hand the item-shape consequence
  (alias items' `filterText`/range — the user hasn't typed anything on the alias side) to §3 and 15.

### ST-T5 — **severity: med** — "both servers keep `isIncomplete: false`" overclaims: 24 controls exactly one of those two servers.

- Zed OR-couples `isIncomplete` across servers with an upstream TODO (`zed` Impact 2,
  `completions.rs:640-651`): if *rumdl* ever returns `true`, both servers re-query every keystroke.
  Traces cannot decide rumdl's flag; rumdl's own handler shape is verified independent of us
  (`trigger` §1). Writing "both servers keep isIncomplete: false" into 24's decision makes 24's
  correctness rest on a clause only 32 can *document* (as a user-facing ask) and nobody can enforce.
- **Suggested change (strengthen):** reword to "Traces emits `isIncomplete` cap-conditional (+ the
  ST-D5 seeding exception); 32 documents the rumdl-side ask and the OR-coupling risk, and recommends
  `lsp_fetch_timeout_ms` if the unbounded `join_all` matters" (`zed` Impact 1–2). Keep the claim on
  our side of the boundary.

### ST-T6 — **severity: low** — Under default rumdl flags the fan-out is the *union* set × *both* servers, not just the overlapping chars — Q2's "overlap costs request volume" understates it, and our miss path must also be cheap for rumdl-only triggers.

- Zed gates the union once (`buffer.completion_triggers()`, `buffer.rs:132,3492-3514`) then fans out
  to every capability+scope-passing server (`lsp_store.rs:7910-7941`) — "Traces receives
  rumdl-triggered ones" too (`zed` Impact 2). So Traces will see `(`, `/`, `-`-triggered
  `TRIGGER_CHARACTER` requests it never registered. The L0 design already makes this safe
  ("none is load-bearing for classification", contract 3, `research/24-completion-architecture.md:507-508`),
  but the decision text should say the degenerate-path contract (ST-T2) applies **to every request,
  registered char or not**, since 32's flag recommendation is advisory, not enforceable at runtime.

### ST-T7 — **severity: low** — Three reachable-by-Q1 contexts have no non-word vehicle at all, and Zed's word-char entry is itself conditional on an edit-prediction setting — the Invoked-only floor needs to be an explicit, per-context decision.

- C6 (frontmatter **key** at line start), C16 (template `include`/`extends` name, `"` deferred), and
  template **root names** (`{{ q…` → `query`) open on word chars or manual invocation only. Zed's
  word-char path is gated by `trigger_in_words = show_edit_predictions_in_menu() ||
  !had_active_edit_prediction` (`zed` Q3(b), `input.rs:507-508`) — i.e. with ghost-text edit
  predictions active and the menu-prediction setting off, word chars may not query at all; the unit
  flags that interplay OPEN (`zed:407`). §9.4 already poses the question
  (`research/24-completion-architecture.md:594-599`); neither Q1 nor Q2 answers it.
- **Suggested change (strengthen):** accept the floor but *decide it per context in the decision
  text* — "C6/C16/root-names are Invoked-first contexts; reachable in Zed only if
  `trigger_in_words` holds, else manual invocation" — and put it in 29's sparse-client floor matrix
  so the capability doc doesn't imply uniform reachability. Verify Zed's setting default before
  claiming the word-char path works out of the box.

### ST-T8 — **severity: low** — The `- [t` ambiguity owed to 24 by 23 is unanswered by both recommendations.

- 23's seam list ends "→ 24: dispatch, trigger registration, shared completion-generation function,
  **`[`-context ambiguity at `- [t`**" (`issues/23:60`); the decomposition repeats it as owed
  (`decomposition` 23 row). Q1's ladder handles `- [` (Incomplete) but not what happens once the
  user types a non-checkbox char inside the brackets.
- **Suggested change:** answer it in one line at the grilling (recommended: non-checkbox char inside
  `- [` ⇒ checkbox layer declines; falls through to C8 only on `::` shape; no list flicker) and fold
  it into ST-D3's tie-break paragraph.

---

## Fog / handoff — must be decided or explicitly routed now

1. **To 32 — `](` ownership pick (trigger Q3/Q4)**: (a) keep rumdl's flag on, (b) accept the gap,
   (c) Traces later registers `(`. Under (b) the gap includes *standard-link* completion **and** C11
   unless ST-D4's narrow claim lands. 15's ownership table has **no row** for standard-link
   completion/navigation — the two things the recommended flags switch off
   (`research/24-completion-architecture.md:548-551`); 15's joint-setup comment
   "Traces handles wikilink completion" is imprecise (rumdl never completed wikilinks,
   `issues/15:204`). Both corrections must travel with 32's resolution, not after it.
2. **To 33 — three unfilled rows**: (a) resolve p95 ≤ 5 ms (D5 says "record it there";
   verified: 33's interactive table has **no** resolve row, `issues/33:45-56`) — load-bearing because
   Zed fires ~20-item serial resolve batches per menu event on the typing path (`zed` Q5,
   20×5 ms = 100 ms serialized); (b) degenerate-path/miss-path row (S2 ≤ 0.3 ms p99, per-trigger
   histogram, ST-T2); (c) the S1/S2 shared-snapshot measurement question (ST-D1), including the
   51.2 KB long-line anomaly already self-flagged by 23 (`issues/23:56`).
3. **To 29 — two floor statements**: the Invoked-only context list (ST-T7) and the `isIncomplete`
   seeding exception (ST-D5), because clients differ on incomplete re-query and manual-invocation
   fallback; both belong in the C-N1..C-N14 block's sparse-client floor (`capability` §MVC-C).
4. **To 15 / 18 — C11 and the alias vehicle**: ST-D4 (claim or amend 18) and ST-T4 (alias items in
   the C1 response) both change 15/18's completion designs and must be settled in 24's grilling, not
   deferred to their (already resolved) tickets without an amendment.
5. **To 23 — stage-2 vehicle confirmation**: ST-D5 requires either the seeding exception or folding
   the date values into the initial list; 23's "triggerSuggest" wording should be amended to the
   portable vehicle when 24 resolves.
6. **To 34 — unchanged**: classifier seam + `scan_marker_prefix`/`TaskScan` widening
   (`research/24-completion-architecture.md:668`) still owed; ST-D1's snapshot object is the seam
   carrier (region table lives on it).

---

## Verdict per decision

- **Q1 (context dispatch, variant (c)): strengthen.** The architecture is right — layered hybrid,
  one completer, static registration, L0-as-hints — but as drafted it budgets a cache that misses on
  the typing path (ST-D1), contradicts 19's `:` guard (ST-D2), leaves the first-`[` and `- [t`
  tie-breaks unanswered (ST-D3/ST-T8), silently drops 18's C11 (ST-D4), and its `isIncomplete`
  contract kills 23's stage-2 flow (ST-D5). None of these weaken the architecture; all of them make
  it sharper and more capable.
- **Q2 (v1 trigger set): strengthen.** Keep the static flat set and the `>`/`^` inclusion — correct
  and 17-justified. But make `.` unconditional (22 is resolved and filed it — ST-T1), move the
  strict-`:`-guard into the decision (ST-T2), decide `/` for wikilink paths rather than citing the
  wrong rumdl rationale (ST-T3), state the `|`-alias vehicle (ST-T4), and stop claiming authority
  over rumdl's `isIncomplete` (ST-T5). The set is otherwise internally consistent with every
  resolved ticket's trigger filings (16 `#`, 19 `[`+`:` (+`(` dropped, matches), 21 `#`+`@`,
  23 no-space, 17 `^`+`>`).
