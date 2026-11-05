//! Executable specification: Gherkin scenarios that spawn the real `traces`
//! binary. Feature files live in `tests/features/<capability>/`; step
//! definitions live in `tests/features/steps/`. See `support` for isolation
//! guarantees.
//!
//! # Feature file change detection
//!
//! Rustc tracks edits to existing `.feature` files through dep-info, but adding
//! a new `.feature` file is not detected automatically without a build script.
//! After you add a feature file, `mise run test` detects the stale build, runs
//! `touch tests/features.rs`, and rebuilds. For any other test run, do it
//! yourself:
//!
//! ```sh
//! touch tests/features.rs
//! ```
//!
//! Do not add a `build.rs`.

#[path = "support/fixtures.rs"]
mod fixtures;
#[expect(
    clippy::disallowed_methods,
    reason = "rstest-bdd step wrappers call std::panic::catch_unwind"
)]
#[path = "features/steps/mod.rs"]
mod steps;
#[expect(
    dead_code,
    reason = "the e2e binary shares this harness and uses helpers this binary \
              does not"
)]
#[path = "support/mod.rs"]
mod support;
