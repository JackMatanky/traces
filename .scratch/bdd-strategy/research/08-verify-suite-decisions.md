# Research 08: Verification of Integration and E2E Suite Decisions

This document checks the decisions and claims from
`.scratch/integration-e2e-suite/` (`spec.md`, issues 01–19,
`test-arch-audit.md`) against current code and Git history.

## 1. Issue Claims Verification Matrix

| Issue | Claimed Decision | Verified Today? | Evidence (File:Line) | Drift & Causing Commit |
|---|---|---|---|---|
| **01** | Visibility baselines frozen (`default`, `test-utils`, `pub-tokens`); check wired into mise/hk/CI; glob re-exports banned. | **Yes** | `mise.toml:121-146`, `hk.pkl:121-124`, `.github/workflows/ci.yml:71-73`, `tests/baselines/*` | Baselines updated by commits adding features (e.g. `ca96e8cd`, `728696d0`, `a4a25382`), but mechanical check remains intact and active. |
| **02** | Query workflow assertions use non-refreshing `IndexerService::load()`; test doc comments match actual execution. | **Yes** | `src/cli/mod.rs:1367`, `tests/e2e/workflow.rs:1-17` | None. Tests continue to verify non-refreshing load. |
| **03** | Integration manifest requires `test-utils`; CI asserts non-empty suites and >=1 doctest (`--no-tests fail`). | **Yes** | `Cargo.toml:49-52`, `.github/workflows/ci.yml:83-165`, `.mise/tasks/test/_default:81` | None. Guard prevents silent skipping in CI and runner. |
| **04** | Config staleness investigated; fresh-refresh class axes pinned in unit test; repro forbidden as outer assertion. | **Yes** | `src/index/service.rs:1255-1279`, `tests/e2e/config.rs:180` | Index config-drift detection implemented in `207cc3fb`/`40378704`, obsoleting original manual reload requirement. |
| **05** | EPIPE/exit-101 decision recorded; release aborts (134), dev panics (101); in-band test shape documented. | **Yes** | `tests/e2e/process_contract.rs:1-25`, `.scratch/integration-e2e-suite/issues/05-epipe-exit101-decision.md` | None. Production panics abort via panic hook in release. |
| **06** | Sole test export is `traces::testing` frozen at 46 identifiers; no root re-export shim. | **Partial** | `src/lib.rs:178-212`, `tests/baselines/public-api-test-utils.txt:279-660` | Drifted from 46 to 47 symbols (`728696d0` added `DriftCause`; `a4a25382` replaced `TzGuard` with `ZoneGuard`). Lib doc at line 182 still claims "46 identifiers" [corrected: baseline has 47]. |
| **07** | Shared test arrangement in `tests/common` without crate imports; `ConfigFixture` supports raw TOML; convention for assertions. | **Partial** | `tests/e2e/config_fixture.rs:20-142`, `tests/e2e/config.rs:1-180` | `tests/common` was deleted and moved to `tests/e2e/config_fixture.rs` in `7b71ed06`. Integration target has no `common` module. |
| **08** | E2E tests strictly spawn binary; `tests/e2e/` imports no crate symbols; `CwdGuard` deleted; `check:layers` enforces boundary. | **Yes** | `mise.toml:148-190`, `tests/e2e/support.rs:25-90`, `tests/e2e/init.rs:1-30` | Check temporarily dead between `7b71ed06` and `938c0748` due to missing directory pathspec. Reactivated in `938c0748`. |
| **09** | Hermetic sandbox env policy with default-deny; platform passthroughs; sandbox path accessors. | **Yes** | `tests/e2e/support.rs:30-100`, `tests/e2e/env_policy.rs:1-120` | None. Default-deny environment scrubbing is enforced. |
| **10** | Process exit code assertions (0, 1, 2, never 101); exact stream splits; PTY/130 rejected. | **Yes** | `tests/e2e/process_contract.rs:20-170`, `src/cli/mod.rs:1653-1660` | None. PTY/130 remains unapproved. |
| **11** | Config TOML drives behavior through spawn; 4 failure stages covered; global config via env. | **Yes** | `tests/e2e/config.rs:20-210` | Refactored with config drift detection, but CLI error and exit contracts remain valid. |
| **12** | Index refresh family: query-visible deltas, fail-open unit, content edit between spawns, divergence canary, corruption heal. | **Yes** | `tests/integration/refresh.rs:1-120`, `tests/e2e/index.rs:1-150` | Store recovery and config drift rebuild logic added, but test contracts remain active. |
| **13** | Query store parity differential; cold inlinks coverage; `@class` template composition root; deleted `schema_field_resolution.rs`. | **Yes** | `tests/integration/query.rs:1-200`, `tests/integration/template_render.rs:120-160`, `tests/e2e/query.rs:250-280` | None. Integration schema file remains deleted. |
| **14** | Template overwrite refusal, non-TTY null stdin, cross-render freshness, index feedback loop. | **Yes** | `tests/e2e/template.rs:1-180`, `tests/integration/template_render.rs:1-110` | Minijinja 3 migration (`221564cb`) updated templates, but capability contracts held. |
| **15** | Duplication analysis recomputed; ledger rows map behavior -> former layer -> defect class -> survivor. | **Yes** | `.scratch/integration-e2e-suite/issues/15-recompute-duplication-ledger.md` | Recomputation was completed and signed off before ticket 16 execution. |
| **16** | 9 duplicate tests + `config_lifecycle.rs` + `task_classification.rs` pruned per ledger; ~74 dead fixture temps removed; golden path kept. | **Yes** | Commit `bc196cdc`, `tests/integration/`, `tests/e2e/query.rs:216` | Golden path was kept and later renamed to `workflow.rs` in ticket 19 (`d5f4b4f6`). |
| **17** | Arrange/trust split in fixtures; in-place helper renames; `impl Default for ConfigService` removed; literals single-sourced. | **Partial** | `tests/e2e/support.rs:90-115`, `src/config/service.rs:197-210`, `tests/baselines/pub-tokens.txt:249-275` | `impl Default for ConfigService` was removed in `61e37e71`, but re-introduced in commit `ca96e8cd` (using OS roots in production). |
| **18** | Canonical runner has 4 phases (`unit`, `integration`, `e2e`, `doc`); timeouts configured; default-features E2E in CI. | **Yes** | `.mise/tasks/test/_default:98-122`, `.config/nextest.toml:6-19`, `.github/workflows/ci.yml:83-165` | None. Runner and CI phases match. |
| **19** | Tests organized into capability trees; `dispatch.rs` dissolved; glossary terms aligned; golden path renamed to `workflow.rs`. | **Yes** | `tests/e2e/*.rs`, `tests/integration/*.rs`, commit `d5f4b4f6` | Tree structure preserved. |

