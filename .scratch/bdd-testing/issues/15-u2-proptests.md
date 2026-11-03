# 15: U2: three proptests

**What to build:** Three property tests protect laws that no example can: table escaping, tag containment and the total order of `NoteFieldValue`. Read spec section 9.8.

**Current state:** No proptest covers these three laws. No proptest regression files are committed.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 06 Coverage-gap bookkeeping and slice 1 close

**Category:** enhancement

**Status:** done

- [x] Each law has one proptest inline with its source.
- [x] The `proptest-regressions` files are committed.
- [x] `mise run verify` passes.

**Out of scope:**

- Other laws.
- Changes to product code.
