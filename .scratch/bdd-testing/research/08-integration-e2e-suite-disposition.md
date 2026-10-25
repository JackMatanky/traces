# Research 08: Integration and E2E suite disposition

This document gives evidence for [ticket
08](../issues/08-layer-responsibilities.md), decisions (b) and (d). It reviews
the completed work in `.scratch/integration-e2e-suite/` (`spec.md`,
`test-arch-audit.md`, `issues/01..19`) against current repository wiring and BDD
decisions from [ticket 06](../issues/06-framework-revalidation.md) and [ticket
07](../issues/07-boundary-tiering.md).

> **Superseded in part by ticket 08.** The recommendations in section 5 are
> input, not decisions. Ticket 08 chose to fold scenarios into the e2e binary
> only (not Option 2). It adopted the suite only in part (not "accepted and
> binding"). Read the Answer in the ticket. Also read
> [08-verify-suite-decisions.md](08-verify-suite-decisions.md) for the drift.

## 1. Per-decision disposition table

All 19 issues from `.scratch/integration-e2e-suite/` are resolved in the
repository today. Ticket 08 adopts their layer invariants and gates as binding.
It does not adopt their per-test classification.

| ID | Decision Gist | Repo Implementation Status | BDD Relevance | Reason |
|---|---|---|---|---|
| 01 | Visibility freeze baselines and check task | Implemented (`mise.toml:123`, `src/lib.rs:70`, `ci.yml:59`) | Compatible | Hard gate. BDD adoption must not change `public-api-default.txt`, `public-api-test-utils.txt`, or `pub-tokens.txt`. |
| 02 | Honesty: fix self-validating assertions and docs | Implemented (`src/cli/init.rs:73`, `tests/e2e/workflow.rs:1`) | Compatible | Aligns with BDD ethos: scenarios must execute what their text asserts. |
| 03 | Silent-green guard: required-features and CI non-empty | Implemented (`Cargo.toml:49`, `ci.yml:83-165`, `_default:81`) | Compatible | Any new BDD target must use `--no-tests fail`. It must declare `required-features` if it uses `test-utils`. |
| 04 | Config-change staleness investigation | Implemented (`tests/e2e/config.rs:180`) | Compatible | Scenarios that change config must trust the project again before index or query operations. |
| 05 | EPIPE and exit-101 contract decision record | Implemented (`tests/e2e/process_contract.rs:1`) | Compatible | Scenarios assert numeric exit codes 0, 1, 2, or 130; release aborts to 134 on panic, never 101. |
| 06 | Facade relocation to `testing` namespace | Implemented (`src/lib.rs:178-212`, `public-api-test-utils.txt`) | Compatible | Hard gate. Scenarios that use the facade seam can import only from `traces::testing` (frozen 47 symbols [corrected: lib.rs doc says 46, actual baseline has 47]). |
| 07 | Structural prerequisites: ConfigFixture, assertions | Implemented (`tests/e2e/config_fixture.rs:20`) | Compatible | Step definitions can reuse `ConfigFixture` for raw TOML and structured configuration setup. |
| 08 | E2E spawn-only invariant and layer boundary | Implemented (`mise.toml:162`, `tests/e2e/`) | Conflicts | Sharp tension. Scenarios that use the facade seam cannot live in `tests/e2e/`. Feature text must avoid `traces ::`. |
| 09 | Sandbox environment policy and accessors | Implemented (`tests/e2e/support.rs:8-30`, `tests/e2e/env_policy.rs`) | Compatible | Hard gate. Spawned scenarios must use the hermetic sandbox policy (`TZ=UTC`, sandbox-owned dirs). |
| 10 | Process contracts: exit codes, argv, streams | Implemented (`tests/e2e/process_contract.rs:1`) | Superseded-by-bdd-candidate | Process boundary contracts are primary Gherkin candidates. BDD scenarios can absorb or twin them. |
| 11 | Config capability: TOML to behavior, failure stages | Implemented (`tests/e2e/config.rs:1`) | Superseded-by-bdd-candidate | External configuration behavior is a primary Gherkin candidate. |
| 12 | Index refresh family: delta, path-set, divergence | Implemented (`tests/integration/refresh.rs:1`, `tests/e2e/index.rs:1`) | Orthogonal | Store-level persistence fidelity stays in integration; visible refresh deltas can earn scenarios. |
| 13 | Query and class expansion: parity and `@class` | Implemented (`tests/integration/query.rs:1`, `tests/e2e/query.rs:1`) | Compatible | Query language surface is a prime scenario candidate; parity differential remains integration-owned. |
| 14 | Template capability: overwrite, non-TTY, feedback | Implemented (`tests/e2e/template.rs:1`, `tests/integration/template_render.rs`) | Superseded-by-bdd-candidate | Template CLI workflows are primary Gherkin candidates. |
| 15 | Recompute duplication analysis and ledger | Implemented (`.scratch/integration-e2e-suite/issues/15`) | Orthogonal | Process model for ticket 12: retire duplicate tests via an explicit ledger, never by test counting. |
| 16 | Pruning per ledger | Implemented (`.scratch/integration-e2e-suite/issues/16`) | Orthogonal | Validates that redundant unit/integration twins can be pruned once outer coverage lands. |
| 17 | Fixture cleanup: arrange/trust split, Default gone | Implemented (`tests/e2e/config_fixture.rs`, `src/config/mod.rs`) | Compatible | `impl Default for ConfigService` was removed; fixtures declare scenario without ambient coupling. |
| 18 | Runner and CI phases | Implemented (`.mise/tasks/test/_default:98`, `ci.yml:83`) | Conflicts | Architectural fork for decision (b). Folding BDD needs 0 phase changes; a new binary needs phase 5. |
| 19 | Naming: capability trees after pruning | Implemented (`tests/e2e/`, `tests/integration/`) | Compatible | Feature files must mirror capability tree names (`features/query.feature`, `features/template.feature`). |

## 2. Current physical wiring

### Targets and manifests

- `Cargo.toml:49-52`: Declares `[[test]] name = "integration"` with
  `required-features = ["test-utils"]`.
- `Cargo.toml:32-35`: Declares `[[bin]] name = "traces"` with `path =
  "src/main.rs"`.
- `tests/integration.rs:1-13`: Single entry for integration target. Enforces
  `#![cfg(feature = "test-utils")]`. Mounts 4 submodules: `persistence.rs`,
  `query.rs`, `refresh.rs`, and `template_render.rs`.
- `tests/e2e.rs:1-31`: Single entry for the E2E target. Discovered automatically
  by Cargo. Mounts `support.rs`, `config_fixture.rs`, and 11 capability modules
  (`completions`, `config`, `env_policy`, `index`, `init`, `process_contract`,
  `query`, `template`, `tracked`, `trust`, `workflow`).

### Canonical runner phases and filtersets

The script `.mise/tasks/test/_default:98-122` coordinates four canonical phases. Each phase passes `--no-tests fail` (`_default:81`):

1. **Unit**: `cargo nextest run -p traces --no-default-features --lib --bin
   traces` (`_default:99`)
2. **Integration**: `cargo nextest run -p traces --features test-utils --test
   integration` (`_default:103`)
3. **E2E**: `cargo nextest run -p traces --no-default-features --test e2e`
   (`_default:108`)
4. **Doctests**: `cargo test --workspace --all-features --doc -- --quiet` via
`.mise/tasks/test/doc:43,83` (`_default:120`)

Variadic arguments (`usage_args`) filter nextest runs via
`parse_passthrough_tokens` (`_default:49-53`). When filters are present, `cargo
nextest list` checks matches per phase to skip empty phases without failure
(`_default:74-80`).

### Timeouts and CI matrix

- `.config/nextest.toml:6-19`: `slow-timeout = { period = "30s", terminate-after
  = 2 }`, `global-timeout = "20m"`, `status-level = "slow"`, `success-output =
  "never"`.
- `.github/workflows/ci.yml:83-165`: Four separate jobs run test phases:
  - `test-unit`: Ubuntu, runs `kind(lib)` and `binary(traces)`
    (`ci.yml:97-101`).
  - `test-integration`: Ubuntu, runs `binary(integration)` with `--features
    test-utils` (`ci.yml:117-118`).
  - `test-e2e`: Matrix across `[ubuntu-latest, macos-latest, windows-latest]`
    with `--no-default-features` (`ci.yml:127, 139-141`).
  - `test-doctests`: Ubuntu, requires regex `test result: ok. [1-9][0-9]*
    passed;` (`ci.yml:160-164`).

### Mechanical layer gates

Defined in `mise.toml` and verified in CI (`ci.yml:71-74`):

1. `check:visibility` (`mise.toml:123-146`):
   - Prohibits glob re-exports: `git grep -E 'pub use .*\*' src/`
     (`mise.toml:131`).
   - Verifies default API: `cargo public-api --color=never | diff -u
     tests/baselines/public-api-default.txt -` (`mise.toml:137`).
   - Verifies test-utils API: `cargo public-api --features test-utils
     --color=never | diff -u tests/baselines/public-api-test-utils.txt -`
     (`mise.toml:140`).
   - Verifies public token snapshot: `git grep -I -E
     '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+' src/ ... | diff -u
     tests/baselines/pub-tokens.txt -` (`mise.toml:143`).
2. `check:layers` (`mise.toml:148-190`):
   - E2E crate import ban: `git grep -n -E
     '(^|[^[:alnum:]_])traces[[:space:]]*(::|\{|;|[[:space:]]+as[[:space:]])'
     tests/e2e.rs tests/e2e/ ... | grep -v -E
     '["`].*traces[[:space:]]*::|^[[:space:]]*//'` (`mise.toml:162-166`).
   - E2E process confinement: `git grep -n -E
     'process[[:space:]]*(::|\{|[[:space:]]+as[[:space:]])' tests/e2e.rs
     tests/e2e/ ':!tests/e2e/support.rs'` (`mise.toml:168-173`).
   - Integration spawn ban: `git grep -n -E
     'process[[:space:]]*(::|\{|[[:space:]]+as[[:space:]])|CARGO_BIN_EXE'
     tests/integration.rs tests/integration/` (`mise.toml:175-180`).
   - Cwd change ban: `git grep -n -E 'set_current_dir' tests/`
     (`mise.toml:182-187`).

