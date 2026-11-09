# Reviewing BDD Executable Specifications

> Obey this 4-step review procedure to audit `.feature` scenarios, step definitions, and `rstest-bdd` harnesses.

## Step 1: Placement and Layer Audit

Check every scenario against the Six Placement Conditions in [`./formulation.md`](./formulation.md) and the layer rules in [`../../rust-testing-router/references/boundaries.md`](../../rust-testing-router/references/boundaries.md):

- Reject any scenario that tests internal struct methods, parser syntax tables, or mathematical edge cases without domain workflow value.
- Recommend demoting those cases to [`../../rust-unit-testing/SKILL.md`](../../rust-unit-testing/SKILL.md), [`../../rust-testing-router/references/table-driven.md`](../../rust-testing-router/references/table-driven.md), or [`../../rust-testing-router/references/property-based.md`](../../rust-testing-router/references/property-based.md).

## Step 2: Formulation and BRIEF Rubric Evaluation

Score each `.feature` file across all six rubric columns and present this Markdown table in your review output:

| Scenario / File | Declarative (Golden Wording) | Single `When` & 3-5 Steps | Concrete Domain Data | Three-Part Negative Outcome | Intention-Revealing Title | Zero Banned Constructs (`Rule:`, `*`, `@ignore`) |
|---|---|---|---|---|---|---|
| `<scenario name>` | Pass / Fail | Pass / Fail | Pass / Fail | Pass / Fail / N/A | Pass / Fail | Pass / Fail |

Flag any imperative step wording (CLI flags, SQL queries, file paths, or JSON syntax) and provide a domain rewrite.

## Step 3: Step and Fixture Hygiene Audit

Inspect the Rust step modules and fixture definitions against [`./rstest-bdd.md`](./rstest-bdd.md):

1. **Domain-grouped step modules:** Confirm step files are organized by domain concept (`steps/workspace.rs`), not coupled 1:1 to `.feature` filenames.
2. **Atomic steps:** Flag conjunction steps (`Given X and Y`) or `Given` steps that secretly execute the `When` action.
3. **Hermetic fixtures:** Confirm all state lives inside the per-scenario `#[fixture]` (`TempDir`, `Command::env_clear()`), with zero `static` state.
4. **Return types and assertions:** Confirm `#[then]` functions return `()` or `Result<(), E>` and use [`../../rust-testing-router/references/assertions.md`](../../rust-testing-router/references/assertions.md).
5. **Narrow lint attributes:** Confirm `#[expect(..., reason = "...")]` is used instead of bare `allow` attributes.

## Step 4: Silent-Green and Empirical Kill Verification

Run and check all four empirical gates before approving a BDD suite:

1. **Step gate check:** Confirm `step_gate.rs` runs and passes with zero undefined, ambiguous (tied-specificity), duplicate, or unused steps.
2. **Scenario discovery count:** Run `cargo nextest list --test features` and confirm generated tests match the `.feature` scenario count.
3. **Red-first assertion check:** Confirm that inverting the assertion or breaking the production command causes the scenario to fail.
4. **Mutation audit:** Cross-check changed modules with [`../../rust-testing-router/references/mutation.md`](../../rust-testing-router/references/mutation.md) to confirm no behavioral mutants escape.

## Completion

Completion: review output contains the Placement verdict, the completed 6-column BRIEF Rubric table, Step Hygiene findings, and verification of `step_gate` plus `cargo nextest list` scenario counts.
