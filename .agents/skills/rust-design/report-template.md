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
<!-- Embed output table directly from: uv run .../diff.py baseline.json final.json
Review mode: zero delta; sections 2/5 emit baseline only; METRICS.md not
consulted (delta scoring); thresholds in METRICS.md, read only when deltas
exist. -->

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
