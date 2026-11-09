# What do the existing benchmarks and targeted new measurements say about the consequential performance claims?

Type: research
Status: resolved
Blocked by: none

## Question

Several candidate decisions rest on performance claims that are reasoned, not
measured. The claims live in `survey/index.md` H1/H2/H4, `survey/query.md`
h1/h5, `survey/note.md` h2/h3/h5, `survey/template.md` h3, and
`survey/root-primitives.md` h1 enum size. produce evidence. use the repository's
criterion benches (`benches/*.rs`, run with `mise run bench -- ...`; never run
benches in parallel, release mode only, no rustflags). where no bench covers a
claim, write a small temporary bench under a temp dir (not committed):

1. cold cli point query (`traces list "#rare_tag"`-equivalent) at 1k / 5k / 20k
   notes. compare `run_from_store` (redb secondary tables) vs "load all notes
   into `workspaceindex` and linear scan". report latency and peak rss.
2. refresh latency: single-file edit vs single-file delete/add vs 100-file
   change at 5k notes. report where the time goes on path-set change (note
   deserialization vs link resolution).
3. `candidatetrie` vs flat nearest-candidate scan at 8/64/128/512 same-stem
   candidates.
4. note parse cost split: second pulldown-cmark pass share; `listitem` size and
   per-task string allocations at 10k tasks.
5. frontmatter parse cost split across `noyalib::value -> fieldvalueref ->
   notefieldvalue` conversions. also measure sizes of
   `fieldvalue`/`notefieldvalue` and a hypothetical unified enum if size_of
   probes can estimate it.
6. (withdrawn: `research/01` refuted the claim that chained `queryset` methods
   clone rows; skip.)
7. redb commit-floor (~34 ms claim, `benches/index_store.rs:113`): confirm and
   identify what dominates.

record machine, commands, raw numbers (table), and state which claims are
confirmed, refuted, or inconclusive. existing baselines in `target/criterion` or
the mise criterion store
(`docs/superpowers/plans/2026-10-18-criterion-store.md`) may be reused if
current.

deliverable: `.scratch/arch-design/research/11-performance-evidence.md` (tables

- verdicts). do not change `src/` or existing benches.

## resolution

all 7 research questions are now measured and recorded in
[.scratch/arch-design/research/11-performance-evidence.md](../research/11-performance-evidence.md).

key findings affecting architectural decisions:

1. **cold cli point query**: `run_from_store` saves 21 mb rss (17 mb vs 39 mb,
   2.23x reduction) at 20,000 notes. it also saves 14 ms latency (60 ms vs 74
   ms). cold in-memory scan is fast enough (< 75 ms). secondary tables prevent
   doubling peak cli memory on large vaults.
2. **refresh path-set cliff**: path-set changes at 5k notes spend 18.5 ms
   (97.3%) deserializing notes from redb. link re-resolution takes only 0.52 ms
   (2.7%). the inverted stem index for link resolution is unjustified. caching
   note records is the biggest gain.
3. **candidatetrie elimination**: flat lca scan is ~10 µs at 64 candidates and
   ~20 µs at 128 candidates. trie build costs 48–87 µs and needs $\ge 5$ queries
   to the same ambiguous stem to break even. confirmed for elimination (~500 loc
   and `indextree` dependency removed).
4. **note parser fused passes & task interning**: second pulldown-cmark pass
   accounts for 20% to 38% of parse time across all notes. fusing saves 20–35%
   parse cpu. storing interned task statuses removes 10,000 `string` allocations
   at 10k tasks. it also reduces `listitem` from 160 bytes to ~128 bytes.
5. **unified value enum**: a unified `value` enum measures exactly 80 bytes
   (matching `notefieldvalue`'s budget). it removes duplicate ast conversions
   and visitors without layout expansion.
6. **template queryset**: the eager cloning claim is refuted. `queryset` is
   already lazy through `executionplan`. chaining costs < 1 µs without row
   cloning. 7. **redb commit floor**: confirmed ~25–34 ms floor magnitude, but
   refuted commit attribution. transaction commit with `durability::none` is 18
   µs. the floor is 99.6% redb file lifecycle open/create + drop overhead.

re-checked (main @ `207cc3fb`): refresh and cold-query path unchanged within
noise (a-b-a control, 44 benchmark ids, median +0.7% vs 1.7% noise floor). see
`reverification.md`.
