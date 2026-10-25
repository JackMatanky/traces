# Research 08: Prior art for layer responsibilities

This note collects external primary sources for [ticket
08](../issues/08-layer-responsibilities.md). It examines how projects and
authorities separate BDD scenarios from other test layers.

## 1. Sources examined

Read directly:

- Cucumber documentation: *Writing better Gherkin*
  (`https://cucumber.io/docs/bdd/better-gherkin/`) and *Anti-patterns*
  (`https://cucumber.io/docs/guides/anti-patterns/`).
- Aslak Hellesøy: *The world's most misunderstood collaboration tool* (2014,
  `https://cucumber.io/blog/collaboration/the-worlds-most-misunderstood-collaboration-tool/`)
  and *BDD is not test automation* (2020,
  `https://cucumber.io/blog/bdd/bdd-is-not-test-automation/`).
- Dan North: *Introducing BDD* (Better Software, 2006,
  `https://dannorth.net/blog/introducing-bdd/`).
- Gojko Adzic: *Focus on key examples* (2014,
  `https://gojko.net/2014/05/05/focus-on-key-examples/`) and *Specification by
  Example* (Manning, 2011).
- Martin Fowler: *TestPyramid* (2012,
  `https://martinfowler.com/bliki/TestPyramid.html`) and *On the Diverse And
  Fantastical Shapes of Testing* (2021,
  `https://martinfowler.com/articles/2021-test-shapes.html`).
- Google Testing Blog: *Test Sizes* (Simon Stewart, 2010,
  `https://testing.googleblog.com/2010/12/test-sizes.html`) and *Just Say No to
  More End-to-End Tests* (Mike Wacker, 2015,
  `https://testing.googleblog.com/2015/04/just-say-no-to-more-end-to-end-tests.html`).
- Kent C. Dodds: *The Testing Trophy and Testing Classifications* (2018/2021,
  `https://kentcdodds.com/blog/the-testing-trophy-and-testing-classifications`).
- Rust reference and tools: The Cargo Book (*Package layout*, *cargo test*); The
  Rust Book (ch. 11); cargo-nextest documentation (`https://nexte.st`); The
  Proptest Book (`https://proptest-rs.github.io/proptest/`).
- Comparable CLI repositories: `BurntSushi/ripgrep`, `rust-lang/cargo`,
  `astral-sh/uv`, and `jj-vcs/jj`.
- Upstream `rstest-bdd` and `rstest-bdd-harness` 0.6.0 source in
  `.rust-docs/cache/crates/`.

## 2. BDD originators: what scenarios cover and omit

- **Scenarios describe user behavior, not technical mechanisms.** Dan North
  (2006) defined scenarios to capture acceptance criteria in domain language.
  Scenarios state *what* the system guarantees, not *how* it executes.
- **BDD is not test automation.** Aslak Hellesøy (2020) states that BDD
  discovers and formulates requirements. Automated scenarios are living
  documentation. Writing Gherkin after writing production code is an
  anti-pattern.
- **Do not use Cucumber for all tests.** Hellesøy (2014) warns that using
  Cucumber only as a test runner creates slow, brittle tests.
- **Focus on key examples, not full combinations.** Gojko Adzic (2014) shows
  that testing all input variations in scenarios harms readability. Teams must
  specify five to ten key examples per rule. Full combinatorial tests belong in
  unit tests.
- **Step definitions must model domain concepts.** Cucumber documentation warns
  against step definitions tied to one feature. Steps must be reusable across
  features.

## 3. Test-layer taxonomies: classification criteria

Authorities classify tests by operational traits, not slogans:

- **Scope and stack depth.** Martin Fowler (2012) separates low-level unit tests
  from broad-stack tests. High-level tests act as a second defense line. When a
  high-level test fails, developers must write a unit test to reproduce and
  repair the bug.
- **Solitary versus sociable unit tests.** Fowler (2021) shows that "unit test"
  has two meanings: solitary (mocked dependencies) and sociable (real in-memory
  collaborators). Debates about pyramids versus testing trophies usually confuse
  these definitions.
- **Execution constraints (Google test sizes).** Simon Stewart (2010) and Mike
  Wacker (2015) classify tests by strict runtime limits:
  - Small: single process, single thread, no disk, no network, <60 s (usually
    <100 ms).
  - Medium: single host, multiple processes, localhost network and disk
    permitted, <300 s.
  - Large: multiple hosts, external network and services permitted, <900 s.
- **Diagnostic precision and flakiness.** Wacker (2015) notes that end-to-end
  tests provide slow feedback, have high flakiness, and hide root causes. Google
  recommends a 70/20/10 ratio (small / medium / large).
