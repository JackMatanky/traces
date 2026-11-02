# BDD Adoption Evidence

## Question

What does primary literature and field evidence say about how to adopt BDD
successfully? How do BDD adoptions fail? What are the criteria for pilots, solo
teams, and when to avoid Gherkin?

## Method and Source List

We examined 26 primary and secondary sources on 2026-11-03. We inspected
original texts, empirical surveys, case studies, and engineering guidelines.

### Primary Sources

1. Dan North, "Introducing BDD" (2006): <https://dannorth.net/introducing-bdd/>
2. Dan North, "What's in a Story?" (2007):
   <https://dannorth.net/whats-in-a-story/>
3. Gojko Adzic, "Specification by Example" (2011, Manning):
   <https://livebook.manning.com/book/specification-by-example/>
4. Gojko Adzic, "SBE 10-Year Retrospective Survey" (2020):
   <https://gojko.net/2020/03/17/sbe-10-years.html>
5. Gojko Adzic, "Popular Collaborative Agile Software Specifications Models"
   (2012): <https://www.methodsandtools.com/archive/speccollab.html>
6. Aslak Hellesoy, "The World's Most Misunderstood Tool" (2014):
   <https://cucumber.io/blog/bdd/the-world-s-most-misunderstood-tool/>
7. Aslak Hellesoy, "Cucumber Anti-Patterns Part 1 & 2" (2016):
   <https://cucumber.io/blog/bdd/cucumber-anti-patterns-part-one/>
8. Cucumber Project, "Writing Better Gherkin":
   <https://cucumber.io/docs/guides/better-gherkin/>
9. Cucumber Project, "When to Use Cucumber":
   <https://cucumber.io/docs/guides/overview/#when-should-i-use-cucumber>
10. Seb Rose and Gaspar Nagy, "Discovery: Explore Behaviour Using Examples"
    (2018): <https://bddbooks.com/>
11. Seb Rose and Gaspar Nagy, "Formulation: Document Examples with
    Given/When/Then" (2021): <https://bddbooks.com/>
12. Seb Rose, "Keep Your Scenarios BRIEF" (2019):
    <https://cucumber.io/blog/bdd/keep-your-scenarios-brief/>
13. Seb Rose, "Gherkin Rules" (2021):
    <https://cucumber.io/blog/bdd/gherkin-rules/>
14. Gaspar Nagy, "Clean Up Bad BDD Scenarios" (2019):
    <https://gasparnagy.com/2019/05/clean-up-bad-bdd-scenarios/>
15. Matt Wynne, "Introducing Example Mapping" (2015):
    <https://cucumber.io/blog/bdd/example-mapping-introduction/>
16. Liz Keogh, "Step Away from the Tools" (2011):
    <https://lizkeogh.com/2011/04/16/step-away-from-the-tools/>
17. Liz Keogh, "Using BDD with Legacy Systems" (2014):
    <https://lizkeogh.com/2014/11/17/using-bdd-with-legacy-systems/>
18. Liz Keogh, "Cynefin for Devs" (2012):
    <https://lizkeogh.com/2012/03/11/cynefin-for-devs/>
19. Liz Keogh, "Acceptance Criteria vs Scenarios" (2011):
    <https://lizkeogh.com/2011/06/20/acceptance-criteria-vs-scenarios/>
20. M. M. Binamungu, S. M. Embury, and N. Konstantinou, "Maintaining Behaviour
    Driven Development Specifications" (SANER 2018, IEEE): DOI
    10.1109/SANER.2018.8330206

### Secondary Sources

21. Chris Parsons, "Why BDD Works Solo" (2011):
    <https://web.archive.org/web/20140209180709/http://chrismdp.com/2011/08/why-bdd-works-solo/>
22. Martin Fowler, "GivenWhenThen" (2013):
    <https://martinfowler.com/bliki/GivenWhenThen.html>
23. Martin Fowler, "SpecificationByExample" (2004/2012):
    <https://martinfowler.com/bliki/SpecificationByExample.html>
24. Ham Vocke, "The Practical Test Pyramid" (2018):
    <https://martinfowler.com/articles/practical-test-pyramid.html>
25. Dave Farley, "Acceptance Testing" (Continuous Delivery, 2010):
    <https://davefarley.net/>
26. Steven Thomas, "Seven Things to do When Starting SBE" (2015):
    <https://itsadeliverything.com/seven-things-to-do-when-starting-specification-by-example-with-cucumber>

