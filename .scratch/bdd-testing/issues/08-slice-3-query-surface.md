# 08: Slice 3: query surface

**What to build:** The query surface is a set of scenarios. Rows: 02-G1 to 02-G5, 02-G6 (flags half), 02-G7 to 02-G9, 02-G14 (equivalence and tie order), 02-G16, 01-G30, and 02-G3 for the unknown selector only. Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Current state:** The guarantees of the owned rows have no scenario. Some Rust e2e tests cover part of them.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** done

- [x] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [x] The Status of each owned row in the invariance map reads `covered`.
- [x] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [x] The two sort tests named "ascending by default" are renamed to say "explicit ascending". The rename does not close 02-G6.
- [x] A converted test is deleted only when this slice completes its last row.
- [x] Each deletion has the proof of spec section 7.
- [x] `mise run verify` passes.
- [x] The ledger section is complete.
- [x] The gap recompute line is in the adoption map.
- [x] The due rubric questions are answered. (none due for this slice)
- [x] The amendment log has an entry.
- [x] The slice branch is merged locally.

**Out of scope:**

- Rows that another slice owns.
- Known defects (spec section 3.3).
- Facade-seam scenarios and Scenario Outlines.
