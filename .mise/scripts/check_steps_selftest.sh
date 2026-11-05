#!/usr/bin/env bash
set -euo pipefail

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
mkdir -p "$root/tests/features/steps" "$root/tests/features/nested"

expect_error() {
    local message=$1
    if python3 .mise/scripts/check_steps.py "$root" > /dev/null 2> "$root/stderr"; then
        echo "ERROR: fixture passed but should report: $message" >&2
        exit 1
    fi
    if ! grep -Fq "$message" "$root/stderr"; then
        echo "ERROR: wrong report for: $message" >&2
        cat "$root/stderr" >&2
        exit 1
    fi
}

printf '#[given("a note \\\n    {name}")]\nfn a() {}\n#[when(r#"I run "{cmd}""#)]\nfn b() {}\n#[when(r#"I run "{cmd}" in zone "{zone}""#)]\nfn c() {}\n' > "$root/tests/features/steps/a.rs"
printf 'Feature: Steps\n  Scenario: One\n    Given a note "x"\n    When I run "list"\n    And I run "list" in zone "UTC"\n      """\n      Given ignored docstring text\n      """\n' > "$root/tests/features/nested/one.feature"
python3 .mise/scripts/check_steps.py "$root"

printf '    Then nothing defines this\n' >> "$root/tests/features/nested/one.feature"
expect_error "ERROR: Undefined step at one.feature:9: then 'nothing defines this'"

printf 'Feature: Steps\n  Scenario: One\n    Given a note "x"\n    When I run "list"\n' > "$root/tests/features/nested/one.feature"
expect_error "ERROR: Unused step definition: when 'I run \"{cmd}\" in zone \"{zone}\"'"

printf '#[when("I run {a} {b}")]\nfn d() {}\n#[when("I run {c} {d}")]\nfn e() {}\n' >> "$root/tests/features/steps/a.rs"
printf '    When I run x y\n' >> "$root/tests/features/nested/one.feature"
expect_error "ERROR: Ambiguous step at one.feature:5: 'I run x y'"
echo 'Step matching gate self-test passed.'
