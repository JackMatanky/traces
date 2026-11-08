#!/usr/bin/env bash
set -euo pipefail

root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
mkdir -p "$root/tests/features/steps" "$root/tests/features/nested"

expect_error() {
    local message=$1
    if .mise/scripts/check_steps.py "$root" > /dev/null 2> "$root/stderr"; then
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
.mise/scripts/check_steps.py "$root"

printf '    Then nothing defines this\n' >> "$root/tests/features/nested/one.feature"
expect_error "ERROR: Undefined step at one.feature:9: then 'nothing defines this'"

printf 'Feature: Steps\n  Scenario: One\n    Given a note "x"\n    When I run "list"\n' > "$root/tests/features/nested/one.feature"
expect_error "ERROR: Unused step definition: when 'I run \"{cmd}\" in zone \"{zone}\"'"

printf '#[when("I run {a} {b}")]\nfn d() {}\n#[when("I run {c} {d}")]\nfn e() {}\n' >> "$root/tests/features/steps/a.rs"
printf '    When I run x y\n' >> "$root/tests/features/nested/one.feature"
expect_error "ERROR: Ambiguous step at one.feature:5: 'I run x y'"

printf '// #[when("commented out")]\n#[when("I run {a} {b}")]\nfn f() {}\n' > "$root/tests/features/steps/a.rs"
printf 'Feature: Steps\n  Scenario: One\n    When I run x y\n    Then commented out\n' > "$root/tests/features/nested/one.feature"
expect_error "ERROR: Undefined step at one.feature:4: then 'commented out'"

printf '#[when("I run {a}")]\nfn g() {}\n#[when("I run {a}")]\nfn h() {}\n' > "$root/tests/features/steps/a.rs"
printf 'Feature: Steps\n  Scenario: One\n    When I run x\n' > "$root/tests/features/nested/one.feature"
expect_error "ERROR: Duplicate step definition: when 'I run {a}'"
printf '#[when("I run {a}")]\nfn i() {}\n' > "$root/tests/features/steps/a.rs"
printf 'Feature: Steps\n  Scenario: One\n    When I run x\n      ```\n      Then fenced text\n      ```\n' > "$root/tests/features/nested/one.feature"
.mise/scripts/check_steps.py "$root"

printf '#[then("the exit code is {code:i32}")]\nfn j() {}\n#[then("the count is {n:u32}")]\nfn k() {}\n' > "$root/tests/features/steps/a.rs"
printf 'Feature: Steps\n  Scenario: One\n    Then the exit code is -2\n    And the count is 7\n' > "$root/tests/features/nested/one.feature"
.mise/scripts/check_steps.py "$root"
printf 'Feature: Steps\n  Scenario: One\n    Then the exit code is two\n    And the count is 7\n' > "$root/tests/features/nested/one.feature"
expect_error "ERROR: Undefined step at one.feature:3: then 'the exit code is two'"
printf 'Feature: Steps\n  Scenario: One\n    Then the exit code is -2\n    And the count is -7\n' > "$root/tests/features/nested/one.feature"
expect_error "ERROR: Undefined step at one.feature:4: then 'the count is -7'"

printf '#[when("say {{x}}")]\nfn l() {}\n' > "$root/tests/features/steps/a.rs"
printf 'Feature: Steps\n  Scenario: One\n    When say {x}\n' > "$root/tests/features/nested/one.feature"
expect_error "ERROR: Step pattern outside the checked syntax: when 'say {{x}}'"

# The mirror must agree with the framework regexes the Rust test pins.
baseline=tests/baselines/step-regexes.tsv
printf '#[when("I run {a}")]\nfn m() {}\n' > "$root/tests/features/steps/a.rs"
printf 'Feature: Steps\n  Scenario: One\n    When I run x\n' > "$root/tests/features/nested/one.feature"
.mise/scripts/check_steps.py "$root" --baseline "$baseline"

# A typed hole beats an untyped hole of equal literal text.
printf '#[when("I run {a}")]\nfn n() {}\n#[when("I run {b:u32}")]\nfn o() {}\n' > "$root/tests/features/steps/a.rs"
printf 'Feature: Steps\n  Scenario: One\n    When I run 5\n    And I run x\n' > "$root/tests/features/nested/one.feature"
.mise/scripts/check_steps.py "$root"
printf '#[when("I run {a}")]\nfn m() {}\n' > "$root/tests/features/steps/a.rs"
printf 'Feature: Steps\n  Scenario: One\n    When I run x\n' > "$root/tests/features/nested/one.feature"

# expect_flag_error FLAG FILE MESSAGE: the script must fail with MESSAGE.
expect_flag_error() {
    local flag=$1 file=$2 message=$3
    if .mise/scripts/check_steps.py "$root" "$flag" "$file" > /dev/null 2> "$root/stderr"; then
        echo "ERROR: $flag passed but should report: $message" >&2
        exit 1
    fi
    if ! grep -Fq "$message" "$root/stderr"; then
        echo "ERROR: wrong $flag report for: $message" >&2
        cat "$root/stderr" >&2
        exit 1
    fi
}

bad="$root/bad.tsv"
# A framework regex change for an integer hint.
awk -F'\t' -v OFS='\t' '$1 == "{x:u32}" { $2 = "^([+-]?\\d+)$" } 1' "$baseline" > "$bad"
expect_flag_error --baseline "$bad" "ERROR: Mirror differs from the framework at bad.tsv:3: '{x:u32}'"
# A framework score change for a typed hole.
awk -F'\t' -v OFS='\t' '$1 == "{x:u32}" { $3 = "0/1/0" } 1' "$baseline" > "$bad"
expect_flag_error --baseline "$bad" "ERROR: Mirror differs from the framework at bad.tsv:3: '{x:u32}'"
# A pattern the framework accepts but the mirror rejects.
printf '{ x }\t^\\{ x \\}$\t5/0/0\n' > "$bad"
expect_flag_error --baseline "$bad" "ERROR: Mirror differs from the framework at bad.tsv:1: '{ x }' mirror ['ERROR', 'ERROR']"
# A pattern the framework rejects but the mirror accepts.
printf '{x}\tERROR\tERROR\n' > "$bad"
expect_flag_error --baseline "$bad" "ERROR: Mirror differs from the framework at bad.tsv:1: '{x}'"

# The registered steps must agree with the mirror and with the step files.
registry="$root/registry.tsv"
printf 'when\tI run {a}\t^I run (.+?)$\t6/1/0\n' > "$registry"
.mise/scripts/check_steps.py "$root" --registry "$registry"
printf 'when\tI run {a}\t^I run (.+?)$\t6/1/1\n' > "$registry"
expect_flag_error --registry "$registry" "ERROR: Mirror differs from the framework registry for when 'I run {a}'"
printf 'when\tI run {a}\t^I run (.+?)$\t6/1/0\nthen\textra\t^extra$\t5/0/0\n' > "$registry"
expect_flag_error --registry "$registry" "ERROR: Step registered but not found in the step files: then 'extra'"
: > "$registry"
expect_flag_error --registry "$registry" "ERROR: Step found in the step files but not registered: when 'I run {a}'"

echo 'Step matching gate self-test passed.'
