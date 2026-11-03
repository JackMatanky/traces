# 13: Migration and reuse policy

Type: grilling
Status: resolved
Blocked by: 07, 08, 10, 11, 12

## Question

What happens to the existing tests when BDD arrives? Which tests convert to
scenarios? Which stay as they are? Which move, get stronger, or die?

Do not use test counts as a goal, a gate or a progress score. This is a
decision of ticket 12. A count is context only. The nextest list at session
start shows these counts (`--all-features`, 2026-11-02):

| Binary | Tests |
|---|---|
| `traces` (inline unit) | 3,802 |
| `traces::bin/traces` | 4 |
| `traces::e2e` | 64 |
| `traces::integration` | 41 |

The older figure of 2,469 tests was stale. The audits of ticket 08 counted 40
integration cases. Count again at session start and write the date. Treat the
audit tables as dated input.

## Inputs

- [Ticket 07](07-boundary-tiering.md): the six conditions, the exception
  classes, and the "retire or relabel incidental pins" duty.
- [Ticket 08](08-layer-responsibilities.md): the classification rule, the
  placement decision, the anti-goals, and the audit caveat. The audits are agent
  judgments from static reading. Spot checks found two defects.
- [research/08-audit-e2e.md](../research/08-audit-e2e.md) and
  [research/08-audit-integration.md](../research/08-audit-integration.md): the
  ledger input. The 27 scenario candidates are in the e2e audit.
- [Ticket 10](10-refactor-resilience.md) and
  [research/10](../research/10-invariance-map.md): the 88-row invariance list
  and the moving seams.
- [Ticket 11](11-feature-step-organization.md): the home for converted tests,
  the step lifecycle, and the header decision left to this ticket.
- [Ticket 12](12-coverage-gap-methodology.md): the coverage definition, the five
  row states, the ledger interface, and the open obligations for this ticket.
- [Ticket 09](09-bdd-e2e-prototype.md): the open CI and `check:layers` risks.
- `.scratch/integration-e2e-suite/spec.md`: read-only input. Its issues 15 and
  16 give the ledger method. Its "implemented" claims and per-test
  classification do not bind (ticket 08, decision d).
- [research/13](../research/13-migration-evidence.md): external evidence on
  retiring, moving and converting tests. It lists corrections to the raw
  reviews. Cite a claim from it only with its tag.

## Rules that earlier tickets already fixed

Do not reopen these rules. Apply them.

1. **The audit class "candidate" is not enough.** The audit judged each test
   against the e2e layer definition only. A conversion needs all six conditions
   of ticket 07 and the rule order of ticket 08. The 27 candidates are an upper
   bound. A candidate that touches a moving seam (research/10) or the condemned
   `FileClassExpander` seam does not convert.
2. **Overlap is allowed.** Ticket 07, condition 4, says that two independent
   test streams are an advantage. A test is not redundant only because a
   scenario proves the same guarantee. Decision (c) must resolve this.
3. **Key examples only.** Ticket 08 allows 5 to 10 key examples next to a
   proptest. It bans combinatorial outlines. A table-driven test converts only
   its key rows.
4. **One principal `When`, 3 to 5 steps** (ticket 11). The seven-command chain
   in `tests/e2e/workflow.rs` does not fit as one scenario.
5. **Assert codes, never miette-wrapped text** (ticket 07). The four weak e2e
   tests that match stderr substrings cannot convert as they are.
6. **No scenario pins a known defect** (ticket 10, decision b).
7. **Exception classes stay unit tests** (ticket 07). They are real-TTY
   rendering, in-process exit 130, `template` abort, timing freshness, and
   byte-level store surgery. They stay unit tests.
8. **The three baselines do not change** (ticket 07). Test deletion must not
   shrink the `traces::testing` facade. The integration audit found 34 unused
   facade symbols. That cleanup is outside this map.
9. **A converted Rust test goes only after this ticket's rules allow it**
   (ticket 11). A step lives only while a scenario uses it. A deleted test can
   leave a dead helper in `tests/e2e/support.rs`. Remove the helper in the same
   change.
10. **Gates and CI come first** (tickets 08 and 09). The placement decision
    binds only after a three-OS CI run. `check:layers` hides `git grep` errors
    until the adoption slice fixes it. Gate-owned rows read as covered only
    after that fix.
11. **The ledger rules of ticket 12 bind.** A converted guarantee re-earns
    "covered" at the deletion step. A scenario-class row with a pin in another
    layer needs a conversion target. The row is a gap if this ticket gives it
    none. Update each `pinned-elsewhere` pointer in the same change that
    deletes or converts the test it names. Record a mutation baseline update in
    the mutation docs, not in this ledger.

