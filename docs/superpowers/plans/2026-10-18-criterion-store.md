# Criterion Durable Store (`.criterion`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make it structurally impossible for any tool (a `mise run clean`, a raw `cargo clean`, cargo-sweep, a CI cache wipe, muscle memory) to delete Criterion benchmark history (1.9 GB, 116k `estimates.json` files, all named critcmp baselines).

**Architecture:** Benchmark history moves out of `target/` into a durable store `<repo>/.criterion/` (repo root, same volume, gitignored). `target/criterion` becomes a *relative symlink* (`../.criterion`) that Criterion and `critcmp` read/write through transparently. Any cleaner may delete the link; no one can delete the data through it. A shared helper (`.mise/scripts/criterion_store.sh`) owns the link contract: migrate stray real directories into the store, park "fork" trees (real dir reappearing after a raw `cargo clean`) non-destructively outside `target/`, and recreate the link after cleans. The existing mv-aside/trap dance in `clean/_default`, which has a data-destroying failure path (`trap` does `mv … || true` then unconditional `rm -rf` of the only copy), is deleted entirely: `clean` now has nothing to restore, by construction.

**Tech Stack:** Bash 3.2-compatible (macOS `/bin/bash`), mise file tasks (`#MISE`/`#USAGE` headers), sandbox regression selftest with a stubbed `cargo`.

**Why (history):** Pre-2026-09-25 `[tasks.clean]` was an unconditional `cargo clean` that destroyed all baselines; oldest surviving criterion data is 2026-09-24. The Sep-25 mv-aside fix protects only one code path and can still destroy data on restore failure. This plan removes the entire class of failure.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `.mise/scripts/criterion_store.sh` | Create | Single source of truth for the store↔link contract (`criterion_link_ok`, `ensure_criterion_store`, `criterion_dry_run_note`). Sourced via `MISE_PROJECT_ROOT` (same pattern as `size_gate.sh`). |
| `.mise/tasks/selftest` | Create | Sandbox regression suite (T1-T7) runnable as `mise run selftest`; the regression tripwire against ever re-introducing a wiping `clean`. |
| `.mise/tasks/clean/_default` | Modify | Delete the backup/trap/restore dance in `exec_target_clean`; call the helper before/after `cargo clean`; fix `--all` long_help. |
| `.mise/tasks/bench/_default` | Modify | Source helper, call `ensure_criterion_store` at `main()` start (guard against raw `cargo clean` forks); fix `snapshot_baseline` `cp -R` nesting bug. |
| `.mise/tasks/bench/report` | Modify | Call `ensure_criterion_store` at `main()` start so the summary sees history again after any full clean. |
| `.gitignore` | Modify | Ignore `/.criterion/` and `/.criterion.fork.*`. |
| Live repo data | One-time migration | `mv target/criterion .criterion && ln -s ../.criterion target/criterion` (done by hand in Task 5, verified by counts). |

## Contract (invariants every task must uphold)

1. `.criterion/` at repo root is the **only** copy of benchmark history. Nothing in any task ever deletes it.
2. `<target>/criterion` is always a relative symlink whose text climbs one `..` per target-dir level (`../.criterion` for `target`, `../../.criterion` for `out/release`): relative text survives repo moves. Absolute target dirs and `..` components are refused by the helper.
3. A real (non-symlink) directory at `target/criterion` is either migrated into the store (first time) or parked as `.criterion.fork.<UTC>` at repo root (fork; outside `target/` so the ensuing `cargo clean` cannot eat it), always non-destructive, always with a warning.
4. Dry-run modes never mutate the filesystem.
5. `bench` snapshots replace an existing same-name baseline; they never nest into it.

## Accepted caveats (document, don't fix)

- **Git worktrees** get their own untracked `.criterion/` (gitignored); first `bench`/`clean` there migrates that worktree's fresh criterion tree. History is not shared across worktrees.
- **`cargo sweep --max-size` sizing** (walkdir, no nested-symlink follow) no longer counts criterion toward the target size, arguably a fix: the 1.9 GB store can no longer make the size floor unreachable.
- **`bench:model` invoked directly** (`mise run bench:model`) does not run the guard; it reads `target/criterion` (works through the link when present). First `bench` or `bench:report` restores the link after a raw clean.
- `critcmp` and Criterion resolve through the symlink (walkdir follows a symlinked root; `std::fs` path components follow). Verified in Task 5 with the real 1.9 GB tree.

---

### Task 1: Store helper + selftest (store-contract tests T1-T5, T8-T13)

**Files:**
- Create: `.mise/scripts/criterion_store.sh`
- Create: `.mise/tasks/selftest`

- [ ] **Step 1: Write the failing selftest**

Create `.mise/tasks/selftest` (executable: `chmod +x`):

