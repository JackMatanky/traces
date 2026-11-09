---
name: rust-unit-testing
description: Rust unit tests (#[cfg(test)], #[test]) and rustdoc doctests. Use when writing, reviewing, or refactoring in-crate unit suites.
metadata:
  version: "1.1.0"
  companion_to: "rust-testing-router"
---

# Rust Unit Testing

Unit tests in `#[cfg(test)] mod tests` check module invariants, edge cases, and error branches. Rustdoc doctests check that public API examples compile and run.

## Precedence Rule

Project conventions, `AGENTS.md`, and local specifications take precedence over default naming and layout rules in this skill.

## Core Principles

1. **Test behavior, not implementation:** Assert observable return values and state transitions rather than private helper call order.
2. **Separate Arrange, Act, and Assert with blank lines:** Do not add `// Arrange`, `// Act`, or `// Assert` banner comments.

```rust
#[test]
fn withdrawal_reduces_balance() {
    let mut account = Account::with_balance(100);

    let receipt = account.withdraw(40).expect("withdraw 40 from 100");

    assert_eq!(receipt.remaining, 60);
    assert_eq!(account.balance(), 60);
}
```

3. **Unwrap and expect policy:** Never use `.unwrap()` in production code. Inside test Arrange/Act setup, prefer `.expect("descriptive setup reason")` or `?` propagation. For the value under test in the Assert phase, assert on `Ok(...)` or `Err(...)` directly instead of calling `.unwrap()`.
4. **Keep test code at production quality:** Use `#[expect(lint, reason = "...")]` for lint suppression. Never use bare `allow` attributes.

## 4-Step Workflow

1. Map the target function or type against the 8-Category Case-Surface Checklist in [`references/writing-suites.md`](references/writing-suites.md).
   Completion: every applicable category is mapped to a planned test case.
2. Name and structure each test using [`references/naming.md`](references/naming.md) and [`references/code-quality.md`](references/code-quality.md).
   Completion: test names state behavior and condition, and phases are separated by blank lines.
3. Add or update executable `/// # Examples` doctests on public items using [`references/doctests.md`](references/doctests.md).
   Completion: `cargo test --doc` compiles and passes with zero `.unwrap()` calls in visible example code.
4. Check the test discovery, red-first failure, and empirical quality using [`references/review.md`](references/review.md).
   Completion: new tests appear in `cargo nextest list -E 'kind(lib)'`, fail when the implementation breaks, and pass clean.

## Reference Guide

- [`references/writing-suites.md`](references/writing-suites.md) - 8-Category Case-Surface Checklist and suite authoring workflow.
- [`references/review.md`](references/review.md) - 7-step review checklist, anti-pattern catalog, and coverage/mutation audit.
- [`references/naming.md`](references/naming.md) - naming patterns (`<unit>_<scenario>_<outcome>`) and module grouping.
- [`references/code-quality.md`](references/code-quality.md) - lint hygiene, helper extraction, and test readability.
- [`references/doctests.md`](references/doctests.md) - writing rustdoc examples with `#` hidden lines and `?` error handling.
- [`../rust-testing-router/SKILL.md`](../rust-testing-router/SKILL.md) - shared references for assertions, `rstest` tables and fixtures, `proptest`, fuzzing, fakes, snapshots, async, concurrency, coverage, and mutation testing.
