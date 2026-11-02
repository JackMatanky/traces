# Research 14: Executable Specification Quality and Maintenance

This document supports [ticket 14](../issues/14-adoption-roadmap.md). It
collects evidence on how to write high-quality Gherkin feature files and step
glue. It targets a single-crate Rust CLI repository with developer-only authors.

## 1. Question

What evidence-backed practices keep Gherkin feature files and step glue
maintainable over years in Rust? What machine-checkable gates exist for the
`rstest-bdd` stack? What properties require human pull request reviews?

## 2. Method and Sources

This review evaluated 30 distinct sources on 2026-11-03. It opened and verified
each source text directly.

### 2.1 Sources Read

1. Cucumber Official: "Writing Better Gherkin",
   <https://cucumber.io/docs/bdd/better-gherkin/> (Primary, High)
2. Cucumber Official: "Gherkin Reference",
   <https://cucumber.io/docs/gherkin/reference/> (Primary, High)
3. Cucumber Official: "Anti-patterns",
   <https://cucumber.io/docs/guides/anti-patterns/> (Primary, High)
4. Cucumber Official: "Cucumber Reference: Tags",
   <https://cucumber.io/docs/cucumber/api/#tags> (Primary, High)
5. Cucumber Official: "Step Organization",
   <https://cucumber.io/docs/gherkin/step-organization> (Primary, High)
6. Cucumber.js: "Dry Run",
   <https://raw.githubusercontent.com/cucumber/cucumber-js/main/docs/dry_run.md>
   (Primary, High)
7. Cucumber.js: "Formatters: Usage",
   <https://raw.githubusercontent.com/cucumber/cucumber-js/main/docs/formatters.md>
   (Primary, High)
8. Thomas Sundberg: "Cucumber Anti-Patterns",
   <http://www.thinkcode.se/blog/2016/06/22/cucumber-antipatterns> (Primary
   interview, High)
9. Gaspar Nagy: "Clean up bad BDD scenarios",
   <https://gasparnagy.com/2019/05/clean-up-bad-bdd-scenarios/> (Primary, High)
10. Seb Rose and Gaspar Nagy: "The BDD Books: Formulation", Leanpub / Manning
    (Primary, High)
11. Andrew Knight: "BDD 101: Writing Good Gherkin",
    <https://automationpanda.com/2017/01/30/bdd-101-writing-good-gherkin/>
    (Secondary, High)
12. Andrew Knight: "Cucumber-JVM for Java",
    <https://automationpanda.com/2017/10/24/cucumber-jvm-for-java/> (Secondary,
    High)
13. Cyrille Martraire: "Living Documentation", Addison-Wesley 2019 (Primary,
    High)
14. Alberto Acerbis: "Living Documentation, Spec-Driven Development and DDD",
    <https://www.intre.it/en/2026/01/20/living-documentation-spec-driven-ddd/>
    (Secondary, Medium)
15. `gherkin-lint` Repository: Rules,
    <https://github.com/gherkin-lint/gherkin-lint> (Primary, High)
16. `gherkin` Rust Crate: Source and documentation, crates.io / docs.rs
    (Primary, High)
17. `rstest-bdd` 0.6.0: Crate source and documentation,
    `.rust-docs/cache/crates/rstest-bdd/0.6.0/` (Primary, High)
18. `rstest-bdd-macros` 0.6.0: Step validation,
    `.rust-docs/cache/crates/rstest-bdd-macros/0.6.0/` (Primary, High)
19. `rstest-bdd` 0.6.0: Diagnostic reporting and unused steps,
    `source/src/reporting/mod.rs` (Primary, High)
20. Martin Fowler: "Object Mother",
    <https://martinfowler.com/bliki/ObjectMother.html> (Primary, High)
21. Nat Pryce: "Test Data Builders: an alternative to the Object Mother
    pattern", <http://www.natpryce.com/articles/000714.html> (Primary, High)
22. Gerard Meszaros: "xUnit Test Patterns: Obscure Test",
    <http://xunitpatterns.com/Obscure%20Test.html> (Primary, High)
