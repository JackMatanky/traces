# 11: Slice 6: schema and temporal at the boundary

**What to build:** Schema and temporal rules are scenarios at the process boundary. Rows: 04-S3, 04-S4, 04-S7, 04-T2, 04-T4, 04-T6 to 04-T8, 04-B2 to 04-B7. 04-B1 is out of scope here. The remainder ticket owns it. Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** ready-for-agent

- [ ] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [ ] The Status of each owned row in the invariance map reads `covered`.
- [ ] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [ ] Resolve the entry condition of spec section 9.7 first, and record the result. A scenario runs in another time zone through the environment hook.
- [ ] A check shows whether Windows honors an IANA `TZ` value.
- [ ] Only tests that this slice completes are converted. The descendant-selector test is not converted here.
- [ ] A converted test is deleted only when this slice completes its last row.
- [ ] Each deletion has the proof of spec section 7.
- [ ] `mise run verify` passes.
- [ ] The ledger section is complete.
- [ ] The gap recompute line is in the adoption map.
- [ ] The due rubric questions are answered.
- [ ] The amendment log has an entry.
- [ ] The slice branch is merged locally.
