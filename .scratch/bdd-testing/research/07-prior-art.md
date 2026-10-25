# Research 07: Prior art for the boundary-tiering decision

Evidence for [ticket 07](../issues/07-boundary-tiering.md). Inputs: Robert C.
Martin (blog 2008-2018; AI-era material 2025-26), Dave Farley's ATDD-for-agents
argument (user-supplied summary of a Modern Software Engineering video, not
watched), and the five inventories (`research/01`-`05`).

## Confidence

- **Read directly**: Martin's blog posts *The Truth about BDD* (2008),
  *First-Class Tests* (2017), *TDD Harms Architecture* (2017), *Pickled State*
  (2018); the `unclebob/empire-2025` file tree and README.
- **Secondhand**: Martin's AI-era positions come from search snippets of his X
  posts (X blocked fetches), a third-party gist of a Matt Pocock interview
  (2026-08-19), and explainx.ai. The noqta.tn "compiler output" and 8-checkpoint
  material is community formalization, not Martin; discount it.

## Martin before AI

- Gherkin is a finite state machine (`GIVEN state / WHEN event / THEN state`).
  Gherkin is a proper subset of the unit-test state machine; the extra unit-test
  states are programming details the business cannot see (Pickled State).
- A one-to-one correspondence between tests and production structure is tight
  coupling; tests need a decoupling API and must be designed (TDD Harms
  Architecture).
- Acceptance tests are written by the business. Integration and system tests are
  "plumbing, not business rules" (First-Class Tests).
- Enumerating states x events finds missing pairs; the thread notes hidden
  internal state weakens the S x E count (Truth about BDD).

## Martin with AI (lower confidence)

- Does not read agent code; surrounds agents with constraints: unit tests,
  Gherkin, QA procedures, coverage, CRAP, dependency rules, mutation testing.
- Agents write unit tests, Gherkin and QA procedures. He reviews Gherkin and QA
  (thoroughly or spot check, by criticality). Gherkin is written "from a human
  operator's point of view ... operating this system at the UI".
- Pipeline of narrow agents: specifier, coder, cleaner (CRAP), hardener
  (mutation), QA.
- "Values, not disciplines": strict TDD nano-cycle not imposed on agents.
- Deterministic tools beat prose rules. Prose spec-driven development "always a
  disaster"; keeps no persisted prose spec list. Yet `empire-2025` persists ~55
  acceptance files [INFERENCE: prose specs are ephemeral, executable acceptance
  tests persist].
- Doubts test overload: "lots of times I just use unit tests and crap".
- `empire-2025` mechanical boundaries: specs may not touch `empire.atoms`; use
  `empire.test-utils`; scripts `check-spec-boundary`,
  `check-acceptance-boundary`, `check-generated-acceptance-boundary`, a
  dependency checker and a CI guard. Same pattern as this repo's `check:layers`
  and `check:visibility`.

## Farley (user summary)

- Specs in domain language, no endpoints/databases/classes: argues against (c).
- Layers: spec, DSL, protocol driver, SUT. Here the step glue is the DSL and
  `Sandbox` (spawn) or `traces::testing` (in-process) is the driver. The tier
  question is which driver a step binds to.
- Echo-chamber risk of tests written after code applies to this retrofit:
  require intent evidence (ADR, glossary, doc claim), not just "currently
  pinned".
- Read-only specs, red-first and small cycles belong to tickets 12/13 and agent
  policy, not 07. Mutation testing is the map's fog item; it is the verifier for
  the "leave as unit test" direction.

## Facts from the inventories that shape 07

1. `test-utils` "guarantee" membership is not observability: `RefreshReport`,
   `DriftCause`, `InlinkMap` are exported but production only logs them (03 E).
2. `check:layers` (05 G6): e2e never names `traces::`; integration never spawns.
   A scenario's tier is which side of that gate its steps sit on.
3. Observable-but-unpinned-at-process: G11, G12, G16, G17,
   `TRACES_CEILING_DIRS`, exit 130 (01, 05). Promotion candidates.
4. Internal-seam-only strong guarantees: 03 section C items 1-12 (arch tickets
   23, 24, 25, 32, 34), 02 G14-G18.
5. Hostile to Gherkin: TTY/pty, signals, byte-level store surgery, `syn` scans,
   mtime/timing, thread-local TZ, miette wrapping (01 H1-H9, 02 H1-H8, 03 E).
6. Already owned elsewhere: 4 proptest blocks (04 section 5),
   `check:visibility`, lint denials, `#[case]` tables.
