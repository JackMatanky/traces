# Research: Precedents in Error Architecture, Component Topology, Diagnostics, and Test Seams

> **Re-verification note (main @ `207cc3fb`).** Counts changed after `207cc3fb`: `traces::testing` now exposes **46** identifiers (was 45), `tests/baselines/pub-tokens.txt` is **1,550** lines (was 1,528), `public-api-test-utils.txt` **4,444** (was 4,306), `public-api-default.txt` unchanged (2,555). `src/lib.rs:177-210` is now `:178-211`. The merge itself edited the baselines and changed a public `testing` type (`RefreshReport` struct to enum), which is a live example of the frozen-baseline friction this file describes.

Research ticket: `.scratch/arch-design/issues/08-research-errors-topology-precedent.md`  
Scope: Comparative study of error handling, diagnostics, crate boundaries, and test seams across 7 mature Rust codebases: **rustc**, **Ruff**, **uv**, **Cargo**, **gitoxide**, **rumdl**, and **jj**. All citations reference real source paths and symbols verified from shallow clones.

---

## 1. Verified Implementation Across Comparables

### rustc (Compiler & Tooling Engine)

- **Diagnostic ownership & codes [V]**: Diagnostic codes are owned by the compiler infrastructure, not the driver. `rustc_errors::codes::ErrCode` (`compiler/rustc_errors/src/codes.rs:17`) is a newtype index formatting as `E{:04}` backed by a static `DIAGNOSTICS` table generated from `rustc_error_codes`.
- **Diagnostic representation [V]**: Compiler passes derive `Diagnostic<'a>` (`compiler/rustc_errors/src/diagnostic.rs:28`) on pass-specific structs. Emission yields `Diag<'a>` (`compiler/rustc_errors/src/diagnostic.rs:354`), which contains `DiagCtxtHandle<'a>` and `Option<Box<DiagInner>>`.
- **Invariants [V]**: `impl !Clone for Diag<'_> {}` (`diagnostic.rs:373`) guarantees diagnostics cannot be accidentally emitted twice. Dropping an unconsumed `Diag` panics unless emitted or cancelled (`diagnostic.rs:360-365`). `Diag` is bounded to `3 * size_of::<usize>()` via boxing (`diagnostic.rs:375`).
- **Spans & Help [V]**: Spans are represented as lightweight `rustc_span::Span` (8 bytes, global `BytePos` offset). `DiagInner` holds `MultiSpan`, `Vec<SubDiagnostic>`, and `Suggestions` (`CodeSuggestion`, `Applicability`). Spans and suggestions remain strongly typed throughout compilation and are never flattened to strings before rendering.

### Ruff (Python Linter, Formatter & Language Server)

- **Topology [V]**: Multi-crate workspace with ~40 crates (`Cargo.toml:1-50`). Core execution crates (`ruff_linter`, `ruff_workspace`, `ruff_formatter`) are strictly decoupled from both the CLI binary (`crates/ruff`) and the LSP binary (`crates/ruff_server`).
- **Diagnostic vs Operational Error separation [V]**:
  - *Diagnostics*: Domain crates produce structured `ruff_db::diagnostic::Diagnostic` (`crates/ruff_db/src/diagnostic/mod.rs:30`), wrapping `Arc<DiagnosticInner>` with `DiagnosticId`, `Severity`, `DiagnosticMessage`, annotations with spans/ranges, and `Option<Fix>`. Rendered via `ruff_annotate_snippets` (`crates/ruff_db/src/diagnostic/render.rs`).
  - *Operational errors*: CLI operational failures (config reading, argument parsing, file I/O) return `anyhow::Result`.
- **CLI presentation & BrokenPipe [V]**: `crates/ruff/src/main.rs:51-78` (`report_error(err: &anyhow::Error) -> ExitCode`) loops over `err.chain()`. If any cause is `std::io::ErrorKind::BrokenPipe`, it exits with 0 (`:56-61`). It prints `ruff failed` with formatted causes using `writeln!(stderr, ...).ok()` to prevent panics when stderr itself is closed (`:64-75`).
- **Test & Bench seams [V]**: Micro-benchmarks live in a dedicated workspace crate `crates/ruff_benchmark` (`Cargo.toml:20-28`), depending on `ruff_db = { features = ["testing"] }` and optional components (`ruff_linter`, `ruff_python_parser`). Integration tests live in dedicated crates (`crates/*_integration_tests`, `crates/ruff_mdtest`). Internal APIs are never exposed through a public CLI facade.

