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
      printf "%s: %s must not contain '..', got '%s'\n" \
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
