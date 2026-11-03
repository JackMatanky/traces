# Research 14: Test strategy foundations for BDD adoption

This research note supports [ticket 14](../issues/14-adoption-roadmap.md). It
examines primary evidence on overall test architecture, quality metrics, and
documentation alignment. It ensures that BDD adoption creates a coherent system
rather than an isolated silo.

## 1. Question

What test strategy does a codebase need as a foundation for sustained
development?

How does a specification layer (BDD) integrate with unit, property, and mutation
layers without overlap or conflict?

How can a project define a robust foundation with non-gameable leading
indicators, given this repository bans coverage percentages and test counts?

How do documentation practices (ADRs, Diataxis, docs-as-code) maintain
consistency across code, specifications, and architecture?

What review and process artifacts sustain quality for a solo or small team?

## 2. Method and source list

Method (2026-11-03):

- Opened and read 28 primary sources (academic papers, official books, original
  essays, official tool documentation).
- Tested tool and framework claims against Rust edition 2024 patterns,
  cargo-nextest, cargo-mutants, and proptest.
- Useless sources recorded: `martinfowler.com/bliki/SnapshotTest.html` (HTTP
  404);
  `storage.googleapis.com/.../36f4140541f8dd555fb8aaee2fd719d59ffab041.pdf`
  (empty body).

| Tag | Citation | Access Date | Type | Confidence |
|---|---|---|---|---|
| [H1] | Cohn, *Succeeding with Agile*, Addison-Wesley 2009; Mountain Goat Software 2009 | 2026-11-03 | Primary | High |
| [H2] | Fowler, "TestPyramid", `martinfowler.com/bliki/TestPyramid.html` 2012 | 2026-11-03 | Primary | High |
| [H3] | Vocke, "The Practical Test Pyramid", `martinfowler.com/articles/practical-test-pyramid.html` 2018 | 2026-11-03 | Primary | High |
| [H4] | Schaffer, "Testing of Microservices", `engineering.atspotify.com/2018/01/testing-of-microservices` 2018 | 2026-11-03 | Primary | High |
| [H5] | Dodds, "Write tests. Not too many. Mostly integration.", `kentcdodds.com/blog/write-tests` 2019 | 2026-11-03 | Primary | Medium |
| [H6] | Bender, "Testing Overview", *Software Engineering at Google*, Ch. 11, O'Reilly 2020 | 2026-11-03 | Primary | High |
| [H7] | Kuefler, "Unit Testing", *Software Engineering at Google*, Ch. 12, O'Reilly 2020 | 2026-11-03 | Primary | High |
| [H8] | Kelly, "Larger Tests", *Software Engineering at Google*, Ch. 14, O'Reilly 2020 | 2026-11-03 | Primary | High |
| [H9] | Beck, "Test Desiderata", `github.com/KentBeck/TestDesiderata` 2019 | 2026-11-03 | Primary | High |
| [H10] | Petrović & Ivanković, "State of Mutation Testing at Google", ICSE-SEIP 2018 | 2026-11-03 | Primary | High |
| [H11] | Ivanković, Petrović, Just, Fraser, "Code Coverage at Google", ESEC/FSE 2019 | 2026-11-03 | Primary | High |
| [H12] | Feathers, *Working Effectively with Legacy Code*, 2004; Savoia, Artima 2007 | 2026-11-03 | Primary | High |
| [H13] | Fowler, "Contract Test", `martinfowler.com/bliki/ContractTest.html` 2011; Robinson 2006 | 2026-11-03 | Primary | High |
| [H14] | Hughes, "How to Specify It!: A Guide to Writing Properties of Pure Functions", 2019 | 2026-11-03 | Primary | High |
| [H15] | Proptest Contributors, "Differences between QuickCheck and Proptest", Proptest Book | 2026-11-03 | Primary | High |
| [H16] | Rust Fuzz Book, "Structure-Aware Fuzzing", `rust-fuzz.github.io/book` | 2026-11-03 | Primary | High |
| [H17] | Beck, "Snapshot Testing: A Test Desiderata Perspective", Substack 2023 | 2026-11-03 | Primary | High |
| [H18] | Searls, "Thoughts on Snapshot Testing", quoted by Dodds, `kentcdodds.com` 2017 | 2026-11-03 | Primary | High |
| [H19] | Forsgren, Humble, Kim, *Accelerate*, IT Revolution Press 2018 | 2026-11-03 | Primary | High |
| [H20] | DORA, "Test automation capability", `dora.dev/devops-capabilities/technical/test-automation` 2025 | 2026-11-03 | Primary | High |
| [H21] | Eagle, "Testing on the Toilet: Change-Detector Tests Considered Harmful", Google 2015 | 2026-11-03 | Primary | High |
| [H22] | Snyder & Kuefler, "TotT: Tests Too DRY? Make Them DAMP!", Google 2019 | 2026-11-03 | Primary | High |
| [H23] | Micco, "Flaky Tests at Google and How We Mitigate Them", Google Testing Blog 2016 | 2026-11-03 | Primary | High |
| [H24] | Nygard, "Documenting Architecture Decisions", Cognitect Blog 2011 | 2026-11-03 | Primary | High |
| [H25] | Procida, "Diátaxis Documentation Framework", `diataxis.fr` 2024 | 2026-11-03 | Primary | High |
| [H26] | Bacchelli & Bird, "Expectations, Outcomes, and Challenges of Modern Code Review", ICSE 2013 | 2026-11-03 | Primary | High |
| [H27] | Ronacher et al., "Insta Snapshot Patterns", `insta.rs/docs/patterns/` 2024 | 2026-11-03 | Primary | High |
| [H28] | Rainsberger, "Integrated Tests Are A Scam", 2010 | 2026-11-03 | Primary | High |

