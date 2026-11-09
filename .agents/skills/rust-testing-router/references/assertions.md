# Assertions

> An assertion that fails without context is a puzzle. An assertion that fails with context is a diagnosis.

## 1. Use `pretty_assertions` and Separate Act from Assert

`std::assert_eq!` and `assert_ne!` print a single-line diff for structs and multi-line strings. Import `pretty_assertions::{assert_eq, assert_ne}` in the test module to get colorized, line-by-line diffs.

Keep the Act call separate from the Assert macro. Do not chain `.unwrap()` inside `assert_eq!` on the call under test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::{assert_eq, assert_ne};

    #[test]
    fn parses_config_from_toml() {
        let parsed = Config::parse(TOML_INPUT);

        assert_eq!(parsed, Ok(Config::default_for_test()));
    }
}
```

## 2. Choosing the Right Assertion

| Situation | Use |
|---|---|
| Two values must be equal or unequal | `pretty_assertions::assert_eq!` / `assert_ne!` |
| Enum variant or structural pattern | `assert!(matches!(value, Variant::Foo { .. }), "got {value:?}")` |
| Boolean predicate (`is_empty`, custom check) | `assert!(cond, "context: {value:?}")` |
| Large structured output (JSON, CLI table, diagnostics) | Snapshot via [`snapshots.md`](snapshots.md) |
| Deliberate API panic contract | `#[should_panic(expected = "...")]` |

## 3. Always Include Failure Context

A bare `assert!(result.is_ok())` hides the error variant when it fails. Pass format arguments to every boolean `assert!`:

```rust
// Bad: failure output says only "assertion failed: result.is_ok()"
assert!(result.is_ok());

// Good: failure output prints the unexpected error
assert!(
    result.is_ok(),
    "expected valid parse, got: {result:?}"
);
```

## 4. Structured Error Assertions

Assert on structured error variants, fields, or stable diagnostic codes rather than brittle human-prose substrings:

```rust
let result = parse_query(invalid_input);

assert!(
    matches!(
        result,
        Err(QueryError::InvalidSyntax { line: 1, column: 5, .. })
    ),
    "expected InvalidSyntax at 1:5, got {result:?}"
);
```

When testing across a CLI boundary where only stderr text is available, assert on stable error tokens or diagnostic codes rather than full sentences.

## 5. Panic Contracts

Use `#[should_panic(expected = "...")]` only when panicking is the documented API contract (for example, out-of-bounds index operators or invariant constructors). Always set `expected = "..."` to match the specific panic message so an unrelated setup panic does not pass the test.

## Anti-Patterns

| Anti-Pattern | Fix |
|---|---|
| `assert_eq!(fallible().unwrap(), expected)` | Assert `assert_eq!(fallible(), Ok(expected))` or bind with `?` / `matches!` |
| Bare `assert!(x.is_ok())` with no message | Use `matches!` or add `"context: {x:?}"` |
| String-matching full error prose in unit tests | Match the structured `Err(Variant { .. })` |
| `assert_eq!` on a 50-line string literal | Use [`snapshots.md`](snapshots.md) |

## Completion

Completion: every test assertion names the expected contract, prints diagnostic values on failure, and avoids unwrapping the Act result without context.
