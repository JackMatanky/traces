//! Provides directory-tree traversal with flat listings and recursive walks
//! with classified, path-contextualized errors.
//!
//! The core entry points are [`DirTree::children`] (flat, one level) and
//! [`DirTree::descendants`] (recursive, whole tree). Both yield [`DirNode`]
//! values and report failures as [`DirTreeError`]. Configure iteration with
//! [`DirTree::filter`] (prune subtrees) and [`DirTree::sorted_by`]
//! (deterministic per-directory order).
//!
//! # Error classification
//!
//! Errors are classified into three variants based on where they occur:
//!
//! - [`DirTreeError::MissingRoot`]: the walk root does not exist. Callers
//!   decide their policy: degrade to empty or propagate.
//! - [`DirTreeError::RootInaccessible`]: the root exists but could not be
//!   opened.
//! - [`DirTreeError::NodeInaccessible`]: a node beneath the root failed
//!   (unreadable directory, metadata failure, mid-readdir glitch).
//!
//! Only [`DirTreeError::MissingRoot`] and [`DirTreeError::RootInaccessible`]
//! can appear from [`DirTree::children`]; [`DirTreeError::NodeInaccessible`]
//! only surfaces in recursive walks.
//!
//! Entry filtering (extensions, stems, hidden files) stays with callers, who
//! see every [`DirNode`] and decide what matches.

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

/// Classifies one raw walkdir failure against the walk's root.
fn classify(fallback: &Path, source: walkdir::Error) -> DirTreeError {
    let depth = source.depth();
    let path = source.path().unwrap_or(fallback).to_path_buf();
    let source = io::Error::from(source);
    match depth {
        0 if source.kind() == io::ErrorKind::NotFound => {
            DirTreeError::MissingRoot {
                path,
                source,
            }
        }
        0 => DirTreeError::RootInaccessible {
            path,
            source,
        },
        _ => DirTreeError::NodeInaccessible {
            path,
            source,
        },
    }
}

/// Represents a node of a directory tree: a file, directory, or symlink yielded
/// by [`DirTree::children`] or [`DirTree::descendants`].
///
/// Wraps walkdir's entry so callers never touch walkdir types. Accessors
/// delegate to the underlying [`walkdir::DirEntry`] and report failures through
/// the same [`DirTreeError`] as iteration.
#[repr(transparent)]
#[derive(Clone, Debug)]
pub(crate) struct DirNode(DirEntry);

/// Represents a borrowed view of a directory node for zero-clone comparisons.
#[derive(Copy, Clone, Debug)]
pub(crate) struct DirNodeRef<'a>(&'a DirEntry);

impl<'a> DirNodeRef<'a> {
    /// Returns the node's final path component.
    #[inline]
    #[must_use]
    pub(crate) fn file_name(self) -> &'a OsStr {
        self.0.file_name()
    }

    /// Returns the node's full path.
    #[inline]
    #[must_use]
    pub(crate) fn path(self) -> &'a Path {
        self.0.path()
    }
}

impl DirNode {
    /// Adapts one raw walkdir item into this module's interface.
    ///
    /// Entries become nodes; failures are classified against the walk's root.
    fn try_new(
        root: &Path,
        result: walkdir::Result<DirEntry>,
    ) -> Result<Self, DirTreeError> {
        match result {
            Ok(entry) => Ok(Self(entry)),
            Err(source) => Err(classify(root, source)),
        }
    }

    /// Returns the node's full path, including the walk root prefix.
    #[must_use]
    pub(crate) fn path(&self) -> &Path {
        self.0.path()
    }

    /// Returns the node's final path component.
    #[must_use]
    pub(crate) fn file_name(&self) -> &OsStr {
        self.0.file_name()
    }

    /// Returns the node's type without following symlinks.
    ///
    /// A symlinked file reports [`FileType::is_symlink`], never its target's
    /// type.
    ///
    /// [`FileType::is_symlink`]: std::fs::FileType::is_symlink
    #[must_use]
    pub(crate) fn file_type(&self) -> fs::FileType {
        self.0.file_type()
    }

    /// Returns the node's filesystem metadata without following symlinks.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if metadata cannot be read.
    pub(crate) fn metadata(&self) -> io::Result<fs::Metadata> {
        self.0.metadata().map_err(io::Error::from)
    }
}

/// Type-erased pruner: the caller's node predicate wrapped so
/// [`walkdir::FilterEntry`] can apply it to raw entries.
type PrunePredicate = Box<dyn FnMut(&DirEntry) -> bool>;

enum InnerIter {
    Plain(walkdir::IntoIter),
    Filtered(walkdir::FilterEntry<walkdir::IntoIter, PrunePredicate>),
}