## Facts from the evidence review

[Research 13](../research/13-migration-evidence.md) checked these facts. Use
them in the decisions.

1. Overlap in coverage or mutation score is weak proof of redundancy. In one
   study, 20 percent of deletions of "redundant" tests lost coverage or
   mutation score.
2. A mutation proof for a deletion compares the escapes before and after the
   change. Scope it with `-m` or `-f`. A `--git-diff` run mutates only changed
   production lines, and a deletion changes none. Confirm this for `mutarust`
   with a probe in the first slice.
3. CI runs e2e tests on three operating systems. It runs unit and integration
   tests on Linux only. A move from e2e to a lower layer drops two systems.
4. CI has no mutation job and no coverage job. A mutation proof is a local
   step. Review must check that the change records it.
5. Two e2e tests carry `#[ignore]` and run only from a parent test. A plain
   inventory diff misses them.
6. The repository has no `proptest-regressions/` directory. Proptest seeds
   follow the source file path. A moved source file loses its seeds.

## Decisions to reach

HITL, grilling. Do not answer for the user.

- (a) **Convert rule.** State when an existing test moves to a scenario. Use
  ticket 07's six conditions and ticket 08's rule order. State the rule first.
  Then apply it to the 27 candidates in one table. Name the research/10 row of
  each candidate. Do not classify the other tests one by one.
- (b) **Keep rule.** State what stays as it is. Include the exception classes,
  tests that a proptest law owns, and unit tests for internal behavior. Do not
  translate for translation's sake. Address the tests of research/01 finding H1.
  They pass without coverage when stdin is a real terminal. Do not count them as
  proof.
- (c) **Die rule.** State what makes a test redundant. Reconcile it with the
  overlap allowance of ticket 07. State the burden of proof for each layer. Say
  how a session re-checks a "twin" claim from the audits before it deletes. State
  whether a mutation run on the module is part of the proof for a unit test
  (`mise run test:mutants -m <module>`). Ticket 12 sets the mutation posture: no
  threshold, a review of each new survivor. Decide who checks the mutation
  record, because CI has none.
- (d) **Coverage preservation.** At no point does the suite prove less than
  before. Decide these items:
  - The check, at guarantee level and not by test count. Use ticket 12's
    recompute and this ticket's ledger.
  - The ledger format: the four columns (behavior, layer, defect class,
    surviving test) plus the research/10 row ID.
  - Where the ledger lives.
  - The order of steps in one change: the scenario lands and passes, then the
    old test goes.
  - How the check treats `#[ignore]` tests.
- (e) **Scope and preconditions.** Decide whether to migrate everything at once
  or only the tests that an adoption wave touches. Name each precondition for
  deletion: the three-OS CI run, the `check:layers` fix, and the state of open
  arch-design tickets that move a seam. A Frozen pin must not leave before the
  seam moves. This decision feeds ticket 14.
- (f) **Move, strengthen and condemned tests.** The audits found tests that are
  not convert, keep or die cases:
  - 5 e2e and 10 integration tests that are unit tests in the wrong place
  - 7 weak assertions
  - 1 test on a condemned seam

  Decide the action for each class and its owner. Decide whether these actions
  belong to the adoption slices or to a separate hygiene change outside this map.
  State whether a move from e2e keeps one e2e smoke test.
- (g) **Headers, pins and inherited duties.** Decide these items:
  - The fate of the `//! Proves` and `Defect class:` headers of existing tests.
    Ticket 11 gives scenarios no `Defect class:` tag. Research/05 (G9) found that
    the taxonomy is undocumented and drifts. The ledger has a defect-class
    column. Decide where its values come from.
  - The same-size mtime canary: relabel it as a known limitation, or retire it.
  - The two sort tests named `..._ascending_by_default`: the rename. A rename
    does not close gap 02-G6.
  - The four unverified yaml.rs claims: who adds the unit pins, and in which
    ticket.

## Boundaries

- Do not redo the discovery of tickets 01 to 05.
- Do not write scenarios or choose feature file names. Ticket 11 owns both.
- Do not resolve a doc and code contradiction. Ticket 12, decision (f), owns it.
- Do not decide the agent policy for specs. Ticket 14 owns it.
- Do not change a baseline, a gate or a mutation setting.
- Do not execute a migration. The map ends before it.

## Answer records

