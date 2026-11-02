# 13: Migration and reuse policy

Type: grilling
Status: open
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
  threshold, a review of each new survivor.
- (d) **Coverage preservation.** At no point does the suite prove less than
  before. State the check at guarantee level, not by test count. Use ticket 12's
  recompute and this ticket's ledger. Decide the ledger format: the four columns
  (behavior, layer, defect class, surviving test) plus the research/10 row ID.
  Decide where it lives. Decide the order of steps in one change: the scenario
  lands and passes, then the old test goes.
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
