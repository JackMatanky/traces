# Research 14: Sequencing and Pilot Slice Evidence

This document provides external research for ticket 14 ("Adoption roadmap"). It
gathers evidence on sequencing incremental adoption and choosing a pilot slice.
It follows Simplified Technical English (ASD-STE100) principles.

## 1. Question

What do primary sources say about sequencing an incremental adoption and
choosing a pilot slice? How must an engineering team order slices when:

1. A slice must bootstrap infrastructure and validate a framework verdict.
2. The domain undergoes active refactoring across mapped moving seams.
3. CI cross-platform execution remains unverified.

## 2. Method and Source List

This research examined 25 distinct sources on 2026-11-03. Primary sources
include technical books, original methodology papers, engineering guidelines,
and official specifications.

### Primary and Key Secondary Sources

1. Cockburn, Alistair (2004). *Crystal Clear: A Human-Powered Methodology for
   Small Teams*. Addison-Wesley. Walking skeleton pattern. [Primary, High]
2. Hunt, Andy and Thomas, Dave (1999). *The Pragmatic Programmer*.
   Addison-Wesley. Tracer bullets pattern. [Primary, High]
3. Fowler, Martin (2004). *Strangler Fig Application*.
   <https://martinfowler.com/bliki/StranglerFigApplication.html> [Primary, High]
4. Sato, Danilo and Fowler, Martin (2014). *Parallel Change*.
   <https://martinfowler.com/bliki/ParallelChange.html> [Primary, High]
5. Fowler, Martin and Hammant, Paul (2014). *Branch by Abstraction*.
   <https://martinfowler.com/bliki/BranchByAbstraction.html> [Primary, High]
6. Wake, Bill (2003). *INVEST in Good Stories, and SMART Tasks*.
   <https://xp123.com/articles/invest-in-good-stories-and-smart-tasks/>
   [Primary, High]
7. Bach, James (1999). *Heuristic Risk-Based Testing*.
   <https://www.satisfice.com/download/heuristic-risk-based-testing> [Primary,
   High]
8. ISTQB (2023). *Certified Tester Foundation Level Syllabus v4.0*, Section 5.2
   "Risk Management". <https://www.istqb.org/> [Primary, High]
9. Reinertsen, Donald (2009). *The Principles of Product Development Flow*.
   Croma House. WSJF and Cost of Delay. [Primary, High]
10. Rother, Mike (2015). *Improvement Kata Reference Guide*. University of
    Michigan.
    <https://public.websites.umich.edu/~jmondisa/TK/KATA_Files/IK_Reference_Guide.pdf>
    [Primary, High]
11. ThoughtWorks (2026). *Technology Radar FAQ*.
    <https://www.thoughtworks.com/en-us/radar/faq> [Primary, High]
12. Wright, Hyrum and Winters, Titus (2020). *Software Engineering at Google*,
    Chapter 22 "Large-Scale Changes".
    <https://abseil.io/resources/swe-book/html/ch22.html> [Primary, High]
13. Nygard, Michael (2011). *Documenting Architecture Decisions*.
    <https://www.cognitect.com/blog/2011/11/15/documenting-architecture-decisions>
    [Primary, High]
14. Singer, Ryan (2019). *Shape Up*, Chapter 5 "Risks and Rabbit Holes".
    <https://basecamp.com/shapeup/1.4-chapter-05> [Primary, High]
15. Singer, Ryan (2019). *Shape Up*, Chapter 6 "Write the Pitch".
    <https://basecamp.com/shapeup/1.5-chapter-06> [Primary, High]
16. Ford, Neal, Parsons, Rebecca, and Kua, Patrick (2017). *Building
    Evolutionary Architectures*. O'Reilly Media. Precis:
    <https://evolutionaryarchitecture.com/precis.html> [Primary, High]
17. Wells, Don (1999). *Spike Solution*.
    <http://www.extremeprogramming.org/rules/spike.html> [Primary, High]
18. Cunningham, Ward, Beck, Kent, and Jeffries, Ron (1997–2006). *Spike
    Solution*. C2 Wiki: <https://c2.com/xp/SpikeSolution.html> [Primary, High]
19. Anderson, David J. and Carmichael, Andy (2016). *Essential Kanban
    Condensed*. Lean Kanban University Press.
    <https://www.thescrummaster.co.uk/wp-content/uploads/2019/09/Essential-Kanban-Condensed-v1.0.0.pdf>
    [Primary, High]
20. Cockburn, Alistair (1998). *Project Management Patterns*. C2 Wiki:
    <https://c2.com/cgi/wiki?ProjectManagementPatterns> [Primary, Medium]
21. Beck, Kent (2002). *Test Driven Development: By Example*. Addison-Wesley.
    Spike and tracer practices. [Primary, High]
