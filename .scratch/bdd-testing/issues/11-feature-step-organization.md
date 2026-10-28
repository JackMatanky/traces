# 11: Feature and step organization

Type: grilling
Status: open
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
- **Placement (tickets 08 and 09).** Scenarios run in the `e2e` binary only. The
  prototype used these paths:
  - `tests/e2e/bdd.rs` holds the steps.
  - `tests/e2e/features/` holds the `.feature` files.
  - `tests/e2e.rs` declares `#[path = "e2e/bdd.rs"] mod bdd;` with one
    `#[expect(clippy::disallowed_methods)]`.
  - The prototype scenario is in [research/09](../research/09-prototype.md),
    section 3.
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
