# Do the resolved decisions cohere, and has every subsystem been scrutinised?

Type: grilling
Status: open
Blocked by: 18, 19, 20, 21, 22, 23, 24, 25, 26, 28, 29, 30, 31, 32, 33, 34

## Question

Reconcile all resolved decisions codebase-wide. Check unresolved competing
invariants, inconsistent boundaries, duplicated responsibilities and added
coupling. Check missed recursive-deepening opportunities and
ordering/dependency constraints for the specification.

Audit coverage across each subsystem and each significant cross-subsystem
interaction. The subsystems include root primitives, temporal, note, schema,
index, query, template, config, dialog, cli, path/file and
tests/benches/tooling. Confirm each received scrutiny. State whether each
subsystem is *redesigned*, *redrawn at the edges*, or *withstands*. Decide
whether any fog patch must graduate before handoff and whether the
destination is reached.

Standing checks added by re-verification (main @ `207cc3fb`):

1. Re-run the drift check before reconciling:
   `git diff 207cc3fb..main -- src tests docs` and any divergence from
   `reverification.md`.
2. Confirm that the freshness decision (ticket 25) replaced the landed
   fingerprint design with a store identity. Confirm that the config-lifecycle
   decision (ticket 22) accounted for the residual digest surface.
3. State in the specification which existing ADRs it supersedes, including
   ADR-0009 (implemented, still `proposed`) and the stale glossary entry
   "Index Fingerprint".
4. Collect the "Code delta" section of each resolved ticket. Order the deltas
   by dependency. This order is the refactor sequence. Report every
   ticket-to-ticket conflict that you find. Do not report a ticket-to-code
   gap as a conflict.

Output: the list of specification-ready decisions with their dependency order
and the explicit non-decisions.

Inputs: every resolved ticket and `research/*`.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The map's
"provisionally withstands scrutiny" claim for the `FileClassExpander` seam is
overturned. Re-test the other provisional claims with the same adversarial
method, and treat the author's vault as one example, not as evidence.

Invariants changed by 20 to re-check:

- The `global` schema was "never applied to a note" and is now a
  lowest-precedence layer for every note (`src/schema/mod.rs:48-51`).
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
IWE's `liwe`/`diwe` split (ticket 34). Corrections to earlier claims are listed
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

A present creation/modification timestamp needs the pinned zone to yield a
civil `cdate`/`mdate`. Absent creation is `Null`, while zone failure is a typed
error. Keep current-code glossaries unchanged until the implementation updates
their domain vocabulary.