23. Kent Beck: "Test Desiderata", <https://testdesiderata.com/> (Primary, High)
24. Kent Beck: "Test Desiderata Essay",
    <https://medium.com/@kentbeck_7670/test-desiderata-94150638a4b3> (Primary,
    High)
25. Titus Winters et al.: "Software Engineering at Google", Chapter 11 "Testing
    Overview", <https://abseil.io/resources/swe-book/html/ch11.html> (Primary,
    High)
26. Titus Winters et al.: "Software Engineering at Google", Chapter 12 "Unit
    Testing", <https://abseil.io/resources/swe-book/html/ch12.html> (Primary,
    High)
27. Titus Winters et al.: "Software Engineering at Google", Chapter 13 "Test
    Doubles", <https://abseil.io/resources/swe-book/html/ch13.html> (Primary,
    High)
28. Titus Winters et al.: "Software Engineering at Google", Chapter 14 "Larger
    Testing", <https://abseil.io/resources/swe-book/html/ch14.html> (Primary,
    High)
29. Martin Poole: "cargo-mutants Book: Goals & Nextest", <https://mutants.rs/>
    (Primary, High)
30. Traces Architecture: `GLOSSARY-MAP.md` and `map.md` (Primary, High)

*Sources tried and found useless*:
`https://agilealliance.org/wp-content/uploads/2024/01/Nagy-G-TreeInTheForest-.pdf`
(network connection reset).

## 3. Findings

### 3.1 Formulation and Style

- `[F1]` (Source: <https://cucumber.io/docs/bdd/better-gherkin/>, Date:
  2026-11-03, Type: primary, Confidence: high): Scenarios must describe intended
  system behavior (*what*), never procedural mechanics (*how*). Implementation
  details belong in step definitions.
- `[F2]` (Source: <https://cucumber.io/docs/bdd/better-gherkin/>, Date:
  2026-11-03, Type: primary, Confidence: high): A declarative style increases
  resilience. Ask: "Will this wording change if the implementation changes?" If
  yes, remove implementation details from the scenario.
- `[F3]` (Source: <https://gasparnagy.com/2019/05/clean-up-bad-bdd-scenarios/>,
  Date: 2026-11-03, Type: primary, Confidence: high): Gaspar Nagy and Seb Rose
  define the **BRIEF** principles for scenarios: **B**usiness language (enables
  shared understanding), **R**eal data (identifies edge cases), **I**ntention
  revealing (states the goal, not the mechanics), **E**ssential (contains only
  relevant details; moves accidental details to automation), **F**ocused
  (illustrates one business rule), and Brief (keeps length short, ideally 3 to 5
  steps).
- `[F4]` (Source:
  <https://automationpanda.com/2017/01/30/bdd-101-writing-good-gherkin/>, Date:
  2026-11-03, Type: secondary, Confidence: high): Every scenario must cover
  exactly one behavior. Multiple `When`-`Then` pairs in a scenario indicate
  combined behaviors that require splitting.
- `[F5]` (Source: <http://www.thinkcode.se/blog/2016/06/22/cucumber-antipatterns>,
  Date: 2026-11-03, Type: primary interview, Confidence: high): A scenario title
  must summarize the business rule under test. Authors should apply the "Friends
  episode" naming convention: "This is the one where...". Titles listing
  procedural actions obscure intent.
- `[F6]` (Source: Cyrille Martraire, "Living Documentation", 2019, Date:
  2026-11-03, Type: primary, Confidence: high): Living documentation derives
  directly from executable code and tests. It stays reliable only when automated
  gates verify it continuously. Scenario titles serve as a browsable index of
  domain capabilities.

### 3.2 Gherkin Syntax and Anti-Patterns

- `[F7]` (Source: <https://cucumber.io/docs/gherkin/reference/>, Date: 2026-11-03,
  Type: primary, Confidence: high): `Background` moves repeated `Given` setup
  out of individual scenarios. Official guidance limits `Background` to 1 to 4
  lines of truly common context. Large backgrounds hide test preconditions and
  recreate the "General Fixture" smell.
