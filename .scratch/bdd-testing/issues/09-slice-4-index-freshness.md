# 09: Slice 4: index freshness

**What to build:** Index freshness is a set of scenarios that observe the index through spawned runs. Rows: 01-G29, 02-G13, 03-P01 to 03-P03, 03-P05 (observable half), 03-P08, 03-P20 to 03-P22, 03-P25, and seams 2, 4 and 5. Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Current state:** The guarantees of the owned rows have no scenario. Some Rust e2e tests cover part of them.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** done

- [x] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [x] The Status of each owned row in the invariance map reads `covered`.
- [x] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [x] The mtime canary stays in Rust as an arch ticket 25 known limit and is never converted.
- [x] The config-edit heal test that arch-design ticket 25 names converts here.
- [x] A stress run of 100 shows no scenario flakes from racy timestamps.
- [x] A converted test is deleted only when this slice completes its last row.
- [x] Each deletion has the proof of spec section 7.
- [x] `mise run verify` passes.
- [x] The ledger section is complete.
- [x] The gap recompute line is in the adoption map.
- [x] The due rubric questions are answered (none due for this slice).
- [x] The amendment log has an entry.
- [x] The slice branch is merged locally.

**Out of scope:**

- Rows that another slice owns.
- Known defects (spec section 3.3).
- Facade-seam scenarios and Scenario Outlines.
