# 11: Feature and step organization

Type: grilling
Status: resolved
Blocked by: 06, 07, 08, 09

## Question

How do you organize feature files, scenarios, step definitions and fixtures so
they stay readable and maintainable as the system evolves, with no unnecessary
abstraction?

## Settled inputs (do not decide again)

Earlier tickets fixed these facts. Build on them.

- **Framework (ticket 06).** Use `rstest-bdd =0.6.0` and `rstest-bdd-macros
  =0.6.0` with `default-features = false`. State is plain `rstest` fixtures.
  There is no World.
- **Placement (tickets 08 and 09).** Scenarios run in the `e2e` binary only.
  The prototype wired them like this:
  - `tests/e2e.rs` declares `#[path = "e2e/bdd.rs"] mod bdd;` with one
    `#[expect(clippy::disallowed_methods)]`.
  - The prototype scenario is in [research/09](../research/09-prototype.md),
    section 3.

  The prototype paths `tests/e2e/bdd.rs` and `tests/e2e/features/` were spike
  choices. They are not conventions. Decision (a) picks the final paths. The
  convention review ([research/11](../research/11-conventions.md)) names
  `tests/features/`.
- **E2E limits (tickets 08 and 09).** The `e2e` binary builds with no
  `test-utils`. It cannot name `traces`. Steps spawn the binary only through
  `Sandbox` in `tests/e2e/support.rs`. A facade-driver binary is deferred.
- **Observable channels (ticket 07, rule 2).** A `Then` step asserts one of
  these: the stdout payload, a stderr diagnostic code, the exit status, or files
  under the project root. It never asserts miette-wrapped text, the `index.redb`
  contents or the store entries.
- **Hermetic runs (ticket 07, rule 5).** Each spawn sets `TZ`.
- **Build (ticket 09).** There is no `build.rs`. After you add a `.feature`
  file, run `touch tests/e2e/bdd.rs`.
- **Scenario scope (tickets 07, 08 and 10).** A scenario passes ticket 07's six
  conditions and ticket 08's classification rule. Ticket 10's invariance list
  sets the order of the first slices. A scenario never pins one of the three
  known defects.
- **Anti-goals (ticket 08, decision e).** The repo has no mock framework, no PTY
  harness, no `harness = false` runner and no combinatorial Scenario Outline
  suites.
- **Exception classes (ticket 07).** These classes never become plain scenarios:
  real-TTY rendering, exit 130, `template` abort, timing freshness and
  byte-level store surgery.
- **No arch bookkeeping (ticket 10).** A scenario carries no tag, row id or
  marker for `.scratch/arch-design/`.

## Constraints that the decisions must handle

Earlier tickets pushed these items to this ticket. Each decision below must
answer the items that it names.

1. **Lint scope** (ticket 06, caveat 1). One `#[expect]` covers only the module
   that it sits on. An `#[expect]` on a module with no steps fails. Every step
   module must be a descendant of `mod bdd`. Step bodies stay thin.
   Helpers live in ordinary modules, where the bans apply.
2. **Placeholder style** (ticket 06, caveat 2). A typed placeholder such as
   `{count:u32}` fails `-D warnings` through
   `literal_string_with_formatting_args`. A bare `{count}` with a typed
   parameter passes. This ticket picks the style.
3. **One step registry** (ticket 06, caveat 5). Every feature in the `e2e`
   binary shares one registry. The same keyword and pattern twice panics at
   start. Overlapping patterns resolve to the most specific one.
4. **Gate patterns** (tickets 07 and 08). `check:layers` searches
   `tests/e2e.rs` and `tests/e2e/` with these patterns:
   - `traces` followed by `::`, `{`, `;` or the word `as`
   - `process` followed by `::`, `{` or the word `as`, outside `support.rs`

   A placeholder directly after the word "process" or "traces" matches. Example:
   `the process {outcome}`. Steps and `.feature` text must avoid these
   patterns. `git grep` reads tracked files only.
5. **Gate scope** (tickets 08 and 09). `check:layers` must scan every path that
   holds feature files and steps. It must fail when `git grep` exits with code
   2 or higher. The adoption slice writes the gate change. This ticket fixes
   the paths that the change names.
