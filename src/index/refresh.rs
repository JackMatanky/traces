//! Plans incremental index refreshes and applies typed updates to the index
//! store.
//!
//! [`RefreshState`] models refresh outcomes. Fresh scans use the store as-is.
//! Stale scans use [`PendingApply`] before they return a persisted store.
//! Drifted scans rebuild the index. Drift takes priority over staleness.
//! [`PersistFailed`] keeps the pending data so [`IndexerService::refresh`] can
//! still build an in-memory index after a write fails.
//!
//! [`IndexerService::refresh`]: super::service::IndexerService::refresh

use std::{cmp::Ordering, collections::HashSet, path::Path};

use super::{
    IndexError, IndexResult, WorkspaceIndex,
    delta::{FileDelta, InlinkDelta},
    fingerprint::{DriftCause, IndexFingerprint},
    inlinks::{self, InlinkMap},
    service::IndexerService,
    sort::SortedByPath,
    store::{IndexDimensions, IndexStore, PersistRequest},
};
use crate::{FileMeta, Note};

/// Reports the changes found and handled by one refresh.
///
/// Count accessors return zero for variants without counts. A config drift
/// report gives its cause instead of row counts.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum RefreshReport {
    /// Nothing changed, so the refresh used the persisted index as-is.
    #[default]
    Unchanged,
    /// The refresh found changed files and updated their rows.
    FilesChanged {
        /// Counts files inserted or updated.
        upserted: usize,
        /// Counts files deleted from the index.
        deleted: usize,
        /// Counts added and removed inbound link edges.
        links_modified: usize,
    },
    /// The config changed, so the refresh rebuilt the index.
    ConfigDrift {
        /// Explains why the stored index no longer matched the config.
        cause: DriftCause,
    },
}

#[cfg_attr(
    not(any(test, feature = "test-utils")),
    expect(
        dead_code,
        reason = "production only logs reports; tests and benches read the \
                  counts through the testing facade"
    )
)]
impl RefreshReport {
    /// Returns the number of files inserted or updated, or zero unless files
    /// changed.
    #[inline]
    #[must_use]
    pub const fn upserted_count(&self) -> usize {
        match self {
            Self::FilesChanged {
                upserted,
                ..
            } => *upserted,
            Self::Unchanged
            | Self::ConfigDrift {
                ..
            } => 0,
        }
    }

    /// Returns the number of files deleted from the index, or zero unless files
    /// changed.
    #[inline]
    #[must_use]
    pub const fn deleted_count(&self) -> usize {
        match self {
            Self::FilesChanged {
                deleted,
                ..
            } => *deleted,
            Self::Unchanged
            | Self::ConfigDrift {
                ..
            } => 0,
        }
    }

    /// Returns the number of inbound link edges updated, or zero unless files
    /// changed.
    #[inline]
    #[must_use]
    pub const fn links_modified_count(&self) -> usize {
        match self {
            Self::FilesChanged {
                links_modified,
                ..
            } => *links_modified,
            Self::Unchanged
            | Self::ConfigDrift {
                ..
            } => 0,
        }
    }
}

/// Holds the refresh state from one scan before persistence.
pub(super) enum RefreshState {
    /// No file metadata changed. The opened store remains current.
    Fresh {
        store: IndexStore,
        files: SortedByPath<FileMeta>,
    },
    /// File metadata changed. The refresh has reconciled the changes but has
    /// not persisted them.
    Stale(Box<PendingApply>),
    /// The store used a different config. The plan rebuilt the index from its
    /// scan and keeps its store handle for the rebuild write.
    Drifted {
        /// Explains why the stored index no longer matched the config.
        cause: DriftCause,
        /// Holds the index rebuilt from the plan's scan.
        index: WorkspaceIndex,
        /// Holds the plan's open store. redb allows only one open handle.
        store: IndexStore,
    },
}

/// Holds reconciled refresh data waiting for a store write.
pub(super) struct PendingApply {
    store: IndexStore,
    update: IndexUpdate,
}

impl PendingApply {
    /// Returns the changed-row report without applying the update.
    #[inline]
    pub(super) fn report(&self) -> RefreshReport {
        self.update.report()
    }

