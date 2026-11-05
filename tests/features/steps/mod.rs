//! Step definitions for the scenarios in `tests/features/`, using `rstest-bdd`.
//!
//! The one `scenarios!` call below discovers every `.feature` file under
//! `tests/features/`, including subfolders. Steps are grouped by domain
//! concept, not by feature file.

use rstest::fixture;
use rstest_bdd_macros::scenarios;

use crate::support::{Run, Sandbox};

/// Stores the outcome of the most recently executed command in a scenario.
pub(crate) struct LastRun(Option<Run>);

impl LastRun {
    /// Returns the last command outcome; fails the scenario when none ran.
    pub(crate) fn run(&self) -> &Run {
        self.0.as_ref().expect("a prior command must have run")
    }

    /// Records the outcome of the command a step just ran.
    pub(crate) fn set(&mut self, run: Run) {
        self.0 = Some(run);
    }
}

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