### uv (Python Package Manager & Workspace Host)

- **Topology [V]**: Fine-grained workspace of 92 crates (`crates/uv-*`). Every crate defines a local `thiserror` enum (e.g., `uv_resolver::ResolveError`, `uv_lock::LockError`, `uv_installer::Error`).
- **miette-free Presentation via `Hinted` [V]**: No `miette` dependency. `crates/uv-errors/src/lib.rs:17` defines `pub trait Hinted { fn hints(&self) -> Hints<'_>; }`. Hints carry an ordering (`HintOrdering::{First, Any, Last}`, `:25-34`).
- **Error Presentation Facade [V]**: `crates/uv/src/commands/diagnostics.rs:27-68` implements `hints_for_error(err: &anyhow::Error) -> Hints<'static>`. It walks `err.chain()` and downcasts causes to known domain types implementing `Hinted`. Output is rendered via `uv_errors::write_error_chain_with_options` (`crates/uv-errors/src/lib.rs:349-363`) with line wrapping, level prefixes, and colored hint blocks.
- **Test seams [V]**: Dedicated `crates/uv-test` and `crates/uv-bench` crates inside the workspace.

### Cargo (Rust Package Manager)

- **Topology [V]**: Hybrid architecture. The root `cargo` crate (~80k SLoC) contains the core engine and CLI commands. Reusable utilities are split into leaf crates: `crates/cargo-util` (process execution, paths, file locks), `crates/cargo-util-schemas`, `crates/cargo-util-terminal`, and `crates/cargo-platform`.
- **Error model [V]**: `src/util/errors.rs:11` defines `pub type CargoResult<T> = anyhow::Result<T>`. Internal errors attach string context via `.with_context(|| ...)`.
- **CLI error presentation [V]**: `src/lib.rs:175-192` (`exit_with_error(err: CliError, shell: &mut Shell) -> !`). `CliError` is a simple struct `{ error: Option<anyhow::Error>, exit_code: i32 }` (`:186`). It downcasts through `err.chain()` for `clap::Error`, `InternalError` (printing bug report URLs, `:198-211`), and `VerboseError` (truncating chains in non-verbose mode, `:222-230`).
- **Test seams [V]**: Dedicated `crates/cargo-test-support` crate (`Cargo.toml:37`). Integration tests in `tests/testsuite/*.rs` and benchmarks in `benches/benchsuite` import `cargo_test_support::*`. Cargo exposes zero private engine types under `cfg(test)` feature flags in its root crate.

### gitoxide (Git Implementation & Tooling)

- **Topology [V]**: 60+ modular crates (`gix-*`). Plumbing crates define localized `thiserror` enums (`gix_config::Error`, `gix_odb::find::Error`). The high-level facade `gix` re-exports a unified error layer from `gix-error` (`gix/src/lib.rs:179`).
- **Error Classification & Recovery [V]**: `gix-error` (`gix-error/src/lib.rs:9-25`) establishes an explicit recovery `Class` enum: `Cancelled`, `Corruption`, `ResourceExhaustion`, `Validation`, `Unsupported`, `Unauthenticated`, `PermissionDenied`, `Conflict`, `NotFound`, `Retryable`. Callers recover based on remedy class rather than string matching or exhaustive enum arms (`:22-31`).
- **Test seams [V]**: Shared fixture and repository test utilities live in `tests/tools/Cargo.toml` as `gix-testtools` (`version = "0.22.0"`).

### rumdl (Markdown Linter & Formatter)

