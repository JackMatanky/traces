# Ambient environment: explicit execution context or process statics?

Type: grilling
Blocked by: 01, 03, 05

## Question

Traces reaches ambient process state from many places: the local time zone and "now" (`temporal::local_zone()` — reportedly called per temporal operation in query filters, sort promotion, template date helpers, and generic `NoteFieldValueRef` cross-kind comparison), environment variables and platform directories via `LazyLock` statics (`env_vars.rs`, `dirs.rs`), the current directory (wrapped by `cli::Cwd`, enforced by `clippy.toml`), and global YAML parser config. Tests compensate with process-wide guards (`ZoneGuard`, `CwdGuard`). A long-running host with several workspaces (the planned LSP) and any reproducible-evaluation requirement conflict with process-global ambient state. Decide: does the redesign introduce an **explicit execution/environment context** (clock, zone, env policy, directories, cwd) passed from the composition root, keep ambient reads behind narrow documented seams, or keep the status quo — and for which of these concerns?

## Evidence status

Survey leads (UNVERIFIED until ticket [Verify core findings](01-research-verify-core-findings.md) lands; its §3/§7 ledger is authoritative): [temporal §9.1, §11.1–2](../survey/temporal.md), [config/trust §11.6](../survey/config-trust.md), [root primitives §4](../survey/root-primitives.md). The Oct-16 `.design` review described a query-captured zone that the temporal survey reports was removed on `main`; do not rely on either until verified. Ticket [Performance evidence](03-research-performance-evidence.md) supplies the cost of per-comparison zone lookups.

**Verified (ledgers landed 2026-10-18)** — [01 §3, §7](../research/01-verify-core-findings.md): `QuerySet` holds no `TimeZone` and `compare_temporal` does not exist on `main` (the Oct-16 `.design` description is obsolete). 24 direct non-test `local_zone()` sites: `temporal` (4: `datetime.rs:128,192,342`, `point.rs:780`), `query/grammar/filter.rs` (9), `template/engine/date.rs` (11); indirect: `file.rs:97`, `note/field.rs:297,299` (cross-kind compare), `query/sort.rs:376`. Ambient `Timestamp::now()`: `template/engine/date.rs:162,1010,1026`. Process statics: `dirs.rs` (HOME, CONFIG_HOME, STATE_HOME, TRACES_STATE_DIR, TRACKED/TRUSTED_CONFIGS), `env_vars.rs`, `yaml.rs`, temporal lookups; the only `std::env` reads outside `cli` are `dirs.rs:193` and `env_vars.rs:9,28`. Test overrides: `HOME` by compile-time `cfg(test)`; `CONFIG_HOME`/`STATE_HOME`/`CEILING_DIRS`/`IGNORED_DIRS` have **no** runtime override; `ZoneGuard` needed in five suites. Cost ([03 §6](../research/03-performance-evidence.md)): sort precomputes keys once per row (20K rows ≈ 565 µs, within 8% of a synthetic floor), so ambient zone lookups are *not* a sort-loop cost; filter calendar evaluation pays ambient lookups per row (0.83 ms plain compare up to 2.7 ms for period functions at 20K rows). Open nuance: ledger 01 says cross-kind `NoteFieldValueRef::compare` can reach `local_zone()` per comparison in tie-break paths, which ledger 03 did not separately measure.

## Alternatives (not exhaustive)

- Context struct (`now`, `zone`, directories, env policy) created once by the composition root and threaded explicitly (cost: signature churn; benefit: determinism, no global test guards).
- Narrow capability seams (a `Clock`, a `ZoneProvider`) injected only where used.
- Keep ambient reads but centralize each behind exactly one accessor with documented semantics, and keep guard-based tests.
- Hybrid: explicit for evaluation semantics that affect results (zone/now), ambient for process-lifetime facts that cannot change (home/state directory).

## Done when

The answer classifies each ambient dependency (explicit vs ambient, with rationale), states the shape of any context type and where it is created and consumed, how it affects the observable semantics of date comparison and `now`, what test infrastructure it removes, and what it implies for the workspace host ticket [Workspace host](17-workspace-host-and-composition.md) and the date-format fog item.
