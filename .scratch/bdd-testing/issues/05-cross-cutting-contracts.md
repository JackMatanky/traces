# 05: Cross-cutting contracts, behavioral discovery

Type: research
Status: resolved

## Question

Which observable behaviors of the root-level modules and the `test-utils` public surface constitute *intentional guarantees*, as opposed to incidental implementation behavior?

**Scope**: root `src/*.rs` modules (`field`, `task`, `file_tracker`, `path`, `delimiter`, `dirtree`, `hash`, `lexer`, `yaml`, `tag`, `position`, `env_vars`, `strsim`, `file_class_expander`), the `pub mod testing` surface gated by `feature = "test-utils"` (`src/lib.rs`, "exactly the 46 identifiers"), and cross-cutting error semantics (miette diagnostic conventions, exit codes, `Defect class:` taxonomy).

**Method**: read source and inline tests breadth-first (the root modules carry ~269 `#[test]`s; `yaml.rs` notably has zero). Read `src/GLOSSARY.md` and the relevant `docs/adr/` (0004 error presentation especially). Read the `tests/` conventions (`//! Proves` headers, `Executes vs asserts` blocks, `Defect class:` tags) and the `docs/agents/` docs that describe these contracts. The `test-utils` surface matters twice: it is both a behavioral contract and the seam through which ticket 08's layer boundaries will run.

**Deliverable**: candidate-guarantee inventory in `.scratch/bdd-testing/research/05-cross-cutting-contracts.md`, same entry format and citation convention as ticket 01. Explicitly inventory what the 46 `test-utils` identifiers expose and whether each is a guarantee or a test convenience. Max ~400 lines.

## Answer

Findings: [../research/05-cross-cutting-contracts.md](../research/05-cross-cutting-contracts.md) (397 lines, source-cited; `src/lib.rs:182` "exactly the 46" spot-verified).

- **`test-utils` inventory**: actually **47 identifiers**, not 46 as `src/lib.rs:182` claims (baseline `tests/baselines/public-api-test-utils.txt` and `check:visibility` enforce 47). Split: 32 guarantees (production types re-exported) vs 15 test conveniences (`TestProject`, `write_note`, fixtures, `ZoneGuard`).
- **Cross-cutting guarantees**: exit codes 0/1/2/130 never 101 (130 pinned in-process only; no signal e2e), stdout/stderr stream split, stable `traces::cli::*` codes (config family pinned), typed-thiserror-only domain modules (structurally pinned, but `src/query/error.rs` derives `Diagnostic` contrary to ADR-0004:67), `test-utils` frozen seam (machine-baseline-pinned), layer discipline (`check:layers`), root confinement (TOCTOU window documented, untestable), YAML config parity (partially pinned), never-101 pinned twice (clippy + per-e2e assertion).
- **New finding**: `Defect class:` taxonomy is 16 free-form drifting tags with no canonical definition; `//! Proves`/`Executes vs asserts` are undocumented conventions (33-34 uses).
- **Doc contradictions**: lib.rs count off-by-one; root-module test count is 278 not ~269; ADR-0004 promises a nonexistent Ctrl-C process test; four `yaml.rs` doc claims untested.
- Feeds tickets 07 (seam = `test-utils`), 08 (layer gates evidence), 10 (fixture design), 11 (taxonomy feeds quality rubric), fog item (`Defect class` fate).
