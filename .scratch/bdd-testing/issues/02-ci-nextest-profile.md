# 02: CI nextest profile with JUnit output

**What to build:** Every nextest job in CI runs a no-retry profile and publishes its JUnit report. Flaky scenarios then fail visibly and are diagnosable on any operating system. Read spec section 9.4 (commit 1).

**Blocked by:** None (can start immediately)

**Category:** enhancement

**Status:** ready-for-agent

- [ ] A nextest `ci` profile exists with JUnit output and no retries.
- [ ] Each CI nextest job selects the profile and uploads its JUnit file as an artifact.
- [ ] A pull-request run shows the artifact on every operating system in the matrix.
- [ ] `mise run verify` passes.
