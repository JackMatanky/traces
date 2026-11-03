# Research: the `cucumber` crate (cucumber-rs)

This file reviews the Rust `cucumber` crate for adoption in this repo. The crate is a Cucumber-style BDD testing framework.

Primary source: **rust-docs-mcp tools** (`structure`, `search_items(_preview)`, `get_item_details`, `get_item_source`, `get_dependencies`, `get_crates_metadata`) against the cached crate version **0.23.0**. Secondary sources: the crate's own source, README, and CHANGELOG in the rust-docs cache; the crates.io API (version, license, MSRV); the nextest docs and the cucumber-rs issue tracker (nextest-compatibility facts only).

Evidence convention: `rust-docs item` is an item name that the rust-docs tools return. `source:NN` is a line in `.rust-docs/cache/crates/cucumber/0.23.0/source/<path>`, grounded with `rust-docs get_item_source`. `file:line` is a line in this repo.

The head-to-head against `rstest-bdd` lives in [rust-bdd-crate-comparison.md](./rust-bdd-crate-comparison.md). The `rstest-bdd` review lives in [rstest-bdd.md](./rstest-bdd.md).

**Tooling note (reproducibility):** the pre-started cache task (`1f9360e8…`, cucumber-0.13.0) and all retries failed at "generate dependency metadata: manifest path `.rust-docs/cache/.../Cargo.toml` does not exist". The root cause: `opencode.json` gives the MCP server a **relative** `RUST_DOCS_MCP_CACHE_DIR` (".rust-docs/cache/"). The metadata step's `cargo metadata` then runs with the wrong working directory. The same call through the CLI succeeded, because the mise environment gives an absolute path. Workaround used: write the missing `dependencies.json` (the plain `cargo metadata --all-features` output, the exact artifact that sibling crates have, e.g. `tempfile/3.27.0/dependencies.json`) into the gitignored `.rust-docs/` cache, then run `rust-docs-mcp call cache-crate` for cucumber 0.23.0 and cucumber-codegen 0.23.0. The MCP tools worked afterwards. **Note: 0.13.0 is not current. The latest is 0.23.0.** Durable fix: make `RUST_DOCS_MCP_CACHE_DIR` absolute in `opencode.json`.

---

## 1. Crate identity

| Fact | Value | Source |
|---|---|---|
| Latest version | **0.23.0** (released 2026-04-23) | crates.io API `crates.io/api/v1/crates/cucumber` (`max_stable_version`, version `created_at`); CHANGELOG.md head of cached source |
| Version history (recent) | 0.22.1 · 2025-12-23, 0.22.0 · 2025-12-12, 0.21.x … | `source/CHANGELOG.md` |
| License | **MIT OR Apache-2.0** (dual) | crates.io API (`license`); `source/Cargo.toml` `license = "MIT OR Apache-2.0"`; `LICENSE-MIT` + `LICENSE-APACHE` in package |
| MSRV | **1.88**, edition **2024** | `source/Cargo.toml` `rust_version = "1.88"`, `edition = "2024"` (confirmed on crates.io API `rust_version`) |
| Downloads | 17,308,711 total; 828,562 for 0.23.0 | crates.io API |
| Description | "Cucumber testing framework for Rust, with async support. Fully native, no external test runners or dependencies." | `source/Cargo.toml` / README.md:10 |
| Maintainers | bbqsrc, tyranron (active: issue comments 2026-06) | crates.io authors; github.com/cucumber-rs/cucumber#370 |
| Companion crates | `gherkin` 0.16 (parser, MIT/Apache, updated 2026-09), `cucumber-codegen` =0.23.0 (proc macros), `cucumber-expressions` 0.5 | `source/Cargo.toml` deps; docs.rs/gherkin |

This repo uses `rust-version = "1.96"` and `edition = "2024"` (Cargo.toml:14-15). Cucumber needs 1.88/2024. The versions are compatible.

