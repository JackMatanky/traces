# Test Commands and Execution Guards

> Run the narrowest command that proves the target layer, and guard against zero-test runs.

## 1. Task-Runner First Rule

Check project task definitions (`mise.toml`, `Justfile`, `Makefile`) before running raw `cargo` commands.

- Run `mise run test` or `mise run verify` when the repository defines task wrappers.
- Task wrappers set required environment variables, build flags, and scenario-count checks automatically.
- Use the raw `cargo` commands only when scoping to a single test target that the task runner does not expose.

## 2. Target Scoping and the `--tests` Trap

`cargo nextest run --tests` (plural) does **not** mean "integration tests only." Cargo compiles unit targets (`[lib]`, `[[bin]]`) and integration targets (`tests/*.rs`) where `test = true` under `--tests`.

Use explicit target selectors to isolate layers:

| Scope | Command | What It Runs |
|---|---|---|
| Library unit suite only | `cargo nextest run --lib` | `src/lib.rs` `#[cfg(test)]` modules only |
| All integration binaries only | `cargo nextest run -E 'kind(test)'` | All targets in `tests/` (excludes `lib` and `bin`) |
| Single integration binary | `cargo nextest run --test <target_name>` | `tests/<target_name>.rs` or `tests/<target_name>/main.rs` |
| Single test by name substring | `cargo nextest run <pattern>` | Matching tests across default targets |
| Doctests | `cargo test --doc` | Executable `///` code blocks in `[lib]` |

`cargo nextest` does not execute rustdoc tests. Run `cargo test --doc` separately whenever you change doc comments or public signatures.

## 3. Silent-Green Guards

A test command that matches zero tests exits `0` by default in standard Cargo. Defeat silent-green runs with these three checks:

1. **Reject empty test runs:** Pass `--no-tests=fail` (or set it in `.config/nextest.toml`) so filter typos fail immediately:

   ```bash
   cargo nextest run --no-tests=fail -E 'test(my_new_test)'
   ```

2. **Check the test discovery before and after:** List discovered tests to confirm that your new test or scenario compiled.

   ```bash
   cargo nextest list --test features
   ```

3. **Pass required feature flags:** When a target sets `required-features = ["test-utils"]`, pass `--features test-utils` or `--all-features` so Cargo runs it:

   ```bash
   cargo nextest run --test integration --features test-utils --no-tests=fail
   ```

## 4. Specialized Verification Commands

| Workflow | Command | Notes |
|---|---|---|
| Clippy on all test targets | `cargo clippy --all-targets --all-features -- -D warnings` | Lints unit, integration, and bench code |
| Benchmark smoke test | `cargo test --benches` or `cargo test --profile bench --benches` | Runs one iteration per benchmark without timing |
| Criterion measurement | `cargo bench --bench <name> -- "<filter>"` | Run serially on an idle machine |
| Line and branch coverage | `cargo llvm-cov nextest --all-features` | See [`coverage.md`](coverage.md) |
| Mutation audit (diff-scoped) | `mise run test:mutants` or `cargo mutarust --git-diff --workers 4` | See [`mutation.md`](mutation.md) |
| Fuzzing (bounded run) | `cargo +nightly fuzz run <target> -- -max_total_time=60` | See [`fuzzing.md`](fuzzing.md) |

## Completion

Completion: the chosen command runs with `--no-tests=fail`, discovers the expected targets, and exits `0` (or the reason for not running is stated).
