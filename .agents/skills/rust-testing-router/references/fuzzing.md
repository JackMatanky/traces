# Coverage-Guided Fuzzing (`cargo-fuzz`)

> Use `cargo-fuzz` (`libFuzzer` + `arbitrary`) to prove parsers, decoders, and state machines never panic or hang on untrusted input.

## 1. Setup

Initialize fuzz targets in the `fuzz/` subdirectory using `cargo-fuzz` and `arbitrary`:

```toml
# fuzz/Cargo.toml
[dependencies]
arbitrary = { version = "1", features = ["derive"] }
libfuzzer-sys = "0.4"
my_crate = { path = ".." }
```

For structured domain fuzzing, derive `arbitrary::Arbitrary` on an input struct instead of feeding raw `&[u8]` when the target sits behind a lexer:

```rust
#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzQuery {
    raw_filter: String,
    limit: u16,
}

fuzz_target!(|input: FuzzQuery| {
    let _ = my_crate::parse_and_plan(&input.raw_filter, usize::from(input.limit));
});
```

## 2. The Four Fuzz Oracles

Every fuzz target must enforce at least one of these four explicit oracles:

1. **Crash and Stack-Overflow Freedom (Deep Nesting):**
   Feed arbitrary byte slices and deeply nested tokens (5,000+ opening brackets, quotes, or AST nodes) into parsers. The parser must return `Ok` or a structured `Err` without panicking, slicing mid-codepoint, or overflowing the call stack.
2. **Round-Trip Identity:**
   For any structured value `v` (or byte slice that parses to `Ok(ast)`), check `parse(&format(ast)) == Ok(ast)`.
3. **Differential Oracle:**
   Compare a fast or optimized parser/query engine against a simple reference implementation on identical generated inputs: `assert_eq!(fast_eval(&input), reference_eval(&input))`.
4. **Structural and Allocation Bounds:**
   Check that output size, recursion depth, and iteration counts stay within documented limits for any input of length $N$.

## 3. Running, Minimizing, and Replaying Crashes

Run the fuzzer for a bounded time window using nightly Rust:

```bash
cargo +nightly fuzz run parse_target -- -max_total_time=60 -max_len=16384
```

When `cargo-fuzz` discovers a crash or timeout in `fuzz/artifacts/<target>/crash-<hash>`:

1. **Minimize the artifact** to the smallest reproducing input:

   ```bash
   cargo +nightly fuzz tmin parse_target fuzz/artifacts/parse_target/crash-<hash>
   ```

2. **Print the minimized debug representation:**

   ```bash
   cargo +nightly fuzz fmt parse_target fuzz/artifacts/parse_target/crash-<hash>
   ```

3. **Promote the minimized crash to a unit `#[test]`** in `src/` so `cargo nextest run` guards the regression on every commit.

## Completion

Completion: the fuzz target runs for the bounded window with zero crashes, and every discovered crash is minimized and promoted to a deterministic unit test.