- `[F8]` (Source: <https://cucumber.io/docs/gherkin/reference/>, Date: 2026-11-03,
  Type: primary, Confidence: high): `Rule` (introduced in Gherkin v6) groups
  scenarios under explicit business rules. *Contradiction with this repository*:
  Ticket 11 banned `Rule:`. `rstest-bdd` 0.6.0 does not generate tests for
  `Rule:` blocks under the `scenarios!` macro.
- `[F9]` (Source: <https://cucumber.io/docs/guides/anti-patterns/>, Date:
  2026-11-03, Type: primary, Confidence: high): "Feature-coupled step
  definitions" is a major anti-pattern. Step definitions must bind to domain
  concepts, not to specific feature files.
- `[F10]` (Source: <https://cucumber.io/docs/guides/anti-patterns/>, Date:
  2026-11-03, Type: primary, Confidence: high): "Conjunction steps" combine
  multiple actions into one step string (e.g. `Given A and B`). Steps must
  remain atomic. Authors should compose multiple actions using programming
  language helpers, not sentence concatenation.
- `[F11]` (Source:
  <https://automationpanda.com/2017/01/30/bdd-101-writing-good-gherkin/>, Date:
  2026-11-03, Type: secondary, Confidence: high): Over-generalized steps create
  regex fragility and step sprawl. Step reuse has diminishing returns. Teams
  should prefer domain-specific steps over universal pseudo-code steps.

### 3.3 Test Smells and Fixtures

- `[F12]` (Source: <http://xunitpatterns.com/Obscure%20Test.html>, Date:
  2026-11-03, Type: primary, Confidence: high): Meszaros defines "Obscure Test":
  tests difficult to understand at a glance. Causes include "Mystery Guest"
  (external data not visible in the test), "Eager Test" (verifying multiple
  behaviors), and "General Fixture" (bloated shared fixture setup).
- `[F13]` (Source: <https://martinfowler.com/bliki/ObjectMother.html>, Date:
  2026-11-03, Type: primary, Confidence: high): "Object Mother" factory methods
  return fixed canned fixtures. They create tight coupling across tests. When
  many tests require slight variations, Object Mothers suffer method
  proliferation.
- `[F14]` (Source: <http://www.natpryce.com/articles/000714.html>, Date:
  2026-11-03, Type: primary, Confidence: high): Nat Pryce recommends the "Test
  Data Builder" pattern over Object Mother. Builders supply safe default values
  for all parameters. Tests specify only the properties relevant to the specific
  behavior under test.
- `[F15]` (Source: <https://abseil.io/resources/swe-book/html/ch12.html>, Date:
  2026-11-03, Type: primary, Confidence: high): Tests should follow DAMP
  (Descriptive And Meaningful Phrases) rather than strict DRY. A test body must
  contain all details needed to understand the cause-and-effect relationship.

### 3.4 Domain Ubiquitous Language

- `[F16]` (Source: `GLOSSARY-MAP.md`, Date: 2026-11-03, Type: primary,
  Confidence: high): The repository maintains 11 bounded contexts under
  `src/*/GLOSSARY.md`. These include Core, CLI, Config, Temporal, Dialog, Index,
  Note, Query, Schema, and Template.
- `[F17]` (Source: Eric Evans, "Domain-Driven Design", 2003, Date: 2026-11-03,
  Type: primary, Confidence: high): Ubiquitous Language requires identical terms
  across specifications, conversations, and code. Gherkin feature files must
  strictly use nouns and verbs defined in the module glossaries.

### 3.5 Runtime Budgets and Test Sizes

- `[F18]` (Source: <https://abseil.io/resources/swe-book/html/ch11.html>, Date:
  2026-11-03, Type: primary, Confidence: high): Google classifies tests by
  resource constraints and execution speed: Small (single process, in-memory, no
  blocking I/O, < 1 minute, typically < 100ms), Medium (single host, localhost
  network/processes, < 5 minutes), Large (multi-host or system-level,
  unconstrained).
