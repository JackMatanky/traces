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
  as unit tests, or promote via a scenario at a higher boundary.
- (c) Give an explicit answer on which option won. Give the reasons.

Consider the goal this whole map serves: behavioral stability with
implementation flexibility, where specs must survive the refactor of ticket 09's
stress case. A seam that the planned refactor would move is a bad scenario
target, at any tier.

**Answer records**: the tiering rule, the internal-seam policy, and any
discovered guarantee classes that forced exceptions.

## Answer

Resolved with the user. Evidence:
[research/07-prior-art.md](../research/07-prior-art.md) and research/01-05.

### (c) Winner: option (b), sharpened

Scenarios stop at a **product boundary**: today only the spawned `traces`
process (the CLI is the only supported surface). Option (c), internal module
seams, is rejected as a tier: the 12 refactor-hostile guarantees in research/03
section C and research/02 G14-G18 sit on seams that arch-design tickets 23, 24,
25, 32 and 34 may move.

- **Default**: the spawned process. Facade seam allowed only if the symbol is
  already in a frozen baseline (`public-api-test-utils.txt`) AND a spawn cannot
  express the behavior (e.g. a value-type observable). The `traces::testing`
  facade is an exception tier, not a peer: arch ticket 32 may re-home it.
- The default-features library API is NOT a supported product surface today
  (`public-api-default.txt` is a frozen internal API, not a scenario seam).
- **Planned workspace** (core, cli, lsp, future mcp): each new shipped entry
  point (LSP over stdio, MCP) becomes a product boundary with its own driver.
  `core`'s public API becomes a seam only if declared supported. Adding a
  boundary reopens this rule; do not pre-build drivers for it (YAGNI, no custom
  infra). Keep step text driver-agnostic where free.

### (a) Tiering rule

A guarantee earns a scenario only if ALL hold:

1. **Observable behavior**: a business-visible state transition OR an
   input-to-output mapping (grammar, sort, date literals, escaping). 2.
   **Observable channel**: stdout payload, stderr diagnostic *codes* (never
   miette-wrapped text), exit status, files written under the project root. Not
   `index.redb` contents, hash-named trust/tracked store entries, or
   `RefreshReport`/`DriftCause`/`InlinkMap` (exported, not user-visible).
2. **Intent evidence AND code agreement**: an ADR, glossary entry, doc claim or
   workflow comment that the code matches. A doc/code contradiction (research 01
   s2, 02 s3, 03 D, 05 s5) is resolved first by a recorded decision. Documented
   known limitations are allowed under a distinct tag (e.g. the same-size
   mtime-restore canary).
3. **No gate/algebra owner**: excluded only when a compile-time gate, lint, or
   proptest law owns it (`check:visibility`, `check:layers`, clippy denials, the
   4 `temporal/` proptest blocks). Overlap with other tests is allowed (two
   independent streams).
4. **Hermetic**: runs under the sandbox default-deny env policy. Time zone is
   set per spawn via `TZ`; it is NOT an exception.
5. **Refactor-stable**: not a seam an open arch-design ticket will move.
   Provisional list: research/03 C items 1-12, research/02 G14-G18. Ticket 09
   owns and refines the list (09 is blocked by 07).

### (b) Internal-seam policy, in order

1. **Promote** to a process-boundary scenario if user-visible. Candidates:
research/01 G11, G12, G16, G17; `TRACES_CEILING_DIRS`; exit 130.
2. **Leave as a unit test**, verified by mutation testing: `mise run
   test:mutants -m <module>` (mutarust; `--all-targets`, baseline
   `mutarust-baseline.json`, `--min-msi`, `--fail-on-escaped`). Ticket 11 sets
   thresholds and scope after reading `mutarust.yml`. 3. **Retire or relabel**
   incidental pins (mtime canary framing, misleading sort test names). Ticket 12
   owns the how. 4. **Wrap in a `test-utils` contract: forbidden by default.**
   Allowed only when an arch ticket creates a real user-visible projection (e.g.
   arch ticket 25 item 5 for refresh reporting).

### Mechanical gate

Adopting BDD must leave `public-api-default.txt`, `public-api-test-utils.txt`
and `pub-tokens.txt` unchanged. A scenario needing a new `pub`/`pub(crate)` item
fails. When the workspace splits, this applies per crate.

### Exception classes (never plain scenarios; each names its alternative)

| Class | Alternative |
|---|---|
| Real-TTY rendering (inquire), research/01 H1 | unit test; no pty harness |
| Exit 130 / SIGINT | one explicit spawned-signal regression case (Unix); in-process unit test stays |
| `template` abort/cancel via `pub(super)` seam | in-process unit with injected provider (`init` abort is reachable via public `PresetDialogProvider`) |
| Timing freshness (racy-timestamp gap, mtime) | unit/property; canary tagged known-limitation |
| Byte-level store surgery, `syn` source scans | unit tests |

Ticket 11 takes this list as coverage-gap input.

### Constraints passed on

- Ticket 10: `check:layers` greps `tests/e2e/` for `traces` followed by `::`,
  `{`, `;` or `as`; `.feature` text under that directory must avoid it.
- Ticket 09: refine the moving-seam list in rule item 6.
- Ticket 11: mutarust policy; exception list as gap input.
- Ticket 12: retire/relabel incidental pins.
- Ticket 13: agent policy (read-only specs, red-first) is out of 07's scope.