- **Return on investment (ROI).** Kent C. Dodds (2018) prioritizes confidence
  per unit of maintenance effort. He favors sociable integration tests over
  heavily mocked unit tests.

## 4. Rust test target organization and tooling

- **Target organization and link cost.** The Cargo Book states that Cargo
  compiles each file in `tests/*.rs` as a separate crate. Many files create many
  binaries. This increases link time.
- **Submodule folding.** The Rust Book (ch. 11) recommends folding integration
  tests into submodules under one test binary (such as `tests/integration.rs` or
  `tests/integration/main.rs`).
- **Nextest execution model.** Cargo-nextest discovers tests per binary. It runs
  each test in its own process. Nextest supports filter expressions
  (`binary(...)`), test groups for concurrency, and `--no-tests fail` to prevent
  false passes.
- **Property testing versus example testing.** The Proptest Book states that
  example tests verify known cases. Property tests search for unknown failures
  by generating inputs against algebraic invariants (such as round-trips and
  parser idempotence).
- **Idiomatic CLI testing crates.** Mature Rust CLI tools use `assert_cmd` to
  test process execution, `snapbox` or `trycmd` for file fixtures, and `insta`
  for output snapshots.

## 5. Comparable Rust CLI projects

Mature Rust CLI tools avoid Gherkin. They use native Rust test harnesses and
snapshot testing:

- **`ripgrep`** (`BurntSushi/ripgrep`): Sets `autotests = false` in
  `Cargo.toml`. Compiles one integration binary (`tests/tests.rs`) with
  submodules (`feature.rs`, `regression.rs`). Uses a custom `TestCommand` in
  `tests/util.rs` to spawn the binary.
- **`cargo`** (`rust-lang/cargo`): Compiles integration tests into a single
  binary (`tests/testsuite/main.rs`). Uses a dedicated test-support crate
  (`crates/cargo-test-support`).
- **`uv`** (`astral-sh/uv`): Separates library unit tests (`src/`) from CLI
  integration tests (`crates/uv/tests/`). Uses `insta` snapshots and a shared
  `uv-test` support crate.
- **`jj`** (`jj-vcs/jj`): Places CLI tests in `cli/tests/`. Uses `insta`
  snapshots to assert command output and repository state.

## 6. Upstream `rstest-bdd` guidance

- **`rstest-bdd-harness` is an attribute emitter.** The crate
  `rstest-bdd-harness` (0.6.0) provides `AttributePolicy`. It does not replace
  the test runner (`harness = false`). It emits attributes such as
  `#[rstest::rstest]` or `#[tokio::test]` onto generated test functions.
- **Step registry is global per test binary.** Step definitions stay in an
  `inventory` registry per test binary. Steps defined in one binary are
  invisible to another binary.
- **Feature discovery.** The `scenarios!("path")` macro searches directories
  relative to `CARGO_MANIFEST_DIR`. It generates a module per feature and a test
  function per scenario.
- **Placement beside ordinary tests.** Scenarios expand to ordinary test
  functions. They run beside standard `#[test]` and `#[rstest]` functions in the
  same binary.
- **Filtering.** Scenarios support compile-time tag filters (`tags = "..."`).
  Runtime filtering uses standard nextest expressions (`-E 'test(...)'`).

## 7. Classification criteria catalogue

| Criterion | Primary Source | Unit (`src/`) | Property (`proptest`) | Integration (`tests/integration/`) | E2E (`tests/e2e/`) | BDD (`rstest-bdd`) |
|---|---|---|---|---|---|---|
| **Audience & Intent** | North (2006); Adzic (2011) | Developer: code correctness | Developer: invariants & edge cases | Developer: subsystem wiring | Operator: shipped CLI contracts | User/Domain: key business workflows |
| **Execution Seam** | Fowler (2012); Rust Book ch. 11 | Internal functions & private state | Pure functions & state machines | In-process public facade (`traces::testing`) | Spawned process (`Sandbox`) | Process boundary (default) or public facade |
| **Runtime Size** | Stewart (2010); Wacker (2015) | Small (<100 ms, single thread) | Small (<1 s, generated iterations) | Medium (<2 s, in-memory I/O) | Medium (<5 s, spawned process) | Medium (matches underlying driver) |
| **Isolation** | Fowler (2021); Google Sizes | Sociable in-memory, zero ambient env | Pure inputs, zero ambient env | Isolated tmpdir, in-process store | Hermetic sandbox, scrubbed env | Hermetic sandbox or isolated facade |
| **Input Coverage** | Adzic (2014); Proptest Book | Targeted edge cases & error paths | Broad generative input space | Typical subsystem integration flows | Critical path smoke flows | 5-10 curated key examples per rule |
| **Diagnostic Fidelity** | Wacker (2015); Fowler (2012) | High: exact file and line | High: minimal shrunk input | Medium: subsystem failure | Low: stderr, stdout, exit code | Medium: step name and failed assertion |

