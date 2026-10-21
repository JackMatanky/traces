# What do the existing benchmarks and targeted new measurements say about the consequential performance claims?

Type: research
Status: resolved
Blocked by: none

## Question

Several candidate decisions rest on performance claims that are currently reasoned, not measured (`survey/index.md` H1/H2/H4, `survey/query.md` H1/H5, `survey/note.md` H2/H3/H5, `survey/template.md` H3, `survey/root-primitives.md` H1 enum size). Produce evidence, using the repository's Criterion benches (`benches/*.rs`, run via `mise run bench -- ...`; NEVER run benches in parallel, release mode only, no RUSTFLAGS) and, where no bench covers a claim, a minimal throwaway bench under a temp dir (not committed):

1. Cold CLI point query (`traces list "#rare_tag"`-equivalent) at 1k / 5k / 20k notes: `run_from_store` (redb secondary tables) vs "load all notes into `WorkspaceIndex` and linear scan". Report latency and peak RSS.
2. Refresh latency: single-file edit vs single-file delete/add vs 100-file change at 5k notes; where the time goes on path-set change (note deserialization vs link resolution).
3. `CandidateTrie` vs flat nearest-candidate scan at 8/64/128/512 same-stem candidates.
4. Note parse cost split: second pulldown-cmark pass share; `ListItem` size and per-task String allocations at 10k tasks.
5. Frontmatter parse cost split across `noyalib::Value -> FieldValueRef -> NoteFieldValue` conversions; sizes of `FieldValue`/`NoteFieldValue` and a hypothetical unified enum if estimable by size_of probes.
6. (WITHDRAWN: `research/01` refuted the claim that chained `QuerySet` methods clone rows; skip.)
7. Redb commit-floor (~34 ms claim, `benches/index_store.rs:113`): confirm and identify what dominates.

Record machine, commands, raw numbers (table), and state which claims are confirmed, refuted, or inconclusive. Existing baselines in `target/criterion` or the mise criterion store (`docs/superpowers/plans/2026-10-18-criterion-store.md`) may be reused if current.

Deliverable: `.scratch/arch-design/research/11-performance-evidence.md` (tables + verdicts). Do not modify `src/` or committed benches.

## Resolution

Empirical performance measurements across all 7 research questions have been completed and recorded in [.scratch/arch-design/research/11-performance-evidence.md](../research/11-performance-evidence.md).

Key findings affecting architectural decisions:

1. **Cold CLI Point Query**: `run_from_store` saves 21 MB RSS (17 MB vs 39 MB, 2.23x reduction) and 14 ms latency (60 ms vs 74 ms) at 20,000 notes. Cold in-memory scan is fast enough (< 75 ms), but secondary tables prevent doubling peak CLI memory on large vaults.
2. **Refresh Path-Set Cliff**: Path-set changes at 5k notes spend 18.5 ms (97.3%) deserializing notes from redb; link re-resolution takes only 0.52 ms (2.7%). Inverted stem index for link resolution is unjustified; caching note records is where the leverage lies.
3. **CandidateTrie Elimination**: Flat LCA scan is ~10 µs at 64 candidates and ~20 µs at 128 candidates. Trie build costs 48–87 µs, requiring $\ge 5$ queries to the same ambiguous stem to break even. Confirmed for elimination (~500 LoC and `indextree` dependency removed).
4. **Note Parser Fused Passes & Task Interning**: Second pulldown-cmark pass accounts for 20% to 38% of parse time across all notes. Fusing saves 20–35% parse CPU. Storing interned task statuses removes 10,000 `String` allocations at 10k tasks and reduces `ListItem` from 160 bytes to ~128 bytes.
5. **Unified Value Enum**: A unified `Value` enum measures exactly 80 bytes (matching `NoteFieldValue`'s budget), eliminating duplicate AST conversions and visitors without layout expansion.
6. **Template QuerySet Laziness**: Refuted eager cloning claim; `QuerySet` is already lazy via `ExecutionPlan` and chaining costs < 1 µs without row cloning.
7. **Redb Commit Floor**: Confirmed ~25–34 ms floor magnitude, but refuted commit attribution; transaction commit with `Durability::None` is 18 µs; the floor is 99.6% redb file lifecycle open/create + drop overhead.

Re-verified (main @ `207cc3fb`): refresh and cold-query path unchanged within noise (A-B-A control, 44 benchmark IDs, median +0.7% vs 1.7% noise floor). See `reverification.md`.
