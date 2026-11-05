#!/usr/bin/env bash
set -euo pipefail

message_file=$1
staged_paths=${2:-}
if [[ -n "$staged_paths" ]]; then
    exec 3< "$staged_paths"
else
    exec 3< <(git diff --cached --name-only --diff-filter=ACMR -z)
fi

while IFS= read -r -d '' path; do
    case "$path" in
        tests/features/*.feature|.scratch/bdd-strategy/research/10-invariance-map.md)
            if ! grep -q '^spec change:' "$message_file"; then
                echo 'ERROR: Scenario or invariance-map changes require a commit message line starting spec change:' >&2
                exit 1
            fi
            exit 0
            ;;
    esac
done <&3
