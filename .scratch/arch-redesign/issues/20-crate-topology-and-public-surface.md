# Crate topology, namespace layout, and public surface

Type: grilling
Blocked by: 11, 17, 18, 19, 08

## Question

Traces is one crate (`traces`) with 25 top-level modules, a minimal `pub` surface (about 14 items), roughly 58 `pub(crate)` re-exports, and a frozen `traces::testing` module of 45 identifiers behind the `test-utils` feature, which every integration test and benchmark requires; `lib.rs` is 900 lines, including a 669-line `test_support` module (`TestProject` is 266 lines of it). Several leaf modules sit at the crate root (`strsim`, `yaml`, `dirs`, `env_vars`, `delimiter`, `lexer`, `position`, `dirtree`, `hash`, `file_tracker`); the ledger shows most of them have two or more consumers. Decide the **topology**: one crate vs a workspace of crates, where each module/primitive lives (by owning consumer vs shared foundation), what the public surface is, how integration tests and benches reach internals without a side-door, and how the dependency direction rules established by [Value model](09-value-model.md) through [Errors and diagnostics](19-error-and-diagnostic-architecture.md) are enforced mechanically (crate boundaries, visibility, `mise` module tasks).

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify system findings](02-research-verify-system-findings.md) §7 lands): [cross-cutting §5, §9–§11](../survey/cross-cutting.md), [root primitives §1, §6, §11](../survey/root-primitives.md). A workspace split is an option, not a presumption; the md-pkm-lsp map (ticket 34) treats a workspace conversion and a second binary as live questions for *its* needs.

**Verified (ledgers landed 2026-10-18)** — [02 §7](../research/02-verify-system-findings.md): the recomputed production dependency matrix is a strict DAG with **0 cycles**; the survey missed `config → temporal` and `index → env_vars`; upward edges are `note → config`, `index → config`, `template → query`, `template → schema`, `query → index`. `traces::testing` has exactly **45** identifiers (`lib.rs:184-209`), used by `tests/integration/*` and all 11 benches. `TestProject` is **266 lines** (`lib.rs:279-544`); the whole `test_support` module is 669 lines (the survey's "600+" conflated the two). The public root exports only leaf models; every service is internal, so any external composition needs `test-utils`. [01 §8](../research/01-verify-core-findings.md) single-consumer census: the ledger's "strictly single-consumer" verdict for `delimiter` and `lexer` is **corrected in the ledger's errata**: `delimiter` is consumed by `note` and `task` (`task.rs:32,227`), `lexer::lex_prefix` by `query::grammar`, `temporal` (`point.rs:147`, `duration.rs:270`) and `note::parser::inline` (`inline.rs:235`), so neither is single-consumer; `dirs` is `config` + `file_tracker`; `strsim` (query, schema), `yaml` (note, template), `env_vars` (config, index), `hash` (config, file_tracker), `position` (note, query, lexer) are multi-consumer; `dirtree` has five consumers. Several "junk drawer" candidates are therefore shared foundations, not misplaced single-owner modules.

## Alternatives (not exhaustive)

- Stay single-crate; re-home single-consumer leaves into their owners; keep a smaller, principled test surface.
- Workspace split along the seams the earlier decisions establish (e.g. foundation/value, note, index, query, template, host, cli), with crate-boundary enforcement of dependency direction.
- A library crate with a stable small public API plus a separately versioned/feature-gated internal crate for tests/benches.
- Visibility-only enforcement (`pub(crate)` + module-tree checks via `mise run modules:*`) vs compile-time crate boundaries; build-time and incremental-compile effects count.

## Done when

The answer gives the crate/module map (what lives where and why), the allowed dependency edges and how they are enforced, the public API tiers, the test/bench access strategy that replaces the side-door, and an explicit statement of which earlier decisions it relies on.
