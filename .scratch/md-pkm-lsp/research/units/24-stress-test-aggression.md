# Ticket 24 — Stress-test: aggression (timidness) pass on the six draft recommendations

Aggression pass — 2026-10-07, pre-grilling. RESEARCH ONLY. Attacks the six draft recommendations
(Q1–Q6, restated below from the consolidated §0/§9) along five angles: deferred capabilities,
hot-path efficiency, UX, caution-due-to-absence-of-evidence, over-engineering. Every claim cites
`file:line` from `.scratch/md-pkm-lsp/`. Sibling units are cited by filename; the consolidation as
`../24-completion-architecture.md:NN`; tickets as `issues/NN:NN`.

The six under test:

1. **Q1** layered hybrid dispatcher: L0 hints → L1 region → L2 line predicates → L3 first-match, `None` ⇒ `[]`.
2. **Q2** static init-time trigger set `[ # : @ . ^ >`; `"`/`(` deferred; `.` phase-derived; never `/ - ` `` ` `` space `| ]` word chars.
3. **Q3** resolve defers ONLY `documentation`; `detail` always initial; no `itemDefaults`; resolve p95 ≤ 5 ms.
4. **Q4** ZERO snippets in v1 (plain-text wikilink, user types `|`); collapse C13/S-18/R7 out of v1.
5. **Q5** frontmatter `:` = keys before, values after; values owned by ticket 20.
6. **Q6** accept client-side re-sort; `sortText` only guaranteed at empty input.

Verdict shape: every recommendation survives; upgrades are precision (staleness, wording,
measurement gates, one machinery cut), not boldness. Ticket statuses checked live 2026-10-07:
15/16/17/18/19/20/21/22/23 all `resolved`; 24 `claimed` (`issues/24:5`).

---

## 1. Findings

### ST-A1 — HIGH — Q2's config-/phase-derived trigger machinery is over-built; hardcode the seven chars

**Claim.** The static-set recommendation is right; the *derivation* mechanism around it is not. The
draft keeps config-derived contents + phase flags alive for `.` (`../24-completion-architecture.md:590-593`,
§9.3 "if 22 ships later"), 17's callouts, and 21-Phase-2 `(`/`"` (`24-completion-trigger-characters-rumdl.md:163`).
Those premises have expired: 22 is resolved with completion decisions locked (D5: never emit
degraded items; member table built once at engine setup — `24-novel-embedded-context-precedent.md:124-127`,
`issues/22:5`), and 17 is resolved with callout completion explicitly in scope (`issues/17:58-66`).
The only surviving phase gate is `(`/`"`, for which *no v1 code exists at all* (21 Phase 1 = tokens
only, `24-snippet-completion-insertions.md:426`). Config machinery whose single remaining flag
enables nothing is pure surface: registration is static at `initialize` (`../24-completion-architecture.md:144-150`),
so a config flip mid-session would inherit rumdl's restart asymmetry problem
(`24-completion-trigger-characters-rumdl.md:73,184`) for zero benefit.

**Evidence.** `../24-completion-architecture.md:590-593,144-150`;
`24-completion-trigger-characters-rumdl.md:114,163,213-214`;
`issues/17:58-66`; `issues/22:5,55`; `24-novel-embedded-context-precedent.md:124-127`.

**Upgrade.** Hardcode `[ # : @ . ^ >` in code. `.` registered unconditionally: C15's miss path is
depth>0 ∧ namespace-root token gate — O(line), ≤0.3 ms, the degenerate budget the architecture
already promises (`../24-completion-architecture.md:255-257`); rumdl's own precedent is that a
high-frequency char becomes an O(line) miss via a fast path, not via registration avoidance
(`24-completion-trigger-characters-rumdl.md:61`). When 21-Phase-2 lands, add `(`/`"` as a code
change, not a config key. **Honest cost:** a user cannot "pre-enable" Phase-2 triggers — nothing to
enable; and we lose the rumdl-style "flip without restart" story — which was never expressible for
Traces anyway (static registration, one flat array, `24-completion-trigger-characters-rumdl.md:162`).

### ST-A2 — HIGH — Q6 surrenders ranking without stating its two counterweights; bind them into the decision

**Claim.** "Accept client-side re-sort; sortText only guaranteed at empty input" is factually right
(VS Code fuzzy score outranks sortText; Zed tier chain score > sortText, verified
`../24-completion-architecture.md:313-318`) but as written it is timid: it concedes ranking in the
*typed-input common case* while two counterweights already exist in the item contract and go
unmentioned in the recommendation. Worse, one counterweight is client-limited: **Zed never reads
preselect** (`../24-completion-architecture.md:493-494`) and an `OtherMatch` demotion voids sortText
authority *entirely* when no split word of `filterText` starts with the query's first char
(`../24-completion-architecture.md:321-323`). In Zed — a primary target — the filterText discipline
is the *sole* ranking counterweight. The consolidation records it as "re-check on any future shape"
(`../24-completion-architecture.md:323`), i.e. advisory; it must be a builder invariant.

**Evidence.** `../24-completion-architecture.md:305-306,313-324,491-496`;
`issues/23:24` (preselect top for task statuses); `issues/20:37` (relevance-ordered file values).

**Upgrade.** Restate Q6 as: accept re-sort; guarantee `sortText` at empty input; counterweight (a)
`preselect` on the single best candidate where the client honors it (VS Code, gated on
`preselectSupport`), (b) **every emitted `filterText` must contain a split word aligned with the
typed prefix — enforced at the item builder, not by reviewer vigilance**; (c) contexts whose
comparator encodes hard ordering (23's config-order statuses, 20's relevance classes) treat
preselect as load-bearing, not decoration. **Honest cost:** one builder assertion + promoting an
existing contract line; no new protocol surface. Any future context that cannot satisfy (b) must
carry a documented "loses Zed ranking" note.

### ST-A3 — MEDIUM — Q2's `^`/`>` "provisional" hedge is stale; 17 is resolved with both features in scope

**Claim.** The draft (following the consolidation) marks `^` and `>` provisional — "drop if 17
descopes" (`../24-completion-architecture.md:203`;
`24-completion-trigger-characters-rumdl.md:115-116,213`). 17 has resolved and descoped nothing:
footnotes are in scope with completion-suggest-labels-when-typing-`[^` as an explicit operation
(`issues/17:28-39`), callouts are in scope with 13-type completion + config extension
(`issues/17:58-66`). Keeping the hedge alive invites a spurious descope decision at the grilling
against a resolved ticket's answer.

