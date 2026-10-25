# 07: Boundary tiering, b versus c

Type: grilling
Status: open
Blocked by: 01, 02, 03, 04, 05

## Question

Where do executable Gherkin specifications stop? Option **(b)** stops at the
process/CLI boundary plus the `test-utils` public surface (library-level
observable contracts). Option **(c)** goes further in: it reaches internal
module seams as well.

Charting produced the working hypothesis **(b)**. The user is explicitly
*considering (c)*. So this ticket must stress-test both options against the
discovery findings. Do not rubber-stamp the default.

**Inputs**: the five candidate-guarantee inventories from tickets 01 to 05. Use
their "Gherkin-suitability" and "intentional vs incidental" verdicts.

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) State the tiering rule. It must be precise enough to classify any future
  guarantee. Say what makes a seam observable and refactor-stable enough for a
  scenario.
- (b) Decide what happens to strong guarantees found at internal seams only.
  They can go in three directions: wrap in a test-utils-visible contract, leave
  as unit tests, or promote via a scenario at a higher boundary.
- (c) Give an explicit answer on which option won. Give the reasons.

Consider the goal this whole map serves: behavioral stability with
implementation flexibility, where specs must survive the refactor of ticket 09's
stress case. A seam that the planned refactor would move is a bad scenario
target, at any tier.

**Answer records**: the tiering rule, the internal-seam policy, and any
discovered guarantee classes that forced exceptions.
