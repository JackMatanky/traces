# 08: Slice 3: query surface

**What to build:** The query surface is a set of scenarios. Rows: 02-G1 to 02-G5, 02-G6 (flags half), 02-G7 to 02-G9, 02-G14 (equivalence and tie order), 02-G16, 01-G30, and 02-G3 for the unknown selector only. Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** ready-for-agent

- [ ] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [ ] The Status of each owned row in the invariance map reads `covered`.
- [ ] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [ ] The two sort tests named "ascending by default" are renamed to say "explicit ascending". The rename does not close 02-G6.
- [ ] A converted test is deleted only when this slice completes its last row.
- [ ] Each deletion has the proof of spec section 7.
- [ ] `mise run verify` passes.
- [ ] The ledger section is complete.
- [ ] The gap recompute line is in the adoption map.
- [ ] The due rubric questions are answered.
- [ ] The amendment log has an entry.
- [ ] The slice branch is merged locally.