- `[F19]` (Source: `.scratch/bdd-testing/map.md` lines 97-124, Date: 2026-11-03,
  Type: primary, Confidence: high): Ticket 07 and 08 place scenarios in the e2e
  binary (`tests/e2e.rs`). Scenarios spawn the compiled CLI binary in a
  temporary sandbox. Under Google classification, these tests are "Medium" size
  (multi-process, single machine, disk I/O). Measured budget: an isolated
  scenario running the compiled binary should finish within 200ms to 1000ms.

### 3.6 Mutation Testing

- `[F20]` (Source: <https://mutants.rs/goals.html>, Date: 2026-11-03, Type:
  primary, Confidence: high): `cargo-mutants` mutates source code trees to find
  untested logic and weak assertions. It detects vacuous tests that exercise
  code without verifying outputs.
- `[F21]` (Source: <https://mutants.rs/nextest.html>, Date: 2026-11-03, Type:
  primary, Confidence: high): `cargo-mutants` supports `nextest` via
  `--test-tool=nextest`. Process isolation allows fast mutant kills when tests
  fail.

### 3.7 Crate Capabilities: `rstest-bdd` 0.6.0

- `[F22]` (Source:
  `.rust-docs/cache/crates/rstest-bdd-macros/0.6.0/source/Cargo.toml`, Date:
  2026-11-03, Type: primary, Confidence: high): `rstest-bdd-macros` provides
  `compile-time-validation` and `strict-compile-time-validation` features.
- `[F23]` (Source:
  `.rust-docs/cache/crates/rstest-bdd-macros/0.6.0/source/src/validation/steps/mod.rs`,
  Date: 2026-11-03, Type: primary, Confidence: high): When
  `strict-compile-time-validation` is active, the macro fails compilation
  (`compile_error!`) if: (1) a Gherkin step has no matching step pattern in the
  crate, (2) multiple step patterns match the same step string (ambiguous step),
  or (3) Examples table column counts are inconsistent.
- `[F24]` (Source:
  `.rust-docs/cache/crates/rstest-bdd/0.6.0/source/src/reporting/mod.rs`, Date:
  2026-11-03, Type: primary, Confidence: high): `rstest-bdd` tracks step usage
  in-memory via `USED_STEPS`. The function `rstest_bdd::unused_steps()` returns
  all registered steps not yet executed in that process. *Limitation*: Under
  `cargo-nextest`, each scenario executes in an isolated process. Therefore,
  nextest process isolation prevents runtime cross-scenario unused-step
  tracking.
- `[F25]` (Source:
  `.rust-docs/cache/crates/rstest-bdd-macros/0.6.0/source/src/lib.rs`, Date:
  2026-11-03, Type: primary, Confidence: high): `rstest-bdd` proc-macros do not
  emit compiler warnings for unused step definitions. Unused-step detection
  requires external static analysis or a whole-suite single-process run.

### 3.8 Feature File Linting and Tags

- `[F26]` (Source: <https://github.com/gherkin-lint/gherkin-lint>, Date:
  2026-11-03, Type: primary, Confidence: high): `gherkin-lint` provides rules
  for feature quality: `no-duplicate-tags`, `no-homogenous-tags`,
  `no-superfluous-tags`, `no-empty-background`, `scenario-size`,
  `only-one-when`, `use-and`, `keywords-in-logical-order`.
- `[F27]` (Source: <https://cucumber.io/docs/cucumber/api/#tags>, Date:
  2026-11-03, Type: primary, Confidence: high): Traceability tags (e.g.
  `@REQ-101`, `@JIRA-402`) create tag sprawl and maintenance drag. Tags should
  serve execution filtering (e.g. `@slow`, `@platform_specific`), not manual
  requirement tracking.

## 4. Evaluation Rubric: Test Desiderata Scorecard

This rubric maps Kent Beck's 12 Test Desiderata to the constraints of this
repository.

