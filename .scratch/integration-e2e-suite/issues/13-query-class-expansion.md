# 13: Query & class expansion — parity differential + `@class` + schema file deletion

**What to build:** The store-backed query parity differential proves in-memory and
persisted-store results structurally identical for a representative selector set including
cold-path inlinks; `@class` source expansion is covered from both external entry points
(template composition root + CLI); and the test-only-constructor schema integration file is
deleted in the same change so reachability coverage transfers rather than vanishes.

**Blocked by:** 06 (frozen facade is the export surface these tests import through),
08 (spawn-only invariant for the paired CLI test).

**Category:** enhancement

**Status:** resolved

- [x] Parity differential: representative selectors (`#tag`, nested tags, paths,
      `(#a or notes/b.md)`, `not #t`) through `QueryService::run` vs `sync_and_run`,
      structurally compared; **persist-first arrangement pinned** so the from-disk fresh branch
      is guaranteed (otherwise the differential degenerates into querying rows written in the
      same call); covers store legs only — config-load and class-expander legs belong to
      tickets 11 and this ticket's template test respectively
- [x] Cold-path inlinks included in the selector set (the real residue of the demoted inlinks
      gap — a plain `list("inlinks")` integration test would near-duplicate an existing
      in-crate test, so it goes here instead)
- [x] `@class` via the template composition root: schemas written to the default schemas dir,
      render `query.from("@book*")` / `class(Book, children)` forms, assert transitive-`extends`
      rows + unknown-class degradation — **zero new exports** (the template service,
      `render_to_file`, `DryRun`, preset dialog provider, schema writer are all exported today;
      the class expander attaches internally)
- [x] Paired CLI E2E: `traces list --from '@book*'` (separate CLI path from the template
      render)
- [x] Direct query-service `@class` test NOT written — proven vacuous (expander is
      `pub(crate)`; `QueryService::new` leaves it `None`)
- [x] `schema_field_resolution` integration file deleted in **the same change**; the
      replacement mapping (what the template-class test covers vs what stays with source-local
      field-inheritance twins) is recorded in this ticket and carried into ticket 15's ledger
- [x] Each new test carries executes-vs-asserts note + defect class (silent-wrong-answers for
      the parity differential; reachability-transfer for the class-expansion tests) — spec
      §Testing Decisions / US62, applied to every new test
- [x] File homes: template-class test lands in the integration template-render seam file;
      paired CLI test in the E2E `query` capability file (final names per ticket 19)
- [x] Visibility freeze: api diffs empty (the zero-new-exports claim is verified, not asserted)

## Comments

Resolved in this ticket branch (merged into `integration-e2e-suite`).

### Parity differential

`tests/integration/index_query.rs::store_backed_and_in_memory_queries_agree_
across_representative_selectors` — persist-first arrangement (build +
persist BEFORE the differential, so `sync_and_run`'s refresh takes the Fresh
branch and store-side rows are served from disk); selectors: `All`, `#todo`,
`#project/active` (nested), `"notes/a.md"` (path), `docs/` (folder glob),
`(#todo or notes/b.md)`, `not #todo`; row-for-row structural equality
(path/depth/line/parent/task fields). The linked-note fixture forces the
store leg through the cold `read_all_links` read.

Cold-path inlink VALUES are asserted at process level in
`tests/e2e/query.rs::inlinks_render_from_the_persisted_store`
(`table --column inlinks`): `QueryRow`'s inlinks accessor is crate-internal
(visibility freeze), and a spawned cold process is the strongest observation
of the persisted link rows. Not a near-duplicate of the in-crate coverage
(accessor bundling unit + refresh re-resolution unit).

### `@class` expansion + unknown-class degradation semantics

`tests/integration/template_render.rs::renders_class_expanded_source_
selectors_through_the_schema_graph` and the paired
`tests/e2e/query.rs::class_descendant_selector_expands_through_the_schema_
graph`.

Empirically pinned degradation semantics: an unknown class (no schema file)
degrades to a **literal match** — no hierarchy expansion, no error. A note
carrying `class: horror` with no `horror` schema still matches `@horror`
(value on the class axis); `@horror*` has no descendants beyond that. The
E2E leg additionally proves a clean empty result when no note carries the
unknown class.

### schema_field_resolution.rs deletion mapping (carried into ticket 15)

Deleted `tests/integration/schema_field_resolution.rs` (5 tests through the
test-only `SchemaService` constructor) in the same change. Replacement
mapping:
- Graph reachability (`children_of`/`descendants_of`/`matches`) → the
  template-class test's `@book*` / `class(book, children)` / `@sci_fi*`
  expansion through the production composition root (TemplateService), plus
  existing in-crate schema-graph tests (`src/schema/graph/*` adjacency,
  cycle, builder units).
- Field inheritance internals (parent-first merge, overrides) → existing
  source-local twins in `src/schema/service.rs` + `src/schema/builder.rs`
  units (the deleted file's own module doc named those units as the
  internals coverage).
- `SchemaService` public-surface reachability → transferred to
  `TemplateService`, which resolves the schemas directory through `Config`
  on the production path.
- Zero new exports: the check:visibility diff stayed empty (verified).

### Notes

- Gate: `check:layers`, `check:visibility`, full suite (lib 3,151 /
  integration 58 / e2e 58 / doctests 83), clippy `-D warnings`, fmt — all
  green.

**Evidence:** audit §10 G-B2 (route correction: vacuous direct test), I-1 (parity differential,
R4 arrangement fixes), G2 (cold-path inlinks residue), §12 (schema file deletion gated here),
§14 P1.9/P1.14.
**Defect class:** silent-wrong-answers (resolver prefilter false negatives, codec drift);
reachability-transfer.
