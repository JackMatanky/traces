# Naming and Organizing Unit Tests

> A failing test name in CI logs should tell you what broke and under what condition without opening the source file.

## 1. Precedence

Project conventions, `AGENTS.md`, and local style guides take precedence over the default naming patterns below. Match the existing naming convention of the module you are editing.

## 2. Default Naming Pattern: `<unit>_<scenario>_<expected_outcome>`

Write snake_case names that state the operation, the input condition, and the expected outcome:

| Weak Name | Strong Name |
|---|---|
| `test_parse` | `parse_port_with_valid_digits_returns_port_number` |
| `test_error` | `parse_port_above_u16_max_returns_overflow_error` |
| `withdraw_test` | `withdraw_exceeding_balance_leaves_account_unchanged` |
| `issue_142` | `lexer_unclosed_quote_at_eof_returns_unterminated_string_error` |

Avoid the redundant `test_` prefix (`#[test]` already namespaces the function in the test runner). When a submodule already names the unit under test, drop the `<unit>_` prefix from each function name.

## 3. Grouping Large Suites with Nested Submodules

When a `#[cfg(test)] mod tests` block tests multiple methods or types, group functions into nested modules named after the method or type. The test runner prints the full path (`tests::withdraw::exceeding_balance_returns_insufficient_funds`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod deposit {
        use super::*;

        #[test]
        fn positive_amount_increases_balance() { /* ... */ }

        #[test]
        fn zero_amount_is_rejected() { /* ... */ }
    }

    mod withdraw {
        use super::*;

        #[test]
        fn amount_within_balance_succeeds() { /* ... */ }

        #[test]
        fn amount_exceeding_balance_returns_insufficient_funds() { /* ... */ }
    }
}
```

## 4. Naming `#[rstest]` Case Rows

When using `#[rstest]`, name every `#[case::descriptive_label(...)]` so `nextest` outputs `fn_name::case_1_descriptive_label` instead of an opaque index:

```rust
#[rstest]
#[case::empty_slice(&[], 0)]
#[case::single_item(&[5], 5)]
#[case::multiple_items(&[2, 3, 4], 9)]
fn sum_returns_total(#[case] items: &[i32], #[case] expected: i32) {
    assert_eq!(sum(items), expected);
}
```

## Completion

Completion: every test function and `#[case::label]` states its condition and expected outcome without redundant `test_` prefixes or opaque issue numbers.
