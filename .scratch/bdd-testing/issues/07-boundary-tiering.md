# 07: Boundary tiering, b versus c

Type: grilling
Status: open
Blocked by: 01, 02, 03, 04, 05

## Question

Where do executable Gherkin specifications stop: option **(b)** at the process/CLI boundary plus the `test-utils` public surface (library-level observable contracts), or option **(c)** further in, reaching internal module seams as well?

The working hypothesis from charting is **(b)**, but the user is explicitly *considering (c)*, so this ticket must stress-test both against the discovery findings rather than rubber-stamp the default.

**Inputs**: the five candidate-guarantee inventories from tickets 01 to 05, especially their "Gherkin-suitability" and "intentional vs incidental" verdicts.

**Decisions to reach** (HITL, grilling): (a) tiering rule stated precisely enough to classify any future guarantee (what makes a seam observable and refactor-stable enough for a scenario?); (b) what happens to strong guarantees discovered at internal seams only (wrap in a test-utils-visible contract? leave as unit tests? promote via scenario at a higher boundary?); (c) explicit answer on which option won and why. Consider the goal this whole map serves: behavioral stability with implementation flexibility, where specs must survive the refactor of ticket 09's stress case; a seam that the planned refactor would move is a bad scenario target regardless of tier.

**Answer records**: the tiering rule, the internal-seam policy, and any discovered guarantee classes that forced exceptions.
