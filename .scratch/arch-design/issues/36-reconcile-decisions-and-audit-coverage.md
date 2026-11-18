# Do the resolved decisions cohere, and has every subsystem been scrutinised?

Type: grilling
Status: open
Blocked by: 18, 19, 20, 21, 22, 23, 24, 25, 27, 28, 29, 30, 31, 32, 33, 34, 35

## Question

Answer each numbered question separately.

1. Decide how to resolve competing invariants across the resolved tickets.
2. Decide how to remove inconsistent boundaries, duplicated responsibilities and
   added coupling.
3. Decide which recursive-deepening opportunities still need a decision.
4. Decide the dependency order for the specification.
5. Audit coverage across each subsystem and each significant cross-subsystem
   interaction. Include these subsystems:
   - Root primitives.
   - Temporal.
   - Note.
   - Schema.
   - Index.
   - Query.
   - Template.
   - Config.
   - Dialog.
   - CLI.
   - Path/file.
   - Tests/benches/tooling.
6. State whether each subsystem is *redesigned*, *redrawn at the edges*, or
   *withstands*. Confirm the evidence for each disposition.
7. Decide whether any fog patch must graduate before handoff.
8. Decide whether the destination is reached.
   Distinguish this audit from final specification readiness.
   Ticket 37 remains downstream of this audit.
9. Decide whether tickets 38 and 39 enter the specification hand-off.
   Include their accepted scope, or state their exclusion explicitly.

Do not introduce a dependency cycle to represent the final readiness check.
Check affected conclusions again after ticket 37 resolves.

Required reconciliation checks (added after re-verification on main @
`207cc3fb`):

- Re-run the drift check before reconciling: `git diff 207cc3fb..main -- src
  tests docs`. Check any divergence from `reverification.md`.
- Confirm that ticket 25 replaced the landed fingerprint design with a store
  identity. Confirm that ticket 22 accounted for the residual digest surface.
- State in the specification which existing ADRs it supersedes. Include ADR-0009
  (implemented, still `proposed`) and the stale glossary entry "Index
  Fingerprint".
- Collect the "Code delta" section of each resolved ticket. Order the deltas by
  dependency. This order is the refactor sequence. Report every ticket-to-ticket
  conflict. Do not report a ticket-to-code gap as a conflict.

Output:

- The specification-ready decisions.
- Their dependency order.
- The explicit non-decisions.
- The remaining conditions for final handoff.

Inputs: every resolved ticket and `research/*`.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The map's
"provisionally withstands scrutiny" claim for the `FileClassExpander` seam is
overturned. Re-test the other provisional claims with the same adversarial
method, and treat the author's vault as one example, not as evidence.

Invariants changed by 20 to re-check:

- The `global` schema was "never applied to a note" and is now a
  lowest-precedence layer for every note (`src/schema/mod.rs`).
- First-listed-wins no longer decides class-versus-class conflicts (it still
  decides `extends` parents).
- The index and fingerprint stay schema-free because class binding expands at
  query time.

## Inputs from the IWE and Fileclass research

Research [12](../research/12-iwe-core-and-hosts-architecture.md),
[13](../research/13-iwe-query-schema-and-agent-write-gating.md) and
[14](../research/14-fileclass-model-binding-and-validation.md) were added after
ticket 20 resolved. Re-test resolved decisions against their per-ticket notes
and the comment appended to ticket 20. Re-test the layering principle against
IWE's `liwe`/`diwe` split (ticket 35). Corrections to earlier claims are listed
in each research file's ledger and in ticket 20's comment.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md) and
the amendment to [ticket 19](./19-coercion-and-comparison-policy.md). Audit the
exclusive period boundary and period-versus-point membership. Audit period
identity, mixed-type sort and the fallible lazy-result API across template
callbacks. Audit index coherence under two policies for one canonical root.
Audit held-config trust at each request admission. Record the `file.day` to
filename-only `file.date` migration and creation-time nullability in the
behavior-change table.

Record the non-TTY/`--no-input` change in the behavior-change table. ADR-0003's
non-TTY first/empty fallback is superseded by ticket 21, while index-by-position
selection survives. Reconcile that accepted ADR's text alongside ADR-0009 before
cutover. Check that new filename prefix recognition is isolated from the frozen
vault-format parser. Record changed week/quarter results in the behavior-change
table.

A present creation/modification timestamp needs the pinned zone to yield a civil
`cdate`/`mdate`. Absent creation is `Null`, while zone failure is a typed error.
Keep current-code glossaries unchanged until the implementation updates their
domain vocabulary.

## Inputs from the live-buffer, query-core and path decisions

See [ticket 28](./28-live-buffer-and-persisted-index-consistency.md), [ticket
29](./29-query-core-and-plan.md) and [ticket
32](./32-path-and-capability-model.md).

- Check task-row group equivalence and Windows/Linux locking.
- Place durable benchmarks with ticket 35.
- Check the target implementation under the shipped `opt-level = "z"` profile.
- Run the Windows path-order property test against the real path type.
- Keep the heavy-note clone, `Arc<Note>` and single-thread rebuild evidence gaps
  explicit until measurements close them.
- Reconcile the shared change-table rows without duplicate owners.
- Keep ticket 27's journal adoption blockers separate from completed research.
- Separate design-changing measurements from production acceptance checks. A
  prototype can settle a design choice. Cutover needs checks of the actual
  implementation.
- Keep current-code glossaries and ADR history accurate until cutover. Record
  successor definitions and supersession notes with the implementation.

## Decision records and evidence gates

Use the same question numbers in each new Answer. Give each question one
disposition:

- Decided, with the choice, reason and evidence.
- Inherited, with the source ticket and decision.
- Evidence-blocked, with the missing evidence and its effect on the choice.
- Deferred, with the user's scope approval and the receiving ticket or fog item.

For each changed rule, name its invariant owner and downstream consumers. Name
the earlier decision that it supersedes. A pointer to conflicting text does not
resolve the conflict. Give each receiving ticket the accepted contract before
that ticket resolves.

Classify each proposed check by its purpose:

- Decision evidence: a result that can change the architecture choice. Complete
  this evidence before resolving the affected question.
- Implementation gate: a check of the implemented design. Record it for cutover.
  Do not claim that an unrun check passes.

Do not reopen a settled question without new evidence or an explicit scope
change. After a defect, repeat the affected checks rather than the whole review.
