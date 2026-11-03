# Research 13: Evidence for the migration and reuse policy

This note supports [ticket 13](../issues/13-migration-reuse-policy.md). It
collects external evidence on how to retire, move, strengthen and convert
tests, and how to prove that a migration loses no guarantee.

Method (2026-11-02):

- Six parallel reviews each searched the web and read primary sources. Topics:
  test retirement, migration patterns, Gherkin retrofit, Rust tooling, test
  value and smells, rollout governance.
- The session then opened more than 20 of the cited pages and papers again. It
  checked quotes and numbers against the text. It tested tool claims against
  this repository.
- Tags: `[READ]` means the session opened the source. `[AGENT-READ]` means a
  review agent opened it and the session did not re-check it. `[INFERENCE]`
  means reasoning, not a source.

## 1. Corrections to the raw reviews

The reviews made errors. The ticket must not cite these claims.

| Claim in a raw review | Result of the check |
|---|---|
| Software Engineering at Google, chapter 12, has a "test deletion policy" with three conditions. | False. The chapter has no such policy. It says only that deleting an obscure test "introduce[s] a hole in test coverage". `[READ]` |
| `cargo mutants --in-diff <diff>` proves that a deleted test lost nothing. | False. The cargo-mutants book says a diff that "only deletes or changes test code won't cause any mutants to run". `[READ]` |
| `cargo mutants --test <name>` shows which mutants one test kills. | No such option appears in the cargo-mutants pages read. The `mutarust` `--per-test` flag selects which tests run for a mutant. It does not report kills for each test. `[READ]` |
| Proptest ties a regression seed to the test function name. | False. The seed file path comes from the source file path. A moved test function keeps its seed. A moved or renamed source file loses it. `[READ]` |
| LLM-written oracles are weak, and 40 percent are discarded. | Not confirmed. Molinelli et al. report a mutation score of 43 percent for LLM oracles against 45 percent for human oracles. The 40 percent figure is not in the text read. `[READ]` |
| "Nan et al., ICSE 2025" proves that agents over-mock. | The paper is a student-competition abstract by Qin. It studies GPT-4o and mocks. Treat it as weak. `[SNIPPET]` |
| Ad-hoc defect tags are a recognized anti-pattern, and a closed six-value list fixes it. | Not sourced. The six values are a reviewer proposal. The Orthogonal Defect Classification paper is real, but the session did not re-open it. `[AGENT-READ]` |
| "15 to 25 tests per slice", "10 percent CI time", "5 days or 50 runs", "3 clean runs". | Reviewer heuristics. No source gives these numbers. |

## 2. Evidence by topic

### 2.1 Deleting tests

| Source | Finding |
|---|---|
| Shi et al., ISSTA 2018, "Evaluating test-suite reduction in real software evolution". `[READ]` | The study used 1478 failed builds from 32 projects. Failed-build detection loss reached "up to 52.2%". The authors write that traditional reduction metrics "are not good predictors" of that loss. |
| Heimdahl and George, ASE 2004. `[READ]` | Reduced suites that kept 100 percent coverage lost at least 7 percent of fault detection in every case. |
| Bhatta et al., MSR 2025, "Understanding Test Deletion in Java Applications". `[READ]` | Only 0.47 percent of commits delete tests. 91.4 percent of deleted tests are obsolete because the production code went. 7 percent are redundant passing tests. "The deletion of 20% of redundant tests reduces code coverage or mutation scores." |
| Rothermel et al., STVR 2002, and Chen and Lau. `[AGENT-READ]` | Coverage-preserving minimization can cut fault detection sharply. Minimization is a set-cover problem that sees coverage, not assertions. |
| Fowler, "TestCoverage". `[READ]` | You test too much when a small code change causes long test changes. Duplication in tests often explains this, not the number of tests. |
| Hammant on Fowler's site, "The Rise of Test Impact Analysis". `[READ]` | Impact analysis shows tests that cover the same production code. Straight duplicates may go. "Often they are not though." |
| Software Engineering at Google, chapter 12. `[READ]` | "The ideal test is unchanging." Obscure tests that engineers delete leave a hole in coverage. |

Reading for ticket 13:

- Coverage or mutation overlap does not prove that a test is redundant. The
  strongest field evidence says the opposite for coverage.
- Real deletion is rare and mostly follows removed code. In this repository the
  main source of obsolete tests is the arch-design work, not the BDD
  migration. `[INFERENCE]`
- One in five deletions of redundant tests lost coverage or mutation score,
  even with human review. A migration needs a check that a person cannot skip.

