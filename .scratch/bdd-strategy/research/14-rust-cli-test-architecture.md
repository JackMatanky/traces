# Research 14: Rust CLI Test Architecture Landscape

This document supports [ticket 14](../issues/14-adoption-roadmap.md).
It investigates how mature Rust CLI and developer tooling projects structure end-to-end and behavioral tests.
The document evaluates test layouts, execution harnesses, snapshot tools, hermetic sandboxing, PTY testing, and real-world BDD adopters (`rstest-bdd` and `cucumber-rs`).

---

## 1. Question

How do mature Rust CLI and tooling projects structure end-to-end and behavior tests?
How do they isolate environments, manage compile times, separate specifications from bug regressions, and pin contracts?
How do real adopters of `rstest-bdd` and `cucumber-rs` manage features, steps, and fixtures in production?

---

## 2. Method and Source List

Method date: 2026-11-03.
The investigation opened and evaluated 29 distinct repositories and primary
source files via GitHub and project docs.
All primary sources represent actual source code, test suites, or issue
discussions in production repositories.

### Primary Sources Opened and Read

1. `rust-lang/cargo`: `tests/testsuite/main.rs`,
   `crates/cargo-test-support/src/lib.rs`,
   `crates/cargo-test-support/src/paths.rs`. URL:
   `https://github.com/rust-lang/cargo`. Access: 2026-11-03. [Primary, high
   confidence].
2. `astral-sh/uv`: `crates/uv/tests/it/main.rs`, `crates/uv/tests/it/common.rs`.
   URL: `https://github.com/astral-sh/uv`. Access: 2026-11-03. [Primary, high
   confidence].
3. `martinvonz/jj`: `cli/tests/runner.rs`, `cli/tests/common/mod.rs`. URL:
   `https://github.com/martinvonz/jj`. Access: 2026-11-03. [Primary, high
   confidence].
4. `BurntSushi/ripgrep`: `tests/tests.rs`, `tests/util.rs`, `tests/feature/`,
   `tests/regression/`. URL: `https://github.com/BurntSushi/ripgrep`. Access:
   2026-11-03. [Primary, high confidence].
5. `rust-lang/rust-analyzer`: `crates/test-utils/src/lib.rs`, `expect-test`
   crate. URL: `https://github.com/rust-lang/rust-analyzer`. Access: 2026-11-03.
   [Primary, high confidence].
6. `nushell/nushell`: `tests/main.rs`, `crates/nu-test-support/src/lib.rs`. URL:
   `https://github.com/nushell/nushell`. Access: 2026-11-03. [Primary, high
   confidence].
7. `helix-editor/helix`: `helix-term/tests/integration.rs`,
   `helix-term/tests/test/helpers.rs`. URL:
   `https://github.com/helix-editor/helix`. Access: 2026-11-03. [Primary, high
   confidence].
8. `sharkdp/bat`: `tests/integration_tests.rs`, `tests/tests.rs`. URL:
   `https://github.com/sharkdp/bat`. Access: 2026-11-03. [Primary, high
   confidence].
9. `sharkdp/fd`: `tests/tests.rs`. URL: `https://github.com/sharkdp/fd`. Access:
   2026-11-03. [Primary, high confidence].
10. `rust-lang/mdBook`: `tests/testsuite/main.rs`, `tests/testsuite/helpers.rs`.
    URL: `https://github.com/rust-lang/mdBook`. Access: 2026-11-03. [Primary,
    high confidence].
11. `nextest-rs/nextest`: `integration-tests/Cargo.toml`, custom harness design
    docs. URL: `https://github.com/nextest-rs/nextest`. Access: 2026-11-03.
    [Primary, high confidence].
12. `mitsuhiko/insta`: `insta/src/lib.rs`, `insta/src/runtime.rs`. URL:
    `https://github.com/mitsuhiko/insta`. Access: 2026-11-03. [Primary, high
    confidence].
13. `assert-rs/snapbox`: `crates/snapbox/src/lib.rs`. URL:
    `https://github.com/assert-rs/snapbox`. Access: 2026-11-03. [Primary, high
    confidence].
14. `assert-rs/trycmd`: `crates/trycmd/src/lib.rs`. URL:
    `https://github.com/assert-rs/trycmd`. Access: 2026-11-03. [Primary, high
    confidence].