```bash
#!/bin/bash
#MISE description="Sandbox regression tests for mise task scripts (criterion store contract)"
#MISE outputs=[]
#MISE cache = { enabled = false }
# Selftest harness for destructive-task safety contracts. Builds throwaway
# sandboxes under $TMPDIR, runs the REAL task scripts against a fake crate,
# and asserts filesystem outcomes. No framework: repo precedent is sandbox +
# grep (docs/digests/plans/2026-09-27-quality-gates.md).

# shellcheck disable=SC2034,SC2154  # CLEAN_TASK/BENCH_TASK used by T6/T7 in later tasks; usage_* reserved
set -euo pipefail

ROOT="${MISE_PROJECT_ROOT:?}"
CLEAN_TASK="${ROOT}/.mise/tasks/clean/_default"
BENCH_TASK="${ROOT}/.mise/tasks/bench/_default"

pass_count=0
fail_count=0

#######################################
# Assert a command succeeds; print ok/FAIL line.
# Arguments: description, then command + args.
#######################################
assert() {
  local desc="${1}"; shift
  if "$@" >/dev/null 2>&1; then
    pass_count=$((pass_count + 1)); printf 'ok   - %s\n' "${desc}"
  else
    fail_count=$((fail_count + 1)); printf 'FAIL - %s\n' "${desc}" >&2
  fi
}

#######################################
# Create a minimal fake cargo crate sandbox with BOTH sourced helpers
# copied in (clean/_default unconditionally sources size_gate.sh, so a
# sandbox missing it would abort the suite under set -e instead of
# recording failures).
# Outputs: sandbox path on stdout.
#######################################
new_sandbox() {
  local sbx
  sbx="$(mktemp -d -t mise_selftest.XXXXXX)"
  mkdir -p "${sbx}/src" "${sbx}/.mise/scripts" "${sbx}/benches"
  printf '[package]\nname="sbx"\nversion="0.1.0"\nedition="2021"\n' \
    > "${sbx}/Cargo.toml"
  echo 'pub fn f() {}' > "${sbx}/src/lib.rs"
  echo '#[cfg(test)] fn t() {}' > "${sbx}/benches/sbx_bench.rs"
  cp "${ROOT}/.mise/scripts/"*.sh "${sbx}/.mise/scripts/"
  # Some task paths call git (baseline naming); give the sandbox a repo
  # so those paths behave like production instead of erroring.
  git -C "${sbx}" init -q 2>/dev/null || true
  git -C "${sbx}" -c user.email=selftest@test -c user.name=selftest \
    commit -q --allow-empty -m init 2>/dev/null || true
  printf '%s' "${sbx}"
}

#######################################
# Plant a criterion tree with a marker file at the leaf.
# Arguments: sandbox dir, marker content.
#######################################
seed_criterion() {
  mkdir -p "${1}/target/criterion/g1/100/new"
  printf '%s\n' "${2}" > "${1}/target/criterion/g1/100/new/estimates.json"
}

#######################################
# Invoke the helper's ensure function inside a sandbox.
# Arguments: sandbox dir, optional CRITERION_TARGET_DIR (default target).
#######################################
run_ensure() {
  # -e -u -o pipefail: match production caller semantics (clean/bench run
  # under set -euo pipefail); the suite must exercise the same failure paths.
  ( cd "${1}" && MISE_PROJECT_ROOT="${1}" \
      CRITERION_TARGET_DIR="${2:-target}" \
      bash -e -u -o pipefail -c \
        'source .mise/scripts/criterion_store.sh; ensure_criterion_store' )
}

#######################################
# T1: first migration, real dir, no store -> store + symlink.
#######################################
test_migration() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" pre-migration
  run_ensure "${sbx}"
  assert "T1 store created from real dir" \
    test -f "${sbx}/.criterion/g1/100/new/estimates.json"
  assert "T1 target/criterion is a symlink" test -L "${sbx}/target/criterion"
  assert "T1 link text is ../.criterion" \
    test "$(readlink "${sbx}/target/criterion")" = "../.criterion"
  assert "T1 marker survives via link" \
    grep -q pre-migration "${sbx}/target/criterion/g1/100/new/estimates.json"
}

#######################################
# T2: idempotence, second ensure on a correct link is a no-op.
#######################################
test_idempotent() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" keep
  run_ensure "${sbx}"
  run_ensure "${sbx}"
  assert "T2 still a symlink after re-ensure" test -L "${sbx}/target/criterion"
  assert "T2 data intact" \
    grep -q keep "${sbx}/.criterion/g1/100/new/estimates.json"
}

#######################################
# T3: fork, real dir reappears while store exists -> parked, never merged
# or deleted; link rebuilt; store untouched.
#######################################
test_fork_parked() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" first-run
  run_ensure "${sbx}"                       # migrate: store has first-run
  rm "${sbx}/target/criterion"              # simulate raw cargo clean link removal
  mkdir -p "${sbx}/target/criterion/g2/1/new"
  echo fork-data > "${sbx}/target/criterion/g2/1/new/estimates.json"
  run_ensure "${sbx}"                       # fork detected
  assert "T3 store still has original data" \
    grep -q first-run "${sbx}/.criterion/g1/100/new/estimates.json"
  assert "T3 link rebuilt" test -L "${sbx}/target/criterion"
  local fork
  fork="$(find "${sbx}" -maxdepth 1 -type d -name '.criterion.fork.*' | head -1)"
  assert "T3 fork parked outside target" \
    grep -q fork-data "${fork}/g2/1/new/estimates.json"
}

#######################################
# T4: wrong/broken symlink -> replaced with the canonical link.
#######################################
test_wrong_link() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" data
  run_ensure "${sbx}"
  rm "${sbx}/target/criterion"
  ln -s /nonexistent/elsewhere "${sbx}/target/criterion"
  run_ensure "${sbx}"
  assert "T4 link repointed" \
    test "$(readlink "${sbx}/target/criterion")" = "../.criterion"
}

#######################################
# T5: link missing entirely (fresh target/ after cargo clean) -> recreated.
################################=======
test_recreate_missing() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" data
  run_ensure "${sbx}"
  rm -rf "${sbx}/target"                    # simulate full cargo clean
  run_ensure "${sbx}"
  assert "T5 link recreated after target wiped" \
    test -L "${sbx}/target/criterion"
  assert "T5 history visible through recreated link" \
    grep -q data "${sbx}/target/criterion/g1/100/new/estimates.json"
}

#######################################
# T8: non-default target dir (custom --target-dir) -> the REAL override
# path is the positional argument; regression for the bug class where the
# link path was computed once at source time and an override would
# silently protect the wrong directory (would leave custom target dirs
# unprotected).
#######################################
test_custom_target_dir() {
  local sbx; sbx="$(new_sandbox)"
  mkdir -p "${sbx}/alt/criterion/g1/100/new"
  printf '%s\n' alt-dir > "${sbx}/alt/criterion/g1/100/new/estimates.json"
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" \
      bash -e -u -o pipefail -c \
        'source .mise/scripts/criterion_store.sh; ensure_criterion_store alt' )
  assert "T8 store created from custom target dir" \
    test -f "${sbx}/.criterion/g1/100/new/estimates.json"
  assert "T8 alt/criterion is a symlink" test -L "${sbx}/alt/criterion"
  assert "T8 link text is ../.criterion" \
    test "$(readlink "${sbx}/alt/criterion")" = "../.criterion"
}

#######################################
# T9: nested target dir (depth 2) -> link text climbs two levels; without
# this the hardcoded ../.criterion would resolve to out/.criterion and
# Criterion would silently grow a second history tree outside the store.
#######################################
test_nested_target_dir() {
  local sbx; sbx="$(new_sandbox)"
  mkdir -p "${sbx}/out/release/criterion/g1/100/new"
  printf '%s\n' nested > "${sbx}/out/release/criterion/g1/100/new/estimates.json"
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" \
      bash -e -u -o pipefail -c \
        'source .mise/scripts/criterion_store.sh; ensure_criterion_store out/release' )
  assert "T9 store created from nested target dir" \
    test -f "${sbx}/.criterion/g1/100/new/estimates.json"
  assert "T9 nested link is a symlink" test -L "${sbx}/out/release/criterion"
  assert "T9 link text climbs two levels" \
    test "$(readlink "${sbx}/out/release/criterion")" = "../../.criterion"
}

#######################################
# T10: unexpected regular file at the link path -> warn, leave untouched,
# still exit 0 (resilient no-op, not a silent handled-state).
#######################################
test_regular_file_at_link() {
  local sbx; sbx="$(new_sandbox)"
  mkdir -p "${sbx}/target"
  echo not-a-dir > "${sbx}/target/criterion"
  local status=0 output
  output="$( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" \
    bash -e -u -o pipefail -c \
      'source .mise/scripts/criterion_store.sh; ensure_criterion_store' \
    2>&1 )" || status=$?
  assert "T10 ensure exits zero on unexpected file" test "${status}" -eq 0
  assert "T10 file left untouched" grep -q not-a-dir "${sbx}/target/criterion"
  # shellcheck disable=SC2016  # $1 expands in the inner bash, not here (intentional)
  assert "T10 warning emitted" \
    bash -c 'grep -q "unexpected non-directory" <<< "$1"' _ "${output}"
}

#######################################
# T11: CRITERION_TARGET_DIR env-var default (no positional arg) is honored
# when set before sourcing.
#######################################
test_target_dir_env_var() {
  local sbx; sbx="$(new_sandbox)"
  mkdir -p "${sbx}/alt/criterion/g1/100/new"
  printf '%s\n' env-dir > "${sbx}/alt/criterion/g1/100/new/estimates.json"
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" CRITERION_TARGET_DIR=alt \
      bash -e -u -o pipefail -c \
        'source .mise/scripts/criterion_store.sh; ensure_criterion_store' )
  assert "T11 link created via CRITERION_TARGET_DIR env" \
    test -L "${sbx}/alt/criterion"
  assert "T11 link text is ../.criterion" \
    test "$(readlink "${sbx}/alt/criterion")" = "../.criterion"
}

#######################################
# T12: path normalization and refusals. `./target` and `out//release`
# normalize to the same depths as `target` / `out/release` (a naive count
# would climb one level too many and orphan history); absolute, `..`-containing,
# and repo-root target dirs are refused with status 1.
#######################################
test_path_normalization_and_refusals() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" data
  local status=0
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" \
      bash -e -u -o pipefail -c \
        'source .mise/scripts/criterion_store.sh; ensure_criterion_store ./target' )
  assert "T12 ./target normalized to one level" \
    test "$(readlink "${sbx}/target/criterion")" = "../.criterion"
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" \
      bash -e -u -o pipefail -c \
        'source .mise/scripts/criterion_store.sh; ensure_criterion_store out//release' ) \
    2>/dev/null
  assert "T12 duplicate slashes normalized to two levels" \
    test "$(readlink "${sbx}/out/release/criterion")" = "../../.criterion"
  status=0
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" \
      bash -e -u -o pipefail -c \
        'source .mise/scripts/criterion_store.sh; ensure_criterion_store /tmp/evil' \
      ) 2>/dev/null || status=$?
  assert "T12 absolute target dir refused" test "${status}" -eq 1
  status=0
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" \
      bash -e -u -o pipefail -c \
        'source .mise/scripts/criterion_store.sh; ensure_criterion_store ../evil' \
      ) 2>/dev/null || status=$?
  assert "T12 dotdot component refused" test "${status}" -eq 1
  status=0
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" \
      bash -e -u -o pipefail -c \
        'source .mise/scripts/criterion_store.sh; ensure_criterion_store .' \
      ) 2>/dev/null || status=$?
  assert "T12 repo-root target dir refused" test "${status}" -eq 1
}

