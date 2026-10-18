# Research: performance evidence for read-model decisions

Type: research
Status: resolved

## Question

Several upcoming decisions (note document model, index read model, query core, ambient-environment cost of zone lookups) hinge on measured facts the surveys only inferred. Establish, with numbers and provenance, where time and memory actually go. **Do not modify production code, tests, benches, or docs.** Methodology, in order of preference:

1. Mine existing recorded results first (`target/criterion/**` if present, `docs/traces-benchmarks-digest.txt`, `docs/superpowers/plans/2026-10-18-criterion-store.md`, `lcov`/bench READMEs under `benches/`). State the commit/date of each datum.
2. Only for gaps, run existing benches sequentially (never in parallel; release profile; no `RUSTFLAGS`/`target-cpu=native`; see AGENTS.md benchmarking rules) via the project's mise `bench` task in `--mode quick`, one module at a time. Do not touch `.worktrees/criterion-store`. Report any file the run created or changed in the repo (there should be none outside `target/`); revert nothing that is not yours.
3. For end-to-end CLI cold/warm timing, generate a throwaway synthetic vault OUTSIDE the repo (e.g. under `/tmp`) at roughly 1K / 5K / 20K notes with realistic frontmatter, tags, wikilinks, and task lists, and time the release binary for: first `index`, no-change refresh, one-file-changed refresh, a selective `--from "#tag"` query, an unfiltered query, and a `--sort` on a date field. Delete the vault afterwards.

Quantities needed (report each as a table with N and confidence; say `UNMEASURED` rather than guessing):

- Cold index build and incremental refresh at the three sizes; share spent in directory scan, `stat`, note parse, inlink derivation, redb write, redb read-all.
- Per-note parse cost and its split (pulldown-cmark pass 1, the second heading-collection pass, list-item lexing, frontmatter YAML conversion). If not separable from existing benches, state what a micro-measurement would need; do not add one to the repo.
- In-memory linear `QueryService::run` vs store-backed `run_from_store` for selective and unselective sources at each size; peak memory in each mode (`benches/memory_footprint.rs`).
- Sorting by a date/datetime/mixed-kind field: cost attributable to repeated ambient `local_zone()` lookups or per-comparison date promotion vs precomputed keys.
- `QueryRow`/`QuerySet` memory per row for list/task queries; template `query` chain clone cost (`benches/template_render.rs`).
- Fixed floors that bound any redesign (redb commit floor, process start, config load/trust check).

## Output

Write `.scratch/arch-redesign/research/03-performance-evidence.md` (tables, provenance per number, a short "what these numbers do and do not support" section listing which survey performance hypotheses are supported, contradicted, or still unmeasured). On finishing, append an `## Answer` section to THIS ticket file (5-8 lines: the three facts most likely to change a design decision) and set `Status: resolved`. Do NOT edit `map.md`.

## Answer

Detailed findings and provenance are documented in `.scratch/arch-redesign/research/03-performance-evidence.md`. The three most decision-altering facts:
1. **redb transaction and open floors dominate I/O latency**: Write commit fsync has a rigid ~36.8 ms floor on macOS APFS (50 notes takes 36.8 ms; 1K takes 37.7 ms), and table open has a ~16.0 ms floor. Cold CLI commands cannot reach sub-10ms latency unless hot paths bypass synchronous redb transactions.
2. **Dual query models create an extreme efficiency chasm**: In-memory `QueryService::run` takes 3.9 µs (48 bytes/row), while store-backed `run_from_store` takes 16–71 ms and allocates 21.2 MB across 129,495 allocations for 1 matching row on 20K notes to assemble a transient `WorkspaceIndex`. Redesigning around a single in-memory read model eliminates this massive allocation penalty.
3. **Ambient temporal lookups hit filter evaluation, not sorting**: Sort keys are precomputed once per row upfront, resulting in zero `local_zone()` calls during the $O(N \log N)$ sort loop (20K date sort takes 565 µs, comparable to bare scalar sorting). However, filter evaluation (`eom`, date arithmetic) invokes `local_zone()` per row, introducing 2.2x–3.2x overhead (2.2–2.7 ms for 20K notes) that strongly supports threading an explicit execution context.