### 2.2 Migration patterns

| Source | Finding |
|---|---|
| Sato on Fowler's site, "Parallel Change". `[READ]` | Expand, migrate, contract. "If the contract phase is not executed you might end up in a worse state than you started." |
| Fowler, "Strangler Fig Application". `[AGENT-READ]` | A big replacement "go[es] down in flames most of the time". Replace in slices. |
| Smith, "Verify Branch by Abstraction". `[AGENT-READ]` | Run old and new code with the same input and fail on a difference. The method costs extra maintenance while both exist. |
| Understand Legacy Code on Feathers. `[READ]` | A characterization test records actual behavior. It gives a base before you refactor. |
| Keogh, "Using BDD with legacy systems". `[READ]` | On legacy code, write locking tests first: do something, capture the output, verify the same output. Behavior in legacy systems often "makes no sense". |
| Approval-testing pattern notes. `[AGENT-READ]` | A rubber-stamped approval makes a bug part of the baseline. |

Reading for ticket 13:

- The order inside one change is: the scenario lands and passes, both tests run,
  then the old test goes. This is expand, migrate, contract.
- An open dual-run state is a risk. A migration without a contract step ends in
  two suites. The ledger needs a row state that blocks closing a slice while
  any row stays in dual-run. `[INFERENCE]`
- A locking test freezes incidental behavior. Ticket 07's intent-evidence
  condition already blocks that for scenarios.

### 2.3 Gherkin retrofit

| Source | Finding |
|---|---|
| Cucumber blog, "Cucumber anti-patterns, part two". `[READ]` | Delete day-one scenarios: "Try and delete scenarios because quite often that basic functionality will be covered by others." |
| Rose, "Keep your scenarios BRIEF". `[READ]` | Keep most scenarios to five lines or fewer. |
| Nagy, "Clean up bad BDD scenarios". `[AGENT-READ]` | A long scenario in the form given-when-then-when-then splits into focused scenarios. |
| Adzic, "Focus on key examples". `[READ]` | "Overly complex examples, or too many examples, are often a sign that some important business concepts are not explicitly described." |
| Binamungu et al., SANER 2018. `[READ]` | Survey of 75 practitioners. 61 percent say duplication makes specifications hard to change. Nearly half (49 percent) say duplication slows execution. |
| ThoughtWorks Radar, "Test at the appropriate level". `[READ]` | BDD tools led teams to put most tests "where the cost to run the tests is the greatest". |
| Keogh, "Scenarios using custom DSLs". `[AGENT-READ]` | If nobody outside the team reads scenarios, a code DSL gives the same clarity with less upkeep. |

Reading for ticket 13:

- A retrofit produces scripts in disguise unless each test is re-formulated.
  A direct translation of a Rust test into steps is the named anti-pattern.
- Duplicate scenarios are a known cost. The repository already holds duplicates
  between layers. The ledger must point from each scenario to the Rust tests it
  overlaps, in both directions. Ticket 11 already requires unique titles.
- The 27 conversion candidates are a small number. The evidence says to keep
  conversion rare and to delete a scenario that adds only the obvious case.

### 2.4 Test value and weak tests

| Source | Finding |
|---|---|
| Khorikov, *Unit Testing Principles, Practices, and Patterns* (2020). `[AGENT-READ]` | Four pillars: protection against regressions, resistance to refactoring, fast feedback, maintainability. A test with no resistance to refactoring has near-zero value. |
| Beck, "Test Desiderata". `[AGENT-READ]` | Twelve properties that trade against each other. "No property should be given up without receiving a property of greater value in return." |
| Software Engineering at Google, chapter 12. `[READ]` | Test through the public API. Test state, not interactions. Prefer DAMP code over DRY code in tests. |
| Cooper, "TDD, where did it all go wrong". `[AGENT-READ]` | Do not write new tests when you refactor. The unit of a test is a behavior. |
| Spadini et al., ICSME 2018. `[AGENT-READ]` | Test smells link to higher defect density in the production classes they test. |
| Niedermayr et al. 2016; Vera-Perez et al. 2019. `[AGENT-READ]` | A method is pseudo-tested when its body can vanish and no test fails. Line coverage hides this. |
| Barr et al., TSE 2015, oracle survey. `[AGENT-READ]` | An assertion-free or `is_err()`-only test reduces the oracle to "did it crash". |
| Meszaros, *xUnit Test Patterns*. `[AGENT-READ]` | Fragile Test, Assertion Roulette, Conditional Test Logic, Erratic Test. |

