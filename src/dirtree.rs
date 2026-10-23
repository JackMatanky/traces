//! Provides directory-tree traversal with classified, path-contextualized
//! errors.
//!
//! [`DirTree::children`] lists one directory level, and
//! [`DirTree::descendants`] walks an entire tree. Both yield [`DirNode`] values
//! and report failures as [`DirTreeError`]. [`DirTree::filter`] prunes
//! directories, while [`DirTree::sorted_by`] establishes deterministic
//! per-directory order.
//!
//! # Error classification
//!
//! Traversal reports failures according to where they occur:
//!
//! - [`DirTreeError::MissingRoot`] if the walk root does not exist. Callers
//!   decide whether to absorb it into an empty walk via
//!   [`DirTree::missing_as_empty`] or propagate the error.
//! - [`DirTreeError::RootInaccessible`] if the root exists but cannot be
//!   opened.
//! - [`DirTreeError::NodeInaccessible`] if a descendant cannot be opened or
//!   read.
//!
//! [`DirTree::children`] can report only [`DirTreeError::MissingRoot`] and
//! [`DirTreeError::RootInaccessible`]. [`DirTree::descendants`] can also report
//! [`DirTreeError::NodeInaccessible`].
//!
//! Callers select yielded entries by extension, stem, or hidden-file status.

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

/// Configured-but-not-started phase: owns the walk configuration until
/// [`walk`](DirTree::walk) consumes it.
///
/// This phase exposes configuration ([`filter`](DirTree::filter),
/// [`sorted_by`](DirTree::sorted_by),
/// [`missing_as_empty`](DirTree::missing_as_empty)) but not iteration through a
/// stored handle, so configuration and execution cannot interleave.
pub(crate) struct Ready {
    builder: WalkDir,
    root: PathBuf,
    prune: Option<PrunePredicate>,
    absorb_missing_root: bool,
}

/// Mid-iteration phase: owns the running walk.
///
/// This phase implements [`Iterator`] but exposes no configuration methods, so
/// reconfiguring a started walk is unrepresentable at compile time.
pub(crate) struct Walking {
    inner: InnerIter,
    root: PathBuf,
    absorb_missing_root: bool,
}

/// Iterates over a directory tree with classified, path-contextualized errors.
///
/// Created by [`DirTree::children`] (flat, one level) or
/// [`DirTree::descendants`] (recursive, whole tree). The typestate parameter
/// tracks the walk phase: the [`Ready`] phase accepts [`DirTree::filter`] and
/// [`DirTree::sorted_by`]; [`DirTree::walk`] (or [`IntoIterator`], used by
/// `for` loops) transitions to the [`Walking`] phase, which yields
/// [`Result<DirNode, DirTreeError>`]. Because configuration methods exist only
/// on the [`Ready`] phase, configuring a walk after it has started is a compile
/// error rather than a silent no-op.
pub(crate) struct DirTree<S> {
    state: S,
}

impl DirTree<Ready> {
    /// Lists the immediate entries of `dir`.
    ///
    /// Yields every direct child of `dir`, including files, directories, and
    /// symlinks. Callers select entries after iteration.
    ///
    /// A missing directory yields exactly one [`DirTreeError::MissingRoot`] and
    /// stops; a file root yields nothing. An unreadable subdirectory remains a
    /// plain entry. [`DirTree::descendants`] reports that failure as
    /// [`DirTreeError::NodeInaccessible`].
    ///
    /// Entry order follows the OS directory read and is unspecified. Use
    /// [`Self::sorted_by`] to establish deterministic order.
    #[must_use]
    pub(crate) fn children(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        Self {
            state: Ready {
                builder: WalkDir::new(dir).min_depth(1).max_depth(1),
                root: dir.to_path_buf(),
                prune: None,
                absorb_missing_root: false,
            },
        }
    }

