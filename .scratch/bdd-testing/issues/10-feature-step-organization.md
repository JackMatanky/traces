# 10: Feature and step organization

Type: grilling
Status: open
Blocked by: 06, 07, 08, 14

## Question

How do you organize feature files, scenarios, step definitions, and fixtures so
they stay readable and maintainable as the system evolves, with no unnecessary
abstraction?

**Inputs**:

- ticket 06's verdict (crate pinned or fallback chosen; its macros' actual API)
- ticket 07's tiering rule (tells you which surfaces scenarios address)
- the repo's conventions: `GLOSSARY-MAP.md` vocabulary should be reflected in
  Gherkin phrasing; existing `tests/` file organization; `tests/integration/e2e`
  layer gates
- rstest-bdd's real capabilities from the cached sources under
  `.rust-docs/cache/crates/rstest-bdd/` (typed placeholders, datatables,
  fixtures, tags, `scenarios!` globbing)

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) Choose the directory layout for `.feature` files: by domain, by workflow,
  and where they live relative to `tests/` and the layer gates.
- (b) Define the step-definition structure: where steps live, naming
  conventions, and how shared vs domain-specific steps are separated. Also
  define how steps bind fixtures instead of a World (rstest-bdd has no World).
- (c) Design the fixtures: which fixtures exist, ownership and isolation, and
  how `test-utils` figures in.
- (d) State the vocabulary rule: scenario text draws from `GLOSSARY.md` terms,
  or the text is re-written.
- (e) Write scenario-writing conventions concrete enough to hand to an author:
  Given/When/Then style, when a Scenario Outline is required, and background
  usage.
- (f) State the anti-abstraction rules: which step-helper layering is banned
  (this map's standing preference is minimal abstraction).

**Answer records**: the layout, the step/fixture rules, and a short
authoring-convention summary that ticket 13's roadmap can quote verbatim.