#######################################
# T13: the store is anchored at MISE_PROJECT_ROOT; running ensure from any
# other cwd must refuse (it would reconcile a foreign target dir and could
# move foreign data into this repo's store).
#######################################
test_cwd_must_be_project_root() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" data
  local status=0
  ( cd "${sbx}/src" && MISE_PROJECT_ROOT="${sbx}" \
      bash -e -u -o pipefail -c \
        "source ${sbx}/.mise/scripts/criterion_store.sh; ensure_criterion_store" \
      ) 2>/dev/null || status=$?
  assert "T13 refuses to run outside project root" test "${status}" -eq 1
  assert "T13 store never created" test ! -e "${sbx}/.criterion"
}

main() {
  test_migration
  test_idempotent
  test_fork_parked
  test_wrong_link
  test_recreate_missing
  test_custom_target_dir
  test_nested_target_dir
  test_regular_file_at_link
  test_target_dir_env_var
  test_path_normalization_and_refusals
  test_cwd_must_be_project_root

  printf '\nselftest: %d passed, %d failed\n' "${pass_count}" "${fail_count}"
  [[ "${fail_count}" -eq 0 ]]
}

main "$@"
```

- [ ] **Step 2: Run selftest; verify it fails (helper missing)**

Run: `mise run selftest` (or `MISE_PROJECT_ROOT=$PWD bash .mise/tasks/selftest`)
Expected: non-zero exit. The suite aborts under `set -e` when T1's `run_ensure` sources the not-yet-existing `.mise/scripts/criterion_store.sh` (bash prints `No such file or directory`). Red satisfied.

- [ ] **Step 3: Create the store helper**

`.mise/scripts/criterion_store.sh`:

```bash
#!/bin/bash
# Helpers for the durable Criterion benchmark store.
#
# Benchmark history (named critcmp baselines, rolling base/, HTML reports)
# must survive every cleaner that exists: `mise run clean`, raw `cargo clean`,
# cargo-sweep, CI cache wipes, muscle memory. It therefore never lives under
# `target/`: the data tree is `<repo>/.criterion/`, and `target/criterion` is
# a RELATIVE symlink that any tool may delete without touching a byte of
# history.
#
# Contract:
#   1. `.criterion/` is the only copy of history. Nothing here deletes it.
#   2. `<target>/criterion` is always a relative symlink whose text climbs
#      one `..` per meaningful path component (`../.criterion` for `target`,
#      `../../.criterion` for `out/release`). Duplicate slashes, `./`
#      components, and trailing slashes are normalized away; absolute
#      target dirs, `..` components, and repo-root (zero-component) target
#      dirs are REFUSED (their resolution depends on layout the store
#      cannot verify).
#   3. A REAL directory at `<target>/criterion` is migrated into the store
#      when the store does not exist yet, or parked as `.criterion.fork.<UTC>`
#      at the repo root (outside target/, so a following `cargo clean` cannot
#      eat it) when the store already exists. Always non-destructive; fork
#      parking prints a warning for manual merging.
#
# Not a task: mise only discovers tasks under the configured file-task
# directories (here `.mise/tasks/`, per docs/refs/mise_tasks/05_file_tasks.md).
# Task `run` bodies source this file via `MISE_PROJECT_ROOT` (same pattern
# as size_gate.sh). Sourced by: clean/_default, bench/_default, bench/report.
#
# Globals:
#   MISE_PROJECT_ROOT  Required; anchors the store at the repo root.
#   CRITERION_TARGET_DIR  Default cargo target dir when no argument is
#     given (default: `target`). Set as an env var before sourcing, or
#     pass the target dir explicitly:
#     `ensure_criterion_store "${TARGET_DIR}"`.
# Arguments (public functions):
#   Optional first argument: the cargo target dir (relative to cwd), e.g.
#   `ensure_criterion_store alt`. Defaults to CRITERION_TARGET_DIR/target.
#   The link path is re-derived from it ON EVERY CALL: nothing is expanded
#   once at source time, so an override can never be silently ignored.

