# 06: Coverage-gap bookkeeping and slice 1 close

**What to build:** The invariance map becomes a working coverage-gap ledger and slice 1 closes. Read spec sections 6, 9.4 (commit 2), 9.5, 9.6 and 9.8.

**Current state:** The invariance map has no Status column. The sixth gate check has nothing to read.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 05 First deletion with proof and ledger

**Category:** enhancement

**Status:** done

- [x] The invariance-map tables have a Status column and one-dialect row IDs in one pass. The pilot rows read `covered`. Other states follow spec section 6.
- [x] The sixth gate check is live and resolves every pilot pointer. It names the row on failure.
- [x] The first gap recompute line is in the map.
- [x] The profraw check is done: coverage numbers either see scenario-executed source or are documented as blind to it.
- [x] The candidate-table row that wrongly maps the default-path init test to 01-G17 is fixed.
- [x] The hygiene backlog file exists and lists the items of spec section 9.8.
- [x] Rubric questions 1 and 2 are answered for the user.
- [x] `mise run verify` passes.
- [x] The ledger section is complete.
- [x] The gap recompute line is in the adoption map.
- [x] The due rubric questions are answered.
- [x] The amendment log has an entry.
- [x] The slice branch is merged locally. The integration branch `bdd-testing` holds the slice.

**Out of scope:**
- Rows of later slices, except to mark them `gap`.
- Numeric targets of any kind.
