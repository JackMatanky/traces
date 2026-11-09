---
name: rust-integration-testing
description: Rust integration tests in tests/ for public APIs, cross-module flows, and CLI binaries. Use when testing crate or CLI boundaries.
metadata:
  version: "1.1.0"
  companion_to: "rust-testing-router"
---

# Rust Integration Testing

Integration tests in `tests/` exercise the crate from the outside using only its public API or compiled binary targets.

## Precedence Rule

Project conventions, `AGENTS.md`, and local specifications take precedence over default layout and harness rules in this skill.

## 5-Step Workflow

1. Check the layer boundary in [`../rust-testing-router/references/boundaries.md`](../rust-testing-router/references/boundaries.md) so you do not duplicate unit branches or Gherkin scenarios.
   Completion: target behavior is classified as a public library workflow or a technical CLI/process boundary.
2. Choose the target binary layout in [`references/boundary-suites.md`](references/boundary-suites.md) (`tests/<target>.rs` or `tests/<target>/main.rs`).
   Completion: every new test module is wired into a Cargo-compiled integration test binary.
3. Select a hermetic dependency strategy from the decision matrix in [`references/boundary-suites.md`](references/boundary-suites.md).
   Completion: every external collaborator has an explicit hermetic strategy (`temp-local`, `hand-written fake`, `local server`, `container`, or `real`).
4. Write the hermetic integration tests using [`references/boundary-suites.md`](references/boundary-suites.md) and [`../rust-testing-router/references/raii-cleanup.md`](../rust-testing-router/references/raii-cleanup.md).
   Completion: suite compiles with zero warnings, zero `unsafe` blocks, and zero shared host filesystem or environment state.
5. Check the test discovery and run red-first execution with [`../rust-testing-router/references/commands.md`](../rust-testing-router/references/commands.md).
   Completion: new tests appear in `cargo nextest list -E 'kind(test)'`, fail when the contract breaks, and pass with `--no-tests=fail`.