## 2. Feature flags

The list comes from `source/Cargo.toml.orig` `[features]`, verified with `rust-docs get_crates_metadata` and cargo metadata.

| Feature | Default? | Enables |
|---|---|---|
| `macros` | **yes** (`default = ["macros"]`) | `#[given]`/`#[when]`/`#[then]` + `#[derive(World)]` via `cucumber-codegen`, `cucumber-expressions`, `inventory`, `anyhow` |
| `timestamps` | no | event timestamps |
| `output-json` | no | Cucumber JSON report (`base64`, `serde`, `serde_with`, `mime`, `Inflector`; implies `timestamps`) |
| `output-junit` | no | JUnit XML report (`junit-report`; implies `timestamps`) |
| `libtest` | no | libtest JSON output format (IDE integration; **not** nextest list-protocol support) |
| `tracing` | no | emits cucumber events into the `tracing` crate (`tracing`, `tracing-subscriber`, `crossbeam-utils`) |

There is **no `tokio`/`smol`/runtime feature** in 0.23. The async story is runtime-agnostic (see §4.8). `tokio` appears only as a *dev*-dependency of cucumber itself (`get_dependencies` kind="dev").

## 3. Dependency footprint

- **Direct dependencies: 33** (incl. 4 dev): `rust-docs get_dependencies(cucumber, 0.23.0)` returns 29 normal (15 non-optional + 14 optional) + dev (`rand`, `serial_test`, `tempfile`, `tokio`).
- The non-optional core is: `clap` 4.3 (derive/env/wrap_help), `console`, `derive_more`, `either`, **`futures` 0.3**, **`gherkin` 0.16**, `globwalk`, `humantime`, `itertools`, `linked-hash-map`, `pin-project`, `ref-cast`, `regex`, `sealed`, `smart-default`.
- The **full resolved tree** (cargo metadata on the cached source): **99 packages are reachable via non-dev edges with default features; 133 with all features; 174 include dev-deps.** All of it lands in the *test profile* if you take cucumber as a dev-dependency. The production build of `traces` is unaffected.
- The overlap with our tree is good for dedupe. We already have `clap` 4.6, `regex` 1.13, `serde`/`serde_json`, `tracing`, `tempfile` (Cargo.toml:119-164). We lack cucumber's `humantime`, `console`, `derive_more`. Watch: `cargo-shear` (`mise audit:shear`) flags any added dev-dep that no code references. The `cargo-deny` license gate is satisfied by MIT/Apache-2.0.

## 4. Core API walkthrough

### 4.1 `World` trait (state, per-scenario)

`rust-docs get_item_source(World, id 245)` → `src/lib.rs:231-322`:

```rust
pub trait World: Sized + 'static {
    type Error: Display;
    fn new() -> impl Future<Output = Result<Self, Self::Error>>;

    #[cfg(feature = "macros")]
    fn collection() -> step::Collection<Self> where Self: Debug + WorldInventory { … inventory::iter::<Self::Given> … }
    #[cfg(feature = "macros")]
    fn cucumber<I: AsRef<Path>>() -> DefaultCucumber<Self, I> { Cucumber::new().steps(Self::collection()) }
    #[cfg(feature = "macros")]
    fn run<I: AsRef<Path>>(input: I) -> impl Future<Output = ()> { Self::cucumber().run_and_exit(input) }
    #[cfg(feature = "macros")]
    fn filter_run<I, F>(input: I, filter: F) -> impl Future<Output = ()> { … filter_run_and_exit(input, filter) … }
}
```

The docs say: "Represents a shared user-defined state for a [Cucumber] run. It lives on per-scenario basis." The design has **no** out-of-box cross-scenario shared state. The docs recommend `std::sync::LazyLock` if you must share state. Each scenario therefore gets a **fresh `World`**. Isolation comes from the construction.

