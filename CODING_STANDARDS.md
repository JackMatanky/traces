# Rust Coding Standards

## Core Principles
- **SHOULD** prioritize clarity and maintainability over cleverness.
- **MUST** write clear, maintainable, idiomatic Rust (DRY).
- All code **MUST** be fully optimized.

## Code Style
- **MUST** follow the Single Responsibility Principle (SRP): keep each type and function focused on one responsibility.
- **MUST** use meaningful, descriptive variable and function names.
- **MUST** obey the Rust API Guidelines.
- Naming conventions:
  - `snake_case` for functions, variables, and modules.
  - `PascalCase` for types and traits.
  - `SCREAMING_SNAKE_CASE` for constants.
- **MUST NOT** use emoji or Unicode that emulates emoji (for example, `✓`, `✗`), except in tests that specifically test multibyte character handling.

### Rust Idioms
- **MUST** call `.clone()` explicitly on non-`Copy` types. Avoid hidden clones in closures and iterators.
- **MUST** use exhaustive pattern matching. Avoid catch-all `_` patterns when possible. Prefer `if let` and `while let` for single-pattern matching.
- **MUST** use `format!` for string formatting, and `enumerate()` instead of manual counter variables.

### Type System
- **MUST** use Rust's type system to prevent bugs at compile time.
- Use newtypes to distinguish values with different meanings that share an underlying type.
  - Parse external input into validated newtypes at system boundaries.
  - **MUST** enforce the same invariants in every construction path, including deserialization.
- Prefer `Option<T>` over sentinel values.

### Numeric Operations
- Use `From` for lossless conversions and `TryFrom` for fallible conversions when the types provide those traits. Clippy warns on `as` conversions.
- Choose integer overflow behavior explicitly:
  - Use checked arithmetic when overflow is an error.
  - Use saturating arithmetic when the result must stop at a bound.
  - Use wrapping arithmetic only when modular arithmetic is intentional.

### Type Design
- **MUST** derive common traits (`Debug`, `Clone`, `PartialEq`, `Default`) when the type supports them.
- **SHOULD** prefer composition over inheritance-like patterns.
- **SHOULD** use the builder pattern for complex construction.
- **MUST** keep fields private by default. Provide accessors when callers need them.
- Types, functions, and modules **MUST** use the narrowest visibility that callers need.

### Function Design
- **MUST** prefer borrowing (`&T`, `&mut T`) over ownership when possible.
- **SHOULD** group more than 5 related parameters into a config struct.
- **SHOULD** return early to reduce nesting.
- **SHOULD** use iterators and combinators over explicit loops when clearer.

### Error Handling
- **MUST** use `Result<T, E>` for fallible operations and propagate recoverable
  errors with `?`.
- **MUST NOT** use `.unwrap()` in production code paths. Use `.expect()` only for
  invariants, with a message explaining why the invariant holds. Tests may use
  `.unwrap()` when it makes failures clear.
- **MUST** use `thiserror` for typed library/domain errors and `anyhow` (with `.context()`) for application-level errors at binary or CLI boundaries.
- **MUST** preserve the source error when wrapping an error or adding context.
- Start error messages with lowercase text and omit trailing punctuation. Preserve the case of identifiers and names.

## Optimization
- **MUST** maximize algorithmic time and space efficiency (Big-O).
- **SHOULD** use parallelization and SIMD for CPU-bound hot paths when benchmarks show a gain.
- **MUST** write only the code needed to solve the task, with no technical debt.
- **MUST** use a small, low-overhead crate when it replaces significant custom code with the best performance.
- **SHOULD** do a second pass over the code before handoff when clear performance or simplicity gains remain.

### Memory Use
- **MUST** avoid unnecessary allocations: prefer `&str` over `String`, use `Cow<'_, str>` when ownership is conditionally needed, and use `Vec::with_capacity()` when size is known.
- **SHOULD** keep iterators lazy and avoid intermediate `.collect()` calls.
- **SHOULD** reuse collection capacity with `clear()` and `extend()`, or `drain()` when removed items are needed.
- **SHOULD** use the `entry` API for map insert-or-update when the key is already owned.
- **SHOULD** wrap unbuffered streams in `BufReader` or `BufWriter` for repeated small I/O operations.
- Use `Arc` and `Rc` judiciously. Prefer borrowing.

