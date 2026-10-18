# What is the index read model, and is there one query execution path or two?

Type: grilling
Status: open
Blocked by: 11, 13, 19, 1

## Question
`QueryService` has two execution models: in-memory linear scan over `WorkspaceIndex` and redb secondary-table pushdown (`run_from_store`) that fabricates a partial `WorkspaceIndex` just to build rows; redb maintains four multimap tables solely for the latter. Decide: one execution path or two; the shape of the read model (immutable all-entries snapshot, lazy store-backed view, columnar facts); what a `QueryRow` borrows from; what the store persists (whole `Note` via postcard vs extracted facts); and whether `WorkspaceIndex` totality is a type-enforced invariant.

Facts: `survey/index.md` §2-3 + H1/H7, `survey/query.md` §3, `src/query/service.rs:129-169,323,395`, `src/index/entry.rs:80-103`, `SortedByPath::assumed_sorted` debug-only check (`src/index/sort.rs:48-57`), commit floor ~34ms.
Alternatives: pure in-memory snapshot (drop 4 tables); keep dual path but formalise a pushdown trait; store-backed lazy rows; keep as is. Decide only after performance evidence.
Measured (`research/11`, M4 Pro): 20k-note cold query 60 ms (store pushdown) vs 74 ms (load-all); peak RSS 17 MB vs 39 MB; the redb ~25 ms commit floor is open/close lifecycle, not durability. Both paths are fast; the contested resource is memory and code (four tables + `SourceResolver` + fabricated partial index).
Inputs: `research/11` (go/no-go evidence), `research/05`, `research/06`, note document model, forward-consumer decision (cold CLI vs long-lived host).
Graduates fog: index internals (CandidateTrie/indextree, store engine/repository split).
