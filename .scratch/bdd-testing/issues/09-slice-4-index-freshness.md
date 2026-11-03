# 09: Slice 4: index freshness

**What to build:** Index freshness is a set of scenarios that observe the index through spawned runs. Rows: 01-G29, 02-G13, 03-P01 to 03-P03, 03-P05 (observable half), 03-P08, 03-P20 to 03-P22, 03-P25, and seams 2, 4 and 5. Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Current state:** The guarantees of the owned rows have no scenario. Some Rust e2e tests cover part of them.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** ready-for-agent

- [ ] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [ ] The Status of each owned row in the invariance map reads `covered`.
- [ ] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [ ] The mtime canary stays in Rust, relabelled as a known limitation owned by arch-design ticket 25, and is never converted.
- [ ] The config-edit heal test that arch-design ticket 25 names converts here.
- [ ] A stress run of 100 shows no scenario flakes from racy timestamps.
- [ ] A converted test is deleted only when this slice completes its last row.
- [ ] Each deletion has the proof of spec section 7.
- [ ] `mise run verify` passes.
- [ ] The ledger section is complete.
- [ ] The gap recompute line is in the adoption map.
- [ ] The due rubric questions are answered.
- [ ] The amendment log has an entry.
- [ ] The slice branch is merged locally.

**Out of scope:**
- Rows that another slice owns.
- Known defects (spec section 3.3).
- Facade-seam scenarios and Scenario Outlines.