### Rule applicability across candidate BDD placements

| Enforcement / Constraint | (i) Inside `tests/integration/` | (ii) Inside `tests/e2e/` | (iii) New Top-Level Target `tests/bdd.rs` |
|---|---|---|---|
| Crate references (`traces::...`) | Allowed (imports `traces::testing`) | **Banned** by `check:layers` line 162 | Not scanned by current `check:layers` |
| Process spawn / `CARGO_BIN_EXE` | **Banned** by `check:layers` line 175 | Allowed only via `tests/e2e/support.rs` | Not scanned by current `check:layers` |
| `env::set_current_dir` | Banned across `tests/` | Banned across `tests/` | Banned across `tests/` |
| Feature text scan (`traces ::`) | Safe from line 162 | **Scanned**: unquoted `traces ::` fails | Safe (not scanned) |
| Target configuration | Uses existing `[[test]]` | Uses auto-discovered `e2e` target | Requires new `[[test]]` in `Cargo.toml` |
| Cargo features | Requires `--features test-utils` | Built with `--no-default-features` | Configurable (optional `required-features`) |
| Runner task (`_default`) | Unchanged (runs in integration phase) | Unchanged (runs in e2e phase) | Requires new phase 5 or runner rewrite |
| CI matrix (`ci.yml`) | Runs in `test-integration` (Linux) | Runs in `test-e2e` (Linux, macOS, Win) | Requires new CI step or job |