CRITERION_STORE="${MISE_PROJECT_ROOT:?}/.criterion"
CRITERION_TARGET_DIR="${CRITERION_TARGET_DIR:-target}"

#######################################
# Resolve the criterion link path for a target dir (re-derived per call).
# Arguments:
#   Optional target dir (default: CRITERION_TARGET_DIR).
# Outputs:
#   The link path on stdout.
#######################################
criterion_link_path() {
  printf '%s/criterion' "${1:-${CRITERION_TARGET_DIR:-target}}"
}

#######################################
# Resolve the relative link text from a target dir back to the store: one
# `..` per meaningful path component. Absolute paths, `..` components, and
# repo-root (zero-component) target dirs are refused. Duplicate slashes,
# `./` components, and trailing slashes are normalized away, so `./target`
# and `target` produce the same text (a naive component count would let
# `./target` climb one level too many and silently orphan history).
# Arguments:
#   Target dir (relative, no `..` components).
# Outputs:
#   Link text on stdout (`../.criterion`, `../../.criterion`, ...).
# Returns:
#   1 with a STDERR message for refused inputs.
#######################################
criterion_link_text() {
  local target_dir="${1:-${CRITERION_TARGET_DIR:-target}}"
  case "${target_dir}" in
    /*)
      printf 'criterion store: refusing absolute target dir: %s\n' \
        "${target_dir}" >&2
      return 1
      ;;
  esac
  local rel="" part depth=0 refused=""
  local IFS=/
  # set -f: a glob metacharacter in the target dir must not expand to
  # unrelated pathnames (which would corrupt the climb count).
  set -f
  # shellcheck disable=SC2034  # loop var splits components; body only grows rel
  for part in ${target_dir}; do
    case "${part}" in
      '' | .)
        continue
        ;;
      ..)
        refused="${target_dir}"
        break
        ;;
    esac
    depth=$((depth + 1))
    rel="../${rel}"
  done
  set +f
  if [[ -n "${refused}" ]]; then
    printf 'criterion store: refusing .. in target dir: %s\n' \
      "${refused}" >&2
    return 1
  fi
  if [[ "${depth}" -eq 0 ]]; then
    printf 'criterion store: refusing target dir at repo root: %s\n' \
      "${target_dir}" >&2
    return 1
  fi
  printf '%s.criterion' "${rel}"
}

#######################################
# Returns 0 when <target>/criterion is a symlink with the canonical target text.
# Note: returns 1 on the false path; under errexit call this inside `if`.
# Arguments:
#   Optional target dir (default: CRITERION_TARGET_DIR).
# Outputs:
#   None
# Returns:
#   0 when the link is correct, 1 otherwise.
#######################################
criterion_link_ok() {
  local link; link="$(criterion_link_path "${1:-}")"
  local text; text="$(criterion_link_text "${1:-}")" || return 1
  [[ -L "${link}" ]] && [[ "$(readlink "${link}")" == "${text}" ]]
}

#######################################
# Enforce the store contract (see header). Mutates the filesystem; never
# touches the contents of CRITERION_STORE itself.
# Arguments:
#   Optional target dir (default: CRITERION_TARGET_DIR).
# Outputs:
#   Fork-parking / unexpected-state warning to STDERR when applicable.
# Returns:
#   0 on all handled states; 1 when the target dir is refused.
#######################################
ensure_criterion_store() {
  local target_dir="${1:-${CRITERION_TARGET_DIR:-target}}"
  local link="${target_dir}/criterion"
  local link_text; link_text="$(criterion_link_text "${target_dir}")" \
    || return 1

  # Case 0: the store is anchored at MISE_PROJECT_ROOT while the link path
  # is cwd-relative; a caller standing outside the project root would
  # reconcile a foreign target dir (and could move foreign data into this
  # repo's store). Refuse loudly. Capture-then-check so a failed capture
  # fails CLOSED (empty side => refuse), never open.
  local cwd root
  cwd="$(pwd -P 2>/dev/null)" || cwd=""
  root="$(cd "${MISE_PROJECT_ROOT:?}" 2>/dev/null && pwd -P)" || root=""
  if [[ -z "${cwd}" || -z "${root}" || "${cwd}" != "${root}" ]]; then
    printf 'criterion store: cwd (%s) is not the project root (%s); refusing\n' \
      "${cwd}" "${MISE_PROJECT_ROOT}" >&2
    return 1
  fi

  # Case 1: link already correct, nothing to do.
  if criterion_link_ok "${target_dir}"; then
    return 0
  fi

  # Case 2: nothing at the link path (fresh target/ after cargo clean).
  if [[ ! -e "${link}" && ! -L "${link}" ]]; then
    if [[ -d "${CRITERION_STORE}" ]]; then
      mkdir -p "${target_dir}"
      ln -s "${link_text}" "${link}"
    fi
    return 0
  fi

  # Case 3: wrong symlink (broken or foreign target), replace it.
  if [[ -L "${link}" ]]; then
    rm "${link}"
    if [[ -d "${CRITERION_STORE}" ]]; then
      ln -s "${link_text}" "${link}"
    fi
    return 0
  fi

  # Case 4: REAL directory at the link path.
  if [[ -d "${link}" ]]; then
    if [[ ! -e "${CRITERION_STORE}" ]]; then
      # First migration: the real tree becomes the store (same-volume mv).
      mv "${link}" "${CRITERION_STORE}"
    else
      # Fork (raw `cargo bench` after a raw `cargo clean`): park the stray
      # tree OUTSIDE target/ and keep it; never merge, never delete.
      # Counter in the loop guarantees progress on repeated collisions.
      local fork n=0
      fork="${CRITERION_STORE}.fork.$(date -u +%Y%m%dT%H%M%SZ)"
      while [[ -e "${fork}" ]]; do
        n=$((n + 1))
        fork="${CRITERION_STORE}.fork.$(date -u +%Y%m%dT%H%M%SZ).$$.${n}"
      done
      mv "${link}" "${fork}"
      printf 'criterion store: parked stray %s -> %s (merge manually if wanted)\n' \
        "${link}" "${fork}" >&2
    fi
    mkdir -p "${target_dir}"
    ln -s "${link_text}" "${link}"
    return 0
  fi

  # Case 5: unexpected non-directory at the link path: leave it alone but
  # surface it (a contract violation must not look handled).
  printf 'criterion store: unexpected non-directory at %s; left untouched\n' \
    "${link}" >&2
  return 0
}

#######################################
# Dry-run reporting: describe what ensure_criterion_store WOULD do, without
# mutating anything (dry-run rule of the clean task).
# Arguments:
#   Optional target dir (default: CRITERION_TARGET_DIR).
# Outputs:
#   One or two preview lines to STDOUT.
# Returns:
#   0 always.
#######################################
criterion_dry_run_note() {
  local target_dir="${1:-${CRITERION_TARGET_DIR:-target}}"
  local link="${target_dir}/criterion"
  local link_text; link_text="$(criterion_link_text "${target_dir}")" \
    || link_text="../.criterion"
  if [[ -d "${link}" && ! -L "${link}" ]]; then
    printf 'clean: would move stray %s into the .criterion/ store first\n' \
      "${link}"
  elif [[ ! -e "${link}" && ! -L "${link}" \
    && -d "${CRITERION_STORE}" ]]; then
    printf 'clean: would restore %s -> %s after cleaning\n' \
      "${link}" "${link_text}"
  fi
  printf 'clean: benchmark history lives in .criterion/ and is never deleted\n'
}
```

- [ ] **Step 4: Run selftest; verify T1-T5 pass**

Run: `mise run selftest`
Expected output ending: `selftest: 30 passed, 0 failed` (T1×4, T2×2, T3×3, T4×1, T5×2, T8×3, T9×3, T10×3, T11×2, T12×5, T13×2), exit 0.

- [ ] **Step 5: Shellcheck both new files**

Run: `shellcheck .mise/scripts/criterion_store.sh .mise/tasks/selftest`
Expected: no findings.

- [ ] **Step 6: Commit**

```bash
git add .mise/scripts/criterion_store.sh .mise/tasks/selftest
git commit -m "feat(mise): durable criterion store helper with sandbox selftest"
```

---

### Task 2: Rewrite `clean/_default`, delete the trap dance, adopt the store

**Files:**
- Modify: `.mise/tasks/clean/_default` (USAGE `--all` long_help; replace `exec_target_clean`)
- Modify: `.mise/tasks/selftest` (append T6 + T2b)

- [ ] **Step 1: Write the failing test (T6, dry-run must not mutate)**

Append to `.mise/tasks/selftest` (before `main()` call, add invocation in `main`):

```bash
#######################################
# T6: clean --dry-run never mutates: task exits zero, no store created,
# no data moved, target/criterion stays a real directory, and no
# CACHEDIR.TAG remains (a transient one may be written and removed to
# satisfy cargo clean --dry-run's tag requirement; the assertion checks the
# final state, which must equal the starting state). Status is captured
# (never bare) so a failing task records FAIL instead of aborting the
# suite under set -e.
#######################################
test_clean_dry_run_no_mutation() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" dry-run-data
  cp "${CLEAN_TASK}" "${sbx}/clean_task"
  local status=0
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" MISE_TASK_DIR="${sbx}" \
      usage_dry_run=true usage_target_dir=target \
      bash ./clean_task ) >/dev/null 2>&1 || status=$?
  assert "T6 task exits zero" test "${status}" -eq 0
  assert "T6 history in place after dry-run" \
    test -f "${sbx}/target/criterion/g1/100/new/estimates.json"
  assert "T6 no store created" test ! -e "${sbx}/.criterion"
  assert "T6 real dir untouched (not a symlink)" \
    test ! -L "${sbx}/target/criterion"
  assert "T6 no CACHEDIR.TAG written" \
    test ! -e "${sbx}/target/CACHEDIR.TAG"
}

#######################################
# T2b: full clean with store present, cargo clean wipes artifacts,
# history survives via the link, link is restored afterwards.
#######################################
test_clean_full_preserves_history() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" precious
  run_ensure "${sbx}"                       # store + link in place
  mkdir -p "${sbx}/target/debug"; echo junk > "${sbx}/target/debug/bin"
  cp "${CLEAN_TASK}" "${sbx}/clean_task"
  local status=0
  ( cd "${sbx}" && MISE_PROJECT_ROOT="${sbx}" MISE_TASK_DIR="${sbx}" \
      usage_dry_run=false usage_target_dir=target \
      bash ./clean_task ) >/dev/null 2>&1 || status=$?
  assert "T2b task exits zero" test "${status}" -eq 0
  assert "T2b build artifact removed" test ! -e "${sbx}/target/debug/bin"
  assert "T2b history survives full clean" \
    grep -q precious "${sbx}/.criterion/g1/100/new/estimates.json"
  assert "T2b link restored after clean" test -L "${sbx}/target/criterion"
}
```

Add `test_clean_dry_run_no_mutation` and `test_clean_full_preserves_history` to `main()`.

- [ ] **Step 2: Run selftest; verify red against current clean**

Run: `mise run selftest`
Expected: non-zero exit with at least these FAILs against the unmodified clean:
- `FAIL - T6 no CACHEDIR.TAG written` (current code writes the tag unconditionally, even on the dry-run path),
- `FAIL - T2b link restored after clean` (current mv-aside dance restores a REAL directory, not the symlink).

Any red outcome satisfies this checkpoint; Step 5's green is the load-bearing gate.

- [ ] **Step 3: Replace `exec_target_clean` in `.mise/tasks/clean/_default`**

Delete the entire current `exec_target_clean` (lines ~140-233: size gate stays, but `criterion_dir`/`has_criterion`/`backup_tmp`/`trap`/restore block all go). Replace with:

```bash
#######################################
# Executes cargo clean with size gating. Benchmark history is protected by
# the durable store contract (.mise/scripts/criterion_store.sh): history
# lives in `.criterion/` behind the `target/criterion` symlink, so `cargo
# clean` can only ever remove the link. A stray real directory is moved into
# the store BEFORE cleaning; after a full clean the link is recreated so
# bench tooling sees history again.
#
# Globals:
#   TARGET_DIR
#   usage_if_over_gib
#   usage_dry_run
#   usage_release
#   usage_doc
#   usage_profile
#   MISE_PROJECT_ROOT
# Arguments:
#   None
# Outputs:
#   Size diagnostics, contract preview/preservation lines to STDOUT.
# Returns:
#   Status of cargo clean.
#######################################
exec_target_clean() {
  if [[ -n "${usage_if_over_gib:-}" ]]; then
    require_nonneg_int "clean" "--if-over-gib" "${usage_if_over_gib}"
    if ! eval_dir_size_gate \
      "clean" "${TARGET_DIR}" "${usage_if_over_gib}"; then
      return 0
    fi
  fi
  local is_full_clean=true
  if [[ "${usage_release:-false}" == "true" \
    || "${usage_doc:-false}" == "true" \
    || -n "${usage_profile:-}" ]]; then
    is_full_clean=false
  fi

  # shellcheck source=/dev/null
  source "${MISE_PROJECT_ROOT:?}/.mise/scripts/criterion_store.sh"

  if [[ "${usage_dry_run:-false}" == "true" ]]; then
    criterion_dry_run_note "${TARGET_DIR}"
  else
    # Migrate/park any stray real target/criterion BEFORE cargo clean can
    # delete it in place. Argument (not env prefix): the link path must be
    # re-derived per call, or a custom --target-dir override is ignored.
    ensure_criterion_store "${TARGET_DIR}"
  fi

  local tag_file="${TARGET_DIR}/CACHEDIR.TAG"
  local dry_run_temp_tag=false
  if [[ -d "${TARGET_DIR}" && ! -f "${tag_file}" ]]; then
    printf '%s\n%s\n' \
      'Signature: 8a477f597d28d172789f06886806bc55' \
      '# This file is a cache directory tag created by cargo.' \
      > "${tag_file}" 2>/dev/null || true
    if [[ "${usage_dry_run:-false}" == "true" ]]; then
      # cargo clean refuses to run without a CACHEDIR.TAG (even with
      # --dry-run), so a dry-run in a tagless target dir writes one
      # transiently and removes it again after cargo: the dry-run must
      # leave the filesystem unchanged.
      dry_run_temp_tag=true
    fi
  fi

  declare -a cargo_args=(clean --target-dir "${TARGET_DIR}")
  [[ "${usage_dry_run:-false}" == "true" ]] && cargo_args+=(--dry-run)
  [[ "${usage_release:-false}" == "true" ]] && cargo_args+=(--release)
  [[ "${usage_doc:-false}" == "true" ]] && cargo_args+=(--doc)
  [[ -n "${usage_profile:-}" ]] && cargo_args+=(--profile "${usage_profile}")

  local cargo_status=0
  cargo "${cargo_args[@]}" || cargo_status=$?
  if [[ "${dry_run_temp_tag}" == "true" ]]; then
    rm -f "${tag_file}"
  fi
  if [[ "${cargo_status}" -ne 0 ]]; then
    return "${cargo_status}"
  fi

  if [[ "${is_full_clean}" == "true" \
    && "${usage_dry_run:-false}" != "true" ]]; then
    # cargo clean removed target/ including the symlink; restore the link
    # (no-op when no store exists) and re-tag the directory.
    ensure_criterion_store "${TARGET_DIR}"
    if [[ -L "${TARGET_DIR}/criterion" ]]; then
      printf '%s\n' \
        'clean: benchmark history in .criterion/ is untouched; target/criterion symlink restored'
      printf '%s\n%s\n' \
        'Signature: 8a477f597d28d172789f06886806bc55' \
        '# This file is a cache directory tag created by cargo.' \
        > "${tag_file}" 2>/dev/null || true
    fi
  fi
}
```

- [ ] **Step 4: Update the `--all` USAGE long_help**

In the `#USAGE flag "--all"` block, replace:

```
#USAGE   Wipes the entire Cargo target directory and all quality report files. Explicitly preserves historical benchmark results in `target/criterion/`.
```

with:

```
#USAGE   Wipes the entire Cargo target directory and all quality report files. Benchmark history is NEVER deleted: it lives in `.criterion/` at the repo root, and `target/criterion` is a symlink into it (recreated after the clean).
```

Also update the file-header comment block (the `# Clean the Cargo target directly…` lines above `plan_clean_target=true`) to add: `# Benchmark history is protected by the durable store contract in .mise/scripts/criterion_store.sh.`

- [ ] **Step 5: Run selftest; verify all green**

Run: `mise run selftest`
Expected: `selftest: 39 passed, 0 failed` (30 + T6×5 + T2b×4), exit 0.

- [ ] **Step 6: Shellcheck**

Run: `shellcheck .mise/tasks/clean/_default .mise/tasks/selftest`
Expected: no findings.

- [ ] **Step 7: Commit**

```bash
git add .mise/tasks/clean/_default .mise/tasks/selftest
git commit -m "refactor(mise): clean guarantees criterion history via store contract"
```

---

### Task 3: Guard `bench/_default` + fix `snapshot_baseline` nesting

**Files:**
- Modify: `.mise/tasks/bench/_default` (source helper after `set -euo pipefail`; ensure in `main()`; replace `snapshot_baseline` body)
- Modify: `.mise/tasks/selftest` (append T7)

- [ ] **Step 1: Write the failing test (T7: replace-not-nest, with stub cargo)**

Append to `.mise/tasks/selftest` (register in `main()`):

```bash
#######################################
# T7: bench snapshot replaces a same-name baseline instead of nesting into
# it (cp -R into an existing dir would copy *inside* it, one level per
# re-run of the same branch+sha). Uses a stub `cargo` on PATH.
#######################################
test_snapshot_replaces_not_nests() {
  local sbx; sbx="$(new_sandbox)"
  seed_criterion "${sbx}" seed
  mkdir -p "${sbx}/bin"
  cat > "${sbx}/bin/cargo" <<'EOF'
#!/bin/bash
# stub: emulate `cargo bench` writing criterion output
if [[ "${1:-}" == "bench" ]]; then
  mkdir -p target/criterion/g1/100/new target/criterion/report
  printf '{"stub":1}\n' > target/criterion/g1/100/new/estimates.json
  printf '<html></html>\n' > target/criterion/report/index.html
fi
exit 0
EOF
  chmod +x "${sbx}/bin/cargo"
  cp "${BENCH_TASK}" "${sbx}/bench_task"
  local run=(
    env MISE_PROJECT_ROOT="${sbx}" MISE_TASK_DIR="${sbx}"
    usage_mode=normal usage_verbose=false usage_report=mybase
    usage_args= usage_files= usage_mod= usage_compare=
    usage_no_report=false usage_model=false usage_plot=false
    PATH="${sbx}/bin:${PATH}" bash ./bench_task
  )
  local st1=0 st2=0
  ( cd "${sbx}" && "${run[@]}" ) >/dev/null 2>&1 || st1=$?
  ( cd "${sbx}" && "${run[@]}" ) >/dev/null 2>&1 || st2=$?    # second run, same name
  assert "T7 bench runs exit zero" test "${st1}-${st2}" = "0-0"
  assert "T7 baseline dir replaced, not nested" \
    test -f "${sbx}/.criterion/g1/100/mybase/estimates.json"
  assert "T7 no nested new/ inside baseline" \
    test ! -d "${sbx}/.criterion/g1/100/mybase/new"
  assert "T7 store link intact for critcmp" test -L "${sbx}/target/criterion"

  # Traversal regression: a `-r ../sib` name must be refused before it can
  # `rm -rf` a sibling of the baseline dir, so the sibling survives and the
  # run exits non-zero. Status captured (never bare) so a failure records
  # FAIL instead of aborting the suite under set -e.
  mkdir -p "${sbx}/.criterion/g1/100/sib"
  printf 'keep\n' > "${sbx}/.criterion/g1/100/sib/estimates.json"
  local bad=( "${run[@]}" )
  bad=( "${bad[@]/usage_report=mybase/usage_report=../sib}" )
  local st3=0
  ( cd "${sbx}" && "${bad[@]}" ) >/dev/null 2>&1 || st3=$?
  assert "T7 traversal baseline name refused" test "${st3}" -ne 0
  assert "T7 sibling baseline survives traversal attempt" \
    test -f "${sbx}/.criterion/g1/100/sib/estimates.json"
}
```

- [ ] **Step 2: Run selftest; verify red against current bench code**

Run: `mise run selftest`
Expected: non-zero exit with at least one of these FAILs against the unmodified bench:
- `FAIL - T7 bench runs exit zero` (current code has no store guard; sandbox/git/env paths may error), and/or
- `FAIL - T7 baseline dir replaced, not nested` + `FAIL - T7 store link intact for critcmp` (no store exists yet; the actual nesting happens under `target/criterion/`, which the fixed code will observe in the store instead).

Any red outcome satisfies this checkpoint; Step 5's green is the load-bearing gate.

- [ ] **Step 3: Guard `bench/_default`**

After the existing `set -euo pipefail` (line ~93), insert:

```bash
# Durable benchmark store contract: target/criterion is a symlink into
# .criterion/; migrate/park strays before any measurement writes data.
# shellcheck source=/dev/null
source "${MISE_PROJECT_ROOT:?}/.mise/scripts/criterion_store.sh"
```

At the top of `main()` (before `collect_bench_files`), insert:

```bash
  # Raw `cargo clean` (or hand-typed `cargo bench` after one) can leave a
  # stray real target/criterion; reconcile before measuring.
  ensure_criterion_store
```

- [ ] **Step 4: Fix `snapshot_baseline`**

Replace the `while` loop body (the `cp -R` at `bench/_default:283-286`) with replace-not-nest semantics, and walk the STORE directly (find on a symlinked root has portability sharp edges; the store path is unambiguous):

```bash
snapshot_baseline() {
  local baseline_name="${1:-}"
  [[ -z "${baseline_name}" ]] && return 0
  [[ "${usage_no_report:-false}" == "true" ]] && return 0
  [[ -d "${CRITERION_STORE}" ]] || return 0

  local new_dir parent_dir dest
  while IFS= read -r -d '' new_dir; do
    parent_dir="${new_dir%/new}"
    dest="${parent_dir}/${baseline_name}"
    # Replace, never nest: `cp -R new existing-dir/` copies INTO the
    # directory, so a re-run of the same branch+sha would nest one level
    # per run (observed in the live store). Same name = superseded by
    # definition; remove the previous snapshot first.
    rm -rf "${dest}"
    cp -R "${new_dir}" "${dest}"
  done < <(find "${CRITERION_STORE}" -type d -name "new" -print0)
}
```

Update the function's doc-comment Globals line to mention `CRITERION_STORE` (from the sourced helper).

- [ ] **Step 4.5: Guard the snapshot name against path traversal**

Task 3 introduced `rm -rf "${dest}"` where `dest="${parent_dir}/${baseline_name}"`. `resolve_baseline_name` currently echoes an explicit `-r <name>` verbatim, so a name like `../sib` or `../..` escapes the baseline dir and can delete a sibling (or the store parent); this is a traversal the store contract (nothing deletes the store) forbids. This did not exist before this task (`cp -R` polluted but could not delete), so it is a delta-introduced escalation that must be closed before merge.

(a) Add to `.mise/scripts/criterion_store.sh` (after `criterion_link_text`), mirroring `require_nonneg_int`'s fail-fast `exit 2` shape (`size_gate.sh:26-33`):

```bash
#######################################
# Validates a benchmark baseline name before it is used to build a path
# under the store. `snapshot_baseline` runs `rm -rf` on
# `<store>/<group>/<id>/<name>`, so a name containing a slash or a `..`
# component would traverse out of the baseline dir and delete siblings.
# Arguments:
#   Task name (for the error message), flag name, baseline name.
# Returns:
#   Exits 2 on an unsafe name; returns 0 on a safe one.
#######################################
require_safe_baseline_name() {
  local task="$1" flag="$2" name="$3"
  local part
  local IFS=/
  # shellcheck disable=SC2034  # loop var splits components; body only inspects
  for part in ${name}; do
    if [[ "${part}" == ".." ]]; then
      printf "%s: %s must not contain '..', got '%s'\n' \
        "${task}" "${flag}" "${name}" >&2
      exit 2
    fi
  done
  if [[ "${name}" == */* ]]; then
    printf "%s: %s must not contain '/', got '%s'\n" \
      "${task}" "${flag}" "${name}" >&2
    exit 2
  fi
  return 0
}
```

Note: the slash check is redundant with the component loop only when a slash is present; keeping it explicit also rejects a leading/trailing slash that the `..` loop alone would not. An empty name never reaches this function (`snapshot_baseline` returns early on empty; `resolve_baseline_name` only routes non-`true`, non-empty `-r` values here).

(b) In `.mise/tasks/bench/_default` `resolve_baseline_name`, call the guard in the explicit-name branch before returning the name:

```bash
  # Explicit `-r <name>`; bare `-r` parses as the literal `true` and falls
  # through to the auto-name below.
  if [[ -n "${usage_report:-}" && "${usage_report}" != "true" ]]; then
    require_safe_baseline_name "bench" "-r/--report" "${usage_report}"
    printf '%s' "${usage_report}"
    return
  fi
