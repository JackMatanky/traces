# 07: Slice 2: process contract

**What to build:** The process contract is a set of scenarios: exit codes 0, 1 and 2, diagnostic output and stream contracts. Rows: 01-G1 (codes 0, 1 and 2, no closed-pipe case), 01-G2, 01-G3, 01-G4 (other codes), 05-G1, 05-G2, 05-G3 (other codes). Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Current state:** The guarantees of the owned rows have no scenario. Some Rust e2e tests cover part of them.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 06 Coverage-gap bookkeeping and slice 1 close

**Category:** enhancement

**Status:** ready-for-agent

- [ ] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [ ] The Status of each owned row in the invariance map reads `covered`.
- [ ] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [ ] A second cold session writes one further scenario from the spec alone, with no framework-source read. Rubric question 3 is answered for the user on that scenario.
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