6. **Missing steps** (ticket 06, caveat 4). Compile-time step validation exists
   only for `#[scenario]`. A missing step under `scenarios!` fails at run time.
   This ticket chooses `scenarios!` or `#[scenario]`, and the way an author
   finds dead or duplicate steps.
7. **Skips** (ticket 06, caveat 7). `skip!()` passes by default. This ticket
   states the skip policy.
8. **Test names** (ticket 06). A test name is the feature file stem plus the
   scenario title. Outline rows add `::case_N`. Nextest filters use these
   names.
9. **Fixture quirk** (ticket 06, caveat 8). A one-line `#[fixture]` body trips
   `unused_braces`.
10. **Three-OS runs** (tickets 06 and 09). Linux and Windows are unverified.
    Step text and fixtures must not depend on one OS.
11. **Existing e2e files.** `tests/e2e/` already holds `completions`, `config`,
    `env_policy`, `index`, `init`, `process_contract`, `query`, `template`,
    `tracked`, `trust` and `workflow`. Ticket 13 decides which of the 27
    scenario candidates convert. The layout must give them a home.
12. **Glossary disputes** (ticket 12, decision f). Ticket 12 records a
    disposition for each doc/code contradiction. Examples: the ADR-0002
    "stale" and "untrusted" terms, and the glossary claims about slugs,
    callouts and footnotes. Scenario text must not use a disputed term before
    ticket 12 resolves it.

## Fog graduated into this ticket

These items were in the map's "Not yet specified" section:

- Scenario density and verbosity norms: how much behavior one scenario holds,
  and when a Scenario Outline beats a plain scenario. See decision (e).
- The fate of the `//! Proves` and `Defect class:` documentation style. Ticket
  05 found that the `Defect class:` taxonomy is undocumented and drifts. See
  decision (g). Ticket 13 decides what happens to the headers of existing tests.

## Other inputs

- The convention review: [research/11](../research/11-conventions.md). It covers
  Cucumber guidance, cucumber-rs and rstest-bdd practice, and the tool facts
  from the `rstest-bdd` 0.6.0 source.
- The repo conventions: the vocabulary in `GLOSSARY-MAP.md` and the 11
  per-module `GLOSSARY.md` files, and the file organization of `tests/`.
- The capabilities of rstest-bdd in the cached sources under
  `.rust-docs/cache/crates/rstest-bdd/`: placeholders, datatables, fixtures,
  tags and `scenarios!` globbing.

## Decisions to reach

HITL, grilling. Do not answer for the user. Reach these decisions:

- (a) Choose the directory layout for `.feature` files.
  - Choose a grouping: by domain, by workflow, or by the existing e2e file
    names.
  - State the exact paths for features and steps. Constraints 4, 5, 8 and 11
    apply.
  - State the naming rule for feature files.
  - State where the `touch` rule is written, so authors find it.
- (b) Define the step-definition structure. Constraints 1, 2, 3 and 6 apply.
  - State where steps live and how step files nest under `mod bdd`.
  - Group step files by domain concept, with domain names. Never use one step
    file for each feature file. Cucumber names that an anti-pattern.
  - State the naming convention for step functions and step text.
  - Separate shared steps from domain steps. State the rule that stops two
    features from registering the same pattern.
  - Choose the placeholder style.
  - Choose `scenarios!` or `#[scenario]`.
  - State how steps bind fixtures, because rstest-bdd has no World.
- (c) Design the fixtures. Constraints 9 and 10 apply.
  - State which fixtures exist, who owns them and how they isolate scenarios.
  - State how `Sandbox` and `TZ` enter each scenario.
  - Settle the role of `test-utils`. The `e2e` binary cannot use it, so decide
    if it has any role before the facade-driver binary exists.
- (d) State the vocabulary rule. Constraint 12 applies.
  - Choose: scenario text draws from `GLOSSARY.md` terms, or the text is
    rewritten.
  - State how an author handles a term that ticket 12 has not resolved.
  - State how step text names the observable channels, in terms that stay
    independent of the driver.
- (e) Write the scenario-writing conventions. They must be concrete enough to
  hand to an author.
  - State the Given/When/Then style and the use of `Background`.
  - State when a Scenario Outline is required, and its size limit. Ticket 08
    allows 5 to 10 key examples next to a proptest, and bans combinatorial
    suites.
  - State the density norm: how much behavior one scenario holds.
  - State the tag set. The set holds the known-limitation tag from ticket 07,
    rule 3, and nothing for arch-design.
  - State the skip policy (constraint 7).
  - State how the text avoids the gate patterns (constraint 4).
