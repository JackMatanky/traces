# 16: U2: three proptests

**What to build:** Three property tests protect laws that no example can: table escaping, tag containment and the total order of `NoteFieldValue`. Read spec section 9.8.

**Blocked by:** 06 Coverage-gap bookkeeping and slice 1 close

**Category:** enhancement

**Status:** ready-for-agent

- [ ] Each law has one proptest inline with its source.
- [ ] The `proptest-regressions` files are committed.
- [ ] `mise run verify` passes.