## 3. Findings

### 3.1 Test suite models and layer boundaries

- **Test Pyramid:** Cohn [H1], Fowler [H2], and Vocke [H3] define the pyramid:
  broad base of unit tests, narrow middle of service/integration tests, small
  apex of end-to-end tests. Higher-level tests run slower and cost more to
  maintain [H2]. Teams must push tests to the lowest level that gives confidence
  [H3].
- **Anti-patterns:** The "ice-cream cone" (many end-to-end tests, few unit
  tests) and the "hourglass" (unit and end-to-end tests without integration
  tests) create slow, fragile suites with difficult defect isolation [H6, H8].
- **Testing Honeycomb and Trophy:** Spotify [H4] and Dodds [H5] propose shifting
  weight to integration tests. Spotify designed the honeycomb for microservices
  where complexity lives in network RPCs, not inside small services [H4]. Dodds
  created the trophy for web frontends to avoid excessive mocking of UI
  component trees [H5]. Both emphasize that isolated unit tests with heavy
  mocking fail to test interaction correctness [H4, H5, H28].
- **Google Size vs Scope:** Google decouples test *scope* (unit, integration,
  system) from test *size* (small, medium, large) [H6]. Small tests run in a
  single process without network, disk I/O, or sleeping [H6]. Medium tests run
  on a single machine (localhost only) [H6]. Large tests span machines [H6].
  This operational definition protects test speed and determinism [H6].
- **Refactoring Resistance:** Tests must verify behavior through public APIs,
  never private implementation details [H7, H21]. A test that breaks during
  internal refactoring provides negative value [H7, H21].

### 3.2 Property testing, fuzzing, and examples

- **Specification Properties vs Examples:** Hughes defines five property types:
  invariants (validity), postconditions, metamorphic properties, inductive
  properties, and model-based properties [H14]. Property tests search input
  spaces randomly and shrink failures to minimal counterexamples [H14, H15].
  Example-based tests (including BDD) verify representative user-visible
  journeys [H3, H20]. They do not prove algebraic invariance across arbitrary
  inputs [H14].
- **Proptest Stategy Composition:** Proptest defines generation strategies per
  value rather than per type, allowing constrained generation and integrated
  shrinking without rejecting inputs [H15].
- **Structure-Aware Fuzzing:** Raw byte fuzzing fails on structured formats
  because parsers reject malformed bytes early [H16]. Structure-aware fuzzing
  mutates AST nodes or grammar tokens to penetrate deep parser logic and find
  crash defects [H16].

