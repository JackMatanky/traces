# Do the resolved decisions cohere, and has every subsystem been scrutinised?

Type: grilling
Status: open
Blocked by: 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28

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

See [ticket 16's Answer](./16-schema-note-typing-and-validation.md). The map's
"provisionally withstands scrutiny" claim for the `FileClassExpander` seam is
overturned. Re-test the other provisional claims with the same adversarial
method, and treat the author's vault as one example, not as evidence.

Invariants changed by 16 to re-check: the `global` schema was "never applied to
a note" and is now a lowest-precedence layer for every note
(`src/schema/mod.rs:48-51`); first-listed-wins no longer decides class-versus-
class conflicts (it still decides `extends` parents); the index and fingerprint
stay schema-free because class binding is expanded at query time.