- **Topology [V]**: Single crate with 341,245 SLoC in `src/` (`Cargo.toml:1-27` defines `[package] name = "rumdl"`, `[lib] name = "rumdl_lib"`, `[[bin]] name = "rumdl"`). No workspace.
- **Diagnostic vs Error model [V]**: Entire 341k SLoC has only ~6 error enums (`LintError`, `ConfigError`, `ExecutorError`). Lint issues are represented as `LintWarning` (`src/rule.rs:38`), containing 1-indexed character-based line/column spans, severity, rule name, and an atomic `Fix` with paired `additional_edits` (`:56-63`).
- **Test & Bench seams [V]**: `src/lib.rs:50-63` exposes internal modules (`config`, `lint_context`, `rules`, `rule`) directly as `pub mod`. Benchmarks in `benches/*.rs` and tests in `tests/*.rs` import `rumdl_lib::*` directly. No `test-utils` feature, no `#[expect(private_interfaces)]`, and zero baseline token tracking.

### jj (Jujutsu VCS)

- **Topology [V]**: 7-crate workspace (`Cargo.toml:3-13`): `cli` (`jj-cli`), `lib` (`jj-lib`), `lib/testutils` (`jj-testutils`), `core`, `dsl`.
- **CommandError struct facade [V]**: `cli/src/command_error.rs:88-102` defines `CommandError` as a struct, NOT an enum:

  ```rust
  pub struct CommandError {
      pub kind: CommandErrorKind, // User, Config, Cli, BrokenPipe, Internal
      pub error: Arc<dyn error::Error + Send + Sync>,
      pub hints: Vec<ErrorHint>,
  }
  ```

- **Error mapping [V]**: `command_error.rs:237-280` implements `From<E> for CommandError` for library errors (`PathError`, `ConfigGetError`, `io::Error`). `io::ErrorKind::BrokenPipe` maps directly to `CommandErrorKind::BrokenPipe`. Hints (e.g. config path reminders, typo suggestions) are attached during conversion (`:261-264, 272-275`).
- **Test seams [V]**: Dedicated `lib/testutils` workspace crate (`jj-testutils`).

---

## 2. Invariants and Invariant Ownership

| Invariant | rustc | Ruff | uv | Cargo | gitoxide | rumdl | jj | Traces (Current) |
|---|---|---|---|---|---|---|---|---|
| **Diagnostic Codes** | Library (`rustc_error_codes`) [V] | Library (`ruff_linter::registry`) [V] | Library per-crate [V] | Library (`annotate-snippets`) [V] | Library (`gix-error::Class`) [V] | Library (`rumdl_lib::rules`) [V] | Library + CLI hints [V] | **CLI matches 4 levels deep** (`src/cli/error.rs`) [V] |
| **Diagnostic Spans** | Compact byte span (`rustc_span::Span`) [V] | Byte range (`TextRange`) [V] | Semantic AST / structs [V] | Byte offsets / lines [V] | Byte offsets [V] | 1-indexed char spans [V] | AST spans in parser errors [V] | **Dropped everywhere except Query** [V] |
| **Operational Errors** | Driver / DiagCtxt [V] | `anyhow::Result` + chain [V] | `anyhow::Result` + `Hinted` [V] | `anyhow::Result` + chain [V] | `gix_error::Result` / `Class` [V] | 6 simple `thiserror` enums [V] | `CommandError` struct (`Arc<dyn Error>`) [V] | **28 enums funnelling into 1.8k-line `CliError`** [V] |
| **BrokenPipe Contract** | Driver handler [V] | Exit 0 via `err.chain()` downcast [V] | Console writer ignores EPIPE [V] | Ignored / Shell error [V] | Handled in CLI runner [V] | CLI printer catches [V] | Exit 0 via `CommandErrorKind::BrokenPipe` [V] | **Unchecked `print!`, panics on pipe** [V] |
| **Test Fixture Seams** | In-tree test harnesses [V] | Dedicated `_test` & `_benchmark` crates [V] | Dedicated `uv-test` & `uv-bench` crates [V] | Dedicated `cargo-test-support` crate [V] | Dedicated `gix-testtools` crate [V] | Direct `pub mod` in `rumdl_lib` [V] | Dedicated `jj-testutils` crate [V] | **`traces::testing` backdoor + 1,528 tokens** [V] |

---

## 3. Tradeoffs and Costs Paid

### Fine-Grained Multi-Crate Workspaces (Ruff, uv, gitoxide)

