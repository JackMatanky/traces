# Table-Driven Cases (`#[rstest]`)

> Use `#[rstest]` with `#[case]` when one behavior contract has multiple named literal input/output pairs.

## 1. When to Use

Use a parameterized `#[rstest]` function when Arrange, Act, and Assert steps match and only literal inputs or outputs change.

```rust
use pretty_assertions::assert_eq;
use rstest::rstest;

#[rstest]
#[case::empty_input("", 0)]
#[case::single_word("rust", 1)]
#[case::trimmed_whitespace("  hello world  ", 2)]
fn counts_words_in_line(#[case] input: &str, #[case] expected: usize) {
    let count = count_words(input);

    assert_eq!(count, expected, "failed for input {input:?}");
}
```

## 2. Case Naming Rules

- Always name every case with `#[case::descriptive_scenario_name(...)]`.
- Name the case after the boundary or condition (`#[case:: trailing_newline]`, `#[case::max_u16_boundary]`), not the raw value (`#[case::case_1]`).
- Keep `Ok` cases and `Err` cases in separate parameterized functions so the test body has zero `if`/`match` branching.

## 3. Promotion Boundaries (When Not to Use `#[case]`)

Check these three boundaries before adding more `#[case]` rows:

- **Promote to [`property-based.md`](property-based.md)** when asserting a general law (round-trip, idempotence, ordering) across a large input space.
- **Promote to a Gherkin `Scenario Outline`** (see [`../../rust-bdd-testing/references/formulation.md`](../../rust-bdd-testing/references/formulation.md)) only when the table expresses a stakeholder-facing business rule at the acceptance layer.
- **Combine with [`rstest-fixtures.md`](rstest-fixtures.md)** when cases share complex setup objects (`#[fixture]`) or test a Cartesian product of orthogonal options (`#[values(...)]`).

## Completion

Completion: each `#[rstest]` table tests one behavior, labels every `#[case::name]`, and contains zero conditional branches in the test body.
