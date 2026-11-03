# 07: Slice 2: process contract

**What to build:** The process contract is a set of scenarios: exit codes 0, 1 and 2, diagnostic output and stream contracts. Rows: 01-G1 (codes 0, 1 and 2, no closed-pipe case), 01-G2, 01-G3, 01-G4 (other codes), 05-G1, 05-G2, 05-G3 (other codes). Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Current state:** The guarantees of the owned rows have no scenario. Some Rust e2e tests cover part of them.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 06 Coverage-gap bookkeeping and slice 1 close

**Category:** enhancement

**Status:** done

- [x] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [x] The Status of each owned row in the invariance map reads `covered`.
- [x] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [x] A second cold session writes one further scenario from the spec alone, with no framework-source read. Rubric question 3 is answered for the user on that scenario. Scenario `Missing template exits with code one and reports template not found diagnostic`: YES, green on the first attempt (see ledger, slice 2).
- [x] A converted test is deleted only when this slice completes its last row. No test was deleted: spec section 9.3 lists no candidate test for slice 2.
- [x] Each deletion has the proof of spec section 7. No deletion happened.
- [x] `mise run verify` passes.
- [x] The ledger section is complete.
- [x] The gap recompute line is in the adoption map.
- [x] The due rubric questions are answered. Question 3 is in the ledger, slice 2 section.
- [x] The amendment log has an entry.
- [x] The slice branch is merged locally. The integration branch `bdd-testing` holds the slice.

**Out of scope:**

- Rows that another slice owns.
- Known defects (spec section 3.3).
- Facade-seam scenarios and Scenario Outlines.
