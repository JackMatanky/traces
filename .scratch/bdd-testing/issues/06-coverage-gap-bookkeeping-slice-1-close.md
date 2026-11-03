# 06: Coverage-gap bookkeeping and slice 1 close

**What to build:** The invariance map becomes a working coverage-gap ledger and slice 1 closes. Read spec sections 6, 9.4 (commit 2), 9.5, 9.6 and 9.8.

**Blocked by:** 05 First deletion with proof and ledger

**Category:** enhancement

**Status:** ready-for-agent

- [ ] The invariance-map tables have a Status column and one-dialect row IDs in one pass. The pilot rows read `covered`. Other states follow spec section 6.
- [ ] The sixth gate check is live and resolves every pilot pointer. It names the row on failure.
- [ ] The first gap recompute line is in the map.
- [ ] The profraw check is done: coverage numbers either see scenario-executed source or are documented as blind to it.
- [ ] The candidate-table row that wrongly maps the default-path init test to 01-G17 is fixed.
- [ ] The hygiene backlog file exists and lists the items of spec section 9.8.
- [ ] Rubric questions 1 and 2 are answered for the user.
- [ ] `mise run verify` passes.
- [ ] The ledger section is complete.
- [ ] The gap recompute line is in the adoption map.
- [ ] The due rubric questions are answered.
- [ ] The amendment log has an entry.
- [ ] The slice branch is merged locally.
