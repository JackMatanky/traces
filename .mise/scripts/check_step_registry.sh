#!/usr/bin/env bash
# Compares the steps the features binary registers with the step gate.
#
# The features binary dumps each registered step with the regex and the
# specificity score of the framework. check_steps.py then compares the dump
# with its own mirror and with the step definitions it parses from the files.
# Run this after check_scenario_count.sh, so the binary is current.
#
# Usage: check_step_registry.sh [CARGO_FEATURE_FLAGS...]
#   The default flags are --no-default-features, the shipped configuration.
set -euo pipefail

declare -a feature_flags=("$@")
if ((${#feature_flags[@]} == 0)); then
  feature_flags=(--no-default-features)
fi
# CARGO_TARGET_TMPDIR is <target dir>/tmp; the repository sets no other target dir.
dump="${CARGO_TARGET_DIR:-target}/tmp/step-registry.tsv"
rm -f "${dump}"

cargo nextest run -p traces "${feature_flags[@]}" --test features \
  --no-tests fail registered_steps_are_dumped >&2
.mise/scripts/check_steps.py --registry "${dump}"