- The convert, keep and die rules, with the burden of proof for each.
- The result for the 27 candidates, one table, with research/10 row IDs.
- The coverage-preservation check and the ledger format and home.
- The migration scope, the preconditions for deletion, and the constraint for
  ticket 14.
- The action and owner for each move, strengthen and condemned class.
- The decisions on headers, the mtime canary, the sort-test rename and the
  yaml.rs pins.

## Answer

Resolved in a grilling session on 2026-11-02. Evidence:
[research/13](../research/13-migration-evidence.md) and
[the candidate table](../research/13-candidate-table.md). The user confirmed
each rule below. Arch-design owns every later test update that its own changes
cause. This map carries no arch bookkeeping beyond the successor note in the
ledger.

### Convert rule (a)

- Only e2e Rust tests convert. Integration and unit tests never convert here.
  A scenario spawns the process (ticket 07). Ticket 07 also allows a facade
  seam when a frozen baseline holds the symbol. This ticket does not use that
  exception. A later slice needs a new decision to use it.
- An e2e test converts only when all of these hold:
  - A slice owns a research/10 row that the test covers.
  - The six conditions of ticket 07 hold.
  - The scenario fits one principal `When` and 3 to 5 steps.
  - The scenario asserts codes, never miette text.
  - The scenario proves all guarantees of the old test.
- A `needs-split` test becomes several key-example scenarios. It never becomes
  an outline.
- The 27 candidates: [the candidate table](../research/13-candidate-table.md)
  gives an eligibility list, not a decision. It shows 22 `convert-eligible`, 3
  `needs-split`, 1 `keep` (the workflow chain below) and 1 `blocked-by-seam`
  (`FileClassExpander`). The table is a
  static agent judgment, so the verdicts are an upper bound. Its condition
  checks are not proof. The slice author re-verifies conditions 3 to 6 before a
  conversion.
- `workflow.rs::init_trust_index_list_table_task_and_template_chain_through_one_project`
  (9 rows) stays as the one Rust composition smoke test. No single scenario
  proves the state that its seven commands carry.

### Keep rule (b)

- These tests stay, and migration never deletes them:
  - exception-class tests (ticket 07)
  - tests that a proptest law owns
  - unit tests for internal behavior
  - the two `#[ignore]` child tests in `tests/e2e/env_policy.rs`
  - the H1 TTY tests
- The H1 tests never count as proof of the interactive branch. CI has no TTY, so
  they run there. A change to them needs a production seam, so it is outside
  migration.

### Die rule and proof (c)

- A same-layer twin moves. A scenario and an e2e Rust test use the same
  `Sandbox` helpers, so they are not independent streams. Two copies double the
  edit surface for arch-design. The Rust test dies after the proof below.
- A cross-layer pin is never deleted only because a scenario exists. Ticket 07,
  condition 4, stands.
- A test with several rows dies only when every row has a survivor. Each
  assertion that the survivor does not cover is covered elsewhere, or the
  ledger records it as incidental and retired, with a reason.
- Every deletion needs all of these:
  - **Assertion comparison.** The survivor asserts the same exit code, the same
    exact stdout and the same exact stderr, empty ones included. A `contains`
    step never replaces an equality assertion.
  - **Kill demonstration.** Break the production code that the old test guards.
    Use the "defect caught" sentence of the ledger row. Run the break on the
    first commit, before the deletion. The old test and the survivor both fail.
    The ledger records the break in one line. This works in
    `src/cli`, which `mutarust.yml` excludes from mutation. The author picks
    the break, so the reviewer re-checks it.
  - **Mutation baseline compare**, only when the guarded code is outside
    `src/cli`. The module must pass `--fail-on-escaped` on the base commit
    first. Scope the run with `-m` or `-f`. Never use `--git-diff` or
    `--per-test` for the proof. A new escape means a lost guarantee.
  - **Twin re-read.** The session reads the twin test again. An audit "twin"
    claim is a lead only.
  - **Ledger row.** See decision (d).
  - **Review.** Review checks that the change records each item. CI has no
    mutation job.
- The same proof applies to duplicates, weak tests, misplaced tests and
  condemned tests. It names the surviving twin test.
- Reason: in one study, 20 percent of deletions of "redundant" tests lost
  coverage or mutation score (research/13, Bhatta et al.).

### Coverage preservation (d)

- **Ledger home:** `.scratch/bdd-testing/ledger.md`. It is append-only, with
  one section per slice. Update it in the same change as the deletion.