22. Feathers, Michael (2004). *Working Effectively with Legacy Code*. Prentice
    Hall. Seams and characterization tests. [Primary, High]
23. Humble, Jez and Farley, David (2010). *Continuous Delivery*. Addison-Wesley.
    Deployment pipeline and automated verification. [Primary, High]
24. Ries, Eric (2011). *The Lean Startup*. Crown Business. Validated learning.
    [Primary, High]
25. ISO/IEC/IEEE 29119-1 (2022). *Software and systems engineering — Software
    testing — Part 1: General concepts*. Risk-based testing. [Primary, High]

*Useless source noted*: Fowler, Martin (2002), *The XP 2002 Conference* report;
it mentions spikes without operational rules.

## 3. Findings

### Walking Skeleton, Tracer Bullets, Spikes, and Vertical Slices

- **[D1]** Cockburn defines a walking skeleton as a minimal end-to-end
  implementation that connects all architectural components. It executes
  production code, builds automatically, and stays permanent (Cockburn 2004;
  high confidence).
- **[D2]** Hunt and Thomas distinguish tracer bullets from prototypes. Tracer
  bullets are production code that traces complete paths through systems.
  Prototypes explore isolated concepts and engineers discard them (Hunt & Thomas
  1999; high confidence).
- **[D3]** Beck and Cunningham define a spike as a thin end-to-end program
  written to explore difficult design choices. Teams discard spike code once
  learning occurs (Wells 1999, C2 2006; high confidence).
- **[D4]** Wake specifies vertical slicing through the INVEST criteria
  (Independent, Negotiable, Valuable, Estimable, Small, Testable). Slices cut
  vertically across all architecture layers like a cake slice (Wake 2003; high
  confidence).

### Strangler Fig, Branch by Abstraction, and Parallel Change

- **[D5]** Fowler describes the Strangler Fig pattern: place an interceptor
  around legacy systems to route requests gradually to replacement systems. Old
  systems shrink until teams decommission them safely (Fowler 2004; high
  confidence).
- **[D6]** Fowler and Hammant define Branch by Abstraction: introduce an
  abstraction layer between callers and suppliers. Build the replacement behind
  the abstraction. Swap implementations gradually in the main trunk without
  long-lived branches (Fowler & Hammant 2014; high confidence).
- **[D7]** Sato and Fowler define Parallel Change in three sequential phases:
  Expand (support both old and new interfaces), Migrate (move callers
  gradually), and Contract (delete old interfaces) (Sato & Fowler 2014; high
  confidence).

### Risk-Based Testing

- **[D8]** ISTQB CTFL v4.0 defines risk as likelihood multiplied by impact. It
  categorizes risks into product risks (functional failures) and project risks
  (schedule, environment, tools). Test sequencing prioritizes high product risk
  areas (ISTQB 2023; high confidence).
- **[D9]** Bach provides heuristics for risk-based testing: inside-out (system
  elements and failure modes) and outside-in (quality criteria). He advocates
  the "principle of diverse half-measures" to avoid single-technique blindness
  (Bach 1999; high confidence).
- **[D10]** Feathers defines a seam as a place where you can alter behavior
  without editing source code. Characterization tests pin actual behavior before
  refactoring begins (Feathers 2004; high confidence).

### Cost of Delay, WSJF, and Queuing Limits

- **[D11]** Reinertsen proves mathematically that Weighted Shortest Job First
  (WSJF / CD3 = Cost of Delay / Duration) minimizes total delay cost in
  multi-item waiting queues (Reinertsen 2009; high confidence).
- **[D12]** Reinertsen notes that WSJF assumes linear delay costs. It fails when
  information value dominates or when urgency changes abruptly. For solo
  developers with single-item queues (WIP=1), queueing delay equals zero; CD3
  calculations provide false precision (Reinertsen 2009; high confidence).
- **[D13]** Anderson applies Little's Law ($WIP = Throughput \times Lead\
  Time$). Limiting WIP to 1 maximizes throughput for single workers by
  eliminating task-switching waste (Anderson & Carmichael 2016; high
  confidence).

### Validated Learning and Experiment-Driven Adoption

- **[D14]** Ries defines validated learning as running minimum experiments that
  verify or refute specific hypotheses before large-scale investment (Ries 2011;
  high confidence).
- **[D15]** Rother defines the Improvement Kata in four steps: (1) Understand
  Challenge, (2) Grasp Current Condition, (3) Establish Next Target Condition,
  and (4) Experiment at the Threshold of Knowledge with PDCA cycles (Rother
  2015; high confidence).

### Tooling Adoption, Stop/Go, and Rollback Criteria

- **[D16]** ThoughtWorks defines Technology Radar rings: Assess (investigate),
  Trial (exercise on real production systems), Adopt (standard default), and
  Caution/Hold (avoid). A tool moves to Trial only after production use
  (ThoughtWorks 2026; high confidence).