**Evidence.** `issues/17:28-39,58-66`; `../24-completion-architecture.md:203`;
`24-completion-trigger-characters-rumdl.md:115-116,213`.

**Upgrade.** Promote `^` and `>` to unconditional members of the hardcoded set (rides ST-A1);
delete the "provisional — drop if 17 descopes" language from the recommendation. **Honest cost:**
none — both chars keep tight server-side guards (`^\s{0,3}>\s*\[!` callout pattern; `^` only when
immediately preceded by `[`, `24-completion-trigger-characters-rumdl.md:115-116`), so the miss path
is O(line).

### ST-A4 — MEDIUM — Q4's zero-snippet call is correct and aggressive, but its degradation story is the wrong row

**Claim.** Zero snippets in v1 is *more* aggressive than the consolidation's §0 bullet (which kept
C1 as "the one v1 snippet", `../24-completion-architecture.md:51`) and it is the right call — the
cursor-stopping range makes plain text structurally achieve the ticket's own placement goal
(`24-snippet-completion-insertions.md:356,361-365,383-386`). But the draft's degradation wording —
"user types `|`" — describes only the *fresh-link* state (no trailing `]]`; row A1 → P1 `[[T]]`,
`24-snippet-completion-insertions.md:379-380`). In the paired state (editing an existing link, row
C1), plain insertion `[[T|` lands the cursor **inside the alias slot for free** — the user never
types `|` (`24-snippet-completion-insertions.md:356,361-365`). Two reachable states, opposite UX;
the recommendation as worded under-sells the shipped experience and mis-frames the cost.

