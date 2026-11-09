---
name: rust-benchmarking
description: Criterion benchmarks in benches/, cargo bench baselines, and regression checks. Use when writing or running Rust microbenchmarks.
metadata:
  version: "1.1.0"
  companion_to: "rust-testing-router"
---

# Rust Benchmarking

Criterion benchmarks in `benches/` measure throughput, latency, and scaling against a baseline. They do not replace correctness tests.

## Precedence Rule

Project conventions, `AGENTS.md`, and local task wrappers (`mise run bench`) take precedence over default commands in this skill.

## 5-Step Workflow

1. Check the `Cargo.toml` benchmark target configuration using [`references/criterion.md`](references/criterion.md).
   Completion: `cargo bench --benches -- --help` runs without an `Unrecognized option` error from libtest.
2. Design the measurement loop using [`references/criterion.md`](references/criterion.md) (`iter`, `iter_batched`, `Throughput`, and `std::hint::black_box`).
   Completion: fixture setup and input cloning are isolated from the timed closure, and inputs/outputs use `black_box`.
3. Document the benchmark contract on every benchmark group and function.
   Completion: every benchmark function carries a doc comment stating measured operation, parameters, fixture boundary, expected scaling trend, and regression signature.
4. Check all benchmark targets with a smoke test (`cargo test --profile bench --benches` or `cargo test --benches`).
   Completion: every benchmark target compiles and passes a single-iteration smoke execution.
5. Run the Criterion benchmark serially in release mode and report results in a Markdown table.
   Completion: point estimates and confidence intervals (or baseline deltas) are reported in a Markdown table in chat, with zero unrequested report files written to disk.
