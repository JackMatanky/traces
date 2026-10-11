# 24 stress test — Q4 snippet/C1 range & Q5 frontmatter `:` split

Stress test, 2026-10-07, pre-grilling. Tests draft recommendations Q4 (wikilink accept,
zero-snippets option B) and Q5 (frontmatter `:` = keys-before/values-after split) against resolved
tickets 15/16/19/20/23, the ticket-24 consolidation and its units, and internal consistency.
Citation shorthand (`snippet:356`, `dispatch:279`, `capability:411`, `item-shape:616`, `zed:320`)
resolves via the table at `research/24-completion-architecture.md:10-25`.

## Findings — ST-S* (Q4, snippet/C1)

### ST-S1 — severity HIGH. (B)'s unpaired-state rationale is factually broken: "emit `[[T]]` and the user types `|`" types the pipe OUTSIDE the link.

Claim: in state A1 (`[[proj`, no trailing `]]`), (B)'s plain insert `[[T]]` leaves the cursor after `]]`; typing `|` then yields `[[T]]|` — the exact residue §4.3 forbids. The draft's "Obsidian/zk muscle memory" claim only holds when the cursor is *before* `]]`, which the A1 branch does not deliver. Unit 5 already proved this: "Do **not** emit `[[T|]]` as the fallback — with the cursor after `]]` that `|` is unreachable residue" (`snippet:379-382`), P1 = `[[T]]` (`snippet:303-310`), row A1 delta "plain loses the slot; leaves an unreachable `|`" (`snippet:352`). The ticket's own motivating example is "cursor left inside `[[|]]`" (`issues/24-completion-architecture.md:14`) — under (B) that example is met **only in the pre-paired state** (row C1, `snippet:356`).

Suggested change: (B) must state its A1 branch honestly and pick one: (a) `[[T]]` + document that alias creation costs `Left Left |` (drop the muscle-memory claim); or (b) `[[T|` (unterminated) + "user types alias then `]]`" (non-alias users must backspace the `|` — a real trade, state it); and record explicitly that in unpaired editors (B) does **not** deliver the ticket's example. Do not ship the recommendation with the current sentence.

### ST-S2 — severity HIGH. (B)'s headline rationale ("identical on every client regardless of snippetSupport") has no referent among the primary target editors — and the state where (B) costs anything is exactly the primary PKM demographic's default.

Claim: `snippetSupport` is verified **true** for VS Code, Neovim core (with real `vim.snippet` expansion — "trustworthy today"), nvim-cmp (override), Zed (`zed:320`), and Helix (config, default on) — `capability:411,434-436`. The gated-off population is eglot/Emacs and a reconfigured Helix. Meanwhile state A1 (no auto-paired `]]`) is the stock-Neovim default (no pair auto-insert), and ticket 15 already names "Neovim — the primary PKM demographic" (`issues/15-links-and-references-model.md:116`). So (B) withdraws the ticket's own example from the primary demographic to buy behavioral determinism for (almost) nobody, while (A) (range through `]]` + gated snippet, `snippet:374-382`, unit 3's paired row `item-shape:616`) delivers `[[T|${1:}]]$0` in *every* state to *every* verified client. Also note (B) *beats* (A)'s plain fallback in the paired state (`[[T|` lands in the slot vs `[[T]]` after) — that is (B)'s one real, snippet-independent win; it does not need zero snippets to keep it if (A) makes its snippet-off track emit `[[T|` when the doc's `]]` is in range (state-dependent plain track — cheap).

Suggested change: put the capability matrix (`capability:411`) and the A1-is-Neovim point on the grilling agenda next to the recommendation. If the stance is "capable over cautious", the stronger default is (A) — or (A) with the state-aware plain track above — not (B). Keep (B) only if the room explicitly accepts the stock-Neovim A1 loss.

### ST-S3 — severity MED. The draft's claimed collateral savings (C13 kind flip, S-18 kind gate, R7 baked prefixes) are already the baseline, not savings created by (B).

