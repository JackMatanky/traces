# 14: Slice 8: spawned-signal exit 130 (Unix)

**What to build:** A Unix-only scenario shows that an interrupted run exits with code 130. Row: the class-level 130 case of 01-G1.

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** ready-for-agent

- [ ] A platform-gating spike runs first. The tag set is closed and `@ignore` is banned. Find how a Unix-only scenario skips on Windows without a banned form. The result is recorded in the amendment log.
- [ ] Choose a signal-sending dev-dependency with the user. Stop and ask the user before you add it.
- [ ] The scenario passes on Unix and the three-OS run stays green.
- [ ] `mise run verify` passes.
- [ ] The ledger section is complete.
- [ ] The gap recompute line is in the adoption map.
- [ ] The due rubric questions are answered.
- [ ] The amendment log has an entry.
- [ ] The slice branch is merged locally.