```

The auto-name branch is safe already: `branch="${branch//\//-}"` (line 245) strips slashes, git short-sha is hex, and `..` cannot occur in a branch+sha concatenation.

- [ ] **Step 5: Run selftest; verify all green**

Run: `mise run selftest`
Expected: `selftest: 45 passed, 0 failed` (39 + T7×6), exit 0.

- [ ] **Step 6: Shellcheck**

Run: `shellcheck .mise/scripts/criterion_store.sh .mise/tasks/bench/_default .mise/tasks/selftest`
Expected: no findings.

- [ ] **Step 7: Commit**

```bash
git add .mise/scripts/criterion_store.sh .mise/tasks/bench/_default .mise/tasks/selftest
git commit -m "fix(mise): bench guards criterion store; snapshot replaces instead of nesting"
```

---

### Task 4: Guard `bench/report`

**Files:**
- Modify: `.mise/tasks/bench/report`

Note: `bench/_default` already declares `#MISE depends_post = ["bench:report"]`, so the report runs even when the bench run fails; with this guard, both entry points self-heal the link after any clean.

**Task 4 forward caveats (from Task 1 final review):** the existing `bench/report` arg defaults to a *criterion dir* (`target/criterion`), not a *target dir*; never pass it to `ensure_criterion_store` (it would compute depth 2 and create `target/criterion/criterion`). Call `ensure_criterion_store` with no args here (target dir stays `target`). Also: this task must not add a `#MISE dir="{{cwd}}"` header (the Case 0 cwd guard would then refuse every run).