**Evidence.** `24-snippet-completion-insertions.md:350-386,465-475`;
`../24-completion-architecture.md:51,566-582`; `issues/24:14`.

**Upgrade.** Keep zero snippets. Restate the contract: `render_new_text` branches on
*trailing-`]`-present* — paired state → `[[T|` (slot pre-opened), fresh state → `[[T]]` (user types
`|` + display); C13/R7 stay collapsed (C13's three coupled changes — kind flip, S-18 kind gate, R7
baked prefixes — are moot without snippets, `24-snippet-completion-insertions.md:391-406`); S-18
demotes from "moot in v1" (`24-snippet-completion-insertions.md:484-485`) to a *recorded rule* for
any future snippet re-adoption (kind 15 must ride the same gate as the format — Helix parses on
kind *or* format, `24-snippet-completion-insertions.md:319-321`). **Honest cost:** one extra
condition in the renderer (two dims, four arms, three reachable); nothing else.

### ST-A5 — MEDIUM — Q2's `(`/`"` deferral is correct, not timid — but §9.12's entry gap must be closed as a decision

**Claim.** Deferring `(`/`"` is the aggressive-honest call: there is no v1 consumer to trigger on.
21 Phase 1 ships token completion only — no query-string scaffold is ticketed for v1
(`24-snippet-completion-insertions.md:426` citing `issues/21:26`); unit 7's classifier row (a)
registers `(`/`"` for a context whose *detection* machinery (string-arg of a `query.*` call) is
Phase-2 work (`24-novel-embedded-context-precedent.md:85-86`). Pulling `(` forward would fire a
request on every prose parenthesis with no completer behind it. But the consolidation flags a real
hole: "how a query-string request starts in v1 is unstated" (`../24-completion-architecture.md:639-643`,
§9.12). It is answerable today: field-name completion in filter expressions
(`issues/21:14`) is entered via `Invoked` (manual invoke / editor quick-suggest) or via the
registered chars that *do* fire inside `{{ }}` — `#` (query tags) and `@` (file-class tokens,
`24-completion-trigger-characters-rumdl.md:111,113`) — with session continuation by `isIncomplete`.

**Evidence.** `../24-completion-architecture.md:639-643,207`;
`24-snippet-completion-insertions.md:426`; `issues/21:14,26`;
`24-novel-embedded-context-precedent.md:85-86`; `24-completion-trigger-characters-rumdl.md:111,113`.

**Upgrade.** Keep the deferral. Close §9.12 explicitly in the recommendation: *v1 query-context entry
= `Invoked` + `#`/`@`-inside-`{{ }}`; `(`/`"` join the hardcoded set when 21-Phase-2 lands (ST-A1's
code change)*. Note the known editor-dependence of word-char entry as an accepted limitation
(`../24-completion-architecture.md:594-599`) — Zed word chars do re-query (as Invoked), so
field-name completion is reachable after the first letter there. **Honest cost:** field-name
completion in filters is discoverable-by-invoke, not fire-on-keystroke, until Phase 2. Stated, not
silent.

### ST-A6 — MEDIUM — Q3's 5 ms resolve budget is reasoned-not-measured, and Zed moved it onto the typing path — make the histogram a launch gate

**Claim.** The resolve *shape* (defer documentation only, `detail` live, no `itemDefaults`) is
settled and not timid — deferring more is crate-blocked (`itemDefaults` gaps,
`../24-completion-architecture.md:368-370`), deferring less reintroduces the S5/S8 blow-up D1
rejects. What remains timid is the *discipline*: ≤5 ms is "reasoned not measured"
(`../24-completion-architecture.md:611-615`), and Zed fires resolve in ~20-item serial batches at
menu open, at every keystroke-refilter, and per selection change — 20 × 5 ms = 100 ms of
serialized server work sitting on the typing path under `concurrency_level(1)`
(`../24-completion-architecture.md:356-360`). §10 lists the S-R row as "input to 33"
(`../24-completion-architecture.md:664`) — owed, not gated.

**Evidence.** `../24-completion-architecture.md:326-373,352-367,611-615,664`.