*Useless sources tried and discarded:* Manning general search snippets on
generative AI and iText; unreadable studfile OCR scans; specdd.ai tool sales
pitch.

## Findings

### 1. Dan North: Core BDD Foundations

- `[A01]` Dan North created BDD to address recurring TDD confusions: what to
  test, what not to test, how to name tests, and why tests fail.
  (dannorth.net/introducing-bdd/, 2026-11-03, primary, confidence: high).
- `[A02]` Test method names should be complete sentences starting with "should".
  If a sentence does not fit, the behaviour belongs in another class.
  (dannorth.net/introducing-bdd/, 2026-11-03, primary, confidence: high).
- `[A03]` A test fails for three reasons: a bug exists, behaviour moved, or the
  specification became obsolete. Obsolete tests must be deleted immediately.
  (dannorth.net/introducing-bdd/, 2026-11-03, primary, confidence: high).
- `[A04]` Stories use the template: As a [role], I want [feature], so that
  [benefit]. Acceptance criteria are scenarios using Given [context], When
  [event], Then [outcome]. (dannorth.net/whats-in-a-story/, 2026-11-03, primary,
  confidence: high).
- `[A05]` Scenarios must describe user activities, not code design. Givens
  specify all necessary context and no more. Events are single actions. Break
  down stories with more than five scenarios. (dannorth.net/whats-in-a-story/,
  2026-11-03, primary, confidence: high).

### 2. Gojko Adzic: Specification by Example and Empirical Survey

- `[A06]` Adzic surveyed 514 practitioners in 2020. Teams using examples
  reported product quality as 22% Great, 71% Good, and 7% Poor. Teams without
  examples reported 8% Great, 78% Good, and 14% Poor.
  (gojko.net/2020/03/17/sbe-10-years.html, 2026-11-03, primary, confidence:
  high).
- `[A07]` Given-When-Then is the dominant specification syntax (71% of surveyed
  teams). Bullet lists accounted for 15% and tables for 9%.
  (gojko.net/2020/03/17/sbe-10-years.html, 2026-11-03, primary, confidence:
  high).
- `[A08]` GWT open-source tools (Cucumber, SpecFlow) dominate automation (82%).
  Developers automate specifications in 48% of teams, testers in 31%, any team
  member in 11%, and external test contractors in 9%.
  (gojko.net/2020/03/17/sbe-10-years.html, 2026-11-03, primary, confidence:
  high).
- `[A09]` 29% of teams use examples without automating them. Automating
  specifications doubled the proportion of teams reporting "Great" quality from
  13% to 26%. (gojko.net/2020/03/17/sbe-10-years.html, 2026-11-03, primary,
  confidence: high).
- `[A10]` For existing codebases, teams must build functional test automation
  before introducing collaborative specification. Automating along system
  boundaries stabilises existing functionality and creates a reliable safety
  net. (Specification by Example ch. 4; livebook.manning.com, 2026-11-03,
  primary, confidence: high).
- `[A11]` Four key warning signs show that specification is failing: tests
  change without code changes, boomerangs (stories returning for rework within
  30 days), test slippage, and shotgun surgery across multiple specifications.
  (Specification by Example ch. 4; livebook.manning.com, 2026-11-03, primary,
  confidence: high).
- `[A12]` Test automation code must not be treated as second-class code.
  Specifications must automate along business boundaries and avoid checking
  business logic through user interfaces. (Specification by Example ch. 4 & 10;
  livebook.manning.com, 2026-11-03, primary, confidence: high).

### 3. Aslak Hellesoy and Cucumber Core Team

- `[A13]` "Cucumber is not a testing tool." Its purpose is establishing and
  validating shared domain understanding before implementation.
  (cucumber.io/blog/bdd/the-world-s-most-misunderstood-tool/, 2026-11-03,
  primary, confidence: high).
- `[A14]` Hellesoy explicitly warns developer-only teams: if non-technical
  stakeholders or domain experts do not read Gherkin documents, teams should
  avoid Cucumber and use standard xUnit tools (JUnit, Mocha, RSpec).
  (cucumber.io/blog/bdd/the-world-s-most-misunderstood-tool/, 2026-11-03,
  primary, confidence: high).
