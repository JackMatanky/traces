//! End-to-end BDD scenarios and step definitions using `rstest-bdd`.
//!
//! # Feature file change detection
//!
//! Rustc tracks edits to existing `.feature` files through dep-info, but
//! adding a new `.feature` file is not detected automatically without a
//! build script. After you add a feature file, run:
//!
//! ```sh
//! touch tests/e2e/bdd/mod.rs
//! ```
//!
//! Do not add a `build.rs`.

use rstest::fixture;
use rstest_bdd_macros::scenarios;

use super::support::{Run, Sandbox};

/// Stores the outcome of the most recently executed command in a scenario.
pub(crate) struct LastRun(pub(crate) Option<Run>);

#[fixture]
fn sandbox() -> Sandbox {
    Sandbox::new()
}

#[fixture]
fn last_run() -> LastRun {
    LastRun(None)
}

mod configuration;
mod index;
mod note;
mod process;
mod query;
mod schema_temporal;
mod template;
mod tracking;
mod trust;

scenarios!("tests/features", fixtures = [sandbox: Sandbox, last_run: LastRun]);