- **Gains [H]**: Independent crate compile units; strict boundary enforcement; zero visibility leakage between CLI and engine; secondary consumers (e.g. `ruff_server` LSP) compile only what they need.
- **Costs [V]**: Heavy workspace management overhead (versioning, feature-flag forwarding, dependency synchronization across 40–92 `Cargo.toml` files). Changes spanning multiple tiers touch multiple crates.

### Single-Crate Monoliths with Open Libraries (rumdl)

- **Gains [H]**: Zero package coordination overhead; instant global refactoring; fast whole-program optimization (fat LTO); trivial test and benchmark access to internals without feature gates.
- **Costs [V]**: Lack of compiler-enforced architectural boundaries; long clean compilation times if modularity slips; inability to link a lightweight subset from a separate binary without pulling the whole crate.

### Dynamic Error Chaining vs Static Monolithic Enums (Cargo, uv, jj vs Traces)

- **Gains [H]**: Using a struct facade (`CommandError` with `dyn Error` or `anyhow::Error` with cause-chain downcasting) eliminates the need for a central enum to import or name every subsystem error type. Eliminates `#[expect(private_interfaces)]` completely. Adding a domain error variant requires zero changes in the CLI.
- **Costs [V]**: Exhaustive compile-time pattern matching across all possible errors is sacrificed in favor of dynamic runtime downcasting.

### Dedicated Test-Support Crates vs `test-utils` Feature Backdoors

- **Gains [H]**: Creating a workspace test-support crate (`gix-testtools`, `cargo-test-support`, `jj-testutils`, `ruff_benchmark`) isolates test fixtures cleanly. No `cfg(feature = "test-utils")` pollution in production code. No frozen `pub-tokens.txt` baseline thrash.
- **Costs [V]**: Requires at least a minimal workspace structure (2–3 crates) rather than a single standalone crate.

---

## 4. Transferable Lessons for Traces

1. **Decouple Diagnostic Presentation from Operational Failure Propagation [H]**:
   - Every comparable cleanly separates *rich diagnostics* (rules, syntax, queries) from *operational errors* (config I/O, file access, directory resolution).
   - In Traces, `QueryError` is a true diagnostic (carrying spans and labels), whereas `ConfigError`, `IndexError`, and `TemplatePathError` are operational failures. Funnelling operational failures into `miette::Diagnostic` with 35 hand-matched dotted codes creates 1,776 lines of boilerplate (`src/cli/error.rs:52-232, 265-337`).
2. **Adopt a Struct-Based CLI Error Facade with Cause Downcasting [H]**:
   - Following `jj`'s `CommandError` (`cli/src/command_error.rs:98`) or `uv`'s `hints_for_error` (`crates/uv/src/commands/diagnostics.rs:27`), the CLI error facade should be a struct containing an exit-code/error-kind classification, `Box<dyn std::error::Error + Send + Sync>`, and a list of remediation hints.
   - This immediately eliminates `#[expect(private_interfaces)]` (`src/cli/error.rs:39-50`), removes the need for `src/cli/error.rs` to match 4 levels deep into subsystem internals, and cuts stack frame explosion (`clippy::large_stack_frames` on `run_with_config`).
3. **Isolate Test Utilities into a Dedicated Support Unit [H]**:
   - Whether Traces becomes a multi-crate workspace or splits out test fixtures, `src/lib.rs:177-210` (`pub mod testing` exposing 45 private types) and the 1,528 frozen tokens in `pub-tokens.txt` should be replaced by a dedicated `traces-test-support` or `traces-bench` unit (following Cargo, Ruff, and jj).
4. **In-Band BrokenPipe Handling in the CLI Driver [H]**:
   - Both Ruff (`crates/ruff/src/main.rs:56-61`) and jj (`cli/src/command_error.rs:239-242`) explicitly downcast for `std::io::ErrorKind::BrokenPipe` in the CLI runner, mapping it to exit code 0 and suppressing diagnostics. This directly resolves Traces' unhandled EPIPE defect (`issues/05-epipe-exit101-decision.md`).

---

## 5. Lessons That Do NOT Transfer to Traces

