# 12: Coverage-gap methodology

Type: grilling
Status: resolved
Blocked by: 08

## Question

How do you find gaps in behavioral coverage and judge the quality of
specifications? Do not chase vanity metrics (test counts, line coverage as a
goal).

**Inputs**:

- ticket 08's layer classification
- the five inventories' pinned/unpinned flags (unpinned guarantees are gap
  candidates)
- existing measurement machinery (`llvm-cov nextest`, `mutarust` mutation
  testing, `docs/refs/mutation_testing.md`, `docs/refs/quality_gates_*`)
- the map's fog item on quality gates
- the doc/code contradictions that the five inventories list: research/01
  section 2, research/02 section 3, research/03 section D and research/05
  section 5. Rule 3 of ticket 07 needs a recorded resolution for each
  contradiction before a scenario can use the guarantee.

**Decisions to reach** (HITL, grilling). Reach these decisions:

- (a) Define what "covered" means per layer: a guarantee has an enforcing test
  of the right kind, not just an exercise.
- (b) Choose a practical gap-detection method a session can run: review cadence,
  mapping guarantees to tests, and tooling if any. Keep it lightweight.
- (c) State the scenario-quality criteria: what makes a specification good
  (precise observable contract, refactoring resilience, readable by a domain
  speaker, failure attribution). Say how review judges quality.
- (d) Name the explicit anti-metrics: the numbers this effort refuses to
  optimize (scenario count, coverage percentage targets) and why.
- (e) Decide the disposition of mutation testing and coverage tooling relative
  to scenarios: unchanged, extended, or explicitly out.
- (f) Triage the contradictions. For each contradiction that touches a scenario
  candidate, record one disposition: the doc wins, the code wins, or drop the
  guarantee. Start with these cases:
  - ADR-0002: a moved directory is "stale" in the ADR, but the code reports it
    as untrusted.
  - ADR-0008: the ADR names `SourceAtom` variants that do not exist.
  - The glossary claims about slugs, callouts and footnotes. No code
    implements them. Drop them, do not specify them.

  Ignore contradictions that touch no scenario candidate, such as stale line
  citations.

**Answer records**: the coverage definition, the gap-detection procedure, the
quality rubric, the anti-metrics list, and the contradiction dispositions.

## Answer

The user and the agent resolved this ticket together. Evidence:
[research/12-coverage-gap-methodology.md](../research/12-coverage-gap-methodology.md).
Section 0 maps the evidence to decisions (a)-(f).

### Decision (a): what covered means

A guarantee is **covered** when a test of the class that ticket 08's
classification rule assigns fails when the guarantee breaks. Exercise alone
never counts. Mutation escapes never un-cover a guarantee by themselves.

The population (the gap list denominator) is the research/10 invariance
list, its class-level dispositions, and ticket 07's exception classes.
Key each row as `<inventory-prefix>-<ID>`, for example `01-G23` or
`05-field`. One dialect only, applied to all research/10 tables in the
ticket-14 rewrite. Inventory items that fail ticket 07 conditions 1-2 are
out of scope. The list is an ID list, never a percentage.

Each row has one of five states:

- `covered: <scenario title> (reviewed <date>)`: scenario-class rows only.
  The title must exist in `tests/features/`.
- `gap`: no enforcing test of the assigned class exists.
- `waived: <owning ticket> (decided <date>)`: a recorded decision. A
  waiver ends when its owning ticket closes, unless a new decision
  re-asserts it.
- `pinned: <where>`: terminal. The assigned class is not scenario, and
  the pin sits in that assigned layer.
- `pinned-elsewhere: <conversion-target>`: scenario-class rows with a pin
  in another layer. The conversion target is mandatory. A row with no
  target after ticket 13 resolves becomes a gap at the next recompute.

Date rules keep the states honest:

- The covered date means "review postdates the current scenario content".
  When you change a scenario, bump the date of every row that cites it.
- The waived date records the decision. Its expiry is the owning ticket's
  status, not the calendar.
- `pinned` and `pinned-elsewhere` carry no date. The pointer and ticket
  13's ledger hold the time facts.

Layer-strict states describe steady state. During a migration, ticket 13's
four-column ledger is authoritative. Any layer that still enforces the
guarantee satisfies the coverage-preservation check.

Mutation coupling: a new escape in a module that owns a covered guarantee
triggers a re-review of that row. Ticket 13's deletion rule is the point
where a converted guarantee re-earns "covered".

Gate-owned rows carry one global note, not a per-row label: they read as
covered only after the `check:layers` exit-code fix lands in the adoption
slice. Flip the note when the fix lands.

Ledger interface between tickets 12, 13 and 14:

- A gap recompute re-derives every row from current facts. It is not a
  recount.
- When ticket 13 deletes or converts a test that a `pinned-elsewhere`
  pointer names, update that row in the same change.