impl Iterator for InnerIter {
    type Item = walkdir::Result<DirEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Plain(iter) => iter.next(),
            Self::Filtered(iter) => iter.next(),
        }
    }
}
/// Configured-but-not-started phase: owns the walk configuration until
/// [`walk`](DirTree::walk) consumes it.
///
/// This phase exposes configuration ([`filter`](DirTree::filter),
/// [`sorted_by`](DirTree::sorted_by)) but not iteration through a stored
/// handle, so configuration and execution cannot interleave.
pub(crate) struct Ready {
    builder: WalkDir,
    root: PathBuf,
    prune: Option<PrunePredicate>,
}

/// Mid-iteration phase: owns the running walk.
///
/// This phase implements [`Iterator`] but exposes no configuration methods,
/// so reconfiguring a started walk is unrepresentable at compile time.
pub(crate) struct Walking {
    inner: InnerIter,
    root: PathBuf,
}

/// Iterates over a directory tree with classified, path-contextualized errors.
///
/// Created by [`DirTree::children`] (flat, one level) or
/// [`DirTree::descendants`] (recursive, whole tree). The typestate parameter
/// tracks the walk phase: the [`Ready`] phase accepts [`DirTree::filter`] and
/// [`DirTree::sorted_by`]; [`DirTree::walk`] (or [`IntoIterator`], used by
/// `for` loops) transitions to the [`Walking`] phase, which yields
/// [`Result<DirNode, DirTreeError>`]. Because configuration methods exist
/// only on the [`Ready`] phase, configuring a walk after it has started is a
/// compile error rather than a silent no-op.
pub(crate) struct DirTree<S> {
    state: S,
}

impl DirTree<Ready> {
    /// Lists a directory's immediate entries (non-recursive).
    ///
    /// Yields every direct child of `dir` (files, directories, symlinks); entry
    /// filtering stays with the caller.
    ///
    /// A missing directory yields exactly one [`DirTreeError::MissingRoot`] and
    /// stops; a file root yields nothing at all. An unreadable subdirectory is
    /// still yielded as a plain entry without an error. Use
    /// [`DirTree::descendants`] to surface such failures as
    /// [`DirTreeError::NodeInaccessible`].
    ///
    /// Entry order follows the OS directory read and is unspecified. Pass
    /// [`Self::sorted_by`] to impose deterministic order.
    #[must_use]
    pub(crate) fn children(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        Self {
            state: Ready {
                builder: WalkDir::new(dir).min_depth(1).max_depth(1),
                root: dir.to_path_buf(),
                prune: None,
            },
        }
    }

    /// Walks a directory tree recursively, starting at the root itself.
    ///
    /// Yields the root node first, then every descendant (files, directories,
    /// symlinks); entry filtering stays with the caller. Symlinks are never
    /// followed. A missing root yields exactly one
    /// [`DirTreeError::MissingRoot`] and stops. An unreadable subdirectory
    /// surfaces [`DirTreeError::NodeInaccessible`] with the failing path.
    ///
    /// Entry order follows the OS directory read and is unspecified. Pass
    /// [`Self::sorted_by`] to impose deterministic per-directory order.
    #[must_use]
    pub(crate) fn descendants(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        Self {
            state: Ready {
                builder: WalkDir::new(dir),
                root: dir.to_path_buf(),
                prune: None,
            },
        }
    }