- (f) State the anti-abstraction rules.
  - Name the step-helper layers that are banned.
  - Obey the standing preference for minimal abstraction.
  - Check the rules against ticket 08's anti-goals.
- (g) State the provenance convention for a scenario.
  - Choose: absorb `//! Proves` and `Defect class:` into Gherkin, keep them
    alongside, or replace them.
  - If a taxonomy stays, state who owns it.
- (h) State how an author finds dead, missing and duplicate steps (constraint
  6), and which command or gate shows them.

## Boundaries

- Do not write scenarios for the slices. Ticket 14 owns the slice list.
- Do not change `mise.toml`. The adoption slice owns the gate change.
- Do not resolve a doc/code contradiction. Ticket 12, decision (f), owns it.
- Do not decide which existing tests convert. Ticket 13 owns that decision.
- Do not decide the agent policy for specs. Ticket 14 owns it.

## Answer records

The answer records these items:

- the layout, with exact paths for features and steps
- the step and fixture rules
- the placeholder style and the skip policy
- a short authoring-convention summary that ticket 14's roadmap can quote
  verbatim
- the paths that the `check:layers` change names (for the adoption slice)
- the home for the e2e scenario candidates (input for ticket 13)
- the provenance convention (decision g)

## Answer

The user confirmed every decision below in this session. Evidence:
[research/11](../research/11-conventions.md). The probes ran on macOS only.

### Decision (a): layout

- Features live in `tests/features/<capability>.feature`. The directory is
  flat. One file covers one behavioral area, such as `trust`, `init`, `config`,
  `query` or `template`. This is the Rust and Cucumber convention.
- Steps live in `tests/e2e/bdd/mod.rs` plus domain-concept files. Wire them in
  `tests/e2e.rs` with `#[path = "e2e/bdd/mod.rs"] mod bdd;` and one
  `#[expect(clippy::disallowed_methods)]`. The form `#[path = "e2e/bdd.rs"]`
  is wrong: child modules then resolve to `tests/e2e/<name>.rs`.
- A scenario title is unique in the whole suite. A duplicate gets an unstable
  numeric suffix.
- The `touch` rule is in the module doc comment of `tests/e2e/bdd/mod.rs`. After
  you add a feature file, run `touch tests/e2e/bdd/mod.rs`.
- **Change to ticket 08.** Ticket 08 says `check:layers` scans every path that
  holds feature files and steps. This ticket reads it as: scan every path that
  holds Rust. The Rust-symbol patterns scan `tests/e2e/bdd/`. They do not scan
  `tests/features/`, because feature text has no Rust code. The feature-text
  checks in decision (h) cover `tests/features/`.
- One directory and one consumer: the `e2e` binary. A second driver, for
  example a facade binary, reopens this layout.

### Decision (b): steps

- Use one `scenarios!("tests/features", fixtures = [...])` call. A missing
  step and a duplicate step are run-time failures.
- Create each step file when its first step exists. A file such as `process.rs`
  holds the cross-feature steps for run, exit code and streams. Domain files
  such as `trust.rs` hold the rest. Never use one step file for each feature
  file. Cucumber names that an anti-pattern.
- Keep steps atomic. Keep helpers in the file of their step.
- A step parameter has the name of its fixture, or uses `#[from(name)]`. Do
  not name a step function `table` or `docstring`.
- Use bare `{name}` placeholders only. Write the quotes in the pattern:
  ``stderr contains "{text}"``. Numbers stay unquoted. A value never holds a
  double quote. Use a docstring for such a value. Typed placeholders such as
  `{count:u32}` fail clippy.
- A literal pattern beats an overlapping placeholder pattern.

### Decision (c): fixtures

- Two plain fixtures: `sandbox: Sandbox` and `last_run: LastRun`. `LastRun` is
  a newtype over `Option<Run>`. There is no World.
- The fixture builds the sandbox. A scenario has no "isolated sandbox" step.
- `Sandbox` sets `TZ=UTC`. A scenario for another zone uses a `When` variant,
  such as `When I run "traces ..." in time zone "Europe/Berlin"`. It calls
  `run_with_envs` with `TZ`. The adoption slice must check the signature.