Claim: C13's snippet body is already "not recommended for v1… defer to a later ticket" (`snippet:391-406`), and S-18 is already "moot in v1 because no item uses kind 15" (`snippet:484-485`). The C13/S-18/R7 chain is *conditional on adopting C13*, which neither (A) nor (B) does — so collapsing it is not an argument for (B). What (B) actually saves vs (A): one snippet body + escaping arm + the Q8 cross-parser acceptance strings (`snippet:522-525`) — against unit 5's already-complete contract S-13..S-19 (`snippet:452-489`). That is ~one match arm plus one test. The "Consequence:" sentence in the draft overclaims and should be rewritten to the real delta.

Suggested change: replace the Consequence sentence with: "(B) defers S-15's C1 snippet body and Q8's cross-parser test to the first future snippet (C13 or a scaffold); the C13/S-18/R7 chain stays deferred either way."

### ST-S4 — severity MED. Neither option handles `[[proj|di]]` with the cursor in the *target* — and C1's current scope includes that state.

Claim: dispatch scopes C1 to "cursor in target" (`dispatch:132`) — which includes `[[proj|di]]` with the cursor between `proj` and `|`. (A)'s paired range "extends past the cursor to cover the closing token" (`item-shape:616`) and would eat `|di` (row B applied to the D1 document, `snippet:354,357`); (B)'s two-branch newText produces either `[[T]]|di]]` or `[[T||di]]` — both broken Markdown. Markdown Oxide completes this state too (`snippet:387-389`, Q3). This is an open hole in the research, not just in the draft.

Suggested change: add a P3.1 ladder guard for whichever option wins: if a `|` lies between the cursor and the link's closing `]]`, either **decline** (defensible — rare state) or extend the range through `]]` preserving the display (row D1's plain shape `[[T|di]]`, `snippet:357`). Write the guard into the chosen option's contract and hand it to 15 with the range decision.

### ST-S5 — severity MED. The entire cost distribution of Q4 turns on bracket auto-pair defaults, which no unit verified.

Claim: (B)'s cost is concentrated 100% in A1; (A)'s cost is uniform. Whether A1 or B1 is the *default* in VS Code Markdown (`editor.autoClosingBrackets = languageDefined`), Zed (`auto_pair`, default true — likely but unverified in the units), stock Neovim (no pair auto-insert), and Helix decides the real-world bill. The units contain a client-behavior verification pass for fan-out/resolve (`zed`) but none for auto-pairing; Q10's "plain ends at the cursor" scope note (`snippet:528-531`) is the closest analogue.

Suggested change: make a 5-minute empirical auto-pair check across the four primaries an explicit pre-grilling/implementation acceptance item (same class as the multi-cursor matrix, `snippet:516-521`), and require the Q4 decision record to cite the observed state distribution.

### ST-S6 — severity LOW. State-walk summary: plain text does **not** achieve cursor placement in all states; the alias-continuation state is the one place (B) is strictly nicer.

Claim: B1 (pre-paired) — plain `[[T|` lands the cursor in the alias slot, equal to (A)'s snippet (`snippet:356`); A1 — see ST-S1 (not achieved); D1/display continuation — Q3 territory, currently declines (`snippet:504-506`); post-accept alias typing under (B) is free word-char typing in the slot (no request; `|` itself sends nothing on Zed, `zed` Q3 via `research/24:600-605`). Also Q6 (`[[T|]]` as a valid terminal rendering, `snippet:513-515`) leans "valid" for Traces' resolver — 15 rejects empty *targets*, not empty aliases (`issues/15:52`, `src/note/links.rs:83-85` cited there) — which slightly favors keeping a `|`-ending plain text in the paired state.

Suggested change: fold the state table (A1/B1/D1/alias-continuation) into the Q4 decision record verbatim; resolve Q6 as part of it since (B)'s paired branch depends on `[[T|]]` being acceptable if abandoned mid-edit.

### HYBRID question — answer: the proposed hybrid (same range, snippet behind the gate) is strictly a non-improvement; the only meaningful hybrid *is* (A).

Claim: with the cursor-stopping range, a snippet `[[T|${1:}` has delta "none" over plain `[[T|` — unit 5's own row C1 says so (`snippet:356`). A hybrid that *also* switches range by capability = (A) with a capability-dependent range fork (classifier seam muddied — range is part of the dispatcher's output contract, `research/24:504-505`; better to keep the range fixed and make the *plain track* state-aware, per ST-S2). The C13/S-18/R7 "chain cost" the draft attributes to any snippet path is already deferred (ST-S3). So: don't design a third option; decide (A) vs (B) on the client-capability evidence (ST-S2) and the A1-branch fix (ST-S1).

