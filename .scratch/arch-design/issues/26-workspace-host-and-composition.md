# What is the workspace host, and what is the CLI allowed to own?

Type: grilling
Status: open
Blocked by: 13, 17, 18, 20, 21

## Question

Where does service composition live (config load + trust, schema load, index
refresh/persist, query execution, template service) and what is the CLI's remit?
Decide: a headless host/session type vs private CLI helpers
(`src/cli/mod.rs:335-502`); ownership of scaffolding (`cli::init`), query-source
normalisation (`normalize_source_input`) and task-filter synthesis; dialog seam
threading (only `init` and `template` prompt; `DialogProvider::select` silent
index-0 default; `NotInteractive` unreachable); stdout writer / EPIPE contract;
async/bridge story for a long-lived host if the forward-consumer decision says
it exists.

Facts: `survey/cli.md`, `survey/config-dialog.md` H4/H5, ADR-0004,
`src/cli/mod.rs`, `src/dialog/*`.

Alternatives: extract `WorkspaceHost`; keep composition in CLI but move domain
leakage out; per-command narrow context. Include keep-as-is. If the
forward-consumer decision refuses a second consumer, test whether the host is
still worth it for testability and cohesion.

Re-verified (main @ `207cc3fb`): `IndexerService::from(&Config)` now also
derives an `IndexFingerprint` from `config.digests()`
(`src/index/service.rs:47-59`), and a drift heal is only logged
(`tracing::info!`), never surfaced to the CLI; both bear on what the host
composes and reports.

Inputs: `research/04`, `research/08` (Cargo/jj/Ruff composition roots),
execution context, config ownership, read model, freshness.

User steer (after the compatibility-envelope ruling): a workspace split
**starting with the CLI crate** is being considered (see the crate-topology
ticket). That makes this ticket the gate for it: whatever composition,
scaffolding, source normalisation and task-filter synthesis the CLI should not
own must be settled here so the CLI crate can shrink to argument parsing,
prompting, output and exit codes. Include in the alternatives a core-API shape
that the CLI crate can consume without widening visibility beyond what an
outward-facing boundary needs.
