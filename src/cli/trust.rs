//! Trust-store mutation and inspection.
//!
//! Handles `traces trust` by resolving a directory or `.traces/config.toml` to
//! trust subjects, then listing, cleaning, showing, or recording trusted
//! workspace roots.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::error::{CliError, CliResult};
use crate::{TrustRequest, config::TrustStore};

/// Arguments for `traces trust`.
///
/// Grants trust to a project root (default), lists trusted roots, shows trust
/// status, or cleans stale entries. Pass `--all` to operate on descendant
/// configs.
#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub(super) struct Trust {
    #[command(subcommand)]
    action: Option<TrustAction>,
    /// Show resolved configuration trust status instead of changing it.
    #[arg(long)]
    show: bool,
    /// Apply trust or status checks to descendant configurations too.
    #[arg(long)]
    all: bool,
    /// Directory or `.traces/config.toml` to trust.
    ///
    /// Defaults to the current directory.
    path: Option<PathBuf>,
}

impl Trust {
    /// Runs `traces trust` with the selected trust action.
    ///
    /// # Errors
    ///
    /// - [`CliError`] if the dispatched trust action fails.
    #[inline]
    pub(super) fn run(self, trust: &TrustStore) -> CliResult {
        match self.action {
            Some(TrustAction::List) => Self::list(trust),
            Some(TrustAction::Clean) => Self::clean(trust),
            None if self.show => self.show(trust),
            None => self.trust(trust),
        }
    }

    /// Prints all trusted directories to stdout, one per line.
    ///
    /// # Errors
    ///
    /// - [`CliError::TrustList`] if reading the trust store fails.
    #[expect(
        clippy::print_stdout,
        reason = "trust list's output is data meant to be piped, not \
                  diagnostic text; see the print_stderr precedent this mirrors"
    )]
    fn list(trust: &TrustStore) -> CliResult {
        let roots: Vec<PathBuf> =
            trust.list().map_err(|source| CliError::TrustList {
                source,
            })?;
        for root in &roots {
            println!("{}", root.display());
        }
        Ok(())
    }

    /// Removes dangling trust entries from the trust store.
    ///
    /// # Errors
    ///
    /// - [`CliError::TrustClean`] if cleaning the trust store fails.
    fn clean(trust: &TrustStore) -> CliResult {
        let removed = trust.clean().map_err(|source| CliError::TrustClean {
            source,
        })?;
        match removed {
            1 => eprintln!("removed 1 stale trust entry"),
            n => eprintln!("removed {n} stale trust entries"),
        }
        Ok(())
    }

    /// Displays trust status for the target configuration.
    ///
    /// # Errors
    ///
    /// - [`CliError::TrustTargetResolve`] if resolving trust targets fails.
    /// - [`CliError::TrustShow`] if reading trust status fails.
    #[expect(
        clippy::print_stdout,
        reason = "trust --show's output is data meant to be piped, not \
                  diagnostic text; see the print_stderr precedent this mirrors"
    )]
    fn show(&self, trust: &TrustStore) -> CliResult {
        self.for_each_subject(|subject: TrustRequest| {
            let root = subject.root_path().to_path_buf();
            let path = subject
                .config_file()
                .unwrap_or_else(|| subject.root_path())
                .to_path_buf();
            let state = trust.trust_status(&subject).map_err(|source| {
                CliError::TrustShow {
                    root,
                    source,
                }
            })?;
            println!("{}\t{}", path.display(), state);
            Ok(())
        })
    }

    /// Grants trust to the specified target directory or configuration.
    ///
    /// # Errors
    ///
    /// - [`CliError::TrustTargetResolve`] if resolving trust targets fails.
    /// - [`CliError::Trust`] if updating the trust store fails.
    fn trust(&self, trust: &TrustStore) -> CliResult {
        self.for_each_subject(|subject: TrustRequest| {
            let root = subject.root_path().to_path_buf();
            if let Err(source) = trust.trust(&subject) {
                return Err(CliError::Trust {
                    root,
                    source,
                });
            }
            eprintln!("trusted {}", root.display());
            Ok(())
        })
    }

    /// Applies `visit` to every trust request resolved from arguments.
    ///
    /// # Errors
    ///
    /// - [`CliError::CurrentDirectory`] if no path was provided and reading
    ///   current directory fails.
    /// - [`CliError::TrustTargetResolve`] if resolving trust targets fails.
    /// - Any [`CliError`] returned by `visit`.
    fn for_each_subject(
        &self,
        mut visit: impl FnMut(TrustRequest) -> CliResult,
    ) -> CliResult {
        let subjects =
            super::resolve_trust_subjects(self.path.as_deref(), self.all)?;
        for subject in subjects {
            visit(subject)?;
        }
        Ok(())
    }
}

