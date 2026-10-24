# What is the workspace host, and what is the CLI allowed to own?

Type: grilling
Status: open
Blocked by: 17, 21, 22, 24, 25, 12, 13

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

See [ticket 20's Answer](./20-schema-note-typing-and-validation.md). Standing
principle: consumers (cli, lsp, mcp) are thin hosts of core; core's app layer
holds use cases as functions with explicit inputs (`check_note(before, after)`,
class resolution, template check). Host policy maps violation codes to ignore,
warn or reject. `traces check --before --after` is a thin cli command over a
core use case.

Fact: the cli currently decides whether to load schemas for a query
(`has_classes`, `src/cli/mod.rs:474-480`), i.e. it owns composition that the app
layer should own.

`check_note(before, after)` takes a path and content on each side, because class
binding depends on the path and tags and a move is a class change; `traces
check` gains `--before-path` and `--after-path`, and distinct exit codes for a
broken schema and for note violations.

Guardrail design pressure (20): the host must support validated CLI write
commands that share the MCP edit use case, `check_note` over a change set with a
per-file base and a destination path (for an isolated session or inbox workflow,
which are product features outside the map), and generating a per-folder
`AGENTS.md` snippet from class bindings.

MCP host (20, protocol revision 2026-07-28, which has no sessions or
`initialize`): the host runs its own guards (content-hash precondition,
validate-then-atomic-write with re-check, undo journal, delete/move/bulk guards)
and confirms class-set changes through an `input_required` result with an
integrity-protected `requestState`, denying when the client declares no
`elicitation`. Open here: stdio or HTTP transport, which decides whether any
per-caller state (a notice of out-of-band changes) is meaningful; it must stay
best effort. See the [round-6
addendum](../research/20-prep/round6/I-mcp-server-guards.md).

## Inputs from the IWE research

See [research 12](../research/12-iwe-core-and-hosts-architecture.md) and
[13](../research/13-iwe-query-schema-and-agent-write-gating.md). IWE is a worked
example of the layering principle from ticket 20 [V]: `liwe` holds pure
operations (`rename`, `extract`, `inline`, `attach`, `delete`) as functions over
`&Graph` returning a plain `Changes { creates, updates, removes }`; `diwe`
applies them (`apply_changes`, workspace check, write only if changed); the CLI
writes the changes, the LSP converts them to a `WorkspaceEdit`, the MCP server
writes and syncs its graph. Read use cases are small structs borrowing the graph
(`DocumentFinder`, `DocumentReader`). No one-implementor trait crosses the seam.
[H] A plain changeset is a candidate shape for any write use case that later
appears, so hosts differ only in how they apply it; ticket 17 keeps write-back
out of scope, so this is design pressure only.

Its agent gate matches the `check_note` shape: `validate_pending_documents`
checks `(key, content)` pairs against bound schemas by building a throwaway
graph before anything is written (`crates/diwe/src/schema.rs`) [V]. The MCP gate
is always on (the round-4 "strict mode" reading was wrong: strict means
mandatory `expect` count guards on `iwe_query` mutations); multi-document
changes are all-or-nothing, and every write tool accepts `dry_run`
(`docs/mcp.md:125`) [V]. `expect` count guards are a stronger bulk guard than a
per-call cap and could be weighed when the MCP host is specified. Non-blocking
stats warnings (orphans, dangling links, near-duplicates) ride on successful
results once per session [V: iwec CHANGELOG], an in-process "notice" channel
with the best-effort limits already recorded in ticket 20.

## Inputs from the execution-context decision

See [ticket 21's Answer](./21-ambient-environment-and-execution-context.md).
Build independent workspaces with injected policies and immutable service
generations; `cli`, `lsp` and `mcp` consume the app layer in core, not CLI
globals. Represent canonical root plus policy/generation identity so a future
multi-root host needs no domain API rewrite; defer a registry. Coordinate at
most one writer policy for a canonical root's index with tickets 24/25/29;
separate in-memory identities do not isolate its store. At each new request,
reverify local config trust/content before using a held workspace; admitted
requests may finish with their pinned generation. Supply a new temporal frame
per request/render, lazily acquiring zone. The adapter selects dialog mode
explicitly: in CLI non-TTY and `--no-input`, authored defaults are permitted but
absent answers and exhausted presets fail. This changes ADR-0003's non-TTY
fallback, not its index-based selection contract; reconcile the accepted ADR
before implementation rather than silently editing its history.
