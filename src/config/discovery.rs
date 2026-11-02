//! Finds Config Files on disk without reading their TOML.
//!
//! [`ConfigLocator`] uses walk ceilings and the global Config File directory.
//! It finds the nearest local Config File, its optional global counterpart, or
//! local Config Files in a subtree. It does not check trust or write to the
//! Tracker Store.

use std::{
    io,
    path::{Path, PathBuf},
};

use super::{
    error::{ConfigDiscoveryError, ConfigDiscoveryResult},
    file::{Discovered, GlobalConfigFile, LocalConfigFile},
};
use crate::{DirTree, dirs, env_vars};

/// Names the subdirectory for the global Config File under the OS config home.
const GLOBAL_CONFIG_DIR: &str = "traces";

/// Holds the Config Files found for one project load.
#[derive(Debug)]
pub(super) struct DiscoveredConfigs {
    /// Holds the nearest local Config File. The builder applies it after
    /// [`Self::global`].
    pub(super) local: LocalConfigFile<Discovered>,
    /// Holds the optional global Config File. The builder applies it before
    /// [`Self::local`].
    pub(super) global: Option<GlobalConfigFile<Discovered>>,
}

/// Finds Config Files from the environment and the given directory.
#[derive(Clone, Debug)]
pub(super) struct ConfigLocator {
    /// Holds directories where an upward walk stops without a check.
    ceilings: Vec<PathBuf>,
    /// Holds the directory containing the global Config File.
    global_dir: PathBuf,
}

impl ConfigLocator {
    /// Builds a Config Locator from the process environment.
    #[must_use]
    pub(super) fn from_env() -> Self {
        Self {
            ceilings: env_vars::CEILING_DIRS.iter().cloned().collect(),
            global_dir: dirs::CONFIG_HOME.join(GLOBAL_CONFIG_DIR),
        }
    }

    /// Builds a Config Locator from explicit test inputs.
    #[cfg(test)]
    #[must_use]
    pub(super) const fn new(
        ceilings: Vec<PathBuf>,
        global_dir: PathBuf,
    ) -> Self {
        Self {
            ceilings,
            global_dir,
        }
    }

    /// Finds the nearest local and optional global Config Files for `cwd` and
    /// returns them.
    ///
    /// # Errors
    ///
    /// - [`NotFound`] if no local Config File exists between `cwd` and a walk
    ///   ceiling.
    /// - [`Inaccessible`] if the file system cannot inspect a candidate path.
    ///
    /// [`NotFound`]: ConfigDiscoveryError::NotFound
    /// [`Inaccessible`]: ConfigDiscoveryError::Inaccessible
    pub(super) fn project(
        &self,
        cwd: &Path,
    ) -> ConfigDiscoveryResult<DiscoveredConfigs> {
        let local = self.nearest_local(cwd)?;
        let global = GlobalConfigFile::in_dir(&self.global_dir);
        let global = is_config_file(global.path())?.then_some(global);
        Ok(DiscoveredConfigs {
            local,
            global,
        })
    }

    /// Finds and returns the nearest local Config File at or above `dir`.
    ///
    /// The walk stops before it checks a ceiling directory.
    /// It also stops when `dir` is itself a ceiling.
    ///
    /// # Errors
    ///
    /// - [`NotFound`] if no local Config File exists before a walk ceiling.
    /// - [`Inaccessible`] if the file system cannot inspect a candidate path.
    ///
    /// [`NotFound`]: ConfigDiscoveryError::NotFound
    /// [`Inaccessible`]: ConfigDiscoveryError::Inaccessible
    pub(super) fn nearest_local(
        &self,
        dir: &Path,
    ) -> ConfigDiscoveryResult<LocalConfigFile<Discovered>> {
        for ancestor in dir.ancestors() {
            if self.ceilings.iter().any(|ceiling| ceiling == ancestor) {
                break;
            }
            let candidate = LocalConfigFile::from_root(ancestor);
            if is_config_file(candidate.path())? {
                return Ok(candidate);
            }
        }
        Err(ConfigDiscoveryError::NotFound {
            cwd: dir.to_path_buf(),
        })
    }

    /// Finds local Config Files at or below `dir` and orders them by root.
    ///
    /// Uses [`Self::nearest_local`] if the subtree has no local Config File.
    ///
    /// # Errors
    ///
    /// - [`NotFound`] if no local Config File exists in the subtree or above it
    ///   before a walk ceiling.
    /// - [`Inaccessible`] if the file system cannot inspect a path.
    ///
    /// [`NotFound`]: ConfigDiscoveryError::NotFound
    /// [`Inaccessible`]: ConfigDiscoveryError::Inaccessible
    pub(super) fn local_subtree(
        &self,
        dir: &Path,
    ) -> ConfigDiscoveryResult<Vec<LocalConfigFile<Discovered>>> {
        let mut local = collect_descendant_configs(dir)?;
        if local.is_empty() {
            local.push(self.nearest_local(dir)?);
        }
        local.sort_by(|left, right| left.root().cmp(right.root()));
        Ok(local)
    }
}

