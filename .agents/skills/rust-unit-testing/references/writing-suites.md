# Writing Unit Test Suites

> Obey this 6-step procedure to design a complete, non-redundant unit test suite for a Rust function, type, or module.

## Step 1: Audit Against the 8-Category Case-Surface Checklist

Inspect the signature, types, and invariants of the target unit. Check which of these eight categories apply before writing test code:

1. **Happy path:** Representative valid input produces the documented return value or state change.
2. **Boundary and adjacent values:** Minimum, maximum, `N - 1`, `N`, `N + 1`, `0`, `1`, `usize::MAX`, or numeric overflow edges.
3. **Empty, zero, and singleton collections:** `""`, `&[]`, `None`, single-element slices, and multi-byte UTF-8 boundaries.
4. **Invalid and malformed input:** Unclosed delimiters, invalid tokens, out-of-range parameters, or bad UTF-8 sequences.
5. **Reachable `Err` variants:** Every documented `Err(Variant)` in the return type is triggered and matched structurally.
6. **State transitions and ordering:** Valid sequences (`init -> active -> closed`), rejected out-of-order calls, and double-close behavior.
7. **Idempotency and round-trip laws:** Applying normalization twice yields the same result, and `parse(format(x)) == Ok(x)`.
8. **Resource and recursion depth limits:** Deeply nested inputs, capacity bounds, and timeout or cancellation paths.

## Step 2: Place the Test Module

Put unit tests in `#[cfg(test)] mod tests` at the bottom of `src/foo.rs`, or in `src/foo/tests.rs` above ~200 lines:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_valid_port() {
        let port = parse_port("8080");

        assert_eq!(port, Ok(8080));
    }
}
```

## Step 3: Structure Each Test (Arrange, Act, Assert)

Separate the three phases with a single blank line. Do not write `// Arrange`, `// Act`, or `// Assert` comments:

- **Arrange:** Construct only the state relevant to this test case. Use the builder pattern or `#[fixture]` functions so irrelevant fields stay out of sight.
- **Act:** Call the single method or function under test and bind its return value (`let actual = ...;`).
- **Assert:** Assert on `actual` using [`../../rust-testing-router/references/assertions.md`](../../rust-testing-router/references/assertions.md).

## Step 4: Choose Parameterized, Property, or Fuzz Techniques

Do not copy-paste five near-identical test functions when one data-driven or generative test is clearer:

- Discrete input/output rows (3+ cases) -> use `#[rstest]` with `#[case]` in [`../../rust-testing-router/references/table-driven.md`](../../rust-testing-router/references/table-driven.md).
- Shared setup with per-test overrides -> use `#[fixture]` in [`../../rust-testing-router/references/rstest-fixtures.md`](../../rust-testing-router/references/rstest-fixtures.md).
- Universal algebraic invariants across arbitrary valid inputs -> use `proptest` in [`../../rust-testing-router/references/property-based.md`](../../rust-testing-router/references/property-based.md).
- Untrusted byte streams or deep parser nesting -> use `cargo-fuzz` in [`../../rust-testing-router/references/fuzzing.md`](../../rust-testing-router/references/fuzzing.md).

## Step 5: Isolate External Collaborators

If the unit under test talks to a clock, network, or disk:

- Inject a narrow trait and use a hand-written fake from [`../../rust-testing-router/references/mocks.md`](../../rust-testing-router/references/mocks.md).
- Use the `tempfile::TempDir` guard from [`../../rust-testing-router/references/raii-cleanup.md`](../../rust-testing-router/references/raii-cleanup.md) for local file operations.

## Step 6: Confirm Discovery and Red-First Failure

1. List the library test targets to confirm the new tests are compiled and discovered:

   ```bash
   cargo nextest list -E 'kind(lib)'
   ```

2. Check that each new test fails when the production branch or invariant it guards is inverted (red-first proof).
3. Run the unit suite with `--no-tests=fail`:

   ```bash
   cargo nextest run --lib --no-tests=fail
   ```

## Completion

Completion: all applicable categories from the 8-Category Case-Surface Checklist are covered, tests are listed in `cargo nextest list -E 'kind(lib)'`, and the suite passes with `--no-tests=fail`.