- [ ] **Step 1: Ensure the link before summarizing**

Replace the opening of `main()` (report:177-178: `main() {` and `if ! has_criterion_output; then`) with:

```bash
main() {
  # Recreate target/criterion (symlink) after a full/raw clean so the
  # summary sees persisted history again.
  # shellcheck source=/dev/null
  source "${MISE_PROJECT_ROOT:?}/.mise/scripts/criterion_store.sh"
  ensure_criterion_store

  if ! has_criterion_output; then
```

(Do not paste this as an insert-in-place: the snippet already contains the two lines it replaces; a literal insert would nest a duplicate `main() {`.)

- [ ] **Step 2: Sanity-run in a T5-style sandbox**

Run:
```bash
REPO="$PWD"
SBX=$(mktemp -d); mkdir -p "$SBX"/{src,benches,.mise/scripts,target/criterion/g1/100/new}
printf '[package]\nname="sbx"\nversion="0.1.0"\nedition="2021"\n' > "$SBX/Cargo.toml"
echo 'pub fn f(){}' > "$SBX/src/lib.rs"; echo 'x' > "$SBX/benches/b.rs"
cp .mise/scripts/criterion_store.sh "$SBX/.mise/scripts/"
echo '{}' > "$SBX/target/criterion/g1/100/new/estimates.json"
( cd "$SBX" && MISE_PROJECT_ROOT="$SBX" bash "$REPO/.mise/tasks/bench/report" )
```
Expected: `## Benchmark Summary` with `- g1: 1 size` and the footer, read through the auto-created link (report migrates the real dir to store first).

