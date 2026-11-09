# BDD Formulation and Gherkin Authoring

> Formulate Gherkin as a business-readable executable specification of observable system behavior, never as a script of internal API calls or UI clicks.

## 1. The Six Placement Conditions (Gate Before Writing Gherkin)

A guarantee belongs in a `.feature` file only if it satisfies all six placement conditions:

1. **Observable behavior:** Describes an externally visible system outcome, not an internal struct layout or private helper.
2. **Observable boundary channel:** Enters and exits through the public CLI, API, or persisted artifact surface.
3. **Stakeholder intent:** A domain reader or product owner cares if this rule changes.
4. **Not owned by a static or property gate:** Types, Clippy lints, or `proptest` laws do not already prove it.
5. **Hermetic sandbox:** Runs deterministically inside a per-scenario `TempDir` and scrubbed environment without live external services.
6. **Refactor-stable:** Renaming internal modules, structs, or private functions never changes the `.feature` text.

Use **Example Mapping** before drafting Gherkin. List the **Rule**, write concrete **Examples**, and capture open **Questions**. Demote combinatorial parser edge cases and arithmetic boundaries to [`../../rust-unit-testing/SKILL.md`](../../rust-unit-testing/SKILL.md) or [`../../rust-testing-router/references/property-based.md`](../../rust-testing-router/references/property-based.md).

## 2. BRIEF Principles and the Golden Wording Test

Every scenario must satisfy the **BRIEF** acronym (Seb Rose & Gaspar Nagy):

- **B - Business language:** Use domain vocabulary (`trace record`, `retention policy`), not implementation types (`Vec<Span>`, `SqlPool`).
- **R - Real data:** Use concrete, intention-revealing examples (`350 ms`, `"auth-service"`) that make cause and effect obvious.
- **I - Intention revealing:** State *what* the actor accomplishes in the `Scenario:` title and steps, not *how* the code executes it.
- **E - Essential:** Include only the preconditions that directly affect the `Then` outcome. Push defaults into fixtures.
- **F - Focused:** Cover a single business rule per scenario with a single `When` trigger and 3 to 5 total steps.

### The Golden Wording Test (and 1922 Test)

Ask two questions of every scenario:

1. **1922 Test:** Could a domain expert understand this scenario if the technology stack changed completely?
2. **Golden Wording Test:** Does this `.feature` text stay unchanged if you replace the CLI flags or database engine?

Check step roles strictly:

- `Given` establishes precondition state (past/present state).
- `When` triggers the single target action (a single `When` per scenario).
- `Then` asserts the observable outcome without mutating state.

## 3. The Three-Part Negative Outcome Contract

Every failure-path or rejection scenario must assert all three parts of the negative contract in its `Then` steps:

1. **Failure signal:** Non-zero exit status or `Err` outcome.
2. **Diagnostic category or stable token:** Match the structured error category, diagnostic code, or customer-facing problem message.
3. **Zero partial side effects:** Existing files, database records, and output streams (`stdout` on error) remain uncorrupted and unchanged.

```gherkin
Scenario: Reject retention policy with zero-day window
  Given an initialized workspace with a 30-day retention policy
  When the operator sets the retention window to 0 days
  Then the command fails with a configuration validation error
  And the workspace retention policy remains 30 days
```

## 4. Gherkin Construct Policy and File Naming

Check local project specs first. When no local spec overrides them, obey these rules:

- **File naming:** Organize features by capability under `tests/features/<cap>/<cap>-<topic>.feature`.
- **Single `When`:** Write a single `When` step per scenario. Use the `And` and `But` keywords to connect multiple `Given` preconditions or `Then` outcomes.
- **`Background`:** Allowed only for <= 3 `Given` lines that apply to every scenario in the file. Never put `When` actions in `Background`.
- **`Scenario Outline`:** Use sparingly only when rows represent distinct business rules (unless banned by the local project spec). Use the `#[rstest]` macro in unit tests for mechanical boundary tables.
- **Banned constructs:**
  - Never use `Rule:` keyword blocks (`rstest-bdd 0.6.0` silently ignores scenarios nested inside `Rule:` unless manually bound).
  - Never use the `*` bullet step keyword (hides `Given`/`When`/`Then` role semantics).
  - Never use `@ignore` tags (`rstest-bdd 0.6.0` does not skip `@ignore` scenarios by default).

## 5. Step and State Anti-Patterns

Avoid these four Cucumber anti-patterns:

1. **Feature-coupled step definitions:** Do not organize Rust step files 1:1 per `.feature` file. Group step definitions by domain concept (`steps/workspace.rs`, `steps/query.rs`) so steps stay reusable across features.
2. **Conjunction steps:** Never write a single step pattern containing `and` (`Given a workspace exists and contains 3 traces`). Split it into two atomic steps connected by Gherkin `And`.
3. **Leaky global state:** Never share state between scenarios through `static` variables, process `env::set_var`, or shared temp directories. Pass all state through a per-scenario `#[fixture]` struct.
4. **Speculative unused steps:** Never commit step functions that are not matched by any `.feature` file. Enforce zero unused steps in `step_gate.rs`.

## Completion

Completion: all scenarios pass the Six Placement Conditions, Golden Wording Test, and Three-Part Negative Outcome Contract, with zero banned constructs.
