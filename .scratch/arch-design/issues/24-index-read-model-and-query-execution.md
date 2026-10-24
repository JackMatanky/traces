# What is the index read model, and is there one query execution path or two?

Type: grilling
Status: open
Blocked by: 11, 17, 21, 23, 1, 12

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

See [ticket 18's Answer](./18-value-model.md). Postcard encodes `Date`,
`DateTime` and `Duration` as text and re-parses them on every cache load; a
compact binary form (via `is_human_readable()`) is a candidate, unmeasured.

## Inputs from the coercion and comparison decision

See [ticket 19's Answer](./19-coercion-and-comparison-policy.md). Naive
datetimes are persisted as civil values, so the cache holds no machine zone and
its format changes. Collation sort keys are never persisted. If a secondary
table pushes down a filter, its candidates must agree with the filter relation
(numeric value for Int/Float, folded targets for links); the deterministic total
sort order does not substitute for period membership or the field-versus-field
civil-midnight rule.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). Class
binding by tag and path is expanded at query time into the existing `Class`,
`Tag` and `Path` atoms, so the index stores no derived class and learns nothing
about schemas. A "has a known explicit class" exclusion is needed in source
selection; a secondary lookup for it would avoid the `Not` full scan.

A "notes changed since the last check" notice (20, agent guardrail) would read
the changed set that refresh already computes; any baseline store sits outside
the index and its fingerprint. Ownership of that store is open here and in
tickets 25 and 31.

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md). IWE is the
measured pure in-memory alternative: no persistent cache, the CLI parses the
whole vault with Rayon on every run. Synthetic ~700 B notes on an M3 Pro: 5k 144
ms, 10k 329 ms, 20k 705 ms cold load (plain; +36% with its BM25 index) [V:
`docs/benchmark.md:155-171`]. Queries then cost 12 ms for a field filter at 20k
and are super-linear for graph filters (`--roots` 60 ms; the unbounded
included-by filter reaches ~94% of the corpus) [V: `docs/benchmark.md:185-208`].
Servers load once and update in place (`Graph::update_document`). Different
machine and corpus from ticket 11 (M4 Pro, 20k cold query 60 to 74 ms), so use
this as a cost model for the keep-as-is comparison, not as a head-to-head. Its
evaluator partitions an `and` into graph-walk generators and scalar predicates
and intersects candidate key sets before running scalar predicates
(`crates/liwe/src/query/eval.rs`, `eval_and`) [V], a candidate-set-first plan
that needs no store.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md).
Whichever read path wins, a query starts from one coherent index generation and
retains its pinned temporal frame across lazy materialization. A refresh from a
different policy must never silently mix stored rows from different generations;
the actual read/store boundary is for this ticket and 25. A root's index is not
isolated merely because two hosts have different in-memory workspace identity.
Absence of filesystem creation time remains `Null`, not a modification-time
substitute. Persist source timestamps, not zone-derived `cdate`/`mdate` civil
projections under a zone-free fingerprint; filename periods remain
zone-independent whether persisted or parsed when queried.
