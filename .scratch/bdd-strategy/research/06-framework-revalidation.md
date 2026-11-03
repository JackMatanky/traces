# Evidence for ticket 06: framework re-validation

This file supports [06: Framework
re-validation](../issues/06-framework-revalidation.md). The spike was a
throwaway crate at `/tmp/bdd-spike`. It is not part of the repo. This file is
the durable record of what the spike did and found.

## Setup

- The scratch package `spike` uses edition 2024 and `rust-version = "1.96"`. It
  has one small lib (`pub const fn add(a: u32, b: u32) -> u32`) and a
  `test-utils` feature.
- Copied from the repo: the `[lints.clippy]`, `[lints.rust]` and
  `[lints.rustdoc]` tables from `Cargo.toml`. Also copied: `clippy.toml`,
  `rust-toolchain.toml`, `rustfmt.toml` and `deny.toml`.
- Dev-deps: `rstest = { version = "0.27.0", default-features = false }`,
  `rstest-bdd = { version = "=0.6.0", default-features = false }`,
  `rstest-bdd-macros = "=0.6.0"`.
- Feature flags: some runs, from the `modular` test onward, had the
  `rstest-bdd-macros` feature `strict-compile-time-validation` on. Rows 8 and 9
  tested that feature on purpose. Row 16 ran with no feature flags. Row 16 is
  the recommended configuration.
- Toolchain: `nightly-2026-10-01`, clippy 0.1.100, macOS arm64 only.
- Run each command from the repo directory, so `mise` selects the toolchain. Set
  `CARGO_TARGET_DIR=/tmp/bdd-spike/target` and use `--manifest-path`.
- Commands:
  - `cargo clippy --all-targets --features test-utils -- -D warnings`
  - `cargo nextest run --features test-utils --test <name> --no-tests fail`
  - `cargo deny check licenses bans sources`

## Final step shape (no World, plain fixtures)

```rust
use rstest::fixture;
use rstest_bdd_macros::{given, scenarios, then, when};

#[derive(Default)]
struct Total(u32);

#[fixture]
fn total() -> Total {
    Total::default()
}

#[given("a calculator")]
fn a_calculator() {}

#[when("I add {a} and {b}")]
fn add(total: &mut Total, a: u32, b: u32) {
    total.0 = spike::add(a, b);
}

#[then("the result is {c}")]
fn result_is(total: &Total, c: u32) {
    assert_eq!(total.0, c);
}

scenarios!("tests/features", fixtures = [total: Total]);
```

The entry file below matches the `#[path]` convention of the repo:

```rust
#![cfg(feature = "test-utils")]
#[expect(
    clippy::disallowed_methods,
    reason = "rstest-bdd step wrappers call std::panic::catch_unwind"
)]
#[path = "final/bdd.rs"]
mod bdd;
```

In a crate-root test file, a plain `mod bdd;` resolves to `tests/bdd.rs`. It
does not resolve to `tests/<entry>/bdd.rs`. Use `#[path]`.

The feature file covered a Background, a Scenario Outline with an Examples
table, a data table step, and a `Result`-returning `Then` step.

## Results

| # | Probe | Outcome |
|---|---|---|
| 1 | Clippy, typed placeholders `{a:u32}` | `literal_string_with_formatting_args` reports an error on each typed placeholder. Bare `{a}` does not. |
| 2 | Clippy, sync steps | `disallowed_methods` (`std::panic::catch_unwind`, `clippy.toml:97`) reports an error on every `#[given]`, `#[when]` and `#[then]`. The call is in the generated sync wrapper. |
| 3 | `#[expect(clippy::disallowed_methods)]` on a step fn, above or below the step attribute | The expectation is unfulfilled on each fn. The original errors remain. |
| 4 | `#[allow]` on the enclosing module, and `#![expect]` at the crate root | Clean. |
| 5 | An `async fn` step next to a sync step | The async step does not report the lint. It uses the `catch_unwind_future` of the library. The sync step reports it. Not adopted. |
| 6 | Nextest | Each scenario is a test. Outline rows are `::case_N`. Names look like `features_scenarios::calc_add`. `--no-tests fail` works. An undefined step and a wrong assertion both fail with clear messages. |
| 7 | `unwrap`, `expect` and `_world`-style parameters in steps | Clean under the repo lints. |
| 8 | `strict-compile-time-validation` with `scenarios!` | An undefined step still compiles. It fails at run time (`Step not found at index N`). |
| 9 | The same feature with `#[scenario]` | Compile error: `No matching step definition found for ...`. |
| 10 | Edit an existing `.feature` file | The rebuild picks up the edit. The rename showed in `nextest list`. |
| 11 | Add a new `.feature` file, with no build script | The build does not pick it up until a `.rs` file changes. |
| 12 | `build.rs` with `println!("cargo::rerun-if-changed=tests/features");` | The build picks up the new file. `cargo build -v` printed `Dirty spike: the file tests/features has changed`. Cargo ran the script again and compiled the lib again. `build.rs` raised no `print_stdout` error. |
| 13 | `cargo deny check licenses bans sources` with the repo `deny.toml` | `bans ok, licenses ok, sources ok`. The advisories check did not run. |
| 14 | `cargo tree -i rstest` | One `rstest v0.27.0`. |
| 15 | A one-line `#[fixture]` body | `unused_braces` error. A multi-line body is clean. This is a quirk of `rstest` and clippy, not of `rstest-bdd`. |
| 16 | The entry file above: `#[expect]` on the `mod bdd` declaration, no step-level suppression, no `rstest-bdd-macros` feature flags | Clippy is clean with `-D warnings`. Nextest ran the scenario and it passed. This is the chosen option. |

## Upstream sources read

- The `rstest-bdd` README, the users' guide and `known-issues.md`.
- ADR-010 (feature-file change detection).
- The 0.6.0 migration guide, the design document, the roadmap and the CI
  sections of the developers' guide.
- The 0.6.0 crate source, read with rust-docs-mcp:
  - `rstest-bdd-macros` `codegen/wrapper/emit/assembly/mod.rs` (sync wrapper,
    `catch_unwind`)
  - `macros/scenarios/` (no step validation)
  - `macros/scenario/mod.rs:147` (validation only for `#[scenario]`)
  - `rstest-bdd` `registry/mod.rs` (duplicate panic, specificity resolution)

## Verified here versus inferred

Verified in the spike: rows 1 to 16 above. All runs used macOS arm64 only.

Inferred from the config files, not run:

- the `check:layers` and `check:visibility` interplay
- the `.config/nextest.toml` timeouts
- the doctest phase
- `required-features` on the existing `integration` target
- the e2e binary built with `--no-default-features`
- Windows and Linux behavior
- if edits to existing feature files also cause the whole-package rebuild under
  a `build.rs`