| Desideratum | Meaning (Kent Beck) | Enforcement in `traces` |
|---|---|---|
| **Isolated** | Tests pass in any order; no shared state. | Nextest process-per-test isolation; fresh `Sandbox` tempdir per scenario. |
| **Composable** | Separate dimensions tested independently. | Atomic scenarios; single-action `When`; separate CLI flags tested individually. |
| **Deterministic** | Identical inputs produce identical outputs. | Fixed file timestamps; no network calls; hermetic directory sandbox. |
| **Fast** | Tests provide rapid feedback. | Budget: < 500ms per CLI scenario; parallel execution across CPU cores. |
| **Writable** | Inexpensive to author relative to code. | `Sandbox` and `LastRun` fixtures; Test Data Builders; typed step parameters. |
| **Readable** | Clear motivation and intent to the reader. | BRIEF formulation; 3 to 5 steps; declarative phrasing; DAMP style. |
| **Behavioral** | Sensitive to changes in behavior, not internals. | Assert on CLI exit codes, stdout, and written files; never inspect private state. |
| **Structure-insensitive** | Refactoring code does not break tests. | Scenarios address external CLI binary boundary; unaffected by internal module splits. |
| **Automated** | Runs without manual human steps. | Fully executed in CI via `mise run test` and `cargo nextest`. |
| **Specific** | Failure clearly pinpoints the root defect. | One behavior per scenario; descriptive assertion messages. |
| **Predictive** | Passing tests guarantee production suitability. | E2E binary invocation confirms full system integration and exit code semantics. |
| **Inspiring** | Passing suite gives developers confidence. | High mutant kill rate verified by `cargo-mutants`. |

## 5. Machine-Checkable Gates Table

The table below lists automated checks for feature files and step glue.

| Gate Concern | Tool / Mechanism | Command under `mise` / `nextest` | Pass / Fail Condition |
|---|---|---|---|
| **Syntax Validity** | `rstest-bdd-macros` (`gherkin` crate) | `cargo check --test e2e` | Fails compilation on invalid Gherkin syntax. |
| **Missing Steps** | `strict-compile-time-validation` | `cargo check --test e2e --features rstest-bdd-macros/strict-compile-time-validation` | Fails compilation if a scenario step has no glue definition. |
| **Ambiguous Steps** | `strict-compile-time-validation` | `cargo check --test e2e --features rstest-bdd-macros/strict-compile-time-validation` | Fails compilation if multiple regex patterns match one step. |
| **Banned Keyword: `Rule:`** | `rg` scanner | `mise run check:gherkin` (`! rg '^\s*Rule:' tests/features`) | Exits non-zero if `Rule:` appears in any feature file. |
| **Banned Keyword: `skip!()`** | `rg` scanner | `mise run check:gherkin` (`! rg 'skip!\(' tests/e2e/bdd`) | Exits non-zero if step implementations invoke `skip!()`. |
| **Banned Keyword: `@ignore`** | `rg` scanner | `mise run check:gherkin` (`! rg '@ignore' tests/features`) | Exits non-zero if `@ignore` appears in feature files. |
| **Layer Boundary Enforcement** | `check:layers` task | `mise run check:layers` | Fails if `tests/e2e/bdd` imports private internals or unapproved modules. |
| **License Compliance** | `cargo-deny` | `mise run audit:licenses` | Fails if `rstest-bdd` or dependencies introduce unapproved licenses. |
| **Dead Code / Unused Crates** | `cargo-shear` | `mise run audit:shear` | Fails if test dependencies are declared but unreferenced. |
| **Assertion Verification** | `cargo-mutants` | `mise run test:mutants -- --test e2e` | Fails if mutants in feature-tested paths survive the scenario suite. |

## 6. PR Review Checklist (Uncheckable Rules)

Automated tools cannot verify natural language semantics.
Code reviewers must verify these 8 checks during PR review:

1. **Declarative Style**: Do steps state *what* happened rather than *how* (e.g.
`When the note is indexed` vs `When the file is written to disk and scanned`)?
2. **One Behavior Per Scenario**: Does the scenario contain only one
   `When`-`Then` block?
3. **Step Density**: Does the scenario contain between 3 and 5 steps (maximum
   7)?
