# 11: Slice 6: schema and temporal at the boundary

**What to build:** Schema and temporal rules are scenarios at the process boundary. Rows: 04-S3, 04-S4, 04-S7, 04-T2, 04-T4, 04-T6 to 04-T8, 04-B1 to 04-B7. Follow spec section 9.3 for the rows and the converted tests, section 7 for each deletion, and section 6.4 for review.

**Current state:** The guarantees of the owned rows have no scenario. Some Rust e2e tests cover part of them.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** done (the deletion wave completed the three deferred test conversions and their mutation compares)

- [x] Every row the slice owns has a scenario of the class that spec section 4 assigns.
- [x] The Status of each owned row in the invariance map reads `covered`.
- [x] Each scenario passes the six conditions of spec section 3.2 and the authoring rules of spec section 5.
- [x] Resolve the entry condition of spec section 9.7 first, and record the result. A scenario runs in another time zone through the environment hook.
- [x] A check shows whether Windows honors an IANA `TZ` value.
- [x] The descendant-selector Rust test stays (spec section 7). A new scenario covers 04-B1 beside it.
- [x] A converted test is deleted only when this slice completes its last row.
  - `class_field_config_drives_class_selection_from_disk` stayed while
    01-G21 was slice 5's row; slice 5 has since covered 01-G21.
- [x] Each deletion has the proof of spec section 7.
  - The deletion wave removed `class_field_config_drives_class_selection_from_disk`,
    `list_filters_rfc3339_field_by_its_exact_instant`, and
    `renders_file_sourced_select_field_in_e2e_template`. The ledger records
    the kill demonstrations, before/after mutation compares, and name diff.
- [x] `mise run verify` passes.
  - Passed on the scenario tree (commit `102253b8`) and on the review-fixes tree after the deletion wave (2026-11-04).
- [x] The ledger section is complete.
- [x] The gap recompute line is in the adoption map.
- [x] The due rubric questions are answered.
- [x] The amendment log has an entry.
- [x] The slice branch is merged locally.

**Out of scope:**

- Rows that another slice owns.
- Known defects (spec section 3.3).
- Facade-seam scenarios and Scenario Outlines.
- A wait for arch-design ticket 20. The scenario states only the selection result that the CLI shows.
