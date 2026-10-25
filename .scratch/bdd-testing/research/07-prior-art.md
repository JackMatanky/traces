# Research 07: Prior art for the boundary-tiering decision

This note is evidence for [ticket 07](../issues/07-boundary-tiering.md). It uses
three inputs:

- Robert C. Martin: blog posts (2008 to 2018) and AI-era material (2025 to
  2026).
- Dave Farley's ATDD-for-agents argument. The user supplied a summary of a
  Modern Software Engineering video. Nobody watched the video.
- The five inventories (`research/01` to `research/05`).

## Confidence

Read directly:

- Martin's blog posts *The Truth about BDD* (2008), *First-Class Tests* (2017),
  *TDD Harms Architecture* (2017) and *Pickled State* (2018).
- The file tree and the README of `unclebob/empire-2025`.

Read second-hand:

- X blocked the fetches. Martin's AI-era positions come from search snippets of
  his X posts.
- A third-party gist extracts a Matt Pocock interview (2026-08-19).
- The explainx.ai article summarizes the X thread.
- The noqta.tn article ("compiler output", 8 checkpoints) is a community
  formalization. It is not Martin's work. Do not use it as evidence.

## Martin before AI

- Gherkin is a finite state machine: `GIVEN state / WHEN event / THEN state`.
  Gherkin is a proper subset of the unit-test state machine. The extra unit-test
  states are programming details that the business cannot see (Pickled State).
- A one-to-one match between test structure and production structure is tight
  coupling. Tests need a decoupling API. You must design the tests (TDD Harms
  Architecture).
- The business writes acceptance tests. Integration tests and system tests are
  "plumbing, not business rules" (First-Class Tests).
- You find missing pairs when you list states and events. The comment thread
  says hidden internal state weakens the count of states times events (Truth
  about BDD).

## Martin with AI (lower confidence)

- He does not read agent code. He puts constraints around the agents: unit
  tests, Gherkin, QA procedures, coverage, CRAP, dependency rules and mutation
  testing.
- Agents write the unit tests, the Gherkin and the QA procedures. He reviews the
  Gherkin and the QA procedures. He reviews some thoroughly and some as a spot
  check, by criticality.
- He writes Gherkin "from a human operator's point of view ... operating this
  system at the UI".
- He uses a pipeline of narrow agents: specifier, coder, cleaner (CRAP),
  hardener (mutation) and QA.
- "Values, not disciplines": he does not impose the strict TDD nano-cycle on
  agents.
- Deterministic tools beat prose rules.
- He calls prose spec-driven development "always a disaster". He keeps no
  persisted list of prose specs. But `empire-2025` keeps about 55 acceptance
  files. [INFERENCE: prose specs are short-lived. Executable acceptance tests
  persist.]
- He doubts the value of test overload: "lots of times I just use unit tests and
  crap".
- `empire-2025` enforces its test boundaries with scripts:
  - Specs must not touch `empire.atoms`. They use `empire.test-utils`.
  - The scripts are `check-spec-boundary`, `check-acceptance-boundary` and
    `check-generated-acceptance-boundary`.
  - A dependency checker and a CI guard also run.
  - This is the same pattern as `check:layers` and `check:visibility` in this
    repo.

## Farley (user summary)

- Specs use domain language. They name no endpoint, database or class. This
  argues against option (c).
- The layers are: spec, DSL, protocol driver and system under test. In this repo
  the step glue is the DSL. `Sandbox` (spawn) or `traces::testing` (in-process)
  is the driver. The tier question is: which driver does a step use?
- Tests that you write after the code can only confirm what the code does. This
  risk applies to a retrofit. You need intent evidence (ADR, glossary, doc
  claim). "Currently pinned" is not enough.
- Read-only specs, red-first and small cycles belong to tickets 12 and 13 and to
  agent policy. They do not belong to ticket 07.
- Mutation testing verifies the "leave as unit test" direction. The tool is
  installed. The task is `mise run test:mutants`. Ticket 11 sets the policy.

## Facts from the inventories that shape ticket 07

1. `test-utils` membership does not mean a user can see the item.
   `RefreshReport`, `DriftCause` and `InlinkMap` are exported. Production only
   logs them (research/03, section E).
2. `check:layers` (research/05, G6): e2e never names `traces::`, and integration
   never spawns. The side of this gate that a scenario's steps use is the tier
   of the scenario.
3. These guarantees are observable but have no spawned test: research/01 G11,
   G12, G16 and G17, `TRACES_CEILING_DIRS` and exit 130. They are candidates for
   promotion.
4. These guarantees are strong but exist only at internal seams: research/03
   section C items 1 to 12 (arch tickets 23, 24, 25, 32 and 34) and research/02
   G14 to G18.
5. These behaviors are hostile to Gherkin:
   - TTY and pty
   - signals
   - byte-level store surgery
   - `syn` scans
   - mtime and timing
   - thread-local time zone
   - miette wrapping

   The sources are research/01 H1 to H9, research/02 H1 to H8 and research/03
   section E.
6. Other owners already hold these guarantees: the 4 proptest blocks
   (research/04, section 5), `check:visibility`, the lint denials and `#[case]`
   tables.
