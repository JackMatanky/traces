# 05: First deletion with proof and ledger

**What to build:** The first old Rust test is deleted with the full proof of spec section 7, and the migration ledger starts. Read spec sections 7 and 9.4 (commit 2).

**Blocked by:** 04 Three-OS pilot gate

**Category:** enhancement

**Status:** ready-for-agent

- [ ] The kill demonstration is recorded: make `untrust` keep the grant, and the old test and S2 both fail.
- [ ] The relative mutation compare on the trust module is run before and after the deletion in the background. Both escape sets are in the ledger. No new escape appears. The mutation baseline file is untouched.
- [ ] The twin test is re-read and recorded.
- [ ] The old untrust test is deleted and the nextest name list before and after shows exactly that name removed.
- [ ] The ledger has its first section and one complete row, as spec section 7 defines it.
- [ ] A `mutarust` probe on a test-only diff is recorded, and the result is labelled as evidence.
- [ ] `mise run verify` passes.
