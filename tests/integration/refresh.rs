//! Refresh seam: proves an incremental refresh persists a query-visible
//! delta, observed through the non-mutating `load()` + query path.

use std::{path::Path, sync::Arc};

use pretty_assertions::assert_eq;
use traces_pkm::testing::{
    QueryBuilder, QueryService, SourceSelector, TestProject,
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