## 8. Implications for ticket 08

### Decision (a): Classification rule

The evidence supports this five-tier assignment rule:

1. **BDD scenarios (`rstest-bdd`):** Reserved for user-visible workflows that obey ticket 07's six-condition rule. Each rule specifies 5 to 10 key examples. Scenarios never test internal logic.
2. **Unit tests (`src/`):** Default home for code logic, combinatorial edge cases, parser rules, and private seams. Unit tests reproduce high-level failures (Fowler, 2012).
3. **Property tests (`proptest`):** Reserved for algebraic invariants, round-trip operations, and crash-freedom across large input spaces.
4. **Integration tests (`tests/integration/`):** Reserved for in-process composition of modules through `traces::testing`. Spawning child processes is banned.
5. **E2E tests (`tests/e2e/`):** Reserved for process-boundary verification in a hermetic sandbox. Access to `traces::` library symbols is banned.

### Decision (b): Where BDD scenarios physically run

- **Option 1: Fold into existing binaries.** Fold scenarios into `tests/integration/` and
  - *Evidence for:* Compiles zero extra test binaries; avoids extra link
    overhead; inherits `check:layers` gates; needs zero changes to nextest tasks
    or CI.
  - *Evidence against:* Step registries remain separate across binaries;
    integration steps cannot reuse e2e steps.
- **Option 2: Dedicated `tests/bdd.rs` binary with a new nextest phase.**
  - *Evidence for:* Clear physical isolation of Gherkin tests.
  - *Evidence against:* Adds link-time overhead for an extra test binary;
    requires modifying `.mise/tasks/test/_default`, `ci.yml`, and
    `nextest.toml`.
- **Note on `rstest-bdd-harness`:** Upstream inspection shows
  `rstest-bdd-harness` is not an alternative runner. It is an attribute-policy
  plug-in. It does not provide a third execution option.
- **Recommendation:** Option 1 (fold into existing binaries). *[Ticket 08
  decision: Fold into `tests/e2e.rs` only; defer integration facade driver until
  needed.]*

### Decision (c): Role and growth of property tests relative to scenarios

- **Evidence:** Proptest and Gherkin serve opposing needs. Proptest tests
  mathematical laws over thousands of generated inputs. Gherkin tests human
  understanding over five to ten key examples.
- **Recommendation:** Scenarios must never replace property tests. Property
  tests grow in modules with algebraic contracts (such as `temporal/`, parser
  grammars, and query algebra). *[Ticket 08 decision: Proptests stay inline in
  `src/`. Query-plan idempotence excluded.]*

### Decision (d): Integration/E2E split and disposition of

`integration-e2e-suite`

- **Disposition of `.scratch/integration-e2e-suite/`:**
  - *Accepted:* The hard E2E invariant (spawn only, hermetic sandbox, no
    `traces::` imports) and in-process integration discipline (no process
    spawns). The `check:layers` and `check:visibility` gates enforce this split
    mechanically.
  - *Adoption rule:* BDD scenarios do not change this split. Scenarios that
    spawn live under `tests/e2e/`. Scenarios that drive the in-process facade
    live under `tests/integration/`. *[Ticket 08 decision: Partially adopted.
    Invariants and gates bind; per-test classification does not.]*

### Decision (e): Anti-goals

The evidence identifies four layers this repo must NOT introduce:

1. **No mock layer:** Ban mock frameworks (such as `mockall`). Fowler and Google evidence shows heavy mocking degrades test fidelity. Use real in-memory types and sociable tests.
2. **No PTY or terminal emulator harness:** Testing interactive prompts (inquire) via PTYs is fragile. Keep TTY tests in unit tests with injected providers.
3. **No custom test harness:** Do not set `harness = false`. All tests must remain native libtest tests under nextest.
4. **No combinatorial BDD scenario suites:** Adzic (2014) identifies thousands of scenario outline rows as an anti-pattern. Keep combinatorial matrix tests in unit or property tests.
5. **No contract-test framework:** Reject extra contract-test layers.
6. **No unguarded test layer:** Every layer must have a gate or ledger to prevent silent failure.

### Open questions for the grilling session

1. Will the user accept folding BDD into existing binaries (Option 1), or do they demand a dedicated BDD phase in CI?
2. When a scenario could run against either driver, does the user prefer defaulting to E2E spawn or integration facade? (Ticket 07 decided spawned process is default).
3. Should existing `tests/e2e/` tests migrate to Gherkin, or should Gherkin be reserved for new specs?
