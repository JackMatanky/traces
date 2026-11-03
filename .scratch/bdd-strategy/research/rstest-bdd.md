# Research: the `rstest-bdd` crate

This file reviews the Rust `rstest-bdd` crate (BDD built on `rstest`) for adoption in this repo. Primary source: **rust-docs-mcp tools** (`structure`, `get_item_*`, `get_dependencies`) against the cached crate version **0.6.0**, plus the crate's own packaged source and README in the rust-docs cache, the crates.io API (versions, license, MSRV), and the cargo registry source of the companion crates `rstest-bdd-macros` 0.6.0 and `rstest-bdd-harness` 0.6.0 (also cached into the rust-docs cache). The cucumber side is not re-reviewed here: facts about it live in [cucumber-rs.md](./cucumber-rs.md) and this file cites them as `cucumber-rs §N`.

Evidence convention: `rust-docs item` is an item path that the rust-docs tools return. `source:NN` is a line in `.rust-docs/cache/crates/<crate>/<ver>/source/<path>`, grounded with `rust-docs get_item_source` or a direct read of the same file. `macros-src:NN` and `harness-src:NN` use the same convention for `.rust-docs/cache/crates/rstest-bdd-macros/0.6.0/source/<path>` and `.rust-docs/cache/crates/rstest-bdd-harness/0.6.0/source/<path>`. `file:line` is a line in this repo.

The head-to-head against `cucumber` lives in [rust-bdd-crate-comparison.md](./rust-bdd-crate-comparison.md).

**Tooling note (reproducibility):** the MCP `cache_crate` call failed twice with the known relative-path bug (`RUST_DOCS_MCP_CACHE_DIR` is relative in `opencode.json`; the metadata step reports "manifest path `.rust-docs/cache/.../Cargo.toml` does not exist"). Unlike the cucumber round, hand-writing `dependencies.json` first did **not** help, because the MCP re-runs metadata generation regardless. What worked: the **CLI** with an absolute cache dir:
`rust-docs-mcp call cache-crate --cache-dir "$PWD/.rust-docs/cache" --params
'{"crate_name":"rstest-bdd","source_type":"cratesio","version":"0.6.0"}'`
(binary at `~/.local/share/mise/installs/cargo-https-github-com-snowmead-rust-docs-mcp/branch-main/bin/rust-docs-mcp`). The same CLI cached `rstest-bdd-macros` 0.6.0 and `rstest-bdd-harness` 0.6.0. The durable fix remains: make `RUST_DOCS_MCP_CACHE_DIR` absolute in `opencode.json`.

---

## 1. Crate identity