/// Nested `traces trust` subcommands.
#[derive(Debug, Subcommand)]
enum TrustAction {
    /// List all trusted directories.
    List,
    /// Remove stale trust entries.
    Clean,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use clap::Parser;

    use super::*;
    use crate::config::ConfigTrustStatus;

    mod fixtures {
        use std::path::PathBuf;

        use clap::Parser;

        use super::super::*;

        /// Wraps [`Trust`] in a minimal top-level parser for clap tests.
        ///
        /// This allows [`Parser::try_parse_from`] to exercise
        /// `args_conflicts_with_subcommands`, which [`clap::Args`] types do not
        /// parse standalone.
        #[derive(Debug, Parser)]
        pub(super) struct TestCli {
            #[command(flatten)]
            pub(super) trust: Trust,
        }

        pub(super) fn parse(args: &[&str]) -> Trust {
            TestCli::try_parse_from(
                std::iter::once("test").chain(args.iter().copied()),
            )
            .expect("parse trust args")
            .trust
        }

        pub(super) fn trust_args(path: Option<PathBuf>) -> Trust {
            Trust {
                action: None,
                show: false,
                all: false,
                path,
            }
        }

        pub(super) fn action_args(action: TrustAction) -> Trust {
            Trust {
                action: Some(action),
                show: false,
                all: false,
                path: None,
            }
        }
    }
    use fixtures::*;

    use crate::test_support::{TestProject, fixture_service};

    mod parsing {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn bare_trust_has_no_path_and_no_action() {
            let args = parse(&[]);

            assert!(args.path.is_none());
            assert!(args.action.is_none());
        }

        #[test]
        fn trust_with_a_path_sets_path_and_no_action() {
            let args = parse(&["some/path"]);

            assert_eq!(args.path, Some(PathBuf::from("some/path")));
            assert!(args.action.is_none());
        }

        #[test]
        fn trust_list_is_the_list_action_not_a_path_named_list() {
            let args = parse(&["list"]);

            assert!(matches!(args.action, Some(TrustAction::List)));
            assert!(args.path.is_none());
        }

        #[test]
        fn trust_clean_is_the_clean_action_not_a_path_named_clean() {
            let args = parse(&["clean"]);

            assert!(matches!(args.action, Some(TrustAction::Clean)));
            assert!(args.path.is_none());
        }

        #[test]
        fn show_mode_parses() {
            let show = parse(&["--show", "some/path"]);

            assert!(show.show);
            assert_eq!(show.path, Some(PathBuf::from("some/path")));
        }

        #[test]
        fn all_mode_parse() {
            let args = parse(&["--all", "some/path"]);

            assert!(args.all);
            assert_eq!(args.path, Some(PathBuf::from("some/path")));
        }

        #[test]
        fn combining_an_action_with_a_path_is_rejected() {
            let result = fixtures::TestCli::try_parse_from([
                "test",
                "list",
                "some/path",
            ]);

            assert!(result.is_err());
        }
    }

    mod trust {
        use pretty_assertions::assert_eq;

        use super::*;
        use crate::cli::CwdGuard;

        #[test]
        fn with_no_path_trusts_the_discovered_project_root() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().join("project");
            let cwd = root.join("notes/daily");
            fs::create_dir_all(&cwd).expect("create nested cwd");
            TestProject::empty(&root).write_file(".traces/config.toml", "");
            let service = fixture_service(temp.path());
            let _guard = CwdGuard::enter(&cwd);