- Ticket 13(d) cites this recompute as its guarantee-level view.
- Ticket 13's ledger rows cite the namespaced research/10 IDs they flip.

This ticket writes no research/10 lines. Status columns and the one-dialect
ID rewrite are a ticket-14 adoption-slice obligation. Until that lands,
this Answer is the record for gap state on the rows it names. The first
full recompute runs at slice-1 close.

### Decision (b): the gap-detection procedure

The method a session runs:

1. Keep research/10 as the rule inventory (the denominator).
2. Read the Status column (or, before ticket 14, this Answer's table).
3. Build the gap list: rows with state `gap`, plus rows whose pointers no
   longer resolve.
4. Review the list at each slice close with Adzic's three questions as the
   script.
5. Apply the PR checklist below to every scenario file you review.

Cadence: the PR checklist runs at review time. The gap recompute runs at
each slice close. Its delta scope: the rows the slice pins, all covered
rows, and all rows whose owning ticket closed. Record the recompute in one
map.md line: `gap recompute: slice N, DATE`. A row whose date is older
than the last recompute line reads as stale.

Drift detection is one new gate check. Ticket 11 authorized five
feature-text checks. This Answer adds a sixth check for ticket 14's
adoption slice:

- Extract Status pointers with a right-anchored parse. The covered form
  is `covered: <title> (reviewed <date>)`.
- Grep `tests/features/` from the filesystem, not from git grep.
- Match line-start `Scenario:` or `Scenario Outline:` with fixed strings.
- Name the research/10 row on failure.

The check proves existence, never enforcement. It stays dormant until the
Status columns exist.

Rejected on the record:

- The rstest-bdd tags probe. It breaks ticket 11's closed tag set.
- Compile-time probes. They couple the e2e binary to a design-doc
  snapshot.
- A nextest-list baseline. It cannot see the inventory-to-scenario link.
- A doc test. It runs in the wrong phase.
- Review-only records. Both drift directions stay silent.

### Decision (c): the quality rubric

The rubric is the review lens. Tickets 07 and 11 stay authoritative for
authoring. When the two differ, the authoring rule wins. This rubric only
judges the result.

Six criteria, all sourced in research/12 section 3.1:

1. The scenario states behavior, not implementation.
2. The scenario has 3 to 5 steps and one business rule.
3. The example data is concrete and realistic, not yes/no.
4. The assertions check observable outcomes and diagnostic codes.
5. A failure names the business rule without a source read.
6. Review produces quality. Authoring does not.

The PR checklist (apply at review of any scenario file):

- Does the scenario state behavior that survives an implementation change?
- Does it hold 3 to 5 steps and one business rule?
- Is the example data concrete and realistic?
- Do the assertions check observable outcomes and diagnostic codes?
- Can you name the failed business rule without a source read?

The paywalled SBES '19 checklist does not block this rubric. Example
Mapping stays an optional tool for hard features.

### Decision (d): anti-metrics

These numbers are barred as targets, gates or scores, permanently:

- Coverage percentage as a gate or a score.
- Scenario counts, test counts and pass-rate as KPIs.
- Any metric that fails Kaner and Bond's foreseeable-side-effect admission
  question.

Line coverage stays a bare diagnostic with no fail-under. The baseline
gate on `--fail-on-escaped` is a per-survivor review trigger, not a
score. No number is optimized, so it passes the admission question.

Two popular aphorisms stay folk heuristics. Do not cite "6-word scenario
title" or "code coverage is a floor, not a ceiling" as sourced rules.

Until ticket 14 runs the profraw count, treat llvm-cov numbers as blind
to scenario-executed src (research/12 section 4.4). The diagnostic is
advisory under this decision regardless.

### Decision (e): mutation and coverage tooling

Both tools stay unchanged.

The mutation posture source is docs/refs/mutation_testing.md section 4
and mutarust.yml. No ADR governs mutation testing.

- Manual per-module runs, baseline plus `--fail-on-escaped`, triage of
  new survivors per change. No universal MSI threshold. No primary source
  supports one. Google triages per change instead.
- Scoping with `-m`, `-f` or `--git-diff` is the cost lever once
  scenarios join the e2e binary.
- `--update-baseline` needs a written burden: mutant id, the reason no
  test should assert it, and the date. The record lives in the mutation
  docs, never in ticket 13's deletion ledger.

Changes applied by this ticket:

- mutarust.yml: the min_msi comment now records the decline. The old
  "enable ONLY if" invitation is gone.
- mutation_testing.md section 4 step 5 carries the escape-acceptance
  burden. Section 5's `--min-msi 60` example is marked not adopted.

Deferrals:

- The profraw empirical check goes to ticket 14 only. Ticket 09 is
  resolved and never ran it.
- The `--all-features` divergence in coverage tasks is a known issue for
  ticket 14. Fix it when the scenario phase is wired.

### Decision (f): contradiction dispositions

| # | Item | Disposition |
| --- | --- | --- |
| 1 | ADR-0002: a moved directory reads "stale"; the code refuses it as untrusted | Code wins. In-ADR Update applied now. The guarantee: a moved config directory is refused as untrusted. |
| 2 | ADR-0008: the ADR names SourceAtom variants that do not exist | Code wins. In-ADR Update applied now with the real variants. |
| 3 | Glossary slugs, callouts and footnotes: no code implements them | Drop the guarantees. Never specify them. In-tree markers are a TODO: this session excluded glossary files. Owner: a doc pass with arch ticket 23. |
| 4 | The doc claims exit 0 for both CommandOutcome variants; Interrupted exits 130 | Code wins. src/cli/mod.rs doc corrected now. G1 stays tier F. Obligation: a Unix spawned-signal regression case to ticket 14. |
| 5 | ADR-0004 promises a process-level Ctrl-C-130 confirmation that does not exist | ADR-0004 stands. Gap: no process-level case. The same obligation goes to ticket 14. |
| 6 | ADR-0002: "existence check only" vs read-and-hash | Code wins. Folded into the ADR-0002 Update. |
| 7 | Two sort tests named `…_ascending_by_default` both set `asc: true` | Code's default-sort guarantee stands: the unit pin at src/cli/mod.rs:843 proves the default half. The flags half is a gap. The two tests prove nothing and are never evidence. The rename decision belongs to ticket 13. Renaming does not close 02-G6. |
| 8 | The template glossary vs the loader order | Code wins. The src/template/GLOSSARY.md correction is a TODO: this session excluded glossary files. The scenario pins the full per-directory order, local stem before global exact. |
| 9 | Duplicate ADR-0009, both proposed | Renumber one file to 0010. Never mark superseded: both are live and pinned. The canonical pointer is recorded here. File surgery: a doc-pass obligation. |
| 10 | yaml.rs four claims, zero pins | Not a contradiction. Four unverified claims: binary-literals-as-integers, overflowing-float-stays-string, u64::MAX-errors, non-scalar-key-errors. G8 is unaffected. Leading-zero and merge-key are already pinned in src/note/metadata.rs. Pins are unit material under ticket 08 rule 6. Obligation: tickets 13/14. |

Family 9 (ADR-0009 on format version, B4): no disposition required. It is
class-level. The research/10 ownership sentence on this point is stale.

### Open obligations

Ticket 14(b) consumes this list:

- A Unix spawned-signal regression case (exit 130) to ticket 14's slice
  list.
- The G6 test rename/relabel decision to ticket 13.
- yaml.rs pins for the four unverified claims to tickets 13/14.
- research/10 Status columns plus the one-dialect ID rewrite to ticket 14,
  one pass.
- The sixth gate check to ticket 14. Quote this Answer's amendment.
- ADR-0009 renumber to a doc pass. Owner: the session that runs it.
- Glossary markers (slugs, callouts, footnotes, Navigation and Resolution
  entries) to a doc pass. TODO: this session excluded glossary files.
- The src/template/GLOSSARY.md loader-order correction. TODO, the same
  exclusion.
- The `check:layers` exit-code fix to the adoption slice (ticket 11 owns
  it).

### Machinery hygiene dispositions

| Item | Disposition |
| --- | --- |
| `check:layers` wraps 3 of 4 greps in `|| true` | Adoption-slice fix, ticket 11. Owner-health note in decision (a). |
| `test:unit` lacks `--no-tests fail` | Defer to the adoption slice. |
| Coverage runs e2e with `--all-features` | Known issue for ticket 14. |
| CRAP doc threshold 20.0 vs config 15.0 | Fixed now: docs/refs/crap_metric.md matches .cargo-crap.toml. |
| src/lib.rs says 46 facade identifiers; the baseline holds 47 | Fixed now: the doc says 47. |
| mutation_testing.md pins mutarust 0.1.10; mise.toml says latest | Fixed now: the doc says latest. |
| `proptest-regressions/` does not exist | Defer to the adoption slice. |

## Comments

- 2026-10-29: Pre-grilling research dossier consolidated from five parallel
  subagent passes (machinery survey, contradiction verification, and external
  best-practices reviews): [research/12-coverage-gap-methodology.md](../research/12-coverage-gap-methodology.md).
  Section 0 maps the evidence to decisions (a)-(f) and lists the open
  empirical questions. Status stays `claimed`; the grilling itself has not
  been held.
- 2026-11-02: Grilling held. The round-3 package is locked: five-state
  coverage definition, inventory-resident gap method with the sixth gate
  check, review-lens rubric, anti-metrics, unchanged tooling, ten
  contradiction dispositions. The Answer above records all five decision
  sets plus open obligations and machinery hygiene. Status: resolved.