The wiring is automatic: `collection()` harvests steps that `#[given]`/`#[when]`/`#[then]` register through `inventory`. Registration is crate-global, not per-module.

`World::run` and `World::filter_run` are **exit-style**. They delegate to `run_and_exit`, which panics on failure (see §4.4).

### 4.2 Derive + step attribute macros

`rust-docs get_item_details(cucumber-codegen, id 14)` returns **`#[derive(World)]`**:

- An optional `#[world(init = path::to::fn)]` attribute exists. The init function may be sync or async, fallible or infallible. The default is `Default::default()`.

`rust-docs get_item_details(cucumber-codegen, id 3)` returns **`#[given]`**. The `when` and `then` macros have the identical shape. They are re-exported as `cucumber::{given, when, then, World, Parameter}` (source `src/lib.rs:195`).

The matcher forms are all **compile-time checked**:

- `#[given(regex = r"(\S+) is (\d+)")]`: Rust `regex` syntax.
- `#[given(expr = "{word} is {int}")]`: Cucumber Expression syntax from the `cucumber-expressions` crate.
- `#[given("literal")]`: an exact literal with no captures.

The function argument rules:

- The first argument must be `&mut <World>`.
- The remaining argument types must implement `FromStr` (or be slices of such types).
- Regex captures map to arguments by position.
- To touch the whole step (keyword, table, docstring), name the parameter `step`, or annotate it with `#[step] s: &Step`.
- The return type may be `Result<_, E: Display>`. An `Err` fails the step.
- Steps may be `async` (README.md:35-60 shows `async fn … (w: &mut World, …)` with `tokio::time::sleep`).

### 4.3 Runner configuration: `Cucumber<W, P, I, R, Wr, Cli>`

`rust-docs get_item_details(Cucumber, id 20)` and the `structure(cucumber)` method list give the builder. The default type alias `DefaultCucumber` sits at `source/src/cucumber.rs:805-811`:

```rust
DefaultCucumber<W, I> = Cucumber<W, parser::Basic, I, runner::Basic<W>,
                                 writer::Summarize<writer::Normalize<writer::Basic>>>;
```

| Method | Purpose | Source |
|---|---|---|
| `new` / `custom` / `with_parser` / `with_runner` / `with_writer` / `with_cli` / `with_default_cli` | construct or replace components; `with_default_cli()` pre-parses process args (`cli::Opts::parsed()`); passing `with_cli(Opts::default())` suppresses env-arg parsing | `cucumber.rs:562` docs ("Consumes already parsed `cli::Opts`") |
| `steps` / `given` / `when` / `then` | manual step registration (no macros) | structure |
| `before` / `after` | scenario hooks (single each, see §4.5) | `cucumber.rs:1026` |
| `fail_on_skipped` / `fail_on_skipped_with` | turn `Skipped` steps into `Failed`; undefined steps then fail the run | `writer::fail_on_skipped::FailOnSkipped` (rust-docs id 47) |
| `repeat_skipped` / `repeat_failed` / `repeat_if` | re-print skipped/failed steps at the end of the output (needs a `NonTransforming` writer) | `writer::repeat::Repeat` (rust-docs id 54), `Cucumber::repeat_if` docs |
| `retries` / `retry_after` / `retry_filter` / `retry_options` | scenario retry policy (count, delay, tag-expression filter) | structure; `runner::basic::RetryOptions` |
| `fail_fast` | stop at the first failure | structure |
| `max_concurrent_scenarios(Some(n))` | cap concurrent scenarios | rust-docs `max_concurrent_scenarios` (id 289): "If `max` is `Some` number of concurrently executed `Scenario`s will be limited." |
| `which_scenario` | resolve `ScenarioType::{Serial, Concurrent}` per scenario; the default checks the `@serial` tag (see §4.6) | `source/src/runner/basic.rs:423-439,519` |
| `language` | Gherkin i18n keyword language | structure |
| `run(input) -> Wr` | run, and **return the writer** | `cucumber.rs:558-560`: `pub async fn run(self, input: I) -> Wr { self.filter_run(input, …).await }` |
| `filter_run(input, Fn(&Feature, Option<&Rule>, &Scenario) -> bool) -> Wr` | programmatic scenario filter | rust-docs id 271 (docs incl. example) |
| `run_and_exit` / `filter_run_and_exit` | exit-style: **panic** with a summary if anything failed | `cucumber.rs:1150-1246` (source below) |

