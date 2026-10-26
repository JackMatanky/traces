# 01: Surface workflows, behavioral discovery

Type: research
Status: resolved

## Question

Which observable behaviors of the surface layer constitute *intentional
guarantees* of the system, as opposed to incidental implementation behavior?

**Scope**: `src/cli/` (command routing, clap argument handling, miette
diagnostic and error presentation, table/output formatting), `src/dialog/` (TTY
vs automated prompt seam, defaults and fallbacks), `src/config/` (two-level TOML
discovery, trust verification, config tracking).

**Method**: read source and inline tests breadth-first. Read `tests/e2e/`
(config, env_policy, init, process_contract, completions, workflow) and
`tests/integration/` where surface behavior leaks through. Read the glossaries:
`src/cli/GLOSSARY.md`, `src/config/GLOSSARY.md`, `src/dialog/GLOSSARY.md`. Read
`docs/adr/0002`, `docs/adr/0003`, `docs/adr/0004`, and any `//! Proves` /
`Defect class:` headers in this scope. Treat docs and existing tests as evidence
to interrogate, not authority.

**Deliverable**: a candidate-guarantee inventory in
`.scratch/bdd-testing/research/01-surface-workflows.md`, following the citation
convention of the existing files in this directory. One entry per guarantee:

- guarantee stated as an observable contract (inputs, observable outcomes, error
  semantics, edge cases)
- class: invariant / state transition / workflow / error semantics / edge case
- verdict: intentional / incidental / uncertain, with evidence (file:line for
  tests, glossary, ADR, or source)
- pinned today or unpinned (does an existing test already enforce it?)
- Gherkin-suitability: good scenario candidate, unit-test material, or poor fit
  (say why)

Also record behaviors that look **untestable or hostile to Gherkin** (timing,
TTY interactivity, environment mutation) so later tickets inherit the
constraint. Max ~400 lines; prefer completeness of the guarantee list over
prose.

## Answer

Findings:
[../research/01-surface-workflows.md](../research/01-surface-workflows.md) (402
lines, source-cited; spot-verified during resolution).

- **Guarantees inventoried** for cli/dialog/config: exit-code table (0/1/2,
  never 101) and stdout/stderr split pinned e2e; config failure-stage table (4
  codes) pinned; four Trust Statuses pinned; `init` transactionality pinned;
  non-TTY dialog fallback pinned.
- **Gaps**: exit 130 not pinned at process level (only in-process unit test);
  "init does not grant trust" unpinned; output-collision refusal half only
  pinned.
- **Doc contradictions**: `src/cli/mod.rs:54` exit-0 claim vs code 130; ADR-0002
  stale trust semantics (moved-dir keying, existence-only check); ADR-0003 cites
  three tests that don't exist.
- **Gherkin-hostile**: TTY tests self-skip under TTY, process-global env/cwd
  (one process per env value), miette line-wrapping (assert codes not text),
  timing-based watchdog, canonicalization `/tmp`→`/private/tmp`.
- Feeds tickets 07 (suitability), 08 (seam visibility), 11 (env fixture design),
  13 (which tests stay).
