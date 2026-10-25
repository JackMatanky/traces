# Research: Rust BDD crate comparison (`rstest-bdd` vs `cucumber`)

This file compares the Rust `rstest-bdd` 0.6.0 crate against `cucumber` 0.23.0 for adoption in this repo. It builds on two source-cited reviews: [rstest-bdd.md](./rstest-bdd.md) and [cucumber-rs.md](./cucumber-rs.md). Facts about each crate are cited as `rstest-bdd §N` or `cucumber-rs §N`, pointing at the section in that file. `file:line` is a line in this repo.

The recommendation at the end is advice, not a decision. Wayfinder ticket [06: Framework re-validation](../issues/06-framework-revalidation.md) re-checks it against the repo's own constraints before any adoption work starts.

---

## 1. Head-to-head

| Axis | rstest-bdd 0.6.0 | cucumber 0.23.0 | Edge |
|---|---|---|---|
| Test model | each scenario (and each outline row) = **real `#[rstest::rstest]` libtest test**; process-per-scenario under nextest (rstest-bdd §4) | one custom runner; scenarios concurrent in-process (default 64) or one big `harness=false` binary (cucumber-rs §4.10, §6.1) | **rstest-bdd** for this repo |
| nextest compatibility | native, no harness contract needed; `--no-tests fail`, filters, retries, timings all work (rstest-bdd §4) | **broken** for `harness=false` (issue #370 open, exit 104; the in-process `#[test]` workaround collapses granularity) (cucumber-rs §6.1) | **rstest-bdd (decisive)** |
| State model | rstest **fixtures** shared across unit/integration/BDD plus `StepContext`/`Slot`/`ScenarioState` derive (rstest-bdd §5.3) | `World` trait, fresh per scenario, inventory-wired (cucumber-rs §4.1) | tie (taste) |
| Hooks | none: fixtures plus Drop plus Gherkin Background (rstest-bdd §5.4) | one `before` + one `after` each (last wins) (cucumber-rs §4.5) | cucumber (marginally) |
| Tags/filtering | **compile-time** tag expressions plus name/index; runtime via nextest filters (rstest-bdd §5.6) | runtime `--tags` tag-expressions, `--name` regex, `CUCUMBER_FILTER_TAGS`, programmatic `filter_run` (cucumber-rs §4.6) | cucumber for runtime; rstest-bdd's compile-time checks are stricter |
| Tables/docstrings | `#[datatable] Rows<T>` plus `DataTableRow`/`DataTable` derives, typed parsing (rstest-bdd §5.1) | `gherkin::Step` accessors, `FromStr` captures (cucumber-rs §4.7) | rstest-bdd (typed) |
| Scenario Outline | → rstest `#[case]` rows, visible individually in runners (rstest-bdd §4) | built into the runner (cucumber-rs §4.3) | tie |
| Async | tokio hard dep; `#[tokio::test]` via attribute policy; async steps via tokio bridge (rstest-bdd §5.8) | runtime-agnostic `futures`; hooks `LocalBoxFuture` (current-thread) (cucumber-rs §4.8, §4.5) | cucumber (agnosticism) / rstest-bdd (ergonomics) |
| Reporters | manual `reporting::{drain,snapshot}` plus junit/json writers (per-process caveat) (rstest-bdd §5.7) | rich writer stack: `Normalize`/`Summarize`/JUnit/JSON/libtest/tracing features (cucumber-rs §4.9) | cucumber |
| Failure semantics | step failure panics → ordinary libtest fail; `skip!` plus `@allow_skipped` plus `RSTEST_BDD_FAIL_ON_SKIPPED` (rstest-bdd §5.5) | `run()` returns writer plus `execution_has_failed()`; `run_and_exit` panics with summary (cucumber-rs §4.4) | tie |
| Feature-file parsing | **compile-time** (proc macro), static scenarios, invalidation engineered (rstest-bdd §4) | runtime globwalk `*.feature`, sorted, `--input` glob (cucumber-rs §5) | trade-off (see §3) |
| Dependency weight (test profile) | **~157 packages** consumer-side incl. tokio and i18n stack (rstest-bdd §3) | **~99 packages** (cucumber-rs §3) | **cucumber** |
| Repo fit: existing rstest | **rstest 0.27 already a dev-dep**, 100+ `#[rstest]` tests; zero rstest coupling in runtime → our version runs (rstest-bdd §3, §6.4) | n/a (own runner) | **rstest-bdd** |
| Lint regime fit | expansion is external-macro (likely clippy-suppressed); step bodies still under our deny lints; tokio dep new to lockfile (rstest-bdd §6.3) | macro-vs-lint collision risk flagged in the review (cucumber-rs §6.4) | tie (check in spike) |
| Maturity / bus factor | 0.6.0, **single maintainer**, ~14 months old, 75k downloads; README/package drift observed (rstest-bdd §7) | 0.23.0, cucumber-rs org, 2 maintainers, 17.3M downloads, active (cucumber-rs §1, §7.7) | **cucumber** |
| License / MSRV | ISC (allow-listed, deny.toml:12) / 1.88 ✓ | MIT OR Apache-2.0 / 1.88 ✓ | tie |

## 2. Recommendation

**Adopt `rstest-bdd` 0.6.0 for this repo; keep `cucumber` as the documented alternative only if runtime Gherkin features ever become mandatory.**

The reasons:

1. The repo's canonical runner is nextest with `--no-tests fail` in every phase and CI job (.mise/tasks/test/_default:81, .github/workflows/ci.yml:97-141). Cucumber's custom runner is *incompatible* with nextest discovery (cucumber-rs §6.1, issue #370, a stated non-goal upstream).
2. rstest-bdd converts that hard constraint into its headline strength: each scenario and each outline row is a first-class libtest test. Per-scenario isolation, filtering, retries, and timing then come from nextest itself, with **zero runner changes**.
3. It builds on the rstest investment already in the tree (Cargo.toml:159, 100+ `#[rstest]` tests) with no version coupling.
4. It has a typed step and datatable API, compile-time step and tag validation, and it passes the license gate (ISC in deny.toml:12).

The costs to accept: a heavier test-profile tree (~157 vs ~99 packages, incl. tokio), a younger single-maintainer crate, and no runtime hooks or tag CLI.

## 3. Risks of the comparison outcome

1. The nextest verdict depends on issue #370 staying open. If upstream lands nextest support, cucumber's top risk falls away and the comparison must be redone.
2. The lint-regime row is unverified for both crates. The spike must run `mise run lint` over representative macro output (rstest-bdd §6.3, cucumber-rs §6.4).
3. The dependency-weight numbers come from resolved graphs on cached sources, not from a cold build of this workspace. The spike measures the real cold-build delta.
4. The comparison assumes both crates stay at the reviewed versions. Pin them; re-review on any minor bump (both are 0.x).

## 4. Adoption steps (nothing implemented)

These steps follow from the recommendation. Ticket [06: Framework re-validation](../issues/06-framework-revalidation.md) confirms or rejects them first.

1. **Spike (half-day)**: add dev-deps `rstest-bdd = "=0.6.0"` and `rstest-bdd-macros = "=0.6.0"` (keep rstest 0.27). Create `tests/features/smoke.feature` and `tests/bdd.rs` with one `#[given]`/`#[then]` pair and `#[scenario(path=…, name=…)]` (or `scenarios!`). Run `cargo nextest list -p traces` to prove that discovery shows the scenario. Run `cargo nextest run -p traces --no-default-features --test bdd` (needs a `[[test]] bdd` entry, or `--test bdd` with auto-discovery). Run `mise run lint` (macro-vs-deny-lints), `mise audit:shear` + `cargo deny check` (dep/license), and time the cold-build delta.
2. **Decide `diagnostics`**: keep it default-on if JUnit/JSON/registry-dump are wanted; otherwise use `default-features = false` for a leaner tree (rstest-bdd §3).
3. **Wire phases**: add a `bdd` phase (or widen the integration filter to `binary(integration) + binary(bdd)`) in `.mise/tasks/test/_default:99-109`. Add or extend the CI `test-integration` job (.github/workflows/ci.yml:115-118). Adopt `required-features = ["test-utils"]` if steps use the lib test API (Cargo.toml:49-51 pattern).
4. **Extend `check:layers`** (mise.toml:148-185) with `tests/bdd.rs` / `tests/bdd/` rules that match the chosen layer: lib-API-only like integration, or binary-spawning like e2e with the support.rs carve-out.
5. **Conventions**: `tests/features/*.feature` in git, one file per behavioral area; a compile-time tag vocabulary (`@smoke`, `@wip`) used via `tags =` on `#[scenario]`/`scenarios!`; pair `@allow_skipped` with `RSTEST_BDD_FAIL_ON_SKIPPED=1` in CI to force pending scenarios red; document the layout under `docs/agents/` next to the test-layer docs.
6. **Optional follow-ups**: nextest JUnit reporting for CI artifacts (instead of the per-process writer, rstest-bdd §5.7); a step-inventory tooling hook via `RSTEST_BDD_DUMP_STEPS=1` and the `--dump-steps` arg passed to the test binary after the separator; revisit cucumber only if hooks or runtime-tag-filtering become must-haves.