---

## 2. Deep Dive: Layer Check Invalidation and Reactivation (`938c0748`)

### What Was Dead and For How Long?

- **Failure mechanism:** In commit `27475edd` (2026-10-09), `mise.toml`'s
  `check:layers` task was changed to scan `tests/e2e.rs tests/e2e/
  tests/common/`. But in commit `7b71ed06` (2026-10-09 22:46), `tests/common/`
  was moved to `tests/e2e/config_fixture.rs`.
- Because `tests/common/` was no longer a directory, `git grep` exited with
  status `128` (fatal error: pathspec 'tests/common/' did not match any files).
- In `check:layers`:

  ```bash
  if git grep ... tests/common/ >&2; then
      exit 1
  fi
  ```

  The exit code `128` caused the `if` condition to evaluate to false (non-zero),
  bypassing the error branch. In the same way, `e2e_process_refs` captured empty
  output, short-circuiting the check.
- **Duration:** Dead from `7b71ed06` (2026-10-09 22:46:06 +0300) until
  `938c0748` (2026-10-21 18:58:32 +0300). This span was **11 days, 20 hours,
  across 224 git commits**.

### Were Rules Violated During the Dead Window?

- Check across all 224 commits (`7b71ed06..938c0748`):
  1. `traces::` references in `tests/e2e/`: **0 violations**.
  2. `std::process` outside `tests/e2e/support.rs`: **0 violations**.
  3. `std::process` / `CARGO_BIN_EXE` in `tests/integration/`: **0 violations**.
  4. `set_current_dir` across `tests/`: **0 violations**.
- **Conclusion:** No boundary violations occurred while the gate was inactive.
  The fix in `938c0748` made `common_paths` conditional on directory existence.
  This change restored enforcement cleanly.

---

## 3. Enforcement Gates in CI and Hooks

### `check:layers`

- **CI:** Run in `.github/workflows/ci.yml:73` under the `lint` job with `mise
  run check:layers`.
- **Hooks:** Configured in `hk.pkl:125-128` under `quality["layer-enforcement"]`
  scanning `tests/**/*.rs`. Triggered on `pre-push` and `hk check`.
- **Mise:** Task defined in `mise.toml:148-190`. Run as part of `mise run check`
  (`mise.toml:116`).

### `check:visibility`

- **CI:** Run in `.github/workflows/ci.yml:72` under the `lint` job with `mise
  run check:visibility`.
- **Hooks:** Configured in `hk.pkl:121-124` under `quality["visibility"]`
  scanning `src/**/*.rs`, `tests/baselines/**`, `Cargo.toml`. Triggered on
  `pre-push` and `hk check`.
- **Mise:** Task defined in `mise.toml:121-146`. Run as part of `mise run check`
  (`mise.toml:115`).

---

## 4. Facade Membership and `traces::testing`

