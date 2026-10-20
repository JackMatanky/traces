# Rust-Design Skill Revision: Test Oracle & Baseline

Scenario results are appended under "## Scenario results" (RED baseline) and "## Post-Edit Results" (GREEN) below.

## Efficiency gates (RED now, GREEN after edits)
- E1: wc -w .agents/skills/rust-design/SKILL.md <= 1100 (baseline: 1633 -> FAILS now = RED)
- E2: wc -w .agents/skills/rust-design/*.md incl. report-template.md <= 5350 (baseline: 6505 -> RED; amended from <=5200 during GREEN before Task 17 verification: the REFACTOR trims of commit cdb006bb reached 5326 by removing a diagram, an "Evaluation Purpose" column, scoreboard example rows, a DISCOVERY scout-isolation reword, and compressing SKILL.md prose, so the next candidates were rule text rather than redundancy; user-approved 2026-10-18, recorded in commit 9bbe59ca)

## Capability assertions (per-mode scoping noted on each line; must pass BEFORE and AFTER edits)
- C1: Deliverables confined to .design/<date>-<mode>-<slug>/ (plus sanctioned keep-commits for Implement runs). Check: `git status --short` shows no new paths outside the scenario's .design/ dir and the sanctioned keep-commits, and `ls .design/` shows exactly one directory per scenario run so far (this run's dir plus prior scenario dirs), with no stray dirs. (Clarified during RED: the original "exactly that one scenario dir" was only true for the first scenario.)
- C2 (Review, Implement only): 10 scout files present in .design/.../scouts/ (Review and Implement) and no scout claims an executed build/test run. Check: ``grep -E 'Ran `cargo (test|build)`|Executed `cargo|I ran cargo (test|build)' .design/<dir>/scouts/*.md`` finds nothing (negated mentions such as "did not run cargo test" are fine). (pattern hardened to ``(test|build)`` on all alternatives; supersedes the shorter Task 1 template form)
- C3 (Review, Implement only): ledger.json exists, sorted by descending EAV; every finding has a disposition; deferrals use only DEFERRED_TOOL_UNAVAILABLE | DEFERRED_OUT_OF_SCOPE | DEFERRED_HIGH_TRACE_COST, total <= 3. Pinned sort check: ``python3 -c "import json;d=json.load(open('.design/<dir>/ledger.json'));h=d.get('hypotheses') or d.get('candidates');e=lambda x:next((v for v in [(x['estimated_eav'].get('eav') if isinstance(x.get('estimated_eav'),dict) else None),(x['eav'].get('value') if isinstance(x.get('eav'),dict) else x.get('eav')),(x['eav'].get('eav') if isinstance(x.get('eav'),dict) else None)] if isinstance(v,(int,float))),None);vs=[e(x) for x in h];nums=[v for v in vs if isinstance(v,(int,float))];print(all(a>=b for a,b in zip(nums,nums[1:])),'skipped=',[x.get('id') for x,v in zip(h,vs) if not isinstance(v,(int,float))])"``. The resolver covers the three ledger EAV shapes seen across runs: RED ledgers store `eav` as an object with a `value` field (`eav.value`), GREEN S1 stores it under `estimated_eav.eav`, and GREEN S3 stores a plain float (`eav`); items with no numeric EAV (e.g. `HYP-L7-NONE`, `HYP-T5`, `HYP-INT`) resolve to `None`, are excluded from the sort, and are listed under `skipped=` rather than silently coerced to 0. Disposition completeness stays semantic (field name not pinned by current docs); the S3 interpretation is recorded in the S3 results block.
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
- Branch state: 4 kept-trial commits on branch (`c22f33fd`, `e5514607`, `5b509c5f`, `7dfc2e1f`) per the skill's keep-commit rules (count verified: `git rev-list --count 1fd11d97..7dfc2e1f` = 4; `e5514607` is trial-2's keep, tree- and message-identical to its trial commit `7590b0b2`, so no fifth commit was folded into the batch); `git status --short` outside `.design/` clean apart from the results and plan files. C1 PASS.
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
- C6 PASS: 2945 <= 3535 (original measurement; stale by +4 because report-template.md shifted 246 -> 250 after this run, giving corrected 2949, the same class of stale-bytes arithmetic the S3 section documents; still passing).
- grill-me offered in final message.

### S2: Model mode (GREEN)

- Dir: `.design/2026-10-19-model-field-parsing-invalid-records/`; target `src/field.rs`.
- C1 PASS: deliverables confined; no run output outside `.design/`.
- C4 N/A: Model mode runs Step 4 only, so C4 was scoped out of the oracle in the RED S2 record.
- C5 PASS: Files Consulted = SKILL.md, MODELING.md, report-template.md; required MODELING.md included.
- C6 PASS: 2162 <= 6157 (original measurement; stale by +4 because report-template.md shifted 246 -> 250, giving corrected 2166, still passing; baseline skill refs incl. SKILL = 2641, also pass).
- Run content (for GREEN comparison): same core fix as baseline (infallible `From<FieldName> for FieldKey` replaced by `TryFrom`).
- grill-me offered.

### S3: Implement/Steady mode (GREEN)

- Previous attempt INVALID: the first recorded S3 GREEN rerun (worktree `/Users/jack/Documents/41_personal/traces/.worktrees/s3-green-run`, which no longer exists per `git worktree list`; its renamed evidence dir now lives in this worktree's `.design/` as `.design/2026-10-19-implement-src-dirtree-rs-INVALID-baseline-bytes/`, formerly `.design/2026-10-19-implement-src-dirtree-rs/`) actually executed the BASELINE skill. Its setup commit `2a6449fa` installed the GREEN docs at a doubled path `.agents/skills/rust-design/rust-design/` while the real skill path stayed baseline (`git show 2a6449fa:.agents/skills/rust-design/SKILL.md | wc -w` = 1633; baseline markers present; no report-template.md). Corroborating file evidence: the dir's `checklist.md` opens with `# Workflow Execution Checklist` (a heading deleted from the skill in Task 6); session evidence: grill-me absent from the closing message. Its recorded assertions and numbers are void, including the old C6 = 5098 (arithmetic over files that run did not actually read), keep-commits `3c71e492` + `ceb8abcc`, C3 7 hypotheses, and patch.diff 286 lines (recovered from the pre-correction record at `227bb4f2`).
- Valid rerun (record of truth): fresh worktree `/Users/jack/Documents/41_personal/traces/.worktrees/s3-green-run-2` from baseline `1fd11d97`; setup commit `e66be7e4` installed the current GREEN skill docs at the correct `.agents/skills/rust-design/` path (SKILL.md = 1100 words, zero baseline markers, report-template.md present, corpus 5348); skill loaded via direct-Read of `.agents/skills/rust-design/SKILL.md` (skill-tool cache bypassed, as documented for all GREEN runs). Scenario: Implement/Steady, target `src/dirtree.rs`.
- Dir: `.design/2026-10-19-implement-dirtree/` (evidence copied into this worktree's `.design/`; note the dir name differs from the invalid attempt's).
- C1 PASS: worktree clean apart from keep-commits; deliverables confined to the scenario dir.
- C2 PASS: 10 scout files; pinned grep no-match (exit 1).
- C3 PASS: 4 hypotheses, EAV-sorted (values 0.675, 0.375, ...); 0 formal deferrals (dismissed candidates in graveyard).
- C4 PASS: 7/7 headings.
- C5 PASS: Files Consulted = all seven skill docs (SKILL, TOOLING, DISCOVERY, DEEPENING, MODELING, METRICS, report-template); METRICS is correct for Implement (step 5.4).
- C6 PASS: 5348 <= 6505.
- C7 PASS: keep-commits `07e41d20` + `ef7d0e76` on branch `s3-green-run-2` per keep rules; patch.diff 278 lines; run reports `cargo public-api` diff zero vs run base; `tests/baselines/pub-tokens.txt` updated inside the keep-commits (required by the visibility freeze for the intentional pub(crate) changes); 2/2 trials kept after gates green (trial 2 needed one refinement after a `walkdir::Error` vs `io::Error` mismatch).
- Disclosures: 10 lenses run in-process (no subagent tool); one `cargo public-api` bad-flag retry; one pre-existing flaky `temporal` TZ test observed.
- grill-me offered in the closing message: yes.

### Method notes and anomalies

- Stale skill cache: the `skill` tool serves a cached copy captured at app start. Early GREEN attempts (S1/S2 attempt 1 and 2, dirs renamed `*-SUPERSEDED-baseline-bytes` and `*-ATTEMPT2-invalid-cached-bytes`) accidentally ran the baseline skill. All recorded GREEN runs bypassed the cache with an explicit direct-Read of the worktree `.agents/skills/rust-design/SKILL.md`.
- S1 attempt 3 (dir `*-ATTEMPT3-C6-fail-METRICS`) was valid GREEN but failed C6 (3727 > 3535) because it consulted METRICS.md for the section 2/5 delta rubric. Task 18 fix: report-template.md section 2 comment scoped Review mode to zero delta and originally redirected SLoC thresholds to DISCOVERY.md (corpus 5326 -> 5344); review-fix commit 5fc169ae (with f6d6ae2c) later corrected the attribution to METRICS.md and single-sourced the policy. S1 attempt 4 (recorded above) passed C6 = 2945 without opening METRICS.md (original measurement; corrected to 2949 after report-template.md's 246 -> 250 shift, still passing).
- S3 failed attempt 1 (permission denial) and attempt 2 (transient certificate verification error) before the run that is now marked INVALID completed (see the S3 GREEN section above; the valid rerun is the `s3-green-run-2` record).
- Baseline scenario runs also dispatched without nested subagent tools; both baseline and GREEN runs disclosed in-process lens execution. Main checkout `.agents/skills/rust-design/` was temporarily synced to GREEN bytes for the skill loader and has been restored to baseline (`git restore` + removed untracked report-template.md).

### Task 19: repo verification gate (`mise run verify`)

- Recorded from the execution session's output (not re-run for this hygiene edit): Task 19 Step 1's `mise run verify` passed at 43750831 (the execution session's final commit; Rust tree at the baseline keep-commit tip `7dfc2e1f`) with exit 0; fmt/check/lint/test all green. Evidence: 3634 unit + 41 integration + 61 e2e tests passed (2 skipped), 86 doctests; `check:visibility` freeze passed with 39 pre-existing non-fatal rustdoc private-link warnings; `check:layers` passed with pre-existing `tests/common/` path warnings. Verify was subsequently re-run green at b11bb827 (covering the review-fix Rust commits 6498eda5 and b11bb827) with identical numbers.
- Task C visibility: the review-fix `pub(crate)` changes (`entry_paths` helper in `src/file_tracker.rs`, `DEFAULT_IGNORED_DIRS` const in `src/env_vars.rs`) were accepted by the `check:visibility` freeze via a `tests/baselines/pub-tokens.txt` baseline update made inside commit 6498eda5, the same mechanism the S3 keep-commits used.

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

### Task E: second review-driven precision fixes

Four small fixes from the second review cycle, applied under the hard gates SKILL.md `wc -w` <= 1100 and 7-doc corpus `wc -w` <= 5350. The Final Word Counts block above is the pre-Task-E snapshot; per-file redistribution from this task: METRICS.md 804 -> 793, report-template.md 250 -> 261, SKILL.md unchanged at 1100; corpus total unchanged at 5348.

- **E-fix-1 (grader-pass step reference):** SKILL.md:120 -> :122. Before: "Run an independent grader pass against the Done Criteria of Steps 1-5". After: "Run an independent grader pass against the Done Criteria of active steps". The old wording contradicted the mode table (SKILL.md:21): Review's active steps are 1, 2, 6 and Model's is only 4, so "Steps 1-5" demanded a range that includes inactive steps for two of three modes. Net-zero word count (2 tokens -> 2 tokens).
- **E-fix-2 (containment tree completeness):** SKILL.md:41-54. Before: the tree listed baseline.json, ledger.json, graveyard.md, report.md, scouts/, patch.diff only. After: bare `final.json` and `final-diff.md` lines inserted after `baseline.json` (both are real Implement-run artifacts, e.g. `.design/2026-10-19-implement-dirtree/`), with in-tree comment trims offsetting the added tokens: `# Ranked hypotheses` -> `# Hypotheses`, `# Implement mode only` -> `# Implement only`, and the root-loose-files sentence (now :54) tightened from "No loose evaluation directories or scout files are created at the workspace root." to "No loose evaluation directories or scout files at the workspace root." (rule preserved). Net-zero word count for SKILL.md.
- **E-fix-3 (gauge glosses):** METRICS.md:19 and :34. Before: "Graph Edge Density" and `DD` appeared unglossed while SLoC/IKL/CRAP were expanded. After: §1 row reads "Graph Edge Density (GED)" and the §2 claim-routing row reads "`DD` (Deletion Dividend)". +3 words, offset by meaning-preserving prose trims elsewhere in METRICS.md (-14): §1 intro "The following metrics are ... and compared" -> "Metrics are ... , compared"; §3 intro drops "architectural" and "semantic"; PS section drops "module" and "internal"; BR drops "directly"; PD drops "architectural"; BBR drops "message"; Deletion Dividend drops a redundant "the"; Isolated Semantic Gauges closes with "an isolated dimension"; §5 intro drops "iterative" and "generated by" -> "from". Every threshold, tool name, rule, and assertion intact.
- **E-fix-4 (Model-mode §5 scoping):** report-template.md:30 (new line after the `## 5. Deletion Dividend Ledger` heading). Before: §5 implied deletions with no Model-mode scoping, while the §2 comment already scoped Review. After: `<!-- Model mode: no trials; record N/A or supersessions only. -->`, matching SKILL.md's Model stop condition (active step 4 only; no trials, so no deletions or graveyard entries).

Re-checked gates after all edits (measured): SKILL.md `wc -w` = 1100 (E1); 7-doc corpus (SKILL+TOOLING+DISCOVERY+DEEPENING+MODELING+METRICS+report-template) = 5348 (E2). No en/em dashes introduced on any added diff line; `git diff --stat` touched only the three skill docs (the results-file append is this record).
