# 12: Index & refresh family — delta, fail-open, path-set, divergence, corruption

**What to build:** Incremental indexing correctness is observed where users see it: a
query-visible refresh delta asserted through the non-mutating observation path, the documented
fail-open guarded at its only inducible point, content edits and delete/rename proven between
spawns, one divergence canary that genuinely distinguishes reload from rebuild, and
corruption self-heal asserted at process level.

**Blocked by:** 07 (prerequisites), 08 (spawn-only invariant), 10 (run accessors: code +
exact-stdout).

**Category:** enhancement

**Status:** resolved

- [x] Integration: build → persist → edit tag/body → assert **query-visible** delta through the
      public query API using non-mutating observation (`IndexerService::load()` +
      `QueryService::run`) — units only assert report counts today; `sync_and_run`/spawned
      refreshers used only when the refresh IS the act under test
- [x] In-crate unit: `refresh()` fail-open (documented at the seam; not inducible from outside
      since the store re-creates its file — unit is its only honest layer)
- [x] E2E content branch: note edit between two spawned `list` runs, asserted through output
- [x] E2E path-set branch: delete + rename between spawns → ghost rows gone, links
      re-resolved (distinct recompute path from content edits)
- [x] Divergence canary: **one canonical recipe** — same-size + restored-mtime content edit
      (portable) — proving reload ≠ rebuild before claiming "the store was read"; `chmod 000`
      variant cfg(unix), skipped when privileged; config-staleness recipe NOT used (ticket 04);
      plain successive-invocation content assertions make no store-read claim
- [x] Corruption self-heal: overwrite the index file with garbage → next spawned query exits 0
      with correct output and a valid file again (catches exit-101 panic, permanent
      `index::failed`, silent empty results)
- [x] Each test carries executes-vs-asserts note + defect class (stale-secondary-index,
      unobservable-reload, panic-on-corruption)
- [x] Visibility freeze: production and test-utils public-api diffs empty; textual pub-token
      diff empty (the new fail-open unit exports nothing)
- [x] Fidelity note respected: persist→load codec fidelity stays with the integration
      persistence file (already owns it)
- [x] E2E tests land in the `index` capability file; integration tests in the refresh seam file
      (final names per ticket 19)

## Comments

Resolved in this ticket branch (merged into `integration-e2e-suite`).

### Maintainer ruling (2026-10-09): corruption canary scope

Empirical finding: corruption recovery is damage-class-dependent.
- **Table-level corruption** (redb file magic preserved, tables trashed):
  a spawned `list` exits 0 with correct output and a rebuilt store — the
  spec's expected self-heal exists for this class, and the integration
  persistence file already owns the service-level contract for it.
- **Header-level destruction** (literal total-garbage overwrite): every
  command fails permanently with `traces::cli::index::failed` (exit 1, no
  panic) — including an explicit `traces index`; only manual deletion of
  `.traces/index.redb` recovers.
Ruling: the E2E canary covers the table-level class (exit-0 self-heal,
writes accepted again); the header-destruction boundary is documented here
and NOT pinned as an assertion. A potential wipe-and-recreate enhancement
for header-level damage is recorded for ticket 15's ledger (product change
→ would need its own spec).

### Review remediation (2026-10-09): original corruption criterion restored

The ruling above describes the original implementation, not the current
contract. `IndexStore::create_db` now recognizes redb's specific
magic-number-mismatch error as recoverable corruption and recreates that
file; unrelated I/O errors still propagate. The spawned
`fully_corrupted_index_self_heals_on_the_next_query` test overwrites the
entire file with garbage, checks the recovered query result, and checks
that the rebuilt store accepts later writes. The fail-open unit now injects
failure through the service's reconciliation branch and checks returned
unpersisted content against the still-old on-disk store.

### Deliverables

- `tests/integration/index_refresh.rs` (new, wired in `tests/integration.rs`):
  `refresh_persists_a_query_visible_delta_observed_without_mutation` — build
  → persist → edit → `refresh_with_report` (upsert 1, delete 0) → observed
  via `load()` + `QueryService::run` (new `#beta` tag resolves a.md, removed
  `#alpha` gone, rewritten task text visible). The refresh is the act; the
  observation path is non-mutating.
- In-crate fail-open unit: `src/index/refresh.rs::tests::
  refresh_service_returns_unpersisted_delta_when_persistence_fails` —
  builds a reconciled Stale pass, injects persistence failure through the
  service branch, and asserts the returned delta and untouched store.
  Defect class: unobservable-reload.
- `tests/e2e/index.rs` (new, wired in `tests/e2e.rs`):
  - `content_edit_between_spawns_is_query_visible` (stale-secondary-index)
  - `path_set_changes_between_spawns_prune_ghosts_and_resolve_links` —
    ambiguous `[[foo]]` re-resolves after deleting the sibling; exact `list`
    output + `inlinks` table row (stale-secondary-index)
  - `same_size_mtime_restored_edit_reads_the_store_not_a_rebuild` — the one
    canonical portable recipe (mtime restored with std
    `File::set_times`); query after the metadata-neutral edit still shows
    the old text, rebuild shows the new; the ticket-04 staleness recipe is
    not used (unobservable-reload)
  - `fully_corrupted_index_self_heals_on_the_next_query` — complete
    garbage overwrite, exit-0 exact-output self-heal, store accepts writes
    after (panic-on-corruption)
- Pre-existing flake repaired: `src/query/service.rs::
  runs_from_store_matching_in_memory_lists` compared whole `QuerySet`s,
  including FileMeta stat fields captured by two independent scans
  (`created_at` birth-time availability / sub-second precision differ under
  parallel I/O — observed twice this session). Narrowed to the seam contract
  (row structure + list content), matching the existing
  `assert_rows_match` precedent in the integration persistence file.
- `Cargo.toml`: mtime restore for the canary uses std `File::set_times` —
  no dev-dependency needed (std has had a portable setter since 1.75).
- Persist→load codec fidelity remains owned by
  `tests/integration/index_persistence_roundtrip.rs` (untouched).
- Gate: `check:layers`, `check:visibility` (empty diffs), full suite
  (lib 3,151 / integration 56 / e2e 55 / doctests 83), clippy `-D warnings`,
  fmt — all green.

**Evidence:** audit §10 G-B3/G4, E-2, plus the potential-defects entry's own disposition for
fail-open ("not inducible from the public API … **in-crate unit**"), §14 P1.10 (divergence
precondition analysis: no tracing subscriber in binary, `traces index` always rebuilds);
spec §Gap tests (refresh family — including corruption wipe-and-recreates).
**Defect class:** stale-secondary-index; unobservable-reload; panic-on-corruption.
