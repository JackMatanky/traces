# 13: Slice 8: spawned-signal exit 130 (Unix)

**What to build:** A Unix-only scenario shows that an interrupted run exits with code 130. Row: the class-level 130 case of 01-G1.

**Current state:** Dropped by user decision (2026-11-03). The spike proved the case cannot be written. Numeric 130 requires a real-TTY interactive prompt. `src/dialog/terminal.rs` answers from a non-TTY fallback otherwise. A piped-stdin child that receives SIGINT dies by signal, so `ExitStatus::code()` is `None`. Deterministic prompt readiness needs a pseudoterminal, which spec section 4 bans. No scenario, no dev-dependency change, no deletion. Spec sections 9.3, 9.7, 9.9 item 13, 10 and 12 plus the ledger record the decision. Research/10 row 01-G1 carries the `pinned` pointer.

**Spec:** `.scratch/bdd-testing/spec.md`

**Blocked by:** 07 Slice 2: process contract

**Category:** enhancement

**Status:** wont-do

- [x] A platform-gating spike runs first. The tag set is closed and `@ignore` is banned. Find how a Unix-only scenario skips on Windows without a banned form. The result is recorded in the amendment log. (Result: a separate `#[cfg(unix)] scenarios!("tests/features/unix", ...)` call over a nested directory; recorded in spec section 12.)
- [ ] Choose a signal-sending dev-dependency with the user. Stop and ask the user before you add it. (Not reached; the user decided to drop the slice instead. `rustix` `process` and `pty` features were identified but stay unadded.)
- [ ] The scenario passes on Unix and the three-OS run stays green. (No scenario exists.)
- [ ] `mise run verify` passes. (Ran on the amendment-only branch.)
- [ ] The ledger section is complete. (Slice 8 section records the spike and the decision.)
- [ ] The gap recompute line is in the adoption map. (`gap recompute: slice 8 dropped, 2026-11-03`.)
- [ ] The due rubric questions are answered. (None due for this slice; rubric question 3 was answered at slice 2, and no scenario was authored.)
- [ ] The amendment log has an entry.
- [ ] The slice branch is merged locally.

**Out of scope:**

- Windows signal behavior.
- Exit codes other than 130.
