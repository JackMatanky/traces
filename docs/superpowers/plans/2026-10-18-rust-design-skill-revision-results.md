# Rust-Design Skill Revision: Test Oracle & Baseline

Scenario results are appended under "## Scenario results" (RED baseline) and "## Post-Edit Results" (GREEN) below.

## Efficiency gates (RED now, GREEN after edits)
- E1: wc -w .agents/skills/rust-design/SKILL.md <= 1100 (baseline: 1633 -> FAILS now = RED)
- E2: wc -w .agents/skills/rust-design/*.md incl. future report-template.md <= 5200 (baseline: 6505 measured on the 6 pre-existing files -> FAILS now = RED)

## Capability assertions (must pass BEFORE and AFTER edits)
- C1: Deliverables under .design/<date>-<mode>-<slug>/ only. Check: `git status --short` shows no new paths outside the scenario's .design/ dir, and `ls .design/` shows exactly that one scenario dir.
- C2: 10 scout files present in .design/.../scouts/ (Review) and no scout claims an executed build/test run. Check: `grep -E 'Ran `cargo (test|build)`|Executed `cargo|I ran cargo' .design/<dir>/scouts/*.md` finds nothing (negated mentions such as "did not run cargo test" are fine).
- C3: ledger.json exists, sorted by descending EAV; every finding has a disposition; deferrals use only DEFERRED_TOOL_UNAVAILABLE | DEFERRED_OUT_OF_SCOPE | DEFERRED_HIGH_TRACE_COST, total <= 3.
- C4: report.md contains all 7 mandated section headings (## 1 through ## 7).
- C5: Scenario's "Files Consulted" includes the mode's required reference doc (Review: DISCOVERY.md; Model: MODELING.md; Implement: DEEPENING.md and METRICS.md).
- C6: "Words consulted" (wc -w of files listed in Files Consulted) <= baseline value recorded per scenario.
- C7 (Implement runs only): patch.diff status reported, every trial logged kept or reverted with a reason, and `git status --short` shows no broken scaffolding left behind.

## Scenario results
(to be filled per scenario)