- **Ledger row:** one row per old test. Columns:
  - the research/10 namespaced row IDs of the test
  - the behavior
  - the former test (`binary-id::path`)
  - the former layer
  - the defect caught, as one plain sentence with no taxonomy
  - the survivor test or scenario title
  - the proof (the kill break, the mutation result)
  - a successor note for arch-design
- A row records no commit SHA. A recovery uses `git log -S '<test name>'`.
- A converted guarantee re-earns "covered" at the deletion step (ticket 12).
  Update each `pinned-elsewhere` pointer in the same change.
- **Step order:** one slice PR with two commits. The first commit adds the
  scenario. The second deletes the old test. The proof is
  `cargo nextest run --stress-count 100` on the scenario, and a green
  three-OS run on the PR head. The count is an unsourced heuristic. A flaky
  scenario fails CI loudly, so the risk is noise, not silent loss.
- **Inventory:** each PR records the name diff of
  `cargo nextest list --all-features --run-ignored all`, before and after.
  Each removed name needs a ledger row. Doctests are outside this check.
- A guarantee check never counts tests. Ticket 12 anti-metrics bind. A
  ratchet on the count of unmigrated tests is banned.

### Scope and preconditions (e)

- Scope is per slice. A slice owns the research/10 rows it names. Only tests
  that cover those rows convert or die. The other candidates stay as they are.
  Editing a file does not make its tests eligible.
- Two preconditions for any deletion:
  - Slice 1 gets one green three-OS run of the e2e placement. The result
    holds after that.
  - The `check:layers` exit-code fix has landed.
- If slice 1 fails on any OS, no deletion happens anywhere until the placement
  decision (ticket 08) is reopened. Adding scenarios stays allowed.
- There is no precondition on the state of open arch-design tickets. A ledger
  row names the successor of a moved or deleted test, so arch tables can find
  it. Arch tickets 22 and 25 already name e2e tests.

### Move, strengthen and condemned tests (f)

- A test under a slice's rows rides that slice with the proof of decision (c).
- Duplicates, misplaced tests and weak assertions outside a slice's rows are
  hygiene work outside this map. The `integration-e2e-suite` or a new issue
  owns them. They use the same proof and the same ledger. Ticket 14's roadmap
  names the owner of this backlog.
- A move out of e2e keeps one e2e smoke test per row. This applies only when
  the row's assigned class is e2e or scenario and no other e2e or scenario
  test pins it. A move drops macOS and Windows runs.
- The condemned `FileClassExpander` test stays. Arch-design updates it.

### Headers, canary, sort tests and yaml.rs (g)

- Existing `//! Proves` and `Defect class:` headers stay as they are. There is
  no mass edit and there are no new tag values. A deleted test takes its header
  with it. Scenarios get none (ticket 11).
- The mtime canary `same_size_mtime_restored_edit_reads_the_store_not_a_rebuild`
  stays in Rust. Relabel its header as a known limitation. The header says
  that arch-design ticket 25 owns the change. Never convert it.
- The two sort tests named `..._ascending_by_default` get a name that says
  "explicit ascending". The ledger records a rename with no coverage change.
  Gap 02-G6 stays a gap. The slice that covers the flags half closes it.
- The four yaml.rs claims: ticket 14's roadmap lists one non-scenario
  unit-test item. The first step verifies each claim against the code. If the
  code disagrees with the doc comment, code wins (ticket 12, decision f).

### Constraints for ticket 14

- Slice 1 carries: the three-OS check, the `check:layers` fix, the first
  ledger file and a probe of `mutarust` behavior on a test-only diff.
- The sixth gate check of ticket 12 also verifies that each ledger survivor
  exists.
- The step vocabulary needs exact-equality steps for stdout and stderr, empty
  ones included. Slice 1 checks docstring comparison on Windows. Add
  `*.feature text eol=lf` to `.gitattributes` if CRLF appears. The repository
  has no `.gitattributes` today.
- The roadmap lists the hygiene backlog with an owner and the yaml.rs unit item.
- The roadmap lists each test that an arch ticket names and a slice could move.

### Open and unverified

- Whether `mutarust --git-diff` ignores a test-only diff is `[INFERENCE]`.
  The proof never uses `--git-diff`, so the answer does not block.
- The Linux and Windows result for the e2e placement stays unverified until
  slice 1.
- The kill demonstration is manual and costs time for each deletion.
- The 100-run stress count is an arithmetic heuristic, not a source.
- The candidate table is a static agent judgment. Its condition checks are
  not proof.