- `[A15]` Major anti-patterns: writing scenarios after code, business analysts
  writing scenarios in isolation, developers writing scenarios without business
  review, UI-scripted steps, and incidental data clutter.
  (cucumber.io/blog/bdd/cucumber-anti-patterns-part-one/, 2026-11-03, primary,
  confidence: high).
- `[A16]` Scenarios must be declarative, not imperative. Steps must describe
  intent and outcomes rather than user interface clicks or technical mechanics.
  (cucumber.io/docs/guides/better-gherkin/, 2026-11-03, primary, confidence:
  high).
- `[A17]` Step definitions coupled to specific features cause duplication and
  maintenance rot. Steps must be organized by domain concept. Step definitions
  must never call other step definitions directly; use domain helper functions
  instead. (cucumber.io/blog/bdd/cucumber-anti-patterns-part-two/, 2026-11-03,
  primary, confidence: high).

### 4. Seb Rose and Gaspar Nagy: Discovery, Formulation, BRIEF

- `[A18]` BDD consists of three distinct practices: Discovery (exploring
  requirements), Formulation (documenting examples as Gherkin), and Automation
  (connecting Gherkin to code). (bddbooks.com;
  cucumber.io/blog/bdd/interview-with-seb-rose-and-gaspar-nagy/, 2026-11-03,
  primary, confidence: high).
- `[A19]` The BRIEF principles define high-quality scenarios: Business language,
  Real data, Intention revealing, Essential details only, Focused on a single
  rule, and Brief (under 5 to 6 steps).
  (cucumber.io/blog/bdd/keep-your-scenarios-brief/, 2026-11-03, primary,
  confidence: high).
- `[A20]` Without business collaboration, scenarios become dry, technical,
  data-driven tests. Business stakeholders stop reading them, feedback loops
  collapse, and Gherkin becomes pure overhead.
  (gasparnagy.com/2019/05/clean-up-bad-bdd-scenarios/, 2026-11-03, primary,
  confidence: high).
- `[A21]` The Gherkin `Rule:` keyword groups scenarios under specific business
  rules. Scenarios should illustrate a single rule, preventing multiple tests
  from failing when one rule changes. (cucumber.io/blog/bdd/gherkin-rules/,
  2026-11-03, primary, confidence: high).

### 5. Liz Keogh: Conversation Over Automation and Legacy Systems

- `[A22]` "BDD is the art of using examples in conversation to illustrate
  behaviour." The priority is: conversations > capturing conversations >
  automating conversations. (lizkeogh.com/2011/04/16/step-away-from-the-tools/,
  2026-11-03, primary, confidence: high).
- `[A23]` If a team cannot hold conversations with stakeholders, they cannot do
  BDD. Teams must not use natural-language BDD tools unless those tools engage
  non-technical stakeholders.
  (lizkeogh.com/2011/04/16/step-away-from-the-tools/, 2026-11-03, primary,
  confidence: high).
- `[A24]` On legacy codebases, do not attempt comprehensive BDD upfront. Write
  characterisation or approval tests in standard test frameworks. Automate only
  2 to 3 critical business paths in BDD before altering behaviour.
  (lizkeogh.com/2014/11/17/using-bdd-with-legacy-systems/, 2026-11-03, primary,
  confidence: high).
- `[A25]` Under the Cynefin framework, BDD discovery belongs in the Complex
  domain, where requirements are emergent. Complicated and simple domains do not
  require heavy BDD ceremonies. (lizkeogh.com/2012/03/11/cynefin-for-devs/,
  2026-11-03, primary, confidence: high).

### 6. Matt Wynne: Example Mapping

- `[A26]` Example Mapping is a 25-minute collaborative timebox using four index
  cards: Story (yellow), Rule (blue), Example (green), and Question (red).
  (cucumber.io/blog/bdd/example-mapping-introduction/, 2026-11-03, primary,
  confidence: high).
- `[A27]` A table covered in red cards means the story is not ready for
  development. A table covered in blue cards means the story must be sliced. A
  rule with many examples indicates excessive complexity.
  (cucumber.io/blog/bdd/example-mapping-introduction/, 2026-11-03, primary,
  confidence: high).
- `[A28]` Amigos must capture rough examples first using the "Friends episode"
  naming convention ("The one where..."). Formal Gherkin must be written after
  the session, not during it.
  (cucumber.io/blog/bdd/example-mapping-introduction/, 2026-11-03, primary,
  confidence: high).