            super::trust_args(None)
                .run(service.trust_store())
                .expect("trust cwd");

            assert_eq!(
                service.trust_store().list().expect("list trusted"),
                vec![root.canonicalize().expect("canonicalize root")]
            );
            assert_eq!(
                service
                    .trust_store()
                    .trust_status(&TrustRequest::from(root.as_path()))
                    .expect("check trust"),
                ConfigTrustStatus::Trusted
            );
        }

        #[test]
        fn accepts_a_config_file_path() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().join("project");
            let config_file =
                TestProject::empty(&root).write_file(".traces/config.toml", "");
            let service = fixture_service(temp.path());

            super::trust_args(Some(config_file))
                .run(service.trust_store())
                .expect("trust config file");

            assert_eq!(
                service.trust_store().list().expect("list trusted"),
                vec![root.canonicalize().expect("canonicalize root")]
            );
        }

        #[test]
        fn missing_config_trusts_the_directory_target() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().join("project");
            fs::create_dir_all(&root).expect("create project dir");
            let service = fixture_service(temp.path());

            super::trust_args(Some(root.clone()))
                .run(service.trust_store())
                .expect("trust directory");

            assert_eq!(
                service.trust_store().list().expect("list trusted"),
                vec![root.canonicalize().expect("canonicalize root")]
            );
            assert_eq!(
                service
                    .trust_store()
                    .trust_status(&TrustRequest::from(root.as_path()))
                    .expect("check trust"),
                ConfigTrustStatus::Trusted
            );
        }

        #[test]
        fn all_mode_trusts_descendant_configs() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let parent = temp.path().join("parent");
            let child = parent.join("child");
            fs::create_dir_all(&child).expect("create child dir");
            TestProject::empty(&parent).write_file(".traces/config.toml", "");
            TestProject::empty(&child).write_file(".traces/config.toml", "");
            let service = fixture_service(temp.path());
            let mut args = super::trust_args(Some(parent));
            args.all = true;

            args.run(service.trust_store()).expect("trust all configs");

            assert_eq!(
                service.trust_store().list().expect("list trusted").len(),
                2
            );
        }
    }

    mod show {
        use super::*;

        #[test]
        fn checks_status_without_changing_trust_store() {
            use pretty_assertions::assert_eq;
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().join("project");
            fs::create_dir_all(&root).expect("create project dir");
            TestProject::empty(&root).write_file(".traces/config.toml", "");
            let service = fixture_service(temp.path());
            let mut args = super::trust_args(Some(root));
            args.show = true;

            args.run(service.trust_store()).expect("show trust status");

            assert_eq!(
                service.trust_store().list().expect("list trusted"),
                Vec::<std::path::PathBuf>::new()
            );
        }
    }

    mod list {
        use super::*;

        #[test]
        fn succeeds_against_an_empty_trust_store() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let service = fixture_service(temp.path());

            super::action_args(TrustAction::List)
                .run(service.trust_store())
                .expect("list empty trust store");
        }
    }

    mod clean {
        use super::*;

        #[test]
        fn removes_a_stale_root() {
            use pretty_assertions::assert_eq;
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().join("project");
            TestProject::empty(&root).write_file(".traces/config.toml", "");
            let service = fixture_service(temp.path());
            super::trust_args(Some(root.clone()))
                .run(service.trust_store())
                .expect("trust root");
            fs::remove_dir_all(&root).expect("delete project dir");

            super::action_args(TrustAction::Clean)
                .run(service.trust_store())
                .expect("clean trust store");

            assert_eq!(
                service.trust_store().list().expect("list trusted"),
                Vec::<std::path::PathBuf>::new()
            );
        }

        #[test]
        fn on_an_empty_trust_store_does_not_error() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let service = fixture_service(temp.path());

            super::action_args(TrustAction::Clean)
                .run(service.trust_store())
                .expect("clean empty trust store");
        }
    }
}