## 3. Measured test counts

Measured on 2026-10-25 with `cargo nextest list` and `cargo test --doc`:

| Layer / Target | Today Measured | Ticket 08 Text | Discrepancy & Cause |
|---|---|---|---|
| Inline unit (`src/lib.rs`) | 3,697 | 2,368 | +1,329 tests (+56%). Expanded parameter tables and unit gap tests. |
| Binary unit (`src/main.rs`) | 4 | (in unit) | +4 tests. CLI dispatch entry-point checks (`src/main.rs:188-218`). |
| Total Unit Phase | 3,701 | 2,368 | +1,333 tests. |
| Integration (`tests/integration/`) | 41 | ~101 (outer) | 41 tests across 4 seam modules. |
| E2E (`tests/e2e/`) | 62 | ~101 (outer) | 62 tests across 11 capability modules. |
| Total Outer Tests | 103 | ~101 | +2 tests. Very close to the ~101 baseline. |
| Doctests (`cargo test --doc`) | 86 | Not stated | 82 functional doctests + 4 compile-fail visibility probes (`src/lib.rs:70-108`). |
| Proptest blocks | 4 blocks | 4 blocks | Identical. 4 blocks in `src/temporal/` (12 total property tests). |
| Total Nextest Tests | 3,804 | 2,469 | +1,335 tests (+54% overall suite growth). |

## 4. Contradictions and tensions

1. **E2E spawn-only invariant vs Ticket 07 facade seam**: Ticket 07 decided
   option (b). Scenarios address the spawned process by default. Scenarios allow
   a facade seam (`traces::testing`) when an observable value-type cannot be
   spawned. But `check:layers` forbids `traces::` references in `tests/e2e/`
   (`mise.toml:162`). It also forbids `process::` and `CARGO_BIN_EXE` in
   `tests/integration/` (`mise.toml:175`). Neither target can host both drivers.
