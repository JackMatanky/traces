# Coverage Auditing (`cargo-llvm-cov`)

> Line and branch coverage is an empirical floor: 0% coverage guarantees a gap, while 100% coverage without strong assertions proves nothing.

## 1. Running `cargo-llvm-cov` with `nextest`

Generate workspace coverage across unit and integration targets with all features enabled:

```bash
# Terminal summary table by file
cargo llvm-cov nextest --all-features

# Show exact uncovered line numbers and regions
cargo llvm-cov nextest --all-features --show-missing-lines

# Scope to library unit tests only for fast iteration
cargo llvm-cov nextest --lib --show-missing-lines
```

Include doctests when auditing public API coverage by adding `--doctests`:

```bash
cargo llvm-cov test --all-features --doctests
```

## 2. Turning Uncovered Lines into Unit Tests

Do not write superficial "line-touching" tests just to raise a percentage. Use uncovered regions as a diagnostic pointer:

1. Inspect uncovered lines in the target module from `--show-missing-lines`.
2. Classify each uncovered block against the 8-Category Case-Surface Checklist in [`../../rust-unit-testing/references/writing-suites.md`](../../rust-unit-testing/references/writing-suites.md):
   - Uncovered `Err(...)` return or `?` branch -> missing failure-path test.
   - Uncovered `match` arm -> missing enum variant case or dead code.
   - Uncovered boundary guard (`if len == 0`) -> missing empty/boundary case.
3. Write a behavioral test in [`../../rust-unit-testing/SKILL.md`](../../rust-unit-testing/SKILL.md) that asserts the exact outcome of that branch, or delete unreachable dead code.
4. Pair high-coverage modules with [`mutation.md`](mutation.md) to check that assertions fail when covered lines are mutated.

## Completion

Completion: uncovered lines and branches in the target files are listed, classified as missing tests or dead code, and covered or explicitly justified.
