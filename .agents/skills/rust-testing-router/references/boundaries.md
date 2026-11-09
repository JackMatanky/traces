# Test Boundaries and Layer Arbitration

> Assign every guarantee to the highest-leverage layer that can prove it deterministically.

## 1. The 7-Level Test Architecture Hierarchy

Evaluate guarantees from top to bottom. Put each check in the earliest layer that catches the failure mode.

1. **Static & Compile-Time Gates (`rustc`, Clippy, type state, baselines)**
   Prove invariants through types, exhaustive pattern matching, lint rules, and public-API token lists. If the compiler rejects invalid states, do not write runtime tests for them.
2. **Property & Fuzz Laws (`proptest`, `cargo-fuzz`)**
   Prove algebraic laws (round-trip, idempotency, invariants) and crash freedom across generated inputs. Use this layer when hand-picked examples cannot cover the input space.
3. **BDD Executable Specifications (`rstest-bdd`, `.feature` files)**
   Prove user-visible domain rules and observable boundary outcomes in business language. Reserve this layer for behavior a stakeholder can read and validate.
4. **Integration & Process-Boundary Suites (`tests/`)**
   Prove cross-module data flow through the public library API or technical CLI mechanics. Use this layer for signals, pipes, non-UTF-8 arguments, and fault injection.
5. **Unit Suites & Doctests (`#[cfg(test)]`, `///` examples)**
   Prove private helper logic, branch-complete error variants, boundary conditions, and public API usage examples. Keep unit tests fast, in-memory, and deterministic.
6. **Criterion Benchmarks (`benches/`)**
   Measure throughput, latency distributions, and algorithmic scaling against a baseline. Benchmarks answer performance questions and do not replace correctness tests.
7. **Mutation & Coverage Audits (`mutarust`, `cargo-mutants`, `cargo llvm-cov`)**
   Audit existing suites empirically to find uncovered branches and weak assertions. Use audit findings to add missing tests in Layers 2 through 5.

## 2. Pairwise Boundary Arbitration Rules

### BDD (`rust-bdd-testing`) vs. Integration (`rust-integration-testing`)

- **Choose BDD** when the scenario states a domain rule with observable business inputs and outputs.
- **Choose Integration** for technical plumbing, OS signals (`SIGINT`), broken pipes (`SIGPIPE`), raw byte streams, races, or `test-utils` seams.
- **Rule:** If a Gherkin step mentions Rust types, internal structs, or POSIX signal numbers, demote the test to `rust-integration-testing`.

### `Scenario Outline` vs. `#[rstest]` Table-Driven Unit Test

- **Choose `Scenario Outline`** only when each row represents a distinct business rule that a domain expert reviews.
- **Choose `#[rstest]`** when rows exercise parser edge cases, boundary integers, character encodings, or combinatoric input matrices.
- **Rule:** Keep `Examples:` tables under 10 rows. Push exhaustive input matrices down to `table-driven.md` in unit suites.

### `proptest` vs. `cargo-fuzz`

- **Choose `proptest`** (`property-based.md`) for fast, deterministic PR-gate checks over structured domain generators with automatic shrinking.
- **Choose `cargo-fuzz`** (`fuzzing.md`) for coverage-guided byte streams, deeply nested grammars (5,000+ tokens), and crash freedom.
- **Rule:** Run `proptest` in standard test suites. Run `cargo-fuzz` in dedicated fuzz targets and convert minimized crashes into unit tests.

### Unit (`rust-unit-testing`) vs. Integration (`rust-integration-testing`)

- **Choose Unit** when you test a single module's private branches, error variants, or isolated trait fakes.
- **Choose Integration** when you test how multiple modules cooperate through the crate's public API or spawned binary.
- **Rule:** Do not duplicate branch-by-branch unit cases in `tests/`. Test the cross-module composition once at the integration boundary.

## Completion

Completion: every guarantee under discussion is assigned to one layer with a stated arbitration reason.