1. **Extreme 60–90 Crate Micro-Decomposition (uv / gitoxide) [H]**:
   - uv and gitoxide split every minor capability into its own crate because they serve as foundational ecosystems with thousands of external downstream consumers and strict compile-time optimization targets.
   - Traces is a single-binary application with an internal 10-tier DAG. Splitting into 20+ micro-crates would introduce massive `Cargo.toml` synchronization and feature-flag plumbing overhead with zero consumer benefit.
2. **Global Salsa Interning Database for Diagnostics (rustc / Ruff) [H]**:
   - Both rustc (`DiagCtxt`) and Ruff (`ruff_db` Salsa database) route all diagnostics through an interning compiler query engine.
   - Traces is an ephemeral CLI batch processor; note parsing is already intentionally infallible (`src/note/parser.rs:106`), and queries are evaluated on-demand. An interning diagnostic database is disproportionate accidental complexity.
3. **Total Untyped Error Erasure (pure `anyhow` across library boundaries) [H]**:
   - While Cargo uses `anyhow::Result` everywhere, Traces' domain subsystems (`query`, `schema`, `temporal`) have genuine programmatic error handling needs (e.g. template execution recovering from missing schema fields or fallible date arithmetic). Subsystem-level `thiserror` enums should remain strongly typed within domain boundaries.

---

## 6. Impact on Strategic Redesign Hypotheses

### [H1] Multi-Crate Workspace Decomposition (survey/cross-cutting.md:48)

- **Challenge / Nuance [H]**: The scout hypothesis advocated splitting Traces into 5–6 workspace crates (`traces-core`, `traces-note`, `traces-index`, `traces-query`, `traces-schema`, `traces-template`, `traces-cli`).
- **Precedent Evidence [V]**:
  - `rumdl` demonstrates that a 341k-LoC tool can thrive as a single crate *if* its internal library surface is cleanly public and not burdened with artificial visibility locks.
  - However, if Traces intends to support an LSP server, headless host, or separate benchmark crate without visibility hacks, a moderate 2–3 crate split (e.g., `traces-core`/`traces-engine`, `traces-test-support`, `traces-cli`) provides the isolation benefits of `jj` or `Cargo` without the 90-crate maintenance tax of `uv`.
  - Splitting purely along the 10 topological tiers is unnecessary and adds high coordination overhead.

### [H3] / [H5] Decompose Monolithic `CliError` (survey/cli.md:49, cross-cutting.md:72)

- **Direct Confirmation [H]**: The 1,776-line `CliError` enum in `src/cli/error.rs` is an architectural anomaly compared to all 7 comparables.
- **Superior Pattern [H]**: Instead of decomposing `CliError` into 10 per-command enums (as proposed in H5), the precedent set by `jj` (`CommandError` struct with `Arc<dyn Error>` + `Vec<ErrorHint>`) and `uv` (`hints_for_error` downcasting) is vastly superior:
  - Preserves centralized exit-code and abort handling in `main.rs`.
  - Eliminates the 28-variant enum and `#[expect(private_interfaces)]`.
  - Enables commands to return standard `Result<(), CommandError>` with localized hint attachment.

### [H4] In-Band `BrokenPipe` Detection (survey/cli.md:55)

- **Direct Confirmation [V]**: Verified directly from `crates/ruff/src/main.rs:56-61` and `jj/cli/src/command_error.rs:239-242`. Both production CLIs catch `io::ErrorKind::BrokenPipe` at the driver boundary and exit 0 silently.

### Pseudo-Public `traces::testing` Backdoor & `pub-tokens.txt` (survey/cross-cutting.md:35, 43)

- **Direct Challenge [H]**: Across all 7 comparables, *not one codebase* uses a `pub mod testing` backdoor guarded by 1,528 frozen visibility tokens in production code to service external benchmarks.
- Mature codebases either:
  1. Make internal library modules `pub mod` directly because the crate is an application, not a semver-stable public crate (rumdl: `src/lib.rs:50-63`).
  2. Move test utilities into a dedicated workspace test crate (Cargo: `cargo-test-support`; jj: `jj-testutils`; gitoxide: `gix-testtools`; Ruff: `ruff_benchmark`).
- Traces' combination of a single crate, private interface suppression, and baseline token freezing is the worst of both worlds: maximum visibility friction with zero modularity benefits.