    /// Persists this pending update through [`IndexStore::persist`].
    ///
    /// The update remains owned by the returned state in both success and error
    /// cases. A failed apply can still build an in-memory index from the same
    /// rows.
    ///
    /// # Errors
    ///
    /// - [`PersistFailed`] if the store write fails. The error retains the
    ///   pending update.
    pub(super) fn apply(
        self,
        dimensions: &IndexDimensions,
        fingerprint: &IndexFingerprint,
    ) -> Result<Persisted, PersistFailed> {
        let result = {
            let notes = self.update.notes_to_upsert();
            self.store.persist(&PersistRequest::incremental(
                dimensions,
                fingerprint,
                self.update.delta(),
                &notes,
                self.update.inlink_delta(),
            ))
        };
        match result {
            Ok(()) => Ok(Persisted {
                pass: self,
            }),
            Err(source) => Err(PersistFailed {
                source,
                pass: Box::new(self),
            }),
        }
    }

    /// Materializes this pending update into an in-memory [`WorkspaceIndex`].
    ///
    /// # Errors
    ///
    /// - [`IndexError::Store`] if the store cannot read unchanged notes.
    pub(super) fn into_index(self) -> IndexResult<WorkspaceIndex> {
        let Self {
            store,
            update,
        } = self;
        update.into_index(&store)
    }
}

/// Represents a successfully persisted refresh state.
pub(super) struct Persisted {
    pass: PendingApply,
}

impl Persisted {
    /// Returns the current store after a successful apply.
    #[inline]
    pub(super) fn into_store(self) -> IndexStore {
        self.pass.store
    }

    /// Materializes the persisted pass into an in-memory [`WorkspaceIndex`].
    ///
    /// # Errors
    ///
    /// - [`IndexError::Store`] if the store cannot read unchanged notes.
    #[inline]
    pub(super) fn into_index(self) -> IndexResult<WorkspaceIndex> {
        self.pass.into_index()
    }
}

/// Holds a persistence error and the data needed for in-memory materialization.
///
/// The type boxes the pending update to keep the returned error small.
pub(super) struct PersistFailed {
    source: IndexError,
    pass: Box<PendingApply>,
}

impl PersistFailed {
    /// Returns the persistence error while preserving the pass.
    #[inline]
    pub(super) const fn source(&self) -> &IndexError {
        &self.source
    }

    /// Consumes the failure into its persistence error.
    #[inline]
    pub(super) fn into_error(self) -> IndexError {
        self.source
    }

    /// Materializes the unpersisted pass into an in-memory [`WorkspaceIndex`].
    ///
    /// # Errors
    ///
    /// - [`IndexError::Store`] if the store cannot read unchanged notes.
    #[inline]
    pub(super) fn into_index(self) -> IndexResult<WorkspaceIndex> {
        (*self.pass).into_index()
    }
}

/// Tracks the notes used to build the reconciled inlink map.
pub(super) struct InlinkReconciliation {
    links: InlinkMap,
    notes: NoteScope,
}

/// Names the note set needed to persist and materialize refresh state.
pub(super) enum NoteScope {
    /// Holds only parsed notes from changed files. Unchanged notes remain in
    /// the store.
    Modified(Vec<Note>),
    /// Holds the complete, path-sorted note set.
    Complete(Vec<Note>),
}

impl NoteScope {
    /// Returns note rows to upsert for this scope.
    ///
    /// `Modified` returns every parsed note because each came from a changed
    /// file. `Complete` searches the rebuilt set and returns notes named by the
    /// file delta.
    #[must_use]
    pub(super) fn to_upsert(&self, upserted: &[FileMeta]) -> Vec<&Note> {
        match self {
            Self::Modified(notes) => notes.iter().collect(),
            Self::Complete(notes) => upserted
                .iter()
                .filter_map(|file| {
                    notes
                        .binary_search_by(|note| note.path().cmp(file.path()))
                        .ok()
                        .and_then(|idx| notes.get(idx))
                })
                .collect(),
        }
    }

    /// Resolves this scope into a complete, path-sorted note set.
    ///
    /// # Errors
    ///
    /// - [`IndexError::Store`] if the store cannot read unchanged notes.
    pub(super) fn resolve(
        self,
        store: &IndexStore,
        delta: &FileDelta,
    ) -> IndexResult<SortedByPath<Note>> {
        match self {
            Self::Modified(notes) => merge_refreshed_notes(store, delta, notes),
            Self::Complete(notes) => Ok(SortedByPath::assumed_sorted(notes)),
        }
    }
}

/// Holds the scanned and diffed state for one index root before note parsing.
pub(super) struct RefreshPlan {
    store: IndexStore,
    current_files: SortedByPath<FileMeta>,
    persisted_files: SortedByPath<FileMeta>,
    delta: FileDelta,
    drift: Option<DriftCause>,
}