Reading for ticket 13:

- A "strengthen" action needs a test of weakness: the assertion passes for the
  wrong reason. `is_err()` passes for any error. A substring match on miette text
  breaks ticket 07's channel rule.
- Table-driven loops carry conditional logic and failure messages that hide the
  failing row. Convert only key rows (ticket 08 already limits this).
- No source supports a project-local defect-class list as a quality tool. See
  section 4, row (g).

### 2.5 Rust tooling

| Source | Finding |
|---|---|
| nextest docs, machine-readable list. `[AGENT-READ]`; the session ran `cargo nextest list --all-features --message-format json`. `[READ]` | The output has `rust-suites.<binary-id>.testcases`. The session read the counts from it: `traces` 3802, `traces::bin/traces` 4, `traces::e2e` 64, `traces::integration` 41. |
| cargo-mutants book. `[READ]` | `outcomes.json`, `caught.txt` and `missed.txt` list each outcome. A diff-scoped run ignores test-only diffs. Nextest does not run doctests. |
| `mutarust` docs, `docs/refs/quality_gates_mutarust.md`. `[READ]` | `--per-test` runs only tests that cover the mutated line. A mutant with no mapping runs the full suite. The task has `--update-baseline` and `--fail-on-escaped`. |
| Proptest docs, `FileFailurePersistence`. `[READ]` | Seeds go to `proptest-regressions/<source path>.txt`. |
| matklad, "How to Test" and "Delete Cargo Integration Tests". `[AGENT-READ]` | Test features, not code. One integration binary avoids repeated linking. |
| `trycmd`, `insta`, `expect-test`. `[AGENT-READ]` | Snapshot update flags can bless a regression without review. |
| `cargo-public-api`, `cargo-semver-checks`. `[AGENT-READ]` | Dump or diff the public API. The repository already holds three frozen baselines. |

Reading for ticket 13:

- A test identity is `binary-id` plus the test path. A move changes the path.
  Without a ledger, a move reads as one deletion and one addition.
- A mutation proof for a deleted test compares escape sets. It does not
  attribute kills to one test. Record the escapes before the change. Delete the
  test. Run the owning module again with `--fail-on-escaped`. A new escape means
  a lost guarantee. Scope the run with `-m <module>` or `-f <file>`. A
  `--git-diff` run selects only changed production lines. `[INFERENCE]` for
  `mutarust`. The cargo-mutants book states the same for its own `--in-diff`.
- Doctests need a separate run. Nextest does not run them.

### 2.6 Rollout safety and governance

| Source | Finding |
|---|---|
| Luo et al., FSE 2014. `[READ]` | 78 percent of flaky tests are flaky when first written. Async wait is the top cause (45 percent of fix commits). |
| Micco, Google Testing Blog 2016. `[READ]` | About 84 percent of pass-to-fail transitions involve a flaky test. Almost 16 percent of tests show some flakiness. |
| Listfield, "Where do our flaky tests come from?". `[AGENT-READ]` | Flake rate grows with test size: 0.5 percent small, 1.6 percent medium, 14 percent large. |
| Ivankovic et al., FSE 2019. `[AGENT-READ]` | Google sets no global coverage target. It shows delta coverage in review. |
| Fowler, "TestCoverage". `[READ]` | A coverage target invites assertion-free tests. |
| Schipper, "Quality ratchets for AI code". `[AGENT-READ]` | A ratchet fixes current debt and blocks growth. The baseline moves only down. |
| Nygard 2011; MADR. `[AGENT-READ]` | Record a decision once so that nobody reopens it by accident. |
| Molinelli et al., ASE 2025. `[READ]` | 13,866 oracles from 135 Java projects, all newer than the model cutoff. LLM oracles reach 43 percent mutation score, human oracles 45 percent. Mutation analysis is the filter. |

Reading for ticket 13:

- Scenarios spawn the binary. They are larger tests. Larger tests flake more.
  Hermetic setup and no sleeps matter more for scenarios than for unit tests.
- A ratchet on the count of unmigrated tests is a count target. Ticket 12
  bans it. A ratchet on a gate baseline is allowed. The repository already has
  three baselines and a mutation baseline.
- One study gives agent-written oracles a mutation score near the human score.
  Mutation checks remain the filter. The evidence for agent-led migration of a
  Rust suite is thin.

## 3. Repository facts that shape the rules

The session checked these on 2026-11-02.