    /// Walks the directory tree rooted at `dir`.
    ///
    /// Yields the root node first, then every descendant, including files,
    /// directories, and symlinks. Callers select entries after iteration.
    /// Symlinks are never followed. A missing root yields exactly one
    /// [`DirTreeError::MissingRoot`] and stops. An unreadable descendant yields
    /// [`DirTreeError::NodeInaccessible`] with its failing path.
    ///
    /// Entry order follows the OS directory read and is unspecified. Use
    /// [`Self::sorted_by`] to establish deterministic per-directory order.
    #[must_use]
    pub(crate) fn descendants(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        Self {
            state: Ready {
                builder: WalkDir::new(dir),
                root: dir.to_path_buf(),
                prune: None,
                absorb_missing_root: false,
            },
        }
    }

    /// Absorbs a missing walk root into an empty walk.
    ///
    /// [`DirTreeError::MissingRoot`] is the only variant this absorbs: the walk
    /// yields nothing instead of one error item. Other failures
    /// ([`DirTreeError::RootInaccessible`], [`DirTreeError::NodeInaccessible`])
    /// still surface, so callers keep their strict handling of unreadable
    /// roots. This is the shared policy for callers that must tolerate a
    /// missing directory and see it as "no entries".
    ///
    /// Available only in the [`Ready`] phase; the [`Walking`] phase has no
    /// configuration methods.
    #[must_use]
    pub(crate) fn missing_as_empty(mut self) -> Self {
        self.state.absorb_missing_root = true;
        self
    }

    /// Prunes every subtree whose directory satisfies `predicate`.
    ///
    /// `predicate` receives a [`DirNodeRef`] for each directory *beneath* the
    /// walk root. Returning `true` removes that directory and everything
    /// beneath it from the walk. Non-matching entries, including files whose
    /// names satisfy the predicate, are yielded unchanged. The walk root itself
    /// is exempt: the predicate never sees it, so a predicate that matches the
    /// root's own name leaves the walk intact.
    ///
    /// Works with [`Self::children`] and [`Self::descendants`]. For flat walks,
    /// matching directories are omitted from the listing.
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
                    entry.depth() == 0
                        || !entry.file_type().is_dir()
                        || !predicate(&DirNodeRef(entry))
                })),
                ..self.state
            },
        }
    }

    /// Orders each listed directory's entries with `compare`.
    ///
    /// Ordering is per-directory; no global ordering spans hierarchy levels.
    /// The sorting closure receives borrowed node views and must be [`Send`]
    /// and [`Sync`].
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

    /// Starts the walk, transitioning from the [`Ready`] to the [`Walking`]
    /// phase.
    ///
    /// The [`Walking`] phase implements [`Iterator`] and has no configuration
    /// methods, so any later reconfiguration attempt fails to compile. `for`
    /// loops transition implicitly via [`IntoIterator`].
    pub(crate) fn walk(self) -> DirTree<Walking> {
        let Ready {
            builder,
            root,
            prune,
            absorb_missing_root,
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
                absorb_missing_root,
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
        while let Some(item) = self
            .state
            .inner
            .next()
            .map(|result| DirNode::try_new(&self.state.root, result))
        {
            let absorbed = self.state.absorb_missing_root
                && matches!(&item, Err(DirTreeError::MissingRoot { .. }));
            if !absorbed {
                return Some(item);
            }
        }
        None
    }
}

/// Represents a file, directory, or symlink yielded by [`DirTree`].
///
/// Wraps [`DirEntry`] so callers do not depend on its concrete type. Accessors
/// delegate to the wrapped entry and report failures through [`DirTreeError`].
#[repr(transparent)]
#[derive(Clone, Debug)]
pub(crate) struct DirNode(DirEntry);

impl DirNode {
    /// Adapts a raw walk result relative to `root` into a directory node.
    ///
    /// # Errors
    ///
    /// - [`DirTreeError::MissingRoot`] if the walk result reports a missing
    ///   root.
    /// - [`DirTreeError::RootInaccessible`] if the walk result reports an
    ///   inaccessible root.
    /// - [`DirTreeError::NodeInaccessible`] if the walk result reports an
    ///   inaccessible descendant.
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

    /// Reports whether the node is a regular file, without following symlinks.
    ///
    /// A symlinked file reports `false`, never its target's type.
    #[must_use]
    pub(crate) fn is_file(&self) -> bool {
        self.0.file_type().is_file()
    }

