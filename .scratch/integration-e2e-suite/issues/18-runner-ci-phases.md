# 18: Runner & CI phases

**What to build:** The runner separates its phases by feature configuration and CI covers them:
unit on the production configuration, integration on the test feature, E2E on the shipped
feature-off binary, and doctests run with `--all-features`; CI requires one passing
doctest without pinning a count. The ticket also adds timeouts, a watch task
that runs the canonical suite, and the non-Linux E2E job unlocked by env policy.
Layer failures become legible in the checks list.

**Category:** enhancement

**Blocked by:** 03 (the per-target and nonzero-doctest assertions are part of
this ticket's acceptance). Per spec, runner/CI is otherwise parallel with the
env-policy work; AC5 alone completes when that ticket lands (no edge recorded
here for it).

**Status:** resolved

- [x] Canonical test task runs four phases: unit/component (production config), integration
      (test feature), E2E (shipped feature-off binary — black-box suite tests the shipped
      configuration, not a test-utils build), doctests explicitly `--all-features` (never
      default — test-support doctests would silently drop; the runner's feature configuration
      is the guard, while CI requires at least one passing doctest without pinning a count)
- [x] Watch task runs the canonical runner (today it runs bare `cargo test` — 0 integration
      tests + the wrong isolation model)
- [x] Broken documented `--feature default` promise fixed or removed (verified hard error today)
- [x] nextest timeouts (`slow-timeout` + terminate, `global-timeout`) and slow-test visibility
      enabled (today: zero timeouts, `status-level = "fail"` hides slow markers)
- [x] CI: default-configuration test run added — `cargo nextest run --no-default-features`
      (there is **no** `default` feature; `--features default` is a hard error — do not emit
      any `--features default` flag anywhere); test job split by suite; doctest and per-target
      assertions still green; the non-Linux E2E job lands **once ticket 09's env
      table exists** (09 runs in parallel — if it hasn't landed when everything else here is
      done, this one item waits on it and no other AC does)
- [x] Stale tooling pointers corrected where they lie to agents: the documented `--test
      init_cli` target that doesn't exist, and the mutants config path docs reference but is
      absent (audit §11 final row / §15 T6.7)
- [x] `test:unit` name and `-m <module>` filter behavior documented or fixed (module filter
      silently drops integration+e2e today)
- [x] Coverage/mutation decision recorded (gate vs advisory) + targeted post-redesign mutation
      runs against the modules new seams protect; **decision + local runs only — no CI gate in
      this ticket** (spec Out of Scope); note recorded that the mutation tool excludes
      the CLI module, so CLI-covered seams rely on the process-level tests from tickets 10–14

## Comments

Resolved in commit `b20c2326` (merged into `integration-e2e-suite`).

### Canonical runner (`.mise/tasks/test/_default`)

Four phases, each pinning its feature configuration:
1. **unit** — `--no-default-features -E 'kind(lib)'` (production config).
2. **integration** — `--features test-utils -E 'binary(integration)'`.
3. **e2e** — `--no-default-features -E 'binary(e2e)'` (shipped binary;
   verified: 3,150 / 43 / 55 tests respectively, all green).
4. **docs** — `mise run test:doc --feature all` (all-features; historical baseline:
   83 passing tests at ticket completion; the current guard is count-independent).
`--no-docs` skips phase 4; filter args forward to every nextest phase.

### Watch, feature flag, timeouts

- `watch` now runs `cargo watch -x check -s 'mise run test --no-docs'`
  (the canonical suite), not bare `cargo test`.
- `test:unit --feature none` added (production config); the old
  `--features default` promise (hard cargo error — no `default` feature
  exists) is gone from task help and code.
- `.config/nextest.toml`: `status-level = "slow"` (slow markers visible),
  `slow-timeout = { 30s, terminate-after = 2 }` (hang guard),
  `global-timeout = "20m"`.

### CI (`.github/workflows/ci.yml`)

Single test job split into `test-unit` (production config),
`test-integration` (`test-utils`), `test-e2e` (matrix ubuntu/macos/windows —
unlocked by ticket 09's env policy), and `test-doctests` (per-target non-empty
assertions plus at least one passing doctest). No `--features default` flag emitted
anywhere. YAML validated.

### Stale pointers

- `--test init_cli` USAGE examples (test + test:unit) replaced with the real
  `integration`/`e2e` targets.
- mise.toml coverage comment no longer references the absent
  `.cargo/mutants.toml`.
- `test:unit -m` documented as lib-scoped by definition (capability files
  carry no module paths); suite scoping via `-E 'binary(...)'`.

### Coverage/mutation decision: ADVISORY (no CI gate — spec Out of Scope)

Targeted runs via `test:mutants` (engine mutarust), module
`src/index/refresh.rs` (the refresh seam tickets 04/12 pin), 30 mutants:

| Run | Killed | Escaped | Errored | Skipped | Score |
|---|---|---|---|---|---|
| fixed 120s timeout | 1 | 0 | 24 | 5 | (invalid — every error a test timeout) |
| disk-contention rerun | 3 | 0 | 24 | 3 | (invalid — ENOSPC from concurrent runs) |
| adaptive timeout, clean | 5 | 7 | 10 | 8 | 76.67% |

The 10 errored mutants are all 60s test timeouts — the full-suite cycle
(rebuild + lib/integration/e2e/doctests) exceeds the adaptive coefficient;
a future gate run needs `--exec-timeout` ≥ 300 or `--per-test` mode.

**Escaped mutants (7) — advisory findings, each maps to an under-exercised
path, recorded for a future spec:**
1. `is_paths_unchanged` → `&& true`: no test covers a **mixed
   upsert+delete** delta (delete-only fixtures make `all()` vacuously true
   on both sides). Ticket 12's E2E covered delete and content-edit
   separately but not combined.
2-4. `merge_refreshed_notes` `take(1)` → `take(0)` / `take(2)` (3 sites):
   the Less/Greater merge arms never fire with distinct persisted-vs-modified
   paths in any fixture — merge-arm fixtures are equal-path-only.
5-7. two `statement/remove` + one more: see `mutants.out`-class diffs
   (log archived at `/tmp/t18-mut4.log` during the session).

Per the audit note, the mutation engine excludes `src/cli/` — CLI-covered
seams (failure stages, argv contracts) rely on the process-level tests from
tickets 10–14 and are invisible to this tool.

**After-state:** no test changes; runner/CI config only. Gate green
(`check:layers`, `check:visibility`, clippy `-D warnings`, fmt; canonical
runner + `--feature none` exercised locally).

**Evidence:** audit §4 (runner composition, `mise watch` wrong on two counts), §13 F1–F8;
spec §Runner & CI.
**Defect class:** shipped-config-untested; silent-skip; wrong-local-command.