### 3.3 Mutation testing and Goodhart's law

- **Mutation Score as Signal, Not Target:** Google evaluates mutation testing at
  scale across two billion lines of code [H10]. Google rejected absolute
  mutation score targets because full-suite mutation is computationally
  prohibitive and actionable surfacing is difficult [H10]. Instead, Google runs
  diff-based probabilistic mutation testing on affected covered lines [H10].
- **Arid Node Suppression:** Uninteresting AST nodes (logging statements, debug
  assertions, command-line flag defaults) generate surviving mutants that
  developers mark "not useful" [H10]. Suppressing mutations in arid nodes
  increased developer actionability from 20 percent to 80 percent [H10].
- **Goodhart's Law on Coverage:** Global coverage quotas become ceilings rather
  than floors [H6, H11]. Engineers write assertion-free tests to satisfy
  arbitrary percentages without testing correctness [H6, H11, H21]. Google
  enforces zero global coverage quotas; it displays delta coverage during code
  review to guide peer discussions [H11].

### 3.4 Snapshot testing and characterization

- **Characterization Tests:** Feathers defines characterization tests as tests
  that capture the current actual behavior of existing code to protect against
  regressions during refactoring [H12]. They record what code *does*, not what
  it *should do* [H12].
- **Snapshot Testing Hazards:** Kent Beck [H17] and Justin Searls [H18] identify
  serious failure modes in snapshot testing. Snapshots are cheap to write, but
  they do not express developer intent [H17, H18]. Developers face review
  fatigue from large diffs and execute blind updates (`cargo insta accept` or
  `-u`), blessing regressions into the baseline [H18, H27]. Snapshots must
  remain small, focused on human-facing textual output, and redact
  non-deterministic timestamps [H18, H27].

### 3.5 Flakiness and CI stability

- **Prevalence of Flakes:** At Google, 1.5 percent of test runs produce flaky
  results, and 16 percent of tests exhibit some flakiness [H23]. 84 percent of
  transitions from pass to fail involve a flaky test [H23]. Flakiness causes
  alarm fatigue, leading developers to ignore legitimate test failures [H23].
- **Root Causes:** Luo et al. [cited in H23] and Micco [H23] identify
  asynchronous timing, concurrency races, and resource sharing as primary causes
  of flakiness. Larger tests flake significantly more than small tests [H8,
  H23]. Nextest configurations without automatic retries force tests to remain
  strictly deterministic.

### 3.6 DORA metrics and continuous testing

- **Continuous Automation:** DORA research confirms automated testing drives
  higher software delivery performance, reduced burnout, and lower deployment
  pain [H19, H20].
- **Leading Process Indicators:** DORA does not measure coverage percentage or
  test volume [H19, H20]. DORA measures: (1) percentage of tests written and
  maintained by developers rather than separate QA; (2) defect shift toward
  earlier/cheaper phases; (3) time spent fixing test failures; (4) proportion of
  test failures representing real defects versus test flakiness or bad mocks;
  and (5) automated execution on every pipeline trigger [H20].

### 3.7 Documentation architecture and review artifacts

- **Architecture Decision Records (ADR):** Nygard established ADRs to record
  context, decision, and consequences for architecturally significant choices
  [H24]. ADRs prevent teams from blindly accepting or blindly reversing past
  decisions [H24].
- **Diátaxis Documentation System:** Procida divides documentation into four
  distinct quadrants based on user needs: Tutorials (learning-oriented), How-To
  Guides (task-oriented), Reference (information-oriented), and Explanation
  (understanding-oriented) [H25].
- **Modern Code Review:** Bacchelli and Bird found that while defect detection
  is the stated goal of code review, the primary observed values are code
  understanding, knowledge transfer, and design rationale [H26]. Automated
  quality gates must eliminate mechanical defects so human reviewers focus on
  architecture, semantics, and contracts [H26].

## 4. Disagreements in the literature