### Concurrency
- **MUST** use `Send` and `Sync` bounds appropriately.
- **SHOULD** prefer `tokio` for async I/O and `rayon` for CPU-bound parallelism when parallelism is warranted.
- Choose synchronization (`RwLock`, lock-free alternatives, `mpsc`/`crossbeam` channels) to fit the workload.
- **MUST NOT** hold a `std::sync::Mutex` or `std::sync::RwLock` guard across `.await`.
- **MUST** use bounded channels when producers can outpace consumers.
- **MUST** check cancellation safety for futures in `tokio::select!` branches.
- Blocking work **MUST NOT** run on async worker threads.

## Testing
- Test new behavior at the right boundary:
  - **Unit tests** (`#[cfg(test)]`, `#[test]`) for local logic.
  - **Integration tests** (`tests/integration`) for public APIs and cross-module flows.
  - **E2E tests** for shipped CLI binary behavior.
  - **Cucumber-style BDD tests** (Gherkin `.feature` files with `rstest-bdd`) for user-facing scenarios.
  - **Doctests** for public API examples.
  - **Property tests** for invariants across broad input ranges. Keep example tests for specific cases.
  - **Mutation tests** (`mise run test:mutants`) when verifying test suite strength.
- Use real temporary files/directories when filesystem behavior matters, and test doubles when a test does not need an external dependency (APIs, databases).
- Use Arrange–Act–Assert for unit and integration tests. Make the expected behavior clear in the assertion.
- Use Given–When–Then for Gherkin scenarios. Given sets the context. When names the action. Then states the observable result.
- **MUST NOT** commit commented-out tests.

### Benchmarking
- **MUST** use the Mise `bench` task (see `AGENTS.md`) instead of invoking Criterion directly.
- **MUST** run benchmark measurements in `release` mode.
- **MUST NOT** run benchmark measurements in `debug` mode.
- **MUST** distinguish smoke checks from measurement runs.
- Use `std::hint::black_box` where needed to prevent the compiler from eliminating benchmark work.
- **MUST NOT** run benchmarks in parallel.
- **MUST NOT** pass `target-cpu=native` or custom `RUSTFLAGS` to measurement runs.
- **MUST** keep comparisons fair, apples-to-apples, and independent (disable caching features if they skew results).
- **MUST NOT** game benchmarks or alter them just to meet a target.
- **MUST** report measured results for all tested benchmarks in a Markdown table at handoff.
- **MUST NOT** save separate benchmark writeups unless the user asks.

## Imports & Dependencies
- **MUST** avoid wildcard imports (`use module::*`) except for preludes, test modules (`use super::*`), and prelude re-exports.
- **SHOULD** group imports in order: standard library, external crates, then local.
- **MUST** declare dependencies with version constraints in `Cargo.toml`.
- Cargo features **MUST** add capabilities without removing existing behavior.

### Preferred Tools
Check `Cargo.toml` before assuming a crate is a project default.

- **CLI progress:** Use `indicatif` for long-running progress bars with messages that describe the current operation.
- **TUI input:** Use `ratatui` and `crossterm`. Provide intuitive mouse controls. Account for scrolling offsets when mapping clicks to items.
- **JSON:** Use `serde` with `serde_json`.
- **Logging:** Use `tracing::error!` for error diagnostics. Do not use `println!` for error diagnostics. Use structured `tracing` fields for diagnostic values. Exclude sensitive data.

## API Documentation
- **MUST** document all public functions, structs, enums, traits, and methods.
- **SHOULD** explain parameters and return values when the signature does not make them clear.
- Include `# Errors`, `# Panics`, and `# Safety` sections when they apply, and `# Examples` for non-trivial functions.
- **MUST** keep doc comments up to date with code changes.

