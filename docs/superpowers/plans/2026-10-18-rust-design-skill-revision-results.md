# Rust-Design Skill Revision: Test Oracle & Baseline

Scenario results are appended under "## Scenario results" (RED baseline) and "## Post-Edit Results" (GREEN) below.

## Efficiency gates (RED now, GREEN after edits)
- E1: wc -w .agents/skills/rust-design/SKILL.md <= 1100 (baseline: 1633 -> FAILS now = RED)
- E2: wc -w .agents/skills/rust-design/*.md incl. report-template.md <= 5350 (baseline: 6505 -> RED; amended from <=5200 during GREEN before Task 17 verification: the REFACTOR trims of commit cdb006bb reached 5326 by removing a diagram, an "Evaluation Purpose" column, scoreboard example rows, a DISCOVERY scout-isolation reword, and compressing SKILL.md prose, so the next candidates were rule text rather than redundancy; user-approved 2026-10-18, recorded in commit 9bbe59ca)

## Capability assertions (per-mode scoping noted on each line; must pass BEFORE and AFTER edits)
- C1: Deliverables under .design/<date>-<mode>-<slug>/ only. Check: `git status --short` shows no new paths outside the scenario's .design/ dir, and `ls .design/` shows exactly one directory per scenario run so far (this run's dir plus prior scenario dirs), with no stray dirs. (Clarified during RED: the original "exactly that one scenario dir" was only true for the first scenario.)
- C2 (Review, Implement only): 10 scout files present in .design/.../scouts/ (Review and Implement) and no scout claims an executed build/test run. Check: ``grep -E 'Ran `cargo (test|build)`|Executed `cargo|I ran cargo (test|build)' .design/<dir>/scouts/*.md`` finds nothing (negated mentions such as "did not run cargo test" are fine). (pattern hardened to ``(test|build)`` on all alternatives; supersedes the shorter Task 1 template form)
- C3 (Review, Implement only): ledger.json exists, sorted by descending EAV; every finding has a disposition; deferrals use only DEFERRED_TOOL_UNAVAILABLE | DEFERRED_OUT_OF_SCOPE | DEFERRED_HIGH_TRACE_COST, total <= 3. Pinned sort check: ``python3 -c "import json;d=json.load(open('.design/<dir>/ledger.json'));h=d.get('hypotheses') or d.get('candidates');vs=[(x['eav']['value'] if isinstance(x.get('eav'),dict) else x.get('eav')) or 0 for x in h];print(all(a>=b for a,b in zip(vs,vs[1:])))"``. Disposition completeness stays semantic (field name not pinned by current docs); the S3 interpretation is recorded in the S3 results block.
- C4 (Review, Implement only): report.md contains all 7 mandated section headings (## 1 through ## 7).
- C5: Scenario's "Files Consulted" includes the mode's required reference doc (Review: DISCOVERY.md; Model: MODELING.md; Implement: DEEPENING.md and METRICS.md).
- C6: "Words consulted" (wc -w of files listed in Files Consulted) <= baseline value recorded per scenario.
- C7 (Implement runs only): patch.diff status reported, every trial logged kept or reverted with a reason, and `git status --short` shows no broken scaffolding left behind.

## Scenario results
(to be filled per scenario)

### S1: Review mode (baseline, RED)

- Dir: `.design/2026-10-18-review-field-rs/`
- C1 PASS: `ls .design/` shows exactly this dir; `git status --short` clean apart from the pre-existing untracked plan file.
- C2 PASS: 10 scout files (lens1-lens10); pinned grep over scouts/*.md found no matches (exit 1).
- C3 PASS: 8 candidates sorted descending by EAV (1.80 -> 0.24); every candidate has a disposition (4 investigated, 4 dismissed); 3 deferrals total within the closed enum: 1x DEFERRED_HIGH_TRACE_COST, 2x DEFERRED_OUT_OF_SCOPE.
- C4 PASS: all 7 mandated headings (`## 1.` through `## 7.`) present. Observation: duplicate `## 2` heading (report.md lines 10 and 13); recorded baseline quirk, not a C4 failure.
- C5 PASS: Files Consulted = SKILL.md, TOOLING.md, DISCOVERY.md; required DISCOVERY.md included.
- C6 BASELINE: words consulted = 3535 (SKILL 1633 + TOOLING 896 + DISCOVERY 1006).
- C7 N/A: not an Implement run.
- Environment note: the scenario subagent had no nested subagent-dispatch tool available; it performed the 10 lens passes itself under the scout isolation rule (disclosed in its report section 7). Re-check this exact point in GREEN (Task 17).

### S2: Model mode (baseline, RED)

- Dir: `.design/2026-10-18-model-field-parsing/` (contains report.md).
- C1 PASS (per clarified wording): `git status --short` shows only the pre-existing results append and untracked plan file; no run output outside `.design/`; `ls .design/` shows exactly the two scenario dirs run so far (S1, S2).
- C2, C3, C4, C7 N/A: Model mode runs Step 4 only (scoped out of the oracle).
- C5 PASS: scenario final message Files Consulted includes required `MODELING.md`.
- C6 BASELINE: Files Consulted = MODELING.md (1008) + src/field.rs (5149) = 6157 words (oracle wording: wc -w of listed files); skill-docs-only subtotal 1008.
- Run content (for GREEN comparison): proposed `CanonicalKey` newtype + `TryFrom<FieldName> for FieldKey` replacing the invariant-bypassing `From` impl; no typestate introduced (citing MODELING.md section 7); report.md notes a DEFERRED_OUT_OF_SCOPE disposition.

### S3: Implement/Steady (baseline, RED)

- Dir: `.design/2026-10-18-implement-dirtree/` (baseline.json, final.json, graveyard.md, ledger.json, logs/, patch.diff, report.md, scouts/, worktrees/). patch.diff produced: yes (4 patches, `1fd11d97..HEAD`, 711 lines).
- Branch state: 4 kept-trial commits on branch (`c22f33fd`, `e5514607`, `5b509c5f`, `7dfc2e1f`) per the skill's keep-commit rules; `git status --short` outside `.design/` clean apart from the results and plan files. C1 PASS.
- C2 PASS: 10 scout files; pinned grep over `scouts/*.md` found no matches (exit 1).
- C3 PASS (interpretation recorded): ledger.json exists; hypotheses sorted descending by EAV (1.133 -> 0.267); every finding has an outcome of record per `disposition_summary` (49-finding universe with merge provenance: 4 investigated-and-tried, 11 dismissed_with_reason, 3 deferred); 3 closed_deferrals all within the enum (2x DEFERRED_OUT_OF_SCOPE, 1x DEFERRED_HIGH_TRACE_COST) <= 3. Interpretation: C3's "disposition" is semantic; current skill docs pin no field name. Per-hypothesis `disposition` field present on 7/16 hypotheses; others carry outcome via `trial` record or `disposition_summary`.
- C4 PASS: 7/7 mandated headings, no duplicates this run.
- C5 PASS: Files Consulted includes required `DEEPENING.md` and `METRICS.md` (plus SKILL, TOOLING, DISCOVERY, MODELING).
- C6 BASELINE: words consulted = 6505 (SKILL 1633 + TOOLING 896 + DISCOVERY 1006 + DEEPENING 925 + MODELING 1008 + METRICS 1037).
- C7 PASS: patch.diff reported; 4/4 trials kept with reasons (trial table in the run's final message; rejected sub-bets in graveyard.md); verify exit 0, no broken scaffolding left behind.
- Notes for GREEN (Task 17): (a) this scenario subagent had no Task/subagent tool, so it dispatched the 10 scouts and both grader passes via `opencode run --pure` instead; re-check this exact point in GREEN. (b) S3 modified branch code via 4 keep-commits; the GREEN S3 rerun must run in a FRESH worktree checked out at `1fd11d97` so the skill faces the same undone work (plan amendment recorded at Task 17). (c) Run honestly reported Max SLoC 662 exceeding the 500 target as a known regression risk (baseline behavior).

## Post-Edit Results

### Efficiency gates (GREEN)

- E1: SKILL.md = 1100 words (bound <=1100) PASS, exact.
- E2: corpus (SKILL + TOOLING + DISCOVERY + DEEPENING + MODELING + METRICS + report-template) = 5348 words (live after review-fix squeeze; GREEN measured 5344; bound <=5350 after documented amendment from 5200) PASS, margin 2.
- Gate margin risk: E1 is exact (SKILL.md = 1100 = bound) and E2 has margin 2 (5348 of 5350), so any future edit to SKILL.md requires re-trimming within SKILL.md (E1 counts SKILL.md alone), while corpus edits (E2) can be offset by trimming any reference doc; a fresh `wc -w` check is needed before commit.

### C6 counting convention

- "Words consulted" = `wc -w` of the skill reference documents the runner listed under `## Files Consulted`, with SKILL.md counted on both the baseline and the post-edit side. Baseline S2's recorded 6157 additionally included src/field.rs; post-edit numbers below are skill references only, which is conservative against the recorded baselines.

### S1: Review mode (GREEN)

- Dir: `.design/2026-10-19-review-field-rs/`; target `src/field.rs`.
- C1 PASS: deliverables confined to the scenario dir; only the pre-existing uncommitted skill edit present.
- C2 PASS: 10 scout files; pinned grep no-match (exit 1).
- C3 PASS: 8 hypotheses EAV-sorted; 1 deferral, enum-only `DEFERRED_OUT_OF_SCOPE`.
- C4 PASS: 7/7 mandated headings, 7 `##` headings total.
- C5 PASS: Files Consulted = SKILL.md, TOOLING.md, DISCOVERY.md, report-template.md; required DISCOVERY.md included.
- C6 PASS: 2945 <= 3535.
- grill-me offered in final message.

### S2: Model mode (GREEN)

- Dir: `.design/2026-10-19-model-field-parsing-invalid-records/`; target `src/field.rs`.
- C1 PASS: deliverables confined; no run output outside `.design/`.
- C4 PASS: 7/7.
- C5 PASS: Files Consulted = SKILL.md, MODELING.md, report-template.md; required MODELING.md included.
- C6 PASS: 2162 <= 6157 (baseline skill refs incl. SKILL = 2641, also pass).
- Run content (for GREEN comparison): same core fix as baseline (infallible `From<FieldName> for FieldKey` replaced by `TryFrom`).
- grill-me offered.

### S3: Implement/Steady mode (GREEN)

- Dir: `.design/2026-10-19-implement-src-dirtree-rs/`; target `src/dirtree.rs`; run in a separate worktree `/Users/jack/Documents/41_personal/traces/.worktrees/s3-green-run`.
- C1 PASS: branch clean; keep-rule commits `3c71e492` + `ceb8abcc` sanctioned by the skill keep-condition.
- C2 PASS: 10 scouts; pinned grep no-match.
- C3 PASS: 7 hypotheses sorted; 3 deferrals, enum-only `DEFERRED_OUT_OF_SCOPE`.
- C4 PASS: 7/7.
- C5 PASS: all six refs (SKILL, TOOLING, DISCOVERY, DEEPENING, MODELING, METRICS).
- C6 PASS: 5098 <= 6505.
- C7 PASS: `mise run check:visibility` pass; both public-api baselines byte-identical; 2/2 trials kept per keep rules; patch.diff 286 lines.
- grill-me not offered (variance; not an oracle assertion).

### Method notes and anomalies

- Stale skill cache: the `skill` tool serves a cached copy captured at app start. Early GREEN attempts (S1/S2 attempt 1 and 2, dirs renamed `*-SUPERSEDED-baseline-bytes` and `*-ATTEMPT2-invalid-cached-bytes`) accidentally ran the baseline skill. All recorded GREEN runs bypassed the cache with an explicit direct-Read of the worktree `.agents/skills/rust-design/SKILL.md`.
- S1 attempt 3 (dir `*-ATTEMPT3-C6-fail-METRICS`) was valid GREEN but failed C6 (3727 > 3535) because it consulted METRICS.md for the section 2/5 delta rubric. Task 18 fix: report-template.md section 2 comment scoped Review mode to zero delta and originally redirected SLoC thresholds to DISCOVERY.md (corpus 5326 -> 5344); review-fix commit 5fc169ae (with f6d6ae2c) later corrected the attribution to METRICS.md and single-sourced the policy. S1 attempt 4 (recorded above) passed C6 = 2945 without opening METRICS.md.
- S3 failed attempt 1 (permission denial) and attempt 2 (transient certificate verification error) before attempt 3 succeeded.
- Baseline scenario runs also dispatched without nested subagent tools; both baseline and GREEN runs disclosed in-process lens execution. Main checkout `.agents/skills/rust-design/` was temporarily synced to GREEN bytes for the skill loader and has been restored to baseline (`git restore` + removed untracked report-template.md).

### Task 19: repo verification gate (`mise run verify`)

- Recorded from the execution session's output (not re-run for this hygiene edit): Task 19 Step 1's `mise run verify` passed at final state with exit 0; fmt/check/lint/test all green. Evidence: 3634 unit + 41 integration + 61 e2e tests passed (2 skipped), 86 doctests; `check:visibility` freeze passed with 39 pre-existing non-fatal rustdoc private-link warnings; `check:layers` passed with pre-existing `tests/common/` path warnings. Verify was subsequently re-run green at b11bb827, after the review-fix Rust commits (6498eda5, b11bb827), with identical numbers.

## Final Word Counts

```
     250 .agents/skills/rust-design/report-template.md
     733 .agents/skills/rust-design/TOOLING.md
     779 .agents/skills/rust-design/DEEPENING.md
     804 .agents/skills/rust-design/METRICS.md
     816 .agents/skills/rust-design/MODELING.md
     866 .agents/skills/rust-design/DISCOVERY.md
    1100 .agents/skills/rust-design/SKILL.md
    5348 total
```