    /// Removes every subtree whose directory satisfies `predicate`.
    ///
    /// `predicate` receives a [`DirNode`] for each directory. Returning `true`
    /// removes that directory *and* everything beneath it from the walk.
    /// Non-matching entries (including files whose name satisfies the
    /// predicate) are yielded unchanged. A predicate matching the walk root
    /// itself empties the whole walk.
    ///
    /// Works on both [`Self::children`] and [`Self::descendants`]: for flat
    /// walks, filtered directories are simply omitted from the listing.
    ///
    /// Available only in the [`Ready`] phase; the [`Walking`] phase has no
    /// configuration methods.
    #[must_use]
    pub(crate) fn filter<F>(self, mut predicate: F) -> Self
    where
        F: FnMut(&DirNodeRef<'_>) -> bool + 'static,
    {
        Self {
            state: Ready {
                prune: Some(Box::new(move |entry: &DirEntry| {
                    !entry.file_type().is_dir()
                        || !predicate(&DirNodeRef(entry))
                })),
                ..self.state
            },
        }
    }

    /// Orders the entries of every listed directory with `compare`.
    ///
    /// Ordering is per-directory; there is no global ordering across hierarchy
    /// levels. Comparators receive borrowed node views, so no allocation
    /// occurs per comparison.
    ///
    /// Available only in the [`Ready`] phase; the [`Walking`] phase has no
    /// configuration methods.
    pub(crate) fn sorted_by<F>(self, mut compare: F) -> Self
    where
        F: FnMut(&DirNodeRef<'_>, &DirNodeRef<'_>) -> std::cmp::Ordering
            + Send
            + Sync
            + 'static,
    {
        let mut state = self.state;
        state.builder = state
            .builder
            .sort_by(move |a, b| compare(&DirNodeRef(a), &DirNodeRef(b)));
        Self {
            state,
        }
    }

    /// Starts the walk, transitioning from the [`Ready`] to the
    /// [`Walking`] phase.
    ///
    /// The [`Walking`] phase implements [`Iterator`] and has no
    /// configuration methods, so any later reconfiguration attempt fails to
    /// compile. `for` loops transition implicitly via [`IntoIterator`].
    pub(crate) fn walk(self) -> DirTree<Walking> {
        let Ready {
            builder,
            root,
            prune,
        } = self.state;
        let inner = match prune {
            Some(predicate) => InnerIter::Filtered(
                WalkDir::into_iter(builder).filter_entry(predicate),
            ),
            None => InnerIter::Plain(WalkDir::into_iter(builder)),
        };
        DirTree {
            state: Walking {
                inner,
                root,
            },
        }
    }
}

impl IntoIterator for DirTree<Ready> {
    type IntoIter = DirTree<Walking>;
    type Item = Result<DirNode, DirTreeError>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.walk()
    }
}

impl Iterator for DirTree<Walking> {
    type Item = Result<DirNode, DirTreeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.state
            .inner
            .next()
            .map(|result| DirNode::try_new(&self.state.root, result))
    }
}

/// Represents a failure raised while traversing a directory tree.
///
/// Callers match to state their missing-root policy and convert remaining
/// errors via [`into_parts`](Self::into_parts).
#[derive(Debug, Error)]
pub(crate) enum DirTreeError {
    /// The walk root does not exist.
    #[error("walk root {path} does not exist")]
    MissingRoot {
        /// The root path passed to the constructor.
        path: PathBuf,
        /// Source I/O error.
        #[source]
        source: io::Error,
    },
    /// The root exists but could not be opened.
    #[error("failed to access walk root {path}")]
    RootInaccessible {
        /// The root path passed to the constructor.
        path: PathBuf,
        /// Source I/O error.
        #[source]
        source: io::Error,
    },
    /// A node beneath the root failed (unreadable directory, metadata failure,
    /// mid-readdir glitch).
    #[error("failed to access node {path}")]
    NodeInaccessible {
        /// The failing node's path, falling back to the walk root when the
        /// path is unavailable (mid-readdir stream errors).
        path: PathBuf,
        /// Source I/O error.
        #[source]
        source: io::Error,
    },
}

impl DirTreeError {
    /// Splits the error into its resolved path and I/O source.
    ///
    /// Returns the resolved [`PathBuf`] and underlying [`std::io::Error`].
    /// Every variant carries the same `{path, source}` shape, allowing callers
    /// to decompose any error variant into its constituent path and underlying
    /// [`std::io::Error`] via tuple destructuring:
    /// `let (path, source) = error.into_parts();`.
    pub(crate) fn into_parts(self) -> (PathBuf, io::Error) {
        match self {
            Self::MissingRoot {
                path,
                source,
            }
            | Self::RootInaccessible {
                path,
                source,
            }
            | Self::NodeInaccessible {
                path,
                source,
            } => (path, source),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Writes `rel` under `dir` with placeholder content, creating parent
    /// directories, and returns the absolute path.
    fn write(dir: &Path, rel: &str) -> PathBuf {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dirs");
        }
        fs::write(&path, "content").expect("write fixture file");
        path
    }
    mod children {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn yields_only_immediate_entries() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "a.md");
            write(root, "sub/nested.md");

            // Act
            let mut names: Vec<String> = DirTree::children(root)
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| node.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();

            // Assert
            assert_eq!(names, vec!["a.md", "sub"]);
        }

        #[test]
        fn missing_directory_yields_one_missing_root_error_then_stops() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let missing = temp.path().join("does-not-exist");

            // Act
            let collected: Vec<_> =
                DirTree::children(&missing).walk().collect();