This example illustrates doc comment structure. It does not use the Traces API.

````rust
/// Calculates the total cost of `items` including `tax_rate` (for example, `0.08` for 8%).
///
/// # Errors
///
/// Returns `CalculationError::EmptyItems` if `items` is empty, or
/// `CalculationError::InvalidTaxRate` if `tax_rate` is negative.
///
/// # Examples
///
/// ```
/// # use traces::{calculate_total, Item};
/// let items = vec![Item { price: 10.0 }, Item { price: 20.0 }];
/// let total = calculate_total(&items, 0.08)?;
/// assert!((total - 32.40).abs() < f64::EPSILON);
/// # Ok::<(), traces::CalculationError>(())
/// ```
pub fn calculate_total(items: &[Item], tax_rate: f64) -> Result<f64, CalculationError> {
    // ...
    # Ok(0.0)
}
````

### Code Comments
- **SHOULD** avoid comments by designing and writing code that is clear on its own.
- When writing a comment, **SHOULD** write it as if all readers are Rust novices and explain Rust-specific nuances.
- **MUST** avoid redundant comments that merely restate what the code or function name already makes clear.
- **MUST NOT** use a comment to justify a missing check or claim a guarantee that the code does not enforce. Enforce guarantees with types, validation, or control flow. Comments **MAY** explain how the code enforces them.
- **MUST NOT** copy user prompts or agent instructions into code comments. Explain the relevant technical reason in clear technical prose.

## Security
- **MUST NOT** hardcode secrets, API keys, or passwords in code, and **NEVER** log sensitive information (tokens, passwords, PII).
- Keep local secret files (such as `.env`) out of Git, and allow CI/deployment secret stores where appropriate. Use `secrecy` for sensitive data types.
- **MUST** read sensitive runtime configuration from environment variables when the deployment uses them, via `std::env` or `dotenvy`.

## Version Control
- **MUST** write clear, descriptive commit messages.
- **MUST NOT** commit commented-out code, debug `println!` statements, `dbg!` macros, or credentials.

## Unsafe Code
**MUST NOT** use `unsafe` unless it is completely unavoidable. Use safe Rust whenever it can meet the requirement. When `unsafe` is necessary, obey all seven rules below:

1. **Scope (`unsafe-minimize-scope`):** Keep each `unsafe` block as small as possible. Put unsafe operations in explicit `unsafe {}` blocks, even inside an `unsafe fn`.
2. **Safety documentation (`unsafe-safety-comment`):** Write a `// SAFETY:` comment above every `unsafe` block explaining why the operation is sound, and a `# Safety` doc section on every `unsafe fn` stating caller obligations.
3. **Initialization (`unsafe-maybeuninit`):** Use `MaybeUninit<T>` for uninitialized memory. Never call `assume_init` before initialization is complete. Never use `mem::uninitialized()`, and never use `mem::zeroed()` for types where zero is not a valid bit pattern.
4. **Thread safety (`unsafe-send-sync-manual`):** Prefer compiler-derived `Send` and `Sync`. For any manual `unsafe impl Send` or `unsafe impl Sync`, document the invariant that makes it sound.
5. **FFI declarations (`unsafe-extern-block`):** In Rust 2024, wrap `extern` blocks in `unsafe extern { }` and mark items `safe` only when callers have no safety obligations.
6. **Linker attributes (`unsafe-no-mangle-unsafe`):** In Rust 2024, use `#[unsafe(no_mangle)]`, `#[unsafe(export_name = "...")]`, and `#[unsafe(link_section = "...")]`, and document how symbol/section constraints are met.
7. **Miri (`unsafe-miri-ci`):** Run `cargo miri test` in CI for every crate that contains `unsafe` code (using the project task policy).

## Verification
See `AGENTS.md` for the Mise task policy and the mandatory `verify` completion gate. Use Rustfmt for formatting and Clippy for lint checks through those tasks. Resolve compiler and Clippy warnings before handoff. Use `-D warnings` in CI and lint tasks rather than `#![deny(warnings)]` in source.