| Fact | Evidence | Effect |
|---|---|---|
| Counts: lib 3802, bin 4, e2e 64, integration 41. | `cargo nextest list --all-features --message-format json` | The figures in the first draft of ticket 13 were stale. |
| E2E runs on `ubuntu-latest`, `macos-latest` and `windows-latest`. Unit and integration run on `ubuntu-latest` only. | `.github/workflows/ci.yml` lines 84 to 140 | A move from e2e to a lower layer drops macOS and Windows runs. A conversion to a scenario keeps them. |
| CI has no mutation job and no coverage job. | `ci.yml` has no match for `mutants`, `mutarust` or `llvm-cov` | A mutation proof is a local step. Review must check that the PR records it. |
| Nextest has no retries. `slow-timeout` is 30 s with `terminate-after = 2`. | `.config/nextest.toml` | A flaky test fails the run. There is no hidden retry that masks flakes. |
| Two e2e tests carry `#[ignore]` and run only from a parent test. | `tests/e2e/env_policy.rs` lines 148 and 204 | A plain run skips them. An inventory diff needs `--run-ignored` or a rule for them. |
| The repository has no `proptest-regressions/` directory. | glob, no match | The seed-orphan risk is zero today. Check again when a proptest fails and persists a seed. |
| `mise run test:mutants` offers `-m`, `-f`, `--git-diff`, `--per-test`, `--update-baseline`, `--fail-on-escaped`. | `.mise/tasks/test/mutants` | Module scope is available. |
| `mutarust.yml` sets `exclude_dirs: src/cli`. The file `mutarust-baseline.json` is tracked. | `mutarust.yml`; `git ls-files` | Mutation never mutates the CLI layer. A mutation proof is blind there. `--fail-on-escaped` compares with the tracked file, not with a run before the change. |
| Nextest 0.9.150 offers `--stress-count` and `--run-ignored all`. | `cargo nextest run --help` | A repeat run can replace a calendar dual-run window. An inventory can include ignored tests. |

## 4. Input to each decision of ticket 13

These rows are inputs. Ticket 13 is a grilling ticket. The user decides. Where a
row differs from the Answer of ticket 13, the Answer wins. The Answer replaced
the dual-run window with a stress run. It added a kill demonstration, because
the mutation proof is blind in `src/cli`.

| Decision | What the evidence supports | Open choice for the user |
|---|---|---|
| (a) Convert | Convert only a test that passes ticket 07's six conditions. Re-formulate it. Do not translate it. Keep it to about five lines and one principal `When`. Delete a scenario that states only the obvious case. | How to treat a test whose only purpose is to lock incidental behavior. |
| (b) Keep | Keep unit tests of internals, tests that a proptest owns, and the exception classes. Resistance to refactoring is the test of value. | Whether to keep example tests that a proptest law already covers. |
| (c) Die | Overlap proves nothing alone. Use four checks: a named survivor, a module-scoped baseline comparison, a survivor that runs on the same operating systems, and human review. Treat "obsolete because the code went" as a separate reason. | Whether a deletion of a duplicate needs the mutation run, or only a named survivor. A cost choice. |
| (d) Preserve | A guarantee-keyed ledger, not a count. Add an inventory diff by `binary-id` plus test path, so that a vanished test must appear in the ledger. Treat `#[ignore]` children as a special case. | Where the ledger lives, and whether a script checks it or review does. |
| (e) Scope | Slice by capability. Close a slice only when no row stays in dual-run. A ratchet on counts breaks ticket 12. | Whether to migrate all 27 candidates or only those an adoption slice touches. |
| (f) Move, strengthen | A move from e2e to unit loses two operating systems in CI. A weak assertion needs a typed error check or a code, not text. | Whether to accept the platform loss or keep one e2e smoke test. Who owns the 15 moves. |
| (g) Headers and pins | No source supports a free-form, undocumented defect tag. Options: drop the ledger column and use the research/10 row ID, or define a short closed list in the docs. | Which option. Whether to relabel or retire the mtime canary. |

## 5. Weak or missing evidence

- The deletion studies that the session read use Java or C. None covers a Rust
  suite.
- No study measures the loss when a test moves into Gherkin.
- No source gives a safe length for a dual-run. Any number is a heuristic.
- No Rust tool gives per-test mutant attribution. A new tool would break the
  "no custom infrastructure" preference in the map.
- Several quotes in sections 2.2 to 2.6 rest on `[AGENT-READ]`. Re-open them
  before a decision cites them as fact.
- The session recounted the suites (section 3). The earlier figure of 2,368
  inline unit tests is wrong. The audit tables stay unrecounted. They are static
  agent readings, and spot checks found two defects.