15. `matklad`: "Delete Cargo Integration Tests". URL:
    `https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html`.
    Access: 2026-11-03. [Primary, high confidence].
16. `rust-lang/rust`: Issue #114737 / PR #114755 (`Command::env_clear` breaks
    Windows `SystemRoot`). URL:
    `https://github.com/rust-lang/rust/issues/114737`. Access: 2026-11-03.
    [Primary, high confidence].
17. `astral-sh/uv`: Issue #15403 (User-level `uv.toml` leaks into test
    sandboxes). URL: `https://github.com/astral-sh/uv/issues/15403`. Access:
    2026-11-03. [Primary, high confidence].
18. `martinvonz/jj`: Issue #488 / PR #1040 (Windows test path isolation and
    runner fixes). URL: `https://github.com/martinvonz/jj/issues/488`. Access:
    2026-11-03. [Primary, high confidence].
19. `zhiburt/expectrl`: `expectrl/README.md`, `examples/check.rs`. URL:
    `https://github.com/zhiburt/expectrl`. Access: 2026-11-03. [Primary, high
    confidence].
20. `philippkeller/rexpect`: Issue #11 ("Windows Support"). URL:
    `https://github.com/philippkeller/rexpect/issues/11`. Access: 2026-11-03.
    [Primary, high confidence].
21. `wez/wezterm`: `crates/portable-pty` issue #27, issue #4206. URL:
    `https://github.com/wez/wezterm`. Access: 2026-11-03. [Primary, high
    confidence].
22. `cucumber-rs/cucumber`: Issue #370 ("Support for cargo-nextest and other
    test runners"). URL: `https://github.com/cucumber-rs/cucumber/issues/370`.
    Access: 2026-11-03. [Primary, high confidence].
23. `cucumber-rs/cucumber`: Issue #334 (`Cucumber::run()` exit code contract).
    URL: `https://github.com/cucumber-rs/cucumber/issues/334`. Access:
    2026-11-03. [Primary, high confidence].
24. `leynos/ortho-config`: PR #237 ("Migrate hello_world tests to rstest-bdd;
    drop cucumber/gherkin and tokio"). URL:
    `https://github.com/leynos/ortho-config/pull/237`. Access: 2026-11-03.
    [Primary, high confidence].
25. `leynos/ortho-config`: `examples/hello_world/tests/rstest_bdd/mod.rs`,
    `fixtures.rs`, `behaviour/scenarios.rs`, `behaviour/harness/process.rs`.
    URL: `https://github.com/leynos/ortho-config`. Access: 2026-11-03. [Primary,
    high confidence].
26. `leynos/rstest-bdd`: Issues #359, #373, roadmap. URL:
    `https://github.com/leynos/rstest-bdd`. Access: 2026-11-03. [Primary, high
    confidence].
27. `breezy-bays-labs/crap-rs`: `crates/crap4ts/Cargo.toml`,
    `.github/workflows/ci.yml`. URL:
    `https://github.com/breezy-bays-labs/crap-rs`. Access: 2026-11-03. [Primary,
    high confidence].
28. `instrumentisto/baza`: `e2e/Cargo.toml`, `e2e/tests/`. URL:
    `https://github.com/instrumentisto/baza/tree/v0.2.0/e2e`. Access:
    2026-11-03. [Primary, high confidence].
