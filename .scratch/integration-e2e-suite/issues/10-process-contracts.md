# 10: Process contracts — exit codes, argv, streams

**What to build:** The process-boundary contract exists as assertions for the first time: the
harness can read numeric exit codes and exact streams, argv failures observe clap's
structurally-unreachable-in-crate exit 2 where it actually runs, representative paths are pinned
to never-101, stream splits are asserted exactly (not `contains`), and the in-crate interrupt
mapping link gains its missing unit. The PTY/130 question is closed with a recorded rejection.

**Blocked by:** 08 (new process-contract tests land in an invariant-conformant E2E suite).

**Category:** enhancement

**Status:** resolved

- [x] Harness gains numeric code accessor, failure predicate, and exact-stdout/stderr
      assertions — test-crate-internal only, zero `src/` changes (fixture additions to exported
      types are forbidden; harness growth is not a surface change)
- [x] Exit codes asserted: success → 0, domain failure → 1 (e.g. an untrusted root), argv
      failure → 2; `--help` / `--version` → 0 with correct stdout
- [x] Argv contract via spawn (unknown subcommand, missing required flag e.g. `table` without
      `--column`, invalid value e.g. `completions --shell tcsh`) — each exit 2 + usage on
      stderr; these are unreachable in-crate (`Cli::parse` exits, `try_parse_from` returns)
- [x] Representative valid and error paths assert **never 101** (no `ExitCode::from(101)` path
      exists in the binary; release `panic = "abort"` → 134 — and because nextest runs
      dev-profile binaries where a panic *would* give 101, this doubles as a no-panic check;
      ticket 05 owns the EPIPE/panic contract decision)
- [x] Stream contracts (exact, not `contains`): `trust list` = `path\tstate`, `trust --show` =
      bare root, `trust clean`, `template --list`, `completions --list-templates`; mutating
      commands (`trust`, `untrust`, `index`, `template -i … --no-input`) assert
      `stdout == ""` with status on stderr; `index` asserts `indexed N file(s)` on stderr;
      `trust --all` + companion `untrust --all` with the count asserted via a follow-up spawned
      `trust list`
- [x] Tests land in `tests/e2e/process_contract.rs` (audit §12's first candidate; spec: new gap
      tests are written directly into their final capability files) — not appended to the legacy
      dispatch catch-all; ticket 19 relocates **legacy** tests only, never these
- [x] In-crate unit: `Cli::run → Aborted(Interrupted)` mapping (today only `Cancelled` is
      tested); the chain `DialogError::UserInterrupted` → `Aborted(Interrupted)` → `main`
      documented in the test doc
- [x] Decision note recorded: **process-level 130 not approved** (the spec's condition — an
      approved portable PTY mechanism — is unmet; a PTY dependency remains out of scope; the
      question reopens only if that condition is met): with no signal handler in the binary, the
      only faithful injection is raw-mode PTY byte (`\x03`) delivery; no surveyed Rust project
      tests Ctrl-C through a PTY (cargo asserts `!success()` only; mise's exact-130 relies on
      its own signal handler), PTY suites have documented flake/quarantine history, and
      `ExitStatus::code()==Some(130)` remains distinguishable from signal death if ever
      revisited. The PTY ecosystem references are **triage research external to the audit** —
      verify against upstream sources
- [x] New tests each carry the executes-vs-asserts note and named defect class

## Comments

Resolved in commit `ea40a75d` (merged into `integration-e2e-suite`).
- `tests/e2e/support.rs::Run` gained `code()`, `is_failure()`, `assert_code(n)`, `assert_success()`, `assert_never_exit_101()`, `assert_stdout_eq()`, and `assert_stderr_eq()`. Zero `src/` changes.
- Implemented `tests/e2e/process_contract.rs` wired into `tests/e2e.rs`:
  - `exit_code_zero_for_help_and_version` (0 + exact stdout/stderr)
  - `argv_failure_exits_code_two_on_invalid_command_or_flags` (2 + exact streams for unknown subcommand, missing required flag, invalid enum value)
  - `domain_failure_exits_code_one_for_untrusted_root` (1 + never 101)
  - `stream_contracts_exact_outputs_and_mutating_empty_stdout` (exact stdout/stderr splits, including `template -i alpha --no-input` writing `wrote <path>/alpha.md` to stderr)
  - `trust_all_and_untrust_all_count_asserted_via_trust_list`
- `Cli::run → Aborted(Interrupted)` verified in-crate via `user_interrupted_during_template_picker_returns_aborted_interrupted` in `src/cli/mod.rs`.
- Recorded decision: process-level exit 130 not approved (no PTY crate dependency permitted); exit-130 remains asserted as in-crate mapping only.
- All tests carry executes-vs-asserts notes and named defect classes.

**Evidence:** audit §10 E-3 (clap exit-2 unreachable in-crate), §6 exit-code/stream rows
(five zero-assertion stdout commands incl. `trust clean`), §14 P1.6 + P1.11; spec §Gap tests
(exit codes, conditional 130), §Testing Decisions; PTY non-approval research (triage-time,
external to the audit: cargo `death.rs`, mise `test_exit_status`, expectrl/deno flake issues —
verify upstream).
**Defect class:** unwitnessed contract (zero numeric exit assertions existed).
