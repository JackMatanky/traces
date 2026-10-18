//! Refresh seam: proves an incremental refresh persists a query-visible
//! delta, observed through the non-mutating `load()` + query path.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use pretty_assertions::assert_eq;
use traces::testing::{
    FileEntry, QueryBuilder, QueryService, SourceSelector, TestProject,
};

/// Verifies a refresh's delta is query-visible from a fresh, non-mutating
/// observation: `IndexerService::load()` plus [`QueryService::run`].
///
/// Units only assert report counts; this asserts what a user querying after
/// a refresh actually sees, without letting the observation itself refresh
/// or mutate anything.
///
/// Executes vs asserts:
/// - Executes: `build`/`persist` under an initial note, a content edit, then
///   `refresh_with_report` (the act under test), then observation via `load()`
///   + queries.
/// - Asserts: the report counts exactly one upsert; the new tag and task text
///   are query-visible from the persisted store; the removed tag is gone.
///
/// Defect class: stale-secondary-index (a refresh that reports success but
/// leaves queries reading the pre-edit store).
#[test]
fn refresh_persists_a_query_visible_delta_observed_without_mutation() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let project = TestProject::untrusted(temp.path().join("project"));
    project.write_note("a.md", "# A\n\n- [ ] old task #alpha\n");
    let (indexer, _built) = project.build_and_persist();

    let query_service = QueryService::new("class");

    // Baseline observation: the persisted store still shows the old tag.
    let loaded = Arc::new(indexer.load().expect("load persisted index"));
    let alpha = query_service.run(
        &loaded,
        QueryBuilder::pages(
            SourceSelector::parse("#alpha").expect("valid selector"),
        ),
    );
    assert_eq!(alpha.len(), 1, "baseline: #alpha resolves a.md");

    // Edit: swapped tag, rewritten task text.
    project.write_note("a.md", "# A\n\n- [ ] new task #beta\n");

    // Act: the refresh is the mutation under test.
    let (_refreshed, report) =
        indexer.refresh_with_report().expect("refresh with report");
    assert_eq!(report.upserted_count(), 1, "one edited file upserted");
    assert_eq!(report.deleted_count(), 0);

    // Observation without further mutation: a fresh load() plus queries.
    let observed = Arc::new(indexer.load().expect("load persisted index"));

    let beta = query_service.run(
        &observed,
        QueryBuilder::pages(
            SourceSelector::parse("#beta").expect("valid selector"),
        ),
    );
    assert_eq!(beta.len(), 1, "new tag must be query-visible");
    assert_eq!(
        beta.iter().next().map(|row| row.file().path()),
        Some(Path::new("a.md"))
    );

    let alpha_gone = query_service.run(
        &observed,
        QueryBuilder::pages(
            SourceSelector::parse("#alpha").expect("valid selector"),
        ),
    );
    assert_eq!(alpha_gone.len(), 0, "removed tag must vanish from queries");

    let tasks =
        query_service.run(&observed, QueryBuilder::tasks(SourceSelector::All));
    let texts: Vec<_> = tasks.iter().map(|row| row.task_text()).collect();
    assert_eq!(
        texts,
        [Some("new task #beta")],
        "rewritten task text must be visible"
    );
}
/// Verifies a path-changing refresh fully rebuilds links and merges modified
/// notes with retained persisted notes in path order.
///
/// The fixture deletes `b.md`, adds `a.md`, and modifies `f.md`. The remaining
/// persisted `d.md` must sort before the modified `f.md`, while added `a.md`
/// must sort before both. An unchanged `source.md` links to added `a.md`, so
/// observing that inlink proves the full-rebuild path includes unchanged notes.
#[test]
fn refresh_with_added_and_deleted_paths_rebuilds_links_and_merges_notes() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let project = TestProject::untrusted(temp.path().join("project"));
    project.write_note("b.md", "# Deleted\n");
    project.write_note("d.md", "# Retained\n");
    project.write_note("f.md", "# Old\n");
    project.write_note("source.md", "# Source\n\n[[a]]\n");
    let (indexer, _built) = project.build_and_persist();

    std::fs::remove_file(project.root().join("b.md"))
        .expect("remove persisted note");
    project.write_note("a.md", "# Added\n");
    project.write_note("f.md", "# New\n");

    let (refreshed, report) =
        indexer.refresh_with_report().expect("refresh path-changing delta");

    assert_eq!(report.upserted_count(), 2);
    assert_eq!(report.deleted_count(), 1);
    assert_eq!(
        refreshed
            .entries()
            .iter()
            .map(|entry| entry.file().path())
            .collect::<Vec<_>>(),
        [
            Path::new("a.md"),
            Path::new("d.md"),
            Path::new("f.md"),
            Path::new("source.md"),
        ]
    );

    let added = refreshed
        .entries()
        .iter()
        .find(|entry| entry.file().path() == Path::new("a.md"))
        .expect("added entry");
    assert_eq!(added.inlinks(), [PathBuf::from("source.md")]);

    let updated = refreshed
        .entries()
        .iter()
        .find(|entry| entry.file().path() == Path::new("f.md"))
        .and_then(FileEntry::note)
        .expect("updated note");
    assert_eq!(
        updated.headings().first().expect("updated heading").text(),
        "New"
    );
}
