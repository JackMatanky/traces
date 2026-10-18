# Rust-Design Skill Revision: Test Oracle & Baseline

## Efficiency gates (RED now, GREEN after edits)
- E1: wc -w SKILL.md <= 1100 (baseline: 1633 -> FAILS now = RED)
- E2: wc -w all skill .md files (incl. report-template.md) <= 5200 (baseline: 6505 -> FAILS now = RED)

## Capability assertions (must pass BEFORE and AFTER edits)
- C1: Deliverables under .design/<date>-<mode>-<slug>/ only; no loose dirs at repo root
- C2: 10 scout files present in .design/.../scouts/ (Review) and no scout output claims a build/test run
- C3: ledger.json exists, sorted by descending EAV; every finding has a disposition; deferrals use the closed enum, <= 3
- C4: report.md contains all 7 mandated section headings
- C5: Scenario's "Files Consulted" includes the mode's required reference doc (Review: DISCOVERY.md; Model: MODELING.md; Implement: DEEPENING.md and METRICS.md)
- C6: "Words consulted" (wc -w of files listed in Files Consulted) <= baseline value recorded per scenario

## Scenario results
(to be filled per scenario)