- `test-utils` has no role in the `e2e` binary. A facade driver is deferred.

### Decision (d): vocabulary

- Use settled glossary terms for behavior. Use ordinary words for setup. Never
  name an internal type.
- Name the observable contract plainly: stdout, stderr, exit code, the file.
- A term that ticket 12 has not resolved stays out of scenarios.
- A command line in a `When` step is allowed. Example: `When I run "traces
  trust"`.

### Decision (e): scenario conventions

- A scenario shows one observable behavior. Prefer 3 to 5 steps. A reviewer asks
  for a reason above 5. Use one principal `When`. Split a scenario that has a
  second `When` and `Then` pair.
- A Scenario Outline holds only rows that distinguish behavior. Ticket 08 caps
  key examples at 5 to 10 next to a proptest and bans combinatorial suites.
- A `Background` is short and holds only context that every scenario in the
  file shares. Reconsider it above 4 lines.
- The tag set is `@known-limitation`. It is inert while no `tags =` filter uses
  it. Do not add `@wip`, `@smoke` or `@slow`.
- `skip!()`, `@ignore` and `Rule:` are banned. `RSTEST_BDD_FAIL_ON_SKIPPED` is
  not set. A `Rule:` block silently generates no tests.

### Decision (f): anti-abstraction

- Steps call `Sandbox` and `Run` methods directly.
- A concrete operation that repeats becomes a plain helper function in the step
  file.
- Banned: a CLI DSL or builder, generic assertion wrappers, macros that
  generate steps, trait-based step contexts, per-domain World structs, and a
  helper module outside the step files.
- A new helper goes into `support.rs` only if an existing e2e test also uses it.

### Decision (g): provenance

- The `Feature` description states the guarantee in business-readable words. It
  links the intent source that ticket 07, rule 3 requires.
- Scenarios carry no `Defect class:` taxonomy.
- Ticket 13 decides what happens to the `//! Proves` and `Defect class:`
  headers of existing tests.

### Decision (h): dead, missing and duplicate steps and the gate

- Add a step only with the first scenario that uses it. Delete a step when its
  last scenario goes. A reviewer checks this.
- Keep the `diagnostics` feature off. When dead steps are suspected, run
  `cargo bdd unused` in a throwaway local change that enables the feature.
- The adoption slice adds these checks to `check:layers`, over `tests/features/`
  and `tests/e2e/bdd/`:
  - no `Rule:` line
  - no `skip!`
  - no `@ignore`
  - no duplicate `Scenario:` title across files
  - a failure when `git grep` exits with code 2 or higher
- The Rust-symbol patterns still apply to `tests/e2e/bdd/`. They match `traces`
  or `process` before `{`, `;`, `::` or the word `as`. Avoid such step
  patterns. Example: `the process {outcome}`.

### Answer records

- **Authoring summary (for ticket 14 to quote).** Features are in
  `tests/features/`, one file for each behavioral area. Steps are in
  `tests/e2e/bdd/`, one file for each domain concept. Use bare `{name}`
  placeholders with the quotes in the pattern. Write 3 to 5 steps and one
  principal `When`. Never use `Rule:`, `skip!()` or `@ignore`. Keep scenario
  titles unique. The `Feature` description links the intent source.
- **Paths for the `check:layers` change.** Rust-symbol patterns scan
  `tests/e2e.rs` and `tests/e2e/`, which include `tests/e2e/bdd/`. Feature-text
  checks scan `tests/features/`.
- **Home for converted e2e tests (input for ticket 13).** A converted test
  becomes a scenario in the feature file named for its capability. The 27
  candidates map to files such as `config`, `template` and `query`. Delete the
  old Rust test only after ticket 13's rules allow it.
- **Constraint for ticket 12.** Disputed glossary terms stay out of scenarios
  until decision (f) of ticket 12 resolves them.
- **Constraint for ticket 14.** The pilot slice creates `tests/features/`,
  `tests/e2e/bdd/mod.rs` with the `touch` doc comment, the `Zlib` entry in
  `deny.toml`, and the `check:layers` changes above. It checks Linux and
  Windows CI.