### 7. Empirical Maintenance Study (Binamungu et al., IEEE SANER 2018)

- `[A29]` Empirical survey of 75 professional BDD practitioners across 26
  countries. Top benefits: domain terms understood by users (67%), improved team
  communication (61%), executable specifications (52%), code intention
  understood by maintainers (50%). (SANER 2018; DOI 10.1109/SANER.2018.8330206,
  2026-11-03, primary, confidence: high).
- `[A30]` Main adoption challenges: alters traditional developer workflows
  (51%), benefits are difficult to quantify (35%), steep learning curve (28%),
  lowers short-term productivity (20%). (SANER 2018; DOI
  10.1109/SANER.2018.8330206, 2026-11-03, primary, confidence: high).
- `[A31]` Maintenance burdens: 61% report duplication makes specifications
  difficult to extend, leading to frozen functionality. 49% report duplication
  causes slow execution. 43% report duplication reduces comprehension. (SANER
  2018; DOI 10.1109/SANER.2018.8330206, 2026-11-03, primary, confidence: high).
- `[A32]` English prose refactoring lacks tooling support. Detecting duplication
  between BDD scenarios and unit tests is extremely difficult for existing
  static analysis tools. (SANER 2018; DOI 10.1109/SANER.2018.8330206,
  2026-11-03, primary, confidence: high).

### 8. Solo and Developer-Only Teams

- `[A33]` Chris Parsons argues BDD works for solo developers by forcing them to
  step into stakeholder shoes ("playing chess against yourself"). It prevents
  over-engineering by asking "does this code provide user value?".
  (chrismdp.com/2011/08/why-bdd-works-solo, 2026-11-03, secondary, confidence:
  medium).
- `[A34]` Ham Vocke and Martin Fowler argue that when external business
  stakeholders do not read specifications, teams achieve identical clarity using
  xUnit tests with well-named helper methods, avoiding DSL translation layers.
  (martinfowler.com/articles/practical-test-pyramid.html, 2026-11-03, secondary,
  confidence: high).

## Explicit List of Failure Modes and Mitigations

| Failure Mode | Root Cause | Observable Symptom | Mitigation from Sources | Source Citation |
|---|---|---|---|---|
| UI-Script Gherkin | Writing manual test steps ("click button", "fill input") in Gherkin | Fragile tests that break on cosmetic UI changes; long scenarios | Write declarative steps focusing on business intent and capability; automate below UI skin | `[A12]`, `[A15]`, `[A16]`, `[A19]` |
| Scenario Bloat | Writing Gherkin for every permutation, edge case, and data combination | Slow test runs; hundreds of unread scenarios; duplicate tests | Restrict Gherkin to illustrative examples; push boundary and mathematical permutations to unit tests | `[A05]`, `[A19]`, `[A27]`, `[A31]` |
| Glue-Code Rot | Feature-coupled step definitions; step regex regex sprawl; steps calling steps | Duplicate step logic; broken steps when wording changes; untraceable failures | Organize step files by domain concepts; call domain helper methods from steps, never steps from steps | `[A17]`, `[A31]`, `[A32]` |
| Duplicated Coverage | Testing the same invariant in unit tests and BDD feature files | Dual maintenance burden; refactorings require changing both test layers | Define clear layer responsibilities: unit tests cover logic paths; BDD tests cover end-to-end user guarantees | `[A31]`, `[A32]`, `[A34]` |
| Scenarios as Test Replacement | Treating BDD as a QA automation tool rather than specification | Writing scenarios after coding; complex test harnesses; missing unit tests | Write specifications before implementation; maintain the test pyramid; do not discard unit tests | `[A13]`, `[A15]`, `[A22]`, `[A34]` |
| No Collaboration (Silo BDD) | Developers or testers writing Gherkin without business involvement | Dry, technical scenarios with dummy data (user1); business never reads them | Stop using Gherkin if no external readers exist; use Example Mapping; use plain code tests | `[A14]`, `[A20]`, `[A23]`, `[A26]` |
| Shotgun Surgery | Scenarios testing multiple rules in one scenario block | One small code change causes failures across 10 unrelated feature files | Group scenarios under the `Rule:` keyword; enforce the single-rule focus per scenario | `[A11]`, `[A19]`, `[A21]` |

## Explicit List of Pilot Success and Kill Criteria