/// Collects and returns local Config Files at or below `dir`.
///
/// It skips subtrees named by [`env_vars::is_ignored_dir`].
/// It does not prune the walk root `dir`.
/// [`DirTree::filter`] does not check the predicate for the root.
/// A root with an ignored name is still scanned.
/// It returns traversal failures instead of treating them as an empty subtree.
///
/// # Errors
///
/// - [`Inaccessible`] if the root is removed or the file system cannot inspect
///   a path.
///
/// [`Inaccessible`]: ConfigDiscoveryError::Inaccessible
fn collect_descendant_configs(
    dir: &Path,
) -> ConfigDiscoveryResult<Vec<LocalConfigFile<Discovered>>> {
    let mut configs = Vec::new();
    for node in DirTree::descendants(dir)
        .filter(|node| env_vars::is_ignored_dir(node.file_name()))
    {
        let node = node.map_err(|error| {
            let (path, source) = error.into_parts();
            ConfigDiscoveryError::Inaccessible {
                path,
                source,
            }
        })?;
        if !node.is_dir() {
            continue;
        }
        let candidate = LocalConfigFile::from_root(node.path());
        if is_config_file(candidate.path())? {
            configs.push(candidate);
        }
    }
    Ok(configs)
}

/// Returns whether `path` is a regular file.
///
/// Returns `false` when the path does not exist.
///
/// # Errors
///
/// - [`Inaccessible`] if the file system cannot inspect the path for another
///   reason.
///
/// [`Inaccessible`]: ConfigDiscoveryError::Inaccessible
fn is_config_file(path: &Path) -> ConfigDiscoveryResult<bool> {
    match path.metadata() {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(ConfigDiscoveryError::Inaccessible {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::config::file::LOCAL_CONFIG_FILE;

    struct Fixture {
        temp: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                temp: tempfile::tempdir().expect("create temp dir"),
            }
        }

        fn path(&self, rel: &str) -> PathBuf {
            self.temp.path().join(rel)
        }

        fn create_dir(&self, rel: &str) -> PathBuf {
            let p = self.path(rel);
            fs::create_dir_all(&p).expect("create dir");
            p
        }

        fn create_config(&self, rel_dir: &str) -> PathBuf {
            let p = self.path(rel_dir).join(LOCAL_CONFIG_FILE);
            fs::create_dir_all(p.parent().expect("config parent"))
                .expect("create config parent");
            fs::write(&p, "[templates]\n").expect("write config");
            p
        }

        fn create_file(&self, rel: &str) -> PathBuf {
            let p = self.path(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).expect("create parent");
            }
            fs::write(&p, "").expect("write file");
            p
        }

        /// Builds a Config Locator with isolated global storage and no
        /// ceilings.
        fn locator(&self) -> ConfigLocator {
            ConfigLocator::new(Vec::new(), self.path("global-home/traces"))
        }
    }

    fn roots(files: &[LocalConfigFile<Discovered>]) -> Vec<PathBuf> {
        files.iter().map(|file| file.root().to_path_buf()).collect()
    }

    mod project {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn finds_the_nearest_local_config_from_a_nested_directory() {
            let fixture = Fixture::new();
            let project = fixture.create_dir("project");
            let cwd = fixture.create_dir("project/notes/daily");
            fixture.create_config("project");

            let found = fixture.locator().project(&cwd).expect("discover");

            assert_eq!(found.local.root(), project.as_path());
            assert!(found.global.is_none());
        }

        #[test]
        fn nearest_local_wins_over_an_outer_one() {
            let fixture = Fixture::new();
            fixture.create_config("parent");
            let child = fixture.create_dir("parent/child");
            fixture.create_config("parent/child");
            let cwd = fixture.create_dir("parent/child/notes");

            let found = fixture.locator().project(&cwd).expect("discover");

            assert_eq!(found.local.root(), child.as_path());
        }

        #[test]
        fn includes_the_global_config_when_it_exists() {
            let fixture = Fixture::new();
            let cwd = fixture.create_dir("project");
            fixture.create_config("project");
            let global_dir = fixture.create_dir("global-home/traces");
            fs::write(global_dir.join("config.toml"), "")
                .expect("write global");

            let found = fixture.locator().project(&cwd).expect("discover");

            let global = found.global.expect("global config");
            assert_eq!(global.path(), global_dir.join("config.toml"));
        }

        #[test]
        fn fails_with_not_found_without_a_local_config() {
            let fixture = Fixture::new();
            let cwd = fixture.create_dir("project/notes");

            let result = fixture.locator().project(&cwd);

            assert!(matches!(
                result,
                Err(ConfigDiscoveryError::NotFound { .. })
            ));
        }
    }

    mod nearest_local {
        use super::*;

        #[test]
        fn fails_with_not_found_when_the_ceiling_is_reached() {
            let fixture = Fixture::new();
            let nested_dir = fixture.create_dir("ceiling/a/b/c");
            fixture.create_config("ceiling");
            let locator = ConfigLocator::new(
                vec![fixture.path("ceiling")],
                fixture.path("global-home/traces"),
            );

            let result = locator.nearest_local(&nested_dir);

            assert!(matches!(
                result,
                Err(ConfigDiscoveryError::NotFound { .. })
            ));
        }

        #[test]
        fn does_not_search_a_ceiling_directory_that_is_the_start_dir() {
            let fixture = Fixture::new();
            let dir = fixture.create_dir("project");
            fixture.create_config("project");
            let locator = ConfigLocator::new(
                vec![dir.clone()],
                fixture.path("global-home/traces"),
            );

            let result = locator.nearest_local(&dir);

            assert!(matches!(
                result,
                Err(ConfigDiscoveryError::NotFound { .. })
            ));
        }
    }

    mod local_subtree {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn discovers_the_directory_and_its_descendants() {
            let fixture = Fixture::new();
            let parent = fixture.create_dir("parent");
            let child = fixture.create_dir("parent/child");
            fixture.create_config("parent");
            fixture.create_config("parent/child");

            let found =
                fixture.locator().local_subtree(&parent).expect("subtree");

            assert_eq!(roots(&found), [parent, child]);
        }

        #[test]
        fn skips_configs_inside_ignored_directories() {
            let fixture = Fixture::new();
            let project = fixture.create_dir("p");
            fixture.create_config("p");
            fixture.create_config("p/node_modules/dep");

            let found =
                fixture.locator().local_subtree(&project).expect("subtree");

            assert_eq!(roots(&found), [project]);
        }

        #[test]
        fn scans_an_ignored_directory_named_explicitly() {
            let fixture = Fixture::new();
            let ignored = fixture.create_dir("p/target");
            fixture.create_config("p");
            fixture.create_config("p/target/inner");

            let found =
                fixture.locator().local_subtree(&ignored).expect("subtree");

            assert_eq!(roots(&found), [fixture.path("p/target/inner")]);
        }

        #[test]
        fn scans_an_ignored_directory_given_a_non_canonical_spelling() {
            let fixture = Fixture::new();
            fixture.create_dir("p/target");
            fixture.create_config("p/target/inner");
            let dotted = fixture.path("p").join(".").join("target");

            let found =
                fixture.locator().local_subtree(&dotted).expect("subtree");

            assert_eq!(
                found
                    .iter()
                    .map(|config| config.root().file_name())
                    .collect::<Vec<_>>(),
                [Some(std::ffi::OsStr::new("inner"))]
            );
        }

        #[test]
        fn finds_descendants_when_the_root_has_no_own_config() {
            let fixture = Fixture::new();
            let root = fixture.create_dir("root");
            let child_config = fixture.create_config("root/child");

            let found =
                fixture.locator().local_subtree(&root).expect("subtree");

            assert_eq!(
                found.iter().map(LocalConfigFile::path).collect::<Vec<_>>(),
                [child_config.as_path()]
            );
        }

        #[test]
        fn falls_back_to_the_enclosing_config_excluding_siblings() {
            let fixture = Fixture::new();
            fixture.create_config("ancestor");
            fixture.create_config("ancestor/sibling");
            let target = fixture.create_dir("ancestor/target");

            let found =
                fixture.locator().local_subtree(&target).expect("subtree");

            assert_eq!(roots(&found), [fixture.path("ancestor")]);
        }

        #[test]
        fn fails_with_not_found_when_nothing_encloses_or_descends() {
            let fixture = Fixture::new();
            let project = fixture.create_dir("project");

            let result = fixture.locator().local_subtree(&project);

            assert!(matches!(
                result,
                Err(ConfigDiscoveryError::NotFound { .. })
            ));
        }
    }

    mod is_config_file {
        use super::*;

        #[test]
        fn returns_false_for_a_missing_path() {
            let fixture = Fixture::new();

            let result = is_config_file(&fixture.path("missing.toml"));

            assert!(!result.expect("check missing path"));
        }

        #[test]
        fn fails_with_inaccessible_when_a_parent_is_not_a_directory() {
            let fixture = Fixture::new();
            let blocking_file = fixture.create_file("blocking");

            let result = is_config_file(&blocking_file.join("config.toml"));

            assert!(matches!(
                result,
                Err(ConfigDiscoveryError::Inaccessible { .. })
            ));
        }
    }
}
