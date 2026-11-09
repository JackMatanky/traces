# Writing Rustdoc Doctests

> Doctests in `/// # Examples` serve two purposes at once: reader documentation and compiled regression tests.

## 1. Where Doctests Run

Cargo runs doctests only for `[lib]` targets (`src/lib.rs` and its modules). Doctests written in binary-only targets (`src/main.rs`) are ignored by `cargo test --doc`. Keep core types and functions in `src/lib.rs` so their examples are tested.

In Rust 2024 edition (`edition = "2024"`), `rustdoc` merges standalone doctests across the crate into fewer binaries, dramatically speeding up `cargo test --doc`.

## 2. Returning `Result` Instead of `.unwrap()`

Never write `.unwrap()` in visible doctest lines because readers copy documentation examples directly into production code. Return `Result<T, E>` from a hidden `main` wrapper so the visible example uses the `?` operator naturally:

```rust
use std::num::ParseIntError;

/// Parses a port number from a decimal string.
///
/// # Examples
///
/// ```
/// use my_crate::parse_port;
///
/// let port = parse_port("8080")?;
/// assert_eq!(port, 8080);
/// # Ok::<(), std::num::ParseIntError>(())
/// ```
///
/// # Errors
///
/// Returns [`ParseIntError`] if `input` is not a valid `u16` integer.
pub fn parse_port(input: &str) -> Result<u16, ParseIntError> {
    input.parse::<u16>()
}
```

## 3. Hiding Boilerplate with `#`

Prefix setup imports or the trailing `# Ok::<(), MyError>(())` line with `#` (hash + space) so `rustdoc` hides them in HTML. Do not hide lines that the reader needs to understand how to call the API.

## 4. Doctest Block Attributes

| Attribute | Meaning | Use When |
|---|---|---|
| ```` ``` ```` (default) | Compiles and runs the example; must exit 0 | Almost all public functions and types |
| ```` ```should_panic ```` | Compiles and runs; passes only if it panics | Documenting an explicit panic precondition |
| ```` ```compile_fail ```` | Passes only if compilation fails | Proving a type-state or borrow-checker guard rejects invalid code |
| ```` ```no_run ```` | Compiles the example but does not execute it | Examples that would bind network ports or require external hardware |

Avoid `ignore` on Rust code blocks because `ignore` skips compilation completely and lets examples rot silently.

## 5. Running Doctests

`cargo nextest run` does not execute rustdoc doctests. Always run `cargo test --doc` (or the project's `mise run verify` task) alongside `nextest`:

```bash
cargo test --doc
```

## Completion

Completion: public functions and types include `/// # Examples` blocks that use `?` instead of `.unwrap()`, avoid `ignore`, and pass `cargo test --doc`.