impl RefreshPlan {
    /// Opens [`IndexStore`], scans the filesystem, and reads stored files and
    /// their fingerprint.
    ///
    /// The filesystem walk runs alongside [`IndexStore::open`] and the store
    /// read. This method does not decode link rows. Reconciliation or index
    /// materialization loads them as needed. A virgin store has no drift; its
    /// first build is a file change.
    ///
    /// # Errors
    ///
    /// - [`IndexError::Scan`] if the scan cannot read a directory, node, or
    ///   metadata.
    /// - [`IndexError::Inspect`] if the scan cannot convert file metadata into
    ///   a record.
    /// - [`IndexError::Path`] if the scan cannot make a safe project-relative
    ///   path.
    /// - [`IndexError::Store`] if the store cannot open or read its tables.
    pub(super) fn collect(
        root: &Path,
        expected: &IndexFingerprint,
    ) -> IndexResult<Self> {
        let (opened, scanned) = rayon::join(
            || -> IndexResult<_> {
                let store = IndexStore::open(root)?;
                let persisted = store.read_files_and_fingerprint()?;
                Ok((store, persisted))
            },
            || IndexerService::scan(root),
        );
        let (store, (persisted_files, stored)) = opened?;
        let current_files = SortedByPath::assumed_sorted(scanned?);
        let delta = FileDelta::compute(&current_files, &persisted_files);
        Ok(Self {
            store,
            current_files,
            persisted_files,
            delta,
            drift: expected.drift(&stored),
        })
    }

    /// Takes the cause of the drift, or `None` if the store matches the running
    /// config.
    #[inline]
    pub(super) const fn take_drift(&mut self) -> Option<DriftCause> {
        self.drift.take()
    }

    /// Consumes the plan into its store handle and the scan it already did, so
    /// a rebuild reuses both.
    #[inline]
    pub(super) fn into_scan(self) -> (IndexStore, SortedByPath<FileMeta>) {
        (self.store, self.current_files)
    }

    /// Reports whether the scan produced no changes.
    #[inline]
    pub(super) fn is_fresh(&self) -> bool {
        self.delta.is_empty()
    }

    /// Returns the files this state would upsert.
    #[inline]
    pub(super) fn upserted_files(&self) -> &[FileMeta] {
        self.delta.upserted()
    }

    #[inline]
    pub(super) fn store(&self) -> &IndexStore {
        &self.store
    }

    #[inline]
    pub(super) fn persisted_files(&self) -> &SortedByPath<FileMeta> {
        &self.persisted_files
    }

    /// Consumes the unchanged plan into [`RefreshState::Fresh`].
    #[inline]
    pub(super) fn into_fresh(self) -> RefreshState {
        RefreshState::Fresh {
            store: self.store,
            files: self.persisted_files,
        }
    }

    /// Reconciles reparsed notes with persisted state.
    ///
    /// # Errors
    ///
    /// - [`IndexError::Store`] if the path-set rebuild cannot read stored
    ///   notes.
    pub(super) fn reconcile(
        self,
        modified_notes: Vec<Note>,
        persisted: InlinkMap,
    ) -> IndexResult<PendingApply> {
        let (inlinks, inlink_delta) =
            if Self::is_paths_unchanged(&self.delta, &self.persisted_files) {
                let edited: HashSet<&Path> =
                    modified_notes.iter().map(Note::path).collect();
                let new_edges = inlinks::resolve_edges_for(
                    &modified_notes,
                    self.current_files.as_slice(),
                );
                let (links, inlink_delta) =
                    persisted.patch_and_diff(&edited, &new_edges);
                (
                    InlinkReconciliation {
                        links,
                        notes: NoteScope::Modified(modified_notes),
                    },
                    inlink_delta,
                )
            } else {
                let (notes_res, resolver) = rayon::join(
                    || {
                        merge_refreshed_notes(
                            &self.store,
                            &self.delta,
                            modified_notes,
                        )
                    },
                    || {
                        inlinks::LinkResolver::new(
                            self.current_files.as_slice(),
                        )
                    },
                );
                let notes = notes_res?;
                let links =
                    InlinkMap::with_resolver(notes.as_slice(), &resolver);
                let inlink_delta = InlinkDelta::compute(&links, &persisted);
                (
                    InlinkReconciliation {
                        links,
                        notes: NoteScope::Complete(notes.into_vec()),
                    },
                    inlink_delta,
                )
            };
        let update = IndexUpdate {
            current_files: self.current_files,
            delta: self.delta,
            inlinks,
            inlink_delta,
        };
        Ok(PendingApply {
            store: self.store,
            update,
        })
    }

