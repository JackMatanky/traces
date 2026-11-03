# 12: Slice 7: note-model rules through query output

**What to build:** The note-model rules are scenarios that observe query output. Rows: 03-N01 to 03-N12, 03-N14, 03-N16 and 03-N17 (defect parts of 03-N05, 03-N07 and 03-N16 excluded), 05-G8, and the 05 field, task and tag rows. Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Current state:** The guarantees of the owned rows have no scenario. Some Rust e2e tests cover part of them.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** ready-for-agent

- [ ] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [ ] The Status of each owned row in the invariance map reads `covered`.
- [ ] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [ ] No scenario pins a known defect (spec section 3.3). The multi-value first-value read and the integer to float loss stay out.
- [ ] The two tests that spec section 9.3 marks needs-split become key-example scenarios, never an outline.
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