Sources provide clear criteria for selecting, evaluating, and aborting a BDD
pilot project:

### 1. Pilot Selection Criteria

- **Size**: Small, self-contained capability. Steven Thomas recommends starting
  with "one scenario in one, preferably small, feature file" (`[A26]`). Gojko
  Adzic recommends a "small sample project" or spike to learn tools before
  applying to core systems (`[A10]`).
- **Domain**: Choose a domain area with genuine business rules and edge cases,
  not simple CRUD or pure glue code (Adzic `[A10]`, Keogh `[A25]`). Avoid the
  most critical legacy systems for tool learning (Adzic `[A10]`).

### 2. Pilot Success Criteria

- **Shared Understanding Demonstrated**: All team members (and stakeholders, if
  present) agree that scenarios clearly describe system capabilities without
  reading implementation code (North `[A01]`, Adzic `[A06]`, Binamungu `[A29]`).
- **Stable Specifications**: Scenarios pass or fail based on business rule
  changes, not technical refactorings or cosmetic edits. Zero shotgun surgery
  (Adzic `[A11]`, Rose `[A19]`).
- **Execution Speed**: Test suite executes quickly in CI. Does not introduce
  significant execution bottlenecks (Binamungu `[A31]`).
- **Clean Glue Layer**: Step definitions remain compact delegation shims to
  public domain APIs without duplicate regex or steps calling steps (Cucumber
  Docs `[A17]`).
- **BRIEF Compliance**: All formulated scenarios satisfy BRIEF principles: under
  6 lines, intention-revealing, concrete domain data (Rose & Nagy `[A19]`).

### 3. Pilot Kill / Abort Criteria

- **The "Nobody Reads Gherkin" Kill Rule (Aslak Hellesoy & Liz Keogh)**: If
  after the pilot, no external or non-programming stakeholders read the feature
  files, abort Gherkin adoption immediately. Revert to standard xUnit tests
  (`[A14]`, `[A23]`).
- **Maintenance Tax Exceeds Payback (Binamungu et al. & Adzic)**: If updating
  step definitions and feature files adds double maintenance for every code
  refactoring without catching new domain misunderstandings, abort (`[A11]`,
  `[A31]`).
- **Glue-Code Explosion (Cucumber Project)**: If step definitions become
  untraceable, feature-coupled, or require deep scaffolding to execute, abort
  tool-driven BDD and keep plain programmatic tests (`[A17]`).
- **Timebox Exceeded (Matt Wynne)**: If formulating a small capability takes
  excessive debate or the team cannot express clear rules after 25 minutes of
  example mapping, abort the feature slice (`[A27]`).

## Disagreements in the Literature

| Topic | Position A | Position B | Evaluation and Evidence |
|---|---|---|---|
| BDD for Developer-Only Teams | Chris Parsons claims BDD provides high discipline and prevents gold-plating for solo developers (`[A33]`). | Aslak Hellesoy (`[A14]`) and Liz Keogh (`[A23]`) advise against Gherkin if non-developers do not read it. Ham Vocke (`[A34]`) demonstrates code tests provide equal value without DSL cost. | Position B is backed by empirical data. Binamungu (`[A30]`, `[A31]`) shows DSL maintenance overhead is significant. Gherkin without non-technical readers introduces a translation layer with zero communication benefit. |
| Automation Timing | Dan North (`[A01]`) and BDD core state automation must be done iteratively during development. | Gojko Adzic (`[A06]`, `[A09]`) found 29% of teams get major quality gains from specifying with examples without automating them at all. | Measured survey data shows conversation and example formulation yield 70% of the benefit; automation provides regression protection but incurs high maintenance. |
| Gherkin vs xUnit Tests | Cucumber guides promote Gherkin as the preferred format for executable specifications (`[A08]`). | Fowler (`[A22]`), Farley (`[A25]`), and Vocke (`[A34]`) state Given-When-Then structure within standard xUnit test methods gives identical specification clarity without text parsing. | For teams without dedicated non-technical analysts, xUnit Given-When-Then tests avoid regex glue code and keep type safety. |

## Does Not Transfer to This Repository

The `traces` workspace is a single-crate Rust library and CLI maintained by a
solo developer or small technical team without external business analysts or
non-technical product owners. The following aspects do not transfer:

1. **Three Amigos Meetings and Workshops**: Ceremonies designed to align
   business analysts, product owners, and manual testers (Adzic `[A05]`, Wynne
   `[A26]`) do not apply. The developer already possesses full domain knowledge.
2. **Natural Language Translation for Non-Programmers**: The primary motivation
   for Gherkin in Cucumber literature (engaging business stakeholders who cannot
   read code, `[A13]`, `[A14]`) is absent.
3. **UI Automation Anti-Patterns**: Warnings regarding Selenium, browser
   latency, and CSS selectors (`[A12]`, `[A15]`) do not apply to a headless Rust
   CLI and library.
4. **Task Tracker as Specification Source of Truth**: Storing specifications in
   Jira or external systems (reported by 57% in Adzic's survey `[A06]`) does not
   apply. All specifications must live in version control alongside code.

What **does** transfer directly:

1. **Separation of What from How**: Writing scenarios that test user guarantees
   rather than internal Rust implementation details.
2. **Declarative Scenario Design (BRIEF)**: Keeping scenarios under 5 lines,
   using real domain data, and focusing each scenario on one business rule.
3. **Glue Layer Hygiene**: Keeping step definitions organized by domain,
   avoiding feature-coupled steps, and calling domain APIs directly.
4. **Layer Partitioning**: Preventing duplicate test coverage between unit tests
   and end-to-end BDD tests.
5. **Strict Pilot Kill Criteria**: Establishing clear abort triggers before
   expanding BDD across all 88 invariants.

## Implications for Ticket 14 (Roadmap Options)

The evidence provides clear options for sequencing adoption slices, pilot
selection, and abort triggers:

- **Option A1 (Pilot Sizing - Narrow Core Capability)**:
  - Select one small, high-value, stable domain capability with well-understood
    rules for the cold-start pilot (e.g. CLI argument parsing error reporting or
    basic trace file reading).
  - *Evidence*: Adzic recommends starting with a small sample project (`[A10]`);
    Steven Thomas recommends one scenario in one feature file (`[A26]`).
- **Option A2 (Pilot Domain - Avoid Complex Legacy Unification First)**:
  - Do not use the index freshness invalidation or cache synchronization as the
    first pilot. Those involve high technical state concurrency.
  - *Evidence*: Liz Keogh warns against using BDD on complex legacy internals
    without established baseline tests (`[A24]`, `[A25]`).
- **Option B1 (Sequence Slices by User Guarantee Independence)**:
  - Slice adoption by user-facing guarantee groups (e.g., CLI command outputs,
    output format invariant, error code mapping), rather than internal code
    modules.
  - *Evidence*: Dan North (`[A05]`) and Seb Rose (`[A19]`) insist scenarios
    represent user-observable capability boundaries.
- **Option B2 (Enforce the Single-Rule Focus in Feature Files)**:
  - Organize all Gherkin files using the `Rule:` keyword to group scenarios
    directly by system guarantee.
  - *Evidence*: Rose (`[A21]`) and Wynne (`[A26]`) show grouping by Rule
    isolates test failures and eliminates shotgun surgery.
- **Option C1 (Include Explicit Pilot Abort Trigger in Spec)**:
  - Formally adopt the Hellesoy/Binamungu kill criteria: if maintaining
    `rstest-bdd` glue steps slows refactoring without discovering functional
    defects, stop expanding BDD and retain existing integration tests.
  - *Evidence*: Hellesoy (`[A14]`), Binamungu (`[A31]`).
- **Option D1 ("Not Now" List Definition)**:
  - Place internal algorithmic components, performance benchmarks, and
    high-permutation parsing logic on the "Not Now / Never BDD" list. Keep those
    strictly in property tests and unit tests.
  - *Evidence*: Binamungu (`[A31]`), Cucumber Anti-Patterns (`[A15]`), Vocke
    (`[A34]`).

## Open Gaps

1. **Rust-Specific DSL Maintenance Costs**: Empirical studies on BDD maintenance
   (Binamungu et al.) focus on Java, .NET (SpecFlow), and Ruby (Cucumber).
   Maintenance characteristics of macro-generated Rust BDD steps (`rstest-bdd`)
   under strict Rust compiler borrow and lifetime semantics are not documented
   in academic literature.
2. **Nextest Integration Overhead**: Literature does not document execution time
   impact of compiling and executing Gherkin feature files inside a consolidated
   e2e binary (`tests/e2e.rs`) under `cargo-nextest`.