    /// Reports whether `delta` changed content only at previously indexed
    /// paths.
    ///
    /// Link resolution for unedited notes stays unchanged only in this case.
    fn is_paths_unchanged(
        delta: &FileDelta,
        persisted_files: &SortedByPath<FileMeta>,
    ) -> bool {
        delta.deleted().is_empty()
            && delta.upserted().iter().all(|file| {
                persisted_files.binary_search_by_path(file.path()).is_ok()
            })
    }
}

/// Holds the results of one reconciliation before persistence.
pub(super) struct IndexUpdate {
    current_files: SortedByPath<FileMeta>,
    delta: FileDelta,
    inlinks: InlinkReconciliation,
    inlink_delta: InlinkDelta,
}

impl IndexUpdate {
    /// Returns changed-row counts for this update.
    pub(super) fn report(&self) -> RefreshReport {
        RefreshReport::FilesChanged {
            upserted: self.delta.upserted().len(),
            deleted: self.delta.deleted().len(),
            links_modified: self
                .inlink_delta
                .upserted()
                .len()
                .saturating_add(self.inlink_delta.deleted().len()),
        }
    }

    #[inline]
    fn delta(&self) -> &FileDelta {
        &self.delta
    }

    #[inline]
    fn inlink_delta(&self) -> &InlinkDelta {
        &self.inlink_delta
    }

    #[inline]
    fn notes_to_upsert(&self) -> Vec<&Note> {
        self.inlinks.notes.to_upsert(self.delta.upserted())
    }

    /// Materializes this update into an in-memory [`WorkspaceIndex`].
    ///
    /// # Errors
    ///
    /// - [`IndexError::Store`] if the store cannot read unchanged notes.
    pub(super) fn into_index(
        self,
        store: &IndexStore,
    ) -> IndexResult<WorkspaceIndex> {
        let InlinkReconciliation {
            links,
            notes,
        } = self.inlinks;
        let notes = notes.resolve(store, &self.delta)?;
        Ok(WorkspaceIndex::assemble(self.current_files, notes, links))
    }
}