- **[D17]** Google structures Large-Scale Changes (LSC) into four phases:
  Authorization, Change Creation, Sharding and Submitting, and Cleanup. Changes
  shard into independent atomic commits. Automated presubmit tests verify each
  shard (Wright & Winters 2020; high confidence).
- **[D18]** Google enforces "No Haunted Graveyards": unmaintained subsystems are
  protected only through thorough automated tests. Machine-enforced linter
  checks prevent backsliding into deprecated patterns (Wright & Winters 2020;
  high confidence).
- **[D19]** Nygard establishes Architecture Decision Records (ADRs). Reversible
  decisions state explicit Context, Decision, Status, and Consequences. When
  reversed or superseded, records retain historical status rather than deletion
  (Nygard 2011; high confidence).

### Deliberate Deferral and "Not Now" Practices

- **[D20]** Singer defines Shape Up boundaries: pitches explicitly list "rabbit
  holes" (known traps with patched solutions) and "no-gos" (deliberately
  excluded features). This bounds project risk to thin-tailed distributions
  (Singer 2019; high confidence).
- **[D21]** Singer describes "scope hammering": during execution, teams
  aggressively cut optional scope to protect deadlines and maintain quality
  (Singer 2019; high confidence).

### Definition of Done and Fitness Functions

- **[D22]** Humble and Farley define production-ready infrastructure increments:
  code commits must trigger automated build, lint, and test execution in
  continuous deployment pipelines (Humble & Farley 2010; high confidence).
- **[D23]** Ford, Parsons, and Kua define architectural fitness functions as
  objective integrity assessments of architectural qualities. Fitness functions
  must be automated, triggered on commit, and machine-enforced (Ford et al.
  2017; high confidence).

### Sequencing Heuristics Comparison

| Heuristic | Optimizes | Failure Mode | Fit for Solo/Small Rust Repo |
|---|---|---|---|
| **Walking Skeleton** | End-to-end integration proof | Overbuilt scaffolding without user value | **High**. Proves build, harness, and CI wiring on day one. |
| **Tracer Bullets** | Real-world production code alignment | Feature scope creep during exploration | **High**. Connects real CLI invocation to assertions. |
| **Spike** | Fast learning under high uncertainty | Keeping throwaway code in production | **Medium**. Good for crate evaluation; must throw away. |
| **WSJF / CD3** | Economic throughput in shared queues | High scoring overhead; false precision | **Low**. Inapplicable when queue capacity is one. |
| **Heuristic Risk-Based** | Coverage of critical failure points | Overlooking non-obvious failure modes | **High**. Targets Frozen guarantees and error exits first. |
| **Parallel Change** | Zero-downtime interface migration | Abandoning cleanup; lingering technical debt | **High**. Safe migration without breaking existing tests. |
| **Shape Up No-Gos** | Bounding scope and preventing derailment | Omitting necessary core capabilities | **High**. Explicitly rejects edge cases and moving seams. |

### Pilot Slice Acceptance Criteria Patterns

Primary sources converge on five required criteria for a pilot slice:

1. **End-to-end execution**: The slice invokes real system entry points and
   asserts observable outputs (Hunt & Thomas 1999; Cockburn 2004).
2. **Infrastructure bootstrapping**: Harness wiring, fixtures, and directory
   structures land in production locations (Cockburn 2004; Humble & Farley
   2010).
3. **Machine-enforced verification**: Automated CI checks and fitness functions
   execute and pass cleanly (Ford et al. 2017).
4. **Thin scope**: The slice pins exactly one stable behavioral capability; all
   optional embellishments are excluded (Wake 2003; Singer 2019).
5. **No regressions**: Existing test suites run without breakage or performance
   degradation (Feathers 2004; Sato & Fowler 2014).

### Rollback and Abort Patterns

When a tooling adoption fails its target conditions, primary sources dictate
explicit rollback patterns:

1. **Threshold of Knowledge abort**: Rother and Ries dictate stopping an
   experiment when measured observations contradict the foundational hypothesis
   (Rother 2015; Ries 2011).
2. **Independent commit reversibility**: Google and Nygard require changes to be
   atomic units. Reverting the commit restores clean baseline behavior without
   leaving orphaned dependencies (Wright & Winters 2020; Nygard 2011).
3. **Expand without contract**: In Parallel Change, if the new branch fails
   during Trial, delete the expansion branch and retain the unchanged original
   code (Sato & Fowler 2014).
4. **ADR deprecation**: When reverting a tool, record an ADR with status
   `Deprecated` or `Superseded`. Document the empirical failure causes for
   future developers (Nygard 2011).

## 4. Disagreements in the Literature

