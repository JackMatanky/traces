# 01: Repair the layer gate and test-task gaps

**What to build:** The machine gates and test tasks stop hiding failures, so the pilot scenarios can rely on them. This is a prefactor: it changes no product behavior. Read spec sections 5, 6.6 and 9.4 (commit 1).

**Current state:** The layer gate hides search errors and does not scan scenario step files. The feature-text rules have no check. The unit-test task passes with zero tests.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** None (can start immediately)

**Category:** enhancement

**Status:** ready-for-agent

- [ ] The layer gate fails when its search command exits with code 2 or higher.
- [ ] No search in the layer gate hides an error.
- [ ] The layer gate scans the scenario step files as well as the existing e2e files.
- [ ] The five feature-text checks exist. They reject a `Rule:` line, `skip!`, `@ignore`, a duplicate `Scenario:` title across feature files, and a search error.
- [ ] Each check fails on a deliberate bad input and then passes.
- [ ] The unit-test task fails when it runs zero tests.
- [ ] The coverage task and CI run e2e with the same feature set.
- [ ] The gate-owned note of the coverage-gap method (spec section 6.2) is flipped.
- [ ] `mise run verify` passes.

**Out of scope:**
- Changes to product code or to the public API baselines.
- New Rust-symbol patterns in the layer gate.