/// Merges stored notes with deletions and reparsed notes for a full recompute.
///
/// Moves `modified_notes` into the result without cloning notes.
///
/// Reads all stored notes because this fallback needs the complete set. It
/// removes deleted paths, then merges the two path-sorted inputs. Modified
/// notes replace stored rows, and the result remains sorted.
///
/// # Errors
///
/// - [`IndexError::Store`] if the store cannot read or decode notes.
fn merge_refreshed_notes(
    store: &IndexStore,
    delta: &FileDelta,
    modified_notes: Vec<Note>,
) -> IndexResult<SortedByPath<Note>> {
    let mut all_notes = store.read_all_notes()?.into_vec();

    if !delta.deleted().is_empty() {
        let deleted: HashSet<&Path> =
            delta.deleted().iter().map(FileMeta::path).collect();
        all_notes.retain(|n| !deleted.contains(n.path()));
    }

    // Both inputs are path-ascending: `read_all_notes` returns sorted rows and
    // `parse_notes` preserves the sorted upsert order, so one merge pass
    // produces the union with modified notes replacing persisted rows.
    let mut merged = Vec::with_capacity(
        all_notes.len().saturating_add(modified_notes.len()),
    );
    let mut persisted = all_notes.into_iter().peekable();
    let mut modified = modified_notes.into_iter().peekable();
    loop {
        match (persisted.peek(), modified.peek()) {
            (None, None) => break,
            (Some(_), None) => merged.extend(persisted.by_ref()),
            (None, Some(_)) => merged.extend(modified.by_ref()),
            (Some(old), Some(new)) => match old.path().cmp(new.path()) {
                Ordering::Less => merged.extend(persisted.by_ref().take(1)),
                Ordering::Greater => merged.extend(modified.by_ref().take(1)),
                Ordering::Equal => {
                    persisted.next();
                    merged.extend(modified.by_ref().take(1));
                }
            },
        }
    }
    Ok(SortedByPath::assumed_sorted(merged))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod report {
        use pretty_assertions::assert_eq;

        use super::*;

        #[test]
        fn counts_upserts_deletes_and_link_edges() {
            let delta = FileDelta::compute(
                &SortedByPath::sorted(vec![FileMeta::note_for_test("a.md")]),
                &SortedByPath::sorted(vec![FileMeta::note_for_test("b.md")]),
            );
            let inlink_delta = InlinkDelta::compute(
                &InlinkMap::default(),
                &InlinkMap::default(),
            );
            let update = IndexUpdate {
                current_files: SortedByPath::default(),
                delta,
                inlinks: InlinkReconciliation {
                    links: InlinkMap::default(),
                    notes: NoteScope::Modified(vec![]),
                },
                inlink_delta,
            };

            assert_eq!(update.report(), RefreshReport::FilesChanged {
                upserted: 1,
                deleted: 1,
                links_modified: 0,
            });
        }
    }

    mod note_scope {
        use std::path::Path;

        use pretty_assertions::assert_eq;

        use super::*;
        use crate::test_support::parse_note;

        #[test]
        fn returns_all_modified_notes_for_upsert() {
            let notes =
                vec![parse_note("a.md", "# A"), parse_note("b.md", "# B")];
            let scope = NoteScope::Modified(notes);
            let upserted = [FileMeta::note_for_test("a.md")];

            let paths: Vec<_> = scope
                .to_upsert(&upserted)
                .into_iter()
                .map(Note::path)
                .collect();

            assert_eq!(paths, [Path::new("a.md"), Path::new("b.md")]);
        }

        #[test]
        fn returns_complete_notes_matching_upserted_paths() {
            let notes =
                vec![parse_note("a.md", "# A"), parse_note("c.md", "# C")];
            let scope = NoteScope::Complete(notes);
            let upserted = [
                FileMeta::note_for_test("a.md"),
                FileMeta::note_for_test("b.md"),
            ];

            let paths: Vec<_> = scope
                .to_upsert(&upserted)
                .into_iter()
                .map(Note::path)
                .collect();

            assert_eq!(paths, [Path::new("a.md")]);
        }

        #[test]
        fn returns_no_complete_notes_when_upserted_paths_miss() {
            let notes = vec![parse_note("a.md", "# A")];
            let scope = NoteScope::Complete(notes);
            let upserted = [FileMeta::note_for_test("b.md")];

            assert_eq!(scope.to_upsert(&upserted), Vec::<&Note>::new());
        }
    }

    /// A rejected store write materializes the reconciled in-memory delta
    /// while leaving the previously persisted index untouched.
    #[test]
    fn materializes_unpersisted_delta_when_store_write_fails() {
        use std::{fs, io};

        use pretty_assertions::assert_eq;

        use super::*;
        use crate::{index::error::StoreError, test_support::parse_note};

        let temp = tempfile::tempdir().expect("create temp dir");
        fs::write(temp.path().join("note.md"), "---\ntitle: old\n---\n# A")
            .expect("write note");
        let indexer = IndexerService::for_tests(temp.path());
        indexer
            .persist(&indexer.build().expect("build index"))
            .expect("persist index");

        fs::write(temp.path().join("note.md"), "---\ntitle: new\n---\n# A")
            .expect("rewrite note");
        let plan =
            RefreshPlan::collect(temp.path(), &IndexFingerprint::for_test())
                .expect("collect refresh plan");
        let persisted_links = plan
            .store()
            .read_all_links(plan.persisted_files())
            .expect("read persisted links");
        let modified = vec![parse_note("note.md", "---\ntitle: new\n---\n# A")];
        let pending = plan
            .reconcile(modified, persisted_links)
            .expect("reconcile stale pass");

        let report = pending.report();
        let index = super::super::materialize_apply(Err(PersistFailed {
            source: IndexError::Store(StoreError::Io {
                path: temp.path().join(".traces/index.redb"),
                source: io::Error::other("induced persist failure"),
            }),
            pass: Box::new(pending),
        }))
        .expect("fail-open materialization");
        assert_eq!(report.upserted_count(), 1);
        assert_eq!(report.deleted_count(), 0);
        let note = index
            .entries()
            .iter()
            .find(|entry| entry.file().path() == Path::new("note.md"))
            .and_then(crate::index::FileEntry::note)
            .expect("note entry");
        assert_eq!(
            note.frontmatter()
                .and_then(|fm| fm.fields().values().next())
                .and_then(|value| value.as_str()),
            Some("new"),
            "unpersisted delta must survive in the returned index"
        );

        // Loading does not refresh, so it cannot repair the store under test.
        let persisted = IndexerService::for_tests(temp.path())
            .load()
            .expect("load untouched store");
        let persisted_note = persisted
            .entries()
            .iter()
            .find(|entry| entry.file().path() == Path::new("note.md"))
            .and_then(crate::index::FileEntry::note)
            .expect("persisted note entry");
        assert_eq!(
            persisted_note
                .frontmatter()
                .and_then(|fm| fm.fields().values().next())
                .and_then(|value| value.as_str()),
            Some("old"),
            "fail-open must not persist the delta as a side effect"
        );
    }
}
