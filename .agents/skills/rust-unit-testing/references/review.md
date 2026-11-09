# Reviewing and Auditing Unit Tests

> Follow this 7-step review workflow to find silent-green tests, brittle assertions, and missing edge cases.

## 7-Step Review Procedure

1. **Check the test discovery and execution:**
   Run `cargo nextest list -E 'kind(lib)'` to confirm the target tests are compiled and not silently skipped behind an inactive `#[cfg(feature = "...")]` gate.
2. **Audit coverage against the 8-Category Case-Surface Checklist:**
   Compare tests against the 8-Category Case-Surface Checklist in [`writing-suites.md`](writing-suites.md) (happy path, boundaries, empty, malformed, `Err` variants, transitions, round-trip, depth).
3. **Inspect assertion strength:**
   Flag weak checks (`assert!(res.is_ok())`, `assert!( res.is_err())`, or `.unwrap()` inside `assert_eq!`) and replace them with exact structural matches from [`../../rust-testing-router/references/assertions.md`](../../rust-testing-router/references/assertions.md).
4. **Check the test isolation and determinism:**
   Flag wall-clock sleeps, fixed `/tmp` paths, shared `static` state, unseeded RNGs, or process environment mutation. Replace them with [`../../rust-testing-router/references/raii-cleanup.md`](../../rust-testing-router/references/raii-cleanup.md) and [`../../rust-testing-router/references/async.md`](../../rust-testing-router/references/async.md).
5. **Audit naming and structure:**
   Check that test names obey [`naming.md`](naming.md), phases use blank lines without comments, and tables use [`../../rust-testing-router/references/table-driven.md`](../../rust-testing-router/references/table-driven.md).
6. **Audit lint hygiene:**
   Flag any bare `allow` attribute in test modules. Use narrow `#[expect(lint, reason = "...")]` attributes with a clear reason from [`code-quality.md`](code-quality.md).
7. **Run empirical suite audits (coverage and mutation):**
   Check uncovered lines with [`../../rust-testing-router/references/coverage.md`](../../rust-testing-router/references/coverage.md) and check assertion kill power with [`../../rust-testing-router/references/mutation.md`](../../rust-testing-router/references/mutation.md).

## Anti-Pattern Catalog

| Smell | Problem | Fix |
|---|---|---|
| Assertion-free smoke test | Calls a function and ignores the result; only catches panics | Assert the exact returned value or state transition |
| `assert!(result.is_err())` | Passes when the wrong error variant is returned | Match `Err(ExpectedVariant { .. })` |
| Chained `assert_eq!(f().unwrap(), x)` | Panics with an unhelpful `unwrap` message on `Err` | Separate `let actual = f();` and `assert_eq!(actual, Ok(x));` |
| Testing private helper wiring | Breaks on harmless internal refactors | Test through the module's public entrypoints |
| Logic loops inside a test | `for` loops and `if/else` branches hide which case failed | Split into `#[rstest]` `#[case::name(...)]` rows |
| Sleeping for async or threads | Slow and flaky under CPU load | Use `start_paused = true` or deterministic channels |

## Completion

Completion: review report lists findings by severity, maps missing cases to the 8-Category Case-Surface Checklist, and confirms zero weak assertions or bare `allow` attributes.