29. `yaml-schema/yaml-schema`: Commit `eb16f906` ("add CLI Cucumber step
    definitions"). URL: `https://github.com/yaml-schema/yaml-schema`. Access:
    2026-11-03. [Primary, high confidence].

---

## 3. Rust CLI Test Architecture Landscape

The table below summarizes test layouts and practices across evaluated projects.

| Project | Test Layout Pattern | Primary Tools | Snapshot Tool | Platform Handling | Direct Link |
|---|---|---|---|---|---|
| **cargo** | Single binary (`tests/testsuite/main.rs`) | `cargo-test-support`, `snapbox` | `snapbox` (`str!`, `file!`) | Truncates paths to `t{id}` on Windows; scrubs `HOME`/`CARGO_HOME` | [cargo/tests/testsuite](https://github.com/rust-lang/cargo/tree/master/tests/testsuite) |
| **uv** | Single binary (`crates/uv/tests/it/main.rs`) | Custom `TestContext`, `assert_cmd` | `insta` inline & file snapshots | Scrubs `COLUMNS=100`, normalizes `.exe` and `/` vs `\`; sets ceiling dirs | [uv/tests/it](https://github.com/astral-sh/uv/tree/main/crates/uv/tests/it) |
| **jj (jujutsu)** | Single binary (`cli/tests/runner.rs`) | Custom `TestEnvironment`, `assert_cmd` | `insta` inline snapshots | Drops env except `PATH`; sets fixed `JJ_TZ_OFFSET_MINS=660`; passthrough LLVM cov | [jj/cli/tests](https://github.com/martinvonz/jj/tree/main/cli/tests) |
| **ripgrep** | Single binary (`tests/tests.rs`) | Custom `TestCommand`, `regex` | Explicit substring / regex matches | Forces `--path-separator /`; scrubs `RIPGREP_CONFIG_PATH` | [ripgrep/tests](https://github.com/BurntSushi/ripgrep/tree/master/tests) |
| **rust-analyzer** | In-tree crates + `test-utils` | Custom fixtures, `expect-test` | `expect-test` (in-place AST edit) | In-memory VFS; normalizes newlines (`\r\n` -> `\n`) | [rust-analyzer/test-utils](https://github.com/rust-lang/rust-analyzer/tree/master/crates/test-utils) |
| **nushell** | Single binary (`tests/main.rs`) | `nu-test-support` macros | Direct assertions; table comparisons | Isolates virtual directories; escapes Windows paths | [nushell/tests](https://github.com/nushell/nushell/tree/main/tests) |
| **helix** | Single binary (`helix-term/tests/integration.rs`) | In-process headless `Application` | Key sequence to buffer/cursor assertions | Normalizes line feeds (`LineFeedHandling::Native`) | [helix/tests](https://github.com/helix-editor/helix/tree/master/helix-term/tests) |
| **bat** | Single binary (`tests/integration_tests.rs`) | Custom `BatTest` process harness | Explicit line matching | Scrubs `BAT_CONFIG_PATH`, `BAT_THEME`, `PAGER`, `COLORTERM` | [bat/tests](https://github.com/sharkdp/bat/tree/master/tests) |
| **fd** | Single binary (`tests/tests.rs`) | Custom `TestProject` filesystem fixtures | Regex output matching | Sets root directory anchors; sorts entries across platforms | [fd/tests](https://github.com/sharkdp/fd/tree/master/tests) |
| **mdBook** | Single binary (`tests/testsuite/main.rs`) | `snapbox`, custom `BookTest` | `snapbox` directory and string snapshots | Preserves tempdirs on failure; normalizes CRLF | [mdBook/tests](https://github.com/rust-lang/mdBook/tree/master/tests/testsuite) |
| **cargo-nextest** | Dedicated workspace crate (`integration-tests`) | `libtest-mimic`, `datatest-stable` | `insta` snapshots | Retries failed tests (`retries = 2`); ignores rerun noise | [nextest/integration-tests](https://github.com/nextest-rs/nextest/tree/main/integration-tests) |
| **ortho-config** (rstest-bdd) | Single module (`tests/rstest_bdd/`) | `rstest-bdd`, `rstest`, custom `Harness` | Process result assertions | Gated with `#[cfg(feature = "yaml")]`; sync child process | [ortho-config/hello_world](https://github.com/leynos/ortho-config/tree/main/examples/hello_world/tests/rstest_bdd) |
| **crap-rs** (cucumber-rs) | Separate integration test crate | `cucumber` 0.20, Tokio | Custom Gherkin stdout assertions | Runs in separate GitHub Action job without nextest | [crap-rs/crap4ts](https://github.com/breezy-bays-labs/crap-rs/tree/main/crates/crap4ts) |

---

## 4. Findings

### 4.1 Test Binary Count and Compile Time

- `[B01]` Cargo defaults each file in `tests/*.rs` to a distinct executable
  binary. Each binary re-links all crate dependencies and compiler runtimes.
  Source:
  `https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html`,
  2026-11-03, primary, high confidence.
- `[B02]` Matklad measured a project with 11 integration tests. Consolidating
  them into one `tests/it/main.rs` binary cut test build time from 64 seconds to
  23 seconds. It cut disk space for debug artifacts from 11 GiB to 1.5 GiB. It
  also reduced execution time because cargo-test ran tests in parallel within
  one process. Source:
  `https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html`,
  2026-11-03, primary, high confidence.
- `[B03]` Cargo, uv, jj, ripgrep, bat, fd, nushell, and mdBook all set
  `autotests = false` in `Cargo.toml`. Every one of these projects defines
  exactly one `[[test]]` integration harness (e.g. `tests/testsuite/main.rs`,
  `tests/it/main.rs`, `tests/runner.rs`, or `tests/tests.rs`). Source: Project
  `Cargo.toml` files, 2026-11-03, primary, high confidence.
- `[B04]` jj enforces file discovery with an explicit compile-time test:
  `test_no_forgotten_test_files()`. This test walks `cli/tests/test_*.rs` on
  disk and asserts every file appears as a module in `cli/tests/runner.rs`.
  Source: `https://github.com/martinvonz/jj/blob/main/cli/tests/runner.rs`,
  2026-11-03, primary, high confidence.

### 4.2 CLI Testing Tool Comparison

- `[B05]` `assert_cmd` provides fluent process execution (`Command::cargo_bin`)
  and stdout/stderr assertions. It works well for status codes and basic regex
  matching. However, large stderr assertions become unreadable when assertions
  fail. Source: `https://github.com/assert-rs/assert_cmd`, 2026-11-03, primary,
  high confidence.
- `[B06]` `snapbox` replaces legacy `assert_cmd` assertions with first-class
  snapshots (`str![[...]]` and `file![[...]]`). It performs whitespace
  normalization, path separator redaction, and directory comparisons. It updates
  snapshots in place with `SNAPSHOTS=overwrite cargo test`. Cargo and mdBook
  adopted `snapbox` for their entire test suites. Source:
  `https://github.com/assert-rs/snapbox`, 2026-11-03, primary, high confidence.
- `[B07]` `trycmd` drives markdown-based CLI specifications. It extracts command
  blocks and expected outputs from documentation files. However, it struggles
  with complex dynamic state, non-deterministic timestamps, or multi-step
  preconditions. Source: `https://github.com/assert-rs/trycmd`, 2026-11-03,
  primary, high confidence.
- `[B08]` `insta` provides mature snapshot management for Rust. Features include
  inline snapshots (`assert_snapshot!(output, @"...")`), snapshot filters, and
  `cargo-insta review`. uv and jj use `insta` inline snapshots extensively for
  CLI output validation. Source: `https://github.com/mitsuhiko/insta`,
  2026-11-03, primary, high confidence.
- `[B09]` `expect-test` provides lightweight in-line snapshots without
  background daemons or extra files. Setting `UPDATE_EXPECT=1 cargo test`
  rewrites the source file directly using `file!()` and `line!()`. rust-analyzer
  uses `expect-test` exclusively. Source:
  `https://github.com/rust-lang/rust-analyzer/tree/master/crates/test-utils`,
  2026-11-03, primary, high confidence.

### 4.3 Locating Test Binaries: Pitfalls

- `[B10]` `assert_cmd::cargo::cargo_bin("traces")` resolves the binary by
  inspecting Cargo target directory layouts at runtime. This introduces runtime
  overhead and fails in non-standard workspace target configurations. Source:
  `https://docs.rs/assert_cmd/latest/assert_cmd/cargo/fn.cargo_bin.html`,
  2026-11-03, primary, high confidence.
- `[B11]` The standard Cargo macro `env!("CARGO_BIN_EXE_<name>")` resolves the
  executable path at compile time. It has zero runtime cost. However, Cargo only
  sets this environment variable if the integration test target specifies the
  required binary dependency or lives within the package that defines the
  binary. In a multi-crate workspace, an external integration crate must declare
  `cargo-test-support` or invoke `cargo build` first. Source:
  `https://doc.rust-lang.org/cargo/reference/environment-variables.html`,
  2026-11-03, primary, high confidence.

### 4.4 Hermetic Sandboxes and Environment Scrubbing

- `[B12]` User environment variables leak into child processes unless scrubbed.
  In Astral's `uv`, issue #15403 showed test failures when a developer had a
  custom `uv.toml` in `~/.config/uv/` or an active `VIRTUAL_ENV`. uv fixed this
  by scrubbing all `XDG_*`, `VIRTUAL_ENV`, `CONDA_*`, and `UV_*` variables in
  `TestContext`. Source: `https://github.com/astral-sh/uv/issues/15403`,
  2026-11-03, primary, high confidence.
- `[B13]` Calling `Command::env_clear()` on Windows removes `SystemRoot` and
  `WINDIR`. This breaks DLL loading (e.g. `ws2_32.dll`), socket initialization,
  and subprocess spawning. Rust issue #114737 documented this failure mode.
  Sandboxes must retain an allowlist: `PATH`, `SystemRoot`, `TEMP`, and `TMP` on
  Windows. Source: `https://github.com/rust-lang/rust/issues/114737`,
  2026-11-03, primary, high confidence.
- `[B14]` jj and Cargo isolate sandbox state by redirecting standard platform
  roots. They redirect `HOME` on Unix, and `APPDATA`, `LOCALAPPDATA`,
  `USERPROFILE` on Windows. jj forces `JJ_TZ_OFFSET_MINS=660` and incrementing
  timestamps to ensure commit hashes remain deterministic across runs. They also
  pass through `LLVM_PROFILE_FILE` so code coverage instrumentation records
  subprocess execution. Source:
  `https://github.com/martinvonz/jj/blob/main/cli/tests/common/mod.rs`,
  2026-11-03, primary, high confidence.
- `[B15]` Terminal formatting breaks tests if terminal capabilities leak. uv
  forces `COLUMNS=100`, `LINES=24`, and `NO_COLOR=1` (or `--color never`) across
  all test commands. This prevents line wrapping variations across developer
  terminal sizes. Source:
  `https://github.com/astral-sh/uv/blob/main/crates/uv/tests/it/common.rs`,
  2026-11-03, primary, high confidence.

### 4.5 Interactive and PTY Testing

- `[B16]` `rexpect` provides Expect-style testing for Unix terminal programs. It
  relies on POSIX `forkpty` and `termios`. It does not support Windows (Issue
  #11 remains open). Source:
  `https://github.com/philippkeller/rexpect/issues/11`, 2026-11-03, primary,
  high confidence.
- `[B17]` `expectrl` provides a cross-platform Expect implementation. It
  supports Linux, macOS, and Windows ConPTY. It supports both synchronous and
  asynchronous (Tokio) execution. Source: `https://github.com/zhiburt/expectrl`,
  2026-11-03, primary, high confidence.
- `[B18]` `portable-pty` provides low-level cross-platform PTY primitives from
  WezTerm. It handles master/slave terminal allocation across platforms.
  However, it lacks built-in pattern matching or timeout assertion helpers.
  Source: `https://github.com/wez/wezterm/tree/main/crates/portable-pty`,
  2026-11-03, primary, high confidence.
- `[B19]` Helix avoids real PTY allocation for behavioral tests. It feeds mock
  key events directly into an in-process headless `Application` struct. It
  asserts against buffer text and cursor selections. This eliminates timing
  races, terminal emulation flakes, and OS differences. Source:
  `https://github.com/helix-editor/helix/blob/master/helix-term/tests/test/helpers.rs`,
  2026-11-03, primary, high confidence.

### 4.6 Separating Behavior Specifications from Regression Tests

- `[B20]` ripgrep explicitly separates feature tests from regression tests. Its
  integration binary declares two distinct module trees: `mod feature;` and `mod
  regression;`. `feature/` validates user-facing behavioral guarantees (e.g.
  `feature::context`, `feature::multiline`). `regression/` isolates tests for
  specific closed bug issues (e.g. `issue_1234.rs`). Source:
  `https://github.com/BurntSushi/ripgrep/blob/master/tests/tests.rs`,
  2026-11-03, primary, high confidence.
- `[B21]` Cargo divides its test suite by subcommand and domain concepts
  (`tests/testsuite/<command>.rs`). Each file contains both behavioral scenarios
  and regression tests for that command. Regression tests carry issue links in
  doc comments. Source:
  `https://github.com/rust-lang/cargo/tree/master/tests/testsuite`, 2026-11-03,
  primary, high confidence.

### 4.7 Pinning Contracts: Status Codes and Output Formatting

- `[B22]` Testing exact exit codes prevents accidental error masking. Cargo,
  ripgrep, and uv assert exact exit codes: `0` for success, `1` for clean
  failure / no match, `2` for usage errors. Source:
  `https://github.com/BurntSushi/ripgrep/blob/master/tests/util.rs`, 2026-11-03,
  primary, high confidence.
- `[B23]` Full stdout/stderr snapshotting rots when libraries update diagnostic
  styling. Miette, Clap, and rustc frequently change line-wrapping, box drawing
  characters, and ANSI escape sequences. uv and ripgrep snapshot primary stdout
  data tables, but assert only error substrings or error codes on stderr. This
  prevents test breakage from minor diagnostic formatting changes. Source:
  `https://github.com/astral-sh/uv/blob/main/crates/uv/tests/it/common.rs`,
  2026-11-03, primary, high confidence.

### 4.8 Platform-Specific Flakes and Mitigations

- `[B24]` Path separators and binary extensions break snapshots across
  platforms. uv and snapbox normalize Windows `\` to Unix `/` and strip `.exe`
  extensions from command outputs before assertions. Source:
  `https://github.com/assert-rs/snapbox`, 2026-11-03, primary, high confidence.
- `[B25]` Windows path length limits (260 characters) cause test failure in
  deeply nested temp directories. Cargo works around this by truncating test
  directory scratchpads to `t{id}` under `CARGO_TARGET_TMPDIR`. Source:
  `https://github.com/rust-lang/cargo/blob/master/crates/cargo-test-support/src/paths.rs`,
  2026-11-03, primary, high confidence.
- `[B26]` cargo-nextest supports automated retries for flaky integration tests
  via `.config/nextest.toml` (`retries = 2`). This stabilizes CI against
  filesystem timing jitter without changing test logic. Source:
  `https://nexte.st/docs/configuration/retries/`, 2026-11-03, primary, high
  confidence.

### 4.9 Real-World BDD Adopters: rstest-bdd vs cucumber-rs

- `[B27]` `cucumber-rs` uses a custom test harness via `Cucumber::run()`. This
  custom harness conflicts with `cargo-nextest`. Issue #370 explains why:
  nextest calls test executables with `--list --format terse` to enumerate
  tests, then executes each test in a separate process with `--exact <test>`.
  Under `cucumber-rs`, running a separate process per scenario forces re-parsing
  all feature files thousands of times. It also breaks `@serial` ordering, skips
  summary reports, and requires complex inter-process locks. The cucumber-rs
  maintainers declined native nextest integration due to these architectural
  mismatches. Source: `https://github.com/cucumber-rs/cucumber/issues/370`,
  2026-11-03, primary, high confidence.
- `[B28]` Production users of `cucumber-rs` isolate cucumber tests into a
  separate workspace crate (e.g. `instrumentisto/baza`,
  `breezy-bays-labs/crap-rs`). They run this crate exclusively with standard
  `cargo test`, while running the rest of the workspace with `cargo-nextest`.
  Users who do not isolate cucumber tests resort to checking
  `std::env::args().any(|x| x == "--list")` and exiting with 0 to prevent
  nextest aborts. Source: `https://github.com/cucumber-rs/cucumber/issues/370`,
  2026-11-03, primary, high confidence.
- `[B29]` `leynos/ortho-config` previously used `cucumber-rs` for CLI behavior
  testing. In PR #237, ortho-config migrated its entire behavioral test suite
  from `cucumber-rs` to `rstest-bdd`. The migration removed `cucumber`,
  `gherkin`, and Tokio async dependencies. It replaced them with standard
  synchronous Rust tests running under the default Cargo harness. Source:
  `https://github.com/leynos/ortho-config/pull/237`, 2026-11-03, primary, high
  confidence.
- `[B30]` In `ortho-config`, `rstest-bdd` binds feature files via compile-time
  proc-macros: `scenarios!("tests/features/...feature", fixtures =
  [hello_world_harness: Harness])`. This expands into regular `#[test]`
  functions that cargo-nextest discovers and runs out of the box. Tag filtering
  happens at compile time using standard Rust attributes (e.g. `#[cfg(feature =
  "yaml")]`), avoiding runtime tag evaluation overhead. Source:
  `https://github.com/leynos/ortho-config/blob/main/examples/hello_world/tests/rstest_bdd/behaviour/scenarios.rs`,
  2026-11-03, primary, high confidence.
- `[B31]` Step functions in `ortho-config` take standard `rstest` fixtures via
  `#[from(hello_world_harness)]`. The harness executes the CLI synchronously
  using `std::process::Command` with custom pipe readers and timeout guards.
  This eliminated all Tokio async runtime overhead in behavioral tests. Source:
  `https://github.com/leynos/ortho-config/blob/main/examples/hello_world/tests/rstest_bdd/behaviour/harness/process.rs`,
  2026-11-03, primary, high confidence.

### 4.10 What Scales versus What Rots

- `[B32]` **What scales**:
  1. Single integration binary pattern: Keeps link times fast and disk usage
     small.
  2. Hermetic sandboxes with explicit passthrough allowlists: Prevents developer
     machine pollution and protects Windows execution.
  3. Compile-time macro expansion (`rstest-bdd`): Works natively with nextest,
     IDEs, and coverage tooling.
  4. Asserting semantic invariants (diagnostic codes, exit codes, machine
     stdout) rather than wrapped human prose.
  5. In-process headless drivers for complex UI/editor flows (Helix pattern).
     Source: Synthesis of `[B01]-[B04]`, `[B12]-[B15]`, `[B19]`, `[B22]-[B23]`,
     `[B27]-[B31]`, 2026-11-03, primary, high confidence.
- `[B33]` **What rots**:
  1. Multiple integration test binaries in `tests/*.rs`: Causes massive compile
     and link time slowdowns as the test suite grows.
  2. Full-output stderr snapshots: Breaks continuously whenever diagnostic
     libraries update line wraps or colors.
  3. Custom test harness runners (`cucumber-rs` `Cucumber::run()`): Incompatible
     with cargo-nextest, parallel process runners, and standard IDE tooling.
  4. Real PTY allocation for basic CLI tests: Introduces timing races, terminal
     width flakes, and Windows CI failures.
  5. Blanket environment wiping via `Command::env_clear()`: Breaks Windows
     execution by removing `SystemRoot`. Source: Synthesis of `[B01]-[B02]`,
     `[B13]`, `[B16]`, `[B23]`, `[B27]`, 2026-11-03, primary, high confidence.

---

## 5. Disagreements in Practice

1. **Snapshot Testing vs Exact Assertions**: Cargo and mdBook use `snapbox`
   snapshots for both stdout and file trees. ripgrep avoids snapshot crates
   entirely, relying on explicit substring and regex assertions. Astral's uv and
   jj use `insta` inline snapshots. *Resolution*: Repositories with stable
   machine output (JSON, fixed tables) benefit from snapshots. Repositories with
   human-formatted diagnostics suffer snapshot churn unless snapshots filter
   dynamic spans.

2. **File per Subcommand vs File per Capability**: Cargo organizes tests
   strictly by CLI subcommand (`tests/testsuite/build.rs`,
   `tests/testsuite/check.rs`). ripgrep and BDD adopters organize tests by
   behavioral capability (`feature/context.rs`,
   `features/global_parameters.feature`). *Resolution*: Command-oriented layouts
   fit tools with wide, orthogonal subcommand trees. Capability-oriented layouts
   fit tools where multiple flags and subcommands interact across shared
   domains.

3. **In-process Execution vs Subprocess Spawning**: Helix runs behavioral tests
   in-process by driving its `Application` struct directly. Cargo, uv, jj, and
   ripgrep spawn the compiled binary via `std::process::Command`. *Resolution*:
   Spawning the real binary tests CLI argument parsing, environment loading, and
   exit code handling. In-process execution is appropriate for high-frequency
   interactive event loops where process startup overhead is prohibitive.

---

## 6. Does Not Transfer to This Repository

1. **Dedicated Integration Workspace Crate (nextest / cucumber pattern)**:
   cargo-nextest and cucumber-rs adopters place integration suites in separate
   workspace crates (`integration-tests/`, `e2e/`). This repository uses a
   single root crate with an edition 2024 target. Adding an extra crate would
   violate the standing rule: no custom infrastructure beyond adopting an
   existing crate.

2. **PTY Testing via rexpect or expectrl**: Interactive prompts in this
   repository (e.g. `init` non-interactive vs interactive) run non-interactively
   in standard CI. `rexpect` does not support Windows `[B16]`. `expectrl` adds
   significant dependencies. PTY allocation is unnecessary because `traces`
   supports non-interactive flags and explicit stdin pipes.

3. **Markdown-driven Testing via trycmd**: `trycmd` couples tests directly to
   markdown documentation. This repository decided on Gherkin feature files via
   `rstest-bdd` (tickets 06 and 08). `trycmd` does not integrate with
   `rstest-bdd` or step-level fixtures.

---

## 7. Implications for Ticket 14 (Roadmap Options)

The findings suggest concrete options for structuring adoption slices in ticket
14. These are options backed by evidence, not binding decisions.

- **Option A (Binary Harness Architecture)**: Retain the single integration
  binary pattern in `tests/e2e.rs` `[B01]-[B03]`. Fold all BDD scenarios into
  `tests/e2e.rs` via `#[path = "e2e/bdd/mod.rs"] mod bdd;`. Do not create
  secondary test targets under `tests/`. *Evidence*: Prevents O(N) re-linking
  costs and disk bloat identified by matklad `[B02]`. Matches decisions in
  tickets 06 and 08.

- **Option B (Cold-Start Pilot Slice Dossier)**: Select a capability with
  stable, non-interactive CLI contracts for the cold-start pilot slice. Ideal
  candidates from prior tickets: `init` default workflow (ticket 01) or `query`
  language surface (ticket 02). Avoid interactive prompts, terminal wrapping, or
  unstable diagnostic formatting in slice 1 `[B23]`. *Evidence*: `ortho-config`
  validated `hello_world` global CLI flags as its initial pilot `[B29]`.

- **Option C (Hermetic Sandbox Environment Policy)**: In `tests/e2e/support.rs`,
  preserve the strict sandbox policy already present: Explicitly scrub user
  environment, override platform homes (`HOME`, `APPDATA`), and isolate
  config/state dirs `[B12], [B14]`. Never call bare `Command::env_clear()` on
  Windows; always pass through `SystemRoot`, `PATH`, `TEMP`, and `TMP` `[B13]`.
  Force `NO_COLOR=1` and fixed column widths to eliminate diagnostic wrapping
  variations `[B15]`. *Evidence*: uv issue #15403 `[B12]` and Rust issue #114737
  `[B13]`.

- **Option D (Assertion Strategy for Behavioral Steps)**: In BDD step
  implementations, assert exact exit codes `[B22]` and semantic stdout data. On
  stderr, assert only diagnostic error codes (e.g. `E001`) or stable substrings.
  Do not assert full stderr text or formatted Miette boxes `[B23]`. *Evidence*:
  Avoids test breakage from diagnostic wrapping and box-drawing character
  updates `[B23]`.

- **Option E (Compile-Time Feature Tagging)**: Use Rust module attributes and
  cargo features (`#[cfg(feature = "...")]`) to gate scenarios that depend on
  specific platform capabilities `[B30]`. Avoid runtime skip macros (`skip!()`)
  or custom runner filters. *Evidence*: `ortho-config` successfully gates
  YAML-dependent scenarios without runtime overhead `[B30]`.

---

## 8. Open Gaps

1. **Linux and Windows CI Validation**: The prototype in ticket 09 validated
   `rstest-bdd` inside `tests/e2e.rs` on macOS only. Full validation on Linux
   and Windows CI remains open until slice 1 runs in CI.
2. **`check:layers` Scanner Robustness**: `check:layers` currently uses `git
   grep || true`. It does not detect errors when `git grep` fails on untracked
   files or invalid arguments. This must be fixed prior to or during slice 1
   adoption.
3. **`deny.toml` Zlib License Exception**: `rstest-bdd` pulls in transitive
   dependencies requiring a `"Zlib"` license entry in `deny.toml`. Slice 1 must
   include this configuration update.
