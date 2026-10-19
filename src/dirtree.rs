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
//!   decide their policy: absorb it into an empty walk via
//!   [`DirTree::missing_as_empty`] or propagate it.
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
}

impl DirNode {
    /// Adapts one raw walkdir item into this module's interface.
    ///
    /// Depth 0 denotes the walk root itself, so a depth-0 failure is
    /// classified against the root rather than as a node beneath it. Errors
    /// that carry no path (mid-readdir stream glitches) fall back to the
    /// walk root.
    fn try_new(
        root: &Path,
        result: walkdir::Result<DirEntry>,
    ) -> Result<Self, DirTreeError> {
        let failure = match result {
            Ok(entry) => return Ok(Self(entry)),
            Err(failure) => failure,
        };
        let depth = failure.depth();
        let path = failure.path().unwrap_or(root).to_path_buf();
        let source = io::Error::from(failure);
        let error = match depth {
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
        };
        Err(error)
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
    /// Terminal state for a walk that never obtained a builder: yields
    /// nothing instead of fabricating a dummy root.
    Done,
}

impl Iterator for InnerIter {
    type Item = walkdir::Result<DirEntry>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Plain(iter) => iter.next(),
            Self::Filtered(iter) => iter.next(),
            Self::Done => None,
        }
    }
}
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
    absorb_missing_root: bool,
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
            absorb_missing_root: false,
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
            absorb_missing_root: false,
        }
    }

    /// Absorbs a missing walk root into an empty walk.
    ///
    /// [`DirTreeError::MissingRoot`] is the only variant this absorbs: the
    /// walk yields nothing instead of one error item. Other failures
    /// ([`DirTreeError::RootInaccessible`], [`DirTreeError::NodeInaccessible`])
    /// still surface, so callers keep their strict handling of unreadable
    /// roots. This is the shared policy for callers that must tolerate a
    /// missing directory and see it as "no entries".
    ///
    /// Has no effect once iteration has begun.
    #[must_use]
    pub(crate) fn missing_as_empty(mut self) -> Self {
        self.absorb_missing_root = true;
        self
    }

    /// Removes every subtree whose directory satisfies `predicate`.
    ///
    /// `predicate` is consulted only for directories *beneath* the walk root.
    /// Returning `true` removes that directory *and* everything beneath it
    /// from the walk. Non-matching entries (including files whose name
    /// satisfies the predicate) are yielded unchanged. The walk root itself
    /// is exempt: the predicate never sees it, so a predicate that matches
    /// the root's own name leaves the walk intact.
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
            entry.depth() == 0
                || !entry.file_type().is_dir()
                || !predicate(&DirNodeRef(entry))
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
            let Some(builder) = builder.take() else {
                return InnerIter::Done;
            };
            let iter = WalkDir::into_iter(builder);
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
        while let Some(item) = self
            .start()
            .next()
            .map(|result| DirNode::try_new(&self.root, result))
        {
            let absorbed = self.absorb_missing_root
                && matches!(&item, Err(DirTreeError::MissingRoot { .. }));
            if !absorbed {
                return Some(item);
            }
        }
        None
    }
}

/// Represents a failure raised while traversing a directory tree.
///
/// Callers absorb [`MissingRoot`](Self::MissingRoot) via
/// [`DirTree::missing_as_empty`] or match it to state their missing-root
/// policy, and convert remaining errors via
/// [`into_parts`](Self::into_parts).
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
        fn sorted_by_orders_immediate_entries() {
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
        fn a_file_root_yields_no_entries_and_no_errors() {
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
        fn filter_removes_matching_directories_from_listing() {
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

        #[test]
        fn filter_configured_after_the_first_next_is_ignored() {
            // Arrange
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "a.md");
            write(root, "b.md");
            write(root, "c.md");

            // Act: pull one entry, then reconfigure the started walk with a
            // predicate that matches everything.
            let mut tree = DirTree::children(root);
            assert!(
                tree.next().expect("entries exist").is_ok(),
                "first entry is fine"
            );
            let tree = tree.filter(|_| true);
            let rest: Vec<_> = tree.collect();

            // Assert: the pre-start builder was already consumed, so the late
            // filter is silently ignored; an honored predicate would empty
            // the remaining walk.
            assert_eq!(rest.len(), 2, "post-start filter must not prune");
            assert!(rest.iter().all(Result::is_ok));
        }

        #[test]
        fn sorted_by_configured_after_the_first_next_is_ignored() {
            // Arrange: the comparator counts its invocations.
            let temp = tempfile::tempdir().expect("create temp dir");
            let root = temp.path();
            write(root, "a.md");
            write(root, "b.md");
            write(root, "c.md");
            let comparisons =
                std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let comparisons_in_compare = std::sync::Arc::clone(&comparisons);

            // Act: pull one entry, then reconfigure the started walk.
            let mut tree = DirTree::children(root);
            assert!(tree.next().expect("entries exist").is_ok());
            let tree = tree.sorted_by(move |a, b| {
                comparisons_in_compare
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                a.file_name().cmp(b.file_name())
            });
            let rest: Vec<_> = tree.collect();

            // Assert: ordering is applied when the walk starts, so a
            // comparator attached afterwards never runs while the remaining
            // entries still yield.
            assert_eq!(rest.len(), 2, "remaining entries still yield");
            assert_eq!(
                comparisons.load(std::sync::atomic::Ordering::Relaxed),
                0,
                "post-start comparator must never run"
            );
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
            let collected: Vec<_> = DirTree::descendants(&missing).collect();

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
                DirTree::children(&missing).missing_as_empty().collect();
            let deep: Vec<_> =
                DirTree::descendants(&missing).missing_as_empty().collect();
            let present: Vec<_> =
                DirTree::children(temp.path()).missing_as_empty().collect();

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
                DirTree::children(root).missing_as_empty().collect();

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
        fn filter_prunes_matching_subtrees_but_keeps_other_entries() {
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
        fn filter_keeps_files_whose_name_matches_the_predicate() {
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
        fn filter_never_consults_the_predicate_for_the_walk_root() {
            // Arrange: the root deliberately carries an "ignored" name, and
            // the predicate counts every directory it is asked about.
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
                .map(|entry| entry.expect("entry is ok"))
                .map(|node| {
                    node.path()
                        .strip_prefix(&root)
                        .expect("under root")
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();

            // Assert: the root survives its own name-matching predicate and
            // the predicate never runs at depth 0, so callers no longer guard
            // against emptying the walk.
            assert_eq!(relatives, vec!["", "note.md"]);
            assert_eq!(
                seen.load(std::sync::atomic::Ordering::Relaxed),
                0,
                "predicate must not be invoked for the walk root"
            );
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
            let collected: Vec<_> = DirTree::children(root).collect();

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

            // Arrange: a subdirectory that cannot be opened. The contract
            // under test is the documented one - a flat listing yields it as
            // a plain entry with no error, while recursive walks surface a
            // NodeInaccessible instead.
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
