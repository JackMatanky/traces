# 04: Three-OS pilot gate

**What to build:** One green run on ubuntu, macos and windows decides whether the e2e placement works. A framework-caused red stops all deletions. Read spec sections 9.4 (the gate) and 9.5.

**Current state:** No scenario has run on Linux or Windows. The audit job failed on 2026-10-29 at a tool install, not on code.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 02 CI nextest profile with JUnit output and 03 Trust lifecycle pilot scenarios

**Category:** enhancement

**Status:** done

- [x] The slice branch is pushed and a draft PR is open as a CI trigger only. PR 3: https://github.com/JackMatanky/traces/pull/3
- [x] Every CI job is green on the head commit (a3782751). Run: https://github.com/JackMatanky/traces/actions/runs/41626080102. This includes `check` and `test-e2e` on ubuntu, macos and windows, and the audit job.
- [x] The tool-install failure of the audit job (2026-10-29) is fixed, or a re-run is clean. The audit job passed in that run (3m26s) with no change to its install step.
- [x] S1 and S2 pass 100 runs in a row locally with the nextest stress count (`cargo nextest run --test e2e --all-features --stress-count 100 -E 'test(/bdd::features_scenarios/)'`: 100/100 iterations passed).
- [x] The run URLs are recorded in the amendment log.
- [x] No operating system failed, so the deletion preconditions hold.

**Out of scope:**
- Any deletion.
- A change to the framework verdict. The user decides it.
