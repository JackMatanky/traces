# Property-Based Testing (`proptest`)

> Use `proptest` when an invariant or algebraic law holds across a class of generated inputs.

## 1. When to Use

Reach for `proptest` when you can state a universal law over a domain:

| Property Law | Shape |
|---|---|
| Round-trip identity | `decode(encode(x)) == Ok(x)` |
| Idempotence | `normalize(normalize(x)) == normalize(x)` |
| Structural invariant | `sorted(v).windows(2).all(\|w\| w[0] <= w[1])` |
| Differential oracle | `fast_impl(x) == reference_impl(x)` |
| Commutativity / Associativity | `merge(a, b) == merge(b, a)` |

Keep deterministic boundary unit tests (`0`, `MIN`, `MAX`, empty) alongside property tests. Property generators sample the space, while explicit unit cases pin exact boundaries on every run.

For coverage-guided exploration of raw byte streams or deep recursion limits, see [`fuzzing.md`](fuzzing.md).

## 2. Basic Usage

Use `prop_assert!` and `prop_assert_eq!` inside `proptest!` so failures trigger automatic shrinking instead of aborting immediately:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn slug_roundtrips_through_formatter(raw in "[a-z0-9]+(-[a-z0-9]+)*") {
        let parsed = Slug::parse(&raw).expect("valid generator input");

        prop_assert_eq!(parsed.as_str(), raw);
    }
}
```

## 3. Composing Domain Strategies (Avoid `prop_assume!` Abuse)

Build valid domain inputs by composing strategies with `.prop_map(...)` and `.prop_flat_map(...)` rather than filtering arbitrary inputs with `prop_assume!`.

- Keep `prop_assume!` rejection rates below 10%. High rejection rates slow test runs and abort `proptest` with `TooManyRejections`.
- Construct valid structures directly:

```rust
use proptest::prelude::*;

fn valid_range_strategy() -> impl Strategy<Value = (u32, u32)> {
    (0..10_000u32).prop_flat_map(|start| (Just(start), start..=10_000u32))
}
```

## 4. Commit `proptest-regressions/` to Git

When `proptest` finds and shrinks a failing case, it writes the minimal seed to a `proptest-regressions/` file next to the source or test file.

- **Always commit `proptest-regressions/` files to version control.**
- Committed regression seeds replay first on every future test run before random generation starts, locking in the fix deterministically.

## Completion

Completion: property tests use `prop_assert!` macros, keep `prop_assume!` rejections under 10%, and track `proptest-regressions/` in git.