- [ ] **Step 3: Shellcheck + commit**

Run: `shellcheck .mise/tasks/bench/report`, no findings.

```bash
git add .mise/tasks/bench/report
git commit -m "feat(mise): bench:report restores criterion link after cleans"
```

---

### Task 5: `.gitignore`, live migration, full verification

**Files:**
- Modify: `.gitignore`
- Live data: `target/criterion` → `.criterion` + symlink

- [ ] **Step 1: Gitignore the store**

In `.gitignore`, directly after the `/target` line in the "Build & Compilation" block, add:

```
# --- Durable Criterion benchmark history (symlinked as target/criterion) --- #
/.criterion/
/.criterion.fork.*
```

- [ ] **Step 2: Commit gitignore**

```bash
git add .gitignore
git commit -m "chore(git): ignore durable criterion store"
```

- [ ] **Step 3: Record pre-migration census**

```bash
PRE_DIRS=$(find target/criterion -type d | wc -l | tr -d ' ')
PRE_JSON=$(find target/criterion -name estimates.json | wc -l | tr -d ' ')
PRE_BASE=$(critcmp --baselines 2>/dev/null | wc -l | tr -d ' ')
echo "dirs=$PRE_DIRS json=$PRE_JSON baselines=$PRE_BASE"
```
Expected (2026-10-18 census): `dirs=118394 json=116313 baselines=13`. Keep these numbers for Step 5.

