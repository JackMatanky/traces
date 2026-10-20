# Rust-Design Skill Revision (Token Reduction) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking; only task-item lines carry the `- [ ]` marker (headers and prose stay unboxed).

**Goal:** Cut the `rust-design` skill's token cost (SKILL.md ≤1100 words from 1633; full corpus ≤5200 words from 6505 (original target, later amended to ≤5350, see E2 note)) with zero capability loss, proven by before/after subagent scenario tests, and add a `grill-me` follow-up offer for all modes.

**Architecture:** RED first: run three capability scenarios against the current skill and record an oracle (assertions + word counts + files consulted). GREEN: apply Tier A cuts (information-preserving relocations), then Tier B cuts (cross-file dedup with REQUIRED pointer hardening). Re-run the same scenarios; any capability regression triggers REFACTOR (restore or re-harden, re-test).

**Tech Stack:** Markdown skill docs under `.agents/skills/rust-design/`, `uv` scripts (`measure.py`/`diff.py`), opencode subagents for scenarios, `wc -w`/`grep` for gates.

**Spec:** Findings + risk tiers from this session's analysis (Tier A = near-certain-safe relocations; Tier B = behavior-affecting dedup requiring tests).

---

## Ground Rules (apply to every task)

- **Iron Law (writing-skills):** No skill edit happens before Tasks 1 through 5 (RED) complete. Editing earlier = delete the edit and restart.
- **Scope:** `git add` only paths listed in each task's commit step. Never stage pre-existing dirty files (`.scratch/md-pkm-lsp/*` is unrelated work-in-progress).
- **Test oracle:** Assertions C1 through C7 and gates E1/E2 (defined in Task 1) are the definition of "capability preserved." A failing C-assertion after an edit = regression, fix before moving on.
- Word-count baseline measured in Task 1: `SKILL.md = 1633`, corpus (6 files) `= 6505`.
- Baseline S3 keep-commits (sanctioned exception to the Scope rule): the baseline Implement scenario (Task 4) ran under the skill's keep-commit rules ("commit wins") and produced commits `c22f33fd`, `e5514607`, `5b509c5f`, and `7dfc2e1f`, touching `src/dirtree.rs`, `src/config/discovery.rs`, `src/file_tracker.rs`, `src/index/service.rs`, `src/schema/service.rs`, `src/template/loader.rs`, and `tests/baselines/pub-tokens.txt`. Count correction (second review): this batch is exactly 4 commits (`git rev-list --count 1fd11d97..7dfc2e1f` = 4); `e5514607` is trial-2's keep cherry-picked into the batch (tree- and message-identical to its trial commit `7590b0b2`), so no fifth commit was folded in. These are intentional branch content (already disclosed in the results file); the Scope ground rule's `git add` restriction governs this plan's own task commits, not sanctioned scenario keep-commits.

## File Structure

- Modify: `.agents/skills/rust-design/SKILL.md` (primary token target; loads on every trigger)
- Modify: `.agents/skills/rust-design/DISCOVERY.md` (gains deferral enum; TOC out; lenses → table; LaTeX → plain)
- Modify: `.agents/skills/rust-design/TOOLING.md` (TOC out; tier ladder → list)
- Modify: `.agents/skills/rust-design/DEEPENING.md` (TOC out; LaTeX → plain)
- Modify: `.agents/skills/rust-design/MODELING.md` (TOC out; §8 trimmed; LaTeX → plain)
- Modify: `.agents/skills/rust-design/METRICS.md` (TOC out; LaTeX → plain)
- Create: `.agents/skills/rust-design/report-template.md` (report schema relocated from SKILL.md)
- Evidence (not skill content): `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md` (oracle + scenario results)
- Cleanup: delete probe artifact `.design/plan-probe-baseline.json` created during planning

---

## Phase RED: Baseline Tests (Tasks 1 through 5)

### Task 1: Freeze efficiency baseline

**Files:**
- Modify: `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md` (create)

- [ ] **Step 1: Delete planning probe artifact**

```bash
rm -f .design/plan-probe-baseline.json
```

- [ ] **Step 2: Record baseline word counts**

Run: `wc -w .agents/skills/rust-design/*.md`
Expected: DEEPENING 925, DISCOVERY 1006, METRICS 1037, MODELING 1008, SKILL 1633, TOOLING 896, total 6505 (±10).

- [ ] **Step 3: Write the baseline record**

Create `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md`:

```markdown
# Rust-Design Skill Revision: Test Oracle & Baseline

Scenario results are appended under "## Scenario results" (RED baseline) and "## Post-Edit Results" (GREEN) below.

## Efficiency gates (RED now, GREEN after edits)
- E1: wc -w .agents/skills/rust-design/SKILL.md <= 1100 (baseline: 1633 -> FAILS now = RED)
- E2: wc -w .agents/skills/rust-design/*.md incl. future report-template.md <= 5350 (baseline: 6505 measured on the 6 pre-existing files -> FAILS now = RED) (amended from <=5200, see results file E2 note)

## Capability assertions (must pass BEFORE and AFTER edits)
- C1: Deliverables confined to .design/<date>-<mode>-<slug>/ (plus sanctioned keep-commits for Implement runs). Check: `git status --short` shows no new paths outside the scenario's .design/ dir and the sanctioned keep-commits, and `ls .design/` shows exactly one directory per scenario run so far (this run's dir plus prior scenario dirs, no stray dirs). (Clarified during RED.)
- C2: 10 scout files present in .design/.../scouts/ (Review) and no scout claims an executed build/test run. Check: ``grep -E 'Ran `cargo (test|build)`|Executed `cargo|I ran cargo (test|build)' .design/<dir>/scouts/*.md`` finds nothing (negated mentions such as "did not run cargo test" are fine).
- C3: ledger.json exists, sorted by descending EAV; every finding has a disposition; deferrals use only DEFERRED_TOOL_UNAVAILABLE | DEFERRED_OUT_OF_SCOPE | DEFERRED_HIGH_TRACE_COST, total <= 3.
- C4: report.md contains all 7 mandated section headings (## 1 through ## 7).
- C5: Scenario's "Files Consulted" includes the mode's required reference doc (Review: DISCOVERY.md; Model: MODELING.md; Implement: DEEPENING.md and METRICS.md).
- C6: "Words consulted" (wc -w of files listed in Files Consulted) <= baseline value recorded per scenario.
- C7 (Implement runs only): patch.diff status reported, every trial logged kept or reverted with a reason, and `git status --short` shows no broken scaffolding left behind.

## Scenario results
(to be filled per scenario)
```

- [ ] **Step 4: Commit the oracle**

```bash
git add docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md
git commit -m "test(rust-design): record revision test oracle and efficiency baseline"
```

### Task 2: Baseline scenario S1 (Review mode)

**Files:**
- Modify: `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md`
- Evidence dir (auto): `.design/<date>-review-*`

- [ ] **Step 1: Dispatch the Review scenario subagent**

Dispatch one `general` subagent with this prompt verbatim:

```
Use the rust-design skill (load it via the skill tool) to run a Review-mode
architectural review with target path `src/field.rs`. Follow the skill's
workflow exactly as written. All deliverables go under `.design/` per the
skill's Deliverable Containment section.

In your final message to me, include:
1. The exact `.design/` directory path you created.
2. A `## Files Consulted` heading listing every skill reference document you
   actually opened (name files you read, not files you intended to read).
3. Every deferred finding and the deferral reason string you used.
```

Note: the subagent will dispatch 10 scouts; allow up to ~15 minutes.

- [ ] **Step 2: Audit against C1 through C6**

Run (replace path with the reported dir):

```bash
ls -R .design/<review-dir>/
git status --short
grep -c '^## ' .design/<review-dir>/report.md
grep -o 'DEFERRED_[A-Z_]*' .design/<review-dir>/ledger.json | sort | uniq -c
python3 -c "import json;d=json.load(open('.design/<review-dir>/ledger.json'));c=d.get('deferral_count');n=len(d.get('deferrals') or d.get('closed_deferrals') or []);print('deferrals',n,'count',c,'ok',n<=3 and (c is None or c==n))"
grep -E 'Ran `cargo (test|build)`|Executed `cargo|I ran cargo (test|build)' .design/<review-dir>/scouts/*.md
```

Expected: `git status --short` shows no new paths outside `.design/` and `ls .design/` shows exactly one dir per scenario run so far (`<review-dir>/` plus any prior scenario dirs) (C1); report has 7 `##` headings (C4); any deferral strings are `DEFERRED_TOOL_UNAVAILABLE|DEFERRED_OUT_OF_SCOPE|DEFERRED_HIGH_TRACE_COST` with total ≤3 (C3; the pinned python count reads `deferrals`/`closed_deferrals` length and cross-checks `deferral_count`); pinned scout grep prints no matches (C2; negated mentions like "did not run cargo test" are not violations).

- [ ] **Step 3: Record S1 results**

Append to `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md`: scenario path, C1 through C6 pass/fail, `Files Consulted`, and `wc -w` of those files (the C6 baseline number).

### Task 3: Baseline scenario S2 (Model mode)

**Files:**
- Modify: `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md`

- [ ] **Step 1: Dispatch the Model scenario subagent**

Dispatch one `general` subagent with this prompt verbatim:

```
Use the rust-design skill (load it via the skill tool) in Model mode for this
request: "Design Rust types so invalid trace-field records cannot be
constructed, for the field-parsing domain in `src/field.rs`." Follow the
skill's workflow exactly as written. Deliverables go under `.design/`.

In your final message include:
1. The exact `.design/` directory path you created.
2. A `## Files Consulted` heading listing every skill reference document you
   actually opened.
3. Your proposed type definitions.
```

- [ ] **Step 2: Audit against C1, C5, C6**

Run: `ls .design/; git status --short`

Expected: `ls .design/` shows exactly one dir per scenario run so far (`<model-dir>/` plus prior scenario dirs); `git status --short` shows no new paths outside `.design/` (C1); `Files Consulted` includes `MODELING.md` (C5).

- [ ] **Step 3: Record S2 results**

Append path, C-pass/fail, Files Consulted + `wc -w` (C6 baseline) to `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md`.

### Task 4: Baseline scenario S3 (Implement/Steady mode)

**Files:**
- Modify: `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md`

- [ ] **Step 1: Dispatch the Implement scenario subagent**

Dispatch one `general` subagent with this prompt verbatim:

```
Use the rust-design skill (load it via the skill tool) in Implement/Steady
mode with target `src/dirtree.rs`: "deepen the seam so callers stop reaching
into internal helpers." Follow the skill's workflow exactly as written,
including its trial rules; deliverables go under `.design/`.

In your final message include:
1. The exact `.design/` path and whether patch.diff was produced.
2. A `## Files Consulted` heading listing every skill reference document you
   actually opened.
3. For each trial: kept or reverted, and the reason.
```

Note: this runs `cargo test` inside sandboxed trials; allow up to ~25 minutes.

- [ ] **Step 2: Audit against C1 through C7 (all applicable; trial discipline included)**

Run:

```bash
ls -la .design/<impl-dir>/
git status --short
ls .design/<impl-dir>/scouts/ | wc -l
grep -c '^## ' .design/<impl-dir>/report.md
grep -o 'DEFERRED_[A-Z_]*' .design/<impl-dir>/ledger.json | sort | uniq -c
python3 -c "import json;d=json.load(open('.design/<impl-dir>/ledger.json'));c=d.get('deferral_count');n=len(d.get('deferrals') or d.get('closed_deferrals') or []);print('deferrals',n,'count',c,'ok',n<=3 and (c is None or c==n))"
grep -E 'Ran `cargo (test|build)`|Executed `cargo|I ran cargo (test|build)' .design/<impl-dir>/scouts/*.md
```

Expected: only `.design/<impl-dir>/` created and worktree clean of broken scaffolding (C1, C7); 10 scout files and pinned C2 grep empty (C2); report has 7 `##` headings (C4); deferral strings within the closed enum, total ≤3 (C3; pinned python count reads `deferrals`/`closed_deferrals` length, cross-checks `deferral_count`); `Files Consulted` includes `DEEPENING.md` and `METRICS.md` (C5); patch.diff status reported and every trial logged kept or reverted with a reason (C7).

- [ ] **Step 3: Record S3 results**

Append path, C1-C7 pass/fail, Files Consulted + `wc -w` (C6 baseline) to `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md`.

### Task 5: RED verdict

- [ ] **Step 1: Confirm RED state**

Run: `grep -E 'E1|E2' docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md`

Expected: E1 and E2 marked FAIL against baseline numbers (1633 / 6505), and every assertion applicable to each scenario recorded as PASS (C2, C3, C4: S1 and S3 only; C7: S3 only). If any C-assertion failed in baseline, fix the scenario prompt or environment first; the current skill is the capability reference; its behavior is what we preserve.

- [ ] **Step 2: Commit baseline evidence**

```bash
git add docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md
git commit -m "test(rust-design): baseline scenario results for skill revision (RED)"
```

Do not `git add .design/`: it is gitignored (`.gitignore:50`) by design; scenario artifacts stay local.

---

## Phase GREEN Tier A: Information-Preserving Relocations (Tasks 6 through 10)

Run `wc -w .agents/skills/rust-design/SKILL.md` after each task; SKILL should reach ≤1250 by Task 10.

### Task 6: Delete the internal workflow checklist duplicate

**Files:**
- Modify: `.agents/skills/rust-design/SKILL.md`

- [ ] **Step 1: Remove the checklist section**

Edit `SKILL.md`: delete this entire block (it duplicates the Step-by-Step Process below it):

````markdown
---

## Interactive Workflow Checklist

Copy and maintain this checklist in the scratchpad across execution:

```markdown
### Workflow Execution Checklist
- [ ] 1. Establish Current Design & Baseline: Probe environment; run measure.py to freeze True Baseline.
- [ ] 2. Discover Hypotheses via 10-Lens Scouting: Dispatch 10 read-only scouts; populate and rank ledger by EAV.
- [ ] 3. Empirical Trial-and-Revert Search Loop: Prototype top bets in sandboxed trials; commit wins or revert to graveyard.
- [ ] 4. Model Design in Rust: Translate semantics into concrete types, parse-don't-validate, and typestates.
- [ ] 5. Verify Implementation: Run test suites, verify API stability, compute deltas via diff.py. (If tests fail, return to Step 3).
- [ ] 6. Boundary Verification & Grader Audit: Verify boundaries, run independent grader pass, emit report.md.
```

---
````

Replace with a single `---` separator so the file reads `...zero loose evaluation directories...` then `---` then `## Step-by-Step Process`.

- [ ] **Step 2: Verify**

Run: `grep -c 'Interactive Workflow Checklist' .agents/skills/rust-design/SKILL.md`
Expected: `0`

### Task 7: Relocate the report schema to report-template.md

**Files:**
- Create: `.agents/skills/rust-design/report-template.md`
- Modify: `.agents/skills/rust-design/SKILL.md`

- [ ] **Step 1: Create the template file**

Create `.agents/skills/rust-design/report-template.md` containing the schema verbatim from current `SKILL.md` (lines 154-194), retitled:

````markdown
# Architectural Report Template

Every execution writes `report.md` under these mandatory Markdown headings:

```markdown
# Architectural Report: [Scope / Target Name]

## 1. Executive Summary & Strategy
- Mode: [Review | Model | Implement] (Strategy: [Steady | Breakthrough])
- Primary Target: [Target Path or Module]
- Deliverable Path: `.design/YYYY-MM-DD-<mode>-<target-slug>/`
- Architectural Thesis: [Summary of recursive depth and knowledge transformation]

## 2. True Architectural Baseline vs. Final State
<!-- Embed output table directly from: uv run .../diff.py baseline.json final.json -->

## 3. Recursive Depth & Seam Decomposition
- Parent Module Simplification: [What vanished from parent obligations]
- Deep Child Modules Extracted: [Child responsibilities and local depth ratios]
- Depth Propagation Evidence: [Why the parent seam is now deeper]

## 4. Predictability & Layout Refactor
- Cohesion Fixes: [Resolved disjoint reference clusters in files]
- Eliminated Anti-Patterns: [Removed junk drawer files, deep path traversals]

## 5. Deletion Dividend Ledger
- Superseded Types & Functions: [Deleted symbols]
- Eliminated Adapters & Wrappers: [Collapsed layers]
- Removed Dependencies: [Eliminated crates]
- Net SLoC Removed: [Exact count from diff.py]

## 6. Architectural Graveyard (Tried & Rejected)
- Candidate: [Concept]
- Hypothesis: [Intended improvement]
- Rejection Evidence: [Concrete failure metrics or errors]

## 7. Verification & Grader Audit
- Seam Behavior Verification: [Tests executed through intended seams]
- Public API Stability: [Diff verified via cargo-public-api]
- Done Criteria Verification: [Audited by independent pass]
```
````

- [ ] **Step 2: Point Step 6 at the template and delete the inlined schema**

In `SKILL.md`, replace step 6 item 3:

old: `` 3. **Emit Architectural Report:** Compile `.design/YYYY-MM-DD-<mode>-<target-slug>/report.md` matching the schema below.``

new: `` 3. **Emit Architectural Report:** Compile `.design/YYYY-MM-DD-<mode>-<target-slug>/report.md` using [`report-template.md`](report-template.md).``

Then delete the entire trailing section from `## Architectural Report Schema` through end of file (starting at the `---` before it).

- [ ] **Step 3: Verify**

Run: `grep -c 'Architectural Report Schema' .agents/skills/rust-design/SKILL.md; grep -c 'report-template' .agents/skills/rust-design/SKILL.md`
Expected: `0` then `1`

### Task 8: Replace LaTeX math with plain notation (reference docs)

**Files:**
- Modify: `DISCOVERY.md`, `DEEPENING.md`, `MODELING.md`, `METRICS.md`

`SKILL.md`'s LaTeX is removed later in Task 12; skip it here.

- [ ] **Step 1: DISCOVERY.md**

Replace the display formula

```markdown
$$\text{EAV} = \frac{\text{Structural or Computational Gain (1–5)} \times \text{Confidence (0.0–1.0)}}{\text{Blast Radius (1–5)} + \text{Implementation Friction (1–5)}}$$
```

with:

```markdown
`EAV = (Structural/Computational Gain [1-5] × Confidence [0.0-1.0]) / (Blast Radius [1-5] + Implementation Friction [1-5])`
```

Replace `A candidate is consequential if $\text{EAV} \ge 1.0$, or if` with `A candidate is consequential if \`EAV >= 1.0\`, or if`.

- [ ] **Step 2: DEEPENING.md**

Replace `Expected Architectural Value $\text{EAV} \ge 1.0$ have been evaluated` with ``Expected Architectural Value `EAV >= 1.0` have been evaluated`` and `no new hypotheses above $\text{EAV} \ge 1.0$, terminate` with ``no new hypotheses above `EAV >= 1.0`, terminate``.

- [ ] **Step 3: MODELING.md**

Replace `($\text{ISR} = 0$)` with ``(`ISR = 0`)`` and `($\text{ITE} = 0$)` with ``(`ITE = 0`)``.

- [ ] **Step 4: METRICS.md**

Replace `$\le 500$ SLoC` with `` `<= 500` SLoC ``.

- [ ] **Step 5: Verify no LaTeX remains outside SKILL.md**

Run: `grep -l '\$\$' .agents/skills/rust-design/*.md; grep -c '\\text{' .agents/skills/rust-design/*.md`
Expected: no files with `$$`; `0` counts everywhere except possibly `SKILL.md` (cleared in Task 12).

### Task 9: TOOLING tier ladder → indented list; strip TOCs from all reference docs

**Files:**
- Modify: `TOOLING.md`, `DISCOVERY.md`, `DEEPENING.md`, `MODELING.md`, `METRICS.md`

- [ ] **Step 1: Replace the TOOLING ASCII ladder**

Delete the fenced ASCII diagram (the `+---...+` block containing Tier 1/2/3 boxes and arrows, ~24 lines) and replace with:

```markdown
1. **Tier 1: Automated Script (preferred):** `uv run .agents/skills/rust-design/scripts/probe.py` (probe), `measure.py <target> [out]`, `diff.py <base> <final>`.
2. **Tier 2: Configured environment tasks (fallback):** inspect `mise tasks`, `just --list`, `cargo make`, or `Makefile`; run configured aliases (`modules:tree`, `mess`, `crap`, `orphans`).
3. **Tier 3: Standard Cargo toolchain (universal minimal):** structure via `cargo check`, LSP references, manual inspection; quality via `cargo clippy --all-targets -- -D warnings`; testing via `cargo test`; dependencies via `cargo tree` / `cargo tree -d`. Mark unavailable metrics `not measured (tool unavailable)`.
```

- [ ] **Step 2: Delete the TOC table from each reference doc**

In each of `DISCOVERY.md`, `DEEPENING.md`, `MODELING.md`, `METRICS.md`, `TOOLING.md`, delete the table directly under the H1 (columns `Section | Focus / Key Concepts | Line Target` plus its separator row), leaving the H1 followed by `---`. Do not touch any other tables.

Example: `DISCOVERY.md` old (delete whole block):

```markdown
| Section | Focus / Key Concepts | Line Target |
| :--- | :--- | :--- |
| [1. Scouting Protocol & Rules](#1-scouting-protocol--rules) | Read-only scout isolation, execution constraints | Line 13 |
| [2. The 10 Architectural Lenses](#2-the-10-architectural-lenses) | 10 modular scout directives and focus targets | Line 23 |
| [3. The Indicator Hypothesis](#3-the-indicator-hypothesis) | Translating raw tool signals into domain questions (SSOT) | Line 89 |
| [4. Expected Architectural Value (EAV)](#4-expected-architectural-value-eav) | EAV formula, scoring factors, consequential triggers | Line 104 |
| [5. Candidate Output & Ledger Schema](#5-candidate-output--ledger-schema) | Scout reporting schema and `ledger.json` format | Line 124 |
```

The other four docs have the same 6 to 8 row shape; delete each identically.

- [ ] **Step 3: Verify**

Run: `grep -c 'Line Target' .agents/skills/rust-design/*.md; grep -c '^+' .agents/skills/rust-design/TOOLING.md`
Expected: `0` for Line Target everywhere; `0` box-drawing rows in TOOLING.md

### Task 10: Tier A checkpoint

- [ ] **Step 1: Word-count gate**

Run: `wc -w .agents/skills/rust-design/*.md`
Expected: SKILL.md ≤1250; corpus total lower than 6505.

- [ ] **Step 2: Commit Tier A**

```bash
git add .agents/skills/rust-design/
git commit -m "refactor(rust-design): relocate report schema, dedupe checklist, flatten diagrams and TOCs"
```

---

## Phase GREEN Tier B: Cross-File Dedup with Hardened Pointers (Tasks 11 through 16)

Every Tier B edit either **moves** content to its single home or replaces it with a `REQUIRED` pointer. No rule becomes homeless.

### Task 11: Move the Closed Deferral Enum into DISCOVERY.md

**Files:**
- Modify: `DISCOVERY.md`, `SKILL.md`

- [ ] **Step 1: Add the enum to DISCOVERY.md §5**

Append to the end of `DISCOVERY.md` §5 (after the "Store candidate hypotheses..." line):

```markdown
### Closed Deferral Reason Enum

Every deferred finding must use one of three variants, capped at 3 deferrals per review without explicit user approval:

- `DEFERRED_TOOL_UNAVAILABLE`: Required diagnostic binary is absent from host.
- `DEFERRED_OUT_OF_SCOPE`: Finding resides entirely outside requested target boundary.
- `DEFERRED_HIGH_TRACE_COST`: Verification requires exhaustive dynamic tracing exceeding turn boundaries.
```

- [ ] **Step 2: Replace SKILL.md Step 2 item 3 with a pointer**

old:

```markdown
3. **Closed Deferral Reason Enum:** Every deferred finding must use one of three enum variants, capped at 3 deferrals per review without explicit user approval:
   - `DEFERRED_TOOL_UNAVAILABLE`: Required diagnostic binary is absent from host.
   - `DEFERRED_OUT_OF_SCOPE`: Finding resides entirely outside requested target boundary.
   - `DEFERRED_HIGH_TRACE_COST`: Verification requires exhaustive dynamic tracing exceeding turn boundaries.
```

new:

```markdown
3. **Closed Deferral Enum:** **REQUIRED:** defer only with the three variants defined in [`DISCOVERY.md`](DISCOVERY.md) §5 (Closed Deferral Reason Enum); maximum 3 per review without explicit user approval.
```

### Task 12: Strip SKILL.md restated mechanics to REQUIRED pointers

**Files:**
- Modify: `SKILL.md`

- [ ] **Step 1: Compress Step 2 EAV block**

old:

```markdown
2. **Expected Architectural Value (EAV):** Rank candidate redesigns in `ledger.json` using:
   $$\text{EAV} = \frac{\text{Structural or Computational Gain (1–5)} \times \text{Confidence (0–1)}}{\text{Blast Radius (1–5)} + \text{Implementation Friction (1–5)}}$$
   A candidate is consequential if $\text{EAV} \ge 1.0$.
```

new:

```markdown
2. **Rank by Expected Architectural Value (EAV):** Score every candidate in `ledger.json` per [`DISCOVERY.md`](DISCOVERY.md) §4 (formula + consequential triggers). Consequential means `EAV >= 1.0` or any listed trigger.
```

- [ ] **Step 2: Compress Step 3 trial mechanics**

old (items 2 through 5 of Step 3):

```markdown
2. **Sandboxed Trial:** Create a clean worktree or branch (`improve/<run-id>-<trial-id>`). For Breakthrough bets, record the explicit drop condition before writing code.
3. **Iterate & Refine:** Apply candidate transformations. Limit iterative compilation attempts to at most 3 refinements per candidate.
4. **Commit or Revert Decision:**
   - *Keep Condition:* Tests pass, external API contracts hold, and the scoreboard demonstrates a positive delta without uncompensated regressions. Update `ledger.json` and advance the baseline.
   - *Revert Condition:* If tests fail or drop conditions trigger, roll back immediately with `git reset --hard` or worktree disposal. Log the trial record and rejection post-mortem to `.design/.../graveyard.md`.
5. **Termination Gates:** Stop when all high-EAV hypotheses are evaluated (Convergence), two consecutive trials fail (Plateau: trigger re-scouting), or budget is exhausted (Budget).
```

new:

```markdown
2. **Sandboxed Trial:** Work in a clean worktree/branch; for Breakthrough bets state the drop condition before editing.
3. **Iterate & Refine:** Apply candidate transformations within the attempt limit.
4. **Commit or Revert:** Keep only on green tests + stable public API + positive scoreboard delta; otherwise revert immediately and log the post-mortem to `.design/.../graveyard.md`.
5. **Termination Gates:** Convergence, Plateau (two consecutive failures → re-scout), or Budget; definitions in [`DEEPENING.md`](DEEPENING.md) §5.
```

And strengthen the Step 3 read-pointer, which currently reads exactly:

```markdown
Read [`DEEPENING.md`](DEEPENING.md) for trial mechanics, graveyard specifications, and termination gates.
```

Replace it with:

```markdown
**REQUIRED:** Read [`DEEPENING.md`](DEEPENING.md) §4 and §5 before the first trial; sandbox rules, drop conditions, attempt limits, rollback, graveyard format, and gate definitions live only there.
```

- [ ] **Step 3: Compress Step 4 to a summary + pointer**

old (the five bullets under `### 4. Model Design in Rust`):

```markdown
- Prefer concrete structs and functions before introducing traits or generics.
- Apply "parse, don't validate" to eliminate invalid states at construction ($\text{ISR} = 0$).
- Use typestates only when states are stable and transitions are central to correctness ($\text{ITE} = 0$).
- Maintain panic-free public seams: entry points return structured `Result<T, E>`.
- Enforce POLS: file layouts match domain ownership; eliminate utility "junk drawers".
```

new:

```markdown
**REQUIRED:** Read [`MODELING.md`](MODELING.md) before proposing any type. In one line: concrete types first, invalid states unconstructable (`ISR = 0`), typestates only when transitions are correctness-critical (`ITE = 0`), panic-free `Result` seams, POLS layout with no junk drawers.
```

- [ ] **Step 4: Compress Step 5 anti-gaming restatement**

old (item 4 of Step 5):

```markdown
4. **Anti-Gaming Guardrails:**
   - *Comment Preservation Rule:* Doc comments (`///`, `//!`) and explanations must not be stripped or compressed to inflate line reduction counts.
   - *Semantic Deletion Dividend:* The deletion dividend evaluates removal of architectural complexity (dead types, obsolete traits, collapsed wrappers), not prose.
```

new:

```markdown
4. **Anti-Gaming Guardrails:** **REQUIRED:** Read [`METRICS.md`](METRICS.md) §4 before scoring any delta; comment preservation and semantic deletion dividend rules live only there.
```

- [ ] **Step 5: Verify SKILL.md gate**

Run: `wc -w .agents/skills/rust-design/SKILL.md`
Expected: ≤1100 (E1 PASS). If over, next lever: trim prose in the Deliverable Containment comments (keep the tree itself).

### Task 13: Deduplicate MODELING.md against rust-skills

**Files:**
- Modify: `MODELING.md`

Risk note: this is the highest-risk Tier B cut; S2 (Model) re-run in Task 17 verifies C5 (MODELING.md still opened) plus C1/C6 (C2-C4 are scoped to Review+Implement only), and the controller additionally compares the delivered type definitions against the baseline run's output during GREEN review.

- [ ] **Step 1: Trim §8 to seam-level content with a lazy-load pointer**

old (`## 8. Error Boundaries & Panic-Free Seams` section body, all bullets):

```markdown
Error types are part of the interface because callers must act on failure modes:

- **Panic-Free Seams:** Public and cross-crate entry points must never expose unhandled `.unwrap()`, `.expect()`, or `panic!` calls. Errors crossing seams must be structured `Result<T, E>` domain enums (`thiserror`).
- **Domain Error Classification:** Hide internal child errors behind cohesive domain classifications rather than mechanically mirroring every internal error variant.
- **Panic Containment:** Seams hosting worker tasks or plugins must isolate thread panics (`std::panic::catch_unwind` or tokio task join handling) to prevent cascading process failure.
- **Contract Evolution (Expand-and-Contract):** Use `#[non_exhaustive]` on public enums so variant additions are backward-compatible; use builder patterns with defaults for struct parameter expansion; employ parallel deprecation (`#[deprecated]`) over breaking signature changes.
```

new:

```markdown
Error types are part of the interface because callers must act on failure modes:

- **Panic-Free Seams:** Public and cross-crate entry points must never expose unhandled `.unwrap()`, `.expect()`, or `panic!`. Errors crossing seams are structured `Result<T, E>` domain enums.
- **Domain Error Classification:** Hide internal child errors behind cohesive domain classifications rather than mirroring every internal variant.
- **Panic Containment:** Seams hosting worker tasks or plugins isolate thread panics (`catch_unwind` / tokio join handling).

Coding-level error and contract-evolution rules (`thiserror` usage, `#[non_exhaustive]`, expand-and-contract, deprecation) live in `rust-skills`; load it when implementing.
```

- [ ] **Step 2: Verify no other modeling content was cut**

Run: `grep -c '^## ' .agents/skills/rust-design/MODELING.md`
Expected: `8` (all eight sections retained).

### Task 14: Lens directives → single dispatch table

**Files:**
- Modify: `DISCOVERY.md`

- [ ] **Step 1: Replace the ten subsections**

Delete the ten `### Lens N: ...` subsections (each with Target/Inspect/Objective bullets) and replace with:

```markdown
## 2. The 10 Architectural Lenses

Dispatch parallel subagents, one row per scout; each row is a self-contained directive:

| # | Lens | Target | Inspect | Objective |
| :--- | :--- | :--- | :--- | :--- |
| 1 | Modern Rust & Upstream Capabilities | Upstream features that obsolete hand-rolled machinery | `std::sync::Exclusive`, modern GAT patterns, slice chunking, `portable_simd`, upstream crate capabilities | Replace custom utilities with std/compiler intrinsics |
| 2 | Data-Oriented Layout & Memory Footprint | Padding holes and cache misses | Field ordering, alignment padding, Struct-of-Arrays (SoA) opportunities, hot/cold field segregation | Maximize cache-line saturation, eliminate padding waste |
| 3 | Allocation, Lifecycles & Zero-Copy | Unnecessary heap allocations and defensive copying | Defensive `.clone()`, unbounded per-item allocation, missing arena/bump allocators, borrowed views or `Cow<'a, T>` | Eliminate runtime allocations on hot paths |
| 4 | Seam Placement & Knowledge Asymmetry | Callers knowing implementation mechanisms | Bypassed abstractions, direct field manipulation, leaked storage/serialization representations, loose visibility | Move knowledge behind deep, minimal interfaces |
| 5 | State & Invariant Ownership | Invariants split across callers or runtime-only | Invariants across multiple types, mutable state disconnected from logic, invalid states representable, runtime assertions → typestates | Enforce "parse, don't validate" (`ISR = 0`) |
| 6 | Boundary Crossings & Vertical Leakage | Lower-level details bubbling upward | Child error types in ancestor APIs, low-level config bubbling, sibling circular imports | Isolate internal churn across seams |
| 7 | Concurrency, I/O & Contention | Lock contention, blocking, unbounded queues | Coarse `Mutex` scopes, blocking on async reactors, serialization round-trips, `unbounded_channel()`, panic propagation across tasks | Bound resource consumption, isolate worker failures |
| 8 | Algorithmic Scaling & Hot-Path Complexity | Complexity and scaling bottlenecks | O(n^2) lookups, redundant hashing, repeated traversals, memoization or early-exit opportunities | Reduce big-O scaling on hot paths |
| 9 | Deletion Dividend & Layer Thinning | Redundant, obsolete, or pass-through structure | Forwarding wrappers, redundant facade traits, one-to-one adapters, dead conversions, compatibility scaffolding | Delete architectural complexity, thin layers |
| 10 | Checks That Can't Fail & Test Ergonomics | Fragile, tautological, or over-mocked tests | Default-state assertions, tautologies, over-mocking hiding regressions, missing error-path verification | Verify domain behavior through intended seams |
```

- [ ] **Step 2: Verify**

Run: `grep -c '^### Lens' .agents/skills/rust-design/DISCOVERY.md; grep -c 'portable_simd\|unbounded_channel\|parse, don' .agents/skills/rust-design/DISCOVERY.md`
Expected: `0` lens subsections; the inspect keywords still present (grep count >=2).

### Task 15: Add the mode-agnostic grilling follow-up offer

**Files:**
- Modify: `SKILL.md`

- [ ] **Step 1: Insert a new section after the Operational Modes block**

Insert this section between the Steady/Breakthrough bullet list and the `---` that precedes `## Deliverable Containment` (Step 6 alone is insufficient: Model mode runs only Step 4, so the offer must live outside the step list to cover all modes):

```markdown
---

## Grilling Follow-Up (All Modes)

After any run produces its deliverable (review report, type model, or verified patch), offer a `grill-me` session (user-invoked only; never auto-chain) to stress-test the design theses, top-ranked `ledger.json` items, or proposed types with the user.
```

- [ ] **Step 2: Verify**

Run: `grep -c 'grill-me' .agents/skills/rust-design/SKILL.md`
Expected: `1`

### Task 16: Tier B commit + static backstop

- [ ] **Step 1: Pointer-target backstop**

Run:

```bash
grep -o '](\([A-Za-z-]*\.md\))' .agents/skills/rust-design/SKILL.md | sort -u
ls .agents/skills/rust-design/{DISCOVERY,DEEPENING,MODELING,METRICS,TOOLING,report-template}.md
```

Expected: every `.md` referenced by SKILL.md exists in the listing (plus `report-template.md`).

- [ ] **Step 2: Rule-home spot check (no homeless rules)**

Run:

```bash
grep -c 'DEFERRED_TOOL_UNAVAILABLE' .agents/skills/rust-design/*.md
grep -c 'Comment Preservation Rule' .agents/skills/rust-design/*.md
grep -c 'improve/<run-id>-<trial-id>' .agents/skills/rust-design/*.md
grep -c 'Structural/Computational Gain' .agents/skills/rust-design/*.md
```

Expected: each pattern appears in exactly one file (deferrals → DISCOVERY.md; comment preservation → METRICS.md; worktree naming → TOOLING.md or DEEPENING.md, whichever still holds it; EAV formula → DISCOVERY.md).

- [ ] **Step 3: Commit Tier B**

```bash
git add .agents/skills/rust-design/
git commit -m "refactor(rust-design): single-source mechanics in reference docs with hardened pointers, lens table, grilling follow-up"
```

- [ ] **REFACTOR amendment (E2 remediation passes 1 through 3, retroactively authorized)**

Commit `cdb006bb` ("REFACTOR trims for E1/E2 gates") removed content beyond the levers literally specified in Tasks 9 through 16. The five cuts, hereby authorized so plan, results, and diff agree: (1) DEEPENING.md's seam diagram and blockquote; (2) METRICS.md §1's "Evaluation Purpose" column (13 rows); (3) two scoreboard example rows; (4) a DISCOVERY.md scout-isolation reword; (5) SKILL.md prose compressions: intro/description, Steady/Breakthrough bullets, grilling section (SKILL 1141 -> 1100). Together they brought the corpus from 5545 to 5326; the resulting corpus was then ratified by the user-approved E2 amendment from ≤5200 to ≤5350 (commit `9bbe59ca`).

---

## Phase GREEN Verification + REFACTOR (Tasks 17 through 20)

### Task 17: Re-run all three scenarios (GREEN)

**Files:**
- Modify: `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md`

- [ ] **Step 1: Re-run S1, S2, S3**

Dispatch the same three prompts from Tasks 2 through 4 verbatim (fresh subagents). Record results in `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md` under a `## Post-Edit Results` heading.

- The S3 GREEN rerun executes in a fresh isolated worktree created from baseline `1fd11d97`, with a follow-up setup commit `e66be7e4` (parent `1fd11d97`) installing the GREEN skill docs at the correct `.agents/skills/rust-design/` path, so the skill faces undone work in the same state as the baseline run (the earlier attempt `2a6449fa` botched this with a doubled `.agents/skills/rust-design/rust-design/` path, so it ran the baseline skill and its results are marked invalid); `.design/` deliverables are later copied back to this worktree's `.design/2026-10-19-implement-dirtree/`.

- [ ] **Step 2: Compare against baseline**

For each scenario assert: every assertion applicable to that scenario PASSes (C2, C3, C4: Review and Implement runs only; C7: Implement run only), and C6 (words consulted) ≤ that scenario's baseline value. Diff the `Files Consulted` lists; the post-edit Review run must still open `DISCOVERY.md`, Model must still open `MODELING.md`, Implement must still open `DEEPENING.md` + `METRICS.md` (C5).

- [ ] **Step 3: Efficiency gates**

Run: `wc -w .agents/skills/rust-design/*.md`
Expected: SKILL.md ≤1100 (E1 PASS); total incl. `report-template.md` ≤5350 (E2 PASS).

### Task 18: REFACTOR loop (conditional)

- [ ] **Step 1: If any C-assertion fails, attribute and repair**

For each failure, identify the cut responsible and apply exactly one remedy, then re-run only the affected scenario:

- Pointer not followed → convert the pointer to `**REQUIRED:** Read [X](X) §N before <action>; rules live only there.`
- Rule lost entirely → restore it in its owning reference doc (never re-duplicate into SKILL.md).
- Scout prompt quality degraded (lens table) → enrich that row's Inspect column with the missing keywords from git history (`git log -p -- .agents/skills/rust-design/DISCOVERY.md`).

- [ ] **Step 2: Repeat until all scenarios pass**

Loop Task 17 → 18. This loop is expected to be short (0 to 2 iterations); a long loop means a Tier B cut was too aggressive; revert that specific cut (`git log -p`) rather than weakening assertions. The oracle in `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md` is never edited to make tests pass.

### Task 19: Final verification and commit

- [ ] **Step 1: Run repo verification gate**

Run: `mise run verify`
Expected: fmt/check/lint/test all pass (skill docs are not compiled; this guards against accidental non-doc changes).

- [ ] **Step 2: Final word-count report**

Run: `wc -w .agents/skills/rust-design/*.md | sort -n`
Expected: E1 and E2 PASS; paste the table into `docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md` under `## Final Word Counts`.

- [ ] **Step 3: Commit evidence + plan**

```bash
git add docs/superpowers/plans/2026-10-18-rust-design-skill-revision-results.md docs/superpowers/plans/2026-10-18-rust-design-skill-revision.md
git commit -m "test(rust-design): post-edit scenario results and final word counts (GREEN)"
```

### Task 20: Self-review against the spec

- [ ] **Step 1: Spec coverage check**

Verify each analysis requirement has a task: efficiency gates (T1, T10, T17), capability preservation (T2 to T4 oracle, T17 re-run), Tier A cuts (T6 to T9), Tier B cuts (T11 to T14), grilling follow-up for all modes (T15), REFACTOR loop (T18). Fix any gap inline.

- [ ] **Step 2: Placeholder scan**

`grep -nE 'TBD|TODO|fill in|Similar to Task' docs/superpowers/plans/2026-10-18-rust-design-skill-revision.md`
Expected: the grep self-matches two lines (the command literal above and this expectation line, which mentions `TBD`); no other matches.

- [ ] **Step 3: Name consistency check**

Confirm every grep pattern in verification steps matches literal text used in the edit steps (e.g., `DEFERRED_TOOL_UNAVAILABLE`, `report-template`, `grill-me`, `Line Target`).
