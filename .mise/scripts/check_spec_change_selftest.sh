#!/usr/bin/env bash
set -euo pipefail

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
printf 'test: update scenarios\n' > "$root/message"
printf 'tests/features/note.feature\0' > "$root/paths"
if bash .mise/scripts/check_spec_change.sh "$root/message" "$root/paths" > "$root/stdout" 2> "$root/stderr"; then
    echo 'ERROR: Scenario change without trailer passed' >&2
    exit 1
fi
if ! grep -q 'require a commit message line starting spec change:' "$root/stderr"; then
    echo 'ERROR: Missing trailer reported the wrong error' >&2
    exit 1
fi
printf 'test: update scenarios\n\nspec change: strengthen value assertions\n' > "$root/message"
bash .mise/scripts/check_spec_change.sh "$root/message" "$root/paths"
printf '.scratch/bdd-strategy/research/10-invariance-map.md\0' > "$root/paths"
bash .mise/scripts/check_spec_change.sh "$root/message" "$root/paths"
printf 'test: update fixtures\n' > "$root/message"
printf 'tests/baselines/output/task.txt\0' > "$root/paths"
bash .mise/scripts/check_spec_change.sh "$root/message" "$root/paths"
echo 'Commit-message matcher self-test passed.'
