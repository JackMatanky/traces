# Test Code Quality and Maintainability

> Test suites are long-lived code: keep them lint-clean, self-contained, and free of hidden control flow.

## 1. Lint Hygiene (`#[expect]` Over `allow`)

Keep test modules subject to the same compiler and Clippy lints as production code. When a test deliberately triggers a lint (for example, testing an intentionally large stack value or a macro expansion that triggers a Clippy rule):

- Attach a narrow `#[expect(clippy::lint_name, reason = "explain why this test triggers the lint")]` attribute to the specific test function or module.
- Never use bare `allow` attributes. `#[expect]` alerts the compiler when the lint condition disappears, preventing stale suppressions.

## 2. Helper Functions vs. Test Logic

- Extract Arrange builders or `#[fixture]` functions when three or more tests construct the same complex struct.
- Keep assertions inside the `#[test]` body so failure line numbers point directly to the failing test case.
- Never put `for` loops, `if/else` branches, or `match` calculations inside a test. Use the `#[rstest]` cases instead.

## 3. Error Handling in Test Setup

- In production code, `.unwrap()` is forbidden.
- In test Arrange setup, use `.expect("valid fixture config")` with a clear invariant message, or return `Result` and use `?`.
- In Assert, never call `.unwrap()` on the Act result. Check the `assert_eq!(actual, Ok(expected))` result so `Err` prints its `Debug` value.

## 4. Readable Test Data

- Use the named constants or clear domain literals (`"alice@example.com"`, `Duration::from_secs(5)`) instead of magic numbers (`42`, `9999`).
- Construct only the parameters that affect the tested behavior. Default the rest with struct update syntax or `#[fixture]` defaults.

## Completion

Completion: test modules compile with zero compiler diagnostics, use `#[expect(..., reason = "...")]` for lint suppression, and contain zero control-flow loops.
