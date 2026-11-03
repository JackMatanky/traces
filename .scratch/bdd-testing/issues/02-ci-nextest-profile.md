# 02: CI nextest profile with JUnit output

**What to build:** Every nextest job in CI runs a no-retry profile and publishes its JUnit report. Flaky scenarios then fail visibly and are diagnosable on any operating system. Read spec section 9.4 (commit 1).

**Current state:** CI nextest jobs use the default profile. They produce no JUnit report and may retry.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** None (can start immediately)

**Category:** enhancement

**Status:** done

- [x] A nextest `ci` profile exists with JUnit output and no retries.
- [x] Each CI nextest job selects the profile and uploads its JUnit file as an artifact.
- [x] A pull-request run shows the artifact on every operating system in the matrix. Run 41626080102 (PR 3) published `junit-test-e2e-{ubuntu,macos,windows}-latest`, `junit-test-unit` and `junit-test-integration`.
- [x] `mise run verify` passes.

**Out of scope:**
- Mutation or coverage jobs in CI.
- Retries.
