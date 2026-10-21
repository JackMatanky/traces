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

## Inputs from the schema typing decision

See [ticket 16's Answer](./16-schema-note-typing-and-validation.md). Standing
principle: consumers (cli, lsp, mcp) are thin hosts of core; core's app layer
holds use cases as functions with explicit inputs (`check_note(before, after)`,
class resolution, template check). Host policy maps violation codes to ignore,
warn or reject. `traces check --before --after` is a thin cli command over a
core use case.

Fact: the cli currently decides whether to load schemas for a query
(`has_classes`, `src/cli/mod.rs:474-480`), i.e. it owns composition that the
app layer should own.

`check_note(before, after)` takes a path and content on each side, because
class binding depends on the path and tags and a move is a class change;
`traces check` gains `--before-path` and `--after-path`, and distinct exit codes
for a broken schema and for note violations.

Guardrail design pressure (16): the host must support validated CLI write
commands that share the MCP edit use case, `check_note` over a change set with
a per-file base and a destination path (for an isolated session or inbox
workflow, which are product features outside the map), and generating a
per-folder `AGENTS.md` snippet from class bindings.

MCP host (16, protocol revision 2026-07-28, which has no sessions or
`initialize`): the host runs its own guards (content-hash precondition,
validate-then-atomic-write with re-check, undo journal, delete/move/bulk
guards) and confirms class-set changes through an `input_required` result with
an integrity-protected `requestState`, denying when the client declares no
`elicitation`. Open here: stdio or HTTP transport, which decides whether any
per-caller state (a notice of out-of-band changes) is meaningful; it must
stay best effort. See the
[round-6 addendum](../research/16-prep/round6/I-mcp-server-guards.md).