| Fact | Value | Source |
|---|---|---|
| Latest version | **0.6.0** (released 2026-09-14) | crates.io API `crates.io/api/v1/crates/rstest-bdd` (`max_stable_version`, `created_at`) |
| Full version history (all 13) | 0.1.0-alpha2 · 2025-08-29, 0.1.0-alpha4 · 2025-09-30, 0.1.0 · 2025-11-08, 0.2.0 · 2025-12-08, 0.3.0 · 2025-12-24, 0.3.1 · 2025-12-27, 0.3.2 · 2025-12-28, 0.4.0 · 2026-01-22, 0.5.0 · 2026-02-06, 0.6.0-beta1 · 2026-05-02, 0.6.0-beta2 · 2026-06-08, 0.6.0-beta3 · 2026-07-07, 0.6.0 · 2026-09-14 | crates.io API `versions[]` |
| License | **ISC** (single) | crates.io API (`license`); `source/Cargo.toml:36` |
| MSRV / edition | **1.88** / **2024** (since 0.3.0; 0.1 to 0.2 were 1.75/2021) | crates.io API (`rust_version`, `edition`); `source/Cargo.toml:13-16` |
| Downloads | 74,901 total; 40,728 recent (bulk from 0.5.0's 57,108) | crates.io API |
| Description | "A BDD testing framework for Rust, built on rstest." | crates.io API; `source/Cargo.toml:24` |
| Maintainer | single author **leynos / Payton McIntosh** (`pmcintosh@df12.net`), github.com/leynos/rstest-bdd | crates.io API `published_by`; `source/Cargo.toml:19` (macros crate author field) |
| Companion crates | `rstest-bdd-macros` 0.6.0 (proc macros, ISC, syn **3**), `rstest-bdd-harness` 0.6.0 (attribute-policy/harness adapters), `rstest-bdd-patterns` 0.6.0, `rstest-bdd-policy` 0.6.0 | `source/Cargo.toml:351-355`; macros `Cargo.toml` `[dependencies]` |
| Gherkin parser | **`gherkin` 0.16** (`features=["parser"]`, `default-features=false`), the same parser family as cucumber 0.23 | `source/Cargo.toml:327-330`; cucumber-rs §1 |

Compatibility with this repo: our `rust-version = "1.96"` and `edition = "2024"` (Cargo.toml:14-15) are both newer than 1.88/2024. **ISC is already in our `deny.toml` allow list** (deny.toml:9-15).

## 2. Feature flags (rstest-bdd 0.6.0)

The list comes from `source/Cargo.toml:39-46`; the crates.io features list confirms it.

| Feature | Default? | Enables |
|---|---|---|
| `diagnostics` | **yes** | `serde` + `serde_json`; gates `reporting::{json, junit}`, `registry::dump_registry`, and the `--dump-steps` ctor (source/Cargo.toml:41-44; source/src/reporting/mod.rs:24-28; source/src/lib.rs:137-173) |
| `mutable_world_macro` | no | an empty guard feature for a rustc ICE that affects macro-driven mutable fixtures (Cargo.toml.orig:66-69) |
| `test-support` | no | async-step testing helpers (`test_support` module, source/src/lib.rs:45-46) |

The macro-crate features apply via the `rstest-bdd-macros/...` CLI or a direct dependency: `compile-time-validation` and `strict-compile-time-validation` (both off by default; macros `Cargo.toml` `[features]`; README.md:55-74). `no-inventory` (README.md:52-53, 415-416) is a fallback registry mode for platforms where linker sections are unwieldy. macOS and Linux do not need it.

## 3. Dependency footprint (consumer side)

- **Direct normal deps of `rstest-bdd` itself: 18** (source/Cargo.toml:313-387): `ctor`, `derive_more`, `fluent`, `gherkin` 0.16, `hashbrown` 0.17, `i18n-embed`, `inventory`, `log`, `regex`, `rstest-bdd-patterns`, `rstest-bdd-policy`, `rust-embed`, `serde`(opt), `serde_json`(opt), `thiserror`, **`tokio` 1** (features `rt-multi-thread`, `macros`, `io-std`, `sync`, **non-optional**, source/Cargo.toml:372-379), `tracing`(log), `unic-langid`.
- **Crucially, `rstest-bdd` has NO `rstest` dependency.** It is not in the normal deps; it appears only as a dev-dep at 0.26.1 (source/Cargo.toml:420-421). The coupling to rstest happens only through macro-emitted `#[rstest::rstest]`, which resolves in the *consumer's* crate (§5.2). Our rstest **0.27** dev-dep (Cargo.toml:159) is the version that runs. A dual-rstest type clash is not possible.
- **`rstest-bdd-macros` normal deps: 17**, including `syn` **3**, `quote`, `proc-macro2`, `proc-macro-error3`, `proc-macro-crate`, `camino`, **`cap-std`**, `convert_case`, `gherkin`, `newt-hype`, `regex`, `walkdir`, plus the three companion crates (macros `Cargo.toml` `[dependencies]`). Feature-file parsing happens **inside the proc macro**, which is why cap-std, walkdir, and gherkin sit on the macro side.
- **Full resolved closure as a consumer (rstest-bdd + rstest-bdd-macros as normal roots, normal edges only): 157 packages.** The count comes from `.rust-docs/cache/crates/rstest-bdd/0.6.0/dependencies.json` resolve graph. Cucumber needs **~99** non-dev packages (cucumber-rs §3).
- Overlap with our tree (dedupe): **14** are already present: `convert_case`, `indexmap`, `regex`, `rustc-hash`, `rustix`, `serde`, `serde_json`, `syn` (we have 2.x for dev; macros want 3.x, both will coexist; deny.toml:26-29 notes that the ecosystem is mid-migration), `thiserror`, `toml`, `tracing`, `unicode-width`, `walkdir`, `winnow`.
- **New to us (~126)**, notable: `tokio` (not in our Cargo.lock today; grep confirms), the gherkin/`peg` parser stack, the Fluent i18n stack (`fluent*`, `i18n-embed*`, `intl-*`, `unic-langid*`, `rust-embed*`, ~25 packages of always-on diagnostics localization), `inventory`, `hashbrown`, `derive_more`, `cap-std`, `parking_lot`, `windows-*` (win-only). All of it is **test-profile only** if you take these crates as dev-deps. The production build is unaffected.
- The footprint lever: `default-features = false` drops `diagnostics`, which drops serde/serde_json from *this* crate and the `reporting::{json,junit}` plus dump machinery. The i18n stack stays, because localization is wired unconditionally into `StepError::Display` (source/src/lib.rs:281-286).

## 4. Test model and nextest compatibility (the decisive axis)

**Verdict: rstest-bdd is nextest-compatible by construction. Every scenario is a real libtest test.** The evidence chain:

1. `#[scenario]` expands to a test function that carries **`#[rstest::rstest]`** (macros-src/src/codegen/scenario/test_attrs.rs:160,228: `PolicyAttribute::Rstest => quote! { #[rstest::rstest] }`). The same file documents the ADR-008 attribute-policy chain, which optionally layers `#[tokio::test]`/`#[gpui::test]` alongside it.
2. Scenario Outlines emit one **`#[case(idx, cells...)]` per Examples row** (macros-src/src/codegen/scenario/helpers.rs:137-141: `quote! { #[case( #idx_lit, #(#cells),* )] }`), and the generated signature gains a hidden `#[case] __rstest_bdd_case_idx: usize` parameter (macros-src/src/codegen/scenario/mod.rs:317-321). Each row therefore becomes its own rstest case, and so its own libtest test name (`fn::case_N`).
3. The `scenarios!("dir")` proc macro emits **one test function per discovered Gherkin scenario** in a directory. The path resolves relative to `CARGO_MANIFEST_DIR`. The macro compile-errors on missing, empty, or unparseable dirs (macros-src/src/lib.rs:175-202).
4. rstest itself is proven under nextest in this repo: `#[rstest]` appears in 100+ unit tests (for example src/note/parser.rs:681, src/task.rs:1163), and all three nextest phases run them today.
5. There is no custom harness anywhere: no `harness = false` requirement, no `--list` protocol of its own. The only `std::process::exit` in the runtime crate is the opt-in `--dump-steps` diagnostics ctor. It is gated on env `RSTEST_BDD_DUMP_STEPS` *and* the `--dump-steps` CLI arg before libtest parses args (source/src/lib.rs:137-157), so it stays inert under nextest.

The consequences under our runner (`.mise/tasks/test/_default:68-82`, `.github/workflows/ci.yml:97-141`, `.config/nextest.toml`): process-per-scenario *and* process-per-outline-row isolation, per-scenario timing and slow-marking, nextest `--retry`, filter expressions, `--no-tests fail`, and stdout capture all work with **zero runner changes**. This is exactly the capability that cucumber lacks (cucumber-rs §6.1: issue #370 open, the maintainer declines process-per-scenario, `harness=false` breaks discovery with exit 104, the workaround is an in-process single `#[test]`).

**Feature-file discovery and version control:** `.feature` files are plain UTF-8 in git (conventionally `tests/features/*.feature`, README.md:88). The proc macros parse them **at compile time**. `#[scenario(path = "...", ...)]` binds a test to one scenario by `name` or `index` (or to the first scenario), and `scenarios!` auto-generates tests for a whole directory (macros-src/src/lib.rs:122-160, 175-202). The path literal resolves at expansion time against `CARGO_MANIFEST_DIR`. Rebuild invalidation is engineered and tested (`source/tests/feature_rebuild_invalidation.rs`, `source/tests/feature_path_metadata.rs` test targets in source/Cargo.toml:150-155; README.md:561-563 mentions OUT_DIR AST caching). The design is static: adding a scenario requires recompiling the test crate (README.md:433-439 Limitations).

## 5. API walkthrough (rstest-bdd 0.6.0)

### 5.1 Step macros `#[given]` / `#[when]` / `#[then]`

The three macros are `#[proc_macro_attribute]`s at `macros-src/src/lib.rs:71-120`. The pattern forms (README.md:183-216, 265-288):

- `#[given("an empty basket")]`: a literal.
- `#[when("I add {count:u32} pumpkins")]`: **typed placeholders**, parsed via `FromStr`. This is the preferred style.
- regex groups `"I search for \"(.*)\""`: the fallback.
- `#[given(expr = "...")]`: cucumber-rs-compatible Cucumber Expression syntax, "supported for migration" (README.md:265-288).

Function parameters work by-name injection: "Implicit fixtures such as `basket` must already be in scope in the test module; `#[from(name)]` only renames a fixture and does not create one" (README.md:206-207). The first parameter may be `&mut <Fixture>`. Typed captures map positionally. Docstrings arrive as `String`. Tables use `#[datatable] datatable: rstest_bdd::datatable::Rows<T>` (`T: DataTableRow` derive) or plain `Vec<Vec<String>>` (README.md:338-358; rust-docs items `rstest_bdd::datatable::{Rows, DataTableRow}`; derives `DataTableRow`/`DataTable` at macros-src/src/lib.rs:210-236). Steps may be `async` (README.md:104-160) or return `StepResult<T, E = StepError>` (source/src/lib.rs:290-294; `StepError` variants `MissingFixture`/`ExecutionError`/`PanicError`, source/src/lib.rs:194-222). A step failure is a normal test panic via a localized message, which libtest reports as a failure with exit 101.

### 5.2 `#[scenario]` and `scenarios!`

The authoritative doc comment is at `macros-src/src/lib.rs:122-160`.

- Selectors: a positional path, or `index = N` or `name = "title"` (default: the first scenario).
- **Compile-time tag expressions**: `tags = "@fast and not (@wip or @flaky)"` with `not`/`and`/`or`, parentheses, and precedence not>and>or. The filter decides which scenarios the attribute binds, before the selectors apply (parser at macros-src/src/parsing/tags/).
- Expansion: `#[rstest::rstest]` plus the preserved user attrs plus a generated body that runs the matched steps in order (Background steps are prefixed, macros-src/src/parsing/feature/mod.rs:240) and then executes the (usually empty) test body (README.md:116-122: "The test body runs *after* all steps have passed").
- Attribute policy chain (ADR-008): explicit `attributes = …` beats `harness = …` (the first-party `TokioHarness`/`GpuiHarness` are recognized), which beats `runtime = …` default. An existing `#[tokio::test]` on the function is detected and not duplicated (macros-src/src/codegen/scenario/test_attrs.rs:1-27, 62-87).

### 5.3 State model, fixtures, not World

There is **no `World` object**. State is ordinary **rstest fixtures** shared between unit, integration, and BDD tests (README.md:17-20, 412-413). At runtime, the fixtures that steps can reach are staged in a per-scenario **`StepContext`** with typed slot borrowing (rust-docs item `rstest_bdd::context::StepContext`; methods `insert`, `borrow_ref`, `borrow_mut`, `get`, `available_fixtures`; guards `FixtureRef`/`FixtureRefMut`; errors `FixtureBorrowError`; structure and output at source/src/context/mod.rs). For struct-based shared state, use `#[derive(ScenarioState)]` plus `Slot<T>` (RefCell-backed) with a clear-hook between runs (rust-docs items `rstest_bdd::state::{ScenarioState, Slot}`; macros-src/src/lib.rs:145-158; source/src/state.rs:1-20). Per-scenario isolation comes from rstest: each test process builds its own fixtures.

### 5.4 Hooks

There are **no before/after scenario hooks**. Setup and teardown are fixtures (including async fixtures via `#[future]`, README.md:118-122) plus Drop impls. Gherkin `Background` is supported: it prefixes the scenario steps (macros-src/src/parsing/feature/mod.rs:240, parsed via `gherkin::Background`, macros-src/src/parsing/feature/support.rs:233-234). Cucumber, by comparison, has exactly one `before` and one `after` hook each (cucumber-rs §4.5).

### 5.5 Skip semantics and runtime config

- `skip!()` or `skip!("msg")` from a step stops the scenario (rust-docs item `rstest_bdd::skip!`; source/src/macros.rs:6-23).
- A scenario that skips without the **`@allow_skipped` tag** panics when `fail_on_skipped` is on (env `RSTEST_BDD_FAIL_ON_SKIPPED` or `config::set_fail_on_skipped`; default false, so the scenario is recorded as Skipped and the test passes; source/src/config.rs:31-54, source/src/skip.rs:6).
- Assertion helpers: `assert_step_ok!`, `assert_step_err!`, `assert_step_skipped!`, `assert_scenario_skipped!` (source/src/macros.rs:39-198).

### 5.6 Tags and filtering summary

| Need | Mechanism |
|---|---|
| Select scenarios by tag | **compile-time** `tags = "expr"` on `#[scenario]`/`#[scenarios]` (§5.2) |
| Select by name/index | `name =` / `index =` on `#[scenario]` |
| Runtime CLI filter | ordinary libtest/nextest: `cargo nextest run -E 'test(...)'` or a name substring |
| Skip policy tag | `@allow_skipped` (§5.5) |

There are no runtime tag expressions, no `--tags` CLI, and no `which_scenario` serial/concurrent switch. Nextest's process-per-test parallelism replaces the concurrency knobs (cucumber-rs §4.6 and §4.10 have all of these).

### 5.7 Reporters

- Per-scenario `ScenarioRecord` (feature path, name, line, tags, status) accumulates in a process-global `REPORTS` (rust-docs items `rstest_bdd::reporting::{record, snapshot, drain, ScenarioRecord, ScenarioStatus, ScenarioMetadata}`; source/src/reporting/mod.rs:161-211).
- The **JUnit XML writer** is `reporting::junit::write(&mut W, &[ScenarioRecord])`: a manual call, one `<testsuite>`, skipped/failure handling (source/src/reporting/junit.rs:1-80, gated on `diagnostics`, source/src/reporting/mod.rs:27-28). A **Cucumber JSON writer** exists the same way (`reporting::json`, mod.rs:24-25).
- Under nextest's process-per-test, `REPORTS` is per scenario-process, so aggregating one XML across a whole run is DIY. Note that nextest has **built-in JUnit reporting** of its own. It covers the CI-artifact use case without touching rstest-bdd's writer.
- A step-registry inventory dump exists for tooling: `RSTEST_BDD_DUMP_STEPS=1` plus `--dump-steps` prints JSON of all registered steps (source/src/lib.rs:137-157; `registry::dump_registry`, rust-docs structure).

### 5.8 Async and runtime coupling

- The runtime crate **hard-depends on tokio** (`rt-multi-thread`, `macros`, `io-std`, `sync`; source/Cargo.toml:372-379) and re-exports it as `__rstest_bdd_tokio`, because "Generated asynchronous step wrappers use this Tokio bridge" (source/src/lib.rs:113-116). Async steps in 0.6.0 are therefore **tokio-flavored**, not runtime-agnostic (README.md:470's "runtime-agnostic via harness adapters" refers to the attribute-policy layer in `rstest-bdd-harness`, harness-src/src/lib.rs:1-35).
- For a sync-only suite, tokio still compiles into the test profile, but the default (Std) path builds no runtime. The default `#[scenario]` emits only `#[rstest::rstest]` (test_attrs.rs:62-66 `DEFAULT`).
- Scenario outlines with placeholders **reject async harnesses** (`reject_async_harness`, macros-src/src/codegen/scenario/mod.rs:271).
- `async_step::sync_to_async` wraps sync step functions for the async path (rust-docs item `rstest_bdd::async_step::sync_to_async`; README.md:218-263).

## 6. Integration fit (this repo)

### 6.1 Where it would live

- `tests/features/*.feature` holds the version-controlled specs. This matches both crates' conventions and our `tests/` layout (tests/ has integration/, e2e/, baselines/ today).
- `tests/bdd.rs` + `tests/bdd/` holds step definitions and fixtures, styled after `tests/integration.rs`. An optional `[[test]] name = "bdd" required-features = ["test-utils"]` inherits the silent-green guard (Cargo.toml:49-51).
- Steps are plain functions in the test crate, so e2e-style BDD (spawning the binary) must obey the `check:layers` rules (mise.toml:148-185): `std::process` is banned outside `tests/e2e/support.rs`, and `tests/integration/` must not spawn processes. A BDD suite that spawns the binary therefore belongs under the e2e rules, and lib-API-driven steps belong under the integration rules.

### 6.2 Runner / CI wiring

- All scenarios are ordinary libtest tests, so they ride the existing nextest phases. Two options: widen the integration phase filter (`.mise/tasks/test/_default:103-104`: `-E 'binary(integration)'` becomes `'binary(integration) + binary(bdd)'`), or add a dedicated `run_phase bdd --features test-utils --test bdd` line. Mirror the choice in CI's `test-integration` job (.github/workflows/ci.yml:115-118). `--no-tests fail` semantics are preserved (.mise/tasks/test/_default:81).
- `.config/nextest.toml` needs nothing; there are no harness quirks. If you want JUnit artifacts in CI, prefer **nextest's built-in JUnit support** over rstest-bdd's per-process writer (§5.7).

### 6.3 Lint regime

- `allow-unwrap-in-tests = true` and `allow-expect-in-tests = true` (clippy.toml:86-87) cover step bodies. Our tests already unwrap freely.
- Our code still denies, including in tests: `print_stdout`, `panic`, `exit`, `dbg_macro`, `indexing_slicing`, `arithmetic_side_effects` (Cargo.toml:186-211). Step bodies must fail via `assert!`/`Result`, never by name `println!`/`panic!`. Generated glue comes from an external proc-macro crate, so clippy's external-macro suppression should apply, as it already does for our 100+ `#[rstest]` expansions. **Check in the spike** with `mise run lint` over a representative scenario file.
- `cargo-shear` (`mise audit:shear`) needs both dev-deps (`rstest-bdd` runtime items *and* `rstest-bdd-macros` imports) to be referenced. They will be, because the macros are imported directly (§7).
- `cargo-deny`: ISC is allow-listed (deny.toml:12). The new i18n/peg/tokio tree needs a license spot-check; all of it is permissive in practice (check in the spike).

### 6.4 Existing rstest usage

The dev-dep is `rstest = { version = "0.27.0", default-features = false }` (Cargo.toml:159). `#[rstest]` and fixtures are idiomatic across src/ unit tests (src/note/parser.rs:681, src/task.rs:1163, src/template/engine/string.rs:233, and 100+ other sites). rstest-bdd's fixtures **are** rstest fixtures, so the BDD layer plugs into an investment the repo has already made. rstest-bdd itself has zero rstest dependency, so our 0.27 is authoritative (§3).

## 7. Risks / limitations (rstest-bdd)

1. **Youth and bus factor (top risk)**: 0.x, single maintainer, 13 versions in 13 months with breaking minors (the 0.6.0 migration guide is referenced at README.md:76-82). Mitigation: pin `=0.6.0` plus the companions, keep the suite small initially, and keep cucumber as a fallback.
2. **README/package drift**: the published 0.6.0 does **not** re-export the proc macros from `rstest_bdd`. Its own tests import `use rstest_bdd_macros::{given, scenario, then, when}` (source/tests/scenario.rs:6), yet the README shows `use rstest_bdd::{scenario, given, …}` and an install snippet that still says `0.6.0-beta3` (README.md:44, 106). Consumers must add **both** `rstest-bdd` and `rstest-bdd-macros` as dev-deps. Trust the source over the docs.
3. **Dependency weight**: ~157 packages vs cucumber's ~99, including a **non-optional tokio** (`rt-multi-thread`) that is absent from our lockfile today, plus an always-on Fluent i18n stack for diagnostics messages. The cost is dev-profile only, but the first cold `mise run test` and cold CI get slower.
4. **Compile-time parsing**: every `.feature` edit re-runs the proc macro. Path typos become compile errors (arguably good). Scenarios are static (README.md:433-439). OUT_DIR AST caching mitigates large suites (README.md:561-563).
5. **No runtime hooks, runtime tag filters, or retries.** Fixtures plus Background plus nextest filters and retries cover most needs, but cucumber's `before`/`after` and `--tags` ergonomics are absent (§5.4-5.6).
6. **JUnit/JSON aggregation is per-process** under nextest (§5.7). Use nextest's own JUnit reporting for CI artifacts.
7. **Clippy-vs-expansion collisions** are unverified for our strict deny set. The spike must run `mise run lint` (§6.3).
8. **Outline rows plus async are unsupported** (`reject_async_harness`, macros-src/src/codegen/scenario/mod.rs:271). This is irrelevant if steps stay sync.
9. **syn 3**: the macros crate wants syn 3 while half our tree is still syn 2 (deny.toml:26-29). The cost is extra compile-unit duplication in test builds only.
