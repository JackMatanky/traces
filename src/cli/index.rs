//! Persisted index rebuild command.
//!
//! Handles `traces index` by loading the trusted project root, refreshing its
//! [`WorkspaceIndex`](crate::index::WorkspaceIndex), and replacing the stored
//! index. Task queries live in the task command module.

use clap::Args;

use super::error::{CliError, CliResult};
use crate::{Config, IndexerService};

/// Arguments for `traces index`.
///
/// Takes no positional or optional arguments. Scans the trusted project root
/// and persists a fresh [`WorkspaceIndex`](crate::index::WorkspaceIndex).
#[derive(Debug, Args)]
pub(super) struct Index;

impl Index {
    /// Runs `traces index` for the trusted project root.
    ///
    /// Scans the root, persists the fresh [`WorkspaceIndex`], and reports the
    /// indexed file count to stderr.
    ///
    /// # Errors
    ///
    /// - [`Index`] if scanning the project root or persisting the
    ///   [`WorkspaceIndex`] fails.
    ///
    /// [`Index`]: CliError::Index
    /// [`WorkspaceIndex`]: crate::index::WorkspaceIndex
    #[expect(
        clippy::unused_self,
        reason = "keeps the dispatch signature consistent with every other \
                  command handler (all args.run(&self, config)) so the \
                  Commands::run match arms are uniform"
    )]
    pub(super) fn run(&self, config: &Config) -> CliResult {
        let root = config.root().to_path_buf();
        let index =
            IndexerService::from(config).rebuild().map_err(|source| {
                CliError::Index {
                    root: root.clone(),
                    source,
                }
            })?;
        eprintln!(
            "indexed {} file(s) under {}",
            index.entries().len(),
            root.display()
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fixtures {
        use crate::index::WorkspaceIndex;

        pub(super) fn record_paths(index: &WorkspaceIndex) -> Vec<String> {
            index
                .entries()
                .iter()
                .map(|entry| entry.file().path().to_string_lossy().into_owned())
                .collect()
        }
    }
    use fixtures::*;

    use crate::test_support::TestProject;

    mod index {
        use std::{fs, path::Path, sync::Arc};

        use clap::Parser;
        use miette::Diagnostic;

        use super::*;
        use crate::{
            DirTreeError, PresetDialogProvider,
            cli::{Cli, CwdGuard},
            config::ConfigError,
            index::IndexError,
        };

        #[test]
        fn persists_covering_every_project_file() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::trusted(temp.path().join("project"));
            project.write_note("notes/todo.md", "- [ ] task");

            Index.run(&project.config()).expect("run index command");

            let loaded =
                project.indexer().load().expect("load persisted index");
            let paths = record_paths(&loaded);
            assert!(!paths.contains(&".traces/config.toml".to_owned()));
        }

        #[test]
        fn survives_a_later_process_invocation_via_load() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::trusted(temp.path().join("project"));
            project.write_note("first.md", "content");
            Index.run(&project.config()).expect("first index run");

            let reloaded =
                project.indexer().load().expect("reload persisted index");

            assert_eq!(reloaded.entries().len(), 1);
        }

        #[test]
        fn rebuilds_rather_than_appends() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::trusted(temp.path().join("project"));
            project.write_note("stale.md", "old");
            Index.run(&project.config()).expect("first index run");
            fs::remove_file(project.root().join("stale.md"))
                .expect("remove stale");
            project.write_note("fresh.md", "new");

            Index.run(&project.config()).expect("second index run");

            let loaded =
                project.indexer().load().expect("load persisted index");
            let paths = record_paths(&loaded);
            assert!(!paths.contains(&"stale.md".to_owned()));
            assert!(paths.contains(&"fresh.md".to_owned()));
        }

        #[test]
        fn fails_when_project_root_is_not_trusted() {
            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::untrusted(temp.path().join("project"));
            let _guard = CwdGuard::enter(project.root());

            let error = Cli::try_parse_from(["traces", "index"])
                .expect("parse index command")
                .run(project.service(), Arc::new(PresetDialogProvider::new()))
                .expect_err("untrusted root fails");

            assert!(matches!(error, CliError::ConfigLoad {
                source: ConfigError::Trust(_),
                ..
            }));
            assert_eq!(
                error.code().map(|code| code.to_string()),
                Some("traces::cli::config_build_untrusted".to_owned())
            );
        }

        #[cfg(unix)]
        #[test]
        fn wraps_scan_failure_as_cli_index_error() {
            use std::os::unix::fs::PermissionsExt as _;

            struct RestorePermissions<'a>(&'a Path);

            impl Drop for RestorePermissions<'_> {
                fn drop(&mut self) {
                    let _ = fs::set_permissions(
                        self.0,
                        fs::Permissions::from_mode(0o700),
                    );
                }
            }

            let temp = tempfile::tempdir().expect("create temp dir");
            let project = TestProject::trusted(temp.path().join("project"));
            let locked = project.root().join("locked");
            fs::create_dir(&locked).expect("create locked dir");
            fs::set_permissions(&locked, fs::Permissions::from_mode(0o000))
                .expect("revoke read permission");
            let _restore = RestorePermissions(&locked);

            let error = Index
                .run(&project.config())
                .expect_err("unreadable subdirectory fails");

            assert!(matches!(error, CliError::Index {
                source: IndexError::Scan(DirTreeError::NodeInaccessible { .. }),
                ..
            }));
        }
    }
}
