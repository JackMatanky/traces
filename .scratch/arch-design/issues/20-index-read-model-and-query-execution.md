# What is the index read model, and is there one query execution path or two?

Type: grilling
Status: open
Blocked by: 11, 13, 19, 1

## Question

`QueryService` has two execution models: in-memory linear scan over
`WorkspaceIndex` and redb secondary-table pushdown (`run_from_store`) that
fabricates a partial `WorkspaceIndex` just to build rows; redb maintains four
multimap tables solely for the latter. Decide: one execution path or two; the
shape of the read model (immutable all-entries snapshot, lazy store-backed view,
columnar facts); what a `QueryRow` borrows from; what the store persists (whole
`Note` via postcard vs extracted facts); and whether `WorkspaceIndex` totality
is a type-enforced invariant.

Facts: `survey/index.md` §2-3 + H1/H7, `survey/query.md` §3,
`src/query/service.rs:129-169,323,395`, `src/index/entry.rs:80-103`,
`SortedByPath::assumed_sorted` debug-only check (`src/index/sort.rs:48-57`),
commit floor ~34ms.

Alternatives: pure in-memory snapshot (drop 4 tables); keep dual path but
formalise a pushdown trait; store-backed lazy rows; keep as is. Decide only
after performance evidence.

Measured (`research/11`, M4 Pro): 20k-note cold query 60 ms (store pushdown) vs
74 ms (load-all); peak RSS 17 MB vs 39 MB; the redb ~25 ms commit floor is
open/close lifecycle, not durability. Both paths are fast; the contested
resource is memory and code (four tables + `SourceResolver` + fabricated partial
index).

Re-verified (main @ `207cc3fb`): `src/query/` and `src/index/entry.rs` are
untouched, so the fabricated partial `WorkspaceIndex` and `Not` full scan stand.
The cold path now also reads the fingerprint in the plan's existing read
transaction and, on drift, rebuilds fully in memory before serving.

Inputs: `research/11` (go/no-go evidence), `research/05`, `research/06`, note
document model, forward-consumer decision (cold CLI vs long-lived host).

Graduates fog: index internals (CandidateTrie/indextree, store engine/repository
split).

## Inputs from the value model decision

See [ticket 14's Answer](./14-value-model.md). Postcard encodes `Date`,
`DateTime` and `Duration` as text and re-parses them on every cache load; a
compact binary form (via `is_human_readable()`) is a candidate, unmeasured.

## Inputs from the coercion and comparison decision

See [ticket 15's Answer](./15-coercion-and-comparison-policy.md). Naive
datetimes are persisted as civil values, so the cache holds no machine zone and
its format changes. Collation sort keys are never persisted. Any secondary-table
key order must agree with the filter relations (Int and Float compare by
value; links by folded target).