### 4.4 Failure semantics / exit codes

`get_item_source(filter_run_and_exit)` returns `source/src/cucumber.rs:1206-1246`. When `writer.execution_has_failed()` holds, the code collects counts of failed steps, parsing errors, and hook errors, then calls **`panic!("{msg}")`**. A grep of cucumber `src/` shows **no `std::process::exit` anywhere**. The consequences:

- On a `harness = false` target: the panic makes the process exit **101**, and `cargo test` reports the target as failed. Success exits 0.
- On an in-process `#[test]`: a `run_and_exit` panic is an ordinary test failure.
- **`run()` (non-exiting) returns the writer.** You then assert on `execution_has_failed()`. That method is defined on `writer::Stats` at `source/src/writer/mod.rs:145-147` and used internally at `cucumber.rs:1216`. The default `Wr = Summarize<Normalize<Basic>>` implements it.
- Our clippy `exit = "deny"` and `panic = "deny"` (Cargo.toml:192,198) apply only to *our* code. Cucumber's panic lives inside the dependency.

### 4.5 Hooks

- `Cucumber::before(fn)`: "executed on each Scenario before running all its Steps, including Background ones. **Only one before hook can be registered; if multiple calls are made, only the last one will be run.**" (`rust-docs get_item_details(before, id 299)`). The signature (`source/src/cucumber.rs:1026-1037`, alias `BeforeHookFn` at `source/src/runner/basic.rs:280-285`) is:
  `for<'a> Fn(&'a gherkin::Feature, Option<&'a gherkin::Rule>, &'a gherkin::Scenario, &'a mut W) -> LocalBoxFuture<'a, ()>`.
- `Cucumber::after(fn)`: runs even after `Skipped`/`Failed` steps. It gets an extra `&event::ScenarioFinished` and an `Option<&mut World>` (the World only if it was initialized). The single-hook rule is the same (`rust-docs get_item_details(after, id 301)`, `AfterHookFn` at `basic.rs:287-295`).
- Hooks are `LocalBoxFuture`, so they are **single-threaded/local**. A multi-thread runtime needs a `LocalSet`. Cucumber's own tests use `#[tokio::test]` current-thread style.

### 4.6 Tags, name filters, serial execution

`rust-docs get_item_details(cli::Opts, id 21)` and `source/src/cli.rs:86-140` document the options:

- `--name/-n <regex>` (alias `--scenario-name`) filters by scenario name. The field is `re_filter`.
- `--tags/-t <tagexpr>` takes a Cucumber tag expression. It **conflicts with `--name`**. The env fallback is **`CUCUMBER_FILTER_TAGS`**. The field is `tags_filter`. Doc note: "Tags from Feature, Rule and Scenario are merged together on filtering."
- `filter_run(…)` takes a closure for programmatic filtering (§4.3).
- For serial execution: the default `which_scenario` closure scans scenario tags for `"serial"` and returns `ScenarioType::Serial` ("Run Scenarios sequentially (one-by-one)", rust-docs id 278). All other scenarios get `Concurrent` (`source/src/runner/basic.rs:423-439`). Override the closure with `Cucumber::which_scenario`.
- Retries interact with tags: `--retry-tag-filter <tagexpr>` limits which failed scenarios the runner retries (`runner::basic::Cli` field `retry_tag_filter`).

### 4.7 Step arguments: tables and docstrings