- **Spec Claim (Issue 06):** Sole test-only export surface is `traces::testing`, frozen at 46 identifiers.
- **Status Today:**
  - `src/lib.rs:182-183` doc comment states: *"The membership of this module is
    frozen: exactly the 46 identifiers required by external test targets under
    the `test-utils` feature."*
  - Actual exported symbol count in `src/lib.rs:185-212` is **47 symbols**:
    1. `Config`, `ConfigService`, `TaskConfig`, `TrustRequest` (4)
    2. `FileFormat` (1)
    3. `Blake3Digest`, `Blake3PathHash` (2)
    4. `DriftCause`, `FileEntry`, `IndexSetting`, `IndexerService`, `InlinkMap`,
       `RefreshReport`, `WorkspaceIndex` (7)
    5. `path_codec` (1)
    6. `QueryBuilder`, `QueryBuilderError`, `QueryResult`, `QueryRow`,
       `QueryService`, `QuerySet`, `SourceSelector` (7)
    7. `Schema`, `SchemaFieldDef`, `SchemaService`, `SchemaServiceError` (4)
    8. `CommitPolicy`, `RenderFailureKind`, `TemplatePathInput`,
       `TemplateService`, `WriteMode`, `WriteOutcome` (6)
    9. `ZoneGuard` (1)
    10. `DEFAULT_SCHEMA_VALUES_DIR`, `DEFAULT_SCHEMAS_DIR`,
        `DEFAULT_TEMPLATES_DIR`, `TestProject`, `build_test_index`,
        `create_trusted_project`, `fixture_service`, `parse_note`,
        `parse_note_str`, `parse_tag`, `resolve_safe_path`, `write_note`,
        `write_schema`, `write_template` (14)
    - Total: $4 + 1 + 2 + 7 + 1 + 7 + 4 + 6 + 1 + 14 = 47$.
  - Drift cause: Commit `728696d0` added `DriftCause` to `src/lib.rs:190` during
    the index drift change. This increased the count from 46 to 47 without
    updating the doc comment. Baseline
    `tests/baselines/public-api-test-utils.txt` was deliberately updated to
    match.

---

## 5. Pruning Ledger vs Surviving Tests

- **Ledger Verification (Issues 15 & 16):**
  - `config_lifecycle.rs` (D1, D10): Deleted. Replaced by `tests/e2e/trust.rs`
    and `process_contract.rs`.
  - `index_persistence_roundtrip.rs::persist_then_load...` (D2): Deleted.
    Replaced by in-crate `service.rs` units. Re-query test survives in
    `tests/integration/persistence.rs`.
  - `dispatch.rs::task_prints_a_checkbox_line_per_task` (D5): Deleted. Merged
    into `tests/e2e/query.rs:216`.
  - `dispatch.rs::task_sorting_orders_output...` (D6): Deleted. Covered
    in-crate.
  - `dispatch.rs::task_table_invalid_column...` (D7): Deleted. Replaced by
    invalid where test.
  - Shell completion twins (D9): Bash/fish deleted; Zsh survives in
    `tests/e2e/completions.rs:1-35`.
  - `dispatch.rs::trust_then_index...` (D12): Deleted. Replaced by
    `list_persists_the_file_index...` in `tests/e2e/index.rs`.
  - `task_classification.rs`: Deleted.
  - Golden path test: Renamed from `golden_path.rs` to `workflow.rs` in
    `d5f4b4f6`. It survives as the high-level workflow test.

---

## 6. Decision Status Summary

1. **Visibility Freeze (01):** Hard gates in mise, hk, and CI prevent unauthorized public API expansion.
2. **Honesty First (02):** Non-refreshing loads and accurate doc comments remain standard.
3. **Silent Green Guard (03):** Required features and non-empty nextest and doctest guards prevent vacuous green passes.
4. **E2E Spawn-Only Invariant (08):** Zero `traces::` crate calls in `tests/e2e/`. Execution occurs strictly through spawned binaries via `Sandbox`.
5. **Sandbox Isolation (09):** Scrubbed default-deny environment variables with explicit platform passthroughs.
6. **Process Exit Contracts (10):** Numeric exit codes 0, 1, 2, and abort-to-134 pinned.
7. **Capability Naming (19):** Directory structure strictly partitioned into capability modules.

### Drifted Decisions

1. **Facade Membership (06):** Symbol count drifted from 46 to 47 with the introduction of `DriftCause`. Doc comment at `src/lib.rs:182` is stale.
2. **`ConfigService::default` Removal (17):** Issue 17 removed `impl Default for ConfigService`. Commit `ca96e8cd` restored `impl Default for ConfigService` to back production CLI runs with OS state directories, replacing `new()` and `at()`.
3. **Shared `tests/common` Structure (07):** Issue 07 specified `tests/common` shared between `integration` and `e2e`. Commit `7b71ed06` moved it exclusively into `tests/e2e/config_fixture.rs`. Integration tests do not use it.
4. **Config Staleness Manual Reload (04):** Issue 04 assumed manual reload upon config modification. Commits `40378704`/`207cc3fb` introduced automatic index invalidation on config drift.

### Dead Decisions & Stale Documentation

- **Stale Lib Doc:** `src/lib.rs:182` claims 46 identifiers; actual count is 47.
- **Dead `tests/common` Pathspec:** `mise.toml:154-157` maintains fallback logic
  for `tests/common/`, which no longer exists in the repository.

```text
```