- [ ] **Step 4: Migrate (same-volume instant renames)**

```bash
test -d target/criterion && test ! -L target/criterion && test ! -e .criterion
mv target/criterion .criterion
ln -s ../.criterion target/criterion
ls -la target/criterion
```
Expected: `target/criterion -> ../.criterion`.

- [ ] **Step 5: Verify integrity + tool transparency**

```bash
POST_DIRS=$(find .criterion -type d | wc -l | tr -d ' ')
POST_JSON=$(find .criterion -name estimates.json | wc -l | tr -d ' ')
POST_BASE=$(critcmp --baselines 2>/dev/null | wc -l | tr -d ' ')
test "$PRE_DIRS" = "$POST_DIRS" && test "$PRE_JSON" = "$POST_JSON" && test "$PRE_BASE" = "$POST_BASE" && echo CENSUS-OK
du -sh .criterion          # ~1.9G
test -f target/criterion/report/index.html && echo HTML-VISIBLE-VIA-LINK
```
Expected: `CENSUS-OK`, `~1.9G`, `HTML-VISIBLE-VIA-LINK`, critcmp reads identical baseline count through the symlink.

- [ ] **Step 6: Live dry-run (must be a pure preview)**

Run: `mise run clean --dry-run` (confirm prompt: `clean` declares `#MISE confirm`, so answer it; confirm guards only the run body per docs/refs/mise_tasks/07_configuration.md:356-358).
Expected: cargo's "would remove" lines plus:
```
clean: benchmark history lives in .criterion/ and is never deleted
```
and afterwards `test -d .criterion && test -L target/criterion`, nothing moved. (Note: unlike the sandbox T6, the live dry-run may print the "would restore" line only when the link is absent; with the link present it prints just the contract line.)

- [ ] **Step 7: Full verification battery**

```bash
mise run selftest                      # 45 passed, 0 failed
shellcheck .mise/scripts/criterion_store.sh .mise/tasks/clean/_default \
  .mise/tasks/bench/_default .mise/tasks/bench/report .mise/tasks/selftest
mise tasks | grep -E '^(clean|bench|selftest)'   # tasks still parse
critcmp --baselines | head              # history reachable
git status --short                      # only intended files staged/changed
```
Expected: all green; `git status` shows no `.criterion` (gitignored).

- [ ] **Step 8: Manual smoke (optional but recommended)**

`mise run bench --mode quick -f <smallest-bench>` then `critcmp --baselines | tail`, a new `main-<sha>[-dirty]-quick` baseline appears *inside the store* (`ls .criterion/<group>/<size>/`), and `target/criterion` is still a symlink after the run.

- [ ] **Step 9: Final commit (if Step 8 changed nothing tracked)**

No commit needed for data (untracked/ignored). If Step 8 revealed doc tweaks, fold them here:

```bash
git add -u && git commit -m "docs(mise): criterion store contract notes" || true
```

---

## Self-Review

1. **Spec coverage:** never-delete guarantee (symlink structure + guards, Tasks 1-4); `.criterion` name (Task 5 migration); dry-run purity (T6, `criterion_dry_run_note`); snapshot nesting fix (Task 3); regression tripwire (`selftest` task); gitignore; live migration with byte-count verification; doc/long_help updates (Task 2 Step 4). Covered.
2. **Placeholders:** none, every step carries complete code or exact commands.
3. **Name consistency:** `ensure_criterion_store`, `criterion_link_ok`, `criterion_link_path`, `criterion_link_text`, `criterion_dry_run_note`, `CRITERION_STORE`, `CRITERION_TARGET_DIR` are defined once in the helper and used identically by clean, bench, report, and selftest, and `require_safe_baseline_name` is defined once in the helper and called by `resolve_baseline_name` before the name reaches `snapshot_baseline`'s `rm -rf`. Target dir is passed as a positional argument (or left to the `CRITERION_TARGET_DIR` default); no link path is expanded at source time; link text is depth-aware and normalized (`./target` == `target`; absolute, `..`, and repo-root target dirs refused); `ensure` refuses to run outside `MISE_PROJECT_ROOT`. `run_ensure`/`seed_criterion`/`new_sandbox` are defined once in Task 1 and reused by later tasks' tests. Assertion totals: 30 (Task 1), 39 (Task 2), 45 (Task 3, Task 5 battery).
