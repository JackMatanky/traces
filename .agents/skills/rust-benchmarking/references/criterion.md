# Criterion Benchmarking Guide

> Use `criterion` to measure warm, statistically rigorous latency and throughput distributions in release builds.

## 1. `Cargo.toml` Configuration (`harness = false`)

Disable the default `libtest` harness on every Criterion target. Set `bench = false` on `[lib]` and `[[bin]]` targets so `cargo bench` accepts Criterion flags:

```toml
[lib]
bench = false

[[bin]]
name = "my_app"
bench = false

[dev-dependencies]
criterion = "0.8"

[[bench]]
name = "parser_bench"
harness = false
```

## 2. Protecting Inputs and Outputs with `std::hint::black_box`

Import `black_box` from `std::hint::black_box` (the `criterion::black_box` path is a legacy alias). Wrap constant inputs so the compiler cannot fold them at compile time, and return or wrap outputs so dead-code elimination cannot delete the work:

```rust
use std::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_parse_header(c: &mut Criterion) {
    let raw = "id: 42\ntitle: Benchmark\n";
    c.bench_function("parse_header", |b| {
        b.iter(|| parse_header(black_box(raw)))
    });
}

criterion_group!(benches, bench_parse_header);
criterion_main!(benches);
```

When using `group.bench_with_input(id, &input, |b, input| ...)`, Criterion passes the `input` parameter through `black_box` automatically.

## 3. Selecting the Timing Loop and `BatchSize`

Choose the loop method that matches ownership and teardown cost:

- `b.iter(|| ...)` - default loop when the routine borrows its input (`&T`) and returns a small value.
- `b.iter_batched(setup, routine, batch_size)` - runs `setup` untimed before each batch, passes owned input to `routine`, and drops outputs outside the timer.
- `b.iter_batched_ref(setup, routine, batch_size)` - passes `&mut T` to `routine` when `routine` mutates the input in place and drops it untimed.
- `b.iter_with_large_drop(|| ...)` - defers dropping large return values until after timing finishes.

```rust
use std::hint::black_box;
use criterion::{BatchSize, Criterion};

fn bench_in_place_sort(c: &mut Criterion) {
    let template: Vec<u64> = (0..10_000).rev().collect();
    c.bench_function("sort_10k_u64", |b| {
        b.iter_batched(
            || template.clone(),
            |mut data| {
                data.sort_unstable();
                black_box(data)
            },
            BatchSize::SmallInput,
        );
    });
}
```

Choose `BatchSize` by memory footprint and clone/drop semantics:

| Variant | Semantics | Use When |
|---|---|---|
| `BatchSize::SmallInput` | Pre-allocates many inputs per batch | Input is small enough that thousands of copies fit in cache/RAM |
| `BatchSize::LargeInput` | Allocates fewer inputs per batch | Input holds kilobytes or megabytes (for example, a large AST or buffer) |
| `BatchSize::PerIteration` | Runs setup and drop before/after every single iteration | Input holds scarce OS resources (temp dirs, file descriptors, sockets) |

## 4. Parameterized Groups and `Throughput`

Measure scaling across input sizes with `BenchmarkId` and `Throughput`:

```rust
use std::hint::black_box;
use criterion::{BenchmarkId, Criterion, Throughput};

fn bench_scanner_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner_scaling");

    for count in [100usize, 1_000, 10_000] {
        let payload = "line_item=42\n".repeat(count);
        group.throughput(Throughput::Bytes(payload.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &payload,
            |b, input| b.iter(|| scan_lines(black_box(input))),
        );
    }

    group.finish();
}
```

## 5. Benchmark Contract Doc Comments

Document every benchmark function so readers know what regression signature to look for:

```rust
/// Measures line-scanner throughput across payload line counts.
///
/// Parameters: varies line count (100, 1k, 10k); reports bytes/sec.
/// Fixture: pre-allocated UTF-8 buffer; string allocation excluded from timing.
///
/// Expected outcomes:
/// - Linear O(n) scaling with stable bytes/sec throughput across sizes.
///
/// Unexpected outcomes:
/// - Quadratic time growth, indicating repeated buffer re-scans.
```

## 6. Smoke Testing, Baselines, and Comparison Tools

1. **Smoke test in `bench` profile:** Run a single unmeasured pass of all benchmarks to check for panics. Pass `--profile bench` when benchmarks construct deep ASTs or large inputs that overflow debug stacks:

   ```bash
   cargo test --profile bench --benches
   ```

2. **Serial execution:** Never run benchmarks in parallel with other builds or tests. Avoid setting `RUSTFLAGS="-C target-cpu=native"` unless requested.
3. **Baselines and `critcmp`:** Save a baseline before optimizing, compare after with `critcmp`, or use `iai-callgrind` in noisy CI:

   ```bash
   cargo bench --bench parser_bench -- --save-baseline before
   cargo bench --bench parser_bench -- --baseline before
   ```

## Completion

Completion: benchmarks use `std::hint::black_box`, isolate setup with `BatchSize`, pass `cargo test --profile bench --benches`, and report a Markdown table.
