# Evidence for ticket 06: framework re-validation

Supports [06: Framework re-validation](../issues/06-framework-revalidation.md).
The spike was a throwaway crate at `/tmp/bdd-spike`, not part of the repo. This
file is the durable record of what it did and found.

## Setup

- Scratch package `spike`: edition 2024, `rust-version = "1.96"`, one tiny lib
  (`pub const fn add(a: u32, b: u32) -> u32`), a `test-utils` feature.
- Copied from the repo: the `[lints.clippy]`, `[lints.rust]` and
  `[lints.rustdoc]` tables from `Cargo.toml`, plus `clippy.toml`,
  `rust-toolchain.toml`, `rustfmt.toml` and `deny.toml`.
- Dev-deps: `rstest = { version = "0.27.0", default-features = false }`,
  `rstest-bdd = { version = "=0.6.0", default-features = false }`,
  `rstest-bdd-macros = "=0.6.0"` (one run used its
  `strict-compile-time-validation` feature).
- Toolchain: `nightly-2026-10-01`, clippy 0.1.100, macOS arm64 only. Run from
  the repo directory so `mise` selects that toolchain, with
  `CARGO_TARGET_DIR=/tmp/bdd-spike/target` and `--manifest-path`.
- Commands: `cargo clippy --all-targets --features test-utils -- -D warnings`,
  `cargo nextest run --features test-utils --test <name> --no-tests fail`,
  `cargo deny check licenses bans sources`.

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

Entry file shape that matches the repo's `#[path]` convention:

```rust
#![cfg(feature = "test-utils")]
#[path = "modular/bdd.rs"]
mod bdd;
```

A plain `mod bdd;` in a crate-root test file resolves to `tests/bdd.rs`, not
`tests/<entry>/bdd.rs`. Use `#[path]`.

The feature file covered Background, Scenario Outline with an Examples table, a
data table step, and a `Result`-returning `Then` step.

## Results

| # | Probe | Outcome |
|---|---|---|
| 1 | Clippy, typed placeholders `{a:u32}` | `literal_string_with_formatting_args` fires on each typed placeholder. Bare `{a}` does not fire. |
| 2 | Clippy, sync steps | `disallowed_methods` (`std::panic::catch_unwind`, `clippy.toml:97`) fires on every `#[given]`/`#[when]`/`#[then]`. The call is in the generated sync wrapper. |
| 3 | `#[expect(clippy::disallowed_methods)]` on a step fn, above or below the step attribute | Unfulfilled expectation on each fn, and the original errors remain. |
| 4 | `#[allow]`/`#[expect]` on the enclosing module (or crate root) | Clean. Chosen option: `#[expect]` on the `mod bdd` declaration. |
| 5 | `async fn` step beside a sync step | Async step does not fire the lint (uses the library's `catch_unwind_future`); the sync step does. Not adopted. |
| 6 | Nextest | Each scenario is a test; outline rows are `::case_N`; names look like `features_scenarios::calc_add`. `--no-tests fail` fine. Undefined step and a wrong assertion both fail with clear messages. |
| 7 | `unwrap`/`expect` and `_world`-style params in steps | Clean under the repo lints. |
| 8 | `strict-compile-time-validation` with `scenarios!` | Undefined step still compiles; fails at runtime (`Step not found at index N`). |
| 9 | Same feature with `#[scenario]` | Compile error: `No matching step definition found for ...`. |
| 10 | Edit an existing `.feature` file | Rebuild picks it up (rename showed in `nextest list`). |
| 11 | Add a new `.feature` file, no build script | Not picked up until a `.rs` file changes. |
| 12 | `build.rs` with `println!("cargo::rerun-if-changed=tests/features");` | New file picked up. `cargo build -v` printed `Dirty spike: the file tests/features has changed`, re-ran the script and recompiled the lib. No `print_stdout` error in `build.rs`. |
| 13 | `cargo deny check licenses bans sources` with repo `deny.toml` | `bans ok, licenses ok, sources ok`. Advisories not run. |
| 14 | `cargo tree -i rstest` | Single `rstest v0.27.0`. |
| 15 | One-line `#[fixture]` body | `unused_braces` error; multi-line body clean (`rstest`/clippy, not `rstest-bdd`). |

## Upstream sources read

- `rstest-bdd` README, users' guide, `known-issues.md`, ADR-010 (feature-file
  change detection), 0.6.0 migration guide, design doc, roadmap, developers'
  guide (CI sections).
- 0.6.0 crate source via rust-docs-mcp: `rstest-bdd-macros`
  `codegen/wrapper/emit/assembly/mod.rs` (sync wrapper, `catch_unwind`),
  `macros/scenarios/` (no step validation), `macros/scenario/mod.rs:147`
  (validation only for `#[scenario]`), `rstest-bdd` `registry/mod.rs` (duplicate
  panic, specificity resolution).

## Verified here versus inferred

Verified in the spike: rows 1 to 14 above (macOS arm64 only).

Inferred from reading config, not run: `check:layers` and `check:visibility`
interplay, `.config/nextest.toml` timeouts, the doctest phase,
`required-features` on the existing `integration` target, the e2e binary built
with `--no-default-features`, Windows and Linux behaviour, whether feature edits
(not just additions) trigger the whole-package rebuild under a `build.rs`.
