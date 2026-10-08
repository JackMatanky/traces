# 05: EPIPE / exit-101 contract decision record

**What to build:** The EPIPE/exit-101 question stops being an ambient unknown: a decision is
recorded in-repo naming the intended contract and the cheap test shape that will guard it. No
production code changes here — this is the "decide, don't fix" half; a fix would be a separate
spec.

**Category:** enhancement

**Blocked by:** None for the repro/observation half (docs-only, start immediately); the
recorded decision lands in the T3b investigation phase per spec ordering (parallel with gap
tests, after the prerequisite slice).

**Status:** resolved

- [x] Premise verified before deciding: reproduce `traces list | head` EPIPE behavior first
      (dev-profile panic → exit 101; release `panic = "abort"` → 134) — the decision records
      what was observed, not what the audit asserts
- [x] Decision recorded under `## Comments` in this ticket (a new ADR if the maintainer
      prefers) with rationale, **status proposed until the maintainer accepts it** — the agent
      records the observation and a recommendation; the maintainer decides the contract.
      Covers the observed behavior: Rust ignores SIGPIPE, so pipe errors arrive in-band as
      `ErrorKind::BrokenPipe`
- [x] Research-informed recommendation documented as the leading option: **in-band
      `BrokenPipe → exit 0` detection** (ecosystem contract converged by ripgrep/bat/fd/ast-grep)
      rather than resetting to `SIG_DFL` (Windows has no SIGPIPE, so in-band detection must
      exist anyway)
- [x] Decision notes the follow-up test shape (no PTY, no signal machinery): real OS pipe, close
      the read end early, spawn, assert the child did not exit 101 — precedents cited
- [x] Decision clarifies the profile distinction: a dev-profile panic gives 101, release gives
      134; ticket 10's never-101 assertion therefore also acts as a no-panic check under
      nextest's dev-profile binaries — exit-101 stays asserted only as "never" until this
      decision is acted on
- [x] Audit §10 potential-defects entry cross-referenced from the decision

## Comments

### Empirical Reproduction Results (2026-10-08)

1. **Test arrangement:**
   A temporary workspace populated with 5,000 indexed markdown notes was queried
   via `traces list` piped directly into `head -n 1`, terminating the reader pipe
   immediately after the first bullet line.

2. **Observed dev-profile binary (`target/debug/traces-pkm`):**
   - Output write triggers `std::io::ErrorKind::BrokenPipe` (OS error 32).
   - Rust standard library `print!` macro panics on write failure:
     `failed printing to stdout: Broken pipe (os error 32)`.
   - Dev profile default panic handler unwinds and exits with status **101**.

3. **Observed release binary (`target/release/traces-pkm`):**
   - Cargo manifest sets `panic = "abort"` in `[profile.release]`.
   - The same `print!` panic immediately invokes `abort()`, terminating via
     signal 6 (`SIGABRT`).
   - Under Unix shells, this evaluates to exit code `128 + 6 = 134` (Python subprocess
     `returncode = -6`).

### Decision Record: Proposed Contract for EPIPE / BrokenPipe

- **Status:** proposed (requires maintainer acceptance before implementation in a dedicated spec).
- **Audit Reference:** audit §10 Potential Defects ("EPIPE / exit-101 contract").

#### Recommendation & Rationale

1. **Leading Option: In-Band `BrokenPipe → exit 0` Detection**
   - Rust runtime ignores `SIGPIPE` by default (`signal(SIGPIPE, SIG_IGN)` in standard
     runtime init), causing broken pipe writes to surface as `io::ErrorKind::BrokenPipe`.
   - Rather than resetting the OS signal handler to `SIG_DFL` (which does not exist on
     Windows and leaves Windows pipe termination unhandled), the modern Rust CLI ecosystem
     (ripgrep #200, bat, fd, ast-grep #2887, rust#46016) has converged on handling
     `ErrorKind::BrokenPipe` in application write loops or standard output wrappers by
     flushing/exiting cleanly with status `0`.
   - For `traces-pkm`, primary output writers (`print!`/`println!` in CLI command runners)
     should route through a buffered stdout writer that swallows `ErrorKind::BrokenPipe`
     and exits cleanly (`exit 0`).

2. **Profile Distinction & Nextest Integration:**
   - Because nextest executes test binaries compiled with the `dev` profile, an unhandled
     stdout panic surfaces as exit 101 during test runs.
   - Ticket 10's process-contract assertion asserting "never 101" therefore functions
     effectively as a no-panic guard for all tested commands under nextest.

3. **Follow-up Test Shape (Future Implementation Spec):**
   - Test harness will use standard OS pipes (`std::process::Stdio::piped()`), close the
     reader end immediately upon reading initial output, and assert that the child
     process exits with code `0` (or `!exit_status.code().contains(101)`).
   - Requires no PTY emulator and no platform-specific signal delivery.

**Evidence:** audit §10 (EPIPE/exit-101 "open question, not a ticket — decide the intended
contract first"). Ecosystem references (ripgrep #200 design contract, ast-grep #2887 prior-art
survey, rust#46016 known footgun) come from **triage research external to the audit** — verify
against the upstream sources; they are not in the repo's evidence base.
**Defect class:** unwitnessed contract (decision debt).