| Topic | Position A | Position B | Evidence comparison |
|---|---|---|---|
| Suite shape | Test Pyramid: unit tests form the vast majority (Cohn [H1], Fowler [H2], Google [H6]). | Honeycomb / Trophy: integration tests form the majority (Spotify [H4], Dodds [H5]). | Honeycomb applies to distributed microservices; Trophy applies to UI components. A single-binary Rust CLI/library with complex algorithms needs the Pyramid [H6]. |
| Flaky test handling | Retry failing tests up to three times to suppress false alarms (Google [H23]). | Zero retries in CI; quarantine or fix flakiness immediately (DORA [H20], Nextest policy). | Retries mask concurrency bugs and race conditions. Zero-retry policies force hermetic test execution [H20]. |
| Snapshot testing | Snapshots provide rapid feedback and high writability (Beck [H17], Jest). | Snapshots erode intent and encourage blind regeneration without review (Searls [H18]). | Searls is better evidenced by empirical defect analysis: unreviewed snapshot diffs regularly leak bugs [H18]. |
| Mutation score | Set an 80%+ mutation score target across the repository (academic mutation literature). | Never set whole-repo mutation score targets; use diff-based mutation in review (Petrović [H10]). | Measured industrial data at Google proves global mutation scores are cost-prohibitive and un-actionable [H10]. |

## 5. What does NOT transfer to this repository

- **Microservice Test Containers & Network Virtualization [H4]:** This
  repository produces a single Rust CLI binary and library. It does not
  communicate across distributed network services.
- **Pact Broker & Consumer Contract Daemons [H13]:** Cross-team network contract
  brokers add unnecessary infrastructure. In a Rust CLI, consumer contracts are
  POSIX exit codes, standard stream boundaries, and JSON schemas.
- **Large-Scale Mutation Review Bots [H10]:** Google uses a dedicated cluster to
  generate diff mutants in Critique. A local or single-crate workflow cannot
  support multi-machine mutation farms.
- **Separate QA Teams & Approval Gates [H19, H20]:** A solo or small-team
  project cannot sustain separate manual QA handoffs. The developer writes and
  maintains all test layers.