    /// Reports whether the node is a directory, without following symlinks.
    ///
    /// A symlinked directory reports `false`, never its target's type.
    #[must_use]
    pub(crate) fn is_dir(&self) -> bool {
        self.0.file_type().is_dir()
    }

    /// Returns the node's filesystem metadata without following symlinks.
    ///
    /// # Errors
    ///
    /// - [`DirTreeError::NodeInaccessible`] naming the node if the metadata
    ///   cannot be read, keeping the walk's error classification intact for
    ///   callers.
    pub(crate) fn metadata(&self) -> Result<fs::Metadata, DirTreeError> {
        self.0.metadata().map_err(|source| DirTreeError::NodeInaccessible {
            path: self.0.path().to_path_buf(),
            source: io::Error::from(source),
        })
    }
}

/// Provides a borrowed directory-node view without cloning for comparisons.
#[derive(Copy, Clone, Debug)]
pub(crate) struct DirNodeRef<'a>(&'a DirEntry);

impl<'a> DirNodeRef<'a> {
    /// Returns the node's final path component.
    #[inline]
    #[must_use]
    pub(crate) fn file_name(self) -> &'a OsStr {
        self.0.file_name()
    }
}

/// Erases a caller-supplied pruning predicate for [`walkdir::FilterEntry`].
type PrunePredicate = Box<dyn FnMut(&DirEntry) -> bool>;

/// Stores either a plain or pruned walk iterator.
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

/// Classifies failures raised while traversing a directory tree.
///
/// Callers can distinguish a missing root from access failures, then decompose
/// any variant with [`Self::into_parts`]. Callers that must tolerate a missing
/// directory can absorb [`MissingRoot`](Self::MissingRoot) into an empty walk
/// via [`DirTree::missing_as_empty`] instead of matching this enum.
#[derive(Debug, Error)]
pub(crate) enum DirTreeError {
    /// Reports a walk root that does not exist.
    #[error("walk root {path} does not exist")]
    MissingRoot {
        /// Stores the root path supplied to the traversal constructor.
        path: PathBuf,
        /// Stores the source [`io::Error`].
        #[source]
        source: io::Error,
    },
    /// Reports a walk root that cannot be opened.
    #[error("failed to access walk root {path}")]
    RootInaccessible {
        /// Stores the root path supplied to the traversal constructor.
        path: PathBuf,
        /// Stores the source [`io::Error`].
        #[source]
        source: io::Error,
    },
    /// Reports a descendant that cannot be opened or read.
    #[error("failed to access node {path}")]
    NodeInaccessible {
        /// Stores the unavailable descendant's path. Falls back to the walk
        /// root when a directory-read failure has no path.
        path: PathBuf,
        /// Stores the source [`io::Error`].
        #[source]
        source: io::Error,
    },
}

impl DirTreeError {
    /// Decomposes the error into its resolved path and source.
    ///
    /// Returns the resolved [`PathBuf`] and source [`io::Error`]. Every variant
    /// carries this shape, allowing callers to use tuple destructuring: `let
    /// (path, source) = error.into_parts();`.
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

/// Classifies a [`walkdir::Error`] relative to the fallback walk root.
///
/// Depth 0 denotes the walk root itself, so a depth-0 failure is classified
/// against the root rather than as a node beneath it. Errors that carry no path
/// (mid-readdir stream glitches) fall back to the walk root.
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Restores directory permissions when dropped so temporary-directory
    /// cleanup can remove a `0o000` directory.
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

    mod children {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_direct_children_without_descendants() {
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
        fn returns_no_entries_when_directory_is_empty() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");

            // Act
            let entries: Vec<_> =
                DirTree::children(temp.path()).walk().collect();

            // Assert
            assert!(entries.is_empty(), "an empty directory has no children");
        }

