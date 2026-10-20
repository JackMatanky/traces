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

/// Iterates over a directory tree with classified, path-contextualized errors.
///
/// Created by [`DirTree::children`] (flat, one level) or
/// [`DirTree::descendants`] (recursive, whole tree). Configure with
/// [`DirTree::filter`] and [`DirTree::sorted_by`] before iterating. Yields
/// [`Result<DirNode, DirTreeError>`].
///
/// Builder-time configuration must be called before the first
/// [`Iterator::next`] call. Once iteration begins, further configuration is
/// silently ignored.
pub(crate) struct DirTree {
    builder: Option<WalkDir>,
    root: PathBuf,
    prune: Option<PrunePredicate>,
    inner: Option<InnerIter>,
}

impl DirTree {
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
            builder: Some(WalkDir::new(dir).min_depth(1).max_depth(1)),
            root: dir.to_path_buf(),
            prune: None,
            inner: None,
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
            builder: Some(WalkDir::new(dir)),
            root: dir.to_path_buf(),
            prune: None,
            inner: None,
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
    /// Has no effect once iteration has begun.
    #[must_use]
    pub(crate) fn filter<F>(mut self, mut predicate: F) -> Self
    where
        F: FnMut(&DirNodeRef<'_>) -> bool + 'static,
    {
        self.prune = Some(Box::new(move |entry: &DirEntry| {
            !entry.file_type().is_dir() || !predicate(&DirNodeRef(entry))
        }));
        self
    }

    /// Orders the entries of every listed directory with `compare`.
    ///
    /// Ordering is per-directory; there is no global ordering across hierarchy
    /// levels. Comparators run on cloned node views (one allocation per
    /// comparison) and must be [`Send`] + [`Sync`].
    ///
    /// Has no effect once iteration has begun.
    pub(crate) fn sorted_by<F>(mut self, mut compare: F) -> Self
    where
        F: FnMut(&DirNodeRef<'_>, &DirNodeRef<'_>) -> std::cmp::Ordering
            + Send
            + Sync
            + 'static,
    {
        if let Some(pending) = self.builder.take() {
            self.builder =
                Some(pending.sort_by(move |a, b| {
                    compare(&DirNodeRef(a), &DirNodeRef(b))
                }));
        }
        self
    }

    /// Lazily starts the walk and returns a reference to the inner iterator.
    ///
    /// The builder is consumed on the first call; subsequent calls return the
    /// same boxed iterator. If a [`filter`](Self::filter) predicate was
    /// configured, it is applied here.
    fn start(&mut self) -> &mut InnerIter {
        let Self {
            builder,
            prune,
            inner,
            ..
        } = self;
        inner.get_or_insert_with(|| {
            let iter = builder.take().map_or_else(
                || WalkDir::new("").into_iter(),
                WalkDir::into_iter,
            );
            match prune.take() {
                Some(predicate) => {
                    InnerIter::Filtered(iter.filter_entry(predicate))
                }
                None => InnerIter::Plain(iter),
            }
        })
    }
}

impl Iterator for DirTree {
    type Item = Result<DirNode, DirTreeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.start().next().map(|result| DirNode::try_new(&self.root, result))
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

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
            let entries: Vec<_> = DirTree::children(temp.path()).collect();

            // Assert
            assert!(entries.is_empty(), "an empty directory has no children");
        }

        #[test]
        fn returns_single_missing_root_error_when_directory_is_missing() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let missing = temp.path().join("does-not-exist");

