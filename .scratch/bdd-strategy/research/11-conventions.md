# Research 11: Convention review for feature and step organization

This file supports
[11: Feature and step organization](../issues/11-feature-step-organization.md).
It records what Cucumber and Rust BDD practice say about layout and
organization. It also records the corrections to the first round of
recommendations in the grilling session.

## 1. Feature file directory

| Source | Finding |
|---|---|
| cucumber-rs `README.md:68`, `tests/junit.rs:54` | Examples run `tests/features/<name>`. |
| rstest-bdd `README.md:88,118,313` | Examples use `tests/features/<name>.feature`. |
| rstest-bdd own test suite | Features live in `tests/features/` with subdirectories such as `bulk_migration/`, `filtered/` and `skip_allowance/`. |
| [research/cucumber-rs.md](./cucumber-rs.md) section 5, [research/rstest-bdd.md](./rstest-bdd.md) section 4 | Both name `tests/features/` as the convention and say "one feature file per behavioral area". |
| [Cucumber: Step organization](https://cucumber.io/docs/gherkin/step-organization) | Cucumber flattens the `features/` tree when it runs. The directory structure is for people. |
| Cargo | Cargo builds a test target only from `tests/*.rs` and `tests/*/main.rs`. A data directory such as `tests/features/` creates no target. `tests/baselines/` already sets this precedent in the repo. |

Result: the convention is `tests/features/`. The path
`tests/e2e/features/` came from the spike of ticket 09. It was never a
convention.

Effect on the gates: `check:layers` scans `tests/e2e.rs` and `tests/e2e/`
today. Features in `tests/features/` need an explicit scan entry. Ticket 08
already requires that entry.

## 2. Step grouping

| Source | Finding |
|---|---|
| [Cucumber: Step organization](https://cucumber.io/docs/gherkin/step-organization) | Create one step file for each domain concept. A good rule is one file for each major domain object. |
| [Cucumber: Anti-patterns](https://cucumber.io/docs/guides/anti-patterns) | "Feature-coupled step definitions" is an anti-pattern: steps that you cannot reuse across features. Organize steps by domain concept. Use domain names, not feature names, for step files. |
| Same page | Do not use conjunction steps. Keep steps atomic. Compose with helper functions in the programming language. |
| [Cucumber: Step organization](https://cucumber.io/docs/gherkin/step-organization) | Do not write a step definition for a step that no scenario uses. A helper method can live in the same file as its step. |

Result: a step file for each feature file is the named anti-pattern. The first
round proposed it in questions 3 and 11. Both are withdrawn.

## 3. Scenario style

| Source | Finding |
|---|---|
| [Cucumber: Better Gherkin](https://cucumber.io/docs/bdd/better-gherkin) | Describe what, not how. Test: "Will this wording need to change if the implementation does?" Prefer a declarative style. |
| [Cucumber hexdocs guide](https://hexdocs.pm/cucumber/best_practices.html) (community, not official) | One behavior for each scenario. Keep it to 3 to 7 steps. Use `Background` only for truly common setup. Tags help filtering. |

Result: the density limit is 3 to 7 steps. The first round proposed 6 with
no source.

## 4. Tool facts from the source of `rstest-bdd` 0.6.0

The scout `BddFacts` found these facts. Its evidence is `file:line` in the
cached crate sources.

- A bare `{name}` keeps the quote characters. Only `{name:string}` strips them.
- `scenarios!` does not generate tests for `Rule` blocks.
- Subdirectory names do not enter test names. Two files with the same stem and
  scenario title get a numeric suffix.
- `skip!()` passes by default. `@ignore` has no effect.
- `cargo bdd` needs the `diagnostics` feature. Ticket 06 turned it off.
- A step parameter that differs from the fixture name fails with
  `MissingFixture`. Use `#[from(name)]` or the same name.
- A step in any compiled module registers. No `pub` or `use` is needed.

## 5. Recommendations that this review changes

| Round 1 or 2 item | Change |
|---|---|
| Q1: `tests/e2e/features/`, flat | Use `tests/features/`. Keep it flat. |
| Q3: one step file for each feature file | Withdrawn. Group steps by domain concept, with domain names. |
| Q11: detect dead steps by the 1:1 file structure | Withdrawn. The structure is the anti-pattern. |
| Q5: at most 6 steps | Superseded by section 6.4: prefer 3 to 5 steps. |
| Q10: ban `skip!()` | Stays, but research/rust-bdd-crate-comparison step 5 recommended `@allow_skipped` with `RSTEST_BDD_FAIL_ON_SKIPPED=1`. The user decides. |

## 6. Second review: prior art, Gherkin authorities and probes

Three reviews tried to break the first recommendations. Their reports are in
the session transcript. The probe crate is `/tmp/bdd-probe-11` (macOS only).

### 6.1 Prior art (6 Rust repositories, 4 other ecosystems)

- `tests/features/` is the Rust convention. cucumber-rs and rstest-bdd nest
  features by topic. A flat layout is also common in small projects.
- No Rust repository found spawns a product binary from Gherkin. The rstest-bdd
  `todo-cli` example keeps binary tests apart from its Gherkin tests, which call
  the library. The pilot of ticket 09 therefore has no Rust precedent. The
  ticket 09 run is the evidence.
- Aruba (Ruby), SUSEConnect and WP-CLI spawn a binary from Gherkin. They write
  `When I run ...`, then assert the exit status, stdout and stderr.
- cucumber-rs and Cucumber group step files by domain concept. copybook-rs does
  the same with many modules shared across features.
- Aruba creates the sandbox in a hook, not in a step.

### 6.2 Gherkin authorities

| Item | What the sources support |
|---|---|
| Steps for each scenario | The Gherkin reference recommends 3 to 5. Automation Panda says fewer than 10. No source sets 7 or "exactly one `When`". WP-CLI scenarios run several commands. |
| Scenario Outline rows | The reference shows 2 rows. No source sets a minimum of 3 or a maximum of 10. Ticket 08 sets the project limit of 5 to 10 key examples next to a proptest. |
| `Background` size | The reference says to review a `Background` of more than 4 lines. No source sets 2. |
| `Rule` | The reference uses `Rule` to group the examples of one business rule. |
| Tags | Cucumber sets no limit and uses `@smoke` in its examples. |
| `Feature` description | It can hold a brief explanation and the business rules. No source requires an ADR citation. |
| Quoting | Automation Panda recommends double quotes around parameter values. |
| CLI assertions | No source calls stdout or stderr assertions brittle. Exact incidental prose changes more often than a code or a meaningful substring. |

### 6.3 Probes (E1 to E12)

- E1: child `mod x;` of a `#[path = "e2e/bdd/mod.rs"]` module resolves to
  `e2e/bdd/x.rs`. Under `#[path = "e2e/bdd.rs"]` it resolves to `e2e/x.rs`.
  The second form picks up the wrong sibling file.
- E2: one `#[expect]` on `mod bdd` covers nested step modules. It fails only
  when no module under it has a step.
- E3: `{name:string}` and `{count:u32}` both fail clippy. A bare `"{name}"` with
  quotes in the pattern passes and captures spaced text. An escaped inner quote
  keeps its backslash.
- E4: a duplicate step compiles. The first lookup panics and poisons the
  registry for the other scenarios. A literal pattern beats an overlapping
  placeholder pattern.
- E5: a mismatched parameter name fails at run time. `#[from(name)]` repairs
  it. One `scenarios!` call binds two fixtures. A `Then` step reads the value
  that a `When` step wrote through `&mut`.
- E6: scenarios inside `Rule:` generate no test and no diagnostic.
- E7: directory names are not in test names. Two scenarios with the same stem
  and title get a numeric suffix.
- E8: a new feature file needs `touch tests/e2e/bdd/mod.rs`. An edit to an
  existing feature file rebuilds with no `touch`.
- E9: `skip!()` passes by default. `RSTEST_BDD_FAIL_ON_SKIPPED=1` fails it.
  `@allow_skipped` exempts it. `@ignore` does nothing.
- E10: the `check:layers` patterns match `traces {x}`, `traces as` and
  `process {x}` in feature text and in step attributes. They do not match
  `the process exits with code {code}` or `stderr contains "{text}"`.
- E11: an Outline with 3 rows gives `case_1` to `case_3`. A `Background` runs
  once for each case. A step function named `table` clashes with a macro local.
- E12: an undefined step compiles and fails at run time.

### 6.4 Changes to the first recommendations

| Item | Change |
|---|---|
| Steps for each scenario | Prefer 3 to 5. Review more than 5. The single `When` is a guideline, not a rule. |
| Outline rows | Drop the 3-row minimum and the 10-row maximum. Keep ticket 08's limit. |
| `Background` | Reconsider it above 4 lines. |
| Tags | `@known-limitation` is inert only while no `tags =` filter uses it. |
| `Feature` description | It states the guarantee and links the intent source. Ticket 07, rule 3 requires the source, so this is a project rule. |
| Bans (`Rule`, `skip!`, `@ignore`) | Each ban needs a gate. The tool never warns. |
| Scenario titles | A title must be unique in the whole suite. A duplicate gets an unstable numeric suffix. |
| Sandbox step | Drop `Given an isolated sandbox`. The fixture builds the sandbox. |
| Step names | Do not name a step function `table` or `docstring`. |
| Quoted values | A value never holds a double quote. Use a docstring for such values. |