- `step::Context` = `{ step: gherkin::Step, matches: Vec<Capture…> }` (`rust-docs get_item_details(Context, id 1361)`, `source/src/step.rs:220-230`). On the macro side, capture arguments are `FromStr`. An argument named `step`, or annotated `#[step]`, receives the full `gherkin::Step`.
- `gherkin::Step` carries **`docstring: Option<String>`** and **`table: Option<gherkin::Table>`** fields plus `docstring()`/`table()` accessors (docs.rs/gherkin/0.16.0/gherkin/struct.Step.html). The `gherkin` crate is re-exported as `cucumber::gherkin` (`source/src/lib.rs:196`).
- Cucumber 0.23 has no separate `Table` type. Tables are `gherkin::Table` reached through the step or the context. (`cucumber::step` also exposes `HashableRegex`, `Location`, `AmbiguousMatchError`, and `Collection` for manual registration; see the rust-docs `structure` output.)

### 4.8 Async / runtime support

- Cucumber 0.23 depends on **`futures` only**. No tokio/smol feature exists (§2). The runner executes everything inside the future that you await.
- The cucumber suite runs steps under `#[tokio::test]` (packaged `source/tests/junit.rs:23`), and the README examples use `#[tokio::main]` (README.md:66-68). `tokio` is a dev-dep only.
- Therefore, with **sync step functions**, you drive `World::cucumber().run(…)` with `futures::executor::block_on`. You add `futures` as a dev-dep. Async steps that need timers or IO need a reactor. You then add `tokio` as a dev-dep, current-thread flavor, because hooks are `LocalBoxFuture`.

### 4.9 Reporters / output

- The default writer is `writer::Summarize<writer::Normalize<writer::Basic>>` (`cucumber.rs:805-811`): human-readable output plus summary stats plus ordering normalization. The normalization is required because scenarios run concurrently.
- **JUnit** (`output-junit`): `writer::JUnit::new(out: impl io::Write, verbosity)`. The docs warn that it "isn't `Normalized` by itself, so should be wrapped into a `writer::Normalize`, otherwise will panic" (`rust-docs get_item_details(JUnit, id 2206)`, `source/src/writer/junit.rs:60-87`). The CLI flag is `--junit…` verbosity (`writer::junit::Cli::verbose`, id 2207). The packaged example wiring is `.with_writer(writer::JUnit::new(file, 1))` (`source/tests/junit.rs:33`).
- **JSON** (`output-json`): the Cucumber ecosystem JSON format (`writer::json`, `RunResult` struct; rust-docs id 2030).
- **libtest** (`libtest`): libtest-style JSON output (`writer::libtest::Cli` fields `format`, `show_output`, `report_time`, `nightly`; rust-docs id 2294). The README says: "Useful for IntelliJ Rust plugin integration." It does **not** implement nextest's discovery protocol (§6.1).
- **tracing** (`tracing`): emits cucumber events through the `tracing` crate. This works with our existing `tracing`/`tracing-subscriber` dev-deps (Cargo.toml:148,164).
- The console writer knobs are: `writer::basic::Cli` (coloring), `Verbosity` (`ShowWorld`/`ShowWorldAndDocString`), `writer::discard::Stats|Arbitrary` (headless, `execution_has_failed`), and the `writer::{Tee, Or}` combinators (structure output).

### 4.10 Parallelism

- The runner CLI has **`--concurrency`, which defaults to 64** ("Number of scenarios to run concurrently. If not specified, uses the value configured in tests runner, or 64 by default." `rust-docs get_item_details(runner::basic::Cli, id 298)`, `source/src/runner/basic.rs:58-93`). The other runner flags are `--fail-fast`, `--retry N`, `--retry-after <humantime>`, and `--retry-tag-filter`.
- Scenarios run concurrently *within* one process. The `@serial` tag (or `which_scenario`) forces specific scenarios to run sequentially. The writer re-orders the events for output through `writer::Normalize`. The `Runner` trait docs (rust-docs id 15) state: "as Scenarios may be executed concurrently, events … can be interrupted … Writer … has to be `Normalized`".

