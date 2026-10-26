# 07: Boundary tiering, b versus c

Type: grilling
Status: resolved
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
their "Gherkin-suitability" and "intentional vs incidental" verdicts. Prior art
and consolidated evidence:
[research/07-prior-art.md](../research/07-prior-art.md).

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) State the tiering rule. It must be precise enough to classify any future
  guarantee. Say what makes a seam observable and refactor-stable enough for a
  scenario.
- (b) Decide what happens to strong guarantees found at internal seams only.
  They can go in three directions: wrap in a test-utils-visible contract, leave
  as unit tests, or promote with a scenario at a higher boundary.
- (c) Give an explicit answer on which option won. Give the reasons.

Consider the goal this whole map serves: behavioral stability with
implementation flexibility, where specs must survive the refactor of ticket 10's
stress case. A seam that the planned refactor would move is a bad scenario
target, at any tier.

**Answer records**: the tiering rule, the internal-seam policy, and any
discovered guarantee classes that forced exceptions.

## Answer

The user and the agent resolved this ticket together. The evidence is in
[research/07-prior-art.md](../research/07-prior-art.md) and in research/01 to
research/05.

### Decision (c): option (b) wins, with a sharper boundary

Scenarios stop at a product boundary. Today the only product boundary is the
spawned `traces` process. The CLI is the only supported surface.

Option (c) is rejected as a tier. It reaches internal module seams. Arch-design
tickets 23, 24, 25, 32 and 34 can move these seams. They hold 12
refactor-hostile guarantees (research/03, section C) and the plan internals in
research/02, G14 to G18.

Boundary rules:

- Default: address the spawned process.
- Use a facade seam only if both conditions are true:
  - The symbol is already in a frozen baseline (`public-api-test-utils.txt`).
  - A spawn cannot express the behavior. Example: a value-type observable.
- The `traces::testing` facade is an exception tier, not a peer. Arch ticket 32
  can re-home it.
- The default-features library API is not a supported product surface today.
  `public-api-default.txt` freezes an internal API. It is not a scenario seam.

Planned workspace (core, cli, lsp, future mcp):

- Each new shipped entry point (LSP over stdio, MCP) becomes a product boundary.
  It has its own driver.
- The public API of `core` becomes a seam only if you declare it supported.
- A new boundary reopens this rule.
- Do not build drivers for these boundaries before they exist. The map allows no
  custom infrastructure.
- Keep step text independent of the driver when this costs nothing.

### Decision (a): tiering rule

A guarantee earns a scenario only if all six conditions are true:

1. **Observable behavior.** The guarantee is a business-visible state transition
   or an input-to-output mapping. Examples of mappings: grammar, sort, date
   literals, escaping.
2. **Observable channel.** The channel is one of these:
   - the stdout payload
   - the stderr diagnostic codes (never miette-wrapped text)
   - the exit status
   - the files under the project root

   These are not channels: the `index.redb` contents, the hash-named trust and
   tracked store entries, and `RefreshReport`, `DriftCause` and `InlinkMap`. The
   last three are exported but are not user-visible.
3. **Intent evidence and code agreement.** An ADR, glossary entry, doc claim or
   workflow comment states the intent. The code matches it. Resolve a doc/code
   contradiction with a recorded decision before you write the scenario. The
   contradictions are in research/01 section 2, research/02 section 3,
   research/03 section D and research/05 section 5. A documented known
   limitation is allowed under a separate tag. Example: the same-size
   mtime-restore canary.
4. **No gate or algebra owner.** Exclude a guarantee only when a compile-time
   gate, a lint or a proptest law owns it. Examples: `check:visibility`,
   `check:layers`, the clippy denials and the 4 `temporal/` proptest blocks.
   Overlap with other tests is allowed. Two independent test streams are an
   advantage.
5. **Hermetic.** The scenario runs under the sandbox default-deny env policy.
   Set the time zone with `TZ` at each spawn. Time zone is not an exception.
6. **Refactor-stable.** No open arch-design ticket will move the seam. The
   provisional list is research/03 section C items 1 to 12 and research/02 G14
   to G18. Ticket 10 owns the list and refines it. Ticket 07 blocks ticket 10.

### Decision (b): policy for internal-seam guarantees

Try these directions in order:

1. **Promote.** Make the guarantee a process-boundary scenario if users can see
   it. Candidates: research/01 G11, G12, G16 and G17, `TRACES_CEILING_DIRS` and
   exit 130.
2. **Keep as a unit test and check it with mutation testing.** Run
   `mise run test:mutants -m <module>`. The task uses mutarust, `--all-targets`,
   `mutarust-baseline.json`, `--min-msi` and `--fail-on-escaped`. Ticket 12 sets
   the thresholds and the scope. Ticket 12 first reads `mutarust.yml`.
3. **Remove or relabel incidental pins.** Examples: the mtime canary framing and
   the misleading sort test names. Ticket 13 decides how.
4. **Do not wrap the guarantee in a `test-utils` contract.** Make an exception
   only when an arch ticket creates a real user-visible projection. Example:
   arch ticket 25, item 5, for refresh reporting.

### Mechanical gate

BDD adoption must not change `public-api-default.txt`,
`public-api-test-utils.txt` or `pub-tokens.txt`. A scenario that needs a new
`pub` or `pub(crate)` item fails the gate. After the workspace split, the gate
applies to each crate.

### Exception classes

These classes are never plain scenarios. Each class has an alternative.

| Class | Alternative |
|---|---|
| Real-TTY rendering (inquire), research/01 H1 | Unit test. Do not build a pty harness. |
| Exit 130 (SIGINT) | One explicit spawned-signal regression case (Unix). The in-process unit test stays. |
| `template` abort or cancel through the `pub(super)` seam | In-process unit test with an injected provider. The `init` abort is reachable through the public `PresetDialogProvider`. |
| Timing freshness (racy-timestamp gap, mtime) | Unit test or property test. Tag the canary as a known limitation. |
| Byte-level store surgery, `syn` source scans | Unit tests. |

Ticket 12 uses this list as input for the coverage gaps.

### Constraints for other tickets

- Ticket 10: refine the moving-seam list (rule 6).
- Ticket 11: `check:layers` searches `tests/e2e/` for `traces` followed by `::`,
  `{`, `;` or the word `as`. Keep the `.feature` text in that directory free of
  these patterns.
- Ticket 12: set the mutarust policy. Use the exception list as input for the
  coverage gaps.
- Ticket 13: retire or relabel the incidental pins.
- Ticket 14: agent policy (read-only specs, red-first) is outside ticket 07.
