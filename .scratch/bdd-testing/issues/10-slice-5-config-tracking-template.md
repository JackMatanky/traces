# 10: Slice 5: config scope, tracking and template

**What to build:** Config scope, tracked paths and template rendering are scenarios. Rows: 01-G7, 01-G11 to 01-G16, 01-G18, 01-G21, 01-G22, 01-G25 to 01-G27, 02-G10 to 02-G12, 02-G18, 05-G7, and the 05 directory-tree, env-var and file-tracker rows. This is the largest slice. If it will not finish in one branch, split it and record the split in the amendment log. Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Current state:** The guarantees of the owned rows have no scenario. Some Rust e2e tests cover part of them.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** done

- [ ] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [ ] The Status of each owned row in the invariance map reads `covered`.
- [ ] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [ ] Review the row placement with the user at entry. Spec section 9.3 lists the rows that were placed by domain affinity.
- [x] A converted test is deleted only when this slice completes its last row.
- [x] Each deletion has the proof of spec section 7.
- [x] `mise run verify` passes.
- [x] The ledger section is complete.
- [x] The gap recompute line is in the adoption map.
- [x] The due rubric questions are answered (none due for this slice).
- [x] The amendment log has an entry.
- [ ] The slice branch is merged locally.

**Out of scope:**
- Rows that another slice owns.
- Known defects (spec section 3.3).
- Facade-seam scenarios and Scenario Outlines.
