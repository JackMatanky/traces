---
name: rust-bdd-testing
description: Gherkin .feature scenarios, Given/When/Then step definitions, and rstest-bdd harnesses. Use when writing or reviewing BDD specs.
metadata:
  version: "1.0.0"
  companion_to: "rust-testing-router"
---

# Rust BDD Testing (`rstest-bdd`)

BDD executable specifications check observable, stakeholder-meaningful capabilities through declarative Gherkin `.feature` files wired to `rstest-bdd` step functions.

## Precedence Rule

Local project specifications (such as `.scratch/bdd-testing/spec.md`), `AGENTS.md`, and project conventions take precedence over default Gherkin construct policies and layout rules in this skill.

## 5-Step Workflow

1. Gate scenario placement using the Six Placement Conditions in [`references/formulation.md`](references/formulation.md).
   Completion: output lists approved BDD scenarios and demotes structural, algebraic, or internal checks to unit, property, or integration layers.
2. Formulate declarative BRIEF Gherkin scenarios using [`references/formulation.md`](references/formulation.md).
   Completion: every scenario passes the Golden Wording Test, obeys the Three-Part Negative Outcome Contract on failure paths, and contains zero banned Gherkin constructs.
3. Automate step definitions and fixtures using [`references/rstest-bdd.md`](references/rstest-bdd.md).
   Completion: every Gherkin step resolves 1:1 to a registered `#[given]`, `#[when]`, or `#[then]` function with zero duplicate or ambiguous patterns.
4. Wire the acceptance harness and defeat silent-green traps using [`references/rstest-bdd.md`](references/rstest-bdd.md).
   Completion: `step_gate` passes, `cargo nextest list` scenario count increases as expected, and red-first assertions fail when the target contract is broken.
5. Review the executable specification using [`references/review.md`](references/review.md).
   Completion: output includes the 4-step review report, the 6-column BRIEF rubric table, and zero silent-green findings.

## Reference Guide

- [`references/formulation.md`](references/formulation.md) - Six Placement Conditions, BRIEF principles, Golden Wording Test, Three-Part Negative Outcome Contract, Gherkin construct policy, and step anti-patterns.
- [`references/rstest-bdd.md`](references/rstest-bdd.md) - `rstest-bdd =0.6.0` pairing, step macros, fixtures, data tables, `scenarios!`, 7 silent-green traps, and `step_gate.rs`.
- [`references/review.md`](references/review.md) - 4-step BDD review procedure and 6-column BRIEF evaluation rubric.
- [`../rust-testing-router/SKILL.md`](../rust-testing-router/SKILL.md) - shared references for layer boundaries, `rstest` fixtures, assertions, RAII cleanup, and mutation testing.
