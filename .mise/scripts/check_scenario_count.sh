#!/usr/bin/env bash
# Makes the features binary list every scenario the feature files declare.
#
# Cargo does not notice a new feature file until tests/features.rs changes, so
# a stale build (a local edit, or a restored CI cache) would silently skip the
# new scenarios. A mismatch touches that file and lists again; only a mismatch
# that survives the rebuild is an error.
#
# Usage: check_scenario_count.sh [CARGO_FEATURE_FLAGS...]
#   The flags select the build that is listed; the default is
#   --no-default-features, the shipped configuration. The count does not depend
#   on the flags, but the listing reuses the build the caller runs next.
set -euo pipefail

declare -a feature_flags=("$@")
if ((${#feature_flags[@]} == 0)); then
  feature_flags=(--no-default-features)
fi

#######################################
# Prints how many scenarios the features binary lists.
# Outputs:
#   Scenario count to STDOUT.
#######################################
count_listed_scenarios() {
  cargo nextest list -p traces "${feature_flags[@]}" --test features \
    --color never --message-format oneline | grep -c 'features_scenarios::' || true
}

declared="$(find tests/features -name '*.feature' -exec grep -hE '^[[:space:]]*Scenario:' {} + | wc -l | tr -d ' ')"
listed="$(count_listed_scenarios)"
if [[ "${listed}" != "${declared}" ]]; then
  echo "==> features binary lists ${listed} scenarios but the feature files declare ${declared}; rebuilding" >&2
  touch tests/features.rs
  listed="$(count_listed_scenarios)"
fi
if [[ "${listed}" != "${declared}" ]]; then
  echo "ERROR: features binary lists ${listed} scenarios but the feature files declare ${declared}" >&2
  exit 1
fi