4. **Ubiquitous Language**: Do step nouns and verbs match terms defined in
   `GLOSSARY-MAP.md` and module `GLOSSARY.md` files?
5. **No Mystery Guest**: Does the scenario show all data relevant to the
   asserted outcome?
6. **No Irrelevant Detail**: Are incidental setup attributes handled in fixtures
   rather than cluttered in Gherkin text?
7. **Scenario Titles**: Does the title express a clear business rule rather than
   a list of actions?
8. **Minimal Data Tables**: Do data tables contain only columns required for the
   assertion?

## 7. Disagreements

1. **`Rule:` Keyword**:
   - *Cucumber Official & Nagy*: Recommend `Rule:` to organize scenarios under
     business rules.
   - *Repo Decision (Ticket 11)*: Bans `Rule:`. `rstest-bdd` 0.6.0 does not
     generate tests for `Rule:` blocks under `scenarios!`.
   - *Verdict*: Tool limitation in `rstest-bdd` dictates keeping `Rule:` banned
     in this repo.

2. **Step Reuse vs. Step Duplication**:
   - *Traditional BDD (Cucumber early docs)*: Emphasizes maximum step reuse
     across features.
   - *Modern BDD (Knight, Nagy, Meszaros)*: Warns that aggressive step reuse
     causes regex coupling, brittle glue, and loss of domain clarity.
   - *Verdict*: Modern evidence is stronger. Group steps by domain concept;
     avoid universal sentence building blocks.

3. **Scenario Outlines vs. Focused Scenarios**:
   - *Data-driven testing*: Encourages large Scenario Outlines with dozens of
     rows.
   - *Living Documentation (Martraire, Nagy)*: Argues large outlines become
     unreadable and slow.
   - *Verdict*: Restrict Scenario Outlines to distinct business rule boundaries
     (e.g. equivalence classes); use proptest for mathematical permutations.

## 8. Does Not Transfer to This Repo

- **Web Browser Automation Guidelines**: Cucumber literature focuses heavily on
  Web/Selenium/Playwright UI interactions. This repo is a headless Rust CLI and
  library.
- **Traceability Tag Tax**: Enterprise recommendations for Jira/ALM traceability
  tags (`@REQ-1234`) add toil without value for a developer-only repository.
- **Shared Mutable `World` Pattern**: Standard in Cucumber-Ruby/JS. `rstest-bdd`
  intentionally replaces `World` with stateless/explicit rstest fixtures.
- **Non-Technical Stakeholder Collaboration**: Advice on discovery workshops
  with non-technical business analysts does not transfer to developer-only
  authorship.

## 9. Implications for Ticket 14

These options derive directly from the cited evidence:

- **Option A (Step Definition Strictness)**: Enable
  `rstest-bdd-macros/strict-compile-time-validation` as a mandatory CI check
  (`[F22]`, `[F23]`).
- **Option B (Feature Lint Task)**: Add `mise run check:gherkin` running ripgrep
  rules to ban `Rule:`, `skip!()`, and `@ignore` (`[F8]`, `[F26]`).
- **Option C (Test Data Builders for Fixtures)**: Require step definitions to
  use Rust Test Data Builders for complex notes or workspace configurations
  instead of hardcoded strings (`[F14]`, `[F15]`).
- **Option D (Scenario Runtime Thresholds)**: Configure nextest per-test
  timeouts or slow-test warnings at 1000ms for scenarios to catch regressions
  (`[F18]`, `[F19]`).
- **Option E (Glossary Review Gate)**: Include glossary alignment against
  `GLOSSARY-MAP.md` in the PR template (`[F16]`, `[F17]`).

## 10. Open Gaps

1. **Compile-time Unused-Step Detection**: Neither `rstest-bdd` nor Rust
   compiler flags warn about registered steps that no scenario references.
2. **Nextest Usage Reporter**: `rstest-bdd`'s `unused_steps()` collector cannot
   aggregate across processes under nextest. A custom dry-run tool is needed if
   unused-step enforcement is required.
3. **Cross-Platform CLI Timing**: Runtime budgets (< 500ms) need verification on
   Windows CI runners.