        #[test]
        fn returns_single_missing_root_error_when_directory_is_missing() {
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
        fn returns_direct_children_in_comparator_order() {
            // Arrange: Create entries in reverse lexicographic order.
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

            // Assert: The iterator yields sorted entries directly.
            assert_eq!(names, vec!["a.md", "z.md"]);
        }

        #[test]
        fn returns_no_entries_when_root_is_file() {
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
        fn returns_entries_without_matching_directories() {
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

            // Assert: Pruning omits the matching directory and retains the
            // file.
            assert_eq!(names, vec!["note.md"]);
        }
    }

    mod descendants {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_complete_tree_when_walking_directory() {
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

            // Assert: The root contributes an empty relative path.
            assert_eq!(relatives, vec!["", "a.md", "b", "b/one.md"]);
        }

        #[test]
        fn returns_missing_root_error_when_root_is_missing() {
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
        fn missing_as_empty_absorbs_a_missing_root_into_an_empty_walk() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let missing = temp.path().join("gone");
            write(temp.path(), "note.md");

            // Act
            let flat: Vec<_> =
                DirTree::children(&missing).missing_as_empty().walk().collect();
            let deep: Vec<_> = DirTree::descendants(&missing)
                .missing_as_empty()
                .walk()
                .collect();
            let present: Vec<_> = DirTree::children(temp.path())
                .missing_as_empty()
                .walk()
                .collect();

            // Assert
            assert!(flat.is_empty(), "flat walk must yield nothing");
            assert!(deep.is_empty(), "recursive walk must yield nothing");
            assert_eq!(present.len(), 1, "existing roots are unaffected");
        }

        #[cfg(unix)]
        #[test]
        fn missing_as_empty_still_surfaces_root_inaccessible() {
            use std::os::unix::fs::PermissionsExt;

            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "inside.md");
            fs::set_permissions(root, fs::Permissions::from_mode(0o000))
                .expect("revoke root permissions");
            let _restore = RestorePermissions(root);

            // Act
            let collected: Vec<_> =
                DirTree::children(root).missing_as_empty().walk().collect();

            // Assert: only MissingRoot is absorbed; other failures propagate.
            assert_eq!(collected.len(), 1);
            assert!(
                matches!(
                    collected.into_iter().next().expect("one item"),
                    Err(DirTreeError::RootInaccessible { .. })
                ),
                "other failures must not be absorbed"
            );
        }

        #[test]
        fn returns_entries_outside_matching_subtree() {
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

            // Assert: Pruning excludes the subtree and retains the matching
            // file.
            assert_eq!(names.len(), 2);
            assert!(names.contains(&"note.md".to_owned()));
            assert!(!names.contains(&"HEAD".to_owned()));
        }

        #[test]
        fn returns_matching_files_when_filter_matches_file_name() {
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

            // Assert: The predicate prunes directories only; matching files
            // pass through with the walk root.
            assert_eq!(names.len(), 2);
            assert!(names.contains(&".git".to_owned()));
        }

        #[test]
        fn returns_entries_in_per_directory_order() {
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

            // Assert: The walk is depth-first, and siblings are ordered within
            // each directory.
            assert_eq!(relatives, vec![
                "",
                "a.md",
                "b",
                "b/one.md",
                "b/zero.md"
            ]);
        }

        #[test]
        fn returns_filtered_entries_in_per_directory_order() {
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

            // Assert: Pruning excludes the subtree, and remaining siblings stay
            // ordered.
            assert_eq!(relatives, vec!["", "b", "b/note.md"]);
        }

        #[test]
        fn filter_never_consults_the_predicate_for_the_walk_root() {
            // Arrange: the root deliberately carries an "ignored" name, and the
            // predicate counts every directory it is asked about.
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().join("target");
            fs::create_dir(&root).expect("create root");
            write(&root, "note.md");
            let seen =
                std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let seen_in_predicate = std::sync::Arc::clone(&seen);
            let root_name = root.file_name().expect("named").to_os_string();

            // Act
            let relatives: Vec<String> = DirTree::descendants(&root)
                .filter(move |node| {
                    seen_in_predicate
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    node.file_name() == root_name.as_os_str()
                })
                .walk()
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| {
                    node.path()
                        .strip_prefix(&root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();

            // Assert: the root survives its own name-matching predicate and the
            // predicate never runs at depth 0, so callers no longer guard
            // against emptying the walk.
            assert_eq!(relatives, vec!["", "note.md"]);
            assert_eq!(
                seen.load(std::sync::atomic::Ordering::Relaxed),
                0,
                "predicate must not be invoked for the walk root"
            );
        }

        #[test]
        fn returns_entries_outside_matching_nested_subtree() {
            // Arrange: Match only the nested `.traces` directory, not the
            // temporary root.
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

            // Assert: Pruning removes `.traces` and its contents while
            // retaining the root and other entries.
            assert_eq!(relatives, vec!["", "proj", "proj/a.md"]);
        }

        #[cfg(unix)]
        #[test]
        fn returns_directory_symlink_without_descending_into_target() {
            use std::os::unix::fs::symlink;

            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            let target = root.join("target");
            write(&target, "nested.md");
            symlink(&target, root.join("link")).expect("create symlink");

            // Act
            let mut relatives: Vec<String> = DirTree::descendants(root)
                .walk()
                .map(|entry| entry.expect("entry is readable"))
                .map(|node| {
                    node.path()
                        .strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            relatives.sort();

            // Assert
            assert_eq!(relatives, vec![
                "",
                "link",
                "target",
                "target/nested.md"
            ]);
        }
    }

    mod classify {
        use pretty_assertions::assert_eq;

        use super::*;

        #[cfg(unix)]
        #[test]
        fn returns_root_inaccessible_error_when_root_is_unreadable() {
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

            // Assert: Permission denial maps to an inaccessible root rather
            // than a missing root.
            assert_eq!(collected.len(), 1);
            let error = collected
                .into_iter()
                .next()
                .expect("one item")
                .expect_err("is an error");
            assert!(
                matches!(&error, DirTreeError::RootInaccessible { .. }),
                "expected RootInaccessible, got {error:?}"
            );
            let (path, source) = error.into_parts();
            assert_eq!(path, root);
            assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
        }

        #[cfg(unix)]
        #[test]
        fn returns_unreadable_child_without_error_when_listing_directory() {
            use std::os::unix::fs::PermissionsExt;

            // Arrange: a subdirectory that cannot be opened. The contract under
            // test is the documented one - a flat listing yields it as a plain
            // entry with no error, while recursive walks surface a
            // NodeInaccessible instead.
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
        fn returns_node_inaccessible_error_when_descendant_is_unreadable() {
            use std::os::unix::fs::PermissionsExt;

            // Arrange: Recursive traversal opens the child and reports its
            // access failure.
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
    }

    mod accessors {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_metadata_for_regular_file() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "daily.md");

            // Act
            let node = DirTree::children(root)
                .walk()
                .next()
                .expect("one entry")
                .expect("entry is readable");
            let metadata = node.metadata().expect("read file metadata");

            // Assert
            assert!(metadata.is_file(), "fixture is a regular file");
            assert_eq!(metadata.len(), 7);
        }

        #[test]
        fn exposes_path_file_name_and_is_file() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            let file = write(root, "daily.md");

            // Act
            let node = DirTree::children(root)
                .walk()
                .next()
                .expect("one entry")
                .expect("entry is readable");

            // Assert
            assert_eq!(node.path(), file);
            assert_eq!(node.file_name(), std::ffi::OsStr::new("daily.md"));
            assert!(node.is_file());
        }

        #[cfg(unix)]
        #[test]
        fn returns_symlink_type_without_following_target() {
            use std::os::unix::fs::symlink;

            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            let target = write(root, "target.md");
            let link = root.join("link.md");
            symlink(&target, &link).expect("create symlink");

            // Act
            let node = DirTree::children(root)
                .walk()
                .find(|entry| matches!(entry, Ok(node) if node.path() == link.as_path()))
                .expect("symlink entry is yielded")
                .expect("symlink entry is readable");
            let metadata = node.metadata().expect("read symlink metadata");

            // Assert
            assert_eq!(node.path(), link);
            assert!(
                !node.is_file() && !node.is_dir(),
                "entry remains a symlink, neither a file nor a directory"
            );
            assert!(
                metadata.file_type().is_symlink(),
                "metadata remains for the symlink rather than its target"
            );
        }
    }

    /// Writes placeholder content at `rel` under `dir`, creating parents and
    /// returning the resulting path.
    fn write(dir: &Path, rel: &str) -> PathBuf {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dirs");
        }
        fs::write(&path, "content").expect("write fixture file");
        path
    }
}
