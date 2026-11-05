#!/usr/bin/env bash
set -euo pipefail

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
mkdir -p "$root/tests/features/nested" "$root/.scratch/bdd-strategy/research" "$root/.scratch/bdd-testing"
printf 'Feature: Ownership\n  Scenario: Count notes\n    Then the count is one\n' > "$root/tests/features/nested/count.feature"
printf '| ID | Guarantee | Tier | Status |\n|---|---|---|---|\n| 01-G1 | Count | F | covered: Count notes (reviewed 2026-11-05) |\n' > "$root/.scratch/bdd-strategy/research/10-invariance-map.md"
printf '| Scenario title | Owner row IDs or reason |\n|---|---|\n' > "$root/.scratch/bdd-testing/scenario-owners.md"

.mise/scripts/check_scenarios.py "$root"
printf '  Scenario: Count tasks\n    Then the count is one\n' >> "$root/tests/features/nested/count.feature"
if .mise/scripts/check_scenarios.py "$root" > "$root/stdout" 2> "$root/stderr"; then
    echo 'ERROR: Uncited fixture passed' >&2
    exit 1
fi
if ! grep -Fq "ERROR: Uncited scenario: 'Count tasks'" "$root/stderr"; then
    echo 'ERROR: Uncited fixture reported the wrong error' >&2
    exit 1
fi
printf '| Scenario title | Owner row IDs or reason |\n|---|---|\n| Count tasks | Supporting count example. |\n' > "$root/.scratch/bdd-testing/scenario-owners.md"
.mise/scripts/check_scenarios.py "$root"
printf '| ID | Guarantee | Tier | Status |\n|---|---|---|---|\n| 01-G1 | Count | F | covered: Missing count (reviewed 2026-11-05) |\n' > "$root/.scratch/bdd-strategy/research/10-invariance-map.md"
if .mise/scripts/check_scenarios.py "$root" > "$root/stdout" 2> "$root/stderr"; then
    echo 'ERROR: Stale row fixture passed' >&2
    exit 1
fi
if ! grep -Fq "ERROR: Research 10 row 01-G1 points to missing scenario: 'Missing count'" "$root/stderr"; then
    echo 'ERROR: Stale row fixture reported the wrong error' >&2
    exit 1
fi
echo 'Scenario ownership gate self-test passed.'