2. **`check:layers` pattern scanner matches `.feature` text**: The regex
   `(^|[^[:alnum:]_])traces[[:space:]]*(::|\{|;|[[:space:]]+as[[:space:]])`
   matches any tracked file in `tests/e2e/`. Plain Gherkin prose in
   `tests/e2e/**/*.feature` with unquoted phrases like `traces ::` will fail CI.
3. **Outer coverage duplication**: The `integration-e2e-suite` added
   process-boundary tests for exit codes, stream splits, argv parsing
   (`process_contract.rs`), and config failure stages (`config.rs`). Tickets 01
   and 05 identified these contracts as prime BDD candidates. Implementing BDD
   scenarios without retiring duplicate E2E tests doubles execution time.
4. **Hermeticity and harness access**: BDD steps that spawn processes must use
   `tests/e2e/support.rs::Sandbox`. Direct calls to `std::process::Command` fail
   both `check:layers` and the sandbox environment policy (`TZ=UTC`,
   sandbox-owned dirs).
5. **Phase count tension**: Ticket 06 assumed that folding BDD into existing
   binaries avoids runner changes. But folding into `integration` denies process
   spawning. Folding into `e2e` denies facade imports. Unifying BDD into one
   target requires adding a fifth phase to `.mise/tasks/test/_default` and CI.

## 5. Implications for ticket 08

### Decision (a): Classification rule

- **BDD scenarios (`rstest-bdd`)**: Observable user contracts at product
  boundary (CLI spawn), plus facade seam exceptions (`traces::testing`) meeting
  all six ticket 07 conditions.
- **Unit tests (`src/`)**: Single-module semantics, parser tables,
  combinatorics, error conversions, non-TTY paths. Checked by `mutarust`.
- **Integration (`tests/integration/`)**: Multi-component in-process pipelines
  through `traces::testing`. Process spawning is prohibited.
- **E2E (`tests/e2e/`)**: Low-level process mechanics (exact stdout/stderr
  streams, signals, completion scripts, corruption recovery).
- **Property tests (`proptest`)**: Mathematical invariants in `temporal/`.
- *Recommendation*: Adopt this classification rule.

### Decision (b): Physical placement of BDD scenarios

- **Option 1: Split into `tests/integration/` (facade) and `tests/e2e/`
  (spawn)**.
  - *Evidence for*: Zero changes to `Cargo.toml`, `.mise/tasks/test/_default`,
    or CI.
  - *Evidence against*: Splits feature files across two directory trees. Authors
    cannot write a feature file with both types of steps. Authors must navigate
    differing feature flags.
- **Option 2: Dedicated target `tests/bdd.rs` with its own runner phase**.
  - *Evidence for*: Consolidates all Gherkin specs. It supports both drivers. It
    avoids `check:layers` regex collisions. It isolates BDD nextest filtering
    (`-E 'binary(bdd)'`).
  - *Evidence against*: Requires adding `[[test]]` to `Cargo.toml`, updating
    `.mise/tasks/test/_default`, and adding a CI job in `ci.yml`.
- *Recommendation*: Option 2 is structurally superior. Option 1 is acceptable
  only if runner edits are strictly prohibited.
- *Open question for grilling*: Does the user prefer zero build configuration
  changes (Option 1) or a consolidated BDD target with an explicit fifth runner
  phase (Option 2)?

### Decision (c): Role and growth of property tests

- Proptests own algebraic and generative properties (currently 4 blocks, 12
  tests in `temporal/`).
- Scenarios own concrete stakeholder examples and state transitions.
- *Recommendation*: Proptest does not compete with BDD. Keep proptest strictly
  bounded to mathematical and generative invariants.

### Decision (d): Integration/E2E split and suite disposition

- The disposition of `.scratch/integration-e2e-suite/` is **accepted and
  binding**.
- The split is preserved: integration is in-process; E2E is spawn-only.
- *Recommendation*: BDD builds on top of this suite. When BDD scenarios
  supersede handwritten tests in `tests/e2e/` or `tests/integration/`, retire
  the replaced tests with the ledger method of issues 15 and 16.

### Decision (e): Anti-goals (layers the repo will NOT introduce)

- No mock or stub frameworks (`mockall`).
- No contract-testing frameworks (`pact`).
- No PTY harness for simulated interactive terminals (rejected in issue 10 and
  ticket 07).
- No visual snapshot or golden-master testing of Miette error outputs (Miette
  line-wrapping is non-deterministic).
- No standalone non-nextest test runners.
- *Recommendation*: Formalize these five anti-goals.

```text
```
