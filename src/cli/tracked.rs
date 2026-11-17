//! Inspects and cleans the tracked-configuration store.
//!
//! Handles `traces tracked` by listing or cleaning the best-effort store of
//! local configuration files that [`crate::ConfigService::load`] reads. This
//! store is separate from trust decisions in [`super::trust`]. Tracking records
//! which configuration files traces loads. Trust controls whether a workspace
//! can run configuration files and templates.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::error::{CliError, CliResult};
use crate::config::TrackerStore;

/// Defines arguments for `traces tracked`.
///
/// Inspects or cleans the best-effort store of local configuration files that
/// [`crate::ConfigService::load`] reads.
#[derive(Debug, Args)]
pub(super) struct Tracked {
    /// Subcommand action to run.
    #[command(subcommand)]
    action: TrackedAction,
}

impl Tracked {
    /// Runs `traces tracked` with the selected action.
    ///
    /// # Errors
    ///
    /// - [`CliError::TrackedList`] if reading the tracked-configuration store
    ///   fails.
    /// - [`CliError::TrackedClean`] if cleaning the tracked-configuration store
    ///   fails.
    pub(super) fn run(self, tracked: &TrackerStore) -> CliResult {
        match self.action {
            TrackedAction::List => Self::list(tracked),
            TrackedAction::Clean => Self::clean(tracked),
        }
    }

    /// Prints all tracked configuration paths to standard output, one per line.
    ///
    /// # Errors
    ///
    /// - [`CliError::TrackedList`] if reading the tracked-configuration store
    ///   fails.
    #[expect(
        clippy::print_stdout,
        reason = "tracked list's output is data meant to be piped, not \
                  diagnostic text; mirrors the precedent in crate::cli::trust"
    )]
    fn list(tracked: &TrackerStore) -> CliResult {
        let paths: Vec<PathBuf> =
            tracked.list().map_err(|source| CliError::TrackedList {
                source,
            })?;
        for path in &paths {
            println!("{}", path.display());
        }
        Ok(())
    }

    /// Removes dangling entries from the tracked-configuration store.
    ///
    /// # Errors
    ///
    /// - [`CliError::TrackedClean`] if cleaning the tracked-configuration store
    ///   fails.
    fn clean(tracked: &TrackerStore) -> CliResult {
        let removed =
            tracked.clean().map_err(|source| CliError::TrackedClean {
                source,
            })?;
        match removed {
            1 => eprintln!("removed 1 stale tracked entry"),
            n => eprintln!("removed {n} stale tracked entries"),
        }
        Ok(())
    }
}

/// Enumerates nested `traces tracked` subcommands.
#[derive(Debug, Subcommand)]
enum TrackedAction {
    /// Lists all tracked local configuration paths.
    List,
    /// Removes stale tracked configuration entries.
    Clean,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fixtures {
        use clap::Parser;

        use super::*;

        /// Wraps [`Tracked`] in a minimal top-level parser for clap tests.
        #[derive(Debug, Parser)]
        pub(super) struct TestCli {
            #[command(flatten)]
            pub(super) tracked: Tracked,
        }

        pub(super) fn parse(args: &[&str]) -> Tracked {
            TestCli::try_parse_from(
                std::iter::once("test").chain(args.iter().copied()),
            )
            .expect("parse tracked args")
            .tracked
        }
    }
    use fixtures::*;

    mod parsing {
        use super::*;

        #[test]
        fn list_argv_maps_to_list_action() {
            let args = parse(&["list"]);
            assert!(matches!(args.action, TrackedAction::List));
        }

        #[test]
        fn clean_argv_maps_to_clean_action() {
            let args = parse(&["clean"]);
            assert!(matches!(args.action, TrackedAction::Clean));
        }
    }
}
