# Do the resolved decisions cohere, and has every subsystem been scrutinised?

Type: grilling
Status: open
Blocked by: 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32

## Question

Reconcile all resolved decisions codebase-wide: unresolved competing invariants,
inconsistent boundaries, duplicated responsibilities, coupling introduced,
missed recursive-deepening opportunities, ordering/dependency constraints for
the specification. Audit coverage: for each subsystem (root primitives,
temporal, note, schema, index, query, template, config, dialog, cli, path/file,
tests/benches/tooling) and each significant cross-subsystem interaction, confirm
it received scrutiny and state whether it is *redesigned*, *redrawn at the
edges*, or *withstands*. Decide whether any fog patch must graduate before
handoff and whether the destination is reached.

Standing checks added by re-verification (main @ `207cc3fb`): (1) re-run the
drift check before reconciling: `git diff 207cc3fb..main -- src tests docs` and
any divergence from `reverification.md`; (2) confirm the freshness decision
ratifies or revises the *landed* fingerprint design rather than re-deriving it,
and that the config-lifecycle decision accounts for the residual digest surface;
(3) the specification must state which existing ADRs it supersedes, including
ADR-0009 (implemented, still `proposed`).

Output: the list of specification-ready decisions with their dependency order
and the explicit non-decisions.

Inputs: every resolved ticket and `research/*`.

## Inputs from the schema typing decision

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). The map's
"provisionally withstands scrutiny" claim for the `FileClassExpander` seam is
overturned. Re-test the other provisional claims with the same adversarial
method, and treat the author's vault as one example, not as evidence.

Invariants changed by 20 to re-check: the `global` schema was "never applied to
a note" and is now a lowest-precedence layer for every note
(`src/schema/mod.rs:48-51`); first-listed-wins no longer decides class-versus-
class conflicts (it still decides `extends` parents); the index and fingerprint
stay schema-free because class binding is expanded at query time.

## Inputs from the IWE and Fileclass research

Research [12](../research/12-iwe-core-and-hosts-architecture.md),
[13](../research/13-iwe-query-schema-and-agent-write-gating.md) and
[14](../research/14-fileclass-model-binding-and-validation.md) were added after
ticket 20 resolved. Re-test the resolved decisions against their per-ticket
notes and the comment appended to ticket 20, and re-test the layering
principle against IWE's `liwe`/`diwe` split (ticket 32). Corrections to earlier
claims are listed in each research file's ledger and in ticket 20's comment.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md)
and the amendment to [ticket 19](./19-coercion-and-comparison-policy.md).
Audit the exclusive period boundary, period-versus-point membership versus
period identity and mixed-type sort, the fallible lazy-result API across
template callbacks, index coherence under two policies for one canonical
root, and held-config trust at each request admission. Record the
`file.day` to filename-only `file.date` migration and creation-time
nullability, plus the non-TTY/`--no-input` change, in the specification's
behavior-change table. ADR-0003's non-TTY first/empty fallback is
superseded by ticket 21, while index-by-position selection survives;
reconcile that accepted ADR's text alongside ADR-0009 before cutover.
Verify new filename prefix recognition is isolated from the frozen
vault-format parser and record changed week/quarter results in the
behavior-change table. A present creation/modification timestamp needs
the pinned zone to yield a civil `cdate`/`mdate`; absent creation is
`Null`, while zone failure is a typed error.
Keep current-code glossaries unchanged until the implementation updates
their domain vocabulary.
