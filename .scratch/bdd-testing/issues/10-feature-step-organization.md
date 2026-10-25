# 10: Feature and step organization

Type: grilling
Status: open
Blocked by: 06, 07

## Question

How are feature files, scenarios, step definitions, and fixtures organized so they stay readable and maintainable as the system evolves, with no unnecessary abstraction?

**Inputs**: ticket 06's verdict (crate pinned or fallback chosen; its macros' actual API), ticket 07's tiering rule (tells us which surfaces scenarios address), the repo's conventions (`GLOSSARY-MAP.md` vocabulary should be reflected in Gherkin phrasing; existing `tests/` file organization; `tests/integration/e2e` layer gates), rstest-bdd's real capabilities from the cached sources under `.rust-docs/cache/crates/rstest-bdd/` (typed placeholders, datatables, fixtures, tags, `scenarios!` globbing).

**Decisions to reach** (HITL, grilling): (a) directory layout for `.feature` files (by domain? by workflow? where do they live relative to `tests/` and the layer gates); (b) step-definition structure: where steps live, naming conventions, how shared vs domain-specific steps are separated, how steps bind fixtures instead of a World (rstest-bdd has no World); (c) fixture design: which fixtures exist, ownership and isolation, how `test-utils` figures in; (d) vocabulary rule: scenario text draws from `GLOSSARY.md` terms or is re-written; (e) scenario-writing conventions (Given/When/Then style, when a Scenario Outline is required, background usage) concrete enough to hand to an author; (f) explicit anti-abstraction rules: what step-helper layering is banned (this map's standing preference is minimal abstraction).

**Answer records**: the layout, the step/fixture rules, and a short authoring-convention summary that ticket 13's roadmap can quote verbatim.