**Upgrade.** No shape change. Promote the resolve-duration histogram (D7's telemetry hook, same
instrumentation as the fail-open rate already required by D4) from "measurement owed" to a
**launch gate**: completion is not done until p95 ≤ 5 ms is *observed* at cap-sized lists, with the
20-item-batch worst case recorded. If it fails, the escape is not "relax the budget" — it is
idle-time pre-resolution of top-K documentation (`../24-completion-architecture.md:447-449`
leverage order, "build during idle"). **Honest cost:** one instrumentation row in 33 + a gate
sentence; the pre-resolve escape stays unimplemented until data demands it (YAGNI).

### ST-A7 — MEDIUM — S1 content memoization is a v1 requirement, not an optimization; promote it to a §7 contract

**Claim.** The stage table already says "must memoize per `(uri, version)` — worst case 18.6 ms for
a 51.2 KB long line" (`../24-completion-architecture.md:421`) — a *single* long-line extraction
blows the entire 20 ms p95 before classification starts (`../24-completion-architecture.md:411-413`).
But the cross-unit contracts list (§7) — the edges that "must survive consolidation"
(`../24-completion-architecture.md:500-530`) — does not carry it. Once hover (26) and diagnostics
(25) land, each handler reaching for content independently re-creates the per-request extraction
the budget forbids. This is the classic hot-path failure the memo exists to prevent, currently held
by one note in a table.

**Evidence.** `../24-completion-architecture.md:409-430,421,500-530`.

**Upgrade.** Add contract 15: *one content snapshot per `(uri, version)`, built lazily, shared by
completion/hover/diagnostics/definition; no handler re-extracts document text.* **Honest cost:**
one line + the memo's own storage (bounded by open documents — the transient tier already budgeted,
`issues/33:83`).

### ST-A8 — LOW — Q1's four layers are three tests and a dispatch; L0 is a contract line wearing a layer's clothes

**Claim.** The layered hybrid is right (mdoxide's first-match chain is the verified precedent,
`24-novel-embedded-context-precedent.md:30,185-189`; per-context registration is unexpressible in
the flat trigger array, `../24-completion-architecture.md:64-68`). But L0 does no work — it is
"trigger_kind is never load-bearing", which the consolidation already carries as contract 3
(`../24-completion-architecture.md:506-508`). Presenting it as a layer overstates the machinery the
grilling is being asked to bless, and feeds the adjacent over-engineering risk: designing the
ticket-34 classifier seam against an imagined span-containment API before ticket 11 exists
(`../24-completion-architecture.md:98-100`).

**Evidence.** `../24-completion-architecture.md:79-100,506-508`;
`24-novel-embedded-context-precedent.md:30,185-189`.

**Upgrade.** Describe the dispatcher as L1 region → L2 line predicates → L3 first-match dispatch
(three tests + one dispatch), with "trigger_kind never classifies" as a preamble contract line.
Keep the `ContextKind` enum seam for 34; explicitly *do not* design the span-containment interface
in 24 — lexical is the v1 implementation, spans swap in behind the enum when 11 lands.
**Honest cost:** none; description-only, plus restraint 34 must respect.

### ST-A9 — LOW — Q5's key/value split is right; state the value-side handoff honestly (word-char entry, no commitCharacters, no command)

**Claim.** Keys-before/values-after is the correct classifier contract — the regions alone resolve
every `:` nesting (`../24-completion-architecture.md:105-107`), 19 owns keys with a settled regex
boundary (key must not be followed by another `:`, `24-completion-trigger-characters-rumdl.md:112`),
and 20 owns frontmatter values with inline fields explicitly deferred (`issues/20:21`) while 23
claims the date-value gap 19/20 provably leave (`issues/23:27`). §9.11's tension
(`../24-completion-architecture.md:635-638`) is closed by the draft. What the draft does *not*
state: what happens at the key→value handoff. C6 inserts `key: ` with the cursor in value position
(`24-snippet-completion-insertions.md:418`); programmatic insert does not fire triggers; the value
list then arrives on the next user word char (Zed: verified re-query as Invoked,
`../24-completion-architecture.md:594-599`; VS Code: editor-dependent quick-suggest,
`24-completion-trigger-characters-rumdl.md:131`) or manual invoke. The two obvious "fixes" are both
worse: `commitCharacters` has zero corpus precedent and Zed never reads them
(`../24-completion-architecture.md:267-268,493-494`); `command`-based re-trigger needs a client
command handler Traces does not have (`../24-completion-architecture.md:403-403`).

