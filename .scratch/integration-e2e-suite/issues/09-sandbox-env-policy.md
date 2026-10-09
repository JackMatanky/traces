# 09: Sandbox environment policy & accessors

**What to build:** The E2E environment is an explicit policy, not a blanket wipe: a defined set
of sandbox-owned variables, a platform passthrough allowlist for runtime prerequisites, opt-in
variables for tests that study them, and an assertion that state actually lands inside the
sandbox. Hermeticity becomes enforced rather than assumed — and the non-Linux E2E CI job
(ticket 18) becomes unblocked.

**Category:** enhancement

**Blocked by:** 08 (both reshape the E2E harness; sequence avoids conflict), 10 (both edit
`tests/e2e/support.rs` — sandbox command builder vs `Run` accessors; spec plan order T3.1 →
T3.7 sequences the accessors first).

**Status:** resolved

- [x] Policy table implemented in the sandbox command builder:
      - sandbox-owned: `TRACES_STATE_DIR`, `XDG_CONFIG_HOME`, `APPDATA`, `LOCALAPPDATA`,
        `HOME`, `USERPROFILE`, `TZ=UTC`
      - passthrough allowlist: `PATH`, `SystemRoot`, `TEMP`, `TMP` (Windows entries cfg-gated)
      - opt-in: `TRACES_CEILING_DIRS`, `TRACES_IGNORED_DIRS` (tests that study them set them
        explicitly)
- [x] **Default-deny semantics**: unlisted variables are *dropped* (clear + allowlist), not
      inherit-and-override — a shell-exported variable cannot leak into any child unless
      allow-listed or explicitly opted-in by the test studying it
- [x] Windows config home isolated structurally here (`APPDATA` policy entry present,
      cfg-gated — the config resolver reads it; `XDG_CONFIG_HOME` alone is ignored there).
      Execution proof lands with ticket 18's non-Linux OS-matrix job; this ticket verifies the
      policy, not a Windows run
- [x] Sandbox accessors for state/config paths exist (test-crate-internal, zero `src/` changes)
- [x] Test asserting state and config files provably land inside the sandbox's accessor paths —
      `env_clear()` alone is not accepted as proof of hermeticity
- [x] New tests (state-lands-in-sandbox, isolation assertion) carry executes-vs-asserts notes
      and named defect classes (assumed-hermeticity)
- [x] Environment isolation becomes assertable for the first time: a test fails if
      `TRACES_STATE_DIR` plumbing breaks

## Comments

Resolved in commit `d3e2a2c3` (merged into `integration-e2e-suite`).
- `tests/e2e/support.rs`: `Sandbox::apply_env_policy` implements the policy
  (`Command::env_clear` + passthrough allowlist + sandbox-owned entries);
  `owned_env`/`passthrough_env` carry the cfg-gated Windows entries
  (`APPDATA`→config home, `LOCALAPPDATA`/`USERPROFILE`→state dir,
  `SystemRoot`/`TEMP`/`TMP` passthrough); Unix `HOME`→state dir anchors the
  `XDG_*` fallbacks. `Sandbox::run_with_envs` is the only sanctioned opt-in
  path (`TRACES_CEILING_DIRS`, `TRACES_IGNORED_DIRS` never pre-declared).
  Accessors: `state_dir()`, `config_home()`. Zero `src/` changes; all
  `std::process` use stays inside `support.rs` (`check:layers` green).
  New `tests/e2e/env_policy.rs` (7 tests, all carrying executes-vs-asserts
  notes and the assumed-hermeticity defect class):
  - `spawned_child_receives_exactly_the_declared_policy_table` — child env
    equals the declared table entry-for-entry; opt-in variables absent.
  - `policy_owns_state_config_and_timezone_entries` — owned entries carry
    sandbox accessor values; `TZ=UTC`.
  - `state_stores_materialize_inside_the_sandbox_state_accessor` —
    trusted-configs and tracked-configs populated under `state_dir()`.
  - `global_config_resolves_from_the_sandbox_config_home_accessor` — a
    malformed global config planted under `config_home()` fails `list` with
    `config_build_config_file_failed` + the planted `unknown_key` content;
    if `XDG_CONFIG_HOME` plumbing broke, the child would see no such file
    and this test fails. (Absolute-path assertions avoided: Miette wraps
    paths mid-component; discriminators are the diagnostic code and planted
    content, both proven wrap-safe.)
  - `state_store_follows_a_state_dir_override_not_the_default_accessor` —
    negative control proving the accessor assertions flip with the plumbing
    (AC: "a test fails if `TRACES_STATE_DIR` plumbing breaks").
  - `opt_in_ignored_dirs_prune_only_when_explicitly_declared` — `index`
    sees 2 files by default, 1 with `TRACES_IGNORED_DIRS=secret`.
  - `ambient_timezone_does_not_shift_naive_datetime_rendering` — a naive
    `2024-02-03T12:00:00` renders as the UTC instant (`1706961600`) through
    the spawned `template --dry-run` while the isolated parent runs with
    `TZ=Pacific/Honolulu`, proving the `TZ=UTC` policy row reaches the
    binary's date parsing (the env-table assertion alone cannot).
- Windows behavior verified structurally (policy table); execution proof
  lands with ticket 18's non-Linux job, per the AC.
- Gate: `check:layers`, `check:visibility` (empty diff), full suite
  (lib 3,150 / integration 43 / e2e 50 / doctests 83), clippy `-D warnings`,
  fmt — all green.

**Evidence:** audit §8 items 3, §6 (env isolation row: "assertion cannot even be written
today"), §14 P1.13 (env isolation as a harness contract); spec §Gap tests (environment policy
table).
**Defect class:** assumed-hermeticity; platform isolation gap.