            // Assert
            assert_eq!(
                collected.len(),
                1,
                "missing root yields exactly one item"
            );
            let error = collected
                .into_iter()
                .next()
                .expect("one item")
                .expect_err("is an error");
            assert!(matches!(error, DirTreeError::MissingRoot { .. }));
            let (path, source) = error.into_parts();
            assert_eq!(path, missing);
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }

        #[test]
        fn sorted_by_orders_immediate_entries() {
            // Arrange: created deliberately out of lexicographic order.
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "z.md");
            write(root, "a.md");

            // Act
            let names: Vec<String> = DirTree::children(root)
                .sorted_by(|a, b| a.file_name().cmp(b.file_name()))
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| node.file_name().to_string_lossy().into_owned())
                .collect();

            // Assert: yielded already ordered; nothing sorted after the walk.
            assert_eq!(names, vec!["a.md", "z.md"]);
        }

        #[test]
        fn a_file_root_yields_no_entries_and_no_errors() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let file = write(temp.path(), "plain.md");

            // Act / Assert
            assert!(
                DirTree::children(&file).walk().next().is_none(),
                "a file root lists nothing"
            );
        }

        #[test]
        fn filter_removes_matching_directories_from_listing() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, ".git/HEAD");
            write(root, "note.md");

            // Act
            let mut names: Vec<String> = DirTree::children(root)
                .filter(|node| node.file_name() == ".git")
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| node.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();

            // Assert: .git directory removed, note.md and remaining entries
            // kept.
            assert_eq!(names, vec!["note.md"]);
        }
    }

    mod descendants {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn walks_the_whole_tree_including_the_root_node() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "a.md");
            write(root, "b/one.md");

            // Act
            let mut relatives: Vec<String> = DirTree::descendants(root)
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| {
                    node.path()
                        .strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            relatives.sort();

            // Assert: the root itself is yielded (empty relative path),
            // matching what index scanning and subtree discovery rely on.
            assert_eq!(relatives, vec!["", "a.md", "b", "b/one.md"]);
        }

        #[test]
        fn missing_root_yields_a_missing_root_error() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let missing = temp.path().join("gone");

            // Act
            let collected: Vec<_> =
                DirTree::descendants(&missing).walk().collect();

            // Assert
            assert_eq!(collected.len(), 1);
            assert!(matches!(
                collected.into_iter().next().expect("one item"),
                Err(DirTreeError::MissingRoot { .. })
            ));
        }

        #[test]
        fn filter_prunes_matching_subtrees_but_keeps_other_entries() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, ".git/HEAD");
            write(root, "note.md");

            // Act
            let mut names: Vec<String> = DirTree::descendants(root)
                .filter(|node| node.file_name() == ".git")
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| node.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();

            // Assert: pruned subtree absent entirely, surviving entry kept.
            assert_eq!(names.len(), 2);
            assert!(names.contains(&"note.md".to_owned()));
            assert!(!names.contains(&"HEAD".to_owned()));
        }

        #[test]
        fn filter_keeps_files_whose_name_matches_the_predicate() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, ".git");

            // Act
            let mut names: Vec<String> = DirTree::descendants(root)
                .filter(|node| node.file_name() == ".git")
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| node.file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();

            // Assert: the predicate only prunes directories; a file named
            // `.git` passes through untouched (alongside the walk root).
            assert_eq!(names.len(), 2);
            assert!(names.contains(&".git".to_owned()));
        }

        #[test]
        fn sorted_by_orders_entries_within_each_directory() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "b/zero.md");
            write(root, "a.md");
            write(root, "b/one.md");

            // Act
            let relatives: Vec<String> = DirTree::descendants(root)
                .sorted_by(|a, b| a.file_name().cmp(b.file_name()))
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| {
                    node.path()
                        .strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();

            // Assert: depth-first with siblings name-ordered per directory;
            // the root itself still comes first.
            assert_eq!(relatives, vec![
                "",
                "a.md",
                "b",
                "b/one.md",
                "b/zero.md"
            ]);
        }

        #[test]
        fn filter_composes_with_sorted_by() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, ".git/HEAD");
            write(root, "b/note.md");

            // Act
            let relatives: Vec<String> = DirTree::descendants(root)
                .filter(|node| node.file_name() == ".git")
                .sorted_by(|a, b| a.file_name().cmp(b.file_name()))
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| {
                    node.path()
                        .strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();

            // Assert: pruned subtree absent AND surviving siblings ordered.
            assert_eq!(relatives, vec!["", "b", "b/note.md"]);
        }
    }

    /// RAII guard that restores permissions on drop so that a `0o000` directory
    /// does not block the tempdir's cleanup.
    #[cfg(unix)]
    struct RestorePermissions<'a>(&'a Path);

    #[cfg(unix)]
    impl Drop for RestorePermissions<'_> {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                fs::set_permissions(self.0, fs::Permissions::from_mode(0o700));
        }
    }

    mod classification {
        use pretty_assertions::assert_eq;

        use super::*;

        #[cfg(unix)]
        #[test]
        fn unreadable_root_reports_root_inaccessible_never_missing_root() {
            use std::os::unix::fs::PermissionsExt;

            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "inside.md");
            fs::set_permissions(root, fs::Permissions::from_mode(0o000))
                .expect("revoke root permissions");
            let _restore = RestorePermissions(root);

            // Act
            let collected: Vec<_> = DirTree::children(root).walk().collect();

            // Assert: stat on the root still succeeds (parent grants it),
            // so this is an access failure, not absence.
            assert_eq!(collected.len(), 1);
            let error = collected
                .into_iter()
                .next()
                .expect("one item")
                .expect_err("is an error");
            assert!(
                matches!(error, DirTreeError::RootInaccessible { .. }),
                "expected RootInaccessible, got {error:?}"
            );
        }

        #[cfg(unix)]
        #[test]
        fn children_yields_an_unreadable_subdirectory_without_error() {
            use std::os::unix::fs::PermissionsExt;

            // Arrange: walkdir opens child directories eagerly even under
            // max_depth(1), but a failed open is stored in the child's dir
            // list and popped before it can be polled, so flat listings
            // yield the locked directory as a plain entry with no error.
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            let kid = root.join("locked-kid");
            fs::create_dir(&kid).expect("create locked dir");
            fs::set_permissions(&kid, fs::Permissions::from_mode(0o000))
                .expect("revoke permissions");
            let _restore = RestorePermissions(&kid);

            // Act
            let collected: Vec<_> = DirTree::children(root).walk().collect();

            // Assert
            assert_eq!(collected.len(), 1);
            assert!(
                !collected.iter().any(Result::is_err),
                "flat listing surfaces no error: {collected:?}"
            );
            let node = collected
                .into_iter()
                .next()
                .expect("one item")
                .expect("entry is ok");
            assert_eq!(node.file_name(), std::ffi::OsStr::new("locked-kid"));
        }

        #[cfg(unix)]
        #[test]
        fn descendants_reports_an_unreadable_subdirectory_naming_it() {
            use std::os::unix::fs::PermissionsExt;

            // Arrange: unlike `children`, an unlimited-depth walk polls the
            // stored open-failure and surfaces it.
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            let kid = root.join("locked-kid");
            fs::create_dir(&kid).expect("create locked dir");
            fs::set_permissions(&kid, fs::Permissions::from_mode(0o000))
                .expect("revoke permissions");
            let _restore = RestorePermissions(&kid);

            // Act
            let mut errors =
                DirTree::descendants(root).walk().filter_map(Result::err);

            // Assert
            let error = errors.next();
            assert!(
                matches!(&error, Some(DirTreeError::NodeInaccessible { .. })),
                "expected NodeInaccessible, got {error:?}"
            );
            let (path, _) = error.expect("present").into_parts();
            assert_eq!(path, kid);
            assert!(errors.next().is_none(), "exactly one error");
        }

        #[test]
        fn filter_prunes_matching_subtree_during_descendants() {
            // Arrange: nested matching dir (not a direct child of root);
            // dot-prefixed tempdir name must NOT match the predicate.
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "proj/.traces/config.toml");
            write(root, "proj/a.md");

            // Act
            let relatives: Vec<String> = DirTree::descendants(root)
                .filter(|node| node.file_name() == ".traces")
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| {
                    node.path()
                        .strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();

            // Assert: the .traces/config.toml file is pruned with its dir;
            // root itself and surviving files remain.
            assert_eq!(relatives, vec!["", "proj", "proj/a.md"]);
        }
    }

    mod dirnode {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn exposes_path_file_name_and_file_type() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            let file = write(root, "daily.md");

            // Act
            let node = DirTree::children(root)
                .walk()
                .next()
                .expect("one entry")
                .expect("entry is ok");

            // Assert
            assert_eq!(node.path(), file);
            assert_eq!(node.file_name(), std::ffi::OsStr::new("daily.md"));
            assert!(node.file_type().is_file());
        }
    }

    mod display {
        use super::*;

        #[test]
        fn messages_are_lowercase_without_trailing_punctuation() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let missing = temp.path().join("gone");
            let error = DirTree::children(&missing)
                .walk()
                .next()
                .expect("one item")
                .expect_err("missing root");

            // Act
            let message = error.to_string();

            // Assert
            assert!(
                message.starts_with(char::is_lowercase),
                "message starts lowercase: {message}"
            );
            assert!(!message.ends_with('.') && !message.ends_with('!'));
        }
    }
}