## 5. Feature-file discovery and version control

- The entry point is `Cucumber::run(input)` / `World::run(input)`. The input is `impl AsRef<Path>`.
- `source/src/parser/basic.rs:63-121` handles the input as follows:
  - a **file** path → parse that file;
  - a **directory** path → recursive globwalk of **`*.feature`**, **sorted by path** (deterministic order, `parser/basic.rs:67`);
  - a CLI override: `--input <glob>` (arg id `input`, `parser::basic::Cli.features` field, rust-docs id 925: "Glob pattern to look for feature files with. If not specified, looks for `*.feature` files in the path configured in the test runner.").
- `path.canonicalize()` resolves relative paths against the **process cwd** (`parser/basic.rs:79-91`). Cargo and nextest run tests from the package root, so `"tests/features"` style paths work (check in the spike).
- The convention in the wild, and in cucumber's own docs and tests, puts features under `tests/features/…`. The README runs `"tests/features/readme"` (README.md:68). The packaged tests use `"tests/features/wait"` (`source/tests/junit.rs:54`). The docs example uses `"./tests/features/doctests.feature"` (codegen docs).
- `.feature` files are plain UTF-8 text, so they are trivially version-controlled in git next to the step-definition test crate. Gherkin i18n keywords are available through `Cucumber::language` (structure). The full Gherkin gamut (Feature/Rule/Scenario/Background/Examples/DocString/Table) comes from the `gherkin` crate (README.md:100).

## 6. Integration fit (this repo)

### 6.1 The decisive constraint: nextest