            // Act
            let collected: Vec<_> = DirTree::children(&missing).collect();

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
            // Arrange: created deliberately out of lexicographic order.
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "z.md");
            write(root, "a.md");

            // Act
            let names: Vec<String> = DirTree::children(root)
                .sorted_by(|a, b| a.file_name().cmp(b.file_name()))
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| node.file_name().to_string_lossy().into_owned())
                .collect();

            // Assert: yielded already ordered; nothing sorted after the walk.
            assert_eq!(names, vec!["a.md", "z.md"]);
        }

        #[test]
        fn returns_no_entries_when_root_is_file() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let file = write(temp.path(), "plain.md");

            // Act / Assert
            assert!(
                DirTree::children(&file).next().is_none(),
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
        fn returns_complete_tree_when_walking_directory() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "a.md");
            write(root, "b/one.md");

            // Act
            let mut relatives: Vec<String> = DirTree::descendants(root)
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
        fn returns_missing_root_error_when_root_is_missing() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let missing = temp.path().join("gone");

            // Act
            let collected: Vec<_> = DirTree::descendants(&missing).collect();

            // Assert
            assert_eq!(collected.len(), 1);
            assert!(matches!(
                collected.into_iter().next().expect("one item"),
                Err(DirTreeError::MissingRoot { .. })
            ));
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
        fn returns_no_entries_when_filter_matches_root() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path().to_path_buf();
            write(&root, "child.md");

            // Act
            let tree = DirTree::descendants(&root);
            let entries: Vec<_> =
                tree.filter(move |node| node.path() == root).collect();

            // Assert
            assert!(
                entries.is_empty(),
                "a matching root prunes the whole directory tree: {entries:?}"
            );
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
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| {
                    node.path()
                        .strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();

            // Assert: depth-first with siblings name-ordered per directory; the
            // root itself still comes first.
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

        #[test]
        fn returns_entries_outside_matching_nested_subtree() {
            // Arrange: nested matching dir (not a direct child of root);
            // dot-prefixed tempdir name must NOT match the predicate.
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "proj/.traces/config.toml");
            write(root, "proj/a.md");

            // Act
            let relatives: Vec<String> = DirTree::descendants(root)
                .filter(|node| node.file_name() == ".traces")
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| {
                    node.path()
                        .strip_prefix(root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();

            // Assert: the .traces/config.toml file is pruned with its dir; root
            // itself and surviving files remain.
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

    mod builder {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn returns_unfiltered_entries_when_filter_is_configured_after_iteration_starts()
         {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "a.md");
            write(root, "b.md");
            write(root, "c.md");
            write(root, "sub/nested.md");
            let mut tree = DirTree::children(root).sorted_by(|left, right| {
                left.file_name().cmp(right.file_name())
            });

            // Act
            let first = tree
                .next()
                .expect("first child")
                .expect("first child is readable");
            let names: Vec<String> = tree
                .filter(|node| node.file_name() == "sub")
                .map(|entry| entry.expect("child is readable"))
                .map(|node| node.file_name().to_string_lossy().into_owned())
                .collect();

            // Assert
            assert_eq!(first.file_name(), std::ffi::OsStr::new("a.md"));
            assert_eq!(names, vec!["b.md", "c.md", "sub"]);
        }

        #[test]
        fn leaves_comparator_uninvoked_when_sorting_is_configured_after_iteration_starts()
         {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "a.md");
            write(root, "b.md");
            write(root, "c.md");
            let comparisons =
                std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let comparisons_in_compare = std::sync::Arc::clone(&comparisons);
            let mut tree = DirTree::children(root);

            // Act
            let first = tree.next().expect("first child");
            let rest: Vec<_> = tree
                .sorted_by(move |left, right| {
                    comparisons_in_compare
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    left.file_name().cmp(right.file_name())
                })
                .collect();

            // Assert
            assert!(first.is_ok(), "first child is readable: {first:?}");
            assert_eq!(rest.len(), 2);
            assert!(
                rest.iter().all(Result::is_ok),
                "remaining children are readable: {rest:?}"
            );
            assert_eq!(
                comparisons.load(std::sync::atomic::Ordering::Relaxed),
                0,
                "late comparator must not run"
            );
        }
    }

    mod classify {
        use pretty_assertions::assert_eq;

        use super::*;

        /// RAII guard that restores permissions on drop so that a `0o000`
        /// directory does not block the tempdir's cleanup.
        #[cfg(unix)]
        struct RestorePermissions<'a>(&'a Path);

        #[cfg(unix)]
        impl Drop for RestorePermissions<'_> {
            fn drop(&mut self) {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(
                    self.0,
                    fs::Permissions::from_mode(0o700),
                );
            }
        }

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
            let collected: Vec<_> = DirTree::children(root).collect();

            // Assert: stat on the root still succeeds (parent grants it), so
            // this is an access failure, not absence.
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

            // Arrange an unreadable child directory. A flat listing must yield
            // it as an entry rather than attempt to open it.
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            let kid = root.join("locked-kid");
            fs::create_dir(&kid).expect("create locked dir");
            fs::set_permissions(&kid, fs::Permissions::from_mode(0o000))
                .expect("revoke permissions");
            let _restore = RestorePermissions(&kid);

            // Act
            let collected: Vec<_> = DirTree::children(root).collect();

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
            let mut errors = DirTree::descendants(root).filter_map(Result::err);

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
                .next()
                .expect("one entry")
                .expect("entry is readable");
            let metadata = node.metadata().expect("read file metadata");

            // Assert
            assert!(metadata.is_file(), "fixture is a regular file");
            assert_eq!(metadata.len(), 7);
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
                .find(|entry| matches!(entry, Ok(node) if node.path() == link.as_path()))
                .expect("symlink entry is yielded")
                .expect("symlink entry is readable");
            let metadata = node.metadata().expect("read symlink metadata");

            // Assert
            assert_eq!(node.path(), link);
            assert!(node.file_type().is_symlink(), "entry remains a symlink");
            assert!(
                metadata.file_type().is_symlink(),
                "metadata remains for the symlink rather than its target"
            );
        }
    }

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
}
