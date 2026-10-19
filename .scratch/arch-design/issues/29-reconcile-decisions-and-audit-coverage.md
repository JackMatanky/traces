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
