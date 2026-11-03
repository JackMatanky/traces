# 04: Three-OS pilot gate

**What to build:** One green run on ubuntu, macos and windows decides whether the e2e placement works. A framework-caused red stops all deletions. Read spec sections 9.4 (the gate) and 9.5.

**Blocked by:** 02 CI nextest profile with JUnit output and 03 Trust lifecycle pilot scenarios

**Category:** enhancement

**Status:** ready-for-agent

- [ ] The slice branch is pushed and a draft PR is open as a CI trigger only.
- [ ] Every CI job is green on the head commit. This includes `check` and `test-e2e` on all three operating systems, and the audit job.
- [ ] The tool-install failure of the audit job (2026-10-29) is fixed, or a re-run is clean.
- [ ] S1 and S2 pass 100 runs in a row locally with the nextest stress count.
- [ ] The run URLs are recorded in the amendment log.
- [ ] If an operating system fails for a framework cause, stop and make no deletion. Ask the user whether to reopen decision tickets 06 and 08 of the adoption map. Adding scenarios stays allowed.