**Evidence.** `../24-completion-architecture.md:105-107,267-268,353-353,403,594-599,635-638`;
`24-completion-trigger-characters-rumdl.md:112,131`; `24-snippet-completion-insertions.md:418`;
`issues/20:21,37`; `issues/23:27`.

**Upgrade.** One sentence into Q5: *key→value handoff relies on word-char/`Invoked` entry
(editor-dependent, accepted); `commitCharacters` stay banned (zero precedent, Zed-blind) and
`command` re-trigger stays unavailable — rechecked 2026-10-07, both still correct.* Also carry §8.1's
amendment: 20's `is_incomplete: true` is cap-conditional on the value list too
(`../24-completion-architecture.md:297-299,536`). **Honest cost:** none — the limitation is real,
the alternatives are worse, and saying so prevents a grilling rabbit hole.

### ST-A10 — LOW — Hot-path attack angles check out as correctly-rejected restraint; record the rejections so they aren't re-litigated

**Claim.** Each efficiency idea the attack brief names is either already present or correctly
absent:

| Attack idea | Verdict | Evidence |
|---|---|---|
| Memoized classifier | **Present** — L1 per `(uri, version)`, L2 O(line) ≤1 ms, no eager `didChange` classification | `../24-completion-architecture.md:131-133` |
| `isIncomplete` reuse / re-trigger sessions | **Present** — server-side incomplete tracking, cap-conditional `isIncomplete`, Zed OR-coupling respected | `../24-completion-architecture.md:297-299,242-244,516-519` |
| Caps | **Present** — `COMPLETION_CAP = 100`, rank-then-truncate | `../24-completion-architecture.md:54,300-302` |
| Incremental/prefix filtering, prepared parse trees | **Rejected, correctly** — full re-parse per edit is the settled substrate (tickets 11/14); skim V2 ≈1.7 ms @10⁴ fits S4's ≤3 ms | `../24-completion-architecture.md:411-413,424` |
| Trie / tag caches | **Rejected, correctly** — precomputed sorted universes + refresh-swap Arc semantics (ticket 12) already bound the work at 20 K files; a trie adds invalidation against immutable-swap for negative gain | `../24-completion-architecture.md:436-445` |
| In-handler rayon / parallel S0–S9 | **Rejected, correctly** — measure first, YAGNI; `concurrency_level(1)` makes handler-internal parallelism the wrong lever | `../24-completion-architecture.md:449-450,412-414` |
| Recency pre-ranking | **Per-context only, correctly** — 20's relevance classes (recently opened → same folder → alpha) and 23's config-order preselect already encode recency where it matters; a *global* recency layer has no precedent, defeats layer sortText prefixes, and client re-sort voids it on typed input anyway | `issues/20:37`; `issues/23:24`; ST-A2 |

**Upgrade.** Transcribe the table into the grilling packet as accepted restraint. **Honest cost:**
none; prevents three re-litigations.

---

## 2. Q1–Q6 upgrade table