- **Tracer Bullets vs. Spikes**: Hunt & Thomas argue that even initial
  exploratory paths must use production code (Tracer Bullets). Beck, Cunningham,
  and Wells insist that initial exploratory code must be thrown away (Spikes).
  For library/tooling evaluation, literature resolves this by using throwaway
  spikes for pre-verdict assessment, followed by tracer bullets for initial
  adoption.
- **Quantitative WSJF vs. Heuristic Risk**: Reinertsen advocates mathematical
  economic optimization (WSJF). Bach and ISTQB recognize that technical risk and
  software defects cannot be accurately modeled with monetary delay numbers. For
  small teams, heuristic risk assessment outperforms WSJF.
- **Strangler Fig vs. Branch by Abstraction**: Fowler recommends Strangler Fig
  for multi-process systems (routing at HTTP/network boundaries). Hammant and
  Fowler recommend Branch by Abstraction for in-process monorepo refactoring.
  For single-crate Rust CLI workspaces, Branch by Abstraction fits better.

## 5. What Does Not Transfer to This Repo

- **Google Rosie and Automated Monorepo Sharding**: Google uses automated
  systems to generate and mail thousands of review shards. This workspace has a
  single developer. Automated sharding infrastructure is unnecessary overhead.
- **SAFe WSJF Multi-Team Scoring**: Estimating business value and time
  criticality points across committee matrices generates bureaucratic waste in
  single-worker repositories.
- **HTTP Reverse-Proxy Strangler Routing**: The `traces` binary is a local
  command-line executable. It does not route requests through API gateways or
  web proxies.
- **Complex Multi-Class Queues**: Kanban service classes (Expedite, Fixed Date,
  Standard) are designed for shared service teams with competing customer
  demands, not solo engineering projects.

## 6. Implications for Ticket 14 (Options Only)

The following evidence-backed options are available for ticket 14 decisions.
These are options, not decisions:

### Handling Bootstrapping and Framework Validation (Condition 1)

- *Option A (Walking Skeleton / Tracer Bullet)*: The pilot slice implements the
  minimal end-to-end BDD infrastructure (`tests/features/<pilot>.feature`,
  `tests/e2e/bdd/` step implementations, and `tests/e2e.rs` integration) while
  asserting one concrete guarantee (backed by [D1], [D2]).
- *Option B (Pure Infrastructure Spike)*: Separate infrastructure setup from
  behavioral testing into two sequential commits (backed by [D3]). *Tradeoff*:
  Lacks real customer value and delays framework revalidation under real
  scenario load.

### Handling Domains Under Active Refactor (Condition 2)

- *Option A (High-Invariance Surface First)*: Sequence initial adoption slices
  on surfaces with Frozen (F) or Default Frozen (D) tiers that pass ticket 07
  conditions and avoid moving seams (e.g. process exit codes, stream contracts,
  or basic note parsing), deferring moving seams until refactors settle (backed
  by [D7], [D10], [D20]).
- *Option B (Parallel Change via Facade)*: Implement scenarios on moving seams
  through facade abstractions (e.g. query dispatch facade) while underlying
  engines change (backed by [D6], [D7]). *Tradeoff*: Contradicts ticket 08/10
  decisions excluding moving facade seams.

### Handling Unverified Cross-Platform CI (Condition 3)

- *Option A (Early Walking Skeleton Gate)*: Require the pilot slice definition
  of done to verify clean execution across Ubuntu, macOS, and Windows runners
  before admitting subsequent feature slices (backed by [D1], [D8], [D22]).
- *Option B (Deferred CI Remediation)*: Land the pilot on Linux first and
  address Windows/macOS path/newline differences in a dedicated infrastructure
  ticket (backed by [D12]). *Tradeoff*: Violates the "stop and fix" principle
  and risks propagating cross-platform flakes across multiple features.

### Structuring the Deliberate Deferral ("Not Now") List

- *Option A (Shape Up No-Gos Pattern)*: Explicitly list deferred domains with
  their rationale (e.g. moving seams, known defects, class-level unit tests) in
  ticket 14(d), closing the adoption fog without generating open tickets (backed
  by [D20]).

### Defining Rollback and Abort Triggers

- *Option A (Trial Gate with Concrete Reversal)*: Adopt ThoughtWorks Trial
  semantics: if `rstest-bdd` introduces CI timeouts, cross-platform path
  incompatibilities, or unresolvable test flakes during the pilot, trigger
  immediate rollback via git revert and log the findings in an ADR update
  (backed by [D16], [D19]).

## 7. Open Gaps

1. **Exact Cross-Platform Execution Time**: The execution time of `rstest-bdd`
   code generation on Windows runners under `cargo-nextest` remains unmeasured
   until the pilot executes.
2. **Compile-Time Scaling Overhead**: Macro expansion overhead in `tests/e2e.rs`
   when scenario counts scale from 1 to 50 is unknown in this codebase.
