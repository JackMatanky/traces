# 08: E2E spawn-only invariant

**What to build:** Every test under the E2E suite exercises product behavior only through the
spawned binary — no in-process command runs, no exceptions — and mechanical checks keep it that
way. The three in-process `init` calls relocate (dialog-only assertions to an in-crate component
test, default path to a genuine spawn), the golden-path first step becomes a spawn, the
now-unused cwd guard is deleted rather than locked, and lint/CI enforce the layer boundary.

**Blocked by:** 07 (prerequisite slice: tests/common, fixture, convention).

**Category:** bug

**Status:** resolved

- [x] Zero in-process product-command runs remain under `tests/e2e/`: the three `Init.run` calls
      relocate — preset/custom-path dialog assertions become an in-crate component test
      (spawning can't reproduce them: the terminal dialog provider short-circuits on non-TTY);
      a genuine default-path `traces init` spawn **on null stdin** asserts success (exit 0 via
      the existing predicate; the exact numeric accessor arrives with ticket 10) +
      `initialised traces in …` stderr (first process-level coverage of the Init dispatch arm)
- [x] Golden-path first step becomes a spawn; its doc updated (honesty from ticket 02 preserved)
- [x] Relocated component test and spawned init test both carry the executes-vs-asserts note and
      a named defect class (layer-discipline / unexecuted-dispatch-arm)
- [x] E2E `CwdGuard` deleted (children already set their own cwd); this supersedes any mutex
      proposal — remove the mutator, don't lock it
- [x] Layer-enforcement mise task + CI step: `tests/e2e/` has zero `use traces_pkm::` and zero
      `std::process` outside `support.rs`; `tests/integration/` has zero `std::process::Command`
      / `CARGO_BIN_EXE`; `env::set_current_dir` absent from `tests/` (the in-crate cwd guard in
      `src` is the one legitimate use and stays)
- [x] clippy `disallow-methods` rejects `set_current_dir`, with a narrowly-scoped allow on the
      in-crate cwd guard only — the invariant is "no cwd mutation outside that guard", not
      "never anywhere" (US11 scopes this to in-test mutation)
- [x] `mise run test` green

## Comments

Resolved in commit `4c446830` (merged into `integration-e2e-suite`).
- Relocated preset/custom-path dialog assertions from `tests/e2e/init.rs` into `src/cli/init.rs::tests::run::scaffolds_preset_defaults_and_refuses_existing_traces_dir`.
- Implemented spawned default-path `traces init` on null stdin in `tests/e2e/init.rs::default_path_init_spawns_and_scaffolds_workspace`.
- Converted `golden_path.rs` step 1 from in-process `Init.run` to `sandbox.run(&["init"])`.
- Deleted E2E `CwdGuard` and its doc section.
- Added `tasks."check:layers"` in `mise.toml` enforcing:
  - zero `use traces_pkm::` in `tests/e2e/`
  - zero `std::process` in `tests/e2e/` outside `support.rs`
  - zero `std::process::Command` / `CARGO_BIN_EXE` in `tests/integration/`
  - zero `set_current_dir` in `tests/`
- Added `std::env::set_current_dir` to `disallowed-methods` in `clippy.toml` with narrowly scoped `#[expect(clippy::disallowed_methods, ...)]` on in-crate `CwdGuard::enter` and `drop`.
- Wired `check:layers` into `tasks.check`, `hk.pkl` `quality`, and `ci.yml`.
- All tests and layer checks green.

**Evidence:** audit §14 P0 items 4–5 (E2E invariant, layer checks); §6 (init never spawned;
cwd race); spec §E2E invariant.
**Defect class:** layer-discipline (conventions don't hold themselves).