| Rec | Upgrade? | What changes | Honest cost |
|---|---|---|---|
| **Q1** dispatcher | Yes (light) | Collapse L0 to a contract line (ST-A8); state S1 content memo as §7 contract 15 (ST-A7); restrain 34's span-seam design | Description-only + one memo |
| **Q2** triggers | Yes (biggest cut) | Hardcode 7 chars — drop config/phase machinery (`.` unconditional, ST-A1); promote `^`/`>` off "provisional" (ST-A3); close §9.12 with the explicit v1 Invoked/`#`/`@` entry for deferred `(`/`"` (ST-A5) | No pre-enabling of non-existent Phase-2 code; one future code change instead of a config key |
| **Q3** resolve | Yes (gate only) | Shape unchanged; resolve p95 ≤ 5 ms becomes a **launch gate** backed by D7 histogram, worst-case 20-item Zed batch recorded (ST-A6) | One instrumentation row in 33 |
| **Q4** snippets | Yes (wording) | Zero-snippet call stands (it is the aggressive-and-correct branch of §9.1); degradation restated as the two-state reality — paired `[[T|` pre-opens the slot for free, fresh `[[T]]` requires the user's `|`; S-18 kept as a recorded re-adoption rule (ST-A4) | One branch in `render_new_text` |
| **Q5** `:` split | Yes (minor) | Decision stands; add the honest key→value handoff sentence (word-char/`Invoked` entry; commitCharacters and `command` rechecked-and-banned); carry 20's cap-conditional `is_incomplete` amendment (ST-A9) | None |
| **Q6** re-sort | Yes (teeth) | Accept-re-sort stands; **bind preselect + the filterText split-word builder invariant into the decision** — Zed ignores preselect and can void sortText via OtherMatch, making filterText discipline the sole Zed counterweight (ST-A2) | One builder assertion; promotes an existing advisory line to invariant |

## 3. Caution — absence of evidence (carry as explicit risk rows, not silence)

1. **rumdl's own `resolveProvider` unverified** — matters only if rumdl advertises resolve while co-tenant; owed to ticket 32 (`../24-completion-architecture.md:191-192`).
2. **Neovim built-in `preselect` honoring unverified** — Q6's counterweight (a) is portable to VS Code (verified) but not yet to Neovim core (`../24-completion-architecture.md:619`).
3. **VS Code resolve with details pane closed** — read as "yes, still sent", unverified (`../24-completion-architecture.md:619-620`).
4. **`[[T|]]` empty-alias terminal validity** — moot for v1 (plain fallback never keeps the `|`, `24-snippet-completion-insertions.md:381-382`) but blocks any future snippet re-adoption (`24-snippet-completion-insertions.md:513-515`).
5. **Multi-cursor plain-insert behavior** — snippets are gone from v1 but plain insert with multiple cursors still deserves the 5-minute manual matrix before 29 documents support (`24-snippet-completion-insertions.md:516-521`).
6. **MS markdown LS cursor-side code gate** — absence-of-evidence with a standing re-fetch instruction; do not cite as precedent without it (`../24-completion-architecture.md:624-625`).
7. **`Object::enumerate()` side effects** — 22 says registration is side-effect-free; enumeration itself unverified; gates the C15 member table built at engine setup (ST-A1's `.` depends on it) (`24-novel-embedded-context-precedent.md:234-236`).
8. **Post-accept trigger re-fire semantics** — whether programmatic/accepted-completion inserts re-fire registered trigger chars (VS Code `onDidType` vs accept path; Zed post-accept) is unverified; ST-A9's key→value handoff assumes they do *not* (next user keystroke opens values). If they do, the value list auto-opens — strictly better, no design change.
9. **Zed multi-cursor keep-menu branch + mouse-hover→selection mapping** — residual from the Zed pass; affects resolve-trigger completeness only (`../24-completion-architecture.md:621-623`).

## 4. Over-engineering check

- **Layered classifier depth:** three tests + dispatch is earned (nesting audit P0–P4, mdoxide precedent); the fourth "layer" (L0) is a contract line — cut the clothing, keep the rule (ST-A8).
- **Config-derived triggers:** the one true over-build — machinery for flags whose consumers are resolved (22, 17) or non-existent (21-Phase-2 `(`/`"`). Hardcode (ST-A1).
- **Classifier seam (ticket 34):** enum seam = cheap, keep; span-containment API design = premature until ticket 11, restrain (ST-A8).
- **Global recency layer, tries, in-handler rayon, incremental filtering:** all correctly absent (ST-A10) — restraint to record, not gaps to fill.

## 5. Stance

**All six recommendations survive the aggression pass unchanged in direction — nothing needs to be bolder; one (Q2's derivation machinery) needs to be smaller, and two (Q6's counterweights, Q4's degradation story) need their existing teeth stated explicitly rather than left in footnote units.**
