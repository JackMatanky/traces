# 03: Trust lifecycle pilot scenarios

**What to build:** The e2e test binary adopts the `rstest-bdd` framework. It gets two scenarios for the trust and init lifecycle. S1: init does not grant trust (new coverage). S2: untrust revokes trust (a conversion of the old Rust test). The old test stays. Read spec sections 2, 5 and 9.4 (commit 1). Slice 1 owns 01-G17, 01-G23, 01-G24, and 01-G4 and 05-G3 for the untrusted diagnostic code only.

**Current state:** No scenario framework, feature file or step file exists. The old Rust test is the only test of the untrust diagnostic.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 01 Repair the layer gate and test-task gaps

**Category:** enhancement

**Status:** done

**Break demonstration:** When init writes a trust grant, S1 fails because the
exit code is 0 instead of 1.

- [x] Both framework crates use the exact version of spec section 2.
- [x] The licence check allows the one new licence that they need.
- [x] The feature file, the step files and the `touch` doc comment follow spec section 5. The single `#[expect]` is proven to fire.
- [x] S1 and S2 assert the exit code, the first stderr line (diagnostic code) and an empty stdout, using exact-equality steps. S2 proves every guarantee of the old test.
- [x] A line-ending rule keeps feature files LF on all platforms.
- [x] One docstring step exercises docstring comparison.
- [x] The break demonstration for S1 is recorded: make `init` write a trust grant and S1 fails.
- [x] The old trust test is not deleted.
- [x] `mise run verify` passes.

**Out of scope:**
- Deleting the old test (implementation ticket 05).
- Scenarios outside the trust and init lifecycle.
- A World type, a CLI DSL or step-generating macros.
