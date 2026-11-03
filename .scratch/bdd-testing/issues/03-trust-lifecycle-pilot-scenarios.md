# 03: Trust lifecycle pilot scenarios

**What to build:** The e2e test binary adopts the `rstest-bdd` framework. It gets two scenarios for the trust and init lifecycle. S1: init does not grant trust (new coverage). S2: untrust revokes trust (a conversion of the old Rust test). The old test stays. Read spec sections 2, 5 and 9.4 (commit 1). Slice 1 owns 01-G17, 01-G23, 01-G24, and 01-G4 and 05-G3 for the untrusted diagnostic code only.

**Blocked by:** 01 Repair the layer gate and test-task gaps

**Category:** enhancement

**Status:** ready-for-agent

- [ ] Both framework crates use the exact version of spec section 2.
- [ ] The licence check allows the one new licence that they need.
- [ ] The feature file, the step files and the `touch` doc comment follow spec section 5. The single `#[expect]` is proven to fire.
- [ ] S1 and S2 assert the exit code, the first stderr line (diagnostic code) and an empty stdout, using exact-equality steps. S2 proves every guarantee of the old test.
- [ ] A line-ending rule keeps feature files LF on all platforms.
- [ ] One docstring step exercises docstring comparison.
- [ ] The break demonstration for S1 is recorded: make `init` write a trust grant and S1 fails.
- [ ] The old trust test is not deleted.
- [ ] `mise run verify` passes.
