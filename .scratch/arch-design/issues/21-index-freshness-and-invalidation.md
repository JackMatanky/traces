# Does the landed freshness model stand: config drift, path-set change, parser/format version?

Type: grilling
Status: open
Blocked by: 5, 18, 20

## Question

Config drift detection **has landed** (main @ `207cc3fb`, merged after the map
was charted; see `reverification.md`). Treat it as the "keep as is" baseline and
decide whether the freshness model as a whole stands, and what completes it:

1. **Ratify or revise the landed drift design.** Inputs enumerated by
exhaustively destructuring `ConfigDigests` into an index-owned
`IndexFingerprint` (10 of 15 settings; `src/index/fingerprint.rs:158-201`),
stored raw in the redb `meta` table (renamed from `cache_format`) in the same
write transaction as the rows (`src/index/store.rs` `write_meta`,
`read_files_and_fingerprint`), compared at every refresh; drift wins over
staleness and heals by full in-memory rebuild reusing the plan's scan and store
(`src/index/service.rs` `prepare_pass`, `Drifted` arm); unreadable fingerprint
fails closed as `DriftCause::Unrecorded`; a failed heal is logged and retried
next refresh. Alternatives to weigh against it: per-axis invalidation (only
`class_field` touches the class axes; `tasks`/`frontmatter` need a reparse),
whole-cache rebuild (landed), keep the old manual `traces index`.
2. **Parser/format-version policy.** The fingerprint covers config only.
Persisted `Note` rows also depend on parser behaviour, guarded solely by the
hand-bumped `META_VERSION = b"1"` (`src/index/store.rs:41`); a version mismatch
wipes the store at `IndexStore::open`, not through `Drifted` as ADR-0009 text
implies. [H] a parser change shipped without a manual bump serves stale `Note`
rows. Decide whether a parser/schema identity enters the fingerprint or the
version is derived.
3. **Path-set change cliff.** Unchanged by the merge ([V]
`reconcile`/`is_paths_unchanged`/`merge_refreshed_notes` untouched, now
`src/index/refresh.rs:410,475,555`). `research/11` shows 97% of the cost is note
deserialization, not link re-resolution; decide note caching/laziness vs
accepting it. The inverted unresolved-stem index suggested by `research/05`
attacks the wrong cost.
4. **Racy-timestamp guard.** [V] still absent: `delta.rs` untouched and no
`racy` handling anywhere in `src/index`; a same-length edit within one mtime
tick is missed (`service.rs` tests sleep to let mtime advance). Decide whether a
content-hash fallback belongs in the model.
5. **Observability of heals.** [V] `RefreshReport` is only logged in production
(`tracing::info!` for drift, `debug!` for file changes; count accessors are
test-only). Decide whether a heal is user-visible (links to the host and
error/diagnostic decisions).

Facts: `docs/adr/0009-index-refresh.md` (status still `proposed`),
`src/index/fingerprint.rs`, `src/index/refresh.rs`, `src/index/service.rs`,
`src/index/store.rs`, `tests/e2e/index.rs`
(`config_edit_heals_the_index_on_the_next_query`), `survey/index.md` H2/H3 (H3
is what landed), `research/05`.

Adversarial probes [H]: fingerprint is deliberately wider than consumed
(`title`, `aliases`, date keys) so harmless edits trigger a full rebuild; a
drift on a cold query blocks that query behind a full parse; an unwritable store
re-heals on every query.

Measured (`research/11`): at 5k notes the path-change cliff is 97% note
deserialization (18.5 ms) and 2.7% link re-resolution (0.5 ms).

Inputs: `research/05`, `research/11` (#2), config-ownership decision (residual
digest surface), read-model decision.

## Inputs from the value model decision

See [ticket 14's Answer](./14-value-model.md). The runtime value's serde shape
changes (`Number` becomes `Int`/`Float`, `Link` gains `embedded` and
normalized display), so the cache format and parser version must invalidate
existing indexes.

## Inputs from the coercion and comparison decision

See [ticket 15's Answer](./15-coercion-and-comparison-policy.md). The cache
format change (civil datetimes, `Int`/`Float`, normalized `Link`) must
invalidate existing indexes. A host zone change need not invalidate the index,
since naive datetimes no longer embed it.

## Inputs from the schema typing decision

See [ticket 16's Answer](./16-schema-note-typing-and-validation.md). Schema
files, including their class `[binding]` tables and the `global` layer, stay
out of the fingerprint: bindings are expanded at query time.