- This repo's canonical runner is **cargo-nextest** in every phase (`.mise/tasks/test/_default`, with three `cargo nextest` phases plus doctests; `.config/nextest.toml`; CI `.github/workflows/ci.yml:97-140` uses `cargo nextest run --no-tests fail` per phase).
- **cucumber is not nextest-compatible out of the box.** Nextest's custom harness contract requires `--list --format terse`, `--list --ignored …`, and `--nocapture --exact` support (nexte.st/docs/design/custom-test-harnesses/).
- Cucumber-rs/cucumber **issue #370 "Support for cargo-nextest" is open** (github.com/cucumber-rs/cucumber/issues/370, updated 2026-06-05). The maintainer explicitly declines the process-per-scenario model. That model would break `@serial`, re-parse features N times, and break summaries.
- Real-world evidence: hivellm/nexus commit `32071b2` shows nextest failing with exit **104 "creating test list failed"**. Their cucumber `harness = false` runner printed feature output during `--list`. Their fix: `default-filter = 'not binary(tck_runner)'` in `.config/nextest.toml` plus a separate `cargo test --test tck_runner` CI step.
- **Implication:** a cucumber target with `harness = false` (the crate's canonical README setup, README.md:72-76) must be *excluded from nextest and run as a separate plain-`cargo test` step*. The better option for this repo: **run cucumber in-process inside a normal `#[test]` (harness = true)**. That is an officially-exercised pattern. Cucumber's own packaged test `source/tests/junit.rs` runs a full cucumber run inside `#[tokio::test]` with the default harness, and `Cucumber::run` returns the writer precisely so the caller can assert (§4.4). In-process: nextest sees one ordinary test, keeps its process-per-test isolation, and keeps `--no-tests fail`, filter expressions, slow-timeout, and global-timeout. Nextest replays cucumber's captured stdout on failure (`success-output = "never"`, `failure-output` default immediate in `.config/nextest.toml`).

### 6.2 Existing test layout and where things would live

- The targets today: `tests/integration.rs` + `tests/integration/*` (lib API only, gated `required-features = ["test-utils"]`, Cargo.toml:49-51), `tests/e2e.rs` + `tests/e2e/*` (spawns the real binary, feature-off), and `tests/baselines/*` (public-API baselines). Single package, lib + bin (Cargo.toml:29-35, `src/lib.rs`, `src/main.rs`).
- The feature flag inventory: only `test-utils` (Cargo.toml:169-170).
- The dev-deps today: criterion, pretty_assertions, proptest, rstest, rustix, stats_alloc, syn, tempfile, tracing-subscriber (Cargo.toml:155-164). **No async runtime, no `futures`.**
- Suggested homes (steps only, not implemented):
  - `tests/features/*.feature`: version-controlled specs. This matches the cucumber convention (§5) and our `tests/` layout.
  - `tests/bdd.rs` + `tests/bdd/`: step definitions and World, styled after `tests/integration.rs` (in-process, lib-API driven, `test-utils`-gated).
  - an optional `[[test]] name = "bdd" required-features = ["test-utils"]` to inherit the silent-green-guard pattern (Cargo.toml:40-51).
- The layer gates need an extension: `mise check:layers` greps only `tests/{e2e.rs,e2e/,integration.rs,integration/}` (mise.toml:148-185). A new `tests/bdd/` needs equivalent rules (for example, "no `std::process` outside support.rs" and "no `set_current_dir`"). The `check:visibility` public-API baselines are unaffected by dev-dependencies (mise.toml:123-146).

### 6.3 Runner / CI wiring

- `mise run test` (`.mise/tasks/test/_default`) runs the phases unit (production config) → integration (`--features test-utils`, nextest filter `-E 'binary(integration)'`) → e2e → doctests. Empty filtered phases are skipped, but unfiltered runs must match something. Adding a separate `bdd` binary means either widening the integration filter (`binary(integration) + binary(bdd)`) or adding a dedicated phase. Use a nextest phase for in-process harness=true, or a plain `cargo test --test bdd` step for harness=false.
- CI (`.github/workflows/ci.yml:80-165`) has the jobs `test-unit`, `test-integration` (`--features test-utils -E 'binary(integration)'`), `test-e2e` (3 OS), and `test-doctests` (≥1 doctest enforced). A BDD suite rides the `test-integration` job's feature set. JUnit XML (feature `output-junit`) can then be uploaded as an artifact if wanted.
- Compile-time: cucumber and its ~99 non-dev packages land in the *dev/test* profile only. `Swatinem/rust-cache` in CI and cargo's local cache absorb the cost after the first build. The `--all-features` lint and doctest phases also compile the `output-json`/`output-junit`/`libtest`/`tracing` code paths if we enable those features.

### 6.4 Lint regime (Cargo.toml:175-261, clippy.toml)

- `allow-unwrap-in-tests = true` and `allow-expect-in-tests = true` (clippy.toml:85-87) let step functions in test targets call `unwrap()` freely. Our tests already do (276 occurrences under `tests/`).
- Our code still denies: `print_stdout`, `panic`, `exit`, `dbg_macro`, `indexing_slicing`, `arithmetic_side_effects`, and `missing_docs` (warn). Cucumber's own printing and panicking live in the dependency and do not lint. Our step bodies must avoid `println!`; use assertions or `tracing`.
- **Risk to check in the spike:** proc-macro output from `#[derive(World)]`/`#[given]` expands into *our* crate. If it generates `unwrap`/`expect`/`panic` at call-site spans outside `#[cfg(test)]` contexts (for example, a World impl in a non-test module), the deny lints fire. You may then need targeted `#[allow(..., reason = …)]`. Our `allow_attributes_without_reason = deny` means any allow needs a reason.

## 7. Risks / limitations

1. **nextest incompatibility (top risk)** (see §6.1): `harness = false` targets break discovery (exit 104 or silent skip). Upstream issue #370 stays open with no near-term fix. Mitigation: run in-process `#[test]` (harness=true), or add a separate `cargo test` phase plus a nextest `default-filter` exclusion.
2. **Compile-time / dependency weight**: ~99 packages (default features) in the test profile; the proc-macro stack (syn/quote/proc-macro2 via `cucumber-codegen`) is added to test builds. The cost is dev-only and cached, but the first `mise run test` and cold CI get slower. The `libtest`/`output-json`/`output-junit` features grow the tree to 133 packages (all features).
3. **Strict-lint collision with macro expansion**: see §6.4. You need a compile spike before commitment.
4. **Scenario concurrency defaults to 64**: Worlds are per-scenario and fresh, but tests that touch process-global state (env vars, cwd, fixed temp paths, global registries) must use per-scenario tempdirs, or set `max_concurrent_scenarios(Some(1))` / `@serial` tags. Our e2e sandbox discipline (tests/e2e/support.rs, `env::set_current_dir` banned, mise.toml:182-184) maps onto per-scenario `World` fixtures.
5. **Single `before`/`after` hook each (last call wins)**: there are no hook chains. You compose manually inside one hook.
6. **Exit-style helpers panic** (`run_and_exit`): the failure message is string-based ("2 steps failed, 1 parsing error"). Prefer `run()` plus `execution_has_failed()` for structured assertions.
7. **Maintenance status**: healthy. 0.23.0 was released 2026-04-23, with patch releases in Dec 2025. The gherkin 0.16 update is from 2026-09. The maintainers are active. But nextest support (#370) is a stated non-goal for now, and the API is pre-1.0: 0.x breaking minor bumps are the norm, and the CHANGELOG lists "BC Breaks" per minor.
8. **Ambiguous step matches** are runtime errors (`step::AmbiguousMatchError`, rust-docs structure). The global `inventory`-based registration means step names must be unique across the whole test crate.

## 8. Concrete adoption suggestion (steps only, nothing implemented)

1. **Spike (half-day):** add the dev-deps `cucumber = { version = "0.23", default-features = true }` plus `futures` for `block_on` (or `tokio` current-thread if async steps are wanted). Create `tests/bdd.rs` with `#[derive(World)]`, one literal or `regex` step, and `tests/features/smoke.feature`. Drive it **in-process**:
   `#[test] fn smoke() { futures::executor::block_on(async { let w = MyWorld::cucumber().run("tests/features").await; assert!(!w.execution_has_failed()); }) }`.
   Run `cargo test --test bdd` **and** `cargo nextest list -p traces` to prove that discovery works. Run `mise run lint` to flush out macro-vs-lint collisions (§6.4).
2. **Decide the harness:** the default proposal is an in-process harness-true test. It fits the nextest phases unchanged. Only if you need pretty un-captured output locally, add a `harness = false` target *plus* a nextest `default-filter = 'not binary(<bdd>)'` and a plain `cargo test --test <bdd>` step in `.mise/tasks/test/_default` and CI (mirror hivellm/nexus's shape).
3. **Wire the pipeline:** extend the integration phase filter, or add a `bdd` phase, in `.mise/tasks/test/_default`. Add CI coverage in the `test-integration` job (`--no-tests fail` semantics preserved). Keep the `required-features = ["test-utils"]` guard pattern if the steps use the `test-utils` surface.
4. **Extend `check:layers`** git-grep rules (mise.toml:148-185) to `tests/bdd.rs` / `tests/bdd/`: no `std::process`, no `set_current_dir`, lib-API-only if the suite is integration-style.
5. **Conventions:** `tests/features/*.feature` in git, one feature file per behavioral area; tag vocabulary (`@smoke`, `@slow`, `@serial`); filters via `--tags` / `CUCUMBER_FILTER_TAGS`; document the layout in `docs/agents/` next to the test-layer conventions.
6. **Optional follow-ups:** `output-junit` plus the `Tee`/`Normalize` writer for CI artifacts; the `tracing` feature to route cucumber events into the existing tracing setup; revisit if upstream lands nextest support (issue #370).