## Findings — ST-F* (Q5, frontmatter `:`)

### ST-F1 — severity HIGH. "AFTER `:` → values owned by ticket 20" contradicts the consolidation's own P4 ladder and ticket 16's Answer: value positions have four owners, not one.

Claim: P4 already routes "tag-list value → C5" (`research/24:116`, C5 owner = 16, `dispatch:137`; 16's Answer: "YAML Frontmatter — Triggered at word boundaries inside frontmatter tag fields (`tags`, `tag`, `keywords`)", `issues/16-tags-and-hierarchical-tags.md:45-47`). C7 (schema values: Select/Boolean/Date — `research/20:289-298`) and C17 (file/class values, `dispatch:145`) are 20's. Inline-field dates in value-ish slots are 23's by explicit proof ("closes a gap 19 (keys only) and 20 (frontmatter only) provably don't cover", `issues/23-task-and-pkm-semantics.md:27`). Wikilink targets in YAML values are currently **unowned** (ST-F4). A Q5 sentence that assigns all post-colon values to 20 overwrites 16's locked ownership and P4 as written.

Suggested change: reword Q5 to: "before first `:` on a key line → C6 keys (19's three layers); after `:` → **value region**; L3 routes by key shape + line predicate: tag-list key → C5 (16), schema-bound key → C7/C17 (20), else decline." Keep "C6/C7 ownership recorded 19-then-20" but scope it to C6/C7 proper, not "values".

### ST-F2 — severity MED-HIGH. 19's `:` regex is a trigger-moment check; as the classifier's after-key predicate it dies on the first typed value character and rejects nested/sequence/quoted keys.

Claim: 19's rule is "`:` trigger: must follow a valid YAML key pattern (`^[a-zA-Z_-][a-zA-Z0-9_-]*\s*$`) … in frontmatter region" (`issues/19-frontmatter-and-inline-field-intelligence.md:80-84`) — validated at the instant `:` is typed. Every *sustained* value position (`status: be|` — cursor after typed text) fails that regex, as do indented nested-map keys (`  child: ` — `^` anchors at line start), sequence-of-map items (`- key: ` — the space after `-` breaks `\s*$`), and quoted keys (`"my key": `). Under a literal reading, value completion works only at the `:` keystroke and nowhere else, and never inside nested structures — frontmatter objects are a real model shape (`NoteFieldValue::Object`). dispatch Q3 is aware of the ambiguity but leaves the predicate undefined ("Classifier emits 'after-key' position only", `dispatch:279-281`).

Suggested change: define the predicate as a position property, not a trigger regex: "line (within the frontmatter region) has a first `:` whose key part matches the bare-word pattern, and the cursor is anywhere after it" — re-derived on every request (Invoked/word-char included, per `research/24:134-137`), with explicit stances for indented keys, `- key:` sequence items, and quoted keys (work or documented-decline; don't leave them to a regex accident). Carry the restated predicate back to 19 as an amendment to `issues/19:80-84`.

### ST-F3 — severity MED. "BEFORE the first `:` on a key line" has no positive line-shape guards: comments, block-scalar bodies, and quoted continuation lines all have "no colon yet" and would get key items.

Claim: a YAML comment line (`# see also:` typed mid-word → no colon yet), a multi-line block-scalar/quoted value continuation line, and an indented body line inside a folded scalar all satisfy "before the first `:`" naively; `#`-prefixed lines are excluded only if 19's bare-word guard is applied to *this* branch too, which the Q5 draft text does not say. 19's scanner handles full YAML structure (block scalars, anchors) for *spans* (`issues/19:36`) but the completion classifier is O(line) lexical (`dispatch` §7 timing split); nothing currently excludes scalar interiors. 19's own `CodeRegion`-first veto (`issues/19:83`) does not cover YAML scalars.

Suggested change: state the guards in Q5: bare-word guard on the before-branch; `#`-comment exclusion; and a block-scalar/quoted-continuation exclusion whose mechanism is named — reuse 19's per-refresh noyalib span scan to mark scalar line ranges in the memoized L1 region table (built per `(uri, version)`, `dispatch` §7), never a per-request YAML parse — with the honest fallback "until then, rare false positives on scalar interior lines" if the scan isn't wired yet.

### ST-F4 — severity MED. The clean split silently drops wikilink completion inside frontmatter values (`related: [[foo`) — a flow Obsidian users have and Traces's model can serve.

Claim: P1 makes frontmatter a mutually exclusive region ("frontmatter → YAML world", `research/24:87-90`); P4 ends "else decline" (`research/24:116`); C7's value table (20) has no Link-typed row (`research/20:289-298` — Select/Boolean/Date/File/Input only); 15 owns wikilink completion but its predicates are body-shaped (`dispatch:132`). Traces already parses `NoteFieldValue::Link` and 19 already gives link values the same hover/def as body links (`issues/19:74-76`) — completion is the one operation left unowned. This is a capability miss invisible in the keys-vs-values framing.

Suggested change: add a P4 rung: "`[[` inside a frontmatter value → C1's completer (15) with a frontmatter-local range" — or, if deliberately deferred, record "frontmatter link-value completion: declined in v1, follow-up to 15" in Q5 so it is a decision, not an accident.

### ST-F5 — severity MED. Value completion's availability rides ticket 20, which is **not** a blocker of 24 — and unbound notes have no value owner at all.

Claim: 24 is "Blocked by: 15, 16, 19, 21, 22, 23" (`issues/24-completion-architecture.md:4`) — 20 is not among them (20 is blocked by 19, `issues/20:5`). At 24-shipping time, the only value completions guaranteed present are C5 (16, tag fields) and C10 (23, inline only); C7/C17 are empty routes until 20's completion lands. Separately, 20's completion requires a resolved schema (every value row keys off `SchemaFieldDef`, `research/20:289-298`); unbound notes (no fileClass) get key completion via 19's layers 2–3 (`issues/19:62-64`) but **no** value completion from anyone — 19 designs keys only, and inventing inferred-value completion would exceed both tickets.

Suggested change: Q5 records two explicit lines: (1) sequencing — C7/C17 ship with 20's implementation; 24's dispatch returns an empty (not erroring) list until then, degradation-shaped per `dispatch` degraded-modes; (2) scope — unbound-note value positions decline, matching 19's keys-only design and 20's schema-required design; no inferred-value completion in v1.

### ST-F6 — severity MED. "Classifier emits region + after-key position only" under-specifies the key handoff, and 20's key-position row overlaps 19's layer 1.

Claim: 20's own algorithm needs the key ("look up key at current line", research/20 §7.2 step 3) and its completion table claims a key row ("Field names not already present", `research/20:283`) that is a strict subset of 19's layer-1 schema fields (`issues/19:42-48`). If the classifier emits only a boolean after-key flag, key extraction is duplicated between dispatch and 20 with no owner; if it emits the key, the slice's grammar (first `:` on line; quotes; trailing `# comment` after a value; duplicate keys) must be defined. "Keyed by the just-typed key" in the draft assumes this without assigning it.

Suggested change: specify the emission as `AfterKey { key: &str }` (or the key span) produced by dispatch's O(line) L2 step, grammar = first-`:` split + bare-word/quote rules, consumed by 20 and C5; and add to §8's correction list: "20's key-position completion row is subsumed by 19's three-layer key completion (layer 1 = schema fields); 20 contributes value positions only."

### ST-F7 — severity LOW. Region precedence between frontmatter and `{{ }}` depth>0 is unpinned, and template files put `{{ }}` in frontmatter.

Claim: P1 lists frontmatter and depth>0 as "mutually exclusive" and parenthetically orders frontmatter first (`research/24:87-90`) but never states the winner when a frontmatter value contains `{{ … }}` (common in template files, per 22's domain). Frontmatter-wins makes C15/C16 unreachable in frontmatter; depth-wins leaves YAML structure inside the expression undefined. Small, but Q5 is the moment to pin it.

Suggested change: one line in Q5: "frontmatter region wins over depth>0; `{{ }}` inside frontmatter values is YAML text (no template completion in v1)" — or the reverse, but stated.

### ST-F8 — severity LOW. 20's completion table still says `jiff` for Date values while 20's locked decision says chrono, syntax-only.

Claim: `issues/20:20` (Q16) locks "Date = syntax validation only (ISO-8601 parseability)… Use existing chrono parser, not jiff"; the completion tables at `research/20:296` and research/20 §7.2 still say "Schema format + `jiff`". Q5's handoff to 20 should carry the correction so the value completer doesn't reintroduce a dropped dependency.

Suggested change: one-line amendment to 20's research tables (chrono; format-aware *placeholder* only, no format validation on completion).

## Internal consistency checks (asked, answered)

- **(B) vs §0 row 4 / §9.1 / the ticket text**: no contradiction with the *research* — §0 row 4 itself says "Open: the C1 range choice (§9.1) can reduce this to zero snippets" (`research/24:51`), and §9.1 spells out both range options (`research/24:566-582`). (B) *selects* an option the research left open; it amends §0 row 4's "one v1 snippet" recommendation, §4's inventory (`research/24:391-392`), S-15/S-16 (`snippet:465-475`), and unit 3's paired-region assumption (`item-shape:616`, §6 table rows 2–3). It *partially* sidesteps the ticket's own example (`issues/24:14`) — met in B1, not in A1 (ST-S1) — which the decision record must state rather than paper over.
- **Q5 vs Q1's L1 step**: consistent — Q5 is exactly L2 (after-key/key-shape predicates) + L3 (route) refinement of P4, matching the "classifier emits 'after-key' position only" seam (`dispatch:279-281`). The only conflict is Q5's ownership *phrasing* (ST-F1) and the unguarded predicates (ST-F2/F3).
- **(B) vs §9.1 "highest-leverage decision"** (`snippet:495-498`): consistent; this stress test is that decision's evidence, and the evidence cuts against the draft's rationale (ST-S2).

## Handoff — what 15 / 19 / 20 / 16 / 29 must receive back (extends §8/§10)

- **15**: the C1 range decision + the chosen A1 branch definition (§10 already owes this); the `|`-between-cursor-and-`]` guard (ST-S4); ST-F4's frontmatter link-value rung or its explicit deferral; already-owed §8.6 corrections stand.
- **19**: restated `:` predicate as a sustained position property with nesting stances (ST-F2); bare-word/comment/block-scalar guards on the before-branch (ST-F3); the L1-memo scalar-exclusion mechanism (ST-F3); confirmation that its three-layer key completion owns C6 exclusively, values deferred (ST-F6).
- **20**: value ownership enumerated — Select/Boolean/Date/File/class only; tag-list values belong to 16, wikilink values to 15 or decline (ST-F1/F4); unbound-note values decline (ST-F5); key-position row subsumed by 19 (ST-F6); chrono-not-jiff (ST-F8); plus the already-listed §8.1 (cap-conditional `isIncomplete`) and §8.5 (kind clamp).
- **16**: C5's entry predicate is supplied by 24's classifier (after-key position on tag-list keys); "triggered at word boundaries" (`issues/16:46`) needs the same Invoked/word-char re-query support §9.4 already flags.
- **29**: `capability:68`'s example column ("ticket 24's own example …, 19's `key: value`, 23's `- [ ] `") is pre-unit-5 framing and is wrong (only C1 ever needed a snippet) and dead under (B); C-N3's gate stays but is v1-inert if (B) wins. C1 range decision also goes to the item-shape contract (paired-row amendment, `item-shape:616`).

## Verdict per decision

- **Q4 (C1 range / zero snippets)**: **weaken** the draft as written — the "identical regardless of snippetSupport" rationale is refuted by the verified capability matrix (all primary clients advertise snippets, ST-S2) and the A1 "user types `|`" flow is broken (ST-S1); under a capability-over-caution stance the stronger recommendation is (A) (paired range + gated snippet, optionally with a state-aware plain track), pending a 5-minute auto-pair check (ST-S5) — (B) survives only if the room consciously accepts the stock-Neovim A1 loss and fixes the A1 branch.
- **Q5 (frontmatter `:` split)**: **keep** the keys-before/values-after split (it correctly resolves dispatch Q3 and 19's regex ambiguity) but **strengthen** the wording — values are a region with four owners (16/20/15/decline), not "owned by 20" (ST-F1); the after-key predicate must be a sustained position property, not 19's trigger-moment regex (ST-F2); guards for comments/indent/sequence/block-scalars and the key-text handoff must be named (ST-F2/F3/F6).
