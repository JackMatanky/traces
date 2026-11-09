---
name: rust-testing-router
description: Cross-layer Rust test strategy, boundary arbitration, mutation, coverage, and fuzzing. Use when choosing test layers or auditing.
metadata:
  version: "1.1.0"
---

# Rust Testing Router

Use this hub to arbitrate test layers, audit suite quality, and load shared references.

## 1. Precedence Rule

Project conventions, `AGENTS.md`, and local specifications take precedence over default rules in these skills. Check local configuration before applying default naming, layout, or Gherkin rules.

## 2. Silent-Green Rule

Do not trust exit code `0` alone. A test run that filters out all targets or skips assertions passes with zero value.

- Check the target discovery with `cargo nextest list` before and after adding tests.
- Run with `--no-tests=fail` so empty filters fail the build.
- Prove every new assertion goes red when you break the behavior under test.

## 3. Arbitration Procedure

1. Assign each guarantee to the highest-leverage layer in [`references/boundaries.md`](references/boundaries.md).
   Completion: every guarantee maps to one layer or an explicit out-of-scope note.
2. Execute the primary layer skill: [`../rust-bdd-testing/SKILL.md`](../rust-bdd-testing/SKILL.md), [`../rust-integration-testing/SKILL.md`](../rust-integration-testing/SKILL.md), [`../rust-unit-testing/SKILL.md`](../rust-unit-testing/SKILL.md), or [`../rust-benchmarking/SKILL.md`](../rust-benchmarking/SKILL.md).
   Completion: all completion criteria in the chosen layer skill pass.
3. Load technique references on demand from the index below and run verification commands.
   Completion: new tests appear in `cargo nextest list` and pass execution.

## 4. Shared Reference Index

### Architecture & Execution

- [`references/boundaries.md`](references/boundaries.md) - 7-level hierarchy and pairwise boundary arbitration rules.
- [`references/commands.md`](references/commands.md) - task runners, `nextest` filters, doctests, and silent-green guards.

### Core Authoring

- [`references/assertions.md`](references/assertions.md) - `pretty_assertions`, `matches!`, structured errors, and panic contracts.
- [`references/rstest-fixtures.md`](references/rstest-fixtures.md) - `#[fixture]` composition, `#[with]`, `#[from]`, lifetimes, and `#[values]`.
- [`references/table-driven.md`](references/table-driven.md) - parameterized `#[rstest]` cases and promotion boundaries.
- [`references/raii-cleanup.md`](references/raii-cleanup.md) - panic-safe `Drop` guards, `TempDir`, and hermetic process environments.
- [`references/mocks.md`](references/mocks.md) - trait seams, native `async fn` fakes, and `mockall`.

### Specialized Oracles

- [`references/property-based.md`](references/property-based.md) - `proptest` strategies, shrinking, and regression persistence.
- [`references/fuzzing.md`](references/fuzzing.md) - `cargo-fuzz`, `Arbitrary`, stack-depth oracles, and crash minimization.
- [`references/snapshots.md`](references/snapshots.md) - `insta` snapshots, redactions, and CI check flags.

### Runtime & Concurrency

- [`references/async.md`](references/async.md) - `#[tokio::test]`, paused virtual time, timeouts, and channels.
- [`references/concurrency.md`](references/concurrency.md) - `loom` atomic permutation models and `check-cfg` setup.

### Suite Quality Auditing

- [`references/mutation.md`](references/mutation.md) - `mutarust` and `cargo-mutants` guardrails, diff scoping, and triage.
- [`references/coverage.md`](references/coverage.md) - `cargo llvm-cov` branch discovery and empirical floor auditing.