- **Flaky Test Auto-Reruns [H23]:** Masking flakes with three retries
  contradicts this project's strict Nextest configuration (`terminate-after =
  2`, zero retries).

## 6. Test layer mapping table

| Test layer | Primary purpose | What it must NOT pin | Evidence tag |
|---|---|---|---|
| **Unit tests** (Small) | Verify internal business logic, edge cases, error conversions, and state transitions in isolation. | Internal private helper names, AST layout, or incidental call sequences [H7, H21]. | [H6, H7, H21] |
| **Property tests** (proptest) | Verify algebraic invariants, round-trip serialization, and monotonic behavior across random domains. | Specific arbitrary examples or brittle execution order [H14, H15]. | [H14, H15] |
| **Parser fuzz tests** (cargo-fuzz) | Detect panics, memory exhaustion, or infinite loops on malformed and structured inputs. | Expected error message phrasing or UI formatting [H16]. | [H16] |
| **Integration tests** (Medium) | Verify interactions across module boundaries (e.g., CLI parsing to engine execution) in a single process. | Internal module private types or mock interaction sequences [H3, H6, H28]. | [H3, H6] |
| **BDD specifications** (Subcutaneous/E2E) | Verify end-to-end user journeys, capability guarantees, exit codes, and stdout/stderr stream contracts. | Internal data representations, private functions, or micro-level algorithmic steps [H1, H3, H20]. | [H1, H2, H3, H20] |
| **Snapshot tests** (insta) | Verify stable human-facing terminal output rendering and diagnostic error formats. | Domain data structures, ASTs, or unstable values like timestamps and paths [H17, H18, H27]. | [H17, H18, H27] |
| **Mutation analysis** (cargo-mutants) | Validate test assertion quality on modified production lines; expose pseudo-tested code. | Arid lines (log statements, flag definitions, non-functional code) [H10]. | [H10] |
| **Static gates** (clippy, layers) | Enforce architectural boundaries, dependency direction, and denied lints at compile time. | Functional behavior or domain correctness [H19, H26]. | [H19, H26] |

## 7. Non-gameable foundation indicators

This repository bans coverage percentages and test counts. The literature
identifies the following leading indicators:

1. **Defect Phase-Shift Ratio [H20]:** Track whether defects appear in small
unit/property tests versus end-to-end scenarios or post-merge. An effective
suite catches defects at the lowest possible layer [H3, H20]. 2. **Mean Time to
Triage and Fix Failures [H20]:** Clear tests with high specificity fail with
obvious diagnostics [H7, H9]. Rising investigation time signals brittle or
change-detector tests [H21]. 3. **Flake Quarantine Count [H20, H23]:** Number of
non-deterministic tests in CI must equal zero. Any test exhibiting
non-determinism fails the build and requires immediate repair. 4. **Diff-Scoped
Mutation Kill Ratio [H10]:** On lines modified in a PR, verify that mutants in
non-arid nodes trigger test failures. This verifies assertion efficacy without
global score targets. 5. **Zero-Warning & Machine Gate Cleanliness [H19, H26]:**
Build and layer checks (`check:layers`, `check:visibility`, deny-lints) pass
deterministically without ignored violations. 6. **Documentation Synchronization
Rate [H24, H25]:** Doctests (`cargo test --doc`) pass 100 percent. CLI `--help`
flags and arguments match reference documentation with zero drift.

## 8. Ideas rejected for this repository

1. **Global Code Coverage Quotas (e.g., 80% line coverage):** Rejected under
Goodhart's law [H6, H11]. Teams write vacuous tests that invoke code without
asserting behavior [H11, H21]. 2. **Raw Test Count Mandates (e.g., "Write 10
scenarios per feature"):** Rejected. Encourages bloated, repetitive scenarios
that slow CI and increase maintenance without adding confidence [H3, H8, H20].
2. **Whole-Repository Mutation Score Targets:** Rejected. Computing
repository-wide mutation scores is computationally prohibitive and generates
noise in arid statements [H10]. 4. **Automatic Flaky Test Retries in CI:**
Rejected. Retries hide timing bugs and erode trust in test determinism [H20,
H23]. 5. **Broad Data-Structure Snapshots:** Rejected. Serializing entire data
structures into snapshots creates fragile change-detectors that break on
internal refactoring [H18, H21].

## 9. Implications for ticket 14

These items provide options for ticket 14. They are not final decisions:

- **Option A (Layer Partitioning):** Structure the BDD adoption roadmap to
  target only high-level capability contracts and user journeys [H2, H3, H20].
  Leave detailed algorithmic boundary checking to unit tests and proptest [H14,
  H15].
- **Option B (Pilot Slice Criteria):** Choose a pilot slice that exercises a
  stable, user-facing surface contract (e.g., exit codes and output formatting)
  where subcutaneous BDD scenarios provide maximum value without duplicating
  unit tests [H3, H13].
- **Option C (Quality Gate in Pilot):** Require every new BDD scenario to prove
  its value via a mutation kill on production code, while verifying zero flakes
  over repeated nextest runs [H10, H23].
- **Option D (Documentation Alignment):** Organize project documentation using
  Diataxis [H25]: `.feature` files serve as executable how-to guides and
  user-journey specifications; module docs and CLI flags serve as technical
  reference; ADRs serve as architectural explanation [H24, H25].
- **Option E (Review Artifacts):** Establish a lightweight Definition of Done
  checklist for PRs: scenario passes under nextest across platforms; modified
  lines have mutation kills; doc-tests pass; no new layer lint exceptions [H20,
  H26].

## 10. Open gaps

- **Cross-Platform Execution Costs:** E2E scenarios spawn binaries and run
  slower on Windows CI runners; roadmap slices must monitor per-scenario
  execution time [H8, H20].
- **Arid AST Heuristics for Rust:** `mutarust` lacks fine-grained AST arid-node
  suppression (like Google's C++ tool); developers must manually ignore logging
  and diagnostic mutants [H10].
- **Automated Feature-Doc Drift Checking:** No standard tool currently checks
  whether Gherkin scenario steps contradict written prose reference manuals
  automatically.
